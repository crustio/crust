// Copyright (C) 2019-2021 Crust Network Technologies Ltd.
// This file is part of Crust.

//! Module to process claims from Ethereum addresses.
#![cfg_attr(not(feature = "std"), no_std)]

use sp_std::{prelude::*, convert::TryInto};
use frame_support::{
    decl_event, decl_storage, decl_module, decl_error, ensure,
    weights::{Weight},
    traits::{LockableCurrency, Get, Currency, WithdrawReasons, LockIdentifier}
};
use frame_system::{ensure_signed, ensure_root};
use codec::{Encode, Decode, HasCompact};
#[cfg(feature = "std")]
use serde::{self, Serialize, Deserialize};

use sp_runtime::{
    RuntimeDebug, DispatchResult, Perbill, traits::Zero
};

use primitives::BlockNumber;
use primitives::traits::LocksInterface;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(any(feature = "runtime-benchmarks", test))]
pub mod benchmarking;

pub mod weight;

const CRU_LOCK_ID: LockIdentifier = *b"crulock ";

pub trait WeightInfo {
    fn unlock() -> Weight;
}

/// The balance type of this module.
pub type BalanceOf<T> = <<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

pub trait Config: frame_system::Config {
    /// The overarching event type.
    type Event: From<Event<Self>> + Into<<Self as frame_system::Config>::Event>;
    type Currency: LockableCurrency<Self::AccountId>;
    /// One unlock period.
    type UnlockPeriod: Get<BlockNumber>;
    /// Weight information for extrinsics in this pallet.
    type WeightInfo: WeightInfo;
}

#[derive(Copy, Clone, Encode, Decode, Default, RuntimeDebug)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct LockType {
    pub delay: BlockNumber, // Init delay time. Currently only 0 and 6 months
    pub lock_period: u32 // 18 or 24
}

pub const CRU18:LockType = LockType {
    delay: 0 as BlockNumber,
    lock_period: 18
};

pub const CRU24:LockType = LockType {
    delay: 0 as BlockNumber,
    lock_period: 24
};

pub const CRU24D6:LockType = LockType {
    delay: 10 * 60 * 24 * 180 as BlockNumber, // 180 days
    lock_period: 18
};

#[derive(Copy, Clone, Encode, Decode, Default, RuntimeDebug)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct Lock<Balance: HasCompact> {
    // Total amount of the lock
    #[codec(compact)]
    pub total: Balance,
    // TODO: add unlocked amount for checking.
    // The last unlock block number
    pub last_unlock_at: BlockNumber,
    // The lock type, which is one of CRU18/CRU24/CRU24D6
    pub lock_type: LockType
}

impl<T: Config> LocksInterface<<T as frame_system::Config>::AccountId, BalanceOf<T>> for Module<T>
{
    fn create_cru18_lock(who: &<T as frame_system::Config>::AccountId, amount: BalanceOf<T>) {
        Self::create_or_extend_lock(who, &amount, CRU18);
    }
}

decl_event!(
    pub enum Event<T> where
        AccountId = <T as frame_system::Config>::AccountId,
    {
        /// Set global unlock from date
        UnlockStartedFrom(BlockNumber),
        /// Unlock success
        UnlockSuccess(AccountId, BlockNumber),
    }
);

decl_error! {
    pub enum Error for Module<T: Config> {
        /// Unlocking period already started and cannot set the unlock from again.
        AlreadyStarted,
        /// Unlocking period has not started.
        NotStarted,
        /// Invalid account which doesn't have CRU18 or CRU24l
        LockNotExist,
        /// Wait for the next unlock date.
        TimeIsNotEnough,
    }
}

decl_storage! {
    trait Store for Module<T: Config> as Claims {
        // Locks of CRU18, CRU24 and CRU24D6
        Locks get(fn locks): map hasher(blake2_128_concat) T::AccountId => Option<Lock<BalanceOf<T>>>;
        // The global unlock date
        UnlockFrom get(fn unlock_from): Option<BlockNumber>;
    }
    add_extra_genesis {
        config(genesis_locks):
            Vec<(T::AccountId, BalanceOf<T>, LockType)>;
        build(|config: &GenesisConfig<T>| {
            for (who, amount, lock_type) in &config.genesis_locks {
                <Module<T>>::issue_and_set_lock(who, amount, lock_type.clone());
            }
        });
    }
}

decl_module! {
    pub struct Module<T: Config> for enum Call where origin: T::Origin {
        type Error = Error<T>;

        fn deposit_event() = default;

        /// Set the global start date
        /// It can only be set once
        #[weight = 1000]
        fn set_unlock_from(origin, date: BlockNumber) -> DispatchResult {
            ensure_root(origin)?;
            let curr_bn = Self::get_current_block_number();

            // 1. If we already set the start date, ensure unlocking have not started.
            if let Some(unlock_from) = Self::unlock_from() {
                ensure!(curr_bn < unlock_from, Error::<T>::AlreadyStarted);
            }

            // 2. Set the start date.
            UnlockFrom::put(date);

            Self::deposit_event(RawEvent::UnlockStartedFrom(date));

            Ok(())
        }

        /// Unlock the CRU18 or CRU24 one period
        #[weight = T::WeightInfo::unlock()]
        fn unlock(origin) -> DispatchResult {
            let who = ensure_signed(origin)?;
            let curr_bn = Self::get_current_block_number();

            // 1. Ensure the unlock from is set and now unlocking period has started
            ensure!(Self::unlock_from().is_some() && curr_bn > Self::unlock_from().unwrap(), Error::<T>::NotStarted);
            // 2. Ensure who has the CRU18 or CRU24
            ensure!(Self::locks(&who).is_some(), Error::<T>::LockNotExist);

            let lock = Self::locks(&who).unwrap();
            let unlock_from = Self::unlock_from().unwrap();
            let curr_period = Self::round_bn_to_period(unlock_from, curr_bn);

            // 3. The first time that we would add the delay into checking
            let last_unlock_at = if lock.last_unlock_at == 0 {
                unlock_from + lock.lock_type.delay
            } else {
                lock.last_unlock_at
            };

            // 4. Ensure who has some CRU to unlock
            ensure!(curr_period > last_unlock_at, Error::<T>::TimeIsNotEnough);

            // 5. Count the total unlock period => Count the total unlock amount => Refresh the remaining locked amount
            let unlock_peroids = curr_period.saturating_sub(unlock_from).saturating_sub(lock.lock_type.delay) / T::UnlockPeriod::get();
            let unlock_amount = Perbill::from_rational_approximation(unlock_peroids, lock.lock_type.lock_period) * lock.total;
            let locked_amount = lock.total - unlock_amount;

            // 6. Update the lock
            Self::update_lock(&who, lock, locked_amount, curr_period);

            Self::deposit_event(RawEvent::UnlockSuccess(who, curr_period));

            Ok(())
        }
    }
}

impl<T: Config> Module<T> {
    fn get_current_block_number() -> BlockNumber {
        let current_block_number = <frame_system::Module<T>>::block_number();
        TryInto::<u32>::try_into(current_block_number).ok().unwrap()
    }

    fn update_lock(who: &T::AccountId, mut lock: Lock<BalanceOf<T>>, locked_amount: BalanceOf<T>, curr_period: BlockNumber) {
        // Remove the lock or set the new lock
        if locked_amount.is_zero() {
            T::Currency::remove_lock(
                CRU_LOCK_ID,
                who
            );
            <Locks<T>>::remove(who);
        } else {
            T::Currency::set_lock(
                CRU_LOCK_ID,
                who,
                locked_amount,
                WithdrawReasons::TRANSFER
            );
            // Update the last unlock at to the current period
            lock.last_unlock_at = curr_period;
            <Locks<T>>::insert(who, lock);
        }

    }

    fn create_or_extend_lock(who: &T::AccountId, amount: &BalanceOf<T>, lock_type: LockType) {
        <Locks<T>>::mutate_exists(&who, |maybe_lock| {
            match *maybe_lock {
                // If the lock already exist
                // Add the amount and set the last_unlock_at to 0
                // Don't change the type
                // Maybe we need to refuse the lock with different lock type
                Some(lock) => {
                    *maybe_lock = Some(Lock {
                        total: lock.total + amount.clone(),
                        last_unlock_at: 0,
                        lock_type: lock.lock_type
                    })
                },
                // Create a new lock
                None => {
                    *maybe_lock = Some(Lock {
                        total: amount.clone(),
                        last_unlock_at: 0,
                        lock_type
                    })
                }
            }

        });

        let total_amount = Self::locks(&who).unwrap().total;
        T::Currency::set_lock(
            CRU_LOCK_ID,
            who,
            total_amount,
            WithdrawReasons::TRANSFER
        );
    }

    fn round_bn_to_period(unlock_bn: BlockNumber, bn: BlockNumber) -> BlockNumber {
        ((bn - unlock_bn) / T::UnlockPeriod::get()) * T::UnlockPeriod::get() + unlock_bn
    }

    pub fn issue_and_set_lock(who: &T::AccountId, amount: &BalanceOf<T>, lock_type: LockType) {
        // Issue the money
        T::Currency::deposit_creating(who, *amount);
        // Create the lock
        Self::create_or_extend_lock(who, amount, lock_type);
    }
}

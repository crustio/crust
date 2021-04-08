// Copyright (C) 2019-2021 Crust Network Technologies Ltd.
// This file is part of Crust.

//! Module to process claims from Ethereum addresses.
#![cfg_attr(not(feature = "std"), no_std)]

use sp_std::{prelude::*, convert::TryInto};
use frame_support::{
    decl_event, decl_storage, decl_module, decl_error, ensure,
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

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

const CRU_LOCK_ID: LockIdentifier = *b"crulock ";

/// The balance type of this module.
pub type BalanceOf<T> = <<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

pub trait Config: frame_system::Config {
    /// The overarching event type.
    type Event: From<Event<Self>> + Into<<Self as frame_system::Config>::Event>;
    type Currency: LockableCurrency<Self::AccountId>;
    /// One unlock period. It should be one month
    type OneUnlockPeriod: Get<BlockNumber>;
}

#[derive(Copy, Clone, Encode, Decode, Default, RuntimeDebug)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct LockType {
    pub delay: BlockNumber,
    pub lock_period: u32
}

pub const CRU18:LockType = LockType {
    delay: 0 as BlockNumber,
    lock_period: 18
};

pub const CRU24:LockType = LockType {
    delay: 0 as BlockNumber,
    lock_period: 24
};

pub const CRU24_WITH_DELAY:LockType = LockType {
    delay: 10 * 60 * 24 * 180 as BlockNumber, // 180 days
    lock_period: 18
};

#[derive(Copy, Clone, Encode, Decode, Default, RuntimeDebug)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct Lock<Balance: HasCompact> {
    #[codec(compact)]
    pub total: Balance,
    pub last_unlock_at: BlockNumber,
    pub lock_type: LockType
}

decl_event!(
    pub enum Event<T> where
        AccountId = <T as frame_system::Config>::AccountId,
    {
        /// Someone be the new Reviewer
        SetStartDateSuccess(BlockNumber),
        /// Remove one month lock success
        FreeOneMonthSuccess(AccountId, BlockNumber),
    }
);

decl_error! {
    pub enum Error for Module<T: Config> {
        /// Superior not exist, should set it first
        AlreadySet,
        /// Not started yet
        NotStarted,
        /// Invalid person who try to unlock
        LockNotExist,
        /// Time is too short
        TimeIsNotEnough,
    }
}

decl_storage! {
    // A macro for the Storage config, and its implementation, for this module.
    // This allows for type-safe usage of the Substrate storage database, so you can
    // keep things around between blocks.
    trait Store for Module<T: Config> as Claims {
        Locks get(fn locks): map hasher(blake2_128_concat) T::AccountId => Option<Lock<BalanceOf<T>>>;
        StartDate get(fn start_date): Option<u32>;
    }
    add_extra_genesis {
        config(genesis_locks):
            Vec<(T::AccountId, BalanceOf<T>, LockType)>;
        build(|config: &GenesisConfig<T>| {
            for (who, amount, lock_type) in &config.genesis_locks {
                <Module<T>>::create_new_lock(who, amount, lock_type.clone());
            }
        });
    }
}

decl_module! {
    pub struct Module<T: Config> for enum Call where origin: T::Origin {
        type Error = Error<T>;

        fn deposit_event() = default;

        #[weight = 1000]
        fn set_start_date(origin, date: BlockNumber) -> DispatchResult {
            ensure_root(origin)?;

            ensure!(Self::start_date().is_none(), Error::<T>::AlreadySet);

            StartDate::put(date);

            Self::deposit_event(RawEvent::SetStartDateSuccess(date));

            Ok(())
        }

        #[weight = 1000]
        fn unlock_one_period(origin) -> DispatchResult {
            let who = ensure_signed(origin)?;

            ensure!(Self::start_date().is_some(), Error::<T>::NotStarted);

            ensure!(Self::locks(&who).is_some(), Error::<T>::LockNotExist);

            let mut lock = Self::locks(&who).unwrap();
            let start_date = Self::start_date().unwrap();
            let curr_bn = Self::get_current_block_number();

            let target_unlock_bn = if lock.last_unlock_at == 0 {
                start_date + lock.lock_type.delay + T::OneUnlockPeriod::get()
            } else {
                lock.last_unlock_at + T::OneUnlockPeriod::get()
            };

            ensure!(curr_bn >= target_unlock_bn, Error::<T>::TimeIsNotEnough);

            Self::update_lock(&who, &lock, start_date + lock.lock_type.delay, target_unlock_bn);
            lock.last_unlock_at = target_unlock_bn;

            <Locks<T>>::insert(&who, lock);

            Self::deposit_event(RawEvent::FreeOneMonthSuccess(who, target_unlock_bn));

            Ok(())
        }
    }
}

impl<T: Config> Module<T> {
    fn get_current_block_number() -> BlockNumber {
        let current_block_number = <frame_system::Module<T>>::block_number();
        TryInto::<u32>::try_into(current_block_number).ok().unwrap()
    }

    fn update_lock(who: &T::AccountId, lock: &Lock<BalanceOf<T>>, start_date: BlockNumber, target_unlock_bn: BlockNumber) {
        let free_periods = (target_unlock_bn - start_date) / T::OneUnlockPeriod::get();
        let free_amount = Perbill::from_rational_approximation(free_periods, lock.lock_type.lock_period) * lock.total;
        let locked_amount = lock.total - free_amount;
        if locked_amount.is_zero() {
            T::Currency::remove_lock(
                CRU_LOCK_ID,
                who
            );
        } else {
            T::Currency::set_lock(
                CRU_LOCK_ID,
                who,
                locked_amount,
                WithdrawReasons::TRANSFER
            );
        }

    }

    pub fn create_new_lock(who: &T::AccountId, amount: &BalanceOf<T>, lock_type: LockType) {
        <Locks<T>>::mutate_exists(&who, |maybe_lock| {
            match *maybe_lock {
                Some(lock) => {
                    *maybe_lock = Some(Lock {
                        total: lock.total + amount.clone(),
                        last_unlock_at: 0,
                        lock_type: lock.lock_type
                    })
                },
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
}

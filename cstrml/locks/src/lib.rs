// Copyright (C) 2019-2021 Crust Network Technologies Ltd.
// This file is part of Crust.

//! Module to process claims from Ethereum addresses.
#![cfg_attr(not(feature = "std"), no_std)]

use sp_std::prelude::*;
use sp_io::{hashing::keccak_256, crypto::secp256k1_ecdsa_recover};
use frame_support::{
    decl_event, decl_storage, decl_module, decl_error, ensure,
    traits::{LockableCurrency, Get}
};
use frame_system::{ensure_signed, ensure_root, ensure_none};
use codec::{Encode, Decode};
#[cfg(feature = "std")]
use serde::{self, Serialize, Deserialize, Serializer, Deserializer};

use sp_runtime::{
    RuntimeDebug, DispatchResult,
    transaction_validity::{
        TransactionLongevity, TransactionValidity, ValidTransaction, InvalidTransaction, TransactionSource,
    },
    traits::{
        Zero, StaticLookup, Saturating
    },
};

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
    /// One release period. It should be one month
    type OnePeriod: Get<BlockNumber>;
}

pub enum LockType {
    CRU18(OffsetType::Now, LockPeriod::Eighteen),
    CRU24(OffsetType::Now, LockPeriod::TwentyFour),
    CRU24WithOffset(OffsetType::SixMonth, LockPeriod::Eighteen)
}

pub enum OffsetType {
    Now = 0 as BlockNumber,
    SixMonth = 2_592_000 as BlockNumber
}

pub enum LockPeriod {
    TwentyFour = 24,
    Eighteen = 18,
}

pub struct Lock<Balance> {
    pub total: Balance,
    pub last_unlock_at: BlockNumber,
    pub offset: OffsetType,
    pub lock_period: LockPeriod
}

decl_event!(
    pub enum Event<T> where
        Balance = BalanceOf<T>,
        AccountId = <T as frame_system::Config>::AccountId,
        BlockNumber = <T as frame_system::Config>::BlockNumber,
    {
        /// Someone be the new Reviewer
        SetStartDateSuccess(BlockNumber),
        /// Remove one month lock success
        FreeOneMonthSuccess,
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
            for &(who, amount, lock_type) in &config.genesis_locks {
                Self::create_new_lock(who, amount, lock_type);
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

            <StartDate<T>>::put(date);

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
                ensure!(curr_bn >= lock.offset.into(), Error::<T>::TimeIsNotEnough);
                start_date + T::OnePeriod::get()
            } else {
                lock.last_unlock_at + T::OnePeriod::get()
            }

            ensure!(curr_bn >= target_unlock_bn, Error::<T>::TimeIsNotEnough);

            Self::update_lock(&who, &lock, start_date, target_unlock_bn);
            lock.last_unlock_at = target_unlock_bn;

            <Locks<T>>::insert(&who, lock);

            Self::deposit_event(RawEvent::FreeOneMonthSuccess);

            Ok(())
        }
    }
}

impl<T: Config> Module<T> {
    fn get_current_block_number() -> BlockNumber {
        let current_block_number = <system::Module<T>>::block_number();
        TryInto::<u32>::try_into(current_block_number).ok().unwrap()
    }

    fn update_lock(who: &T::AccountId, lock: &Lock<BalanceOf<T>>, start_date: BlockNumber, target_unlock_bn: BlockNumber) {
        let free_periods = (target_unlock_bn - start_date) / T::OnePeriod::get();
        let free_amount = Perbill::from_rational_approximation(free_periods, lock.lock_period.into()) * lock.total;
        let locked_amount = lock.total - free_amount;
        T::Currency::set_lock(
            CRU_LOCK_ID,
            who,
            locked_amount,
            WithdrawReasons::TRANSFER
        );
    }

    fn create_new_lock(who: &T::AccountId, amount: &BalanceOf<T>, lock_type: LockType) {
        <Locks<T>>::mutate_exists(&who, |maybe_lock| match maybe_lock {
            Some(ref lock) => {
                lock.total += amount.clone();
                lock.last_unlock_at = 0;
            },
            None() => {
                Lock {
                    total: amount.clone(),
                    last_unlock_at: 0,
                    offset: lock_type.0,
                    lock_period: lock_type.1
                }
            }
        });

        let total_amount = Self::locks().unwrap().total;
        T::Currency::set_lock(
            CRU_LOCK_ID,
            who,
            total_amount,
            WithdrawReasons::TRANSFER
        );

        let _ = T::Currency::deposit_creating(who, amount.clone());
    }
}

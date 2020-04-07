//! The Substrate Node runtime. This can be compiled with `#[no_std]`, ready for Wasm.

#![cfg_attr(not(feature = "std"), no_std)]
#![feature(option_result_contains)]

use codec::{Decode, Encode};
use frame_support::{decl_event, decl_module, decl_storage, dispatch::DispatchResult, ensure};
use sp_std::{str};
use system::ensure_signed;
use sp_runtime::{traits::StaticLookup};
use tee;

#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};

// Crust runtime modules
use primitives::{MerkleRoot, Balance, BlockNumber};

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[derive(Debug, PartialEq, Eq, Clone, Encode, Decode, Default)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct StorageOrder<T> {
    pub file_indetifier: MerkleRoot,
    pub file_size: u64,
    pub expired_duration: BlockNumber,
    pub expired_on: BlockNumber,
    pub destination: T
}


/// An event handler for sending storage order
pub trait OnOrderStorage<AccountId> {
    fn storage_fee_transfer(transactor: &AccountId, dest: &AccountId, value: Balance) -> u64;
}

impl<AId> OnOrderStorage<AId> for () {
    fn storage_fee_transfer(_: &AId, _: &AId, _: Balance) -> u64 {
        // transfer the fee and return order id
        0
    }
}

/// The module's configuration trait.
pub trait Trait: system::Trait + tee::Trait {
    /// The overarching event type.
    type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
    type OnOrderStorage: OnOrderStorage<Self::AccountId>;
}

// This module's storage items.
decl_storage! {
    trait Store for Module<T: Trait> as Market {
        pub StorageOrders get(fn storage_orders): map (T::AccountId, u64) => Option<StorageOrder<T::AccountId>>; // Cannot use MerkleRoot as the key. cannot open the apps
    }
}

// The module's dispatchable functions.
decl_module! {
    /// The module declaration.
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        // Initializing events
        // this is needed only if you are using events in your module
        fn deposit_event() = default;
        /// TODO: organize these parameters into a struct.
        fn store_storage_order(
            origin,
            dest: <T::Lookup as StaticLookup>::Source,
            #[compact] value: Balance, // Keep it now and refactor it later
            file_indetifier: MerkleRoot,
            file_size: u64,
            expired_duration: BlockNumber,
            expired_on: BlockNumber
        ) -> DispatchResult
            {
                let who = ensure_signed(origin)?;
                let dest = T::Lookup::lookup(dest)?;
                let storage_order = StorageOrder::<T::AccountId> {
                    file_indetifier,
                    file_size,
                    expired_duration,
                    expired_on,
                    destination: dest.clone()
                };
                // 1. Do check and should do something

                ensure!(Self::check_storage_order(&storage_order).is_ok(), "Storage Order is invalid!");

                Self::maybe_insert_sorder(&who, &dest, value, &storage_order);
                
                // 3. Emit storage order event
                Self::deposit_event(RawEvent::ReportStorageOrders(who, storage_order));

                Ok(())
            }
    }
}

impl<T: Trait> Module<T> {
    // private function can be built in here
    fn check_storage_order(so: &StorageOrder<T::AccountId>) -> DispatchResult {
        ensure!(
            &<tee::Module<T>>::get_work_report(&so.destination).is_some(),
            "Cannot find work report!"
        );
        ensure!(
            &<tee::Module<T>>::get_work_report(&so.destination).unwrap().empty_workload >= &so.file_size,
            "Empty work load is not enough!"
        );
        Ok(())
    }
    // sorder is equal to storage order
    fn maybe_insert_sorder(source: &T::AccountId, dest: &T::AccountId, value: Balance, so: &StorageOrder<T::AccountId>) {
        let fee_id = T::OnOrderStorage::storage_fee_transfer(&source, &dest, value);
        <StorageOrders<T>>::insert((&source, &fee_id), &so);
    }
}

decl_event!(
    pub enum Event<T>
    where
        AccountId = <T as system::Trait>::AccountId,
    {
        ReportStorageOrders(AccountId, StorageOrder<AccountId>),
    }
);

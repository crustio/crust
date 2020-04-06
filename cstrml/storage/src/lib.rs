//! The Substrate Node runtime. This can be compiled with `#[no_std]`, ready for Wasm.

#![cfg_attr(not(feature = "std"), no_std)]
#![feature(option_result_contains)]

use codec::{Decode, Encode};
use frame_support::{decl_event, decl_module, decl_storage, dispatch::DispatchResult, ensure,
    traits::{
        Currency, ExistenceRequirement, LockableCurrency,
	}};
use sp_std::{str, vec::Vec};
use system::ensure_signed;
use sp_runtime::{traits::StaticLookup};
use tee;

#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};

// Crust runtime modules
use primitives::{MerkleRoot};

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

pub type BalanceOf<T> =
    <<T as Trait>::Currency as Currency<<T as system::Trait>::AccountId>>::Balance;

#[derive(Debug, PartialEq, Eq, Clone, Encode, Decode, Default)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct StorageOrder<T> {
    pub file_indetifier: MerkleRoot,
    pub file_size: u64,
    pub expired_duration: u64,
    pub expired_on: u64,
    pub destination: T
}


/// An event handler for 
pub trait OnOrderStroage<AccountId, T> {
    fn storage_fee_transfer(transactor: &AccountId, dest: &AccountId, value: BalanceOf<T>) => u64
}

impl<AId, T> OnOrderStroage<AId, T> for () {
    fn storage_fee_transfer(_: &AId, _: &AId, _: BalanceOf<T>){
        0
    }
}

/// The module's configuration trait.
pub trait Trait: system::Trait + tee::Trait {
    /// The overarching event type.
    type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
    type OnOrderStroage: OnOrderStroage<Self::AccountId, Self>;
    // To do. Support work report check
}

// TODO: add add_extra_genesis to unify chain_spec
// This module's storage items.
decl_storage! {
    trait Store for Module<T: Trait> as Storage {
        pub StorageOrders get(fn storage_orders) config(): map (T::AccountId, u64) => Option<StorageOrder<T::AccountId>>; // Cannot use MerkleRoot as the key. cannot open the apps
    }
}

// The module's dispatchable functions.
decl_module! {
    /// The module declaration.
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        // Initializing events
        // this is needed only if you are using events in your module
        fn deposit_event() = default;

        fn store_storage_order(
            origin,
            dest: <T::Lookup as StaticLookup>::Source,
            #[compact] value: BalanceOf<T>,
            file_indetifier: MerkleRoot,
            file_size: u64,
            expired_duration: u64,
            expired_on: u64
        ) -> DispatchResult
            {
                let who = ensure_signed(origin)?;
                let dest = T::Lookup::lookup(dest)?;
                let fee_id = T::OnOrderStroage::storage_fee_transfer(&who, &dest, value);

                let storage_order = StorageOrder::<T::AccountId> {
                    file_indetifier,
                    file_size,
                    expired_duration,
                    expired_on,
                    destination: dest
                };
                // 1. Do check and should do something

                ensure!(Self::storage_order_check(&storage_order).is_ok(), "Storage Order is invalid!");

                // 2. Store the storage order
                <StorageOrders<T>>::insert((&who, &fee_id), &storage_order);

                // // 3. Emit storage order event
                // Self::deposit_event(RawEvent::ReportStorageOrders(who, storage_order));

                Ok(())
            }
    }
}

impl<T: Trait> Module<T> {
    // private function can be built in here
    fn storage_order_check(so: &StorageOrder<T::AccountId>) -> DispatchResult {
        ensure!(
            &<tee::Module<T>>::get_last_work_report(&so.destination).unwrap().empty_workload >= &so.file_size,
            "Cannot find work report or empty work load is not enough!"
        );
        Ok(())
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

//! The Substrate Node runtime. This can be compiled with `#[no_std]`, ready for Wasm.

#![cfg_attr(not(feature = "std"), no_std)]
#![feature(option_result_contains)]

use codec::{Decode, Encode};
use frame_support::{
    decl_event, decl_module, decl_storage, decl_error, dispatch::DispatchResult, ensure,
    weights::SimpleDispatchInfo
};
use sp_std::{str, vec::Vec, convert::TryInto};
use system::ensure_signed;
use sp_runtime::{traits::StaticLookup};
use tee;

#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};

// Crust runtime modules
use primitives::{
    MerkleRoot, Balance, BlockNumber, Hash,
    constants::time::MINUTES
};

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[derive(Debug, PartialEq, Eq, Clone, Encode, Decode, Default)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct StorageOrder<AccountId> {
    pub file_identifier: MerkleRoot,
    pub file_size: u64,
    pub created_at: BlockNumber,
    pub expired_on: BlockNumber,
    pub provider: AccountId,
    pub client: AccountId,
}


/// An event handler for sending storage order
pub trait OnOrderStorage<AccountId> {
    fn pay_order(transactor: &AccountId, dest: &AccountId, value: Balance) -> Hash;
}

impl<AId> OnOrderStorage<AId> for () {
    fn pay_order(_: &AId, _: &AId, _: Balance) -> Hash {
        // transfer the fee and return order id
        // TODO: using random to generate non-duplicated order id
        Hash::default()
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
        /// A mapping from storage provider to order id
        pub Providers get(fn providers):
        map hasher(twox_64_concat) T::AccountId => Option<Vec<Hash>>;

        /// A mapping from clients to order id
        pub Clients get(fn clients):
        map hasher(twox_64_concat) T::AccountId => Option<Vec<Hash>>;

        /// Order details iterated by order id
        pub StorageOrders get(fn storage_orders):
        map hasher(twox_64_concat) Hash => Option<StorageOrder<T::AccountId>>;
    }
}

decl_error! {
    /// Error for the market module.
    pub enum Error for Module<T: Trait> {
        /// Duplicate order id.
		DuplicateOrderId,
    }
}

// The module's dispatchable functions.
decl_module! {
    /// The module declaration.
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        // Initializing events
        // this is needed only if you are using events in your module
        fn deposit_event() = default;

        type Error = Error<T>;

        /// TODO: organize these parameters into a struct.
        #[weight = SimpleDispatchInfo::default()]
        fn add_storage_order(
            origin,
            dest: <T::Lookup as StaticLookup>::Source,
            #[compact] value: Balance,
            file_identifier: MerkleRoot,
            file_size: u64,
            expired_on: BlockNumber
        ) -> DispatchResult
            {
                let who = ensure_signed(origin)?;
                let provider = T::Lookup::lookup(dest)?;
                let created_at = TryInto::<u32>::try_into(<system::Module<T>>::block_number()).ok().unwrap();

                // 1. Expired should be greater than created
                ensure!(created_at + 30 * MINUTES < expired_on, "order should keep at least 30 minutes");

                // 2. Construct storage order
                let storage_order = StorageOrder::<T::AccountId> {
                    file_identifier,
                    file_size,
                    created_at,
                    expired_on,
                    provider: provider.clone(),
                    client: who.clone()
                };

                // 3. Do check and should do something
                ensure!(Self::check_storage_order(&storage_order).is_ok(), "storage order is invalid!");

                // 4. Pay the order and (maybe) add storage order
                if Self::maybe_insert_sorder(&who, &provider, value, &storage_order) {
                    // a. emit storage order event
                    Self::deposit_event(RawEvent::ReportStorageOrders(who, storage_order));
                } else {
                    // b. emit error
                    Err(Error::<T>::DuplicateOrderId)?
                }

                Ok(())
            }
    }
}

impl<T: Trait> Module<T> {
    // IMMUTABLE PRIVATE
    fn check_storage_order(so: &StorageOrder<T::AccountId>) -> DispatchResult {
        ensure!(
            <tee::Module<T>>::get_work_report(&so.provider).is_some(),
            "Cannot find work report!"
        );
        ensure!(
            <tee::Module<T>>::get_work_report(&so.provider).unwrap().empty_workload >= so.file_size,
            "Empty work load is not enough!"
        );
        Ok(())
    }

    // MUTABLE PRIVATE
    // sorder is equal to storage order
    fn maybe_insert_sorder(client: &T::AccountId,
                           provider: &T::AccountId,
                           value: Balance,
                           so: &StorageOrder<T::AccountId>) -> bool {
        let order_id = T::OnOrderStorage::pay_order(&client, &provider, value);

        // This should be false, cause we don't allow duplicated `order_id`
        if <StorageOrders<T>>::contains_key(&order_id) {
            false
        } else {
            let mut client_orders = Self::clients(client).unwrap_or_default();
            let mut provider_orders = Self::providers(provider).unwrap_or_default();
            client_orders.push(order_id.clone());
            provider_orders.push(order_id.clone());

            <Clients<T>>::insert(client, client_orders);
            <Providers<T>>::insert(provider, provider_orders);
            <StorageOrders<T>>::insert(order_id, so);
            true
        }
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

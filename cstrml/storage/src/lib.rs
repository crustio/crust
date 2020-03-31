//! The Substrate Node runtime. This can be compiled with `#[no_std]`, ready for Wasm.

#![cfg_attr(not(feature = "std"), no_std)]
#![feature(option_result_contains)]

use codec::{Decode, Encode};
use frame_support::{decl_event, decl_module, decl_storage, dispatch::DispatchResult, ensure,
    traits::{
        ExistenceRequirement,
	}};
use sp_std::convert::TryInto;
use sp_std::{str, vec::Vec};
use system::ensure_signed;
use sp_runtime::{
	RuntimeDebug, DispatchError,
	traits::{
		Zero, StaticLookup, Member, CheckedAdd, CheckedSub,
		MaybeSerializeDeserialize, Saturating, Bounded,
	},
};

#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};

// Crust runtime modules
use primitives::{MerkleRoot, PubKey};

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[derive(Debug, PartialEq, Eq, Clone, Encode, Decode, Default)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct StorageOrder { // Need to confirm this name. FileOrder?
    pub file_indetify: MerkleRoot,
    pub file_size: u64,
    pub expired_duration: u64,
    pub expired_on: u64
}

/// The module's configuration trait.
pub trait Trait: system::Trait {
    /// The overarching event type.
    type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
    type Balance: Default;
    // To do. Support work report check
}

// TODO: add add_extra_genesis to unify chain_spec
// This module's storage items.
decl_storage! {
    trait Store for Module<T: Trait> as Storage {
        pub StorageOrders get(fn storage_orders) config(): map T::AccountId => Option<StorageOrder>; //这个结构要再考虑一下 List of Option?
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
            #[compact] value: T::Balance,
            storage_order: StorageOrder) -> DispatchResult
            {
                let who = ensure_signed(origin)?;
                T::Balance::transfer(&who, &dest, &value, ExistenceRequirement::AllowDeath);

                // 1. Do check and should do something
                ensure!(Self::storage_order_check(&storage_order).is_ok(), "Storage Order is invalid!");

                // 2. Store the storage order
                <StorageOrders<T>>::insert(&who, &storage_order);

                // 3. Emit storage order event
                Self::deposit_event(RawEvent::StorageOrders(who, storage_order));

                Ok(())
            }
    }
}

impl<T: Trait> Module<T> {
    // IMMUTABLE PUBLIC
    pub fn storage_order_check_pub(so: &StorageOrder) -> DispatchResult {
        Ok(())
    }

    // private function can be built in here
    fn storage_order_check(so: &StorageOrder) -> DispatchResult {
        Ok(())
    }
}

decl_event!(
    pub enum Event<T>
    where
        AccountId = <T as system::Trait>::AccountId,
    {
        StorageOrders(AccountId, StorageOrder),
    }
);

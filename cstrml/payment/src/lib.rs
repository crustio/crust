#![cfg_attr(not(feature = "std"), no_std)]
#![feature(option_result_contains)]

use codec::{Decode, Encode};
use frame_support::{
    decl_event, decl_module, decl_storage, decl_error, dispatch::DispatchResult, ensure, Parameter,
    weights::SimpleDispatchInfo,
    traits::
    {
        Randomness, LockableCurrency, schedule::Named as ScheduleNamed,
        WithdrawReasons, Currency, LockIdentifier
    }
};
use sp_std::{prelude::*, convert::TryInto};
use system::{ensure_signed, ensure_root};
use sp_runtime::{traits::{StaticLookup, Dispatchable}};

#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};

// Crust runtime modules
use primitives::{
    Balance, BlockNumber,
};

use market::{OrderStatus, MarketInterface, Payment};

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

const PAYMENT_ID: LockIdentifier = *b"payment ";

pub type BalanceOf<T> =
    <<T as Trait>::Currency as Currency<<T as system::Trait>::AccountId>>::Balance;

impl<T: Trait> Payment<<T as system::Trait>::AccountId,
    <T as system::Trait>::Hash, BalanceOf<T>> for Module<T>
{
    fn pay_sorder(client: &<T as system::Trait>::AccountId,
                  provider: &<T as system::Trait>::AccountId,
                  value: BalanceOf<T>) -> T::Hash {
        let bn = <system::Module<T>>::block_number();
        let bh: T::Hash = <system::Module<T>>::block_hash(bn);
        let seed = [
            &bh.as_ref()[..],
            &client.encode()[..],
            &provider.encode()[..],
        ].concat();

        // it can cover most cases, for the "real" random
        let sorder_id = T::Randomness::random(seed.as_slice());
        let when = <system::Module<T>>::block_number();
        T::Currency::set_lock(
            PAYMENT_ID,
            &client,
            value,
            WithdrawReasons::all(),
        );

        let result = T::Scheduler::schedule_named(
            sorder_id,
            when,
            None,
            63,
            Call::payment_by_instalments(
                client.clone(),
                provider.clone(),
                value,
                sorder_id.clone()
            ).into(),
        );
        <Payments<T>>::insert(sorder_id, value);
        return sorder_id;
    }
}

/// The module's configuration trait.
pub trait Trait: system::Trait {
    type Proposal: Parameter + Dispatchable<Origin=Self::Origin> + From<Call<Self>>;
    /// The payment balance.
    type Currency: LockableCurrency<Self::AccountId, Moment = Self::BlockNumber>;

    /// The overarching event type.
    type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;

    /// Something that provides randomness in the runtime.
    type Randomness: Randomness<Self::Hash>;

    /// Interface for interacting with a market module.
    type MarketInterface: MarketInterface<Self::AccountId, Self::Hash>;

    /// The Scheduler.
	type Scheduler: ScheduleNamed<Self::BlockNumber, Self::Proposal>;
}

// This module's storage items.
decl_storage! {
    trait Store for Module<T: Trait> as Market {
        /// A mapping from storage provider to order id
        pub Payments get(fn payments):
        map hasher(twox_64_concat) T::Hash => BalanceOf<T>;
    }
}

decl_error! {
    /// Error for the market module.
    pub enum Error for Module<T: Trait> {
        /// Duplicate order id.
		DuplicateOrderId,
		/// No workload
		NoWorkload,
		/// Not provider
		NotProvider,
		/// File duration is too short
		DurationTooShort
    }
}

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        type Error = Error<T>;

        // Initializing events
        // this is needed only if you are using events in your module
        fn deposit_event() = default;

        /// Enact a proposal from a referendum. For now we just make the weight be the maximum.
		#[weight = SimpleDispatchInfo::MaxNormal]
        fn payment_by_instalments(
            origin,
            client: <T as system::Trait>::AccountId,
            provider: <T as system::Trait>::AccountId,
            value: BalanceOf<T>,
            order_id: T::Hash
        ) -> DispatchResult {
			ensure_root(origin)?;
            Self::do_payment_by_instalments(&client, &provider, value, &order_id);
            Self::deposit_event(RawEvent::PaymentSuccess(client));
            Ok(())
		}
    }
}

impl<T: Trait> Module<T> {

    fn do_payment_by_instalments(
        client: &<T as system::Trait>::AccountId,
        provider: &<T as system::Trait>::AccountId,
        value: BalanceOf<T>,
        order_id: &T::Hash
    ) -> DispatchResult {
        let sorder =
        T::MarketInterface::maybe_get_sorder(order_id).unwrap_or_default();
        Ok(())
    }
}

decl_event!(
    pub enum Event<T>
    where
        AccountId = <T as system::Trait>::AccountId,
    {
        PaymentSuccess(AccountId),
    }
);
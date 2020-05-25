#![cfg_attr(not(feature = "std"), no_std)]
#![feature(option_result_contains)]

use codec::{Decode, Encode, HasCompact};
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
use sp_runtime::{traits::{StaticLookup, Dispatchable, Zero}};

#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};

// Crust runtime modules
use primitives::{
    Balance, BlockNumber, constants::time::MINUTES
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
        Self::add_new_payments_lock(client, value.clone());
        <Payments<T>>::insert(sorder_id, PaymentLedger{
            total: value,
            already_paid: Zero::zero()
        });
        return sorder_id;
    }

    fn start_delayed_pay(sorder_id: &T::Hash) {
            let sorder = T::MarketInterface::maybe_get_sorder(sorder_id).unwrap_or_default();
            let interal =  TryInto::<T::BlockNumber>::try_into(MINUTES).ok().unwrap();
            let times = (sorder.expired_on - sorder.completed_on)/MINUTES + 1;
            let value = Self::payments(sorder_id).unwrap_or_default().total;
            let piece_value: BalanceOf<T> = BalanceOf::<T>::from(TryInto::<u32>::try_into(value).ok().unwrap()/times);
            let result = T::Scheduler::schedule_named(
                sorder_id,
                <system::Module<T>>::block_number(),
                Some((interal, times)),
                63,
                Call::payment_by_instalments(
                    sorder.client.clone(),
                    sorder.provider.clone(),
                    piece_value,
                    sorder_id.clone()
                ).into(),
            );
        }
}

#[derive(Debug, PartialEq, Eq, Clone, Encode, Decode, Default)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct PaymentLedger<Balance: HasCompact + Zero> {
    #[codec(compact)]
    pub total: Balance,
    #[codec(compact)]
    pub already_paid: Balance
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
        map hasher(twox_64_concat) T::Hash => Option<PaymentLedger<BalanceOf<T>>>;
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
			ensure_root(origin.clone())?;
            Self::do_payment_by_instalments(origin, &client, &provider, value, &order_id);
            Self::deposit_event(RawEvent::PaymentSuccess(client));
            Ok(())
		}
    }
}

pub trait BalanceInterface<Origin, AccountId, Balance>: system::Trait {
    /// Disable a given validator by stash ID.
    ///
    /// Returns `true` if new era should be forced at the end of this session.
    /// This allows preventing a situation where there is too many validators
    /// disabled and block production stalls.
    fn transfer(origin: Origin, client: &AccountId, provider: &AccountId, value: Balance);
}

impl<T: Trait> BalanceInterface<T::Origin, <T as system::Trait>::AccountId, BalanceOf<T>> for T where T: balances::Trait {
    fn transfer(
        origin: T::Origin,
        client: &<T as system::Trait>::AccountId,
        provider: &<T as system::Trait>::AccountId,
        value: BalanceOf<T>) {
        <balances::Module<T>>::force_transfer(origin, client, provider, value);
    }
}

impl<T: Trait> Module<T> {

    fn do_payment_by_instalments(
        origin: T::Origin,
        client: &<T as system::Trait>::AccountId,
        provider: &<T as system::Trait>::AccountId,
        value: BalanceOf<T>,
        order_id: &T::Hash
    ) -> DispatchResult {
        Ok(())
    }

    fn add_new_payments_lock(
        client: &<T as system::Trait>::AccountId,
        value: BalanceOf<T>
    ) {
        let current_lock = Self::total_lock(client);
        T::Currency::set_lock(
            PAYMENT_ID,
            &client,
            value + current_lock,
            WithdrawReasons::all(),
        );
    }

    fn total_lock(client: &<T as system::Trait>::AccountId) -> BalanceOf<T> {
        T::MarketInterface::clients(client).unwrap_or_default().iter().fold(
            Zero::zero(), |acc, &order_id| {
                let payment_ledger = Self::payments(order_id).unwrap_or_default();
                acc + payment_ledger.total - payment_ledger.already_paid
            }
        )
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
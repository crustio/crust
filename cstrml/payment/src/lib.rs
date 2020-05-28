#![cfg_attr(not(feature = "std"), no_std)]
#![feature(option_result_contains)]

use codec::{Decode, Encode, HasCompact};
use frame_support::{
    decl_event, decl_module, decl_storage, decl_error, dispatch::DispatchResult, Parameter,
    traits::
    {
        Randomness, schedule::Named as ScheduleNamed,
        Currency, ReservableCurrency
    }
};
use sp_std::{prelude::*, convert::TryInto};
use system::{ensure_root};
use sp_runtime::{traits::{StaticLookup, Dispatchable, Zero, Convert}};

#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};

// Crust runtime modules
use primitives::{
    constants::time::MINUTES
};

use market::{OrderStatus, MarketInterface, Payment};

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

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
        if T::Currency::reserve(&client, value).is_ok() {
            <Payments<T>>::insert(sorder_id, PaymentLedger{
                total: value,
                already_paid: Zero::zero()
            });
        }
        return sorder_id;
    }

    fn start_delayed_pay(sorder_id: &T::Hash) {
            let sorder = T::MarketInterface::maybe_get_sorder(sorder_id).unwrap_or_default();
            let interval =  TryInto::<T::BlockNumber>::try_into(MINUTES).ok().unwrap();
            let times = (sorder.expired_on - sorder.completed_on)/MINUTES + 1;
            let total = Self::payments(sorder_id).unwrap_or_default().total;
            let piece_value: BalanceOf<T> = BalanceOf::<T>::from(TryInto::<u32>::try_into(total).ok().unwrap()/times + 1);
            let _ = T::Scheduler::schedule_named(
                sorder_id.encode(),
                <system::Module<T>>::block_number() + interval, // must have a delay
                Some((interval, times)),
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
    type Currency: ReservableCurrency<Self::AccountId>;

    /// The overarching event type.
    type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;

    /// Something that provides randomness in the runtime.
    type Randomness: Randomness<Self::Hash>;

    /// Interface for interacting with a market module.
    type MarketInterface: MarketInterface<Self::AccountId, Self::Hash>;

    /// The Scheduler.
    type Scheduler: ScheduleNamed<Self::BlockNumber, Self::Proposal>;
    
    /// Used to transfer
    type CurrencyToBalance: Convert<BalanceOf<Self>, u64> + Convert<u128, BalanceOf<Self>>;

    type BalanceInterface: self::BalanceInterface<Self::Origin, Self::AccountId, BalanceOf<Self>>;
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
    }
}

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        type Error = Error<T>;

        // Initializing events
        // this is needed only if you are using events in your module
        fn deposit_event() = default;

        /// Enact a proposal from a referendum. For now we just make the weight be the maximum.
		#[weight = 1_000_000]
        fn payment_by_instalments(
            origin,
            client: <T as system::Trait>::AccountId,
            provider: <T as system::Trait>::AccountId,
            value: BalanceOf<T>,
            order_id: T::Hash
        ) -> DispatchResult {
            ensure_root(origin.clone())?;
            // 0. check left currency
            let payment_ledger = Self::payments(order_id).unwrap_or_default();
            let real_value = value.min(payment_ledger.total - payment_ledger.already_paid);

            if !Zero::is_zero(&real_value) {
                // 1. unreserve one piece currency
                T::Currency::unreserve(
                    &client,
                    real_value);
                // Check the order status
                if let Some(sorder) = T::MarketInterface::maybe_get_sorder(&order_id) {
                    match sorder.order_status {
                        OrderStatus::Success => {
                            // 3. (Maybe) transfer the currency
                            if T::BalanceInterface::maybe_transfer(origin.clone(), &client, &provider, real_value) {
                                // 4. update payments
                                <Payments<T>>::mutate(&order_id, |payment_ledger| {
                                    if let Some(p) = payment_ledger {
                                        p.already_paid += real_value;
                                    }
                                });
                                Self::deposit_event(RawEvent::PaymentSuccess(client));
                            }

                        },
                        // TODO: Deal with failure status
                        _ => {}
                    }
                }
            }
            Ok(())
		}
    }
}

pub trait BalanceInterface<Origin, AccountId, Balance>: system::Trait {
    fn maybe_transfer(origin: Origin, client: &AccountId, provider: &AccountId, value: Balance) -> bool;
}

impl<T: Trait> BalanceInterface<T::Origin, <T as system::Trait>::AccountId, BalanceOf<T>> for T where T: balances::Trait {
    fn maybe_transfer(
        origin: T::Origin,
        client: &<T as system::Trait>::AccountId,
        provider: &<T as system::Trait>::AccountId,
        value: BalanceOf<T>) -> bool {
            let to_balance = |b: BalanceOf<T>| T::Balance::from(<T::CurrencyToBalance as Convert<BalanceOf<T>, u64>>::convert(b) as u32);
            <balances::Module<T>>::force_transfer(
                origin,
                T::Lookup::unlookup(client.clone()),
                T::Lookup::unlookup(provider.clone()),
                to_balance(value)
            ).is_ok()
    }
}

impl<T: Trait> Module<T> {
    // Unused function right now
    fn total_reserved(client: &<T as system::Trait>::AccountId) -> BalanceOf<T> {
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
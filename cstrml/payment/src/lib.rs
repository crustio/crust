#![cfg_attr(not(feature = "std"), no_std)]
#![feature(option_result_contains)]

use codec::{Decode, Encode, HasCompact};
use frame_support::{
    decl_event, decl_module, decl_storage, decl_error, ensure, dispatch::DispatchResult, Parameter,
    traits::{
        schedule::Named as ScheduleNamed, schedule::HARD_DEADLINE, ExistenceRequirement,
        Currency, ReservableCurrency
    }
};
use sp_std::{prelude::*, convert::{TryInto}};
use system::{ensure_root};
use sp_runtime::{traits::{Dispatchable, Zero, Convert}};

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

type BalanceOf<T> =
    <<T as Trait>::Currency as Currency<<T as system::Trait>::AccountId>>::Balance;

impl<T: Trait> Payment<<T as system::Trait>::AccountId,
    <T as system::Trait>::Hash, BalanceOf<T>> for Module<T>
{
    fn reserve_sorder(sorder_id: &T::Hash, client: &T::AccountId, amount: BalanceOf<T>) -> bool {
        if T::Currency::reserve(&client, amount.clone()).is_ok() {
            <Payments<T>>::insert(sorder_id, Ledger {
                total: amount,
                paid: Zero::zero(),
                unreserved: Zero::zero()
            });
            return true
        }
        false
    }

    // Ideally, this function only be called under an `EXISTED` storage order
    // TODO: We should return whether `Scheduler` successful
    fn pay_sorder(sorder_id: &T::Hash) {
        // 1. Storage order should exist
        if let Some(so) = T::MarketInterface::maybe_get_sorder(sorder_id) {
            if let Some(ledger) = Self::payments(sorder_id) {
                // 2. Calculate duration
                let minute = TryInto::<T::BlockNumber>::try_into(MINUTES).ok().unwrap();
                let duration = (so.expired_on - so.completed_on) / MINUTES + 1;

                // 3. Calculate slot payment amount
                let total_amount = ledger.total;
                let slot_amount: BalanceOf<T> =
                    <T::CurrencyToBalance as Convert<u128, BalanceOf<T>>>::
                    convert(<T::CurrencyToBalance as Convert<BalanceOf<T>, u128>>::
                    convert(total_amount) / duration as u128 + 1);

                // 4. Arrange a scheduler
                // TODO: What if returning an error?
                let _ = T::Scheduler::schedule_named(
                    sorder_id.encode(),
                    <system::Module<T>>::block_number() + minute, // must have a delay
                    Some((minute, duration)),
                    HARD_DEADLINE,
                    Call::slot_pay(sorder_id.clone(), slot_amount).into(),
                );
            }
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Encode, Decode, Default)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct Ledger<Balance: HasCompact + Zero> {
    #[codec(compact)]
    pub total: Balance,
    #[codec(compact)]
    pub paid: Balance,
    #[codec(compact)]
    pub unreserved: Balance,
}

/// The module's configuration trait.
pub trait Trait: system::Trait {
    type Proposal: Parameter + Dispatchable<Origin=Self::Origin> + From<Call<Self>>;

    /// The payment balance.
    type Currency: ReservableCurrency<Self::AccountId>;

    /// The overarching event type.
    type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;

    /// Used to transfer
    type CurrencyToBalance: Convert<BalanceOf<Self>, u128> + Convert<u128, BalanceOf<Self>>;

    /// The Scheduler.
    type Scheduler: ScheduleNamed<Self::BlockNumber, Self::Proposal>;

    /// Interface for interacting with a market module.
    type MarketInterface: MarketInterface<Self::AccountId, Self::Hash, BalanceOf<Self>>;
}

// This module's storage items.
decl_storage! {
    trait Store for Module<T: Trait> as Market {
        /// A mapping from storage order id to payment ledger info
        pub Payments get(fn payments):
        map hasher(twox_64_concat) T::Hash => Option<Ledger<BalanceOf<T>>>;
    }
}

decl_error! {
    /// Error for the market module.
    pub enum Error for Module<T: Trait> {
        /// No more payment
        NoMoreAmount,
        /// Storage order not exist
        NoStorageOrder,
        /// Payment ledger not exist
        NoPaymentInfo,
    }
}

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        type Error = Error<T>;

        // Initializing events
        // this is needed only if you are using events in your module
        fn deposit_event() = default;

        /// Called by `Scheduler`(with root rights) ONLY, and for now only support `StorageOrder`.
        /// This function will check `sorder_id` is added in `Payments` and `StorageOrders`
        ///
        /// <weight>
        /// - Independent of the arguments. Moderate complexity.
        /// - O(1).
        /// - 8 extra DB entries.
        /// </weight>
        #[weight = 1_000_000]
        fn slot_pay(origin, sorder_id: T::Hash, amount: BalanceOf<T>) -> DispatchResult {
            ensure_root(origin.clone())?;

            // 1. Ensure payment ledger existed
            ensure!(Self::payments(&sorder_id).is_some(), Error::<T>::NoPaymentInfo);

            // 2. Ensure storage order existed
            ensure!(T::MarketInterface::maybe_get_sorder(&sorder_id).is_some(), Error::<T>::NoStorageOrder);

            // 3. Prepare payment amount
            let ledger = Self::payments(&sorder_id).unwrap_or_default();
            let real_amount = amount.min(ledger.total - ledger.paid);

            // 4. Ensure amount > 0
            ensure!(!Zero::is_zero(&real_amount), Error::<T>::NoMoreAmount);

            // 5. Get storage order
            let sorder = T::MarketInterface::maybe_get_sorder(&sorder_id).unwrap_or_default();
            let client = sorder.client.clone();
            let provider = sorder.provider.clone();

            // 6. [DB Write] Unreserved 1 slot amount
            T::Currency::unreserve(
                &client,
                real_amount);

            // 7. [DB Write] Nice move 🥳
            // Unreserved value will be added anyway.
            // If the status of storage order status is `Failed`,
            // the CRUs will be just unreserved(aka, unlocked) to client-self.
            <Payments<T>>::mutate(&sorder_id, |ledger| {
                if let Some(p) = ledger {
                    p.unreserved += real_amount;
                }
            });

            // 8. Check storage order status
            match sorder.status {
                OrderStatus::Success => {
                    // 9. [DB Write] (Maybe) Transfer the amount
                    // TODO: What if this failed several time, paid will be smaller than it should be?
                    if T::Currency::transfer(&client, &provider, real_amount, ExistenceRequirement::AllowDeath).is_ok() {
                        // 10. [DB Write] Update ledger
                        <Payments<T>>::mutate(&sorder_id, |ledger| {
                            if let Some(l) = ledger {
                                l.paid += real_amount;
                            }
                        });
                        Self::deposit_event(RawEvent::PaymentSuccess(client));
                    }
                },
                _ => {}
            }

            Ok(())
        }
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
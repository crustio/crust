#![cfg_attr(not(feature = "std"), no_std)]
#![feature(option_result_contains)]

use codec::{Decode, Encode, HasCompact};
use frame_support::{
    decl_event, decl_module, decl_storage, decl_error, dispatch::DispatchResult, Parameter,
    storage::IterableStorageDoubleMap,
    weights::Weight,
    traits::{
        ExistenceRequirement, Currency, ReservableCurrency, Get
    }
};
use sp_std::{
    prelude::*,
    convert::{TryInto}
};
use sp_runtime::{
    traits::{
        Dispatchable, Zero, Convert, CheckedDiv
    }
};

#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};

// Crust runtime modules
use primitives::BlockNumber;

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
            <PaymentLedgers<T>>::insert(sorder_id, PaymentLedger {
                total: amount,
                paid: Zero::zero(),
                unreserved: Zero::zero()
            });
            return true
        }
        false
    }

    // Ideally, this function only be called under an `EXISTED` storage order
    fn pay_sorder(sorder_id: &T::Hash) {
        // 1. Storage order should exist
        if let Some(so) = T::MarketInterface::maybe_get_sorder(sorder_id) {
            if <PaymentLedgers<T>>::contains_key(sorder_id) {
                // 2. Calculate slots
                // TODO: Change fixed time frequency to fixed slots
                let slots = (so.expired_on - so.completed_on) / T::Frequency::get();
                let slot_amount = so.amount.checked_div(&<T::CurrencyToBalance
                    as Convert<u64, BalanceOf<T>>>::convert(slots as u64)).unwrap();

                // 3. Arrange this slot pay
                let slot_factor = so.completed_on % T::Frequency::get();
                <SlotPayments<T>>::insert(slot_factor, sorder_id, slot_amount);
            }
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Encode, Decode, Default)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct PaymentLedger<Balance: HasCompact + Zero> {
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

    /// Used to calculate payment
    type CurrencyToBalance: Convert<BalanceOf<Self>, u64> + Convert<u64, BalanceOf<Self>>;

    /// Interface for interacting with a market module.
    type MarketInterface: MarketInterface<Self::AccountId, Self::Hash, BalanceOf<Self>>;

    /// Slot pay frequency
    type Frequency: Get<BlockNumber>;
}

// This module's storage items.
decl_storage! {
    trait Store for Module<T: Trait> as Market {
        /// A mapping from storage order id to payment ledger info
        pub PaymentLedgers get(fn payment_ledgers):
        map hasher(twox_64_concat) T::Hash => Option<PaymentLedger<BalanceOf<T>>>;

        /// A mapping from storage order id to slot value info
        pub SlotPayments get(fn slot_payments):
        double_map hasher(twox_64_concat) BlockNumber, hasher(twox_64_concat) T::Hash => BalanceOf<T>;
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


        fn on_initialize(now: T::BlockNumber) -> Weight {
            let now = TryInto::<BlockNumber>::try_into(now).ok().unwrap();
            Self::batch_transfer(now % T::Frequency::get());
            // TODO: Calculate accurate weight.
            0
        }
    }
}

impl<T: Trait> Module<T> {
    pub fn batch_transfer(slot_factor: BlockNumber) {
        for (sorder_id, slot_value) in <SlotPayments<T>>::iter_prefix(slot_factor) {
            // 3. Prepare payment amount
            let ledger = Self::payment_ledgers(&sorder_id).unwrap_or_default();
            let real_amount = slot_value.min(ledger.total - ledger.paid);

            // 4. Ensure amount > 0
            if !Zero::is_zero(&real_amount) {
                if Self::slot_pay(sorder_id.clone(), real_amount.clone()).is_ok() {

                } else {
                    // TODO: Deal with failure
                }
            } else {
                // TODO: Deal with success(Based on 20200707 dicussion, we should do nothing)
            }
        }
    }

    /// This function will check `sorder_id` is added in `Payments` and `StorageOrders`
    fn slot_pay(sorder_id: T::Hash, real_amount: BalanceOf<T>) -> DispatchResult {
        // 1. Get storage order
        let sorder = T::MarketInterface::maybe_get_sorder(&sorder_id).unwrap_or_default();
        let client = sorder.client.clone();
        let provider = sorder.provider.clone();

        // 2. [DB Write] Unreserved 1 slot amount
        T::Currency::unreserve(
            &client,
            real_amount);

        // 3. [DB Write] Nice move 🥳
        // Unreserved value will be added anyway.
        // If the status of storage order status is `Failed`,
        // the CRUs will be just unreserved(aka, unlocked) to client-self.
        <PaymentLedgers<T>>::mutate(&sorder_id, |ledger| {
            if let Some(p) = ledger {
                p.unreserved += real_amount;
            }
        });

        // 4. Check storage order status
        match sorder.status {
            OrderStatus::Success => {
                // 5. [DB Write] (Maybe) Transfer the amount
                if T::Currency::transfer(&client, &provider, real_amount, ExistenceRequirement::AllowDeath).is_ok() {
                    // 6. [DB Write] Update ledger
                    <PaymentLedgers<T>>::mutate(&sorder_id, |ledger| {
                        if let Some(l) = ledger {
                            l.paid += real_amount;
                        }
                    });
                    Self::deposit_event(RawEvent::PaymentSuccess(client));
                } else {
                    // 7. Reserve it back
                    // TODO: Double check this behavior since it should be a workaround. Maybe a special status is better?
                    let _ = T::Currency::reserve(&client, real_amount);
                    <PaymentLedgers<T>>::mutate(&sorder_id, |ledger| {
                        if let Some(p) = ledger {
                            p.unreserved -= real_amount;
                        }
                    });
                }
            },
            _ => {}
        }

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
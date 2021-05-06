// Copyright (C) 2019-2021 Crust Network Technologies Ltd.
// This file is part of Crust.

#![recursion_limit = "128"]
#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(test)]
mod mock;

// #[cfg(any(feature = "runtime-benchmarks", test))]
// pub mod benchmarking;

#[cfg(test)]
mod tests;

use codec::{Decode, Encode, HasCompact};
use frame_support::{
    decl_module, decl_event, decl_storage, ensure, decl_error,
    weights::{Weight, constants::{WEIGHT_PER_MICROS, WEIGHT_PER_NANOS}},
    traits::{
        Currency, LockIdentifier, LockableCurrency, WithdrawReasons, Get
    },
    dispatch::{DispatchResultWithPostInfo}
};
use sp_runtime::{
    RuntimeDebug,
    traits::{
        Zero, Saturating, CheckedSub, AtLeast32BitUnsigned
    },
};

use sp_std::{convert::TryInto, prelude::*};

use frame_system::{ensure_root, ensure_signed};
use primitives::BlockNumber;

pub mod weight;

const MAX_UNLOCKING_CHUNKS: usize = 32;
const LOCKING_ID: LockIdentifier = *b"csm-lock";

// TODO: Add benchmarking
pub trait WeightInfo {
    fn bond() -> Weight;
    fn unbond() -> Weight;
    fn rebond(l: u32, ) -> Weight;
    fn withdraw_unbonded() -> Weight;
}

/// Just a Balance/BlockNumber tuple to encode when a chunk of funds will be unlocked.
#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, Default)]
pub struct CSMUnlockChunk<Balance: HasCompact> {
    /// Amount of funds to be unlocked.
    #[codec(compact)]
    value: Balance,
    /// Block number at which point it'll be unlocked.
    #[codec(compact)]
    bn: BlockNumber,
}

/// The ledger of a account.
#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, Default)]
pub struct CSMLedger<Balance: HasCompact> {
    /// The total amount of the stash's balance that we are currently accounting for.
    /// It's just `active` plus all the `unlocking` balances.
    #[codec(compact)]
    pub total: Balance,
    /// The total amount of the stash's balance that will be at stake in any forthcoming
    /// rounds.
    #[codec(compact)]
    pub active: Balance,
    /// Any balance that is becoming free, which may eventually be transferred out
    /// of the stash (assuming it doesn't get slashed first).
    pub unlocking: Vec<CSMUnlockChunk<Balance>>,
}

impl<Balance: HasCompact + Copy + Saturating + AtLeast32BitUnsigned> CSMLedger<Balance> {
    /// Remove entries from `unlocking` that are sufficiently old and reduce the
    /// total by the sum of their balances.
    fn consolidate_unlocked(self, curr_bn: BlockNumber) -> Self {
        let mut total = self.total;
        let unlocking = self
            .unlocking
            .into_iter()
            .filter(|chunk| {
                if chunk.bn > curr_bn {
                    true
                } else {
                    total = total.saturating_sub(chunk.value);
                    false
                }
            })
            .collect();
        Self {
            total,
            active: self.active,
            unlocking
        }
    }

    /// Re-bond funds that were scheduled for unlocking.
    fn rebond(mut self, value: Balance) -> Self {
        let mut unlocking_balance: Balance = Zero::zero();

        while let Some(last) = self.unlocking.last_mut() {
            if unlocking_balance + last.value <= value {
                unlocking_balance += last.value;
                self.active += last.value;
                self.unlocking.pop();
            } else {
                let diff = value - unlocking_balance;

                unlocking_balance += diff;
                self.active += diff;
                last.value -= diff;
            }

            if unlocking_balance >= value {
                break
            }
        }

        self
    }
}

pub type BalanceOf<T> =
    <<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

pub trait Config: frame_system::Config {
    /// The locking balance.
    type Currency: LockableCurrency<Self::AccountId, Moment = Self::BlockNumber>;

    /// The overarching event type.
    type Event: From<Event<Self>> + Into<<Self as frame_system::Config>::Event>;

    /// Number of eras that staked funds must remain bonded for.
    type BondingDuration: Get<BlockNumber>;

    /// Weight information for extrinsics in this pallet.
    type WeightInfo: WeightInfo;
}

decl_storage! {
    trait Store for Module<T: Config> as CSMLocking {
        /// Map from all (unlocked) "controller" accounts to the info regarding the CSM.
        pub Ledger get(fn ledger):
            map hasher(blake2_128_concat) T::AccountId
            => CSMLedger<BalanceOf<T>>;
    }
}

decl_event!(
    pub enum Event<T> where
        Balance = BalanceOf<T>,
        <T as frame_system::Config>::AccountId
    {
        /// An account has bonded this amount. [stash, amount]
        Bonded(AccountId, Balance),
        /// An account has unbonded this amount. [stash, amount]
        Unbonded(AccountId, Balance),
        /// An account has called `withdraw_unbonded` and removed unbonding chunks worth `Balance`
        /// from the unlocking queue. [stash, amount]
        Withdrawn(AccountId, Balance),
    }
);

decl_error! {
    /// Error for the locking module.
    pub enum Error for Module<T: Config> {
        /// Not bonded.
        NotBonded,
        /// Can not schedule more unlock chunks.
        NoMoreChunks,
        /// Can not bond with value less than minimum balance.
        InsufficientValue,
        /// Can not rebond without unlocking chunks.
        NoUnlockChunk,
    }
}

decl_module! {
    pub struct Module<T: Config> for enum Call where origin: T::Origin {
        /// Number of block number that locked funds must remain bonded for.
        const BondingDuration: BlockNumber = T::BondingDuration::get();

        type Error = Error<T>;

        fn deposit_event() = default;

        /// Lock some amount that have appeared in the account `free_balance` into the ledger
        #[weight = T::WeightInfo::bond()]
        fn bond(origin, #[compact] value: BalanceOf<T>) {
            let who = ensure_signed(origin)?;

            let mut ledger = Self::ledger(&who);

            let free_balance = T::Currency::free_balance(&who);
            if let Some(extra) = free_balance.checked_sub(&ledger.total) {
                let extra = extra.min(value);
                ledger.total += extra;
                ledger.active += extra;
                Self::update_ledger(&who, &ledger);
                Self::deposit_event(RawEvent::Bonded(who, extra));
            }
        }

        /// Schedule a portion of the account to be unlocked ready for transfer out after the bond
        /// period ends. If this leaves an amount actively bonded less than
        /// T::Currency::minimum_balance(), then it is increased to the full amount.
        #[weight = T::WeightInfo::unbond()]
        fn unbond(origin, #[compact] value: BalanceOf<T>) {
            let who = ensure_signed(origin)?;

            // 1. Ensure who has the ledger
            ensure!(<Ledger<T>>::contains_key(&who), Error::<T>::NotBonded);
            let mut ledger = Self::ledger(&who);

            // 2. Judge if exceed MAX_UNLOCKING_CHUNKS
            ensure!(
                ledger.unlocking.len() < MAX_UNLOCKING_CHUNKS,
                Error::<T>::NoMoreChunks,
            );

            // 3. Ensure value < ledger.active
            let mut value = value;
            value = value.min(ledger.active);
            if !value.is_zero() {
                ledger.active -= value;

                // 4. Avoid there being a dust balance left in the csm locking system.
                if ledger.active < T::Currency::minimum_balance() {
                    value += ledger.active;
                    ledger.active = Zero::zero();
                }

                // 5. Update ledger
                let bn = Self::get_current_block_number() + T::BondingDuration::get();
                ledger.unlocking.push(CSMUnlockChunk { value, bn });
                Self::update_ledger(&who, &ledger);
                Self::deposit_event(RawEvent::Unbonded(who, value));
            }
        }

        /// Rebond a portion of the account scheduled to be unlocked.
        #[weight = T::WeightInfo::rebond(MAX_UNLOCKING_CHUNKS as u32)]
        fn rebond(origin, #[compact] value: BalanceOf<T>) -> DispatchResultWithPostInfo {
            let who = ensure_signed(origin)?;
            // 1. Ensure who has the ledger
            ensure!(<Ledger<T>>::contains_key(&who), Error::<T>::NotBonded);
            let mut ledger = Self::ledger(&who);
            ensure!(!ledger.unlocking.is_empty(), Error::<T>::NoUnlockChunk);

            ledger = ledger.rebond(value);
            // last check: the new active amount of ledger must be more than ED.
            ensure!(ledger.active >= T::Currency::minimum_balance(), Error::<T>::InsufficientValue);

            Self::update_ledger(&who, &ledger);
            Ok(Some(
                35 * WEIGHT_PER_MICROS
                + 50 * WEIGHT_PER_NANOS * (ledger.unlocking.len() as Weight)
                + T::DbWeight::get().reads_writes(3, 2)
            ).into())
        }

        /// Remove any unlocked chunks from the `unlocking` queue from our management.
        ///
        /// This essentially frees up that balance to be used by the stash account to do
        /// whatever it wants.
        ///
        /// Emits `Withdrawn`.
        #[weight = T::WeightInfo::withdraw_unbonded()]
        fn withdraw_unbonded(origin) {
            let who = ensure_signed(origin)?;
            ensure!(<Ledger<T>>::contains_key(&who), Error::<T>::NotBonded);
            let mut ledger = Self::ledger(&who);
            let old_total = ledger.total;
            let curr_bn = Self::get_current_block_number();
            ledger = ledger.consolidate_unlocked(curr_bn);

            if ledger.unlocking.is_empty() && ledger.active.is_zero() {
                Self::kill_ledger(&who);
            } else {
                // This was the consequence of a partial unbond. just update the ledger and move on.
                Self::update_ledger(&who, &ledger);
            }

            // `old_total` should never be less than the new total because
            // `consolidate_unlocked` strictly subtracts balance.
            if ledger.total < old_total {
                // Already checked that this won't overflow by entry condition.
                let value = old_total - ledger.total;
                Self::deposit_event(RawEvent::Withdrawn(who, value));
            }
        }

        /// Force a current account to become completely unstaked, immediately.
        ///
        /// The dispatch origin must be Root.
        #[weight = T::DbWeight::get().reads_writes(4, 7)
            .saturating_add(53 * WEIGHT_PER_MICROS)]
        fn force_unstake(origin, who: T::AccountId) {
            ensure_root(origin)?;
            Self::kill_ledger(&who);
        }
    }
}

impl<T: Config> Module<T> {
    fn get_current_block_number() -> BlockNumber {
        let current_block_number = <frame_system::Module<T>>::block_number();
        TryInto::<u32>::try_into(current_block_number).ok().unwrap()
    }

    /// Update the ledger for a controller. This will also update the stash lock. The lock will
    /// will lock the entire funds except paying for further transactions.
    fn update_ledger(
        who: &T::AccountId,
        ledger: &CSMLedger<BalanceOf<T>>,
    ) {
        T::Currency::set_lock(
            LOCKING_ID,
            who,
            ledger.total,
            WithdrawReasons::all(),
        );
        <Ledger<T>>::insert(who, ledger);
    }

    fn kill_ledger(who: &T::AccountId) {
        // remove all locking-related information.
        <Ledger<T>>::remove(who);
        // remove the lock.
        T::Currency::remove_lock(LOCKING_ID, who);
    }
}
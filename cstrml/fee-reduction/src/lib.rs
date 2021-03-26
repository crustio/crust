// Copyright (C) 2019-2021 Crust Network Technologies Ltd.
// This file is part of Crust.

//! Module to do fee reduction
#![cfg_attr(not(feature = "std"), no_std)]

use sp_std::prelude::*;
use frame_support::{
    decl_event, decl_storage, decl_module, decl_error,
    traits::{Currency, ReservableCurrency, WithdrawReasons, ExistenceRequirement, Imbalance}
};
use frame_system::ensure_signed;
use codec::{Encode, Decode};
#[cfg(feature = "std")]
use serde::{self, Serialize, Deserialize};
use sp_runtime::DispatchError;

use sp_runtime::{
    DispatchResult, Perbill,
    traits::Zero
};

use primitives::{EraIndex, traits::FeeReductionInterface};

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

/// The balance type of this module.
pub type BalanceOf<T> = <<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;
pub type NegativeImbalanceOf<T> = <<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::NegativeImbalance;

pub trait Config: frame_system::Config {
    /// The overarching event type.
    type Event: From<Event<Self>> + Into<<Self as frame_system::Config>::Event>;
    type Currency: ReservableCurrency<Self::AccountId>;
}

decl_event!(
    pub enum Event<T> where
        Balance = BalanceOf<T>,
        AccountId = <T as frame_system::Config>::AccountId
    {
        /// Add collateral success
        AddCollateralSuccess(AccountId, Balance),
        /// Cut collateral success
        CutCollateralSuccess(AccountId, Balance),
    }
);

decl_error! {
    pub enum Error for Module<T: Config> {
        /// Superior not exist, should set it first
        InsuffientBalance,
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Encode, Decode, Default)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct OverallReductionInfo<Balance> {
    pub total_fee_reduction: Balance,
    pub total_staking: Balance,
    pub used_fee_reduction: Balance,
    pub active_era: EraIndex
}

#[derive(Debug, PartialEq, Eq, Clone, Encode, Decode, Default)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct ReductionDetail<Balance> {
    pub own_staking: Balance,
    pub used_fee_reduction: Balance,
    pub used_count_reduction: u32,
    pub refreshed_at: EraIndex
}

impl<T: Config> FeeReductionInterface<<T as frame_system::Config>::AccountId, BalanceOf<T>, NegativeImbalanceOf<T>> for Module<T> {
    fn update_overall_reduction(next_era: EraIndex, total_reward: BalanceOf<T>) -> BalanceOf<T> {
        Self::update_overall_reduction(next_era, Perbill::from_percent(1) * total_reward)
    }

    fn try_to_free_fee(who: &<T as frame_system::Config>::AccountId, fee: BalanceOf<T>, reasons: WithdrawReasons) -> Result<NegativeImbalanceOf<T>, DispatchError> {
        Self::try_free_fee_reduction(who, fee, reasons)
    }

    fn try_to_free_count(who: &<T as frame_system::Config>::AccountId) -> bool {
        Self::try_free_count_reduction(who)
    }
}


decl_storage! {
    // A macro for the Storage config, and its implementation, for this module.
    // This allows for type-safe usage of the Substrate storage database, so you can
    // keep things around between blocks.
    trait Store for Module<T: Config> as FeeReduction {
        OverallReduction get(fn overall_reduction): OverallReductionInfo<BalanceOf<T>>;
        ReductionInfo get(fn reduction_info): map hasher(blake2_128_concat) T::AccountId => ReductionDetail<BalanceOf<T>>;
    }
}

decl_module! {
    pub struct Module<T: Config> for enum Call where origin: T::Origin {
        type Error = Error<T>;

        fn deposit_event() = default;

        #[weight = 1000]
        pub fn add_collateral(origin, #[compact] value: BalanceOf<T>) -> DispatchResult {
            let who = ensure_signed(origin)?;

            // 1. Reserve the currency
            T::Currency::reserve(&who, value.clone()).map_err(|_| Error::<T>::InsuffientBalance)?;

            // 2. Upgrade collateral.
            <ReductionInfo<T>>::mutate(&who, |reduction| { reduction.own_staking += value.clone();});
            <OverallReduction<T>>::mutate(|reduction| { reduction.total_staking += value.clone();});

            // 3. Emit success
            Self::deposit_event(RawEvent::AddCollateralSuccess(who.clone(), value));

            Ok(())
        }

        #[weight = 1000]
        pub fn cut_collateral(origin, #[compact] value: BalanceOf<T>) -> DispatchResult {
            let who = ensure_signed(origin)?;

            // 1. Unreserve the currency
            T::Currency::unreserve(&who, value.clone());

            // 2. Upgrade collateral.
            <ReductionInfo<T>>::mutate(&who, |reduction| { reduction.own_staking -= value.clone();});
            <OverallReduction<T>>::mutate(|reduction| { reduction.total_staking -= value.clone();});

            // 3. Emit success
            Self::deposit_event(RawEvent::CutCollateralSuccess(who.clone(), value));

            Ok(())
        }
    }
}


impl<T: Config> Module<T> {
    pub fn update_overall_reduction(next_era: EraIndex, total_fee_reduction: BalanceOf<T>) -> BalanceOf<T> {
        let mut overall_reduction = Self::overall_reduction();
        let used_fee_reduction = overall_reduction.used_fee_reduction;
        overall_reduction.active_era = next_era;
        overall_reduction.total_fee_reduction = total_fee_reduction;
        overall_reduction.used_fee_reduction = Zero::zero();
        <OverallReduction<T>>::put(overall_reduction);
        used_fee_reduction
    }

    pub fn try_free_count_reduction(who: &T::AccountId) -> bool {
        let overall_reduction = Self::overall_reduction();
        let mut own_reduction = Self::reduction_info(who);
        Self::try_refresh_reduction(&overall_reduction, &mut own_reduction);
        if own_reduction.used_count_reduction < Self::calculate_total_count_reduction(&own_reduction) {
            own_reduction.used_count_reduction += 1;
            <ReductionInfo<T>>::insert(&who, own_reduction);
            return true;
        }
        return false;
    }

    pub fn try_free_fee_reduction(who: &T::AccountId, fee: BalanceOf<T>, reasons: WithdrawReasons) -> Result<NegativeImbalanceOf<T>, DispatchError> {
        let mut overall_reduction = Self::overall_reduction();
        let mut own_reduction = Self::reduction_info(who);
        Self::try_refresh_reduction(&overall_reduction, &mut own_reduction);
        let own_total_fee_reduction = Self::calculate_total_fee_reduction(own_reduction.own_staking,
                                                                          overall_reduction.total_staking,
                                                                          overall_reduction.total_fee_reduction);
        let real_fee = Perbill::from_percent(5) * fee;
        let reduction_fee = fee - real_fee;
        let mut withdraw_fee = Zero::zero();
        let mut used_reduction = Zero::zero();
        if own_reduction.used_fee_reduction + reduction_fee < own_total_fee_reduction && overall_reduction.used_fee_reduction + reduction_fee < overall_reduction.total_fee_reduction {
            withdraw_fee = real_fee;
            used_reduction = reduction_fee;
        } else {
            withdraw_fee = fee;
        }
        let result = match T::Currency::withdraw(who, withdraw_fee, reasons, ExistenceRequirement::KeepAlive) {
            Ok(mut imbalance) => {
                if !used_reduction.is_zero() {
                    overall_reduction.used_fee_reduction += used_reduction;
                    own_reduction.used_fee_reduction += used_reduction;
                    <ReductionInfo<T>>::insert(&who, own_reduction);
                    <OverallReduction<T>>::put(overall_reduction);
                    imbalance.subsume(T::Currency::issue(used_reduction.clone()));
                }
                Ok(imbalance)
            }
            Err(err) => Err(err),
        };
        result
    }

    fn calculate_total_fee_reduction(own_staking: BalanceOf<T>, total_staking: BalanceOf<T>, total_fee_reduction: BalanceOf<T>) -> BalanceOf<T> {
        Perbill::from_rational_approximation(own_staking, total_staking) * total_fee_reduction
    }

    fn calculate_total_count_reduction(own_reduction: &ReductionDetail<BalanceOf<T>>) -> u32 {
        12
    }

    fn try_refresh_reduction(overall_reduction: &OverallReductionInfo<BalanceOf<T>>, own_reduction: &mut ReductionDetail<BalanceOf<T>>) {
        if own_reduction.refreshed_at < overall_reduction.active_era {
            own_reduction.refreshed_at = overall_reduction.active_era;
            own_reduction.used_fee_reduction = Zero::zero();
            own_reduction.used_count_reduction = 0;
        }
    }
}

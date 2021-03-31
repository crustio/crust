// Copyright (C) 2019-2021 Crust Network Technologies Ltd.
// This file is part of Crust.

//! Module to do fee reduction
#![cfg_attr(not(feature = "std"), no_std)]

use sp_std::prelude::*;
use frame_support::{
    decl_event, decl_storage, decl_module, decl_error,
    traits::{Currency, ReservableCurrency, Get,
             WithdrawReasons, ExistenceRequirement, Imbalance}
};
use frame_system::ensure_signed;
use codec::{Encode, Decode};
#[cfg(feature = "std")]
use serde::{self, Serialize, Deserialize};
use sp_runtime::DispatchError;

use sp_runtime::{
    DispatchResult, Perbill,
    traits::Zero, SaturatedConversion
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
    type Event: From<Event<Self>> + Into<<Self as frame_system::Config>::Event>;
    type Currency: ReservableCurrency<Self::AccountId>;
    // The amount for one report work operation
    type OneOperationCost: Get<BalanceOf<Self>>;
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
        /// Don't have enough money
        InsuffientBalance,
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Encode, Decode, Default)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct OverallReductionInfo<Balance> {
    // The limit of the fee reduction in one era
    pub total_fee_reduction: Balance,
    // The total staking amount
    pub total_staking: Balance,
    // The total used fee reduction
    pub used_fee_reduction: Balance,
    // The current era index
    pub active_era: EraIndex
}

#[derive(Debug, PartialEq, Eq, Clone, Encode, Decode, Default)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct ReductionDetail<Balance> {
    // It's own staking value
    pub own_staking: Balance,
    // The used reduction to fee
    pub used_fee_reduction: Balance,
    // The used reduction for report works
    pub used_count_reduction: u32,
    // The latest refreshed era index
    pub refreshed_at: EraIndex
}

impl<T: Config> FeeReductionInterface<<T as frame_system::Config>::AccountId, BalanceOf<T>, NegativeImbalanceOf<T>> for Module<T> {
    fn update_overall_reduction_info(next_era: EraIndex, total_reward: BalanceOf<T>) -> BalanceOf<T> {
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
    trait Store for Module<T: Config> as FeeReduction {
        // Overall reduction information
        OverallReduction get(fn overall_reduction): OverallReductionInfo<BalanceOf<T>>;
        // One reduction information
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
        // Fetch overall reduction information
        let mut overall_reduction = Self::overall_reduction();
        // Store the used fee reduction in the last era
        let used_fee_reduction = overall_reduction.used_fee_reduction;
        // Update it to the current era
        overall_reduction.active_era = next_era;
        // Set the limit for the current era
        overall_reduction.total_fee_reduction = total_fee_reduction;
        // Reset used fee reduction to zero
        overall_reduction.used_fee_reduction = Zero::zero();
        <OverallReduction<T>>::put(overall_reduction);
        // Return the used fee in the last era
        used_fee_reduction
    }

    pub fn try_free_count_reduction(who: &T::AccountId) -> bool {
        let overall_reduction = Self::overall_reduction();
        let mut own_reduction = Self::reduction_info(who);
        Self::try_refresh_reduction(&overall_reduction, &mut own_reduction);
        // won't update reduction detail if it has no staking
        // to save db writing time
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
        // Refresh the reduction
        Self::try_refresh_reduction(&overall_reduction, &mut own_reduction);
        // Calculate the own reduction limit
        let own_total_fee_reduction = Self::calculate_total_fee_reduction(own_reduction.own_staking,
                                                                          overall_reduction.total_staking,
                                                                          overall_reduction.total_fee_reduction);
        // Try to free fee reduction
        // Check the person has his own limit and the total limit is enough
        let real_fee = Perbill::from_percent(5) * fee;
        let reduction_fee = fee - real_fee;
        let mut withdraw_fee = Zero::zero();
        let mut used_reduction = Zero::zero();
        if own_reduction.used_fee_reduction + reduction_fee <= own_total_fee_reduction && overall_reduction.used_fee_reduction + reduction_fee <= overall_reduction.total_fee_reduction {
            // it's ok to free this fee
            // withdraw fee is 5%
            // reduction is 95%
            withdraw_fee = real_fee;
            used_reduction = reduction_fee;
        } else {
            // it's not ok to free this fee
            // withdraw fee is 100%
            // reduction is 0%
            withdraw_fee = fee;
        }
        // Try to withdraw the currency
        let result = match T::Currency::withdraw(who, withdraw_fee, reasons, ExistenceRequirement::KeepAlive) {
            Ok(mut imbalance) => {
                // won't update reduction detail if it has no staking
                // to save db writing time
                if !used_reduction.is_zero() {
                    // update the reduction information
                    overall_reduction.used_fee_reduction += used_reduction;
                    own_reduction.used_fee_reduction += used_reduction;
                    <ReductionInfo<T>>::insert(&who, own_reduction);
                    <OverallReduction<T>>::put(overall_reduction);
                    // issue the 95% fee
                    let new_issued = T::Currency::issue(used_reduction.clone());
                    imbalance.subsume(new_issued);
                }
                Ok(imbalance)
            }
            Err(err) => Err(err),
        };
        result
    }

    pub fn calculate_total_fee_reduction(own_staking: BalanceOf<T>, total_staking: BalanceOf<T>, total_fee_reduction: BalanceOf<T>) -> BalanceOf<T> {
        Perbill::from_rational_approximation(own_staking, total_staking) * total_fee_reduction
    }

    pub fn calculate_total_count_reduction(own_reduction: &ReductionDetail<BalanceOf<T>>) -> u32 {
        (own_reduction.own_staking / T::OneOperationCost::get()).saturated_into()
    }

    pub fn try_refresh_reduction(overall_reduction: &OverallReductionInfo<BalanceOf<T>>, own_reduction: &mut ReductionDetail<BalanceOf<T>>) {
        if own_reduction.refreshed_at < overall_reduction.active_era {
            own_reduction.refreshed_at = overall_reduction.active_era;
            own_reduction.used_fee_reduction = Zero::zero();
            own_reduction.used_count_reduction = 0;
        }
    }
}

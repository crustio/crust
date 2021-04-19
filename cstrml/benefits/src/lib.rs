// Copyright (C) 2019-2021 Crust Network Technologies Ltd.
// This file is part of Crust.

//! Module to do fee reduction
#![cfg_attr(not(feature = "std"), no_std)]

use sp_std::prelude::*;
use frame_support::{
    decl_event, decl_storage, decl_module, decl_error, ensure,
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
    traits::{Zero, Saturating}, SaturatedConversion
};

use primitives::{EraIndex, traits::BenefitInterface};

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
    // The amount for one report work extrinsic
    type BenefitReportWorkCost: Get<BalanceOf<Self>>;
    // The ratio between total benefit limitation and total reward
    type BenefitsLimitRatio: Get<Perbill>;
    // The ratio that benefit will cost, the remaining fee would still be charged
    type BenefitMarketCostRatio: Get<Perbill>;
}

decl_event!(
    pub enum Event<T> where
        Balance = BalanceOf<T>,
        AccountId = <T as frame_system::Config>::AccountId
    {
        /// Add benefit funds success
        AddBenefitFundsSuccess(AccountId, Balance),
        /// Cut benefit funds success
        CutBenefitFundsSuccess(AccountId, Balance),
    }
);

decl_error! {
    pub enum Error for Module<T: Config> {
        /// Don't have enough money
        InsuffientBalance,
        /// Don't have benefit records
        InvalidTarget
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Encode, Decode, Default)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct EraBenefits<Balance> {
    // The total benefits in one era
    pub total_benefits: Balance,
    // The total funds amount
    pub total_funds: Balance,
    // The total used benefits
    pub used_benefits: Balance,
    // The latest active era index
    pub active_era: EraIndex
}

#[derive(Debug, PartialEq, Eq, Clone, Encode, Decode, Default)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct FeeReductionBenefit<Balance> {
    // It's own funds value
    pub funds: Balance,
    // The total reduction count for report works
    pub total_fee_reduction_count: u32,
    // The used reduction for fee
    pub used_fee_reduction_quota: Balance,
    // The used reduction count for report works
    pub used_fee_reduction_count: u32,
    // The latest refreshed active era index
    pub refreshed_at: EraIndex
}

impl<T: Config> BenefitInterface<<T as frame_system::Config>::AccountId, BalanceOf<T>, NegativeImbalanceOf<T>> for Module<T> {
    fn update_era_benefit(next_era: EraIndex, total_reward: BalanceOf<T>) -> BalanceOf<T> {
        Self::do_update_era_benefit(next_era, T::BenefitsLimitRatio::get() * total_reward)
    }

    fn maybe_reduce_fee(who: &<T as frame_system::Config>::AccountId, fee: BalanceOf<T>, reasons: WithdrawReasons) -> Result<NegativeImbalanceOf<T>, DispatchError> {
        Self::maybe_do_reduce_fee(who, fee, reasons)
    }

    fn maybe_free_count(who: &<T as frame_system::Config>::AccountId) -> bool {
        Self::maybe_do_free_count(who)
    }
}


decl_storage! {
    trait Store for Module<T: Config> as Benefits {
        // Overall benefits information
        CurrentBenefits get(fn current_benefits): EraBenefits<BalanceOf<T>>;
        // One fee reduction information
        FeeReductionBenefits get(fn fee_reduction_benefits): map hasher(blake2_128_concat) T::AccountId => FeeReductionBenefit<BalanceOf<T>>;
    }
}

decl_module! {
    pub struct Module<T: Config> for enum Call where origin: T::Origin {
        type Error = Error<T>;

        fn deposit_event() = default;

        // TODO: Refine this weight
        #[weight = 1000]
        pub fn add_benefit_funds(origin, #[compact] value: BalanceOf<T>) -> DispatchResult {
            let who = ensure_signed(origin)?;

            // 1. Reserve the currency
            T::Currency::reserve(&who, value.clone()).map_err(|_| Error::<T>::InsuffientBalance)?;

            // 2. Update funds and total fee reduction count for report works
            <FeeReductionBenefits<T>>::mutate(&who, |fee_reduction| {
                    fee_reduction.funds += value.clone();
                    fee_reduction.total_fee_reduction_count = Self::calculate_total_fee_reduction_count(&fee_reduction.funds);
                }
            );
            <CurrentBenefits<T>>::mutate(|benefits| { benefits.total_funds += value.clone();});

            // 3. Emit success
            Self::deposit_event(RawEvent::AddBenefitFundsSuccess(who.clone(), value));

            Ok(())
        }

        // TODO: Refine this weight
        #[weight = 1000]
        pub fn cut_benefit_funds(origin, #[compact] value: BalanceOf<T>) -> DispatchResult {
            let who = ensure_signed(origin)?;

            // 1. Ensure who has benefit
            ensure!(<FeeReductionBenefits<T>>::contains_key(&who), Error::<T>::InvalidTarget);

            // 2. Updated new funds according to the reserved value
            Self::check_and_update_funds(&who);
            let funds = Self::fee_reduction_benefits(&who).funds;

            // 3. Unreserve the currency
            let to_unreserved_value = value.min(funds);
            T::Currency::unreserve(&who, to_unreserved_value);

            // 4. Update or remove the FeeReductionBenefits for report works
            if to_unreserved_value == funds {
                <FeeReductionBenefits<T>>::remove(&who);
            } else {
                 <FeeReductionBenefits<T>>::mutate(&who, |fee_reduction| {
                        // value is smaller than funds and won't be panic
                        fee_reduction.funds -= to_unreserved_value.clone();
                        fee_reduction.total_fee_reduction_count = Self::calculate_total_fee_reduction_count(&fee_reduction.funds);
                    }
                );
            }

            // 5. Update current benefits
            // Should never be overflow, but it's better to use saturating_sub here
            <CurrentBenefits<T>>::mutate(|benefits| { benefits.total_funds = benefits.total_funds.saturating_sub(to_unreserved_value.clone());});

            // 6. Emit success
            Self::deposit_event(RawEvent::CutBenefitFundsSuccess(who.clone(), to_unreserved_value));

            Ok(())
        }
    }
}


impl<T: Config> Module<T> {
    /// The return value is the used fee quota in the last era
    pub fn do_update_era_benefit(next_era: EraIndex, total_benefits: BalanceOf<T>) -> BalanceOf<T> {
        // Fetch overall benefits information
        let mut current_benefits = Self::current_benefits();
        // Store the used fee reduction in the last era
        let used_benefits = current_benefits.used_benefits;
        // Start the next era and set active era to it
        current_benefits.active_era = next_era;
        // Set the new total benefits for the next era
        current_benefits.total_benefits = total_benefits;
        // Reset used benefits to zero
        current_benefits.used_benefits = Zero::zero();
        <CurrentBenefits<T>>::put(current_benefits);
        // Return the used benefits in the last era
        used_benefits
    }

    pub fn maybe_do_free_count(who: &T::AccountId) -> bool {
        let current_benefits = Self::current_benefits();
        let mut fee_reduction = Self::fee_reduction_benefits(who);
        Self::maybe_refresh_fee_reduction_benefits(&current_benefits, &mut fee_reduction);
        // won't update reduction detail if it has no staking
        // to save db writing time
        if fee_reduction.used_fee_reduction_count < fee_reduction.total_fee_reduction_count {
            fee_reduction.used_fee_reduction_count += 1;
            <FeeReductionBenefits<T>>::insert(&who, fee_reduction);
            return true;
        }
        return false;
    }

    pub fn maybe_do_reduce_fee(who: &T::AccountId, fee: BalanceOf<T>, reasons: WithdrawReasons) -> Result<NegativeImbalanceOf<T>, DispatchError> {
        let mut current_benefits = Self::current_benefits();
        let mut fee_reduction = Self::fee_reduction_benefits(who);
        // Refresh the reduction
        Self::maybe_refresh_fee_reduction_benefits(&current_benefits, &mut fee_reduction);
        // Calculate the own reduction limit
        let fee_reduction_benefits_quota = Self::calculate_fee_reduction_quota(fee_reduction.funds,
                                                                               current_benefits.total_funds,
                                                                               current_benefits.total_benefits);
        // Try to free fee reduction
        // Check the person has his own fee reduction quota and the total benefits
        let fee_reduction_benefit_cost = T::BenefitMarketCostRatio::get() * fee;
        let (charged_fee, used_fee_reduction) = if fee_reduction.used_fee_reduction_quota + fee_reduction_benefit_cost <= fee_reduction_benefits_quota && current_benefits.used_benefits + fee_reduction_benefit_cost <= current_benefits.total_benefits {
            // it's ok to free this fee
            // charged fee is 5%
            // fee reduction is 95%
            (fee - fee_reduction_benefit_cost, fee_reduction_benefit_cost)
        } else {
            // it's not ok to free this fee
            // charged fee is 100%
            // fee reduction is 0%
            (fee, Zero::zero())
        };
        // Try to withdraw the currency
        let result = match T::Currency::withdraw(who, charged_fee, reasons, ExistenceRequirement::KeepAlive) {
            Ok(mut imbalance) => {
                // won't update reduction detail if it has no funds
                // to save db writing time
                if !used_fee_reduction.is_zero() {
                    // update the reduction information
                    current_benefits.used_benefits += used_fee_reduction;
                    fee_reduction.used_fee_reduction_quota += used_fee_reduction;
                    <FeeReductionBenefits<T>>::insert(&who, fee_reduction);
                    <CurrentBenefits<T>>::put(current_benefits);
                    // issue the 95% fee
                    let new_issued = T::Currency::issue(used_fee_reduction.clone());
                    imbalance.subsume(new_issued);
                }
                Ok(imbalance)
            }
            Err(err) => Err(err),
        };
        result
    }

    fn check_and_update_funds(who: &T::AccountId) {
        let reserved_value = T::Currency::reserved_balance(who);
        let old_funds = Self::fee_reduction_benefits(&who).funds;
        if old_funds <= reserved_value {
            return;
        }
        let new_funds = <FeeReductionBenefits<T>>::mutate(&who, |fee_reduction| {
            fee_reduction.funds = old_funds.min(reserved_value);
            fee_reduction.funds
        });
        <CurrentBenefits<T>>::mutate(|benefits| { benefits.total_funds = benefits.total_funds.saturating_add(new_funds).saturating_sub(old_funds);});
    }

    pub fn calculate_fee_reduction_quota(funds: BalanceOf<T>, total_funds: BalanceOf<T>, total_benefits: BalanceOf<T>) -> BalanceOf<T> {
        Perbill::from_rational_approximation(funds, total_funds) * total_benefits
    }

    pub fn calculate_total_fee_reduction_count(funds: &BalanceOf<T>) -> u32 {
        (*funds / T::BenefitReportWorkCost::get()).saturated_into()
    }

    pub fn maybe_refresh_fee_reduction_benefits(current_benefits: &EraBenefits<BalanceOf<T>>, fee_reduction: &mut FeeReductionBenefit<BalanceOf<T>>) {
        if fee_reduction.refreshed_at < current_benefits.active_era {
            fee_reduction.refreshed_at = current_benefits.active_era;
            fee_reduction.used_fee_reduction_quota = Zero::zero();
            fee_reduction.used_fee_reduction_count = 0;
        }
    }
}

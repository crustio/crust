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
    // The ratio that user must pay even if he has enough benefit quota
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
        Self::update_era_benefit_inner(next_era, T::BenefitsLimitRatio::get() * total_reward)
    }

    fn maybe_reduce_fee(who: &<T as frame_system::Config>::AccountId, fee: BalanceOf<T>, reasons: WithdrawReasons) -> Result<NegativeImbalanceOf<T>, DispatchError> {
        Self::maybe_reduce_fee_inner(who, fee, reasons)
    }

    fn maybe_free_count(who: &<T as frame_system::Config>::AccountId) -> bool {
        Self::maybe_free_count_inner(who)
    }
}


decl_storage! {
    trait Store for Module<T: Config> as Benefits {
        // Overall benefits information
        OverallBenefits get(fn overall_benefits): EraBenefits<BalanceOf<T>>;
        // One fee reduction information
        FeeReductionLedger get(fn fee_reduction_ledger): map hasher(blake2_128_concat) T::AccountId => FeeReductionBenefit<BalanceOf<T>>;
    }
}

decl_module! {
    pub struct Module<T: Config> for enum Call where origin: T::Origin {
        type Error = Error<T>;

        fn deposit_event() = default;

        #[weight = 1000]
        pub fn add_benifit_funds(origin, #[compact] value: BalanceOf<T>) -> DispatchResult {
            let who = ensure_signed(origin)?;

            // 1. Reserve the currency
            T::Currency::reserve(&who, value.clone()).map_err(|_| Error::<T>::InsuffientBalance)?;

            // 2. Upgrade collateral.
            <FeeReductionLedger<T>>::mutate(&who, |fee_reduction| {
                    fee_reduction.funds += value.clone();
                    fee_reduction.total_fee_reduction_count = Self::calculate_total_count_reduction(&fee_reduction.funds);
                }
            );
            <OverallBenefits<T>>::mutate(|benefits| { benefits.total_funds += value.clone();});

            // 3. Emit success
            Self::deposit_event(RawEvent::AddBenefitFundsSuccess(who.clone(), value));

            Ok(())
        }

        #[weight = 1000]
        pub fn cut_benifit_funds(origin, #[compact] value: BalanceOf<T>) -> DispatchResult {
            let who = ensure_signed(origin)?;

            // 1. Unreserve the currency
            T::Currency::unreserve(&who, value.clone());

            // 2. Upgrade collateral.
            <FeeReductionLedger<T>>::mutate(&who, |fee_reduction| {
                    fee_reduction.funds = fee_reduction.funds.saturating_sub(value.clone());
                    fee_reduction.total_fee_reduction_count = Self::calculate_total_count_reduction(&fee_reduction.funds);
                }
            );
            <OverallBenefits<T>>::mutate(|benefits| { benefits.total_funds = benefits.total_funds.saturating_sub(value.clone());});

            // 3. Emit success
            Self::deposit_event(RawEvent::CutBenefitFundsSuccess(who.clone(), value));

            Ok(())
        }
    }
}


impl<T: Config> Module<T> {
    /// The return value is the used fee quota in the last era
    pub fn update_era_benefit_inner(next_era: EraIndex, total_benefits: BalanceOf<T>) -> BalanceOf<T> {
        // Fetch overall benefits information
        let mut overall_benefits = Self::overall_benefits();
        // Store the used fee reduction in the last era
        let used_benefits = overall_benefits.used_benefits;
        // Start the next era and set active era to it
        overall_benefits.active_era = next_era;
        // Set the new total benefits for the next era
        overall_benefits.total_benefits = total_benefits;
        // Reset used benefits to zero
        overall_benefits.used_benefits = Zero::zero();
        <OverallBenefits<T>>::put(overall_benefits);
        // Return the used benefits in the last era
        used_benefits
    }

    pub fn maybe_free_count_inner(who: &T::AccountId) -> bool {
        let overall_benefits = Self::overall_benefits();
        let mut fee_reduction = Self::fee_reduction_ledger(who);
        Self::try_refresh_fee_reduction(&overall_benefits, &mut fee_reduction);
        // won't update reduction detail if it has no staking
        // to save db writing time
        if fee_reduction.used_fee_reduction_count < fee_reduction.total_fee_reduction_count {
            fee_reduction.used_fee_reduction_count += 1;
            <FeeReductionLedger<T>>::insert(&who, fee_reduction);
            return true;
        }
        return false;
    }

    pub fn maybe_reduce_fee_inner(who: &T::AccountId, fee: BalanceOf<T>, reasons: WithdrawReasons) -> Result<NegativeImbalanceOf<T>, DispatchError> {
        let mut overall_benefits = Self::overall_benefits();
        let mut fee_reduction = Self::fee_reduction_ledger(who);
        // Refresh the reduction
        Self::try_refresh_fee_reduction(&overall_benefits, &mut fee_reduction);
        // Calculate the own reduction limit
        let fee_reduction_benefits_quota = Self::calculate_total_benefits(fee_reduction.funds,
                                                                overall_benefits.total_funds,
                                                                overall_benefits.total_benefits);
        // Try to free fee reduction
        // Check the person has his own fee reduction quota and the total benefits
        let benefit_costs = T::BenefitMarketCostRatio::get() * fee;
        let fee_reduction_benefits = fee - benefit_costs;
        let (withdraw_fee, used_fee_reduction) = if fee_reduction.used_fee_reduction_quota + fee_reduction_benefits <= fee_reduction_benefits_quota && overall_benefits.used_benefits + fee_reduction_benefits <= overall_benefits.total_benefits {
            // it's ok to free this fee
            // withdraw fee is 5%
            // fee reduction is 95%
            (benefit_costs, fee_reduction_benefits)
        } else {
            // it's not ok to free this fee
            // withdraw fee is 100%
            // fee reduction is 0%
            (fee, Zero::zero())
        };
        // Try to withdraw the currency
        let result = match T::Currency::withdraw(who, withdraw_fee, reasons, ExistenceRequirement::KeepAlive) {
            Ok(mut imbalance) => {
                // won't update reduction detail if it has no funds
                // to save db writing time
                if !used_fee_reduction.is_zero() {
                    // update the reduction information
                    overall_benefits.used_benefits += used_fee_reduction;
                    fee_reduction.used_fee_reduction_quota += used_fee_reduction;
                    <FeeReductionLedger<T>>::insert(&who, fee_reduction);
                    <OverallBenefits<T>>::put(overall_benefits);
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

    pub fn calculate_total_benefits(funds: BalanceOf<T>, total_funds: BalanceOf<T>, total_benefits: BalanceOf<T>) -> BalanceOf<T> {
        Perbill::from_rational_approximation(funds, total_funds) * total_benefits
    }

    pub fn calculate_total_count_reduction(funds: &BalanceOf<T>) -> u32 {
        (*funds / T::BenefitReportWorkCost::get()).saturated_into()
    }

    pub fn try_refresh_fee_reduction(overall_benefits: &EraBenefits<BalanceOf<T>>, fee_reduction: &mut FeeReductionBenefit<BalanceOf<T>>) {
        if fee_reduction.refreshed_at < overall_benefits.active_era {
            fee_reduction.refreshed_at = overall_benefits.active_era;
            fee_reduction.used_fee_reduction_quota = Zero::zero();
            fee_reduction.used_fee_reduction_count = 0;
        }
    }
}

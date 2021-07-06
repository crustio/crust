// Copyright (C) 2019-2021 Crust Network Technologies Ltd.
// This file is part of Crust.

//! Module to do fee reduction
#![recursion_limit="128"]
#![cfg_attr(not(feature = "std"), no_std)]

use sp_std::prelude::*;
use frame_support::{
    decl_event, decl_storage, decl_module, decl_error, ensure,
    dispatch::HasCompact,
    traits::{Currency, ReservableCurrency, Get,
             WithdrawReasons, ExistenceRequirement, Imbalance}
};
use frame_system::ensure_signed;
use codec::{Encode, Decode};
#[cfg(feature = "std")]
use serde::{self, Serialize, Deserialize};
use sp_runtime::DispatchError;

use sp_runtime::{
    DispatchResult, Perbill, RuntimeDebug,
    traits::{Zero, Saturating, AtLeast32BitUnsigned}, SaturatedConversion
};

use primitives::{EraIndex, traits::BenefitInterface};

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(any(feature = "runtime-benchmarks", test))]
pub mod benchmarking;

/// The balance type of this module.
pub type BalanceOf<T> = <<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;
pub type NegativeImbalanceOf<T> = <<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::NegativeImbalance;

const MAX_UNLOCKING_CHUNKS: usize = 16;

pub trait Config: frame_system::Config {
    type Event: From<Event<Self>> + Into<<Self as frame_system::Config>::Event>;
    type Currency: ReservableCurrency<Self::AccountId>;
    // The amount for one report work extrinsic
    type BenefitReportWorkCost: Get<BalanceOf<Self>>;
    // The ratio between total benefit limitation and total reward
    type BenefitsLimitRatio: Get<Perbill>;
    // The ratio that benefit will cost, the remaining fee would still be charged
    type BenefitMarketCostRatio: Get<Perbill>;
    /// Number of eras that staked funds must remain bonded for.
    type BondingDuration: Get<EraIndex>;
}

decl_event!(
    pub enum Event<T> where
        Balance = BalanceOf<T>,
        AccountId = <T as frame_system::Config>::AccountId
    {
        /// Add benefit funds success.
        /// The first item is the account.
        /// The second item is the added benefit amount.
        AddBenefitFundsSuccess(AccountId, Balance, FundsType),
        /// Cut benefit funds success
        /// The first item is the account.
        /// The second item is the decreased benefit amount.
        CutBenefitFundsSuccess(AccountId, Balance, FundsType),
        /// Rebond benefit funds success
        /// The first item is the account.
        /// The second item is the rebonded benefit amount.
        RebondBenefitFundsSuccess(AccountId, Balance, FundsType),
        /// Withdraw benefit funds success
        /// The first item is the account.
        /// The second item is the withdrawed benefit amount.
        WithdrawBenefitFundsSuccess(AccountId, Balance),
    }
);

decl_error! {
    pub enum Error for Module<T: Config> {
        /// Don't have enough money
        InsuffientBalance,
        /// Don't have benefit records
        InvalidTarget,
        /// Can not schedule more unlock chunks.
        NoMoreChunks,
        /// Can not rebond without unlocking chunks.
        NoUnlockChunk,
        /// Can not bond with value less than minimum balance.
        InsufficientValue
    }
}

/// Just a Balance/BlockNumber tuple to encode when a chunk of funds will be unlocked.
#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, Default)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct FundsUnlockChunk<Balance: HasCompact> {
    /// Amount of funds to be unlocked.
    #[codec(compact)]
    value: Balance,
    /// Era number at which point it'll be unlocked.
    #[codec(compact)]
    era: EraIndex,
}

#[derive(Debug, PartialEq, Eq, Clone, Encode, Decode, Default)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct EraBenefits<Balance: HasCompact> {
    /// The total fee reduction quota in one era
    #[codec(compact)]
    pub total_fee_reduction_quota: Balance,
    /// The total market active funds amount
    #[codec(compact)]
    pub total_market_active_funds: Balance,
    /// The total used fee reduction quota
    #[codec(compact)]
    pub used_fee_reduction_quota: Balance,
    /// The latest active era index
    #[codec(compact)]
    pub active_era: EraIndex
}

#[derive(Debug, PartialEq, Eq, Clone, Encode, Decode, Default)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct MarketBenefit<Balance: HasCompact> {
    /// It's total funds value
    #[codec(compact)]
    pub total_funds: Balance,
    /// It's own active funds value
    #[codec(compact)]
    pub active_funds: Balance,
    /// The used reduction for fee
    #[codec(compact)]
    pub used_fee_reduction_quota: Balance,
    /// The file reward for market
    #[codec(compact)]
    pub file_reward: Balance,
    /// The latest refreshed active era index
    #[codec(compact)]
    pub refreshed_at: EraIndex,
    /// Any balance that is becoming free
    pub unlocking_funds: Vec<FundsUnlockChunk<Balance>>,
}

#[derive(Debug, PartialEq, Eq, Clone, Encode, Decode, Default)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct SworkBenefit<Balance: HasCompact> {
    /// It's total funds value
    #[codec(compact)]
    pub total_funds: Balance,
    /// It's own active funds value
    #[codec(compact)]
    pub active_funds: Balance,
    /// The total reduction count for report works
    pub total_fee_reduction_count: u32,
    /// The used reduction count for report works
    pub used_fee_reduction_count: u32,
    /// The latest refreshed active era index
    #[codec(compact)]
    pub refreshed_at: EraIndex,
    /// Any balance that is becoming free
    pub unlocking_funds: Vec<FundsUnlockChunk<Balance>>,
}

impl<Balance: HasCompact + Copy + Saturating + AtLeast32BitUnsigned> SworkBenefit<Balance> {
    fn consolidate_unlocked(mut self, current_era: EraIndex) -> Self {
        let mut total_funds = self.total_funds;
        let unlocking_funds = self
            .unlocking_funds
            .into_iter()
            .filter(|chunk| {
                if chunk.era > current_era {
                    true
                } else {
                    total_funds = total_funds.saturating_sub(chunk.value);
                    false
                }
            })
            .collect();
        self.total_funds = total_funds;
        self.unlocking_funds = unlocking_funds;
        self
    }

    /// Re-bond funds that were scheduled for unlocking.
    fn rebond(mut self, value: Balance) -> Self {
        let mut unlocking_balance: Balance = Zero::zero();

        while let Some(last) = self.unlocking_funds.last_mut() {
            if unlocking_balance + last.value <= value {
                unlocking_balance += last.value;
                self.active_funds += last.value;
                self.unlocking_funds.pop();
            } else {
                let diff = value - unlocking_balance;

                unlocking_balance += diff;
                self.active_funds += diff;
                last.value -= diff;
            }

            if unlocking_balance >= value {
                break
            }
        }

        self
    }
}

// TODO: Refine the following code and merge with SworkBenefit
impl<Balance: HasCompact + Copy + Saturating + AtLeast32BitUnsigned> MarketBenefit<Balance> {
    fn consolidate_unlocked(mut self, current_era: EraIndex) -> Self {
        let mut total_funds = self.total_funds;
        let unlocking_funds = self
            .unlocking_funds
            .into_iter()
            .filter(|chunk| {
                if chunk.era > current_era {
                    true
                } else {
                    total_funds = total_funds.saturating_sub(chunk.value);
                    false
                }
            })
            .collect();
        self.total_funds = total_funds;
        self.unlocking_funds = unlocking_funds;
        self
    }

    /// Re-bond funds that were scheduled for unlocking.
    fn rebond(mut self, value: Balance) -> Self {
        let mut unlocking_balance: Balance = Zero::zero();

        while let Some(last) = self.unlocking_funds.last_mut() {
            if unlocking_balance + last.value <= value {
                unlocking_balance += last.value;
                self.active_funds += last.value;
                self.unlocking_funds.pop();
            } else {
                let diff = value - unlocking_balance;

                unlocking_balance += diff;
                self.active_funds += diff;
                last.value -= diff;
            }

            if unlocking_balance >= value {
                break
            }
        }

        self
    }
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

    fn get_collateral_and_reward(who: &<T as frame_system::Config>::AccountId) -> (BalanceOf<T>, BalanceOf<T>) {
        let market_benefits = Self::market_benefits(who);
        (market_benefits.active_funds, market_benefits.file_reward)
    }

    fn update_reward(who: &<T as frame_system::Config>::AccountId, value: BalanceOf<T>) {
        let mut market_benefit = Self::market_benefits(who);
        market_benefit.file_reward = value;

        // Remove the dead benefit
        if market_benefit.unlocking_funds.is_empty() && market_benefit.active_funds.is_zero() && market_benefit.file_reward.is_zero() {
            <MarketBenefits<T>>::remove(&who);
        } else {
            <MarketBenefits<T>>::insert(&who, market_benefit);
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Encode, Decode)]
pub enum FundsType {
    SWORK = 0,
    MARKET = 1,
}


decl_storage! {
    trait Store for Module<T: Config> as Benefits {
        /// The global benefits information
        CurrentBenefits get(fn current_benefits): EraBenefits<BalanceOf<T>>;
        /// The market benefit
        MarketBenefits get(fn market_benefits): map hasher(blake2_128_concat) T::AccountId => MarketBenefit<BalanceOf<T>>;
        /// The sworker benefit
        SworkBenefits get(fn swork_benefits): map hasher(blake2_128_concat) T::AccountId => SworkBenefit<BalanceOf<T>>;
    }
}

decl_module! {
    pub struct Module<T: Config> for enum Call where origin: T::Origin {
        type Error = Error<T>;

        fn deposit_event() = default;

        /// Add benefit funds
        // TODO: Refine this weight
        #[weight = 1000]
        pub fn add_benefit_funds(origin, #[compact] value: BalanceOf<T>, funds_type: FundsType) -> DispatchResult {
            let who = ensure_signed(origin)?;

            // 1. Reserve the currency
            T::Currency::reserve(&who, value.clone()).map_err(|_| Error::<T>::InsuffientBalance)?;

            // 2. Change the benefits status
            match funds_type {
                FundsType::SWORK => {
                    <SworkBenefits<T>>::mutate(&who, |swork_benefit| {
                        swork_benefit.active_funds += value.clone();
                        swork_benefit.total_funds += value.clone();
                        swork_benefit.total_fee_reduction_count = Self::calculate_total_fee_reduction_count(&swork_benefit.active_funds);
                    });
                },
                FundsType::MARKET => {
                    <MarketBenefits<T>>::mutate(&who, |market_benefit| {
                        market_benefit.active_funds += value.clone();
                        market_benefit.total_funds += value.clone();
                    });
                    <CurrentBenefits<T>>::mutate(|benefits| { benefits.total_market_active_funds += value.clone();});
                }
            }

            // 3. Emit success
            Self::deposit_event(RawEvent::AddBenefitFundsSuccess(who.clone(), value, funds_type));

            Ok(())
        }

        /// Cut benefit funds
        // TODO: Refine this weight
        #[weight = 1000]
        pub fn cut_benefit_funds(origin, #[compact] value: BalanceOf<T>, funds_type: FundsType) -> DispatchResult {
            let who = ensure_signed(origin)?;

            match funds_type {
                FundsType::SWORK => {
                    // 1. Get benefit
                    ensure!(<SworkBenefits<T>>::contains_key(&who), Error::<T>::InvalidTarget);
                    let mut benefit = Self::swork_benefits(&who);

                    // 2. Judge if exceed MAX_UNLOCKING_CHUNKS
                    ensure!(
                        benefit.unlocking_funds.len() < MAX_UNLOCKING_CHUNKS,
                        Error::<T>::NoMoreChunks
                    );
                    // 3. Ensure value < benefit.active_funds
                    let mut value = value;
                    value = value.min(benefit.active_funds);

                    if !value.is_zero() {
                        benefit.active_funds -= value;

                        // 4. Avoid there being a dust balance left in the benefit system.
                        if benefit.active_funds < T::Currency::minimum_balance() {
                            value += benefit.active_funds;
                            benefit.active_funds = Zero::zero();
                        }

                        // 5. Update benefit
                        let era = Self::current_benefits().active_era + T::BondingDuration::get();
                        benefit.unlocking_funds.push(FundsUnlockChunk { value, era });
                        benefit.total_fee_reduction_count = Self::calculate_total_fee_reduction_count(&benefit.active_funds);
                        <SworkBenefits<T>>::insert(&who, benefit);
                    }
                },
                FundsType::MARKET => {
                    // 1. Get benefit
                    ensure!(<MarketBenefits<T>>::contains_key(&who), Error::<T>::InvalidTarget);
                    let mut benefit = Self::market_benefits(&who);

                    // 2. Judge if exceed MAX_UNLOCKING_CHUNKS
                    ensure!(
                        benefit.unlocking_funds.len() < MAX_UNLOCKING_CHUNKS,
                        Error::<T>::NoMoreChunks
                    );
                    // 3. Ensure value < benefit.active_funds
                    let mut value = value;
                    value = value.min(benefit.active_funds);

                    if !value.is_zero() {
                        benefit.active_funds -= value;

                        // 4. Avoid there being a dust balance left in the benefit system.
                        if benefit.active_funds < T::Currency::minimum_balance() {
                            value += benefit.active_funds;
                            benefit.active_funds = Zero::zero();
                        }

                        // 5. Update benefit
                        let era = Self::current_benefits().active_era + T::BondingDuration::get();
                        benefit.unlocking_funds.push(FundsUnlockChunk { value, era });
                        <MarketBenefits<T>>::insert(&who, benefit);
                        <CurrentBenefits<T>>::mutate(|benefits| { benefits.total_market_active_funds = benefits.total_market_active_funds.saturating_sub(value.clone());});
                    }
                }
            };
            // 6. Send event
            Self::deposit_event(RawEvent::CutBenefitFundsSuccess(who.clone(), value, funds_type));
            Ok(())
        }

        /// Withdraw benefit funds
        // TODO: Refine this weight
        #[weight = 1000]
        pub fn withdraw_benefit_funds(origin) -> DispatchResult {
            let who = ensure_signed(origin)?;

            let mut unreserved_value: BalanceOf<T> = Zero::zero();
            if <SworkBenefits<T>>::contains_key(&who) {
                Self::check_and_update_swork_funds(&who);
                let mut benefit = Self::swork_benefits(&who);

                // 2. Update total funds
                let old_total_funds = benefit.total_funds;
                let active_era = Self::current_benefits().active_era;
                benefit = benefit.consolidate_unlocked(active_era);

                // 3. Unreserve the currency
                let to_unreserved_value = old_total_funds.saturating_sub(benefit.total_funds);
                T::Currency::unreserve(&who, to_unreserved_value);

                // 4. Update or remove the who's fee reduction benefits for report works
                if benefit.unlocking_funds.is_empty() && benefit.active_funds.is_zero() {
                    <SworkBenefits<T>>::remove(&who);
                } else {
                    <SworkBenefits<T>>::insert(&who, benefit);
                }

                unreserved_value = unreserved_value.saturating_add(to_unreserved_value);
            }
            if <MarketBenefits<T>>::contains_key(&who) {
                Self::check_and_update_market_funds(&who);
                let mut benefit = Self::market_benefits(&who);

                // 2. Update total funds
                let old_total_funds = benefit.total_funds;
                let active_era = Self::current_benefits().active_era;
                benefit = benefit.consolidate_unlocked(active_era);

                // 3. Unreserve the currency
                let to_unreserved_value = old_total_funds.saturating_sub(benefit.total_funds);
                T::Currency::unreserve(&who, to_unreserved_value);

                // 4. Update or remove the who's fee reduction benefits for report works
                if benefit.unlocking_funds.is_empty() && benefit.active_funds.is_zero() && benefit.file_reward.is_zero() {
                    <MarketBenefits<T>>::remove(&who);
                } else {
                    <MarketBenefits<T>>::insert(&who, benefit);
                }

                unreserved_value = unreserved_value.saturating_add(to_unreserved_value);
            }

            // 5. Emit success
            Self::deposit_event(RawEvent::WithdrawBenefitFundsSuccess(who.clone(), unreserved_value));

            Ok(())
        }

        /// Withdraw benefit funds
        // TODO: Refine this weight
        #[weight = 1000]
        pub fn rebond_benefit_funds(origin, #[compact] value: BalanceOf<T>, funds_type: FundsType) -> DispatchResult {
            let who = ensure_signed(origin)?;

            match funds_type {
                FundsType::SWORK => {
                    // 1. Get benefit
                    ensure!(<SworkBenefits<T>>::contains_key(&who), Error::<T>::InvalidTarget);
                    let mut benefit = Self::swork_benefits(&who);
                    ensure!(!benefit.unlocking_funds.is_empty(), Error::<T>::NoUnlockChunk);

                    // 2. Rebond benefit
                    benefit = benefit.rebond(value);

                    ensure!(benefit.active_funds >= T::Currency::minimum_balance(), Error::<T>::InsufficientValue);
                    // 3. Update total fee reduction count according to active funds
                    benefit.total_fee_reduction_count = Self::calculate_total_fee_reduction_count(&benefit.active_funds);
                    <SworkBenefits<T>>::insert(&who, benefit);
                },
                FundsType::MARKET => {
                    // 1. Get benefit
                    ensure!(<MarketBenefits<T>>::contains_key(&who), Error::<T>::InvalidTarget);
                    let mut benefit = Self::market_benefits(&who);
                    let old_active_funds = benefit.active_funds;
                    ensure!(!benefit.unlocking_funds.is_empty(), Error::<T>::NoUnlockChunk);

                    // 2. Rebond benefit
                    benefit = benefit.rebond(value);

                    ensure!(benefit.active_funds >= T::Currency::minimum_balance(), Error::<T>::InsufficientValue);
                    let new_active_funds = benefit.active_funds;
                    <MarketBenefits<T>>::insert(&who, benefit);
                    // 3. Update current benefits
                    <CurrentBenefits<T>>::mutate(|benefits| { benefits.total_market_active_funds = benefits.total_market_active_funds.saturating_add(new_active_funds).saturating_sub(old_active_funds);});
                }
            };
            // 4. Send event
            Self::deposit_event(RawEvent::RebondBenefitFundsSuccess(who.clone(), value, funds_type));
            Ok(())
        }
    }
}


impl<T: Config> Module<T> {
    /// The return value is the used fee quota in the last era
    pub fn do_update_era_benefit(next_era: EraIndex, total_fee_reduction_quota: BalanceOf<T>) -> BalanceOf<T> {
        // Fetch overall benefits information
        let mut current_benefits = Self::current_benefits();
        // Store the used fee reduction in the last era
        let used_fee_reduction_quota = current_benefits.used_fee_reduction_quota;
        // Start the next era and set active era to it
        current_benefits.active_era = next_era;
        // Set the new total benefits for the next era
        current_benefits.total_fee_reduction_quota = total_fee_reduction_quota;
        // Reset used benefits to zero
        current_benefits.used_fee_reduction_quota = Zero::zero();
        <CurrentBenefits<T>>::put(current_benefits);
        // Return the used benefits in the last era
        used_fee_reduction_quota
    }

    pub fn maybe_do_free_count(who: &T::AccountId) -> bool {
        let active_era = Self::current_benefits().active_era;
        let mut swork_benefit = Self::swork_benefits(who);
        Self::maybe_refresh_swork_benefits(active_era, &mut swork_benefit);
        // won't update reduction detail if it has no staking
        // to save db writing time
        if swork_benefit.used_fee_reduction_count < swork_benefit.total_fee_reduction_count {
            swork_benefit.used_fee_reduction_count += 1;
            <SworkBenefits<T>>::insert(&who, swork_benefit);
            return true;
        }
        return false;
    }

    pub fn maybe_do_reduce_fee(who: &T::AccountId, fee: BalanceOf<T>, reasons: WithdrawReasons) -> Result<NegativeImbalanceOf<T>, DispatchError> {
        let mut current_benefits = Self::current_benefits();
        let mut market_benefit = Self::market_benefits(who);
        // Refresh the reduction
        Self::maybe_refresh_market_benefit(current_benefits.active_era, &mut market_benefit);
        // Calculate the own reduction limit
        let fee_reduction_benefits_quota = Self::calculate_fee_reduction_quota(market_benefit.active_funds,
                                                                               current_benefits.total_market_active_funds,
                                                                               current_benefits.total_fee_reduction_quota);
        // Try to free fee reduction
        // Check the person has his own fee reduction quota and the total benefits
        let fee_reduction_benefit_cost = T::BenefitMarketCostRatio::get() * fee;
        let (charged_fee, used_fee_reduction) = if market_benefit.used_fee_reduction_quota + fee_reduction_benefit_cost <= fee_reduction_benefits_quota && current_benefits.used_fee_reduction_quota + fee_reduction_benefit_cost <= current_benefits.total_fee_reduction_quota {
            // it's ok to free this fee
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
                // won't update reduction detail if it has no active_funds
                // to save db writing time
                if !used_fee_reduction.is_zero() {
                    // update the reduction information
                    current_benefits.used_fee_reduction_quota += used_fee_reduction;
                    market_benefit.used_fee_reduction_quota += used_fee_reduction;
                    <MarketBenefits<T>>::insert(&who, market_benefit);
                    <CurrentBenefits<T>>::put(current_benefits);
                    // issue the free fee
                    let new_issued = T::Currency::issue(used_fee_reduction.clone());
                    imbalance.subsume(new_issued);
                }
                Ok(imbalance)
            }
            Err(err) => Err(err),
        };
        result
    }

    fn check_and_update_swork_funds(who: &T::AccountId) {
        let mut swork_benefit = Self::swork_benefits(&who);
        let reserved_value = T::Currency::reserved_balance(who);
        if swork_benefit.total_funds <= reserved_value {
            return;
        }
        // Something wrong, fix it
        let old_total_funds = swork_benefit.total_funds;
        swork_benefit.total_funds = reserved_value;
        swork_benefit.active_funds = swork_benefit.active_funds.saturating_add(swork_benefit.total_funds).saturating_sub(old_total_funds);
        swork_benefit.total_fee_reduction_count = Self::calculate_total_fee_reduction_count(&swork_benefit.active_funds);
        <SworkBenefits<T>>::insert(&who, swork_benefit);
    }

    fn check_and_update_market_funds(who: &T::AccountId) {
        let mut market_benefit = Self::market_benefits(&who);
        let reserved_value = T::Currency::reserved_balance(who);
        if market_benefit.total_funds <= reserved_value {
            return;
        }
        // Something wrong, fix it
        let old_total_funds = market_benefit.total_funds;
        let old_active_funds = market_benefit.active_funds;
        market_benefit.total_funds = reserved_value;
        market_benefit.active_funds = market_benefit.active_funds.saturating_add(market_benefit.total_funds).saturating_sub(old_total_funds);
        <CurrentBenefits<T>>::mutate(|benefits| { benefits.total_market_active_funds = benefits.total_market_active_funds.saturating_add(market_benefit.active_funds).saturating_sub(old_active_funds);});
        <MarketBenefits<T>>::insert(&who, market_benefit);
    }

    pub fn calculate_fee_reduction_quota(market_active_funds: BalanceOf<T>, total_market_active_funds: BalanceOf<T>, total_fee_reduction_quota: BalanceOf<T>) -> BalanceOf<T> {
        Perbill::from_rational_approximation(market_active_funds, total_market_active_funds) * total_fee_reduction_quota
    }

    pub fn calculate_total_fee_reduction_count(active_funds: &BalanceOf<T>) -> u32 {
        (*active_funds / T::BenefitReportWorkCost::get()).saturated_into()
    }

    pub fn maybe_refresh_market_benefit(latest_active_era: EraIndex, market_benefit: &mut MarketBenefit<BalanceOf<T>>) {
        if market_benefit.refreshed_at < latest_active_era {
            market_benefit.refreshed_at = latest_active_era;
            market_benefit.used_fee_reduction_quota = Zero::zero();
        }
    }

    pub fn maybe_refresh_swork_benefits(latest_active_era: EraIndex, swork_benefit: &mut SworkBenefit<BalanceOf<T>>) {
        if swork_benefit.refreshed_at < latest_active_era {
            swork_benefit.refreshed_at = latest_active_era;
            swork_benefit.used_fee_reduction_count = 0;
        }
    }
}

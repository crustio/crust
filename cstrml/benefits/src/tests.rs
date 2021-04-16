// Copyright (C) 2019-2021 Crust Network Technologies Ltd.
// This file is part of Crust.

use super::*;
use crate::{mock::*, EraBenefits, FeeReductionBenefit};
use frame_support::assert_ok;
use balances::NegativeImbalance;

#[test]
fn update_overall_info_should_work() {
    new_test_ext().execute_with(|| {
        Benefits::update_era_benefit(10u32.into(), 100);
        assert_eq!(Benefits::current_benefits(), EraBenefits {
            total_benefits: 1,
            total_funds: 0,
            used_benefits: 0,
            active_era: 10
        });
    });
}

#[test]
fn add_benefit_funds_should_work() {
    new_test_ext().execute_with(|| {
        Benefits::update_era_benefit(10u32.into(), 100);
        assert_eq!(Benefits::current_benefits(), EraBenefits {
            total_benefits: 1,
            total_funds: 0,
            used_benefits: 0,
            active_era: 10
        });
        let _ = Balances::make_free_balance_be(&ALICE, 200_000);
        assert_ok!(Benefits::add_benefit_funds(Origin::signed(ALICE.clone()), 100));
        assert_eq!(Benefits::fee_reduction_benefits(&ALICE), FeeReductionBenefit {
            funds: 100,
            total_fee_reduction_count: 2,
            used_fee_reduction_quota: 0,
            used_fee_reduction_count: 0,
            refreshed_at: 0
        });
        assert_eq!(Benefits::current_benefits(), EraBenefits {
            total_benefits: 1,
            total_funds: 100,
            used_benefits: 0,
            active_era: 10
        });
    });
}

#[test]
fn cut_benefit_funds_should_work() {
    new_test_ext().execute_with(|| {
        Benefits::update_era_benefit(10u32.into(), 100);
        assert_eq!(Benefits::current_benefits(), EraBenefits {
            total_benefits: 1,
            total_funds: 0,
            used_benefits: 0,
            active_era: 10
        });
        let _ = Balances::make_free_balance_be(&ALICE, 200_000);

        assert_ok!(Benefits::add_benefit_funds(Origin::signed(ALICE.clone()), 100));
        assert_eq!(Benefits::fee_reduction_benefits(&ALICE), FeeReductionBenefit {
            funds: 100,
            total_fee_reduction_count: 2,
            used_fee_reduction_quota: 0,
            used_fee_reduction_count: 0,
            refreshed_at: 0
        });
        assert_eq!(Benefits::current_benefits(), EraBenefits {
            total_benefits: 1,
            total_funds: 100,
            used_benefits: 0,
            active_era: 10
        });

        assert_ok!(Benefits::cut_benefit_funds(Origin::signed(ALICE.clone()), 50));
        assert_eq!(Benefits::fee_reduction_benefits(&ALICE), FeeReductionBenefit {
            funds: 50,
            total_fee_reduction_count: 1,
            used_fee_reduction_quota: 0,
            used_fee_reduction_count: 0,
            refreshed_at: 0
        });
        assert_eq!(Benefits::current_benefits(), EraBenefits {
            total_benefits: 1,
            total_funds: 50,
            used_benefits: 0,
            active_era: 10
        });
    });
}

#[test]
fn maybe_free_count_should_work() {
    new_test_ext().execute_with(|| {
        Benefits::update_era_benefit(10u32.into(), 100);
        assert_eq!(Benefits::maybe_free_count(&ALICE), false);
        // won't update reduction detail since it has not staking
        assert_eq!(Benefits::fee_reduction_benefits(&ALICE), FeeReductionBenefit {
            funds: 0,
            total_fee_reduction_count: 0,
            used_fee_reduction_quota: 0,
            used_fee_reduction_count: 0,
            refreshed_at: 0
        });
        assert_eq!(Benefits::current_benefits(), EraBenefits {
            total_benefits: 1,
            total_funds: 0,
            used_benefits: 0,
            active_era: 10
        });
        let _ = Balances::make_free_balance_be(&ALICE, 200_000);
        assert_ok!(Benefits::add_benefit_funds(Origin::signed(ALICE.clone()), 105));
        assert_eq!(Benefits::maybe_free_count(&ALICE), true);
        assert_eq!(Benefits::fee_reduction_benefits(&ALICE), FeeReductionBenefit {
            funds: 105,
            total_fee_reduction_count: 2,
            used_fee_reduction_quota: 0,
            used_fee_reduction_count: 1,
            refreshed_at: 10
        });
        assert_eq!(Benefits::maybe_free_count(&ALICE), true);
        assert_eq!(Benefits::fee_reduction_benefits(&ALICE), FeeReductionBenefit {
            funds: 105,
            total_fee_reduction_count: 2,
            used_fee_reduction_quota: 0,
            used_fee_reduction_count: 2,
            refreshed_at: 10
        });
        // Reach the limitation => cannot free this operation
        assert_eq!(Benefits::maybe_free_count(&ALICE), false);
        assert_eq!(Benefits::fee_reduction_benefits(&ALICE), FeeReductionBenefit {
            funds: 105,
            total_fee_reduction_count: 2,
            used_fee_reduction_quota: 0,
            used_fee_reduction_count: 2,
            refreshed_at: 10
        });
        // New era => refresh the limitation
        Benefits::update_era_benefit(11u32.into(), 100);
        assert_eq!(Benefits::maybe_free_count(&ALICE), true);
        assert_eq!(Benefits::fee_reduction_benefits(&ALICE), FeeReductionBenefit {
            funds: 105,
            total_fee_reduction_count: 2,
            used_fee_reduction_quota: 0,
            used_fee_reduction_count: 1,
            refreshed_at: 11
        });
        assert_eq!(Benefits::maybe_free_count(&ALICE), true);
        assert_eq!(Benefits::fee_reduction_benefits(&ALICE), FeeReductionBenefit {
            funds: 105,
            total_fee_reduction_count: 2,
            used_fee_reduction_quota: 0,
            used_fee_reduction_count: 2,
            refreshed_at: 11
        });
        // Reach the limitation => cannot free this operation
        assert_eq!(Benefits::maybe_free_count(&ALICE), false);
        assert_eq!(Benefits::fee_reduction_benefits(&ALICE), FeeReductionBenefit {
            funds: 105,
            total_fee_reduction_count: 2,
            used_fee_reduction_quota: 0,
            used_fee_reduction_count: 2,
            refreshed_at: 11
        });
    });
}

#[test]
fn maybe_reduce_fee_should_work() {
    new_test_ext().execute_with(|| {
        let _ = Balances::make_free_balance_be(&ALICE, 200);
        let target_fee = NegativeImbalance::new(20);
        assert_eq!(Balances::total_issuance(), 200);
        Benefits::update_era_benefit(10u32.into(), 10_000);
        assert_eq!(Benefits::current_benefits(), EraBenefits {
            total_benefits: 100,
            total_funds: 0,
            used_benefits: 0,
            active_era: 10
        });
        assert_eq!(Benefits::maybe_reduce_fee(&ALICE, 20, WithdrawReasons::TRANSACTION_PAYMENT).unwrap(), target_fee);
        // won't update reduction detail since it has not staking
        assert_eq!(Benefits::fee_reduction_benefits(&ALICE), FeeReductionBenefit {
            funds: 0,
            total_fee_reduction_count: 0,
            used_fee_reduction_quota: 0,
            used_fee_reduction_count: 0,
            refreshed_at: 0
        });
        assert_eq!(Balances::total_balance(&ALICE), 180);
        assert_eq!(Balances::total_issuance(), 180);
        assert_ok!(Benefits::add_benefit_funds(Origin::signed(ALICE.clone()), 105));
        assert_eq!(Benefits::maybe_reduce_fee(&ALICE, 20, WithdrawReasons::TRANSACTION_PAYMENT).unwrap(), target_fee);
        assert_eq!(Benefits::fee_reduction_benefits(&ALICE), FeeReductionBenefit {
            funds: 105,
            total_fee_reduction_count: 2,
            used_fee_reduction_quota: 19,
            used_fee_reduction_count: 0,
            refreshed_at: 10
        });
        assert_eq!(Benefits::current_benefits(), EraBenefits {
            total_benefits: 100,
            total_funds: 105,
            used_benefits: 19,
            active_era: 10
        });
        assert_eq!(Balances::total_issuance(), 179); // 180 - 20 + 19
        assert_eq!(Balances::total_balance(&ALICE), 179);
    });
}

#[test]
fn update_overall_info_should_return_used_fee() {
    new_test_ext().execute_with(|| {
        Benefits::update_era_benefit(10u32.into(), 10_000);
        let _ = Balances::make_free_balance_be(&ALICE, 200);
        let target_fee = NegativeImbalance::new(20);
        assert_ok!(Benefits::add_benefit_funds(Origin::signed(ALICE.clone()), 105));
        assert_eq!(Benefits::maybe_reduce_fee(&ALICE, 20, WithdrawReasons::TRANSACTION_PAYMENT).unwrap(), target_fee);
        assert_eq!(Benefits::fee_reduction_benefits(&ALICE), FeeReductionBenefit {
            funds: 105,
            total_fee_reduction_count: 2,
            used_fee_reduction_quota: 19,
            used_fee_reduction_count: 0,
            refreshed_at: 10
        });
        assert_eq!(Benefits::current_benefits(), EraBenefits {
            total_benefits: 100,
            total_funds: 105,
            used_benefits: 19,
            active_era: 10
        });
        assert_eq!(Benefits::update_era_benefit(11u32.into(), 10_000), 19);
        assert_eq!(Benefits::current_benefits(), EraBenefits {
            total_benefits: 100,
            total_funds: 105,
            used_benefits: 0,
            active_era: 11
        });
    });
}

#[test]
fn free_fee_limit_should_work() {
    new_test_ext().execute_with(|| {
        let _ = Balances::make_free_balance_be(&ALICE, 200);
        let _ = Balances::make_free_balance_be(&BOB, 200);
        let target_fee = NegativeImbalance::new(40);
        assert_eq!(Balances::total_issuance(), 400);
        Benefits::update_era_benefit(10u32.into(), 10_000);
        assert_eq!(Benefits::current_benefits(), EraBenefits {
            total_benefits: 100,
            total_funds: 0,
            used_benefits: 0,
            active_era: 10
        });
        assert_ok!(Benefits::add_benefit_funds(Origin::signed(ALICE.clone()), 100));
        assert_ok!(Benefits::add_benefit_funds(Origin::signed(BOB.clone()), 100));
        assert_eq!(Benefits::maybe_reduce_fee(&ALICE, 40, WithdrawReasons::TRANSACTION_PAYMENT).unwrap(), target_fee);
        assert_eq!(Benefits::fee_reduction_benefits(&ALICE), FeeReductionBenefit {
            funds: 100,
            total_fee_reduction_count: 2,
            used_fee_reduction_quota: 38,
            used_fee_reduction_count: 0,
            refreshed_at: 10
        });
        assert_eq!(Benefits::current_benefits(), EraBenefits {
            total_benefits: 100,
            total_funds: 200,
            used_benefits: 38,
            active_era: 10
        });
        assert_eq!(Balances::total_issuance(), 398); // 400 - 40 + 38
        assert_eq!(Balances::total_balance(&ALICE), 198);

        // Reach his own limitation
        assert_eq!(Benefits::maybe_reduce_fee(&ALICE, 40, WithdrawReasons::TRANSACTION_PAYMENT).unwrap(), target_fee);
        assert_eq!(Benefits::fee_reduction_benefits(&ALICE), FeeReductionBenefit {
            funds: 100,
            total_fee_reduction_count: 2,
            used_fee_reduction_quota: 38,
            used_fee_reduction_count: 0,
            refreshed_at: 10
        });
        assert_eq!(Benefits::current_benefits(), EraBenefits {
            total_benefits: 100,
            total_funds: 200,
            used_benefits: 38,
            active_era: 10
        });
        assert_eq!(Balances::total_issuance(), 358); // 398 - 40
        assert_eq!(Balances::total_balance(&ALICE), 158);
        assert_ok!(Benefits::cut_benefit_funds(Origin::signed(BOB.clone()), 100));
        assert_eq!(Benefits::current_benefits(), EraBenefits {
            total_benefits: 100,
            total_funds: 100,
            used_benefits: 38,
            active_era: 10
        });

        // Since Bob cut his collateral, Alice has more limitation, it's free again
        assert_eq!(Benefits::maybe_reduce_fee(&ALICE, 40, WithdrawReasons::TRANSACTION_PAYMENT).unwrap(), target_fee);
        assert_eq!(Benefits::fee_reduction_benefits(&ALICE), FeeReductionBenefit {
            funds: 100,
            total_fee_reduction_count: 2,
            used_fee_reduction_quota: 76,
            used_fee_reduction_count: 0,
            refreshed_at: 10
        });
        assert_eq!(Benefits::current_benefits(), EraBenefits {
            total_benefits: 100,
            total_funds: 100,
            used_benefits: 76,
            active_era: 10
        });
        assert_eq!(Balances::total_issuance(), 356); // 358 - 2
        assert_eq!(Balances::total_balance(&ALICE), 156);

        assert_ok!(Benefits::add_benefit_funds(Origin::signed(BOB.clone()), 100));
        // Bob has his own limitation, but total free fee reduction is not enough
        assert_eq!(Benefits::maybe_reduce_fee(&BOB, 40, WithdrawReasons::TRANSACTION_PAYMENT).unwrap(), target_fee);
        assert_eq!(Benefits::fee_reduction_benefits(&BOB), FeeReductionBenefit {
            funds: 100,
            total_fee_reduction_count: 2,
            used_fee_reduction_quota: 0,
            used_fee_reduction_count: 0,
            refreshed_at: 0
        });
        assert_eq!(Benefits::current_benefits(), EraBenefits {
            total_benefits: 100,
            total_funds: 200,
            used_benefits: 76,
            active_era: 10
        });
        assert_eq!(Balances::total_issuance(), 316); // 358 - 40
        assert_eq!(Balances::total_balance(&BOB), 160);

    });
}

#[test]
fn currency_is_insufficient() {
    new_test_ext().execute_with(|| {
        let _ = Balances::make_free_balance_be(&ALICE, 30);
        assert_eq!(Balances::total_issuance(), 30);
        Benefits::update_era_benefit(10u32.into(), 10_000);
        assert_eq!(Benefits::current_benefits(), EraBenefits {
            total_benefits: 100,
            total_funds: 0,
            used_benefits: 0,
            active_era: 10
        });
        assert_eq!(Benefits::maybe_reduce_fee(&ALICE, 40, WithdrawReasons::TRANSACTION_PAYMENT).is_err(), true);
        assert_ok!(Benefits::add_benefit_funds(Origin::signed(ALICE.clone()), 25));
        assert_eq!(Benefits::maybe_reduce_fee(&ALICE, 200, WithdrawReasons::TRANSACTION_PAYMENT).is_err(), true);
    });
}
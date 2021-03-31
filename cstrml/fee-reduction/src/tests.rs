// Copyright (C) 2019-2021 Crust Network Technologies Ltd.
// This file is part of Crust.

use super::*;
use crate::{mock::*, OverallReductionInfo, ReductionDetail};
use frame_support::assert_ok;
use balances::NegativeImbalance;

#[test]
fn update_overall_info_should_work() {
    new_test_ext().execute_with(|| {
        FeeReduction::update_overall_reduction_info(10u32.into(), 100);
        assert_eq!(FeeReduction::overall_reduction(), OverallReductionInfo {
            total_fee_reduction: 1,
            total_staking: 0,
            used_fee_reduction: 0,
            active_era: 10
        });
    });
}

#[test]
fn add_collateral_should_work() {
    new_test_ext().execute_with(|| {
        FeeReduction::update_overall_reduction_info(10u32.into(), 100);
        assert_eq!(FeeReduction::overall_reduction(), OverallReductionInfo {
            total_fee_reduction: 1,
            total_staking: 0,
            used_fee_reduction: 0,
            active_era: 10
        });
        let _ = Balances::make_free_balance_be(&ALICE, 200_000);
        assert_ok!(FeeReduction::add_collateral(Origin::signed(ALICE.clone()), 100));
        assert_eq!(FeeReduction::reduction_info(&ALICE), ReductionDetail {
            own_staking: 100,
            used_fee_reduction: 0,
            used_count_reduction: 0,
            refreshed_at: 0
        });
        assert_eq!(FeeReduction::overall_reduction(), OverallReductionInfo {
            total_fee_reduction: 1,
            total_staking: 100,
            used_fee_reduction: 0,
            active_era: 10
        });
    });
}

#[test]
fn cut_collateral_should_work() {
    new_test_ext().execute_with(|| {
        FeeReduction::update_overall_reduction_info(10u32.into(), 100);
        assert_eq!(FeeReduction::overall_reduction(), OverallReductionInfo {
            total_fee_reduction: 1,
            total_staking: 0,
            used_fee_reduction: 0,
            active_era: 10
        });
        let _ = Balances::make_free_balance_be(&ALICE, 200_000);

        assert_ok!(FeeReduction::add_collateral(Origin::signed(ALICE.clone()), 100));
        assert_eq!(FeeReduction::reduction_info(&ALICE), ReductionDetail {
            own_staking: 100,
            used_fee_reduction: 0,
            used_count_reduction: 0,
            refreshed_at: 0
        });
        assert_eq!(FeeReduction::overall_reduction(), OverallReductionInfo {
            total_fee_reduction: 1,
            total_staking: 100,
            used_fee_reduction: 0,
            active_era: 10
        });

        assert_ok!(FeeReduction::cut_collateral(Origin::signed(ALICE.clone()), 50));
        assert_eq!(FeeReduction::reduction_info(&ALICE), ReductionDetail {
            own_staking: 50,
            used_fee_reduction: 0,
            used_count_reduction: 0,
            refreshed_at: 0
        });
        assert_eq!(FeeReduction::overall_reduction(), OverallReductionInfo {
            total_fee_reduction: 1,
            total_staking: 50,
            used_fee_reduction: 0,
            active_era: 10
        });
    });
}

#[test]
fn try_to_free_count_should_work() {
    new_test_ext().execute_with(|| {
        FeeReduction::update_overall_reduction_info(10u32.into(), 100);
        assert_eq!(FeeReduction::try_to_free_count(&ALICE), false);
        // won't update reduction detail since it has not staking
        assert_eq!(FeeReduction::reduction_info(&ALICE), ReductionDetail {
            own_staking: 0,
            used_fee_reduction: 0,
            used_count_reduction: 0,
            refreshed_at: 0
        });
        assert_eq!(FeeReduction::overall_reduction(), OverallReductionInfo {
            total_fee_reduction: 1,
            total_staking: 0,
            used_fee_reduction: 0,
            active_era: 10
        });
        let _ = Balances::make_free_balance_be(&ALICE, 200_000);
        assert_ok!(FeeReduction::add_collateral(Origin::signed(ALICE.clone()), 105));
        assert_eq!(FeeReduction::try_to_free_count(&ALICE), true);
        assert_eq!(FeeReduction::reduction_info(&ALICE), ReductionDetail {
            own_staking: 105,
            used_fee_reduction: 0,
            used_count_reduction: 1,
            refreshed_at: 10
        });
        assert_eq!(FeeReduction::try_to_free_count(&ALICE), true);
        assert_eq!(FeeReduction::reduction_info(&ALICE), ReductionDetail {
            own_staking: 105,
            used_fee_reduction: 0,
            used_count_reduction: 2,
            refreshed_at: 10
        });
        // Reach the limitation => cannot free this operation
        assert_eq!(FeeReduction::try_to_free_count(&ALICE), false);
        assert_eq!(FeeReduction::reduction_info(&ALICE), ReductionDetail {
            own_staking: 105,
            used_fee_reduction: 0,
            used_count_reduction: 2,
            refreshed_at: 10
        });
        // New era => refresh the limitation
        FeeReduction::update_overall_reduction_info(11u32.into(), 100);
        assert_eq!(FeeReduction::try_to_free_count(&ALICE), true);
        assert_eq!(FeeReduction::reduction_info(&ALICE), ReductionDetail {
            own_staking: 105,
            used_fee_reduction: 0,
            used_count_reduction: 1,
            refreshed_at: 11
        });
        assert_eq!(FeeReduction::try_to_free_count(&ALICE), true);
        assert_eq!(FeeReduction::reduction_info(&ALICE), ReductionDetail {
            own_staking: 105,
            used_fee_reduction: 0,
            used_count_reduction: 2,
            refreshed_at: 11
        });
        // Reach the limitation => cannot free this operation
        assert_eq!(FeeReduction::try_to_free_count(&ALICE), false);
        assert_eq!(FeeReduction::reduction_info(&ALICE), ReductionDetail {
            own_staking: 105,
            used_fee_reduction: 0,
            used_count_reduction: 2,
            refreshed_at: 11
        });
    });
}

#[test]
fn try_to_free_fee_should_work() {
    new_test_ext().execute_with(|| {
        let _ = Balances::make_free_balance_be(&ALICE, 200);
        let target_fee = NegativeImbalance::new(20);
        assert_eq!(Balances::total_issuance(), 200);
        FeeReduction::update_overall_reduction_info(10u32.into(), 10_000);
        assert_eq!(FeeReduction::overall_reduction(), OverallReductionInfo {
            total_fee_reduction: 100,
            total_staking: 0,
            used_fee_reduction: 0,
            active_era: 10
        });
        assert_eq!(FeeReduction::try_to_free_fee(&ALICE, 20, WithdrawReasons::TRANSACTION_PAYMENT).unwrap(), target_fee);
        // won't update reduction detail since it has not staking
        assert_eq!(FeeReduction::reduction_info(&ALICE), ReductionDetail {
            own_staking: 0,
            used_fee_reduction: 0,
            used_count_reduction: 0,
            refreshed_at: 0
        });
        assert_eq!(Balances::total_balance(&ALICE), 180);
        assert_eq!(Balances::total_issuance(), 180);
        assert_ok!(FeeReduction::add_collateral(Origin::signed(ALICE.clone()), 105));
        assert_eq!(FeeReduction::try_to_free_fee(&ALICE, 20, WithdrawReasons::TRANSACTION_PAYMENT).unwrap(), target_fee);
        assert_eq!(FeeReduction::reduction_info(&ALICE), ReductionDetail {
            own_staking: 105,
            used_fee_reduction: 19,
            used_count_reduction: 0,
            refreshed_at: 10
        });
        assert_eq!(FeeReduction::overall_reduction(), OverallReductionInfo {
            total_fee_reduction: 100,
            total_staking: 105,
            used_fee_reduction: 19,
            active_era: 10
        });
        assert_eq!(Balances::total_issuance(), 179); // 180 - 20 + 19
        assert_eq!(Balances::total_balance(&ALICE), 179);
    });
}

#[test]
fn update_overall_info_should_return_used_fee() {
    new_test_ext().execute_with(|| {
        FeeReduction::update_overall_reduction_info(10u32.into(), 10_000);
        let _ = Balances::make_free_balance_be(&ALICE, 200);
        let target_fee = NegativeImbalance::new(20);
        assert_ok!(FeeReduction::add_collateral(Origin::signed(ALICE.clone()), 105));
        assert_eq!(FeeReduction::try_to_free_fee(&ALICE, 20, WithdrawReasons::TRANSACTION_PAYMENT).unwrap(), target_fee);
        assert_eq!(FeeReduction::reduction_info(&ALICE), ReductionDetail {
            own_staking: 105,
            used_fee_reduction: 19,
            used_count_reduction: 0,
            refreshed_at: 10
        });
        assert_eq!(FeeReduction::overall_reduction(), OverallReductionInfo {
            total_fee_reduction: 100,
            total_staking: 105,
            used_fee_reduction: 19,
            active_era: 10
        });
        assert_eq!(FeeReduction::update_overall_reduction_info(11u32.into(), 10_000), 19);
        assert_eq!(FeeReduction::overall_reduction(), OverallReductionInfo {
            total_fee_reduction: 100,
            total_staking: 105,
            used_fee_reduction: 0,
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
        FeeReduction::update_overall_reduction_info(10u32.into(), 10_000);
        assert_eq!(FeeReduction::overall_reduction(), OverallReductionInfo {
            total_fee_reduction: 100,
            total_staking: 0,
            used_fee_reduction: 0,
            active_era: 10
        });
        assert_ok!(FeeReduction::add_collateral(Origin::signed(ALICE.clone()), 100));
        assert_ok!(FeeReduction::add_collateral(Origin::signed(BOB.clone()), 100));
        assert_eq!(FeeReduction::try_to_free_fee(&ALICE, 40, WithdrawReasons::TRANSACTION_PAYMENT).unwrap(), target_fee);
        assert_eq!(FeeReduction::reduction_info(&ALICE), ReductionDetail {
            own_staking: 100,
            used_fee_reduction: 38,
            used_count_reduction: 0,
            refreshed_at: 10
        });
        assert_eq!(FeeReduction::overall_reduction(), OverallReductionInfo {
            total_fee_reduction: 100,
            total_staking: 200,
            used_fee_reduction: 38,
            active_era: 10
        });
        assert_eq!(Balances::total_issuance(), 398); // 400 - 40 + 38
        assert_eq!(Balances::total_balance(&ALICE), 198);

        // Reach his own limitation
        assert_eq!(FeeReduction::try_to_free_fee(&ALICE, 40, WithdrawReasons::TRANSACTION_PAYMENT).unwrap(), target_fee);
        assert_eq!(FeeReduction::reduction_info(&ALICE), ReductionDetail {
            own_staking: 100,
            used_fee_reduction: 38,
            used_count_reduction: 0,
            refreshed_at: 10
        });
        assert_eq!(FeeReduction::overall_reduction(), OverallReductionInfo {
            total_fee_reduction: 100,
            total_staking: 200,
            used_fee_reduction: 38,
            active_era: 10
        });
        assert_eq!(Balances::total_issuance(), 358); // 398 - 40
        assert_eq!(Balances::total_balance(&ALICE), 158);
        assert_ok!(FeeReduction::cut_collateral(Origin::signed(BOB.clone()), 100));
        assert_eq!(FeeReduction::overall_reduction(), OverallReductionInfo {
            total_fee_reduction: 100,
            total_staking: 100,
            used_fee_reduction: 38,
            active_era: 10
        });

        // Since Bob cut his collateral, Alice has more limitation, it's free again
        assert_eq!(FeeReduction::try_to_free_fee(&ALICE, 40, WithdrawReasons::TRANSACTION_PAYMENT).unwrap(), target_fee);
        assert_eq!(FeeReduction::reduction_info(&ALICE), ReductionDetail {
            own_staking: 100,
            used_fee_reduction: 76,
            used_count_reduction: 0,
            refreshed_at: 10
        });
        assert_eq!(FeeReduction::overall_reduction(), OverallReductionInfo {
            total_fee_reduction: 100,
            total_staking: 100,
            used_fee_reduction: 76,
            active_era: 10
        });
        assert_eq!(Balances::total_issuance(), 356); // 358 - 2
        assert_eq!(Balances::total_balance(&ALICE), 156);

        assert_ok!(FeeReduction::add_collateral(Origin::signed(BOB.clone()), 100));
        // Bob has his own limitation, but total free fee reduction is not enough
        assert_eq!(FeeReduction::try_to_free_fee(&BOB, 40, WithdrawReasons::TRANSACTION_PAYMENT).unwrap(), target_fee);
        assert_eq!(FeeReduction::reduction_info(&BOB), ReductionDetail {
            own_staking: 100,
            used_fee_reduction: 0,
            used_count_reduction: 0,
            refreshed_at: 0
        });
        assert_eq!(FeeReduction::overall_reduction(), OverallReductionInfo {
            total_fee_reduction: 100,
            total_staking: 200,
            used_fee_reduction: 76,
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
        FeeReduction::update_overall_reduction_info(10u32.into(), 10_000);
        assert_eq!(FeeReduction::overall_reduction(), OverallReductionInfo {
            total_fee_reduction: 100,
            total_staking: 0,
            used_fee_reduction: 0,
            active_era: 10
        });
        assert_eq!(FeeReduction::try_to_free_fee(&ALICE, 40, WithdrawReasons::TRANSACTION_PAYMENT).is_err(), true);
        assert_ok!(FeeReduction::add_collateral(Origin::signed(ALICE.clone()), 25));
        assert_eq!(FeeReduction::try_to_free_fee(&ALICE, 200, WithdrawReasons::TRANSACTION_PAYMENT).is_err(), true);
    });
}
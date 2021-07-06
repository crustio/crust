// Copyright (C) 2019-2021 Crust Network Technologies Ltd.
// This file is part of Crust.

use super::*;
use crate::{mock::*, EraBenefits};
use frame_support::{assert_ok, assert_noop};
use balances::NegativeImbalance;

#[test]
fn update_overall_info_should_work() {
    new_test_ext().execute_with(|| {
        Benefits::update_era_benefit(10u32.into(), 100);
        assert_eq!(Benefits::current_benefits(), EraBenefits {
            total_fee_reduction_quota: 1,
            total_market_active_funds: 0,
            used_fee_reduction_quota: 0,
            active_era: 10
        });
    });
}

#[test]
fn add_benefit_funds_should_work() {
    new_test_ext().execute_with(|| {
        Benefits::update_era_benefit(10u32.into(), 100);
        assert_eq!(Benefits::current_benefits(), EraBenefits {
            total_fee_reduction_quota: 1,
            total_market_active_funds: 0,
            used_fee_reduction_quota: 0,
            active_era: 10
        });
        let _ = Balances::make_free_balance_be(&ALICE, 200_000);
        // add swork benefit
        assert_ok!(Benefits::add_benefit_funds(Origin::signed(ALICE.clone()), 100, FundsType::SWORK));
        assert_eq!(Benefits::swork_benefits(&ALICE), SworkBenefit {
            total_funds: 100,
            active_funds: 100,
            total_fee_reduction_count: 2,
            used_fee_reduction_count: 0,
            refreshed_at: 0,
            unlocking_funds: vec![]
        });
        assert_eq!(Benefits::market_benefits(&ALICE), MarketBenefit {
            total_funds: 0,
            active_funds: 0,
            used_fee_reduction_quota: 0,
            file_reward: 0,
            refreshed_at: 0,
            unlocking_funds: vec![]
        });
        assert_eq!(Benefits::current_benefits(), EraBenefits {
            total_fee_reduction_quota: 1,
            total_market_active_funds: 0,
            used_fee_reduction_quota: 0,
            active_era: 10
        });

        // add market benefit
        assert_ok!(Benefits::add_benefit_funds(Origin::signed(ALICE.clone()), 100, FundsType::MARKET));
        assert_eq!(Benefits::swork_benefits(&ALICE), SworkBenefit {
            total_funds: 100,
            active_funds: 100,
            total_fee_reduction_count: 2,
            used_fee_reduction_count: 0,
            refreshed_at: 0,
            unlocking_funds: vec![]
        });
        assert_eq!(Benefits::market_benefits(&ALICE), MarketBenefit {
            total_funds: 100,
            active_funds: 100,
            used_fee_reduction_quota: 0,
            file_reward: 0,
            refreshed_at: 0,
            unlocking_funds: vec![]
        });
        assert_eq!(Benefits::current_benefits(), EraBenefits {
            total_fee_reduction_quota: 1,
            total_market_active_funds: 100,
            used_fee_reduction_quota: 0,
            active_era: 10
        });
    });
}

#[test]
fn cut_benefit_funds_should_work() {
    new_test_ext().execute_with(|| {
        Benefits::update_era_benefit(10u32.into(), 100);
        let _ = Balances::make_free_balance_be(&ALICE, 200_000);

        assert_ok!(Benefits::add_benefit_funds(Origin::signed(ALICE.clone()), 100, FundsType::SWORK));
        assert_ok!(Benefits::cut_benefit_funds(Origin::signed(ALICE.clone()), 50, FundsType::SWORK));
        assert_eq!(Benefits::swork_benefits(&ALICE), SworkBenefit {
            total_funds: 100,
            active_funds: 50,
            total_fee_reduction_count: 1,
            used_fee_reduction_count: 0,
            refreshed_at: 0,
            unlocking_funds: vec![FundsUnlockChunk { value: 50, era: 12}]
        });
        assert_eq!(Benefits::market_benefits(&ALICE), MarketBenefit {
            total_funds: 0,
            active_funds: 0,
            used_fee_reduction_quota: 0,
            file_reward: 0,
            refreshed_at: 0,
            unlocking_funds: vec![]
        });
        assert_eq!(Benefits::current_benefits(), EraBenefits {
            total_fee_reduction_quota: 1,
            total_market_active_funds: 0,
            used_fee_reduction_quota: 0,
            active_era: 10
        });

        assert_ok!(Benefits::cut_benefit_funds(Origin::signed(ALICE.clone()), 50, FundsType::SWORK));
        assert_eq!(Benefits::swork_benefits(&ALICE), SworkBenefit {
            total_funds: 100,
            active_funds: 0,
            total_fee_reduction_count: 0,
            used_fee_reduction_count: 0,
            refreshed_at: 0,
            unlocking_funds: vec![FundsUnlockChunk { value: 50, era: 12}, FundsUnlockChunk { value: 50, era: 12}]
        });
        assert_eq!(Benefits::current_benefits(), EraBenefits {
            total_fee_reduction_quota: 1,
            total_market_active_funds: 0,
            used_fee_reduction_quota: 0,
            active_era: 10
        });

        assert_ok!(Benefits::add_benefit_funds(Origin::signed(ALICE.clone()), 100, FundsType::MARKET));
        assert_ok!(Benefits::cut_benefit_funds(Origin::signed(ALICE.clone()), 50, FundsType::MARKET));
        assert_eq!(Benefits::swork_benefits(&ALICE), SworkBenefit {
            total_funds: 100,
            active_funds: 0,
            total_fee_reduction_count: 0,
            used_fee_reduction_count: 0,
            refreshed_at: 0,
            unlocking_funds: vec![FundsUnlockChunk { value: 50, era: 12}, FundsUnlockChunk { value: 50, era: 12}]
        });
        assert_eq!(Benefits::market_benefits(&ALICE), MarketBenefit {
            total_funds: 100,
            active_funds: 50,
            used_fee_reduction_quota: 0,
            file_reward: 0,
            refreshed_at: 0,
            unlocking_funds: vec![FundsUnlockChunk { value: 50, era: 12}]
        });
        assert_eq!(Benefits::current_benefits(), EraBenefits {
            total_fee_reduction_quota: 1,
            total_market_active_funds: 50,
            used_fee_reduction_quota: 0,
            active_era: 10
        });

        assert_ok!(Benefits::cut_benefit_funds(Origin::signed(ALICE.clone()), 50, FundsType::MARKET));
        assert_eq!(Benefits::market_benefits(&ALICE), MarketBenefit {
            total_funds: 100,
            active_funds: 0,
            used_fee_reduction_quota: 0,
            file_reward: 0,
            refreshed_at: 0,
            unlocking_funds: vec![FundsUnlockChunk { value: 50, era: 12}, FundsUnlockChunk { value: 50, era: 12}]
        });
        assert_eq!(Benefits::current_benefits(), EraBenefits {
            total_fee_reduction_quota: 1,
            total_market_active_funds: 0,
            used_fee_reduction_quota: 0,
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
        assert_eq!(Benefits::swork_benefits(&ALICE), SworkBenefit {
            total_funds: 0,
            active_funds: 0,
            total_fee_reduction_count: 0,
            used_fee_reduction_count: 0,
            refreshed_at: 0,
            unlocking_funds: vec![]
        });
        assert_eq!(Benefits::current_benefits(), EraBenefits {
            total_fee_reduction_quota: 1,
            total_market_active_funds: 0,
            used_fee_reduction_quota: 0,
            active_era: 10
        });
        let _ = Balances::make_free_balance_be(&ALICE, 200_000);
        assert_ok!(Benefits::add_benefit_funds(Origin::signed(ALICE.clone()), 105, FundsType::SWORK));
        assert_eq!(Benefits::maybe_free_count(&ALICE), true);
        assert_eq!(Benefits::swork_benefits(&ALICE), SworkBenefit {
            total_funds: 105,
            active_funds: 105,
            total_fee_reduction_count: 2,
            used_fee_reduction_count: 1,
            refreshed_at: 10,
            unlocking_funds: vec![]
        });
        assert_eq!(Benefits::maybe_free_count(&ALICE), true);
        assert_eq!(Benefits::swork_benefits(&ALICE), SworkBenefit {
            total_funds: 105,
            active_funds: 105,
            total_fee_reduction_count: 2,
            used_fee_reduction_count: 2,
            refreshed_at: 10,
            unlocking_funds: vec![]
        });
        // Reach the limitation => cannot free this operation
        assert_eq!(Benefits::maybe_free_count(&ALICE), false);
        assert_eq!(Benefits::swork_benefits(&ALICE), SworkBenefit {
            total_funds: 105,
            active_funds: 105,
            total_fee_reduction_count: 2,
            used_fee_reduction_count: 2,
            refreshed_at: 10,
            unlocking_funds: vec![]
        });
        // New era => refresh the limitation
        Benefits::update_era_benefit(11u32.into(), 100);
        assert_eq!(Benefits::maybe_free_count(&ALICE), true);
        assert_eq!(Benefits::swork_benefits(&ALICE), SworkBenefit {
            total_funds: 105,
            active_funds: 105,
            total_fee_reduction_count: 2,
            used_fee_reduction_count: 1,
            refreshed_at: 11,
            unlocking_funds: vec![]
        });
        assert_eq!(Benefits::maybe_free_count(&ALICE), true);
        assert_eq!(Benefits::swork_benefits(&ALICE), SworkBenefit {
            total_funds: 105,
            active_funds: 105,
            total_fee_reduction_count: 2,
            used_fee_reduction_count: 2,
            refreshed_at: 11,
            unlocking_funds: vec![]
        });
        // Reach the limitation => cannot free this operation
        assert_eq!(Benefits::maybe_free_count(&ALICE), false);
        assert_eq!(Benefits::swork_benefits(&ALICE), SworkBenefit {
            total_funds: 105,
            active_funds: 105,
            total_fee_reduction_count: 2,
            used_fee_reduction_count: 2,
            refreshed_at: 11,
            unlocking_funds: vec![]
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
            total_fee_reduction_quota: 100,
            total_market_active_funds: 0,
            used_fee_reduction_quota: 0,
            active_era: 10
        });
        assert_eq!(Benefits::maybe_reduce_fee(&ALICE, 20, WithdrawReasons::TRANSACTION_PAYMENT).unwrap(), target_fee);
        // won't update reduction detail since it has not staking
        assert_eq!(Benefits::market_benefits(&ALICE), MarketBenefit {
            total_funds: 0,
            active_funds: 0,
            used_fee_reduction_quota: 0,
            file_reward: 0,
            refreshed_at: 0,
            unlocking_funds: vec![]
        });
        assert_eq!(Balances::total_balance(&ALICE), 180);
        assert_eq!(Balances::total_issuance(), 180);
        assert_ok!(Benefits::add_benefit_funds(Origin::signed(ALICE.clone()), 105, FundsType::MARKET));
        assert_eq!(Benefits::maybe_reduce_fee(&ALICE, 20, WithdrawReasons::TRANSACTION_PAYMENT).unwrap(), target_fee);
        assert_eq!(Benefits::market_benefits(&ALICE), MarketBenefit {
            total_funds: 105,
            active_funds: 105,
            used_fee_reduction_quota: 19,
            file_reward: 0,
            refreshed_at: 10,
            unlocking_funds: vec![]
        });
        assert_eq!(Benefits::current_benefits(), EraBenefits {
            total_fee_reduction_quota: 100,
            total_market_active_funds: 105,
            used_fee_reduction_quota: 19,
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
        assert_ok!(Benefits::add_benefit_funds(Origin::signed(ALICE.clone()), 105, FundsType::MARKET));
        assert_eq!(Benefits::maybe_reduce_fee(&ALICE, 20, WithdrawReasons::TRANSACTION_PAYMENT).unwrap(), target_fee);
        assert_eq!(Benefits::market_benefits(&ALICE), MarketBenefit {
            total_funds: 105,
            active_funds: 105,
            used_fee_reduction_quota: 19,
            file_reward: 0,
            refreshed_at: 10,
            unlocking_funds: vec![]
        });
        assert_eq!(Benefits::current_benefits(), EraBenefits {
            total_fee_reduction_quota: 100,
            total_market_active_funds: 105,
            used_fee_reduction_quota: 19,
            active_era: 10
        });
        assert_eq!(Benefits::update_era_benefit(11u32.into(), 10_000), 19);
        assert_eq!(Benefits::current_benefits(), EraBenefits {
            total_fee_reduction_quota: 100,
            total_market_active_funds: 105,
            used_fee_reduction_quota: 0,
            active_era: 11
        });
    });
}

#[test]
fn free_fee_limit_should_work() {
    new_test_ext().execute_with(|| {
        let _ = Balances::make_free_balance_be(&ALICE, 200);
        let _ = Balances::make_free_balance_be(&BOB, 300);
        let target_fee = NegativeImbalance::new(40);
        assert_eq!(Balances::total_issuance(), 500);
        Benefits::update_era_benefit(10u32.into(), 10_000);
        assert_eq!(Benefits::current_benefits(), EraBenefits {
            total_fee_reduction_quota: 100,
            total_market_active_funds: 0,
            used_fee_reduction_quota: 0,
            active_era: 10
        });
        assert_ok!(Benefits::add_benefit_funds(Origin::signed(ALICE.clone()), 100, FundsType::MARKET));
        assert_ok!(Benefits::add_benefit_funds(Origin::signed(BOB.clone()), 100, FundsType::MARKET));
        assert_eq!(Benefits::maybe_reduce_fee(&ALICE, 40, WithdrawReasons::TRANSACTION_PAYMENT).unwrap(), target_fee);
        assert_eq!(Benefits::market_benefits(&ALICE), MarketBenefit {
            total_funds: 100,
            active_funds: 100,
            used_fee_reduction_quota: 38,
            file_reward: 0,
            refreshed_at: 10,
            unlocking_funds: vec![]
        });
        assert_eq!(Benefits::current_benefits(), EraBenefits {
            total_fee_reduction_quota: 100,
            total_market_active_funds: 200,
            used_fee_reduction_quota: 38,
            active_era: 10
        });
        assert_eq!(Balances::total_issuance(), 498); // 500 - 40 + 38
        assert_eq!(Balances::total_balance(&ALICE), 198);

        // Reach his own limitation
        assert_eq!(Benefits::maybe_reduce_fee(&ALICE, 40, WithdrawReasons::TRANSACTION_PAYMENT).unwrap(), target_fee);
        assert_eq!(Benefits::market_benefits(&ALICE), MarketBenefit {
            total_funds: 100,
            active_funds: 100,
            used_fee_reduction_quota: 38,
            file_reward: 0,
            refreshed_at: 10,
            unlocking_funds: vec![]
        });
        assert_eq!(Benefits::current_benefits(), EraBenefits {
            total_fee_reduction_quota: 100,
            total_market_active_funds: 200,
            used_fee_reduction_quota: 38,
            active_era: 10
        });
        assert_eq!(Balances::total_issuance(), 458); // 498 - 40
        assert_eq!(Balances::total_balance(&ALICE), 158);
        assert_ok!(Benefits::cut_benefit_funds(Origin::signed(BOB.clone()), 100, FundsType::MARKET));
        assert_eq!(Benefits::current_benefits(), EraBenefits {
            total_fee_reduction_quota: 100,
            total_market_active_funds: 100,
            used_fee_reduction_quota: 38,
            active_era: 10
        });

        // Since Bob cut his collateral, Alice has more limitation, it's free again
        assert_eq!(Benefits::maybe_reduce_fee(&ALICE, 40, WithdrawReasons::TRANSACTION_PAYMENT).unwrap(), target_fee);
        assert_eq!(Benefits::market_benefits(&ALICE), MarketBenefit {
            total_funds: 100,
            active_funds: 100,
            used_fee_reduction_quota: 76,
            file_reward: 0,
            refreshed_at: 10,
            unlocking_funds: vec![]
        });
        assert_eq!(Benefits::current_benefits(), EraBenefits {
            total_fee_reduction_quota: 100,
            total_market_active_funds: 100,
            used_fee_reduction_quota: 76,
            active_era: 10
        });
        assert_eq!(Balances::total_issuance(), 456); // 458 - 2
        assert_eq!(Balances::total_balance(&ALICE), 156);

        assert_ok!(Benefits::add_benefit_funds(Origin::signed(BOB.clone()), 100, FundsType::MARKET));
        // Bob has his own limitation, but total free fee reduction is not enough
        assert_eq!(Benefits::maybe_reduce_fee(&BOB, 40, WithdrawReasons::TRANSACTION_PAYMENT).unwrap(), target_fee);
        assert_eq!(Benefits::market_benefits(&BOB), MarketBenefit {
            total_funds: 200,
            active_funds: 100,
            used_fee_reduction_quota: 0,
            file_reward: 0,
            refreshed_at: 0,
            unlocking_funds: vec![FundsUnlockChunk { value: 100, era: 12}]
        });
        assert_eq!(Benefits::current_benefits(), EraBenefits {
            total_fee_reduction_quota: 100,
            total_market_active_funds: 200,
            used_fee_reduction_quota: 76,
            active_era: 10
        });
        assert_eq!(Balances::total_issuance(), 416); // 458 - 40
        assert_eq!(Balances::total_balance(&BOB), 260);

    });
}

#[test]
fn currency_is_insufficient() {
    new_test_ext().execute_with(|| {
        let _ = Balances::make_free_balance_be(&ALICE, 30);
        assert_eq!(Balances::total_issuance(), 30);
        Benefits::update_era_benefit(10u32.into(), 10_000);
        assert_eq!(Benefits::current_benefits(), EraBenefits {
            total_fee_reduction_quota: 100,
            total_market_active_funds: 0,
            used_fee_reduction_quota: 0,
            active_era: 10
        });
        assert_eq!(Benefits::maybe_reduce_fee(&ALICE, 40, WithdrawReasons::TRANSACTION_PAYMENT).is_err(), true);
        assert_ok!(Benefits::add_benefit_funds(Origin::signed(ALICE.clone()), 25, FundsType::MARKET));
        assert_eq!(Benefits::maybe_reduce_fee(&ALICE, 200, WithdrawReasons::TRANSACTION_PAYMENT).is_err(), true);
    });
}

#[test]
fn cut_benefits_funds_should_exceed_max_chunks_limit() {
    new_test_ext().execute_with(|| {
        Benefits::update_era_benefit(10u32.into(), 100);
        assert_eq!(Benefits::current_benefits(), EraBenefits {
            total_fee_reduction_quota: 1,
            total_market_active_funds: 0,
            used_fee_reduction_quota: 0,
            active_era: 10
        });
        let _ = Balances::make_free_balance_be(&ALICE, 200);
        assert_ok!(Benefits::add_benefit_funds(Origin::signed(ALICE.clone()), 100, FundsType::MARKET));

        for index in 0..MAX_UNLOCKING_CHUNKS {
            assert_ok!(Benefits::cut_benefit_funds(Origin::signed(ALICE.clone()), 5, FundsType::MARKET));
            assert_eq!(Benefits::market_benefits(&ALICE).active_funds, 95 - index as u64 * 5);
            assert_eq!(Benefits::market_benefits(&ALICE).unlocking_funds.len(), index + 1);
            assert_eq!(Benefits::current_benefits(), EraBenefits {
                total_fee_reduction_quota: 1,
                total_market_active_funds: 95 - index as u64 * 5,
                used_fee_reduction_quota: 0,
                active_era: 10
            });
        }

        assert_noop!(Benefits::cut_benefit_funds(Origin::signed(ALICE.clone()), 5, FundsType::MARKET),
        DispatchError::Module {
            index: 2,
            error: 2,
            message: Some("NoMoreChunks")
        });
    });
}

#[test]
fn withdraw_benefits_funds_should_work() {
    new_test_ext().execute_with(|| {
        Benefits::update_era_benefit(10u32.into(), 100);
        assert_eq!(Benefits::current_benefits(), EraBenefits {
            total_fee_reduction_quota: 1,
            total_market_active_funds: 0,
            used_fee_reduction_quota: 0,
            active_era: 10
        });
        let _ = Balances::make_free_balance_be(&ALICE, 200);
        assert_ok!(Benefits::add_benefit_funds(Origin::signed(ALICE.clone()), 100, FundsType::MARKET));
        assert_ok!(Benefits::cut_benefit_funds(Origin::signed(ALICE.clone()), 15, FundsType::MARKET));
        Benefits::update_era_benefit(11u32.into(), 100);
        assert_ok!(Benefits::cut_benefit_funds(Origin::signed(ALICE.clone()), 10, FundsType::MARKET));
        Benefits::update_era_benefit(12u32.into(), 100);
        assert_ok!(Benefits::cut_benefit_funds(Origin::signed(ALICE.clone()), 5, FundsType::MARKET));
        Benefits::update_era_benefit(13u32.into(), 100);
        assert_ok!(Benefits::withdraw_benefit_funds(Origin::signed(ALICE.clone())));
        assert_eq!(Benefits::market_benefits(&ALICE), MarketBenefit {
            total_funds: 75,
            active_funds: 70,
            used_fee_reduction_quota: 0,
            file_reward: 0,
            refreshed_at: 0,
            unlocking_funds: vec![FundsUnlockChunk { value: 5, era: 14}]
        });
        assert_ok!(Benefits::cut_benefit_funds(Origin::signed(ALICE.clone()), 15, FundsType::MARKET));
        Benefits::update_era_benefit(14u32.into(), 100);
        assert_ok!(Benefits::withdraw_benefit_funds(Origin::signed(ALICE.clone())));
        assert_eq!(Benefits::market_benefits(&ALICE), MarketBenefit {
            total_funds: 70,
            active_funds: 55,
            used_fee_reduction_quota: 0,
            file_reward: 0,
            refreshed_at: 0,
            unlocking_funds: vec![FundsUnlockChunk { value: 15, era: 15}]
        });
        Benefits::update_era_benefit(15u32.into(), 100);
        assert_ok!(Benefits::withdraw_benefit_funds(Origin::signed(ALICE.clone())));
        assert_eq!(Benefits::market_benefits(&ALICE), MarketBenefit {
            total_funds: 55,
            active_funds: 55,
            used_fee_reduction_quota: 0,
            file_reward: 0,
            refreshed_at: 0,
            unlocking_funds: vec![]
        });

        assert_ok!(Benefits::cut_benefit_funds(Origin::signed(ALICE.clone()), 55, FundsType::MARKET));
        Benefits::update_era_benefit(17u32.into(), 100);
        Benefits::update_reward(&ALICE, 100);
        assert_ok!(Benefits::withdraw_benefit_funds(Origin::signed(ALICE.clone())));
        assert_eq!(Benefits::market_benefits(&ALICE), MarketBenefit {
            total_funds: 0,
            active_funds: 0,
            used_fee_reduction_quota: 0,
            file_reward: 100,
            refreshed_at: 0,
            unlocking_funds: vec![]
        });
        Benefits::update_reward(&ALICE, 0);
        assert_eq!(<MarketBenefits<Test>>::contains_key(&ALICE), false);
    });
}

#[test]
fn rebond_market_benefits_funds_should_work() {
    new_test_ext().execute_with(|| {
        Benefits::update_era_benefit(10u32.into(), 100);
        assert_eq!(Benefits::current_benefits(), EraBenefits {
            total_fee_reduction_quota: 1,
            total_market_active_funds: 0,
            used_fee_reduction_quota: 0,
            active_era: 10
        });
        let _ = Balances::make_free_balance_be(&ALICE, 200);
        assert_ok!(Benefits::add_benefit_funds(Origin::signed(ALICE.clone()), 100, FundsType::MARKET));
        assert_ok!(Benefits::cut_benefit_funds(Origin::signed(ALICE.clone()), 15, FundsType::MARKET));
        Benefits::update_era_benefit(11u32.into(), 100);
        assert_ok!(Benefits::cut_benefit_funds(Origin::signed(ALICE.clone()), 10, FundsType::MARKET));
        Benefits::update_era_benefit(12u32.into(), 100);
        assert_ok!(Benefits::cut_benefit_funds(Origin::signed(ALICE.clone()), 5, FundsType::MARKET));
        Benefits::update_era_benefit(13u32.into(), 100);
        assert_ok!(Benefits::cut_benefit_funds(Origin::signed(ALICE.clone()), 15, FundsType::MARKET));
        Benefits::update_era_benefit(14u32.into(), 100);

        assert_ok!(Benefits::rebond_benefit_funds(Origin::signed(ALICE.clone()), 10, FundsType::MARKET));
        assert_eq!(Benefits::market_benefits(&ALICE), MarketBenefit {
            total_funds: 100,
            active_funds: 65,
            used_fee_reduction_quota: 0,
            file_reward: 0,
            refreshed_at: 0,
            unlocking_funds: vec![FundsUnlockChunk { value: 15, era: 12},
                                  FundsUnlockChunk { value: 10, era: 13},
                                  FundsUnlockChunk { value: 5, era: 14},
                                  FundsUnlockChunk { value: 5, era: 15}]
        });

        assert_eq!(Benefits::current_benefits(), EraBenefits {
            total_fee_reduction_quota: 1,
            total_market_active_funds: 65,
            used_fee_reduction_quota: 0,
            active_era: 14
        });
        assert_ok!(Benefits::rebond_benefit_funds(Origin::signed(ALICE.clone()), 10, FundsType::MARKET));
        assert_eq!(Benefits::market_benefits(&ALICE), MarketBenefit {
            total_funds: 100,
            active_funds: 75,
            used_fee_reduction_quota: 0,
            file_reward: 0,
            refreshed_at: 0,
            unlocking_funds: vec![FundsUnlockChunk { value: 15, era: 12},
                                  FundsUnlockChunk { value: 10, era: 13}]
        });

        assert_eq!(Benefits::current_benefits(), EraBenefits {
            total_fee_reduction_quota: 1,
            total_market_active_funds: 75,
            used_fee_reduction_quota: 0,
            active_era: 14
        });

        assert_ok!(Benefits::rebond_benefit_funds(Origin::signed(ALICE.clone()), 100, FundsType::MARKET));
        assert_eq!(Benefits::market_benefits(&ALICE), MarketBenefit {
            total_funds: 100,
            active_funds: 100,
            used_fee_reduction_quota: 0,
            file_reward: 0,
            refreshed_at: 0,
            unlocking_funds: vec![]
        });

        assert_eq!(Benefits::current_benefits(), EraBenefits {
            total_fee_reduction_quota: 1,
            total_market_active_funds: 100,
            used_fee_reduction_quota: 0,
            active_era: 14
        });
    });
}

#[test]
fn rebond_swork_benefits_funds_should_work() {
    new_test_ext().execute_with(|| {
        Benefits::update_era_benefit(10u32.into(), 100);
        assert_eq!(Benefits::current_benefits(), EraBenefits {
            total_fee_reduction_quota: 1,
            total_market_active_funds: 0,
            used_fee_reduction_quota: 0,
            active_era: 10
        });
        let _ = Balances::make_free_balance_be(&ALICE, 200);
        assert_ok!(Benefits::add_benefit_funds(Origin::signed(ALICE.clone()), 100, FundsType::SWORK));
        assert_ok!(Benefits::cut_benefit_funds(Origin::signed(ALICE.clone()), 15, FundsType::SWORK));
        Benefits::update_era_benefit(11u32.into(), 100);
        assert_ok!(Benefits::cut_benefit_funds(Origin::signed(ALICE.clone()), 10, FundsType::SWORK));
        Benefits::update_era_benefit(12u32.into(), 100);
        assert_ok!(Benefits::cut_benefit_funds(Origin::signed(ALICE.clone()), 5, FundsType::SWORK));
        Benefits::update_era_benefit(13u32.into(), 100);
        assert_ok!(Benefits::cut_benefit_funds(Origin::signed(ALICE.clone()), 15, FundsType::SWORK));
        Benefits::update_era_benefit(14u32.into(), 100);

        assert_ok!(Benefits::rebond_benefit_funds(Origin::signed(ALICE.clone()), 10, FundsType::SWORK));
        assert_eq!(Benefits::swork_benefits(&ALICE), SworkBenefit {
            total_funds: 100,
            active_funds: 65,
            total_fee_reduction_count: 1,
            used_fee_reduction_count: 0,
            refreshed_at: 0,
            unlocking_funds: vec![FundsUnlockChunk { value: 15, era: 12},
                                  FundsUnlockChunk { value: 10, era: 13},
                                  FundsUnlockChunk { value: 5, era: 14},
                                  FundsUnlockChunk { value: 5, era: 15}]
        });

        assert_ok!(Benefits::rebond_benefit_funds(Origin::signed(ALICE.clone()), 10, FundsType::SWORK));
        assert_eq!(Benefits::swork_benefits(&ALICE), SworkBenefit {
            total_funds: 100,
            active_funds: 75,
            total_fee_reduction_count: 1,
            used_fee_reduction_count: 0,
            refreshed_at: 0,
            unlocking_funds: vec![FundsUnlockChunk { value: 15, era: 12},
                                  FundsUnlockChunk { value: 10, era: 13}]
        });

        assert_ok!(Benefits::rebond_benefit_funds(Origin::signed(ALICE.clone()), 100, FundsType::SWORK));
        assert_eq!(Benefits::swork_benefits(&ALICE), SworkBenefit {
            total_funds: 100,
            active_funds: 100,
            total_fee_reduction_count: 2,
            used_fee_reduction_count: 0,
            refreshed_at: 0,
            unlocking_funds: vec![]
        });
    });
}

#[test]
fn withdraw_market_benefits_funds_in_weird_situation() {
    new_test_ext().execute_with(|| {
        Benefits::update_era_benefit(10u32.into(), 100);
        assert_eq!(Benefits::current_benefits(), EraBenefits {
            total_fee_reduction_quota: 1,
            total_market_active_funds: 0,
            used_fee_reduction_quota: 0,
            active_era: 10
        });
        let _ = Balances::make_free_balance_be(&ALICE, 200);
        assert_ok!(Benefits::add_benefit_funds(Origin::signed(ALICE.clone()), 100, FundsType::MARKET));
        assert_eq!(Benefits::market_benefits(&ALICE), MarketBenefit {
            total_funds: 100,
            active_funds: 100,
            used_fee_reduction_quota: 0,
            file_reward: 0,
            refreshed_at: 0,
            unlocking_funds: vec![]
        });
        assert_eq!(Benefits::current_benefits(), EraBenefits {
            total_fee_reduction_quota: 1,
            total_market_active_funds: 100,
            used_fee_reduction_quota: 0,
            active_era: 10
        });
        assert_ok!(Benefits::cut_benefit_funds(Origin::signed(ALICE.clone()), 200, FundsType::MARKET));
        assert_eq!(Benefits::market_benefits(&ALICE), MarketBenefit {
            total_funds: 100,
            active_funds: 0,
            used_fee_reduction_quota: 0,
            file_reward: 0,
            refreshed_at: 0,
            unlocking_funds: vec![FundsUnlockChunk { value: 100, era: 12}]
        });
        assert_eq!(Benefits::current_benefits(), EraBenefits {
            total_fee_reduction_quota: 1,
            total_market_active_funds: 0,
            used_fee_reduction_quota: 0,
            active_era: 10
        });

        // Slash this account with 150 and reserved should be 50
        let _ = Balances::slash(&ALICE, 150);
        assert_eq!(Balances::free_balance(&ALICE), 0);
        assert_eq!(Balances::reserved_balance(&ALICE), 50);

        Benefits::update_era_benefit(13u32.into(), 100);
        assert_ok!(Benefits::withdraw_benefit_funds(Origin::signed(ALICE.clone())));
        assert_eq!(<MarketBenefits<Test>>::contains_key(&ALICE), false);
        assert_eq!(Benefits::current_benefits(), EraBenefits {
            total_fee_reduction_quota: 1,
            total_market_active_funds: 0,
            used_fee_reduction_quota: 0,
            active_era: 13
        });

        let _ = Balances::make_free_balance_be(&ALICE, 200);
        assert_ok!(Benefits::add_benefit_funds(Origin::signed(ALICE.clone()), 100, FundsType::MARKET));
        let _ = Balances::slash(&ALICE, 150);
        assert_eq!(Balances::free_balance(&ALICE), 0);
        assert_eq!(Balances::reserved_balance(&ALICE), 50);
        assert_ok!(Benefits::withdraw_benefit_funds(Origin::signed(ALICE.clone())));
        assert_eq!(Benefits::market_benefits(&ALICE), MarketBenefit {
            total_funds: 50,
            active_funds: 50,
            used_fee_reduction_quota: 0,
            file_reward: 0,
            refreshed_at: 0,
            unlocking_funds: vec![]
        });
        assert_eq!(Benefits::current_benefits(), EraBenefits {
            total_fee_reduction_quota: 1,
            total_market_active_funds: 50,
            used_fee_reduction_quota: 0,
            active_era: 13
        });
    });
}

#[test]
fn withdraw_swork_benefits_funds_in_weird_situation() {
    new_test_ext().execute_with(|| {
        Benefits::update_era_benefit(10u32.into(), 100);
        assert_eq!(Benefits::current_benefits(), EraBenefits {
            total_fee_reduction_quota: 1,
            total_market_active_funds: 0,
            used_fee_reduction_quota: 0,
            active_era: 10
        });
        let _ = Balances::make_free_balance_be(&ALICE, 200);
        assert_ok!(Benefits::add_benefit_funds(Origin::signed(ALICE.clone()), 100, FundsType::SWORK));
        assert_eq!(Benefits::swork_benefits(&ALICE), SworkBenefit {
            total_funds: 100,
            active_funds: 100,
            total_fee_reduction_count: 2,
            used_fee_reduction_count: 0,
            refreshed_at: 0,
            unlocking_funds: vec![]
        });
        assert_ok!(Benefits::cut_benefit_funds(Origin::signed(ALICE.clone()), 200, FundsType::SWORK));
        assert_eq!(Benefits::swork_benefits(&ALICE), SworkBenefit {
            total_funds: 100,
            active_funds: 0,
            total_fee_reduction_count: 0,
            used_fee_reduction_count: 0,
            refreshed_at: 0,
            unlocking_funds: vec![FundsUnlockChunk { value: 100, era: 12}]
        });

        // Slash this account with 150 and reserved should be 50
        let _ = Balances::slash(&ALICE, 150);
        assert_eq!(Balances::free_balance(&ALICE), 0);
        assert_eq!(Balances::reserved_balance(&ALICE), 50);

        Benefits::update_era_benefit(13u32.into(), 100);
        assert_ok!(Benefits::withdraw_benefit_funds(Origin::signed(ALICE.clone())));
        assert_eq!(<SworkBenefits<Test>>::contains_key(&ALICE), false);
        assert_eq!(Benefits::current_benefits(), EraBenefits {
            total_fee_reduction_quota: 1,
            total_market_active_funds: 0,
            used_fee_reduction_quota: 0,
            active_era: 13
        });

        let _ = Balances::make_free_balance_be(&ALICE, 200);
        assert_ok!(Benefits::add_benefit_funds(Origin::signed(ALICE.clone()), 100, FundsType::SWORK));
        let _ = Balances::slash(&ALICE, 150);
        assert_eq!(Balances::free_balance(&ALICE), 0);
        assert_eq!(Balances::reserved_balance(&ALICE), 50);
        assert_ok!(Benefits::withdraw_benefit_funds(Origin::signed(ALICE.clone())));
        assert_eq!(Benefits::swork_benefits(&ALICE), SworkBenefit {
            total_funds: 50,
            active_funds: 50,
            total_fee_reduction_count: 1,
            used_fee_reduction_count: 0,
            refreshed_at: 0,
            unlocking_funds: vec![]
        });
    });
}

#[test]
fn test_collateral_and_reward_interface() {
    new_test_ext().execute_with(|| {
        let _ = Balances::make_free_balance_be(&ALICE, 200);
        assert_ok!(Benefits::add_benefit_funds(Origin::signed(ALICE.clone()), 100, FundsType::MARKET));
        assert_eq!(Benefits::market_benefits(&ALICE), MarketBenefit {
            total_funds: 100,
            active_funds: 100,
            used_fee_reduction_quota: 0,
            file_reward: 0,
            refreshed_at: 0,
            unlocking_funds: vec![]
        });
        Benefits::update_reward(&ALICE, 100);
        assert_eq!(Benefits::market_benefits(&ALICE), MarketBenefit {
            total_funds: 100,
            active_funds: 100,
            used_fee_reduction_quota: 0,
            file_reward: 100,
            refreshed_at: 0,
            unlocking_funds: vec![]
        });
        assert_eq!(Benefits::get_collateral_and_reward(&ALICE), (100, 100));
    });
}
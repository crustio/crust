//! Tests for the module.

use super::*;
use crate::mock::*;
use frame_support::{
    assert_noop, assert_ok,
    dispatch::DispatchError,
    traits::{Currency, ReservableCurrency, OnInitialize},
};
use sp_runtime::{
    assert_eq_error_rate,
    traits::BadOrigin,
};
use sp_staking::offence::OffenceDetails;
use substrate_test_utils::assert_eq_uvec;

// TODO: new validate test case cannot pass because of stake limit, fix with better genesis initial

#[test]
fn force_unstake_works() {
    // Verifies initial conditions of mock
    ExtBuilder::default().build().execute_with(|| {
        // Account 11 is stashed and locked, and account 10 is the controller
        assert_eq!(Staking::bonded(&11), Some(10));
        // Cant transfer
        assert_noop!(
            Balances::transfer(Origin::signed(11), 1, 10),
            DispatchError::Module {
                index: 0,
                error: 1,
                message: Some("LiquidityRestrictions"),
            }
        );
        // Force unstake requires root.
        assert_noop!(Staking::force_unstake(Origin::signed(11), 11), BadOrigin);
        // We now force them to unstake
        assert_ok!(Staking::force_unstake(Origin::root(), 11));
        // No longer bonded.
        assert_eq!(Staking::bonded(&11), None);
        // Transfer works.
        assert_ok!(Balances::transfer(Origin::signed(11), 1, 10));
    });
}

#[test]
fn basic_setup_works() {
    // Verifies initial conditions of mock
    ExtBuilder::default().build().execute_with(|| {
        // Account 11 is stashed and locked, and account 10 is the controller
        assert_eq!(Staking::bonded(&11), Some(10));
        // Account 21 is stashed and locked, and account 20 is the controller
        assert_eq!(Staking::bonded(&21), Some(20));
        // Account 1 is not a stashed
        assert_eq!(Staking::bonded(&1), None);

        // Account 10 controls the stash from account 11, which is 100 * balance_factor units
        assert_eq!(
            Staking::ledger(&10),
            Some(StakingLedger {
                stash: 11,
                total: 1000,
                active: 1000,
                valid: 1000,
                unlocking: vec![],
                claimed_rewards:vec![],
            })
        );
        // Account 20 controls the stash from account 21, which is 200 * balance_factor units
        assert_eq!(
            Staking::ledger(&20),
            Some(StakingLedger {
                stash: 21,
                total: 1000,
                active: 1000,
                valid: 1000,
                unlocking: vec![],
                claimed_rewards:vec![],
            })
        );
        // Account 1 does not control any stash
        assert_eq!(Staking::ledger(&1), None);

        // Validations are default
        assert_eq!(
            <Validators<Test>>::iter().collect::<Vec<_>>(),
            vec![
                (31, Validations::default()),
                (
                    21,
                    Validations {
                        total: 250,
                        guarantee_fee: Perbill::one(),
                        guarantors: vec![101]
                    }
                ),
                (
                    11,
                    Validations {
                        total: 250,
                        guarantee_fee: Perbill::one(),
                        guarantors: vec![101]
                    }
                )
            ]
        );

        assert_eq!(
            Staking::ledger(100),
            Some(StakingLedger {
                stash: 101,
                total: 500,
                active: 500,
                valid: 500,
                unlocking: vec![],
                claimed_rewards:vec![],
            })
        );
        assert_eq!(Staking::guarantors(101).unwrap().targets, vec![11, 21]);
        assert_eq!(Staking::guarantee_rel(101, 11).get(&(0 as u32)), Some(&(250 as Balance)));
        assert_eq!(Staking::guarantee_rel(101, 21).get(&(0 as u32)), Some(&(250 as Balance)));

        if cfg!(feature = "equalize") {
            assert_eq!(
                Staking::stakers(11),
                Exposure {
                    total: 1250,
                    own: 1000,
                    others: vec![IndividualExposure {
                        who: 101,
                        value: 250
                    }]
                }
            );
            assert_eq!(
                Staking::stakers(21),
                Exposure {
                    total: 1250,
                    own: 1000,
                    others: vec![IndividualExposure {
                        who: 101,
                        value: 250
                    }]
                }
            );
            // initial total_stakes = 1250+1250+1
            assert_eq!(Staking::eras_total_stakes(0), 2501);
        } else {
            assert_eq!(
                Staking::stakers(11),
                Exposure {
                    total: 1125,
                    own: 1000,
                    others: vec![IndividualExposure {
                        who: 101,
                        value: 125
                    }]
                }
            );
            assert_eq!(
                Staking::stakers(21),
                Exposure {
                    total: 1375,
                    own: 1000,
                    others: vec![IndividualExposure {
                        who: 101,
                        value: 375
                    }]
                }
            );
            // initial total_stakes = 1125+1375+1
            assert_eq!(Staking::eras_total_stakes(0), 2501);
        }

        // The number of validators required.
        assert_eq!(Staking::validator_count(), 2);

        // Initial Era and session
        assert_eq!(Staking::current_era().unwrap_or(0), 0);

        // Account 10 has `balance_factor` free balance
        assert_eq!(Balances::free_balance(&10), 1);
        assert_eq!(Balances::free_balance(&10), 1);

        // New era is not being forced
        assert_eq!(Staking::force_era(), Forcing::NotForcing);

        // All exposures must be correct.
        check_exposure_all();
        check_guarantor_all();
    });
}

#[test]
fn change_controller_works() {
    ExtBuilder::default().build().execute_with(|| {
        assert_eq!(Staking::bonded(&11), Some(10));

        assert!(<Validators<Test>>::iter()
            .map(|(c, _)| c)
            .collect::<Vec<u64>>()
            .contains(&11));
        // 10 can control 11 who is initially a validator.
        assert_ok!(Staking::chill(Origin::signed(10)));
        assert!(!<Validators<Test>>::iter()
            .map(|(c, _)| c)
            .collect::<Vec<u64>>()
            .contains(&11));

        assert_ok!(Staking::set_controller(Origin::signed(11), 5));

        start_era(1, false);

        assert_noop!(
            Staking::validate(Origin::signed(10), Perbill::default()),
            Error::<Test>::NotController,
        );
        assert_ok!(Staking::validate(Origin::signed(5), Perbill::default()));
    })
}

#[test]
fn rewards_should_work() {
    // should check that:
    // * rewards get recorded per session
    // * rewards get paid per Era
    // * Check that guarantors are also rewarded
    ExtBuilder::default()
        .guarantee(false)
        .build()
        .execute_with(|| {
            // Init some balances
            let _ = Balances::make_free_balance_be(&2, 500);

            let delay = 1000;
            let init_balance_2 = Balances::total_balance(&2);
            let init_balance_10 = Balances::total_balance(&10);
            let init_balance_11 = Balances::total_balance(&11);

            // Set payee to controller
            assert_ok!(Staking::set_payee(
                Origin::signed(10),
                RewardDestination::Controller
            ));

            // Initial config should be correct
            assert_eq!(Staking::current_era().unwrap_or(0), 0);
            assert_eq!(Session::current_index(), 0);

            // Add a dummy guarantor.
            //
            // Equal division indicates that the reward will be equally divided among validator and
            // guarantor.
            <ErasStakers<Test>>::insert(
                0,
                &11,
                Exposure {
                    own: 500,
                    total: 1000,
                    others: vec![IndividualExposure { who: 2, value: 500 }],
                },
            );

            <Payee<Test>>::insert(&2, RewardDestination::Stash);
            assert_eq!(Staking::payee(2), RewardDestination::Stash);
            assert_eq!(Staking::payee(11), RewardDestination::Controller);

            let mut block = 3; // Block 3 => Session 1 => Era 0
            System::set_block_number(block);
            Timestamp::set_timestamp(block * 5000); // on time.
            Session::on_initialize(System::block_number());
            assert_eq!(Staking::current_era().unwrap_or(0), 0);
            assert_eq!(Session::current_index(), 1);
            <Module<Test>>::reward_by_ids(vec![(11, 50)]);
            <Module<Test>>::reward_by_ids(vec![(11, 50)]);
            // This is the second validator of the current elected set.
            <Module<Test>>::reward_by_ids(vec![(21, 50)]);
            // This must be no-op as it is not an elected validator.
            <Module<Test>>::reward_by_ids(vec![(1001, 10_000)]);

            // Compute total payout now for whole duration as other parameter won't change
            let staking_reward = Staking::staking_rewards_in_era(Staking::current_era().unwrap_or(0));
            let authoring_reward = Staking::authoring_rewards_in_era();
            assert_eq!(Staking::eras_total_stakes(0), 2001);
            assert_eq!(Balances::total_balance(&2), init_balance_2);
            assert_eq!(Balances::total_balance(&10), init_balance_10);
            assert_eq!(Balances::total_balance(&11), init_balance_11);

            block = 6; // Block 6 => Session 2 => Era 0
            System::set_block_number(block);
            Timestamp::set_timestamp(block * 5000 + delay); // a little late.
            Session::on_initialize(System::block_number());
            assert_eq!(Staking::current_era().unwrap_or(0), 0);
            assert_eq!(Session::current_index(), 2);

            block = 9; // Block 9 => Session 3 => Era 1
            System::set_block_number(block);
            Timestamp::set_timestamp(block * 5000); // back to being on time. no delays
            Session::on_initialize(System::block_number());
            assert_eq!(Staking::current_era().unwrap_or(0), 1);
            assert_eq!(Session::current_index(), 3);
            Staking::reward_stakers(Origin::signed(10), 11, 0).unwrap();
            // 11 validator has 2/3 of the total rewards and half half for it and its guarantor
            assert_eq_error_rate!(
                Balances::total_balance(&2) / 1000000,
                (init_balance_2 + staking_reward * 500 / 2001) / 1000000,
                1
            );
            assert_eq_error_rate!(
                Balances::total_balance(&10) / 1000000,
                (init_balance_10 + authoring_reward * 2 / 3 + staking_reward * 500 / 2001) / 1000000,
                1
            );
            assert_eq!(Balances::total_balance(&11), init_balance_11);
        });
}

#[test]
fn multi_era_reward_should_work() {
    // Should check that:
    // The value of current_session_reward is set at the end of each era, based on
    // total_stakes and session_reward.
    ExtBuilder::default()
        .guarantee(false)
        .own_workload(u128::max_value())
        .build()
        .execute_with(|| {
            let init_balance_10 = Balances::total_balance(&10);
            let init_balance_21 = Balances::total_balance(&21);

            // Set payee to controller
            assert_ok!(Staking::set_payee(
                Origin::signed(10),
                RewardDestination::Controller
            ));

            // Compute now as other parameter won't change
            let total_authoring_payout = Staking::authoring_rewards_in_era();
            let total_staking_payout_0 = Staking::staking_rewards_in_era(Staking::current_era().unwrap_or(0));
            assert!(total_staking_payout_0 > 10); // Test is meaningful if reward something
            assert_eq!(Staking::eras_total_stakes(0), 2001);
            <Module<Test>>::reward_by_ids(vec![(11, 1)]);

            start_session(0, true);
            start_session(1, true);
            start_session(2, true);
            start_session(3, true);
            payout_all_stakers(0);

            assert_eq!(Staking::current_era().unwrap_or(0), 1);
            assert_eq!(Staking::eras_total_stakes(1), 2001);
            // rewards may round to 0.000001
            assert_eq!(
                Balances::total_balance(&10) / 1000000,
                (init_balance_10 + total_authoring_payout + total_staking_payout_0 * 1000 / 2001) / 1000000
            );
            let stakes_21 = Balances::total_balance(&21);
            // candidates should have rewards
            assert_eq!(
                stakes_21 / 1000000,
                (init_balance_21 + total_staking_payout_0 * 1000 / 2001) / 1000000
            );

            start_session(4, true);

            <Module<Test>>::reward_by_ids(vec![(11, 101)]); // meaningless points
            let total_authoring_payout_1 = Staking::authoring_rewards_in_era();
            // new era is triggered here.
            start_session(5, true);
            let total_staking_payout_1 = Staking::staking_rewards_in_era(Staking::current_era().unwrap_or(0));
            assert!(total_staking_payout_1 > 10); // Test is meaningful if reward something
            payout_all_stakers(1);
            assert_eq!(Staking::eras_total_stakes(2), 142667657428421);
            // pay time
            assert_eq!(
                Balances::total_balance(&10) / 1000000,
                (init_balance_10 + total_authoring_payout + total_staking_payout_0 * 1000 / 2001
                    + total_authoring_payout_1 + (total_staking_payout_1 * 1000 / 2001)) / 1000000
            );
            assert_eq!(
                Balances::total_balance(&21) / 1000000,
                (stakes_21 + (total_staking_payout_1 * 1000 / 2001)) / 1000000
            );
        });
}

#[test]
fn staking_should_work() {
    // should test:
    // * new validators can be added to the default set
    // * new ones will be chosen per era
    // * either one can unlock the stash and back-down from being a validator via `chill`ing.
    ExtBuilder::default()
        .guarantee(false)
        .fair(false) // to give 20 more staked value
        .build()
        .execute_with(|| {
            // remember + compare this along with the test.
            assert_eq_uvec!(validator_controllers(), vec![20, 10]);

            // put some money in account that we'll use.
            for i in 1..5 {
                let _ = Balances::make_free_balance_be(&i, 3000);
            }

            // --- Block 1:
            start_session(1, false);
            // add a new candidate for being a validator. account 3 controlled by 4.
            assert_ok!(Staking::bond(
                Origin::signed(3),
                4,
                2500,
                RewardDestination::Controller
            ));
            Staking::upsert_stake_limit(&3, 3000);
            assert_ok!(Staking::validate(Origin::signed(4), Perbill::one()));

            // No effects will be seen so far.
            assert_eq_uvec!(validator_controllers(), vec![20, 10]);

            // --- Block 2:
            start_session(2, false);

            // No effects will be seen so far. Era has not been yet triggered.
            assert_eq_uvec!(validator_controllers(), vec![20, 10]);

            // --- Block 3: the validators will now be queued.
            start_session(3, false);
            assert_eq!(Staking::current_era().unwrap_or(0), 1);

            // --- Block 4: the validators will now be changed.
            start_session(4, false);

            assert_eq_uvec!(validator_controllers(), vec![4, 20]);
            // --- Block 4: Unstake 4 as a validator, freeing up the balance stashed in 3
            // 4 will chill
            Staking::chill(Origin::signed(4)).unwrap();

            // --- Block 5: nothing. 4 is still there.
            start_session(5, false);
            assert_eq_uvec!(validator_controllers(), vec![4, 20]);

            // --- Block 6: since we are using TestStaking instead of real Staking for Tee, 11 and 21 would still be in Validators.
            start_session(7, false);
            assert!(<Validators<Test>>::contains_key(&11));
            assert!(<Validators<Test>>::contains_key(&21));
            assert!(!<Validators<Test>>::contains_key(&3));
            assert_eq_uvec!(validator_controllers(), vec![20, 10]);

            // Note: the stashed value of 4 is still lock, and valid will not be updated, cause all
            // validators gone
            assert_eq!(
                Staking::ledger(&4),
                Some(StakingLedger {
                    stash: 3,
                    total: 2500,
                    active: 2500,
                    valid: 2500,
                    unlocking: vec![],
                    claimed_rewards: vec![],
                })
            );
            // e.g. it cannot spend more than 500 that it has free from the total 2000
            assert_noop!(
                Balances::reserve(&3, 501),
                DispatchError::Module {
                    index: 0,
                    error: 1,
                    message: Some("LiquidityRestrictions"),
                }
            );
            assert_ok!(Balances::reserve(&3, 409));
        });
}

#[test]
fn less_than_needed_candidates_works() {
    ExtBuilder::default()
        .minimum_validator_count(1)
        .validator_count(4)
        .guarantee(false)
        .num_validators(3)
        .build()
        .execute_with(|| {
            assert_eq!(Staking::validator_count(), 4);
            assert_eq!(Staking::minimum_validator_count(), 1);
            assert_eq_uvec!(validator_controllers(), vec![30, 20, 10]);

            start_era(1, false);

            // Previous set is selected. NO election algorithm is even executed.
            assert_eq_uvec!(validator_controllers(), vec![30, 20, 10]);

            // But the exposure is updated in a simple way. No external votes exists.
            // This is purely self-vote.
            assert_eq!(Staking::stakers(10).others.len(), 0);
            assert_eq!(Staking::stakers(20).others.len(), 0);
            assert_eq!(Staking::stakers(30).others.len(), 0);
            check_exposure_all();
            check_guarantor_all();
        });
}

#[test]
fn no_candidate_emergency_condition() {
    ExtBuilder::default()
        .minimum_validator_count(10)
        .validator_count(15)
        .num_validators(4)
        .validator_pool(true)
        .guarantee(false)
        .build()
        .execute_with(|| {
            // initial validators
            assert_eq_uvec!(validator_controllers(), vec![10, 20, 30, 40]);

            // set the minimum validator count.
            <Staking as crate::Store>::MinimumValidatorCount::put(10);
            <Staking as crate::Store>::ValidatorCount::put(15);
            assert_eq!(Staking::validator_count(), 15);

            let _ = Staking::chill(Origin::signed(10));

            // trigger era
            System::set_block_number(1);
            Session::on_initialize(System::block_number());

            // Previous ones are elected. chill is invalidates. TODO: #2494
            assert_eq_uvec!(validator_controllers(), vec![10, 20, 30, 40]);
            assert_eq!(Staking::current_elected().len(), 0);
        });
}

#[test]
fn guaranteeing_and_rewards_should_work() {
    ExtBuilder::default()
        .guarantee(false)
        .validator_pool(true)
        .build()
        .execute_with(|| {
            // initial validators -- everyone is actually even.
            assert_eq_uvec!(validator_controllers(), vec![40, 30]);

            // Set payee to controller
            assert_ok!(Staking::set_payee(
                Origin::signed(10),
                RewardDestination::Controller
            ));
            assert_ok!(Staking::set_payee(
                Origin::signed(20),
                RewardDestination::Controller
            ));
            assert_ok!(Staking::set_payee(
                Origin::signed(30),
                RewardDestination::Controller
            ));
            assert_ok!(Staking::set_payee(
                Origin::signed(40),
                RewardDestination::Controller
            ));

            // give the man some money
            let initial_balance = 1000;
            for i in [1, 2, 3, 4, 5, 10, 11, 20, 21].iter() {
                let _ = Balances::make_free_balance_be(i, initial_balance);
            }

            // bond two account pairs and state interest in nomination.
            // 2 will guarantee for 10, 20, 30
            assert_ok!(Staking::bond(
                Origin::signed(1),
                2,
                1000,
                RewardDestination::Controller
            ));
            assert_ok!(Staking::guarantee(
                Origin::signed(2),
                (11, 333)
            ));
            assert_ok!(Staking::guarantee(
                Origin::signed(2),
                (21, 333)
            ));
            assert_ok!(Staking::guarantee(
                Origin::signed(2),
                (31, 333)
            ));
            // 4 will guarantee for 10, 20, 40
            assert_ok!(Staking::bond(
                Origin::signed(3),
                4,
                1000,
                RewardDestination::Controller
            ));
            assert_ok!(Staking::guarantee(
                Origin::signed(4),
                (11, 333)
            ));
            assert_ok!(Staking::guarantee(
                Origin::signed(4),
                (21, 333)
            ));
            assert_ok!(Staking::guarantee(
                Origin::signed(4),
                (41, 333)
            ));

            // the total reward for era 0
            let total_authoring_payout_0 = Staking::authoring_rewards_in_era();
            let total_staking_payout_0 = Staking::staking_rewards_in_era(Staking::current_era().unwrap_or(0));
            assert!(total_staking_payout_0 > 100); // Test is meaningful if reward something
            <Module<Test>>::reward_by_ids(vec![(41, 1)]);
            <Module<Test>>::reward_by_ids(vec![(31, 1)]);
            <Module<Test>>::reward_by_ids(vec![(21, 10)]); // must be no-op
            <Module<Test>>::reward_by_ids(vec![(11, 10)]); // must be no-op

            start_era(1, true);

            // 10 and 20 have more votes, they will be chosen by top-down.
            assert_eq_uvec!(validator_controllers(), vec![20, 10]);
            assert_eq!(Staking::eras_total_stakes(1), 5998);

            payout_all_stakers(0);
            Staking::reward_stakers(Origin::signed(10), 41, 0).unwrap();
            // OLD validators must have already received some rewards.
            assert_eq!(Balances::total_balance(&40), 1 + total_authoring_payout_0 / 2 + total_staking_payout_0 / 4);
            assert_eq!(Balances::total_balance(&30), 1 + total_authoring_payout_0 / 2 + total_staking_payout_0 / 4);

            // ------ check the staked value of all parties.

            if cfg!(feature = "equalize") {
                // total expo of 10, with 1200 coming from guarantors (externals), according to phragmen.
                // TODO: tmp change for equalize strategy(with voting to candidates)
                assert_eq!(Staking::stakers(11).own, 1000);
                assert_eq_error_rate!(Staking::stakers(11).total, 1000 + 666, 2);
                // 2 and 4 supported 10, each with stake 600, according to phragmen.
                assert_eq!(
                    Staking::stakers(11)
                        .others
                        .iter()
                        .map(|e| e.value)
                        .collect::<Vec<BalanceOf<Test>>>(),
                    vec![333, 333]
                );
                assert_eq!(
                    Staking::stakers(11)
                        .others
                        .iter()
                        .map(|e| e.who)
                        .collect::<Vec<u64>>(),
                    vec![1, 3]
                );
                // total expo of 20, with 500 coming from guarantors (externals), according to phragmen.
                // TODO: tmp change for equalize strategy(with voting to candidates)
                assert_eq!(Staking::stakers(21).own, 1000);
                assert_eq_error_rate!(Staking::stakers(21).total, 1000 + 666, 2);
                // 2 and 4 supported 20, each with stake 250, according to phragmen.
                assert_eq!(
                    Staking::stakers(21)
                        .others
                        .iter()
                        .map(|e| e.value)
                        .collect::<Vec<BalanceOf<Test>>>(),
                    vec![333, 333]
                );
                assert_eq!(
                    Staking::stakers(21)
                        .others
                        .iter()
                        .map(|e| e.who)
                        .collect::<Vec<u64>>(),
                    vec![1, 3]
                );
            } else {
                // total expo of 10, with 1200 coming from guarantors (externals), according to phragmen.
                assert_eq!(Staking::stakers(11).own, 1000);
                assert_eq!(Staking::stakers(11).total, 1000 + 800);
                // 2 and 4 supported 10, each with stake 600, according to phragmen.
                assert_eq!(
                    Staking::stakers(11)
                        .others
                        .iter()
                        .map(|e| e.value)
                        .collect::<Vec<BalanceOf<Test>>>(),
                    vec![400, 400]
                );
                assert_eq!(
                    Staking::stakers(11)
                        .others
                        .iter()
                        .map(|e| e.who)
                        .collect::<Vec<u64>>(),
                    vec![3, 1]
                );
                // total expo of 20, with 500 coming from guarantors (externals), according to phragmen.
                assert_eq!(Staking::stakers(21).own, 1000);
                assert_eq_error_rate!(Staking::stakers(21).total, 1000 + 1200, 2);
                // 2 and 4 supported 20, each with stake 250, according to phragmen.
                assert_eq!(
                    Staking::stakers(21)
                        .others
                        .iter()
                        .map(|e| e.value)
                        .collect::<Vec<BalanceOf<Test>>>(),
                    vec![600, 600]
                );
                assert_eq!(
                    Staking::stakers(21)
                        .others
                        .iter()
                        .map(|e| e.who)
                        .collect::<Vec<u64>>(),
                    vec![3, 1]
                );
            }

            // They are not chosen anymore
            // TODO: tmp change for equalize strategy(with voting to candidates)
            assert_eq!(Staking::stakers(31).total, 1333);
            assert_eq!(Staking::stakers(41).total, 1333);

            // the total reward for era 1
            // TODO: tmp change for equalize strategy(with voting to candidates)
            // TODO: add era_2 for guarantor's rewards
            /*let total_staking_payout_1 = Staking::staking_rewards_in_era(Staking::current_era().unwrap_or(0));
            assert!(total_staking_payout_1 > 100); // Test is meaningfull if reward something
            <Module<Test>>::reward_by_ids(vec![(41, 10)]); // must be no-op
            <Module<Test>>::reward_by_ids(vec![(31, 10)]); // must be no-op
            <Module<Test>>::reward_by_ids(vec![(21, 2)]);
            <Module<Test>>::reward_by_ids(vec![(11, 1)]);

            start_era(2, true);

            // nothing else will happen, era ends and rewards are paid again,
            // it is expected that guarantors will also be paid. See below

            let payout_for_10 = total_authoring_payout_0 / 3;
            let payout_for_20 = 2 * total_authoring_payout_0 / 3;
            if cfg!(feature = "equalize") {
                // Guarantor 2: has [333 / 2000 ~ 1 / 5 from 10] + [333 / 2000 ~ 3 / 10 from 20]'s reward.
                assert_eq_error_rate!(
                    Balances::total_balance(&2),
                    initial_balance + payout_for_10 / 5 + payout_for_20 * 3 / 10,
                    2,
                );
                // Guarantor 4: has [400 / 2000 ~ 1 / 5 from 20] + [600 / 2000 ~ 3 / 10 from 10]'s reward.
                assert_eq_error_rate!(
                    Balances::total_balance(&4),
                    initial_balance + payout_for_20 / 5 + payout_for_10 * 3 / 10,
                    2,
                );

                // Validator 10: got 1000 / 2000 external stake.
                assert_eq_error_rate!(
                    Balances::total_balance(&10),
                    initial_balance + payout_for_10 / 2,
                    1,
                );
                // Validator 20: got 1000 / 2000 external stake.
                assert_eq_error_rate!(
                    Balances::total_balance(&20),
                    initial_balance + payout_for_20 / 2,
                    1,
                );
            } else {
                // Guarantor 2: has [400/1800 ~ 2/9 from 10] + [600/2200 ~ 3/11 from 20]'s reward. ==> 2/9 + 3/11
                assert_eq_error_rate!(
                    Balances::total_balance(&2),
                    initial_balance + (2 * payout_for_10 / 9 + 3 * payout_for_20 / 11),
                    1,
                );
                // Guarantor 4: has [400/1800 ~ 2/9 from 10] + [600/2200 ~ 3/11 from 20]'s reward. ==> 2/9 + 3/11
                assert_eq_error_rate!(
                    Balances::total_balance(&4),
                    initial_balance + (2 * payout_for_10 / 9 + 3 * payout_for_20 / 11),
                    1,
                );

                // Validator 10: got 800 / 1800 external stake => 8/18 =? 4/9 => Validator's share = 5/9
                assert_eq_error_rate!(
                    Balances::total_balance(&10),
                    initial_balance + 5 * payout_for_10 / 9,
                    1,
                );
                // Validator 20: got 1200 / 2200 external stake => 12/22 =? 6/11 => Validator's share = 5/11
                assert_eq_error_rate!(
                    Balances::total_balance(&20),
                    initial_balance + 5 * payout_for_20 / 11,
                    1,
                );
            }*/

            check_exposure_all();
            check_guarantor_all();
        });
}

#[test]
fn guarantors_also_get_slashed() {
    // A guarantor should be slashed if the validator they guaranteed is slashed
    // Here is the breakdown of roles:
    // 10 - is the controller of 11
    // 11 - is the stash.
    // 2 - is the guarantor of 20, 10
    ExtBuilder::default()
        .guarantee(false)
        .build()
        .execute_with(|| {
            Staking::upsert_stake_limit(&20, 2000);
            Staking::upsert_stake_limit(&10, 2000);

            assert_eq!(Staking::validator_count(), 2);

            // Set payee to controller
            assert_ok!(Staking::set_payee(
                Origin::signed(10),
                RewardDestination::Controller
            ));

            // give the man some money.
            let initial_balance = 1000;
            for i in [1, 2, 3, 10].iter() {
                let _ = Balances::make_free_balance_be(i, initial_balance);
            }

            // 2 want to guarantee for 10, 20
            let guarantor_stake = 500;
            assert_ok!(Staking::bond(
                Origin::signed(1),
                2,
                guarantor_stake,
                RewardDestination::default()
            ));
            // but it won't work, cause 10&20 are not validators
            assert_noop!(
                Staking::guarantee(Origin::signed(2), (20, 250)),
                DispatchError::Module {
                    index: 0,
                    error: 6,
                    message: Some("InvalidTarget"),
                }
            );

            assert_noop!(
                Staking::guarantee(Origin::signed(2), (10, 250)),
                DispatchError::Module {
                    index: 0,
                    error: 6,
                    message: Some("InvalidTarget"),
                }
            );

            let total_payout = Staking::staking_rewards_in_era(Staking::current_era().unwrap_or(0));
            assert!(total_payout > 100); // Test is meaningfull if reward something
            <Module<Test>>::reward_by_ids(vec![(11, 1)]);

            // new era, pay rewards,
            start_era(1, false);

            // Guarantor stash didn't collect any.
            assert_eq!(Balances::total_balance(&2), initial_balance);

            // 10 goes offline
            on_offence_now(
                &[OffenceDetails {
                    offender: (11, Staking::stakers(&11)),
                    reporters: vec![],
                }],
                &[Perbill::from_percent(5)],
            );
            let expo = Staking::stakers(11);
            let slash_value = 50;
            let total_slash = expo.total.min(slash_value);
            let validator_slash = expo.own.min(total_slash);
            let guarantor_slash = guarantor_stake.min(total_slash - validator_slash);

            // initial + first era reward + slash
            assert_eq!(
                Balances::total_balance(&11),
                initial_balance - validator_slash
            );
            assert_eq!(
                Balances::total_balance(&2),
                initial_balance - guarantor_slash
            );
            check_exposure_all();
            check_guarantor_all();
            // Because slashing happened.
            assert!(is_disabled(10));
        });
}

#[test]
fn double_staking_should_fail() {
    // should test (in the same order):
    // * an account already bonded as stash cannot be be stashed again.
    // * an account already bonded as stash cannot guarantee.
    // * an account already bonded as controller can guarantee.
    ExtBuilder::default().build().execute_with(|| {
        let arbitrary_value = 5000;
        Staking::upsert_stake_limit(&11, 2000);
        let _ = Balances::make_free_balance_be(&1, 1000000);
        // 2 = controller, 1 stashed => ok
        assert_ok!(Staking::bond(
            Origin::signed(1),
            2,
            arbitrary_value,
            RewardDestination::default()
        ));
        // 4 = not used so far, 1 stashed => not allowed.
        assert_noop!(
            Staking::bond(
                Origin::signed(1),
                4,
                arbitrary_value,
                RewardDestination::default()
            ),
            Error::<Test>::AlreadyBonded,
        );
        // 1 = stashed => attempting to guarantee should fail.
        assert_noop!(
            Staking::guarantee(Origin::signed(1), (1, arbitrary_value)),
            Error::<Test>::NotController
        );

        // 2 = controller  => guarantee should work. But only 750 is invalid since stake limit and 100's 250
        assert_ok!(Staking::guarantee(
            Origin::signed(2),
            (11, arbitrary_value)
        ));
        assert_eq!(Staking::guarantee_rel(1, 11).get(&(0 as u32)), Some(&(750 as Balance)));
        assert_eq!(Staking::guarantee_rel(101, 11).get(&(0 as u32)), Some(&(250 as Balance)));
        start_era(1, false);
        assert_eq!(
            Staking::ledger(&2),
            Some(StakingLedger {
                stash: 1,
                total: arbitrary_value,
                active: arbitrary_value,
                valid: 750,
                unlocking: vec![],
                claimed_rewards: vec![]
            })
        );
    });
}

#[test]
fn double_controlling_should_fail() {
    // should test (in the same order):
    // * an account already bonded as controller CANNOT be reused as the controller of another account.
    ExtBuilder::default().build().execute_with(|| {
        let arbitrary_value = 5;
        // 2 = controller, 1 stashed => ok
        assert_ok!(Staking::bond(
            Origin::signed(1),
            2,
            arbitrary_value,
            RewardDestination::default(),
        ));
        // 2 = controller, 3 stashed (Note that 2 is reused.) => no-op
        assert_noop!(
            Staking::bond(
                Origin::signed(3),
                2,
                arbitrary_value,
                RewardDestination::default()
            ),
            Error::<Test>::AlreadyPaired,
        );
    });
}

#[test]
fn session_and_eras_work() {
    ExtBuilder::default().build().execute_with(|| {
        assert_eq!(Staking::current_era().unwrap_or(0), 0);

        // Block 1: No change.
        start_session(0, false);
        assert_eq!(Session::current_index(), 1);
        assert_eq!(Staking::current_era().unwrap_or(0), 0);

        // Block 2: Simple era change.
        start_session(2, false);
        assert_eq!(Session::current_index(), 3);
        assert_eq!(Staking::current_era().unwrap_or(0), 1);

        // Block 3: Schedule an era length change; no visible changes.
        start_session(3, false);
        assert_eq!(Session::current_index(), 4);
        assert_eq!(Staking::current_era().unwrap_or(0), 1);

        // Block 4: Era change kicks in.
        start_session(5, false);
        assert_eq!(Session::current_index(), 6);
        assert_eq!(Staking::current_era().unwrap_or(0), 2);

        // Block 5: No change.
        start_session(6, false);
        assert_eq!(Session::current_index(), 7);
        assert_eq!(Staking::current_era().unwrap_or(0), 2);

        // Block 6: No change.
        start_session(7, false);
        assert_eq!(Session::current_index(), 8);
        assert_eq!(Staking::current_era().unwrap_or(0), 2);

        // Block 7: Era increment.
        start_session(8, false);
        assert_eq!(Session::current_index(), 9);
        assert_eq!(Staking::current_era().unwrap_or(0), 3);
    });
}

#[test]
fn forcing_new_era_works() {
    ExtBuilder::default().build().execute_with(|| {
        // normal flow of session.
        assert_eq!(Staking::current_era().unwrap_or(0), 0);
        start_session(0, false);
        assert_eq!(Staking::current_era().unwrap_or(0), 0);
        start_session(1, false);
        assert_eq!(Staking::current_era().unwrap_or(0), 0);
        start_session(2, false);
        assert_eq!(Staking::current_era().unwrap_or(0), 1);

        // no era change.
        ForceEra::put(Forcing::ForceNone);
        start_session(3, false);
        assert_eq!(Staking::current_era().unwrap_or(0), 1);
        start_session(4, false);
        assert_eq!(Staking::current_era().unwrap_or(0), 1);
        start_session(5, false);
        assert_eq!(Staking::current_era().unwrap_or(0), 1);
        start_session(6, false);
        assert_eq!(Staking::current_era().unwrap_or(0), 1);

        // back to normal.
        // this immediately starts a new session.
        ForceEra::put(Forcing::NotForcing);
        start_session(7, false);
        assert_eq!(Staking::current_era().unwrap_or(0), 2);
        start_session(8, false);
        assert_eq!(Staking::current_era().unwrap_or(0), 2);

        // forceful change
        ForceEra::put(Forcing::ForceAlways);
        start_session(9, false);
        assert_eq!(Staking::current_era().unwrap_or(0), 3);
        start_session(10, false);
        assert_eq!(Staking::current_era().unwrap_or(0), 4);
        start_session(11, false);
        assert_eq!(Staking::current_era().unwrap_or(0), 5);

        // just one forceful change
        ForceEra::put(Forcing::ForceNew);
        start_session(12, false);
        assert_eq!(Staking::current_era().unwrap_or(0), 6);

        assert_eq!(ForceEra::get(), Forcing::NotForcing);
        start_session(13, false);
        assert_eq!(Staking::current_era().unwrap_or(0), 6);
    });
}

#[test]
fn cannot_transfer_staked_balance() {
    // Tests that a stash account cannot transfer funds
    ExtBuilder::default()
        .guarantee(false)
        .build()
        .execute_with(|| {
            // Confirm account 11 is stashed
            assert_eq!(Staking::bonded(&11), Some(10));
            // Confirm account 11 has some free balance
            assert_eq!(Balances::free_balance(&11), 1000);
            // Confirm account 11 (via controller 10) is totally staked
            assert_eq!(Staking::stakers(&11).total, 1000);
            // Confirm account 11 cannot transfer as a result
            assert_noop!(
                Balances::transfer(Origin::signed(11), 20, 1),
                DispatchError::Module {
                    index: 0,
                    error: 1,
                    message: Some("LiquidityRestrictions"),
                }
            );

            // Give account 11 extra free balance
            let _ = Balances::make_free_balance_be(&11, 10000);
            // Confirm that account 11 can now transfer some balance
            assert_ok!(Balances::transfer(Origin::signed(11), 20, 1));
        });
}

#[test]
fn cannot_transfer_staked_balance_2() {
    // Tests that a stash account cannot transfer funds
    // Same test as above but with 20, and more accurate.
    // 21 has 2000 free balance but 1000 at stake
    ExtBuilder::default()
        .guarantee(false)
        .fair(true)
        .build()
        .execute_with(|| {
            // Confirm account 21 is stashed
            assert_eq!(Staking::bonded(&21), Some(20));
            // Confirm account 21 has some free balance
            assert_eq!(Balances::free_balance(&21), 2000);
            // Confirm account 21 (via controller 20) is totally staked
            assert_eq!(Staking::stakers(&21).total, 1000);
            // Confirm account 21 can transfer at most 1000
            assert_noop!(
                Balances::transfer(Origin::signed(21), 20, 1001),
                DispatchError::Module {
                    index: 0,
                    error: 1,
                    message: Some("LiquidityRestrictions"),
                }
            );
            assert_ok!(Balances::transfer(Origin::signed(21), 20, 1000));
        });
}

#[test]
fn cannot_reserve_staked_balance() {
    // Checks that a bonded account cannot reserve balance from free balance
    ExtBuilder::default().build().execute_with(|| {
        // Confirm account 11 is stashed
        assert_eq!(Staking::bonded(&11), Some(10));
        // Confirm account 11 has some free balance
        assert_eq!(Balances::free_balance(&11), 1000);
        // Confirm account 11 (via controller 10) is totally staked
        assert_eq!(Staking::stakers(&11).own, 1000);
        // Confirm account 11 cannot transfer as a result
        assert_noop!(
            Balances::reserve(&11, 1),
            DispatchError::Module {
                index: 0,
                error: 1,
                message: Some("LiquidityRestrictions"),
            }
        );

        // Give account 11 extra free balance
        let _ = Balances::make_free_balance_be(&11, 10000);
        // Confirm account 11 can now reserve balance
        assert_ok!(Balances::reserve(&11, 1));
    });
}

#[test]
// TODO: destination is duplicate with other test cases, but we should check it separately
/*fn reward_destination_works() {
    // Rewards go to the correct destination as determined in Payee
    ExtBuilder::default()
        .guarantee(false)
        .build()
        .execute_with(|| {
            // Check that account 11 is a validator
            // Account 11's limit is 2000
            assert!(Staking::current_elected().contains(&11));
            // Check the balance of the validator account
            assert_eq!(Balances::free_balance(&10), 1);
            // Check the balance of the stash account
            assert_eq!(Balances::free_balance(&11), 1000);
            // Check how much is at stake
            assert_eq!(
                Staking::ledger(&10),
                Some(StakingLedger {
                    stash: 11,
                    total: 1000,
                    active: 1000,
                    valid: 1000,
                    unlocking: vec![],
                })
            );

            // Compute total payout now for whole duration as other parameter won't change
            let total_authoring_payout = Staking::authoring_rewards_in_era();
            let total_staking_payout = Staking::staking_rewards_in_era(Staking::current_era().unwrap_or(0));
            assert!(total_staking_payout > 100); // Test is meaningfull if reward something
            <Module<Test>>::reward_by_ids(vec![(11, 1)]);

            // After an era, stake limit calculate using last era's reward
            start_era(1, true);

            // Check that RewardDestination is Staked (default)
            assert_eq!(Staking::payee(&11), RewardDestination::Staked);
            // Check that reward went to the stash account of validator
            let reward_0 = total_authoring_payout + total_staking_payout * Perbill::from_rational_approximation(1000, 2001);
            let stakes_11 = Balances::free_balance(&11);
            let total_stakes = Staking::total_stakes();
            assert_eq!(Balances::free_balance(&11),
                       1000 + reward_0);
            // Check that amount at stake increased accordingly
            assert_eq!(
                Staking::ledger(&10),
                Some(StakingLedger {
                    stash: 11,
                    total: 1000 + reward_0,
                    active: 1000 + reward_0,
                    valid: 1000 + reward_0, // Test should be modified.
                    unlocking: vec![],
                })
            );

            //Change RewardDestination to Stash
            <Payee<Test>>::insert(&11, RewardDestination::Stash);

            // Compute total payout now for whole duration as other parameter won't change
            <Module<Test>>::reward_by_ids(vec![(11, 1)]);

            start_era(2, true);

            // Check that RewardDestination is Stash
            let reward_1 = total_authoring_payout*2 + total_staking_payout * Perbill::from_rational_approximation(1000, 2001)
                + total_staking_payout * Perbill::from_rational_approximation(1000, 2001)
                + total_staking_payout * Perbill::from_rational_approximation(stakes_11, total_stakes);
            assert_eq!(Staking::payee(&11), RewardDestination::Stash);
            // Check that reward went to the stash account
            assert_eq!(
                Balances::free_balance(&11),
                1000 + reward_1
            );
            // Record this value
            let recorded_stash_balance = 1000 + reward_1;
            // Check that amount at stake is NOT increased
            assert_eq!(
                Staking::ledger(&10),
                Some(StakingLedger {
                    stash: 11,
                    total: 1000 + reward_0,
                    active: 1000 + reward_0,
                    valid: 1000 + reward_0, // Test should be modified.
                    unlocking: vec![],
                })
            );

            // Change RewardDestination to Controller
            <Payee<Test>>::insert(&11, RewardDestination::Controller);

            // Check controller balance
            assert_eq!(Balances::free_balance(&10), 1);

            // Compute total payout now for whole duration as other parameter won't change
            let total_payout_2 = total_authoring_payout;
            assert_eq!(total_payout_2, 0); // Test is meaningfull if reward something
            <Module<Test>>::reward_by_ids(vec![(11, 1)]);

            // work report should be outdated, and rewards should be 0
            start_era(3, true);

            // Check that RewardDestination is Controller
            assert_eq!(Staking::payee(&11), RewardDestination::Controller);
            // Check that reward went to the controller account
            assert_eq!(Balances::free_balance(&10), 1 + total_staking_payout);
            // Check that amount at stake is NOT increased
            assert_eq!(
                Staking::ledger(&10),
                Some(StakingLedger {
                    stash: 11,
                    total: 1000 + total_staking_payout,
                    active: 1000 + total_staking_payout,
                    valid: 1000 + total_staking_payout, // Test should be modified.
                    unlocking: vec![],
                })
            );
            // Check that amount in staked account is NOT increased.
            assert_eq!(Balances::free_balance(&11), recorded_stash_balance);
        });
}*/

#[test]
fn staking_reward_change_work() {
    ExtBuilder::default()
        .guarantee(false)
        .build()
        .execute_with(|| {
            // Make 1 account be max balance
            let _ = Balances::make_free_balance_be(&11, Balance::max_value());
            // If 1 era is 30 min, Julian year should contains 17532 eras.
            // If era_num < 17532, staking_rewards should be
            assert_eq!(Staking::staking_rewards_in_era(17530), 285192790326260);
            assert_eq!(Staking::staking_rewards_in_era(17531), 285192790326260);
            // era_num >= 17532 & era_num <= 35064, staking_rewards should be
            assert_eq!(Staking::staking_rewards_in_era(35000), 228154232261008);
            // TODO: for test case max issue is 18446744
            // era_num > 210384 * 3, inflation rate will reduce less than 1%, then it should be
            assert_eq!(Balances::total_issuance(), u64::max_value());
            assert_eq!(Staking::staking_rewards_in_era(631152), (184467440737095516 / ((36525*48) / 100)));
        })
}

#[test]
fn validator_payment_prefs_work() {
    // Test that validator preferences are correctly honored
    // Note: unstake threshold is being directly tested in slashing tests.
    // This test will focus on validator payment.
    ExtBuilder::default()
        .guarantee(false)
        .build()
        .execute_with(|| {
        // Initial config
        let stash_initial_balance = Balances::total_balance(&11);

        // check the balance of a validator accounts.
        assert_eq!(Balances::total_balance(&10), 1);
        // check the balance of a validator's stash accounts.
        assert_eq!(Balances::total_balance(&11), stash_initial_balance);
        // and the guarantor (to-be)
        let _ = Balances::make_free_balance_be(&2, 500);

        // add a dummy guarantor.
        <ErasStakers<Test>>::insert(
            0,
            &11,
            Exposure {
                own: 500, // equal division indicates that the reward will be equally divided among validator and guarantor.
                total: 1000,
                others: vec![IndividualExposure { who: 2, value: 500 }],
            },
        );
        <ErasValidatorPrefs<Test>>::insert(
            0,
            &11,
            ValidatorPrefs {
                guarantee_fee: Perbill::from_percent(50)
            },
        );
        <Payee<Test>>::insert(&2, RewardDestination::Stash);
        <Validators<Test>>::insert(
            &11,
            Validations {
                total: 0,
                guarantee_fee: Perbill::from_percent(50),
                guarantors: vec![],
            },
        );

        // Compute total payout now for whole duration as other parameter won't change
        let total_authoring_payout_0 = Staking::authoring_rewards_in_era();
        let total_staking_payout_0 = Staking::staking_rewards_in_era(Staking::current_era().unwrap_or(0));
        assert_eq!(Staking::eras_total_stakes(0), 2001); // Test is meaningfull if reward something
        <Module<Test>>::reward_by_ids(vec![(11, 1)]);


        start_era(1, true);
        Staking::reward_stakers(Origin::signed(10), 11, 0).unwrap();

        let shared_cut = total_staking_payout_0 * 500 / 2001;
        // Validator's payee is Staked account, 11, reward will be paid here.
        // Round to 0.000001
        assert_eq!(
            Balances::total_balance(&11) / 1000000,
            (stash_initial_balance + shared_cut / 2 + shared_cut + total_authoring_payout_0) / 1000000
        );
        // Controller account will not get any reward.
        assert_eq!(Balances::total_balance(&10), 1);
        // Rest of the reward will be shared and paid to the guarantor in stake.
        assert_eq!(Balances::total_balance(&2) / 1000000, (500 + shared_cut / 2) / 1000000);

        check_exposure_all();
        check_guarantor_all();
    });
}

#[test]
fn bond_extra_works() {
    // Tests that extra `free_balance` in the stash can be added to stake
    // NOTE: this tests only verifies `StakingLedger` for correct updates
    // See `bond_extra_and_withdraw_unbonded_works` for more details and updates on `Exposure`.
    ExtBuilder::default().build().execute_with(|| {
        // Check that account 10 is a validator
        assert!(<Validators<Test>>::contains_key(11));
        // Check that account 10 is bonded to account 11
        assert_eq!(Staking::bonded(&11), Some(10));
        // Check how much is at stake
        assert_eq!(
            Staking::ledger(&10),
            Some(StakingLedger {
                stash: 11,
                total: 1000,
                active: 1000,
                valid: 1000,
                unlocking: vec![],
                claimed_rewards: vec![]
            })
        );

        // Give account 11 some large free balance greater than total
        let _ = Balances::make_free_balance_be(&11, 1500);

        // Call the bond_extra function from controller, add only 100
        assert_ok!(Staking::bond_extra(Origin::signed(11), 100));
        // There should be 100 more `total` and `active` in the ledger
        assert_eq!(
            Staking::ledger(&10),
            Some(StakingLedger {
                stash: 11,
                total: 1000 + 100,
                active: 1000 + 100,
                valid: 1000,
                unlocking: vec![],
                claimed_rewards: vec![]
            })
        );

        // Call the bond_extra function with a large number, should handle it
        assert_ok!(Staking::bond_extra(Origin::signed(11), u64::max_value()));
        // The full amount of the funds should now be in the total and active
        assert_eq!(
            Staking::ledger(&10),
            Some(StakingLedger {
                stash: 11,
                total: 1500,
                active: 1500,
                valid: 1000,
                unlocking: vec![],
                claimed_rewards: vec![]
            })
        );

        // Stake limit should work
        // Give account 11 some large free balance greater than total
        let _ = Balances::make_free_balance_be(&11, 1000000);

        // Call the bond_extra function from controller, add only 100
        assert_ok!(Staking::bond_extra(Origin::signed(11), u64::max_value()));
        // There should be at most 2000 limit
        assert_eq!(
            Staking::ledger(&10),
            Some(StakingLedger {
                stash: 11,
                total: 2000,
                active: 2000,
                valid: 1000,
                unlocking: vec![],
                claimed_rewards: vec![]
            })
        );

        assert_ok!(Staking::validate(Origin::signed(10), Perbill::default()));
        start_era(1, false);

        assert_eq!(
            Staking::ledger(&10),
            Some(StakingLedger {
                stash: 11,
                total: 2000,
                active: 2000,
                valid: 2000,
                unlocking: vec![],
                claimed_rewards: vec![]
            })
        );
    });
}

#[test]
fn bond_extra_and_withdraw_unbonded_works() {
    // * Should test
    // * Given an account being bonded [and chosen as a validator](not mandatory)
    // * It can add extra funds to the bonded account.
    // * it can unbond a portion of its funds from the stash account.
    // * Once the unbonding period is done, it can actually take the funds out of the stash.
    ExtBuilder::default()
        .guarantee(false)
        .build()
        .execute_with(|| {
            // Set payee to controller. avoids confusion
            assert_ok!(Staking::set_payee(
                Origin::signed(10),
                RewardDestination::Controller
            ));

            // Give account 11 some large free balance greater than total
            let _ = Balances::make_free_balance_be(&11, 1000000);

            // Initial config should be correct
            assert_eq!(Staking::current_era().unwrap_or(0), 0);
            assert_eq!(Session::current_index(), 0);

            // check the balance of a validator accounts.
            assert_eq!(Balances::total_balance(&10), 1);

            // confirm that 10 is a normal validator and gets paid at the end of the era.
            start_era(1, false);

            // Initial state of 10
            assert_eq!(
                Staking::ledger(&10),
                Some(StakingLedger {
                    stash: 11,
                    total: 1000,
                    active: 1000,
                    valid: 1000,
                    unlocking: vec![],
                    claimed_rewards: vec![]
                })
            );
            assert_eq!(
                Staking::stakers(&11),
                Exposure {
                    total: 1000,
                    own: 1000,
                    others: vec![]
                }
            );

            // deposit the extra 100 units
            Staking::bond_extra(Origin::signed(11), 100).unwrap();

            assert_eq!(
                Staking::ledger(&10),
                Some(StakingLedger {
                    stash: 11,
                    total: 1000 + 100,
                    active: 1000 + 100,
                    valid: 1000,
                    unlocking: vec![],
                    claimed_rewards: vec![]
                })
            );
            // Exposure is a snapshot! only updated after the next era update.
            assert_ne!(
                Staking::stakers(&11),
                Exposure {
                    total: 1000 + 100,
                    own: 1000 + 100,
                    others: vec![]
                }
            );

            // trigger next era.
            Timestamp::set_timestamp(10);
            start_era(2, false);
            assert_eq!(Staking::current_era().unwrap_or(0), 2);

            // ledger should be the same.
            assert_eq!(
                Staking::ledger(&10),
                Some(StakingLedger {
                    stash: 11,
                    total: 1000 + 100,
                    active: 1000 + 100,
                    valid: 1000 + 100, // stake limit is 3000 right now
                    unlocking: vec![],
                    claimed_rewards: vec![]
                })
            );
            // Exposure is now updated.
            assert_eq!(
                Staking::stakers(&11),
                Exposure {
                    total: 1100,
                    own: 1100,
                    others: vec![]
                }
            );

            // Unbond almost all of the funds in stash.
            Staking::unbond(Origin::signed(10), 1000).unwrap();
            assert_eq!(
                Staking::ledger(&10),
                Some(StakingLedger {
                    stash: 11,
                    total: 1000 + 100,
                    active: 100,
                    valid: 100,
                    unlocking: vec![UnlockChunk {
                        value: 1000,
                        era: 2 + 3
                    }],
                    claimed_rewards: vec![]
                })
            );

            // Attempting to free the balances now will fail. 2 eras need to pass.
            Staking::withdraw_unbonded(Origin::signed(10)).unwrap();
            assert_eq!(
                Staking::ledger(&10),
                Some(StakingLedger {
                    stash: 11,
                    total: 1000 + 100,
                    active: 100,
                    valid: 100,
                    unlocking: vec![UnlockChunk {
                        value: 1000,
                        era: 2 + 3
                    }],
                    claimed_rewards: vec![]
                })
            );

            // trigger next era.
            start_era(3, false);

            // nothing yet
            Staking::withdraw_unbonded(Origin::signed(10)).unwrap();
            assert_eq!(
                Staking::ledger(&10),
                Some(StakingLedger {
                    stash: 11,
                    total: 1000 + 100,
                    active: 100,
                    valid: 100,
                    unlocking: vec![UnlockChunk {
                        value: 1000,
                        era: 2 + 3
                    }],
                    claimed_rewards: vec![]
                })
            );

            // trigger next era.
            start_era(5, false);

            Staking::withdraw_unbonded(Origin::signed(10)).unwrap();
            // Now the value is free and the staking ledger is updated.
            assert_eq!(
                Staking::ledger(&10),
                Some(StakingLedger {
                    stash: 11,
                    total: 100,
                    active: 100,
                    valid: 100,
                    unlocking: vec![],
                    claimed_rewards: vec![]
                })
            );
        })
}

#[test]
fn too_many_unbond_calls_should_not_work() {
    ExtBuilder::default().build().execute_with(|| {
        // locked at era 0 until 3
        for _ in 0..MAX_UNLOCKING_CHUNKS - 1 {
            assert_ok!(Staking::unbond(Origin::signed(10), 1));
        }

        start_era(1, false);

        // locked at era 1 until 4
        assert_ok!(Staking::unbond(Origin::signed(10), 1));
        // can't do more.
        assert_noop!(
            Staking::unbond(Origin::signed(10), 1),
            Error::<Test>::NoMoreChunks
        );

        start_era(3, false);

        assert_noop!(
            Staking::unbond(Origin::signed(10), 1),
            Error::<Test>::NoMoreChunks
        );
        // free up.
        assert_ok!(Staking::withdraw_unbonded(Origin::signed(10)));

        // Can add again.
        assert_ok!(Staking::unbond(Origin::signed(10), 1));
        assert_eq!(Staking::ledger(&10).unwrap().unlocking.len(), 2);
    })
}

#[test]
// TODO: tmp comment for duplicate to other test cases
/*fn total_stakes_is_least_staked_validator_and_exposure_defines_maximum_punishment() {
    // Test that total_stakes is determined by the least staked validator
    // Test that total_stakes is the maximum punishment that can happen to a validator
    ExtBuilder::default()
        .guarantee(false)
        .fair(false)
        .build()
        .execute_with(|| {
            // Confirm validator count is 2
            assert_eq!(Staking::validator_count(), 2);
            // Confirm account 10 and 20 are validators
            assert!(<Validators<Test>>::contains_key(&11) && <Validators<Test>>::contains_key(&21));

            assert_eq!(Staking::stakers(&11).total, 1000);
            assert_eq!(Staking::stakers(&21).total, 2000);

            // Give the man some money.
            let _ = Balances::make_free_balance_be(&10, 1000);
            let _ = Balances::make_free_balance_be(&20, 1000);

            // We confirm initialized total_stakes is this value
            assert_eq!(Staking::total_stakes(), 2000 + 1000 + 1);

            // Now lets lower account 20 stake
            <Stakers<Test>>::insert(
                &21,
                Exposure {
                    total: 69,
                    own: 69,
                    others: vec![],
                },
            );
            assert_eq!(Staking::stakers(&21).total, 69);
            <Ledger<Test>>::insert(
                &20,
                StakingLedger {
                    stash: 22,
                    total: 69,
                    active: 69,
                    valid: 69,
                    unlocking: vec![],
                },
            );

            // Compute total payout now for whole duration as other parameter won't change
            let total_authoring_payout_0 = Staking::authoring_rewards_in_era();
            let total_staking_payout_0 = Staking::staking_rewards_in_era(Staking::current_era().unwrap_or(0));
            assert!(total_staking_payout_0 > 100); // Test is meaningfull if reward something
            <Module<Test>>::reward_by_ids(vec![(11, 1)]);
            <Module<Test>>::reward_by_ids(vec![(21, 1)]);

            // New era --> rewards are paid --> stakes are changed
            start_era(1, true);

            // -- new balances + reward
            // 11's stake limit is 45454556881, 21's stake limit is 4000
            // round to 0.000001
            assert_eq!(Staking::stakers(&11).total / 1000000,
                       (1000 + total_authoring_payout_0 / 2 + total_staking_payout_0 * 1000 / 3001) / 1000000);
            assert_eq!(Staking::stakers(&21).total, 4000);

            let _11_balance = Balances::free_balance(&11);
            assert_eq!(_11_balance / 1000000,
                       (1000 + total_authoring_payout_0 / 2 + total_staking_payout_0 * 1000 / 3001) / 1000000);

            // -- slot stake should also be updated.
            assert_eq!(Staking::total_stakes(), 4000);

            check_exposure_all();
            check_guarantor_all();
        });
}*/

#[test]
fn on_free_balance_zero_stash_removes_validator() {
    // Tests that validator storage items are cleaned up when stash is empty
    // Tests that storage items are untouched when controller is empty
    ExtBuilder::default()
        .existential_deposit(10)
        .build()
        .execute_with(|| {
            // Check the balance of the validator account
            assert_eq!(Balances::free_balance(&10), 256);
            // Check the balance of the stash account
            assert_eq!(Balances::free_balance(&11), 256000);
            // Check these two accounts are bonded
            assert_eq!(Staking::bonded(&11), Some(10));

            // Set some storage items which we expect to be cleaned up
            // Set payee information
            assert_ok!(Staking::set_payee(
                Origin::signed(10),
                RewardDestination::Stash
            ));

            // Check storage items that should be cleaned up
            assert!(<Ledger<Test>>::contains_key(&10));
            assert!(<Bonded<Test>>::contains_key(&11));
            assert!(<Validators<Test>>::contains_key(&11));
            assert!(<Payee<Test>>::contains_key(&11));

            // Reduce free_balance of controller to 0
            let _ = Balances::slash(&10, u64::max_value());

            // Check the balance of the stash account has not been touched
            assert_eq!(Balances::free_balance(&11), 256000);
            // Check these two accounts are still bonded
            assert_eq!(Staking::bonded(&11), Some(10));

            // Check storage items have not changed
            assert!(<Ledger<Test>>::contains_key(&10));
            assert!(<Bonded<Test>>::contains_key(&11));
            assert!(<Validators<Test>>::contains_key(&11));
            assert!(<Payee<Test>>::contains_key(&11));

            // Reduce free_balance of stash to 0
            let _ = Balances::slash(&11, u64::max_value());
            // Check total balance of stash
            assert_eq!(Balances::total_balance(&11), 0);

            // Reap the stash
            assert_ok!(Staking::reap_stash(Origin::none(), 11));

            // Check storage items do not exist
            assert!(!<Ledger<Test>>::contains_key(&10));
            assert!(!<Bonded<Test>>::contains_key(&11));
            assert!(!<Validators<Test>>::contains_key(&11));
            assert!(!<Guarantors<Test>>::contains_key(&11));
            assert!(!<Payee<Test>>::contains_key(&11));
        });
}

#[test]
fn on_free_balance_zero_stash_removes_guarantor() {
    // Tests that guarantor storage items are cleaned up when stash is empty
    // Tests that storage items are untouched when controller is empty
    ExtBuilder::default()
        .existential_deposit(10)
        .build()
        .execute_with(|| {
            // Make 10 a guarantor
            assert_ok!(Staking::guarantee(Origin::signed(10), (21, 100)));
            // Check that account 10 is a guarantor
            assert!(<Guarantors<Test>>::contains_key(11));
            // Check the balance of the guarantor account
            assert_eq!(Balances::free_balance(&10), 256);
            // Check the balance of the stash account
            assert_eq!(Balances::free_balance(&11), 256000);

            // Set payee information
            assert_ok!(Staking::set_payee(
                Origin::signed(10),
                RewardDestination::Stash
            ));

            // Check storage items that should be cleaned up
            assert!(<Ledger<Test>>::contains_key(&10));
            assert!(<Bonded<Test>>::contains_key(&11));
            assert!(<Guarantors<Test>>::contains_key(&11));
            assert!(<Payee<Test>>::contains_key(&11));

            // Reduce free_balance of controller to 0
            let _ = Balances::slash(&10, u64::max_value());
            // Check total balance of account 10
            assert_eq!(Balances::total_balance(&10), 0);

            // Check the balance of the stash account has not been touched
            assert_eq!(Balances::free_balance(&11), 256000);
            // Check these two accounts are still bonded
            assert_eq!(Staking::bonded(&11), Some(10));

            // Check storage items have not changed
            assert!(<Ledger<Test>>::contains_key(&10));
            assert!(<Bonded<Test>>::contains_key(&11));
            assert!(<Guarantors<Test>>::contains_key(&11));
            assert!(<Payee<Test>>::contains_key(&11));

            // Reduce free_balance of stash to 0
            let _ = Balances::slash(&11, u64::max_value());
            // Check total balance of stash
            assert_eq!(Balances::total_balance(&11), 0);

            // Reap the stash
            assert_ok!(Staking::reap_stash(Origin::none(), 11));

            // Check storage items do not exist
            assert!(!<Ledger<Test>>::contains_key(&10));
            assert!(!<Bonded<Test>>::contains_key(&11));
            assert!(!<Validators<Test>>::contains_key(&11));
            assert!(!<Guarantors<Test>>::contains_key(&11));
            assert!(!<Payee<Test>>::contains_key(&11));
        });
}

#[test]
fn switching_roles() {
    // Test that it should be possible to switch between roles (guarantor, validator, idle) with minimal overhead.
    ExtBuilder::default()
        .guarantee(false)
        .build()
        .execute_with(|| {
            Staking::upsert_stake_limit(&5, 3000);

            // Reset reward destination
            for i in &[10, 20] {
                assert_ok!(Staking::set_payee(
                    Origin::signed(*i),
                    RewardDestination::Controller
                ));
            }

            assert_eq_uvec!(validator_controllers(), vec![20, 10]);

            // put some money in account that we'll use.
            for i in 1..7 {
                let _ = Balances::deposit_creating(&i, 5000);
            }

            // add a new validator candidate
            assert_ok!(Staking::bond(
                Origin::signed(5),
                6,
                1000,
                RewardDestination::Controller
            ));
            assert_ok!(Staking::validate(Origin::signed(6), Perbill::default()));

            // add 2 guarantors
            assert_ok!(Staking::bond(
                Origin::signed(1),
                2,
                2000,
                RewardDestination::Controller
            ));
            assert_ok!(Staking::guarantee(
                Origin::signed(2),
                (11, 1000)
            ));
            assert_ok!(Staking::guarantee(
                Origin::signed(2),
                (5, 1000)
            ));

            assert_ok!(Staking::bond(
                Origin::signed(3),
                4,
                500,
                RewardDestination::Controller
            ));
            assert_ok!(Staking::guarantee(
                Origin::signed(4),
                (21, 250)
            ));
            assert_noop!(
                Staking::guarantee(Origin::signed(4), (1, 250)), // 1 is not validator
                DispatchError::Module {
                    index: 0,
                    error: 6,
                    message: Some("InvalidTarget"),
                }
            );

            // new block
            start_session(1, false);

            // no change
            assert_eq_uvec!(validator_controllers(), vec![20, 10]);

            // new block
            start_session(2, false);

            // no change
            assert_eq_uvec!(validator_controllers(), vec![20, 10]);

            // new block --> ne era --> new validators
            start_session(3, false);

            // with current guarantors 10 and 5 have the most stake
            assert_eq_uvec!(validator_controllers(), vec![6, 10]);

            // 2 decides to be a validator. Consequences:
            assert_ok!(Staking::validate(Origin::signed(2), Perbill::default()));
            // new stakes:
            // 10: 1000 self vote
            // 20: 1000 self vote + 250 vote
            // 6 : 1000 self vote
            // 2 : 2000 self vote + 250 vote.
            // Winners: 20 and 2

            start_session(4, false);
            assert_eq_uvec!(validator_controllers(), vec![6, 10]);

            start_session(5, false);
            assert_eq_uvec!(validator_controllers(), vec![6, 10]);

            // ne era
            start_session(6, false);
            assert_eq_uvec!(validator_controllers(), vec![2, 20]);

            check_exposure_all();
            check_guarantor_all();
        });
}

#[test]
fn wrong_vote_is_null() {
    ExtBuilder::default()
        .guarantee(false)
        .validator_pool(true)
        .build()
        .execute_with(|| {
            assert_eq_uvec!(validator_controllers(), vec![40, 30]);

            // put some money in account that we'll use.
            for i in 1..3 {
                let _ = Balances::deposit_creating(&i, 5000);
            }

            // add 1 guarantors
            assert_ok!(Staking::bond(
                Origin::signed(1),
                2,
                2000,
                RewardDestination::default()
            ));
            assert_ok!(Staking::guarantee(
                Origin::signed(2),
                (11, 500)
            ));
            assert_ok!(Staking::guarantee(
                Origin::signed(2),
                (21, 500)
            ));
            assert_noop!(
                Staking::guarantee(Origin::signed(2), (1, 50)), // 1 is not validtor
                DispatchError::Module {
                    index: 0,
                    error: 6,
                    message: Some("InvalidTarget"),
                }
            );
            assert_noop!(
                Staking::guarantee(Origin::signed(2), (2, 50)), // 2 cannot gurantee itself
                DispatchError::Module {
                    index: 0,
                    error: 6,
                    message: Some("InvalidTarget"),
                }
            );
            assert_noop!(
                Staking::guarantee(Origin::signed(2), (15, 50)), // 15 doesn't exist
                DispatchError::Module {
                    index: 0,
                    error: 6,
                    message: Some("InvalidTarget"),
                }
            );

            // new block
            start_era(1, false);

            assert_eq_uvec!(validator_controllers(), vec![20, 10]);
        });
}

#[test]
fn bond_with_no_staked_value() {
    // Behavior when someone bonds with no staked value.
    // Particularly when she votes and the candidate is elected.
    ExtBuilder::default()
        .validator_count(3)
        .existential_deposit(5)
        .guarantee(false)
        .minimum_validator_count(1)
        .build()
        .execute_with(|| {
            // Can't bond with 1
            assert_noop!(
                Staking::bond(Origin::signed(1), 2, 1, RewardDestination::Controller),
                Error::<Test>::InsufficientValue,
            );
            // bonded with absolute minimum value possible.
            assert_ok!(Staking::bond(
                Origin::signed(1),
                2,
                5,
                RewardDestination::Controller
            ));
            assert_eq!(Balances::locks(&1)[0].amount, 5);

            // unbonding even 1 will cause all to be unbonded.
            assert_ok!(Staking::unbond(Origin::signed(2), 1));
            assert_eq!(
                Staking::ledger(2),
                Some(StakingLedger {
                    stash: 1,
                    active: 0,
                    total: 5,
                    valid: 0,
                    unlocking: vec![UnlockChunk { value: 5, era: 3 }],
                    claimed_rewards: vec![]
                })
            );

            start_era(1, false);
            start_era(2, false);

            // not yet removed.
            assert_ok!(Staking::withdraw_unbonded(Origin::signed(2)));
            assert!(Staking::ledger(2).is_some());
            assert_eq!(Balances::locks(&1)[0].amount, 5);

            start_era(3, false);

            // poof. Account 1 is removed from the staking system.
            assert_ok!(Staking::withdraw_unbonded(Origin::signed(2)));
            assert!(Staking::ledger(2).is_none());
            assert_eq!(Balances::locks(&1).len(), 0);
        });
}

#[test]
fn bond_with_little_staked_value_bounded_by_total_stakes() {
    // Behavior when someone bonds with little staked value.
    // Particularly when she votes and the candidate is elected.
    ExtBuilder::default()
        .validator_count(3)
        .guarantee(false)
        .own_workload(u128::max_value())
        .minimum_validator_count(1)
        .build()
        .execute_with(|| {
            // setup
            assert_ok!(Staking::chill(Origin::signed(30)));
            assert_ok!(Staking::set_payee(
                Origin::signed(10),
                RewardDestination::Controller
            ));
            let init_balance_2 = Balances::free_balance(&2);
            let init_balance_10 = Balances::free_balance(&10);

            // Stingy validator.
            assert_ok!(Staking::bond(
                Origin::signed(1),
                2,
                2,
                RewardDestination::Controller
            ));
            Staking::upsert_stake_limit(&1, u64::max_value());
            assert_ok!(Staking::validate(Origin::signed(2), Perbill::default()));

            let total_staking_payout_0 = Staking::staking_rewards_in_era(Staking::current_era().unwrap_or(0));
            let total_authoring_payout = Staking::authoring_rewards_in_era();
            assert_eq!(total_staking_payout_0, 285192790326260);
            assert_eq!(total_authoring_payout, 5703855806525);
            reward_all_elected();
            start_era(1, true);

            // 2 is elected.
            // and fucks up the slot stake.
            assert_eq_uvec!(validator_controllers(), vec![20, 10, 2]);
            assert_eq!(Staking::eras_total_stakes(1), 2002);
            payout_all_stakers(0);
            Staking::reward_stakers(Origin::signed(10), 1, 0).unwrap();
            // Old ones are rewarded, round to 0.000001 CRU
            assert_eq!(
                Balances::free_balance(&10) / 1000000,
                (init_balance_10 + total_authoring_payout / 3 + total_staking_payout_0 * 1000 / 2001) / 1000000
            );
            // no rewards paid to 2. This was initial election.
            assert_eq!(Balances::free_balance(&2), init_balance_2);

            let total_staking_payout_1 = Staking::staking_rewards_in_era(Staking::current_era().unwrap_or(0));
            assert_eq!(total_staking_payout_1, 285192790326260); // Test is meaningful if reward something
            reward_all_elected();
            start_era(2, true);

            assert_eq_uvec!(validator_controllers(), vec![20, 10, 2]);
            // Stake limit strategy effecting
            assert_eq!(Staking::eras_total_stakes(2), /*291396645848756*/ 144426417598396);
            payout_all_stakers(1);
            Staking::reward_stakers(Origin::signed(10), 1, 1).unwrap();
            // round to 0.000001 CRU
            assert_eq!(
                Balances::free_balance(&2) / 1000000,
                (init_balance_2 + total_authoring_payout / 3 + total_staking_payout_1 * 2 / 2002) / 1000000 - 1,
            );
            // round to 0.000001 CRU
            assert_eq!(
                Balances::free_balance(&10) / 1000000,
                (init_balance_10 + total_authoring_payout / 3 * 2 +
                    total_staking_payout_0 * 1000 / 2001 + total_staking_payout_1 * 1000 / 2002) / 1000000 - 1,
            );
            check_exposure_all();
            check_guarantor_all();
        });
}

#[test]
fn new_era_elects_correct_number_of_validators() {
    ExtBuilder::default()
        .guarantee(true)
        .validator_pool(true)
        .fair(true)
        .validator_count(1)
        .build()
        .execute_with(|| {
            assert_eq!(Staking::validator_count(), 1);
            assert_eq!(validator_controllers().len(), 1);

            System::set_block_number(1);
            Session::on_initialize(System::block_number());

            assert_eq!(validator_controllers().len(), 1);
            check_exposure_all();
            check_guarantor_all();
        })
}

#[test]
fn topdown_should_not_overflow_validators() {
    ExtBuilder::default()
        .guarantee(false)
        .own_workload(u128::max_value())
        .total_workload(1)
        .build()
        .execute_with(|| {
            let _ = Staking::chill(Origin::signed(10));
            let _ = Staking::chill(Origin::signed(20));

            bond_validator(2, u64::max_value());
            bond_validator(4, u64::max_value());

            // TODO: this will broken the stake limit of mock set
            start_era(1, false);

            assert_eq_uvec!(validator_controllers(), vec![4, 2]);

            // This test will fail this. Will saturate.
            // check_exposure_all();
            assert_eq!(Staking::stakers(3).total, 18446744073709551615);
            assert_eq!(Staking::stakers(5).total, 18446744073709551615);
        })
}

#[test]
fn topdown_should_not_overflow_guarantors() {
    ExtBuilder::default()
        .guarantee(false)
        .own_workload(u128::max_value())
        .total_workload(1)
        .build()
        .execute_with(|| {
            let _ = Staking::chill(Origin::signed(10));
            let _ = Staking::chill(Origin::signed(20));

            bond_validator(2, u64::max_value() / 8);
            bond_validator(4, u64::max_value() / 8);

            start_era(1, false);

            assert_eq_uvec!(validator_controllers(), vec![4, 2]);

            // Saturate.
            // `new_era` will update stake limit
            assert_eq!(Staking::stakers(3).total, u64::max_value() / 8);
            assert_eq!(Staking::stakers(5).total, u64::max_value() / 8);

            bond_guarantor(6,
                u64::max_value(),
                vec![(3, u64::max_value() / 2), (5, u64::max_value() / 2)]);
            bond_guarantor(8,
                u64::max_value(),
                vec![(3, u64::max_value() / 2), (5, u64::max_value() / 2)]);
            start_era(2, false);

            assert_eq_uvec!(validator_controllers(), vec![4, 2]);
        })
}

#[test]
fn reward_validator_slashing_validator_doesnt_overflow() {
    ExtBuilder::default().build().execute_with(|| {
        let stake = u32::max_value() as u64 * 2;
        let reward_slash = u32::max_value() as u64 * 2;

        // Assert multiplication overflows in balance arithmetic.
        assert!(stake.checked_mul(reward_slash).is_none());

        // Set staker
        let _ = Balances::make_free_balance_be(&11, stake);
        <ErasStakers<Test>>::insert(
            0,
            &11,
            Exposure {
                total: stake,
                own: stake,
                others: vec![],
            },
        );

        <ErasStakingPayout<Test>>::insert(
            0,
            reward_slash
        );

        // Check reward
        let _ = Staking::reward_stakers(Origin::signed(10), 11, 0);
        assert_eq!(Balances::total_balance(&11), stake * 2);

        // Set staker
        let _ = Balances::make_free_balance_be(&11, stake);
        let _ = Balances::make_free_balance_be(&2, stake);

        // only slashes out of bonded stake are applied. without this line,
        // it is 0.
        Staking::bond(
            Origin::signed(2),
            20000,
            stake - 1,
            RewardDestination::default(),
        )
        .unwrap();
        <ErasStakers<Test>>::insert(
            0,
            &11,
            Exposure {
                total: stake,
                own: 1,
                others: vec![IndividualExposure {
                    who: 2,
                    value: stake - 1,
                }],
            },
        );

        // Check slashing
        on_offence_now(
            &[OffenceDetails {
                offender: (11, Staking::eras_stakers(0, &11)),
                reporters: vec![],
            }],
            &[Perbill::from_percent(100)],
        );

        assert_eq!(Balances::total_balance(&11), stake - 1);
        assert_eq!(Balances::total_balance(&2), 1);
    })
}

#[test]
fn reward_from_authorship_event_handler_works() {
    ExtBuilder::default().build().execute_with(|| {
        use pallet_authorship::EventHandler;

        assert_eq!(<pallet_authorship::Module<Test>>::author(), 11);

        <Module<Test>>::note_author(11);
        <Module<Test>>::note_uncle(21, 1);
        // An uncle author that is not currently elected doesn't get rewards,
        // but the block producer does get reward for referencing it.
        <Module<Test>>::note_uncle(31, 1);
        // Rewarding the same two times works.
        <Module<Test>>::note_uncle(11, 1);

        // Not mandatory but must be coherent with rewards
        assert_eq!(<CurrentElected<Test>>::get(), vec![21, 11]);

        // 21 is rewarded as an uncle producer
        // 11 is rewarded as a block producer and uncle referencer and uncle producer
        assert_eq!(
            CurrentEraPointsEarned::get().individual,
            vec![1, 20 + 2 * 3 + 1]
        );
        assert_eq!(CurrentEraPointsEarned::get().total, 28);
    })
}

#[test]
fn add_reward_points_fns_works() {
    ExtBuilder::default().build().execute_with(|| {
        let validators = <Module<Test>>::current_elected();
        // Not mandatory but must be coherent with rewards
        assert_eq!(validators, vec![21, 11]);

        <Module<Test>>::reward_by_indices(vec![(0, 1), (1, 1), (2, 1), (1, 1)]);

        <Module<Test>>::reward_by_ids(vec![(21, 1), (11, 1), (31, 1), (11, 1)]);

        assert_eq!(CurrentEraPointsEarned::get().individual, vec![2, 4]);
        assert_eq!(CurrentEraPointsEarned::get().total, 6);
    })
}

#[test]
fn unbonded_balance_is_not_slashable() {
    ExtBuilder::default().build().execute_with(|| {
        // total amount staked is slashable.
        assert_eq!(Staking::slashable_balance_of(&11), 1000);

        assert_ok!(Staking::unbond(Origin::signed(10), 800));

        // only the active portion.
        assert_eq!(Staking::slashable_balance_of(&11), 200);
    })
}

#[test]
fn era_is_always_same_length() {
    // This ensures that the sessions is always of the same length if there is no forcing no
    // session changes.
    ExtBuilder::default().build().execute_with(|| {
        start_era(1, false);
        assert_eq!(
            Staking::current_era_start_session_index(),
            SessionsPerEra::get()
        );

        start_era(2, false);
        assert_eq!(
            Staking::current_era_start_session_index(),
            SessionsPerEra::get() * 2
        );

        let session = Session::current_index();
        ForceEra::put(Forcing::ForceNew);
        advance_session();
        assert_eq!(Staking::current_era().unwrap_or(0), 3);
        assert_eq!(Staking::current_era_start_session_index(), session + 1);

        start_era(4, false);
        assert_eq!(
            Staking::current_era_start_session_index(),
            session + SessionsPerEra::get() + 1
        );
    });
}

#[test]
fn offence_forces_new_era() {
    ExtBuilder::default().build().execute_with(|| {
        on_offence_now(
            &[OffenceDetails {
                offender: (11, Staking::stakers(&11)),
                reporters: vec![],
            }],
            &[Perbill::from_percent(5)],
        );

        assert_eq!(Staking::force_era(), Forcing::ForceNew);
    });
}

#[test]
fn offence_ensures_new_era_without_clobbering() {
    ExtBuilder::default().build().execute_with(|| {
        assert_ok!(Staking::force_new_era_always(Origin::root()));

        on_offence_now(
            &[OffenceDetails {
                offender: (11, Staking::stakers(&11)),
                reporters: vec![],
            }],
            &[Perbill::from_percent(5)],
        );

        assert_eq!(Staking::force_era(), Forcing::ForceAlways);
    });
}

#[test]
fn offence_deselects_validator_when_slash_is_zero() {
    ExtBuilder::default().build().execute_with(|| {
        assert!(<Validators<Test>>::contains_key(11));
        on_offence_now(
            &[OffenceDetails {
                offender: (11, Staking::stakers(&11)),
                reporters: vec![],
            }],
            &[Perbill::from_percent(0)],
        );
        assert_eq!(Staking::force_era(), Forcing::ForceNew);
        assert!(!<Validators<Test>>::contains_key(11));
    });
}

#[test]
fn slashing_performed_according_exposure() {
    // This test checks that slashing is performed according the exposure (or more precisely,
    // historical exposure), not the current balance.
    ExtBuilder::default().build().execute_with(|| {
        assert_eq!(Staking::stakers(&11).own, 1000);

        // Handle an offence with a historical exposure.
        on_offence_now(
            &[OffenceDetails {
                offender: (
                    11,
                    Exposure {
                        total: 500,
                        own: 500,
                        others: vec![],
                    },
                ),
                reporters: vec![],
            }],
            &[Perbill::from_percent(50)],
        );

        // The stash account should be slashed for 250 (50% of 500).
        assert_eq!(Balances::free_balance(&11), 1000 - 250);
    });
}

#[test]
fn slash_in_old_span_does_not_deselect() {
    ExtBuilder::default()
    .build()
    .execute_with(|| {
        start_era(1, false);

        assert!(<Validators<Test>>::contains_key(11));
        on_offence_now(
            &[OffenceDetails {
                offender: (11, Staking::stakers(&11)),
                reporters: vec![],
            }],
            &[Perbill::from_percent(0)],
        );
        assert_eq!(Staking::force_era(), Forcing::ForceNew);
        assert!(!<Validators<Test>>::contains_key(11));

        // ForceNew, this will trigger update on CurrentEraStartSessionIndex, this will not
        // trigger wr outdated, so we actually won't support ForceNew strategy
        start_era(2, false);

        Staking::validate(Origin::signed(10), Default::default()).unwrap();
        assert_eq!(Staking::force_era(), Forcing::NotForcing);
        assert!(<Validators<Test>>::contains_key(11));

        start_era(3, false);

        // this staker is in a new slashing span now, having re-registered after
        // their prior slash.

        on_offence_in_era(
            &[OffenceDetails {
                offender: (11, Staking::stakers(&11)),
                reporters: vec![],
            }],
            &[Perbill::from_percent(0)],
            1,
        );

        // not for zero-slash.
        assert_eq!(Staking::force_era(), Forcing::NotForcing);
        assert!(<Validators<Test>>::contains_key(11));

        on_offence_in_era(
            &[OffenceDetails {
                offender: (11, Staking::stakers(&11)),
                reporters: vec![],
            }],
            &[Perbill::from_percent(100)],
            1,
        );

        // or non-zero.
        assert_eq!(Staking::force_era(), Forcing::NotForcing);
        assert!(<Validators<Test>>::contains_key(11));
        assert_ledger_consistent(11);
    });
}

#[test]
fn reporters_receive_their_slice() {
    // This test verifies that the reporters of the offence receive their slice from the slashed
    // amount.
    ExtBuilder::default().build().execute_with(|| {
        // The reporters' reward is calculated from the total exposure.
        #[cfg(feature = "equalize")]
        let initial_balance = 1250;
        #[cfg(not(feature = "equalize"))]
        let initial_balance = 1125;

        assert_eq!(Staking::stakers(&11).total, initial_balance);

        on_offence_now(
            &[OffenceDetails {
                offender: (11, Staking::stakers(&11)),
                reporters: vec![1, 2],
            }],
            &[Perbill::from_percent(50)],
        );

        // F1 * (reward_proportion * slash - 0)
        // 50% * (10% * initial_balance / 2)
        let reward = (initial_balance / 20) / 2;
        let reward_each = reward / 2; // split into two pieces.
        assert_eq!(Balances::free_balance(&1), 10 + reward_each);
        assert_eq!(Balances::free_balance(&2), 20 + reward_each);
        assert_ledger_consistent(11);
    });
}

#[test]
fn subsequent_reports_in_same_span_pay_out_less() {
    // This test verifies that the reporters of the offence receive their slice from the slashed
    // amount.
    ExtBuilder::default().build().execute_with(|| {
        // The reporters' reward is calculated from the total exposure.
        #[cfg(feature = "equalize")]
        let initial_balance = 1250;
        #[cfg(not(feature = "equalize"))]
        let initial_balance = 1125;

        assert_eq!(Staking::stakers(&11).total, initial_balance);

        on_offence_now(
            &[OffenceDetails {
                offender: (11, Staking::stakers(&11)),
                reporters: vec![1],
            }],
            &[Perbill::from_percent(20)],
        );

        // F1 * (reward_proportion * slash - 0)
        // 50% * (10% * initial_balance * 20%)
        let reward = (initial_balance / 5) / 20;
        assert_eq!(Balances::free_balance(&1), 10 + reward);

        on_offence_now(
            &[OffenceDetails {
                offender: (11, Staking::stakers(&11)),
                reporters: vec![1],
            }],
            &[Perbill::from_percent(50)],
        );

        let prior_payout = reward;

        // F1 * (reward_proportion * slash - prior_payout)
        // 50% * (10% * (initial_balance / 2) - prior_payout)
        let reward = ((initial_balance / 20) - prior_payout) / 2;
        assert_eq!(Balances::free_balance(&1), 10 + prior_payout + reward);
        assert_ledger_consistent(11);
    });
}

#[test]
fn invulnerables_are_not_slashed() {
    // For invulnerable validators no slashing is performed.
    ExtBuilder::default()
        .invulnerables(vec![11])
        .build()
        .execute_with(|| {
            assert_eq!(Balances::free_balance(&11), 1000);
            assert_eq!(Balances::free_balance(&21), 2000);

            let exposure = Staking::stakers(&21);
            let initial_balance = Staking::slashable_balance_of(&21);

            let guarantor_balances: Vec<_> = exposure
                .others
                .iter()
                .map(|o| Balances::free_balance(&o.who))
                .collect();

            on_offence_now(
                &[
                    OffenceDetails {
                        offender: (11, Staking::stakers(&11)),
                        reporters: vec![],
                    },
                    OffenceDetails {
                        offender: (21, Staking::stakers(&21)),
                        reporters: vec![],
                    },
                ],
                &[Perbill::from_percent(50), Perbill::from_percent(20)],
            );

            // The validator 11 hasn't been slashed, but 21 has been.
            assert_eq!(Balances::free_balance(&11), 1000);
            // 2000 - (0.2 * initial_balance)
            assert_eq!(
                Balances::free_balance(&21),
                2000 - (2 * initial_balance / 10)
            );

            // ensure that guarantors were slashed as well.
            for (initial_balance, other) in guarantor_balances.into_iter().zip(exposure.others) {
                assert_eq!(
                    Balances::free_balance(&other.who),
                    initial_balance - (2 * other.value / 10),
                );
            }
            assert_ledger_consistent(11);
            assert_ledger_consistent(21);
        });
}

#[test]
fn dont_slash_if_fraction_is_zero() {
    // Don't slash if the fraction is zero.
    ExtBuilder::default().build().execute_with(|| {
        assert_eq!(Balances::free_balance(&11), 1000);

        on_offence_now(
            &[OffenceDetails {
                offender: (11, Staking::stakers(&11)),
                reporters: vec![],
            }],
            &[Perbill::from_percent(0)],
        );

        // The validator hasn't been slashed. The new era is not forced.
        assert_eq!(Balances::free_balance(&11), 1000);
        assert_ledger_consistent(11);
    });
}

#[test]
fn only_slash_for_max_in_era() {
    ExtBuilder::default().build().execute_with(|| {
        assert_eq!(Balances::free_balance(&11), 1000);

        on_offence_now(
            &[OffenceDetails {
                offender: (11, Staking::stakers(&11)),
                reporters: vec![],
            }],
            &[Perbill::from_percent(50)],
        );

        // The validator has been slashed and has been force-chilled.
        assert_eq!(Balances::free_balance(&11), 500);
        assert_eq!(Staking::force_era(), Forcing::ForceNew);

        on_offence_now(
            &[OffenceDetails {
                offender: (11, Staking::stakers(&11)),
                reporters: vec![],
            }],
            &[Perbill::from_percent(25)],
        );

        // The validator has not been slashed additionally.
        assert_eq!(Balances::free_balance(&11), 500);

        on_offence_now(
            &[OffenceDetails {
                offender: (11, Staking::stakers(&11)),
                reporters: vec![],
            }],
            &[Perbill::from_percent(60)],
        );

        // The validator got slashed 10% more.
        assert_eq!(Balances::free_balance(&11), 400);
        assert_ledger_consistent(11);
    })
}

#[test]
fn garbage_collection_after_slashing() {
    ExtBuilder::default()
        .existential_deposit(2)
        .build()
        .execute_with(|| {
            assert_eq!(Balances::free_balance(&11), 256_000);

            on_offence_now(
                &[OffenceDetails {
                    offender: (11, Staking::stakers(&11)),
                    reporters: vec![],
                }],
                &[Perbill::from_percent(10)],
            );

            assert_eq!(Balances::free_balance(&11), 256_000 - 25_600);
            assert!(<Staking as crate::Store>::SlashingSpans::get(&11).is_some());
            assert_eq!(
                <Staking as crate::Store>::SpanSlash::get(&(11, 0)).amount_slashed(),
                &25_600
            );

            on_offence_now(
                &[OffenceDetails {
                    offender: (11, Staking::stakers(&11)),
                    reporters: vec![],
                }],
                &[Perbill::from_percent(100)],
            );

            // validator and guarantor slash in era are garbage-collected by era change,
            // so we don't test those here.

            assert_eq!(Balances::free_balance(&11), 0);
            assert_eq!(Balances::total_balance(&11), 0);

            assert_ok!(Staking::reap_stash(Origin::none(), 11));
            assert!(<Staking as crate::Store>::SlashingSpans::get(&11).is_none());
            assert_eq!(
                <Staking as crate::Store>::SpanSlash::get(&(11, 0)).amount_slashed(),
                &0
            );
        })
}

#[test]
fn garbage_collection_on_window_pruning() {
    ExtBuilder::default().build().execute_with(|| {
        start_era(1, false);

        assert_eq!(Balances::free_balance(&11), 1000);

        let exposure = Staking::stakers(&11);
        assert_eq!(Balances::free_balance(&101), 2000);
        let guaranteed_value = exposure.others.iter().find(|o| o.who == 101).unwrap().value;

        on_offence_now(
            &[OffenceDetails {
                offender: (11, Staking::stakers(&11)),
                reporters: vec![],
            }],
            &[Perbill::from_percent(10)],
        );

        let now = Staking::current_era().unwrap_or(0);

        assert_eq!(Balances::free_balance(&11), 900);
        assert_eq!(Balances::free_balance(&101), 2000 - (guaranteed_value / 10));

        assert!(<Staking as crate::Store>::ValidatorSlashInEra::get(&now, &11).is_some());
        assert!(<Staking as crate::Store>::GuarantorSlashInEra::get(&now, &101).is_some());

        // + 1 because we have to exit the bonding window.
        for era in (0..(BondingDuration::get() + 1)).map(|offset| offset + now + 1) {
            assert!(<Staking as crate::Store>::ValidatorSlashInEra::get(&now, &11).is_some());
            assert!(<Staking as crate::Store>::GuarantorSlashInEra::get(&now, &101).is_some());

            start_era(era, false);
        }

        assert!(<Staking as crate::Store>::ValidatorSlashInEra::get(&now, &11).is_none());
        assert!(<Staking as crate::Store>::GuarantorSlashInEra::get(&now, &101).is_none());
    })
}

#[test]
fn slashing_guarantors_by_span_max() {
    ExtBuilder::default().build().execute_with(|| {
        start_era(1, false);
        start_era(2, false);
        start_era(3, false);

        assert_eq!(Balances::free_balance(&11), 1000);
        assert_eq!(Balances::free_balance(&21), 2000);
        assert_eq!(Balances::free_balance(&101), 2000);
        assert_eq!(Staking::slashable_balance_of(&21), 1000);

        let exposure_11 = Staking::stakers(&11);
        let exposure_21 = Staking::stakers(&21);
        assert_eq!(Balances::free_balance(&101), 2000);
        let guaranteed_value_11 = exposure_11
            .others
            .iter()
            .find(|o| o.who == 101)
            .unwrap()
            .value;
        let guaranteed_value_21 = exposure_21
            .others
            .iter()
            .find(|o| o.who == 101)
            .unwrap()
            .value;

        on_offence_in_era(
            &[OffenceDetails {
                offender: (11, Staking::stakers(&11)),
                reporters: vec![],
            }],
            &[Perbill::from_percent(10)],
            2,
        );

        assert_eq!(Balances::free_balance(&11), 900);

        let slash_1_amount = Perbill::from_percent(10) * guaranteed_value_11;
        assert_eq!(Balances::free_balance(&101), 2000 - slash_1_amount);

        let expected_spans = vec![
            slashing::SlashingSpan {
                index: 1,
                start: 4,
                length: None,
            },
            slashing::SlashingSpan {
                index: 0,
                start: 0,
                length: Some(4),
            },
        ];

        let get_span = |account| <Staking as crate::Store>::SlashingSpans::get(&account).unwrap();

        assert_eq!(get_span(11).iter().collect::<Vec<_>>(), expected_spans,);

        assert_eq!(get_span(101).iter().collect::<Vec<_>>(), expected_spans,);

        // second slash: higher era, higher value, same span.
        on_offence_in_era(
            &[OffenceDetails {
                offender: (21, Staking::stakers(&21)),
                reporters: vec![],
            }],
            &[Perbill::from_percent(30)],
            3,
        );

        // 11 was not further slashed, but 21 and 101 were.
        assert_eq!(Balances::free_balance(&11), 900);
        assert_eq!(Balances::free_balance(&21), 1700);

        let slash_2_amount = Perbill::from_percent(30) * guaranteed_value_21;
        assert!(slash_2_amount > slash_1_amount);

        // only the maximum slash in a single span is taken.
        assert_eq!(Balances::free_balance(&101), 2000 - slash_2_amount);

        // third slash: in same era and on same validator as first, higher
        // in-era value, but lower slash value than slash 2.
        on_offence_in_era(
            &[OffenceDetails {
                offender: (11, Staking::stakers(&11)),
                reporters: vec![],
            }],
            &[Perbill::from_percent(20)],
            2,
        );

        // 11 was further slashed, but 21 and 101 were not.
        assert_eq!(Balances::free_balance(&11), 800);
        assert_eq!(Balances::free_balance(&21), 1700);

        let slash_3_amount = Perbill::from_percent(20) * guaranteed_value_21;
        assert!(slash_3_amount < slash_2_amount);
        assert!(slash_3_amount > slash_1_amount);

        // only the maximum slash in a single span is taken.
        assert_eq!(Balances::free_balance(&101), 2000 - slash_2_amount);
    });
}

#[test]
fn slashes_are_summed_across_spans() {
    ExtBuilder::default().build().execute_with(|| {
        start_era(1, false);
        start_era(2, false);
        start_era(3, false);

        assert_eq!(Balances::free_balance(&21), 2000);
        assert_eq!(Staking::slashable_balance_of(&21), 1000);

        let get_span = |account| <Staking as crate::Store>::SlashingSpans::get(&account).unwrap();

        on_offence_now(
            &[OffenceDetails {
                offender: (21, Staking::stakers(&21)),
                reporters: vec![],
            }],
            &[Perbill::from_percent(10)],
        );

        let expected_spans = vec![
            slashing::SlashingSpan {
                index: 1,
                start: 4,
                length: None,
            },
            slashing::SlashingSpan {
                index: 0,
                start: 0,
                length: Some(4),
            },
        ];

        assert_eq!(get_span(21).iter().collect::<Vec<_>>(), expected_spans);
        assert_eq!(Balances::free_balance(&21), 1900);

        // 21 has been force-chilled. re-signal intent to validate.
        Staking::validate(Origin::signed(20), Default::default()).unwrap();

        start_era(4, false);

        assert_eq!(Staking::slashable_balance_of(&21), 900);

        on_offence_now(
            &[OffenceDetails {
                offender: (21, Staking::stakers(&21)),
                reporters: vec![],
            }],
            &[Perbill::from_percent(10)],
        );

        let expected_spans = vec![
            slashing::SlashingSpan {
                index: 2,
                start: 5,
                length: None,
            },
            slashing::SlashingSpan {
                index: 1,
                start: 4,
                length: Some(1),
            },
            slashing::SlashingSpan {
                index: 0,
                start: 0,
                length: Some(4),
            },
        ];

        assert_eq!(get_span(21).iter().collect::<Vec<_>>(), expected_spans);
        assert_eq!(Balances::free_balance(&21), 1810);
    });
}

#[test]
fn deferred_slashes_are_deferred() {
    ExtBuilder::default()
        .slash_defer_duration(2)
        .build()
        .execute_with(|| {
            start_era(1, false);

            assert_eq!(Balances::free_balance(&11), 1000);

            let exposure = Staking::stakers(&11);
            assert_eq!(Balances::free_balance(&101), 2000);
            let guaranteed_value = exposure.others.iter().find(|o| o.who == 101).unwrap().value;

            on_offence_now(
                &[OffenceDetails {
                    offender: (11, Staking::stakers(&11)),
                    reporters: vec![],
                }],
                &[Perbill::from_percent(10)],
            );

            assert_eq!(Balances::free_balance(&11), 1000);
            assert_eq!(Balances::free_balance(&101), 2000);

            start_era(2, false);

            assert_eq!(Balances::free_balance(&11), 1000);
            assert_eq!(Balances::free_balance(&101), 2000);

            start_era(3, false);

            assert_eq!(Balances::free_balance(&11), 1000);
            assert_eq!(Balances::free_balance(&101), 2000);

            // at the start of era 4, slashes from era 1 are processed,
            // after being deferred for at least 2 full eras.
            start_era(4, false);

            assert_eq!(Balances::free_balance(&11), 900);
            assert_eq!(Balances::free_balance(&101), 2000 - (guaranteed_value / 10));
        })
}

#[test]
fn remove_deferred() {
    ExtBuilder::default()
        .slash_defer_duration(2)
        .build()
        .execute_with(|| {
            start_era(1, false);

            assert_eq!(Balances::free_balance(&11), 1000);

            let exposure = Staking::stakers(&11);
            assert_eq!(Balances::free_balance(&101), 2000);
            let guaranteed_value = exposure.others.iter().find(|o| o.who == 101).unwrap().value;

            on_offence_now(
                &[OffenceDetails {
                    offender: (11, exposure.clone()),
                    reporters: vec![],
                }],
                &[Perbill::from_percent(10)],
            );

            assert_eq!(Balances::free_balance(&11), 1000);
            assert_eq!(Balances::free_balance(&101), 2000);

            start_era(2, false);

            on_offence_in_era(
                &[OffenceDetails {
                    offender: (11, exposure.clone()),
                    reporters: vec![],
                }],
                &[Perbill::from_percent(15)],
                1,
            );

            Staking::cancel_deferred_slash(Origin::root(), 1, vec![0]).unwrap();

            assert_eq!(Balances::free_balance(&11), 1000);
            assert_eq!(Balances::free_balance(&101), 2000);

            start_era(3, false);

            assert_eq!(Balances::free_balance(&11), 1000);
            assert_eq!(Balances::free_balance(&101), 2000);

            // at the start of era 4, slashes from era 1 are processed,
            // after being deferred for at least 2 full eras.
            start_era(4, false);

            // the first slash for 10% was cancelled, so no effect.
            assert_eq!(Balances::free_balance(&11), 1000);
            assert_eq!(Balances::free_balance(&101), 2000);

            start_era(5, false);

            let slash_10 = Perbill::from_percent(10);
            let slash_15 = Perbill::from_percent(15);
            let initial_slash = slash_10 * guaranteed_value;

            let total_slash = slash_15 * guaranteed_value;
            let actual_slash = total_slash - initial_slash;

            // 5% slash (15 - 10) processed now.
            assert_eq!(Balances::free_balance(&11), 950);
            assert_eq!(Balances::free_balance(&101), 2000 - actual_slash);
        })
}

#[test]
fn remove_multi_deferred() {
    ExtBuilder::default()
        .slash_defer_duration(2)
        .build()
        .execute_with(|| {
            start_era(1, false);

            assert_eq!(Balances::free_balance(&11), 1000);

            let exposure = Staking::stakers(&11);
            assert_eq!(Balances::free_balance(&101), 2000);

            on_offence_now(
                &[OffenceDetails {
                    offender: (11, exposure.clone()),
                    reporters: vec![],
                }],
                &[Perbill::from_percent(10)],
            );

            on_offence_now(
                &[OffenceDetails {
                    offender: (21, Staking::stakers(&21)),
                    reporters: vec![],
                }],
                &[Perbill::from_percent(10)],
            );

            on_offence_now(
                &[OffenceDetails {
                    offender: (11, exposure.clone()),
                    reporters: vec![],
                }],
                &[Perbill::from_percent(25)],
            );

            assert_eq!(<Staking as Store>::UnappliedSlashes::get(&1).len(), 3);
            Staking::cancel_deferred_slash(Origin::root(), 1, vec![0, 2]).unwrap();

            let slashes = <Staking as Store>::UnappliedSlashes::get(&1);
            assert_eq!(slashes.len(), 1);
            assert_eq!(slashes[0].validator, 21);
        })
}

#[test]
fn update_stakers_should_work_new_era() {
    ExtBuilder::default()
    .guarantee(false)
    .fair(false) // to give 20 more staked value
    .build()
    .execute_with(|| {
        assert!(!<Stakers<Test>>::contains_key(&5));

        // remember + compare this along with the test.
        assert_eq_uvec!(validator_controllers(), vec![20, 10]);

        // put some money in account that we'll use.
        for i in 1..10 {
            let _ = Balances::make_free_balance_be(&i, 3000);
        }

        // --- Block 1:
        start_session(1, false);
        // add a new candidate for being a validator. account 3 controlled by 4.
        assert_ok!(Staking::bond(
            Origin::signed(5),
            4,
            1000,
            RewardDestination::Controller
        ));

        assert_ok!(Staking::bond(
            Origin::signed(3),
            2,
            1000,
            RewardDestination::Controller
        ));

        assert_ok!(Staking::bond(
            Origin::signed(7),
            6,
            1000,
            RewardDestination::Controller
        ));

        assert_ok!(Staking::bond(
            Origin::signed(9),
            8,
            1000,
            RewardDestination::Controller
        ));
        Staking::upsert_stake_limit(&5, 3000);
        assert_ok!(Staking::validate(Origin::signed(4), Perbill::default()));

        assert_ok!(Staking::guarantee(Origin::signed(2), (5, 500)));
        assert_ok!(Staking::guarantee(Origin::signed(6), (5, 500)));
        assert_ok!(Staking::guarantee(Origin::signed(8), (5, 500)));

        // No effects will be seen so far.
        assert_eq_uvec!(validator_controllers(), vec![20, 10]);

        // --- Block 2:
        start_session(2, false);

        // --- Block 3: the validators will now be queued.
        start_session(3, false);
        assert_eq!(Staking::current_era().unwrap_or(0), 1);

        // --- Block 4: the validators will now be changed.
        start_session(4, false);

        assert_eq_uvec!(validator_controllers(), vec![4, 20]);
        assert!(<Stakers<Test>>::contains_key(&5));
        assert_eq!(
            Staking::stakers(&5),
            Exposure {
                total: 2500,
                own: 1000,
                others: vec![IndividualExposure {
                    who: 3,
                    value: 500
                },
                IndividualExposure {
                    who: 7,
                    value: 500
                },
                IndividualExposure {
                    who: 9,
                    value: 500
                }]
            }
        )
    });
}

#[test]
fn guarantee_limit_should_work() {
    ExtBuilder::default()
        .guarantee(false)
        .own_workload(0)
        .total_workload(0)
        .build()
        .execute_with(|| {
            // put some money in account that we'll use.
            for i in 1..7 {
                let _ = Balances::deposit_creating(&i, 5000);
            }

            Staking::upsert_stake_limit(&5, 1500);
            assert_eq!(Staking::stake_limit(&5).unwrap_or_default(), 1500);

            // add a new validator candidate
            assert_ok!(Staking::bond(
                Origin::signed(5),
                6,
                1000,
                RewardDestination::Controller
            ));
            assert_ok!(Staking::validate(Origin::signed(6), Perbill::default()));

            // add guarantor
            assert_ok!(Staking::bond(
                Origin::signed(1),
                2,
                2000,
                RewardDestination::Controller
            ));
            assert_ok!(Staking::guarantee(Origin::signed(2), (5, 1000)));

            // guarantor's info guarantors should ✅
            assert_eq!(
                Staking::guarantors(&1),
                Some(Nominations {
                    targets: vec![5],
                    total: 500,
                    submitted_in: 0,
                    suppressed: false
                })
            );
            assert_eq!(Staking::guarantee_rel(1, 5).get(&(0 as u32)), Some(&(500 as Balance)));

            // After a era, valid stake should updated.
            start_era_with_new_workloads(1, false, 1000, 3000);

            assert_eq!(
                Staking::guarantors(&1),
                Some(Nominations {
                    targets: vec![5],
                    total: 500,
                    submitted_in: 0,
                    suppressed: false
                })
            );
            assert_eq!(Staking::guarantee_rel(1, 5).get(&(0 as u32)), Some(&(500 as Balance)));

            assert_eq!(
                Staking::ledger(&2),
                Some(StakingLedger {
                    stash: 1,
                    total: 2000,
                    active: 2000,
                    valid: 500,
                    unlocking: vec![],
                    claimed_rewards: vec![]
                })
            );

            Staking::upsert_stake_limit(&5, 5000);
            assert_ok!(Staking::guarantee(Origin::signed(2), (5, 2000)));
            assert_eq!(
                Staking::guarantors(&1),
                Some(Nominations {
                    targets: vec![5],
                    total: 2000,
                    submitted_in: 1,
                    suppressed: false
                })
            );
            assert_eq!(Staking::guarantee_rel(1, 5).get(&(0 as u32)), Some(&(500 as Balance)));
            assert_eq!(Staking::guarantee_rel(1, 5).get(&(1 as u32)), Some(&(1500 as Balance)));

            // Next era, wr should be outdated
            start_era(2, false);

            assert_eq!(
                Staking::guarantors(&1),
                Some(Nominations {
                    targets: vec![5],
                    total: 2000,
                    submitted_in: 1,
                    suppressed: false
                })
            );
            assert_eq!(Staking::guarantee_rel(1, 5).get(&(0 as u32)), Some(&(500 as Balance)));
            assert_eq!(Staking::guarantee_rel(1, 5).get(&(1 as u32)), Some(&(1500 as Balance)));

            assert_eq!(
                Staking::ledger(&2),
                Some(StakingLedger {
                    stash: 1,
                    total: 2000,
                    active: 2000,
                    valid: 2000,
                    unlocking: vec![],
                    claimed_rewards: vec![]
                })
            );
        });
}

#[test]
fn guarantee_order_should_work() {
    ExtBuilder::default()
        .guarantee(false)
        .build()
        .execute_with(|| {
            // put some money in account that we'll use.
            for i in 1..10 {
                let _ = Balances::deposit_creating(&i, 5000);
            }

            Staking::upsert_stake_limit(&5, 5000);
            assert_eq!(Staking::stake_limit(&5).unwrap_or_default(), 5000);

            // add a new validator candidate
            assert_ok!(Staking::bond(
                Origin::signed(5),
                6,
                1000,
                RewardDestination::Controller
            ));
            assert_ok!(Staking::validate(Origin::signed(6), Perbill::default()));

            // add guarantor
            assert_ok!(Staking::bond(
                Origin::signed(1),
                2,
                2000,
                RewardDestination::Controller
            ));

            // add guarantor
            assert_ok!(Staking::bond(
                Origin::signed(3),
                4,
                2000,
                RewardDestination::Controller
            ));

            assert_ok!(Staking::bond(
                Origin::signed(7),
                8,
                2000,
                RewardDestination::Controller
            ));

            assert_ok!(Staking::guarantee(Origin::signed(2), (5, 250)));
            assert_ok!(Staking::guarantee(Origin::signed(8), (5, 1000)));
            assert_ok!(Staking::guarantee(Origin::signed(2), (5, 250)));
            assert_ok!(Staking::guarantee(Origin::signed(2), (5, 250)));
            assert_ok!(Staking::guarantee(Origin::signed(4), (5, 1000)));
            assert_ok!(Staking::guarantee(Origin::signed(2), (5, 250)));

            // guarantor's info guarantors should ✅
            assert_eq!(
                Staking::guarantors(&1),
                Some(Nominations {
                    targets: vec![5],
                    total: 1000,
                    submitted_in: 0,
                    suppressed: false
                })
            );

            assert_eq!(
                Staking::guarantors(&3),
                Some(Nominations {
                    targets: vec![5],
                    total: 1000,
                    submitted_in: 0,
                    suppressed: false
                })
            );
            assert_eq!(Staking::guarantee_rel(3, 5).get(&(0 as u32)), Some(&(1000 as Balance)));

            assert_eq!(
                Staking::guarantors(&7),
                Some(Nominations {
                    targets: vec![5],
                    total: 1000,
                    submitted_in: 0,
                    suppressed: false
                })
            );
            assert_eq!(Staking::guarantee_rel(7, 5).get(&(0 as u32)), Some(&(1000 as Balance)));

            assert_eq!(
                Staking::validators(&5),
                Validations{
                    total: 3000,
                    guarantee_fee: Default::default(),
                    guarantors: vec![1, 7, 1, 1, 3, 1]
                }
            );
            // re-guarantee. The order should change
            assert_ok!(Staking::guarantee(Origin::signed(2), (5, 500)));

            assert_eq!(
                Staking::validators(&5),
                Validations{
                    total: 3500,
                    guarantee_fee: Default::default(),
                    guarantors: vec![1, 7, 1, 1, 3, 1, 1]
                }
            );
            assert_eq!(Staking::guarantee_rel(1, 5).get(&(0 as u32)), Some(&(250 as Balance)));
            assert_eq!(Staking::guarantee_rel(1, 5).get(&(1 as u32)), Some(&(250 as Balance)));
            assert_eq!(Staking::guarantee_rel(1, 5).get(&(2 as u32)), Some(&(250 as Balance)));
            assert_eq!(Staking::guarantee_rel(1, 5).get(&(3 as u32)), Some(&(250 as Balance)));
            assert_eq!(Staking::guarantee_rel(1, 5).get(&(4 as u32)), Some(&(500 as Balance)));
            assert_eq!(Staking::guarantee_rel(3, 5).get(&(0 as u32)), Some(&(1000 as Balance)));
            assert_eq!(Staking::guarantee_rel(7, 5).get(&(0 as u32)), Some(&(1000 as Balance)));


            assert_ok!(Staking::cut_guarantee(Origin::signed(2), (5, 1000)));
            assert_eq!(
                Staking::validators(&5),
                Validations{
                    total: 2500,
                    guarantee_fee: Default::default(),
                    guarantors: vec![1, 7, 1, 3]
                }
            );
            assert_eq!(Staking::guarantee_rel(1, 5).get(&(0 as u32)), Some(&(250 as Balance)));
            assert_eq!(Staking::guarantee_rel(1, 5).get(&(1 as u32)), Some(&(250 as Balance)));
            assert_eq!(Staking::guarantee_rel(1, 5).get(&(2 as u32)), None);
            assert_eq!(Staking::guarantee_rel(1, 5).get(&(3 as u32)), None);
            assert_eq!(Staking::guarantee_rel(1, 5).get(&(4 as u32)), None);
            assert_eq!(Staking::guarantee_rel(3, 5).get(&(0 as u32)), Some(&(1000 as Balance)));
            assert_eq!(Staking::guarantee_rel(7, 5).get(&(0 as u32)), Some(&(1000 as Balance)));


            assert_ok!(Staking::guarantee(Origin::signed(2), (5, 500)));
            assert_eq!(
                Staking::validators(&5),
                Validations{
                    total: 3000,
                    guarantee_fee: Default::default(),
                    guarantors: vec![1, 7, 1, 3, 1]
                }
            );
            assert_eq!(Staking::guarantee_rel(1, 5).get(&(0 as u32)), Some(&(250 as Balance)));
            assert_eq!(Staking::guarantee_rel(1, 5).get(&(1 as u32)), Some(&(250 as Balance)));
            assert_eq!(Staking::guarantee_rel(1, 5).get(&(2 as u32)), Some(&(500 as Balance)));
            assert_eq!(Staking::guarantee_rel(1, 5).get(&(3 as u32)), None);
            assert_eq!(Staking::guarantee_rel(1, 5).get(&(4 as u32)), None);
            assert_eq!(Staking::guarantee_rel(3, 5).get(&(0 as u32)), Some(&(1000 as Balance)));
            assert_eq!(Staking::guarantee_rel(7, 5).get(&(0 as u32)), Some(&(1000 as Balance)));
        });
}

#[test]
fn multi_guarantees_with_stake_limit_should_work() {
    ExtBuilder::default()
        .guarantee(false)
        .own_workload(2)
        .total_workload(100000000)
        .build()
        .execute_with(|| {
            // put some money in account that we'll use.
            for i in 1..10 {
                let _ = Balances::deposit_creating(&i, 5000);
            }

            // Stake limit effecting
            start_era(1, false);
            assert_eq!(Staking::stake_limit(&11).unwrap_or_default(), 5000);
            start_era(4, false);
            assert_eq!(Staking::stake_limit(&11).unwrap_or_default(), 5000);
            // add guarantor
            assert_ok!(Staking::bond(
                Origin::signed(1),
                2,
                2000,
                RewardDestination::Controller
            ));
            assert_ok!(Staking::guarantee(Origin::signed(2), (11, 250)));
            assert_ok!(Staking::guarantee(Origin::signed(2), (11, 250)));

            // add guarantor
            assert_ok!(Staking::bond(
                Origin::signed(3),
                4,
                1000,
                RewardDestination::Controller
            ));
            assert_ok!(Staking::guarantee(Origin::signed(4), (11, 2000)));

            // guarantor's info guarantors should ✅
            assert_eq!(
                Staking::guarantors(&1),
                Some(Nominations {
                    targets: vec![11],
                    total: 500,
                    submitted_in: 4,
                    suppressed: false
                })
            );
            assert_eq!(Staking::guarantee_rel(1, 11).get(&(0 as u32)), Some(&(250 as Balance)));
            assert_eq!(Staking::guarantee_rel(1, 11).get(&(1 as u32)), Some(&(250 as Balance)));

            assert_eq!(
                Staking::guarantors(&3),
                Some(Nominations {
                    targets: vec![11],
                    total: 1000,
                    submitted_in: 4,
                    suppressed: false
                })
            );
            assert_eq!(Staking::guarantee_rel(3, 11).get(&(0 as u32)), Some(&(1000 as Balance)));

            assert_eq!(
                Staking::validators(&11),
                Validations{
                    total: 1500,
                    guarantee_fee: Perbill::one(),
                    guarantors: vec![1, 1, 3]
                }
            );

            assert_ok!(Staking::cut_guarantee(Origin::signed(2), (11, 250)));
            assert_eq!(Staking::guarantee_rel(1, 11).get(&(0 as u32)), Some(&(250 as Balance)));
            assert_eq!(
                Staking::validators(&11),
                Validations{
                    total: 1250,
                    guarantee_fee: Perbill::one(),
                    guarantors: vec![1, 3]
                }
            );

            assert_ok!(Staking::guarantee(Origin::signed(2), (11, 750)));
            assert_eq!(Staking::guarantee_rel(1, 11).get(&(0 as u32)), Some(&(250 as Balance)));
            assert_eq!(Staking::guarantee_rel(1, 11).get(&(1 as u32)), Some(&(750 as Balance)));
            assert_eq!(
                Staking::validators(&11),
                Validations{
                    total: 2000,
                    guarantee_fee: Perbill::one(),
                    guarantors: vec![1, 3, 1]
                }
            );

            start_era_with_new_workloads(5, false, 1, 200000000);
            assert_eq!(Staking::stake_limit(&11).unwrap_or_default(), 2500);
            // 3 would be removed from validators due to stake limit
            assert_eq!(
                Staking::validators(&11),
                Validations{
                    total: 1500,
                    guarantee_fee: Perbill::one(),
                    guarantors: vec![1, 3, 1]
                }
            );
            assert_eq!(Staking::guarantee_rel(1, 11).get(&(0 as u32)), Some(&(250 as Balance)));
            assert_eq!(Staking::guarantee_rel(1, 11).get(&(1 as u32)), Some(&(250 as Balance)));
            assert_eq!(
                Staking::stakers(11),
                Exposure {
                    total: 2500,
                    own: 1000,
                    others: vec![IndividualExposure {
                        who: 1,
                        value: 500
                    },
                    IndividualExposure {
                        who: 3,
                        value: 1000
                    }]
                }
            );
            assert_eq!(
                Staking::ledger(&4),
                Some(StakingLedger {
                    stash: 3,
                    total: 1000,
                    active: 1000,
                    valid: 1000,
                    unlocking: vec![],
                    claimed_rewards: vec![0, 1, 2, 3]
                })
            );

            assert_eq!(
                Staking::ledger(&2),
                Some(StakingLedger {
                    stash: 1,
                    total: 2000,
                    active: 2000,
                    valid: 500,
                    unlocking: vec![],
                    claimed_rewards: vec![0, 1, 2, 3]
                })
            );
            
        });
}

#[test]
fn new_era_with_stake_limit_should_work() {
    ExtBuilder::default()
        .guarantee(false)
        .own_workload(2)
        .total_workload(100000000)
        .build()
        .execute_with(|| {
            // put some money in account that we'll use.
            for i in 1..10 {
                let _ = Balances::deposit_creating(&i, 5000);
            }

            start_era(1, false);
            assert_eq!(Staking::stake_limit(&11).unwrap_or_default(), 5000);
            start_era(4, false);
            assert_eq!(Staking::stake_limit(&11).unwrap_or_default(), 5000);
            // add guarantor
            assert_ok!(Staking::bond(
                Origin::signed(1),
                2,
                2000,
                RewardDestination::Controller
            ));
            assert_ok!(Staking::guarantee(Origin::signed(2), (11, 2000)));

            // add guarantor
            assert_ok!(Staking::bond(
                Origin::signed(3),
                4,
                2000,
                RewardDestination::Controller
            ));
            assert_ok!(Staking::guarantee(Origin::signed(4), (11, 3000)));

            // guarantor's info guarantors should ✅
            assert_eq!(
                Staking::guarantors(&1),
                Some(Nominations {
                    targets: vec![11],
                    total: 2000,
                    submitted_in: 4,
                    suppressed: false
                })
            );
            assert_eq!(Staking::guarantee_rel(1, 11).get(&(0 as u32)), Some(&(2000 as Balance)));

            assert_eq!(
                Staking::guarantors(&3),
                Some(Nominations {
                    targets: vec![11],
                    total: 2000,
                    submitted_in: 4,
                    suppressed: false
                })
            );
            assert_eq!(Staking::guarantee_rel(3, 11).get(&(0 as u32)), Some(&(2000 as Balance)));

            assert_eq!(
                Staking::validators(&11),
                Validations{
                    total: 4000,
                    guarantee_fee: Perbill::one(),
                    guarantors: vec![1, 3]
                }
            );

            start_era_with_new_workloads(5, false, 1, 200000000);
            assert_eq!(Staking::stake_limit(&11).unwrap_or_default(), 2500);
            // 3 would be removed from validators due to stake limite
            assert_eq!(
                Staking::validators(&11),
                Validations{
                    total: 1500,
                    guarantee_fee: Perbill::one(),
                    guarantors: vec![1]
                }
            );
            assert_eq!(Staking::guarantee_rel(1, 11).get(&(0 as u32)), Some(&(1500 as Balance)));
            assert_eq!(
                Staking::stakers(11),
                Exposure {
                    total: 2500,
                    own: 1000,
                    others: vec![IndividualExposure {
                        who: 1,
                        value: 1500
                    }]
                }
            );
            assert_eq!(
                Staking::ledger(&4),
                Some(StakingLedger {
                    stash: 3,
                    total: 2000,
                    active: 2000,
                    valid: 0,
                    unlocking: vec![], // in time. no lock for such scenario.
                    claimed_rewards: vec![0, 1, 2, 3]
                })
            );
            start_era_with_new_workloads(6, false, 1, 200000000);
            start_era_with_new_workloads(7, false, 1, 200000000);
            start_era_with_new_workloads(8, false, 1, 200000000);
            assert_eq!(Staking::stake_limit(&11).unwrap_or_default(), 2500);
            // 3 would be removed from validators due to stake limit
            assert_eq!(
                Staking::validators(&11),
                Validations{
                    total: 1500,
                    guarantee_fee: Perbill::one(),
                    guarantors: vec![1]
                }
            );
            assert_eq!(Staking::guarantee_rel(1, 11).get(&(0 as u32)), Some(&(1500 as Balance)));
            assert_eq!(
                Staking::stakers(11),
                Exposure {
                    total: 2500,
                    own: 1000,
                    others: vec![IndividualExposure {
                        who: 1,
                        value: 1500
                    }]
                }
            );
            assert_eq!(
                Staking::ledger(&4),
                Some(StakingLedger {
                    stash: 3,
                    total: 2000,
                    active: 2000,
                    valid: 0,
                    unlocking: vec![], // in time. no lock for such scenario.
                    claimed_rewards: vec![0, 1, 2, 3]
                })
            );
        });
}

#[test]
fn new_era_with_stake_limit_for_nominations_should_work() {
    ExtBuilder::default()
        .guarantee(false)
        .own_workload(2)
        .total_workload(100000000)
        .build()
        .execute_with(|| {
            // put some money in account that we'll use.
            for i in 1..10 {
                let _ = Balances::deposit_creating(&i, 5000);
            }

            start_era(1, false);
            assert_eq!(Staking::stake_limit(&11).unwrap_or_default(), 5000);
            assert_eq!(Staking::stake_limit(&21).unwrap_or_default(), 5000);
            start_era(4, false);
            assert_eq!(Staking::stake_limit(&11).unwrap_or_default(), 5000);
            assert_eq!(Staking::stake_limit(&21).unwrap_or_default(), 5000);
            // add guarantor
            assert_ok!(Staking::bond(
                Origin::signed(1),
                2,
                4000,
                RewardDestination::Controller
            ));
            assert_ok!(Staking::guarantee(Origin::signed(2), (11, 500)));
            assert_ok!(Staking::guarantee(Origin::signed(2), (11, 500)));

            // add guarantor
            assert_ok!(Staking::bond(
                Origin::signed(3),
                4,
                4000,
                RewardDestination::Controller
            ));
            assert_ok!(Staking::guarantee(Origin::signed(4), (11, 750)));
            assert_ok!(Staking::guarantee(Origin::signed(4), (11, 500)));
            assert_ok!(Staking::guarantee(Origin::signed(4), (11, 500)));
            assert_ok!(Staking::guarantee(Origin::signed(4), (11, 500)));

            // guarantor's info guarantors should ✅
            assert_eq!(
                Staking::guarantors(&1),
                Some(Nominations {
                    targets: vec![11],
                    total: 1000,
                    submitted_in: 4,
                    suppressed: false
                })
            );
            assert_eq!(Staking::guarantee_rel(1, 11).get(&(0 as u32)), Some(&(500 as Balance)));
            assert_eq!(Staking::guarantee_rel(1, 11).get(&(1 as u32)), Some(&(500 as Balance)));


            assert_eq!(
                Staking::guarantors(&3),
                Some(Nominations {
                    targets: vec![11],
                    total: 2250,
                    submitted_in: 4,
                    suppressed: false
                })
            );
            assert_eq!(Staking::guarantee_rel(3, 11).get(&(0 as u32)), Some(&(750 as Balance)));
            assert_eq!(Staking::guarantee_rel(3, 11).get(&(1 as u32)), Some(&(500 as Balance)));
            assert_eq!(Staking::guarantee_rel(3, 11).get(&(2 as u32)), Some(&(500 as Balance)));
            assert_eq!(Staking::guarantee_rel(3, 11).get(&(3 as u32)), Some(&(500 as Balance)));

            assert_eq!(
                Staking::validators(&11),
                Validations{
                    total: 3250,
                    guarantee_fee: Perbill::one(),
                    guarantors: vec![1, 1, 3, 3, 3, 3]
                }
            );


            // add guarantor
            assert_ok!(Staking::guarantee(Origin::signed(2), (21, 500)));
            assert_ok!(Staking::guarantee(Origin::signed(2), (21, 500)));
            assert_ok!(Staking::guarantee(Origin::signed(2), (21, 750)));

            assert_ok!(Staking::guarantee(Origin::signed(4), (21, 500)));
            assert_ok!(Staking::guarantee(Origin::signed(4), (21, 500)));
            assert_ok!(Staking::guarantee(Origin::signed(4), (21, 500)));

            // guarantor's info guarantors should ✅
            assert_eq!(
                Staking::guarantors(&1),
                Some(Nominations {
                    targets: vec![11, 21],
                    total: 2750,
                    submitted_in: 4,
                    suppressed: false
                })
            );
            assert_eq!(Staking::guarantee_rel(1, 21).get(&(0 as u32)), Some(&(500 as Balance)));
            assert_eq!(Staking::guarantee_rel(1, 21).get(&(1 as u32)), Some(&(500 as Balance)));
            assert_eq!(Staking::guarantee_rel(1, 21).get(&(2 as u32)), Some(&(750 as Balance)));


            assert_eq!(
                Staking::guarantors(&3),
                Some(Nominations {
                    targets: vec![11, 21],
                    total: 3750,
                    submitted_in: 4,
                    suppressed: false
                })
            );
            assert_eq!(Staking::guarantee_rel(3, 21).get(&(0 as u32)), Some(&(500 as Balance)));
            assert_eq!(Staking::guarantee_rel(3, 21).get(&(1 as u32)), Some(&(500 as Balance)));
            assert_eq!(Staking::guarantee_rel(3, 21).get(&(2 as u32)), Some(&(500 as Balance)));

            assert_eq!(
                Staking::validators(&21),
                Validations{
                    total: 3250,
                    guarantee_fee: Perbill::one(),
                    guarantors: vec![1, 1, 1, 3, 3, 3]
                }
            );

            start_era_with_new_workloads(5, false, 1, 200000000);
            assert_eq!(Staking::stake_limit(&11).unwrap_or_default(), 2500);
            assert_eq!(Staking::stake_limit(&21).unwrap_or_default(), 2500);
            // 3 would be removed from validators due to stake limit
            assert_eq!(
                Staking::validators(&11),
                Validations{
                    total: 1500,
                    guarantee_fee: Perbill::one(),
                    guarantors: vec![1, 1, 3]
                }
            );

            assert_eq!(
                Staking::validators(&21),
                Validations{
                    total: 1500,
                    guarantee_fee: Perbill::one(),
                    guarantors: vec![1, 1, 1]
                }
            );

            assert_eq!(
                Staking::guarantors(&1),
                Some(Nominations {
                    targets: vec![11, 21],
                    total: 2500,
                    submitted_in: 4,
                    suppressed: false
                })
            );
            assert_eq!(Staking::guarantee_rel(1, 11).get(&(0 as u32)), Some(&(500 as Balance)));
            assert_eq!(Staking::guarantee_rel(1, 11).get(&(1 as u32)), Some(&(500 as Balance)));
            assert_eq!(Staking::guarantee_rel(1, 21).get(&(0 as u32)), Some(&(500 as Balance)));
            assert_eq!(Staking::guarantee_rel(1, 21).get(&(1 as u32)), Some(&(500 as Balance)));
            assert_eq!(Staking::guarantee_rel(1, 21).get(&(2 as u32)), Some(&(500 as Balance)));
            assert_eq!(
                Staking::stakers(11),
                Exposure {
                    total: 2500,
                    own: 1000,
                    others: vec![IndividualExposure {
                        who: 1,
                        value: 1000
                    },
                    IndividualExposure {
                        who: 3,
                        value: 500
                    }
                    ]
                }
            );

            assert_eq!(
                Staking::stakers(21),
                Exposure {
                    total: 2500,
                    own: 1000,
                    others: vec![IndividualExposure {
                        who: 1,
                        value: 1500
                    }]
                }
            );
            assert_eq!(
                Staking::ledger(&4),
                Some(StakingLedger {
                    stash: 3,
                    total: 4000,
                    active: 4000,
                    valid: 500,
                    unlocking: vec![], // in time. no lock for such scenario.
                    claimed_rewards: vec![0, 1, 2, 3]
                })
            );

            assert_eq!(
                Staking::ledger(&2),
                Some(StakingLedger {
                    stash: 1,
                    total: 4000,
                    active: 4000,
                    valid: 2500,
                    unlocking: vec![], // in time. no lock for such scenario.
                    claimed_rewards: vec![0, 1, 2, 3]
                })
            );

            assert_eq!(
                Staking::guarantors(&3),
                Some(Nominations {
                    targets: vec![11],
                    total: 500,
                    submitted_in: 4,
                    suppressed: false
                })
            );
            assert_eq!(Staking::guarantee_rel(3, 11).get(&(0 as u32)), Some(&(500 as Balance)));
            assert_eq!(Staking::guarantee_rel(3, 21).len(), 0);
            start_era_with_new_workloads(6, false, 1, 200000000);
            start_era_with_new_workloads(8, false, 1, 200000000);
            assert_eq!(Staking::stake_limit(&11).unwrap_or_default(), 2500);
            assert_eq!(Staking::stake_limit(&21).unwrap_or_default(), 2500);
            // 3 would be removed from validators due to stake limite
            assert_eq!(
                Staking::validators(&11),
                Validations{
                    total: 1500,
                    guarantee_fee: Perbill::one(),
                    guarantors: vec![1, 1, 3]
                }
            );

            assert_eq!(
                Staking::validators(&21),
                Validations{
                    total: 1500,
                    guarantee_fee: Perbill::one(),
                    guarantors: vec![1, 1, 1]
                }
            );

            assert_eq!(
                Staking::guarantors(&1),
                Some(Nominations {
                    targets: vec![11, 21],
                    total: 2500,
                    submitted_in: 4,
                    suppressed: false
                })
            );
            assert_eq!(Staking::guarantee_rel(1, 11).get(&(0 as u32)), Some(&(500 as Balance)));
            assert_eq!(Staking::guarantee_rel(1, 11).get(&(1 as u32)), Some(&(500 as Balance)));
            assert_eq!(Staking::guarantee_rel(1, 21).get(&(0 as u32)), Some(&(500 as Balance)));
            assert_eq!(Staking::guarantee_rel(1, 21).get(&(1 as u32)), Some(&(500 as Balance)));
            assert_eq!(Staking::guarantee_rel(1, 21).get(&(2 as u32)), Some(&(500 as Balance)));
            assert_eq!(
                Staking::stakers(11),
                Exposure {
                    total: 2500,
                    own: 1000,
                    others: vec![IndividualExposure {
                        who: 1,
                        value: 1000
                    },
                    IndividualExposure {
                        who: 3,
                        value: 500
                    }
                    ]
                }
            );

            assert_eq!(
                Staking::stakers(21),
                Exposure {
                    total: 2500,
                    own: 1000,
                    others: vec![IndividualExposure {
                        who: 1,
                        value: 1500
                    }]
                }
            );
            assert_eq!(
                Staking::ledger(&4),
                Some(StakingLedger {
                    stash: 3,
                    total: 4000,
                    active: 4000,
                    valid: 500,
                    unlocking: vec![], // in time. no lock for such scenario.
                    claimed_rewards: vec![0, 1, 2, 3]
                })
            );

            assert_eq!(
                Staking::ledger(&2),
                Some(StakingLedger {
                    stash: 1,
                    total: 4000,
                    active: 4000,
                    valid: 2500,
                    unlocking: vec![], // in time. no lock for such scenario.
                    claimed_rewards: vec![0, 1, 2, 3]
                })
            );

            assert_eq!(
                Staking::guarantors(&3),
                Some(Nominations {
                    targets: vec![11],
                    total: 500,
                    submitted_in: 4,
                    suppressed: false
                })
            );
        });
}

#[test]
fn cut_guarantee_should_work() {
    ExtBuilder::default()
    .guarantee(false)
    .build()
    .execute_with(|| {
        // put some money in account that we'll use.
        for i in 1..10 {
            let _ = Balances::deposit_creating(&i, 5000);
        }

        Staking::upsert_stake_limit(&5, 5000);
        assert_eq!(Staking::stake_limit(&5).unwrap_or_default(), 5000);
        Staking::upsert_stake_limit(&7, 5000);
        assert_eq!(Staking::stake_limit(&7).unwrap_or_default(), 5000);

        // add a new validator candidate
        assert_ok!(Staking::bond(
            Origin::signed(5),
            6,
            1000,
            RewardDestination::Controller
        ));
        assert_ok!(Staking::validate(Origin::signed(6), Perbill::default()));

        assert_ok!(Staking::bond(
            Origin::signed(7),
            8,
            2000,
            RewardDestination::Controller
        ));
        assert_ok!(Staking::validate(Origin::signed(8), Perbill::default()));

        // add guarantor
        assert_ok!(Staking::bond(
            Origin::signed(1),
            2,
            2000,
            RewardDestination::Controller
        ));

        // add guarantor
        assert_ok!(Staking::bond(
            Origin::signed(3),
            4,
            2000,
            RewardDestination::Controller
        ));

        assert_noop!(
            Staking::cut_guarantee(Origin::signed(2), (2, 250)),
            DispatchError::Module {
                index: 0,
                error: 6,
                message: Some("InvalidTarget"),
            }
        );

        // guarantor's info guarantors should ✅
        assert_eq!(
            Staking::guarantors(&1),
            None
        );
        assert_ok!(Staking::guarantee(Origin::signed(2), (5, 250)));
        assert_ok!(Staking::guarantee(Origin::signed(2), (5, 250)));
        assert_ok!(Staking::guarantee(Origin::signed(2), (7, 250)));
        assert_ok!(Staking::guarantee(Origin::signed(2), (7, 250)));
        assert_ok!(Staking::guarantee(Origin::signed(4), (5, 1000)));
        assert_ok!(Staking::guarantee(Origin::signed(2), (5, 250)));

        // guarantor's info guarantors should ✅
        assert_eq!(
            Staking::guarantors(&1),
            Some(Nominations {
                targets: vec![5, 7],
                total: 1250,
                submitted_in: 0,
                suppressed: false
            })
        );

        assert_eq!(
            Staking::guarantors(&3),
            Some(Nominations {
                targets: vec![5],
                total: 1000,
                submitted_in: 0,
                suppressed: false
            })
        );
        assert_eq!(Staking::guarantee_rel(3, 5).get(&(0 as u32)), Some(&(1000 as Balance)));
        assert_eq!(Staking::guarantee_rel(1, 5).get(&(0 as u32)), Some(&(250 as Balance)));
        assert_eq!(Staking::guarantee_rel(1, 5).get(&(1 as u32)), Some(&(250 as Balance)));
        assert_eq!(Staking::guarantee_rel(1, 5).get(&(2 as u32)), Some(&(250 as Balance)));

        assert_eq!(
            Staking::validators(&5),
            Validations{
                total: 1750,
                guarantee_fee: Default::default(),
                guarantors: vec![1, 1, 3, 1]
            }
        );
        // re-guarantee. The order should change
        assert_ok!(Staking::guarantee(Origin::signed(2), (5, 500)));

        assert_eq!(
            Staking::validators(&5),
            Validations{
                total: 2250,
                guarantee_fee: Default::default(),
                guarantors: vec![1, 1, 3, 1, 1]
            }
        );
        assert_eq!(Staking::guarantee_rel(1, 5).get(&(3 as u32)), Some(&(500 as Balance)));

        assert_ok!(Staking::cut_guarantee(Origin::signed(2), (5, 600)));
        assert_eq!(
            Staking::validators(&5),
            Validations{
                total: 1650,
                guarantee_fee: Default::default(),
                guarantors: vec![1, 1, 3, 1]
            }
        );

        assert_eq!(Staking::guarantee_rel(3, 5).get(&(0 as u32)), Some(&(1000 as Balance)));
        assert_eq!(Staking::guarantee_rel(1, 5).get(&(0 as u32)), Some(&(250 as Balance)));
        assert_eq!(Staking::guarantee_rel(1, 5).get(&(1 as u32)), Some(&(250 as Balance)));
        assert_eq!(Staking::guarantee_rel(1, 5).get(&(2 as u32)), Some(&(150 as Balance)));
        assert_eq!(Staking::guarantee_rel(1, 5).get(&(3 as u32)), None);

        assert_ok!(Staking::cut_guarantee(Origin::signed(4), (5, 1000)));

        assert_eq!(
            Staking::validators(&5),
            Validations{
                total: 650,
                guarantee_fee: Default::default(),
                guarantors: vec![1, 1, 1]
            }
        );

        assert_eq!(Staking::guarantee_rel(3, 5).len(), 0);
        assert_eq!(Staking::guarantee_rel(1, 5).get(&(0 as u32)), Some(&(250 as Balance)));
        assert_eq!(Staking::guarantee_rel(1, 5).get(&(1 as u32)), Some(&(250 as Balance)));
        assert_eq!(Staking::guarantee_rel(1, 5).get(&(2 as u32)), Some(&(150 as Balance)));
        assert_eq!(Staking::guarantee_rel(1, 5).get(&(3 as u32)), None);

        assert_ok!(Staking::cut_guarantee(Origin::signed(2), (7, 1000))); // only 500 is valid
        assert_noop!(
            Staking::cut_guarantee(Origin::signed(2), (7, 1000)),
            DispatchError::Module {
                index: 0,
                error: 6,
                message: Some("InvalidTarget"),
            }
        );
        assert_eq!(
            Staking::validators(&7),
            Validations{
                total: 0,
                guarantee_fee: Default::default(),
                guarantors: vec![]
            }
        );
        assert_eq!(
            Staking::guarantors(&1),
            Some(Nominations {
                targets: vec![5],
                total: 650,
                submitted_in: 0,
                suppressed: false
            })
        );

        assert_ok!(Staking::guarantee(Origin::signed(2), (5, 500)));
        assert_eq!(
            Staking::validators(&5),
            Validations{
                total: 1150,
                guarantee_fee: Default::default(),
                guarantors: vec![1, 1, 1, 1]
            }
        );
        assert_eq!(
            Staking::guarantors(&1),
            Some(Nominations {
                targets: vec![5],
                total: 1150,
                submitted_in: 0,
                suppressed: false
            })
        );
    });
}

#[test]
fn chill_stash_should_work() {
    ExtBuilder::default()
    .guarantee(false)
    .build()
    .execute_with(|| {

        for i in 1..10 {
            let _ = Balances::deposit_creating(&i, 5000);
        }

        Staking::upsert_stake_limit(&5, 5000);
        assert_eq!(Staking::stake_limit(&5).unwrap_or_default(), 5000);
        Staking::upsert_stake_limit(&7, 5000);
        assert_eq!(Staking::stake_limit(&7).unwrap_or_default(), 5000);

        // add a new validator candidate
        assert_ok!(Staking::bond(
            Origin::signed(5),
            6,
            1000,
            RewardDestination::Controller
        ));
        assert_ok!(Staking::validate(Origin::signed(6), Perbill::default()));
        // add a new validator candidate
        assert_ok!(Staking::bond(
            Origin::signed(7),
            8,
            2000,
            RewardDestination::Controller
        ));
        assert_ok!(Staking::validate(Origin::signed(8), Perbill::default()));

        // add guarantor
        assert_ok!(Staking::bond(
            Origin::signed(1),
            2,
            2000,
            RewardDestination::Controller
        ));

        // add guarantor
        assert_ok!(Staking::bond(
            Origin::signed(3),
            4,
            2000,
            RewardDestination::Controller
        ));

        assert_ok!(Staking::guarantee(Origin::signed(2), (5, 250)));
        assert_ok!(Staking::guarantee(Origin::signed(2), (5, 250)));
        assert_ok!(Staking::guarantee(Origin::signed(2), (7, 250)));
        assert_ok!(Staking::guarantee(Origin::signed(2), (7, 250)));
        assert_ok!(Staking::guarantee(Origin::signed(4), (5, 250)));
        assert_ok!(Staking::guarantee(Origin::signed(4), (7, 250)));

        assert_eq!(Staking::guarantee_rel(1, 5).get(&(0 as u32)), Some(&(250 as Balance)));
        assert_eq!(Staking::guarantee_rel(1, 5).get(&(1 as u32)), Some(&(250 as Balance)));
        assert_eq!(Staking::guarantee_rel(1, 7).get(&(0 as u32)), Some(&(250 as Balance)));
        assert_eq!(Staking::guarantee_rel(1, 7).get(&(1 as u32)), Some(&(250 as Balance)));
        assert_eq!(Staking::guarantee_rel(3, 5).get(&(0 as u32)), Some(&(250 as Balance)));
        assert_eq!(Staking::guarantee_rel(3, 7).get(&(0 as u32)), Some(&(250 as Balance)));

        assert_eq!(
            Staking::guarantors(&1),
            Some(Nominations {
                targets: vec![5, 7],
                total: 1000,
                submitted_in: 0,
                suppressed: false
            })
        );

        assert_eq!(
            Staking::validators(&5),
            Validations{
                total: 750,
                guarantee_fee: Default::default(),
                guarantors: vec![1, 1, 3]
            }
        );

        assert_ok!(Staking::chill(Origin::signed(6)));

        assert_eq!(Staking::guarantee_rel(1, 5).get(&(0 as u32)), None);
        assert_eq!(Staking::guarantee_rel(1, 5).get(&(1 as u32)), None);
        assert_eq!(Staking::guarantee_rel(1, 7).get(&(0 as u32)), Some(&(250 as Balance)));
        assert_eq!(Staking::guarantee_rel(1, 7).get(&(1 as u32)), Some(&(250 as Balance)));
        assert_eq!(Staking::guarantee_rel(3, 5).get(&(0 as u32)), None);
        assert_eq!(Staking::guarantee_rel(3, 7).get(&(0 as u32)), Some(&(250 as Balance)));

        assert_eq!(
            Staking::guarantors(&1),
            Some(Nominations {
                targets: vec![7],
                total: 500,
                submitted_in: 0,
                suppressed: false
            })
        );

        assert_eq!(
            Staking::guarantors(&3),
            Some(Nominations {
                targets: vec![7],
                total: 250,
                submitted_in: 0,
                suppressed: false
            })
        );

        assert!(!<Validators<Test>>::contains_key(&5));
    });
}

#[test]
fn double_claim_rewards_should_fail() {
    ExtBuilder::default()
        .guarantee(false)
        .own_workload(u128::max_value())
        .build()
        .execute_with(|| {
            let init_balance_10 = Balances::total_balance(&10);
            let init_balance_21 = Balances::total_balance(&21);

            // Set payee to controller
            assert_ok!(Staking::set_payee(
                Origin::signed(10),
                RewardDestination::Controller
            ));

            // Compute now as other parameter won't change
            let total_authoring_payout = Staking::authoring_rewards_in_era();
            let total_staking_payout_0 = Staking::staking_rewards_in_era(Staking::current_era().unwrap_or(0));
            assert!(total_staking_payout_0 > 10); // Test is meaningful if reward something
            assert_eq!(Staking::eras_total_stakes(0), 2001);
            <Module<Test>>::reward_by_ids(vec![(11, 1)]);

            start_era(1, true);
            payout_all_stakers(0);

            assert_eq!(Staking::current_era().unwrap_or(0), 1);
            assert_eq!(Staking::eras_total_stakes(1), 2001);
            // rewards may round to 0.000001
            assert_eq!(
                Balances::total_balance(&10) / 1000000,
                (init_balance_10 + total_authoring_payout + total_staking_payout_0 * 1000 / 2001) / 1000000
            );
            let stakes_21 = Balances::total_balance(&21);
            // candidates should have rewards
            assert_eq!(
                stakes_21 / 1000000,
                (init_balance_21 + total_staking_payout_0 * 1000 / 2001) / 1000000
            );
            assert_noop!(
                Staking::reward_stakers(Origin::signed(10), 11, 0),
                DispatchError::Module {
                    index: 0,
                    error: 14,
                    message: Some("AlreadyClaimed"),
                }
            );
            assert_noop!(
                Staking::reward_stakers(Origin::signed(10), 21, 0),
                DispatchError::Module {
                    index: 0,
                    error: 14,
                    message: Some("AlreadyClaimed"),
                }
            );
            assert_noop!(
                Staking::reward_stakers(Origin::signed(10), 31, 0),
                DispatchError::Module {
                    index: 0,
                    error: 14,
                    message: Some("AlreadyClaimed"),
                }
            );
        });
}

#[test]
fn era_clean_should_work() {
    ExtBuilder::default()
        .guarantee(false)
        .own_workload(u128::max_value())
        .build()
        .execute_with(|| {
            // Set payee to controller
            assert_ok!(Staking::set_payee(
                Origin::signed(10),
                RewardDestination::Controller
            ));
            // We left 3 buffer here even we set it to 84
            start_era(84, true);
            assert!(<ErasStakingPayout<Test>>::contains_key(0));
            assert!(<ErasTotalStakes<Test>>::contains_key(0));
            assert!(<ErasStakers<Test>>::contains_key(0, 11));
            assert!(<ErasValidatorPrefs<Test>>::contains_key(0, 11));
            start_era(85, true);
            assert!(!<ErasStakingPayout<Test>>::contains_key(0));
            assert!(!<ErasTotalStakes<Test>>::contains_key(0));
            assert!(!<ErasStakers<Test>>::contains_key(0, 11));
            assert!(!<ErasValidatorPrefs<Test>>::contains_key(0, 11));
        });
}
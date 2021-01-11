// Copyright (C) 2019-2021 Crust Network Technologies Ltd.
// This file is part of Crust.

use super::*;

use crate::mock::*;
use frame_support::{
    assert_noop, assert_ok,
    dispatch::DispatchError,
};
use hex;
use crate::MerchantLedger;

/// Register & Pledge test cases
#[test]
fn register_should_work() {
    new_test_ext().execute_with(|| {
        // generate 50 blocks first
        run_to_block(50);
        let merchant = MERCHANT;
        let pledge_pot = Market::pledge_pot();
        let _ = Balances::make_free_balance_be(&merchant, 200);
        assert_ok!(Market::register(Origin::signed(merchant.clone()), 180));
        assert_eq!(Market::merchant_ledgers(&merchant), MerchantLedger {
            pledge: 180,
            reward: 0
        });
        assert_eq!(Balances::free_balance(&pledge_pot), 180);
        assert_ok!(Market::cut_pledge(Origin::signed(merchant.clone()), 20));
        assert_eq!(Market::merchant_ledgers(&merchant), MerchantLedger {
            pledge: 160,
            reward: 0
        });
        assert_eq!(Balances::free_balance(&pledge_pot), 160);
        assert_ok!(Market::pledge_extra(Origin::signed(merchant.clone()), 10));
        assert_eq!(Market::merchant_ledgers(merchant), MerchantLedger {
            pledge: 170,
            reward: 0
        });
        assert_eq!(Balances::free_balance(&pledge_pot), 170);
    });
}

#[test]
fn register_should_fail_due_to_insufficient_currency() {
    new_test_ext().execute_with(|| {
        // generate 50 blocks first
        run_to_block(50);

        let merchant = MERCHANT;
        let _ = Balances::make_free_balance_be(&merchant, 100);
        assert_noop!(
            Market::register(
                Origin::signed(merchant),
                200
            ),
            DispatchError::Module {
                index: 0,
                error: 0,
                message: Some("InsufficientCurrency")
            }
        );
    });
}

#[test]
fn register_should_fail_due_to_double_register() {
    new_test_ext().execute_with(|| {
        // generate 50 blocks first
        run_to_block(50);
        let merchant = MERCHANT;
        let _ = Balances::make_free_balance_be(&merchant, 200);
        assert_ok!(Market::register(Origin::signed(merchant.clone()), 70));
        assert_noop!(
            Market::register(
                Origin::signed(merchant),
                70
            ),
            DispatchError::Module {
                index: 0,
                error: 4,
                message: Some("AlreadyRegistered")
            }
        );
    });
}

#[test]
fn pledge_extra_should_fail_due_to_merchant_not_register() {
    new_test_ext().execute_with(|| {
        // generate 50 blocks first
        run_to_block(50);

        let merchant = MERCHANT;
        let _ = Balances::make_free_balance_be(&merchant, 200);
        assert_noop!(
            Market::pledge_extra(
                Origin::signed(merchant),
                200
            ),
            DispatchError::Module {
                index: 0,
                error: 3,
                message: Some("NotRegister")
            }
        );
    });
}

#[test]
fn cut_pledge_should_fail_due_to_reward() {
    new_test_ext().execute_with(|| {
        // generate 50 blocks first
        run_to_block(50);

        let merchant = MERCHANT;
        let _ = Balances::make_free_balance_be(&merchant, 200);
        assert_ok!(Market::register(Origin::signed(merchant.clone()), 180));
        assert_eq!(Market::merchant_ledgers(&merchant), MerchantLedger {
            pledge: 180,
            reward: 0
        });

        <self::MerchantLedgers<Test>>::insert(&merchant, MerchantLedger {
            pledge: 180,
            reward: 120
        });
        assert_ok!(Market::cut_pledge(Origin::signed(merchant.clone()), 20));
        assert_eq!(Market::merchant_ledgers(&merchant), MerchantLedger {
            pledge: 160,
            reward: 120
        });
        assert_noop!(
            Market::cut_pledge(
                Origin::signed(merchant),
                50
            ),
            DispatchError::Module {
                index: 0,
                error: 1,
                message: Some("InsufficientPledge")
            }
        );
    });
}

/// Place storage order test cases
#[test]
fn place_storage_order_should_work() {
    new_test_ext().execute_with(|| {
        // generate 50 blocks first
        run_to_block(50);

        let source = ALICE;
        let merchant = MERCHANT;

        let cid =
        hex::decode("4e2883ddcbc77cf19979770d756fd332d0c8f815f9de646636169e460e6af6ff").unwrap();
        let file_size = 100; // should less than
        let reserved_pot = Market::reserved_pot();
        let staking_pot = Market::staking_pot();
        let storage_pot = Market::storage_pot();
        assert_eq!(Balances::free_balance(&staking_pot), 0);
        let _ = Balances::make_free_balance_be(&source, 20000);
        let _ = Balances::make_free_balance_be(&merchant, 200);

        assert_ok!(Market::register(Origin::signed(merchant.clone()), 60));

        assert_ok!(Market::place_storage_order(
            Origin::signed(source), cid.clone(),
            file_size, 0, false
        ));

        assert_eq!(Market::files(&cid).unwrap_or_default(), (
            FileInfo {
                file_size,
                expired_on: 50,
                claimed_at: 50,
                amount: 360, // ( 1000 + 1000 * 1 + 0 ) * 0.2
                expected_replica_count: 4,
                reported_replica_count: 0,
                replicas: vec![]
            },
            UsedInfo {
                used_size: file_size,
                groups: BTreeSet::from_iter(vec![].into_iter())
            })
        );
        assert_eq!(Balances::free_balance(reserved_pot), 200);
        assert_eq!(Balances::free_balance(staking_pot), 1440);
        assert_eq!(Balances::free_balance(storage_pot), 360);
    });
}

#[test]
// For extends:
// 1. Add amount
// 2. Extend duration
// 3. Extend replicas
fn place_storage_order_should_work_for_extend_scenarios() {
    new_test_ext().execute_with(|| {
        // generate 50 blocks first
        run_to_block(50);

        let source = ALICE;
        let merchant = MERCHANT;

        let cid =
            hex::decode("5bb706320afc633bfb843108e492192b17d2b6b9d9ee0b795ee95417fe08b660").unwrap();
        let file_size = 100; // should less than merchant
        let staking_pot = Market::staking_pot();
        let storage_pot = Market::storage_pot();
        assert_eq!(Balances::free_balance(&staking_pot), 0);
        let _ = Balances::make_free_balance_be(&source, 20000);
        let _ = Balances::make_free_balance_be(&merchant, 20000);

        assert_ok!(Market::register(Origin::signed(merchant.clone()), 6000));

        // 1. New storage order
        assert_ok!(Market::place_storage_order(
            Origin::signed(source.clone()), cid.clone(),
            file_size, 0, false
        ));

        assert_eq!(Market::files(&cid).unwrap_or_default(), (
            FileInfo {
                file_size,
                expired_on: 50,
                claimed_at: 50,
                amount: 360, // ( 1000 + 1000 * 1 + 0 ) * 0.2
                expected_replica_count: 4,
                reported_replica_count: 0,
                replicas: vec![]
            },
            UsedInfo {
                used_size: file_size,
                groups: BTreeSet::from_iter(vec![].into_iter())
            })
        );
        assert_eq!(Balances::free_balance(&staking_pot), 1440);
        assert_eq!(Balances::free_balance(&storage_pot), 360);

        run_to_block(250);
        
        // 2. Add amount for sOrder not begin should work
        assert_ok!(Market::place_storage_order(
            Origin::signed(source.clone()), cid.clone(),
            file_size, 0, false
        ));

        assert_eq!(Market::files(&cid).unwrap_or_default(), (
            FileInfo {
                file_size,
                expired_on: 50,
                claimed_at: 50,
                amount: 720, // ( 1000 + 1000 * 1 + 0 ) * 0.2
                expected_replica_count: 4,
                reported_replica_count: 0,
                replicas: vec![]
            },
            UsedInfo {
                used_size: file_size,
                groups: BTreeSet::from_iter(vec![].into_iter())
            })
        );
        assert_eq!(Balances::free_balance(&staking_pot), 2880);
        assert_eq!(Balances::free_balance(&storage_pot), 720);

        let legal_wr_info = legal_work_report_with_added_files();
        let legal_pk = legal_wr_info.curr_pk.clone();

        // Report this cid's works
        register(&legal_pk, LegalCode::get());
        run_to_block(400);
        assert_ok!(Swork::report_works(
                Origin::signed(merchant.clone()),
                legal_wr_info.curr_pk,
                legal_wr_info.prev_pk,
                legal_wr_info.block_number,
                legal_wr_info.block_hash,
                legal_wr_info.free,
                legal_wr_info.used,
                legal_wr_info.added_files,
                legal_wr_info.deleted_files,
                legal_wr_info.srd_root,
                legal_wr_info.files_root,
                legal_wr_info.sig
            ));

        assert_eq!(Market::files(&cid).unwrap_or_default(), (
            FileInfo {
                file_size,
                expired_on: 1400,
                claimed_at: 400,
                amount: 720, // ( 1000 + 1000 * 1 + 0 ) * 0.2
                expected_replica_count: 4,
                reported_replica_count: 1,
                replicas: vec![Replica {
                    who: merchant.clone(),
                    valid_at: 303,
                    anchor: legal_pk.clone()
                }]
            },
            UsedInfo {
                used_size: file_size,
                groups: BTreeSet::from_iter(vec![legal_pk.clone()].into_iter())
            })
        );

        // Calculate mannual claim reward should work
        run_to_block(500);
        <swork::ReportedInSlot>::insert(legal_pk.clone(), 0, true);
        assert_ok!(Market::calculate_reward(Origin::signed(merchant.clone()), cid.clone()));
        assert_eq!(Market::files(&cid).unwrap_or_default(), (
            FileInfo {
                file_size,
                expired_on: 1400,
                claimed_at: 500,
                amount: 649,
                expected_replica_count: 4,
                reported_replica_count: 1,
                replicas: vec![Replica {
                    who: merchant.clone(),
                    valid_at: 303,
                    anchor: legal_pk.clone()
                }]
            },
            UsedInfo {
                used_size: file_size,
                groups: BTreeSet::from_iter(vec![legal_pk.clone()].into_iter())
            })
        );

        // 3. Extend duration should work
        run_to_block(600);
        assert_ok!(Market::place_storage_order(
            Origin::signed(source.clone()), cid.clone(),
            file_size, 0, false
        ));

        assert_eq!(Market::files(&cid).unwrap_or_default(), (
            FileInfo {
                file_size,
                expired_on: 1600,
                claimed_at: 600,
                amount: 938,
                expected_replica_count: 4,
                reported_replica_count: 1,
                replicas: vec![Replica {
                    who: merchant.clone(),
                    valid_at: 303,
                    anchor: legal_pk.clone()
                }]
            },
            UsedInfo {
                used_size: file_size,
                groups: BTreeSet::from_iter(vec![legal_pk.clone()].into_iter())
            })
        );

        // 4. Extend replicas should work
        run_to_block(800);
        assert_ok!(Market::place_storage_order(
            Origin::signed(source.clone()), cid.clone(),
            file_size, 200, true
        ));

        assert_eq!(Market::files(&cid).unwrap_or_default(), (
            FileInfo {
                file_size,
                expired_on: 1800,
                claimed_at: 800,
                amount: 1147,
                expected_replica_count: 8,
                reported_replica_count: 1,
                replicas: vec![Replica {
                    who: merchant.clone(),
                    valid_at: 303,
                    anchor: legal_pk.clone()
                }]
            },
            UsedInfo {
                used_size: file_size,
                groups: BTreeSet::from_iter(vec![legal_pk.clone()].into_iter())
            })
        );
    });
}


/// Payouts test cases
#[test]
// Payout should be triggered by:
// 1. Delete file(covered by swork module)
// 2. Claim reward
// 3. Place started storage order(covered by `place_storage_order_should_work_for_extend_scenarios`)
fn calculate_payout_should_work() {
    new_test_ext().execute_with(|| {
        // generate 50 blocks first
        run_to_block(50);
 
        let source = ALICE;
        let merchant = MERCHANT;

        let cid =
            hex::decode("5bb706320afc633bfb843108e492192b17d2b6b9d9ee0b795ee95417fe08b660").unwrap();
        let file_size = 100; // should less than merchant
        let staking_pot = Market::staking_pot();
        let storage_pot = Market::storage_pot();
        let reserved_pot = Market::reserved_pot();
        assert_eq!(Balances::free_balance(&staking_pot), 0);
        let _ = Balances::make_free_balance_be(&source, 20000);
        let _ = Balances::make_free_balance_be(&merchant, 20000);

        assert_ok!(Market::register(Origin::signed(merchant.clone()), 6000));

        // 1. Place an order 1st
        assert_ok!(Market::place_storage_order(
            Origin::signed(source), cid.clone(),
            file_size, 0, false
        ));

        assert_eq!(Market::files(&cid).unwrap_or_default(), (
            FileInfo {
                file_size,
                expired_on: 50,
                claimed_at: 50,
                amount: 360, // ( 1000 + 1000 * 1 + 0 ) * 0.9 * 0.2
                expected_replica_count: 4,
                reported_replica_count: 0,
                replicas: vec![]
            },
            UsedInfo {
                used_size: file_size,
                groups: BTreeSet::from_iter(vec![].into_iter())
            })
        );

        assert_eq!(Balances::free_balance(reserved_pot), 200);
        assert_eq!(Balances::free_balance(staking_pot), 1440);
        assert_eq!(Balances::free_balance(storage_pot), 360);

        run_to_block(303);
        let legal_wr_info = legal_work_report_with_added_files();
        let legal_pk = legal_wr_info.curr_pk.clone();

        register(&legal_pk, LegalCode::get());

        // 2. Report this file's work, let file begin
        assert_ok!(Swork::report_works(
                Origin::signed(merchant.clone()),
                legal_wr_info.curr_pk,
                legal_wr_info.prev_pk,
                legal_wr_info.block_number,
                legal_wr_info.block_hash,
                legal_wr_info.free,
                legal_wr_info.used,
                legal_wr_info.added_files,
                legal_wr_info.deleted_files,
                legal_wr_info.srd_root,
                legal_wr_info.files_root,
                legal_wr_info.sig
            ));

        assert_eq!(Market::files(&cid).unwrap_or_default(), (
            FileInfo {
                file_size,
                expired_on: 1303,
                claimed_at: 303,
                amount: 360, // ( 1000 + 1000 * 1 + 0 ) * 0.2
                expected_replica_count: 4,
                reported_replica_count: 1,
                replicas: vec![Replica {
                    who: merchant.clone(),
                    valid_at: 303,
                    anchor: legal_pk.clone()
                }]
            },
            UsedInfo {
                used_size: file_size,
                groups: BTreeSet::from_iter(vec![legal_pk.clone()].into_iter())
            })
        );

        // 3. Go along with some time, and get reward
        run_to_block(606);
        <swork::ReportedInSlot>::insert(legal_pk.clone(), 300, true);
        assert_ok!(Market::calculate_reward(Origin::signed(merchant.clone()), cid.clone()));
        assert_eq!(Market::files(&cid).unwrap_or_default(), (
            FileInfo {
                file_size,
                expired_on: 1303,
                claimed_at: 606,
                amount: 252, // ( 1000 + 1000 * 1 + 0 ) * 0.2
                expected_replica_count: 4,
                reported_replica_count: 1,
                replicas: vec![Replica {
                    who: merchant.clone(),
                    valid_at: 303,
                    anchor: legal_pk.clone()
                }]
            },
            UsedInfo {
                used_size: file_size,
                groups: BTreeSet::from_iter(vec![legal_pk.clone()].into_iter())
            })
        );
        assert_eq!(Market::merchant_ledgers(&merchant), MerchantLedger {
            pledge: 6000,
            reward: 108
        })
    });
}

#[test]
fn calculate_payout_should_fail_due_to_insufficient_pledge() {
    new_test_ext().execute_with(|| {
        // generate 50 blocks first
        run_to_block(50);

        let source = ALICE;
        let merchant = MERCHANT;

        let cid =
            hex::decode("5bb706320afc633bfb843108e492192b17d2b6b9d9ee0b795ee95417fe08b660").unwrap();
        let file_size = 100; // should less than merchant
        let _ = Balances::make_free_balance_be(&source, 20000);
        let _ = Balances::make_free_balance_be(&merchant, 20000);

        assert_ok!(Market::register(Origin::signed(merchant.clone()), 60));

        assert_ok!(Market::place_storage_order(
            Origin::signed(source), cid.clone(),
            file_size, 0, false
        ));

        assert_eq!(Market::files(&cid).unwrap_or_default(), (
            FileInfo {
                file_size,
                expired_on: 50,
                claimed_at: 50,
                amount: 360, // ( 1000 + 1000 * 1 + 0 ) * 0.2
                expected_replica_count: 4,
                reported_replica_count: 0,
                replicas: vec![]
            },
            UsedInfo {
                used_size: file_size,
                groups: BTreeSet::from_iter(vec![].into_iter())
            })
        );

        run_to_block(303);

        let legal_wr_info = legal_work_report_with_added_files();
        let legal_pk = legal_wr_info.curr_pk.clone();

        register(&legal_pk, LegalCode::get());

        assert_ok!(Swork::report_works(
                Origin::signed(merchant.clone()),
                legal_wr_info.curr_pk,
                legal_wr_info.prev_pk,
                legal_wr_info.block_number,
                legal_wr_info.block_hash,
                legal_wr_info.free,
                legal_wr_info.used,
                legal_wr_info.added_files,
                legal_wr_info.deleted_files,
                legal_wr_info.srd_root,
                legal_wr_info.files_root,
                legal_wr_info.sig
            ));

        assert_eq!(Market::files(&cid).unwrap_or_default(), (
            FileInfo {
                file_size,
                expired_on: 1303,
                claimed_at: 303,
                amount: 360, // ( 1000 + 1000 * 1 + 0 ) * 0.2
                expected_replica_count: 4,
                reported_replica_count: 1,
                replicas: vec![Replica {
                    who: merchant.clone(),
                    valid_at: 303,
                    anchor: legal_pk.clone()
                }]
            },
            UsedInfo {
                used_size: file_size,
                groups: BTreeSet::from_iter(vec![legal_pk.clone()].into_iter())
            })
        );

        run_to_block(603);
        <swork::ReportedInSlot>::insert(legal_pk.clone(), 300, true);
        assert_ok!(Market::calculate_reward(Origin::signed(merchant.clone()), cid.clone()));
        assert_eq!(Market::files(&cid).unwrap_or_default(), (
            FileInfo {
                file_size,
                expired_on: 1303,
                claimed_at: 603,
                amount: 360, // ( 1000 + 1000 * 1 + 0 ) * 0.2
                expected_replica_count: 4,
                reported_replica_count: 1,
                replicas: vec![Replica {
                    who: merchant.clone(),
                    valid_at: 303,
                    anchor: legal_pk.clone()
                }]
            },
            UsedInfo {
                used_size: file_size,
                groups: BTreeSet::from_iter(vec![legal_pk.clone()].into_iter())
            })
        );

        // pledge is 60 < 121 reward
        assert_eq!(Market::merchant_ledgers(&merchant), MerchantLedger {
            pledge: 60,
            reward: 0
        });

        run_to_block(903);
        <swork::ReportedInSlot>::insert(legal_pk.clone(), 600, true);
        assert_ok!(Market::pledge_extra(Origin::signed(merchant.clone()), 6000));
        assert_ok!(Market::calculate_reward(Origin::signed(merchant.clone()), cid.clone()));
        assert_eq!(Market::files(&cid).unwrap_or_default(), (
            FileInfo {
                file_size,
                expired_on: 1303,
                claimed_at: 903,
                amount: 207, // ( 1000 + 1000 * 1 + 0 ) * 0.2
                expected_replica_count: 4,
                reported_replica_count: 1,
                replicas: vec![Replica {
                    who: merchant.clone(),
                    valid_at: 303,
                    anchor: legal_pk.clone()
                }]
            },
            UsedInfo {
                used_size: file_size,
                groups: BTreeSet::from_iter(vec![legal_pk.clone()].into_iter())
            })
        );

        assert_eq!(Market::merchant_ledgers(&merchant), MerchantLedger {
            pledge: 6060,
            reward: 153
        })
    });
}

#[test]
fn calculate_payout_should_move_file_to_trash_due_to_expired() {
    new_test_ext().execute_with(|| {
        // generate 50 blocks first
        run_to_block(50);

        let source = ALICE;
        let merchant = MERCHANT;

        let cid =
            hex::decode("5bb706320afc633bfb843108e492192b17d2b6b9d9ee0b795ee95417fe08b660").unwrap();
        let file_size = 100; // should less than merchant
        let _ = Balances::make_free_balance_be(&source, 20000);
        let _ = Balances::make_free_balance_be(&merchant, 20000);

        assert_ok!(Market::register(Origin::signed(merchant.clone()), 6000));

        assert_ok!(Market::place_storage_order(
            Origin::signed(source), cid.clone(),
            file_size, 0, false
        ));

        assert_eq!(Market::files(&cid).unwrap_or_default(), (
            FileInfo {
                file_size,
                expired_on: 50,
                claimed_at: 50,
                amount: 360, // ( 1000 + 1000 * 1 + 0 ) * 0.2
                expected_replica_count: 4,
                reported_replica_count: 0,
                replicas: vec![]
            },
            UsedInfo {
                used_size: file_size,
                groups: BTreeSet::from_iter(vec![].into_iter())
            })
        );

        run_to_block(303);

        let legal_wr_info = legal_work_report_with_added_files();
        let legal_pk = legal_wr_info.curr_pk.clone();

        register(&legal_pk, LegalCode::get());

        assert_ok!(Swork::report_works(
                Origin::signed(merchant.clone()),
                legal_wr_info.curr_pk,
                legal_wr_info.prev_pk,
                legal_wr_info.block_number,
                legal_wr_info.block_hash,
                legal_wr_info.free,
                legal_wr_info.used,
                legal_wr_info.added_files,
                legal_wr_info.deleted_files,
                legal_wr_info.srd_root,
                legal_wr_info.files_root,
                legal_wr_info.sig
            ));

        assert_eq!(Market::files(&cid).unwrap_or_default(), (
            FileInfo {
                file_size,
                expired_on: 1303,
                claimed_at: 303,
                amount: 360, // ( 1000 + 1000 * 1 + 0 ) * 0.2
                expected_replica_count: 4,
                reported_replica_count: 1,
                replicas: vec![Replica {
                    who: merchant.clone(),
                    valid_at: 303,
                    anchor: legal_pk.clone()
                }]
            },
            UsedInfo {
                used_size: file_size,
                groups: BTreeSet::from_iter(vec![legal_pk.clone()].into_iter())
            })
        );

        run_to_block(1506);
        // expired_on is 1303. So to check 900
        <swork::ReportedInSlot>::insert(legal_pk.clone(), 900, true);
        assert_ok!(Market::calculate_reward(Origin::signed(merchant.clone()), cid.clone()));
        assert_eq!(Market::files(&cid), None);

        assert_eq!(Market::used_trash_i(&cid).unwrap_or_default(), UsedInfo {
            used_size: file_size,
            groups: BTreeSet::from_iter(vec![legal_pk.clone()].into_iter())
        });

        assert_eq!(Market::merchant_ledgers(&merchant), MerchantLedger {
            pledge: 6000,
            reward: 359
        })
    });
}

#[test]
fn calculate_payout_should_work_in_complex_timeline() {
    new_test_ext().execute_with(|| {
        // generate 50 blocks first
        run_to_block(50);

        let cid =
            hex::decode("5bb706320afc633bfb843108e492192b17d2b6b9d9ee0b795ee95417fe08b660").unwrap();
        let file_size = 100; // should less than merchant
        let source = ALICE;
        let merchant = BOB;
        let charlie = CHARLIE;
        let dave = DAVE;
        let eve = EVE;

        let staking_pot = Market::staking_pot();
        let storage_pot = Market::storage_pot();
        let reserved_pot = Market::reserved_pot();
        assert_eq!(Balances::free_balance(&staking_pot), 0);
        assert_eq!(Balances::free_balance(&reserved_pot), 0);
        let _ = Balances::make_free_balance_be(&source, 20000);
        let merchants = vec![merchant.clone(), charlie.clone(), dave.clone(), eve.clone()];
        for who in merchants.iter() {
            let _ = Balances::make_free_balance_be(&who, 20000);
            assert_ok!(Market::register(Origin::signed(who.clone()), 6000));
        }

        assert_ok!(Market::place_storage_order(
            Origin::signed(source), cid.clone(),
            file_size, 0, false
        ));

        assert_eq!(Market::files(&cid).unwrap_or_default(), (
            FileInfo {
                file_size,
                expired_on: 50,
                claimed_at: 50,
                amount: 360, // ( 1000 + 1000 * 1 + 0 ) * 0.2
                expected_replica_count: 4,
                reported_replica_count: 0,
                replicas: vec![]
            },
            UsedInfo {
                used_size: file_size,
                groups: BTreeSet::from_iter(vec![].into_iter())
            })
        );

        assert_eq!(Balances::free_balance(&reserved_pot), 200);
        assert_eq!(Balances::free_balance(&staking_pot), 1440);
        assert_eq!(Balances::free_balance(&storage_pot), 360);

        run_to_block(303);

        let legal_wr_info = legal_work_report_with_added_files();
        let legal_pk = legal_wr_info.curr_pk.clone();

        register(&legal_pk, LegalCode::get());

        assert_ok!(Swork::report_works(
                Origin::signed(merchant.clone()),
                legal_wr_info.curr_pk,
                legal_wr_info.prev_pk,
                legal_wr_info.block_number,
                legal_wr_info.block_hash,
                legal_wr_info.free,
                legal_wr_info.used,
                legal_wr_info.added_files,
                legal_wr_info.deleted_files,
                legal_wr_info.srd_root,
                legal_wr_info.files_root,
                legal_wr_info.sig
            ));

        assert_eq!(Market::files_size(), file_size as u128);

        assert_eq!(Market::files(&cid).unwrap_or_default(), (
            FileInfo {
                file_size,
                expired_on: 1303,
                claimed_at: 303,
                amount: 360, // ( 1000 + 1000 * 1 + 0 ) * 0.2
                expected_replica_count: 4,
                reported_replica_count: 1,
                replicas: vec![Replica {
                    who: merchant.clone(),
                    valid_at: 303,
                    anchor: legal_pk.clone()
                }]
            },
            UsedInfo {
                used_size: file_size,
                groups: BTreeSet::from_iter(vec![legal_pk.clone()].into_iter())
            })
        );

        run_to_block(503);
        <swork::ReportedInSlot>::insert(legal_pk.clone(), 0, true);
        assert_ok!(Market::calculate_reward(Origin::signed(merchant.clone()), cid.clone()));
        assert_eq!(Market::files(&cid).unwrap_or_default(), (
            FileInfo {
                file_size,
                expired_on: 1303,
                claimed_at: 503,
                amount: 289,
                expected_replica_count: 4,
                reported_replica_count: 1,
                replicas: vec![Replica {
                    who: merchant.clone(),
                    valid_at: 303,
                    anchor: legal_pk.clone()
                }]
            },
            UsedInfo {
                used_size: file_size,
                groups: BTreeSet::from_iter(vec![legal_pk.clone()].into_iter())
            })
        );
        assert_eq!(Market::merchant_ledgers(&merchant), MerchantLedger {
            pledge: 6000,
            reward: 71
        });

        add_who_into_replica(&cid, charlie.clone(), legal_pk.clone(), None);

        run_to_block(603);
        <swork::ReportedInSlot>::insert(legal_pk.clone(), 300, true);
        assert_ok!(Market::calculate_reward(Origin::signed(merchant.clone()), cid.clone()));
        assert_eq!(Market::files(&cid).unwrap_or_default(), (
            FileInfo {
                file_size,
                expired_on: 1303,
                claimed_at: 603,
                amount: 255,
                expected_replica_count: 4,
                reported_replica_count: 2,
                replicas: vec![
                    Replica {
                        who: merchant.clone(),
                        valid_at: 303,
                        anchor: legal_pk.clone()
                    },
                    Replica {
                        who: charlie.clone(),
                        valid_at: 503,
                        anchor: legal_pk.clone()
                    }]
            },
            UsedInfo {
                used_size: file_size,
                groups: BTreeSet::from_iter(vec![legal_pk.clone()].into_iter())
            })
        );
        assert_eq!(Market::merchant_ledgers(&merchant), MerchantLedger {
            pledge: 6000,
            reward: 88
        });
        assert_eq!(Market::merchant_ledgers(&charlie), MerchantLedger {
            pledge: 6000,
            reward: 17
        });

        assert_eq!(Market::files_size(), (file_size * 2) as u128);

        add_who_into_replica(&cid, dave.clone(), hex::decode("11").unwrap(), None);
        run_to_block(703);
        assert_ok!(Market::calculate_reward(Origin::signed(merchant.clone()), cid.clone()));
        assert_eq!(Market::files(&cid).unwrap_or_default(), (
            FileInfo {
                file_size,
                expired_on: 1303,
                claimed_at: 703,
                amount: 233,
                expected_replica_count: 4,
                reported_replica_count: 2,
                replicas: vec![
                    Replica {
                        who: merchant.clone(),
                        valid_at: 303,
                        anchor: legal_pk.clone()
                    },
                    Replica {
                        who: charlie.clone(),
                        valid_at: 503,
                        anchor: legal_pk.clone()
                    },
                    Replica {
                        who: dave.clone(),
                        valid_at: 703, // did't report. change it to curr bn
                        anchor: hex::decode("11").unwrap()
                    }]
            },
            UsedInfo {
                used_size: file_size,
                groups: BTreeSet::from_iter(vec![legal_pk.clone(), hex::decode("11").unwrap()].into_iter())
            })
        );
        assert_eq!(Market::merchant_ledgers(&merchant), MerchantLedger {
            pledge: 6000,
            reward: 99
        });
        assert_eq!(Market::merchant_ledgers(&charlie), MerchantLedger {
            pledge: 6000,
            reward: 28
        });

        assert_eq!(Market::files_size(), (file_size * 2) as u128);

        run_to_block(903);
        <swork::ReportedInSlot>::insert(hex::decode("11").unwrap(), 600, true);
        assert_ok!(Market::calculate_reward(Origin::signed(merchant.clone()), cid.clone()));
        assert_eq!(Market::files(&cid).unwrap_or_default(), (
            FileInfo {
                file_size,
                expired_on: 1303,
                claimed_at: 903,
                amount: 208,
                expected_replica_count: 4,
                reported_replica_count: 1,
                replicas: vec![
                    Replica {
                        who: dave.clone(),
                        valid_at: 703,
                        anchor: hex::decode("11").unwrap()
                    },
                    Replica {
                        who: merchant.clone(),
                        valid_at: 903, // did't report. change it to curr bn
                        anchor: legal_pk.clone()
                    },
                    Replica {
                        who: charlie.clone(),
                        valid_at: 903, // did't report. change it to curr bn
                        anchor: legal_pk.clone()
                    }]
            },
            UsedInfo {
                used_size: file_size,
                groups: BTreeSet::from_iter(vec![legal_pk.clone(), hex::decode("11").unwrap()].into_iter())
            })
        );
        assert_eq!(Market::merchant_ledgers(&merchant), MerchantLedger {
            pledge: 6000,
            reward: 99
        });
        assert_eq!(Market::merchant_ledgers(&charlie), MerchantLedger {
            pledge: 6000,
            reward: 28
        });
        assert_eq!(Market::merchant_ledgers(&dave), MerchantLedger {
            pledge: 6000,
            reward: 25
        });

        assert_eq!(Market::files_size(), file_size as u128);

        run_to_block(1203);
        <swork::ReportedInSlot>::insert(hex::decode("11").unwrap(), 900, true);
        <swork::ReportedInSlot>::insert(legal_pk.clone(), 900, true);
        assert_ok!(Market::calculate_reward(Origin::signed(merchant.clone()), cid.clone()));
        assert_eq!(Market::files(&cid).unwrap_or_default(), (
            FileInfo {
                file_size,
                expired_on: 1303,
                claimed_at: 1203,
                amount: 55,
                expected_replica_count: 4,
                reported_replica_count: 3,
                replicas: vec![
                    Replica {
                        who: dave.clone(),
                        valid_at: 703,
                        anchor: hex::decode("11").unwrap()
                    },
                    Replica {
                        who: merchant.clone(),
                        valid_at: 903,
                        anchor: legal_pk.clone()
                    },
                    Replica {
                        who: charlie.clone(),
                        valid_at: 903,
                        anchor: legal_pk.clone()
                    }]
            },
            UsedInfo {
                used_size: file_size,
                groups: BTreeSet::from_iter(vec![legal_pk.clone(), hex::decode("11").unwrap()].into_iter())
            })
        );
        assert_eq!(Market::merchant_ledgers(&merchant), MerchantLedger {
            pledge: 6000,
            reward: 150
        });
        assert_eq!(Market::merchant_ledgers(&charlie), MerchantLedger {
            pledge: 6000,
            reward: 79
        });
        assert_eq!(Market::merchant_ledgers(&dave), MerchantLedger {
            pledge: 6000,
            reward: 76
        });
        assert_eq!(Market::files_size(), (file_size * 3) as u128);

        run_to_block(1803);
        assert_ok!(Market::calculate_reward(Origin::signed(merchant.clone()), cid.clone()));
        let mut groups = <BTreeSet<SworkerAnchor>>::new();
        groups.insert(legal_pk.clone());
        groups.insert(hex::decode("11").unwrap());
        assert_eq!(Market::used_trash_i(&cid).unwrap_or_default(), UsedInfo {
            used_size: file_size,
            groups
        });

        assert_eq!(Market::merchant_ledgers(&merchant), MerchantLedger {
            pledge: 6000,
            reward: 167
        });
        assert_eq!(Market::merchant_ledgers(&charlie), MerchantLedger {
            pledge: 6000,
            reward: 96
        });
        assert_eq!(Market::merchant_ledgers(&dave), MerchantLedger {
            pledge: 6000,
            reward: 93
        });
        assert_eq!(Market::files_size(), 0);
        assert_eq!(Balances::free_balance(&reserved_pot), 204);
    });
}

#[test]
fn calculate_payout_should_fail_due_to_not_live() {
    new_test_ext().execute_with(|| {
        // generate 50 blocks first
        run_to_block(50);

        let source = ALICE;
        let merchant = MERCHANT;

        let cid =
            hex::decode("5bb706320afc633bfb843108e492192b17d2b6b9d9ee0b795ee95417fe08b660").unwrap();
        let file_size = 100; // should less than merchant

        let _ = Balances::make_free_balance_be(&source, 20000);
        let _ = Balances::make_free_balance_be(&merchant, 20000);

        // pledge is 60 < 121 reward
        assert_ok!(Market::register(Origin::signed(merchant.clone()), 6000));

        assert_ok!(Market::place_storage_order(
            Origin::signed(source), cid.clone(),
            file_size, 0, false
        ));

        assert_eq!(Market::files(&cid).unwrap_or_default(), (
            FileInfo {
                file_size,
                expired_on: 50,
                claimed_at: 50,
                amount: 360, // ( 1000 + 1000 * 1 + 0 ) * 0.2
                expected_replica_count: 4,
                reported_replica_count: 0,
                replicas: vec![]
            },
            UsedInfo {
                used_size: file_size,
                groups: BTreeSet::from_iter(vec![].into_iter())
            })
        );

        run_to_block(303);

        let legal_wr_info = legal_work_report_with_added_files();
        let legal_pk = legal_wr_info.curr_pk.clone();

        register(&legal_pk, LegalCode::get());

        assert_eq!(Market::files(&cid).unwrap_or_default(), (
            FileInfo {
                file_size,
                expired_on: 50,
                claimed_at: 50,
                amount: 360, // ( 1000 + 1000 * 1 + 0 ) * 0.2
                expected_replica_count: 4,
                reported_replica_count: 0,
                replicas: vec![]
            },
            UsedInfo {
                used_size: file_size,
                groups: BTreeSet::from_iter(vec![].into_iter())
            })
        );

        run_to_block(1506);
        assert_ok!(Market::calculate_reward(Origin::signed(merchant.clone()), cid.clone()));

        assert_eq!(Market::files(&cid).unwrap_or_default(), (
            FileInfo {
                file_size,
                expired_on: 50,
                claimed_at: 50,
                amount: 360, // ( 1000 + 1000 * 1 + 0 ) * 0.2
                expected_replica_count: 4,
                reported_replica_count: 0,
                replicas: vec![]
            },
            UsedInfo {
                used_size: file_size,
                groups: BTreeSet::from_iter(vec![].into_iter())
            })
        );
    });
}

#[test]
fn calculate_payout_should_work_for_more_replicas() {
    new_test_ext().execute_with(|| {
        // generate 50 blocks first
        run_to_block(50);

        let source = ALICE;
        let merchant = MERCHANT;
        let charlie = CHARLIE;
        let dave = DAVE;
        let eve = EVE;
        let ferdie = FERDIE;

        let cid =
            hex::decode("5bb706320afc633bfb843108e492192b17d2b6b9d9ee0b795ee95417fe08b660").unwrap();
        let file_size = 100; // should less than merchant
        let _ = Balances::make_free_balance_be(&source, 20000);
        let merchants = vec![merchant.clone(), charlie.clone(), dave.clone(), eve.clone(), ferdie.clone()];
        for who in merchants.iter() {
            let _ = Balances::make_free_balance_be(&who, 20000);
            assert_ok!(Market::register(Origin::signed(who.clone()), 6000));
        }

        assert_ok!(Market::place_storage_order(
            Origin::signed(source), cid.clone(),
            file_size, 0, false
        ));

        assert_eq!(Market::files(&cid).unwrap_or_default(), (
            FileInfo {
                file_size,
                expired_on: 50,
                claimed_at: 50,
                amount: 360, // ( 1000 + 1000 * 1 + 0 ) * 0.2
                expected_replica_count: 4,
                reported_replica_count: 0,
                replicas: vec![]
            },
            UsedInfo {
                used_size: file_size,
                groups: BTreeSet::from_iter(vec![].into_iter())
            })
        );

        run_to_block(303);

        let legal_wr_info = legal_work_report_with_added_files();
        let legal_pk = legal_wr_info.curr_pk.clone();

        add_who_into_replica(&cid, ferdie.clone(), legal_pk.clone(), Some(303u32));
        assert_eq!(Market::files_size(), file_size as u128);
        add_who_into_replica(&cid, charlie.clone(), legal_pk.clone(), Some(403u32));
        assert_eq!(Market::files_size(), (file_size * 2) as u128);
        add_who_into_replica(&cid, dave.clone(), legal_pk.clone(), Some(503u32));
        assert_eq!(Market::files_size(), (file_size * 3) as u128);

        register(&legal_pk, LegalCode::get());

        assert_ok!(Swork::report_works(
                Origin::signed(merchant.clone()),
                legal_wr_info.curr_pk,
                legal_wr_info.prev_pk,
                legal_wr_info.block_number,
                legal_wr_info.block_hash,
                legal_wr_info.free,
                legal_wr_info.used,
                legal_wr_info.added_files,
                legal_wr_info.deleted_files,
                legal_wr_info.srd_root,
                legal_wr_info.files_root,
                legal_wr_info.sig
            ));

        assert_eq!(Market::files_size(), (file_size * 4) as u128);

        add_who_into_replica(&cid, eve.clone(), legal_pk.clone(), Some(503u32));

        assert_eq!(Market::files(&cid).unwrap_or_default(), (
            FileInfo {
                file_size,
                expired_on: 1303,
                claimed_at: 303,
                amount: 360, // ( 1000 + 1000 * 1 + 0 ) * 0.2
                expected_replica_count: 4,
                reported_replica_count: 5,
                replicas: vec![
                    Replica {
                        who: ferdie.clone(),
                        valid_at: 303,
                        anchor: legal_pk.clone()
                    },
                    Replica {
                        who: merchant.clone(),
                        valid_at: 303,
                        anchor: legal_pk.clone()
                    },
                    Replica {
                        who: charlie.clone(),
                        valid_at: 403,
                        anchor: legal_pk.clone()
                    },
                    Replica {
                        who: dave.clone(),
                        valid_at: 503,
                        anchor: legal_pk.clone()
                    },
                    Replica {
                        who: eve.clone(),
                        valid_at: 503,
                        anchor: legal_pk.clone()
                    }
                ]
            },
            UsedInfo {
                used_size: file_size,
                groups: BTreeSet::from_iter(vec![legal_pk.clone()].into_iter())
            })
        );
        run_to_block(503);
        <swork::ReportedInSlot>::insert(legal_pk.clone(), 0, true);
        assert_ok!(Market::calculate_reward(Origin::signed(merchant.clone()), cid.clone()));
        assert_eq!(Market::files(&cid).unwrap_or_default(), (
            FileInfo {
                file_size,
                expired_on: 1303,
                claimed_at: 503,
                amount: 292,
                expected_replica_count: 4,
                reported_replica_count: 5,
                replicas: vec![
                    Replica {
                        who: ferdie.clone(),
                        valid_at: 303,
                        anchor: legal_pk.clone()
                    },
                    Replica {
                        who: merchant.clone(),
                        valid_at: 303,
                        anchor: legal_pk.clone()
                    },
                    Replica {
                        who: charlie.clone(),
                        valid_at: 403,
                        anchor: legal_pk.clone()
                    },
                    Replica {
                        who: dave.clone(),
                        valid_at: 503,
                        anchor: legal_pk.clone()
                    },
                    Replica {
                        who: eve.clone(),
                        valid_at: 503,
                        anchor: legal_pk.clone()
                    }
                ]
            },
            UsedInfo {
                used_size: file_size,
                groups: BTreeSet::from_iter(vec![legal_pk.clone()].into_iter())
            })
        );

        assert_eq!(Market::merchant_ledgers(&ferdie), MerchantLedger {
            pledge: 6000,
            reward: 17
        });
        assert_eq!(Market::merchant_ledgers(&merchant), MerchantLedger {
            pledge: 6000,
            reward: 17
        });
        assert_eq!(Market::merchant_ledgers(&charlie), MerchantLedger {
            pledge: 6000,
            reward: 17
        });
        assert_eq!(Market::merchant_ledgers(&dave), MerchantLedger {
            pledge: 6000,
            reward: 17
        });
        assert_eq!(Market::merchant_ledgers(&eve), MerchantLedger {
            pledge: 6000,
            reward: 0
        });
    });
}

/// Trash test case
#[test]
fn clear_trash_should_work() {
    new_test_ext().execute_with(|| {
        // generate 50 blocks first
        run_to_block(50);

        let source = ALICE;
        let merchant = MERCHANT;

        let cid1 =
            hex::decode("5bb706320afc633bfb843108e492192b17d2b6b9d9ee0b795ee95417fe08b660").unwrap();
        let cid2 =
            hex::decode("5bb706320afc633bfb843108e492192b17d2b6b9d9ee0b795ee95417fe08b661").unwrap();
        let cid3 =
            hex::decode("5bb706320afc633bfb843108e492192b17d2b6b9d9ee0b795ee95417fe08b662").unwrap();
        let cid4 =
            hex::decode("5bb706320afc633bfb843108e492192b17d2b6b9d9ee0b795ee95417fe08b663").unwrap();
        let cid5 =
            hex::decode("5bb706320afc633bfb843108e492192b17d2b6b9d9ee0b795ee95417fe08b664").unwrap();
        let cid6 =
            hex::decode("5bb706320afc633bfb843108e492192b17d2b6b9d9ee0b795ee95417fe08b665").unwrap();
        let file_lists = vec![cid1.clone(), cid2.clone(), cid3.clone(), cid4.clone(), cid5.clone(), cid6.clone()];
        let file_size = 100; // should less than merchant
        let _ = Balances::make_free_balance_be(&source, 20000);
        let _ = Balances::make_free_balance_be(&merchant, 20000);

        assert_ok!(Market::register(Origin::signed(merchant.clone()), 6000));

        for cid in file_lists.clone().iter() {
            assert_ok!(Market::place_storage_order(
                Origin::signed(source.clone()), cid.clone(),
                file_size, 0, false
            ));
            assert_eq!(Market::files(&cid).unwrap_or_default(), (
            FileInfo {
                file_size,
                expired_on: 50,
                claimed_at: 50,
                amount: 360, // ( 1000 + 1000 * 1 + 0 ) * 0.2
                expected_replica_count: 4,
                reported_replica_count: 0,
                replicas: vec![]
            },
            UsedInfo {
                used_size: file_size,
                groups: BTreeSet::from_iter(vec![].into_iter())
            })
            );
        }

        run_to_block(303);
        let legal_wr_info = legal_work_report_with_added_files();
        let legal_pk = legal_wr_info.curr_pk.clone();
        register(&legal_pk, LegalCode::get());
        for cid in file_lists.clone().iter() {
            add_who_into_replica(&cid, merchant.clone(), legal_pk.clone(), None);
        }

        for cid in file_lists.clone().iter() {
            assert_eq!(Market::files(&cid).unwrap_or_default(), (
                FileInfo {
                    file_size,
                    expired_on: 1303,
                    claimed_at: 303,
                    amount: 360, // ( 1000 + 1000 * 1 + 0 ) * 0.2
                    expected_replica_count: 4,
                    reported_replica_count: 1,
                    replicas: vec![Replica {
                        who: merchant.clone(),
                        valid_at: 303,
                        anchor: legal_pk.clone()
                    }]
                },
                UsedInfo {
                    used_size: file_size,
                    groups: BTreeSet::from_iter(vec![legal_pk.clone()].into_iter())
                })
            );
        }

        run_to_block(1500);
        <swork::ReportedInSlot>::insert(legal_pk.clone(), 900, true);
        // close files one by one
        assert_ok!(Market::calculate_reward(Origin::signed(merchant.clone()), cid1.clone()));
        assert_eq!(Market::used_trash_i(&cid1).unwrap_or_default(), UsedInfo {
            used_size: file_size,
            groups: BTreeSet::from_iter(vec![legal_pk.clone()].into_iter())
        });

        assert_ok!(Market::calculate_reward(Origin::signed(merchant.clone()), cid2.clone()));
        assert_eq!(Market::used_trash_i(&cid2).unwrap_or_default(), UsedInfo {
            used_size: file_size,
            groups: BTreeSet::from_iter(vec![legal_pk.clone()].into_iter())
        });

        assert_ok!(Market::calculate_reward(Origin::signed(merchant.clone()), cid3.clone()));
        assert_eq!(Market::used_trash_ii(&cid3).unwrap_or_default(), UsedInfo {
            used_size: file_size,
            groups: BTreeSet::from_iter(vec![legal_pk.clone()].into_iter())
        });

        assert_ok!(Market::calculate_reward(Origin::signed(merchant.clone()), cid4.clone()));
        assert_eq!(Market::used_trash_ii(&cid4).unwrap_or_default(), UsedInfo {
            used_size: file_size,
            groups: BTreeSet::from_iter(vec![legal_pk.clone()].into_iter())
        });
        assert_eq!(Market::used_trash_i(&cid1), None);
        assert_eq!(Market::used_trash_i(&cid2), None);

        assert_ok!(Market::calculate_reward(Origin::signed(merchant.clone()), cid5.clone()));
        assert_eq!(Market::used_trash_i(&cid5).unwrap_or_default(), UsedInfo {
            used_size: file_size,
            groups: BTreeSet::from_iter(vec![legal_pk.clone()].into_iter())
        });

        assert_ok!(Market::calculate_reward(Origin::signed(merchant.clone()), cid6.clone()));
        assert_eq!(Market::used_trash_i(&cid6).unwrap_or_default(), UsedInfo {
            used_size: file_size,
            groups: BTreeSet::from_iter(vec![legal_pk.clone()].into_iter())
        });

        assert_eq!(Market::used_trash_ii(&cid3), None);
        assert_eq!(Market::used_trash_ii(&cid4), None);
    });
}

/// Update file price test case
#[test]
fn update_price_should_work() {
    new_test_ext().execute_with(|| {
        // generate 50 blocks first
        // 0 / 0 => None => decrease
        Market::update_file_price();
        assert_eq!(Market::file_price(), 990);

        run_to_block(50);
        // first class storage is 0
        <swork::Free>::put(10000);
        <swork::Used>::put(10000);
        assert_eq!(Swork::get_total_capacity(), 20000);
        Market::update_file_price();
        assert_eq!(Market::file_price(), 980);

        // first class storage is 11000 => increase 1%
        FilesSize::put(11000);
        Market::update_file_price();
        assert_eq!(Market::file_price(), 990);

        // price is 40 and cannot decrease
        <FilePrice<Test>>::put(40);
        FilesSize::put(10);
        Market::update_file_price();
        assert_eq!(Market::file_price(), 40);

        // price is 40 and will increase by 1
        FilesSize::put(20000);
        Market::update_file_price();
        assert_eq!(Market::file_price(), 41);
    });
}

/// Withdraw staking pot should work
#[test]
fn withdraw_staking_pot_should_work() {
    new_test_ext().execute_with(|| {
        // generate 50 blocks first
        run_to_block(50);

        let source = ALICE;
        let merchant = MERCHANT;

        let cid =
            hex::decode("4e2883ddcbc77cf19979770d756fd332d0c8f815f9de646636169e460e6af6ff").unwrap();
        let file_size = 100; // should less than
        let reserved_pot = Market::reserved_pot();
        let staking_pot = Market::staking_pot();
        let storage_pot = Market::storage_pot();
        assert_eq!(Balances::free_balance(&staking_pot), 0);
        let _ = Balances::make_free_balance_be(&source, 20000);
        let _ = Balances::make_free_balance_be(&merchant, 200);

        assert_ok!(Market::register(Origin::signed(merchant.clone()), 60));

        assert_ok!(Market::place_storage_order(
            Origin::signed(source), cid.clone(),
            file_size, 0, false
        ));

        assert_eq!(Market::files(&cid).unwrap_or_default(), (
            FileInfo {
                file_size,
                expired_on: 50,
                claimed_at: 50,
                amount: 360, // ( 1000 + 1000 * 1 + 0 ) * 0.2
                expected_replica_count: 4,
                reported_replica_count: 0,
                replicas: vec![]
            },
            UsedInfo {
                used_size: file_size,
                groups: BTreeSet::from_iter(vec![].into_iter())
            })
        );
        assert_eq!(Balances::free_balance(&reserved_pot), 200);
        assert_eq!(Balances::free_balance(&staking_pot), 1440);
        assert_eq!(Balances::free_balance(&storage_pot), 360);

        assert_eq!(Market::withdraw_staking_pot(), 1439);
        assert_eq!(Balances::free_balance(&staking_pot), 1);
    });
}

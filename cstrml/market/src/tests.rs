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
use swork::Identity;

/// Register & Collateral test cases
#[test]
fn register_should_work() {
    new_test_ext().execute_with(|| {
        // generate 50 blocks first
        run_to_block(50);
        let merchant = MERCHANT;
        let collateral_pot = Market::collateral_pot();
        let _ = Balances::make_free_balance_be(&merchant, 200);
        assert_ok!(Market::register(Origin::signed(merchant.clone()), 180));
        assert_eq!(Market::merchant_ledgers(&merchant), MerchantLedger {
            collateral: 180,
            reward: 0
        });
        assert_eq!(Balances::free_balance(&collateral_pot), 180);
        assert_ok!(Market::cut_collateral(Origin::signed(merchant.clone()), 20));
        assert_eq!(Market::merchant_ledgers(&merchant), MerchantLedger {
            collateral: 160,
            reward: 0
        });
        assert_eq!(Balances::free_balance(&collateral_pot), 160);
        assert_ok!(Market::add_collateral(Origin::signed(merchant.clone()), 10));
        assert_eq!(Market::merchant_ledgers(merchant), MerchantLedger {
            collateral: 170,
            reward: 0
        });
        assert_eq!(Balances::free_balance(&collateral_pot), 170);
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
                index: 3,
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
                index: 3,
                error: 4,
                message: Some("AlreadyRegistered")
            }
        );
    });
}

#[test]
fn collateral_extra_should_fail_due_to_merchant_not_register() {
    new_test_ext().execute_with(|| {
        // generate 50 blocks first
        run_to_block(50);

        let merchant = MERCHANT;
        let _ = Balances::make_free_balance_be(&merchant, 200);
        assert_noop!(
            Market::add_collateral(
                Origin::signed(merchant),
                200
            ),
            DispatchError::Module {
                index: 3,
                error: 3,
                message: Some("NotRegister")
            }
        );
    });
}

#[test]
fn cut_collateral_should_fail_due_to_reward() {
    new_test_ext().execute_with(|| {
        // generate 50 blocks first
        run_to_block(50);

        let merchant = MERCHANT;
        let _ = Balances::make_free_balance_be(&merchant, 200);
        assert_ok!(Market::register(Origin::signed(merchant.clone()), 180));
        assert_eq!(Market::merchant_ledgers(&merchant), MerchantLedger {
            collateral: 180,
            reward: 0
        });

        <self::MerchantLedgers<Test>>::insert(&merchant, MerchantLedger {
            collateral: 180,
            reward: 120
        });
        assert_ok!(Market::cut_collateral(Origin::signed(merchant.clone()), 20));
        assert_eq!(Market::merchant_ledgers(&merchant), MerchantLedger {
            collateral: 160,
            reward: 120
        });
        assert_noop!(
            Market::cut_collateral(
                Origin::signed(merchant),
                50
            ),
            DispatchError::Module {
                index: 3,
                error: 1,
                message: Some("InsufficientCollateral")
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
            file_size, 0
        ));

        assert_eq!(Market::files(&cid).unwrap_or_default(), (
            FileInfo {
                file_size,
                expired_on: 0,
                calculated_at: 50,
                amount: 360, // ( 1000 + 1000 * 1 + 0 ) * 0.2
                prepaid: 0,
                reported_replica_count: 0,
                replicas: vec![]
            },
            UsedInfo {
                used_size: 0,
                reported_group_count: 0,
                groups: BTreeMap::new()
            })
        );
        assert_eq!(Balances::free_balance(reserved_pot), 200);
        assert_eq!(Balances::free_balance(staking_pot), 1440);
        assert_eq!(Balances::free_balance(storage_pot), 360);
    });
}

#[test]
fn place_storage_order_should_fail_due_to_too_large_file_size() {
    new_test_ext().execute_with(|| {
        // generate 50 blocks first
        run_to_block(50);

        let source = ALICE;
        let merchant = MERCHANT;

        let cid =
            hex::decode("4e2883ddcbc77cf19979770d756fd332d0c8f815f9de646636169e460e6af6ff").unwrap();
        let file_size = 437_438_953_472;
        let staking_pot = Market::staking_pot();
        assert_eq!(Balances::free_balance(&staking_pot), 0);
        let _ = Balances::make_free_balance_be(&source, 20000);
        let _ = Balances::make_free_balance_be(&merchant, 200);

        assert_ok!(Market::register(Origin::signed(merchant.clone()), 60));

        assert_noop!(Market::place_storage_order(
            Origin::signed(source), cid.clone(),
            file_size, 0
        ),
        DispatchError::Module {
            index: 3,
            error: 11,
            message: Some("FileTooLarge")
        });
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
            "QmdwgqZy1MZBfWPi7GcxVsYgJEtmvHg6rsLzbCej3tf3oF".as_bytes().to_vec();
        let file_size = 134289408; // 134289408 / 1_048_576 = 129
        let staking_pot = Market::staking_pot();
        let storage_pot = Market::storage_pot();
        assert_eq!(Balances::free_balance(&staking_pot), 0);
        let _ = Balances::make_free_balance_be(&source, 20_000_000);
        let _ = Balances::make_free_balance_be(&merchant, 20_000_000);

        assert_ok!(Market::register(Origin::signed(merchant.clone()), 6_000_000));

        // 1. New storage order
        assert_ok!(Market::place_storage_order(
            Origin::signed(source.clone()), cid.clone(),
            file_size, 0
        ));

        assert_eq!(Market::files(&cid).unwrap_or_default(), (
            FileInfo {
                file_size,
                expired_on: 0,
                calculated_at: 50,
                amount: 23400, // ( 1000 + 1000 * 129 + 0 ) * 0.18
                prepaid: 0,
                reported_replica_count: 0,
                replicas: vec![]
            },
            UsedInfo {
                used_size: 0,
                reported_group_count: 0,
                groups: BTreeMap::new()
            })
        );
        assert_eq!(Balances::free_balance(&staking_pot), 93600);
        assert_eq!(Balances::free_balance(&storage_pot), 23400);

        run_to_block(250);
        
        // 2. Add amount for sOrder not begin should work
        assert_ok!(Market::place_storage_order(
            Origin::signed(source.clone()), cid.clone(),
            file_size, 0
        ));

        assert_eq!(Market::files(&cid).unwrap_or_default(), (
            FileInfo {
                file_size,
                expired_on: 0,
                calculated_at: 50,
                amount: 46800, // ( 1000 + 1000 * 129 + 0 ) * 0.18 * 2
                prepaid: 0,
                reported_replica_count: 0,
                replicas: vec![]
            },
            UsedInfo {
                used_size: 0,
                reported_group_count: 0,
                groups: BTreeMap::new()
            })
        );
        assert_eq!(Balances::free_balance(&staking_pot), 187200);
        assert_eq!(Balances::free_balance(&storage_pot), 46800);

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
                calculated_at: 400,
                amount: 46800, // ( 1000 + 1000 * 129 + 0 ) * 0.18 * 2
                prepaid: 0,
                reported_replica_count: 1,
                replicas: vec![Replica {
                    who: merchant.clone(),
                    valid_at: 303,
                    anchor: legal_pk.clone(),
                    is_reported: true
                }]
            },
            UsedInfo {
                used_size: file_size * 2,
                reported_group_count: 1,
                groups: BTreeMap::from_iter(vec![(legal_pk.clone(), true)].into_iter())
            })
        );

        // Calculate reward should work
        run_to_block(500);
        <swork::ReportedInSlot>::insert(legal_pk.clone(), 0, true);
        Market::do_calculate_reward(&cid, System::block_number().try_into().unwrap());
        assert_eq!(Market::files(&cid).unwrap_or_default(), (
            FileInfo {
                file_size,
                expired_on: 1400,
                calculated_at: 500,
                amount: 42121,
                prepaid: 0,
                reported_replica_count: 1,
                replicas: vec![Replica {
                    who: merchant.clone(),
                    valid_at: 303,
                    anchor: legal_pk.clone(),
                    is_reported: true
                }]
            },
            UsedInfo {
                used_size: file_size * 2,
                reported_group_count: 1,
                groups: BTreeMap::from_iter(vec![(legal_pk.clone(), true)].into_iter())
            })
        );

        // 3. Extend duration should work
        run_to_block(600);
        assert_ok!(Market::place_storage_order(
            Origin::signed(source.clone()), cid.clone(),
            file_size, 0
        ));

        assert_eq!(Market::files(&cid).unwrap_or_default(), (
            FileInfo {
                file_size,
                expired_on: 1600,
                calculated_at: 600,
                amount: 60842,
                prepaid: 0,
                reported_replica_count: 1,
                replicas: vec![Replica {
                    who: merchant.clone(),
                    valid_at: 303,
                    anchor: legal_pk.clone(),
                    is_reported: true
                }]
            },
            UsedInfo {
                used_size: file_size * 2,
                reported_group_count: 1,
                groups: BTreeMap::from_iter(vec![(legal_pk.clone(), true)].into_iter())
            })
        );

        // 4. Extend replicas should work
        run_to_block(800);
        assert_ok!(Market::place_storage_order(
            Origin::signed(source.clone()), cid.clone(),
            file_size, 200
        ));

        assert_eq!(Market::files(&cid).unwrap_or_default(), (
            FileInfo {
                file_size,
                expired_on: 1800,
                calculated_at: 800,
                amount: 72111,
                prepaid: 0,
                reported_replica_count: 1,
                replicas: vec![Replica {
                    who: merchant.clone(),
                    valid_at: 303,
                    anchor: legal_pk.clone(),
                    is_reported: true
                }]
            },
            UsedInfo {
                used_size: file_size * 2,
                reported_group_count: 1,
                groups: BTreeMap::from_iter(vec![(legal_pk.clone(), true)].into_iter())
            })
        );
    });
}


/// Payouts test cases
#[test]
// Payout should be triggered by:
// 1. Delete file(covered by swork module)
// 2. Calculate reward
// 3. Place started storage order(covered by `place_storage_order_should_work_for_extend_scenarios`)
fn do_calculate_reward_should_work() {
    new_test_ext().execute_with(|| {
        // generate 50 blocks first
        run_to_block(50);
 
        let source = ALICE;
        let merchant = MERCHANT;

        let cid =
            "QmdwgqZy1MZBfWPi7GcxVsYgJEtmvHg6rsLzbCej3tf3oF".as_bytes().to_vec();
        let file_size = 134289408; // 134289408 / 1_048_576 = 129
        let staking_pot = Market::staking_pot();
        let storage_pot = Market::storage_pot();
        let reserved_pot = Market::reserved_pot();
        assert_eq!(Balances::free_balance(&staking_pot), 0);
        let _ = Balances::make_free_balance_be(&source, 20_000_000);
        let _ = Balances::make_free_balance_be(&merchant, 20_000_000);

        assert_ok!(Market::register(Origin::signed(merchant.clone()), 6_000_000));

        // 1. New storage order
        assert_ok!(Market::place_storage_order(
            Origin::signed(source.clone()), cid.clone(),
            file_size, 0
        ));

        assert_eq!(Market::files(&cid).unwrap_or_default(), (
            FileInfo {
                file_size,
                expired_on: 0,
                calculated_at: 50,
                amount: 23400, // ( 1000 + 1000 * 129 + 0 ) * 0.18
                prepaid: 0,
                reported_replica_count: 0,
                replicas: vec![]
            },
            UsedInfo {
                used_size: 0,
                reported_group_count: 0,
                groups: BTreeMap::new()
            })
        );
        assert_eq!(Balances::free_balance(&staking_pot), 93600);
        assert_eq!(Balances::free_balance(&storage_pot), 23400);
        assert_eq!(Balances::free_balance(&reserved_pot), 13000);

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
                calculated_at: 303,
                amount: 23400, // ( 1000 + 1000 * 129 + 0 ) * 0.18
                prepaid: 0,
                reported_replica_count: 1,
                replicas: vec![Replica {
                    who: merchant.clone(),
                    valid_at: 303,
                    anchor: legal_pk.clone(),
                    is_reported: true
                }]
            },
            UsedInfo {
                used_size: file_size * 2,
                reported_group_count: 1,
                groups: BTreeMap::from_iter(vec![(legal_pk.clone(), true)].into_iter())
            })
        );

        // 3. Go along with some time, and get reward
        run_to_block(606);
        <swork::ReportedInSlot>::insert(legal_pk.clone(), 300, true);
        Market::do_calculate_reward(&cid, System::block_number().try_into().unwrap());
        assert_eq!(Market::files(&cid).unwrap_or_default(), (
            FileInfo {
                file_size,
                expired_on: 1303,
                calculated_at: 606,
                amount: 16311,
                prepaid: 0,
                reported_replica_count: 1,
                replicas: vec![Replica {
                    who: merchant.clone(),
                    valid_at: 303,
                    anchor: legal_pk.clone(),
                    is_reported: true
                }]
            },
            UsedInfo {
                used_size: file_size * 2,
                reported_group_count: 1,
                groups: BTreeMap::from_iter(vec![(legal_pk.clone(), true)].into_iter())
            })
        );
        assert_eq!(Market::merchant_ledgers(&merchant), MerchantLedger {
            collateral: 6_000_000,
            reward: 7089
        })
    });
}

#[test]
fn do_calculate_reward_should_fail_due_to_insufficient_collateral() {
    new_test_ext().execute_with(|| {
        // generate 50 blocks first
        run_to_block(50);

        let source = ALICE;
        let merchant = MERCHANT;

        let cid =
            "QmdwgqZy1MZBfWPi7GcxVsYgJEtmvHg6rsLzbCej3tf3oF".as_bytes().to_vec();
        let file_size = 134289408; // should less than merchant
        let _ = Balances::make_free_balance_be(&source, 20_000_000);
        let _ = Balances::make_free_balance_be(&merchant, 20_000_000);

        assert_ok!(Market::register(Origin::signed(merchant.clone()), 70_000));

        assert_ok!(Market::place_storage_order(
            Origin::signed(source), cid.clone(),
            file_size, 0
        ));

        assert_eq!(Market::files(&cid).unwrap_or_default(), (
            FileInfo {
                file_size,
                expired_on: 0,
                calculated_at: 50,
                amount: 23400, // ( 1000 + 1000 * 1 + 0 ) * 0.2
                prepaid: 0,
                reported_replica_count: 0,
                replicas: vec![]
            },
            UsedInfo {
                used_size: 0,
                reported_group_count: 0,
                groups: BTreeMap::new()
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
                calculated_at: 303,
                amount: 23400, // ( 1000 + 1000 * 1 + 0 ) * 0.2
                prepaid: 0,
                reported_replica_count: 1,
                replicas: vec![Replica {
                    who: merchant.clone(),
                    valid_at: 303,
                    anchor: legal_pk.clone(),
                    is_reported: true
                }]
            },
            UsedInfo {
                used_size: file_size * 2,
                reported_group_count: 1,
                groups: BTreeMap::from_iter(vec![(legal_pk.clone(), true)].into_iter())
            })
        );

        run_to_block(603);
        <swork::ReportedInSlot>::insert(legal_pk.clone(), 300, true);
        Market::do_calculate_reward(&cid, System::block_number().try_into().unwrap());
        assert_eq!(Market::files(&cid).unwrap_or_default(), (
            FileInfo {
                file_size,
                expired_on: 1303,
                calculated_at: 603,
                amount: 23400, // ( 1000 + 1000 * 1 + 0 ) * 0.2
                prepaid: 0,
                reported_replica_count: 1,
                replicas: vec![Replica {
                    who: merchant.clone(),
                    valid_at: 303,
                    anchor: legal_pk.clone(),
                    is_reported: true
                }]
            },
            UsedInfo {
                used_size: file_size * 2,
                reported_group_count: 1,
                groups: BTreeMap::from_iter(vec![(legal_pk.clone(), true)].into_iter())
            })
        );

        // collateral is 7020 * 10 < 70000 reward
        assert_eq!(Market::merchant_ledgers(&merchant), MerchantLedger {
            collateral: 70_000,
            reward: 0
        });

        run_to_block(903);
        <swork::ReportedInSlot>::insert(legal_pk.clone(), 600, true);
        assert_ok!(Market::add_collateral(Origin::signed(merchant.clone()), 6_000_000));
        Market::do_calculate_reward(&cid, System::block_number().try_into().unwrap());
        assert_eq!(Market::files(&cid).unwrap_or_default(), (
            FileInfo {
                file_size,
                expired_on: 1303,
                calculated_at: 903,
                amount: 13372, // ( 1000 + 1000 * 1 + 0 ) * 0.2
                prepaid: 0,
                reported_replica_count: 1,
                replicas: vec![Replica {
                    who: merchant.clone(),
                    valid_at: 303,
                    anchor: legal_pk.clone(),
                    is_reported: true
                }]
            },
            UsedInfo {
                used_size: file_size * 2,
                reported_group_count: 1,
                groups: BTreeMap::from_iter(vec![(legal_pk.clone(), true)].into_iter())
            })
        );

        assert_eq!(Market::merchant_ledgers(&merchant), MerchantLedger {
            collateral: 6_070_000,
            reward: 10028
        });
    });
}

#[test]
fn do_calculate_reward_should_move_file_to_trash_due_to_expired() {
    new_test_ext().execute_with(|| {
        // generate 50 blocks first
        run_to_block(50);

        let source = ALICE;
        let merchant = MERCHANT;

        let cid =
            "QmdwgqZy1MZBfWPi7GcxVsYgJEtmvHg6rsLzbCej3tf3oF".as_bytes().to_vec();
        let file_size = 134289408; // should less than merchant
        let _ = Balances::make_free_balance_be(&source, 20_000_000);
        let _ = Balances::make_free_balance_be(&merchant, 20_000_000);

        assert_ok!(Market::register(Origin::signed(merchant.clone()), 6_000_000));

        assert_ok!(Market::place_storage_order(
            Origin::signed(source), cid.clone(),
            file_size, 0
        ));

        assert_eq!(Market::files(&cid).unwrap_or_default(), (
            FileInfo {
                file_size,
                expired_on: 0,
                calculated_at: 50,
                amount: 23400,
                prepaid: 0,
                reported_replica_count: 0,
                replicas: vec![]
            },
            UsedInfo {
                used_size: 0,
                reported_group_count: 0,
                groups: BTreeMap::new()
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
                calculated_at: 303,
                amount: 23400,
                prepaid: 0,
                reported_replica_count: 1,
                replicas: vec![Replica {
                    who: merchant.clone(),
                    valid_at: 303,
                    anchor: legal_pk.clone(),
                    is_reported: true
                }]
            },
            UsedInfo {
                used_size: file_size * 2,
                reported_group_count: 1,
                groups: BTreeMap::from_iter(vec![(legal_pk.clone(), true)].into_iter())
            })
        );

        run_to_block(1506);
        <swork::ReportedInSlot>::insert(legal_pk.clone(), 1200, true);
        assert_ok!(Market::calculate_reward(Origin::signed(merchant.clone()), cid.clone()));
        assert_eq!(Market::files(&cid), None);

        assert_eq!(Market::used_trash_i(&cid).unwrap_or_default(), UsedInfo {
            used_size: file_size * 2,
            reported_group_count: 1,
            groups: BTreeMap::from_iter(vec![(legal_pk.clone(), true)].into_iter())
        });

        assert_eq!(Market::merchant_ledgers(&merchant), MerchantLedger {
            collateral: 6_000_000,
            reward: 23399
        })
    });
}

#[test]
fn do_calculate_reward_should_work_in_complex_timeline() {
    new_test_ext().execute_with(|| {
        // generate 50 blocks first
        run_to_block(50);

        let cid =
            "QmdwgqZy1MZBfWPi7GcxVsYgJEtmvHg6rsLzbCej3tf3oF".as_bytes().to_vec();
        let file_size = 134289408; // should less than merchant
        let source = ALICE;
        let merchant = BOB;
        let charlie = CHARLIE;
        let dave = DAVE;
        let eve = EVE;

        let staking_pot = Market::staking_pot();
        let reserved_pot = Market::reserved_pot();
        assert_eq!(Balances::free_balance(&staking_pot), 0);
        assert_eq!(Balances::free_balance(&reserved_pot), 0);
        let _ = Balances::make_free_balance_be(&source, 20_000_000);
        let merchants = vec![merchant.clone(), charlie.clone(), dave.clone(), eve.clone()];
        for who in merchants.iter() {
            let _ = Balances::make_free_balance_be(&who, 20_000_000);
            assert_ok!(Market::register(Origin::signed(who.clone()), 6_000_000));
        }

        assert_ok!(Market::place_storage_order(
            Origin::signed(source), cid.clone(),
            file_size, 0
        ));

        assert_eq!(Market::files(&cid).unwrap_or_default(), (
            FileInfo {
                file_size,
                expired_on: 0,
                calculated_at: 50,
                amount: 23400,
                prepaid: 0,
                reported_replica_count: 0,
                replicas: vec![]
            },
            UsedInfo {
                used_size: 0,
                reported_group_count: 0,
                groups: BTreeMap::new()
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

        assert_eq!(Market::files_size(), file_size as u128);

        assert_eq!(Market::files(&cid).unwrap_or_default(), (
            FileInfo {
                file_size,
                expired_on: 1303,
                calculated_at: 303,
                amount: 23400,
                prepaid: 0,
                reported_replica_count: 1,
                replicas: vec![Replica {
                    who: merchant.clone(),
                    valid_at: 303,
                    anchor: legal_pk.clone(),
                    is_reported: true
                }]
            },
            UsedInfo {
                used_size: file_size * 2,
                reported_group_count: 1,
                groups: BTreeMap::from_iter(vec![(legal_pk.clone(), true)].into_iter())
            })
        );

        run_to_block(503);
        <swork::ReportedInSlot>::insert(legal_pk.clone(), 0, true);
        Market::do_calculate_reward(&cid, System::block_number().try_into().unwrap());
        assert_eq!(Market::files(&cid).unwrap_or_default(), (
            FileInfo {
                file_size,
                expired_on: 1303,
                calculated_at: 503,
                amount: 18721,
                prepaid: 0,
                reported_replica_count: 1,
                replicas: vec![Replica {
                    who: merchant.clone(),
                    valid_at: 303,
                    anchor: legal_pk.clone(),
                    is_reported: true
                }]
            },
            UsedInfo {
                used_size: file_size * 2,
                reported_group_count: 1,
                groups: BTreeMap::from_iter(vec![(legal_pk.clone(), true)].into_iter())
            })
        );
        assert_eq!(Market::merchant_ledgers(&merchant), MerchantLedger {
            collateral: 6_000_000,
            reward: 4679
        });

        add_who_into_replica(&cid, file_size, charlie.clone(), legal_pk.clone(), None, None);

        run_to_block(603);
        <swork::ReportedInSlot>::insert(legal_pk.clone(), 300, true);
        Market::do_calculate_reward(&cid, System::block_number().try_into().unwrap());
        assert_eq!(Market::files(&cid).unwrap_or_default(), (
            FileInfo {
                file_size,
                expired_on: 1303,
                calculated_at: 603,
                amount: 16383,
                prepaid: 0,
                reported_replica_count: 2,
                replicas: vec![
                    Replica {
                        who: merchant.clone(),
                        valid_at: 303,
                        anchor: legal_pk.clone(),
                        is_reported: true
                    },
                    Replica {
                        who: charlie.clone(),
                        valid_at: 503,
                        anchor: legal_pk.clone(),
                        is_reported: true
                    }]
            },
            UsedInfo {
                used_size: file_size * 2,
                reported_group_count: 1,
                groups: BTreeMap::from_iter(vec![(legal_pk.clone(), true)].into_iter())
            })
        );
        assert_eq!(Market::merchant_ledgers(&merchant), MerchantLedger {
            collateral: 6_000_000,
            reward: 5848
        });
        assert_eq!(Market::merchant_ledgers(&charlie), MerchantLedger {
            collateral: 6_000_000,
            reward: 1169
        });

        assert_eq!(Market::files_size(), file_size as u128);

        add_who_into_replica(&cid, file_size, dave.clone(), hex::decode("11").unwrap(), None, None);
        run_to_block(703);
        Market::do_calculate_reward(&cid, System::block_number().try_into().unwrap());
        assert_eq!(Market::files(&cid).unwrap_or_default(), (
            FileInfo {
                file_size,
                expired_on: 1303,
                calculated_at: 703,
                amount: 14825,
                prepaid: 0,
                reported_replica_count: 2,
                replicas: vec![
                    Replica {
                        who: merchant.clone(),
                        valid_at: 303,
                        anchor: legal_pk.clone(),
                        is_reported: true
                    },
                    Replica {
                        who: charlie.clone(),
                        valid_at: 503,
                        anchor: legal_pk.clone(),
                        is_reported: true
                    },
                    Replica {
                        who: dave.clone(),
                        valid_at: 703, // did't report. change it to curr bn
                        anchor: hex::decode("11").unwrap(),
                        is_reported: false
                    }]
            },
            UsedInfo {
                used_size: file_size * 2,
                reported_group_count: 1,
                groups: BTreeMap::from_iter(vec![(legal_pk.clone(), true), (hex::decode("11").unwrap(), false)].into_iter())
            })
        );
        assert_eq!(Market::merchant_ledgers(&merchant), MerchantLedger {
            collateral: 6_000_000,
            reward: 6627
        });
        assert_eq!(Market::merchant_ledgers(&charlie), MerchantLedger {
            collateral: 6_000_000,
            reward: 1948
        });

        assert_eq!(Market::files_size(), file_size as u128);

        run_to_block(903);
        <swork::ReportedInSlot>::insert(hex::decode("11").unwrap(), 600, true);
        Market::do_calculate_reward(&cid, System::block_number().try_into().unwrap());
        assert_eq!(Market::files(&cid).unwrap_or_default(), (
            FileInfo {
                file_size,
                expired_on: 1303,
                calculated_at: 903,
                amount: 13179,
                prepaid: 0,
                reported_replica_count: 1,
                replicas: vec![
                    Replica {
                        who: dave.clone(),
                        valid_at: 703,
                        anchor: hex::decode("11").unwrap(),
                        is_reported: true
                    },
                    Replica {
                        who: merchant.clone(),
                        valid_at: 903, // did't report. change it to curr bn
                        anchor: legal_pk.clone(),
                        is_reported: false
                    },
                    Replica {
                        who: charlie.clone(),
                        valid_at: 903, // did't report. change it to curr bn
                        anchor: legal_pk.clone(),
                        is_reported: false
                    }]
            },
            UsedInfo {
                used_size: file_size * 2,
                reported_group_count: 1,
                groups: BTreeMap::from_iter(vec![(legal_pk.clone(), false), (hex::decode("11").unwrap(), true)].into_iter())
            })
        );
        assert_eq!(Market::merchant_ledgers(&merchant), MerchantLedger {
            collateral: 6_000_000,
            reward: 6627
        });
        assert_eq!(Market::merchant_ledgers(&charlie), MerchantLedger {
            collateral: 6_000_000,
            reward: 1948
        });
        assert_eq!(Market::merchant_ledgers(&dave), MerchantLedger {
            collateral: 6_000_000,
            reward: 1646
        });

        assert_eq!(Market::files_size(), file_size as u128);

        run_to_block(1203);
        <swork::ReportedInSlot>::insert(hex::decode("11").unwrap(), 900, true);
        <swork::ReportedInSlot>::insert(legal_pk.clone(), 900, true);
        Market::do_calculate_reward(&cid, System::block_number().try_into().unwrap());
        assert_eq!(Market::files(&cid).unwrap_or_default(), (
            FileInfo {
                file_size,
                expired_on: 1303,
                calculated_at: 1203,
                amount: 3297,
                prepaid: 0,
                reported_replica_count: 3,
                replicas: vec![
                    Replica {
                        who: dave.clone(),
                        valid_at: 703,
                        anchor: hex::decode("11").unwrap(),
                        is_reported: true
                    },
                    Replica {
                        who: merchant.clone(),
                        valid_at: 903,
                        anchor: legal_pk.clone(),
                        is_reported: true
                    },
                    Replica {
                        who: charlie.clone(),
                        valid_at: 903,
                        anchor: legal_pk.clone(),
                        is_reported: true
                    }]
            },
            UsedInfo {
                used_size: file_size * 2,
                reported_group_count: 2,
                groups: BTreeMap::from_iter(vec![(legal_pk.clone(), true), (hex::decode("11").unwrap(), true)].into_iter())
            })
        );
        assert_eq!(Market::merchant_ledgers(&merchant), MerchantLedger {
            collateral: 6_000_000,
            reward: 9921
        });
        assert_eq!(Market::merchant_ledgers(&charlie), MerchantLedger {
            collateral: 6_000_000,
            reward: 5242
        });
        assert_eq!(Market::merchant_ledgers(&dave), MerchantLedger {
            collateral: 6_000_000,
            reward: 4940
        });
        assert_eq!(Market::files_size(), (file_size * 2) as u128);

        run_to_block(1803);
        assert_ok!(Market::calculate_reward(Origin::signed(merchant.clone()), cid.clone()));
        assert_eq!(Market::used_trash_i(&cid).unwrap_or_default(), UsedInfo {
            used_size: file_size * 2,
            reported_group_count: 1,
            groups: BTreeMap::from_iter(vec![(legal_pk.clone(), false), (hex::decode("11").unwrap(), false)].into_iter())
        });

        assert_eq!(Market::merchant_ledgers(&merchant), MerchantLedger {
            collateral: 6_000_000,
            reward: 9921
        });
        assert_eq!(Market::merchant_ledgers(&charlie), MerchantLedger {
            collateral: 6_000_000,
            reward: 5242
        });
        assert_eq!(Market::merchant_ledgers(&dave), MerchantLedger {
            collateral: 6_000_000,
            reward: 4940
        });
        assert_eq!(Market::files_size(), 0);
        assert_eq!(Balances::free_balance(&reserved_pot), 16297);
    });
}

#[test]
fn do_calculate_reward_should_fail_due_to_not_live() {
    new_test_ext().execute_with(|| {
        // generate 50 blocks first
        run_to_block(50);

        let source = ALICE;
        let merchant = MERCHANT;

        let cid =
            "QmdwgqZy1MZBfWPi7GcxVsYgJEtmvHg6rsLzbCej3tf3oF".as_bytes().to_vec();
        let file_size = 100; // should less than merchant

        let _ = Balances::make_free_balance_be(&source, 20000);
        let _ = Balances::make_free_balance_be(&merchant, 20000);

        // collateral is 60 < 121 reward
        assert_ok!(Market::register(Origin::signed(merchant.clone()), 6000));

        assert_ok!(Market::place_storage_order(
            Origin::signed(source), cid.clone(),
            file_size, 0
        ));

        assert_eq!(Market::files(&cid).unwrap_or_default(), (
            FileInfo {
                file_size,
                expired_on: 0,
                calculated_at: 50,
                amount: 360, // ( 1000 + 1000 * 1 + 0 ) * 0.2
                prepaid: 0,
                reported_replica_count: 0,
                replicas: vec![]
            },
            UsedInfo {
                used_size: 0,
                reported_group_count: 0,
                groups: BTreeMap::new()
            })
        );

        run_to_block(303);

        let legal_wr_info = legal_work_report_with_added_files();
        let legal_pk = legal_wr_info.curr_pk.clone();

        register(&legal_pk, LegalCode::get());

        assert_eq!(Market::files(&cid).unwrap_or_default(), (
            FileInfo {
                file_size,
                expired_on: 0,
                calculated_at: 50,
                amount: 360, // ( 1000 + 1000 * 1 + 0 ) * 0.2
                prepaid: 0,
                reported_replica_count: 0,
                replicas: vec![]
            },
            UsedInfo {
                used_size: 0,
                reported_group_count: 0,
                groups: BTreeMap::new()
            })
        );

        run_to_block(1506);
        Market::do_calculate_reward(&cid, System::block_number().try_into().unwrap());

        assert_eq!(Market::files(&cid).unwrap_or_default(), (
            FileInfo {
                file_size,
                expired_on: 0,
                calculated_at: 50,
                amount: 360, // ( 1000 + 1000 * 1 + 0 ) * 0.2
                prepaid: 0,
                reported_replica_count: 0,
                replicas: vec![]
            },
            UsedInfo {
                used_size: 0,
                reported_group_count: 0,
                groups: BTreeMap::new()
            })
        );
    });
}

#[test]
fn do_calculate_reward_should_work_for_more_replicas() {
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
            "QmdwgqZy1MZBfWPi7GcxVsYgJEtmvHg6rsLzbCej3tf3oF".as_bytes().to_vec();
        let file_size = 134289408;
        let _ = Balances::make_free_balance_be(&source, 20_000_000);
        let merchants = vec![merchant.clone(), charlie.clone(), dave.clone(), eve.clone(), ferdie.clone()];
        for who in merchants.iter() {
            let _ = Balances::make_free_balance_be(&who, 20_000_000);
            assert_ok!(Market::register(Origin::signed(who.clone()), 6_000_000));
        }

        assert_ok!(Market::place_storage_order(
            Origin::signed(source), cid.clone(),
            file_size, 0
        ));

        assert_eq!(Market::files(&cid).unwrap_or_default(), (
            FileInfo {
                file_size,
                expired_on: 0,
                calculated_at: 50,
                amount: 23400,
                prepaid: 0,
                reported_replica_count: 0,
                replicas: vec![]
            },
            UsedInfo {
                used_size: 0,
                reported_group_count: 0,
                groups: BTreeMap::new()
            })
        );

        run_to_block(303);

        let legal_wr_info = legal_work_report_with_added_files();
        let legal_pk = legal_wr_info.curr_pk.clone();

        add_who_into_replica(&cid, file_size, ferdie.clone(), legal_pk.clone(), Some(303u32), None);
        assert_eq!(Market::files_size(), file_size as u128);
        add_who_into_replica(&cid, file_size, charlie.clone(), legal_pk.clone(), Some(403u32), None);
        assert_eq!(Market::files_size(), (file_size * 2) as u128);
        add_who_into_replica(&cid, file_size, dave.clone(), legal_pk.clone(), Some(503u32), None);
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

        add_who_into_replica(&cid, file_size, eve.clone(), legal_pk.clone(), Some(503u32), None);

        assert_eq!(Market::files(&cid).unwrap_or_default(), (
            FileInfo {
                file_size,
                expired_on: 1303,
                calculated_at: 303,
                amount: 23400,
                prepaid: 0,
                reported_replica_count: 5,
                replicas: vec![
                    Replica {
                        who: ferdie.clone(),
                        valid_at: 303,
                        anchor: legal_pk.clone(),
                        is_reported: true
                    },
                    Replica {
                        who: merchant.clone(),
                        valid_at: 303,
                        anchor: legal_pk.clone(),
                        is_reported: true
                    },
                    Replica {
                        who: charlie.clone(),
                        valid_at: 403,
                        anchor: legal_pk.clone(),
                        is_reported: true
                    },
                    Replica {
                        who: dave.clone(),
                        valid_at: 503,
                        anchor: legal_pk.clone(),
                        is_reported: true
                    },
                    Replica {
                        who: eve.clone(),
                        valid_at: 503,
                        anchor: legal_pk.clone(),
                        is_reported: true
                    }
                ]
            },
            UsedInfo {
                used_size: file_size * 2,
                reported_group_count: 5,
                groups: BTreeMap::from_iter(vec![(legal_pk.clone(), true)].into_iter())
            })
        );
        run_to_block(503);
        <swork::ReportedInSlot>::insert(legal_pk.clone(), 0, true);
        Market::do_calculate_reward(&cid, System::block_number().try_into().unwrap());
        assert_eq!(Market::files(&cid).unwrap_or_default(), (
            FileInfo {
                file_size,
                expired_on: 1303,
                calculated_at: 503,
                amount: 18724,
                prepaid: 0,
                reported_replica_count: 5,
                replicas: vec![
                    Replica {
                        who: ferdie.clone(),
                        valid_at: 303,
                        anchor: legal_pk.clone(),
                        is_reported: true
                    },
                    Replica {
                        who: merchant.clone(),
                        valid_at: 303,
                        anchor: legal_pk.clone(),
                        is_reported: true
                    },
                    Replica {
                        who: charlie.clone(),
                        valid_at: 403,
                        anchor: legal_pk.clone(),
                        is_reported: true
                    },
                    Replica {
                        who: dave.clone(),
                        valid_at: 503,
                        anchor: legal_pk.clone(),
                        is_reported: true
                    },
                    Replica {
                        who: eve.clone(),
                        valid_at: 503,
                        anchor: legal_pk.clone(),
                        is_reported: true
                    }
                ]
            },
            UsedInfo {
                used_size: file_size * 2,
                reported_group_count: 1,
                groups: BTreeMap::from_iter(vec![(legal_pk.clone(), true)].into_iter())
            })
        );

        assert_eq!(Market::merchant_ledgers(&ferdie), MerchantLedger {
            collateral: 6_000_000,
            reward: 1169
        });
        assert_eq!(Market::merchant_ledgers(&merchant), MerchantLedger {
            collateral: 6_000_000,
            reward: 1169
        });
        assert_eq!(Market::merchant_ledgers(&charlie), MerchantLedger {
            collateral: 6_000_000,
            reward: 1169
        });
        assert_eq!(Market::merchant_ledgers(&dave), MerchantLedger {
            collateral: 6_000_000,
            reward: 1169
        });
        assert_eq!(Market::merchant_ledgers(&eve), MerchantLedger {
            collateral: 6_000_000,
            reward: 0
        });
    });
}

#[test]
fn do_calculate_reward_should_only_pay_the_groups() {
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
            "QmdwgqZy1MZBfWPi7GcxVsYgJEtmvHg6rsLzbCej3tf3oF".as_bytes().to_vec();
        let file_size = 134289408;
        let _ = Balances::make_free_balance_be(&source, 20_000_000);
        let merchants = vec![merchant.clone(), charlie.clone(), dave.clone(), eve.clone(), ferdie.clone()];
        for who in merchants.iter() {
            let _ = Balances::make_free_balance_be(&who, 20_000_000);
            assert_ok!(Market::register(Origin::signed(who.clone()), 6_000_000));
        }

        assert_ok!(Market::place_storage_order(
            Origin::signed(source), cid.clone(),
            file_size, 0
        ));

        assert_eq!(Market::files(&cid).unwrap_or_default(), (
            FileInfo {
                file_size,
                expired_on: 0,
                calculated_at: 50,
                amount: 23400,
                prepaid: 0,
                reported_replica_count: 0,
                replicas: vec![]
            },
            UsedInfo {
                used_size: 0,
                reported_group_count: 0,
                groups: BTreeMap::new()
            })
        );

        run_to_block(303);

        let legal_wr_info = legal_work_report_with_added_files();
        let legal_pk = legal_wr_info.curr_pk.clone();

        <swork::Identities<Test>>::insert(ferdie.clone(), Identity {
            anchor: hex::decode("11").unwrap(),
            punishment_deadline: 0,
            group: None
        });
        <swork::Identities<Test>>::insert(charlie.clone(), Identity {
            anchor: hex::decode("22").unwrap(),
            punishment_deadline: 0,
            group: None
        });
        add_who_into_replica(&cid, file_size, ferdie.clone(), hex::decode("11").unwrap(), Some(303u32), Some(BTreeSet::from_iter(vec![charlie.clone(), ferdie.clone()].into_iter())));
        assert_eq!(Market::files_size(), file_size as u128);
        add_who_into_replica(&cid, file_size, charlie.clone(), hex::decode("22").unwrap(), Some(403u32), Some(BTreeSet::from_iter(vec![charlie.clone(), ferdie.clone()].into_iter())));
        assert_eq!(Market::files_size(), file_size as u128);
        add_who_into_replica(&cid, file_size, dave.clone(), hex::decode("33").unwrap(), Some(503u32), None);
        assert_eq!(Market::files_size(), (file_size * 2) as u128);

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

        assert_eq!(Market::files_size(), (file_size * 3) as u128);

        add_who_into_replica(&cid, file_size, eve.clone(), legal_pk.clone(), Some(503u32), None);

        assert_eq!(Market::files(&cid).unwrap_or_default(), (
            FileInfo {
                file_size,
                expired_on: 1303,
                calculated_at: 303,
                amount: 23400,
                prepaid: 0,
                reported_replica_count: 5,
                replicas: vec![
                    Replica {
                        who: ferdie.clone(),
                        valid_at: 303,
                        anchor: hex::decode("11").unwrap(),
                        is_reported: true
                    },
                    Replica {
                        who: merchant.clone(),
                        valid_at: 303,
                        anchor: legal_pk.clone(),
                        is_reported: true
                    },
                    Replica {
                        who: charlie.clone(),
                        valid_at: 403,
                        anchor: hex::decode("22").unwrap(),
                        is_reported: true
                    },
                    Replica {
                        who: dave.clone(),
                        valid_at: 503,
                        anchor: hex::decode("33").unwrap(),
                        is_reported: true
                    },
                    Replica {
                        who: eve.clone(),
                        valid_at: 503,
                        anchor: legal_pk.clone(),
                        is_reported: true
                    }
                ]
            },
            UsedInfo {
                used_size: file_size * 2,
                reported_group_count: 4,
                groups: BTreeMap::from_iter(vec![(legal_pk.clone(), true), (hex::decode("11").unwrap(), true), (hex::decode("33").unwrap(), true)].into_iter())
            })
        );
        run_to_block(503);
        <swork::ReportedInSlot>::insert(legal_pk.clone(), 0, true);
        <swork::ReportedInSlot>::insert(hex::decode("11").unwrap(), 0, true);
        <swork::ReportedInSlot>::insert(hex::decode("22").unwrap(), 0, true);
        <swork::ReportedInSlot>::insert(hex::decode("33").unwrap(), 0, true);
        Market::do_calculate_reward(&cid, System::block_number().try_into().unwrap());
        assert_eq!(Market::files(&cid).unwrap_or_default(), (
            FileInfo {
                file_size,
                expired_on: 1303,
                calculated_at: 503,
                amount: 18724,
                prepaid: 0,
                reported_replica_count: 5,
                replicas: vec![
                    Replica {
                        who: ferdie.clone(),
                        valid_at: 303,
                        anchor: hex::decode("11").unwrap(),
                        is_reported: true
                    },
                    Replica {
                        who: merchant.clone(),
                        valid_at: 303,
                        anchor: legal_pk.clone(),
                        is_reported: true
                    },
                    Replica {
                        who: charlie.clone(),
                        valid_at: 403,
                        anchor: hex::decode("22").unwrap(),
                        is_reported: true
                    },
                    Replica {
                        who: dave.clone(),
                        valid_at: 503,
                        anchor: hex::decode("33").unwrap(),
                        is_reported: true
                    },
                    Replica {
                        who: eve.clone(),
                        valid_at: 503,
                        anchor: legal_pk.clone(),
                        is_reported: true
                    }
                ]
            },
            UsedInfo {
                used_size: file_size * 2,
                reported_group_count: 3,
                groups: BTreeMap::from_iter(vec![(legal_pk.clone(), true), (hex::decode("11").unwrap(), true), (hex::decode("33").unwrap(), true)].into_iter())
            })
        );

        assert_eq!(Market::merchant_ledgers(&ferdie), MerchantLedger {
            collateral: 6_000_000,
            reward: 1169
        });
        assert_eq!(Market::merchant_ledgers(&merchant), MerchantLedger {
            collateral: 6_000_000,
            reward: 1169
        });
        // charlie won't get payed
        assert_eq!(Market::merchant_ledgers(&charlie), MerchantLedger {
            collateral: 6_000_000,
            reward: 0
        });
        assert_eq!(Market::merchant_ledgers(&dave), MerchantLedger {
            collateral: 6_000_000,
            reward: 1169
        });
        assert_eq!(Market::merchant_ledgers(&eve), MerchantLedger {
            collateral: 6_000_000,
            reward: 1169
        });
    });
}


#[test]
fn insert_replica_should_work_for_complex_scenario() {
    new_test_ext().execute_with(|| {
        // generate 50 blocks first
        run_to_block(50);

        let source = ALICE;
        let merchant = MERCHANT;
        let charlie = CHARLIE;
        let dave = DAVE;
        let eve = EVE;
        let ferdie = FERDIE;
        let zikun = ZIKUN;

        let cid = "QmdwgqZy1MZBfWPi7GcxVsYgJEtmvHg6rsLzbCej3tf3oF".as_bytes().to_vec();
        let file_size = 134289408;
        let _ = Balances::make_free_balance_be(&source, 20_000_000);
        let merchants = vec![merchant.clone(), charlie.clone(), dave.clone(), eve.clone(), ferdie.clone(), zikun.clone()];
        for who in merchants.iter() {
            let _ = Balances::make_free_balance_be(&who, 20_000_000);
            assert_ok!(Market::register(Origin::signed(who.clone()), 6_000_000));
        }

        assert_ok!(Market::place_storage_order(
            Origin::signed(source), cid.clone(),
            file_size, 0
        ));

        assert_eq!(Market::files(&cid).unwrap_or_default(), (
            FileInfo {
                file_size,
                expired_on: 0,
                calculated_at: 50,
                amount: 23400,
                prepaid: 0,
                reported_replica_count: 0,
                replicas: vec![]
            },
            UsedInfo {
                used_size: 0,
                reported_group_count: 0,
                groups: BTreeMap::new()
            })
        );

        run_to_block(303);

        let legal_wr_info = legal_work_report_with_added_files();
        let legal_pk = legal_wr_info.curr_pk.clone();

        add_who_into_replica(&cid, file_size, ferdie.clone(), legal_pk.clone(), Some(503u32), None);

        assert_eq!(Market::files(&cid).unwrap_or_default(), (
            FileInfo {
                file_size,
                expired_on: 1303,
                calculated_at: 303,
                amount: 23400,
                prepaid: 0,
                reported_replica_count: 1,
                replicas: vec![
                Replica {
                    who: ferdie.clone(),
                    valid_at: 503,
                    anchor: legal_pk.clone(),
                    is_reported: true
                }]
            },
            UsedInfo {
                used_size: file_size * 2,
                reported_group_count: 1,
                groups: BTreeMap::from_iter(vec![(legal_pk.clone(), true)].into_iter())
            })
        );

        add_who_into_replica(&cid, file_size, charlie.clone(), legal_pk.clone(), Some(303u32), None);

        assert_eq!(Market::files(&cid).unwrap_or_default(), (
            FileInfo {
                file_size,
                expired_on: 1303,
                calculated_at: 303,
                amount: 23400,
                prepaid: 0,
                reported_replica_count: 2,
                replicas: vec![
                    Replica {
                        who: charlie.clone(),
                        valid_at: 303,
                        anchor: legal_pk.clone(),
                        is_reported: true
                    },
                    Replica {
                        who: ferdie.clone(),
                        valid_at: 503,
                        anchor: legal_pk.clone(),
                        is_reported: true
                    }]
            },
            UsedInfo {
                used_size: file_size * 2,
                reported_group_count: 2,
                groups: BTreeMap::from_iter(vec![(legal_pk.clone(), true)].into_iter())
            })
        );

        add_who_into_replica(&cid, file_size, dave.clone(), legal_pk.clone(), Some(103u32), None);

        assert_eq!(Market::files(&cid).unwrap_or_default(), (
            FileInfo {
                file_size,
                expired_on: 1303,
                calculated_at: 303,
                amount: 23400,
                prepaid: 0,
                reported_replica_count: 3,
                replicas: vec![
                    Replica {
                        who: dave.clone(),
                        valid_at: 103,
                        anchor: legal_pk.clone(),
                        is_reported: true
                    },
                    Replica {
                        who: charlie.clone(),
                        valid_at: 303,
                        anchor: legal_pk.clone(),
                        is_reported: true
                    },
                    Replica {
                        who: ferdie.clone(),
                        valid_at: 503,
                        anchor: legal_pk.clone(),
                        is_reported: true
                    }]
            },
            UsedInfo {
                used_size: file_size * 2,
                reported_group_count: 3,
                groups: BTreeMap::from_iter(vec![(legal_pk.clone(), true)].into_iter())
            })
        );

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

        assert_eq!(Market::files(&cid).unwrap_or_default(), (
            FileInfo {
                file_size,
                expired_on: 1303,
                calculated_at: 303,
                amount: 23400,
                prepaid: 0,
                reported_replica_count: 4,
                replicas: vec![
                    Replica {
                        who: dave.clone(),
                        valid_at: 103,
                        anchor: legal_pk.clone(),
                        is_reported: true
                    },
                    Replica {
                        who: charlie.clone(),
                        valid_at: 303,
                        anchor: legal_pk.clone(),
                        is_reported: true
                    },
                    Replica {
                        who: merchant.clone(),
                        valid_at: 303,
                        anchor: legal_pk.clone(),
                        is_reported: true
                    },
                    Replica {
                        who: ferdie.clone(),
                        valid_at: 503,
                        anchor: legal_pk.clone(),
                        is_reported: true
                    }]
            },
            UsedInfo {
                used_size: file_size * 2,
                reported_group_count: 4,
                groups: BTreeMap::from_iter(vec![(legal_pk.clone(), true)].into_iter())
            })
        );
        add_who_into_replica(&cid, file_size, eve.clone(), legal_pk.clone(), Some(703u32), None);
        assert_eq!(Market::files(&cid).unwrap_or_default(), (
            FileInfo {
                file_size,
                expired_on: 1303,
                calculated_at: 303,
                amount: 23400,
                prepaid: 0,
                reported_replica_count: 5,
                replicas: vec![
                    Replica {
                        who: dave.clone(),
                        valid_at: 103,
                        anchor: legal_pk.clone(),
                        is_reported: true
                    },
                    Replica {
                        who: charlie.clone(),
                        valid_at: 303,
                        anchor: legal_pk.clone(),
                        is_reported: true
                    },
                    Replica {
                        who: merchant.clone(),
                        valid_at: 303,
                        anchor: legal_pk.clone(),
                        is_reported: true
                    },
                    Replica {
                        who: ferdie.clone(),
                        valid_at: 503,
                        anchor: legal_pk.clone(),
                        is_reported: true
                    },
                    Replica {
                        who: eve.clone(),
                        valid_at: 703,
                        anchor: legal_pk.clone(),
                        is_reported: true
                    }]
            },
            UsedInfo {
                used_size: file_size * 2,
                reported_group_count: 5,
                groups: BTreeMap::from_iter(vec![(legal_pk.clone(), true)].into_iter())
            })
        );

        add_who_into_replica(&cid, file_size, zikun.clone(), legal_pk.clone(), Some(255u32), None);
        assert_eq!(Market::files(&cid).unwrap_or_default(), (
            FileInfo {
                file_size,
                expired_on: 1303,
                calculated_at: 303,
                amount: 23400,
                prepaid: 0,
                reported_replica_count: 6,
                replicas: vec![
                    Replica {
                        who: dave.clone(),
                        valid_at: 103,
                        anchor: legal_pk.clone(),
                        is_reported: true
                    },
                    Replica {
                        who: zikun.clone(),
                        valid_at: 255,
                        anchor: legal_pk.clone(),
                        is_reported: true
                    },
                    Replica {
                        who: charlie.clone(),
                        valid_at: 303,
                        anchor: legal_pk.clone(),
                        is_reported: true
                    },
                    Replica {
                        who: merchant.clone(),
                        valid_at: 303,
                        anchor: legal_pk.clone(),
                        is_reported: true
                    },
                    Replica {
                        who: ferdie.clone(),
                        valid_at: 503,
                        anchor: legal_pk.clone(),
                        is_reported: true
                    },
                    Replica {
                        who: eve.clone(),
                        valid_at: 703,
                        anchor: legal_pk.clone(),
                        is_reported: true
                    }]
            },
            UsedInfo {
                used_size: file_size * 2,
                reported_group_count: 6,
                groups: BTreeMap::from_iter(vec![(legal_pk.clone(), true)].into_iter())
            })
        );
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
                file_size, 0
            ));
            assert_eq!(Market::files(&cid).unwrap_or_default(), (
            FileInfo {
                file_size,
                expired_on: 0,
                calculated_at: 50,
                amount: 360, // ( 1000 + 1000 * 1 + 0 ) * 0.2
                prepaid: 0,
                reported_replica_count: 0,
                replicas: vec![]
            },
            UsedInfo {
                used_size: 0,
                reported_group_count: 0,
                groups: BTreeMap::new()
            })
            );
        }

        run_to_block(303);
        let legal_wr_info = legal_work_report_with_added_files();
        let legal_pk = legal_wr_info.curr_pk.clone();
        register(&legal_pk, LegalCode::get());
        for cid in file_lists.clone().iter() {
            add_who_into_replica(&cid, file_size, merchant.clone(), legal_pk.clone(), None, None);
        }

        for cid in file_lists.clone().iter() {
            assert_eq!(Market::files(&cid).unwrap_or_default(), (
                FileInfo {
                    file_size,
                    expired_on: 1303,
                    calculated_at: 303,
                    amount: 360, // ( 1000 + 1000 * 1 + 0 ) * 0.2
                    prepaid: 0,
                    reported_replica_count: 1,
                    replicas: vec![Replica {
                        who: merchant.clone(),
                        valid_at: 303,
                        anchor: legal_pk.clone(),
                        is_reported: true
                    }]
                },
                UsedInfo {
                    used_size: file_size * 2,
                    reported_group_count: 1,
                    groups: BTreeMap::from_iter(vec![(legal_pk.clone(), true)].into_iter())
                })
            );
        }

        run_to_block(1500);
        <swork::ReportedInSlot>::insert(legal_pk.clone(), 1200, true);
        // close files one by one
        assert_ok!(Market::calculate_reward(Origin::signed(merchant.clone()), cid1.clone()));
        assert_eq!(Market::used_trash_i(&cid1).unwrap_or_default(), UsedInfo {
            used_size: file_size * 2,
            reported_group_count: 1,
            groups: BTreeMap::from_iter(vec![(legal_pk.clone(), true)].into_iter())
        });

        assert_ok!(Market::calculate_reward(Origin::signed(merchant.clone()), cid2.clone()));
        assert_eq!(Market::used_trash_i(&cid2).unwrap_or_default(), UsedInfo {
            used_size: file_size * 2,
            reported_group_count: 1,
            groups: BTreeMap::from_iter(vec![(legal_pk.clone(), true)].into_iter())
        });

        assert_ok!(Market::calculate_reward(Origin::signed(merchant.clone()), cid3.clone()));
        assert_eq!(Market::used_trash_ii(&cid3).unwrap_or_default(), UsedInfo {
            used_size: file_size * 2,
            reported_group_count: 1,
            groups: BTreeMap::from_iter(vec![(legal_pk.clone(), true)].into_iter())
        });

        assert_ok!(Market::calculate_reward(Origin::signed(merchant.clone()), cid4.clone()));
        assert_eq!(Market::used_trash_ii(&cid4).unwrap_or_default(), UsedInfo {
            used_size: file_size * 2,
            reported_group_count: 1,
            groups: BTreeMap::from_iter(vec![(legal_pk.clone(), true)].into_iter())
        });
        assert_eq!(Market::used_trash_i(&cid1), None);
        assert_eq!(Market::used_trash_i(&cid2), None);

        assert_ok!(Market::calculate_reward(Origin::signed(merchant.clone()), cid5.clone()));
        assert_eq!(Market::used_trash_i(&cid5).unwrap_or_default(), UsedInfo {
            used_size: file_size * 2,
            reported_group_count: 1,
            groups: BTreeMap::from_iter(vec![(legal_pk.clone(), true)].into_iter())
        });

        assert_ok!(Market::calculate_reward(Origin::signed(merchant.clone()), cid6.clone()));
        assert_eq!(Market::used_trash_i(&cid6).unwrap_or_default(), UsedInfo {
            used_size: file_size * 2,
            reported_group_count: 1,
            groups: BTreeMap::from_iter(vec![(legal_pk.clone(), true)].into_iter())
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
        <swork::ReportedFilesSize>::put(10000);
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
        assert_eq!(Market::withdraw_staking_pot(), 0);
        assert_eq!(Balances::free_balance(&staking_pot), 0);
        let _ = Balances::make_free_balance_be(&source, 20000);
        let _ = Balances::make_free_balance_be(&merchant, 200);

        assert_ok!(Market::register(Origin::signed(merchant.clone()), 60));

        assert_ok!(Market::place_storage_order(
            Origin::signed(source), cid.clone(),
            file_size, 0
        ));

        assert_eq!(Market::files(&cid).unwrap_or_default(), (
            FileInfo {
                file_size,
                expired_on: 0,
                calculated_at: 50,
                amount: 360, // ( 1000 + 1000 * 1 + 0 ) * 0.2
                prepaid: 0,
                reported_replica_count: 0,
                replicas: vec![]
            },
            UsedInfo {
                used_size: 0,
                reported_group_count: 0,
                groups: BTreeMap::new()
            })
        );
        assert_eq!(Balances::free_balance(&reserved_pot), 200);
        assert_eq!(Balances::free_balance(&staking_pot), 1440);
        assert_eq!(Balances::free_balance(&storage_pot), 360);

        assert_eq!(Market::withdraw_staking_pot(), 1439);
        assert_eq!(Balances::free_balance(&staking_pot), 1);
    });
}

/// reported file size is not same with file size
#[test]
fn scenario_test_for_reported_file_size_is_not_same_with_file_size() {
    new_test_ext().execute_with(|| {
        // generate 50 blocks first
        run_to_block(50);

        let source = ALICE;
        let merchant = MERCHANT;

        let cid1 =
            hex::decode("5bb706320afc633bfb843108e492192b17d2b6b9d9ee0b795ee95417fe08b660").unwrap();
        let cid2 =
            hex::decode("5bb706320afc633bfb843108e492192b17d2b6b9d9ee0b795ee95417fe08b661").unwrap();
        let file_lists = vec![cid1.clone(), cid2.clone()];
        let file_size = 100; // should less than merchant
        let reported_file_size_cid1 = 90;
        let reported_file_size_cid2 = 1000;
        let storage_pot = Market::storage_pot();
        let _ = Balances::make_free_balance_be(&storage_pot, 1);
        let _ = Balances::make_free_balance_be(&source, 20000);
        let _ = Balances::make_free_balance_be(&merchant, 20000);

        assert_ok!(Market::register(Origin::signed(merchant.clone()), 6000));

        for cid in file_lists.clone().iter() {
            assert_ok!(Market::place_storage_order(
                Origin::signed(source.clone()), cid.clone(),
                file_size, 0
            ));
            assert_eq!(Market::files(&cid).unwrap_or_default(), (
                FileInfo {
                    file_size,
                    expired_on: 0,
                    calculated_at: 50,
                    amount: 360, // ( 1000 + 1000 * 1 + 0 ) * 0.2
                    prepaid: 0,
                    reported_replica_count: 0,
                    replicas: vec![]
                },
                UsedInfo {
                    used_size: 0,
                    reported_group_count: 0,
                    groups: BTreeMap::new()
                })
            );
        }
        assert_eq!(Balances::free_balance(&storage_pot), 721);

        run_to_block(303);
        let legal_wr_info = legal_work_report_with_added_files();
        let legal_pk = legal_wr_info.curr_pk.clone();
        register(&legal_pk, LegalCode::get());
        // reported_file_size_cid1 = 90 < 100 => update file size in file info
        add_who_into_replica(&cid1, reported_file_size_cid1, merchant.clone(), legal_pk.clone(), None, None);
        assert_eq!(Market::files(&cid1).unwrap_or_default(), (
            FileInfo {
                file_size: reported_file_size_cid1,
                expired_on: 1303,
                calculated_at: 303,
                amount: 360, // ( 1000 + 1000 * 1 + 0 ) * 0.2
                prepaid: 0,
                reported_replica_count: 1,
                replicas: vec![Replica {
                    who: merchant.clone(),
                    valid_at: 303,
                    anchor: legal_pk.clone(),
                    is_reported: true
                }]
            },
            UsedInfo {
                used_size: reported_file_size_cid1 * 2,
                reported_group_count: 1,
                groups: BTreeMap::from_iter(vec![(legal_pk.clone(), true)].into_iter())
            })
        );
        assert_eq!(Balances::free_balance(&storage_pot), 721);
        // reported_file_size_cid2 = 1000 > 100 => close this file
        add_who_into_replica(&cid2, reported_file_size_cid2, merchant.clone(), legal_pk.clone(), None, None);
        assert_eq!(Market::files(&cid2).is_none(), true);
        assert_eq!(Market::merchant_ledgers(&merchant), MerchantLedger {
            collateral: 6000,
            reward: 360
        });
        assert_eq!(Balances::free_balance(&storage_pot), 721);
    })
}


/// reported file size is not same with file size
#[test]
fn double_place_storage_order_file_size_check_should_work() {
    new_test_ext().execute_with(|| {
        // generate 50 blocks first
        run_to_block(50);

        let source = ALICE;
        let merchant = MERCHANT;

        let cid1 =
            hex::decode("5bb706320afc633bfb843108e492192b17d2b6b9d9ee0b795ee95417fe08b660").unwrap();
        let file_size = 100; // should less than merchant
        let reported_file_size_cid1 = 90;
        let _ = Balances::make_free_balance_be(&source, 20000);
        let _ = Balances::make_free_balance_be(&merchant, 20000);

        assert_ok!(Market::register(Origin::signed(merchant.clone()), 6000));

        assert_ok!(Market::place_storage_order(
            Origin::signed(source.clone()), cid1.clone(),
            file_size, 0
        ));
        assert_eq!(Market::files(&cid1).unwrap_or_default(), (
            FileInfo {
                file_size,
                expired_on: 0,
                calculated_at: 50,
                amount: 360, // ( 1000 + 1000 * 1 + 0 ) * 0.2
                prepaid: 0,
                reported_replica_count: 0,
                replicas: vec![]
            },
            UsedInfo {
                used_size: 0,
                reported_group_count: 0,
                groups: BTreeMap::new()
            })
        );

        run_to_block(303);
        let legal_wr_info = legal_work_report_with_added_files();
        let legal_pk = legal_wr_info.curr_pk.clone();
        register(&legal_pk, LegalCode::get());
        add_who_into_replica(&cid1, reported_file_size_cid1, merchant.clone(), legal_pk.clone(), None, None);
        assert_eq!(Market::files(&cid1).unwrap_or_default(), (
            FileInfo {
                file_size: reported_file_size_cid1,
                expired_on: 1303,
                calculated_at: 303,
                amount: 360, // ( 1000 + 1000 * 1 + 0 ) * 0.2
                prepaid: 0,
                reported_replica_count: 1,
                replicas: vec![Replica {
                    who: merchant.clone(),
                    valid_at: 303,
                    anchor: legal_pk.clone(),
                    is_reported: true
                }]
            },
            UsedInfo {
                used_size: reported_file_size_cid1 * 2,
                reported_group_count: 1, // duplicate legal pk and this scenario only occurs in test
                groups: BTreeMap::from_iter(vec![(legal_pk.clone(), true)].into_iter())
            })
        );

        // 80 < 100 => throw an error
        assert_noop!(Market::place_storage_order(
            Origin::signed(source.clone()), cid1.clone(), 80, 0),
            DispatchError::Module {
                index: 3,
                error: 6,
                message: Some("FileSizeNotCorrect")
            }
        );

        // 12000000 > 100. Only need amount for 100
        assert_ok!(Market::place_storage_order(
            Origin::signed(source.clone()), cid1.clone(),
            12000000, 0
        ));

        assert_eq!(Market::files(&cid1).unwrap_or_default(), (
            FileInfo {
                file_size: reported_file_size_cid1,
                expired_on: 1303,
                calculated_at: 303,
                amount: 720, // ( 1000 + 1000 * 1 + 0 ) * 0.2
                prepaid: 0,
                reported_replica_count: 0,
                replicas: vec![Replica {
                    who: merchant.clone(),
                    valid_at: 303,
                    anchor: legal_pk.clone(),
                    is_reported: false
                }]
            },
            UsedInfo {
                used_size: 0,
                reported_group_count: 0,
                groups: BTreeMap::from_iter(vec![(legal_pk.clone(), false)].into_iter())
            })
        );
    })
}

#[test]
fn place_storage_order_for_expired_file_should_inherit_the_status() {
    new_test_ext().execute_with(|| {
        // generate 50 blocks first
        run_to_block(50);

        let cid =
            "QmdwgqZy1MZBfWPi7GcxVsYgJEtmvHg6rsLzbCej3tf3oF".as_bytes().to_vec();
        let file_size = 134289408; // should less than merchant
        let source = ALICE;
        let merchant = BOB;
        let charlie = CHARLIE;
        let dave = DAVE;
        let eve = EVE;

        let staking_pot = Market::staking_pot();
        let reserved_pot = Market::reserved_pot();
        assert_eq!(Balances::free_balance(&staking_pot), 0);
        assert_eq!(Balances::free_balance(&reserved_pot), 0);
        let _ = Balances::make_free_balance_be(&source, 20_000_000);
        let merchants = vec![merchant.clone(), charlie.clone(), dave.clone(), eve.clone()];
        for who in merchants.iter() {
            let _ = Balances::make_free_balance_be(&who, 20_000_000);
            assert_ok!(Market::register(Origin::signed(who.clone()), 6_000_000));
        }

        assert_ok!(Market::place_storage_order(
            Origin::signed(source.clone()), cid.clone(),
            file_size, 0
        ));

        assert_eq!(Market::files(&cid).unwrap_or_default(), (
            FileInfo {
                file_size,
                expired_on: 0,
                calculated_at: 50,
                amount: 23400,
                prepaid: 0,
                reported_replica_count: 0,
                replicas: vec![]
            },
            UsedInfo {
                used_size: 0,
                reported_group_count: 0,
                groups: BTreeMap::new()
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

        assert_eq!(Market::files_size(), file_size as u128);

        assert_eq!(Market::files(&cid).unwrap_or_default(), (
            FileInfo {
                file_size,
                expired_on: 1303,
                calculated_at: 303,
                amount: 23400,
                prepaid: 0,
                reported_replica_count: 1,
                replicas: vec![Replica {
                    who: merchant.clone(),
                    valid_at: 303,
                    anchor: legal_pk.clone(),
                    is_reported: true
                }]
            },
            UsedInfo {
                used_size: file_size * 2,
                reported_group_count: 1,
                groups: BTreeMap::from_iter(vec![(legal_pk.clone(), true)].into_iter())
            })
        );

        run_to_block(503);
        <swork::ReportedInSlot>::insert(legal_pk.clone(), 0, true);
        Market::do_calculate_reward(&cid, System::block_number().try_into().unwrap());
        assert_eq!(Market::files(&cid).unwrap_or_default(), (
            FileInfo {
                file_size,
                expired_on: 1303,
                calculated_at: 503,
                amount: 18721,
                prepaid: 0,
                reported_replica_count: 1,
                replicas: vec![Replica {
                    who: merchant.clone(),
                    valid_at: 303,
                    anchor: legal_pk.clone(),
                    is_reported: true
                }]
            },
            UsedInfo {
                used_size: file_size * 2,
                reported_group_count: 1,
                groups: BTreeMap::from_iter(vec![(legal_pk.clone(), true)].into_iter())
            })
        );
        assert_eq!(Market::merchant_ledgers(&merchant), MerchantLedger {
            collateral: 6_000_000,
            reward: 4679
        });

        add_who_into_replica(&cid, file_size, charlie.clone(), legal_pk.clone(), None, None);

        assert_eq!(Market::files(&cid).unwrap_or_default(), (
            FileInfo {
                file_size,
                expired_on: 1303,
                calculated_at: 503,
                amount: 18721,
                prepaid: 0,
                reported_replica_count: 2,
                replicas: vec![
                    Replica {
                        who: merchant.clone(),
                        valid_at: 303,
                        anchor: legal_pk.clone(),
                        is_reported: true
                    },
                    Replica {
                        who: charlie.clone(),
                        valid_at: 503,
                        anchor: legal_pk.clone(),
                        is_reported: true
                    }]
            },
            UsedInfo {
                used_size: file_size * 2,
                reported_group_count: 2,
                groups: BTreeMap::from_iter(vec![(legal_pk.clone(), true)].into_iter())
            })
        );

        run_to_block(1803);
        <swork::ReportedInSlot>::insert(legal_pk.clone(), 1500, true);
        assert_ok!(Market::place_storage_order(
            Origin::signed(source), cid.clone(),
            file_size, 0
        ));
        assert_eq!(Market::files(&cid).unwrap_or_default(), (
            FileInfo {
                file_size,
                expired_on: 2303,
                calculated_at: 1303,
                amount: 23403,
                prepaid: 0,
                reported_replica_count: 2,
                replicas: vec![
                    Replica {
                        who: merchant.clone(),
                        valid_at: 303,
                        anchor: legal_pk.clone(),
                        is_reported: true
                    },
                    Replica {
                        who: charlie.clone(),
                        valid_at: 503,
                        anchor: legal_pk.clone(),
                        is_reported: true
                    }]
            },
            UsedInfo {
                used_size: file_size * 2,
                reported_group_count: 1,
                groups: BTreeMap::from_iter(vec![(legal_pk.clone(), true)].into_iter())
            })
        );

        assert_eq!(Market::merchant_ledgers(&merchant), MerchantLedger {
            collateral: 6_000_000,
            reward: 14038
        });
        assert_eq!(Market::merchant_ledgers(&charlie), MerchantLedger {
            collateral: 6_000_000,
            reward: 9359
        });
        assert_eq!(Market::files_size(), file_size as u128);
        assert_eq!(Balances::free_balance(&reserved_pot), 26000);
    });
}


#[test]
fn place_storage_order_for_expired_file_should_make_it_pending_if_replicas_is_zero() {
    new_test_ext().execute_with(|| {
        // generate 50 blocks first
        run_to_block(50);

        let cid =
            "QmdwgqZy1MZBfWPi7GcxVsYgJEtmvHg6rsLzbCej3tf3oF".as_bytes().to_vec();
        let file_size = 134289408; // should less than merchant
        let source = ALICE;
        let merchant = BOB;
        let charlie = CHARLIE;
        let dave = DAVE;
        let eve = EVE;

        let staking_pot = Market::staking_pot();
        let reserved_pot = Market::reserved_pot();
        assert_eq!(Balances::free_balance(&staking_pot), 0);
        assert_eq!(Balances::free_balance(&reserved_pot), 0);
        let _ = Balances::make_free_balance_be(&source, 20_000_000);
        let merchants = vec![merchant.clone(), charlie.clone(), dave.clone(), eve.clone()];
        for who in merchants.iter() {
            let _ = Balances::make_free_balance_be(&who, 20_000_000);
            assert_ok!(Market::register(Origin::signed(who.clone()), 6_000_000));
        }

        assert_ok!(Market::place_storage_order(
            Origin::signed(source.clone()), cid.clone(),
            file_size, 0
        ));

        assert_eq!(Market::files(&cid).unwrap_or_default(), (
            FileInfo {
                file_size,
                expired_on: 0,
                calculated_at: 50,
                amount: 23400,
                prepaid: 0,
                reported_replica_count: 0,
                replicas: vec![]
            },
            UsedInfo {
                used_size: 0,
                reported_group_count: 0,
                groups: BTreeMap::new()
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

        assert_eq!(Market::files_size(), file_size as u128);

        assert_eq!(Market::files(&cid).unwrap_or_default(), (
            FileInfo {
                file_size,
                expired_on: 1303,
                calculated_at: 303,
                amount: 23400,
                prepaid: 0,
                reported_replica_count: 1,
                replicas: vec![Replica {
                    who: merchant.clone(),
                    valid_at: 303,
                    anchor: legal_pk.clone(),
                    is_reported: true
                }]
            },
            UsedInfo {
                used_size: file_size * 2,
                reported_group_count: 1,
                groups: BTreeMap::from_iter(vec![(legal_pk.clone(), true)].into_iter())
            })
        );

        run_to_block(503);
        <swork::ReportedInSlot>::insert(legal_pk.clone(), 0, true);
        Market::do_calculate_reward(&cid, System::block_number().try_into().unwrap());
        assert_eq!(Market::files(&cid).unwrap_or_default(), (
            FileInfo {
                file_size,
                expired_on: 1303,
                calculated_at: 503,
                amount: 18721,
                prepaid: 0,
                reported_replica_count: 1,
                replicas: vec![Replica {
                    who: merchant.clone(),
                    valid_at: 303,
                    anchor: legal_pk.clone(),
                    is_reported: true
                }]
            },
            UsedInfo {
                used_size: file_size * 2,
                reported_group_count: 1,
                groups: BTreeMap::from_iter(vec![(legal_pk.clone(), true)].into_iter())
            })
        );
        assert_eq!(Market::merchant_ledgers(&merchant), MerchantLedger {
            collateral: 6_000_000,
            reward: 4679
        });

        add_who_into_replica(&cid, file_size, charlie.clone(), legal_pk.clone(), None, None);

        assert_eq!(Market::files(&cid).unwrap_or_default(), (
            FileInfo {
                file_size,
                expired_on: 1303,
                calculated_at: 503,
                amount: 18721,
                prepaid: 0,
                reported_replica_count: 2,
                replicas: vec![
                    Replica {
                        who: merchant.clone(),
                        valid_at: 303,
                        anchor: legal_pk.clone(),
                        is_reported: true
                    },
                    Replica {
                        who: charlie.clone(),
                        valid_at: 503,
                        anchor: legal_pk.clone(),
                        is_reported: true
                    }]
            },
            UsedInfo {
                used_size: file_size * 2,
                reported_group_count: 2,
                groups: BTreeMap::from_iter(vec![(legal_pk.clone(), true)].into_iter())
            })
        );
        Market::delete_replica(&merchant, &cid, &legal_pk);
        Market::delete_replica(&charlie, &cid, &legal_pk);

        run_to_block(903);
        Market::do_calculate_reward(&cid, System::block_number().try_into().unwrap());
        // calculated_at should be updated
        assert_eq!(Market::files(&cid).unwrap_or_default(), (
            FileInfo {
                file_size,
                expired_on: 1303,
                calculated_at: 903,
                amount: 18721,
                prepaid: 0,
                reported_replica_count: 0,
                replicas: vec![]
            },
            UsedInfo {
                used_size: 0,
                reported_group_count: 0,
                groups: BTreeMap::from_iter(vec![].into_iter())
            })
        );

        run_to_block(1803);

        assert_ok!(Market::place_storage_order(
            Origin::signed(source), cid.clone(),
            file_size, 0
        ));
        assert_eq!(Market::files(&cid).unwrap_or_default(), (
            FileInfo {
                file_size,
                expired_on: 0,
                calculated_at: 1803,
                amount: 42121,
                prepaid: 0,
                reported_replica_count: 0,
                replicas: vec![]
            },
            UsedInfo {
                used_size: 0,
                reported_group_count: 0,
                groups: BTreeMap::from_iter(vec![].into_iter())
            })
        );
    });
}


#[test]
fn dynamic_used_size_should_work() {
    new_test_ext().execute_with(|| {
        // generate 50 blocks first
        run_to_block(50);

        let source = ALICE;
        let merchant = MERCHANT;

        let cid =
            hex::decode("5bb706320afc633bfb843108e492192b17d2b6b9d9ee0b795ee95417fe08b660").unwrap();
        let file_size = 134289408;
        let _ = Balances::make_free_balance_be(&source, 20_000_000);
        let merchants = vec![merchant.clone()];
        for who in merchants.iter() {
            let _ = Balances::make_free_balance_be(&who, 20_000_000);
            assert_ok!(Market::register(Origin::signed(who.clone()), 6_000_000));
        }

        assert_ok!(Market::place_storage_order(
            Origin::signed(source), cid.clone(),
            file_size, 0
        ));

        assert_eq!(Market::files(&cid).unwrap_or_default(), (
            FileInfo {
                file_size,
                expired_on: 0,
                calculated_at: 50,
                amount: 23400,
                prepaid: 0,
                reported_replica_count: 0,
                replicas: vec![]
            },
            UsedInfo {
                used_size: 0,
                reported_group_count: 0,
                groups: BTreeMap::new()
            })
        );

        run_to_block(303);

        let legal_wr_info = legal_work_report_with_added_files();
        let legal_pk = legal_wr_info.curr_pk.clone();

        for _ in 0..10 {
            add_who_into_replica(&cid, file_size, merchant.clone(), legal_pk.clone(), Some(303u32), None);
        }
        assert_eq!(Market::files(&cid).unwrap_or_default().1,
            UsedInfo {
                used_size: file_size * 2,
                reported_group_count: 10,
                groups: BTreeMap::from_iter(vec![(legal_pk.clone(), true)].into_iter())
            }
        );
        for _ in 0..10 {
            add_who_into_replica(&cid, file_size, merchant.clone(), legal_pk.clone(), Some(303u32), None);
        }
        assert_eq!(Market::files(&cid).unwrap_or_default().1,
           UsedInfo {
               used_size: file_size * 4,
               reported_group_count: 20,
               groups: BTreeMap::from_iter(vec![(legal_pk.clone(), true)].into_iter())
           }
        );
        for _ in 0..200 {
            add_who_into_replica(&cid, file_size, merchant.clone(), legal_pk.clone(), Some(303u32), None);
        }
        assert_eq!(Market::files(&cid).unwrap_or_default().1,
           UsedInfo {
               used_size: 0,
               reported_group_count: 220,
               groups: BTreeMap::from_iter(vec![(legal_pk.clone(), true)].into_iter())
           }
        );
        for _ in 0..140 {
            Market::delete_replica(&merchant, &cid, &legal_pk);
        }
        assert_eq!(Market::files(&cid).unwrap_or_default().1,
           UsedInfo {
               used_size: 0,
               reported_group_count: 219, // This would never happen in the real world.
               groups: BTreeMap::new()
           }
        );
    });
}

#[test]
fn delete_used_size_should_work() {
    new_test_ext().execute_with(|| {
        // generate 50 blocks first
        run_to_block(50);

        let source = ALICE;
        let merchant = MERCHANT;

        let cid =
            hex::decode("5bb706320afc633bfb843108e492192b17d2b6b9d9ee0b795ee95417fe08b660").unwrap();
        let file_size = 134289408;
        let _ = Balances::make_free_balance_be(&source, 20_000_000);
        let merchants = vec![merchant.clone()];
        for who in merchants.iter() {
            let _ = Balances::make_free_balance_be(&who, 20_000_000);
            assert_ok!(Market::register(Origin::signed(who.clone()), 6_000_000));
        }

        assert_ok!(Market::place_storage_order(
            Origin::signed(source), cid.clone(),
            file_size, 0
        ));

        assert_eq!(Market::files(&cid).unwrap_or_default(), (
            FileInfo {
                file_size,
                expired_on: 0,
                calculated_at: 50,
                amount: 23400,
                prepaid: 0,
                reported_replica_count: 0,
                replicas: vec![]
            },
            UsedInfo {
                used_size: 0,
                reported_group_count: 0,
                groups: BTreeMap::new()
            })
        );

        run_to_block(303);

        let mut expected_groups = BTreeMap::new();
        for i in 10..30 {
            let key = hex::decode(i.to_string()).unwrap();
            add_who_into_replica(&cid, file_size, merchant.clone(), key.clone(), Some(303u32), None);
            <swork::ReportedInSlot>::insert(key.clone(), 0, true);
            expected_groups.insert(key.clone(), true);
        }
        assert_eq!(Market::files(&cid).unwrap_or_default().1,
           UsedInfo {
               used_size: file_size * 4,
               reported_group_count: 20,
               groups: expected_groups.clone()
           }
        );
        Market::delete_replica(&merchant, &cid, &hex::decode("10").unwrap());
        expected_groups.remove(&hex::decode("10").unwrap());
        assert_eq!(Market::files(&cid).unwrap_or_default().1,
           UsedInfo {
               used_size: file_size * 4,
               reported_group_count: 19,
               groups: expected_groups.clone()
           }
        );

        for i in 11..30 {
            let key = hex::decode(i.to_string()).unwrap();
            <swork::ReportedInSlot>::insert(key.clone(), 300, true);
            expected_groups.insert(key.clone(), true);
        }
        Market::delete_replica(&merchant, &cid, &hex::decode("21").unwrap()); // delete 21. 21 won't be deleted twice.
        expected_groups.remove(&hex::decode("21").unwrap());
        assert_eq!(Market::files(&cid).unwrap_or_default().1,
           UsedInfo {
               used_size: file_size * 4,
               reported_group_count: 18, // this should be nine instead of eight.
               groups: expected_groups.clone()
           }
        );
    });
}


#[test]
fn files_size_should_not_be_decreased_twice() {
    new_test_ext().execute_with(|| {
        // generate 50 blocks first
        run_to_block(50);

        let cid =
            "QmdwgqZy1MZBfWPi7GcxVsYgJEtmvHg6rsLzbCej3tf3oF".as_bytes().to_vec();
        let file_size = 134289408; // should less than merchant
        let source = ALICE;
        let merchant = BOB;
        let charlie = CHARLIE;
        let dave = DAVE;

        let staking_pot = Market::staking_pot();
        let reserved_pot = Market::reserved_pot();
        assert_eq!(Balances::free_balance(&staking_pot), 0);
        assert_eq!(Balances::free_balance(&reserved_pot), 0);
        let _ = Balances::make_free_balance_be(&source, 20_000_000);
        let merchants = vec![merchant.clone(), charlie.clone(), dave.clone()];
        for who in merchants.iter() {
            let _ = Balances::make_free_balance_be(&who, 20_000_000);
            assert_ok!(Market::register(Origin::signed(who.clone()), 6_000_000));
        }

        assert_ok!(Market::place_storage_order(
            Origin::signed(source), cid.clone(),
            file_size, 0
        ));

        assert_eq!(Market::files(&cid).unwrap_or_default(), (
            FileInfo {
                file_size,
                expired_on: 0,
                calculated_at: 50,
                amount: 23400,
                prepaid: 0,
                reported_replica_count: 0,
                replicas: vec![]
            },
            UsedInfo {
                used_size: 0,
                reported_group_count: 0,
                groups: BTreeMap::new()
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

        assert_eq!(Market::files_size(), file_size as u128);
        run_to_block(503);
        add_who_into_replica(&cid, file_size, charlie.clone(), legal_pk.clone(), None, None);
        run_to_block(603);
        add_who_into_replica(&cid, file_size, dave.clone(), hex::decode("11").unwrap(), None, None);
        assert_eq!(Market::files_size(), (file_size * 3) as u128);
        run_to_block(703);
        <swork::ReportedInSlot>::insert(legal_pk.clone(), 300, true);
        Market::do_calculate_reward(&cid, System::block_number().try_into().unwrap());
        assert_eq!(Market::files(&cid).unwrap_or_default(), (
            FileInfo {
                file_size,
                expired_on: 1303,
                calculated_at: 703,
                amount: 17162,
                prepaid: 0,
                reported_replica_count: 2,
                replicas: vec![
                    Replica {
                        who: merchant.clone(),
                        valid_at: 303,
                        anchor: legal_pk.clone(),
                        is_reported: true
                    },
                    Replica {
                        who: charlie.clone(),
                        valid_at: 503,
                        anchor: legal_pk.clone(),
                        is_reported: true
                    },
                    Replica {
                        who: dave.clone(),
                        valid_at: 703, // did't report. change it to curr bn
                        anchor: hex::decode("11").unwrap(),
                        is_reported: false
                    }]
            },
            UsedInfo {
                used_size: file_size * 2,
                reported_group_count: 1,
                groups: BTreeMap::from_iter(vec![(legal_pk.clone(), true), (hex::decode("11").unwrap(), false)].into_iter())
            })
        );
        assert_eq!(Market::files_size(), file_size as u128);
        Market::delete_replica(&dave, &cid, &hex::decode("11").unwrap()); // delete 11. 11 won't be deleted twice.
        assert_eq!(Market::files(&cid).unwrap_or_default(), (
            FileInfo {
                file_size,
                expired_on: 1303,
                calculated_at: 703,
                amount: 17162,
                prepaid: 0,
                reported_replica_count: 2,
                replicas: vec![
                    Replica {
                        who: merchant.clone(),
                        valid_at: 303,
                        anchor: legal_pk.clone(),
                        is_reported: true
                    },
                    Replica {
                        who: charlie.clone(),
                        valid_at: 503,
                        anchor: legal_pk.clone(),
                        is_reported: true
                    }
                    ]
            },
            UsedInfo {
                used_size: file_size * 2,
                reported_group_count: 1,
                groups: BTreeMap::from_iter(vec![(legal_pk.clone(), true)].into_iter())
            })
        );
        assert_eq!(Market::files_size(), file_size as u128);
    });
}

#[test]
fn clear_same_file_in_trash_should_work() {
    new_test_ext().execute_with(|| {
        // generate 50 blocks first
        run_to_block(50);

        let cid =
            "QmdwgqZy1MZBfWPi7GcxVsYgJEtmvHg6rsLzbCej3tf3oF".as_bytes().to_vec();
        let file_size = 134289408; // should less than merchant
        let source = ALICE;
        let merchant = BOB;
        let charlie = CHARLIE;
        let dave = DAVE;

        let storage_pot = Market::storage_pot();
        assert_eq!(Balances::free_balance(&storage_pot), 0);
        let _ = Balances::make_free_balance_be(&storage_pot, 1);

        let _ = Balances::make_free_balance_be(&source, 20_000_000);
        let merchants = vec![merchant.clone(), charlie.clone(), dave.clone()];
        for who in merchants.iter() {
            let _ = Balances::make_free_balance_be(&who, 20_000_000);
            assert_ok!(Market::register(Origin::signed(who.clone()), 6_000_000));
        }

        assert_ok!(Market::place_storage_order(
            Origin::signed(source.clone()), cid.clone(),
            file_size, 0
        ));

        assert_eq!(Market::files(&cid).unwrap_or_default(), (
            FileInfo {
                file_size,
                expired_on: 0,
                calculated_at: 50,
                amount: 23400,
                prepaid: 0,
                reported_replica_count: 0,
                replicas: vec![]
            },
            UsedInfo {
                used_size: 0,
                reported_group_count: 0,
                groups: BTreeMap::new()
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

        run_to_block(503);
        add_who_into_replica(&cid, file_size, charlie.clone(), legal_pk.clone(), None, None);
        run_to_block(603);
        add_who_into_replica(&cid, file_size, dave.clone(), hex::decode("11").unwrap(), None, None);
        run_to_block(1803);
        <swork::ReportedInSlot>::insert(legal_pk.clone(), 1500, true);
        <swork::ReportedInSlot>::insert(hex::decode("11").unwrap(), 1500, true);
        assert_ok!(Market::calculate_reward(Origin::signed(merchant.clone()), cid.clone()));

        // Prepare a file in the trash
        assert_eq!(Market::files(&cid).is_none(), true);
        assert_eq!(Market::used_trash_i(&cid).unwrap_or_default(), (
            UsedInfo {
                used_size: file_size * 2,
                reported_group_count: 1,
                groups: BTreeMap::from_iter(vec![(legal_pk.clone(), true), (hex::decode("11").unwrap(), true)].into_iter())
            })
        );
        assert_eq!(Market::used_trash_mapping_i(legal_pk.clone()), file_size * 2);
        assert_eq!(Market::used_trash_mapping_i(hex::decode("11").unwrap()), file_size * 2);
        assert_eq!(Market::used_trash_size_i(), 1);

        // place a same storage order
        assert_ok!(Market::place_storage_order(
            Origin::signed(source), cid.clone(),
            file_size, 0
        ));

        assert_eq!(Market::files(&cid).unwrap_or_default(), (
            FileInfo {
                file_size,
                expired_on: 0,
                calculated_at: 1803,
                amount: 23400,
                prepaid: 0,
                reported_replica_count: 0,
                replicas: vec![]
            },
            UsedInfo {
                used_size: 0,
                reported_group_count: 0,
                groups: BTreeMap::new()
            })
        );
        // trash has been cleared.
        assert_eq!(Market::used_trash_i(&cid).is_none(), true);
        assert_eq!(Market::used_trash_mapping_i(legal_pk.clone()), 0);
        assert_eq!(Market::used_trash_mapping_i(hex::decode("11").unwrap()), 0);
        assert_eq!(Market::used_trash_size_i(), 0);
    });
}


#[test]
fn reward_liquidator_should_work() {
    new_test_ext().execute_with(|| {
        // generate 50 blocks first
        run_to_block(50);


        let cid =
            "QmdwgqZy1MZBfWPi7GcxVsYgJEtmvHg6rsLzbCej3tf3oF".as_bytes().to_vec();
        let file_size = 134289408; // should less than merchant
        let source = ALICE;
        let merchant = BOB;
        let charlie = CHARLIE;

        let storage_pot = Market::storage_pot();
        assert_eq!(Balances::free_balance(&storage_pot), 0);
        let _ = Balances::make_free_balance_be(&storage_pot, 1);

        let _ = Balances::make_free_balance_be(&source, 20_000_000);
        let merchants = vec![merchant.clone()];
        for who in merchants.iter() {
            let _ = Balances::make_free_balance_be(&who, 20_000_000);
            assert_ok!(Market::register(Origin::signed(who.clone()), 6_000_000));
        }

        assert_ok!(Market::calculate_reward(Origin::signed(charlie.clone()), cid.clone()));

        assert_ok!(Market::place_storage_order(
            Origin::signed(source.clone()), cid.clone(),
            file_size, 0
        ));

        assert_noop!(
            Market::calculate_reward(Origin::signed(charlie.clone()), cid.clone()),
            DispatchError::Module {
                index: 3,
                error: 9,
                message: Some("NotInRewardPeriod")
        });

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

        assert_noop!(
            Market::calculate_reward(Origin::signed(charlie.clone()), cid.clone()),
            DispatchError::Module {
                index: 3,
                error: 9,
                message: Some("NotInRewardPeriod")
        });

        run_to_block(2503);
        // 20% would be rewarded to liquidator charlie
        assert_ok!(Market::calculate_reward(Origin::signed(charlie.clone()), cid.clone()));
        assert_eq!(Balances::free_balance(&charlie), 4680);
        assert_eq!(Market::files(&cid).is_none(), true);
        assert_eq!(Market::used_trash_i(&cid).is_some(), true);

        assert_ok!(Market::place_storage_order(
            Origin::signed(source.clone()), cid.clone(),
            file_size, 0
        ));

        add_who_into_replica(&cid, file_size, merchant.clone(), legal_pk.clone(), None, None);

        run_to_block(4000); // 3503 - 4503 => no reward to liquidator charlie
        assert_ok!(Market::calculate_reward(Origin::signed(charlie.clone()), cid.clone()));
        assert_eq!(Balances::free_balance(&charlie), 4680);

        assert_ok!(Market::place_storage_order(
            Origin::signed(source.clone()), cid.clone(),
            file_size, 0
        ));

        add_who_into_replica(&cid, file_size, merchant.clone(), legal_pk.clone(), None, None);

        run_to_block(8000); // expired_on 6000 => all reward to liquidator charlie
        assert_ok!(Market::calculate_reward(Origin::signed(charlie.clone()), cid.clone()));
        assert_eq!(Balances::free_balance(&charlie), 28080);
    });
}

#[test]
fn reward_merchant_should_work() {
    new_test_ext().execute_with(|| {
        // generate 50 blocks first
        run_to_block(50);

        let merchant = MERCHANT;
        let alice = ALICE;
        let storage_pot = Market::storage_pot();
        let _ = Balances::make_free_balance_be(&storage_pot, 121);

        <self::MerchantLedgers<Test>>::insert(&merchant, MerchantLedger {
            collateral: 180,
            reward: 120
        });

        assert_ok!(Market::reward_merchant(Origin::signed(merchant.clone())));
        assert_eq!(Market::merchant_ledgers(&merchant), MerchantLedger {
            collateral: 180,
            reward: 0
        });
        assert_eq!(Balances::free_balance(&merchant), 120);
        assert_eq!(Balances::free_balance(&storage_pot), 1);
        assert_noop!(
            Market::reward_merchant(
                Origin::signed(merchant)
            ),
            DispatchError::Module {
                index: 3,
                error: 10,
                message: Some("NotEnoughReward")
            }
        );

        assert_noop!(
            Market::reward_merchant(
                Origin::signed(alice)
            ),
            DispatchError::Module {
                index: 3,
                error: 3,
                message: Some("NotRegister")
            }
        );
    });
}

#[test]
fn set_global_switch_should_work() {
    new_test_ext().execute_with(|| {
        // generate 50 blocks first
        run_to_block(50);

        let source = ALICE;
        let merchant = MERCHANT;

        let cid =
            hex::decode("4e2883ddcbc77cf19979770d756fd332d0c8f815f9de646636169e460e6af6ff").unwrap();
        let file_size = 100; // should less than
        let staking_pot = Market::staking_pot();
        assert_eq!(Balances::free_balance(&staking_pot), 0);
        let _ = Balances::make_free_balance_be(&source, 20000);
        let _ = Balances::make_free_balance_be(&merchant, 200);

        assert_ok!(Market::register(Origin::signed(merchant.clone()), 60));

        assert_ok!(Market::place_storage_order(
            Origin::signed(source.clone()), cid.clone(),
            file_size, 0
        ));
        assert_ok!(Market::set_market_switch(
            Origin::root(),
            false
        ));
        assert_noop!(Market::place_storage_order(
            Origin::signed(source), cid.clone(),
            file_size, 0
        ),
        DispatchError::Module {
            index: 3,
            error: 12,
            message: Some("PlaceOrderNotAvailable")
        });
    });
}

#[test]
fn renew_file_should_work() {
    new_test_ext().execute_with(|| {
        // generate 50 blocks first
        run_to_block(50);


        let cid =
            "QmdwgqZy1MZBfWPi7GcxVsYgJEtmvHg6rsLzbCej3tf3oF".as_bytes().to_vec();
        let file_size = 134289408; // should less than merchant
        let source = ALICE;
        let merchant = BOB;
        let charlie = CHARLIE;

        let storage_pot = Market::storage_pot();
        let reserved_pot = Market::reserved_pot();
        assert_eq!(Balances::free_balance(&storage_pot), 0);
        assert_eq!(Balances::free_balance(&reserved_pot), 0);
        let _ = Balances::make_free_balance_be(&storage_pot, 1);

        let _ = Balances::make_free_balance_be(&source, 20_000_000);
        let merchants = vec![merchant.clone()];
        for who in merchants.iter() {
            let _ = Balances::make_free_balance_be(&who, 20_000_000);
            assert_ok!(Market::register(Origin::signed(who.clone()), 6_000_000));
        }

        assert_noop!(
            Market::add_prepaid(Origin::signed(source.clone()), cid.clone(), 400_000),
            DispatchError::Module {
                index: 3,
                error: 8,
                message: Some("FileNotExist")
        });

        assert_ok!(Market::place_storage_order(
            Origin::signed(source.clone()), cid.clone(),
            file_size, 0
        ));
        assert_eq!(Balances::free_balance(&reserved_pot), 13000);
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

        assert_ok!(Market::add_prepaid(Origin::signed(source.clone()), cid.clone(), 400_000));

        assert_eq!(Market::files(&cid).unwrap_or_default(), (
            FileInfo {
                file_size,
                expired_on: 1303,
                calculated_at: 303,
                amount: 23400,
                prepaid: 400_000,
                reported_replica_count: 1,
                replicas: vec![Replica {
                    who: merchant.clone(),
                    valid_at: 303,
                    anchor: legal_pk.clone(),
                    is_reported: true
                }]
            },
            UsedInfo {
                used_size: file_size * 2,
                reported_group_count: 1,
                groups: BTreeMap::from_iter(vec![(legal_pk.clone(), true)].into_iter())
            })
        );

        run_to_block(2503);
        // 20% would be rewarded to liquidator charlie
        assert_ok!(Market::calculate_reward(Origin::signed(charlie.clone()), cid.clone()));

        assert_eq!(Market::files(&cid).unwrap_or_default(), (
            FileInfo {
                file_size,
                expired_on: 2303,
                calculated_at: 1303,
                amount: 42120, // 23400 * 0.8 + 23400
                prepaid: 263_500,
                reported_replica_count: 0,
                replicas: vec![Replica {
                    who: merchant.clone(),
                    valid_at: 2503,
                    anchor: legal_pk.clone(),
                    is_reported: false
                }]
            },
            UsedInfo {
                used_size: 0,
                reported_group_count: 0,
                groups: BTreeMap::from_iter(vec![(legal_pk.clone(), false)].into_iter())
            })
        );


        assert_eq!(Balances::free_balance(&charlie), 11180);
        assert_eq!(Market::used_trash_i(&cid).is_none(), true);
        assert_eq!(Balances::free_balance(&reserved_pot), 26000);

        run_to_block(8000); // expired_on 2303 => all reward to liquidator charlie
        assert_ok!(Market::calculate_reward(Origin::signed(charlie.clone()), cid.clone()));
        assert_eq!(Balances::free_balance(&charlie), 59800); // 42120 + 11180 + 6500
        assert_eq!(Market::files(&cid).unwrap_or_default(), (
            FileInfo {
                file_size,
                expired_on: 3303,
                calculated_at: 2303,
                amount: 23400,
                prepaid: 127000,
                reported_replica_count: 0,
                replicas: vec![Replica {
                    who: merchant.clone(),
                    valid_at: 8000,
                    anchor: legal_pk.clone(),
                    is_reported: false
                }]
            },
            UsedInfo {
                used_size: 0,
                reported_group_count: 0,
                groups: BTreeMap::from_iter(vec![(legal_pk.clone(), false)].into_iter())
            })
        );
        assert_eq!(Balances::free_balance(&reserved_pot), 39000);
        assert_eq!(Market::used_trash_i(&cid).is_none(), true);
        run_to_block(9000); // expired_on 3303 => all reward to liquidator charlie
        assert_ok!(Market::calculate_reward(Origin::signed(charlie.clone()), cid.clone()));
        assert_eq!(Balances::free_balance(&charlie), 83200); // 42120 + 11180 + 6500 + 23400
        assert_eq!(Market::used_trash_i(&cid).is_some(), true);
        assert_eq!(Market::files(&cid).is_none(), true);
        assert_eq!(Balances::free_balance(&reserved_pot), 166000); // 39000 + 127000
    });
}

#[test]
fn change_base_fee_should_work() {
    new_test_ext().execute_with(|| {
        // generate 50 blocks first
        run_to_block(50);

        let source = ALICE;
        let merchant = MERCHANT;

        let cid =
            "QmdwgqZy1MZBfWPi7GcxVsYgJEtmvHg6rsLzbCej3tf3oF".as_bytes().to_vec();
        let file_size = 134289408; // should less than
        let reserved_pot = Market::reserved_pot();
        let staking_pot = Market::staking_pot();
        let storage_pot = Market::storage_pot();
        assert_eq!(Balances::free_balance(&staking_pot), 0);
        let _ = Balances::make_free_balance_be(&source, 2_000_000);
        let _ = Balances::make_free_balance_be(&merchant, 200);

        assert_ok!(Market::register(Origin::signed(merchant.clone()), 60));

        // Change base fee to 10000
        assert_ok!(Market::set_base_fee(Origin::root(), 50000));

        assert_ok!(Market::place_storage_order(
            Origin::signed(source.clone()), cid.clone(),
            file_size, 0
        ));

        assert_eq!(Market::files(&cid).unwrap_or_default(), (
            FileInfo {
                file_size,
                expired_on: 0,
                calculated_at: 50,
                amount: 32_220, // ( 50000 + 1000 * 129 + 0 ) * 0.18
                prepaid: 0,
                reported_replica_count: 0,
                replicas: vec![]
            },
            UsedInfo {
                used_size: 0,
                reported_group_count: 0,
                groups: BTreeMap::new()
            })
        );
        assert_eq!(Balances::free_balance(reserved_pot), 17900);
        assert_eq!(Balances::free_balance(staking_pot), 128880);
        assert_eq!(Balances::free_balance(storage_pot), 32220);

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

        assert_ok!(Market::add_prepaid(Origin::signed(source.clone()), cid.clone(), 200_000));

        assert_eq!(Market::files(&cid).unwrap_or_default(), (
            FileInfo {
                file_size,
                expired_on: 1303,
                calculated_at: 303,
                amount: 32_220,
                prepaid: 200_000,
                reported_replica_count: 1,
                replicas: vec![Replica {
                    who: merchant.clone(),
                    valid_at: 303,
                    anchor: legal_pk.clone(),
                    is_reported: true
                }]
            },
            UsedInfo {
                used_size: file_size * 2,
                reported_group_count: 1,
                groups: BTreeMap::from_iter(vec![(legal_pk.clone(), true)].into_iter())
            })
        );

        run_to_block(2503);
        // 20% would be rewarded to liquidator charlie
        assert_ok!(Market::calculate_reward(Origin::signed(source.clone()), cid.clone()));

        assert_eq!(Market::files(&cid).unwrap_or_default(), (
            FileInfo {
                file_size,
                expired_on: 2303,
                calculated_at: 1303,
                amount: 57996, // 32_220 * 0.8 + 32_220
                prepaid: 12050, // 200000 -187950
                reported_replica_count: 0,
                replicas: vec![Replica {
                    who: merchant.clone(),
                    valid_at: 2503,
                    anchor: legal_pk.clone(),
                    is_reported: false
                }]
            },
            UsedInfo {
                used_size: 0,
                reported_group_count: 0,
                groups: BTreeMap::from_iter(vec![(legal_pk.clone(), false)].into_iter())
            })
        );
    });
}

#[test]
fn storage_pot_should_be_balanced() {
    new_test_ext().execute_with(|| {
        // generate 50 blocks first
        run_to_block(50);


        let cid =
            "QmdwgqZy1MZBfWPi7GcxVsYgJEtmvHg6rsLzbCej3tf3oF".as_bytes().to_vec();
        let file_size = 134289408; // should less than merchant
        let source = ALICE;
        let merchant = BOB;
        let charlie = CHARLIE;

        let storage_pot = Market::storage_pot();
        let reserved_pot = Market::reserved_pot();
        let _ = Balances::make_free_balance_be(&storage_pot, 1);
        assert_eq!(Balances::free_balance(&storage_pot), 1);
        assert_eq!(Balances::free_balance(&reserved_pot), 0);

        let _ = Balances::make_free_balance_be(&source, 20_000_000);
        let merchants = vec![merchant.clone()];
        for who in merchants.iter() {
            let _ = Balances::make_free_balance_be(&who, 20_000_000);
            assert_ok!(Market::register(Origin::signed(who.clone()), 6_000_000));
        }

        assert_ok!(Market::place_storage_order(
            Origin::signed(source.clone()), cid.clone(),
            file_size, 0
        ));
        assert_eq!(Balances::free_balance(&storage_pot), 23401);
        assert_eq!(Balances::free_balance(&reserved_pot), 13000);
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

        assert_ok!(Market::add_prepaid(Origin::signed(source.clone()), cid.clone(), 400_000));
        assert_eq!(Balances::free_balance(&storage_pot), 423401);

        run_to_block(2503);
        // 20% would be rewarded to liquidator charlie
        assert_ok!(Market::calculate_reward(Origin::signed(charlie.clone()), cid.clone()));
        assert_eq!(Balances::free_balance(&storage_pot), 305621);

        run_to_block(8000); // expired_on 6000 => all reward to liquidator charlie
        assert_ok!(Market::calculate_reward(Origin::signed(charlie.clone()), cid.clone()));
        assert_eq!(Balances::free_balance(&storage_pot), 150401);

        run_to_block(9000);
        assert_ok!(Market::calculate_reward(Origin::signed(charlie.clone()), cid.clone()));
        assert_eq!(Balances::free_balance(&storage_pot), 1);
        assert_eq!(Balances::free_balance(&reserved_pot), 166000); // 39000 + 127000
    });
}
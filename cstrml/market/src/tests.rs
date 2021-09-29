// Copyright (C) 2019-2021 Crust Network Technologies Ltd.
// This file is part of Crust.

use super::*;

use crate::mock::*;
use frame_support::{
    assert_noop, assert_ok,
    dispatch::DispatchError,
    traits::OnInitialize
};
use hex;
use swork::Identity;
use sp_std::collections::btree_map::BTreeMap;

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
        let _ = Balances::make_free_balance_be(&source, 4000);
        let _ = Balances::make_free_balance_be(&merchant, 200);

        mock_bond_owner(&merchant, &merchant);
        add_collateral(&merchant, 60);

        <FileKeysCountFee<Test>>::put(1000);
        assert_ok!(Market::place_storage_order(
            Origin::signed(source), cid.clone(),
            file_size, 0, vec![]
        ));
        assert_eq!(Market::files(&cid).unwrap_or_default(), FileInfo {
                file_size,
                spower: 0,
                expired_at: 0,
                calculated_at: 50,
                amount: 360, // ( 1000 * 1 + 0 + 1000 ) * 0.18
                prepaid: 0,
                reported_replica_count: 0,
                replicas: vec![]
            }
        );
        assert_eq!(Balances::free_balance(reserved_pot), 1200);
        assert_eq!(Balances::free_balance(staking_pot), 1440);
        assert_eq!(Balances::free_balance(storage_pot), 360);
        assert_eq!(Market::orders_count(), 1);
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

        mock_bond_owner(&merchant, &merchant);
        add_collateral(&merchant, 60);

        assert_noop!(Market::place_storage_order(
            Origin::signed(source), cid.clone(),
            file_size, 0, vec![]
        ),
        DispatchError::Module {
            index: 3,
            error: 4,
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

        mock_bond_owner(&merchant, &merchant);
        add_collateral(&merchant, 6_000_000);

        // 1. New storage order
        assert_ok!(Market::place_storage_order(
            Origin::signed(source.clone()), cid.clone(),
            file_size, 0, vec![]
        ));

        assert_eq!(Market::files(&cid).unwrap_or_default(),
            FileInfo {
                file_size,
                spower: 0,
                expired_at: 0,
                calculated_at: 50,
                amount: 23220, // ( 1000 * 129 + 0 ) * 0.18
                prepaid: 0,
                reported_replica_count: 0,
                replicas: vec![]
            }
        );
        assert_eq!(Balances::free_balance(&staking_pot), 92880);
        assert_eq!(Balances::free_balance(&storage_pot), 23220);

        run_to_block(250);

        // 2. Add amount for sOrder not begin should work
        assert_ok!(Market::place_storage_order(
            Origin::signed(source.clone()), cid.clone(),
            file_size, 0, vec![]
        ));

        assert_eq!(Market::files(&cid).unwrap_or_default(),
            FileInfo {
                file_size,
                spower: 0,
                expired_at: 0,
                calculated_at: 50,
                amount: 46440, // ( 1000 * 129 + 0 ) * 0.18 * 2
                prepaid: 0,
                reported_replica_count: 0,
                replicas: vec![]
            }
        );
        assert_eq!(Balances::free_balance(&staking_pot), 185760);
        assert_eq!(Balances::free_balance(&storage_pot), 46440);

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
        update_spower_info();
        assert_eq!(Market::files(&cid).unwrap_or_default(),
            FileInfo {
                file_size,
                spower: Market::calculate_spower(file_size, 1),
                expired_at: 1400,
                calculated_at: 400,
                amount: 46440, // ( 1000 * 129 + 0 ) * 0.18 * 2
                prepaid: 0,
                reported_replica_count: 1,
                replicas: vec![Replica {
                    who: merchant.clone(),
                    valid_at: 303,
                    anchor: legal_pk.clone(),
                    is_reported: true,
                    created_at: Some(303)
                }]
            }
        );

        // Calculate reward should work
        run_to_block(500);
        <swork::ReportedInSlot>::insert(legal_pk.clone(), 0, true);
        Market::do_calculate_reward(&cid, System::block_number().try_into().unwrap());
        assert_eq!(Market::files(&cid).unwrap_or_default(),
            FileInfo {
                file_size,
                spower: Market::calculate_spower(file_size, 1),
                expired_at: 1400,
                calculated_at: 500,
                amount: 41797,
                prepaid: 0,
                reported_replica_count: 1,
                replicas: vec![Replica {
                    who: merchant.clone(),
                    valid_at: 303,
                    anchor: legal_pk.clone(),
                    is_reported: true,
                    created_at: Some(303)
                }]
            }
        );

        // 3. Extend duration should work
        run_to_block(600);
        assert_ok!(Market::place_storage_order(
            Origin::signed(source.clone()), cid.clone(),
            file_size, 0, vec![]
        ));

        assert_eq!(Market::files(&cid).unwrap_or_default(),
            FileInfo {
                file_size,
                spower: Market::calculate_spower(file_size, 1),
                expired_at: 1600,
                calculated_at: 600,
                amount: 60374,
                prepaid: 0,
                reported_replica_count: 1,
                replicas: vec![Replica {
                    who: merchant.clone(),
                    valid_at: 303,
                    anchor: legal_pk.clone(),
                    is_reported: true,
                    created_at: Some(303)
                }]
            }
        );

        // 4. Extend replicas should work
        run_to_block(800);
        assert_ok!(Market::place_storage_order(
            Origin::signed(source.clone()), cid.clone(),
            file_size, 200, vec![]
        ));

        assert_eq!(Market::files(&cid).unwrap_or_default(),
            FileInfo {
                file_size,
                spower: Market::calculate_spower(file_size, 1),
                expired_at: 1800,
                calculated_at: 800,
                amount: 71720,
                prepaid: 0,
                reported_replica_count: 1,
                replicas: vec![Replica {
                    who: merchant.clone(),
                    valid_at: 303,
                    anchor: legal_pk.clone(),
                    is_reported: true,
                    created_at: Some(303)
                }]
            }
        );
        assert_eq!(Market::orders_count(), 4);
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

        mock_bond_owner(&merchant, &merchant);
        add_collateral(&merchant, 6_000_000);

        // 1. New storage order
        assert_ok!(Market::place_storage_order(
            Origin::signed(source.clone()), cid.clone(),
            file_size, 0, vec![]
        ));

        assert_eq!(Market::files(&cid).unwrap_or_default(),
            FileInfo {
                file_size,
                spower: 0,
                expired_at: 0,
                calculated_at: 50,
                amount: 23220, // ( 1000 * 129 + 0 ) * 0.18
                prepaid: 0,
                reported_replica_count: 0,
                replicas: vec![]
            }
        );
        assert_eq!(Balances::free_balance(&staking_pot), 92880);
        assert_eq!(Balances::free_balance(&storage_pot), 23220);
        assert_eq!(Balances::free_balance(&reserved_pot), 13900);

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
        update_spower_info();
        assert_eq!(Market::files(&cid).unwrap_or_default(),
            FileInfo {
                file_size,
                spower: Market::calculate_spower(file_size, 1),
                expired_at: 1303,
                calculated_at: 303,
                amount: 23220, // ( 1000 * 129 + 0 ) * 0.18
                prepaid: 0,
                reported_replica_count: 1,
                replicas: vec![Replica {
                    who: merchant.clone(),
                    valid_at: 303,
                    anchor: legal_pk.clone(),
                    is_reported: true,
                    created_at: Some(303)
                }]
            }
        );

        // 3. Go along with some time, and get reward
        run_to_block(606);
        <swork::ReportedInSlot>::insert(legal_pk.clone(), 300, true);
        Market::do_calculate_reward(&cid, System::block_number().try_into().unwrap());
        assert_eq!(Market::files(&cid).unwrap_or_default(),
            FileInfo {
                file_size,
                spower: Market::calculate_spower(file_size, 1),
                expired_at: 1303,
                calculated_at: 606,
                amount: 16185,
                prepaid: 0,
                reported_replica_count: 1,
                replicas: vec![Replica {
                    who: merchant.clone(),
                    valid_at: 303,
                    anchor: legal_pk.clone(),
                    is_reported: true,
                    created_at: Some(303)
                }]
            }
        );
        assert_eq!(merchant_ledgers(&merchant), MockMerchantLedger {
            collateral: 6_000_000,
            reward: 7035
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

        mock_bond_owner(&merchant, &merchant);
        add_collateral(&merchant, 60_000);

        assert_ok!(Market::place_storage_order(
            Origin::signed(source), cid.clone(),
            file_size, 0, vec![]
        ));

        assert_eq!(Market::files(&cid).unwrap_or_default(),
            FileInfo {
                file_size,
                spower: 0,
                expired_at: 0,
                calculated_at: 50,
                amount: 23220, // ( 1000 + 1000 * 1 + 0 ) * 0.2
                prepaid: 0,
                reported_replica_count: 0,
                replicas: vec![]
            }
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
        update_spower_info();
        assert_eq!(Market::files(&cid).unwrap_or_default(),
            FileInfo {
                file_size,
                spower: Market::calculate_spower(file_size, 1),
                expired_at: 1303,
                calculated_at: 303,
                amount: 23220,
                prepaid: 0,
                reported_replica_count: 1,
                replicas: vec![Replica {
                    who: merchant.clone(),
                    valid_at: 303,
                    anchor: legal_pk.clone(),
                    is_reported: true,
                    created_at: Some(303)
                }]
            }
        );

        run_to_block(603);
        <swork::ReportedInSlot>::insert(legal_pk.clone(), 300, true);
        Market::do_calculate_reward(&cid, System::block_number().try_into().unwrap());
        assert_eq!(Market::files(&cid).unwrap_or_default(),
            FileInfo {
                file_size,
                spower: Market::calculate_spower(file_size, 1),
                expired_at: 1303,
                calculated_at: 603,
                amount: 23220,
                prepaid: 0,
                reported_replica_count: 1,
                replicas: vec![Replica {
                    who: merchant.clone(),
                    valid_at: 303,
                    anchor: legal_pk.clone(),
                    is_reported: true,
                    created_at: Some(303)
                }]
            }
        );

        // collateral is 6965 * 10 < 60000 reward
        assert_eq!(merchant_ledgers(&merchant), MockMerchantLedger {
            collateral: 60_000,
            reward: 0
        });

        run_to_block(903);
        <swork::ReportedInSlot>::insert(legal_pk.clone(), 600, true);
        add_collateral(&merchant, 6_000_000);
        Market::do_calculate_reward(&cid, System::block_number().try_into().unwrap());
        assert_eq!(Market::files(&cid).unwrap_or_default(),
            FileInfo {
                file_size,
                spower: Market::calculate_spower(file_size, 1),
                expired_at: 1303,
                calculated_at: 903,
                amount: 13270,
                prepaid: 0,
                reported_replica_count: 1,
                replicas: vec![Replica {
                    who: merchant.clone(),
                    valid_at: 303,
                    anchor: legal_pk.clone(),
                    is_reported: true,
                    created_at: Some(303)
                }]
            }
        );

        assert_eq!(merchant_ledgers(&merchant), MockMerchantLedger {
            collateral: 6_000_000,
            reward: 9950
        });
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
            mock_bond_owner(&who, &who);
            add_collateral(who, 6_000_000);
        }

        assert_ok!(Market::place_storage_order(
            Origin::signed(source), cid.clone(),
            file_size, 0, vec![]
        ));

        assert_eq!(Market::files(&cid).unwrap_or_default(),
            FileInfo {
                file_size,
                spower: 0,
                expired_at: 0,
                calculated_at: 50,
                amount: 23220,
                prepaid: 0,
                reported_replica_count: 0,
                replicas: vec![]
            }
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

        update_spower_info();
        assert_eq!(Market::files(&cid).unwrap_or_default(),
            FileInfo {
                file_size,
                spower: Market::calculate_spower(file_size, 1),
                expired_at: 1303,
                calculated_at: 303,
                amount: 23220,
                prepaid: 0,
                reported_replica_count: 1,
                replicas: vec![Replica {
                    who: merchant.clone(),
                    valid_at: 303,
                    anchor: legal_pk.clone(),
                    is_reported: true,
                    created_at: Some(303)
                }]
            }
        );

        run_to_block(503);
        <swork::ReportedInSlot>::insert(legal_pk.clone(), 0, true);
        Market::do_calculate_reward(&cid, System::block_number().try_into().unwrap());
        assert_eq!(Market::files(&cid).unwrap_or_default(),
            FileInfo {
                file_size,
                spower: Market::calculate_spower(file_size, 1),
                expired_at: 1303,
                calculated_at: 503,
                amount: 18577,
                prepaid: 0,
                reported_replica_count: 1,
                replicas: vec![Replica {
                    who: merchant.clone(),
                    valid_at: 303,
                    anchor: legal_pk.clone(),
                    is_reported: true,
                    created_at: Some(303)
                }]
            }
        );
        assert_eq!(merchant_ledgers(&merchant), MockMerchantLedger {
            collateral: 6_000_000,
            reward: 4643
        });

        add_who_into_replica(&cid, file_size, charlie.clone(), legal_pk.clone(), None, None);

        run_to_block(603);
        <swork::ReportedInSlot>::insert(legal_pk.clone(), 300, true);
        Market::do_calculate_reward(&cid, System::block_number().try_into().unwrap());
        assert_eq!(Market::files(&cid).unwrap_or_default(),
            FileInfo {
                file_size,
                spower: Market::calculate_spower(file_size, 1),
                expired_at: 1303,
                calculated_at: 603,
                amount: 16257,
                prepaid: 0,
                reported_replica_count: 2,
                replicas: vec![
                    Replica {
                        who: merchant.clone(),
                        valid_at: 303,
                        anchor: legal_pk.clone(),
                        is_reported: true,
                        created_at: Some(303)
                    },
                    Replica {
                        who: charlie.clone(),
                        valid_at: 503,
                        anchor: legal_pk.clone(),
                        is_reported: true,
                        created_at: Some(503)
                    }]
            }
        );
        assert_eq!(merchant_ledgers(&merchant), MockMerchantLedger {
            collateral: 6_000_000,
            reward: 5803
        });
        assert_eq!(merchant_ledgers(&charlie), MockMerchantLedger {
            collateral: 6_000_000,
            reward: 1160
        });

        add_who_into_replica(&cid, file_size, dave.clone(), hex::decode("11").unwrap(), None, None);
        run_to_block(703);
        Market::do_calculate_reward(&cid, System::block_number().try_into().unwrap());
        assert_eq!(Market::files(&cid).unwrap_or_default(),
            FileInfo {
                file_size,
                spower: Market::calculate_spower(file_size, 1),
                expired_at: 1303,
                calculated_at: 703,
                amount: 14711,
                prepaid: 0,
                reported_replica_count: 2,
                replicas: vec![
                    Replica {
                        who: merchant.clone(),
                        valid_at: 303,
                        anchor: legal_pk.clone(),
                        is_reported: true,
                        created_at: Some(303)
                    },
                    Replica {
                        who: charlie.clone(),
                        valid_at: 503,
                        anchor: legal_pk.clone(),
                        is_reported: true,
                        created_at: Some(503)
                    },
                    Replica {
                        who: dave.clone(),
                        valid_at: 703, // did't report. change it to curr bn
                        anchor: hex::decode("11").unwrap(),
                        is_reported: false,
                        created_at: Some(603)
                    }]
            }
        );
        assert_eq!(merchant_ledgers(&merchant), MockMerchantLedger {
            collateral: 6_000_000,
            reward: 6576
        });
        assert_eq!(merchant_ledgers(&charlie), MockMerchantLedger {
            collateral: 6_000_000,
            reward: 1933
        });

        run_to_block(903);
        <swork::ReportedInSlot>::insert(hex::decode("11").unwrap(), 600, true);
        Market::do_calculate_reward(&cid, System::block_number().try_into().unwrap());
        assert_eq!(Market::files(&cid).unwrap_or_default(),
            FileInfo {
                file_size,
                spower: Market::calculate_spower(file_size, 1),
                expired_at: 1303,
                calculated_at: 903,
                amount: 13077,
                prepaid: 0,
                reported_replica_count: 1,
                replicas: vec![
                    Replica {
                        who: dave.clone(),
                        valid_at: 703,
                        anchor: hex::decode("11").unwrap(),
                        is_reported: true,
                        created_at: Some(603)
                    },
                    Replica {
                        who: merchant.clone(),
                        valid_at: 903, // did't report. change it to curr bn
                        anchor: legal_pk.clone(),
                        is_reported: false,
                        created_at: Some(303)
                    },
                    Replica {
                        who: charlie.clone(),
                        valid_at: 903, // did't report. change it to curr bn
                        anchor: legal_pk.clone(),
                        is_reported: false,
                        created_at: Some(503)
                    }]
            }
        );
        assert_eq!(merchant_ledgers(&merchant), MockMerchantLedger {
            collateral: 6_000_000,
            reward: 6576
        });
        assert_eq!(merchant_ledgers(&charlie), MockMerchantLedger {
            collateral: 6_000_000,
            reward: 1933
        });
        assert_eq!(merchant_ledgers(&dave), MockMerchantLedger {
            collateral: 6_000_000,
            reward: 1634
        });

        run_to_block(1203);
        <swork::ReportedInSlot>::insert(hex::decode("11").unwrap(), 900, true);
        <swork::ReportedInSlot>::insert(legal_pk.clone(), 900, true);
        Market::do_calculate_reward(&cid, System::block_number().try_into().unwrap());
        assert_eq!(Market::files(&cid).unwrap_or_default(),
            FileInfo {
                file_size,
                spower: Market::calculate_spower(file_size, 1),
                expired_at: 1303,
                calculated_at: 1203,
                amount: 3273,
                prepaid: 0,
                reported_replica_count: 3,
                replicas: vec![
                    Replica {
                        who: dave.clone(),
                        valid_at: 703,
                        anchor: hex::decode("11").unwrap(),
                        is_reported: true,
                        created_at: Some(603)
                    },
                    Replica {
                        who: merchant.clone(),
                        valid_at: 903,
                        anchor: legal_pk.clone(),
                        is_reported: true,
                        created_at: Some(303)
                    },
                    Replica {
                        who: charlie.clone(),
                        valid_at: 903,
                        anchor: legal_pk.clone(),
                        is_reported: true,
                        created_at: Some(503)
                    }]
            }
        );
        assert_eq!(merchant_ledgers(&merchant), MockMerchantLedger {
            collateral: 6_000_000,
            reward: 9844
        });
        assert_eq!(merchant_ledgers(&charlie), MockMerchantLedger {
            collateral: 6_000_000,
            reward: 5201
        });
        assert_eq!(merchant_ledgers(&dave), MockMerchantLedger {
            collateral: 6_000_000,
            reward: 4902
        });

        run_to_block(1803);
        assert_ok!(Market::calculate_reward(Origin::signed(merchant.clone()), cid.clone()));

        assert_eq!(merchant_ledgers(&merchant), MockMerchantLedger {
            collateral: 6_000_000,
            reward: 9844
        });
        assert_eq!(merchant_ledgers(&charlie), MockMerchantLedger {
            collateral: 6_000_000,
            reward: 5201
        });
        assert_eq!(merchant_ledgers(&dave), MockMerchantLedger {
            collateral: 6_000_000,
            reward: 4902
        });
        assert_eq!(Balances::free_balance(&reserved_pot), 17173);
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
        mock_bond_owner(&merchant, &merchant);
        add_collateral(&merchant, 6000);

        assert_ok!(Market::place_storage_order(
            Origin::signed(source), cid.clone(),
            file_size, 0, vec![]
        ));

        assert_eq!(Market::files(&cid).unwrap_or_default(),
            FileInfo {
                file_size,
                spower: 0,
                expired_at: 0,
                calculated_at: 50,
                amount: 180, // ( 1000 * 1 + 0 ) * 0.2
                prepaid: 0,
                reported_replica_count: 0,
                replicas: vec![]
            }
        );

        run_to_block(303);

        let legal_wr_info = legal_work_report_with_added_files();
        let legal_pk = legal_wr_info.curr_pk.clone();

        register(&legal_pk, LegalCode::get());

        assert_eq!(Market::files(&cid).unwrap_or_default(),
            FileInfo {
                file_size,
                spower: 0,
                expired_at: 0,
                calculated_at: 50,
                amount: 180, // ( 1000 * 1 + 0 ) * 0.2
                prepaid: 0,
                reported_replica_count: 0,
                replicas: vec![]
            }
        );

        run_to_block(1506);
        Market::do_calculate_reward(&cid, System::block_number().try_into().unwrap());

        assert_eq!(Market::files(&cid).unwrap_or_default(),
            FileInfo {
                file_size,
                spower: 0,
                expired_at: 0,
                calculated_at: 50,
                amount: 180, // ( 1000 * 1 + 0 ) * 0.2
                prepaid: 0,
                reported_replica_count: 0,
                replicas: vec![]
            }
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
            mock_bond_owner(&who, &who);
            add_collateral(who, 6_000_000);
        }

        assert_ok!(Market::place_storage_order(
            Origin::signed(source), cid.clone(),
            file_size, 0, vec![]
        ));

        assert_eq!(Market::files(&cid).unwrap_or_default(),
            FileInfo {
                file_size,
                spower: 0,
                expired_at: 0,
                calculated_at: 50,
                amount: 23220,
                prepaid: 0,
                reported_replica_count: 0,
                replicas: vec![]
            }
        );

        run_to_block(303);

        let legal_wr_info = legal_work_report_with_added_files();
        let legal_pk = legal_wr_info.curr_pk.clone();

        add_who_into_replica(&cid, file_size, ferdie.clone(), legal_pk.clone(), Some(303u32), None);
        add_who_into_replica(&cid, file_size, charlie.clone(), legal_pk.clone(), Some(403u32), None);
        add_who_into_replica(&cid, file_size, dave.clone(), legal_pk.clone(), Some(503u32), None);

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

        add_who_into_replica(&cid, file_size, eve.clone(), legal_pk.clone(), Some(503u32), None);
        update_spower_info();
        assert_eq!(Market::files(&cid).unwrap_or_default(),
            FileInfo {
                file_size,
                spower: Market::calculate_spower(file_size, 5),
                expired_at: 1303,
                calculated_at: 303,
                amount: 23220,
                prepaid: 0,
                reported_replica_count: 5,
                replicas: vec![
                    Replica {
                        who: ferdie.clone(),
                        valid_at: 303,
                        anchor: legal_pk.clone(),
                        is_reported: true,
                        created_at: Some(303)
                    },
                    Replica {
                        who: merchant.clone(),
                        valid_at: 303,
                        anchor: legal_pk.clone(),
                        is_reported: true,
                        created_at: Some(303)
                    },
                    Replica {
                        who: charlie.clone(),
                        valid_at: 403,
                        anchor: legal_pk.clone(),
                        is_reported: true,
                        created_at: Some(403)
                    },
                    Replica {
                        who: dave.clone(),
                        valid_at: 503,
                        anchor: legal_pk.clone(),
                        is_reported: true,
                        created_at: Some(503)
                    },
                    Replica {
                        who: eve.clone(),
                        valid_at: 503,
                        anchor: legal_pk.clone(),
                        is_reported: true,
                        created_at: Some(503)
                    }
                ]
            }
        );
        run_to_block(503);
        <swork::ReportedInSlot>::insert(legal_pk.clone(), 0, true);
        Market::do_calculate_reward(&cid, System::block_number().try_into().unwrap());
        assert_eq!(Market::files(&cid).unwrap_or_default(),
            FileInfo {
                file_size,
                spower: Market::calculate_spower(file_size, 5),
                expired_at: 1303,
                calculated_at: 503,
                amount: 18580,
                prepaid: 0,
                reported_replica_count: 5,
                replicas: vec![
                    Replica {
                        who: ferdie.clone(),
                        valid_at: 303,
                        anchor: legal_pk.clone(),
                        is_reported: true,
                        created_at: Some(303)
                    },
                    Replica {
                        who: merchant.clone(),
                        valid_at: 303,
                        anchor: legal_pk.clone(),
                        is_reported: true,
                        created_at: Some(303)
                    },
                    Replica {
                        who: charlie.clone(),
                        valid_at: 403,
                        anchor: legal_pk.clone(),
                        is_reported: true,
                        created_at: Some(403)
                    },
                    Replica {
                        who: dave.clone(),
                        valid_at: 503,
                        anchor: legal_pk.clone(),
                        is_reported: true,
                        created_at: Some(503)
                    },
                    Replica {
                        who: eve.clone(),
                        valid_at: 503,
                        anchor: legal_pk.clone(),
                        is_reported: true,
                        created_at: Some(503)
                    }
                ]
            }
        );

        assert_eq!(merchant_ledgers(&ferdie), MockMerchantLedger {
            collateral: 6_000_000,
            reward: 1160
        });
        assert_eq!(merchant_ledgers(&merchant), MockMerchantLedger {
            collateral: 6_000_000,
            reward: 1160
        });
        assert_eq!(merchant_ledgers(&charlie), MockMerchantLedger {
            collateral: 6_000_000,
            reward: 1160
        });
        assert_eq!(merchant_ledgers(&dave), MockMerchantLedger {
            collateral: 6_000_000,
            reward: 1160
        });
        assert_eq!(merchant_ledgers(&eve), MockMerchantLedger {
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

        assert_ok!(Market::place_storage_order(
            Origin::signed(source), cid.clone(),
            file_size, 0, vec![]
        ));

        assert_eq!(Market::files(&cid).unwrap_or_default(),
            FileInfo {
                file_size,
                spower: 0,
                expired_at: 0,
                calculated_at: 50,
                amount: 23220,
                prepaid: 0,
                reported_replica_count: 0,
                replicas: vec![]
            }
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
        add_who_into_replica(&cid, file_size, charlie.clone(), hex::decode("22").unwrap(), Some(403u32), Some(BTreeSet::from_iter(vec![charlie.clone(), ferdie.clone()].into_iter())));
        add_who_into_replica(&cid, file_size, dave.clone(), hex::decode("33").unwrap(), Some(503u32), None);

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

        add_who_into_replica(&cid, file_size, eve.clone(), legal_pk.clone(), Some(503u32), None);
        for who in merchants.iter() {
            let _ = Balances::make_free_balance_be(&who, 20_000_000);
            mock_bond_owner(&who, &who);
            add_collateral(who, 6_000_000);
        }
        update_spower_info();
        assert_eq!(Market::files(&cid).unwrap_or_default(),
            FileInfo {
                file_size,
                spower: Market::calculate_spower(file_size, 4),
                expired_at: 1303,
                calculated_at: 303,
                amount: 23220,
                prepaid: 0,
                reported_replica_count: 4,
                replicas: vec![
                    Replica {
                        who: ferdie.clone(),
                        valid_at: 303,
                        anchor: hex::decode("11").unwrap(),
                        is_reported: true,
                        created_at: Some(303)
                    },
                    Replica {
                        who: merchant.clone(),
                        valid_at: 303,
                        anchor: legal_pk.clone(),
                        is_reported: true,
                        created_at: Some(303)
                    },
                    Replica {
                        who: dave.clone(),
                        valid_at: 503,
                        anchor: hex::decode("33").unwrap(),
                        is_reported: true,
                        created_at: Some(503)
                    },
                    Replica {
                        who: eve.clone(),
                        valid_at: 503,
                        anchor: legal_pk.clone(),
                        is_reported: true,
                        created_at: Some(503)
                    }
                ]
            }
        );
        run_to_block(503);
        <swork::ReportedInSlot>::insert(legal_pk.clone(), 0, true);
        <swork::ReportedInSlot>::insert(hex::decode("11").unwrap(), 0, true);
        <swork::ReportedInSlot>::insert(hex::decode("22").unwrap(), 0, true);
        <swork::ReportedInSlot>::insert(hex::decode("33").unwrap(), 0, true);
        Market::do_calculate_reward(&cid, System::block_number().try_into().unwrap());
        assert_eq!(Market::files(&cid).unwrap_or_default(),
            FileInfo {
                file_size,
                spower: Market::calculate_spower(file_size, 4),
                expired_at: 1303,
                calculated_at: 503,
                amount: 18580,
                prepaid: 0,
                reported_replica_count: 4,
                replicas: vec![
                    Replica {
                        who: ferdie.clone(),
                        valid_at: 303,
                        anchor: hex::decode("11").unwrap(),
                        is_reported: true,
                        created_at: Some(303)
                    },
                    Replica {
                        who: merchant.clone(),
                        valid_at: 303,
                        anchor: legal_pk.clone(),
                        is_reported: true,
                        created_at: Some(303)
                    },
                    Replica {
                        who: dave.clone(),
                        valid_at: 503,
                        anchor: hex::decode("33").unwrap(),
                        is_reported: true,
                        created_at: Some(503)
                    },
                    Replica {
                        who: eve.clone(),
                        valid_at: 503,
                        anchor: legal_pk.clone(),
                        is_reported: true,
                        created_at: Some(503)
                    }
                ]
            }
        );

        assert_eq!(merchant_ledgers(&ferdie), MockMerchantLedger {
            collateral: 6_000_000,
            reward: 1160
        });
        assert_eq!(merchant_ledgers(&merchant), MockMerchantLedger {
            collateral: 6_000_000,
            reward: 1160
        });
        // charlie won't get payed
        assert_eq!(merchant_ledgers(&charlie), MockMerchantLedger {
            collateral: 6_000_000,
            reward: 0
        });
        assert_eq!(merchant_ledgers(&dave), MockMerchantLedger {
            collateral: 6_000_000,
            reward: 1160
        });
        assert_eq!(merchant_ledgers(&eve), MockMerchantLedger {
            collateral: 6_000_000,
            reward: 1160
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
            mock_bond_owner(&who, &who);
            add_collateral(who, 6_000_000);
        }

        assert_ok!(Market::place_storage_order(
            Origin::signed(source), cid.clone(),
            file_size, 0, vec![]
        ));

        assert_eq!(Market::files(&cid).unwrap_or_default(),
            FileInfo {
                file_size,
                spower: 0,
                expired_at: 0,
                calculated_at: 50,
                amount: 23220,
                prepaid: 0,
                reported_replica_count: 0,
                replicas: vec![]
            }
        );

        run_to_block(303);

        let legal_wr_info = legal_work_report_with_added_files();
        let legal_pk = legal_wr_info.curr_pk.clone();

        add_who_into_replica(&cid, file_size, ferdie.clone(), legal_pk.clone(), Some(503u32), None);
        update_spower_info();
        assert_eq!(Market::files(&cid).unwrap_or_default(),
            FileInfo {
                file_size,
                spower: Market::calculate_spower(file_size, 1),
                expired_at: 1303,
                calculated_at: 303,
                amount: 23220,
                prepaid: 0,
                reported_replica_count: 1,
                replicas: vec![
                Replica {
                    who: ferdie.clone(),
                    valid_at: 503,
                    anchor: legal_pk.clone(),
                    is_reported: true,
                    created_at: Some(503)
                }]
            }
        );

        add_who_into_replica(&cid, file_size, charlie.clone(), legal_pk.clone(), Some(303u32), None);
        update_spower_info();
        assert_eq!(Market::files(&cid).unwrap_or_default(),
            FileInfo {
                file_size,
                spower: Market::calculate_spower(file_size, 2),
                expired_at: 1303,
                calculated_at: 303,
                amount: 23220,
                prepaid: 0,
                reported_replica_count: 2,
                replicas: vec![
                    Replica {
                        who: charlie.clone(),
                        valid_at: 303,
                        anchor: legal_pk.clone(),
                        is_reported: true,
                        created_at: Some(303)
                    },
                    Replica {
                        who: ferdie.clone(),
                        valid_at: 503,
                        anchor: legal_pk.clone(),
                        is_reported: true,
                        created_at: Some(503)
                    }]
            }
        );

        add_who_into_replica(&cid, file_size, dave.clone(), legal_pk.clone(), Some(103u32), None);
        update_spower_info();
        assert_eq!(Market::files(&cid).unwrap_or_default(),
            FileInfo {
                file_size,
                spower: Market::calculate_spower(file_size, 3),
                expired_at: 1303,
                calculated_at: 303,
                amount: 23220,
                prepaid: 0,
                reported_replica_count: 3,
                replicas: vec![
                    Replica {
                        who: dave.clone(),
                        valid_at: 103,
                        anchor: legal_pk.clone(),
                        is_reported: true,
                        created_at: Some(103)
                    },
                    Replica {
                        who: charlie.clone(),
                        valid_at: 303,
                        anchor: legal_pk.clone(),
                        is_reported: true,
                        created_at: Some(303)
                    },
                    Replica {
                        who: ferdie.clone(),
                        valid_at: 503,
                        anchor: legal_pk.clone(),
                        is_reported: true,
                        created_at: Some(503)
                    }]
            }
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

        update_spower_info();
        assert_eq!(Market::files(&cid).unwrap_or_default(),
            FileInfo {
                file_size,
                spower: Market::calculate_spower(file_size, 4),
                expired_at: 1303,
                calculated_at: 303,
                amount: 23220,
                prepaid: 0,
                reported_replica_count: 4,
                replicas: vec![
                    Replica {
                        who: dave.clone(),
                        valid_at: 103,
                        anchor: legal_pk.clone(),
                        is_reported: true,
                        created_at: Some(103)
                    },
                    Replica {
                        who: charlie.clone(),
                        valid_at: 303,
                        anchor: legal_pk.clone(),
                        is_reported: true,
                        created_at: Some(303)
                    },
                    Replica {
                        who: merchant.clone(),
                        valid_at: 303,
                        anchor: legal_pk.clone(),
                        is_reported: true,
                        created_at: Some(303)
                    },
                    Replica {
                        who: ferdie.clone(),
                        valid_at: 503,
                        anchor: legal_pk.clone(),
                        is_reported: true,
                        created_at: Some(503)
                    }]
            }
        );
        add_who_into_replica(&cid, file_size, eve.clone(), legal_pk.clone(), Some(703u32), None);
        update_spower_info();
        assert_eq!(Market::files(&cid).unwrap_or_default(),
            FileInfo {
                file_size,
                spower: Market::calculate_spower(file_size, 5),
                expired_at: 1303,
                calculated_at: 303,
                amount: 23220,
                prepaid: 0,
                reported_replica_count: 5,
                replicas: vec![
                    Replica {
                        who: dave.clone(),
                        valid_at: 103,
                        anchor: legal_pk.clone(),
                        is_reported: true,
                        created_at: Some(103)
                    },
                    Replica {
                        who: charlie.clone(),
                        valid_at: 303,
                        anchor: legal_pk.clone(),
                        is_reported: true,
                        created_at: Some(303)
                    },
                    Replica {
                        who: merchant.clone(),
                        valid_at: 303,
                        anchor: legal_pk.clone(),
                        is_reported: true,
                        created_at: Some(303)
                    },
                    Replica {
                        who: ferdie.clone(),
                        valid_at: 503,
                        anchor: legal_pk.clone(),
                        is_reported: true,
                        created_at: Some(503)
                    },
                    Replica {
                        who: eve.clone(),
                        valid_at: 703,
                        anchor: legal_pk.clone(),
                        is_reported: true,
                        created_at: Some(703)
                    }]
            }
        );

        add_who_into_replica(&cid, file_size, zikun.clone(), legal_pk.clone(), Some(255u32), None);
        update_spower_info();
        assert_eq!(Market::files(&cid).unwrap_or_default(),
            FileInfo {
                file_size,
                spower: Market::calculate_spower(file_size, 6),
                expired_at: 1303,
                calculated_at: 303,
                amount: 23220,
                prepaid: 0,
                reported_replica_count: 6,
                replicas: vec![
                    Replica {
                        who: dave.clone(),
                        valid_at: 103,
                        anchor: legal_pk.clone(),
                        is_reported: true,
                        created_at: Some(103)
                    },
                    Replica {
                        who: zikun.clone(),
                        valid_at: 255,
                        anchor: legal_pk.clone(),
                        is_reported: true,
                        created_at: Some(255)
                    },
                    Replica {
                        who: charlie.clone(),
                        valid_at: 303,
                        anchor: legal_pk.clone(),
                        is_reported: true,
                        created_at: Some(303)
                    },
                    Replica {
                        who: merchant.clone(),
                        valid_at: 303,
                        anchor: legal_pk.clone(),
                        is_reported: true,
                        created_at: Some(303)
                    },
                    Replica {
                        who: ferdie.clone(),
                        valid_at: 503,
                        anchor: legal_pk.clone(),
                        is_reported: true,
                        created_at: Some(503)
                    },
                    Replica {
                        who: eve.clone(),
                        valid_at: 703,
                        anchor: legal_pk.clone(),
                        is_reported: true,
                        created_at: Some(703)
                    }]
            }
        );
    });
}

/// Update file price test case
#[test]
fn update_file_byte_fee_should_work() {
    new_test_ext().execute_with(|| {
        // generate 50 blocks first
        // 0 / 0 => None => decrease
        Market::update_file_byte_fee();
        assert_eq!(Market::file_byte_fee(), 990);

        run_to_block(50);
        <swork::Free>::put(40000);
        <swork::ReportedFilesSize>::put(10000);
        Market::update_file_byte_fee();
        assert_eq!(Market::file_byte_fee(), 980);

        // first class storage is 11000 => increase 1%
        <swork::ReportedFilesSize>::put(60000);
        Market::update_file_byte_fee();
        assert_eq!(Market::file_byte_fee(), 990);

        // price is 40 and cannot decrease
        <FileByteFee<Test>>::put(40);
        <swork::ReportedFilesSize>::put(1000);
        Market::update_file_byte_fee();
        assert_eq!(Market::file_byte_fee(), 40);

        // price is 40 and will increase by 1
        <swork::ReportedFilesSize>::put(60000);
        Market::update_file_byte_fee();
        assert_eq!(Market::file_byte_fee(), 41);
    });
}

#[test]
fn update_base_fee_should_work() {
    new_test_ext().execute_with(|| {
        assert_eq!(Market::file_base_fee(), 1000);

        // orders count == 0 => decrease 3%
        <swork::AddedFilesCount>::put(500);
        OrdersCount::put(0);
        Market::update_base_fee();
        assert_eq!(Market::file_base_fee(), 970);
        assert_eq!(Swork::added_files_count(), 0);
        assert_eq!(Market::orders_count(), 0);

        // alpha == 20 => keep same
        <swork::AddedFilesCount>::put(200);
        OrdersCount::put(10);
        Market::update_base_fee();
        assert_eq!(Market::file_base_fee(), 970);
        assert_eq!(Swork::added_files_count(), 0);
        assert_eq!(Market::orders_count(), 0);

        // alpha == 0 => increase 9%
        <swork::AddedFilesCount>::put(0);
        OrdersCount::put(100);
        Market::update_base_fee();
        assert_eq!(Market::file_base_fee(), 1057);
        assert_eq!(Swork::added_files_count(), 0);
        assert_eq!(Market::orders_count(), 0);

        // alpha == 6 => increase 5%
        <swork::AddedFilesCount>::put(60);
        OrdersCount::put(10);
        Market::update_base_fee();
        assert_eq!(Market::file_base_fee(), 1110);
        assert_eq!(Swork::added_files_count(), 0);
        assert_eq!(Market::orders_count(), 0);

        // alpha == 150 => decrease 3%
        <swork::AddedFilesCount>::put(1500);
        OrdersCount::put(10);
        Market::update_base_fee();
        assert_eq!(Market::file_base_fee(), 1077);
        assert_eq!(Swork::added_files_count(), 0);
        assert_eq!(Market::orders_count(), 0);
    });
}

#[test]
fn update_file_byte_fee_per_blocks_should_work() {
    new_test_ext().execute_with(|| {
        let source = ALICE;
        let cid =
            hex::decode("4e2883ddcbc77cf19979770d756fd332d0c8f815f9de646636169e460e6af6ff").unwrap();
        let file_size = 100; // should less than
        let _ = Balances::make_free_balance_be(&source, 20000);

        assert_ok!(Market::place_storage_order(
            Origin::signed(source.clone()), cid.clone(),
            file_size, 0, vec![]
        ));

        // 6 + 3 % 10 is not zero
        Market::on_initialize(6);
        assert_eq!(Market::file_byte_fee(), 1000);
        // update file price
        Market::on_initialize(7);
        assert_eq!(Market::file_byte_fee(), 990);
        <swork::Free>::put(10000);
        <swork::ReportedFilesSize>::put(10000);
        // no new order => don't update
        Market::on_initialize(17);
        assert_eq!(Market::file_byte_fee(), 990);
        assert_ok!(Market::place_storage_order(
            Origin::signed(source.clone()), cid.clone(),
            file_size, 0, vec![]
        ));
        // 26 + 3 % 10 is not zero
        Market::on_initialize(26);
        assert_eq!(Market::file_byte_fee(), 990);
        // update file price
        Market::on_initialize(27);
        assert_eq!(Market::file_byte_fee(), 980);
    });
}

#[test]
fn update_file_keys_count_fee_per_blocks_should_work() {
    new_test_ext().execute_with(|| {
        let source = ALICE;
        let cid =
            hex::decode("4e2883ddcbc77cf19979770d756fd332d0c8f815f9de646636169e460e6af6ff").unwrap();
        let file_size = 100; // should less than
        let _ = Balances::make_free_balance_be(&source, 20000);

        assert_ok!(Market::place_storage_order(
            Origin::signed(source.clone()), cid.clone(),
            file_size, 0, vec![]
        ));

        // 6 + 3 % 10 is not zero
        <FileKeysCountFee<Test>>::put(1000);
        Market::on_initialize(6);
        assert_eq!(Market::file_keys_count_fee(), 1000);
        // update file price
        Market::on_initialize(7);
        assert_eq!(Market::file_keys_count_fee(), 990);
        // no new order => don't update
        Market::on_initialize(17);
        assert_eq!(Market::file_keys_count_fee(), 990);
        assert_ok!(Market::place_storage_order(
            Origin::signed(source.clone()), cid.clone(),
            file_size, 0, vec![]
        ));
        // 26 + 3 % 10 is not zero
        Market::on_initialize(26);
        assert_eq!(Market::file_keys_count_fee(), 990);
        // update file price
        Market::on_initialize(27);
        assert_eq!(Market::file_keys_count_fee(), 980);

        // price is 40 and cannot decrease
        <FileKeysCountFee<Test>>::put(40);
        FileKeysCount::put(20_000_000);
        assert_ok!(Market::place_storage_order(
            Origin::signed(source.clone()), cid.clone(),
            file_size, 0, vec![]
        ));
        Market::on_initialize(37);
        assert_eq!(Market::file_keys_count_fee(), 40);

        // price is 40 and will increase by 1
        FileKeysCount::put(20_000_001);
        assert_ok!(Market::place_storage_order(
            Origin::signed(source.clone()), cid.clone(),
            file_size, 0, vec![]
        ));
        Market::on_initialize(37);
        assert_eq!(Market::file_keys_count_fee(), 41);
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

        mock_bond_owner(&merchant, &merchant);
        add_collateral(&merchant, 60);

        assert_ok!(Market::place_storage_order(
            Origin::signed(source), cid.clone(),
            file_size, 0, vec![]
        ));

        assert_eq!(Market::files(&cid).unwrap_or_default(),
            FileInfo {
                file_size,
                spower: 0,
                expired_at: 0,
                calculated_at: 50,
                amount: 180, // ( 1000 * 1 + 0 ) * 0.2
                prepaid: 0,
                reported_replica_count: 0,
                replicas: vec![]
            }
        );
        assert_eq!(Balances::free_balance(&reserved_pot), 1100);
        assert_eq!(Balances::free_balance(&staking_pot), 720);
        assert_eq!(Balances::free_balance(&storage_pot), 180);

        assert_eq!(Market::withdraw_staking_pot(), 719);
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
            "QmdwgqZy1MZBfWPi7GcxVsYgJEtmvHg6rsLzbCej3tf3oF".as_bytes().to_vec();
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

        mock_bond_owner(&merchant, &merchant);
        add_collateral(&merchant, 6000);

        for cid in file_lists.clone().iter() {
            assert_ok!(Market::place_storage_order(
                Origin::signed(source.clone()), cid.clone(),
                file_size, 0, vec![]
            ));
            assert_eq!(Market::files(&cid).unwrap_or_default(),
                FileInfo {
                    file_size,
                    spower: 0,
                    expired_at: 0,
                    calculated_at: 50,
                    amount: 180, // ( 1000 * 1 + 0 ) * 0.2
                    prepaid: 0,
                    reported_replica_count: 0,
                    replicas: vec![]
                }
            );
        }
        assert_eq!(Balances::free_balance(&storage_pot), 361);
        assert_eq!(Market::orders_count(), 2);

        run_to_block(303);
        let legal_wr_info = legal_work_report_with_added_files();
        let legal_pk = legal_wr_info.curr_pk.clone();
        register(&legal_pk, LegalCode::get());
        // reported_file_size_cid1 = 90 < 100 => update file size in file info
        add_who_into_replica(&cid1, reported_file_size_cid1, merchant.clone(), legal_pk.clone(), None, None);
        update_spower_info();
        assert_eq!(Market::files(&cid1).unwrap_or_default(),
            FileInfo {
                file_size: reported_file_size_cid1,
                spower: Market::calculate_spower(reported_file_size_cid1, 1),
                expired_at: 1303,
                calculated_at: 303,
                amount: 180, // ( 1000 * 1 + 0 ) * 0.2
                prepaid: 0,
                reported_replica_count: 1,
                replicas: vec![Replica {
                    who: merchant.clone(),
                    valid_at: 303,
                    anchor: legal_pk.clone(),
                    is_reported: true,
                    created_at: Some(303)
                }]
            }
        );
        assert_eq!(Market::orders_count(), 2);
        assert_eq!(Balances::free_balance(&storage_pot), 361);
        // reported_file_size_cid2 = 1000 > 100 => close this file
        add_who_into_replica(&cid2, reported_file_size_cid2, merchant.clone(), legal_pk.clone(), None, None);
        assert_eq!(Market::files(&cid2).is_none(), true);
        assert_eq!(merchant_ledgers(&merchant), MockMerchantLedger {
            collateral: 6000,
            reward: 180
        });
        assert_eq!(Balances::free_balance(&storage_pot), 361);
        assert_eq!(Market::orders_count(), 1);
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
            "QmdwgqZy1MZBfWPi7GcxVsYgJEtmvHg6rsLzbCej3tf3oF".as_bytes().to_vec();
        let file_size = 100; // should less than merchant
        let reported_file_size_cid1 = 90;
        let _ = Balances::make_free_balance_be(&source, 20000);
        let _ = Balances::make_free_balance_be(&merchant, 20000);

        mock_bond_owner(&merchant, &merchant);
        add_collateral(&merchant, 6000);

        assert_ok!(Market::place_storage_order(
            Origin::signed(source.clone()), cid1.clone(),
            file_size, 0, vec![]
        ));
        assert_eq!(Market::files(&cid1).unwrap_or_default(),
            FileInfo {
                file_size,
                spower: 0,
                expired_at: 0,
                calculated_at: 50,
                amount: 180, // ( 1000 * 1 + 0 ) * 0.2
                prepaid: 0,
                reported_replica_count: 0,
                replicas: vec![]
            }
        );

        run_to_block(303);
        let legal_wr_info = legal_work_report_with_added_files();
        let legal_pk = legal_wr_info.curr_pk.clone();
        register(&legal_pk, LegalCode::get());
        add_who_into_replica(&cid1, reported_file_size_cid1, merchant.clone(), legal_pk.clone(), None, None);
        update_spower_info();
        assert_eq!(Market::files(&cid1).unwrap_or_default(),
            FileInfo {
                file_size: reported_file_size_cid1,
                spower: Market::calculate_spower(reported_file_size_cid1, 1),
                expired_at: 1303,
                calculated_at: 303,
                amount: 180, // ( 1000 * 1 + 0 ) * 0.2
                prepaid: 0,
                reported_replica_count: 1,
                replicas: vec![Replica {
                    who: merchant.clone(),
                    valid_at: 303,
                    anchor: legal_pk.clone(),
                    is_reported: true,
                    created_at: Some(303)
                }]
            }
        );

        // 80 < 100 => throw an error
        assert_noop!(Market::place_storage_order(
            Origin::signed(source.clone()), cid1.clone(), 80, 0, vec![]),
            DispatchError::Module {
                index: 3,
                error: 1,
                message: Some("FileSizeNotCorrect")
            }
        );

        // 12000000 > 100. Only need amount for 100
        assert_ok!(Market::place_storage_order(
            Origin::signed(source.clone()), cid1.clone(),
            12000000, 0, vec![]
        ));

        assert_eq!(Market::files(&cid1).unwrap_or_default(),
            FileInfo {
                file_size: reported_file_size_cid1,
                spower: 0,
                expired_at: 1303,
                calculated_at: 303,
                amount: 360,
                prepaid: 0,
                reported_replica_count: 0,
                replicas: vec![Replica {
                    who: merchant.clone(),
                    valid_at: 303,
                    anchor: legal_pk.clone(),
                    is_reported: false,
                    created_at: Some(303)
                }]
            }
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
            mock_bond_owner(&who, &who);
            add_collateral(who, 6_000_000);
        }

        assert_ok!(Market::place_storage_order(
            Origin::signed(source.clone()), cid.clone(),
            file_size, 0, vec![]
        ));

        assert_eq!(Market::files(&cid).unwrap_or_default(),
            FileInfo {
                file_size,
                spower: 0,
                expired_at: 0,
                calculated_at: 50,
                amount: 23220,
                prepaid: 0,
                reported_replica_count: 0,
                replicas: vec![]
            }
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

        update_spower_info();
        assert_eq!(Market::files(&cid).unwrap_or_default(),
            FileInfo {
                file_size,
                spower: Market::calculate_spower(file_size, 1),
                expired_at: 1303,
                calculated_at: 303,
                amount: 23220,
                prepaid: 0,
                reported_replica_count: 1,
                replicas: vec![Replica {
                    who: merchant.clone(),
                    valid_at: 303,
                    anchor: legal_pk.clone(),
                    is_reported: true,
                    created_at: Some(303)
                }]
            }
        );

        run_to_block(503);
        <swork::ReportedInSlot>::insert(legal_pk.clone(), 0, true);
        Market::do_calculate_reward(&cid, System::block_number().try_into().unwrap());
        assert_eq!(Market::files(&cid).unwrap_or_default(),
            FileInfo {
                file_size,
                spower: Market::calculate_spower(file_size, 1),
                expired_at: 1303,
                calculated_at: 503,
                amount: 18577,
                prepaid: 0,
                reported_replica_count: 1,
                replicas: vec![Replica {
                    who: merchant.clone(),
                    valid_at: 303,
                    anchor: legal_pk.clone(),
                    is_reported: true,
                    created_at: Some(303)
                }]
            }
        );
        assert_eq!(merchant_ledgers(&merchant), MockMerchantLedger {
            collateral: 6_000_000,
            reward: 4643
        });

        add_who_into_replica(&cid, file_size, charlie.clone(), legal_pk.clone(), None, None);
        update_spower_info();
        assert_eq!(Market::files(&cid).unwrap_or_default(),
            FileInfo {
                file_size,
                spower: Market::calculate_spower(file_size, 2),
                expired_at: 1303,
                calculated_at: 503,
                amount: 18577,
                prepaid: 0,
                reported_replica_count: 2,
                replicas: vec![
                    Replica {
                        who: merchant.clone(),
                        valid_at: 303,
                        anchor: legal_pk.clone(),
                        is_reported: true,
                        created_at: Some(303)
                    },
                    Replica {
                        who: charlie.clone(),
                        valid_at: 503,
                        anchor: legal_pk.clone(),
                        is_reported: true,
                        created_at: Some(503)
                    }]
            }
        );

        run_to_block(1803);
        <swork::ReportedInSlot>::insert(legal_pk.clone(), 1500, true);
        assert_ok!(Market::place_storage_order(
            Origin::signed(source), cid.clone(),
            file_size, 0, vec![]
        ));
        assert_eq!(Market::files(&cid).unwrap_or_default(),
            FileInfo {
                file_size,
                spower: Market::calculate_spower(file_size, 1),
                expired_at: 2803,
                calculated_at: 1803,
                amount: 23223,
                prepaid: 0,
                reported_replica_count: 2,
                replicas: vec![
                    Replica {
                        who: merchant.clone(),
                        valid_at: 303,
                        anchor: legal_pk.clone(),
                        is_reported: true,
                        created_at: Some(303)
                    },
                    Replica {
                        who: charlie.clone(),
                        valid_at: 503,
                        anchor: legal_pk.clone(),
                        is_reported: true,
                        created_at: Some(503)
                    }]
            }
        );

        assert_eq!(merchant_ledgers(&merchant), MockMerchantLedger {
            collateral: 6_000_000,
            reward: 13930
        });
        assert_eq!(merchant_ledgers(&charlie), MockMerchantLedger {
            collateral: 6_000_000,
            reward: 9287
        });
        assert_eq!(Balances::free_balance(&reserved_pot), 27800);
    });
}

#[test]
fn place_storage_order_for_file_should_make_it_pending_if_replicas_is_zero() {
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
            mock_bond_owner(&who, &who);
            add_collateral(who, 6_000_000);
        }

        assert_ok!(Market::place_storage_order(
            Origin::signed(source.clone()), cid.clone(),
            file_size, 0, vec![]
        ));

        assert_eq!(Market::files(&cid).unwrap_or_default(),
            FileInfo {
                file_size,
                spower: 0,
                expired_at: 0,
                calculated_at: 50,
                amount: 23220,
                prepaid: 0,
                reported_replica_count: 0,
                replicas: vec![]
            }
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

        update_spower_info();
        assert_eq!(Market::files(&cid).unwrap_or_default(),
            FileInfo {
                file_size,
                spower: Market::calculate_spower(file_size, 1),
                expired_at: 1303,
                calculated_at: 303,
                amount: 23220,
                prepaid: 0,
                reported_replica_count: 1,
                replicas: vec![Replica {
                    who: merchant.clone(),
                    valid_at: 303,
                    anchor: legal_pk.clone(),
                    is_reported: true,
                    created_at: Some(303)
                }]
            }
        );

        run_to_block(503);
        <swork::ReportedInSlot>::insert(legal_pk.clone(), 0, true);
        Market::do_calculate_reward(&cid, System::block_number().try_into().unwrap());
        assert_eq!(Market::files(&cid).unwrap_or_default(),
            FileInfo {
                file_size,
                spower: Market::calculate_spower(file_size, 1),
                expired_at: 1303,
                calculated_at: 503,
                amount: 18577,
                prepaid: 0,
                reported_replica_count: 1,
                replicas: vec![Replica {
                    who: merchant.clone(),
                    valid_at: 303,
                    anchor: legal_pk.clone(),
                    is_reported: true,
                    created_at: Some(303)
                }]
            }
        );
        assert_eq!(merchant_ledgers(&merchant), MockMerchantLedger {
            collateral: 6_000_000,
            reward: 4643
        });

        add_who_into_replica(&cid, file_size, charlie.clone(), legal_pk.clone(), None, None);
        update_spower_info();
        assert_eq!(Market::files(&cid).unwrap_or_default(),
            FileInfo {
                file_size,
                spower: Market::calculate_spower(file_size, 2),
                expired_at: 1303,
                calculated_at: 503,
                amount: 18577,
                prepaid: 0,
                reported_replica_count: 2,
                replicas: vec![
                    Replica {
                        who: merchant.clone(),
                        valid_at: 303,
                        anchor: legal_pk.clone(),
                        is_reported: true,
                        created_at: Some(303)
                    },
                    Replica {
                        who: charlie.clone(),
                        valid_at: 503,
                        anchor: legal_pk.clone(),
                        is_reported: true,
                        created_at: Some(503)
                    }]
            }
        );
        Market::delete_replica(&merchant, &cid, &legal_pk);
        Market::delete_replica(&charlie, &cid, &legal_pk);

        run_to_block(903);
        Market::do_calculate_reward(&cid, System::block_number().try_into().unwrap());
        // calculated_at should be updated
        assert_eq!(Market::files(&cid).unwrap_or_default(),
            FileInfo {
                file_size,
                spower: 0,
                expired_at: 1303,
                calculated_at: 903,
                amount: 18577,
                prepaid: 0,
                reported_replica_count: 0,
                replicas: vec![]
            }
        );

        run_to_block(1203);

        assert_ok!(Market::place_storage_order(
            Origin::signed(source), cid.clone(),
            file_size, 0, vec![]
        ));
        assert_eq!(Market::files(&cid).unwrap_or_default(),
            FileInfo {
                file_size,
                spower: 0,
                expired_at: 0,
                calculated_at: 1203,
                amount: 41797,
                prepaid: 0,
                reported_replica_count: 0,
                replicas: vec![]
            }
        );
    });
}

#[test]
fn dynamic_spower_should_work() {
    new_test_ext().execute_with(|| {
        // generate 50 blocks first
        run_to_block(50);

        let source = ALICE;
        let merchant = MERCHANT;

        let cid =
            "QmdwgqZy1MZBfWPi7GcxVsYgJEtmvHg6rsLzbCej3tf3oF".as_bytes().to_vec();
        let file_size = 134289408;
        let _ = Balances::make_free_balance_be(&source, 20_000_000);
        let merchants = vec![merchant.clone()];
        for who in merchants.iter() {
            let _ = Balances::make_free_balance_be(&who, 20_000_000);
            mock_bond_owner(&who, &who);
            add_collateral(who, 6_000_000);
        }

        assert_ok!(Market::place_storage_order(
            Origin::signed(source), cid.clone(),
            file_size, 0, vec![]
        ));

        assert_eq!(Market::files(&cid).unwrap_or_default(),
            FileInfo {
                file_size,
                spower: 0,
                expired_at: 0,
                calculated_at: 50,
                amount: 23220,
                prepaid: 0,
                reported_replica_count: 0,
                replicas: vec![]
            }
        );

        run_to_block(303);

        let legal_wr_info = legal_work_report_with_added_files();
        let legal_pk = legal_wr_info.curr_pk.clone();

        for _ in 0..10 {
            add_who_into_replica(&cid, file_size, merchant.clone(), legal_pk.clone(), Some(303u32), None);
        }
        update_spower_info();
        assert_eq!(Market::files(&cid).unwrap_or_default().spower, Market::calculate_spower(file_size, 10));
        assert_eq!(Market::files(&cid).unwrap_or_default().reported_replica_count, 10);

        for _ in 0..10 {
            add_who_into_replica(&cid, file_size, merchant.clone(), legal_pk.clone(), Some(303u32), None);
        }
        update_spower_info();
        assert_eq!(Market::files(&cid).unwrap_or_default().spower, Market::calculate_spower(file_size, 20));
        assert_eq!(Market::files(&cid).unwrap_or_default().reported_replica_count, 20);
        for _ in 0..200 {
            add_who_into_replica(&cid, file_size, merchant.clone(), legal_pk.clone(), Some(303u32), None);
        }
        update_spower_info();
        assert_eq!(Market::files(&cid).unwrap_or_default().spower, Market::calculate_spower(file_size, 200));
        assert_eq!(Market::files(&cid).unwrap_or_default().reported_replica_count, 200);
        for _ in 0..140 {
            Market::delete_replica(&merchant, &cid, &legal_pk);
        }
        update_spower_info();
        assert_eq!(Market::files(&cid).unwrap_or_default().spower, Market::calculate_spower(file_size, 0));
        assert_eq!(Market::files(&cid).unwrap_or_default().reported_replica_count, 0);
    });
}

#[test]
fn calculate_spower_should_work() {
    new_test_ext().execute_with(|| {
        let file_size = 1000;
        assert_eq!(Market::calculate_spower(file_size, 0) , 0);
        assert_eq!(Market::calculate_spower(file_size, 200) , file_size * 10);
        assert_eq!(Market::calculate_spower(file_size, 250) , file_size * 10);
        assert_eq!(Market::calculate_spower(file_size, 146) , file_size * 9 + file_size * 2 / 5);
        assert_eq!(Market::calculate_spower(file_size, 16) , file_size + file_size / 5);
        assert_eq!(Market::calculate_spower(file_size, 128) , file_size * 9 + file_size / 5);
    });
}

#[test]
fn delete_spower_should_work() {
    new_test_ext().execute_with(|| {
        // generate 50 blocks first
        run_to_block(50);

        let source = ALICE;
        let bob = BOB;
        let charlie = CHARLIE;
        let merchant = MERCHANT;

        let cid =
            "QmdwgqZy1MZBfWPi7GcxVsYgJEtmvHg6rsLzbCej3tf3oF".as_bytes().to_vec();
        let file_size = 134289408;
        let _ = Balances::make_free_balance_be(&source, 20_000_000);
        let merchants = vec![merchant.clone()];
        for who in merchants.iter() {
            let _ = Balances::make_free_balance_be(&who, 20_000_000);
            mock_bond_owner(&who, &who);
            add_collateral(who, 6_000_000);
        }

        assert_ok!(Market::place_storage_order(
            Origin::signed(source), cid.clone(),
            file_size, 0, vec![]
        ));

        assert_eq!(Market::files(&cid).unwrap_or_default(),
            FileInfo {
                file_size,
                spower: 0,
                expired_at: 0,
                calculated_at: 50,
                amount: 23220,
                prepaid: 0,
                reported_replica_count: 0,
                replicas: vec![]
            }
        );

        run_to_block(303);

        let mut expected_groups = BTreeMap::new();
        for i in 10..28 {
            let key = hex::decode(i.to_string()).unwrap();
            add_who_into_replica(&cid, file_size, merchant.clone(), key.clone(), Some(303u32), None);
            <swork::ReportedInSlot>::insert(key.clone(), 0, true);
            expected_groups.insert(key.clone(), true);
        }
        add_who_into_replica(&cid, file_size, bob.clone(), hex::decode("29").unwrap(), Some(303u32), None);
        <swork::ReportedInSlot>::insert(hex::decode("29").unwrap(), 0, true);
        add_who_into_replica(&cid, file_size, charlie.clone(), hex::decode("30").unwrap(), Some(303u32), None);
        <swork::ReportedInSlot>::insert(hex::decode("30").unwrap(), 0, true);
        update_spower_info();
        assert_eq!(Market::files(&cid).unwrap_or_default().spower, Market::calculate_spower(file_size, 20));
        assert_eq!(Market::files(&cid).unwrap_or_default().reported_replica_count, 20);
        Market::delete_replica(&merchant, &cid, &hex::decode("10").unwrap());
        expected_groups.remove(&hex::decode("10").unwrap());
        update_spower_info();
        assert_eq!(Market::files(&cid).unwrap_or_default().spower, Market::calculate_spower(file_size, 2));
        assert_eq!(Market::files(&cid).unwrap_or_default().reported_replica_count, 2);

        for i in 28..30 {
            let key = hex::decode(i.to_string()).unwrap();
            <swork::ReportedInSlot>::insert(key.clone(), 300, true);
            expected_groups.insert(key.clone(), true);
        }
        Market::delete_replica(&bob, &cid, &hex::decode("29").unwrap()); // delete 21. 21 won't be deleted twice.
        update_spower_info();
        assert_eq!(Market::files(&cid).unwrap_or_default().spower, Market::calculate_spower(file_size, 1));
        assert_eq!(Market::files(&cid).unwrap_or_default().reported_replica_count, 1);
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
            mock_bond_owner(&who, &who);
            add_collateral(who, 6_000_000);
        }

        assert_ok!(Market::calculate_reward(Origin::signed(charlie.clone()), cid.clone()));

        assert_ok!(Market::place_storage_order(
            Origin::signed(source.clone()), cid.clone(),
            file_size, 0, vec![]
        ));

        assert_noop!(
            Market::calculate_reward(Origin::signed(charlie.clone()), cid.clone()),
            DispatchError::Module {
                index: 3,
                error: 2,
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

        // // Calculate reward cannot work in the middle of the file
        // assert_noop!(
        //     Market::calculate_reward(Origin::signed(charlie.clone()), cid.clone()),
        //     DispatchError::Module {
        //         index: 3,
        //         error: 6,
        //         message: Some("NotInRewardPeriod")
        // });

        run_to_block(2503);
        // 20% would be rewarded to liquidator charlie
        assert_ok!(Market::calculate_reward(Origin::signed(charlie.clone()), cid.clone()));
        assert_eq!(Balances::free_balance(&charlie), 4644);
        assert_eq!(Market::files(&cid).is_none(), true);

        assert_ok!(Market::place_storage_order(
            Origin::signed(source.clone()), cid.clone(),
            file_size, 0, vec![]
        ));

        add_who_into_replica(&cid, file_size, merchant.clone(), legal_pk.clone(), None, None);

        run_to_block(4000); // 3503 - 4503 => no reward to liquidator charlie
        assert_ok!(Market::calculate_reward(Origin::signed(charlie.clone()), cid.clone()));
        assert_eq!(Balances::free_balance(&charlie), 4644);

        assert_ok!(Market::place_storage_order(
            Origin::signed(source.clone()), cid.clone(),
            file_size, 0, vec![]
        ));

        add_who_into_replica(&cid, file_size, merchant.clone(), legal_pk.clone(), None, None);

        run_to_block(8000); // file_keys_count 6000 => all reward to liquidator charlie
        assert_ok!(Market::calculate_reward(Origin::signed(charlie.clone()), cid.clone()));
        assert_eq!(Balances::free_balance(&charlie), 27864);
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

        add_collateral(&merchant, 180);
        add_reward(&merchant, 120);

        assert_ok!(Market::reward_merchant(Origin::signed(merchant.clone())));
        assert_eq!(merchant_ledgers(&merchant), MockMerchantLedger {
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
                error: 3,
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
                message: Some("NotEnoughReward")
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

        mock_bond_owner(&merchant, &merchant);
        add_collateral(&merchant, 60);

        assert_ok!(Market::place_storage_order(
            Origin::signed(source.clone()), cid.clone(),
            file_size, 0, vec![]
        ));
        assert_ok!(Market::set_enable_market(
            Origin::root(),
            false
        ));
        assert_noop!(Market::place_storage_order(
            Origin::signed(source), cid.clone(),
            file_size, 0, vec![]
        ),
        DispatchError::Module {
            index: 3,
            error: 5,
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
            mock_bond_owner(&who, &who);
            add_collateral(who, 6_000_000);
        }

        assert_noop!(
            Market::add_prepaid(Origin::signed(source.clone()), cid.clone(), 400_000),
            DispatchError::Module {
                index: 3,
                error: 6,
                message: Some("FileNotExist")
        });

        assert_ok!(Market::place_storage_order(
            Origin::signed(source.clone()), cid.clone(),
            file_size, 0, vec![]
        ));
        assert_eq!(Balances::free_balance(&reserved_pot), 13900);
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
        update_spower_info();
        assert_eq!(Market::files(&cid).unwrap_or_default(),
            FileInfo {
                file_size,
                spower: Market::calculate_spower(file_size, 1),
                expired_at: 1303,
                calculated_at: 303,
                amount: 23220,
                prepaid: 400_000,
                reported_replica_count: 1,
                replicas: vec![Replica {
                    who: merchant.clone(),
                    valid_at: 303,
                    anchor: legal_pk.clone(),
                    is_reported: true,
                    created_at: Some(303)
                }]
            }
        );

        run_to_block(2503);
        // 20% would be rewarded to liquidator charlie
        assert_ok!(Market::calculate_reward(Origin::signed(charlie.clone()), cid.clone()));

        assert_eq!(Market::files(&cid).unwrap_or_default(),
            FileInfo {
                file_size,
                spower: Market::calculate_spower(file_size, 0),
                expired_at: 3503,
                calculated_at: 2503,
                amount: 41796, // 23220 * 0.8 + 23220
                prepaid: 263_500,
                reported_replica_count: 0,
                replicas: vec![Replica {
                    who: merchant.clone(),
                    valid_at: 2503,
                    anchor: legal_pk.clone(),
                    is_reported: false,
                    created_at: Some(303)
                }]
            }
        );


        assert_eq!(Balances::free_balance(&charlie), 11144);
        assert_eq!(Balances::free_balance(&reserved_pot), 27800);

        run_to_block(8000); // expired_on 2303 => all reward to liquidator charlie
        assert_ok!(Market::calculate_reward(Origin::signed(charlie.clone()), cid.clone()));
        assert_eq!(Balances::free_balance(&charlie), 59440); // 41796 + 11144 + 6500
        assert_eq!(Market::files(&cid).unwrap_or_default(),
            FileInfo {
                file_size,
                spower: Market::calculate_spower(file_size, 0),
                expired_at: 9000,
                calculated_at: 8000,
                amount: 23220,
                prepaid: 127000,
                reported_replica_count: 0,
                replicas: vec![Replica {
                    who: merchant.clone(),
                    valid_at: 8000,
                    anchor: legal_pk.clone(),
                    is_reported: false,
                    created_at: Some(303)
                }]
            }
        );
        assert_eq!(Balances::free_balance(&reserved_pot), 41700);
        run_to_block(9000); // expired_on 3303 => all reward to liquidator charlie
        assert_ok!(Market::calculate_reward(Origin::signed(charlie.clone()), cid.clone()));
        assert_eq!(Balances::free_balance(&charlie), 59440); // 41796 + 11144 + 6500
        assert_eq!(Market::files(&cid).is_none(), true);
        assert_eq!(Balances::free_balance(&reserved_pot), 191920); // 41700 + 127000 + 23220
    });
}

#[test]
fn renew_onging_file_should_not_work() {
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
            mock_bond_owner(&who, &who);
            add_collateral(who, 6_000_000);
        }

        assert_noop!(
            Market::add_prepaid(Origin::signed(source.clone()), cid.clone(), 400_000),
            DispatchError::Module {
                index: 3,
                error: 6,
                message: Some("FileNotExist")
        });

        assert_ok!(Market::place_storage_order(
            Origin::signed(source.clone()), cid.clone(),
            file_size, 0, vec![]
        ));
        assert_eq!(Balances::free_balance(&reserved_pot), 13900);
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
        update_spower_info();
        assert_eq!(Market::files(&cid).unwrap_or_default(),
           FileInfo {
               file_size,
               spower: Market::calculate_spower(file_size, 1),
               expired_at: 1303,
               calculated_at: 303,
               amount: 23220,
               prepaid: 400_000,
               reported_replica_count: 1,
               replicas: vec![Replica {
                   who: merchant.clone(),
                   valid_at: 303,
                   anchor: legal_pk.clone(),
                   is_reported: true,
                   created_at: Some(303)
               }]
           }
        );

        run_to_block(503);
        // 20% would be rewarded to liquidator charlie
        assert_ok!(Market::calculate_reward(Origin::signed(charlie.clone()), cid.clone()));

        assert_eq!(Market::files(&cid).unwrap_or_default(),
           FileInfo {
               file_size,
               spower: Market::calculate_spower(file_size, 0),
               expired_at: 1303,
               calculated_at: 503,
               amount: 23220, // 23220 * 0.8 + 23220
               prepaid: 400_000,
               reported_replica_count: 0,
               replicas: vec![Replica {
                   who: merchant.clone(),
                   valid_at: 503,
                   anchor: legal_pk.clone(),
                   is_reported: false,
                   created_at: Some(303)
               }]
           }
        );
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

        add_collateral(&merchant, 60);

        // Change base fee to 10000
        assert_ok!(Market::set_base_fee(Origin::root(), 50000));

        assert_ok!(Market::place_storage_order(
            Origin::signed(source.clone()), cid.clone(),
            file_size, 0, vec![]
        ));

        assert_eq!(Market::files(&cid).unwrap_or_default(),
            FileInfo {
                file_size,
                spower: 0,
                expired_at: 0,
                calculated_at: 50,
                amount: 23_220, // ( 1000 * 129 + 0 ) * 0.18
                prepaid: 0,
                reported_replica_count: 0,
                replicas: vec![]
            }
        );
        assert_eq!(Balances::free_balance(reserved_pot), 62900);
        assert_eq!(Balances::free_balance(staking_pot), 92880);
        assert_eq!(Balances::free_balance(storage_pot), 23220);

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
        update_spower_info();
        assert_eq!(Market::files(&cid).unwrap_or_default(),
            FileInfo {
                file_size,
                spower: Market::calculate_spower(file_size, 1),
                expired_at: 1303,
                calculated_at: 303,
                amount: 23_220,
                prepaid: 200_000,
                reported_replica_count: 1,
                replicas: vec![Replica {
                    who: merchant.clone(),
                    valid_at: 303,
                    anchor: legal_pk.clone(),
                    is_reported: true,
                    created_at: Some(303)
                }]
            }
        );
        run_to_block(2503);
        // 20% would be rewarded to liquidator charlie
        assert_ok!(Market::calculate_reward(Origin::signed(source.clone()), cid.clone()));

        assert_eq!(Market::files(&cid).unwrap_or_default(),
            FileInfo {
                file_size,
                spower: Market::calculate_spower(file_size, 0),
                expired_at: 3503,
                calculated_at: 2503,
                amount: 41796, // 23_220 * 0.8 + 23_220
                prepaid: 12050, // 200000 -187950
                reported_replica_count: 0,
                replicas: vec![Replica {
                    who: merchant.clone(),
                    valid_at: 2503,
                    anchor: legal_pk.clone(),
                    is_reported: false,
                    created_at: Some(303)
                }]
            }
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
            mock_bond_owner(&who, &who);
            add_collateral(who, 6_000_000);
        }

        assert_ok!(Market::place_storage_order(
            Origin::signed(source.clone()), cid.clone(),
            file_size, 0, vec![]
        ));
        assert_eq!(Balances::free_balance(&storage_pot), 23221);
        assert_eq!(Balances::free_balance(&reserved_pot), 13900);
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
        assert_eq!(Balances::free_balance(&storage_pot), 423221);

        run_to_block(2503);
        // 20% would be rewarded to liquidator charlie
        assert_ok!(Market::calculate_reward(Origin::signed(charlie.clone()), cid.clone()));
        assert_eq!(Balances::free_balance(&storage_pot), 305297); // 423221 - 1000 - 6500 - 105780 (129000 * 0.82) - 4644 (20%)

        run_to_block(8000); // expired_on 6000 => all reward to liquidator charlie
        assert_ok!(Market::calculate_reward(Origin::signed(charlie.clone()), cid.clone()));
        assert_eq!(Balances::free_balance(&storage_pot), 150221); // 305297 - 1000 - 6500 - 105780 (129000 * 0.82) - 23220 (100%) - 18576 (80%)

        run_to_block(9000);
        assert_ok!(Market::calculate_reward(Origin::signed(charlie.clone()), cid.clone()));
        assert_eq!(Balances::free_balance(&storage_pot), 1);
        assert_eq!(Balances::free_balance(&reserved_pot), 191920); // 41700 + 127000 + 23220
    });
}

#[test]
fn one_owner_should_work() {
    new_test_ext().execute_with(|| {
        // generate 50 blocks first
        run_to_block(50);

        let source = ALICE;
        let merchant = MERCHANT;
        let charlie = CHARLIE;
        let dave = DAVE;
        let eve = EVE;
        let ferdie = FERDIE;

        let bob = BOB; // owner 1, have merchant, charlie and dave
        let zikun = ZIKUN; // owner 2 have eve and ferdie

        let cid =
            "QmdwgqZy1MZBfWPi7GcxVsYgJEtmvHg6rsLzbCej3tf3oF".as_bytes().to_vec();
        let file_size = 134289408;
        let _ = Balances::make_free_balance_be(&source, 20_000_000);
        let group1 = vec![merchant.clone(), charlie.clone(), dave.clone()];
        let group2 = vec![eve.clone(), ferdie.clone()];
        for who in group1.iter() {
            mock_bond_owner(&who, &bob);
        }
        for who in group2.iter() {
            mock_bond_owner(&who, &zikun);
        }

        let _ = Balances::make_free_balance_be(&bob, 20_000_000);
        add_collateral(&bob, 6_000_000);

        let _ = Balances::make_free_balance_be(&zikun, 20_000_000);
        add_collateral(&zikun, 6_000_000);

        assert_ok!(Market::place_storage_order(
            Origin::signed(source), cid.clone(),
            file_size, 0, vec![]
        ));

        run_to_block(303);

        let legal_wr_info = legal_work_report_with_added_files();
        let legal_pk = legal_wr_info.curr_pk.clone();

        add_who_into_replica(&cid, file_size, ferdie.clone(), legal_pk.clone(), Some(303u32), None);
        add_who_into_replica(&cid, file_size, charlie.clone(), legal_pk.clone(), Some(403u32), None);
        add_who_into_replica(&cid, file_size, dave.clone(), legal_pk.clone(), Some(503u32), None);

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

        add_who_into_replica(&cid, file_size, eve.clone(), legal_pk.clone(), Some(503u32), None);

        run_to_block(503);
        <swork::ReportedInSlot>::insert(legal_pk.clone(), 0, true);
        Market::do_calculate_reward(&cid, System::block_number().try_into().unwrap());

        assert_eq!(merchant_ledgers(&bob), MockMerchantLedger {
            collateral: 6_000_000,
            reward: 3480 // 1160 * 3
        });

        assert_eq!(merchant_ledgers(&zikun), MockMerchantLedger {
            collateral: 6_000_000,
            reward: 1160 // 1160 + 0
        });
    });
}

#[test]
fn no_bonded_owner_should_work() {
    new_test_ext().execute_with(|| {
        // generate 50 blocks first
        run_to_block(50);

        let source = ALICE;
        let merchant = MERCHANT;
        let charlie = CHARLIE;
        let dave = DAVE;
        let eve = EVE;
        let ferdie = FERDIE;

        let bob = BOB; // owner 1 have charlie and dave
        let zikun = ZIKUN; // owner 2 have eve and ferdie

        let cid =
            "QmdwgqZy1MZBfWPi7GcxVsYgJEtmvHg6rsLzbCej3tf3oF".as_bytes().to_vec();
        let file_size = 134289408;
        let _ = Balances::make_free_balance_be(&source, 20_000_000);
        let group1 = vec![charlie.clone(), dave.clone()];
        let group2 = vec![eve.clone(), ferdie.clone()];
        for who in group1.iter() {
            mock_bond_owner(&who, &bob);
        }
        for who in group2.iter() {
            mock_bond_owner(&who, &zikun);
        }

        let _ = Balances::make_free_balance_be(&bob, 20_000_000);
        add_collateral(&bob, 6_000_000);

        let _ = Balances::make_free_balance_be(&zikun, 20_000_000);
        add_collateral(&zikun, 6_000_000);

        // merchant doesn't have any collateral

        assert_ok!(Market::place_storage_order(
            Origin::signed(source), cid.clone(),
            file_size, 0, vec![]
        ));

        run_to_block(303);

        let legal_wr_info = legal_work_report_with_added_files();
        let legal_pk = legal_wr_info.curr_pk.clone();

        add_who_into_replica(&cid, file_size, ferdie.clone(), legal_pk.clone(), Some(303u32), None);
        add_who_into_replica(&cid, file_size, charlie.clone(), legal_pk.clone(), Some(403u32), None);
        add_who_into_replica(&cid, file_size, dave.clone(), legal_pk.clone(), Some(503u32), None);

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

        add_who_into_replica(&cid, file_size, eve.clone(), legal_pk.clone(), Some(503u32), None);

        run_to_block(503);
        <swork::ReportedInSlot>::insert(legal_pk.clone(), 0, true);
        Market::do_calculate_reward(&cid, System::block_number().try_into().unwrap());

        assert_eq!(merchant_ledgers(&bob), MockMerchantLedger {
            collateral: 6_000_000,
            reward: 2320 // 1160 * 2
        });

        assert_eq!(merchant_ledgers(&zikun), MockMerchantLedger {
            collateral: 6_000_000,
            reward: 2320 // 1160 * 2
        });
    });
}

#[test]
fn max_replicas_and_groups_should_work() {
    new_test_ext().execute_with(|| {
        // generate 50 blocks first
        run_to_block(50);

        let source = ALICE;
        let merchant = MERCHANT;

        let cid =
            "QmdwgqZy1MZBfWPi7GcxVsYgJEtmvHg6rsLzbCej3tf3oF".as_bytes().to_vec();
        let file_size = 134289408;
        let _ = Balances::make_free_balance_be(&source, 20_000_000);
        let merchants = vec![merchant.clone()];
        for who in merchants.iter() {
            let _ = Balances::make_free_balance_be(&who, 20_000_000);
            mock_bond_owner(&who, &who);
            add_collateral(who, 6_000_000);
        }

        assert_ok!(Market::place_storage_order(
            Origin::signed(source), cid.clone(),
            file_size, 0, vec![]
        ));

        assert_eq!(Market::files(&cid).unwrap_or_default(),
            FileInfo {
                file_size,
                spower: 0,
                expired_at: 0,
                calculated_at: 50,
                amount: 23220,
                prepaid: 0,
                reported_replica_count: 0,
                replicas: vec![]
            }
        );

        run_to_block(303);

        for index in 0..200 {
            let who = AccountId32::new([index as u8; 32]);
            let pk = hex::decode(format!("{:04}", index)).unwrap();
            add_who_into_replica(&cid, file_size, who, pk, Some(303u32), None);
        }

        for index in 200..512 {
            let who = AccountId32::new([index as u8; 32]);
            let pk = hex::decode(format!("{:04}", index)).unwrap();
            assert_eq!(add_who_into_replica(&cid, file_size, who, pk, Some(303u32), None), 0);
        }

        assert_eq!(Market::files(&cid).unwrap_or_default().reported_replica_count, 200);
        assert_eq!(Market::files(&cid).unwrap_or_default().spower, 0);
        update_spower_info();
        assert_eq!(Market::files(&cid).unwrap_or_default().spower, Market::calculate_spower(file_size, 200));
        assert_eq!(Market::files(&cid).unwrap_or_default().replicas.len(), 200); // Only store the first 200 candidates
    });
}

#[test]
fn update_spower_info_should_work() {
    new_test_ext().execute_with(|| {
        // generate 50 blocks first
        run_to_block(50);

        let source = ALICE;
        let merchant = MERCHANT;

        let mut file_lists = vec![];
        let files_number = 25;
        for index in 0..files_number {
            let cid = hex::decode(format!("5bb706320afc633bfb843108e492192b17d2b6b9d9ee0b795ee95417fe08b6{:04}", index)).unwrap();
            file_lists.push(cid);
        }
        let file_size = 100; // should less than merchant
        let _ = Balances::make_free_balance_be(&source, 200000);
        let _ = Balances::make_free_balance_be(&merchant, 200000);

        mock_bond_owner(&merchant, &merchant);
        add_collateral(&merchant, 60000);

        for cid in file_lists.clone().iter() {
            assert_ok!(Market::place_storage_order(
                Origin::signed(source.clone()), cid.clone(),
                file_size, 0, vec![]
            ));
        }

        run_to_block(303);
        let legal_wr_info = legal_work_report_with_added_files();
        let legal_pk = legal_wr_info.curr_pk.clone();
        register(&legal_pk, LegalCode::get());
        for cid in file_lists.clone().iter() {
            add_who_into_replica(&cid, file_size, merchant.clone(), legal_pk.clone(), None, None);
        }

        assert_eq!(Market::pending_files().len(), files_number);
        Market::on_initialize(105);
        assert_eq!(Market::pending_files().len(), files_number - MAX_PENDING_FILES);
        update_spower_info();
        assert_eq!(Market::pending_files().len(), 0);

        for cid in file_lists.clone().iter() {
            assert_eq!(Market::files(&cid).unwrap_or_default(),
                FileInfo {
                    file_size,
                    spower: Market::calculate_spower(file_size, 1),
                    expired_at: 1303,
                    calculated_at: 303,
                    amount: 180, // ( 1000 * 1 + 0 ) * 0.2
                    prepaid: 0,
                    reported_replica_count: 1,
                    replicas: vec![Replica {
                        who: merchant.clone(),
                        valid_at: 303,
                        anchor: legal_pk.clone(),
                        is_reported: true,
                        created_at: Some(303)
                    }]
                }
            );
        }

        let legal_pk2 = hex::decode("11").unwrap();
        for cid in file_lists.clone().iter() {
            add_who_into_replica(&cid, file_size, merchant.clone(), legal_pk2.clone(), None, None);
        }
        assert_eq!(Market::pending_files().len(), files_number);
        Market::on_initialize(105);
        assert_eq!(Market::pending_files().len(), files_number - MAX_PENDING_FILES);
        update_spower_info();
        assert_eq!(Market::pending_files().len(), 0);

        for cid in file_lists.clone().iter() {
            assert_eq!(Market::files(&cid).unwrap_or_default(),
                FileInfo {
                    file_size,
                    spower: Market::calculate_spower(file_size, 2),
                    expired_at: 1303,
                    calculated_at: 303,
                    amount: 180, // ( 1000 * 1 + 0 ) * 0.2
                    prepaid: 0,
                    reported_replica_count: 2,
                    replicas: vec![
                        Replica {
                            who: merchant.clone(),
                            valid_at: 303,
                            anchor: legal_pk.clone(),
                            is_reported: true,
                            created_at: Some(303)
                        },
                        Replica {
                            who: merchant.clone(),
                            valid_at: 303,
                            anchor: legal_pk2.clone(),
                            is_reported: true,
                            created_at: Some(303)
                        }]
                }
            );
        }
        for cid in file_lists.clone().iter() {
            Market::delete_replica(&merchant, &cid, &legal_pk);
            Market::delete_replica(&merchant, &cid, &legal_pk2);
        }
        assert_eq!(Market::pending_files().len(), files_number);
        Market::on_initialize(105);
        assert_eq!(Market::pending_files().len(), files_number - MAX_PENDING_FILES);
        update_spower_info();
        assert_eq!(Market::pending_files().len(), 0);

        for cid in file_lists.clone().iter() {
            assert_eq!(Market::files(&cid).unwrap_or_default(),
                FileInfo {
                    file_size,
                    spower: 0,
                    expired_at: 1303,
                    calculated_at: 303,
                    amount: 180, // ( 1000 * 1 + 0 ) * 0.2
                    prepaid: 0,
                    reported_replica_count: 0,
                    replicas: vec![]
                }
            );
        }
    });
}

#[test]
fn place_storage_order_with_discount_should_work() {
    new_test_ext().execute_with(|| {
        // generate 50 blocks first
        run_to_block(50);
        set_discount_ratio(1, 20); // 5% discount

        let source = ALICE;
        let merchant = MERCHANT;

        let cid =
            hex::decode("4e2883ddcbc77cf19979770d756fd332d0c8f815f9de646636169e460e6af6ff").unwrap();
        let file_size = 100; // should less than
        let reserved_pot = Market::reserved_pot();
        let staking_pot = Market::staking_pot();
        let storage_pot = Market::storage_pot();
        assert_eq!(Balances::free_balance(&staking_pot), 0);
        let _ = Balances::make_free_balance_be(&source, 10000);
        let _ = Balances::make_free_balance_be(&merchant, 200);

        mock_bond_owner(&merchant, &merchant);

        <FileKeysCountFee<Test>>::put(1000);
        assert_ok!(Market::place_storage_order(
            Origin::signed(source.clone()), cid.clone(),
            file_size, 0, vec![]
        ));
        assert_eq!(Market::files(&cid).unwrap_or_default(),
            FileInfo {
                file_size,
                spower: 0,
                expired_at: 0,
                calculated_at: 50,
                amount: 360, // ( 1000 * 1 + 0 + 1000 ) * 0.18
                prepaid: 0,
                reported_replica_count: 0,
                replicas: vec![]
            }
        );
        assert_eq!(Balances::free_balance(&reserved_pot), 1050);
        assert_eq!(Balances::free_balance(&staking_pot), 1440);
        assert_eq!(Balances::free_balance(&storage_pot), 360);
        assert_eq!(Balances::free_balance(&source), 7150);

        set_discount_ratio(1, 10); // 10% discount

        assert_ok!(Market::place_storage_order(
            Origin::signed(source.clone()), cid.clone(),
            file_size, 0, vec![]
        ));
        assert_eq!(Market::files(&cid).unwrap_or_default(),
            FileInfo {
                file_size,
                spower: 0,
                expired_at: 0,
                calculated_at: 50,
                amount: 720, // ( 1000 + 1000 * 1 + 0 + 1000 ) * 0.18
                prepaid: 0,
                reported_replica_count: 0,
                replicas: vec![]
            }
        );
        assert_eq!(Balances::free_balance(&reserved_pot), 1950); // 150 + 0
        assert_eq!(Balances::free_balance(&staking_pot), 2880);
        assert_eq!(Balances::free_balance(&storage_pot), 720);
        assert_eq!(Balances::free_balance(&source), 4450);


        set_discount_ratio(1, 5); // 10% discount

        assert_ok!(Market::place_storage_order(
            Origin::signed(source.clone()), cid.clone(),
            file_size, 0, vec![]
        ));
        assert_eq!(Market::files(&cid).unwrap_or_default(),
           FileInfo {
               file_size,
               spower: 0,
               expired_at: 0,
               calculated_at: 50,
               amount: 1080, // ( 1000 + 1000 * 1 + 0 + 1000 ) * 0.18
               prepaid: 0,
               reported_replica_count: 0,
               replicas: vec![]
           }
        );
        assert_eq!(Balances::free_balance(reserved_pot), 2850); // 150 + 0
        assert_eq!(Balances::free_balance(staking_pot), 4320);
        assert_eq!(Balances::free_balance(storage_pot), 1080);
        assert_eq!(Balances::free_balance(&source), 1750);
    });
}

#[test]
fn spower_delay_should_work() {
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
        SpowerReadyPeriod::put(300);

        let staking_pot = Market::staking_pot();
        let reserved_pot = Market::reserved_pot();
        assert_eq!(Balances::free_balance(&staking_pot), 0);
        assert_eq!(Balances::free_balance(&reserved_pot), 0);
        let _ = Balances::make_free_balance_be(&source, 20_000_000);
        let merchants = vec![merchant.clone(), charlie.clone(), dave.clone(), eve.clone()];
        for who in merchants.iter() {
            let _ = Balances::make_free_balance_be(&who, 20_000_000);
            mock_bond_owner(&who, &who);
            add_collateral(who, 6_000_000);
        }

        assert_ok!(Market::place_storage_order(
            Origin::signed(source), cid.clone(),
            file_size, 0, vec![]
        ));

        assert_eq!(Market::files(&cid).unwrap_or_default(),
           FileInfo {
               file_size,
               spower: 0,
               expired_at: 0,
               calculated_at: 50,
               amount: 23220,
               prepaid: 0,
               reported_replica_count: 0,
               replicas: vec![]
           }
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

        update_spower_info();
        assert_eq!(Market::files(&cid).unwrap_or_default(),
           FileInfo {
               file_size,
               spower: Market::calculate_spower(file_size, 1),
               expired_at: 1303,
               calculated_at: 303,
               amount: 23220,
               prepaid: 0,
               reported_replica_count: 1,
               replicas: vec![Replica {
                   who: merchant.clone(),
                   valid_at: 303,
                   anchor: legal_pk.clone(),
                   is_reported: true,
                   created_at: Some(303)
               }]
           }
        );

        run_to_block(503);
        <swork::ReportedInSlot>::insert(legal_pk.clone(), 0, true);
        Market::do_calculate_reward(&cid, System::block_number().try_into().unwrap());
        assert_eq!(Market::files(&cid).unwrap_or_default(),
           FileInfo {
               file_size,
               spower: Market::calculate_spower(file_size, 1),
               expired_at: 1303,
               calculated_at: 503,
               amount: 18577,
               prepaid: 0,
               reported_replica_count: 1,
               replicas: vec![Replica {
                   who: merchant.clone(),
                   valid_at: 303,
                   anchor: legal_pk.clone(),
                   is_reported: true,
                   created_at: Some(303)
               }]
           }
        );
        assert_eq!(merchant_ledgers(&merchant), MockMerchantLedger {
            collateral: 6_000_000,
            reward: 4643
        });

        add_who_into_replica(&cid, file_size, charlie.clone(), legal_pk.clone(), None, None);

        run_to_block(603);
        <swork::ReportedInSlot>::insert(legal_pk.clone(), 300, true);
        Market::do_calculate_reward(&cid, System::block_number().try_into().unwrap());
        assert_eq!(Market::files(&cid).unwrap_or_default(),
           FileInfo {
               file_size,
               spower: Market::calculate_spower(file_size, 1),
               expired_at: 1303,
               calculated_at: 603,
               amount: 16257,
               prepaid: 0,
               reported_replica_count: 2,
               replicas: vec![
                   Replica {
                       who: merchant.clone(),
                       valid_at: 303,
                       anchor: legal_pk.clone(),
                       is_reported: true,
                       created_at: None
                   },
                   Replica {
                       who: charlie.clone(),
                       valid_at: 503,
                       anchor: legal_pk.clone(),
                       is_reported: true,
                       created_at: Some(503)
                   }]
           }
        );
        assert_eq!(merchant_ledgers(&merchant), MockMerchantLedger {
            collateral: 6_000_000,
            reward: 5803
        });
        assert_eq!(merchant_ledgers(&charlie), MockMerchantLedger {
            collateral: 6_000_000,
            reward: 1160
        });

        add_who_into_replica(&cid, file_size, dave.clone(), hex::decode("11").unwrap(), None, None);
        run_to_block(703);
        Market::do_calculate_reward(&cid, System::block_number().try_into().unwrap());
        assert_eq!(Market::files(&cid).unwrap_or_default(),
           FileInfo {
               file_size,
               spower: Market::calculate_spower(file_size, 1),
               expired_at: 1303,
               calculated_at: 703,
               amount: 14711,
               prepaid: 0,
               reported_replica_count: 2,
               replicas: vec![
                   Replica {
                       who: merchant.clone(),
                       valid_at: 303,
                       anchor: legal_pk.clone(),
                       is_reported: true,
                       created_at: None
                   },
                   Replica {
                       who: charlie.clone(),
                       valid_at: 503,
                       anchor: legal_pk.clone(),
                       is_reported: true,
                       created_at: Some(503)
                   },
                   Replica {
                       who: dave.clone(),
                       valid_at: 703, // did't report. change it to curr bn
                       anchor: hex::decode("11").unwrap(),
                       is_reported: false,
                       created_at: Some(603)
                   }]
           }
        );
        assert_eq!(merchant_ledgers(&merchant), MockMerchantLedger {
            collateral: 6_000_000,
            reward: 6576
        });
        assert_eq!(merchant_ledgers(&charlie), MockMerchantLedger {
            collateral: 6_000_000,
            reward: 1933
        });

        run_to_block(903);
        <swork::ReportedInSlot>::insert(hex::decode("11").unwrap(), 600, true);
        Market::do_calculate_reward(&cid, System::block_number().try_into().unwrap());
        assert_eq!(Market::files(&cid).unwrap_or_default(),
           FileInfo {
               file_size,
               spower: Market::calculate_spower(file_size, 1),
               expired_at: 1303,
               calculated_at: 903,
               amount: 13077,
               prepaid: 0,
               reported_replica_count: 1,
               replicas: vec![
                   Replica {
                       who: dave.clone(),
                       valid_at: 703,
                       anchor: hex::decode("11").unwrap(),
                       is_reported: true,
                       created_at: None
                   },
                   Replica {
                       who: merchant.clone(),
                       valid_at: 903, // did't report. change it to curr bn
                       anchor: legal_pk.clone(),
                       is_reported: false,
                       created_at: None
                   },
                   Replica {
                       who: charlie.clone(),
                       valid_at: 903, // did't report. change it to curr bn
                       anchor: legal_pk.clone(),
                       is_reported: false,
                       created_at: None
                   }]
           }
        );
        assert_eq!(merchant_ledgers(&merchant), MockMerchantLedger {
            collateral: 6_000_000,
            reward: 6576
        });
        assert_eq!(merchant_ledgers(&charlie), MockMerchantLedger {
            collateral: 6_000_000,
            reward: 1933
        });
        assert_eq!(merchant_ledgers(&dave), MockMerchantLedger {
            collateral: 6_000_000,
            reward: 1634
        });

        run_to_block(1203);
        <swork::ReportedInSlot>::insert(hex::decode("11").unwrap(), 900, true);
        <swork::ReportedInSlot>::insert(legal_pk.clone(), 900, true);
        Market::do_calculate_reward(&cid, System::block_number().try_into().unwrap());
        assert_eq!(Market::files(&cid).unwrap_or_default(),
           FileInfo {
               file_size,
               spower: Market::calculate_spower(file_size, 1),
               expired_at: 1303,
               calculated_at: 1203,
               amount: 3273,
               prepaid: 0,
               reported_replica_count: 3,
               replicas: vec![
                   Replica {
                       who: dave.clone(),
                       valid_at: 703,
                       anchor: hex::decode("11").unwrap(),
                       is_reported: true,
                       created_at: None
                   },
                   Replica {
                       who: merchant.clone(),
                       valid_at: 903,
                       anchor: legal_pk.clone(),
                       is_reported: true,
                       created_at: None
                   },
                   Replica {
                       who: charlie.clone(),
                       valid_at: 903,
                       anchor: legal_pk.clone(),
                       is_reported: true,
                       created_at: None
                   }]
           }
        );
        assert_eq!(merchant_ledgers(&merchant), MockMerchantLedger {
            collateral: 6_000_000,
            reward: 9844
        });
        assert_eq!(merchant_ledgers(&charlie), MockMerchantLedger {
            collateral: 6_000_000,
            reward: 5201
        });
        assert_eq!(merchant_ledgers(&dave), MockMerchantLedger {
            collateral: 6_000_000,
            reward: 4902
        });

        run_to_block(1803);
        assert_ok!(Market::calculate_reward(Origin::signed(merchant.clone()), cid.clone()));

        assert_eq!(merchant_ledgers(&merchant), MockMerchantLedger {
            collateral: 6_000_000,
            reward: 9844
        });
        assert_eq!(merchant_ledgers(&charlie), MockMerchantLedger {
            collateral: 6_000_000,
            reward: 5201
        });
        assert_eq!(merchant_ledgers(&dave), MockMerchantLedger {
            collateral: 6_000_000,
            reward: 4902
        });
        assert_eq!(Balances::free_balance(&reserved_pot), 17173);
    });
}

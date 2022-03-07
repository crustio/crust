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
        assert_eq!(Market::filesv2(&cid).unwrap_or_default(), FileInfoV2 {
                file_size,
                spower: 0,
                expired_at: 0,
                calculated_at: 50,
                amount: 360, // ( 1000 * 1 + 0 + 1000 ) * 0.18
                prepaid: 0,
                reported_replica_count: 0,
                remaining_paid_count: 4,
                replicas: BTreeMap::from_iter(vec![].into_iter())
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

        assert_eq!(Market::filesv2(&cid).unwrap_or_default(),
            FileInfoV2 {
                file_size,
                spower: 0,
                expired_at: 0,
                calculated_at: 50,
                amount: 23220, // ( 1000 * 129 + 0 ) * 0.18
                prepaid: 0,
                reported_replica_count: 0,
                remaining_paid_count: 4,
                replicas: BTreeMap::from_iter(vec![].into_iter())
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

        assert_eq!(Market::filesv2(&cid).unwrap_or_default(),
            FileInfoV2 {
                file_size,
                spower: 0,
                expired_at: 0,
                calculated_at: 50,
                amount: 46440, // ( 1000 * 129 + 0 ) * 0.18 * 2
                prepaid: 0,
                reported_replica_count: 0,
                remaining_paid_count: 4,
                replicas: BTreeMap::from_iter(vec![].into_iter())
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
        run_to_block(700);
        Market::update_replicas(&cid, System::block_number().try_into().unwrap());
        assert_eq!(Market::filesv2(&cid).unwrap_or_default(),
            FileInfoV2 {
                file_size,
                spower: Market::calculate_spower(file_size, 1),
                expired_at: 1400,
                calculated_at: 700,
                amount: 39990, // ( 1000 * 129 + 0 ) * 0.18 * 2 - ( 1000 * 129 + 0 ) * 0.025 * 2
                prepaid: 0,
                reported_replica_count: 1,
                remaining_paid_count: 3,
                replicas: BTreeMap::from_iter(vec![(merchant.clone(), Replica {
                    who: merchant.clone(),
                    valid_at: 303,
                    anchor: legal_pk.clone(),
                    is_reported: true,
                    created_at: Some(303)
                })])
            }
        );

        // Calculate reward should work
        run_to_block(800);
        <swork::ReportedInSlot>::insert(legal_pk.clone(), 0, true);
        Market::update_replicas(&cid, System::block_number().try_into().unwrap());
        assert_eq!(Market::filesv2(&cid).unwrap_or_default(),
            FileInfoV2 {
                file_size,
                spower: Market::calculate_spower(file_size, 1),
                expired_at: 1400,
                calculated_at: 800,
                amount: 39990, // ( 1000 * 129 + 0 ) * 0.18 * 2 - ( 1000 * 129 + 0 ) * 0.025 * 2
                prepaid: 0,
                reported_replica_count: 1,
                remaining_paid_count: 3,
                replicas: BTreeMap::from_iter(vec![(merchant.clone(), Replica {
                    who: merchant.clone(),
                    valid_at: 303,
                    anchor: legal_pk.clone(),
                    is_reported: true,
                    created_at: Some(303)
                })])
            }
        );

        // 3. Extend duration should work
        run_to_block(900);
        assert_ok!(Market::place_storage_order(
            Origin::signed(source.clone()), cid.clone(),
            file_size, 0, vec![]
        ));

        assert_eq!(Market::filesv2(&cid).unwrap_or_default(),
            FileInfoV2 {
                file_size,
                spower: Market::calculate_spower(file_size, 1),
                expired_at: 1900,
                calculated_at: 800,
                amount: 63210, // 39990 + 23220
                prepaid: 0,
                reported_replica_count: 1,
                remaining_paid_count: 3,
                replicas: BTreeMap::from_iter(vec![(merchant.clone(), Replica {
                    who: merchant.clone(),
                    valid_at: 303,
                    anchor: legal_pk.clone(),
                    is_reported: true,
                    created_at: Some(303)
                })])
            }
        );

        // 4. Extend replicas should work
        run_to_block(1000);
        assert_ok!(Market::place_storage_order(
            Origin::signed(source.clone()), cid.clone(),
            file_size, 200, vec![]
        ));

        assert_eq!(Market::filesv2(&cid).unwrap_or_default(),
            FileInfoV2 {
                file_size,
                spower: Market::calculate_spower(file_size, 1),
                expired_at: 2000,
                calculated_at: 800,
                amount: 86630, // 39990 + 23220
                prepaid: 0,
                reported_replica_count: 1,
                remaining_paid_count: 3,
                replicas: BTreeMap::from_iter(vec![(merchant.clone(), Replica {
                    who: merchant.clone(),
                    valid_at: 303,
                    anchor: legal_pk.clone(),
                    is_reported: true,
                    created_at: Some(303)
                })])
            }
        );
        assert_eq!(Market::orders_count(), 4);
    });
}

#[test]
fn update_replicas_should_fail_due_to_not_live() {
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


        assert_eq!(Market::filesv2(&cid).unwrap_or_default(),
            FileInfoV2 {
                file_size,
                spower: 0,
                expired_at: 0,
                calculated_at: 50,
                amount: 180,
                prepaid: 0,
                reported_replica_count: 0,
                remaining_paid_count: 4,
                replicas: BTreeMap::from_iter(vec![].into_iter())
            }
        );

        run_to_block(1506);
        Market::update_replicas(&cid, System::block_number().try_into().unwrap());

        assert_eq!(Market::filesv2(&cid).unwrap_or_default(),
            FileInfoV2 {
                file_size,
                spower: 0,
                expired_at: 0,
                calculated_at: 50,
                amount: 180,
                prepaid: 0,
                reported_replica_count: 0,
                remaining_paid_count: 4,
                replicas: BTreeMap::from_iter(vec![].into_iter())
            }
        );
    });
}

#[test]
fn update_replicas_should_work_for_more_replicas() {
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

        assert_eq!(Market::filesv2(&cid).unwrap_or_default(),
            FileInfoV2 {
                file_size,
                spower: 0,
                expired_at: 0,
                calculated_at: 50,
                amount: 23220,
                prepaid: 0,
                reported_replica_count: 0,
                remaining_paid_count: 4,
                replicas: BTreeMap::from_iter(vec![].into_iter())
            }
        );

        run_to_block(303);

        let legal_wr_info = legal_work_report_with_added_files();
        let legal_pk = legal_wr_info.curr_pk.clone();

        add_who_into_replica(&cid, file_size, ferdie.clone(), ferdie.clone(), legal_pk.clone(), Some(303u32), None);
        add_who_into_replica(&cid, file_size, charlie.clone(), charlie.clone(), legal_pk.clone(), Some(403u32), None);
        add_who_into_replica(&cid, file_size, dave.clone(), dave.clone(), legal_pk.clone(), Some(503u32), None);

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

        add_who_into_replica(&cid, file_size, eve.clone(), eve.clone(), legal_pk.clone(), Some(503u32), None);
        <swork::ReportedInSlot>::insert(legal_pk.clone(), 0, true);
        Market::update_replicas(&cid, System::block_number().try_into().unwrap());
        assert_eq!(Market::filesv2(&cid).unwrap_or_default(),
            FileInfoV2 {
                file_size,
                spower: Market::calculate_spower(file_size, 5),
                expired_at: 1303,
                calculated_at: 303,
                amount: 10320,
                prepaid: 0,
                reported_replica_count: 5,
                remaining_paid_count: 0,
                replicas: BTreeMap::from_iter(vec![
                    (ferdie.clone(),
                    Replica {
                        who: ferdie.clone(),
                        valid_at: 303,
                        anchor: legal_pk.clone(),
                        is_reported: true,
                        created_at: Some(303)
                    }),
                    (merchant.clone(),
                    Replica {
                        who: merchant.clone(),
                        valid_at: 303,
                        anchor: legal_pk.clone(),
                        is_reported: true,
                        created_at: Some(303)
                    }),
                    (charlie.clone(),
                    Replica {
                        who: charlie.clone(),
                        valid_at: 403,
                        anchor: legal_pk.clone(),
                        is_reported: true,
                        created_at: Some(403)
                    }),
                    (dave.clone(),
                    Replica {
                        who: dave.clone(),
                        valid_at: 503,
                        anchor: legal_pk.clone(),
                        is_reported: true,
                        created_at: Some(503)
                    }),
                    (eve.clone(),
                    Replica {
                        who: eve.clone(),
                        valid_at: 503,
                        anchor: legal_pk.clone(),
                        is_reported: true,
                        created_at: Some(503)
                    })
                ])
            }
        );
        run_to_block(503);
        <swork::ReportedInSlot>::insert(legal_pk.clone(), 0, true);
        Market::update_replicas(&cid, System::block_number().try_into().unwrap());
        assert_eq!(Market::filesv2(&cid).unwrap_or_default(),
            FileInfoV2 {
                file_size,
                spower: Market::calculate_spower(file_size, 5),
                expired_at: 1303,
                calculated_at: 503,
                amount: 10320,
                prepaid: 0,
                reported_replica_count: 5,
                remaining_paid_count: 0,
                replicas: BTreeMap::from_iter(vec![
                    (ferdie.clone(),
                    Replica {
                        who: ferdie.clone(),
                        valid_at: 303,
                        anchor: legal_pk.clone(),
                        is_reported: true,
                        created_at: Some(303)
                    }),
                    (merchant.clone(),
                    Replica {
                        who: merchant.clone(),
                        valid_at: 303,
                        anchor: legal_pk.clone(),
                        is_reported: true,
                        created_at: Some(303)
                    }),
                    (charlie.clone(),
                    Replica {
                        who: charlie.clone(),
                        valid_at: 403,
                        anchor: legal_pk.clone(),
                        is_reported: true,
                        created_at: Some(403)
                    }),
                    (dave.clone(),
                    Replica {
                        who: dave.clone(),
                        valid_at: 503,
                        anchor: legal_pk.clone(),
                        is_reported: true,
                        created_at: Some(503)
                    }),
                    (eve.clone(),
                    Replica {
                        who: eve.clone(),
                        valid_at: 503,
                        anchor: legal_pk.clone(),
                        is_reported: true,
                        created_at: Some(503)
                    })
                ])
            }
        );

        assert_eq!(merchant_ledgers(&ferdie), MockMerchantLedger {
            collateral: 6_000_000,
            reward: 3225
        });
        assert_eq!(merchant_ledgers(&merchant), MockMerchantLedger {
            collateral: 6_000_000,
            reward: 3225
        });
        assert_eq!(merchant_ledgers(&charlie), MockMerchantLedger {
            collateral: 6_000_000,
            reward: 3225
        });
        assert_eq!(merchant_ledgers(&dave), MockMerchantLedger {
            collateral: 6_000_000,
            reward: 3225
        });
        assert_eq!(merchant_ledgers(&eve), MockMerchantLedger {
            collateral: 6_000_000,
            reward: 0
        });
    });
}

#[test]
fn update_replicas_should_only_pay_the_groups() {
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

        assert_eq!(Market::filesv2(&cid).unwrap_or_default(),
            FileInfoV2 {
                file_size,
                spower: 0,
                expired_at: 0,
                calculated_at: 50,
                amount: 23220,
                prepaid: 0,
                reported_replica_count: 0,
                remaining_paid_count: 4,
                replicas: BTreeMap::from_iter(vec![].into_iter())
            }
        );

        for who in merchants.iter() {
            let _ = Balances::make_free_balance_be(&who, 20_000_000);
            mock_bond_owner(&who, &who);
            add_collateral(who, 6_000_000);
        }

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
        add_who_into_replica(&cid, file_size, ferdie.clone(), ferdie.clone(), hex::decode("11").unwrap(), Some(303u32), Some(BTreeSet::from_iter(vec![charlie.clone(), ferdie.clone()].into_iter())));
        add_who_into_replica(&cid, file_size, charlie.clone(), ferdie.clone(), hex::decode("22").unwrap(), Some(403u32), Some(BTreeSet::from_iter(vec![charlie.clone(), ferdie.clone()].into_iter())));
        add_who_into_replica(&cid, file_size, dave.clone(), dave.clone(), hex::decode("33").unwrap(), Some(503u32), None);

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

        add_who_into_replica(&cid, file_size, eve.clone(), eve.clone(), legal_pk.clone(), Some(503u32), None);
        <swork::ReportedInSlot>::insert(legal_pk.clone(), 0, true);
        <swork::ReportedInSlot>::insert(hex::decode("11").unwrap(), 0, true);
        <swork::ReportedInSlot>::insert(hex::decode("22").unwrap(), 0, true);
        <swork::ReportedInSlot>::insert(hex::decode("33").unwrap(), 0, true);
        Market::update_replicas(&cid, System::block_number().try_into().unwrap());
        assert_eq!(Market::filesv2(&cid).unwrap_or_default(),
            FileInfoV2 {
                file_size,
                spower: Market::calculate_spower(file_size, 4),
                expired_at: 1303,
                calculated_at: 303,
                amount: 10320,
                prepaid: 0,
                reported_replica_count: 4,
                remaining_paid_count: 0,
                replicas: BTreeMap::from_iter(vec![
                    (ferdie.clone(),
                    Replica {
                        who: ferdie.clone(),
                        valid_at: 303,
                        anchor: hex::decode("11").unwrap(),
                        is_reported: true,
                        created_at: Some(303)
                    }),
                    (merchant.clone(),
                    Replica {
                        who: merchant.clone(),
                        valid_at: 303,
                        anchor: legal_pk.clone(),
                        is_reported: true,
                        created_at: Some(303)
                    }),
                    (dave.clone(),
                    Replica {
                        who: dave.clone(),
                        valid_at: 503,
                        anchor: hex::decode("33").unwrap(),
                        is_reported: true,
                        created_at: Some(503)
                    }),
                    (eve.clone(),
                    Replica {
                        who: eve.clone(),
                        valid_at: 503,
                        anchor: legal_pk.clone(),
                        is_reported: true,
                        created_at: Some(503)
                    })
                ])
            }
        );

        assert_eq!(merchant_ledgers(&ferdie), MockMerchantLedger {
            collateral: 6_000_000,
            reward: 3225
        });
        assert_eq!(merchant_ledgers(&merchant), MockMerchantLedger {
            collateral: 6_000_000,
            reward: 3225
        });
        // charlie won't get payed
        assert_eq!(merchant_ledgers(&charlie), MockMerchantLedger {
            collateral: 6_000_000,
            reward: 0
        });
        assert_eq!(merchant_ledgers(&dave), MockMerchantLedger {
            collateral: 6_000_000,
            reward: 3225
        });
        assert_eq!(merchant_ledgers(&eve), MockMerchantLedger {
            collateral: 6_000_000,
            reward: 3225
        });
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

        // orders count == 0 => decrease 5%
        <swork::AddedFilesCount>::put(500);
        OrdersCount::put(0);
        Market::update_base_fee();
        assert_eq!(Market::file_base_fee(), 950);
        assert_eq!(Swork::added_files_count(), 0);
        assert_eq!(Market::orders_count(), 0);

        // alpha == 20 => keep same
        <swork::AddedFilesCount>::put(200);
        OrdersCount::put(10);
        Market::update_base_fee();
        assert_eq!(Market::file_base_fee(), 950);
        assert_eq!(Swork::added_files_count(), 0);
        assert_eq!(Market::orders_count(), 0);

        // alpha == 0 => increase 20%
        <swork::AddedFilesCount>::put(0);
        OrdersCount::put(100);
        Market::update_base_fee();
        assert_eq!(Market::file_base_fee(), 1140);
        assert_eq!(Swork::added_files_count(), 0);
        assert_eq!(Market::orders_count(), 0);

        // alpha == 6 => increase 8%
        <swork::AddedFilesCount>::put(60);
        OrdersCount::put(10);
        Market::update_base_fee();
        assert_eq!(Market::file_base_fee(), 1231);
        assert_eq!(Swork::added_files_count(), 0);
        assert_eq!(Market::orders_count(), 0);

        // alpha == 150 => decrease 5%
        <swork::AddedFilesCount>::put(1500);
        OrdersCount::put(10);
        Market::update_base_fee();
        assert_eq!(Market::file_base_fee(), 1169);
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

        assert_eq!(Market::filesv2(&cid).unwrap_or_default(),
            FileInfoV2 {
                file_size,
                spower: 0,
                expired_at: 0,
                calculated_at: 50,
                amount: 180, // ( 1000 * 1 + 0 ) * 0.2
                prepaid: 0,
                reported_replica_count: 0,
                remaining_paid_count: 4,
                replicas: BTreeMap::from_iter(vec![].into_iter())
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
            assert_eq!(Market::filesv2(&cid).unwrap_or_default(),
                FileInfoV2 {
                    file_size,
                    spower: 0,
                    expired_at: 0,
                    calculated_at: 50,
                    amount: 180, // ( 1000 * 1 + 0 ) * 0.2
                    prepaid: 0,
                    reported_replica_count: 0,
                    remaining_paid_count: 4,
                    replicas: BTreeMap::from_iter(vec![].into_iter())
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
        add_who_into_replica(&cid1, reported_file_size_cid1, merchant.clone(), merchant.clone(), legal_pk.clone(), None, None);
        assert_eq!(Market::filesv2(&cid1).unwrap_or_default(),
            FileInfoV2 {
                file_size: reported_file_size_cid1,
                spower: 0,
                expired_at: 1303,
                calculated_at: 303,
                amount: 155, // ( 1000 * 1 + 0 ) * 0.2
                prepaid: 0,
                reported_replica_count: 1,
                remaining_paid_count: 3,
                replicas: BTreeMap::from_iter(vec![(merchant.clone(), Replica {
                    who: merchant.clone(),
                    valid_at: 303,
                    anchor: legal_pk.clone(),
                    is_reported: true,
                    created_at: Some(303)
                })])
            }
        );
        assert_eq!(Market::orders_count(), 2);
        assert_eq!(Balances::free_balance(&storage_pot), 361);
        // reported_file_size_cid2 = 1000 > 100 => close this file
        add_who_into_replica(&cid2, reported_file_size_cid2, merchant.clone(), merchant.clone(), legal_pk.clone(), None, None);
        assert_eq!(Market::filesv2(&cid2).is_none(), true);
        assert_eq!(merchant_ledgers(&merchant), MockMerchantLedger {
            collateral: 6000,
            reward: 205
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
        assert_eq!(Market::filesv2(&cid1).unwrap_or_default(),
            FileInfoV2 {
                file_size,
                spower: 0,
                expired_at: 0,
                calculated_at: 50,
                amount: 180, // ( 1000 * 1 + 0 ) * 0.2
                prepaid: 0,
                reported_replica_count: 0,
                remaining_paid_count: 4,
                replicas: BTreeMap::from_iter(vec![].into_iter())
            }
        );

        run_to_block(303);
        let legal_wr_info = legal_work_report_with_added_files();
        let legal_pk = legal_wr_info.curr_pk.clone();
        register(&legal_pk, LegalCode::get());
        add_who_into_replica(&cid1, reported_file_size_cid1, merchant.clone(), merchant.clone(), legal_pk.clone(), None, None);
        assert_eq!(Market::filesv2(&cid1).unwrap_or_default(),
            FileInfoV2 {
                file_size: reported_file_size_cid1,
                spower: 0,
                expired_at: 1303,
                calculated_at: 303,
                amount: 155, // ( 1000 * 1 + 0 ) * 0.2
                prepaid: 0,
                reported_replica_count: 1,
                remaining_paid_count: 3,
                replicas: BTreeMap::from_iter(vec![(merchant.clone(), Replica {
                    who: merchant.clone(),
                    valid_at: 303,
                    anchor: legal_pk.clone(),
                    is_reported: true,
                    created_at: Some(303)
                })])
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

        assert_eq!(Market::filesv2(&cid1).unwrap_or_default(),
            FileInfoV2 {
                file_size: reported_file_size_cid1,
                spower: 0,
                expired_at: 1303,
                calculated_at: 303,
                amount: 335,
                prepaid: 0,
                reported_replica_count: 1,
                remaining_paid_count: 3,
                replicas: BTreeMap::from_iter(vec![(merchant.clone(), Replica {
                    who: merchant.clone(),
                    valid_at: 303,
                    anchor: legal_pk.clone(),
                    is_reported: true,
                    created_at: Some(303)
                })])
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

        assert_eq!(Market::filesv2(&cid).unwrap_or_default(),
            FileInfoV2 {
                file_size,
                spower: 0,
                expired_at: 0,
                calculated_at: 50,
                amount: 23220,
                prepaid: 0,
                reported_replica_count: 0,
                remaining_paid_count: 4,
                replicas: BTreeMap::from_iter(vec![].into_iter())
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

        assert_eq!(Market::filesv2(&cid).unwrap_or_default(),
            FileInfoV2 {
                file_size,
                spower: 0,
                expired_at: 1303,
                calculated_at: 303,
                amount: 19995,
                prepaid: 0,
                reported_replica_count: 1,
                remaining_paid_count: 3,
                replicas: BTreeMap::from_iter(vec![(merchant.clone(), Replica {
                    who: merchant.clone(),
                    valid_at: 303,
                    anchor: legal_pk.clone(),
                    is_reported: true,
                    created_at: Some(303)
                })])
            }
        );

        run_to_block(503);
        <swork::ReportedInSlot>::insert(legal_pk.clone(), 0, true);
        Market::update_replicas(&cid, System::block_number().try_into().unwrap());
        assert_eq!(Market::filesv2(&cid).unwrap_or_default(),
            FileInfoV2 {
                file_size,
                spower: Market::calculate_spower(file_size, 1),
                expired_at: 1303,
                calculated_at: 503,
                amount: 19995,
                prepaid: 0,
                reported_replica_count: 1,
                remaining_paid_count: 3,
                replicas: BTreeMap::from_iter(vec![(merchant.clone(), Replica {
                    who: merchant.clone(),
                    valid_at: 303,
                    anchor: legal_pk.clone(),
                    is_reported: true,
                    created_at: Some(303)
                })])
            }
        );
        assert_eq!(merchant_ledgers(&merchant), MockMerchantLedger {
            collateral: 6_000_000,
            reward: 3225
        });

        add_who_into_replica(&cid, file_size, charlie.clone(), charlie.clone(), legal_pk.clone(), None, None);
        assert_eq!(Market::filesv2(&cid).unwrap_or_default(),
            FileInfoV2 {
                file_size,
                spower: Market::calculate_spower(file_size, 1),
                expired_at: 1303,
                calculated_at: 503,
                amount: 16770,
                prepaid: 0,
                reported_replica_count: 2,
                remaining_paid_count: 2,
                replicas: BTreeMap::from_iter(vec![
                    (merchant.clone(), Replica {
                        who: merchant.clone(),
                        valid_at: 303,
                        anchor: legal_pk.clone(),
                        is_reported: true,
                        created_at: Some(303)
                    }),
                    (charlie.clone(), Replica {
                        who: charlie.clone(),
                        valid_at: 503,
                        anchor: legal_pk.clone(),
                        is_reported: true,
                        created_at: Some(503)
                    })])
            }
        );

        run_to_block(1803);
        <swork::ReportedInSlot>::insert(legal_pk.clone(), 1500, true);
        assert_ok!(Market::place_storage_order(
            Origin::signed(source), cid.clone(),
            file_size, 0, vec![]
        ));
        assert_eq!(Market::filesv2(&cid).unwrap_or_default(),
            FileInfoV2 {
                file_size,
                spower: Market::calculate_spower(file_size, 1),
                expired_at: 2803,
                calculated_at: 503,
                amount: 39990,
                prepaid: 0,
                reported_replica_count: 2,
                remaining_paid_count: 2,
                replicas: BTreeMap::from_iter(vec![
                    (merchant.clone(), Replica {
                        who: merchant.clone(),
                        valid_at: 303,
                        anchor: legal_pk.clone(),
                        is_reported: true,
                        created_at: Some(303)
                    }),
                    (charlie.clone(), Replica {
                        who: charlie.clone(),
                        valid_at: 503,
                        anchor: legal_pk.clone(),
                        is_reported: true,
                        created_at: Some(503)
                    })])
            }
        );

        assert_eq!(merchant_ledgers(&merchant), MockMerchantLedger {
            collateral: 6_000_000,
            reward: 3225
        });
        assert_eq!(merchant_ledgers(&charlie), MockMerchantLedger {
            collateral: 6_000_000,
            reward: 3225
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

        assert_eq!(Market::filesv2(&cid).unwrap_or_default(),
            FileInfoV2 {
                file_size,
                spower: 0,
                expired_at: 0,
                calculated_at: 50,
                amount: 23220,
                prepaid: 0,
                reported_replica_count: 0,
                remaining_paid_count: 4,
                replicas: BTreeMap::from_iter(vec![].into_iter())
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

        assert_eq!(Market::filesv2(&cid).unwrap_or_default(),
            FileInfoV2 {
                file_size,
                spower: 0,
                expired_at: 1303,
                calculated_at: 303,
                amount: 19995,
                prepaid: 0,
                reported_replica_count: 1,
                remaining_paid_count: 3,
                replicas: BTreeMap::from_iter(vec![(merchant.clone(), Replica {
                    who: merchant.clone(),
                    valid_at: 303,
                    anchor: legal_pk.clone(),
                    is_reported: true,
                    created_at: Some(303)
                })])
            }
        );

        run_to_block(503);
        <swork::ReportedInSlot>::insert(legal_pk.clone(), 0, true);
        Market::update_replicas(&cid, System::block_number().try_into().unwrap());
        assert_eq!(Market::filesv2(&cid).unwrap_or_default(),
            FileInfoV2 {
                file_size,
                spower: Market::calculate_spower(file_size, 1),
                expired_at: 1303,
                calculated_at: 503,
                amount: 19995,
                prepaid: 0,
                reported_replica_count: 1,
                remaining_paid_count: 3,
                replicas: BTreeMap::from_iter(vec![(merchant.clone(), Replica {
                    who: merchant.clone(),
                    valid_at: 303,
                    anchor: legal_pk.clone(),
                    is_reported: true,
                    created_at: Some(303)
                })])
            }
        );
        assert_eq!(merchant_ledgers(&merchant), MockMerchantLedger {
            collateral: 6_000_000,
            reward: 3225
        });

        add_who_into_replica(&cid, file_size, charlie.clone(), charlie.clone(), legal_pk.clone(), None, None);
        assert_eq!(Market::filesv2(&cid).unwrap_or_default(),
            FileInfoV2 {
                file_size,
                spower: Market::calculate_spower(file_size, 1),
                expired_at: 1303,
                calculated_at: 503,
                amount: 16770,
                prepaid: 0,
                reported_replica_count: 2,
                remaining_paid_count: 2,
                replicas: BTreeMap::from_iter(vec![
                    (merchant.clone(), Replica {
                        who: merchant.clone(),
                        valid_at: 303,
                        anchor: legal_pk.clone(),
                        is_reported: true,
                        created_at: Some(303)
                    }),
                    (charlie.clone(), Replica {
                        who: charlie.clone(),
                        valid_at: 503,
                        anchor: legal_pk.clone(),
                        is_reported: true,
                        created_at: Some(503)
                    })])
            }
        );
        Market::delete_replica(&merchant, merchant.clone(), &cid, &legal_pk);
        Market::delete_replica(&charlie, charlie.clone(), &cid, &legal_pk);

        run_to_block(903);
        Market::update_replicas(&cid, System::block_number().try_into().unwrap());
        // calculated_at should be updated
        assert_eq!(Market::filesv2(&cid).unwrap_or_default(),
            FileInfoV2 {
                file_size,
                spower: 0,
                expired_at: 1303,
                calculated_at: 903,
                amount: 16770,
                prepaid: 0,
                reported_replica_count: 0,
                remaining_paid_count: 2,
                replicas: BTreeMap::from_iter(vec![].into_iter())
            }
        );

        run_to_block(1203);

        assert_ok!(Market::place_storage_order(
            Origin::signed(source), cid.clone(),
            file_size, 0, vec![]
        ));
        assert_eq!(Market::filesv2(&cid).unwrap_or_default(),
            FileInfoV2 {
                file_size,
                spower: 0,
                expired_at: 0,
                calculated_at: 903,
                amount: 39990,
                prepaid: 0,
                reported_replica_count: 0,
                remaining_paid_count: 2,
                replicas: BTreeMap::from_iter(vec![].into_iter())
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

        assert_eq!(Market::filesv2(&cid).unwrap_or_default(),
            FileInfoV2 {
                file_size,
                spower: 0,
                expired_at: 0,
                calculated_at: 50,
                amount: 23220,
                prepaid: 0,
                reported_replica_count: 0,
                remaining_paid_count: 4,
                replicas: BTreeMap::from_iter(vec![].into_iter())
            }
        );

        run_to_block(303);

        let legal_wr_info = legal_work_report_with_added_files();
        let legal_pk = legal_wr_info.curr_pk.clone();

        <swork::ReportedInSlot>::insert(legal_pk.clone(), 0, true);
        for index in 0..10 {
            add_who_into_replica(&cid, file_size, AccountId32::new([index as u8; 32]), AccountId32::new([index as u8; 32]), legal_pk.clone(), Some(303u32), None);
        }

        Market::update_replicas(&cid, System::block_number().try_into().unwrap());
        assert_eq!(Market::filesv2(&cid).unwrap_or_default().spower, Market::calculate_spower(file_size, 10));
        assert_eq!(Market::filesv2(&cid).unwrap_or_default().reported_replica_count, 10);

        for index in 10..20 {
            add_who_into_replica(&cid, file_size, AccountId32::new([index as u8; 32]), AccountId32::new([index as u8; 32]), legal_pk.clone(), Some(303u32), None);
        }
        Market::update_replicas(&cid, System::block_number().try_into().unwrap());
        assert_eq!(Market::filesv2(&cid).unwrap_or_default().spower, Market::calculate_spower(file_size, 20));
        assert_eq!(Market::filesv2(&cid).unwrap_or_default().reported_replica_count, 20);
        for index in 20..220 {
            add_who_into_replica(&cid, file_size, AccountId32::new([index as u8; 32]), AccountId32::new([index as u8; 32]), legal_pk.clone(), Some(303u32), None);
        }
        Market::update_replicas(&cid, System::block_number().try_into().unwrap());
        assert_eq!(Market::filesv2(&cid).unwrap_or_default().spower, Market::calculate_spower(file_size, 200));
        assert_eq!(Market::filesv2(&cid).unwrap_or_default().reported_replica_count, 200);
        for index in 0..140 {
            Market::delete_replica(&AccountId32::new([index as u8; 32]), AccountId32::new([index as u8; 32]), &cid, &legal_pk);
        }
        Market::update_replicas(&cid, System::block_number().try_into().unwrap());
        assert_eq!(Market::filesv2(&cid).unwrap_or_default().spower, Market::calculate_spower(file_size, 60));
        assert_eq!(Market::filesv2(&cid).unwrap_or_default().reported_replica_count, 60);
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

        assert_eq!(Market::filesv2(&cid).unwrap_or_default(),
            FileInfoV2 {
                file_size,
                spower: 0,
                expired_at: 0,
                calculated_at: 50,
                amount: 23220,
                prepaid: 0,
                reported_replica_count: 0,
                remaining_paid_count: 4,
                replicas: BTreeMap::from_iter(vec![].into_iter())
            }
        );

        run_to_block(303);

        let mut expected_groups = BTreeMap::new();
        for i in 10..28 {
            let key = hex::decode(i.to_string()).unwrap();
            add_who_into_replica(&cid, file_size, AccountId32::new([i as u8; 32]), AccountId32::new([i as u8; 32]), key.clone(), Some(303u32), None);
            <swork::ReportedInSlot>::insert(key.clone(), 0, true);
            expected_groups.insert(key.clone(), true);
        }
        add_who_into_replica(&cid, file_size, bob.clone(), bob.clone(), hex::decode("29").unwrap(), Some(303u32), None);
        <swork::ReportedInSlot>::insert(hex::decode("29").unwrap(), 0, true);
        add_who_into_replica(&cid, file_size, charlie.clone(), charlie.clone(), hex::decode("30").unwrap(), Some(303u32), None);
        <swork::ReportedInSlot>::insert(hex::decode("30").unwrap(), 0, true);
        Market::update_replicas(&cid, System::block_number().try_into().unwrap());
        assert_eq!(Market::filesv2(&cid).unwrap_or_default().spower, Market::calculate_spower(file_size, 20));
        assert_eq!(Market::filesv2(&cid).unwrap_or_default().reported_replica_count, 20);
        Market::delete_replica(&AccountId32::new([10u8; 32]), AccountId32::new([10u8; 32]), &cid, &hex::decode("10").unwrap());
        expected_groups.remove(&hex::decode("10").unwrap());
        Market::update_replicas(&cid, System::block_number().try_into().unwrap());
        assert_eq!(Market::filesv2(&cid).unwrap_or_default().spower, Market::calculate_spower(file_size, 19));
        assert_eq!(Market::filesv2(&cid).unwrap_or_default().reported_replica_count, 19);

        for i in 28..30 {
            let key = hex::decode(i.to_string()).unwrap();
            <swork::ReportedInSlot>::insert(key.clone(), 300, true);
            expected_groups.insert(key.clone(), true);
        }
        Market::delete_replica(&bob, bob.clone(), &cid, &hex::decode("29").unwrap()); // delete 21. 21 won't be deleted twice.
        Market::update_replicas(&cid, System::block_number().try_into().unwrap());
        assert_eq!(Market::filesv2(&cid).unwrap_or_default().spower, Market::calculate_spower(file_size, 18));
        assert_eq!(Market::filesv2(&cid).unwrap_or_default().reported_replica_count, 18);
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
        // all would be rewarded to liquidator charlie
        assert_ok!(Market::calculate_reward(Origin::signed(charlie.clone()), cid.clone()));
        assert_eq!(Balances::free_balance(&charlie), 19995);
        assert_eq!(Market::filesv2(&cid).is_none(), true);
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
        assert_eq!(Market::filesv2(&cid).unwrap_or_default(),
            FileInfoV2 {
                file_size,
                spower: 0,
                expired_at: 1303,
                calculated_at: 303,
                amount: 19995,
                prepaid: 400_000,
                reported_replica_count: 1,
                remaining_paid_count: 3,
                replicas: BTreeMap::from_iter(vec![(merchant.clone(), Replica {
                    who: merchant.clone(),
                    valid_at: 303,
                    anchor: legal_pk.clone(),
                    is_reported: true,
                    created_at: Some(303)
                })])
            }
        );

        run_to_block(2503);
        // all would be rewarded to liquidator charlie
        assert_ok!(Market::calculate_reward(Origin::signed(charlie.clone()), cid.clone()));

        assert_eq!(Market::filesv2(&cid).unwrap_or_default(),
            FileInfoV2 {
                file_size,
                spower: 0,
                expired_at: 3503,
                calculated_at: 2503,
                amount: 23220, // 23220
                prepaid: 270000,
                reported_replica_count: 0,
                remaining_paid_count: 3,
                replicas: BTreeMap::from_iter(vec![(merchant.clone(), Replica {
                    who: merchant.clone(),
                    valid_at: 2503,
                    anchor: legal_pk.clone(),
                    is_reported: false,
                    created_at: Some(303)
                })])
            }
        );


        assert_eq!(Balances::free_balance(&charlie), 19995);
        assert_eq!(Balances::free_balance(&reserved_pot), 27800);

        run_to_block(8000); // expired_on 2303 => all reward to liquidator charlie
        assert_ok!(Market::calculate_reward(Origin::signed(charlie.clone()), cid.clone()));
        assert_eq!(Balances::free_balance(&charlie), 43215); // 19995 + 23220
        assert_eq!(Market::filesv2(&cid).unwrap_or_default(),
            FileInfoV2 {
                file_size,
                spower: Market::calculate_spower(file_size, 0),
                expired_at: 9000,
                calculated_at: 8000,
                amount: 23220,
                prepaid: 140000,
                reported_replica_count: 0,
                remaining_paid_count: 3,
                replicas: BTreeMap::from_iter(vec![(merchant.clone(), Replica {
                    who: merchant.clone(),
                    valid_at: 8000,
                    anchor: legal_pk.clone(),
                    is_reported: false,
                    created_at: Some(303)
                })])
            }
        );
        assert_eq!(Balances::free_balance(&reserved_pot), 41700);
        assert_eq!(Balances::free_balance(&charlie), 43215); // 19995 + 23220
        run_to_block(10000); // expired_on 3303 => all reward to liquidator charlie
        assert_ok!(Market::calculate_reward(Origin::signed(charlie.clone()), cid.clone()));
        assert_eq!(Balances::free_balance(&charlie), 66435); // 19995 + 23220 + 23220

        assert_eq!(Market::filesv2(&cid).unwrap_or_default(),
            FileInfoV2 {
                file_size,
                spower: Market::calculate_spower(file_size, 0),
                expired_at: 11000,
                calculated_at: 10000,
                amount: 23220,
                prepaid: 10000,
                reported_replica_count: 0,
                remaining_paid_count: 3,
                replicas: BTreeMap::from_iter(vec![(merchant.clone(), Replica {
                    who: merchant.clone(),
                    valid_at: 10000,
                    anchor: legal_pk.clone(),
                    is_reported: false,
                    created_at: Some(303)
                })])
            }
        );

        assert_eq!(Balances::free_balance(&reserved_pot), 55600); // 41700 + 13900
        run_to_block(11000); // expired_on 3303 => all reward to liquidator charlie
        assert_ok!(Market::calculate_reward(Origin::signed(charlie.clone()), cid.clone()));
        assert_eq!(Balances::free_balance(&charlie), 89655); // 19995 + 23220 + 23220 + 23220

        assert_eq!(Market::filesv2(&cid).is_none(), true);
        assert_eq!(Balances::free_balance(&reserved_pot), 65600); // 55600 + 10000
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
        assert_eq!(Market::filesv2(&cid).unwrap_or_default(),
            FileInfoV2 {
                file_size,
                spower: 0,
                expired_at: 1303,
                calculated_at: 303,
                amount: 19995,
                prepaid: 400_000,
                reported_replica_count: 1,
                remaining_paid_count: 3,
                replicas: BTreeMap::from_iter(vec![(merchant.clone(), Replica {
                    who: merchant.clone(),
                    valid_at: 303,
                    anchor: legal_pk.clone(),
                    is_reported: true,
                    created_at: Some(303)
                })])
            }
        );

        run_to_block(503);
        // 20% would be rewarded to liquidator charlie
        assert_ok!(Market::calculate_reward(Origin::signed(charlie.clone()), cid.clone()));

        assert_eq!(Market::filesv2(&cid).unwrap_or_default(),
            FileInfoV2 {
                file_size,
                spower: 0,
                expired_at: 1303,
                calculated_at: 503,
                amount: 19995,
                prepaid: 400_000,
                reported_replica_count: 0,
                remaining_paid_count: 3,
                replicas: BTreeMap::from_iter(vec![(merchant.clone(), Replica {
                    who: merchant.clone(),
                    valid_at: 503,
                    anchor: legal_pk.clone(),
                    is_reported: false,
                    created_at: Some(303)
                })])
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

        add_collateral(&merchant, 60_000);

        // Change base fee to 10000
        assert_ok!(Market::set_base_fee(Origin::root(), 50000));

        assert_ok!(Market::place_storage_order(
            Origin::signed(source.clone()), cid.clone(),
            file_size, 0, vec![]
        ));

        assert_eq!(Market::filesv2(&cid).unwrap_or_default(),
            FileInfoV2 {
                file_size,
                spower: 0,
                expired_at: 0,
                calculated_at: 50,
                amount: 23220,
                prepaid: 0,
                reported_replica_count: 0,
                remaining_paid_count: 4,
                replicas: BTreeMap::from_iter(vec![].into_iter())
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
        assert_eq!(Market::filesv2(&cid).unwrap_or_default(),
            FileInfoV2 {
                file_size,
                spower: 0,
                expired_at: 1303,
                calculated_at: 303,
                amount: 19995,
                prepaid: 200_000,
                reported_replica_count: 1,
                remaining_paid_count: 3,
                replicas: BTreeMap::from_iter(vec![(merchant.clone(), Replica {
                    who: merchant.clone(),
                    valid_at: 303,
                    anchor: legal_pk.clone(),
                    is_reported: true,
                    created_at: Some(303)
                })])
            }
        );
        run_to_block(2503);
        // 20% would be rewarded to liquidator charlie
        assert_ok!(Market::calculate_reward(Origin::signed(source.clone()), cid.clone()));

        assert_eq!(Market::filesv2(&cid).unwrap_or_default(),
            FileInfoV2 {
                file_size,
                spower: Market::calculate_spower(file_size, 0),
                expired_at: 3503,
                calculated_at: 2503,
                amount: 23220, // 23_220
                prepaid: 21000, // 200000 -129000 - 50000
                reported_replica_count: 0,
                remaining_paid_count: 3,
                replicas: BTreeMap::from_iter(vec![(merchant.clone(), Replica {
                    who: merchant.clone(),
                    valid_at: 2503,
                    anchor: legal_pk.clone(),
                    is_reported: false,
                    created_at: Some(303)
                })])
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
        // all would be rewarded to liquidator charlie
        assert_ok!(Market::calculate_reward(Origin::signed(charlie.clone()), cid.clone()));
        assert_eq!(Balances::free_balance(&storage_pot), 296446); // 423221 - 1000 - 105780 (129000 * 0.82) - 19995

        run_to_block(8000); // expired_on 6000 => all reward to liquidator charlie
        assert_ok!(Market::calculate_reward(Origin::signed(charlie.clone()), cid.clone()));
        assert_eq!(Balances::free_balance(&storage_pot), 166446); // 296446 - 1000 - 105780 (129000 * 0.82) - 23220 (100%)

        run_to_block(9000);
        assert_ok!(Market::calculate_reward(Origin::signed(charlie.clone()), cid.clone()));
        assert_eq!(Balances::free_balance(&storage_pot), 36446); // 166446 - 1000 - 105780 (129000 * 0.82) - 23220 (100%)

        run_to_block(10000); // expired_on 6000 => all reward to liquidator charlie
        assert_ok!(Market::calculate_reward(Origin::signed(charlie.clone()), cid.clone()));
        assert_eq!(Balances::free_balance(&storage_pot), 3226); // 3225 for merchant + 1
        assert_eq!(Balances::free_balance(&reserved_pot), 65600); // 13900 * 4 + 10000
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

        add_who_into_replica(&cid, file_size, ferdie.clone(), zikun.clone(), legal_pk.clone(), Some(303u32), None);
        add_who_into_replica(&cid, file_size, charlie.clone(), bob.clone(), legal_pk.clone(), Some(403u32), None);
        add_who_into_replica(&cid, file_size, dave.clone(), bob.clone(), legal_pk.clone(), Some(503u32), None);

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

        add_who_into_replica(&cid, file_size, eve.clone(), zikun.clone(), legal_pk.clone(), Some(503u32), None);

        run_to_block(503);
        <swork::ReportedInSlot>::insert(legal_pk.clone(), 0, true);
        Market::update_replicas(&cid, System::block_number().try_into().unwrap());

        assert_eq!(merchant_ledgers(&bob), MockMerchantLedger {
            collateral: 6_000_000,
            reward: 3225 // 3225 * 1
        });

        assert_eq!(merchant_ledgers(&zikun), MockMerchantLedger {
            collateral: 6_000_000,
            reward: 3225 // 3225 * 1
        });

    });
}

// #[test]
// fn place_storage_order_with_discount_should_work() {
//     new_test_ext().execute_with(|| {
//         // generate 50 blocks first
//         run_to_block(50);
//         set_discount_ratio(1, 20); // 5% discount

//         let source = ALICE;
//         let merchant = MERCHANT;

//         let cid =
//             hex::decode("4e2883ddcbc77cf19979770d756fd332d0c8f815f9de646636169e460e6af6ff").unwrap();
//         let file_size = 100; // should less than
//         let reserved_pot = Market::reserved_pot();
//         let staking_pot = Market::staking_pot();
//         let storage_pot = Market::storage_pot();
//         assert_eq!(Balances::free_balance(&staking_pot), 0);
//         let _ = Balances::make_free_balance_be(&source, 10000);
//         let _ = Balances::make_free_balance_be(&merchant, 200);

//         mock_bond_owner(&merchant, &merchant);

//         <FileKeysCountFee<Test>>::put(1000);
//         assert_ok!(Market::place_storage_order(
//             Origin::signed(source.clone()), cid.clone(),
//             file_size, 0, vec![]
//         ));
//         assert_eq!(Market::filesv2(&cid).unwrap_or_default(),
//             FileInfoV2 {
//                 file_size,
//                 spower: 0,
//                 expired_at: 0,
//                 calculated_at: 50,
//                 amount: 360, // ( 1000 * 1 + 0 + 1000 ) * 0.18
//                 prepaid: 0,
//                 reported_replica_count: 0,
//                 replicas: vec![]
//             }
//         );
//         assert_eq!(Balances::free_balance(&reserved_pot), 1050);
//         assert_eq!(Balances::free_balance(&staking_pot), 1440);
//         assert_eq!(Balances::free_balance(&storage_pot), 360);
//         assert_eq!(Balances::free_balance(&source), 7150);

//         set_discount_ratio(1, 10); // 10% discount

//         assert_ok!(Market::place_storage_order(
//             Origin::signed(source.clone()), cid.clone(),
//             file_size, 0, vec![]
//         ));
//         assert_eq!(Market::filesv2(&cid).unwrap_or_default(),
//             FileInfoV2 {
//                 file_size,
//                 spower: 0,
//                 expired_at: 0,
//                 calculated_at: 50,
//                 amount: 720, // ( 1000 + 1000 * 1 + 0 + 1000 ) * 0.18
//                 prepaid: 0,
//                 reported_replica_count: 0,
//                 replicas: vec![]
//             }
//         );
//         assert_eq!(Balances::free_balance(&reserved_pot), 1950); // 150 + 0
//         assert_eq!(Balances::free_balance(&staking_pot), 2880);
//         assert_eq!(Balances::free_balance(&storage_pot), 720);
//         assert_eq!(Balances::free_balance(&source), 4450);


//         set_discount_ratio(1, 5); // 10% discount

//         assert_ok!(Market::place_storage_order(
//             Origin::signed(source.clone()), cid.clone(),
//             file_size, 0, vec![]
//         ));
//         assert_eq!(Market::filesv2(&cid).unwrap_or_default(),
//            FileInfoV2 {
//                file_size,
//                spower: 0,
//                expired_at: 0,
//                calculated_at: 50,
//                amount: 1080, // ( 1000 + 1000 * 1 + 0 + 1000 ) * 0.18
//                prepaid: 0,
//                reported_replica_count: 0,
//                replicas: vec![]
//            }
//         );
//         assert_eq!(Balances::free_balance(reserved_pot), 2850); // 150 + 0
//         assert_eq!(Balances::free_balance(staking_pot), 4320);
//         assert_eq!(Balances::free_balance(storage_pot), 1080);
//         assert_eq!(Balances::free_balance(&source), 1750);
//     });
// }

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

        assert_eq!(Market::filesv2(&cid).unwrap_or_default(),
           FileInfoV2 {
               file_size,
               spower: 0,
               expired_at: 0,
               calculated_at: 50,
               amount: 23220,
               prepaid: 0,
               reported_replica_count: 0,
               remaining_paid_count: 4,
               replicas: BTreeMap::from_iter(vec![].into_iter())
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

        assert_eq!(Market::filesv2(&cid).unwrap_or_default(),
            FileInfoV2 {
                file_size,
                spower: 0,
                expired_at: 1303,
                calculated_at: 303,
                amount: 19995,
                prepaid: 0,
                reported_replica_count: 1,
                remaining_paid_count: 3,
                replicas: BTreeMap::from_iter(vec![(merchant.clone(), Replica {
                    who: merchant.clone(),
                    valid_at: 303,
                    anchor: legal_pk.clone(),
                    is_reported: true,
                    created_at: Some(303)
                })])
            }
        );

        run_to_block(503);
        <swork::ReportedInSlot>::insert(legal_pk.clone(), 0, true);
        Market::update_replicas(&cid, System::block_number().try_into().unwrap());
        assert_eq!(Market::filesv2(&cid).unwrap_or_default(),
           FileInfoV2 {
                file_size,
                spower: Market::calculate_spower(file_size, 1),
                expired_at: 1303,
                calculated_at: 503,
                amount: 19995,
                prepaid: 0,
                reported_replica_count: 1,
                remaining_paid_count: 3,
                replicas: BTreeMap::from_iter(vec![(merchant.clone(), Replica {
                    who: merchant.clone(),
                    valid_at: 303,
                    anchor: legal_pk.clone(),
                    is_reported: true,
                    created_at: Some(303)
                })])
           }
        );
        assert_eq!(merchant_ledgers(&merchant), MockMerchantLedger {
            collateral: 6_000_000,
            reward: 3225
        });

        add_who_into_replica(&cid, file_size, charlie.clone(), charlie.clone(), legal_pk.clone(), None, None);

        run_to_block(603);
        <swork::ReportedInSlot>::insert(legal_pk.clone(), 300, true);
        Market::update_replicas(&cid, System::block_number().try_into().unwrap());
        assert_eq!(Market::filesv2(&cid).unwrap_or_default(),
           FileInfoV2 {
                file_size,
                spower: Market::calculate_spower(file_size, 1),
                expired_at: 1303,
                calculated_at: 603,
                amount: 16770,
                prepaid: 0,
                reported_replica_count: 2,
                remaining_paid_count: 2,
                replicas: BTreeMap::from_iter(vec![
                    (merchant.clone(), Replica {
                    who: merchant.clone(),
                    valid_at: 303,
                    anchor: legal_pk.clone(),
                    is_reported: true,
                    created_at: None
                    }),
                    (charlie.clone(), Replica {
                    who: charlie.clone(),
                    valid_at: 503,
                    anchor: legal_pk.clone(),
                    is_reported: true,
                    created_at: Some(503)
                    })
                ])
           }
        );

        add_who_into_replica(&cid, file_size, dave.clone(), dave.clone(), hex::decode("11").unwrap(), None, None);
        run_to_block(703);
        Market::update_replicas(&cid, System::block_number().try_into().unwrap());
        assert_eq!(Market::filesv2(&cid).unwrap_or_default(),
           FileInfoV2 {
               file_size,
               spower: Market::calculate_spower(file_size, 1),
               expired_at: 1303,
               calculated_at: 703,
               amount: 13545,
               prepaid: 0,
               reported_replica_count: 2,
               remaining_paid_count: 1,
                replicas: BTreeMap::from_iter(vec![
                    (merchant.clone(), Replica {
                    who: merchant.clone(),
                    valid_at: 303,
                    anchor: legal_pk.clone(),
                    is_reported: true,
                    created_at: None
                    }),
                    (charlie.clone(), Replica {
                    who: charlie.clone(),
                    valid_at: 503,
                    anchor: legal_pk.clone(),
                    is_reported: true,
                    created_at: Some(503)
                    }),
                    (dave.clone(), Replica {
                    who: dave.clone(),
                    valid_at: 703,
                    anchor: hex::decode("11").unwrap(),
                    is_reported: false,
                    created_at: Some(603)
                    })
                ])
           }
        );

        run_to_block(903);
        <swork::ReportedInSlot>::insert(hex::decode("11").unwrap(), 600, true);
        Market::update_replicas(&cid, System::block_number().try_into().unwrap());
        assert_eq!(Market::filesv2(&cid).unwrap_or_default(),
           FileInfoV2 {
               file_size,
               spower: Market::calculate_spower(file_size, 1),
               expired_at: 1303,
               calculated_at: 903,
               amount: 13545,
               prepaid: 0,
               reported_replica_count: 1,
               remaining_paid_count: 1,
                replicas: BTreeMap::from_iter(vec![
                    (merchant.clone(), Replica {
                    who: merchant.clone(),
                    valid_at: 903,
                    anchor: legal_pk.clone(),
                    is_reported: false,
                    created_at: None
                    }),
                    (charlie.clone(), Replica {
                    who: charlie.clone(),
                    valid_at: 903,
                    anchor: legal_pk.clone(),
                    is_reported: false,
                    created_at: None
                    }),
                    (dave.clone(), Replica {
                    who: dave.clone(),
                    valid_at: 703,
                    anchor: hex::decode("11").unwrap(),
                    is_reported: true,
                    created_at: None
                    })
                ])
           }
        );

        run_to_block(1203);
        <swork::ReportedInSlot>::insert(hex::decode("11").unwrap(), 900, true);
        <swork::ReportedInSlot>::insert(legal_pk.clone(), 900, true);
        Market::update_replicas(&cid, System::block_number().try_into().unwrap());
        assert_eq!(Market::filesv2(&cid).unwrap_or_default(),
           FileInfoV2 {
               file_size,
               spower: Market::calculate_spower(file_size, 1),
               expired_at: 1303,
               calculated_at: 1203,
               amount: 13545,
               prepaid: 0,
               reported_replica_count: 3,
               remaining_paid_count: 1,
                replicas: BTreeMap::from_iter(vec![
                    (merchant.clone(), Replica {
                    who: merchant.clone(),
                    valid_at: 903,
                    anchor: legal_pk.clone(),
                    is_reported: true,
                    created_at: None
                    }),
                    (charlie.clone(), Replica {
                    who: charlie.clone(),
                    valid_at: 903,
                    anchor: legal_pk.clone(),
                    is_reported: true,
                    created_at: None
                    }),
                    (dave.clone(), Replica {
                    who: dave.clone(),
                    valid_at: 703,
                    anchor: hex::decode("11").unwrap(),
                    is_reported: true,
                    created_at: None
                    })
                ])
           }
        );
    });
}

// TODO
// 1. add_files_into_v1 => done in swork module
// 2. delete_files_from_v1 => done in swork module
// 3. illegal_files_with_v1
// 4. added_and_deleted_test
// 5. migration test
//    1. spower_delay_with_migration => donw in swork module
//    2. file_close with migration => donw in swork module
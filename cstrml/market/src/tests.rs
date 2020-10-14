use super::*;

use crate::mock::*;
use frame_support::{
    assert_noop, assert_ok,
    dispatch::DispatchError,
};
use hex;
use crate::{SorderStatus, SorderInfo, MerchantInfo, SorderPunishment};
use sp_core::H256;

#[test]
fn test_for_storage_order_should_work() {
    new_test_ext().execute_with(|| {
        // generate 50 blocks first
        run_to_block(50);

        let source = 0;
        let file_identifier =
        hex::decode("4e2883ddcbc77cf19979770d756fd332d0c8f815f9de646636169e460e6af6ff").unwrap();
        let merchant: u64 = 100;
        let client: u64 = 0;
        let file_size = 16; // should less than merchant
        let duration = 10;
        let fee: u64 = 1;
        let amount: u64 = 10;
        let address_info = "ws://127.0.0.1:8855".as_bytes().to_vec();
        let file_alias = "/test/file1".as_bytes().to_vec();
        let _ = Balances::make_free_balance_be(&source, 60);

        // 1. Normal flow, aka happy pass 😃
        let _ = Balances::make_free_balance_be(&merchant, 60);
        assert_ok!(Market::pledge(Origin::signed(merchant.clone()), 60));
        assert_ok!(Market::cut_pledge(Origin::signed(merchant.clone()), 60));
        assert!(!<Pledges<Test>>::contains_key(merchant.clone()));
        assert_eq!(Balances::locks(merchant.clone()).len(), 0);

        assert_ok!(Market::pledge(Origin::signed(merchant.clone()), 60));
        assert_ok!(Market::register(Origin::signed(merchant.clone()), address_info.clone(), fee));

        assert_noop!(
            Market::place_storage_order(
                Origin::signed(merchant.clone()), merchant.clone(),
                file_identifier.clone(), file_size, duration, file_alias.clone()
            ),
            DispatchError::Module {
                index: 0,
                error: 9,
                message: Some("PlaceSelfOrder"),
            }
        );
        assert_ok!(Market::place_storage_order(
            Origin::signed(source), merchant,
            file_identifier.clone(), file_size, duration, file_alias.clone()
        ));

        let order_id = H256::default();
        assert_eq!(Market::merchants(&merchant).unwrap(), MerchantInfo {
            address_info,
            storage_price: fee,
            file_map: vec![(file_identifier.clone(), vec![order_id.clone()])].into_iter().collect()
        });
        assert_eq!(Market::clients(&client, file_alias).unwrap(), vec![order_id.clone()]);
        assert_eq!(Market::sorder_infos(&order_id).unwrap(), SorderInfo {
            file_identifier: file_identifier.clone(),
            file_size: 16,
            created_on: 50,
            merchant,
            client,
            amount,
            duration: 100
        });
        assert_eq!(Market::sorder_statuses(&order_id).unwrap(), SorderStatus {
            completed_on: 50,
            expired_on: 50+10*10,
            status: OrderStatus::Pending,
            claimed_at: 50
        });

        // 2. Register after get order, address should update but others should not
        let another_address_info = "ws://127.0.0.1:9900".as_bytes().to_vec();
        let another_price: u64 = 2;
        assert_ok!(Market::register(Origin::signed(merchant.clone()), another_address_info.clone(), another_price));
        assert_eq!(Market::merchants(&merchant).unwrap(), MerchantInfo {
            address_info: another_address_info.clone(),
            storage_price: another_price,
            file_map: vec![(file_identifier.clone(), vec![order_id.clone()])].into_iter().collect()
        });
        assert_eq!(Market::pledges(merchant), Pledge {
            total: 60,
            used: amount
        });
    });
}

#[test]
fn test_for_storage_order_should_fail_due_to_file_size() {
    new_test_ext().execute_with(|| {
        // generate 50 blocks first
        run_to_block(50);

        let source = 0;
        let file_identifier =
        hex::decode("4e2883ddcbc77cf19979770d756fd332d0c8f815f9de646636169e460e6af6ff").unwrap();
        let merchant = 100;
        let file_size = 200; // should less than merchant
        let duration = 10;
        let fee = 1;
        let address = "ws://127.0.0.1:8855".as_bytes().to_vec();
        let file_alias = "/test/file1".as_bytes().to_vec();


        assert_ok!(Market::pledge(Origin::signed(merchant.clone()), 0));
        assert_ok!(Market::register(Origin::signed(merchant), address.clone(), fee));
        assert_noop!(
            Market::place_storage_order(
                Origin::signed(source.clone()), merchant,
                file_identifier, file_size, duration, file_alias
            ),
            DispatchError::Module {
                index: 0,
                error: 1,
                message: Some("NoWorkload"),
            }
        );
    });
}

#[test]
fn test_for_storage_order_should_fail_due_to_duration_too_short() {
    new_test_ext().execute_with(|| {
        // generate 50 blocks first
        run_to_block(50);

        let source = 0;
        let file_identifier =
            hex::decode("4e2883ddcbc77cf19979770d756fd332d0c8f815f9de646636169e460e6af6ff").unwrap();
        let merchant = 100;
        let file_size = 60; // should less than merchant
        let duration = 1; // shoule more than 1 minute
        let fee = 1;
        let address = "ws://127.0.0.1:8855".as_bytes().to_vec();
        let file_alias = "/test/file1".as_bytes().to_vec();

        assert_ok!(Market::pledge(Origin::signed(merchant.clone()), 0));
        assert_ok!(Market::register(Origin::signed(merchant), address.clone(), fee));
        assert_noop!(
            Market::place_storage_order(
                Origin::signed(source.clone()), merchant,
                file_identifier, file_size, duration, file_alias
            ),
            DispatchError::Module {
                index: 0,
                error: 3,
                message: Some("DurationTooShort"),
            }
        );
    });
}

#[test]
fn test_for_register_should_fail_due_to_low_price() {
    new_test_ext().execute_with(|| {
        // generate 50 blocks first
        run_to_block(50);

        let merchant = 100;
        let address = "ws://127.0.0.1:8855".as_bytes().to_vec();
        assert_ok!(Market::pledge(Origin::signed(merchant.clone()), 0));
        assert_noop!(
            Market::register(Origin::signed(merchant), address.clone(), 0),
            DispatchError::Module {
                index: 0,
                error: 10,
                message: Some("LowStoragePrice"),
            }
        );
    });
}

#[test]
fn test_for_storage_order_should_fail_due_to_wr_not_exist() {
    new_test_ext().execute_with(|| {
        // generate 50 blocks first
        run_to_block(50);

        let merchant = 400; // Invalid merchant. No work report
        let address = "ws://127.0.0.1:8855".as_bytes().to_vec();
        assert_ok!(Market::pledge(Origin::signed(merchant.clone()), 0));
        assert_noop!(
            Market::register(Origin::signed(merchant), address.clone(), 1),
            DispatchError::Module {
                index: 0,
                error: 1,
                message: Some("NoWorkload"),
            }
        );
    });
}

#[test]
fn test_for_storage_order_should_fail_due_to_merchant_not_register() {
    new_test_ext().execute_with(|| {
        // generate 50 blocks first
        run_to_block(50);

        let source = 0;
        let file_identifier =
            hex::decode("4e2883ddcbc77cf19979770d756fd332d0c8f815f9de646636169e460e6af6ff").unwrap();
        let merchant = 100;
        let file_size = 80; // should less than merchant
        let duration = 10;
        let file_alias = "/test/file1".as_bytes().to_vec();

        assert_noop!(
            Market::place_storage_order(
                Origin::signed(source.clone()), merchant,
                file_identifier, file_size, duration, file_alias
            ),
            DispatchError::Module {
                index: 0,
                error: 2,
                message: Some("NotMerchant"),
            }
        );
    });
}

#[test]
fn test_for_pledge_should_work() {
    new_test_ext().execute_with(|| {
        // generate 50 blocks first
        run_to_block(50);
        let merchant = 100;
        let address_info = "ws://127.0.0.1:8855".as_bytes().to_vec();
        let _ = Balances::make_free_balance_be(&merchant, 200);
        assert_ok!(Market::pledge(Origin::signed(merchant.clone()), 180));
        assert_ok!(Market::register(Origin::signed(merchant), address_info.clone(), 1));
        assert_eq!(Market::pledges(merchant), Pledge {
            total: 180,
            used: 0
        });
        assert_ok!(Market::cut_pledge(Origin::signed(merchant), 20));
        assert_eq!(Market::pledges(merchant), Pledge {
            total: 160,
            used: 0
        });
        assert_ok!(Market::pledge_extra(Origin::signed(merchant), 10));
        assert_eq!(Market::pledges(merchant), Pledge {
            total: 170,
            used: 0
        });
    });
}

#[test]
fn test_for_pledge_extra_should_fail_due_to_merchant_not_pledges() {
    new_test_ext().execute_with(|| {
        // generate 50 blocks first
        run_to_block(50);

        let merchant = 100;
        let _ = Balances::make_free_balance_be(&merchant, 200);
        assert_noop!(
            Market::pledge_extra(
                Origin::signed(merchant),
                200
            ),
            DispatchError::Module {
                index: 0,
                error: 7,
                message: Some("NotPledged")
            }
        );
    });
}

#[test]
fn test_for_pledge_should_fail_due_to_insufficient_currency() {
    new_test_ext().execute_with(|| {
        // generate 50 blocks first
        run_to_block(50);

        let merchant = 100;
        let _ = Balances::make_free_balance_be(&merchant, 100);
        assert_noop!(
            Market::pledge(
                Origin::signed(merchant),
                200
            ),
            DispatchError::Module {
                index: 0,
                error: 4,
                message: Some("InsufficientCurrency")
            }
        );
    });
}

#[test]
fn test_for_pledge_should_fail_due_to_insufficient_pledge() {
    new_test_ext().execute_with(|| {
        // generate 50 blocks first
        run_to_block(50);

        let source = 0;
        let file_identifier =
        hex::decode("4e2883ddcbc77cf19979770d756fd332d0c8f815f9de646636169e460e6af6ff").unwrap();
        let merchant = 100;
        let file_size = 16; // should less than merchant
        let duration = 50;
        let fee = 1;
        let amount: u64 = 50;
        let address_info = "ws://127.0.0.1:8855".as_bytes().to_vec();
        let file_alias = "/test/file1".as_bytes().to_vec();
        let _ = Balances::make_free_balance_be(&source, 80);
        let _ = Balances::make_free_balance_be(&merchant, 80);
        assert_ok!(Market::pledge(Origin::signed(merchant.clone()), 70));

        assert_ok!(Market::register(Origin::signed(merchant), address_info.clone(), fee));
        assert_ok!(Market::place_storage_order(
            Origin::signed(source), merchant,
            file_identifier.clone(), file_size, duration, file_alias
        ));
        assert_eq!(Market::pledges(merchant), Pledge {
            total: 70,
            used: amount
        });
        assert_ok!(Market::cut_pledge(Origin::signed(merchant), 20));
        assert_eq!(Market::pledges(merchant), Pledge {
            total: 50,
            used: amount
        });
        assert_noop!(
            Market::cut_pledge(
                Origin::signed(merchant),
                1
            ),
            DispatchError::Module {
                index: 0,
                error: 5,
                message: Some("InsufficientPledge")
            }
        );
    });
}

#[test]
fn test_for_storage_order_should_fail_due_to_insufficient_pledge() {
    new_test_ext().execute_with(|| {
        // generate 50 blocks first
        run_to_block(50);

        let source = 0;
        let file_identifier =
        hex::decode("4e2883ddcbc77cf19979770d756fd332d0c8f815f9de646636169e460e6af6ff").unwrap();
        let merchant = 100;
        let file_size = 16; // should less than merchant
        let duration = 60;
        let fee = 1;
        let amount: u64 = 60;
        let address_info = "ws://127.0.0.1:8855".as_bytes().to_vec();
        let file_alias = "/test/file1".as_bytes().to_vec();
        let _ = Balances::make_free_balance_be(&source, 80);
        let _ = Balances::make_free_balance_be(&merchant, 80);
        assert_ok!(Market::pledge(Origin::signed(merchant), 0));
        assert_ok!(Market::register(Origin::signed(merchant), address_info.clone(), fee));
        assert_noop!(
            Market::place_storage_order(
                Origin::signed(source.clone()), merchant,
                file_identifier.clone(), file_size, duration, file_alias.clone()
            ),
            DispatchError::Module {
                index: 0,
                error: 5,
                message: Some("InsufficientPledge"),
            }
        );

        assert_ok!(Market::pledge_extra(Origin::signed(merchant), 60));
        assert_eq!(Market::pledges(merchant), Pledge {
            total: 60,
            used: 0
        });
        assert_ok!(Market::place_storage_order(
            Origin::signed(source), merchant,
            file_identifier.clone(), file_size, duration, file_alias.clone()
        ));

        let order_id = H256::default();
        
        assert_eq!(Market::merchants(&merchant).unwrap(), MerchantInfo {
            address_info,
            storage_price: fee,
            file_map: vec![(file_identifier.clone(), vec![order_id.clone()])].into_iter().collect()
        });
        assert_eq!(Market::clients(&0, file_alias).unwrap(), vec![order_id.clone()]);
        assert_eq!(Market::sorder_infos(&order_id).unwrap(), SorderInfo {
            file_identifier: file_identifier.clone(),
            file_size: 16,
            created_on: 50,
            merchant,
            client: source.clone(),
            amount,
            duration: duration*10
        });
        assert_eq!(Market::sorder_statuses(&order_id).unwrap(), SorderStatus {
            completed_on: 50,
            expired_on: 50+duration*10,
            status: OrderStatus::default(),
            claimed_at: 50
        });
        assert_eq!(Market::pledges(merchant), Pledge {
            total: 60,
            used: 60
        });
    });
}

#[test]
fn test_for_pledge_should_fail_due_to_double_pledge() {
    new_test_ext().execute_with(|| {
        // generate 50 blocks first
        run_to_block(50);
        let merchant = 100;
        let _ = Balances::make_free_balance_be(&merchant, 200);
        assert_ok!(Market::pledge(Origin::signed(merchant.clone()), 70));
        assert_noop!(
            Market::pledge(
                Origin::signed(merchant),
                70
            ),
            DispatchError::Module {
                index: 0,
                error: 8,
                message: Some("AlreadyPledged")
            }
        );
    });
}

#[test]
fn test_for_pledge_should_work_without_register() {
    new_test_ext().execute_with(|| {
        // generate 50 blocks first
        run_to_block(50);
        let merchant = 100;
        let _ = Balances::make_free_balance_be(&merchant, 80);
        assert_ok!(Market::pledge(Origin::signed(merchant.clone()), 70));
        assert_noop!(
            Market::pledge_extra(
                Origin::signed(merchant.clone()),
                70
            ),
            DispatchError::Module {
                index: 0,
                error: 4,
                message: Some("InsufficientCurrency")
            }
        );
        assert_ok!(Market::pledge_extra(Origin::signed(merchant.clone()), 10));
        assert_ok!(Market::cut_pledge(Origin::signed(merchant.clone()), 70));
    });
}

#[test]
fn test_for_close_sorder() {
    new_test_ext().execute_with(|| {
        // generate 50 blocks first
        run_to_block(50);

        let source = 0;
        let file_identifier =
        hex::decode("4e2883ddcbc77cf19979770d756fd332d0c8f815f9de646636169e460e6af6ff").unwrap();
        let merchant: u64 = 100;
        let client: u64 = 0;
        let file_size = 16; // should less than merchant
        let duration = 10;
        let fee: u64 = 1;
        let amount: u64 = 10;
        let address_info = "ws://127.0.0.1:8855".as_bytes().to_vec();
        let file_alias = "/test/file1".as_bytes().to_vec();
        let _ = Balances::make_free_balance_be(&source, 60);

        // 1. Normal flow, aka happy pass 😃
        let _ = Balances::make_free_balance_be(&merchant, 200);
        assert_ok!(Market::pledge(Origin::signed(merchant.clone()), 60));
        assert_ok!(Market::register(Origin::signed(merchant.clone()), address_info.clone(), fee));
        assert_ok!(Market::place_storage_order(
            Origin::signed(source), merchant,
            file_identifier.clone(), file_size, duration, file_alias
        ));

        let order_id = H256::default();
        assert_eq!(Market::sorder_infos(&order_id).unwrap(), SorderInfo {
            file_identifier: file_identifier.clone(),
            file_size: 16,
            created_on: 50,
            merchant,
            client,
            amount,
            duration: 100
        });
        assert_eq!(Market::sorder_statuses(&order_id).unwrap(), SorderStatus {
            completed_on: 50,
            expired_on: 50+10*10,
            status: OrderStatus::Pending,
            claimed_at: 50
        });

        Market::close_sorder(&order_id, amount.clone());

        // storage order has been closed
        assert_eq!(Balances::free_balance(&merchant), 200);
        assert_eq!(Market::pledges(&merchant), Pledge {
            total: 60,
            used: 0
        });
        assert!(!<SorderInfos<Test>>::contains_key(&order_id));
        assert!(!<SorderStatuses<Test>>::contains_key(&order_id));
        assert!(!<SorderPunishments<Test>>::contains_key(&order_id));
        assert_eq!(Market::merchants(&merchant).unwrap(), MerchantInfo {
            address_info: address_info.clone(),
            storage_price: fee,
            file_map: vec![].into_iter().collect()
        });


        // delete it twice would not have bad effect
        Market::close_sorder(&order_id, amount);
        assert_eq!(Balances::free_balance(&merchant), 200);
        assert_eq!(Market::pledges(&merchant), Pledge {
            total: 60,
            used: 0
        });
        assert!(!<SorderInfos<Test>>::contains_key(&order_id));
        assert!(!<SorderStatuses<Test>>::contains_key(&order_id));
        assert!(!<SorderPunishments<Test>>::contains_key(&order_id));
    });
}

#[test]
fn test_scenario_for_file_alias_should_work() {
    new_test_ext().execute_with(|| {
        // generate 50 blocks first
        run_to_block(50);

        let source = 0;
        let file_identifier =
        hex::decode("4e2883ddcbc77cf19979770d756fd332d0c8f815f9de646636169e460e6af6ff").unwrap();
        let merchant: u64 = 100;
        let client: u64 = 0;
        let file_size = 16; // should less than merchant
        let duration = 10;
        let fee: u64 = 1;
        let amount: u64 = 10;
        let address_info = "ws://127.0.0.1:8855".as_bytes().to_vec();
        let file_alias = "/test/file1".as_bytes().to_vec();
        let _ = Balances::make_free_balance_be(&source, 60);

        // 1. Normal flow, aka happy pass 😃
        let _ = Balances::make_free_balance_be(&merchant, 60);
        assert_ok!(Market::pledge(Origin::signed(merchant.clone()), 60));
        assert_ok!(Market::cut_pledge(Origin::signed(merchant.clone()), 60));
        assert!(!<Pledges<Test>>::contains_key(merchant.clone()));
        assert_eq!(Balances::locks(merchant.clone()).len(), 0);

        assert_ok!(Market::pledge(Origin::signed(merchant.clone()), 60));
        assert_ok!(Market::register(Origin::signed(merchant.clone()), address_info.clone(), fee));

        assert_ok!(Market::place_storage_order(
            Origin::signed(source.clone()), merchant.clone(),
            file_identifier.clone(), file_size, duration, file_alias.clone()
        ));

        let order_id = H256::default();
        assert_eq!(Market::merchants(&merchant).unwrap(), MerchantInfo {
            address_info,
            storage_price: fee,
            file_map: vec![(file_identifier.clone(), vec![order_id.clone()])].into_iter().collect()
        });
        assert_eq!(Market::clients(&client, &file_alias).unwrap(), vec![order_id.clone()]);
        assert_eq!(Market::sorder_infos(&order_id).unwrap(), SorderInfo {
            file_identifier: file_identifier.clone(),
            file_size: 16,
            created_on: 50,
            merchant,
            client,
            amount,
            duration: 100
        });
        assert_eq!(Market::sorder_statuses(&order_id).unwrap(), SorderStatus {
            completed_on: 50,
            expired_on: 50+10*10,
            status: OrderStatus::Pending,
            claimed_at: 50
        });


        let new_file_alias = "/test/file2".as_bytes().to_vec();
        assert_noop!(
            Market::set_file_alias(
                Origin::signed(source.clone()), new_file_alias.clone(), file_alias.clone()
            ),
            DispatchError::Module {
                index: 0,
                error: 11,
                message: Some("InvalidFileAlias"),
            }
        );
        Market::set_file_alias(Origin::signed(source.clone()), file_alias.clone(), new_file_alias.clone()).unwrap();
        assert!(!<Clients<Test>>::contains_key(&client, &file_alias));
        assert_eq!(Market::clients(&client, &new_file_alias).unwrap(), vec![order_id.clone()]);
    });
}

#[test]
fn test_for_update_punishment_for_merchant() {
    new_test_ext().execute_with(|| {
        // generate 50 blocks first
        run_to_block(50);

        let source = 0;
        let file_identifier =
        hex::decode("4e2883ddcbc77cf19979770d756fd332d0c8f815f9de646636169e460e6af6ff").unwrap();
        let merchant: u64 = 100;
        let fee: u64 = 1;
        let address_info = "ws://127.0.0.1:8855".as_bytes().to_vec();
        let _ = Balances::make_free_balance_be(&source, 60);

        // 1. Normal flow, aka happy pass 😃
        let _ = Balances::make_free_balance_be(&merchant, 200);
        assert_ok!(Market::pledge(Origin::signed(merchant.clone()), 60));
        assert_ok!(Market::register(Origin::signed(merchant.clone()), address_info.clone(), fee));
        insert_sorder(&merchant, &file_identifier, 0, 1000, OrderStatus::Success);

        let order_id = Hash::repeat_byte(0);
        assert_eq!(Market::sorder_infos(&order_id).unwrap(), SorderInfo {
            file_identifier: file_identifier.clone(),
            file_size: 0,
            created_on: 0,
            merchant,
            client: 100,
            amount: 10,
            duration: 50
        });
        assert_eq!(Market::sorder_statuses(&order_id).unwrap(), SorderStatus {
            completed_on: 0,
            expired_on: 1000,
            status: OrderStatus::Success,
            claimed_at: 50
        });
        assert_eq!(Market::sorder_punishments(&order_id).unwrap(), SorderPunishment {
            success: 0,
            failed: 0,
            updated_at: 50
        });
        Market::update_sorder_punishment(&order_id, &100, &OrderStatus::Success);
        assert_eq!(Market::sorder_punishments(&order_id).unwrap(), SorderPunishment {
            success: 50,
            failed: 0,
            updated_at: 100
        });
        Market::update_sorder_punishment(&order_id, &120, &OrderStatus::Failed);
        assert_eq!(Market::sorder_punishments(&order_id).unwrap(), SorderPunishment {
            success: 50,
            failed: 20,
            updated_at: 120
        });
    });
}

#[test]
fn test_for_pay_sorders() {
    new_test_ext().execute_with(|| {
        // generate 50 blocks first
        run_to_block(50);

        let source = 0;
        let file_identifier =
        hex::decode("4e2883ddcbc77cf19979770d756fd332d0c8f815f9de646636169e460e6af6ff").unwrap();
        let merchant: u64 = 100;
        let client: u64 = 0;
        let file_size = 16; // should less than merchant
        let duration: u32 = 100;
        let fee: u64 = 1000;
        let amount: u64 = duration as u64 * fee;
        let address_info = "ws://127.0.0.1:8855".as_bytes().to_vec();
        let file_alias = "/test/file1".as_bytes().to_vec();
        let _ = Balances::make_free_balance_be(&source, 500000);

        // 1. Normal flow, aka happy pass 😃
        let _ = Balances::make_free_balance_be(&merchant, 500000);
        assert_ok!(Market::pledge(Origin::signed(merchant), 500000));
        assert_ok!(Market::register(Origin::signed(merchant), address_info.clone(), fee));
        assert_ok!(Market::place_storage_order(
            Origin::signed(source), merchant,
            file_identifier.clone(), file_size, duration, file_alias
        ));

        let order_id = H256::default();
        assert_eq!(Market::sorder_infos(&order_id).unwrap(), SorderInfo {
            file_identifier: file_identifier.clone(),
            file_size: 16,
            created_on: 50,
            merchant,
            client,
            amount,
            duration: duration*10
        });
        assert_eq!(Market::sorder_statuses(&order_id).unwrap(), SorderStatus {
            completed_on: 50,
            expired_on: 50+duration*10,
            status: OrderStatus::Pending,
            claimed_at: 50
        });
        let mut so = Market::sorder_statuses(&order_id).unwrap();
        so.status = OrderStatus::Success;
        <SorderStatuses<Test>>::insert(order_id.clone(), so);
        // 91% SLA
        Market::update_sorder_punishment(&order_id, &141, &OrderStatus::Success);
        Market::update_sorder_punishment(&order_id, &150, &OrderStatus::Failed);
        assert_eq!(Market::sorder_punishments(&order_id).unwrap(), SorderPunishment {
            success: 91,
            failed: 9,
            updated_at: 150
        });
        run_to_block(150);
        assert_eq!(Balances::free_balance(&merchant), 500000);
        assert_eq!(Balances::free_balance(&source), 400000);
        assert_ok!(Market::pay_sorders(Origin::signed(source), vec![order_id]));
        assert_eq!(Balances::free_balance(&merchant), 508000);
        assert_eq!(Balances::free_balance(&source), 402000);

        // 95% SLA
        Market::update_sorder_punishment(&order_id, &249, &OrderStatus::Success);
        Market::update_sorder_punishment(&order_id, &250, &OrderStatus::Failed);
        assert_eq!(Market::sorder_punishments(&order_id).unwrap(), SorderPunishment {
            success: 190,
            failed: 10,
            updated_at: 250
        });
        run_to_block(250);
        assert_eq!(Balances::free_balance(&merchant), 508000);
        assert_eq!(Balances::free_balance(&source), 402000);
        assert_ok!(Market::pay_sorders(Origin::signed(source), vec![order_id]));
        assert_eq!(Balances::free_balance(&merchant), 517000);
        assert_eq!(Balances::free_balance(&source), 403000);

        // ~50% SLA
        Market::update_sorder_punishment(&order_id, &450, &OrderStatus::Failed);
        assert_eq!(Market::sorder_punishments(&order_id).unwrap(), SorderPunishment {
            success: 190,
            failed: 210,
            updated_at: 450
        });
        run_to_block(450);
        assert_eq!(Balances::free_balance(&merchant), 517000);
        assert_eq!(Balances::free_balance(&source), 403000);
        assert_eq!(Market::pledges(&merchant), Pledge {
            total: 500000,
            used: 100000
        });
        assert_ok!(Market::pay_sorders(Origin::signed(source), vec![order_id]));
        // 50% pledge is slashed
        assert_eq!(Balances::free_balance(&merchant), 467000);
        assert_eq!(Balances::free_balance(&source), 483000);
        assert_eq!(Market::pledges(&merchant), Pledge {
            total: 450000,
            used: 0
        });
        assert!(!<SorderStatuses<Test>>::contains_key(&order_id));
        assert!(!<SorderInfos<Test>>::contains_key(&order_id));
        assert!(!<SorderPunishments<Test>>::contains_key(&order_id));
    });
}

#[test]
fn test_for_pay_sorders_for_out_dated_order() {
    new_test_ext().execute_with(|| {
        // generate 50 blocks first
        run_to_block(50);

        let source = 0;
        let file_identifier =
        hex::decode("4e2883ddcbc77cf19979770d756fd332d0c8f815f9de646636169e460e6af6ff").unwrap();
        let merchant: u64 = 100;
        let client: u64 = 0;
        let file_size = 16; // should less than merchant
        let duration: u32 = 100;
        let fee: u64 = 1000;
        let amount: u64 = duration as u64 * fee;
        let address_info = "ws://127.0.0.1:8855".as_bytes().to_vec();
        let file_alias = "/test/file1".as_bytes().to_vec();
        let _ = Balances::make_free_balance_be(&source, 500000);

        // 1. Normal flow, aka happy pass 😃
        let _ = Balances::make_free_balance_be(&merchant, 500000);
        assert_ok!(Market::pledge(Origin::signed(merchant), 500000));
        assert_ok!(Market::register(Origin::signed(merchant), address_info.clone(), fee));
        assert_ok!(Market::place_storage_order(
            Origin::signed(source), merchant,
            file_identifier.clone(), file_size, duration, file_alias
        ));

        let order_id = H256::default();
        assert_eq!(Market::sorder_infos(&order_id).unwrap(), SorderInfo {
            file_identifier: file_identifier.clone(),
            file_size: 16,
            created_on: 50,
            merchant,
            client,
            amount,
            duration: duration*10
        });
        assert_eq!(Market::sorder_statuses(&order_id).unwrap(), SorderStatus {
            completed_on: 50,
            expired_on: 50+duration*10,
            status: OrderStatus::Pending,
            claimed_at: 50
        });
        let mut so = Market::sorder_statuses(&order_id).unwrap();
        so.status = OrderStatus::Success;
        <SorderStatuses<Test>>::insert(order_id.clone(), so);
        
        run_to_block(1050);
        assert_eq!(Balances::free_balance(&merchant), 500000);
        assert_eq!(Balances::free_balance(&source), 400000);
        assert_eq!(Market::pledges(&merchant), Pledge {
            total: 500000,
            used: 100000
        });
        assert_ok!(Market::pay_sorders(Origin::signed(source), vec![order_id]));
        // 50% pledge is slashed
        assert_eq!(Balances::free_balance(&merchant), 600000);
        assert_eq!(Balances::free_balance(&source), 400000);
        assert_eq!(Market::pledges(&merchant), Pledge {
            total: 500000,
            used: 0
        });
        assert!(!<SorderStatuses<Test>>::contains_key(&order_id));
        assert!(!<SorderInfos<Test>>::contains_key(&order_id));
        assert!(!<SorderPunishments<Test>>::contains_key(&order_id));
    });
}
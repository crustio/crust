use super::*;
use crate::mock::*;
use frame_support::{
    assert_noop, assert_ok,
    dispatch::DispatchError,
};
use hex;
use crate::{StorageOrder, Provision};
use sp_core::H256;

#[test]
fn test_for_storage_order_should_work() {
    new_test_ext().execute_with(|| {
        // generate 50 blocks first
        run_to_block(50);

        let source = 0;
        let file_identifier =
        hex::decode("4e2883ddcbc77cf19979770d756fd332d0c8f815f9de646636169e460e6af6ff").unwrap();
        let provider: u64 = 100;
        let client: u64 = 0;
        let file_size = 16; // should less than provider
        let duration = 360; // file should store at least 30 minutes
        let fee = 10;
        let address_info = "ws://127.0.0.1:8855".as_bytes().to_vec();
        let _ = Balances::make_free_balance_be(&source, 80);

        // 1. Normal flow, aka happy pass 😃
        let _ = Balances::make_free_balance_be(&provider, 80);
        assert_ok!(Market::register(Origin::signed(provider), address_info.clone()));
        assert_ok!(Market::pledge(Origin::signed(provider), 60));
        assert_ok!(Market::place_storage_order(
            Origin::signed(source), provider, fee,
            file_identifier.clone(), file_size, duration
        ));

        let order_id = H256::default();
        assert_eq!(Market::providers(&provider).unwrap(), Provision {
            address_info,
            file_map: vec![(file_identifier.clone(), vec![order_id.clone()])].into_iter().collect()
        });
        assert_eq!(Market::clients(&client).unwrap(), vec![order_id.clone()]);
        assert_eq!(Market::storage_orders(&order_id).unwrap(), StorageOrder {
            file_identifier: file_identifier.clone(),
            file_size: 16,
            created_on: 50,
            completed_on: 50,
            expired_on: 410,
            provider,
            client,
            amount: fee,
            status: OrderStatus::Pending
        });

        // 2. Register after get order, address should update but others should not
        let another_address_info = "ws://127.0.0.1:9900".as_bytes().to_vec();
        assert_ok!(Market::register(Origin::signed(provider), another_address_info.clone()));
        assert_eq!(Market::providers(&provider).unwrap(), Provision {
            address_info: another_address_info.clone(),
            file_map: vec![(file_identifier.clone(), vec![order_id.clone()])].into_iter().collect()
        });
        assert_eq!(Market::pledge_ledgers(provider).unwrap(), PledgeLedger {
            total: 60,
            unused: 50
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
        let provider = 100;
        let file_size = 200; // should less than provider
        let duration = 360;
        let fee = 10;
        let address = "ws://127.0.0.1:8855".as_bytes().to_vec();

        assert_ok!(Market::register(Origin::signed(provider), address.clone()));
        assert_noop!(
            Market::place_storage_order(
                Origin::signed(source.clone()), provider, fee,
                file_identifier, file_size, duration
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
fn test_for_storage_order_should_fail_due_to_wrong_expired() {
    new_test_ext().execute_with(|| {
        // generate 50 blocks first
        run_to_block(50);

        let source = 0;
        let file_identifier =
            hex::decode("4e2883ddcbc77cf19979770d756fd332d0c8f815f9de646636169e460e6af6ff").unwrap();
        let provider = 100;
        let file_size = 60; // should less than provider
        let duration = 20;
        let fee = 10;
        let address = "ws://127.0.0.1:8855".as_bytes().to_vec();

        assert_ok!(Market::register(Origin::signed(provider), address.clone()));
        assert_noop!(
            Market::place_storage_order(
                Origin::signed(source.clone()), provider, fee,
                file_identifier, file_size, duration
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
fn test_for_storage_order_should_fail_due_to_exist_of_wr() {
    new_test_ext().execute_with(|| {
        // generate 50 blocks first
        run_to_block(50);

        let provider = 400; // Invalid provider. No work report
        let address = "ws://127.0.0.1:8855".as_bytes().to_vec();

        assert_noop!(
            Market::register(Origin::signed(provider), address.clone()),
            DispatchError::Module {
                index: 0,
                error: 1,
                message: Some("NoWorkload"),
            }
        );
    });
}

#[test]
fn test_for_storage_order_should_fail_due_to_provider_not_register() {
    new_test_ext().execute_with(|| {
        // generate 50 blocks first
        run_to_block(50);

        let source = 0;
        let file_identifier =
            hex::decode("4e2883ddcbc77cf19979770d756fd332d0c8f815f9de646636169e460e6af6ff").unwrap();
        let provider = 100;
        let file_size = 80; // should less than provider
        let duration = 360;
        let fee = 10;

        assert_noop!(
            Market::place_storage_order(
                Origin::signed(source.clone()), provider, fee,
                file_identifier, file_size, duration
            ),
            DispatchError::Module {
                index: 0,
                error: 2,
                message: Some("NotProvider"),
            }
        );
    });
}

#[test]
fn test_for_pledge_should_work() {
    new_test_ext().execute_with(|| {
        // generate 50 blocks first
        run_to_block(50);
        let provider = 100;
        let address_info = "ws://127.0.0.1:8855".as_bytes().to_vec();
        let _ = Balances::make_free_balance_be(&provider, 200);
        assert_ok!(Market::register(Origin::signed(provider), address_info.clone()));
        assert_ok!(Market::pledge(Origin::signed(provider), 180));
        assert_eq!(Market::pledge_ledgers(provider).unwrap(), PledgeLedger {
            total: 180,
            unused: 180
        });
        assert_ok!(Market::pledge(Origin::signed(provider), 160));
        assert_eq!(Market::pledge_ledgers(provider).unwrap(), PledgeLedger {
            total: 160,
            unused: 160
        });
        assert_ok!(Market::pledge(Origin::signed(provider), 150));
        assert_eq!(Market::pledge_ledgers(provider).unwrap(), PledgeLedger {
            total: 150,
            unused: 150
        });
    });
}

#[test]
fn test_for_pledge_should_fail_due_to_provider_not_register() {
    new_test_ext().execute_with(|| {
        // generate 50 blocks first
        run_to_block(50);

        let provider = 100;
        let _ = Balances::make_free_balance_be(&provider, 200);
        assert_noop!(
            Market::pledge(
                Origin::signed(provider),
                200
            ),
            DispatchError::Module {
                index: 0,
                error: 2,
                message: Some("NotProvider")
            }
        );
    });
}


#[test]
fn test_for_pledge_should_fail_due_to_insufficient_currency() {
    new_test_ext().execute_with(|| {
        // generate 50 blocks first
        run_to_block(50);

        let provider = 100;
        let address_info = "ws://127.0.0.1:8855".as_bytes().to_vec();
        let _ = Balances::make_free_balance_be(&provider, 100);
        assert_ok!(Market::register(Origin::signed(provider), address_info.clone()));
        assert_noop!(
            Market::pledge(
                Origin::signed(provider),
                200
            ),
            DispatchError::Module {
                index: 0,
                error: 4,
                message: Some("InsufficientCurrecy")
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
        let provider = 100;
        let file_size = 16; // should less than provider
        let duration = 360; // file should store at least 30 minutes
        let fee = 50;
        let address_info = "ws://127.0.0.1:8855".as_bytes().to_vec();
        let _ = Balances::make_free_balance_be(&source, 80);
        let _ = Balances::make_free_balance_be(&provider, 80);
        assert_ok!(Market::register(Origin::signed(provider), address_info.clone()));
        assert_ok!(Market::pledge(Origin::signed(provider), 70));
        assert_ok!(Market::place_storage_order(
            Origin::signed(source), provider, fee,
            file_identifier.clone(), file_size, duration
        ));
        assert_eq!(Market::pledge_ledgers(provider).unwrap(), PledgeLedger {
            total: 70,
            unused: 20
        });
        assert_ok!(Market::pledge(Origin::signed(provider), 50));
        assert_eq!(Market::pledge_ledgers(provider).unwrap(), PledgeLedger {
            total: 50,
            unused: 0
        });
        assert_noop!(
            Market::pledge(
                Origin::signed(provider),
                49
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
        let provider = 100;
        let file_size = 16; // should less than provider
        let duration = 360; // file should store at least 30 minutes
        let fee = 60;
        let address_info = "ws://127.0.0.1:8855".as_bytes().to_vec();
        let _ = Balances::make_free_balance_be(&source, 80);
        let _ = Balances::make_free_balance_be(&provider, 80);
        assert_ok!(Market::register(Origin::signed(provider), address_info.clone()));
        assert_noop!(
            Market::place_storage_order(
                Origin::signed(source.clone()), provider, fee,
                file_identifier.clone(), file_size, duration
            ),
            DispatchError::Module {
                index: 0,
                error: 5,
                message: Some("InsufficientPledge"),
            }
        );
        assert_ok!(Market::pledge(Origin::signed(provider), 40));
        assert_eq!(Market::pledge_ledgers(provider).unwrap(), PledgeLedger {
            total: 40,
            unused: 40
        });
        assert_noop!(
            Market::place_storage_order(
                Origin::signed(source.clone()), provider, fee,
                file_identifier.clone(), file_size, duration
            ),
            DispatchError::Module {
                index: 0,
                error: 5,
                message: Some("InsufficientPledge"),
            }
        );
        assert_ok!(Market::pledge(Origin::signed(provider), 60));
        assert_eq!(Market::pledge_ledgers(provider).unwrap(), PledgeLedger {
            total: 60,
            unused: 60
        });
        assert_ok!(Market::place_storage_order(
            Origin::signed(source), provider, fee,
            file_identifier.clone(), file_size, duration
        ));

        let order_id = H256::default();
        
        assert_eq!(Market::providers(&provider).unwrap(), Provision {
            address_info,
            file_map: vec![(file_identifier.clone(), vec![order_id.clone()])].into_iter().collect()
        });
        assert_eq!(Market::clients(0).unwrap(), vec![order_id.clone()]);
        assert_eq!(Market::storage_orders(order_id).unwrap(), StorageOrder {
            file_identifier,
            file_size: 16,
            created_on: 50,
            completed_on: 50,
            expired_on: 410,
            provider: 100,
            client: 0,
            amount: fee,
            status: Default::default()
        });
        assert_eq!(Market::pledge_ledgers(provider).unwrap(), PledgeLedger {
            total: 60,
            unused: 0
        });
    });
}
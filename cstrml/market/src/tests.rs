use super::*;
use crate::mock::*;
use frame_support::{
    assert_noop, assert_ok,
    dispatch::DispatchError,
};
use hex;
use crate::{StorageOrder, Provision};
use sp_core::H256;

fn set_punishment_in_success_count(order_id: &H256, success_count: EraIndex) {
    let mut so = Market::storage_orders(&order_id).unwrap();
    so.status = OrderStatus::Success;
    Market::maybe_set_sorder(&order_id, &so);
    for _ in 0 .. success_count {
        Market::maybe_punish_provider(&order_id);
    }
    so.status = OrderStatus::Failed;
    Market::maybe_set_sorder(&order_id, &so);
    for _ in success_count .. <mock::Test as Trait>::PunishDuration::get(){
        Market::maybe_punish_provider(&order_id);
    }
}

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
        let duration = 10;
        let fee: u64 = 1;
        let amount: u64 = 10;
        let address_info = "ws://127.0.0.1:8855".as_bytes().to_vec();
        let _ = Balances::make_free_balance_be(&source, 60);

        // 1. Normal flow, aka happy pass 😃
        let _ = Balances::make_free_balance_be(&provider, 60);
        assert_ok!(Market::pledge(Origin::signed(provider.clone()), 60));
        assert_ok!(Market::cut_pledge(Origin::signed(provider.clone()), 60));
        assert!(!<Pledges<Test>>::contains_key(provider.clone()));
        assert_eq!(Balances::locks(provider.clone()).len(), 0);

        assert_ok!(Market::pledge(Origin::signed(provider.clone()), 60));
        assert_ok!(Market::register(Origin::signed(provider.clone()), address_info.clone(), fee));

        assert_noop!(
            Market::place_storage_order(
                Origin::signed(provider.clone()), provider.clone(),
                file_identifier.clone(), file_size, duration
            ),
            DispatchError::Module {
                index: 0,
                error: 9,
                message: Some("PlaceSelfOrder"),
            }
        );
        assert_ok!(Market::place_storage_order(
            Origin::signed(source), provider,
            file_identifier.clone(), file_size, duration
        ));

        let order_id = H256::default();
        assert_eq!(Market::providers(&provider).unwrap(), Provision {
            address_info,
            storage_price: fee,
            file_map: vec![(file_identifier.clone(), vec![order_id.clone()])].into_iter().collect()
        });
        assert_eq!(Market::clients(&client).unwrap(), vec![order_id.clone()]);
        assert_eq!(Market::storage_orders(&order_id).unwrap(), StorageOrder {
            file_identifier: file_identifier.clone(),
            file_size: 16,
            created_on: 50,
            completed_on: 50,
            expired_on: 50+10*10,
            provider,
            client,
            amount,
            status: OrderStatus::Pending
        });

        // 2. Register after get order, address should update but others should not
        let another_address_info = "ws://127.0.0.1:9900".as_bytes().to_vec();
        let another_price: u64 = 2;
        assert_ok!(Market::register(Origin::signed(provider.clone()), another_address_info.clone(), another_price));
        assert_eq!(Market::providers(&provider).unwrap(), Provision {
            address_info: another_address_info.clone(),
            storage_price: another_price,
            file_map: vec![(file_identifier.clone(), vec![order_id.clone()])].into_iter().collect()
        });
        assert_eq!(Market::pledges(provider), Pledge {
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
        let provider = 100;
        let file_size = 200; // should less than provider
        let duration = 10;
        let fee = 1;
        let address = "ws://127.0.0.1:8855".as_bytes().to_vec();


        assert_ok!(Market::pledge(Origin::signed(provider.clone()), 0));
        assert_ok!(Market::register(Origin::signed(provider), address.clone(), fee));
        assert_noop!(
            Market::place_storage_order(
                Origin::signed(source.clone()), provider,
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
fn test_for_storage_order_should_fail_due_to_duration_too_short() {
    new_test_ext().execute_with(|| {
        // generate 50 blocks first
        run_to_block(50);

        let source = 0;
        let file_identifier =
            hex::decode("4e2883ddcbc77cf19979770d756fd332d0c8f815f9de646636169e460e6af6ff").unwrap();
        let provider = 100;
        let file_size = 60; // should less than provider
        let duration = 1; // shoule more than 1 minute
        let fee = 1;
        let address = "ws://127.0.0.1:8855".as_bytes().to_vec();

        assert_ok!(Market::pledge(Origin::signed(provider.clone()), 0));
        assert_ok!(Market::register(Origin::signed(provider), address.clone(), fee));
        assert_noop!(
            Market::place_storage_order(
                Origin::signed(source.clone()), provider,
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
fn test_for_register_should_fail_due_to_low_price() {
    new_test_ext().execute_with(|| {
        // generate 50 blocks first
        run_to_block(50);

        let provider = 100;
        let address = "ws://127.0.0.1:8855".as_bytes().to_vec();
        assert_ok!(Market::pledge(Origin::signed(provider.clone()), 0));
        assert_noop!(
            Market::register(Origin::signed(provider), address.clone(), 0),
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

        let provider = 400; // Invalid provider. No work report
        let address = "ws://127.0.0.1:8855".as_bytes().to_vec();
        assert_ok!(Market::pledge(Origin::signed(provider.clone()), 0));
        assert_noop!(
            Market::register(Origin::signed(provider), address.clone(), 1),
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
        let duration = 10;

        assert_noop!(
            Market::place_storage_order(
                Origin::signed(source.clone()), provider,
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
        assert_ok!(Market::pledge(Origin::signed(provider.clone()), 180));
        assert_ok!(Market::register(Origin::signed(provider), address_info.clone(), 1));
        assert_eq!(Market::pledges(provider), Pledge {
            total: 180,
            used: 0
        });
        assert_ok!(Market::cut_pledge(Origin::signed(provider), 20));
        assert_eq!(Market::pledges(provider), Pledge {
            total: 160,
            used: 0
        });
        assert_ok!(Market::pledge_extra(Origin::signed(provider), 10));
        assert_eq!(Market::pledges(provider), Pledge {
            total: 170,
            used: 0
        });
    });
}

#[test]
fn test_for_pledge_extra_should_fail_due_to_provider_not_pledges() {
    new_test_ext().execute_with(|| {
        // generate 50 blocks first
        run_to_block(50);

        let provider = 100;
        let _ = Balances::make_free_balance_be(&provider, 200);
        assert_noop!(
            Market::pledge_extra(
                Origin::signed(provider),
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

        let provider = 100;
        let _ = Balances::make_free_balance_be(&provider, 100);
        assert_noop!(
            Market::pledge(
                Origin::signed(provider),
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
        let provider = 100;
        let file_size = 16; // should less than provider
        let duration = 50;
        let fee = 1;
        let amount: u64 = 50;
        let address_info = "ws://127.0.0.1:8855".as_bytes().to_vec();
        let _ = Balances::make_free_balance_be(&source, 80);
        let _ = Balances::make_free_balance_be(&provider, 80);
        assert_ok!(Market::pledge(Origin::signed(provider.clone()), 70));

        assert_ok!(Market::register(Origin::signed(provider), address_info.clone(), fee));
        assert_ok!(Market::place_storage_order(
            Origin::signed(source), provider,
            file_identifier.clone(), file_size, duration
        ));
        assert_eq!(Market::pledges(provider), Pledge {
            total: 70,
            used: amount
        });
        assert_ok!(Market::cut_pledge(Origin::signed(provider), 20));
        assert_eq!(Market::pledges(provider), Pledge {
            total: 50,
            used: amount
        });
        assert_noop!(
            Market::cut_pledge(
                Origin::signed(provider),
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
        let provider = 100;
        let file_size = 16; // should less than provider
        let duration = 60;
        let fee = 1;
        let amount: u64 = 60;
        let address_info = "ws://127.0.0.1:8855".as_bytes().to_vec();
        let _ = Balances::make_free_balance_be(&source, 80);
        let _ = Balances::make_free_balance_be(&provider, 80);
        assert_ok!(Market::pledge(Origin::signed(provider), 0));
        assert_ok!(Market::register(Origin::signed(provider), address_info.clone(), fee));
        assert_noop!(
            Market::place_storage_order(
                Origin::signed(source.clone()), provider,
                file_identifier.clone(), file_size, duration
            ),
            DispatchError::Module {
                index: 0,
                error: 5,
                message: Some("InsufficientPledge"),
            }
        );

        assert_ok!(Market::pledge_extra(Origin::signed(provider), 60));
        assert_eq!(Market::pledges(provider), Pledge {
            total: 60,
            used: 0
        });
        assert_ok!(Market::place_storage_order(
            Origin::signed(source), provider,
            file_identifier.clone(), file_size, duration
        ));

        let order_id = H256::default();
        
        assert_eq!(Market::providers(&provider).unwrap(), Provision {
            address_info,
            storage_price: fee,
            file_map: vec![(file_identifier.clone(), vec![order_id.clone()])].into_iter().collect()
        });
        assert_eq!(Market::clients(&0).unwrap(), vec![order_id.clone()]);
        assert_eq!(Market::storage_orders(order_id).unwrap(), StorageOrder {
            file_identifier,
            file_size: 16,
            created_on: 50,
            completed_on: 50,
            expired_on: 50+60*10,
            provider: 100,
            client: 0,
            amount,
            status: Default::default()
        });
        assert_eq!(Market::pledges(provider), Pledge {
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
        let provider = 100;
        let _ = Balances::make_free_balance_be(&provider, 200);
        assert_ok!(Market::pledge(Origin::signed(provider.clone()), 70));
        assert_noop!(
            Market::pledge(
                Origin::signed(provider),
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
        let provider = 100;
        let _ = Balances::make_free_balance_be(&provider, 80);
        assert_ok!(Market::pledge(Origin::signed(provider.clone()), 70));
        assert_noop!(
            Market::pledge_extra(
                Origin::signed(provider.clone()),
                70
            ),
            DispatchError::Module {
                index: 0,
                error: 4,
                message: Some("InsufficientCurrency")
            }
        );
        assert_ok!(Market::pledge_extra(Origin::signed(provider.clone()), 10));
        assert_ok!(Market::cut_pledge(Origin::signed(provider.clone()), 70));
    });
}

#[test]
fn test_for_half_punish_should_work() {
    new_test_ext().execute_with(|| {
        // generate 50 blocks first
        run_to_block(50);

        let source = 0;
        let file_identifier =
        hex::decode("4e2883ddcbc77cf19979770d756fd332d0c8f815f9de646636169e460e6af6ff").unwrap();
        let provider: u64 = 100;
        let client: u64 = 0;
        let file_size = 16; // should less than provider
        let duration = 10;
        let fee: u64 = 1;
        let amount: u64 = 10;
        let address_info = "ws://127.0.0.1:8855".as_bytes().to_vec();
        let _ = Balances::make_free_balance_be(&source, 60);

        // 1. Normal flow, aka happy pass 😃
        let _ = Balances::make_free_balance_be(&provider, 200);

        assert_ok!(Market::pledge(Origin::signed(provider.clone()), 60));
        assert_ok!(Market::register(Origin::signed(provider.clone()), address_info.clone(), fee));

        assert_ok!(Market::place_storage_order(
            Origin::signed(source), provider,
            file_identifier.clone(), file_size, duration
        ));

        let order_id = H256::default();
        assert_eq!(Market::storage_orders(&order_id).unwrap(), StorageOrder {
            file_identifier: file_identifier.clone(),
            file_size: 16,
            created_on: 50,
            completed_on: 50,
            expired_on: 50+10*10,
            provider,
            client,
            amount,
            status: OrderStatus::Pending
        });

        set_punishment_in_success_count(&order_id, 90);

        assert_eq!(Balances::free_balance(&provider), 195);
        assert_eq!(Market::pledges(&provider), Pledge {
            total: 55,
            used: 5
        });

        set_punishment_in_success_count(&order_id, 90);

        assert_eq!(Balances::free_balance(&provider), 190);
        assert_eq!(Market::pledges(&provider), Pledge {
            total: 50,
            used: 0
        });

        // total fee has been punished.
        set_punishment_in_success_count(&order_id, 90);

        assert_eq!(Balances::free_balance(&provider), 190);
        assert_eq!(Market::pledges(&provider), Pledge {
            total: 50,
            used: 0
        });
    });
}

#[test]
fn test_for_full_punish_should_work() {
    new_test_ext().execute_with(|| {
        // generate 50 blocks first
        run_to_block(50);

        let source = 0;
        let file_identifier =
        hex::decode("4e2883ddcbc77cf19979770d756fd332d0c8f815f9de646636169e460e6af6ff").unwrap();
        let provider: u64 = 100;
        let client: u64 = 0;
        let file_size = 16; // should less than provider
        let duration = 10;
        let fee: u64 = 1;
        let amount: u64 = 10;
        let address_info = "ws://127.0.0.1:8855".as_bytes().to_vec();
        let _ = Balances::make_free_balance_be(&source, 60);

        // 1. Normal flow, aka happy pass 😃
        let _ = Balances::make_free_balance_be(&provider, 200);
        assert_ok!(Market::pledge(Origin::signed(provider.clone()), 60));
        assert_ok!(Market::register(Origin::signed(provider.clone()), address_info.clone(), fee));

        assert_ok!(Market::place_storage_order(
            Origin::signed(source), provider,
            file_identifier.clone(), file_size, duration
        ));

        let order_id = H256::default();
        assert_eq!(Market::storage_orders(&order_id).unwrap(), StorageOrder {
            file_identifier: file_identifier.clone(),
            file_size: 16,
            created_on: 50,
            completed_on: 50,
            expired_on: 50+10*10,
            provider,
            client,
            amount,
            status: OrderStatus::Pending
        });

        set_punishment_in_success_count(&order_id, 95);

        assert_eq!(Balances::free_balance(&provider), 200);
        assert_eq!(Market::pledges(&provider), Pledge {
            total: 60,
            used: 10
        });

        set_punishment_in_success_count(&order_id, 89);

        assert_eq!(Balances::free_balance(&provider), 190);
        assert_eq!(Market::pledges(&provider), Pledge {
            total: 50,
            used: 0
        });

        set_punishment_in_success_count(&order_id, 90);

        assert_eq!(Balances::free_balance(&provider), 190);
        assert_eq!(Market::pledges(&provider), Pledge {
            total: 50,
            used: 0
        });
    });
}
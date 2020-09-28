use super::*;
use crate::mock::*;
use crate::mock::Payment as CstrmlPayment;
use crate::PaymentLedger;
use frame_support::{
    assert_noop, assert_ok,
    dispatch::DispatchError,
};
use hex;

use keyring::Sr25519Keyring;
use sp_core::{crypto::AccountId32, H256};
type AccountId = AccountId32;

#[test]
fn test_for_storage_order_and_payment_should_work() {
    new_test_ext().execute_with(|| {
        // generate 50 blocks first
        run_to_block(50);
        let pledge_amount = 200;

        // Prepare test data
        let client: AccountId = Sr25519Keyring::Alice.to_account_id();
        let file_identifier =
        hex::decode("5bb706320afc633bfb843108e492192b17d2b6b9d9ee0b795ee95417fe08b660").unwrap();
        let merchant: AccountId = Sr25519Keyring::Bob.to_account_id();
        let file_size = 16; // should less than merchant
        let duration = 30;
        let fee = 2;
        let address_info = "ws://127.0.0.1:8855".as_bytes().to_vec();
        let file_alias = "/test/file1".as_bytes().to_vec();
        let _ = Balances::make_free_balance_be(&client, 70);
        let _ = Balances::make_free_balance_be(&merchant, pledge_amount);
        assert_eq!(Balances::free_balance(client.clone()), 70);

        // Call register and place storage order
        assert_ok!(Market::pledge(Origin::signed(merchant.clone()), pledge_amount));
        assert_ok!(Market::register(Origin::signed(merchant.clone()), address_info.clone(), fee));
        assert_ok!(Market::place_storage_order(
            Origin::signed(client.clone()), merchant.clone(),
            file_identifier.clone(), file_size, duration, file_alias
        ));

        let order_id = H256::default();
        assert_eq!(Balances::free_balance(client.clone()), 10);
        assert_eq!(Balances::reserved_balance(client.clone()), 60);
        assert_eq!(CstrmlPayment::payment_ledgers(order_id).unwrap(), PaymentLedger {
            total: 60,
            paid: 0,
            unreserved: 0
        });
        run_to_block(303);
        assert_eq!(CstrmlPayment::payment_ledgers(order_id).unwrap(), PaymentLedger {
            total: 60,
            paid: 0,
            unreserved: 0
        });

        add_work_report(&merchant);
        assert_eq!(Balances::reserved_balance(client.clone()), 60);
        assert_eq!(Balances::free_balance(merchant.clone()), pledge_amount);
        assert_eq!(CstrmlPayment::payment_ledgers(order_id).unwrap(), PaymentLedger {
            total: 60,
            paid: 0,
            unreserved: 0
        });

        for i in 0..31 {
            run_to_block(303 + i * 10);
            assert_eq!(Balances::reserved_balance(client.clone()), 60 - i * 2);
            assert_eq!(Balances::free_balance(client.clone()), 10);
            assert_eq!(Balances::free_balance(merchant.clone()), pledge_amount + i * 2);
            assert_eq!(CstrmlPayment::payment_ledgers(order_id).unwrap(), PaymentLedger {
                total: 60,
                paid: i * 2,
                unreserved: i * 2,
            });
        }
    });
}


#[test]
fn test_for_storage_order_and_payment_should_failed_by_insufficient_currency() {
    new_test_ext().execute_with(|| {
        // generate 50 blocks first
        run_to_block(50);

        let source: AccountId = Sr25519Keyring::Alice.to_account_id();
        let file_identifier =
        hex::decode("5bb706320afc633bfb843108e492192b17d2b6b9d9ee0b795ee95417fe08b660").unwrap();
        let merchant: AccountId = Sr25519Keyring::Bob.to_account_id();
        let file_size = 16; // should less than merchant
        let duration = 60;
        let fee = 1;
        let address_info = "ws://127.0.0.1:8855".as_bytes().to_vec();
        let file_alias = "/test/file1".as_bytes().to_vec();

        Balances::make_free_balance_be(&source, 40);
        Balances::make_free_balance_be(&merchant, 60);
        assert_eq!(Balances::free_balance(source.clone()), 40);
        assert_eq!(Balances::free_balance(merchant.clone()), 60);

        assert_ok!(Market::pledge(Origin::signed(merchant.clone()), 60));
        assert_ok!(Market::register(Origin::signed(merchant.clone()), address_info.clone(), fee));
        assert_noop!(
            Market::place_storage_order(
            Origin::signed(source.clone()), merchant.clone(),
            file_identifier.clone(), file_size, duration, file_alias),
            DispatchError::Module {
                index: 0,
                error: 4,
                message: Some("InsufficientCurrency"),
            }
        );

    });
}

#[test]
fn test_for_storage_order_and_payment_should_suspend() {
    new_test_ext().execute_with(|| {
        // generate 50 blocks first
        run_to_block(50);
        let pledge_amount = 200;
        let source: AccountId = Sr25519Keyring::Alice.to_account_id();
        let file_identifier =
        hex::decode("5bb706320afc633bfb843108e492192b17d2b6b9d9ee0b795ee95417fe08b660").unwrap();
        let merchant: AccountId = Sr25519Keyring::Bob.to_account_id();
        let file_size = 16; // should less than merchant
        let duration = 30;
        let fee = 2;
        let address_info = "ws://127.0.0.1:8855".as_bytes().to_vec();
        let file_alias = "/test/file1".as_bytes().to_vec();
        let _ = Balances::make_free_balance_be(&source, 70);
        let _ = Balances::make_free_balance_be(&merchant, pledge_amount);
        assert_eq!(Balances::free_balance(source.clone()), 70);

        assert_ok!(Market::pledge(Origin::signed(merchant.clone()), pledge_amount));
        assert_ok!(Market::register(Origin::signed(merchant.clone()), address_info.clone(), fee));
        assert_ok!(Market::place_storage_order(
            Origin::signed(source.clone()), merchant.clone(),
            file_identifier.clone(), file_size, duration, file_alias
        ));

        let order_id = H256::default();
        assert_eq!(Balances::free_balance(source.clone()), 10);
        assert_eq!(Balances::reserved_balance(source.clone()), 60);
        assert_eq!(CstrmlPayment::payment_ledgers(order_id).unwrap(), PaymentLedger {
            total: 60,
            paid: 0,
            unreserved: 0
        });

        run_to_block(303);
        assert_eq!(CstrmlPayment::payment_ledgers(order_id).unwrap(), PaymentLedger {
            total: 60,
            paid: 0,
            unreserved: 0
        });

        add_work_report(&merchant);
        assert_eq!(Balances::reserved_balance(source.clone()), 60);
        assert_eq!(Balances::free_balance(merchant.clone()), pledge_amount);
        assert_eq!(CstrmlPayment::payment_ledgers(order_id).unwrap(), PaymentLedger {
            total: 60,
            paid: 0,
            unreserved: 0
        });

        for i in 1..11 {
            run_to_block(303 + i * 10);
            assert_eq!(Balances::reserved_balance(source.clone()), 60 - i * 2);
            assert_eq!(Balances::free_balance(source.clone()), 10);
            assert_eq!(Balances::free_balance(merchant.clone()), pledge_amount + i * 2);
            assert_eq!(CstrmlPayment::payment_ledgers(order_id).unwrap(), PaymentLedger {
                total: 60,
                paid: i * 2,
                unreserved: i * 2
            });
        }

        <market::StorageOrders<Test>>::mutate(&order_id, |sorder| {
            if let Some(so) = sorder {
                so.status = OrderStatus::Failed;
            }
        });
        for i in 11..21 {
            run_to_block(303 + i * 10);
            assert_eq!(Balances::reserved_balance(source.clone()), 60 - i * 2);
            assert_eq!(Balances::free_balance(source.clone()), 10 + (i - 10) * 2 );
            assert_eq!(Balances::free_balance(merchant.clone()), pledge_amount + 20);
            assert_eq!(CstrmlPayment::payment_ledgers(order_id).unwrap(), PaymentLedger {
                total: 60,
                paid: 20,
                unreserved: i * 2
                
            });
        }

        <market::StorageOrders<Test>>::mutate(&order_id, |sorder| {
            if let Some(so) = sorder {
                so.status = OrderStatus::Success;
            }
        });

        for i in 21..31 {
            run_to_block(303 + i * 10);
            assert_eq!(Balances::reserved_balance(source.clone()), 60 - i * 2);
            assert_eq!(Balances::free_balance(source.clone()), 30 );
            assert_eq!(Balances::free_balance(merchant.clone()), pledge_amount + 20 + (i - 20) * 2);
            assert_eq!(CstrmlPayment::payment_ledgers(order_id).unwrap(), PaymentLedger {
                total: 60,
                paid: 20 + (i - 20) * 2,
                unreserved: i * 2
            });
        }
    });
}


#[test]
fn test_for_close_storage_order_in_payment() {
    new_test_ext().execute_with(|| {
        // generate 50 blocks first
        run_to_block(50);
        let pledge_amount = 200;
        let source: AccountId = Sr25519Keyring::Alice.to_account_id();
        let file_identifier =
        hex::decode("5bb706320afc633bfb843108e492192b17d2b6b9d9ee0b795ee95417fe08b660").unwrap();
        let merchant: AccountId = Sr25519Keyring::Bob.to_account_id();
        let file_size = 16; // should less than merchant
        let duration = 30;
        let fee = 2;
        let address_info = "ws://127.0.0.1:8855".as_bytes().to_vec();
        let file_alias = "/test/file1".as_bytes().to_vec();
        let _ = Balances::make_free_balance_be(&source, 70);
        let _ = Balances::make_free_balance_be(&merchant, pledge_amount);
        assert_eq!(Balances::free_balance(source.clone()), 70);

        assert_ok!(Market::pledge(Origin::signed(merchant.clone()), pledge_amount));
        assert_ok!(Market::register(Origin::signed(merchant.clone()), address_info.clone(), fee));
        assert_ok!(Market::place_storage_order(
            Origin::signed(source.clone()), merchant.clone(),
            file_identifier.clone(), file_size, duration, file_alias
        ));

        let order_id = H256::default();
        assert_eq!(Balances::free_balance(source.clone()), 10);
        assert_eq!(Balances::reserved_balance(source.clone()), 60);
        assert_eq!(CstrmlPayment::payment_ledgers(order_id).unwrap(), PaymentLedger {
            total: 60,
            paid: 0,
            unreserved: 0
        });

        run_to_block(303);
        assert_eq!(CstrmlPayment::payment_ledgers(order_id).unwrap(), PaymentLedger {
            total: 60,
            paid: 0,
            unreserved: 0
        });

        add_work_report(&merchant);
        assert_eq!(Balances::reserved_balance(source.clone()), 60);
        assert_eq!(Balances::free_balance(merchant.clone()), pledge_amount);
        assert_eq!(CstrmlPayment::payment_ledgers(order_id).unwrap(), PaymentLedger {
            total: 60,
            paid: 0,
            unreserved: 0
        });
        assert_eq!(CstrmlPayment::slot_payments(3, order_id.clone()), fee);

        for i in 1..11 {
            run_to_block(303 + i * 10);
            assert_eq!(Balances::reserved_balance(source.clone()), 60 - i * 2);
            assert_eq!(Balances::free_balance(source.clone()), 10);
            assert_eq!(Balances::free_balance(merchant.clone()), pledge_amount + i * 2);
            assert_eq!(CstrmlPayment::payment_ledgers(order_id).unwrap(), PaymentLedger {
                total: 60,
                paid: i * 2,
                unreserved: i * 2
            });
        }
        CstrmlPayment::close_sorder(&order_id, &source, &333);
        assert_eq!(Balances::reserved_balance(source.clone()), 0);
        assert_eq!(Balances::free_balance(source.clone()), 50);
        assert!(!<PaymentLedgers<Test>>::contains_key(order_id.clone()));
        assert!(!<SlotPayments<Test>>::contains_key(3, &order_id));

        for i in 11..21 {
            run_to_block(303 + i * 10);
            assert_eq!(Balances::reserved_balance(source.clone()), 0);
            assert_eq!(Balances::free_balance(source.clone()), 50);
            assert_eq!(Balances::free_balance(merchant.clone()), pledge_amount + 20);
        }
    });
}
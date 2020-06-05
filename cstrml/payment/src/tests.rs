use super::*;
use crate::mock::*;
use crate::mock::Payment as CstrmlPayment;
use frame_support::{
    assert_noop, assert_ok,
    dispatch::DispatchError,
};
use hex;
use tee::WorkReport;
use crate::Ledger;

use keyring::Sr25519Keyring;
use sp_core::{crypto::AccountId32, H256};
type AccountId = AccountId32;


fn get_valid_work_report() -> WorkReport {
    let pub_key = hex::decode("b0b0c191996073c67747eb1068ce53036d76870516a2973cef506c29aa37323892c5cc5f379f17e63a64bb7bc69fbea14016eea76dae61f467c23de295d7f689").unwrap();
    let block_hash = hex::decode("05404b690b0c785bf180b2dd82a431d88d29baf31346c53dbda95e83e34c8a75").unwrap();
    let files: Vec<(Vec<u8>, u64)> = [
        (hex::decode("5bb706320afc633bfb843108e492192b17d2b6b9d9ee0b795ee95417fe08b660").unwrap(), 134289408),
        (hex::decode("88cdb315c8c37e2dc00fa2a8c7fe51b8149b363d29f404441982f96d2bbae65f").unwrap(), 268578816)
    ].to_vec();
    let sig = hex::decode("9c12986c01efe715ed8bed80b7e391601c45bf152e280693ffcfd10a4b386deaaa0f088fc26b0ebeca64c33cf122d372ebd787aa77beaaba9d2e499ce40a76e6").unwrap();


    WorkReport {
        pub_key,
        block_number: 300,
        block_hash,
        used: 0,
        reserved: 4294967296,
        sig,
        files
    }
}

#[test]
fn test_for_storage_order_and_payment_should_work() {
    new_test_ext().execute_with(|| {
        // generate 50 blocks first
        run_to_block(50);

        // Prepare test data
        let client: AccountId = Sr25519Keyring::Alice.to_account_id();
        let file_identifier =
        hex::decode("5bb706320afc633bfb843108e492192b17d2b6b9d9ee0b795ee95417fe08b660").unwrap();
        let provider: AccountId = Sr25519Keyring::Bob.to_account_id();
        let file_size = 16; // should less than provider
        let duration = 360; // file should store at least 30 minutes
        let fee = 60;
        let address_info = "ws://127.0.0.1:8855".as_bytes().to_vec();
        let _ = Balances::make_free_balance_be(&client, 70);
        assert_eq!(Balances::free_balance(client.clone()), 70);

        // Call register and place storage order
        assert_ok!(Market::register(Origin::signed(provider.clone()), address_info.clone()));
        assert_ok!(Market::place_storage_order(
            Origin::signed(client.clone()), provider.clone(), fee,
            file_identifier.clone(), file_size, duration
        ));

        let order_id = H256::default();
        assert_eq!(Balances::free_balance(client.clone()), 10);
        assert_eq!(Balances::reserved_balance(client.clone()), 60);
        assert_eq!(CstrmlPayment::payments(order_id).unwrap(), Ledger {
            total: 60,
            paid: 0,
            unreserved: 0
        });
        run_to_block(303);
        assert_eq!(CstrmlPayment::payments(order_id).unwrap(), Ledger {
            total: 60,
            paid: 0,
            unreserved: 0
        });

        // Check workloads
        assert_eq!(Tee::reserved(), 0);

        assert_ok!(Tee::report_works(
            Origin::signed(provider.clone()),
            get_valid_work_report()
        ));
        assert_eq!(Balances::reserved_balance(client.clone()), 60);
        assert_eq!(Balances::free_balance(provider.clone()), 0);
        assert_eq!(CstrmlPayment::payments(order_id).unwrap(), Ledger {
            total: 60,
            paid: 0,
            unreserved: 0
        });

        for i in 1..30 {
            run_to_block(303 + i * 10);
            assert_eq!(Balances::reserved_balance(client.clone()), 60 - i * 2);
            assert_eq!(Balances::free_balance(client.clone()), 10);
            assert_eq!(Balances::free_balance(provider.clone()), i * 2);
            assert_eq!(CstrmlPayment::payments(order_id).unwrap(), Ledger {
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
        let provider: AccountId = Sr25519Keyring::Bob.to_account_id();
        let file_size = 16; // should less than provider
        let duration = 360; // file should store at least 30 minutes
        let fee = 60;
        let address_info = "ws://127.0.0.1:8855".as_bytes().to_vec();
        let _ = Balances::make_free_balance_be(&source, 40);
        assert_eq!(Balances::free_balance(source.clone()), 40);

        assert_ok!(Market::register(Origin::signed(provider.clone()), address_info.clone()));
        assert_noop!(
            Market::place_storage_order(
            Origin::signed(source.clone()), provider.clone(), fee,
            file_identifier.clone(), file_size, duration
            ),
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

        let source: AccountId = Sr25519Keyring::Alice.to_account_id();
        let file_identifier =
        hex::decode("5bb706320afc633bfb843108e492192b17d2b6b9d9ee0b795ee95417fe08b660").unwrap();
        let provider: AccountId = Sr25519Keyring::Bob.to_account_id();
        let file_size = 16; // should less than provider
        let duration = 360; // file should store at least 30 minutes
        let fee = 60;
        let address_info = "ws://127.0.0.1:8855".as_bytes().to_vec();
        let _ = Balances::make_free_balance_be(&source, 70);
        assert_eq!(Balances::free_balance(source.clone()), 70);

        assert_ok!(Market::register(Origin::signed(provider.clone()), address_info.clone()));
        assert_ok!(Market::place_storage_order(
            Origin::signed(source.clone()), provider.clone(), fee,
            file_identifier.clone(), file_size, duration
        ));

        let order_id = H256::default();
        assert_eq!(Balances::free_balance(source.clone()), 10);
        assert_eq!(Balances::reserved_balance(source.clone()), 60);
        assert_eq!(CstrmlPayment::payments(order_id).unwrap(), Ledger {
            total: 60,
            paid: 0,
            unreserved: 0
        });

        run_to_block(303);
        assert_eq!(CstrmlPayment::payments(order_id).unwrap(), Ledger {
            total: 60,
            paid: 0,
            unreserved: 0
        });

        // Check workloads
        assert_eq!(Tee::reserved(), 0);

        assert_ok!(Tee::report_works(
            Origin::signed(provider.clone()),
            get_valid_work_report()
        ));
        assert_eq!(Balances::reserved_balance(source.clone()), 60);
        assert_eq!(Balances::free_balance(provider.clone()), 0);
        assert_eq!(CstrmlPayment::payments(order_id).unwrap(), Ledger {
            total: 60,
            paid: 0,
            unreserved: 0
        });

        for i in 1..11 {
            run_to_block(303 + i * 10);
            assert_eq!(Balances::reserved_balance(source.clone()), 60 - i * 2);
            assert_eq!(Balances::free_balance(source.clone()), 10);
            assert_eq!(Balances::free_balance(provider.clone()), i * 2);
            assert_eq!(CstrmlPayment::payments(order_id).unwrap(), Ledger {
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
            assert_eq!(Balances::free_balance(provider.clone()), 20);
            assert_eq!(CstrmlPayment::payments(order_id).unwrap(), Ledger {
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

        for i in 21..30 {
            run_to_block(303 + i * 10);
            assert_eq!(Balances::reserved_balance(source.clone()), 60 - i * 2);
            assert_eq!(Balances::free_balance(source.clone()), 30 );
            assert_eq!(Balances::free_balance(provider.clone()), 20 + (i - 20) * 2);
            assert_eq!(CstrmlPayment::payments(order_id).unwrap(), Ledger {
                total: 60,
                paid: 20 + (i - 20) * 2,
                unreserved: i * 2
            });
        }
    });
}
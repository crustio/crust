use super::*;
use crate::mock::*;
use frame_support::{
    assert_noop, assert_ok,
    dispatch::DispatchError,
};
use hex;
use market::{StorageOrder, Provision};
use tee::WorkReport;
use balances::{BalanceLock, Reasons};
use crate::PaymentLedger;

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
fn test_for_storage_order_should_work() {
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
        let _ = Balances::make_free_balance_be(&source, 2);
        assert_eq!(Balances::free_balance(source.clone()), 2);
        assert_ok!(Market::register(Origin::signed(provider.clone()), address_info.clone()));
        assert_ok!(Market::place_storage_order(
            Origin::signed(source.clone()), provider.clone(), fee,
            file_identifier.clone(), file_size, duration
        ));

        let order_id = H256::default();
        assert_eq!(Market::providers(provider.clone()).unwrap(), Provision {
            address_info,
            file_map: vec![(file_identifier.clone(), order_id.clone())].into_iter().collect()
        });
        assert_eq!(Market::clients(source.clone()).unwrap(), vec![order_id.clone()]);
        assert_eq!(Market::storage_orders(order_id).unwrap(), StorageOrder {
            file_identifier: file_identifier.clone(),
            file_size: 16,
            created_on: 50,
            completed_on: 50,
            expired_on: 410,
            provider: provider.clone(),
            client: source.clone(),
            order_status: Default::default()
        });
        assert_eq!(Balances::free_balance(source.clone()), 2);
        assert_eq!(crate::mock::Payment::payments(order_id).unwrap(), PaymentLedger {
            total: 60,
            already_paid: 0
        });
        run_to_block(303);
        assert_eq!(crate::mock::Payment::payments(order_id).unwrap(), PaymentLedger {
            total: 60,
            already_paid: 0
        });

        // Check workloads
        assert_eq!(Tee::reserved(), 0);

        assert_ok!(Tee::report_works(
            Origin::signed(provider.clone()),
            get_valid_work_report()
        ));
        assert_eq!(Balances::reserved_balance(source.clone()), 0);
        assert_eq!(Balances::free_balance(provider.clone()), 0);
        assert_eq!(crate::mock::Payment::payments(order_id).unwrap(), PaymentLedger {
            total: 60,
            already_paid: 0
        });
        assert_eq!(Balances::locks(source.clone()), [
            BalanceLock { 
                id: [112, 97, 121, 109, 101, 110, 116, 32],
                amount: 60,
                reasons: Reasons::All
            }
            ]);
        run_to_block(313);
        assert_eq!(crate::mock::Payment::payments(order_id).unwrap(), PaymentLedger {
            total: 60,
            already_paid: 1
        });
        assert_eq!(Balances::free_balance(provider.clone()), 1);
        assert_eq!(Balances::free_balance(source.clone()), 1);
        assert_eq!(Balances::locks(source.clone()), [
            BalanceLock { 
                id: [112, 97, 121, 109, 101, 110, 116, 32],
                amount: 59,
                reasons: Reasons::All
            }
            ]);
        run_to_block(323);
        assert_eq!(crate::mock::Payment::payments(order_id).unwrap(), PaymentLedger {
            total: 60,
            already_paid: 2
        });
        run_to_block(333);
        assert_eq!(crate::mock::Payment::payments(order_id).unwrap(), PaymentLedger {
            total: 60,
            already_paid: 3
        });
        assert_eq!(Balances::locks(source.clone()), [
            BalanceLock { 
                id: [112, 97, 121, 109, 101, 110, 116, 32],
                amount: 57,
                reasons: Reasons::All
            }
            ]);
        run_to_block(343);
        assert_eq!(crate::mock::Payment::payments(order_id).unwrap(), PaymentLedger {
            total: 60,
            already_paid: 4
        });
    });
}
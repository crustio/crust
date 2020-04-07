use super::*;

use crate::mock::{new_test_ext, run_to_block, Origin, StorageOrder};
use frame_support::assert_ok;
use hex;
use keyring::Sr25519Keyring;
use sp_core::crypto::{AccountId32, Ss58Codec};

type AccountId = AccountId32;

fn get_valid_identity() -> tee::Identity<AccountId> {
    // Bob is validator in genesis block
    let applier: AccountId =
        AccountId::from_ss58check("5HZFQohYpN4MVyGjiq8bJhojt9yCVa8rXd4Kt9fmh5gAbQqA")
            .expect("valid ss58 address");
    let validator: AccountId = Sr25519Keyring::Bob.to_account_id();

    let a_pk = hex::decode("e9e055da2ad974421c5cf73b466b75ba24910091759a5ddc51adeff5d7bf3c16b345aefbb244a02a4643ea1ca862c888a3acf28ee7528e0a6abccf666621a24a").unwrap();
    let v_pk = hex::decode("0fb42b36f26b69b7bbd3f60b2e377e66a4dacf0284877731bb59ca2cc9ce2759390dfb4b7023986e238d74df027f0f7f34b51f4b0dbf60e5f0ac90812d977499").unwrap();
    let sig= hex::decode("1d41cea5287fcc6e2ce91eea3fb6fb0fa93ce1c784e159d2e240395dad0d3c28769308f75cd70f2dab4b1b1d9577a4055f0ac3c10443fd289d54669e720a5cd2").expect("Invalid hex");

    tee::Identity {
        pub_key: a_pk.clone(),
        account_id: applier.clone(),
        validator_pub_key: v_pk.clone(),
        validator_account_id: validator.clone(),
        sig: sig.clone(),
    }
}

fn get_valid_work_report() -> tee::WorkReport {
    let pub_key = hex::decode("0fb42b36f26b69b7bbd3f60b2e377e66a4dacf0284877731bb59ca2cc9ce2759390dfb4b7023986e238d74df027f0f7f34b51f4b0dbf60e5f0ac90812d977499").unwrap();
    let block_hash = [0; 32].to_vec();
    let empty_root =
        hex::decode("4e2883ddcbc77cf19979770d756fd332d0c8f815f9de646636169e460e6af6ff").unwrap();
    let sig = hex::decode("d178f20e2f2abfa72d056ce7689fab358977597b961bdf530b33e1e0da0f447e87ef414cf687d12aa6a63739c471de207e435d147900fe43f66bcff19668b955").unwrap();

    tee::WorkReport {
        pub_key,
        block_number: 300,
        block_hash,
        empty_root,
        empty_workload: 4294967296,
        meaningful_workload: 1676266280,
        sig,
    }
}

#[test]
fn test_for_storage_order_show_work() {
    new_test_ext().execute_with(|| {
        // generate 50 blocks first
        run_to_block(50);

        let source = 0;
        let file_indetifier = 
        hex::decode("4e2883ddcbc77cf19979770d756fd332d0c8f815f9de646636169e460e6af6ff").unwrap();
        let destination = 100;
        let file_size = 16; // should less than destination
        let expired_duration = 16;
        let expired_on = 20;
        let fee = 10;
        assert_ok!(StorageOrder::store_storage_order(
            Origin::signed(source.clone()), destination, fee,
            file_indetifier, file_size, expired_duration, expired_on
        ));
    });
}


#[test]
fn test_for_storage_order_show_fail_due_to_file_size() {
    new_test_ext().execute_with(|| {
        // generate 50 blocks first
        run_to_block(50);

        let source = 0;
        let file_indetifier = 
        hex::decode("4e2883ddcbc77cf19979770d756fd332d0c8f815f9de646636169e460e6af6ff").unwrap();
        let destination = 100;
        let file_size = 200; // should less than destination
        let expired_duration = 16;
        let expired_on = 20;
        let fee = 10;
        assert!(StorageOrder::store_storage_order(
            Origin::signed(source.clone()), destination, fee,
            file_indetifier, file_size, expired_duration, expired_on
        ).is_err());
    });
}

#[test]
fn test_for_storage_order_show_fail_due_to_exist_of_wr() {
    new_test_ext().execute_with(|| {
        // generate 50 blocks first
        run_to_block(50);

        let source = 0;
        let file_indetifier = 
        hex::decode("4e2883ddcbc77cf19979770d756fd332d0c8f815f9de646636169e460e6af6ff").unwrap();
        let destination = 400; // Invalid destination. No work report
        let file_size = 200;
        let expired_duration = 16;
        let expired_on = 20;
        let fee = 10;
        assert!(StorageOrder::store_storage_order(
            Origin::signed(source.clone()), destination, fee,
            file_indetifier, file_size, expired_duration, expired_on
        ).is_err());
    });
}

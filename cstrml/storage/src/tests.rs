use super::*;

use crate::mock::{new_test_ext, run_to_block, Origin, Tee, Storage};
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
fn test_for_report_works_success() {
    new_test_ext().execute_with(|| {
        // generate 303 blocks first
        run_to_block(303);

        let account: AccountId = Sr25519Keyring::Bob.to_account_id();

        // Check workloads
        assert_eq!(Tee::empty_workload(), 0);

        assert_ok!(Tee::report_works(
            Origin::signed(account.clone()),
            get_valid_work_report()
        ));

        // Check workloads after work report
        assert_eq!(Tee::empty_workload(), 4294967296);
        assert_eq!(Tee::meaningful_workload(), 1676266280);

        let source: AccountId = Sr25519Keyring::Alice.to_account_id();
        let file_indetifier = 
        hex::decode("4e2883ddcbc77cf19979770d756fd332d0c8f815f9de646636169e460e6af6ff").unwrap();
        let destination: AccountId = Sr25519Keyring::Bob.to_account_id();
        let file_size = 16;
        let expired_duration = 16;
        let expired_on = 20;
        let fee = 10;
        let order_id = 0;
        assert_ok!(Storage::store_storage_order(
            Origin::signed(source.clone()), destination, fee,
            order_id, file_indetifier, file_size, expired_duration, expired_on
        ));
    });
}

// #[test]
// fn test_for_report_works_failed_by_pub_key_is_not_found() {
//     new_test_ext().execute_with(|| {
//         let account: AccountId32 = Sr25519Keyring::Alice.to_account_id();

//         let mut works = get_valid_work_report();
//         works.pub_key = "another_pub_key".as_bytes().to_vec();

//         assert!(Tee::report_works(Origin::signed(account), works).is_err());
//     });
// }

// #[test]
// fn test_for_report_works_failed_by_reporter_is_not_registered() {
//     new_test_ext().execute_with(|| {
//         let account: AccountId32 = Sr25519Keyring::Bob.to_account_id();

//         let works = WorkReport {
//             pub_key: "pub_key_bob".as_bytes().to_vec(),
//             block_number: 50,
//             block_hash: "block_hash".as_bytes().to_vec(),
//             empty_root: "merkle_root_bob".as_bytes().to_vec(),
//             empty_workload: 2000,
//             meaningful_workload: 2000,
//             sig: "sig_key_bob".as_bytes().to_vec(),
//         };

//         assert!(Tee::report_works(Origin::signed(account), works).is_err());
//     });
// }

// #[test]
// fn test_for_work_report_timing_check_failed_by_wrong_hash() {
//     new_test_ext().execute_with(|| {
//         // generate 50 blocks first
//         run_to_block(50);

//         let account: AccountId32 = Sr25519Keyring::Alice.to_account_id();
//         let block_hash = [1; 32].to_vec();

//         let works = WorkReport {
//             pub_key: "pub_key_alice".as_bytes().to_vec(),
//             block_number: 50,
//             block_hash,
//             empty_root: "merkle_root_alice".as_bytes().to_vec(),
//             empty_workload: 0,
//             meaningful_workload: 1000,
//             sig: "sig_key_alice".as_bytes().to_vec(),
//         };

//         assert!(Tee::report_works(Origin::signed(account), works).is_err());
//     });
// }

// #[test]
// fn test_for_work_report_timing_check_failed_by_slot_outdated() {
//     new_test_ext().execute_with(|| {
//         // generate 50 blocks first
//         run_to_block(103);

//         let account: AccountId32 = Sr25519Keyring::Alice.to_account_id();
//         let block_hash = [0; 32].to_vec();

//         let works = WorkReport {
//             pub_key: "pub_key_alice".as_bytes().to_vec(),
//             block_number: 50,
//             block_hash,
//             empty_root: "merkle_root_alice".as_bytes().to_vec(),
//             empty_workload: 5000,
//             meaningful_workload: 1000,
//             sig: "sig_key_alice".as_bytes().to_vec(),
//         };

//         assert!(Tee::report_works(Origin::signed(account), works).is_err());
//     });
// }

// #[test]
// fn test_for_work_report_sig_check_failed() {
//     new_test_ext().execute_with(|| {
//         // generate 53 blocks first
//         run_to_block(53);

//         let account: AccountId32 = Sr25519Keyring::Alice.to_account_id();
//         let pub_key = hex::decode("19817c0e3be0793b9c27b6064aeac6a82df3335a2ccdac7ea3d4c56e96a315a1e4dfe23491330c1ba11347ca4d6474151636ec15a7fc45d219d034eb9a33bb75").unwrap();
//         let block_hash= [0; 32].to_vec();
//         let empty_root = hex::decode("ae56f97320fbebbd1dd44d573486261709f5497d9d8391b2b2b6c23287927f5d").unwrap();
//         let sig = hex::decode("4b23f3a95015387c735100dae6fdcb78445a2db50d08e19713bff900b69bc8c0719bf62750b30b92d082adfe8e0fa705a6cbe909c09961b4d0cc5f22d5c91599").unwrap();

//         let works = WorkReport {
//             pub_key,
//             block_number: 50,
//             block_hash,
//             empty_root,
//             empty_workload: 4294967296,
//             meaningful_workload: 1676266280,
//             sig
//         };

//         assert!(Tee::report_works(Origin::signed(account), works).is_err());
//     });
// }

// #[test]
// fn test_for_oudated_work_reports() {
//     new_test_ext().execute_with(|| {
//         let account: AccountId = Sr25519Keyring::Alice.to_account_id();
//         // generate 303 blocks first
//         run_to_block(303);

//         assert_ok!(Tee::report_works(
//             Origin::signed(account.clone()),
//             get_valid_work_report()
//         ));

//         // check work report and workload
//         assert_eq!(Tee::update_and_get_workload(&account, 300), 0);
//         assert_eq!(
//             Tee::work_reports((&account, 300)),
//             Some(get_valid_work_report())
//         );
//         // Check workloads
//         assert_eq!(Tee::empty_workload(), 4294967296);
//         assert_eq!(Tee::meaningful_workload(), 1676266280);

//         // generate 401 blocks, wr still valid
//         run_to_block(401);
//         assert_eq!(
//             Tee::work_reports((&account, 300)),
//             Some(get_valid_work_report())
//         );

//         // generate 602 blocks, 300 work report should be removed
//         run_to_block(602);

//         assert_eq!(Tee::update_and_get_workload(&account, 600), 5971233576);
//         assert_eq!(
//             Tee::work_reports((&account, 300)),
//             None
//         );
//         assert_eq!(
//             Tee::work_reports((&account, 600)),
//             Some(get_valid_work_report())
//         );

//         run_to_block(903);
//         assert_eq!(Tee::update_and_get_workload(&account, 900), 0);
//         assert_eq!(
//             Tee::work_reports((&account, 600)),
//             None
//         );
//         assert_eq!(
//             Tee::work_reports((&account, 900)),
//             None
//         );

//         // Check workloads
//         assert_eq!(Tee::empty_workload(), 0);
//         assert_eq!(Tee::meaningful_workload(), 0);
//     });
// }

// #[test]
// fn test_abnormal_era() {
//     new_test_ext().execute_with(|| {
//         let account: AccountId = Sr25519Keyring::Alice.to_account_id();

//         // If new era happens in 101, next work is not reported, we should keep last work report
//         run_to_block(101);
//         Tee::update_identities();
//         assert_eq!(
//             Tee::work_reports((&account, 0)),
//             Some(Default::default())
//         );
//         assert_eq!(
//             Tee::last_report_slot(),
//             0
//         );

//         // If new era happens in 301, we should update work report and last report slot
//         run_to_block(301);
//         Tee::update_identities();
//         assert_eq!(
//             Tee::work_reports((&account, 0)),
//             None
//         );
//         assert_eq!(
//             Tee::work_reports((&account, 300)),
//             Some(Default::default())
//         );
//         assert_eq!(
//             Tee::last_report_slot(),
//             300
//         );

//     })
// }

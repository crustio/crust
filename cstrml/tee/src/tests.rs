use super::*;

use crate::mock::{new_test_ext, run_to_block, Origin, Tee};
use frame_support::assert_ok;
use hex;
use keyring::Sr25519Keyring;
use sp_core::crypto::{AccountId32, Ss58Codec};

type AccountId = AccountId32;

fn get_valid_identity() -> Identity<AccountId> {
    // Alice is validator in genesis block
    let applier: AccountId =
        AccountId::from_ss58check("5HZFQohYpN4MVyGjiq8bJhojt9yCVa8rXd4Kt9fmh5gAbQqA")
            .expect("valid ss58 address");
    let validator: AccountId = Sr25519Keyring::Alice.to_account_id();

    let a_pk = hex::decode("e9e055da2ad974421c5cf73b466b75ba24910091759a5ddc51adeff5d7bf3c16b345aefbb244a02a4643ea1ca862c888a3acf28ee7528e0a6abccf666621a24a").unwrap();
    let v_pk = hex::decode("0fb42b36f26b69b7bbd3f60b2e377e66a4dacf0284877731bb59ca2cc9ce2759390dfb4b7023986e238d74df027f0f7f34b51f4b0dbf60e5f0ac90812d977499").unwrap();
    let sig= hex::decode("1d41cea5287fcc6e2ce91eea3fb6fb0fa93ce1c784e159d2e240395dad0d3c28769308f75cd70f2dab4b1b1d9577a4055f0ac3c10443fd289d54669e720a5cd2").expect("Invalid hex");

    Identity {
        pub_key: a_pk.clone(),
        account_id: applier.clone(),
        validator_pub_key: v_pk.clone(),
        validator_account_id: validator.clone(),
        sig: sig.clone(),
    }
}

// From Bob's pk
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
fn test_for_register_identity_success() {
    new_test_ext().execute_with(|| {
        // Alice is validator in genesis block
        let applier: AccountId =
            AccountId::from_ss58check("5HZFQohYpN4MVyGjiq8bJhojt9yCVa8rXd4Kt9fmh5gAbQqA")
                .expect("valid ss58 address");
        let id = get_valid_identity();

        assert_ok!(Tee::register_identity(
            Origin::signed(applier.clone()),
            id.clone()
        ));

        let id_registered = Tee::tee_identities(applier.clone()).unwrap();

        assert_eq!(id.clone(), id_registered);
    });
}

#[test]
fn test_for_register_identity_success_with_genesis_validator() {
    new_test_ext().execute_with(|| {
        // Alice can report anything she wants cause she is genesis validators
        let alice: AccountId32 = Sr25519Keyring::Alice.to_account_id();

        let id = Identity {
            pub_key: "pub_key".as_bytes().to_vec(),
            account_id: alice.clone(),
            validator_pub_key: "pub_key".as_bytes().to_vec(),
            validator_account_id: alice.clone(),
            sig: "sig".as_bytes().to_vec(),
        };

        assert_ok!(Tee::register_identity(
            Origin::signed(alice.clone()),
            id.clone()
        ));
    });
}

#[test]
fn test_for_register_identity_failed_by_validator_illegal() {
    new_test_ext().execute_with(|| {
        // Bob is not validator before
        let account: AccountId32 = Sr25519Keyring::Charlie.to_account_id();

        let id = Identity {
            pub_key: "pub_key_bob".as_bytes().to_vec(),
            account_id: account.clone(),
            validator_pub_key: "pub_key_bob".as_bytes().to_vec(),
            validator_account_id: account.clone(),
            sig: "sig_bob".as_bytes().to_vec(),
        };

        assert!(Tee::register_identity(Origin::signed(account.clone()), id.clone()).is_err());
    });
}

#[test]
fn test_for_register_identity_failed_by_validate_for_self() {
    new_test_ext().execute_with(|| {
        let applier: AccountId =
            AccountId::from_ss58check("5HZFQohYpN4MVyGjiq8bJhojt9yCVa8rXd4Kt9fmh5gAbQqA")
                .expect("valid ss58 address");

        // 1. register bob by alice
        let mut id = get_valid_identity();

        assert_ok!(Tee::register_identity(
            Origin::signed(applier.clone()),
            id.clone()
        ));

        // 2. register bob by bob
        id.validator_account_id = applier.clone();

        assert!(Tee::register_identity(Origin::signed(applier.clone()), id.clone()).is_err());
    });
}

#[test]
fn test_for_identity_sig_check_failed() {
    new_test_ext().execute_with(|| {
        // Alice is validator in genesis block
        let applier: AccountId = AccountId::from_ss58check("5HZFQohYpN4MVyGjiq8bJhojt9yCVa8rXd4Kt9fmh5gAbQqA").expect("valid ss58 address");
        let validator: AccountId = Sr25519Keyring::Alice.to_account_id();

        let pk = hex::decode("1228875e855ad2af220194090e3de95e497a3f257665a005bdb9c65d012ac98b2ca6ca77740bb47ba300033b29873db46a869755e82570d8bc8f426bb153eff6").expect("Invalid hex");
        let sig= hex::decode("9b252b7112c6d38215726a5fbeaa53172e1a343ce96f8aa7441561f4947b08248ffdc568aee62d07c7651c0b881bcaa437e0b9e1fb6ffc807d3cd8287fedc54c").expect("Invalid hex");

        let id = Identity {
            pub_key: pk.clone(),
            account_id: applier.clone(),
            validator_pub_key: pk.clone(),
            validator_account_id: validator.clone(),
            sig: sig.clone()
        };

        assert!(!Tee::identity_sig_check(&id));
    });
}

#[test]
fn test_for_identity_failed_by_duplicate_pk() {
    new_test_ext().execute_with(|| {
        // 1. Register applier
        let applier: AccountId =
            AccountId::from_ss58check("5HZFQohYpN4MVyGjiq8bJhojt9yCVa8rXd4Kt9fmh5gAbQqA")
                .expect("valid ss58 address");
        let id = get_valid_identity();

        assert_ok!(Tee::register_identity(
            Origin::signed(applier.clone()),
            id.clone()
        ));

        let id_registered = Tee::tee_identities(applier.clone()).unwrap();

        assert_eq!(id.clone(), id_registered);

        // 2. Register same pk applier
        let dup_id = get_valid_identity();
        assert!(Tee::register_identity(
            Origin::signed(applier.clone()),
            dup_id.clone()
        ).is_err());
    });
}

#[test]
fn test_for_report_works_success() {
    new_test_ext().execute_with(|| {
        // generate 53 blocks first
        run_to_block(303);

        let account: AccountId = Sr25519Keyring::Bob.to_account_id();

        // Check workloads
        assert_eq!(Tee::reserved(), 0);

        assert_ok!(Tee::report_works(
            Origin::signed(account.clone()),
            get_valid_work_report()
        ));

        // Check workloads after work report
        assert_eq!(Tee::reserved(), 4294967296);
        assert_eq!(Tee::used(), 402868224);
    });
}

#[test]
fn test_for_report_works_failed_by_pub_key_is_not_found() {
    new_test_ext().execute_with(|| {
        let account: AccountId32 = Sr25519Keyring::Bob.to_account_id();

        let mut works = get_valid_work_report();
        works.pub_key = "another_pub_key".as_bytes().to_vec();

        assert!(Tee::report_works(Origin::signed(account), works).is_err());
    });
}

#[test]
fn test_for_report_works_failed_by_reporter_is_not_registered() {
    new_test_ext().execute_with(|| {
        let account: AccountId32 = Sr25519Keyring::Dave.to_account_id();

        let works = WorkReport {
            pub_key: "pub_key_bob".as_bytes().to_vec(),
            block_number: 50,
            block_hash: "block_hash".as_bytes().to_vec(),
            used: 2000,
            reserved: 2000,
            sig: "sig_key_bob".as_bytes().to_vec(),
            files: vec![]
        };

        assert!(Tee::report_works(Origin::signed(account), works).is_err());
    });
}

#[test]
fn test_for_work_report_timing_check_failed_by_wrong_hash() {
    new_test_ext().execute_with(|| {
        // generate 50 blocks first
        run_to_block(50);

        let account: AccountId32 = Sr25519Keyring::Bob.to_account_id();
        let block_hash = [1; 32].to_vec();

        let works = WorkReport {
            pub_key: "pub_key_alice".as_bytes().to_vec(),
            block_number: 50,
            block_hash,
            used: 0,
            reserved: 0,
            sig: "sig_key_alice".as_bytes().to_vec(),
            files: vec![]
        };

        assert!(Tee::report_works(Origin::signed(account), works).is_err());
    });
}

#[test]
fn test_for_work_report_timing_check_failed_by_slot_outdated() {
    new_test_ext().execute_with(|| {
        // generate 50 blocks first
        run_to_block(103);

        let account: AccountId32 = Sr25519Keyring::Bob.to_account_id();
        let block_hash = [0; 32].to_vec();

        let works = WorkReport {
            pub_key: "pub_key_alice".as_bytes().to_vec(),
            block_number: 50,
            block_hash,
            used: 0,
            reserved: 1999,
            sig: "sig_key_alice".as_bytes().to_vec(),
            files: vec![]
        };

        assert!(Tee::report_works(Origin::signed(account), works).is_err());
    });
}

#[test]
fn test_for_work_report_sig_check_failed() {
    new_test_ext().execute_with(|| {
        // generate 53 blocks first
        run_to_block(53);

        let account: AccountId32 = Sr25519Keyring::Bob.to_account_id();
        let pub_key = hex::decode("b0b0c191996073c67747eb1068ce53036d76870516a2973cef506c29aa37323892c5cc5f379f17e63a64bb7bc69fbea14016eea76dae61f467c23de295d7f689").unwrap();
        let block_hash = hex::decode("05404b690b0c785bf180b2dd82a431d88d29baf31346c53dbda95e83e34c8a75").unwrap();
        let files: Vec<(Vec<u8>, u64)> = [
            (hex::decode("5bb706320afc633bfb843108e492192b17d2b6b9d9ee0b795ee95417fe08b660").unwrap(), 134289408),
            (hex::decode("88cdb315c9c37e2dc00fa2a8c7fe51b8149b363d29f404441982f96d2bbae65f").unwrap(), 268578816)
        ].to_vec();
        let sig = hex::decode("9c12986c01efe715ed8bed80b7e391601c45bf152e280693ffcfd10a4b386deaaa0f088fc26b0ebeca64c33cf122d372ebd787aa77beaaba9d2e499ce40a76e6").unwrap();

        let works = WorkReport {
            pub_key,
            block_number: 300,
            block_hash,
            used: 0,
            reserved: 4294967296,
            sig,
            files
        };

        assert!(Tee::report_works(Origin::signed(account), works).is_err());
    });
}

#[test]
fn test_for_outdated_work_reports() {
    new_test_ext().execute_with(|| {
        let account: AccountId = Sr25519Keyring::Bob.to_account_id();
        let mut final_wr = get_valid_work_report();
        final_wr.used = 402868224;
        // generate 303 blocks first
        run_to_block(303);

        // report works should ok
        assert_ok!(Tee::report_works(
            Origin::signed(account.clone()),
            get_valid_work_report()
        ));
        assert_eq!(
            Tee::work_reports(&account),
            Some(final_wr.clone())
        );

        // check work report and workload, current_report_slot updating should work
        assert_eq!(Tee::current_report_slot(), 0);
        Tee::update_identities();
        assert_eq!(Tee::current_report_slot(), 300);
        // Check workloads
        assert_eq!(Tee::reserved(), 4294967296);
        assert_eq!(Tee::used(), 402868224);

        // generate 401 blocks, wr still valid
        run_to_block(401);
        assert_eq!(
            Tee::work_reports(&account),
            Some(final_wr.clone())
        );
        assert!(Tee::reported_in_slot(&account, 300));

        // generate 602 blocks
        run_to_block(602);
        assert_eq!(Tee::current_report_slot(), 300);
        Tee::update_identities();
        assert_eq!(Tee::current_report_slot(), 600);
        assert_eq!(
            Tee::work_reports(&account),
            Some(final_wr.clone())
        );
        assert!(!Tee::reported_in_slot(&account, 600));

        // Check workloads
        assert_eq!(Tee::reserved(), 4294967296);
        assert_eq!(Tee::used(), 402868224);

        run_to_block(903);
        assert_eq!(Tee::current_report_slot(), 600);
        Tee::update_identities();
        assert_eq!(Tee::current_report_slot(), 900);

        // Check workloads
        assert_eq!(Tee::work_reports(&account), None);
        assert_eq!(Tee::reserved(), 0);
        assert_eq!(Tee::used(), 0);
    });
}

#[test]
fn test_abnormal_era() {
    new_test_ext().execute_with(|| {
        let account: AccountId = Sr25519Keyring::Bob.to_account_id();
        let mut final_wr = get_valid_work_report();
        final_wr.used = 402868224;

        // If new era happens in 101, next work is not reported
        run_to_block(101);
        Tee::update_identities();
        assert_eq!(
            Tee::work_reports(&account),
            Some(Default::default())
        );
        assert_eq!(Tee::reserved(), 0);
        assert_eq!(Tee::current_report_slot(), 0);

        // If new era happens on 301, we should update work report and current report slot
        run_to_block(301);
        Tee::update_identities();
        assert_eq!(
            Tee::work_reports(&account),
            Some(Default::default())
        );
        assert_eq!(
            Tee::current_report_slot(),
            300
        );
        assert!(Tee::reported_in_slot(&account, 0));

        // If next new era happens on 303, then nothing should happen
        run_to_block(303);
        Tee::update_identities();
        assert_eq!(
            Tee::work_reports(&account),
            Some(Default::default())
        );
        assert_eq!(
            Tee::current_report_slot(),
            300
        );
        assert!(Tee::reported_in_slot(&account, 0));
        assert!(!Tee::reported_in_slot(&account, 300));

        // Then report works
        // reserved: 4294967296,
        // used: 1676266280,
        run_to_block(304);
        assert_ok!(Tee::report_works(
            Origin::signed(account.clone()),
            get_valid_work_report()
        ));
        assert_eq!(Tee::work_reports(&account), Some(final_wr));
        // total workload should keep same, cause we only updated in a new era
        assert_eq!(Tee::reserved(), 4294967296);
        assert_eq!(Tee::used(), 402868224);
        assert!(Tee::reported_in_slot(&account, 300));
    })
}
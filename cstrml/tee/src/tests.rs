use super::*;

use crate::mock::{Tee, Staking, Origin, new_test_ext, run_to_block};
use frame_support::assert_ok;
use sp_core::crypto::{AccountId32, Ss58Codec};
use keyring::Sr25519Keyring;
use hex;

use cstrml_staking as staking;
use staking::{StakingLedger, Exposure, IndividualExposure};
use primitives::constants::currency::CRUS;

type AccountId = AccountId32;

fn get_valid_identity() -> Identity<AccountId> {
    // Alice is validator in genesis block
    let applier: AccountId = AccountId::from_ss58check("5Cowt7B9CbBa3CffyusJTCuhT33WcwpqRoULdSQwwmKHNRW2").expect("valid ss58 address");
    let validator: AccountId = Sr25519Keyring::Alice.to_account_id();

    let pk = hex::decode("5c4af2d40f305ce58aed1c6a8019a61d004781396c1feae5784a5f28cc8c40abe4229b13bc803ae9fbe93f589a60220b9b4816a5a199dfdab4a39b36c86a4c37").unwrap();
    let sig= hex::decode("5188fad93d76346415581218082d6239ea5c0a4be251aa20d2464080d662259f791bf78dbe1bd090abb382a6d13959538371890bc2741f08090465eac91dee4a").expect("Invalid hex");

    Identity {
        pub_key: pk.clone(),
        account_id: applier.clone(),
        validator_pub_key: pk.clone(),
        validator_account_id: validator.clone(),
        sig: sig.clone()
    }
}

fn get_valid_work_report() -> WorkReport {
    let pub_key = hex::decode("19817c0e3be0793b9c27b6064aeac6a82df3335a2ccdac7ea3d4c56e96a315a1e4dfe23491330c1ba11347ca4d6474151636ec15a7fc45d219d034eb9a33bb75").unwrap();
    let block_hash= [0; 32].to_vec();
    let empty_root = hex::decode("ae56f97320fbebbd1dd44d573486261709f5497d9d8391b2b2b6c23287927f5d").unwrap();
    let sig = hex::decode("4b23f3a95015387c735100dae6fdcb78445a2db50d08e19713bff900b69bc8c0719bf62750b30b92d082adfe8e0fa705a6cbe909c09961b4d0cc5f22d5c91581").unwrap();

    WorkReport {
        pub_key,
        block_number: 50,
        block_hash,
        empty_root,
        empty_workload: 4294967296,
        meaningful_workload: 1676266280,
        sig
    }
}

#[test]
fn test_for_register_identity_success() {
    new_test_ext().execute_with(|| {
        // Alice is validator in genesis block
        let applier: AccountId = AccountId::from_ss58check("5Cowt7B9CbBa3CffyusJTCuhT33WcwpqRoULdSQwwmKHNRW2").expect("valid ss58 address");
        let id = get_valid_identity();

        assert_ok!(Tee::register_identity(Origin::signed(applier.clone()), id.clone()));

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
            sig: "sig".as_bytes().to_vec()
        };

        assert_ok!(Tee::register_identity(Origin::signed(alice.clone()), id.clone()));
    });
}


#[test]
fn test_for_register_identity_failed_by_validator_illegal() {
    new_test_ext().execute_with(|| {
        // Bob is not validator before
        let account: AccountId32 = Sr25519Keyring::Bob.to_account_id();

        let id = Identity {
            pub_key: "pub_key_bob".as_bytes().to_vec(),
            account_id: account.clone(),
            validator_pub_key: "pub_key_bob".as_bytes().to_vec(),
            validator_account_id: account.clone(),
            sig: "sig_bob".as_bytes().to_vec()
        };

        assert!(Tee::register_identity(Origin::signed(account.clone()), id.clone()).is_err());
    });
}

#[test]
fn test_for_register_identity_failed_by_validate_for_self() {
    new_test_ext().execute_with(|| {
        let applier: AccountId = AccountId::from_ss58check("5Cowt7B9CbBa3CffyusJTCuhT33WcwpqRoULdSQwwmKHNRW2").expect("valid ss58 address");

        // 1. register bob by alice
        let mut id = get_valid_identity();

        assert_ok!(Tee::register_identity(Origin::signed(applier.clone()), id.clone()));

        // 2. register bob by bob
        id.validator_account_id = applier.clone();

        assert!(Tee::register_identity(Origin::signed(applier.clone()), id.clone()).is_err());
    });
}

#[test]
fn test_for_identity_sig_check_failed() {
    new_test_ext().execute_with(|| {
        // Alice is validator in genesis block
        let applier: AccountId = AccountId::from_ss58check("5Cowt7B9CbBa3CffyusJTCuhT33WcwpqRoULdSQwwmKHNRW2").expect("valid ss58 address");
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
fn test_for_report_works_success() {
    new_test_ext().execute_with(|| {
        // generate 53 blocks first
        run_to_block(53);

        let account: AccountId = Sr25519Keyring::Alice.to_account_id();
        let stash_account: AccountId = Sr25519Keyring::One.to_account_id();
        let works = get_valid_work_report();

        let ledger = Staking::ledger(&account).unwrap();
        let stakers = Staking::stakers(&stash_account);
        let stakes = Staking::slot_stake();

        assert_eq!(&ledger.stash, &stash_account);
        assert_eq!(
            Staking::stakers(&stash_account),
            Exposure { total: 15_000 * CRUS, own: 10_000 * CRUS, others: vec![IndividualExposure { who: Sr25519Keyring::Two.to_account_id(), value: 5000 * CRUS }] }
        );

        // Check workloads
        assert_eq!(Tee::workloads(), None);

        assert_ok!(Tee::report_works(Origin::signed(account.clone()), works));

        let limited_stakes = 5971233576 * (CRUS / 1_000_000);
        // Check how much is at stake
        assert_eq!(Staking::ledger(&account), Some(StakingLedger {
            stash: stash_account,
            total: limited_stakes,
            active: limited_stakes,
            unlocking: vec![],
        }));

        // Check workloads after work report
        assert_eq!(Tee::workloads(), Some(5971233576));
    });
}


#[test]
fn test_for_report_works_failed_by_reporter_is_not_registered() {
    new_test_ext().execute_with(|| {
        let account: AccountId32 = Sr25519Keyring::Bob.to_account_id();

        let works = WorkReport {
            pub_key: "pub_key_bob".as_bytes().to_vec(),
            block_number: 50,
            block_hash: "block_hash".as_bytes().to_vec(),
            empty_root: "merkle_root_bob".as_bytes().to_vec(),
            empty_workload: 2000,
            meaningful_workload: 2000,
            sig: "sig_key_bob".as_bytes().to_vec()
        };

        assert!(Tee::report_works(Origin::signed(account), works).is_err());
    });
}

#[test]
fn test_for_work_report_timing_check_failed_by_wrong_hash() {
    new_test_ext().execute_with(|| {
        // generate 50 blocks first
        run_to_block(50);

        let account: AccountId32 = Sr25519Keyring::Alice.to_account_id();
        let block_hash= [1; 32].to_vec();

        let works = WorkReport {
            pub_key: "pub_key_alice".as_bytes().to_vec(),
            block_number: 50,
            block_hash,
            empty_root: "merkle_root_alice".as_bytes().to_vec(),
            empty_workload: 0,
            meaningful_workload: 1000,
            sig: "sig_key_alice".as_bytes().to_vec()
        };

        assert!(Tee::report_works(Origin::signed(account), works).is_err());
    });
}

#[test]
fn test_for_work_report_timing_check_failed_by_slot_outdated() {
    new_test_ext().execute_with(|| {
        // generate 50 blocks first
        run_to_block(103);

        let account: AccountId32 = Sr25519Keyring::Alice.to_account_id();
        let block_hash= [0; 32].to_vec();

        let works = WorkReport {
            pub_key: "pub_key_alice".as_bytes().to_vec(),
            block_number: 50,
            block_hash,
            empty_root: "merkle_root_alice".as_bytes().to_vec(),
            empty_workload: 5000,
            meaningful_workload: 1000,
            sig: "sig_key_alice".as_bytes().to_vec()
        };

        assert!(Tee::report_works(Origin::signed(account), works).is_err());
    });
}

#[test]
fn test_for_work_report_sig_check_failed() {
    new_test_ext().execute_with(|| {
        // generate 53 blocks first
        run_to_block(53);

        let account: AccountId32 = Sr25519Keyring::Alice.to_account_id();
        let pub_key = hex::decode("19817c0e3be0793b9c27b6064aeac6a82df3335a2ccdac7ea3d4c56e96a315a1e4dfe23491330c1ba11347ca4d6474151636ec15a7fc45d219d034eb9a33bb75").unwrap();
        let block_hash= [0; 32].to_vec();
        let empty_root = hex::decode("ae56f97320fbebbd1dd44d573486261709f5497d9d8391b2b2b6c23287927f5d").unwrap();
        let sig = hex::decode("4b23f3a95015387c735100dae6fdcb78445a2db50d08e19713bff900b69bc8c0719bf62750b30b92d082adfe8e0fa705a6cbe909c09961b4d0cc5f22d5c91599").unwrap();

        let works = WorkReport {
            pub_key,
            block_number: 50,
            block_hash,
            empty_root,
            empty_workload: 4294967296,
            meaningful_workload: 1676266280,
            sig
        };

        assert!(Tee::report_works(Origin::signed(account), works).is_err());
    });
}

#[test]
fn test_for_check_and_set_stake_limitation_success() {
    new_test_ext().execute_with(|| {
        let account: AccountId = Sr25519Keyring::Alice.to_account_id();
        let stash_account: AccountId = Sr25519Keyring::One.to_account_id();

        // Alice is validator and staked 1,000 CRUs
        Tee::check_and_set_stake_limitation(&account, 5000_000_000);

        let limited_stakes = 5000_000_000 * (CRUS / 1_000_000);
        // Check how much is at stake
        assert_eq!(Staking::ledger(&account), Some(StakingLedger {
            stash: stash_account,
            total: limited_stakes,
            active: limited_stakes,
            unlocking: vec![],
        }));
    });
}


// Copyright (C) 2019-2021 Crust Network Technologies Ltd.
// This file is part of Crust.

use super::*;
use crate::{mock::*, Error};
use frame_support::{assert_noop, assert_ok, dispatch::DispatchError};

/// CRU claims test cases
#[test]
fn happy_pass_should_work() {
    new_test_ext().execute_with(|| {
        // 0. Set miner and superior
        assert_ok!(CrustClaims::change_miner(Origin::root(), 1));
        assert_ok!(CrustClaims::change_superior(Origin::root(), 2));

        // 1. Set claim limit = 100
        assert_ok!(CrustClaims::set_claim_limit(Origin::signed(2), 100));
        assert_eq!(CrustClaims::claim_limit(), 100);

        // 2. Mint a claim
        let tx_hash = get_legal_tx_hash();
        let eth_addr = get_legal_eth_addr();
        assert_ok!(CrustClaims::mint_claim(Origin::signed(1), tx_hash.clone(), eth_addr.clone(), 100));

        // 3. Storage should ok
        assert_eq!(CrustClaims::claims(&tx_hash), Some((eth_addr.clone(), 100))); // new tx
        assert_eq!(CrustClaims::claimed(&tx_hash), false); // tx has not be claimed
        assert_eq!(CrustClaims::claim_limit(), 0);

        // 4. Claim it with correct msg sig
        // Pay RUSTs to the TEST account:0100000000000000
        let sig = get_legal_eth_sig();
        assert_eq!(Balances::free_balance(1), 0);
        assert_ok!(CrustClaims::claim(Origin::none(), 1, tx_hash.clone(), sig.clone()));

        // 5. Claim success
        assert_eq!(Balances::free_balance(1), 100);
        assert_eq!(CrustClaims::claimed(&tx_hash), true); // tx has already be claimed
    });
}

#[test]
fn change_miner_should_work() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            CrustClaims::change_miner(Origin::signed(1), 1),
            DispatchError::BadOrigin
        );

        // 0. Set miner
        assert_ok!(CrustClaims::change_miner(Origin::root(), 1)); // 1 is miner

        // 1. Mint a claim with 2, no way
        let tx_hash = get_legal_tx_hash();
        let eth_addr = get_legal_eth_addr();
        assert_noop!(
            CrustClaims::mint_claim(Origin::signed(2), tx_hash.clone(), eth_addr.clone(), 100),
            Error::<Test>::IllegalMiner
        );
    });
}

#[test]
fn tx_should_exist() {
    new_test_ext().execute_with(|| {
        let tx_hash = get_legal_tx_hash();
        let sig = get_legal_eth_sig();
        assert_noop!(
            CrustClaims::claim(Origin::none(), 1, tx_hash, sig),
            Error::<Test>::SignerHasNoClaim
        );
    });
}

#[test]
fn illegal_sig_claim_should_failed() {
    new_test_ext().execute_with(|| {
        // 0. Set miner and superior
        assert_ok!(CrustClaims::change_miner(Origin::root(), 1));
        assert_ok!(CrustClaims::change_superior(Origin::root(), 2));

        // 1. Set limitation
        assert_ok!(CrustClaims::set_claim_limit(Origin::signed(2), 100));

        // 2. Mint a claim
        let tx_hash = get_legal_tx_hash();
        let eth_addr = get_legal_eth_addr();
        assert_ok!(CrustClaims::mint_claim(Origin::signed(1), tx_hash.clone(), eth_addr.clone(), 100));

        // 3. Claim it with illegal sig
        // 3.1 Another eth account wanna this money, go fuck himself
        let sig1 = get_another_account_eth_sig();
        assert_noop!(
            CrustClaims::claim(Origin::none(), 1, tx_hash.clone(), sig1.clone()),
            Error::<Test>::SignatureNotMatch
        );

        // 3.2 Sig with wrong message should failed
        let sig2 = get_wrong_msg_eth_sig();
        assert_noop!(
            CrustClaims::claim(Origin::none(), 1, tx_hash.clone(), sig2.clone()),
            Error::<Test>::SignatureNotMatch
        );
    });
}

#[test]
fn double_mint_should_failed() {
    new_test_ext().execute_with(|| {
        // 0. Set miner and superior
        assert_ok!(CrustClaims::change_miner(Origin::root(), 1));
        assert_ok!(CrustClaims::change_superior(Origin::root(), 2));

        // 1. Set limit
        assert_ok!(CrustClaims::set_claim_limit(Origin::signed(2), 100));

        // 2. Mint a claim
        let tx_hash = get_legal_tx_hash();
        let eth_addr = get_legal_eth_addr();
        assert_ok!(CrustClaims::mint_claim(Origin::signed(1), tx_hash.clone(), eth_addr.clone(), 100));

        // 3. Mint the same eth again
        assert_noop!(
            CrustClaims::mint_claim(Origin::signed(1), tx_hash.clone(), eth_addr.clone(), 100),
            Error::<Test>::AlreadyBeMint
        );
    });
}

#[test]
fn double_claim_should_failed() {
    new_test_ext().execute_with(|| {
        // 0. Set miner and superior
        assert_ok!(CrustClaims::change_miner(Origin::root(), 1));
        assert_ok!(CrustClaims::change_superior(Origin::root(), 2));

        // 1. Set limitation
        assert_ok!(CrustClaims::set_claim_limit(Origin::signed(2), 100));

        // 2. Mint a claim
        let tx_hash = get_legal_tx_hash();
        let eth_addr = get_legal_eth_addr();
        assert_ok!(CrustClaims::mint_claim(Origin::signed(1), tx_hash.clone(), eth_addr.clone(), 100));

        // 3. Claim it
        // Pay RUSTs to the TEST account:0100000000000000
        let sig = get_legal_eth_sig();
        assert_eq!(Balances::free_balance(1), 0);
        assert_ok!(CrustClaims::claim(Origin::none(), 1, tx_hash.clone(), sig.clone()));
        assert_eq!(Balances::free_balance(1), 100);

        // 4. Claim again, in ur dream 🙂
        assert_noop!(
            CrustClaims::claim(Origin::none(), 1, tx_hash.clone(), sig.clone()),
            Error::<Test>::AlreadyBeClaimed
        );
        assert_eq!(Balances::free_balance(1), 100);
    });
}

#[test]
fn claim_limit_should_work() {
    new_test_ext().execute_with(|| {
        // 0. Set miner and superior
        assert_ok!(CrustClaims::change_miner(Origin::root(), 1));
        assert_ok!(CrustClaims::change_superior(Origin::root(), 2));

        // 1. Mint a claim should failed without limitation
        let tx_hash = get_legal_tx_hash();
        let eth_addr = get_legal_eth_addr();
        assert_noop!(
            CrustClaims::mint_claim(Origin::signed(1), tx_hash.clone(), eth_addr.clone(), 10),
            Error::<Test>::ExceedClaimLimit
        );

        // 2. Set limitation
        assert_ok!(CrustClaims::set_claim_limit(Origin::signed(2), 10));
        assert_eq!(CrustClaims::claim_limit(), 10);

        // 3. Claim amount with limitation should be ok
        assert_ok!(CrustClaims::mint_claim(Origin::signed(1), tx_hash.clone(), eth_addr.clone(), 10));
        assert_eq!(CrustClaims::claim_limit(), 0);
    });
}

#[test]
fn bond_eth_should_work() {
    new_test_ext().execute_with(|| {
        let eth_addr = get_legal_eth_addr();
        assert_ok!(CrustClaims::bond_eth(Origin::signed(1), eth_addr.clone()));
        assert_eq!(CrustClaims::bonded_eth(1), Some(eth_addr));
    });
}

#[test]
fn force_claim_should_work() {
    new_test_ext().execute_with(|| {
        // 0. Set miner and superior
        assert_ok!(CrustClaims::change_miner(Origin::root(), 1));
        assert_ok!(CrustClaims::change_superior(Origin::root(), 2));

        // 1. Set claim limit = 100
        assert_ok!(CrustClaims::set_claim_limit(Origin::signed(2), 100));
        assert_eq!(CrustClaims::claim_limit(), 100);

        // 2. Mint a claim
        let tx_hash = get_legal_tx_hash1();
        let eth_addr = get_legal_eth_addr();
        assert_ok!(CrustClaims::mint_claim(Origin::signed(1), tx_hash.clone(), eth_addr.clone(), 100));

        // 3. Total issuance should be 0
        assert_eq!(Balances::total_issuance(), 0);

        // 4. Force claim should be ok
        assert_ok!(CrustClaims::force_claim(Origin::root(), tx_hash.clone()));
        assert_eq!(CrustClaims::claimed(tx_hash.clone()), true);

        // 5. Total issuance should not change
        assert_eq!(Balances::total_issuance(), 0);

        // 6. Claim should failed
        let legal_sig = get_claim_legal_eth_sig();
        assert_noop!(
            CrustClaims::claim(Origin::none(), 1, tx_hash.clone(), legal_sig.clone()),
            Error::<Test>::AlreadyBeClaimed
        );
    });
}

/// CRU18 claims test cases
#[test]
fn cru18_happy_pass_should_work() {
    new_test_ext().execute_with(|| {
        // 0. Set cru18 miner
        assert_ok!(CrustClaims::set_cru18_miner(Origin::root(), 1));

        // 1. Mint a pre claim
        let eth_addr = get_legal_eth_addr();
        assert_ok!(CrustClaims::mint_cru18_claim(Origin::signed(1), eth_addr.clone(), 100));

        // 3. Check state
        assert_eq!(CrustClaims::cru18_pre_claims(&eth_addr), Some(100)); // new locked cru mapping with address
        assert_eq!(CrustClaims::cru18_claimed(&eth_addr), false); // address with cru has not be claimed
        assert_eq!(CrustClaims::cru18_total_claimed(), 0); // address with cru has not be claimed

        // 4. Claim it with correct msg sig
        // Pay RUSTs to the TEST account:0100000000000000
        let sig = get_legal_eth_sig();
        assert_eq!(Balances::free_balance(1), 0);
        assert_ok!(CrustClaims::claim_cru18(Origin::none(), 1, sig));

        // 5. Claim should success
        assert_eq!(CrustClaims::cru18_claims(&eth_addr, 1), Some(100)); // new locked cru  has already be claimed
        assert_eq!(CrustClaims::cru18_claimed(&eth_addr), true); // address with cru has not be claimed
        assert_eq!(CrustClaims::cru18_total_claimed(), 100); // address with cru has not be claimed
    });
}

#[test]
fn change_cru18_miner_should_work() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            CrustClaims::set_cru18_miner(Origin::signed(1), 1),
            DispatchError::BadOrigin
        );

        // 0. Set cru18 miner
        assert_ok!(CrustClaims::set_cru18_miner(Origin::root(), 1)); // 1 is miner

        // 1. Mint a pre claim with 2, no way
        let eth_addr = get_legal_eth_addr();
        assert_noop!(
            CrustClaims::mint_cru18_claim(Origin::signed(2), eth_addr.clone(), 100),
            Error::<Test>::IllegalMiner
        );
    });
}

#[test]
fn cru18_claim_should_failed_with_illegal_sig() {
    new_test_ext().execute_with(|| {
        // 0. Set cru18 miner
        assert_ok!(CrustClaims::set_cru18_miner(Origin::root(), 1));

        // 1. Mint a claim
        let eth_addr = get_legal_eth_addr();
        assert_ok!(CrustClaims::mint_cru18_claim(Origin::signed(1), eth_addr.clone(), 100));

        // 2. Claim it with illegal sig
        // 2.1 Another eth account wanna this money, go fuck himself
        let sig1 = get_another_account_eth_sig();
        assert_noop!(
            CrustClaims::claim_cru18(Origin::none(), 1, sig1.clone()),
            Error::<Test>::SignerHasNoPreClaim
        );

        // 2.2 Sig with wrong message should failed
        let sig2 = get_wrong_msg_eth_sig();
        assert_noop!(
            CrustClaims::claim_cru18(Origin::none(), 1, sig2.clone()),
            Error::<Test>::SignerHasNoPreClaim
        );
    });
}

#[test]
fn double_cru18_mint_should_failed() {
    new_test_ext().execute_with(|| {
        // 0. Set cru18 miner
        assert_ok!(CrustClaims::set_cru18_miner(Origin::root(), 1));

        // 1. Mint a claim
        let eth_addr = get_legal_eth_addr();
        assert_ok!(CrustClaims::mint_cru18_claim(Origin::signed(1), eth_addr.clone(), 100));

        // 2. Mint the same eth again
        assert_noop!(
            CrustClaims::mint_cru18_claim(Origin::signed(1), eth_addr.clone(), 100),
            Error::<Test>::AlreadyBeMint
        );
    });
}

#[test]
fn double_cru18_claim_should_failed() {
    new_test_ext().execute_with(|| {
        // 0. Set cru18 miner
        assert_ok!(CrustClaims::set_cru18_miner(Origin::root(), 1));

        // 1. Mint a cru18 claim
        let eth_addr = get_legal_eth_addr();
        assert_ok!(CrustClaims::mint_cru18_claim(Origin::signed(1), eth_addr.clone(), 100));

        // 2. Claim it
        // Pay RUSTs to the TEST account:0100000000000000
        let sig = get_legal_eth_sig();
        assert_ok!(CrustClaims::claim_cru18(Origin::none(), 1, sig.clone()));
        assert_eq!(
            CrustClaims::cru18_claims(eth_addr.clone(), 1),
            Some(100)
        );

        // 4. Claim again, in ur shit dream 🙂
        assert_noop!(
            CrustClaims::claim_cru18(Origin::none(), 1, sig.clone()),
            Error::<Test>::AlreadyBeClaimed
        );
        // Should not changed
        assert_eq!(
            CrustClaims::cru18_claims(eth_addr.clone(), 1),
            Some(100)
        );
    });
}

#[test]
fn force_delete_cru18_preclaim_should_work() {
    new_test_ext().execute_with(|| {
        // 0. Set cru18 miner
        assert_ok!(CrustClaims::set_cru18_miner(Origin::root(), 1));

        // 1. Mint a cru18 claim
        let eth_addr = get_legal_eth_addr();
        assert_ok!(CrustClaims::mint_cru18_claim(Origin::signed(1), eth_addr.clone(), 100));

        // 2. Force delete preclaim
        assert_ok!(CrustClaims::force_delete_cru18_preclaim(Origin::root(), eth_addr.clone()));

        // 3. Check storage
        assert_eq!(CrustClaims::cru18_pre_claims(eth_addr.clone()), None);
        assert_eq!(CrustClaims::cru18_claimed(eth_addr.clone()), false);

        // 4. Claim with legal sig should failed
        // Pay RUSTs to the TEST account:0100000000000000
        let sig = get_cru18_claim_legal_eth_sig();
        assert_noop!(
            CrustClaims::claim_cru18(Origin::none(), 1, sig.clone()),
            Error::<Test>::SignerHasNoPreClaim
        );
    });
}

/// CSM claims test cases
#[test]
fn csm_happy_pass_should_work() {
    new_test_ext().execute_with(|| {
        // 0. Set csm miner and superior
        assert_ok!(CrustClaims::change_csm_miner(Origin::root(), 1));
        assert_ok!(CrustClaims::change_superior(Origin::root(), 2));

        // 1. Set csm claim limit = 100
        assert_ok!(CrustClaims::set_csm_claim_limit(Origin::signed(2), 100));
        assert_eq!(CrustClaims::csm_claim_limit(), 100);

        // 2. Mint a csm claim
        let tx_hash = get_legal_tx_hash1();
        let eth_addr = get_legal_eth_addr();
        assert_ok!(CrustClaims::mint_csm_claim(Origin::signed(1), tx_hash.clone(), eth_addr.clone(), 100));

        // 3. Storage should ok
        assert_eq!(CrustClaims::csm_claims(&tx_hash), Some((eth_addr.clone(), 100))); // new tx
        assert_eq!(CrustClaims::csm_claimed(&tx_hash), false); // tx has not be claimed
        assert_eq!(CrustClaims::csm_claim_limit(), 0);

        // 4. Claim csm with correct msg sig
        // Pay RUSTs to the TEST account:0100000000000000
        let sig = get_claim_legal_eth_sig();
        assert_eq!(CSM::free_balance(1), 0);
        assert_ok!(CrustClaims::claim_csm(Origin::none(), 1, tx_hash.clone(), sig.clone()));

        // 5. CSM claim success
        assert_eq!(CSM::free_balance(1), 100);
        assert_eq!(CrustClaims::csm_claimed(&tx_hash), true); // tx has already be claimed
    });
}

#[test]
fn change_csm_miner_should_work() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            CrustClaims::change_csm_miner(Origin::signed(1), 1),
            DispatchError::BadOrigin
        );

        // 0. Set CSM miner
        assert_ok!(CrustClaims::change_csm_miner(Origin::root(), 1)); // 1 is csm miner

        // 1. Mint a csm claim with 2, no way
        let tx_hash = get_legal_tx_hash1();
        let eth_addr = get_legal_eth_addr();
        assert_noop!(
            CrustClaims::mint_csm_claim(Origin::signed(2), tx_hash.clone(), eth_addr.clone(), 100),
            Error::<Test>::IllegalMiner
        );
    });
}

#[test]
fn csm_tx_should_exist() {
    new_test_ext().execute_with(|| {
        let tx_hash = get_legal_tx_hash1();
        let sig = get_claim_legal_eth_sig();
        assert_noop!(
            CrustClaims::claim_csm(Origin::none(), 1, tx_hash, sig),
            Error::<Test>::SignerHasNoCsmClaim
        );
    });
}

#[test]
fn illegal_sig_csm_claim_should_failed() {
    new_test_ext().execute_with(|| {
        // 0. Set CSM miner and superior
        assert_ok!(CrustClaims::change_csm_miner(Origin::root(), 1));
        assert_ok!(CrustClaims::change_superior(Origin::root(), 2));

        // 1. Set limitation
        assert_ok!(CrustClaims::set_csm_claim_limit(Origin::signed(2), 200));

        // 2. Mint a csm claim
        let tx_hash1 = get_legal_tx_hash1();
        let tx_hash2 = get_legal_tx_hash2();
        let eth_addr = get_legal_eth_addr();
        let sig = get_claim_legal_eth_sig();
        assert_ok!(CrustClaims::mint_csm_claim(Origin::signed(1), tx_hash1.clone(), eth_addr.clone(), 100));
        // This should only claim use both `eth_addr` and `sig(1, tx_hash2)`, that means `sig` cannot unlock it.
        assert_ok!(CrustClaims::mint_csm_claim(Origin::signed(1), tx_hash2.clone(), eth_addr.clone(), 90));

        // 3. Claim csm with illegal sig
        // 3.1 Another eth account wanna this money, go fuck himself
        let sig1 = get_claim_another_account_eth_sig();
        assert_noop!(
            CrustClaims::claim_csm(Origin::none(), 1, tx_hash1.clone(), sig1.clone()),
            Error::<Test>::SignatureNotMatch
        );

        // 3.2 Sig with wrong message should failed
        let sig2 = get_wrong_msg_eth_sig();
        assert_noop!(
            CrustClaims::claim_csm(Origin::none(), 1, tx_hash1.clone(), sig2.clone()),
            Error::<Test>::SignatureNotMatch
        );

        // 3.3 Sig with Puzzle {1, tx_hash2} but got Key {1, tx_hash1}
        assert_noop!(
            CrustClaims::claim_csm(Origin::none(), 1, tx_hash2.clone(), sig.clone()),
            Error::<Test>::SignatureNotMatch
        );
    });
}

#[test]
fn double_mint_csm_should_failed() {
    new_test_ext().execute_with(|| {
        // 0. Set CSM miner and superior
        assert_ok!(CrustClaims::change_csm_miner(Origin::root(), 1));
        assert_ok!(CrustClaims::change_superior(Origin::root(), 2));

        // 1. Set limit
        assert_ok!(CrustClaims::set_csm_claim_limit(Origin::signed(2), 100));

        // 2. Mint a claim
        let tx_hash = get_legal_tx_hash1();
        let eth_addr = get_legal_eth_addr();
        assert_ok!(CrustClaims::mint_csm_claim(Origin::signed(1), tx_hash.clone(), eth_addr.clone(), 100));

        // 3. Mint with the same eth hash again
        assert_noop!(
            CrustClaims::mint_csm_claim(Origin::signed(1), tx_hash.clone(), eth_addr.clone(), 100),
            Error::<Test>::AlreadyBeMint
        );
    });
}

#[test]
fn double_csm_claim_should_failed() {
    new_test_ext().execute_with(|| {
        // 0. Set CSM miner and superior
        assert_ok!(CrustClaims::change_csm_miner(Origin::root(), 1));
        assert_ok!(CrustClaims::change_superior(Origin::root(), 2));

        // 1. Set CSM limitation
        assert_ok!(CrustClaims::set_csm_claim_limit(Origin::signed(2), 100));

        // 2. Mint a CSM claim
        let tx_hash = get_legal_tx_hash1();
        let eth_addr = get_legal_eth_addr();
        assert_ok!(CrustClaims::mint_csm_claim(Origin::signed(1), tx_hash.clone(), eth_addr.clone(), 100));

        // 3. Claim it
        // Pay RUSTs to the TEST account:0100000000000000
        let sig = get_claim_legal_eth_sig();
        assert_eq!(CSM::free_balance(1), 0);
        assert_ok!(CrustClaims::claim_csm(Origin::none(), 1, tx_hash.clone(), sig.clone()));
        assert_eq!(CSM::free_balance(1), 100);

        // 4. Claim again, in ur dream 🙂
        assert_noop!(
            CrustClaims::claim_csm(Origin::none(), 1, tx_hash.clone(), sig.clone()),
            Error::<Test>::CsmAlreadyBeClaimed
        );
        assert_eq!(CSM::free_balance(1), 100);
    });
}

#[test]
fn csm_claim_limit_should_work() {
    new_test_ext().execute_with(|| {
        // 0. Set miner and superior
        assert_ok!(CrustClaims::change_csm_miner(Origin::root(), 1));
        assert_ok!(CrustClaims::change_superior(Origin::root(), 2));

        // 1. Mint a claim should failed without limitation
        let tx_hash = get_legal_tx_hash1();
        let eth_addr = get_legal_eth_addr();
        assert_noop!(
            CrustClaims::mint_csm_claim(Origin::signed(1), tx_hash.clone(), eth_addr.clone(), 10),
            Error::<Test>::ExceedCsmClaimLimit
        );

        // 2. Set limitation
        assert_ok!(CrustClaims::set_csm_claim_limit(Origin::signed(2), 10));
        assert_eq!(CrustClaims::csm_claim_limit(), 10);

        // 3. Claim amount with limitation should be ok
        assert_ok!(CrustClaims::mint_csm_claim(Origin::signed(1), tx_hash.clone(), eth_addr.clone(), 10));
        assert_eq!(CrustClaims::csm_claim_limit(), 0);
    });
}

#[test]
fn force_csm_claim_should_work() {
    new_test_ext().execute_with(|| {
        // 0. Set CSM miner and superior
        assert_ok!(CrustClaims::change_csm_miner(Origin::root(), 1));
        assert_ok!(CrustClaims::change_superior(Origin::root(), 2));

        // 1. Set CSM claim limit = 100
        assert_ok!(CrustClaims::set_csm_claim_limit(Origin::signed(2), 100));
        assert_eq!(CrustClaims::csm_claim_limit(), 100);

        // 2. Mint a claim
        let tx_hash = get_legal_tx_hash1();
        let eth_addr = get_legal_eth_addr();
        assert_ok!(CrustClaims::mint_csm_claim(Origin::signed(1), tx_hash.clone(), eth_addr.clone(), 100));

        // 3. Total issuance should be 0
        assert_eq!(CSM::total_issuance(), 0);

        // 4. Force CSM claim should be ok
        assert_ok!(CrustClaims::force_csm_claim(Origin::root(), tx_hash.clone()));
        assert_eq!(CrustClaims::csm_claimed(tx_hash.clone()), true);

        // 5. Total issuance should not change
        assert_eq!(CSM::total_issuance(), 0);

        // 6. Claim should failed
        let legal_sig = get_claim_legal_eth_sig();
        assert_noop!(
            CrustClaims::claim_csm(Origin::none(), 1, tx_hash.clone(), legal_sig.clone()),
            Error::<Test>::CsmAlreadyBeClaimed
        );
    });
}
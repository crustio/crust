// Copyright (C) 2019-2021 Crust Network Technologies Ltd.
// This file is part of Crust.

use super::*;
use crate::{mock::*, Error};
use frame_support::{assert_noop, assert_ok, dispatch::DispatchError};

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
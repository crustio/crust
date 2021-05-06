// Copyright (C) 2019-2021 Crust Network Technologies Ltd.
// This file is part of Crust.

//! Tests for the module.
use super::*;
use crate::mock::*;
use frame_support::{
    assert_noop, assert_ok,
    traits::{Currency},
};
use crate::CSMUnlockChunk;

#[test]
fn bond_should_work() {
    ExtBuilder::default().build().execute_with(|| {
        let _ = Balances::make_free_balance_be(&11, 1500);
        assert_ok!(CSMLocking::bond(Origin::signed(11), 1000));
        assert_eq!(
            CSMLocking::ledger(&11),
            CSMLedger {
                total: 1000,
                active: 1000,
                unlocking: vec![]
            }
        );
        assert_ok!(CSMLocking::bond(Origin::signed(11), 200));
        assert_eq!(
            CSMLocking::ledger(&11),
            CSMLedger {
                total: 1200,
                active: 1200,
                unlocking: vec![]
            }
        );

        let _ = Balances::make_free_balance_be(&11, 3000);

        assert_ok!(CSMLocking::bond(Origin::signed(11), 1800));
        assert_eq!(
            CSMLocking::ledger(&11),
            CSMLedger {
                total: 3000,
                active: 3000,
                unlocking: vec![]
            }
        );

        assert_ok!(CSMLocking::bond(Origin::signed(11), 1000));
        assert_eq!(
            CSMLocking::ledger(&11),
            CSMLedger {
                total: 3000,
                active: 3000,
                unlocking: vec![]
            }
        );
    });
}

#[test]
fn unbond_should_work() {
    ExtBuilder::default().build().execute_with(|| {
        let _ = Balances::make_free_balance_be(&11, 1500);
        assert_noop!(
            CSMLocking::unbond(Origin::signed(11), 500),
            Error::<Test>::NotBonded,
        );

        assert_ok!(CSMLocking::bond(Origin::signed(11), 1000));
        assert_eq!(
            CSMLocking::ledger(&11),
            CSMLedger {
                total: 1000,
                active: 1000,
                unlocking: vec![]
            }
        );
        run_to_block(300);
        assert_ok!(CSMLocking::unbond(Origin::signed(11), 200));
        assert_eq!(
            CSMLocking::ledger(&11),
            CSMLedger {
                total: 1000,
                active: 800,
                unlocking: vec![
                    CSMUnlockChunk {
                        value: 200,
                        bn: 1500
                    }
                ]
            }
        );

        let _ = Balances::make_free_balance_be(&11, 3000);

        assert_ok!(CSMLocking::bond(Origin::signed(11), 1800));
        // There is no limits
        assert_eq!(
            CSMLocking::ledger(&11),
            CSMLedger {
                total: 2800,
                active: 2600,
                unlocking: vec![
                    CSMUnlockChunk {
                        value: 200,
                        bn: 1500
                    }
                ]
            }
        );

        run_to_block(700);
        assert_ok!(CSMLocking::unbond(Origin::signed(11), 400));
        assert_eq!(
            CSMLocking::ledger(&11),
            CSMLedger {
                total: 2800,
                active: 2200,
                unlocking: vec![
                    CSMUnlockChunk {
                        value: 200,
                        bn: 1500
                    },
                    CSMUnlockChunk {
                        value: 400,
                        bn: 1900
                    }
                ]
            }
        );

        for _ in 0..30 {
            assert_ok!(CSMLocking::unbond(Origin::signed(11), 1));
        }
        assert_noop!(
            CSMLocking::unbond(Origin::signed(11), 500),
            Error::<Test>::NoMoreChunks,
        );
    });
}

#[test]
fn rebond_should_work() {
    ExtBuilder::default().build().execute_with(|| {
        let _ = Balances::make_free_balance_be(&11, 3000);
        assert_noop!(
            CSMLocking::rebond(Origin::signed(11), 500),
            Error::<Test>::NotBonded,
        );

        assert_ok!(CSMLocking::bond(Origin::signed(11), 1000));
        assert_eq!(
            CSMLocking::ledger(&11),
            CSMLedger {
                total: 1000,
                active: 1000,
                unlocking: vec![]
            }
        );
        run_to_block(300);
        assert_ok!(CSMLocking::unbond(Origin::signed(11), 200));
        assert_ok!(CSMLocking::bond(Origin::signed(11), 1800));
        run_to_block(700);
        assert_ok!(CSMLocking::unbond(Origin::signed(11), 400));

        run_to_block(1000);
        assert_ok!(CSMLocking::rebond(Origin::signed(11), 300));
        assert_eq!(
            CSMLocking::ledger(&11),
            CSMLedger {
                total: 2800,
                active: 2500,
                unlocking: vec![
                    CSMUnlockChunk {
                        value: 200,
                        bn: 1500
                    },
                    CSMUnlockChunk {
                        value: 100,
                        bn: 1900
                    }
                ]
            }
        );
        run_to_block(2000);
        assert_ok!(CSMLocking::rebond(Origin::signed(11), 200));
        assert_eq!(
            CSMLocking::ledger(&11),
            CSMLedger {
                total: 2800,
                active: 2700,
                unlocking: vec![
                    CSMUnlockChunk {
                        value: 100,
                        bn: 1500
                    },
                ]
            }
        );
        run_to_block(3000);
        assert_ok!(CSMLocking::rebond(Origin::signed(11), 2000));
        assert_eq!(
            CSMLocking::ledger(&11),
            CSMLedger {
                total: 2800,
                active: 2800,
                unlocking: vec![]
            }
        );

        assert_noop!(
            CSMLocking::rebond(Origin::signed(11), 500),
            Error::<Test>::NoUnlockChunk,
        );
    });
}


#[test]
fn withdraw_should_work() {
    ExtBuilder::default().build().execute_with(|| {
        let _ = Balances::make_free_balance_be(&11, 3000);
        assert_noop!(
            CSMLocking::withdraw_unbonded(Origin::signed(11)),
            Error::<Test>::NotBonded,
        );

        assert_ok!(CSMLocking::bond(Origin::signed(11), 1000));
        assert_eq!(
            CSMLocking::ledger(&11),
            CSMLedger {
                total: 1000,
                active: 1000,
                unlocking: vec![]
            }
        );
        run_to_block(300);
        assert_ok!(CSMLocking::unbond(Origin::signed(11), 200));
        assert_ok!(CSMLocking::bond(Origin::signed(11), 1800));
        run_to_block(700);
        assert_ok!(CSMLocking::unbond(Origin::signed(11), 400));
        assert_ok!(CSMLocking::unbond(Origin::signed(11), 400));

        run_to_block(1000);
        assert_ok!(CSMLocking::withdraw_unbonded(Origin::signed(11)));
        assert_eq!(
            CSMLocking::ledger(&11),
            CSMLedger {
                total: 2800,
                active: 1800,
                unlocking: vec![
                    CSMUnlockChunk {
                        value: 200,
                        bn: 1500
                    },
                    CSMUnlockChunk {
                        value: 400,
                        bn: 1900
                    },
                    CSMUnlockChunk {
                        value: 400,
                        bn: 1900
                    }
                ]
            }
        );

        run_to_block(1550);
        assert_ok!(CSMLocking::withdraw_unbonded(Origin::signed(11)));
        assert_eq!(
            CSMLocking::ledger(&11),
            CSMLedger {
                total: 2600,
                active: 1800,
                unlocking: vec![
                    CSMUnlockChunk {
                        value: 400,
                        bn: 1900
                    },
                    CSMUnlockChunk {
                        value: 400,
                        bn: 1900
                    }
                ]
            }
        );

        run_to_block(2000);
        assert_ok!(CSMLocking::withdraw_unbonded(Origin::signed(11)));
        assert_eq!(
            CSMLocking::ledger(&11),
            CSMLedger {
                total: 1800,
                active: 1800,
                unlocking: vec![]
            }
        );
        assert_ok!(CSMLocking::unbond(Origin::signed(11), 2200));
        run_to_block(3300);
        assert_ok!(CSMLocking::withdraw_unbonded(Origin::signed(11)));
        assert_eq!(<Ledger<Test>>::contains_key(&11), false);
        assert_eq!(Balances::locks(&1).len(), 0);
    });
}

#[test]
fn force_unstake_should_work() {
    ExtBuilder::default().build().execute_with(|| {
        let _ = Balances::make_free_balance_be(&11, 3000);
        assert_ok!(CSMLocking::bond(Origin::signed(11), 1000));
        assert_eq!(
            CSMLocking::ledger(&11),
            CSMLedger {
                total: 1000,
                active: 1000,
                unlocking: vec![]
            }
        );
        assert_ok!(CSMLocking::force_unstake(Origin::root(), 11));
        assert_eq!(<Ledger<Test>>::contains_key(&11), false);
        assert_eq!(Balances::locks(&1).len(), 0);
    });
}
// Copyright (C) 2019-2021 Crust Network Technologies Ltd.
// This file is part of Crust.

use super::*;
use crate::mock::*;
use frame_support::{assert_noop, assert_ok, dispatch::DispatchError};
use crate::{CRU18, CRU24, CRU_LOCK_ID};

pub const CRU24D6:LockType = LockType {
    delay: 6000 as BlockNumber, // use 6000 as the test
    lock_period: 18
};

#[test]
fn issue_and_set_lock_should_work() {
    new_test_ext().execute_with(|| {
        CrustLocks::issue_and_set_lock(&1, &200, CRU24);
        assert_eq!(Balances::locks(&1)[0].amount, 200);
        assert_eq!(Balances::locks(&1)[0].id, CRU_LOCK_ID);
        assert_eq!(Balances::free_balance(&1), 200);
        assert_eq!(Balances::total_issuance(), 200);
    });
}

#[test]
fn create_cru18_lock_should_work() {
    new_test_ext().execute_with(|| {
        assert_ok!(CrustLocks::set_unlock_from(Origin::root(), 1000));
        assert_eq!(CrustLocks::unlock_from().unwrap(), 1000);
        // Already run to 1800 blocks
        run_to_block(9000);
        // Create a new cru 18 account
        let _ = Balances::make_free_balance_be(&1, 1800);
        assert_eq!(Balances::free_balance(&1), 1800);
        assert_eq!(Balances::total_issuance(), 1800);
        CrustLocks::create_cru18_lock(&1, 1800);
        assert_eq!(Balances::locks(&1)[0].amount, 1800);
        assert_eq!(Balances::locks(&1)[0].id, CRU_LOCK_ID);

        // Test unlock
        assert_ok!(CrustLocks::unlock(Origin::signed(1)));
        assert_eq!(Balances::locks(&1)[0].amount, 1000);
        assert_eq!(Balances::locks(&1)[0].id, CRU_LOCK_ID);
    });
}

#[test]
fn set_unlock_from_should_work() {
    new_test_ext().execute_with(|| {
        run_to_block(300);
        assert_noop!(
            CrustLocks::unlock(Origin::signed(1)),
            DispatchError::Module {
                index: 2,
                error: 1,
                message: Some("NotStarted"),
            }
        );
        assert_ok!(CrustLocks::set_unlock_from(Origin::root(), 1000));
        assert_eq!(CrustLocks::unlock_from().unwrap(), 1000);

        let _ = Balances::make_free_balance_be(&1, 1800);

        run_to_block(1100);
        assert_noop!(
            CrustLocks::set_unlock_from(Origin::root(), 1000),
            DispatchError::Module {
                index: 2,
                error: 0,
                message: Some("AlreadyStarted"),
            }
        );
        assert_noop!(
            CrustLocks::unlock(Origin::signed(1)),
            DispatchError::Module {
                index: 2,
                error: 2,
                message: Some("LockNotExist"),
            }
        );

        CrustLocks::issue_and_set_lock(&1, &1800, CRU18);
        assert_eq!(Balances::locks(&1)[0].amount, 1800);
        assert_eq!(Balances::locks(&1)[0].id, CRU_LOCK_ID);

        assert_noop!(
            CrustLocks::unlock(Origin::signed(1)),
            DispatchError::Module {
                index: 2,
                error: 3,
                message: Some("TimeIsNotEnough"),
            }
        );
    });
}

#[test]
fn unlock_cru18_should_work() {
    new_test_ext().execute_with(|| {
        run_to_block(300);
        assert_ok!(CrustLocks::set_unlock_from(Origin::root(), 1000));
        assert_eq!(CrustLocks::unlock_from().unwrap(), 1000);

        let _ = Balances::make_free_balance_be(&1, 1800);

        CrustLocks::issue_and_set_lock(&1, &1800, CRU18);
        assert_eq!(Balances::locks(&1)[0].amount, 1800);
        assert_eq!(Balances::locks(&1)[0].id, CRU_LOCK_ID);

        run_to_block(1100);
        assert_noop!(
            CrustLocks::unlock(Origin::signed(1)),
            DispatchError::Module {
                index: 2,
                error: 3,
                message: Some("TimeIsNotEnough"),
            }
        );
        run_to_block(2000);

        assert_ok!(CrustLocks::unlock(Origin::signed(1)));
        assert_eq!(Balances::locks(&1)[0].amount, 1700);
        assert_eq!(Balances::locks(&1)[0].id, CRU_LOCK_ID);

        assert_noop!(
            CrustLocks::unlock(Origin::signed(1)),
            DispatchError::Module {
                index: 2,
                error: 3,
                message: Some("TimeIsNotEnough"),
            }
        );


        run_to_block(7700);
        assert_ok!(CrustLocks::unlock(Origin::signed(1)));
        assert_eq!(Balances::locks(&1)[0].amount, 1200);
        assert_eq!(Balances::locks(&1)[0].id, CRU_LOCK_ID);

        run_to_block(17700);
        assert_ok!(CrustLocks::unlock(Origin::signed(1)));
        assert_eq!(Balances::locks(&1)[0].amount, 200);
        assert_eq!(Balances::locks(&1)[0].id, CRU_LOCK_ID);

        run_to_block(19000);
        assert_ok!(CrustLocks::unlock(Origin::signed(1)));
        assert_eq!(Balances::locks(&1).len(), 0);
        assert_eq!(<Locks<Test>>::contains_key(&1), false);
        assert_noop!(
            CrustLocks::unlock(Origin::signed(1)),
            DispatchError::Module {
                index: 2,
                error: 2,
                message: Some("LockNotExist"),
            }
        );
    });
}

#[test]
fn unlock_cru24_should_work() {
    new_test_ext().execute_with(|| {
        run_to_block(300);
        assert_ok!(CrustLocks::set_unlock_from(Origin::root(), 1000));
        assert_eq!(CrustLocks::unlock_from().unwrap(), 1000);

        let _ = Balances::make_free_balance_be(&1, 2400);

        CrustLocks::issue_and_set_lock(&1, &2400, CRU24);
        assert_eq!(Balances::locks(&1)[0].amount, 2400);
        assert_eq!(Balances::locks(&1)[0].id, CRU_LOCK_ID);

        run_to_block(1100);
        assert_noop!(
            CrustLocks::unlock(Origin::signed(1)),
            DispatchError::Module {
                index: 2,
                error: 3,
                message: Some("TimeIsNotEnough"),
            }
        );
        run_to_block(2000);

        assert_ok!(CrustLocks::unlock(Origin::signed(1)));
        assert_eq!(Balances::locks(&1)[0].amount, 2300);
        assert_eq!(Balances::locks(&1)[0].id, CRU_LOCK_ID);

        assert_noop!(
            CrustLocks::unlock(Origin::signed(1)),
            DispatchError::Module {
                index: 2,
                error: 3,
                message: Some("TimeIsNotEnough"),
            }
        );

        run_to_block(7700);
        assert_ok!(CrustLocks::unlock(Origin::signed(1)));
        assert_eq!(Balances::locks(&1)[0].amount, 1800);
        assert_eq!(Balances::locks(&1)[0].id, CRU_LOCK_ID);

        run_to_block(23700);
        assert_ok!(CrustLocks::unlock(Origin::signed(1)));
        assert_eq!(Balances::locks(&1)[0].amount, 200);
        assert_eq!(Balances::locks(&1)[0].id, CRU_LOCK_ID);

        run_to_block(25000);
        assert_ok!(CrustLocks::unlock(Origin::signed(1)));
        assert_eq!(Balances::locks(&1).len(), 0);
        assert_eq!(<Locks<Test>>::contains_key(&1), false);
        assert_noop!(
            CrustLocks::unlock(Origin::signed(1)),
            DispatchError::Module {
                index: 2,
                error: 2,
                message: Some("LockNotExist"),
            }
        );
    });
}

#[test]
fn unlock_cru24d6_should_work() {
    new_test_ext().execute_with(|| {
        run_to_block(300);
        assert_ok!(CrustLocks::set_unlock_from(Origin::root(), 1000));
        assert_eq!(CrustLocks::unlock_from().unwrap(), 1000);

        let _ = Balances::make_free_balance_be(&1, 1800);

        CrustLocks::issue_and_set_lock(&1, &1800, CRU24D6);
        assert_eq!(Balances::locks(&1)[0].amount, 1800);
        assert_eq!(Balances::locks(&1)[0].id, CRU_LOCK_ID);

        run_to_block(7900);
        assert_noop!(
            CrustLocks::unlock(Origin::signed(1)),
            DispatchError::Module {
                index: 2,
                error: 3,
                message: Some("TimeIsNotEnough"),
            }
        );
        run_to_block(8000);

        assert_ok!(CrustLocks::unlock(Origin::signed(1)));
        assert_eq!(Balances::locks(&1)[0].amount, 1700);
        assert_eq!(Balances::locks(&1)[0].id, CRU_LOCK_ID);

        assert_noop!(
            CrustLocks::unlock(Origin::signed(1)),
            DispatchError::Module {
                index: 2,
                error: 3,
                message: Some("TimeIsNotEnough"),
            }
        );

        run_to_block(17321);
        assert_ok!(CrustLocks::unlock(Origin::signed(1)));
        assert_eq!(Balances::locks(&1)[0].amount, 800);
        assert_eq!(Balances::locks(&1)[0].id, CRU_LOCK_ID);

        run_to_block(23700);
        assert_ok!(CrustLocks::unlock(Origin::signed(1)));
        assert_eq!(Balances::locks(&1)[0].amount, 200);
        assert_eq!(Balances::locks(&1)[0].id, CRU_LOCK_ID);


        run_to_block(25000);
        assert_ok!(CrustLocks::unlock(Origin::signed(1)));
        assert_eq!(Balances::locks(&1).len(), 0);
        assert_eq!(<Locks<Test>>::contains_key(&1), false);
        assert_noop!(
            CrustLocks::unlock(Origin::signed(1)),
            DispatchError::Module {
                index: 2,
                error: 2,
                message: Some("LockNotExist"),
            }
        );
    });
}

#[test]
fn lock_should_be_removed_at_last() {
    new_test_ext().execute_with(|| {
        run_to_block(300);
        assert_ok!(CrustLocks::set_unlock_from(Origin::root(), 1000));
        assert_eq!(CrustLocks::unlock_from().unwrap(), 1000);

        let _ = Balances::make_free_balance_be(&1, 12364595);

        CrustLocks::issue_and_set_lock(&1, &12364596, CRU24);
        assert_eq!(Balances::locks(&1)[0].amount, 12364596);
        assert_eq!(Balances::locks(&1)[0].id, CRU_LOCK_ID);

        run_to_block(25000);
        assert_ok!(CrustLocks::unlock(Origin::signed(1)));
        assert_eq!(Balances::locks(&1).len(), 0);
        assert_eq!(<Locks<Test>>::contains_key(&1), false);
    });
}

#[test]
fn extend_lock_should_work() {
    new_test_ext().execute_with(|| {
        run_to_block(300);
        assert_ok!(CrustLocks::set_unlock_from(Origin::root(), 1000));
        assert_eq!(CrustLocks::unlock_from().unwrap(), 1000);

        let _ = Balances::make_free_balance_be(&1, 2400);

        CrustLocks::issue_and_set_lock(&1, &2400, CRU24);
        assert_eq!(Balances::locks(&1)[0].amount, 2400);
        assert_eq!(Balances::locks(&1)[0].id, CRU_LOCK_ID);

        run_to_block(2000);

        assert_ok!(CrustLocks::unlock(Origin::signed(1)));
        assert_eq!(Balances::locks(&1)[0].amount, 2300);
        assert_eq!(Balances::locks(&1)[0].id, CRU_LOCK_ID);

        run_to_block(19156);
        assert_ok!(CrustLocks::unlock(Origin::signed(1)));
        assert_eq!(Balances::locks(&1)[0].amount, 600);
        assert_eq!(Balances::locks(&1)[0].id, CRU_LOCK_ID);

        CrustLocks::issue_and_set_lock(&1, &2400, CRU24);
        assert_eq!(Balances::locks(&1)[0].amount, 4800);
        assert_eq!(Balances::locks(&1)[0].id, CRU_LOCK_ID);

        assert_ok!(CrustLocks::unlock(Origin::signed(1)));
        assert_eq!(Balances::locks(&1)[0].amount, 1200);
        assert_eq!(Balances::locks(&1)[0].id, CRU_LOCK_ID);
    });
}
// Copyright (C) 2019-2021 Crust Network Technologies Ltd.
// This file is part of Crust.

use super::*;
use crate::{mock::*, Error};
use frame_support::{assert_noop, assert_ok, dispatch::DispatchError};
use crate::{CRU18, CRU24, CRU_LOCK_ID};

pub const CRU24_WITH_DELAY:LockType = LockType {
    delay: 6000 as BlockNumber, // use 6000 as the test
    lock_period: 18
};

#[test]
fn create_new_lock_should_work() {
    new_test_ext().execute_with(|| {
        let _ = Balances::make_free_balance_be(&1, 200);
        CrustLocks::create_new_lock(&1, &200, CRU24);
        assert_eq!(Balances::locks(&1)[0].amount, 200);
        assert_eq!(Balances::locks(&1)[0].id, CRU_LOCK_ID);
    });
}

#[test]
fn set_start_date_should_work() {
    new_test_ext().execute_with(|| {
        run_to_block(300);
        assert_noop!(
            CrustLocks::unlock_one_period(Origin::signed(1)),
            DispatchError::Module {
                index: 2,
                error: 1,
                message: Some("NotStarted"),
            }
        );
        assert_ok!(CrustLocks::set_start_date(Origin::root(), 1000));
        assert_eq!(CrustLocks::start_date().unwrap(), 1000);

        let _ = Balances::make_free_balance_be(&1, 1800);

        assert_noop!(
            CrustLocks::unlock_one_period(Origin::signed(1)),
            DispatchError::Module {
                index: 2,
                error: 2,
                message: Some("LockNotExist"),
            }
        );

        CrustLocks::create_new_lock(&1, &1800, CRU18);
        assert_eq!(Balances::locks(&1)[0].amount, 1800);
        assert_eq!(Balances::locks(&1)[0].id, CRU_LOCK_ID);

        assert_noop!(
            CrustLocks::unlock_one_period(Origin::signed(1)),
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
        assert_ok!(CrustLocks::set_start_date(Origin::root(), 1000));
        assert_eq!(CrustLocks::start_date().unwrap(), 1000);

        let _ = Balances::make_free_balance_be(&1, 1800);

        CrustLocks::create_new_lock(&1, &1800, CRU18);
        assert_eq!(Balances::locks(&1)[0].amount, 1800);
        assert_eq!(Balances::locks(&1)[0].id, CRU_LOCK_ID);

        run_to_block(1100);
        assert_noop!(
            CrustLocks::unlock_one_period(Origin::signed(1)),
            DispatchError::Module {
                index: 2,
                error: 3,
                message: Some("TimeIsNotEnough"),
            }
        );
        run_to_block(2000);

        assert_ok!(CrustLocks::unlock_one_period(Origin::signed(1)));
        assert_eq!(Balances::locks(&1)[0].amount, 1700);
        assert_eq!(Balances::locks(&1)[0].id, CRU_LOCK_ID);

        assert_noop!(
            CrustLocks::unlock_one_period(Origin::signed(1)),
            DispatchError::Module {
                index: 2,
                error: 3,
                message: Some("TimeIsNotEnough"),
            }
        );

        for i in 3..19 {
            run_to_block(i*1000);
            assert_ok!(CrustLocks::unlock_one_period(Origin::signed(1)));
            assert_eq!(Balances::locks(&1)[0].amount, 1800 - (i - 1)*100);
            assert_eq!(Balances::locks(&1)[0].id, CRU_LOCK_ID);
        }

        assert_eq!(Balances::locks(&1)[0].amount, 100);
        assert_eq!(Balances::locks(&1)[0].id, CRU_LOCK_ID);

        run_to_block(19000);
        assert_ok!(CrustLocks::unlock_one_period(Origin::signed(1)));
        assert_eq!(Balances::locks(&1).len(), 0);
    });
}

#[test]
fn unlock_cru24_should_work() {
    new_test_ext().execute_with(|| {
        run_to_block(300);
        assert_ok!(CrustLocks::set_start_date(Origin::root(), 1000));
        assert_eq!(CrustLocks::start_date().unwrap(), 1000);

        let _ = Balances::make_free_balance_be(&1, 2400);

        CrustLocks::create_new_lock(&1, &2400, CRU24);
        assert_eq!(Balances::locks(&1)[0].amount, 2400);
        assert_eq!(Balances::locks(&1)[0].id, CRU_LOCK_ID);

        run_to_block(1100);
        assert_noop!(
            CrustLocks::unlock_one_period(Origin::signed(1)),
            DispatchError::Module {
                index: 2,
                error: 3,
                message: Some("TimeIsNotEnough"),
            }
        );
        run_to_block(2000);

        assert_ok!(CrustLocks::unlock_one_period(Origin::signed(1)));
        assert_eq!(Balances::locks(&1)[0].amount, 2300);
        assert_eq!(Balances::locks(&1)[0].id, CRU_LOCK_ID);

        assert_noop!(
            CrustLocks::unlock_one_period(Origin::signed(1)),
            DispatchError::Module {
                index: 2,
                error: 3,
                message: Some("TimeIsNotEnough"),
            }
        );

        for i in 3..25 {
            run_to_block(i*1000);
            assert_ok!(CrustLocks::unlock_one_period(Origin::signed(1)));
            assert_eq!(Balances::locks(&1)[0].amount, 2400 - (i - 1)*100);
            assert_eq!(Balances::locks(&1)[0].id, CRU_LOCK_ID);
        }

        assert_eq!(Balances::locks(&1)[0].amount, 100);
        assert_eq!(Balances::locks(&1)[0].id, CRU_LOCK_ID);

        run_to_block(25000);
        assert_ok!(CrustLocks::unlock_one_period(Origin::signed(1)));
        assert_eq!(Balances::locks(&1).len(), 0);
    });
}

#[test]
fn unlock_cru24_with_delay_should_work() {
    new_test_ext().execute_with(|| {
        run_to_block(300);
        assert_ok!(CrustLocks::set_start_date(Origin::root(), 1000));
        assert_eq!(CrustLocks::start_date().unwrap(), 1000);

        let _ = Balances::make_free_balance_be(&1, 1800);

        CrustLocks::create_new_lock(&1, &1800, CRU24_WITH_DELAY);
        assert_eq!(Balances::locks(&1)[0].amount, 1800);
        assert_eq!(Balances::locks(&1)[0].id, CRU_LOCK_ID);

        run_to_block(7900);
        assert_noop!(
            CrustLocks::unlock_one_period(Origin::signed(1)),
            DispatchError::Module {
                index: 2,
                error: 3,
                message: Some("TimeIsNotEnough"),
            }
        );
        run_to_block(8000);

        assert_ok!(CrustLocks::unlock_one_period(Origin::signed(1)));
        assert_eq!(Balances::locks(&1)[0].amount, 1700);
        assert_eq!(Balances::locks(&1)[0].id, CRU_LOCK_ID);

        assert_noop!(
            CrustLocks::unlock_one_period(Origin::signed(1)),
            DispatchError::Module {
                index: 2,
                error: 3,
                message: Some("TimeIsNotEnough"),
            }
        );

        for i in 9..25 {
            run_to_block(i*1000);
            assert_ok!(CrustLocks::unlock_one_period(Origin::signed(1)));
            assert_eq!(Balances::locks(&1)[0].amount, 1800 - (i - 7)*100);
            assert_eq!(Balances::locks(&1)[0].id, CRU_LOCK_ID);
        }

        assert_eq!(Balances::locks(&1)[0].amount, 100);
        assert_eq!(Balances::locks(&1)[0].id, CRU_LOCK_ID);

        run_to_block(25000);
        assert_ok!(CrustLocks::unlock_one_period(Origin::signed(1)));
        assert_eq!(Balances::locks(&1).len(), 0);
    });
}

#[test]
fn lock_should_be_removed_at_last() {
    new_test_ext().execute_with(|| {
        run_to_block(300);
        assert_ok!(CrustLocks::set_start_date(Origin::root(), 1000));
        assert_eq!(CrustLocks::start_date().unwrap(), 1000);

        let _ = Balances::make_free_balance_be(&1, 12364595);

        CrustLocks::create_new_lock(&1, &12364596, CRU24);
        assert_eq!(Balances::locks(&1)[0].amount, 12364596);
        assert_eq!(Balances::locks(&1)[0].id, CRU_LOCK_ID);

        for i in 2..25 {
            run_to_block(i*1000);
            assert_ok!(CrustLocks::unlock_one_period(Origin::signed(1)));
            assert_eq!(Balances::locks(&1)[0].id, CRU_LOCK_ID);
        }

        run_to_block(25000);
        assert_ok!(CrustLocks::unlock_one_period(Origin::signed(1)));
        assert_eq!(Balances::locks(&1).len(), 0);
    });
}
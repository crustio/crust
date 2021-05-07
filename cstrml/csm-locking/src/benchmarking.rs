// Copyright (C) 2019-2021 Crust Network Technologies Ltd.
// This file is part of Crust.

// CSM Locking pallet benchmarking.
use super::*;
use frame_system::RawOrigin;
use frame_benchmarking::{benchmarks, account};
use frame_support::traits::Currency;
use sp_std::vec;

const SEED: u32 = 0;
const ACCOUNT_INIT_BALANCE: u32 = 1_000_000_000;

use crate::Module as CSMLocking;

fn create_funded_user<T: Config>(string: &'static str, n: u32) -> T::AccountId {
    let user = account(string, n, SEED);
    let balance = T::Currency::minimum_balance() * ACCOUNT_INIT_BALANCE.into();
    T::Currency::make_free_balance_be(&user, balance);
    user
}

benchmarks! {
    bond {
        let user = create_funded_user::<T>("user", 100);
    }: _(RawOrigin::Signed(user.clone()), T::Currency::minimum_balance() * 1000u32.into())
    verify {
        assert_eq!(CSMLocking::<T>::ledger(&user).active, T::Currency::minimum_balance() * 1000u32.into());
    }

    unbond {
        let user = create_funded_user::<T>("user", 100);
        CSMLocking::<T>::bond(RawOrigin::Signed(user.clone()).into(), T::Currency::minimum_balance() * 2000u32.into()).expect("Something wrong during bonding");
    }: _(RawOrigin::Signed(user.clone()), T::Currency::minimum_balance() * 1500u32.into())
    verify {
        assert_eq!(CSMLocking::<T>::ledger(&user).active, T::Currency::minimum_balance() * 500u32.into());
    }

    rebond {
        let user = create_funded_user::<T>("user", 100);
        CSMLocking::<T>::bond(RawOrigin::Signed(user.clone()).into(), T::Currency::minimum_balance() * 2000u32.into()).expect("Something wrong during bonding");
        CSMLocking::<T>::unbond(RawOrigin::Signed(user.clone()).into(), T::Currency::minimum_balance() * 1000u32.into()).expect("Something wrong during bonding");
    }: _(RawOrigin::Signed(user.clone()), T::Currency::minimum_balance() * 500u32.into())
    verify {
        assert_eq!(CSMLocking::<T>::ledger(&user).active, T::Currency::minimum_balance() * 1500u32.into());
    }

    withdraw_unbonded {
        let user = create_funded_user::<T>("user", 100);
        CSMLocking::<T>::bond(RawOrigin::Signed(user.clone()).into(), T::Currency::minimum_balance() * 2000u32.into()).expect("Something wrong during bonding");
        frame_system::Module::<T>::set_block_number(100u32.into());
        CSMLocking::<T>::unbond(RawOrigin::Signed(user.clone()).into(), T::Currency::minimum_balance() * 1000u32.into()).expect("Something wrong during bonding");
        frame_system::Module::<T>::set_block_number(30000000u32.into());
    }: _(RawOrigin::Signed(user.clone()))
    verify {
        assert_eq!(CSMLocking::<T>::ledger(&user).total, T::Currency::minimum_balance() * 1000u32.into());
    }

}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::mock::{ExtBuilder, Test};
    use frame_support::assert_ok;

    #[test]
    fn bond() {
        ExtBuilder::default().build().execute_with(|| {
            assert_ok!(test_benchmark_bond::<Test>());
        });
    }

    #[test]
    fn unbond() {
        ExtBuilder::default().build().execute_with(|| {
            assert_ok!(test_benchmark_unbond::<Test>());
        });
    }

    #[test]
    fn rebond() {
        ExtBuilder::default().build().execute_with(|| {
            assert_ok!(test_benchmark_rebond::<Test>());
        });
    }

    #[test]
    fn withdraw_unbonded() {
        ExtBuilder::default().build().execute_with(|| {
            assert_ok!(test_benchmark_withdraw_unbonded::<Test>());
        });
    }
}

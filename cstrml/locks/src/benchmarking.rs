// Copyright (C) 2019-2021 Crust Network Technologies Ltd.
// This file is part of Crust.

// Locks pallet benchmarking.
use super::*;
use frame_system::RawOrigin;
use frame_benchmarking::{benchmarks, account};
use frame_support::traits::Currency;
use sp_std::vec;

const SEED: u32 = 0;
const ACCOUNT_INIT_BALANCE: u32 = 1_000_000_000;

use crate::Module as Locks;

fn create_funded_user<T: Config>(string: &'static str, n: u32) -> T::AccountId {
    let user = account(string, n, SEED);
    let balance = T::Currency::minimum_balance() * ACCOUNT_INIT_BALANCE.into();
    T::Currency::make_free_balance_be(&user, balance);
    user
}

benchmarks! {
    unlock {
        let user = create_funded_user::<T>("user", 100);
        frame_system::Module::<T>::set_block_number(100u32.into());
        Locks::<T>::issue_and_set_lock(&user, &(T::Currency::minimum_balance() * 1800u32.into()), CRU18);
        frame_system::Module::<T>::set_block_number(30000000u32.into());
        Locks::<T>::set_unlock_from(RawOrigin::Root.into(), 100u32.into()).expect("Something wrong during set unlock from");
    }: _(RawOrigin::Signed(user.clone()))
    verify {
        assert_eq!(Locks::<T>::locks(&user).is_none(), true);
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::mock::{new_test_ext, Test};
    use frame_support::assert_ok;

    #[test]
    fn unlock() {
        new_test_ext().execute_with(|| {
            assert_ok!(test_benchmark_unlock::<Test>());
        });
    }
}

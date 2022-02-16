// Copyright (C) 2019-2021 Crust Network Technologies Ltd.
// This file is part of Crust.

//! Balances pallet benchmarking.
use super::*;
use frame_system::RawOrigin;
use frame_benchmarking::{benchmarks, account};
use frame_support::traits::Currency;
use primitives::*;
use sp_std::vec;

const SEED: u32 = 0;
const ACCOUNT_INIT_BALANCE: u32 = 1_000_000_000;

use crate::Module as Market;

fn create_funded_user<T: Config>(string: &'static str, n: u32) -> T::AccountId {
    let user = account(string, n, SEED);
    let balance = T::Currency::minimum_balance() * ACCOUNT_INIT_BALANCE.into();
    T::Currency::make_free_balance_be(&user, balance);
    user
}

fn build_market_file<T: Config>(user: &T::AccountId, pub_key: &Vec<u8>, file_size: u64, valid_at: BlockNumber, expired_at: BlockNumber, calculated_at: BlockNumber, amount: u32)
    -> FileInfo<T::AccountId, BalanceOf<T>>
{
    let mut replicas: Vec<Replica<T::AccountId>> = vec![];
    for _ in 0..200 {
        let new_replica = Replica {
            who: user.clone(),
            valid_at,
            anchor: pub_key.clone(),
            is_reported: true,
            created_at: None
        };
        replicas.push(new_replica);
    }
    let file_info = FileInfo {
        file_size,
        spower: file_size,
        expired_at,
        calculated_at,
        amount: T::Currency::minimum_balance() * amount.into(),
        prepaid: Zero::zero(),
        reported_replica_count: 0,
        replicas
    };
    file_info
}

benchmarks! {
    place_storage_order {
        Market::<T>::set_enable_market(RawOrigin::Root.into(), true).expect("Something wrong during set market switch");
        let user = create_funded_user::<T>("user", 100);
        let cid = vec![0];
        let file_size: u64 = 10;
        let pub_key = vec![1];
        <self::Files<T>>::insert(&cid, build_market_file::<T>(&user, &pub_key, file_size, 300, 1000, 400, 1000));
        system::Module::<T>::set_block_number(600u32.into());
    }: _(RawOrigin::Signed(user.clone()), cid.clone(), file_size, T::Currency::minimum_balance() * 10u32.into(), vec![])
    verify {
        assert_eq!(Market::<T>::files(&cid).unwrap_or_default().calculated_at, 600);
    }

    calculate_reward {
        let user = create_funded_user::<T>("user", 100);
        let cid = vec![0];
        let file_size: u64 = 10;
        let pub_key = vec![1];
        <self::Files<T>>::insert(&cid, build_market_file::<T>(&user, &pub_key, file_size, 300, 1000, 400, 1000u32.into()));
        system::Module::<T>::set_block_number(2600u32.into());
        <T as crate::Config>::Currency::make_free_balance_be(&crate::Module::<T>::storage_pot(), T::Currency::minimum_balance() * 2000u32.into());
    }: _(RawOrigin::Signed(user.clone()), cid.clone())
    verify {
        assert_eq!(Market::<T>::files(&cid).is_none(), true);
    }

}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mock::{new_test_ext, Test};
    use frame_support::assert_ok;

    #[test]
    fn place_storage_order() {
        new_test_ext().execute_with(|| {
            assert_ok!(test_benchmark_place_storage_order::<Test>());
        });
    }

    #[test]
    fn calculate_reward() {
        new_test_ext().execute_with(|| {
            assert_ok!(test_benchmark_calculate_reward::<Test>());
        });
    }

}



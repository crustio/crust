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

fn build_market_file<T: Config>(user: &T::AccountId, pub_key: &Vec<u8>, file_size: u64, valid_at: BlockNumber, expired_on: BlockNumber, claimed_at: BlockNumber, amount: u32, expected_replica_count: u32)
    -> (FileInfo<T::AccountId, BalanceOf<T>>, UsedInfo)
{
    let used_info = UsedInfo {
        used_size: file_size * 2,
        reported_group_count: 1,
        groups: <BTreeMap<SworkerAnchor, bool>>::new()
    };
    let mut replicas: Vec<Replica<T::AccountId>> = vec![];
    for _ in 0..200 {
        let new_replica = Replica {
            who: user.clone(),
            valid_at,
            anchor: pub_key.clone(),
            is_reported: true
        };
        replicas.push(new_replica);
    }
    let file_info = FileInfo {
        file_size,
        expired_on,
        claimed_at,
        amount: T::Currency::minimum_balance() * amount.into(),
        expected_replica_count,
        reported_replica_count: 0,
        replicas
    };
    (file_info, used_info)
}

benchmarks! {
    _{}

    register {
        let user = create_funded_user::<T>("user", 100);
        let member_lookup: <T::Lookup as StaticLookup>::Source = T::Lookup::unlookup(user.clone());
        Market::<T>::add_member_into_allow_list(RawOrigin::Root.into(), member_lookup).expect("Give permission failed");
    }: _(RawOrigin::Signed(user.clone()), T::Currency::minimum_balance() * 10u32.into())
    verify {
        assert_eq!(Market::<T>::merchant_ledgers(&user), MerchantLedger {
            pledge: T::Currency::minimum_balance() * 10u32.into(),
            reward: 0u32.into()
        });
    }

    pledge_extra {
        let user = create_funded_user::<T>("user", 100);
        let member_lookup: <T::Lookup as StaticLookup>::Source = T::Lookup::unlookup(user.clone());
        Market::<T>::add_member_into_allow_list(RawOrigin::Root.into(), member_lookup).expect("Give permission failed");
        Market::<T>::register(RawOrigin::Signed(user.clone()).into(), T::Currency::minimum_balance() * 10u32.into()).expect("Something wrong during registering");
    }: _(RawOrigin::Signed(user.clone()), T::Currency::minimum_balance() * 10u32.into())
    verify {
        assert_eq!(Market::<T>::merchant_ledgers(&user), MerchantLedger {
            pledge: T::Currency::minimum_balance() * 20u32.into(),
            reward: 0u32.into()
        });
    }

    cut_pledge {
        let user = create_funded_user::<T>("user", 100);
        let member_lookup: <T::Lookup as StaticLookup>::Source = T::Lookup::unlookup(user.clone());
        Market::<T>::add_member_into_allow_list(RawOrigin::Root.into(), member_lookup).expect("Give permission failed");
        Market::<T>::register(RawOrigin::Signed(user.clone()).into(), T::Currency::minimum_balance() * 100u32.into()).expect("Something wrong during registering");
    }: _(RawOrigin::Signed(user.clone()), T::Currency::minimum_balance() * 10u32.into())
    verify {
        assert_eq!(Market::<T>::merchant_ledgers(&user), MerchantLedger {
            pledge: T::Currency::minimum_balance() * 90u32.into(),
            reward: 0u32.into()
        });
    }

    place_storage_order {
        let user = create_funded_user::<T>("user", 100);
        let member_lookup: <T::Lookup as StaticLookup>::Source = T::Lookup::unlookup(user.clone());
        Market::<T>::add_member_into_allow_list(RawOrigin::Root.into(), member_lookup).expect("Give permission failed");
        let cid = vec![0];
        let file_size: u64 = 10;
        let pub_key = vec![1];
        <self::Files<T>>::insert(&cid, build_market_file::<T>(&user, &pub_key, file_size, 300, 1000, 400, 1000, 4));
        system::Module::<T>::set_block_number(600u32.into());
    }: _(RawOrigin::Signed(user.clone()), cid.clone(), file_size, T::Currency::minimum_balance() * 10u32.into(), true)
    verify {
        assert_eq!(Market::<T>::files(&cid).unwrap_or_default().0.claimed_at, 600);
    }

    calculate_reward {
        let user = create_funded_user::<T>("user", 100);
        let member_lookup: <T::Lookup as StaticLookup>::Source = T::Lookup::unlookup(user.clone());
        Market::<T>::add_member_into_allow_list(RawOrigin::Root.into(), member_lookup).expect("Give permission failed");
        let cid = vec![0];
        let file_size: u64 = 10;
        let pub_key = vec![1];
        <self::Files<T>>::insert(&cid, build_market_file::<T>(&user, &pub_key, file_size, 300, 1000, 400, 1000, 4));
        system::Module::<T>::set_block_number(600u32.into());
    }: _(RawOrigin::Signed(user.clone()), cid.clone())
    verify {
        assert_eq!(Market::<T>::files(&cid).unwrap_or_default().0.claimed_at, 600);
    }

}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mock::{new_test_ext, Test};
    use frame_support::assert_ok;

    #[test]
    fn register() {
        new_test_ext().execute_with(|| {
            assert_ok!(test_benchmark_register::<Test>());
        });
    }

    #[test]
    fn pledge_extra() {
        new_test_ext().execute_with(|| {
            assert_ok!(test_benchmark_pledge_extra::<Test>());
        });
    }

    #[test]
    fn cut_pledge() {
        new_test_ext().execute_with(|| {
            assert_ok!(test_benchmark_cut_pledge::<Test>());
        });
    }

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



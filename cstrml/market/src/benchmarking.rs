//! Balances pallet benchmarking.
use super::*;

use system::{self as frame_system, RawOrigin};
use frame_benchmarking::{benchmarks, account};

use crate::Module as Market;

const SEED: u32 = 0;
const MAX_EXISTENTIAL_DEPOSIT: u32 = 1000;
const MAX_USER_INDEX: u32 = 1000;
const ACCOUNT_BALANCE_RATIO: u32 = 10000000;

fn create_funded_user<T: Trait>(string: &'static str, n: u32) -> T::AccountId {
    let user = account(string, n, SEED);
    let balance = T::Currency::minimum_balance() * ACCOUNT_BALANCE_RATIO.into();
    T::Currency::make_free_balance_be(&user, balance);
    user
}

benchmarks! {
    _ {
        let e in 2 .. MAX_EXISTENTIAL_DEPOSIT => ();
        let u in 1 .. MAX_USER_INDEX => ();
    }
    pledge {
        let u in ...;
        let stash = create_funded_user::<T>("stash",u);
        let amount = T::Currency::minimum_balance() * 10.into();
    }: _(RawOrigin::Signed(stash), amount)
    
    pledge_extra {
        let u in ...;
        let stash = create_funded_user::<T>("stash",u);
        let amount = T::Currency::minimum_balance() * 10.into();
        Market::<T>::pledge(RawOrigin::Signed(stash.clone()).into(), amount).expect("pledge failed");
    }: _(RawOrigin::Signed(stash), amount)
    
    cut_pledge {
        let u in ...;
        let stash = create_funded_user::<T>("stash",u);
        let amount = T::Currency::minimum_balance() * 10.into();
        Market::<T>::pledge(RawOrigin::Signed(stash.clone()).into(), amount).expect("pledge failed");
    }: _(RawOrigin::Signed(stash), amount)

    register {
        let u in ...;
        let stash = create_funded_user::<T>("stash",u);
        let address_info = "ws://127.0.0.1:8855".as_bytes().to_vec();
        let amount = T::Currency::minimum_balance() * 10.into();
        Market::<T>::pledge(RawOrigin::Signed(stash.clone()).into(), amount).expect("pledge failed");
    }: _(RawOrigin::Signed(stash), address_info, amount)
}


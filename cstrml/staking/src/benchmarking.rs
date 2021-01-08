// Copyright (C) 2019-2021 Crust Network Technologies Ltd.
// This file is part of Crust.

// Substrate is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Substrate is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Substrate.  If not, see <http://www.gnu.org/licenses/>.

//! Staking pallet benchmarking.

use super::*;
use rand_chacha::{rand_core::{RngCore, SeedableRng}, ChaChaRng};

// use sp_runtime::traits::One;
use sp_io::hashing::blake2_256;

use frame_system::RawOrigin;
use frame_benchmarking::{benchmarks, account};

use crate::Module as Staking;

const SEED: u32 = 0;
const ACCOUNT_BALANCE_RATIO: u32 = 10000000;
const STAKE_LIMIT_RATIO: u32 = 100000000;

fn create_funded_user<T: Config>(string: &'static str, n: u32) -> T::AccountId {
    let user = account(string, n, SEED);
    let balance = T::Currency::minimum_balance() * ACCOUNT_BALANCE_RATIO.into();
    T::Currency::make_free_balance_be(&user, balance);
    user
}

pub fn create_stash_controller<T: Config>(n: u32) -> Result<(T::AccountId, T::AccountId), &'static str> {
    let stash = create_funded_user::<T>("stash", n);
    let controller = create_funded_user::<T>("controller", n);
    let controller_lookup: <T::Lookup as StaticLookup>::Source = T::Lookup::unlookup(controller.clone());
    let reward_destination = RewardDestination::Staked;
    let amount = T::Currency::minimum_balance() * ACCOUNT_BALANCE_RATIO.into();
    Staking::<T>::bond(RawOrigin::Signed(stash.clone()).into(), controller_lookup, amount, reward_destination)?;
    return Ok((stash, controller))
}

// This function generates v validators and n guarantor who are randomly nominating up to MAX_GUARANTEE.
pub fn create_validators_with_guarantors_for_era<T: Config>(v: u32, n: u32, m: u32) -> Result<(T::AccountId, <T::Lookup as StaticLookup>::Source), &'static str> {
    let mut validators: Vec<<T::Lookup as StaticLookup>::Source> = Vec::with_capacity(v as usize);
    let mut rng = ChaChaRng::from_seed(SEED.using_encoded(blake2_256));

    // Create v validators
    let (v_stash, v_controller) = create_stash_controller::<T>(0)?;
    Staking::<T>::upsert_stake_limit(&v_stash, T::Currency::minimum_balance() * STAKE_LIMIT_RATIO.into() * STAKE_LIMIT_RATIO.into());
    Staking::<T>::validate(RawOrigin::Signed(v_controller.clone()).into(), Default::default())?;
    let stash_lookup: <T::Lookup as StaticLookup>::Source = T::Lookup::unlookup(v_stash.clone());
    validators.push(stash_lookup.clone());
    let saved_v_lookup = stash_lookup;
    for i in 1 .. v {
        let (v_stash, v_controller) = create_stash_controller::<T>(i)?;
        Staking::<T>::upsert_stake_limit(&v_stash, T::Currency::minimum_balance() * STAKE_LIMIT_RATIO.into() * STAKE_LIMIT_RATIO.into());
        Staking::<T>::validate(RawOrigin::Signed(v_controller.clone()).into(), Default::default())?;
        let stash_lookup: <T::Lookup as StaticLookup>::Source = T::Lookup::unlookup(v_stash.clone());
        validators.push(stash_lookup.clone());
    }

    // Create n guarantor
    let (_n_stash, n_controller) = create_stash_controller::<T>(u32::max_value())?;

    // Have them randomly validate
    let available_validators = validators.clone();
    for _ in 0 .. m {
        let selected = rng.next_u32() as usize % available_validators.len();
        let validator = available_validators.get(selected).unwrap();
        Staking::<T>::guarantee(RawOrigin::Signed(n_controller.clone()).into(), (validator.clone(), T::Currency::minimum_balance() * 10u32.into()))?;
    }

    let saved_n_controller = n_controller;
    for j in 1 .. n {
        let (_n_stash, n_controller) = create_stash_controller::<T>(u32::max_value() - j)?;

        // Have them randomly validate
        let available_validators = validators.clone();
        for _ in 0 .. m {
            let selected = rng.next_u32() as usize % available_validators.len();
            let validator = available_validators.get(selected).unwrap();
            Staking::<T>::guarantee(RawOrigin::Signed(n_controller.clone()).into(), (validator.clone(), T::Currency::minimum_balance() * 10u32.into()))?;
        }
    }

    ValidatorCount::put(v);

    Ok((saved_n_controller, saved_v_lookup))
}

// This function generates one validator and one guarantor
pub fn create_one_validator_with_one_guarantor<T: Config>(n: u32) -> Result<(T::AccountId, T::AccountId), &'static str> {
    let (v_stash, v_controller) = create_stash_controller::<T>(n)?;
    Staking::<T>::upsert_stake_limit(&v_stash, T::Currency::minimum_balance() * STAKE_LIMIT_RATIO.into());
    Staking::<T>::validate(RawOrigin::Signed(v_controller.clone()).into(), Default::default())?;

    let (_n_stash, n_controller) = create_stash_controller::<T>(u32::max_value() - n)?;

    return Ok((n_controller, v_stash))
}

benchmarks! {
    _{}

    bond {
        let stash = create_funded_user::<T>("stash",100);
        let controller = create_funded_user::<T>("controller", 100);
        let controller_lookup: <T::Lookup as StaticLookup>::Source = T::Lookup::unlookup(controller);
        let reward_destination = RewardDestination::Staked;
        let amount = T::Currency::minimum_balance() * 10u32.into();
    }: _(RawOrigin::Signed(stash), controller_lookup, amount, reward_destination)


    bond_extra {
        let (stash, controller) = create_stash_controller::<T>(100)?;
        Staking::<T>::upsert_stake_limit(&stash, T::Currency::minimum_balance() * STAKE_LIMIT_RATIO.into());
        Staking::<T>::validate(RawOrigin::Signed(controller.clone()).into(), Default::default())?;
        let max_additional = T::Currency::minimum_balance() * 10u32.into();
    }: _(RawOrigin::Signed(stash), max_additional)


    unbond {
        let stash = create_funded_user::<T>("stash",100);
        let controller = create_funded_user::<T>("controller", 100);
        let controller_lookup: <T::Lookup as StaticLookup>::Source = T::Lookup::unlookup(controller.clone());
        let max_additional = T::Currency::minimum_balance() * 10u32.into();
        let reward_destination = RewardDestination::Staked;
        let amount = T::Currency::minimum_balance() * 10u32.into();
        Staking::<T>::bond(RawOrigin::Signed(stash.clone()).into(), controller_lookup, amount, reward_destination)?;
    }: _(RawOrigin::Signed(controller), max_additional)


    // Withdraw only updates the ledger
    withdraw_unbonded {
        let stash = create_funded_user::<T>("stash",100);
        let controller = create_funded_user::<T>("controller", 100);
        let controller_lookup: <T::Lookup as StaticLookup>::Source = T::Lookup::unlookup(controller.clone());
        let max_additional = T::Currency::minimum_balance() * 10u32.into();
        let reward_destination = RewardDestination::Staked;
        let amount = T::Currency::minimum_balance() * 10u32.into();
        Staking::<T>::bond(RawOrigin::Signed(stash.clone()).into(), controller_lookup, amount, reward_destination)?;
        Staking::<T>::unbond(RawOrigin::Signed(controller.clone()).into(), max_additional)?;
    }: _(RawOrigin::Signed(controller.clone()))


    validate {
        let (stash, controller) = create_stash_controller::<T>(100)?;
        let prefs = Default::default();
        Staking::<T>::upsert_stake_limit(&stash, T::Currency::minimum_balance() * STAKE_LIMIT_RATIO.into());
    }: _(RawOrigin::Signed(controller), prefs)


    guarantee {
        MinimumValidatorCount::put(1);
        let (g_controller, v_lookup) = create_validators_with_guarantors_for_era::<T>(100, 1000, MAX_GUARANTEE.try_into().unwrap())?;
    }: _(RawOrigin::Signed(g_controller), (v_lookup, T::Currency::minimum_balance() * 10u32.into()))


    cut_guarantee {
        MinimumValidatorCount::put(1);
        let (g_controller, v_lookup) = create_validators_with_guarantors_for_era::<T>(100, 1000, MAX_GUARANTEE.try_into().unwrap())?;
        Staking::<T>::guarantee(RawOrigin::Signed(g_controller.clone()).into(),
        (v_lookup.clone(), T::Currency::minimum_balance() * 10u32.into()))?;
    }: _(RawOrigin::Signed(g_controller), (v_lookup, T::Currency::minimum_balance() * 10u32.into()))


    chill {
        let (_, controller) = create_stash_controller::<T>(100)?;
    }: _(RawOrigin::Signed(controller))

    set_payee {
        let (stash, controller) = create_stash_controller::<T>(100)?;
        assert_eq!(Payee::<T>::get(&stash), RewardDestination::Staked);
    }: _(RawOrigin::Signed(controller), RewardDestination::Controller)
    verify {
        assert_eq!(Payee::<T>::get(&stash), RewardDestination::Controller);
    }

    set_controller {
        let (stash, _) = create_stash_controller::<T>(100)?;
        let new_controller = create_funded_user::<T>("new_controller", 100);
        let new_controller_lookup = T::Lookup::unlookup(new_controller.clone());
    }: _(RawOrigin::Signed(stash), new_controller_lookup)
    verify {
        assert!(Ledger::<T>::contains_key(&new_controller));
    }


    new_era {
        let v in 1 .. 2;
        let n in 1 .. 2;
        let m in 1 .. MAX_GUARANTEE.try_into().unwrap();
        MinimumValidatorCount::put(1);
        create_validators_with_guarantors_for_era::<T>(10u32.pow(v), 10u32.pow(n), m)?;
        let session_index = SessionIndex::one();
    }: {
        let validators = Staking::<T>::new_era(session_index).ok_or("`new_era` failed")?;
    }

    select_and_update_validators {
        let v in 1 .. 2;
        let n in 1 .. 2;
        let m in 1 .. MAX_GUARANTEE.try_into().unwrap();
        MinimumValidatorCount::put(1);
        create_validators_with_guarantors_for_era::<T>(10u32.pow(v), 10u32.pow(n), m)?;
        let session_index = SessionIndex::one();
    }: {
        Staking::<T>::select_and_update_validators(0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mock::{ExtBuilder, Test};
    use frame_support::assert_ok;

    #[test]
    fn create_validators_with_guarantors_for_era_works() {
        ExtBuilder::default().build().execute_with(|| {
            let v = 10;
            let n = 10;
            let m = 10;

            create_validators_with_guarantors_for_era::<Test>(v,n,m).unwrap();

            let count_validators = Validators::<Test>::iter().count();
            let count_guarantor = Guarantors::<Test>::iter().count();

            // 3 extra validators and 1 extra guarantor in mock Test
            assert_eq!(count_validators, (v + 3) as usize);
            assert_eq!(count_guarantor, (n + 1) as usize);
        });
    }

    #[test]
    fn test_benchmarks() {
        ExtBuilder::default().build().execute_with(|| {
            assert_ok!(test_benchmark_bond::<Test>());
            assert_ok!(test_benchmark_bond_extra::<Test>());
            assert_ok!(test_benchmark_unbond::<Test>());
            assert_ok!(test_benchmark_validate::<Test>());
            assert_ok!(test_benchmark_guarantee::<Test>());
            assert_ok!(test_benchmark_cut_guarantee::<Test>());
            assert_ok!(test_benchmark_new_era::<Test>());
            assert_ok!(test_benchmark_select_and_update_validators::<Test>());
            assert_ok!(test_benchmark_chill::<Test>());
            assert_ok!(test_benchmark_set_payee::<Test>());
            assert_ok!(test_benchmark_set_controller::<Test>());
            assert_ok!(test_benchmark_withdraw_unbonded::<Test>());
        });
    }
}

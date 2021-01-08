// Copyright (C) 2019-2021 Crust Network Technologies Ltd.
// This file is part of Crust.

// Ensure we're `no_std` when compiling for Wasm.
#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::{Parameter, decl_module, decl_event, decl_storage, decl_error, ensure, traits::Vec};
use sp_runtime::traits::{Member, AtLeast32BitUnsigned, Zero, StaticLookup};
use frame_system::{ensure_signed, ensure_root};

/// The module configuration trait.
pub trait Config: frame_system::Config {
    /// The overarching event type.
    type Event: From<Event<Self>> + Into<<Self as frame_system::Config>::Event>;

    /// The units in which we record balances.
    type Balance: Member + Parameter + AtLeast32BitUnsigned + Default + Copy;
}

// ETH Address
type ETHAddress = Vec<u8>;

decl_event! {
	pub enum Event<T> where
		<T as frame_system::Config>::AccountId,
		<T as Config>::Balance,
	{
		/// Some assets were issued. \[owner, total_supply\]
		CandyIssued(AccountId, Balance),
		/// Some assets were transferred. \[from, to, amount\]
		CandyTransferred(AccountId, AccountId, Balance),
		/// Some assets were burned. \[from, balance\]
		CandyBurned(AccountId, Balance),
		/// ETH address was bonded
		BondEthSuccess(AccountId, ETHAddress),
	}
}

decl_error! {
	pub enum Error for Module<T: Config> {
		/// Transfer amount should be non-zero
		AmountZero,
		/// Account balance must be greater than or equal to the transfer amount
		BalanceLow,
		/// Balance should be non-zero
		BalanceZero,
	}
}

decl_storage! {
	trait Store for Module<T: Config> as Assets {
		/// The number of units of candy held by any given account.
		Balances get(fn balances): map hasher(blake2_128_concat) T::AccountId => T::Balance;
		/// The total unit supply of candy.
		Total get(fn total): T::Balance;
		/// The bonded eth address to any given account.
		BondedEth get(fn bonded_eth): map hasher(blake2_128_concat) T::AccountId => Option<ETHAddress>;
	}
}

decl_module! {
	pub struct Module<T: Config> for enum Call where origin: T::Origin {
		type Error = Error<T>;

		fn deposit_event() = default;

		/// Issue crust candy. There are, and will only ever be, `total`
		/// such candy and they'll all belong to the `root` initially.
		///
		/// # <weight>
		/// - `O(1)`
		/// - 2 storage mutate (condec `O(1)`).
		/// - 1 event.
		/// # </weight>
		#[weight = 0]
		fn issue(origin,
		    target: <T::Lookup as StaticLookup>::Source,
		    #[compact] total: T::Balance) {
			ensure_root(origin)?;
			let target = T::Lookup::lookup(target)?;

			<Balances<T>>::mutate(target.clone(), |root_total| *root_total += total);
			<Total<T>>::mutate(|total_supply| *total_supply += total);

			Self::deposit_event(RawEvent::CandyIssued(target, total));
		}

		/// Move candy from one holder to another.
		///
		/// # <weight>
		/// - `O(1)`
		/// - 1 static lookup
		/// - 2 storage mutations (codec `O(1)`).
		/// - 1 event.
		/// # </weight>
		#[weight = 1_000_000]
		fn transfer(origin,
			target: <T::Lookup as StaticLookup>::Source,
			#[compact] amount: T::Balance
		) {
			let from = ensure_signed(origin)?;
			let from_balances = Self::balances(&from);
			let to = T::Lookup::lookup(target)?;
			ensure!(!amount.is_zero(), Error::<T>::AmountZero);
			ensure!(from_balances >= amount, Error::<T>::BalanceLow);

            Self::deposit_event(RawEvent::CandyTransferred(from.clone(), to.clone(), amount));

			<Balances<T>>::insert(from, from_balances - amount);
			<Balances<T>>::mutate(to, |balance| *balance += amount);
		}

		/// Destroy candy from `target` account. Only been called by `root`
		///
		/// # <weight>
		/// - `O(1)`
		/// - 2 storage mutation (codec `O(1)`).
		/// - 1 event.
		/// # </weight>
		#[weight = 0]
		fn burn(origin,
		    target: <T::Lookup as StaticLookup>::Source,
		    #[compact] amount: T::Balance) {
			ensure_root(origin)?;
			let target = T::Lookup::lookup(target)?;
			let remains = Self::balances(&target);
			let burned_balances = remains.min(amount);

			ensure!(!burned_balances.is_zero(), Error::<T>::BalanceZero);

            Self::deposit_event(RawEvent::CandyBurned(target.clone(), burned_balances));

			<Total<T>>::mutate(|total_supply| *total_supply -= burned_balances);
			<Balances<T>>::insert(target, remains - burned_balances);
		}

		/// Register one ETH address for an given account
		///
		/// # <weight>
		/// - `O(1)`
		/// - 1 storage mutations (codec `O(1)`).
		/// - 1 event.
		/// # </weight>
		#[weight = 1_000_000]
		fn bond_eth(origin, address: ETHAddress) {
			let who = ensure_signed(origin)?;

			<BondedEth<T>>::insert(&who, &address);

			Self::deposit_event(RawEvent::BondEthSuccess(who, address));
		}
	}
}

#[cfg(test)]
mod tests {
    use super::*;

    use frame_support::{impl_outer_origin, assert_ok, assert_noop, parameter_types};
    use sp_core::H256;
    use sp_runtime::{traits::{BlakeTwo256, IdentityLookup}, testing::Header};

    impl_outer_origin! {
		pub enum Origin for Test where system = frame_system {}
	}

    #[derive(Clone, Eq, PartialEq)]
    pub struct Test;
    parameter_types! {
		pub const BlockHashCount: u64 = 250;
	}
    impl frame_system::Config for Test {
        type BaseCallFilter = ();
        type BlockWeights = ();
        type BlockLength = ();
        type Origin = Origin;
        type Call = ();
        type Index = u64;
        type BlockNumber = u64;
        type Hash = H256;
        type Hashing = BlakeTwo256;
        type AccountId = u64;
        type Lookup = IdentityLookup<Self::AccountId>;
        type Header = Header;
        type Event = ();
        type BlockHashCount = BlockHashCount;
        type DbWeight = ();
        type Version = ();
        type PalletInfo = ();
        type AccountData = ();
        type OnNewAccount = ();
        type OnKilledAccount = ();
        type SystemWeightInfo = ();
    }
    impl Config for Test {
        type Event = ();
        type Balance = u64;
    }
    type Candy = Module<Test>;

    fn new_test_ext() -> sp_io::TestExternalities {
        frame_system::GenesisConfig::default().build_storage::<Test>().unwrap().into()
    }

    #[test]
    fn issuing_asset_units_to_issuer_should_work() {
        new_test_ext().execute_with(|| {
            assert_ok!(Candy::issue(Origin::root(), 1, 100));
            assert_eq!(Candy::balances(1), 100);
            assert_eq!(Candy::total(), 100);

            // Issue again should work
            assert_ok!(Candy::issue(Origin::root(), 2, 100));
            assert_eq!(Candy::balances(2), 100);
            assert_eq!(Candy::total(), 200);
        });
    }

    #[test]
    fn querying_total_supply_should_work() {
        new_test_ext().execute_with(|| {
            assert_ok!(Candy::issue(Origin::root(), 1, 100));
            assert_eq!(Candy::balances(1), 100);
            assert_ok!(Candy::transfer(Origin::signed(1), 2, 50));
            assert_eq!(Candy::balances(1), 50);
            assert_eq!(Candy::balances(2), 50);
            assert_ok!(Candy::transfer(Origin::signed(2), 3, 31));
            assert_eq!(Candy::balances(1), 50);
            assert_eq!(Candy::balances(2), 19);
            assert_eq!(Candy::balances(3), 31);
            assert_ok!(Candy::burn(Origin::root(), 3, 31));
            assert_eq!(Candy::total(), 69);
        });
    }

    #[test]
    fn transferring_amount_above_available_balance_should_work() {
        new_test_ext().execute_with(|| {
            assert_ok!(Candy::issue(Origin::root(), 1, 100));
            assert_eq!(Candy::balances(1), 100);
            assert_ok!(Candy::transfer(Origin::signed(1), 2, 50));
            assert_eq!(Candy::balances(1), 50);
            assert_eq!(Candy::balances(2), 50);
        });
    }

    #[test]
    fn transferring_amount_more_than_available_balance_should_not_work() {
        new_test_ext().execute_with(|| {
            assert_ok!(Candy::issue(Origin::root(), 1, 100));
            assert_eq!(Candy::balances(1), 100);
            assert_ok!(Candy::transfer(Origin::signed(1), 2, 50));
            assert_eq!(Candy::balances(1), 50);
            assert_eq!(Candy::balances(2), 50);
            assert_ok!(Candy::burn(Origin::root(), 1, 50));
            assert_eq!(Candy::balances(1), 0);
            assert_noop!(Candy::transfer(Origin::signed(1), 1, 50), Error::<Test>::BalanceLow);
        });
    }

    #[test]
    fn transferring_less_than_one_unit_should_not_work() {
        new_test_ext().execute_with(|| {
            assert_ok!(Candy::issue(Origin::root(), 1, 100));
            assert_eq!(Candy::balances(1), 100);
            assert_noop!(Candy::transfer(Origin::signed(1), 2, 0), Error::<Test>::AmountZero);
        });
    }

    #[test]
    fn transferring_more_units_than_total_supply_should_not_work() {
        new_test_ext().execute_with(|| {
            assert_ok!(Candy::issue(Origin::root(), 1, 100));
            assert_eq!(Candy::balances(1), 100);
            assert_noop!(Candy::transfer(Origin::signed(1), 2, 101), Error::<Test>::BalanceLow);
        });
    }

    #[test]
    fn burning_asset_balance_with_positive_balance_should_work() {
        new_test_ext().execute_with(|| {
            assert_ok!(Candy::issue(Origin::root(), 1, 100));
            assert_eq!(Candy::balances(1), 100);
            assert_ok!(Candy::burn(Origin::root(), 1, 100));
        });
    }

    #[test]
    fn burning_asset_balance_with_zero_balance_should_not_work() {
        new_test_ext().execute_with(|| {
            assert_ok!(Candy::issue(Origin::root(), 1, 100));
            assert_eq!(Candy::balances(2), 0);
            assert_noop!(Candy::burn(Origin::root(), 2, 0), Error::<Test>::BalanceZero);
        });
    }

    #[test]
    fn bond_eth_should_work() {
        new_test_ext().execute_with(|| {
            assert_ok!(Candy::bond_eth(Origin::signed(1), hex::decode("0000000000000000000000000000000000000000").unwrap()));
        });
    }
}
// Copyright (C) 2019-2021 Crust Network Technologies Ltd.
// This file is part of Crust.

// Ensure we're `no_std` when compiling for Wasm.
#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::{
    dispatch::DispatchResult,
    traits::{
        Get, ExistenceRequirement::KeepAlive, Currency
    },
    Parameter, decl_module, decl_event, decl_storage, decl_error, ensure,
};
use sp_runtime::{
    ModuleId,
    traits::{CheckedDiv, Convert, Member, AtLeast32BitUnsigned, Zero, StaticLookup, AccountIdConversion},
    transaction_validity::{
        TransactionLongevity, TransactionValidity, ValidTransaction, InvalidTransaction, TransactionSource,
    }
};
use codec::{Encode};
use frame_system::{ensure_signed, ensure_root, ensure_none};
use primitives::{
    traits::UsableCurrency
};
use sp_std::prelude::*;

type BalanceOf<T> =
<<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

/// The module configuration trait.
pub trait Config: frame_system::Config {
    /// The overarching event type.
    type Event: From<Event<Self>> + Into<<Self as frame_system::Config>::Event>;

    /// The units in which we record balances.
    type Balance: Member + Parameter + AtLeast32BitUnsigned + Default + Copy;

    /// The candy's module id, used for staking pot
    type ModuleId: Get<ModuleId>;

    /// The candy balance.
    type Currency: UsableCurrency<Self::AccountId, Moment = Self::BlockNumber>;

    type MinimalCandy: Get<Self::Balance>;

    type ExchangeRatio: Get<Self::Balance>;

    type CurrencyToVote: Convert<Self::Balance, BalanceOf<Self>>;
}

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
		/// No Candy
		NoCandy,
		/// Candy is not enough
		NotEnoughCandy,
	}
}

decl_storage! {
	trait Store for Module<T: Config> as Assets {
		/// The number of units of candy held by any given account.
		Balances get(fn balances): map hasher(blake2_128_concat) T::AccountId => T::Balance;
		/// The total unit supply of candy.
		Total get(fn total): T::Balance;
	}
}

decl_module! {
	pub struct Module<T: Config> for enum Call where origin: T::Origin {
		type Error = Error<T>;

		const CandyPot: T::AccountId = T::ModuleId::get().into_sub_account("cndy");

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

		/// Exchange candy
        /// Unsigned transaction with tx pool validation
        #[weight = 0]
        fn exchange_candy(origin, dest: T::AccountId) -> DispatchResult {
            let _ = ensure_none(origin)?;

            // 1. Check the dest have candy again
            ensure!(<Balances<T>>::contains_key(&dest), Error::<T>::NoCandy);

            // 2. Check he has enough candy
            let candy = Self::balances(&dest);
            ensure!(candy >= T::MinimalCandy::get(), Error::<T>::NotEnoughCandy);

            // 3. Transfer CRU
            let cru = candy.checked_div(&T::ExchangeRatio::get()).unwrap();
            let to_balance = |e: T::Balance| <T::CurrencyToVote as Convert<T::Balance, BalanceOf<T>>>::convert(e);
            T::Currency::transfer(&Self::candy_pot(), &dest, to_balance(cru), KeepAlive)?;

            // 4. Remove the record
            <Total<T>>::mutate(|total_supply| *total_supply -= candy);
			<Balances<T>>::remove(dest);

			Ok(())
        }
	}
}

impl<T: Config> Module<T> {
    /// Staking pot for authoring reward and staking reward
    pub fn candy_pot() -> T::AccountId {
        // "modl" ++ "candying" ++ "cndy" is 16 bytes
        T::ModuleId::get().into_sub_account("cndy")
    }
}

impl<T: Config> sp_runtime::traits::ValidateUnsigned for Module<T> {
    type Call = Call<T>;

    fn validate_unsigned(_source: TransactionSource, call: &Self::Call) -> TransactionValidity {
        const PRIORITY: u64 = 100;

        let account = match call {
            Call::exchange_candy(account) => {
                // 1. Check the dest have candy again
                let e = InvalidTransaction::Custom(0u8.into());
                ensure!(<Balances<T>>::contains_key(account), e);
                let candy = Self::balances(account);
                // 2. Check he has enough candy
                let e = InvalidTransaction::Custom(1u8.into());
                ensure!(candy >= T::MinimalCandy::get(), e);
                account
            }
            _ => return Err(InvalidTransaction::Call.into()),
        };

        Ok(ValidTransaction {
            priority: PRIORITY,
            requires: vec![],
            provides: vec![("exchange_candy", account).encode()],
            longevity: TransactionLongevity::max_value(),
            propagate: true,
        })
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    use frame_support::{assert_ok, assert_noop, parameter_types};
    use sp_core::H256;
    use sp_runtime::{traits::{BlakeTwo256, IdentityLookup}, testing::Header};
    use crate as candy;

    pub struct CurrencyToVoteHandler;
    impl Convert<u64, u64> for CurrencyToVoteHandler {
        fn convert(x: u64) -> u64 {
            x
        }
    }

    parameter_types! {
		pub const BlockHashCount: u64 = 250;
        pub const CandyModuleId: ModuleId = ModuleId(*b"candying");
        pub const MinimalCandy: u64 = 2000;
        pub const ExchangeRatio: u64 = 1000;
        pub const ExistentialDeposit: u64 = 1;
	}

    impl frame_system::Config for Test {
        type BaseCallFilter = ();
        type BlockWeights = ();
        type BlockLength = ();
        type Origin = Origin;
        type Call = Call;
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
        type PalletInfo = PalletInfo;
        type AccountData = balances::AccountData<u64>;
        type OnNewAccount = ();
        type OnKilledAccount = ();
        type SystemWeightInfo = ();
        type SS58Prefix = ();
    }

    impl balances::Config for Test {
        type Balance = u64;
        type DustRemoval = ();
        type Event = ();
        type ExistentialDeposit = ExistentialDeposit;
        type AccountStore = System;
        type WeightInfo = ();
        type MaxLocks = ();
    }

    impl Config for Test {
        type Event = ();
        type Balance = u64;
        type ModuleId = CandyModuleId;
        type Currency = Balances;
        type MinimalCandy = MinimalCandy;
        type ExchangeRatio = ExchangeRatio;
        type CurrencyToVote = CurrencyToVoteHandler;
    }
    type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
    type Block = frame_system::mocking::MockBlock<Test>;

    frame_support::construct_runtime!(
	pub enum Test where
		Block = Block,
		NodeBlock = Block,
		UncheckedExtrinsic = UncheckedExtrinsic
	{
		System: frame_system::{Module, Call, Config, Storage, Event<T>},
		Balances: balances::{Module, Call, Storage, Config<T>, Event<T>},
		Candy: candy::{Module, Call, Storage, Event<T>, ValidateUnsigned},
	}
);

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
    fn exchange_candy_should_work() {
        new_test_ext().execute_with(|| {
            let _ = Balances::make_free_balance_be(&Candy::candy_pot(), 100);

            assert_ok!(Candy::issue(Origin::root(), 1, 10000));
            assert_ok!(Candy::issue(Origin::root(), 2, 2000));
            assert_ok!(Candy::issue(Origin::root(), 3, 1000));
            assert_ok!(Candy::exchange_candy(Origin::none(), 1));
            assert_eq!(Balances::free_balance(1), 10);
            assert_eq!(Candy::balances(1), 0);
            assert_eq!(Candy::total(), 3000);
            assert_eq!(Balances::free_balance(&Candy::candy_pot()), 90);

            assert_ok!(Candy::exchange_candy(Origin::none(), 2));
            assert_eq!(Balances::free_balance(2), 2);
            assert_eq!(Candy::balances(2), 0);
            assert_eq!(Candy::total(), 1000);
            assert_eq!(Balances::free_balance(&Candy::candy_pot()), 88);

            assert_noop!(
                Candy::exchange_candy(Origin::none(), 3),
                Error::<Test>::NotEnoughCandy
            );
            assert_eq!(Candy::balances(3), 1000);
            assert_eq!(Candy::total(), 1000);
            assert_eq!(Balances::free_balance(&Candy::candy_pot()), 88);

            assert_noop!(
                Candy::exchange_candy(Origin::none(), 4),
                Error::<Test>::NoCandy
            );
        });
    }
}
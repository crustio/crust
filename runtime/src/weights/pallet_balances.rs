// Copyright (C) 2019-2021 Crust Network Technologies Ltd.
// This file is part of Crust.

#![allow(unused_parens)]
#![allow(unused_imports)]

use frame_support::{traits::Get, weights::Weight};
use sp_std::marker::PhantomData;

/// Weight functions for pallet_balances.
pub struct WeightInfo<T>(PhantomData<T>);
impl<T: frame_system::Config> balances::WeightInfo for WeightInfo<T> {
	fn transfer() -> Weight {
		(93_087_000 as Weight)
			.saturating_add(T::DbWeight::get().reads(1 as Weight))
			.saturating_add(T::DbWeight::get().writes(1 as Weight))
	}
	fn transfer_keep_alive() -> Weight {
		(63_971_000 as Weight)
			.saturating_add(T::DbWeight::get().reads(1 as Weight))
			.saturating_add(T::DbWeight::get().writes(1 as Weight))
	}
	fn set_balance_creating() -> Weight {
		(35_018_000 as Weight)
			.saturating_add(T::DbWeight::get().reads(1 as Weight))
			.saturating_add(T::DbWeight::get().writes(1 as Weight))
	}
	fn set_balance_killing() -> Weight {
		(44_144_000 as Weight)
			.saturating_add(T::DbWeight::get().reads(1 as Weight))
			.saturating_add(T::DbWeight::get().writes(1 as Weight))
	}
	fn force_transfer() -> Weight {
		(91_707_000 as Weight)
			.saturating_add(T::DbWeight::get().reads(2 as Weight))
			.saturating_add(T::DbWeight::get().writes(2 as Weight))
	}
}
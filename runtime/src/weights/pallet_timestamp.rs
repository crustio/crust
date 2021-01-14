// Copyright (C) 2019-2021 Crust Network Technologies Ltd.
// This file is part of Crust.

#![allow(unused_parens)]
#![allow(unused_imports)]

use frame_support::{traits::Get, weights::Weight};
use sp_std::marker::PhantomData;

/// Weight functions for pallet_timestamp.
pub struct WeightInfo<T>(PhantomData<T>);
impl<T: frame_system::Config> pallet_timestamp::WeightInfo for WeightInfo<T> {
	fn set() -> Weight {
		(11_338_000 as Weight)
			.saturating_add(T::DbWeight::get().reads(2 as Weight))
			.saturating_add(T::DbWeight::get().writes(1 as Weight))
	}
	fn on_finalize() -> Weight {
		(6_080_000 as Weight)
	}
}

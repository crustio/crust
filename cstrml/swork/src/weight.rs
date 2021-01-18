// Copyright (C) 2019-2021 Crust Network Technologies Ltd.
// This file is part of Crust.

//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 2.0.0

#![allow(unused_parens)]
#![allow(unused_imports)]

use frame_support::weights::{Weight, constants::RocksDbWeight as DbWeight};

pub struct WeightInfo;
impl crate::WeightInfo for WeightInfo {
	fn upgrade() -> Weight {
		(3_000_000 as Weight)
			.saturating_add(DbWeight::get().writes(2 as Weight))
	}
	fn register() -> Weight {
		(555_000_000 as Weight)
			.saturating_add(DbWeight::get().reads(6 as Weight))
			.saturating_add(DbWeight::get().writes(4 as Weight))
	}
	fn report_works() -> Weight {
		(495_000_000 as Weight)
			.saturating_add(DbWeight::get().reads(12 as Weight))
			.saturating_add(DbWeight::get().writes(6 as Weight))
	}
	fn chill_pk() -> Weight {
		(41_000_000 as Weight)
			.saturating_add(DbWeight::get().reads(6 as Weight))
			.saturating_add(DbWeight::get().writes(5 as Weight))
	}
}

// Copyright (C) 2019-2021 Crust Network Technologies Ltd.
// This file is part of Crust.

//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 2.0.0

#![allow(unused_parens)]
#![allow(unused_imports)]

use frame_support::weights::{Weight, constants::RocksDbWeight as DbWeight};

pub struct WeightInfo;
impl crate::WeightInfo for WeightInfo {
	fn bond() -> Weight {
		(67_000_000 as Weight)
			.saturating_add(DbWeight::get().reads(10 as Weight))
			.saturating_add(DbWeight::get().writes(7 as Weight))
	}
	fn bond_extra() -> Weight {
		(31_000_000 as Weight)
			.saturating_add(DbWeight::get().reads(4 as Weight))
	}
	fn unbond() -> Weight {
		(52_000_000 as Weight)
			.saturating_add(DbWeight::get().reads(8 as Weight))
			.saturating_add(DbWeight::get().writes(5 as Weight))
	}
	fn rebond(l: u32, ) -> Weight {
		(37_039_000 as Weight)
			// Standard Error: 2_000
			.saturating_add((93_000 as Weight).saturating_mul(l as Weight))
			.saturating_add(DbWeight::get().reads(3 as Weight))
			.saturating_add(DbWeight::get().writes(3 as Weight))
	}
	fn withdraw_unbonded() -> Weight {
		(34_000_000 as Weight)
			.saturating_add(DbWeight::get().reads(4 as Weight))
			.saturating_add(DbWeight::get().writes(3 as Weight))
	}
	fn validate() -> Weight {
		(12_000_000 as Weight)
			.saturating_add(DbWeight::get().reads(1 as Weight))
			.saturating_add(DbWeight::get().writes(2 as Weight))
	}
	fn guarantee() -> Weight {
		(79_000_000 as Weight)
			.saturating_add(DbWeight::get().reads(4 as Weight))
			.saturating_add(DbWeight::get().writes(2 as Weight))
	}
	fn cut_guarantee() -> Weight {
		(57_000_000 as Weight)
			.saturating_add(DbWeight::get().reads(2 as Weight))
			.saturating_add(DbWeight::get().writes(1 as Weight))
	}
	fn chill() -> Weight {
		(12_000_000 as Weight)
			.saturating_add(DbWeight::get().reads(1 as Weight))
			.saturating_add(DbWeight::get().writes(2 as Weight))
	}
	fn set_payee() -> Weight {
		(11_000_000 as Weight)
			.saturating_add(DbWeight::get().reads(1 as Weight))
			.saturating_add(DbWeight::get().writes(1 as Weight))
	}
	fn set_controller() -> Weight {
		(26_000_000 as Weight)
			.saturating_add(DbWeight::get().reads(3 as Weight))
			.saturating_add(DbWeight::get().writes(3 as Weight))
	}
	fn new_era(v: u32, n: u32, m: u32, ) -> Weight {
		(0 as Weight)
			.saturating_add((6_597_064_000 as Weight).saturating_mul(v as Weight))
			.saturating_add((3_025_897_000 as Weight).saturating_mul(n as Weight))
			.saturating_add((123_334_000 as Weight).saturating_mul(m as Weight))
			.saturating_add(DbWeight::get().reads((454 as Weight).saturating_mul(v as Weight)))
			.saturating_add(DbWeight::get().reads((118 as Weight).saturating_mul(n as Weight)))
			.saturating_add(DbWeight::get().reads((1 as Weight).saturating_mul(m as Weight)))
			.saturating_add(DbWeight::get().writes((270 as Weight).saturating_mul(v as Weight)))
	}
	fn select_and_update_validators(v: u32, n: u32, m: u32, ) -> Weight {
		(0 as Weight)
			.saturating_add((6_598_237_000 as Weight).saturating_mul(v as Weight))
			.saturating_add((3_391_570_000 as Weight).saturating_mul(n as Weight))
			.saturating_add((124_417_000 as Weight).saturating_mul(m as Weight))
			.saturating_add(DbWeight::get().reads((454 as Weight).saturating_mul(v as Weight)))
			.saturating_add(DbWeight::get().reads((118 as Weight).saturating_mul(n as Weight)))
			.saturating_add(DbWeight::get().reads((1 as Weight).saturating_mul(m as Weight)))
			.saturating_add(DbWeight::get().writes((270 as Weight).saturating_mul(v as Weight)))
	}
	fn recharge_staking_pot() -> Weight {
		(64_828_000 as Weight)
			.saturating_add(DbWeight::get().reads(1 as Weight))
			.saturating_add(DbWeight::get().writes(1 as Weight))

	}
}

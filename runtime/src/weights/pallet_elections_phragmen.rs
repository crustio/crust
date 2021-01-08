// Copyright (C) 2019-2021 Crust Network Technologies Ltd.
// This file is part of Crust.

#![allow(unused_parens)]
#![allow(unused_imports)]

use frame_support::{traits::Get, weights::Weight};
use sp_std::marker::PhantomData;

/// Weight functions for pallet_elections_phragmen.
pub struct WeightInfo<T>(PhantomData<T>);
impl<T: frame_system::Config> pallet_elections_phragmen::WeightInfo for WeightInfo<T> {
	fn vote(v: u32, ) -> Weight {
		(86_473_000 as Weight)
			// Standard Error: 9_000
			.saturating_add((199_000 as Weight).saturating_mul(v as Weight))
			.saturating_add(T::DbWeight::get().reads(5 as Weight))
			.saturating_add(T::DbWeight::get().writes(2 as Weight))
	}
	fn vote_update(v: u32, ) -> Weight {
		(53_531_000 as Weight)
			// Standard Error: 8_000
			.saturating_add((126_000 as Weight).saturating_mul(v as Weight))
			.saturating_add(T::DbWeight::get().reads(5 as Weight))
			.saturating_add(T::DbWeight::get().writes(2 as Weight))
	}
	fn remove_voter() -> Weight {
		(69_725_000 as Weight)
			.saturating_add(T::DbWeight::get().reads(2 as Weight))
			.saturating_add(T::DbWeight::get().writes(2 as Weight))
	}
	fn report_defunct_voter_correct(c: u32, v: u32, ) -> Weight {
		(0 as Weight)
			// Standard Error: 2_000
			.saturating_add((1_673_000 as Weight).saturating_mul(c as Weight))
			// Standard Error: 53_000
			.saturating_add((33_921_000 as Weight).saturating_mul(v as Weight))
			.saturating_add(T::DbWeight::get().reads(7 as Weight))
			.saturating_add(T::DbWeight::get().writes(3 as Weight))
	}
	fn report_defunct_voter_incorrect(c: u32, v: u32, ) -> Weight {
		(0 as Weight)
			// Standard Error: 0
			.saturating_add((1_696_000 as Weight).saturating_mul(c as Weight))
			// Standard Error: 12_000
			.saturating_add((33_906_000 as Weight).saturating_mul(v as Weight))
			.saturating_add(T::DbWeight::get().reads(6 as Weight))
			.saturating_add(T::DbWeight::get().writes(2 as Weight))
	}
	fn submit_candidacy(c: u32, ) -> Weight {
		(70_603_000 as Weight)
			// Standard Error: 0
			.saturating_add((276_000 as Weight).saturating_mul(c as Weight))
			.saturating_add(T::DbWeight::get().reads(3 as Weight))
			.saturating_add(T::DbWeight::get().writes(1 as Weight))
	}
	fn renounce_candidacy_candidate(c: u32, ) -> Weight {
		(42_985_000 as Weight)
			// Standard Error: 0
			.saturating_add((140_000 as Weight).saturating_mul(c as Weight))
			.saturating_add(T::DbWeight::get().reads(1 as Weight))
			.saturating_add(T::DbWeight::get().writes(1 as Weight))
	}
	fn renounce_candidacy_members() -> Weight {
		(76_320_000 as Weight)
			.saturating_add(T::DbWeight::get().reads(3 as Weight))
			.saturating_add(T::DbWeight::get().writes(4 as Weight))
	}
	fn renounce_candidacy_runners_up() -> Weight {
		(46_198_000 as Weight)
			.saturating_add(T::DbWeight::get().reads(1 as Weight))
			.saturating_add(T::DbWeight::get().writes(1 as Weight))
	}
	fn remove_member_with_replacement() -> Weight {
		(115_357_000 as Weight)
			.saturating_add(T::DbWeight::get().reads(4 as Weight))
			.saturating_add(T::DbWeight::get().writes(5 as Weight))
	}
	fn remove_member_wrong_refund() -> Weight {
		(8_869_000 as Weight)
			.saturating_add(T::DbWeight::get().reads(1 as Weight))
	}
}

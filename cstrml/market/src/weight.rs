
//! Autogenerated weights for market
//!
//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 2.0.0
//! DATE: 2021-01-25, STEPS: [], REPEAT: 20, LOW RANGE: [], HIGH RANGE: []
//! EXECUTION: Some(Wasm), WASM-EXECUTION: Compiled, CHAIN: None, DB CACHE: 128

// Executed Command:
// ../target/release/crust
// benchmark
// --execution
// wasm
// --pallet
// market
// --extrinsic
// *
// --repeat
// 20
// --wasm-execution
// compiled
// --output
// ../market-weight


#![allow(unused_parens)]
#![allow(unused_imports)]

use frame_support::{traits::Get, weights::Weight};
use sp_std::marker::PhantomData;

/// Weight functions for market.
pub struct WeightInfo<T>(PhantomData<T>);
impl<T: frame_system::Config> crate::WeightInfo for WeightInfo<T> {
	fn register() -> Weight {
		(166_000_000 as Weight)
			.saturating_add(T::DbWeight::get().reads(8 as Weight))
			.saturating_add(T::DbWeight::get().writes(5 as Weight))
	}
	fn add_collateral() -> Weight {
		(137_000_000 as Weight)
			.saturating_add(T::DbWeight::get().reads(8 as Weight))
			.saturating_add(T::DbWeight::get().writes(5 as Weight))
	}
	fn cut_collateral() -> Weight {
		(128_000_000 as Weight)
			.saturating_add(T::DbWeight::get().reads(7 as Weight))
			.saturating_add(T::DbWeight::get().writes(5 as Weight))
	}
	fn place_storage_order() -> Weight {
		(2_811_000_000 as Weight)
			.saturating_add(T::DbWeight::get().reads(16 as Weight))
			.saturating_add(T::DbWeight::get().writes(9 as Weight))
	}
	fn calculate_reward() -> Weight {
		(2_096_000_000 as Weight)
			.saturating_add(T::DbWeight::get().reads(8 as Weight))
			.saturating_add(T::DbWeight::get().writes(4 as Weight))
	}
	fn reward_merchant() -> Weight {
		(296_000_000 as Weight)
			.saturating_add(T::DbWeight::get().reads(8 as Weight))
			.saturating_add(T::DbWeight::get().writes(4 as Weight))
	}
}

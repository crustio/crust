#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]

use frame_support::{traits::Get, weights::{Weight, constants::RocksDbWeight}};
use sp_std::marker::PhantomData;

/// Weight functions needed for pallet_asset_manager.
pub trait WeightInfo {
	fn default_claim_weight() -> Weight;
	fn claim_weight() -> Weight;
}

/// Weights for pallet_asset_manager using the Substrate node and recommended hardware.
pub struct SubstrateWeight<T>(PhantomData<T>);
impl<T: frame_system::Config> WeightInfo for SubstrateWeight<T> {
	/// Storage: AssetManager AssetIdType (r:1 w:1)
	/// Proof Skipped: AssetManager AssetIdType (max_values: None, max_size: None, mode: Measured)
	/// Storage: Assets Asset (r:1 w:1)
	/// Proof: Assets Asset (max_values: None, max_size: Some(174), added: 2649, mode: MaxEncodedLen)
	/// Storage: Assets Metadata (r:1 w:1)
	/// Proof: Assets Metadata (max_values: None, max_size: Some(152), added: 2627, mode: MaxEncodedLen)
	/// Storage: AssetManager AssetTypeId (r:0 w:1)
	/// Proof Skipped: AssetManager AssetTypeId (max_values: None, max_size: None, mode: Measured)
	fn default_claim_weight() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `82`
		//  Estimated: `10885`
		// Minimum execution time: 51_631_000 picoseconds.
		Weight::from_parts(1_000_000, 10885)
			.saturating_add(T::DbWeight::get().reads(3_u64))
			.saturating_add(T::DbWeight::get().writes(4_u64))
	}

	fn claim_weight() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `82`
		//  Estimated: `10885`
		// Minimum execution time: 51_631_000 picoseconds.
		Weight::from_parts(0, 0)
	}
}

// For backwards compatibility and tests
impl WeightInfo for () {
	/// Storage: AssetManager AssetIdType (r:1 w:1)
	/// Proof Skipped: AssetManager AssetIdType (max_values: None, max_size: None, mode: Measured)
	/// Storage: Assets Asset (r:1 w:1)
	/// Proof: Assets Asset (max_values: None, max_size: Some(174), added: 2649, mode: MaxEncodedLen)
	/// Storage: Assets Metadata (r:1 w:1)
	/// Proof: Assets Metadata (max_values: None, max_size: Some(152), added: 2627, mode: MaxEncodedLen)
	/// Storage: AssetManager AssetTypeId (r:0 w:1)
	/// Proof Skipped: AssetManager AssetTypeId (max_values: None, max_size: None, mode: Measured)
	fn default_claim_weight() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `82`
		//  Estimated: `10885`
		// Minimum execution time: 51_631_000 picoseconds.
		Weight::from_parts(1_000_000, 10885)
			.saturating_add(RocksDbWeight::get().reads(3_u64))
			.saturating_add(RocksDbWeight::get().writes(4_u64))
	}

	fn claim_weight() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `82`
		//  Estimated: `10885`
		// Minimum execution time: 51_631_000 picoseconds.
		Weight::from_parts(0, 0)
	}
}
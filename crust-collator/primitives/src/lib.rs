// Copyright 2020-2021 Parity Technologies (UK) Ltd.
// This file is part of Cumulus.

// Substrate is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Substrate is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Cumulus.  If not, see <http://www.gnu.org/licenses/>.

//! Primitives used by the Parachains Tick, Trick and Track.

#![cfg_attr(not(feature = "std"), no_std)]

use sp_runtime::{
	generic,
	traits::{BlakeTwo256, IdentifyAccount, Verify},
	MultiSignature,
};
use sp_std::vec::Vec;

pub mod constants;
pub mod traits;

pub use sp_runtime::OpaqueExtrinsic as UncheckedExtrinsic;

/// Opaque block header type.
pub type Header = generic::Header<BlockNumber, BlakeTwo256>;
/// Opaque block type.
pub type Block = generic::Block<Header, UncheckedExtrinsic>;
/// Opaque block identifier type.
pub type BlockId = generic::BlockId<Block>;
/// An index to a block.
pub type BlockNumber = u32;

/// Counter for the number of eras that have passed.
pub type EraIndex = u32;

/// Alias to 512-bit hash when used in the context of a transaction signature on the chain.
pub type Signature = MultiSignature;

/// Some way of identifying an account on the chain. We intentionally make it equivalent
/// to the public key of our transaction signing scheme.
pub type AccountId = <<Signature as Verify>::Signer as IdentifyAccount>::AccountId;

/// The type for looking up accounts. We don't expect more than 4 billion of them, but you
/// never know...
pub type AccountIndex = u32;

/// Balance of an account.
pub type Balance = u128;

/// Index of a transaction in the chain.
pub type Index = u32;

/// A hash of some data used by the chain.
pub type Hash = sp_core::H256;

/// Digest item type.
pub type DigestItem = generic::DigestItem;

/// An instant or duration in time.
pub type Moment = u64;

/// The IAS signature type
pub type IASSig = Vec<u8>;

/// The ISV body type, contains the enclave code and public key
pub type ISVBody = Vec<u8>;

/// sworker certification type, begin with `-----BEGIN CERTIFICATE-----`
/// and end with `-----END CERTIFICATE-----`
pub type SworkerCert = Vec<u8>;

/// sworker public key, little-endian-format, 64 bytes vec
pub type SworkerPubKey = Vec<u8>;

/// sworker anchor, just use SworkerPubKey right now, 64 bytes vec
pub type SworkerAnchor = SworkerPubKey;

/// sworker signature, little-endian-format, 64 bytes vec
pub type SworkerSignature = Vec<u8>;

/// sworker enclave code
pub type SworkerCode = Vec<u8>;

/// Work report empty workload/storage merkle root
pub type MerkleRoot = Vec<u8>;

/// File Alias for a file
pub type FileAlias = Vec<u8>;

/// Report index, always be a multiple of era number
pub type ReportSlot = u64;

/// Market vendor's address info
pub type AddressInfo = Vec<u8>;

// Defines the trait to obtain a generic AssetType from a generic AssetId and viceversa
pub trait AssetTypeGetter<AssetId, AssetType> {
	// Get asset type from assetId
	fn get_asset_type(asset_id: AssetId) -> Option<AssetType>;

	// Get assetId from assetType
	fn get_asset_id(asset_type: AssetType) -> Option<AssetId>;
}

// Defines the trait to obtain the units per second of a give asset_type for local execution
// This parameter will be used to charge for fees upon asset_type deposit
pub trait UnitsToWeightRatio<AssetType> {
	// Whether payment in a particular asset_type is suppotrted
	fn payment_is_supported(asset_type: AssetType) -> bool;
	// Get units per second from asset type
	fn get_units_per_second(asset_type: AssetType) -> Option<u128>;
}
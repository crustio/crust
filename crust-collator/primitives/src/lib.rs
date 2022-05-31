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

use frame_support::{
	traits::{tokens::fungibles::Mutate, Get, OriginTrait},
	weights::{constants::WEIGHT_PER_SECOND, Weight},
};
use sp_runtime::traits::{CheckedConversion, Zero};
use sp_std::{borrow::Borrow, vec::Vec};
use sp_std::{
	convert::{TryFrom, TryInto},
	marker::PhantomData,
};
use xcm::latest::{
	AssetId as xcmAssetId, Error as XcmError, Fungibility,
	Junction::{AccountKey20, Parachain},
	Junctions::*,
	MultiAsset, MultiLocation, NetworkId,
};
use xcm_builder::TakeRevenue;
use xcm_executor::traits::{FilterAssetLocation, MatchesFungible, MatchesFungibles, WeightTrader};

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

pub type AssetId = u128;

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

/// XCM fee depositor to which we implement the TakeRevenue trait
/// It receives a fungibles::Mutate implemented argument, a matcher to convert MultiAsset into
/// AssetId and amount, and the fee receiver account
pub struct XcmFeesToAccount<Assets, Matcher, AccountId, ReceiverAccount>(
	PhantomData<(Assets, Matcher, AccountId, ReceiverAccount)>,
);
impl<
		Assets: Mutate<AccountId>,
		Matcher: MatchesFungibles<Assets::AssetId, Assets::Balance>,
		AccountId: Clone,
		ReceiverAccount: Get<AccountId>,
	> TakeRevenue for XcmFeesToAccount<Assets, Matcher, AccountId, ReceiverAccount>
{
	fn take_revenue(revenue: MultiAsset) {
		match Matcher::matches_fungibles(&revenue) {
			Ok((asset_id, amount)) => {
				if !amount.is_zero() {
					let ok = Assets::mint_into(asset_id, &ReceiverAccount::get(), amount).is_ok();
					debug_assert!(ok, "`mint_into` cannot generally fail; qed");
				}
			}
			Err(_) => log::debug!(
				target: "xcm",
				"take revenue failed matching fungible"
			),
		}
	}
}

/// Converter struct implementing `AssetIdConversion` converting a numeric asset ID
/// (must be `TryFrom/TryInto<u128>`) into a MultiLocation Value and Viceversa through
/// an intermediate generic type AssetType.
/// The trait bounds enforce is that the AssetTypeGetter trait is also implemented for
/// AssetIdInfoGetter
pub struct AsAssetType<AssetId, AssetType, AssetIdInfoGetter>(
	PhantomData<(AssetId, AssetType, AssetIdInfoGetter)>,
);
impl<AssetId, AssetType, AssetIdInfoGetter> xcm_executor::traits::Convert<MultiLocation, AssetId>
	for AsAssetType<AssetId, AssetType, AssetIdInfoGetter>
where
	AssetId: Clone,
	AssetType: From<MultiLocation> + Into<Option<MultiLocation>> + Clone,
	AssetIdInfoGetter: AssetTypeGetter<AssetId, AssetType>,
{
	fn convert_ref(id: impl Borrow<MultiLocation>) -> Result<AssetId, ()> {
		if let Some(asset_id) = AssetIdInfoGetter::get_asset_id(id.borrow().clone().into()) {
			Ok(asset_id)
		} else {
			Err(())
		}
	}
	fn reverse_ref(what: impl Borrow<AssetId>) -> Result<MultiLocation, ()> {
		if let Some(asset_type) = AssetIdInfoGetter::get_asset_type(what.borrow().clone()) {
			if let Some(location) = asset_type.into() {
				Ok(location)
			} else {
				Err(())
			}
		} else {
			Err(())
		}
	}
}

// We need to know how to charge for incoming assets
// This takes the first fungible asset, and takes whatever UnitPerSecondGetter establishes
// UnitsToWeightRatio trait, which needs to be implemented by AssetIdInfoGetter
pub struct FirstAssetTrader<
	AssetType: From<MultiLocation> + Clone,
	AssetIdInfoGetter: UnitsToWeightRatio<AssetType>,
	R: TakeRevenue,
>(
	Weight,
	Option<(MultiLocation, u128, u128)>,
	PhantomData<(AssetType, AssetIdInfoGetter, R)>,
);
impl<
		AssetType: From<MultiLocation> + Clone,
		AssetIdInfoGetter: UnitsToWeightRatio<AssetType>,
		R: TakeRevenue,
	> WeightTrader for FirstAssetTrader<AssetType, AssetIdInfoGetter, R>
{
	fn new() -> Self {
		FirstAssetTrader(0, None, PhantomData)
	}
	fn buy_weight(
		&mut self,
		weight: Weight,
		payment: xcm_executor::Assets,
	) -> Result<xcm_executor::Assets, XcmError> {
		log::trace!(
			target: "xcm::weight",
			"FirstAssetTrader::buy_weight weight: {:?}, payment: {:?}",
			weight, payment,
		);
		let first_asset = payment
			.clone()
			.fungible_assets_iter()
			.next()
			.ok_or(XcmError::TooExpensive)?;

		// We are only going to check first asset for now. This should be sufficient for simple token
		// transfers. We will see later if we change this.
		match (first_asset.id, first_asset.fun) {
			(xcmAssetId::Concrete(id), Fungibility::Fungible(_)) => {
				let asset_type: AssetType = id.clone().into();
				// Shortcut if we know the asset is not supported
				// This involves the same db read per block, mitigating any attack based on
				// non-supported assets
				if !AssetIdInfoGetter::payment_is_supported(asset_type.clone()) {
					return Err(XcmError::TooExpensive);
				}
				if let Some(units_per_second) = AssetIdInfoGetter::get_units_per_second(asset_type)
				{
					let amount = units_per_second.saturating_mul(weight as u128)
						/ (WEIGHT_PER_SECOND as u128);

					// We dont need to proceed if the amount is 0
					// For cases (specially tests) where the asset is very cheap with respect
					// to the weight needed
					if amount.is_zero() {
						return Ok(payment);
					}

					let required = MultiAsset {
						fun: Fungibility::Fungible(amount),
						id: xcmAssetId::Concrete(id.clone()),
					};
					let unused = payment
						.checked_sub(required)
						.map_err(|_| XcmError::TooExpensive)?;
					self.0 = self.0.saturating_add(weight);

					// In case the asset matches the one the trader already stored before, add
					// to later refund

					// Else we are always going to substract the weight if we can, but we latter do
					// not refund it

					// In short, we only refund on the asset the trader first succesfully was able
					// to pay for an execution
					let new_asset = match self.1.clone() {
						Some((prev_id, prev_amount, units_per_second)) => {
							if prev_id == id.clone() {
								Some((id, prev_amount.saturating_add(amount), units_per_second))
							} else {
								None
							}
						}
						None => Some((id, amount, units_per_second)),
					};

					// Due to the trait bound, we can only refund one asset.
					if let Some(new_asset) = new_asset {
						self.0 = self.0.saturating_add(weight);
						self.1 = Some(new_asset);
					};
					return Ok(unused);
				} else {
					return Err(XcmError::TooExpensive);
				};
			}
			_ => return Err(XcmError::TooExpensive),
		}
	}

	fn refund_weight(&mut self, weight: Weight) -> Option<MultiAsset> {
		if let Some((id, prev_amount, units_per_second)) = self.1.clone() {
			let weight = weight.min(self.0);
			self.0 -= weight;
			let amount = units_per_second * (weight as u128) / (WEIGHT_PER_SECOND as u128);
			self.1 = Some((
				id.clone(),
				prev_amount.saturating_sub(amount),
				units_per_second,
			));
			Some(MultiAsset {
				fun: Fungibility::Fungible(amount),
				id: xcmAssetId::Concrete(id.clone()),
			})
		} else {
			None
		}
	}
}

/// Deal with spent fees, deposit them as dictated by R
impl<
		AssetType: From<MultiLocation> + Clone,
		AssetIdInfoGetter: UnitsToWeightRatio<AssetType>,
		R: TakeRevenue,
	> Drop for FirstAssetTrader<AssetType, AssetIdInfoGetter, R>
{
	fn drop(&mut self) {
		if let Some((id, amount, _)) = self.1.clone() {
			R::take_revenue((id, amount).into());
		}
	}
}
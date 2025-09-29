use crate::{AssetIdType, AssetTypeId, AssetTypeUnitsPerSecond, Config, SupportedFeePaymentAssets};
#[cfg(feature = "try-runtime")]
use frame_support::storage::{generator::StorageValue, migration::get_storage_value};
use frame_support::{
	pallet_prelude::PhantomData,
	storage::migration::storage_key_iter,
	traits::{Get, OnRuntimeUpgrade},
	weights::Weight,
	Blake2_128Concat,
};
use parity_scale_codec::{Decode, Encode};
use sp_std::{vec::Vec, convert::TryInto};
use xcm::v5::prelude::*;

#[derive(Clone, Eq, Debug, PartialEq, Ord, PartialOrd, Encode, Decode)]
pub(crate) enum OldAssetType {
	Xcm(xcm::v3::MultiLocation),
}

impl Into<Option<xcm::v3::MultiLocation>> for OldAssetType {
	fn into(self) -> Option<xcm::v3::MultiLocation> {
		match self {
			Self::Xcm(location) => Some(location),
		}
	}
}

pub struct XcmV3ToV5AssetManager<T>(PhantomData<T>);
impl<T: Config> OnRuntimeUpgrade for XcmV3ToV5AssetManager<T>
where
	T::AssetType: From<Location>,
{
	fn on_runtime_upgrade() -> Weight {
		log::trace!(
			target: "XcmV3ToV5AssetManager",
			"Running XcmV3ToV5AssetManager migration"
		);
		
		// Db (read, write) count
		let mut db_weight_count: (u64, u64) = (0, 0);

		// Migrate `AssetIdType` value
		let _ = AssetIdType::<T>::translate::<OldAssetType, _>(|_key, value| {
			db_weight_count.0 += 1;
			db_weight_count.1 += 1;
			let old_multilocation: Option<xcm::v3::MultiLocation> = value.into();
			let old_multilocation: xcm::v3::MultiLocation =
				old_multilocation.expect("old storage convert to XcmV3 MultiLocation");
			let versioned: xcm::VersionedLocation = xcm::VersionedLocation::V3(old_multilocation.clone());
			let new_location: Location = versioned.try_into().expect("VersionedLocation v3 -> v5");
			Some(new_location.into())
		});

		// Migrate `AssetTypeId` key using new storage key iteration
		db_weight_count.0 += 1;
		let old_data = storage_key_iter::<OldAssetType, T::AssetId, Blake2_128Concat>(
			b"AssetManager",
			b"AssetTypeId",
		)
		.drain()
		.collect::<Vec<(OldAssetType, T::AssetId)>>();
		
		for (old_key, value) in old_data {
			db_weight_count.1 += 1;
			let old_key: Option<xcm::v3::MultiLocation> = old_key.into();
			let old_key: xcm::v3::MultiLocation =
				old_key.expect("old storage convert to XcmV3 MultiLocation");
			let versioned: xcm::VersionedLocation = xcm::VersionedLocation::V3(old_key.clone());
			let v5_location: Location = versioned.try_into().expect("VersionedLocation v3 -> v5");
			let new_key: T::AssetType = v5_location.into();
			AssetTypeId::<T>::insert(new_key, value);
		}

		// Migrate `AssetTypeUnitsPerSecond` key
		db_weight_count.0 += 1;
		let old_data = storage_key_iter::<OldAssetType, u128, Blake2_128Concat>(
			b"AssetManager",
			b"AssetTypeUnitsPerSecond",
		)
		.drain()
		.collect::<Vec<(OldAssetType, u128)>>();
		
		for (old_key, value) in old_data {
			db_weight_count.1 += 1;
			let old_key: Option<xcm::v3::MultiLocation> = old_key.into();
			let old_key: xcm::v3::MultiLocation =
				old_key.expect("old storage convert to XcmV3 MultiLocation");
			let versioned: xcm::VersionedLocation = xcm::VersionedLocation::V3(old_key.clone());
			let v5_location: Location = versioned.try_into().expect("VersionedLocation v3 -> v5");
			let new_key: T::AssetType = v5_location.into();
			AssetTypeUnitsPerSecond::<T>::insert(new_key, value);
		}

		// Migrate `SupportedFeePaymentAssets` value
		let _ = SupportedFeePaymentAssets::<T>::translate::<Vec<OldAssetType>, _>(|value| {
			db_weight_count.0 += 1;
			db_weight_count.1 += 1;
			let new_value: Vec<T::AssetType> = value
				.unwrap_or_default()
				.into_iter()
				.map(|old_value| {
					let old_multilocation: Option<xcm::v3::MultiLocation> = old_value.into();
					let old_multilocation: xcm::v3::MultiLocation =
						old_multilocation.expect("old storage convert to XcmV3 MultiLocation");
					let versioned: xcm::VersionedLocation = xcm::VersionedLocation::V3(old_multilocation.clone());
					let new_location: Location = versioned.try_into().expect("VersionedLocation v3 -> v5");
					new_location.into()
				})
				.collect();
			Some(new_value)
		});

		T::DbWeight::get().reads_writes(db_weight_count.0, db_weight_count.1)
	}
}
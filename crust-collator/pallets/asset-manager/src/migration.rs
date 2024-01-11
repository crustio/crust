use crate::{AssetIdType, AssetTypeId, AssetTypeUnitsPerSecond, Config, SupportedFeePaymentAssets};
#[cfg(feature = "try-runtime")]
use frame_support::storage::{generator::StorageValue, migration::get_storage_value};
use frame_support::{
	pallet_prelude::PhantomData,
	storage::migration::storage_key_iter,
	traits::{Get, OnRuntimeUpgrade},
	weights::Weight,
	Blake2_128Concat, StoragePrefixedMap,
};
use parity_scale_codec::{Decode, Encode};
//TODO sometimes this is unused, sometimes its necessary
use sp_std::{vec::Vec,convert::TryInto};
use xcm::latest::prelude::*;


#[derive(Clone, Eq, Debug, PartialEq, Ord, PartialOrd, Encode, Decode)]
pub(crate) enum OldAssetType {
	Xcm(xcm::v2::MultiLocation),
}

impl Into<Option<xcm::v2::MultiLocation>> for OldAssetType {
	fn into(self) -> Option<xcm::v2::MultiLocation> {
		match self {
			Self::Xcm(location) => Some(location),
		}
	}
}

pub struct XcmV2ToV3AssetManager<T>(PhantomData<T>);
impl<T: Config> OnRuntimeUpgrade for XcmV2ToV3AssetManager<T>
where
	T::AssetType: From<MultiLocation>,
{
	fn on_runtime_upgrade() -> Weight {
		log::trace!(
			target: "XcmV2ToV3AssetManager",
			"Running XcmV2ToV3AssetManager migration"
		);
		// Migrates the pallet's storage from Xcm V2 to V3:
		//	- AssetIdType -> migrate map's value
		//	- AssetTypeId -> migrate map's key
		//	- AssetTypeUnitsPerSecond -> migrate map's key
		//	- SupportedFeePaymentAssets -> migrate value

		// Shared module prefix
		let module_prefix = AssetIdType::<T>::module_prefix();
		// AssetTypeId
		let asset_type_id_storage_prefix = AssetTypeId::<T>::storage_prefix();
		// AssetTypeUnitsPerSecond
		let units_per_second_storage_prefix = AssetTypeUnitsPerSecond::<T>::storage_prefix();

		// Db (read, write) count
		let mut db_weight_count: (u64, u64) = (0, 0);

		// Migrate `AssetIdType` value
		let _ = AssetIdType::<T>::translate::<OldAssetType, _>(|_key, value| {
			db_weight_count.0 += 1;
			db_weight_count.1 += 1;
			let old_multilocation: Option<xcm::v2::MultiLocation> = value.into();
			let old_multilocation: xcm::v2::MultiLocation =
				old_multilocation.expect("old storage convert to XcmV2 Multilocation");
			let new_multilocation: MultiLocation = old_multilocation
				.try_into()
				.expect("Multilocation v2 to v3");
			Some(new_multilocation.into())
		});

		// Migrate `AssetTypeId` key
		db_weight_count.0 += 1;
		let old_data = storage_key_iter::<OldAssetType, T::AssetId, Blake2_128Concat>(
			&module_prefix,
			asset_type_id_storage_prefix,
		)
		.drain()
		.collect::<Vec<(OldAssetType, T::AssetId)>>();
		for (old_key, value) in old_data {
			db_weight_count.1 += 1;
			let old_key: Option<xcm::v2::MultiLocation> = old_key.into();
			let old_key: xcm::v2::MultiLocation =
				old_key.expect("old storage convert to XcmV2 Multilocation");
			let v3_multilocation: MultiLocation =
				old_key.try_into().expect("Multilocation v2 to v3");
			let new_key: T::AssetType = v3_multilocation.into();
			AssetTypeId::<T>::insert(new_key, value);
		}

		// Migrate `AssetTypeUnitsPerSecond` key
		db_weight_count.0 += 1;
		let old_data = storage_key_iter::<OldAssetType, u128, Blake2_128Concat>(
			&module_prefix,
			units_per_second_storage_prefix,
		)
		.drain()
		.collect::<Vec<(OldAssetType, u128)>>();
		for (old_key, value) in old_data {
			db_weight_count.1 += 1;
			let old_key: Option<xcm::v2::MultiLocation> = old_key.into();
			let old_key: xcm::v2::MultiLocation =
				old_key.expect("old storage convert to XcmV2 Multilocation");
			let v3_multilocation: MultiLocation =
				old_key.try_into().expect("Multilocation v2 to v3");
			let new_key: T::AssetType = v3_multilocation.into();
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
					let old_multilocation: Option<xcm::v2::MultiLocation> = old_value.into();
					let old_multilocation: xcm::v2::MultiLocation =
						old_multilocation.expect("old storage convert to XcmV2 Multilocation");
					let new_multilocation: MultiLocation = old_multilocation
						.try_into()
						.expect("Multilocation v2 to v3");
					new_multilocation.into()
				})
				.collect();
			Some(new_value)
		});

		T::DbWeight::get().reads_writes(db_weight_count.0, db_weight_count.1)
	}
}
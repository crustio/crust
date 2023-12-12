use super::{AssetIdType, AssetTypeId, AssetTypeUnitsPerSecond, Config, SupportedFeePaymentAssets};
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
use sp_std::vec::Vec;
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

#[cfg(feature = "try-runtime")]
#[derive(Clone, Eq, Debug, PartialEq, Ord, PartialOrd, Encode, Decode)]
enum PreUpgradeState<T: Config> {
	AssetIdType(Vec<(T::AssetId, OldAssetType)>),
	AssetTypeId(Vec<(OldAssetType, T::AssetId)>),
	AssetTypeUnitsPerSecond(Vec<(OldAssetType, u128)>),
	SupportedFeePaymentAssets(Vec<OldAssetType>),
}

#[cfg(feature = "try-runtime")]
#[derive(Clone, Eq, Debug, PartialEq, Ord, PartialOrd, Encode, Decode)]
enum PostUpgradeState<T: Config> {
	AssetIdType(Vec<(T::AssetId, T::ForeignAssetType)>),
	AssetTypeId(Vec<(T::ForeignAssetType, T::AssetId)>),
	AssetTypeUnitsPerSecond(Vec<(T::ForeignAssetType, u128)>),
	SupportedFeePaymentAssets(Vec<T::ForeignAssetType>),
}

#[cfg(feature = "try-runtime")]
impl<T: Config> From<PreUpgradeState<T>> for PostUpgradeState<T>
where
	T::ForeignAssetType: From<MultiLocation>,
{
	fn from(pre: PreUpgradeState<T>) -> PostUpgradeState<T> {
		match pre {
			PreUpgradeState::AssetIdType(items) => {
				let mut out: Vec<(T::AssetId, T::ForeignAssetType)> = Vec::new();
				for (key, value) in items.into_iter() {
					let old_multilocation: Option<xcm::v2::MultiLocation> = value.into();
					let old_multilocation: xcm::v2::MultiLocation =
						old_multilocation.expect("old storage convert to XcmV2 Multilocation");
					let new_multilocation: MultiLocation = old_multilocation
						.try_into()
						.expect("Multilocation v2 to v3");
					out.push((key, new_multilocation.into()));
				}
				PostUpgradeState::AssetIdType(out)
			}
			PreUpgradeState::AssetTypeId(items) => {
				let mut out: Vec<(T::ForeignAssetType, T::AssetId)> = Vec::new();
				for (key, value) in items.into_iter() {
					let old_multilocation: Option<xcm::v2::MultiLocation> = key.into();
					let old_multilocation: xcm::v2::MultiLocation =
						old_multilocation.expect("old storage convert to XcmV2 Multilocation");
					let new_multilocation: MultiLocation = old_multilocation
						.try_into()
						.expect("Multilocation v2 to v3");
					let new_key: T::ForeignAssetType = new_multilocation.into();
					out.push((new_key, value));
				}
				PostUpgradeState::AssetTypeId(out)
			}
			PreUpgradeState::AssetTypeUnitsPerSecond(items) => {
				let mut out: Vec<(T::ForeignAssetType, u128)> = Vec::new();
				for (key, value) in items.into_iter() {
					let old_multilocation: Option<xcm::v2::MultiLocation> = key.into();
					let old_multilocation: xcm::v2::MultiLocation =
						old_multilocation.expect("old storage convert to XcmV2 Multilocation");
					let new_multilocation: MultiLocation = old_multilocation
						.try_into()
						.expect("Multilocation v2 to v3");
					out.push((new_multilocation.into(), value));
				}
				PostUpgradeState::AssetTypeUnitsPerSecond(out)
			}
			PreUpgradeState::SupportedFeePaymentAssets(items) => {
				let mut out: Vec<T::ForeignAssetType> = Vec::new();
				for value in items.into_iter() {
					let old_multilocation: Option<xcm::v2::MultiLocation> = value.into();
					let old_multilocation: xcm::v2::MultiLocation =
						old_multilocation.expect("old storage convert to XcmV2 Multilocation");
					let new_multilocation: MultiLocation = old_multilocation
						.try_into()
						.expect("Multilocation v2 to v3");
					out.push(new_multilocation.into());
				}
				PostUpgradeState::SupportedFeePaymentAssets(out)
			}
		}
	}
}

pub struct XcmV2ToV3AssetManager<T>(PhantomData<T>);
impl<T: Config> OnRuntimeUpgrade for XcmV2ToV3AssetManager<T>
where
	T::ForeignAssetType: From<MultiLocation>,
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
			let new_key: T::ForeignAssetType = v3_multilocation.into();
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
			let new_key: T::ForeignAssetType = v3_multilocation.into();
			AssetTypeUnitsPerSecond::<T>::insert(new_key, value);
		}

		// Migrate `SupportedFeePaymentAssets` value
		let _ = SupportedFeePaymentAssets::<T>::translate::<Vec<OldAssetType>, _>(|value| {
			db_weight_count.0 += 1;
			db_weight_count.1 += 1;
			let new_value: Vec<T::ForeignAssetType> = value
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

	#[cfg(feature = "try-runtime")]
	fn pre_upgrade() -> Result<Vec<u8>, &'static str> {
		log::trace!(
			target: "XcmV2ToV3AssetManager",
			"Running XcmV2ToV3AssetManager pre_upgrade hook"
		);
		// Shared module prefix
		let module_prefix = AssetIdType::<T>::module_prefix();
		// AssetIdType
		let asset_id_type_storage_prefix = AssetIdType::<T>::storage_prefix();
		// AssetTypeId
		let asset_type_id_storage_prefix = AssetTypeId::<T>::storage_prefix();
		// AssetTypeUnitsPerSecond
		let units_per_second_storage_prefix = AssetTypeUnitsPerSecond::<T>::storage_prefix();
		// SupportedFeePaymentAssets
		let supported_fee_storage_prefix = SupportedFeePaymentAssets::<T>::storage_prefix();

		let mut result: Vec<PreUpgradeState<T>> = Vec::new();

		// AssetIdType pre-upgrade data
		let asset_id_type_storage_data: Vec<_> = storage_key_iter::<
			T::AssetId,
			OldAssetType,
			Blake2_128Concat,
		>(module_prefix, asset_id_type_storage_prefix)
		.collect();
		result.push(PreUpgradeState::<T>::AssetIdType(
			asset_id_type_storage_data,
		));

		// AssetTypeId pre-upgrade data
		let asset_type_id_storage_data: Vec<_> = storage_key_iter::<
			OldAssetType,
			T::AssetId,
			Blake2_128Concat,
		>(module_prefix, asset_type_id_storage_prefix)
		.collect();
		result.push(PreUpgradeState::<T>::AssetTypeId(
			asset_type_id_storage_data,
		));

		// AssetTypeUnitsPerSecond pre-upgrade data
		let units_per_second_storage_data: Vec<_> =
			storage_key_iter::<OldAssetType, u128, Blake2_128Concat>(
				module_prefix,
				units_per_second_storage_prefix,
			)
			.collect();
		result.push(PreUpgradeState::<T>::AssetTypeUnitsPerSecond(
			units_per_second_storage_data,
		));

		// SupportedFeePaymentAssets pre-upgrade data
		let supported_fee_storage_data: Vec<_> = get_storage_value::<Vec<OldAssetType>>(
			module_prefix,
			supported_fee_storage_prefix,
			&[],
		)
		.expect("SupportedFeePaymentAssets value");
		result.push(PreUpgradeState::<T>::SupportedFeePaymentAssets(
			supported_fee_storage_data,
		));

		Ok(result.encode())
	}

	#[cfg(feature = "try-runtime")]
	fn post_upgrade(state: Vec<u8>) -> Result<(), &'static str> {
		log::trace!(
			target: "XcmV2ToV3AssetManager",
			"Running XcmV2ToV3AssetManager post_upgrade hook"
		);
		let pre_upgrade_state: Vec<PreUpgradeState<T>> =
			Decode::decode(&mut &state[..]).expect("pre_upgrade provides a valid state; qed");

		// Shared module prefix
		let module_prefix = AssetIdType::<T>::module_prefix();
		// AssetIdType
		let asset_id_type_storage_prefix = AssetIdType::<T>::storage_prefix();
		// AssetTypeId
		let asset_type_id_storage_prefix = AssetTypeId::<T>::storage_prefix();
		// AssetTypeUnitsPerSecond
		let units_per_second_storage_prefix = AssetTypeUnitsPerSecond::<T>::storage_prefix();

		// First we convert pre-state to post-state. This is equivalent to what the migration
		// should do. If this conversion and the result of the migration match, we consider it a
		// success.
		let to_post_upgrade: Vec<PostUpgradeState<T>> = pre_upgrade_state
			.into_iter()
			.map(|value| value.into())
			.collect();

		// Because the order of the storage and the pre-upgrade vector is likely different,
		// we encode everything, which is easier to sort and compare.
		let mut expected_post_upgrade_state: Vec<Vec<u8>> = Vec::new();
		for item in to_post_upgrade.iter() {
			match item {
				// Vec<(T::AssetId, T::ForeignAssetType)>
				PostUpgradeState::AssetIdType(items) => {
					for inner in items.into_iter() {
						expected_post_upgrade_state.push(inner.encode())
					}
				}
				// Vec<(T::ForeignAssetType, T::AssetId)>
				PostUpgradeState::AssetTypeId(items) => {
					for inner in items.into_iter() {
						expected_post_upgrade_state.push(inner.encode())
					}
				}
				// Vec<(T::ForeignAssetType, u128)>
				PostUpgradeState::AssetTypeUnitsPerSecond(items) => {
					for inner in items.into_iter() {
						expected_post_upgrade_state.push(inner.encode())
					}
				}
				// Vec<T::ForeignAssetType>
				PostUpgradeState::SupportedFeePaymentAssets(items) => {
					for inner in items.into_iter() {
						expected_post_upgrade_state.push(inner.encode())
					}
				}
			}
		}

		// Then we retrieve the actual state after migration.
		let mut actual_post_upgrade_state: Vec<Vec<u8>> = Vec::new();

		// Actual AssetIdType post-upgrade data
		let asset_id_type_storage_data: Vec<_> = storage_key_iter::<
			T::AssetId,
			T::ForeignAssetType,
			Blake2_128Concat,
		>(module_prefix, asset_id_type_storage_prefix)
		.collect();
		for item in asset_id_type_storage_data.iter() {
			actual_post_upgrade_state.push(item.encode())
		}

		// Actual AssetTypeId post-upgrade data
		let asset_type_id_storage_data: Vec<_> = storage_key_iter::<
			T::ForeignAssetType,
			T::AssetId,
			Blake2_128Concat,
		>(module_prefix, asset_type_id_storage_prefix)
		.collect();
		for item in asset_type_id_storage_data.iter() {
			actual_post_upgrade_state.push(item.encode())
		}

		// Actual AssetTypeUnitsPerSecond post-upgrade data
		let units_per_second_storage_data: Vec<_> =
			storage_key_iter::<T::ForeignAssetType, u128, Blake2_128Concat>(
				module_prefix,
				units_per_second_storage_prefix,
			)
			.collect();
		for item in units_per_second_storage_data.iter() {
			actual_post_upgrade_state.push(item.encode())
		}

		// Actual SupportedFeePaymentAssets post-upgrade data
		let supported_fee_storage_data: Vec<_> = SupportedFeePaymentAssets::<T>::get();
		for item in supported_fee_storage_data.iter() {
			actual_post_upgrade_state.push(item.encode())
		}

		// Both state blobs are sorted.
		expected_post_upgrade_state.sort();
		actual_post_upgrade_state.sort();

		// Assert equality
		assert_eq!(expected_post_upgrade_state, actual_post_upgrade_state);

		Ok(())
	}
}
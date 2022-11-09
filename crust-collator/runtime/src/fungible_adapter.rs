use crate::traits::*;
use super::NativeAssetChecker;
use sp_std::{convert::Into, marker::PhantomData, result::Result, vec::Vec};
use xcm::latest::{prelude::*, Error as XcmError, MultiAsset, MultiLocation, Result as XcmResult};
use xcm_executor::{traits::TransactAsset, Assets};

const LOG_TARGET: &str = "runtime::fungbible-adapter";

pub struct CrustTransferAdapter<NativeAdapter, AssetsAdapter, NativeChecker>(
	PhantomData<(NativeAdapter, AssetsAdapter, NativeChecker)>,
);

impl<
		NativeAdapter: TransactAsset,
		AssetsAdapter: TransactAsset,
		NativeChecker: NativeAssetChecker,
	> TransactAsset for CrustTransferAdapter<NativeAdapter, AssetsAdapter, NativeChecker>
{
	fn can_check_in(origin: &MultiLocation, what: &MultiAsset) -> XcmResult {
		if NativeChecker::is_native_asset(what) {
			return NativeAdapter::can_check_in(origin, what);
		} else {
			return AssetsAdapter::can_check_in(origin, what);
		};
	}

	fn check_in(origin: &MultiLocation, what: &MultiAsset) {
		if NativeChecker::is_native_asset(what) {
			NativeAdapter::check_in(origin, what)
		} else {
			AssetsAdapter::check_in(origin, what)
		};
	}

	fn check_out(dest: &MultiLocation, what: &MultiAsset) {
		if NativeChecker::is_native_asset(what) {
			NativeAdapter::check_out(dest, what)
		} else {
			AssetsAdapter::check_out(dest, what)
		};
	}

	fn deposit_asset(what: &MultiAsset, who: &MultiLocation) -> XcmResult {
		if NativeChecker::is_native_asset(what) {
			return  NativeAdapter::deposit_asset(what, who);
		} else {
			return AssetsAdapter::deposit_asset(what, who);
		};
	}

	fn withdraw_asset(what: &MultiAsset, who: &MultiLocation) -> Result<Assets, XcmError> {
		if NativeChecker::is_native_asset(what) {
			return NativeAdapter::withdraw_asset(what, who);
		} else {
			return  AssetsAdapter::withdraw_asset(what, who);
		};
	}

	fn internal_transfer_asset(
		what: &MultiAsset,
		from: &MultiLocation,
		to: &MultiLocation,
	) -> Result<Assets, XcmError> {
		if NativeChecker::is_native_asset(what) {
			return NativeAdapter::internal_transfer_asset(what, from, to);
		} else {
			return AssetsAdapter::internal_transfer_asset(what, from, to);
		};
	}
}
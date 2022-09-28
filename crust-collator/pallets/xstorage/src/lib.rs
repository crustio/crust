#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::pallet;
pub use pallet::*;

#[pallet]
pub mod pallet {
	use sp_std::prelude::*;
	use frame_support::{pallet_prelude::*, PalletId};
	use frame_system::pallet_prelude::*;

	use xcm::v2::prelude::*;
	use sp_std::convert::TryInto;
	use sp_runtime::traits::{AccountIdConversion, Convert};

	use xcm_executor::traits::TransactAsset;

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

	/// The AssetManagers's pallet id
	pub const PALLET_ID: PalletId = PalletId(*b"xstorage");

	/// Configure the pallet by specifying the parameters and types on which it depends.
	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// Overarching event type.
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

		type XcmpMessageSender: SendXcm;

		/// AssetTransactor allows us to transfer asset
		type AssetTransactor: TransactAsset;

		/// Currency Id.
		type CurrencyId: Parameter + Member + Clone;

		/// Convert `T::CurrencyId` to `MultiLocation`.
		type CurrencyIdToMultiLocation: Convert<Self::CurrencyId, Option<MultiLocation>>;

		/// Convert `T::AccountId` to `MultiLocation`.
		type AccountIdToMultiLocation: Convert<Self::AccountId, MultiLocation>;

		/// Origin that is allowed to create and modify storage fee information
		type StorageFeeOwner: EnsureOrigin<Self::Origin>;
	}

	/// An error that can occur while executing the mapping pallet's logic.
	#[pallet::error]
	pub enum Error<T> {
		NotCrossChainTransferableCurrency,
		NotSupportedCurrency,
		UnableToTransferStorageFee,
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(crate) fn deposit_event)]
	pub enum Event<T: Config> {
		/// New asset with the asset manager is registered
		FileSuccess {
			account: T::AccountId,
			cid: Vec<u8>,
			size: u64
		},
		StorageFeeRegistered {
			currency_id: T::CurrencyId,
			amount: u128
		}
	}

	#[pallet::storage]
	#[pallet::getter(fn storage_fee_per_currency)]
	pub type StorageFeePerCurrency<T: Config> =
		StorageMap<_, Blake2_128Concat, T::CurrencyId, u128>;

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		// The index cannot be changed.
		#[pallet::weight(1_000_000)]
		pub fn place_storage_order_through_parachain(
			origin: OriginFor<T>,
			cid: Vec<u8>,
			size: u64
		) -> DispatchResult {
			let who = ensure_signed(origin)?;

			Self::deposit_event(Event::FileSuccess {
				account: who,
				cid,
				size,
			});

			Ok(().into())
		}

		#[pallet::weight(1_000_000)]
		pub fn place_storage_order(
			origin: OriginFor<T>,
			cid: Vec<u8>,
			size: u64,
			currency_id: T::CurrencyId
		) -> DispatchResult {
			let who = ensure_signed(origin)?;

			let location: MultiLocation =
				T::CurrencyIdToMultiLocation::convert(currency_id.clone()).ok_or(Error::<T>::NotCrossChainTransferableCurrency)?;

			let amount = StorageFeePerCurrency::<T>::get(&currency_id)
			.ok_or(Error::<T>::NotSupportedCurrency)?;

			let fee: MultiAsset = MultiAsset {
				id: Concrete(location),
				fun: Fungible(amount),
			};

			// Convert origin to multilocation
			let origin_as_mult = T::AccountIdToMultiLocation::convert(who.clone());
			let dest_as_mult = T::AccountIdToMultiLocation::convert(Self::account_id());

			T::AssetTransactor::internal_transfer_asset(&fee.clone().into(), &origin_as_mult, &dest_as_mult)
				.map_err(|_| Error::<T>::UnableToTransferStorageFee)?;

			Self::deposit_event(Event::FileSuccess {
				account: who,
				cid,
				size,
			});

			Ok(().into())
		}

		#[pallet::weight(1_000_000)]
		pub fn register_storage_fee(
			origin: OriginFor<T>,
			currency_id: T::CurrencyId,
			amount: u128
		) -> DispatchResult {
			T::StorageFeeOwner::ensure_origin(origin)?;

			let _: MultiLocation =
				T::CurrencyIdToMultiLocation::convert(currency_id.clone()).ok_or(Error::<T>::NotCrossChainTransferableCurrency)?;

			<StorageFeePerCurrency<T>>::insert(currency_id.clone(), amount);

			Self::deposit_event(Event::StorageFeeRegistered {
				currency_id,
				amount,
			});

			Ok(().into())
		}
	}

	impl<T: Config> Pallet<T> {
		/// The account ID of AssetManager
		pub fn account_id() -> T::AccountId {
			PALLET_ID.into_account_truncating()
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	use frame_support::parameter_types;
	use xcm::latest::prelude::*;
	use xcm::latest::{Junction, NetworkId::Any, NetworkId};
	use xcm_builder::{
		Account32Hash
	};
	use xcm_executor::traits::Convert;
	pub use crust_parachain_primitives::{
		constants::{currency::*}, traits::*,
		AssetId, *
	};
	use codec::Encode;
	use hex_literal::hex;

	fn account20() -> Junction {
		// AccountKey20 { network: Any, key: [35,44,156,153,241,115,33,39,211,198,18,135,234,216,198,58,169,206,63,43] }
		AccountKey20 { network: Any, key: hex!("173023992F5DD374193963cbD3F5F7eA6d3eAf15").into() }
	}
	#[test]
	fn convert_location() {
		parameter_types! {
			pub const RelayNetwork: NetworkId = NetworkId::Kusama;
		}

		let input = MultiLocation::new(1, X2(Parachain(1000), account20()));
		let output = hex::encode(Account32Hash::<RelayNetwork, AccountId>::convert_ref(&input).unwrap().encode());
		assert_eq!(output, "39391a315541eb4aa52c745b78e35aefcecf1a0ff1525e94e63b4dd006f81846");
	}
}
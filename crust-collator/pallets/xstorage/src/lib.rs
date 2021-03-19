#![cfg_attr(not(feature = "std"), no_std)]

use codec::Encode;
use frame_support::{decl_error, decl_module, decl_storage, pallet_prelude::*};
use frame_system::{ensure_signed, ensure_root};
use sp_std::prelude::*;

use cumulus_primitives_core::{ParaId, HrmpMessageSender, OutboundHrmpMessage};
use xcm::v0::{Xcm, OriginKind};

pub trait PrepareStorageOrder {
	fn prepare_storage_order(cid: Vec<u8>, size: u64) -> Vec<u8>;
}

pub trait DoPlaceStorageOrder {
	fn do_place_storage_order(cid: Vec<u8>, size: u64);
}

impl DoPlaceStorageOrder for () {
	fn do_place_storage_order(_: Vec<u8>, _: u64) {

	}
}

pub trait Config: frame_system::Config {
	/// Something to send an HRMP message.
	type HrmpMessageSender: HrmpMessageSender;

	type Preparator: PrepareStorageOrder;

	type DoPlaceStorageOrder: DoPlaceStorageOrder;
}

decl_error! {
	pub enum Error for Module<T: Config> {
		// Failed to send
		FailedToSend
	}
}

decl_storage! {
	trait Store for Module<T: Config> as ParachainInfo {
		CrossChainFiles get(fn crost_chain_files): map hasher(twox_64_concat) Vec<u8> => u64;
	}
}


decl_module! {
	pub struct Module<T: Config> for enum Call where origin: <T as frame_system::Config>::Origin {
		#[weight = 1_000]
		pub fn inner_place_storage_order(origin, cid: Vec<u8>, size: u64) -> DispatchResultWithPostInfo {
			let _ = ensure_root(origin)?;
			CrossChainFiles::insert(&cid, size);
			T::DoPlaceStorageOrder::do_place_storage_order(cid, size);
			Ok(().into())
		}

		#[weight = 1_000]
		pub fn place_storage_order_cross_parachain(
			origin,
			cid: Vec<u8>,
			size: u64,
		) -> DispatchResultWithPostInfo {
			let _who = ensure_signed(origin)?;

			let set_call = <T as Config>::Preparator::prepare_storage_order(cid, size);

			// TODO: Use RelayedFrom instead of Transact to include account id
			let transact = Xcm::Transact {
				origin_type: OriginKind::Superuser,
				call: set_call
			};

			let message = xcm::VersionedXcm::V0(transact);
			let recipient: ParaId = 7777.into();

			let data = message.encode();
			let outbound_message = OutboundHrmpMessage {
				recipient,
				data,
			};

			// TODO: Use Xtoken as well to pay this order
			T::HrmpMessageSender::send_hrmp_message(outbound_message).map_err(|_| Error::<T>::FailedToSend)?;

			Ok(().into())
		}
	}
}
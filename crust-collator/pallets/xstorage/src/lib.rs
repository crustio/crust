#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::{decl_error, decl_module, decl_storage, pallet_prelude::*};
use frame_system::ensure_signed;
use sp_std::prelude::*;

use xcm::v2::prelude::*;
use codec::Encode;
use sp_std::convert::TryInto;

pub trait Config: frame_system::Config {
	/// Something to send an HRMP message.
	type XcmpMessageSender: SendXcm;
}

decl_error! {
	pub enum Error for Module<T: Config> {
		// Failed to send
		FailedToSend
	}
}

decl_storage! {
	trait Store for Module<T: Config> as ParachainInfo { }
}


decl_module! {
	pub struct Module<T: Config> for enum Call where origin: <T as frame_system::Config>::Origin {
		#[weight = 1_000]
		pub fn place_storage_order_cross_parachain(
			origin,
			cid: Vec<u8>,
			size: u64,
		) -> DispatchResultWithPostInfo {
			let _who = ensure_signed(origin)?;

			let set_call = (100u8, 0u8, cid, size).encode();

			// TODO: Use RelayedFrom instead of Transact to include account id
			let transact = Xcm(vec![Instruction::Transact {
				origin_type: OriginKind::Superuser,
				require_weight_at_most: 1_000,
				call: set_call.into(),
			}]);

			// TODO: Use Xtoken as well to pay this order
			T::XcmpMessageSender::send_xcm(
				MultiLocation { parents: 0, interior: X1(Parachain(2008)) },
				transact).map_err(|_| Error::<T>::FailedToSend)?;

			Ok(().into())
		}
	}
}
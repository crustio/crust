// Ensure we're `no_std` when compiling for Wasm.
#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::traits::{Currency, EnsureOrigin, ExistenceRequirement::AllowDeath, Get};
use frame_support::{
	decl_error, decl_event, decl_module, decl_storage, dispatch::DispatchResult, ensure,
};
use frame_system::{self as system, ensure_root, ensure_signed};
use cstrml_bridge as bridge;
use sp_arithmetic::traits::SaturatedConversion;
use sp_core::U256;
use sp_std::prelude::*;
use sp_runtime::traits::Saturating;

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;


type BalanceOf<T> =
	<<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

pub trait Config: system::Config + bridge::Config {
	type Event: From<Event<Self>> + Into<<Self as frame_system::Config>::Event>;

	/// Specifies the origin check provided by the bridge for calls that can only be called by the bridge pallet
	type BridgeOrigin: EnsureOrigin<Self::Origin, Success = Self::AccountId>;

	/// The currency mechanism.
	type Currency: Currency<Self::AccountId>;

	type BridgeTokenId: Get<[u8; 32]>;
}

decl_storage! {
	trait Store for Module<T: Config> as BridgeTransfer {
		BridgeFee get(fn bridge_fee): map hasher(opaque_blake2_256) u8 => (BalanceOf<T>, u32);
	}
}

decl_event! {
	pub enum Event<T>
	where
		Balance = BalanceOf<T>,
	{
		/// [chainId, min_fee, fee_scale]
		FeeUpdated(u8, Balance, u32),
	}
}

decl_error! {
	pub enum Error for Module<T: Config>{
		InvalidTransfer,
		InvalidCommand,
		InvalidPayload,
		InvalidFeeOption,
		FeeOptionsMissiing,
		LessThanFee,
	}
}

decl_module! {
	pub struct Module<T: Config> for enum Call where origin: T::Origin {
		type Error = Error<T>;
		//
		// Initiation calls. These start a bridge transfer.
		//

		fn deposit_event() = default;

		/// Change extra bridge transfer fee that user should pay
		#[weight = 195_000_000]
		pub fn sudo_change_fee(origin, min_fee: BalanceOf<T>, fee_scale: u32, dest_id: u8) -> DispatchResult {
			ensure_root(origin)?;
			ensure!(fee_scale <= 1000u32, Error::<T>::InvalidFeeOption);
			BridgeFee::<T>::insert(dest_id, (min_fee, fee_scale));
			Self::deposit_event(RawEvent::FeeUpdated(dest_id, min_fee, fee_scale));
			Ok(())
		}

		/// Transfers some amount of the native token to some recipient on a (whitelisted) destination chain.
		#[weight = 195_000_000]
		pub fn transfer_native(origin, amount: BalanceOf<T>, recipient: Vec<u8>, dest_id: u8) -> DispatchResult {
			let source = ensure_signed(origin)?;
			ensure!(<bridge::Module<T>>::chain_whitelisted(dest_id), Error::<T>::InvalidTransfer);
			let bridge_id = <bridge::Module<T>>::account_id();
			ensure!(BridgeFee::<T>::contains_key(&dest_id), Error::<T>::FeeOptionsMissiing);
			let (min_fee, fee_scale) = Self::bridge_fee(dest_id);
			let fee_estimated = amount * fee_scale.into() / 1000u32.into();
			let fee = if fee_estimated > min_fee {
				fee_estimated
			} else {
				min_fee
			};
			ensure!(amount > fee, Error::<T>::LessThanFee);
			T::Currency::transfer(&source, &bridge_id, amount.into(), AllowDeath)?;

			<bridge::Module<T>>::transfer_fungible(dest_id, T::BridgeTokenId::get(), recipient, U256::from(amount.saturating_sub(fee).saturated_into::<u128>()))
		}

		//
		// Executable calls. These can be triggered by a bridge transfer initiated on another chain
		//

		/// Executes a simple currency transfer using the bridge account as the source
		#[weight = 195_000_000]
		pub fn transfer(origin, to: T::AccountId, amount: BalanceOf<T>, _rid: [u8; 32]) -> DispatchResult {
			let source = T::BridgeOrigin::ensure_origin(origin)?;
			<T as Config>::Currency::transfer(&source, &to, amount.into(), AllowDeath)?;
			Ok(())
		}
	}
}

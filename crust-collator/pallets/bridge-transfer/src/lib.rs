#![deny(warnings)]
#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::{
    Call,
    Config,
    Error,
    Event,
    Pallet,
    *,
};

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

mod types {
    use crate::pallet::Config;
    use frame_support::traits::Currency;

    pub type BalanceOf<T> = <<T as Config>::Currency as Currency<
        <T as frame_system::Config>::AccountId,
    >>::Balance;
}

#[frame_support::pallet]
pub mod pallet {
    use crate::types::{
        BalanceOf,
    };
    use frame_support::{
        pallet_prelude::*,
        traits::{
            Currency,
            ExistenceRequirement::AllowDeath,
        },
    };
    use frame_system::pallet_prelude::*;
    use frame_support::sp_runtime::traits::Saturating;
    use frame_support::sp_runtime::SaturatedConversion;
    use sp_core::U256;
    use sp_std::convert::TryInto;
    use sp_std::vec::Vec;
    use cstrml_bridge as bridge;

    #[pallet::pallet]
    #[pallet::generate_store(pub(super) trait Store)]
    pub struct Pallet<T>(_);

    /// Tracks current relayer set
    #[pallet::storage]
    #[pallet::getter(fn bridge_fee)]
    pub type BridgeFee<T: Config> =
        StorageMap<_, Blake2_256, u8, (BalanceOf<T>, u32), ValueQuery>;

    /// Configure the pallet by specifying the parameters and types on which it depends.
    #[pallet::config]
    pub trait Config:
        frame_system::Config
        + bridge::Config
    {
        /// Because this pallet emits events, it depends on the runtime's definition of an event.
        type RuntimeEvent: From<Event<Self>>
            + IsType<<Self as frame_system::Config>::RuntimeEvent>;

        /// Specifies the origin check provided by the bridge for calls that can only be called by
        /// the bridge pallet
        type BridgeOrigin: EnsureOrigin<Self::RuntimeOrigin, Success = Self::AccountId>;

        /// The currency mechanism
        type Currency: Currency<Self::AccountId>;

        /// Ids can be defined by the runtime and passed in, perhaps from blake2b_128 hashes.
        type BridgeTokenId: Get<[u8; 32]>;
    }

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        FeeUpdated(u8, BalanceOf<T>, u32),
    }

    // Errors inform users that something went wrong.
    #[pallet::error]
    pub enum Error<T> {
        InvalidTransfer,
		InvalidCommand,
		InvalidPayload,
		InvalidFeeOption,
		FeeOptionsMissiing,
		LessThanFee
    }

    // Dispatchable functions allows users to interact with the pallet and invoke state changes.
    // These functions materialize as "extrinsics", which are often compared to transactions.
    // Dispatchable functions must be annotated with a weight and must return a DispatchResult.
    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Change extra bridge transfer fee that user should pay
		#[pallet::weight(195_000_000)]
		pub fn sudo_change_fee(origin: OriginFor<T>, min_fee: BalanceOf<T>, fee_scale: u32, dest_id: u8) -> DispatchResult {
			ensure_root(origin)?;
			ensure!(fee_scale <= 1000u32, Error::<T>::InvalidFeeOption);
			BridgeFee::<T>::insert(dest_id, (min_fee, fee_scale));
			Self::deposit_event(Event::FeeUpdated(dest_id, min_fee, fee_scale));
			Ok(())
		}

		/// Transfers some amount of the native token to some recipient on a (whitelisted) destination chain.
		#[pallet::weight(195_000_000)]
		pub fn transfer_native(origin: OriginFor<T>, amount: BalanceOf<T>, recipient: Vec<u8>, dest_id: u8) -> DispatchResult {
			let source = ensure_signed(origin)?;
			ensure!(<bridge::Pallet<T>>::chain_whitelisted(dest_id), Error::<T>::InvalidTransfer);
			let bridge_id = <bridge::Pallet<T>>::account_id();
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

			<bridge::Pallet<T>>::transfer_fungible(dest_id, T::BridgeTokenId::get(), recipient, U256::from(amount.saturating_sub(fee).saturated_into::<u128>()))
		}

		//
		// Executable calls. These can be triggered by a bridge transfer initiated on another chain
		//

		/// Executes a simple currency transfer using the bridge account as the source
		#[pallet::weight(195_000_000)]
		pub fn transfer(origin: OriginFor<T>, to: T::AccountId, amount: BalanceOf<T>, _rid: [u8; 32]) -> DispatchResult {
			let source = T::BridgeOrigin::ensure_origin(origin)?;
			<T as Config>::Currency::transfer(&source, &to, amount.into(), AllowDeath)?;
			Ok(())
		}

    }
}

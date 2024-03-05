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

pub mod weights;

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
            Currency, WithdrawReasons,
            ExistenceRequirement::AllowDeath,
        },
    };
    use frame_system::pallet_prelude::*;
    use frame_support::sp_runtime::traits::Saturating;
    use frame_support::sp_runtime::SaturatedConversion;
    use sp_core::U256;
    use sp_std::convert::TryInto;
    use sp_std::vec::Vec;
    use sp_runtime::traits::StaticLookup;
    use cstrml_bridge as bridge;
    use crate::weights::WeightInfo;

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    /// Tracks current relayer set
    #[pallet::storage]
    #[pallet::getter(fn bridge_fee)]
    pub type BridgeFee<T: Config> =
        StorageMap<_, Blake2_256, u8, (BalanceOf<T>, u32), ValueQuery>;

    #[pallet::storage]
	#[pallet::getter(fn bridge_limit)]
	pub(super) type BridgeLimit<T: Config> = StorageValue<_, BalanceOf<T>, ValueQuery>;

    #[pallet::storage]
	#[pallet::getter(fn superior)]
	pub(super) type Superior<T: Config> = StorageValue<_, T::AccountId, OptionQuery>;

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

        type BridgeTransferWeightInfo: WeightInfo;
    }

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        FeeUpdated(u8, BalanceOf<T>, u32),
        /// Someone be the new superior
        SuperiorChanged(T::AccountId),
        /// Set limit successfully
        SetLimitSuccess(BalanceOf<T>),
    }

    // Errors inform users that something went wrong.
    #[pallet::error]
    pub enum Error<T> {
        InvalidTransfer,
		InvalidCommand,
		InvalidPayload,
		InvalidFeeOption,
		FeeOptionsMissiing,
		LessThanFee,
        /// Superior not exist, should set it first
        IllegalSuperior,
        ExceedBridgeLimit
    }

    // Dispatchable functions allows users to interact with the pallet and invoke state changes.
    // These functions materialize as "extrinsics", which are often compared to transactions.
    // Dispatchable functions must be annotated with a weight and must return a DispatchResult.
    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Change extra bridge transfer fee that user should pay
        #[pallet::call_index(0)]
		#[pallet::weight(T::BridgeTransferWeightInfo::default_bridge_transfer_weight())]
		pub fn sudo_change_fee(origin: OriginFor<T>, min_fee: BalanceOf<T>, fee_scale: u32, dest_id: u8) -> DispatchResult {
			ensure_root(origin)?;
			ensure!(fee_scale <= 1000u32, Error::<T>::InvalidFeeOption);
			BridgeFee::<T>::insert(dest_id, (min_fee, fee_scale));
			Self::deposit_event(Event::FeeUpdated(dest_id, min_fee, fee_scale));
			Ok(())
		}
        #[pallet::call_index(3)]
		#[pallet::weight(T::BridgeTransferWeightInfo::default_bridge_transfer_weight())]
        pub fn change_superior(origin: OriginFor<T>, new_superior: <T::Lookup as StaticLookup>::Source) -> DispatchResult {
            ensure_root(origin)?;

            let new_superior = T::Lookup::lookup(new_superior)?;

            Superior::<T>::put(new_superior.clone());

            Self::deposit_event(Event::SuperiorChanged(new_superior));

            Ok(())
        }
        #[pallet::call_index(4)]
		#[pallet::weight(T::BridgeTransferWeightInfo::default_bridge_transfer_weight())]
        pub fn set_bridge_limit(origin: OriginFor<T>, limit: BalanceOf<T>) -> DispatchResult {
            let signer = ensure_signed(origin)?;
            let maybe_superior = Self::superior();

            // 1. Check if superior exist
            ensure!(maybe_superior.is_some(), Error::<T>::IllegalSuperior);

            // 2. Check if signer is superior
            ensure!(Some(&signer) == maybe_superior.as_ref(), Error::<T>::IllegalSuperior);

            // 3. Set claim limit
            BridgeLimit::<T>::put(limit);

            Self::deposit_event(Event::SetLimitSuccess(limit));
            Ok(())
        }

		/// Transfers some amount of the native token to some recipient on a (whitelisted) destination chain.
        #[pallet::call_index(1)]
		#[pallet::weight(T::BridgeTransferWeightInfo::default_bridge_transfer_weight())]
		pub fn transfer_native(origin: OriginFor<T>, amount: BalanceOf<T>, recipient: Vec<u8>, dest_id: u8) -> DispatchResult {
			let source = ensure_signed(origin)?;
			ensure!(<bridge::Pallet<T>>::chain_whitelisted(dest_id), Error::<T>::InvalidTransfer);
			ensure!(BridgeFee::<T>::contains_key(&dest_id), Error::<T>::FeeOptionsMissiing);
			let (min_fee, fee_scale) = Self::bridge_fee(dest_id);
			let fee_estimated = amount * fee_scale.into() / 1000u32.into();
			let fee = if fee_estimated > min_fee {
				fee_estimated
			} else {
				min_fee
			};
			ensure!(amount > fee, Error::<T>::LessThanFee);
			let _ = T::Currency::withdraw(&source, amount.into(), WithdrawReasons::all(), AllowDeath)?;

			<bridge::Pallet<T>>::transfer_fungible(dest_id, T::BridgeTokenId::get(), recipient, U256::from(amount.saturating_sub(fee).saturated_into::<u128>()))
		}

		//
		// Executable calls. These can be triggered by a bridge transfer initiated on another chain
		//

		/// Executes a simple currency transfer using the bridge account as the source
        #[pallet::call_index(2)]
		#[pallet::weight(T::BridgeTransferWeightInfo::default_bridge_transfer_weight())]
		pub fn transfer(origin: OriginFor<T>, to: T::AccountId, amount: BalanceOf<T>, _rid: [u8; 32]) -> DispatchResult {
			let _source = T::BridgeOrigin::ensure_origin(origin)?;
            // 1. Check bridge limit
            ensure!(Self::bridge_limit() >= amount, Error::<T>::ExceedBridgeLimit);
            BridgeLimit::<T>::mutate(|l| *l = l.saturating_sub(amount));
			let _ = <T as Config>::Currency::deposit_creating(&to, amount.into());
			Ok(())
		}

    }
}

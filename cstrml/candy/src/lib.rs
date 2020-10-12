// Ensure we're `no_std` when compiling for Wasm.
#![cfg_attr(not(feature = "std"), no_std)]

use pallet_assets as assets;
use frame_support::{decl_module, dispatch};
use frame_system::ensure_root;

pub trait Trait: assets::Trait { }

decl_module! {
	pub struct Module<T: Trait> for enum Call where origin: T::Origin {

		#[weight = 1_000]
		pub fn issue(origin, #[compact] total: T::Balance) -> dispatch::DispatchResult {
			let _ = ensure_root(origin.clone())?;

		    Self::issue(origin, total)
		}


	}
}
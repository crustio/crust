/// A runtime module template with necessary imports

/// Feel free to remove or edit this file as needed.
/// If you change the name of this file, make sure to update its references in runtime/src/lib.rs
/// If you remove this file, you can remove those references


/// For more guidance on Substrate modules, see the example module
/// https://github.com/paritytech/substrate/blob/master/frame/example/src/lib.rs
use frame_support::{decl_module, decl_storage, decl_event, dispatch::DispatchResult};
use system::ensure_signed;
use sp_std::vec::Vec;
use crate::AccountId;

#[cfg(feature = "std")]
use serde::{self, Serialize, Deserialize};

#[cfg(feature = "std")]
#[derive(Serialize, Deserialize, Debug)]
struct TeeIdentity {
	pub_key: String,
	account_id: AccountId,
	validator_pub_key: String,
	validator_account_id: AccountId,
	sig: String,
}

/// The module's configuration trait.
pub trait Trait: system::Trait {
	/// The overarching event type.
	type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
}

// This module's storage items.
decl_storage! {
	trait Store for Module<T: Trait> as TeeModule {
		TeeIdentities get(tee_identities): map T::AccountId => Vec<u8>;
	}
}

// The module's dispatchable functions.
decl_module! {
	/// The module declaration.
	pub struct Module<T: Trait> for enum Call where origin: T::Origin {
		// Initializing events
		// this is needed only if you are using events in your module
		fn deposit_event() = default;

		pub fn store_tee_identity(origin, tee_identity: Vec<u8>) -> DispatchResult {
			let who = ensure_signed(origin)?;
			// TODO: add validation logic

			// let tee_identity_struct: TeeIdentity = serde_json::from_str(&from_utf8(&tee_identity).unwrap()).unwrap();

            // Store the tee identity
            TeeIdentities::<T>::insert(who.clone(), &tee_identity);

			Self::deposit_event(RawEvent::TeeIdentityStored(tee_identity, who));
			Ok(())
		}
	}
}

decl_event!(
	pub enum Event<T> where AccountId = <T as system::Trait>::AccountId {
		TeeIdentityStored(Vec<u8>, AccountId),
	}
);

/// tests for this module
#[cfg(test)]
mod tests {
	use super::*;

	use sp_core::H256;
	use frame_support::{impl_outer_origin, assert_ok, parameter_types, weights::Weight};
	use sp_runtime::{
		traits::{BlakeTwo256, IdentityLookup}, testing::Header, Perbill,
	};

	impl_outer_origin! {
		pub enum Origin for Test {}
	}

	// For testing the module, we construct most of a mock runtime. This means
	// first constructing a configuration type (`Test`) which `impl`s each of the
	// configuration traits of modules we want to use.
	#[derive(Clone, Eq, PartialEq)]
	pub struct Test;
	parameter_types! {
		pub const BlockHashCount: u64 = 250;
		pub const MaximumBlockWeight: Weight = 1024;
		pub const MaximumBlockLength: u32 = 2 * 1024;
		pub const AvailableBlockRatio: Perbill = Perbill::from_percent(75);
	}
	impl system::Trait for Test {
		type Origin = Origin;
		type Call = ();
		type Index = u64;
		type BlockNumber = u64;
		type Hash = H256;
		type Hashing = BlakeTwo256;
		type AccountId = u64;
		type Lookup = IdentityLookup<Self::AccountId>;
		type Header = Header;
		type Event = ();
		type BlockHashCount = BlockHashCount;
		type MaximumBlockWeight = MaximumBlockWeight;
		type MaximumBlockLength = MaximumBlockLength;
		type AvailableBlockRatio = AvailableBlockRatio;
		type Version = ();
		type ModuleToIndex = ();
	}
	impl Trait for Test {
		type Event = ();
	}
	type TeeModule = Module<Test>;

	// This function basically just builds a genesis storage key/value store according to
	// our desired mockup.
	fn new_test_ext() -> sp_io::TestExternalities {
		system::GenesisConfig::default().build_storage::<Test>().unwrap().into()
	}

	#[test]
	fn it_works_for_default_value() {
		new_test_ext().execute_with(|| {
			let test_tee_identity =
			"{\
			 \"pub_key\":\"pub\",\
			 \"account_id\":\"5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY\",\
			 \"validator_pub_key\":\"pub_v\",\
			 \"validator_account_id\":\"account_v\",\
			 \"sig\":\"sig\"\
			 }";
			assert_ok!(TeeModule::store_tee_identity(Origin::signed(1), test_tee_identity.as_bytes().to_vec()));
		});
	}
}

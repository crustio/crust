use frame_support::{decl_module, decl_storage, decl_event, ensure, dispatch::DispatchResult};
use system::ensure_signed;
use sp_std::vec::Vec;
use sp_std::str;
use codec::{Encode, Decode};

#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};

/// Define TEE basic elements
type PubKey = Vec<u8>;
type Signature = Vec<u8>;
type MerkleRoot = Vec<u8>;

// TODO: add timestamp
#[derive(Debug, PartialEq, Eq, Clone, Encode, Decode, Default)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct Identity<T> {
    pub_key: PubKey,
    account_id: T,
    validator_pub_key: PubKey,
    validator_account_id: T,
    sig: Signature,
}

// TODO: add timestamp
#[derive(Debug, PartialEq, Eq, Clone, Encode, Decode, Default)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct WorkReport{
	pub_key: PubKey,
	empty_root: MerkleRoot,
	workload: u64,
	sig: Signature,
}

/// The module's configuration trait.
pub trait Trait: system::Trait {
    /// The overarching event type.
    type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
}

// This module's storage items.
decl_storage! {
	trait Store for Module<T: Trait> as Tee {
		TeeIdentities get(tee_identities) config(): map T::AccountId => Option<Identity<T::AccountId>>;
		WorkReports get(work_reports): map T::AccountId => Option<WorkReport>;
	}
}

// The module's dispatchable functions.
decl_module! {
	/// The module declaration.
	pub struct Module<T: Trait> for enum Call where origin: T::Origin {
		// Initializing events
		// this is needed only if you are using events in your module
		fn deposit_event() = default;

		fn register_identity(origin, identity: Identity<T::AccountId>) -> DispatchResult {
		    let who = ensure_signed(origin)?;

            let applier = &identity.account_id;
            let validator = &identity.validator_account_id;

            // 1. TODO: Extract sig_hash from sig using v_pub_key
            // 2. TODO: Ensure identity report is legal

            // 3. Ensure who is applier
            ensure!(&who == applier, "Tee applier must be the extrinsic sender");

            // 4. applier cannot be validator
            ensure!(&applier != &validator, "You cannot verify yourself");

            // 5. If TeeIdentities contains v_account_id
            ensure!(<TeeIdentities<T>>::exists(validator), "Validator needs to be validated before");

            // 6. Applier is new add or needs to be updated
            if !<TeeIdentities<T>>::get(applier).contains(&identity) {
                // Store the tee identity
                <TeeIdentities<T>>::insert(applier, &identity);

                // Emit event
                Self::deposit_event(RawEvent::RegisterIdentity(who, identity));
            }

            Ok(())
		}

		fn report_works(origin, work_report: WorkReport) -> DispatchResult {
            let who = ensure_signed(origin)?;

            // 1. Ensure reporter is verified
            ensure!(<TeeIdentities<T>>::exists(&who), "Reporter must be registered before");

            // 2. Upsert works
            <WorkReports<T>>::insert(&who, &work_report);

            // 3. Emit event
            Self::deposit_event(RawEvent::ReportWorks(who, work_report));

            Ok(())
		}
	}
}

decl_event!(
	pub enum Event<T> where AccountId = <T as system::Trait>::AccountId {
		RegisterIdentity(AccountId, Identity<AccountId>),
		ReportWorks(AccountId, WorkReport),
	}
);

/// tests for this module
#[cfg(test)]
mod tests {
    use super::*;

    use sp_core::H256;
    use frame_support::{impl_outer_origin, assert_ok, parameter_types, weights::Weight};
    use sp_runtime::{
        traits::{BlakeTwo256, IdentityLookup}, testing::Header, Perbill
    };
    use sp_core::crypto::AccountId32;
    use keyring::Sr25519Keyring;

    type AccountId = AccountId32;

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
        type AccountId = AccountId;
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

    type Tee = Module<Test>;

    // This function basically just builds a genesis storage key/value store according to
    // our desired mockup.
    fn new_test_ext() -> sp_io::TestExternalities {
        let mut t = system::GenesisConfig::default().build_storage::<Test>().unwrap();
        let tee_ids = [
            Sr25519Keyring::Alice.to_account_id()
        ];

        GenesisConfig::<Test> {
            tee_identities: tee_ids
                .iter()
                .map(|x| (x.clone(), Default::default()))
                .collect()
        }.assimilate_storage(&mut t).unwrap();

        t.into()
    }

    #[test]
    fn test_for_register_identity_success() {
        new_test_ext().execute_with(|| {
            // Alice is validator in genesis block
            let applier: AccountId32 = Sr25519Keyring::Bob.to_account_id();
            let validator: AccountId32 = Sr25519Keyring::Alice.to_account_id();

            let id = Identity {
                pub_key: "pub_key_bob".as_bytes().to_vec(),
                account_id: applier.clone(),
                validator_pub_key: "pub_key_alice".as_bytes().to_vec(),
                validator_account_id: validator.clone(),
                sig: "sig_alice".as_bytes().to_vec()
            };

            assert_ok!(Tee::register_identity(Origin::signed(applier.clone()), id.clone()));

            let id_registered = Tee::tee_identities(applier.clone()).unwrap();

			assert_eq!(id.clone(), id_registered);
        });
    }

    #[test]
    fn test_for_register_identity_failed() {
        new_test_ext().execute_with(|| {
            // Bob is not validator before
            let account: AccountId32 = Sr25519Keyring::Bob.to_account_id();

            let id = Identity {
                pub_key: "pub_key_bob".as_bytes().to_vec(),
                account_id: account.clone(),
                validator_pub_key: "pub_key_bob".as_bytes().to_vec(),
                validator_account_id: account.clone(),
                sig: "sig_bob".as_bytes().to_vec()
            };

            assert!(Tee::register_identity(Origin::signed(account.clone()), id.clone()).is_err());
        });
    }

    #[test]
    fn test_for_register_identity_for_self() {
        new_test_ext().execute_with(|| {
            // Bob is not validator before
            let account: AccountId32 = Sr25519Keyring::Alice.to_account_id();

            let id = Identity {
                pub_key: "pub_key_self".as_bytes().to_vec(),
                account_id: account.clone(),
                validator_pub_key: "pub_key_self".as_bytes().to_vec(),
                validator_account_id: account.clone(),
                sig: "sig_self".as_bytes().to_vec()
            };

            assert!(Tee::register_identity(Origin::signed(account.clone()), id.clone()).is_err());
        });
    }

	#[test]
	fn test_for_report_works_success() {
		new_test_ext().execute_with(|| {
            let account: AccountId32 = Sr25519Keyring::Alice.to_account_id();

            let works = WorkReport {
                pub_key: "pub_key_alice".as_bytes().to_vec(),
                empty_root: "merkle_root_alice".as_bytes().to_vec(),
                workload: 1000,
                sig: "sig_key_alice".as_bytes().to_vec()
            };

			assert_ok!(Tee::report_works(Origin::signed(account), works));
		});
	}

    #[test]
    fn test_for_report_works_failed() {
        new_test_ext().execute_with(|| {
            let account: AccountId32 = Sr25519Keyring::Bob.to_account_id();

            let works = WorkReport {
                pub_key: "pub_key_bob".as_bytes().to_vec(),
                empty_root: "merkle_root_bob".as_bytes().to_vec(),
                workload: 2000,
                sig: "sig_key_bob".as_bytes().to_vec()
            };

            assert!(Tee::report_works(Origin::signed(account), works).is_err());
        });
    }
}

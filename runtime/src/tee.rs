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

            // 4. If TeeIdentities contains v_account_id
            ensure!(<TeeIdentities<T>>::exists(validator), "Validator needs to be validated before");

            // 5. Applier is new add or needs to be updated
            if !<TeeIdentities<T>>::exists(validator) || <TeeIdentities<T>>::get(validator).unwrap() != identity {
                // Store the tee identity
                <TeeIdentities<T>>::insert(validator, &identity);

                // Emit event
                Self::deposit_event(RawEvent::RegisterIdentity(who, identity));
            }

            Ok(())
		}

		fn report_works(origin, work_report: WorkReport) -> DispatchResult {
		    // TODO: add validation logic
            let who = ensure_signed(origin)?;


            if !WorkReports::<T>::exists(&who) || WorkReports::<T>::get(&who).unwrap() != work_report {
                // Store the tee identity
                <WorkReports<T>>::insert(&who, &work_report);

                // Emit event
                Self::deposit_event(RawEvent::ReportWorks(who, work_report));
            }

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
    fn test_for_store_tee_identity() {
        new_test_ext().execute_with(|| {
            let account: AccountId32 = Sr25519Keyring::Alice.to_account_id();

            let id = Identity {
                pub_key: "pub_key".as_bytes().to_vec(),
                account_id: account.clone(),
                validator_pub_key: "v_pub_key".as_bytes().to_vec(),
                validator_account_id: account.clone(),
                sig: "sig".as_bytes().to_vec()
            };

            assert_ok!(Tee::register_identity(Origin::signed(account.clone()), id.clone()));

            let id_registered = Tee::tee_identities(account.clone()).unwrap();

			assert_eq!(id.clone(), id_registered);
        });
    }

	/*#[test]
	fn test_for_store_tee_work_report() {
		new_test_ext().execute_with(|| {
			let work_report = "{\
			 \"pub_key\":\"pub\",\
			 \"empty_root\":\"XXXXXXXXXX\",\
			 \"workload\":1000000,\
			 \"sig\":\"sig\"\
			 }";
			assert_ok!(Tee::store_work_report(Origin::signed(1), work_report.as_bytes().to_vec()));
			let work_report_out = Tee::work_reports(1).unwrap();
			assert_eq!(work_report, sp_std::str::from_utf8(&work_report_out).unwrap());
		});
	}*/
}

use frame_support::{decl_module, decl_storage, decl_event, ensure, dispatch::{DispatchResult, DispatchError} };
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

#[derive(Debug, PartialEq, Eq, Clone, Encode, Decode, Default)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct Identity<T> {
    pub_key: PubKey,
    account_id: T,
    validator_pub_key: PubKey,
    validator_account_id: T,
    sig: Signature,
}

#[derive(Debug, PartialEq, Eq, Clone, Encode, Decode, Default)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct WorkReport{
    pub_key: PubKey,
    block_height: u64,
    block_hash: Vec<u8>,
	empty_root: MerkleRoot,
    empty_workload: u64,
    meaningful_workload: u64,
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
            let applier_pk = &identity.pub_key;
            let validator_pr = &identity.validator_pub_key;

            // 1. Ensure who is applier
            ensure!(&who == applier, "Tee applier must be the extrinsic sender");

            // 2. applier cannot be validator
            ensure!(&applier != &validator, "You cannot verify yourself");
            ensure!(&applier_pk != &validator_pr, "You cannot verify yourself");

            // 3. v_account_id should been validated before
            ensure!(<TeeIdentities<T>>::exists(validator), "Validator needs to be validated before");

            // 4. Judge identity sig is legal
            let is_identity_legal = Self::is_identity_legal(&identity)?;

            // 5. Ensure sig_hash == report_hash
            ensure!(is_identity_legal, "Tee report signature is illegal");

            // 6. applier is new add or needs to be updated
            if !<TeeIdentities<T>>::get(applier).contains(&identity) {
                // Store the tee identity
                <TeeIdentities<T>>::insert(applier, &identity);

                // Emit event
                Self::deposit_event(RawEvent::RegisterIdentity(who, identity));
            }

            Ok(())
		}

		fn report_works(origin, work_report: WorkReport) -> DispatchResult {
		    // TODO: 1. validate block information to determine real-time report
		    // TODO: 2. Tee applier must be the extrinsic sender (can find public key, which is used in 3)
		    // TODO: 3. validate public key to determine identity Information
		    // TODO: 4. validate signature
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

impl<T: Trait> Module<T> {
    pub fn is_identity_legal(id: &Identity<T::AccountId>) -> Result<bool, DispatchError> {
        // 1. Transfer 128 {pub_key, sig} bytes into 64 bytes
        let applier_pk = &id.pub_key;
        let validator_pk = &id.validator_pub_key;
        let id_sig = &id.sig;

        // 2. Change account_id into byte array
        let applier_id = &id.account_id.encode();
        let validator_id = &id.validator_account_id.encode();

        // 3. Concat identity byte arrays by defined sequence
        // {
        //    pub_key: PubKey,
        //    account_id: T,
        //    validator_pub_key: PubKey,
        //    validator_account_id: T
        // }
        let data = [&applier_pk[..], &applier_id[..], &validator_pk[..], &validator_id[..]].concat();

        // 4. Construct ecdsa Signature
        //let ecdsa_sig = EcdsaSig::from_der(id_sig).unwrap();
        //let ecdsa_pk =

        Ok(true)
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
    use sp_core::crypto::{AccountId32, Ss58Codec};
    use keyring::Sr25519Keyring;
    use hex;
    use std::vec::Vec;
    use signatory_ring::ecdsa::p256::{PublicKey, Verifier};
    use signatory::{
        ecdsa::curve::nistp256::FixedSignature,
        signature::{Signature as _, Verifier as _},
    };

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
    fn test_for_verify_sig_success() {
        new_test_ext().execute_with(|| {
            // Alice is validator in genesis block
            let applier: AccountId = AccountId::from_ss58check("5FqazaU79hjpEMiWTWZx81VjsYFst15eBuSBKdQLgQibD7CX").expect("valid ss58 address");
            let validator: AccountId = Sr25519Keyring::Alice.to_account_id();

            let pk = hex::decode("aa57e79045d6f24c74eb8f99e35f45c836f374860cf\
            20bb2014d98b2f7318a7034f345951d988ca7761c8111a22c150df89429a074eb661aa20ae7b4ee52ec01").expect("Invalid hex");
            let sig = hex::decode("8583b094b65946adc31d28b77bb681cbeccf46d529f4\
            8941b9d595076515c84bc3e15d5a43cbc0a63dabd30351d9e9d60a875d7739b78b7ed98db09895b0652c").expect("Invalid hex");

            let id = Identity {
                pub_key: pk.clone(),
                account_id: applier.clone(),
                validator_pub_key: pk.clone(),
                validator_account_id: validator.clone(),
                sig: sig.clone()
            };

            /*let v128_to_v64 = |v128: &Vec<u8>| -> Vec<u8> {
                let v128_hex_str = str::from_utf8(v128).expect("128 bytes array cannot convert into hex string");
                hex::decode(v128_hex_str).expect("hex string cannot convert into 64 bytes array")
            };*/

            // For test
            // 1.
            let mut applier_pk: Vec<u8> = vec![4];
            applier_pk.extend(&id.pub_key);

            let mut validator_pk: Vec<u8> = vec![4];
            validator_pk.extend(&id.validator_pub_key);

            let sig_id= &id.sig;

            // 2. Change account_id into byte array
            let applier_id = &id.account_id.to_ss58check().as_bytes().to_vec();
            let validator_id = &id.validator_account_id.to_ss58check().as_bytes().to_vec();

            // 3. Concat identity byte arrays by defined sequence
            // {
            //    pub_key: PubKey,
            //    account_id: T,
            //    validator_pub_key: PubKey,
            //    validator_account_id: T
            // }
            let data: Vec<u8> = [&applier_pk[..], &applier_id[..], &validator_pk[..], &validator_id[..]].concat();

            // 4. Construct sig and pub_key
            /*let mut pk65 = vec![4];
            pk65.extend(validator_pk);*/

            let p256_pk = PublicKey::from_bytes(&validator_pk).expect("public key illegal");
            let p256_sig = FixedSignature::from_bytes(sig_id.as_slice()).expect("sig illegal");

            let p256_v = Verifier::from(&p256_pk);

            let rst = p256_v.verify(data.as_slice(), &p256_sig);

            assert!(rst.is_ok());

            assert!(Tee::is_identity_legal(&id).unwrap());
        });
    }

	#[test]
	fn test_for_report_works_success() {
		new_test_ext().execute_with(|| {
            let account: AccountId32 = Sr25519Keyring::Alice.to_account_id();

            let works = WorkReport {
                pub_key: "pub_key_alice".as_bytes().to_vec(),
                block_height: 50,
                block_hash: "block_hash".as_bytes().to_vec(),
                empty_root: "merkle_root_alice".as_bytes().to_vec(),
                empty_workload: 1000,
                meaningful_workload: 1000,
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
                block_height: 50,
                block_hash: "block_hash".as_bytes().to_vec(),
                empty_root: "merkle_root_bob".as_bytes().to_vec(),
                empty_workload: 2000,
                meaningful_workload: 2000,
                sig: "sig_key_bob".as_bytes().to_vec()
            };

            assert!(Tee::report_works(Origin::signed(account), works).is_err());
        });
    }
}

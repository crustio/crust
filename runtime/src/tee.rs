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
    use signatory_ring::ecdsa::p256::{PublicKey, Verifier, Signer};
    use signatory::{
        ecdsa::curve::nistp256::{FixedSignature, Asn1Signature},
        ecdsa::generic_array::GenericArray,
        signature::{Signature as _, Signer as _, Verifier as _},
        public_key::PublicKeyed,
        encoding::FromPkcs8,
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
            let applier: AccountId = AccountId::from_ss58check("5Cowt7B9CbBa3CffyusJTCuhT33WcwpqRoULdSQwwmKHNRW2").expect("valid ss58 address");
            let validator: AccountId = AccountId::from_ss58check("5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY").expect("valid ss58 address");

            let mut pk = hex::decode("cca98ffd68a64b2453749d23e57fee0a4ff843e2525377a5b6d18012885a2be09343a006d85151f519ff56aaacb73d27e01b73938bf408935c1ab45207657dcf").expect("Invalid hex");
            // from/to little/big endian

           /* let mut ppk = hex::decode("118ced2248d5a29743fc0725dc5f0f493a47486e9a660958d4db71d0e4de0bd5").expect("invalid hex string");
            ppk.reverse();*/

            let mut sig= hex::decode("0c4ed44eb7664137247e8e51a928802bbb9fcf26cb150b342e07ffc20efe679d1ac5c8e940df6640515c596b4e7195a5b5c90c9aeba34ea472002f02a69f7a49").expect("Invalid hex");
            sig[0..32].reverse();
            sig[32..].reverse();

            let id = Identity {
                pub_key: pk.clone(),
                account_id: applier.clone(),
                validator_pub_key: pk.clone(),
                validator_account_id: validator.clone(),
                sig: sig.clone()
            };

            // For test
            // 1.
            let applier_pk = hex::encode(&id.pub_key);
            let validator_pk = hex::encode(&id.validator_pub_key);
            let sig_id= &id.sig;

            // 2. Change account_id into byte array
            let applier_id = &id.account_id.to_ss58check();
            let validator_id = &id.validator_account_id.to_ss58check();

            // 3. Concat identity byte arrays by defined sequence
            // {
            //    pub_key: PubKey,
            //    account_id: T,
            //    validator_pub_key: PubKey,
            //    validator_account_id: T
            // }
            //let data: Vec<u8> = [&applier_pk[..], &applier_id[..], &validator_pk[..], &validator_id[..]].concat();
            let data_raw = format!("{}{}{}{}", applier_pk, applier_id, validator_pk, validator_id);
            let data = data_raw.as_bytes().to_vec();

            // 4. Construct sig and pub_key

            /// PKCS#8 header for a NIST P-256 private key
            /*const P256_PKCS8_HEADER: &[u8] = b"\x30\x81\x87\x02\x01\x00\x30\x13\x06\x07\x2a\x86\x48\xce\x3d\x02\x01\x06\
            \x08\x2a\x86\x48\xce\x3d\x03\x01\x07\x04\x6d\x30\x6b\x02\x01\x01\x04\x20";

            /// PKCS#8 interstitial part for a NIST P-256 private key
            const P256_PKCS8_PUBKEY_PREFIX: &[u8] = b"\xa1\x44\x03\x42\x00\x04";

            let to_pkcs8 = |s: &Vec<u8>, p: &Vec<u8>| -> Vec<u8> {
                // TODO: better serializer than giant hardcoded bytestring literals, like a PKCS#8 library,
                // or at least a less bogus internal PKCS#8 implementation
                let mut pkcs8_document = P256_PKCS8_HEADER.to_vec();

                pkcs8_document.extend_from_slice(s);
                pkcs8_document.extend_from_slice(P256_PKCS8_PUBKEY_PREFIX);
                pkcs8_document.extend_from_slice(p);

                pkcs8_document
            };*/

            pk[0..32].reverse();
            pk[32..].reverse();

            let p256_pk = PublicKey::from_untagged_point(&GenericArray::from_slice(&pk));
            /*let signer: Signer<FixedSignature> = Signer::from_pkcs8(to_pkcs8(&ppk, &pk)).expect("invalid pk");

            assert_eq!(signer.public_key().unwrap(), p256_pk);*/

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

#![cfg_attr(not(feature = "std"), no_std)]

use sp_runtime_interface::runtime_interface;
use sp_std::vec::Vec;

#[cfg(feature = "std")]
use signatory_ring::ecdsa::p256::{PublicKey, Verifier};
#[cfg(feature = "std")]
use signatory::{
    ecdsa::curve::nistp256::FixedSignature,
    ecdsa::generic_array::GenericArray,
    signature::{Signature as _, Verifier as _}
};
#[cfg(feature = "std")]
use hex;

#[runtime_interface]
pub trait Crypto {
    fn verify_identity(applier_pk: &Vec<u8>, validator_pk: &Vec<u8>, raw_sig: &Vec<u8>) -> bool {
        // 1. Encode public key and sig
        let mut pk = applier_pk.clone();
        let mut sig = raw_sig.clone();

        // 2. Change account_id into byte array
        // TODO: [HARD CODE!]change to AccountId Byte
        let applier_id = "5Cowt7B9CbBa3CffyusJTCuhT33WcwpqRoULdSQwwmKHNRW2";
        let validator_id = "5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY";

        // 3. Construct identity data
        // {
        //    pub_key: PubKey,
        //    account_id: T,
        //    validator_pub_key: PubKey,
        //    validator_account_id: T
        // }
        let applier_pk_str = hex::encode(&applier_pk);
        let validator_pk_str = hex::encode(&validator_pk);
        let data_raw = format!("{}{}{}{}", applier_pk_str, applier_id, validator_pk_str, validator_id);
        let data = data_raw.as_bytes().to_vec();

        // 4. le/be convert
        pk[0..32].reverse();
        pk[32..].reverse();

        sig[0..32].reverse();
        sig[32..].reverse();

        // 5. Construct public key and signature
        let p256_pk = PublicKey::from_untagged_point(&GenericArray::from_slice(&pk));
        let p256_sig = FixedSignature::from_bytes(sig.as_slice()).expect("sig illegal");

        // 6. Do verify
        let p256_v = Verifier::from(&p256_pk);
        let result = p256_v.verify(data.as_slice(), &p256_sig);

        match result {
            Ok(_) => true,
            Err(_) => false,
        }
    }
}

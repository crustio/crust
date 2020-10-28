#![cfg_attr(not(feature = "std"), no_std)]
use sp_runtime_interface::runtime_interface;
use sp_std::vec::Vec;

#[cfg(feature = "std")]
use signatory::{
    ecdsa::curve::nistp256::FixedSignature,
    ecdsa::generic_array::GenericArray,
    signature::{Signature as _, Verifier as _},
};
#[cfg(feature = "std")]
use signatory_ring::ecdsa::p256::{PublicKey, Verifier};

#[runtime_interface]
pub trait Crypto {
    fn verify_p256_sig(be_pk: &Vec<u8>, data: &Vec<u8>, be_sig: &Vec<u8>) -> bool {
        // 1. le/be convert
        let mut pk = be_pk.clone();
        let mut sig = be_sig.clone();

        pk[0..32].reverse();
        pk[32..].reverse();

        sig[0..32].reverse();
        sig[32..].reverse();

        // 2. Construct public key and signature
        let p256_pk = PublicKey::from_untagged_point(&GenericArray::from_slice(&pk));
        let p256_sig = match FixedSignature::from_bytes(sig.as_slice()) {
            Ok(sig) => sig,
            Err(_) => return false,
        };

        // 3. Do verify
        let p256_v = Verifier::from(&p256_pk);
        let result = p256_v.verify(data.as_slice(), &p256_sig);

        result.is_ok()
    }
    // TODO: use wasm version or (p256 -> scep256k1)
}

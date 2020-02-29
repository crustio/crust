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
        let p256_sig = FixedSignature::from_bytes(sig.as_slice()).expect("sig illegal");

        // 3. Do verify
        let p256_v = Verifier::from(&p256_pk);
        let result = p256_v.verify(data.as_slice(), &p256_sig);

        match result {
            Ok(_) => true,
            Err(_) => false,
        }
    }

    fn verify_identity_sig(
        applier_pk: &Vec<u8>,
        applier_id: &Vec<u8>,
        validator_pk: &Vec<u8>,
        validator_id: &Vec<u8>,
        sig: &Vec<u8>,
    ) -> bool {
        // 1. Construct identity data
        // {
        //    pub_key: PubKey,
        //    account_id: T,
        //    validator_pub_key: PubKey,
        //    validator_account_id: T
        // }
        let data: Vec<u8> = [
            &applier_pk[..],
            &applier_id[..],
            &validator_pk[..],
            &validator_id[..],
        ]
        .concat();

        // 2. do p256 sig check
        Self::verify_p256_sig(validator_pk, &data, sig)
    }

    fn verify_work_report_sig(
        pk: &Vec<u8>,
        bn: u64,
        block_hash: &Vec<u8>,
        empty_root: &Vec<u8>,
        ew: u64,
        mw: u64,
        sig: &Vec<u8>,
    ) -> bool {
        // 1. Encode u64
        let block_number = bn.to_string().as_bytes().to_vec();
        let empty_workload = ew.to_string().as_bytes().to_vec();
        let meaningful_workload = mw.to_string().as_bytes().to_vec();

        // 2. Construct identity data
        //{
        //    pub_key: PubKey,
        //    block_number: u64,
        //    block_hash: Vec<u8>,
        //    empty_root: MerkleRoot,
        //    empty_workload: u64,
        //    meaningful_workload: u64
        //}
        let data: Vec<u8> = [
            &pk[..],
            &block_number[..],
            &block_hash[..],
            &empty_root[..],
            &empty_workload[..],
            &meaningful_workload[..],
        ]
        .concat();

        // 3. do p256 sig check
        Self::verify_p256_sig(pk, &data, sig)
    }
}

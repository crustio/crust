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
use primitives::{
    SworkerSignature,
    IASSig, SworkerCert, ISVBody, SworkerCode
};

#[cfg(feature = "std")]
use openssl::{
    x509::X509,
    sign::Verifier as CAVerifier,
    hash::MessageDigest
};

#[cfg(feature = "std")]
use serde_json::{Result as JsonResult, Value};

#[runtime_interface]
pub trait Crypto {
    fn verify_identity(
        ias_sig: &IASSig,
        ias_cert: &SworkerCert,
        account_id: &Vec<u8>,
        isv_body: &ISVBody,
        sig: &SworkerSignature,
        enclave_code: &SworkerCode
    ) -> Option<Vec<u8>> {
        // 0. Define this fucking big root certificate💩
        let root_ca: Vec<u8> = "-----BEGIN CERTIFICATE-----
MIIFSzCCA7OgAwIBAgIJANEHdl0yo7CUMA0GCSqGSIb3DQEBCwUAMH4xCzAJBgNV
BAYTAlVTMQswCQYDVQQIDAJDQTEUMBIGA1UEBwwLU2FudGEgQ2xhcmExGjAYBgNV
BAoMEUludGVsIENvcnBvcmF0aW9uMTAwLgYDVQQDDCdJbnRlbCBTR1ggQXR0ZXN0
YXRpb24gUmVwb3J0IFNpZ25pbmcgQ0EwIBcNMTYxMTE0MTUzNzMxWhgPMjA0OTEy
MzEyMzU5NTlaMH4xCzAJBgNVBAYTAlVTMQswCQYDVQQIDAJDQTEUMBIGA1UEBwwL
U2FudGEgQ2xhcmExGjAYBgNVBAoMEUludGVsIENvcnBvcmF0aW9uMTAwLgYDVQQD
DCdJbnRlbCBTR1ggQXR0ZXN0YXRpb24gUmVwb3J0IFNpZ25pbmcgQ0EwggGiMA0G
CSqGSIb3DQEBAQUAA4IBjwAwggGKAoIBgQCfPGR+tXc8u1EtJzLA10Feu1Wg+p7e
LmSRmeaCHbkQ1TF3Nwl3RmpqXkeGzNLd69QUnWovYyVSndEMyYc3sHecGgfinEeh
rgBJSEdsSJ9FpaFdesjsxqzGRa20PYdnnfWcCTvFoulpbFR4VBuXnnVLVzkUvlXT
L/TAnd8nIZk0zZkFJ7P5LtePvykkar7LcSQO85wtcQe0R1Raf/sQ6wYKaKmFgCGe
NpEJUmg4ktal4qgIAxk+QHUxQE42sxViN5mqglB0QJdUot/o9a/V/mMeH8KvOAiQ
byinkNndn+Bgk5sSV5DFgF0DffVqmVMblt5p3jPtImzBIH0QQrXJq39AT8cRwP5H
afuVeLHcDsRp6hol4P+ZFIhu8mmbI1u0hH3W/0C2BuYXB5PC+5izFFh/nP0lc2Lf
6rELO9LZdnOhpL1ExFOq9H/B8tPQ84T3Sgb4nAifDabNt/zu6MmCGo5U8lwEFtGM
RoOaX4AS+909x00lYnmtwsDVWv9vBiJCXRsCAwEAAaOByTCBxjBgBgNVHR8EWTBX
MFWgU6BRhk9odHRwOi8vdHJ1c3RlZHNlcnZpY2VzLmludGVsLmNvbS9jb250ZW50
L0NSTC9TR1gvQXR0ZXN0YXRpb25SZXBvcnRTaWduaW5nQ0EuY3JsMB0GA1UdDgQW
BBR4Q3t2pn680K9+QjfrNXw7hwFRPDAfBgNVHSMEGDAWgBR4Q3t2pn680K9+Qjfr
NXw7hwFRPDAOBgNVHQ8BAf8EBAMCAQYwEgYDVR0TAQH/BAgwBgEB/wIBADANBgkq
hkiG9w0BAQsFAAOCAYEAeF8tYMXICvQqeXYQITkV2oLJsp6J4JAqJabHWxYJHGir
IEqucRiJSSx+HjIJEUVaj8E0QjEud6Y5lNmXlcjqRXaCPOqK0eGRz6hi+ripMtPZ
sFNaBwLQVV905SDjAzDzNIDnrcnXyB4gcDFCvwDFKKgLRjOB/WAqgscDUoGq5ZVi
zLUzTqiQPmULAQaB9c6Oti6snEFJiCQ67JLyW/E83/frzCmO5Ru6WjU4tmsmy8Ra
Ud4APK0wZTGtfPXU7w+IBdG5Ez0kE1qzxGQaL4gINJ1zMyleDnbuS8UicjJijvqA
152Sq049ESDz+1rRGc2NVEqh1KaGXmtXvqxXcTB+Ljy5Bw2ke0v8iGngFBPqCTVB
3op5KBG3RjbF6RRSzwzuWfL7QErNC8WEy5yDVARzTA5+xmBc388v9Dm21HGfcC8O
DD+gT9sSpssq0ascmvH49MOgjt1yoysLtdCtJW/9FZpoOypaHx0R+mJTLwPXVMrv
DaVzWh5aiEx+idkSGMnX
-----END CERTIFICATE-----".as_bytes().to_vec();

        // 1. Construct ias_sig, (root+temp) cert and ias_sig's public key
        let decoded_ias_sig = match base64::decode(ias_sig) {
            Ok(sig) => sig,
            Err(_) => return None,
        };
        let root_ca = match X509::from_pem(root_ca.as_slice()) {
            Ok(ca) => ca,
            Err(_) => return None,
        };
        let temp_ca = match X509::from_pem(ias_cert.as_slice()) {
            Ok(ca) => ca,
            Err(_) => return None,
        };
        let ca_pk: openssl::pkey::PKey<openssl::pkey::Public> = match temp_ca.public_key() {
            Ok(pk) => pk,
            Err(_) => return None,
        };

        // 2. Verify CA chain
        let is_legal_ca = root_ca.issued(&temp_ca).as_raw() == 0;
        if !is_legal_ca {
            return None;
        }

        // 3. Verify ISV body
        let mut verifier = CAVerifier::new(MessageDigest::sha256(), &ca_pk).unwrap();
        let _ = verifier.update(isv_body.as_slice());
        let is_legal_body = match verifier.verify(decoded_ias_sig.as_slice()) {
            Ok(verify_rst) => verify_rst,
            Err(_) => return None
        };
        if !is_legal_body {
            return None;
        }

        // 4. Get ISV public key
        if let Some(quote_body) = get_isv_quote_body(&isv_body) {
            // 5. Verify sig: {ias_cert + (undecode)sig + isv_body + account_id}
            let decoded_quote_body = match base64::decode(quote_body) {
                Ok(decoded_qb) => decoded_qb,
                Err(_) => return None,
            };
            let id_code = &decoded_quote_body[112..144].to_vec();

            // 6. Verify enclave code
            if id_code != enclave_code {
                return None;
            }

            // 7. Get public key and decode account id
            let pk = &decoded_quote_body[368..].to_vec();

            let data: Vec<u8> = [
                &ias_cert[..],
                &ias_sig[..],
                &isv_body[..],
                &account_id[..],
            ].concat();

            // 8. Verify signature
            let is_legal_sig = verify_p256_sig(&pk, &data, &sig);

            if !is_legal_sig {
                return None;
            }

            return Some(pk.clone())
        }

        None
    }

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

    fn get_isv_quote_body(body: &Vec<u8>) -> Option<Vec<u8>> {
        let maybe_isv_body: JsonResult<Value> = serde_json::from_slice(body);
        if maybe_isv_body.is_ok() {
            let isv_body = maybe_isv_body.unwrap();
            let maybe_isv_quote_body = &isv_body["isvEnclaveQuoteBody"];
            if maybe_isv_quote_body.is_string() {
                return Some(maybe_isv_quote_body.as_str().unwrap().as_bytes().to_vec())
            }
        }
        None
    }
    // ✅ 1. use wasm version, (p256 -> scep256k1 | x509 -> wasm version)
    // 2. offchain_worker? forkless upgrade?
}

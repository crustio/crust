use sp_std::prelude::*;
use primitives::{
    SworkerSignature,
    IASSig, SworkerCert, ISVBody, SworkerCode,
    SworkerPubKey
};
use serde_json::Value;
use p256::ecdsa::{VerifyKey, signature::{Verifier, Signature}};

pub static IAS_SERVER_ROOTS: webpki::TLSServerTrustAnchors = webpki::TLSServerTrustAnchors(&[
    /*
     * -----BEGIN CERTIFICATE-----
     * MIIFSzCCA7OgAwIBAgIJANEHdl0yo7CUMA0GCSqGSIb3DQEBCwUAMH4xCzAJBgNV
     * BAYTAlVTMQswCQYDVQQIDAJDQTEUMBIGA1UEBwwLU2FudGEgQ2xhcmExGjAYBgNV
     * BAoMEUludGVsIENvcnBvcmF0aW9uMTAwLgYDVQQDDCdJbnRlbCBTR1ggQXR0ZXN0
     * YXRpb24gUmVwb3J0IFNpZ25pbmcgQ0EwIBcNMTYxMTE0MTUzNzMxWhgPMjA0OTEy
     * MzEyMzU5NTlaMH4xCzAJBgNVBAYTAlVTMQswCQYDVQQIDAJDQTEUMBIGA1UEBwwL
     * U2FudGEgQ2xhcmExGjAYBgNVBAoMEUludGVsIENvcnBvcmF0aW9uMTAwLgYDVQQD
     * DCdJbnRlbCBTR1ggQXR0ZXN0YXRpb24gUmVwb3J0IFNpZ25pbmcgQ0EwggGiMA0G
     * CSqGSIb3DQEBAQUAA4IBjwAwggGKAoIBgQCfPGR+tXc8u1EtJzLA10Feu1Wg+p7e
     * LmSRmeaCHbkQ1TF3Nwl3RmpqXkeGzNLd69QUnWovYyVSndEMyYc3sHecGgfinEeh
     * rgBJSEdsSJ9FpaFdesjsxqzGRa20PYdnnfWcCTvFoulpbFR4VBuXnnVLVzkUvlXT
     * L/TAnd8nIZk0zZkFJ7P5LtePvykkar7LcSQO85wtcQe0R1Raf/sQ6wYKaKmFgCGe
     * NpEJUmg4ktal4qgIAxk+QHUxQE42sxViN5mqglB0QJdUot/o9a/V/mMeH8KvOAiQ
     * byinkNndn+Bgk5sSV5DFgF0DffVqmVMblt5p3jPtImzBIH0QQrXJq39AT8cRwP5H
     * afuVeLHcDsRp6hol4P+ZFIhu8mmbI1u0hH3W/0C2BuYXB5PC+5izFFh/nP0lc2Lf
     * 6rELO9LZdnOhpL1ExFOq9H/B8tPQ84T3Sgb4nAifDabNt/zu6MmCGo5U8lwEFtGM
     * RoOaX4AS+909x00lYnmtwsDVWv9vBiJCXRsCAwEAAaOByTCBxjBgBgNVHR8EWTBX
     * MFWgU6BRhk9odHRwOi8vdHJ1c3RlZHNlcnZpY2VzLmludGVsLmNvbS9jb250ZW50
     * L0NSTC9TR1gvQXR0ZXN0YXRpb25SZXBvcnRTaWduaW5nQ0EuY3JsMB0GA1UdDgQW
     * BBR4Q3t2pn680K9+QjfrNXw7hwFRPDAfBgNVHSMEGDAWgBR4Q3t2pn680K9+Qjfr
     * NXw7hwFRPDAOBgNVHQ8BAf8EBAMCAQYwEgYDVR0TAQH/BAgwBgEB/wIBADANBgkq
     * hkiG9w0BAQsFAAOCAYEAeF8tYMXICvQqeXYQITkV2oLJsp6J4JAqJabHWxYJHGir
     * IEqucRiJSSx+HjIJEUVaj8E0QjEud6Y5lNmXlcjqRXaCPOqK0eGRz6hi+ripMtPZ
     * sFNaBwLQVV905SDjAzDzNIDnrcnXyB4gcDFCvwDFKKgLRjOB/WAqgscDUoGq5ZVi
     * zLUzTqiQPmULAQaB9c6Oti6snEFJiCQ67JLyW/E83/frzCmO5Ru6WjU4tmsmy8Ra
     * Ud4APK0wZTGtfPXU7w+IBdG5Ez0kE1qzxGQaL4gINJ1zMyleDnbuS8UicjJijvqA
     * 152Sq049ESDz+1rRGc2NVEqh1KaGXmtXvqxXcTB+Ljy5Bw2ke0v8iGngFBPqCTVB
     * 3op5KBG3RjbF6RRSzwzuWfL7QErNC8WEy5yDVARzTA5+xmBc388v9Dm21HGfcC8O
     * DD+gT9sSpssq0ascmvH49MOgjt1yoysLtdCtJW/9FZpoOypaHx0R+mJTLwPXVMrv
     * DaVzWh5aiEx+idkSGMnX
     * -----END CERTIFICATE-----
     */
    webpki::TrustAnchor {
        subject: b"1\x0b0\t\x06\x03U\x04\x06\x13\x02US1\x0b0\t\x06\x03U\x04\x08\x0c\x02CA1\x140\x12\x06\x03U\x04\x07\x0c\x0bSanta Clara1\x1a0\x18\x06\x03U\x04\n\x0c\x11Intel Corporation100.\x06\x03U\x04\x03\x0c\'Intel SGX Attestation Report Signing CA",
        spki: b"0\r\x06\t*\x86H\x86\xf7\r\x01\x01\x01\x05\x00\x03\x82\x01\x8f\x000\x82\x01\x8a\x02\x82\x01\x81\x00\x9f<d~\xb5w<\xbbQ-\'2\xc0\xd7A^\xbbU\xa0\xfa\x9e\xde.d\x91\x99\xe6\x82\x1d\xb9\x10\xd51w7\twFjj^G\x86\xcc\xd2\xdd\xeb\xd4\x14\x9dj/c%R\x9d\xd1\x0c\xc9\x877\xb0w\x9c\x1a\x07\xe2\x9cG\xa1\xae\x00IHGlH\x9fE\xa5\xa1]z\xc8\xec\xc6\xac\xc6E\xad\xb4=\x87g\x9d\xf5\x9c\t;\xc5\xa2\xe9ilTxT\x1b\x97\x9euKW9\x14\xbeU\xd3/\xf4\xc0\x9d\xdf\'!\x994\xcd\x99\x05\'\xb3\xf9.\xd7\x8f\xbf)$j\xbe\xcbq$\x0e\xf3\x9c-q\x07\xb4GTZ\x7f\xfb\x10\xeb\x06\nh\xa9\x85\x80!\x9e6\x91\tRh8\x92\xd6\xa5\xe2\xa8\x08\x03\x19>@u1@N6\xb3\x15b7\x99\xaa\x82Pt@\x97T\xa2\xdf\xe8\xf5\xaf\xd5\xfec\x1e\x1f\xc2\xaf8\x08\x90o(\xa7\x90\xd9\xdd\x9f\xe0`\x93\x9b\x12W\x90\xc5\x80]\x03}\xf5j\x99S\x1b\x96\xdei\xde3\xed\"l\xc1 }\x10B\xb5\xc9\xab\x7f@O\xc7\x11\xc0\xfeGi\xfb\x95x\xb1\xdc\x0e\xc4i\xea\x1a%\xe0\xff\x99\x14\x88n\xf2i\x9b#[\xb4\x84}\xd6\xff@\xb6\x06\xe6\x17\x07\x93\xc2\xfb\x98\xb3\x14X\x7f\x9c\xfd%sb\xdf\xea\xb1\x0b;\xd2\xd9vs\xa1\xa4\xbdD\xc4S\xaa\xf4\x7f\xc1\xf2\xd3\xd0\xf3\x84\xf7J\x06\xf8\x9c\x08\x9f\r\xa6\xcd\xb7\xfc\xee\xe8\xc9\x82\x1a\x8eT\xf2\\\x04\x16\xd1\x8cF\x83\x9a_\x80\x12\xfb\xdd=\xc7M%by\xad\xc2\xc0\xd5Z\xffo\x06\"B]\x1b\x02\x03\x01\x00\x01",
        name_constraints: None
    },
]);

type SignatureAlgorithms = &'static [&'static webpki::SignatureAlgorithm];
static SUPPORTED_SIG_ALGS: SignatureAlgorithms = &[
    &webpki::RSA_PKCS1_2048_8192_SHA256,
    &webpki::RSA_PKCS1_2048_8192_SHA384,
    &webpki::RSA_PKCS1_2048_8192_SHA512,
    &webpki::RSA_PKCS1_3072_8192_SHA384,
];

pub fn verify_identity (
    ias_sig: &IASSig,
    ias_cert: &SworkerCert,
    account_id: &Vec<u8>,
    isv_body: &ISVBody,
    ab_upgrade_pk: &SworkerPubKey,
    sig: &SworkerSignature,
    enclave_code: &SworkerCode
) -> Option<Vec<u8>> {
    // 1. Decode ias cert from base64
    let ias_cert_dec = match base64::decode_config(&ias_cert, base64::STANDARD) {
        Ok(c) => c,
        Err(_) => return None,
    };
    let sig_cert: webpki::EndEntityCert = match webpki::EndEntityCert::from(&ias_cert_dec) {
        Ok(c) => c,
        Err(_) => return None,
    };

    let intermediate_certs: Vec<&[u8]> = Vec::new();
    let now_func = webpki::Time::from_seconds_since_unix_epoch(1603843200); // 2020-10-28 12:00:00 (UTC)

    // 2. Verify ias cert
    match sig_cert.verify_is_valid_tls_server_cert(
        SUPPORTED_SIG_ALGS,
        &IAS_SERVER_ROOTS,
        &intermediate_certs,
        now_func
    ) {
        Ok(()) => {},
        Err(_e) => return None,
    };

    let ias_sig_dec: Vec<u8> = match base64::decode(ias_sig) {
        Ok(x) => x,
        Err(_) => panic!("decode sig failed")
    };

    // 3. Verify isv body signature
    match sig_cert.verify_signature(
        &webpki::RSA_PKCS1_2048_8192_SHA256,
        isv_body,
        &ias_sig_dec
    ) {
        Ok(()) => {},
        Err(_e) => return None,
    };

    // 4. Parse isv body
    let maybe_isv_body: Value = match serde_json::from_slice(isv_body) {
        Ok(body) => body,
        Err(_) => return None,
    };

    if let Value::String(maybe_isv_quote_body) = &maybe_isv_body["isvEnclaveQuoteBody"] {
        // 5. Decode isv quote body
        let decoded_quote_body = match base64::decode(&maybe_isv_quote_body) {
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
            &ab_upgrade_pk[..],
        ].concat();

        // 8. Verify signature
        let is_legal_sig = verify_p256_sig(&pk, &data, &sig);

        if !is_legal_sig {
            return None;
        }

        return Some(pk.clone())
    };

    None
}

pub fn encode_files(fs: &Vec<(Vec<u8>, u64)>) -> Vec<u8> {
    // "["
    let open_square_brackets_bytes: Vec<u8> = [91].to_vec();
    // "\"hash\":\""
    let hash_bytes: Vec<u8> = [123, 34, 104, 97, 115,104, 34, 58, 34].to_vec();
    // "\",\"size\":"
    let size_bytes: Vec<u8> = [34, 44, 34, 115, 105, 122, 101, 34, 58].to_vec();
    // "}"
    let close_curly_brackets_bytes: Vec<u8> = [125].to_vec();
    // ","
    let comma_bytes: Vec<u8> = [44].to_vec();
    // "]"
    let close_square_brackets_bytes: Vec<u8> = [93].to_vec();
    let mut rst: Vec<u8> = open_square_brackets_bytes.clone();
    let len = fs.len();
    for (pos, (hash, size)) in fs.iter().enumerate() {
        rst.extend(hash_bytes.clone());
        rst.extend(encode_file_root(hash.clone()));
        rst.extend(size_bytes.clone());
        rst.extend(encode_u64_to_string_to_bytes(*size));
        rst.extend(close_curly_brackets_bytes.clone());
        if pos != len-1 { rst.extend(comma_bytes.clone()) }
    }

    rst.extend(close_square_brackets_bytes.clone());

    rst
}

pub fn verify_p256_sig(be_pk: &Vec<u8>, data: &Vec<u8>, be_sig: &Vec<u8>) -> bool {
    let mut pk = be_pk.clone();
    let mut sig = be_sig.clone();

    pk[0..32].reverse();
    pk[32..].reverse();

    sig[0..32].reverse();
    sig[32..].reverse();

    // VerifyKey need pk with prefix 0x04
    let pk_with_prefix: Vec<u8> = [
        &vec![4][..],
        &pk[..]
    ].concat();

    let p256_sig = Signature::from_bytes(&sig).unwrap();
    let verify_key = VerifyKey::new(&pk_with_prefix[..]).unwrap();

    verify_key.verify(data, &p256_sig).is_ok()
}

// Simulate the process u64.to_string().as_bytes().to_vec()
// eg. 127 -> "127" -> 49 50 55
pub fn encode_u64_to_string_to_bytes(number: u64) -> Vec<u8> {
    let mut value = number;
    let mut encoded_number: Vec<u8> = [].to_vec();
    loop {
        encoded_number.push((value%10) as u8 + 48u8); // "0" is 48u8
        value /= 10;
        if value == 0 {
            break;
        }
    }
    encoded_number.reverse();
    encoded_number
}

// encode file root hash to hex based string
// then represent this string to vec u8
// eg. [91, 92] -> [5b, 5c] -> ["5b", "5c"] -> [53, 98, 53, 99]
fn encode_file_root(fs: Vec<u8>) -> Vec<u8> {
    let mut rst: Vec<u8> = [].to_vec();
    for v in fs.iter() {
        rst.extend(encode_u8_to_hex_string_to_bytes(*v));
    }
    rst
}

// encode one u8 value to hex based string
// then encode this string to vec u8
// eg. 91 -> 5b -> "5b" -> 53 98
fn encode_u8_to_hex_string_to_bytes(number: u8) -> Vec<u8> {
    let upper_value = number / 16 as u8; // 16 is due to hex based
    let lower_value = number % 16 as u8;
    [encode_u8_to_hex_char_to_u8(upper_value), encode_u8_to_hex_char_to_u8(lower_value)].to_vec()
}

// encode 0~16(u8) to hex based char
// then encode this char to corresponding u8
// eg. 5 -> "5" -> 53
// eg. 11 -> "b" -> 98
fn encode_u8_to_hex_char_to_u8(number: u8) -> u8 {
    if number < 10u8 {
        return number + 48u8; // '0' is 48u8
    } else {
        return number - 10u8 + 97u8; // 'a' is 97u8
    }
}
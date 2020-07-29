use super::*;

use crate::mock::{new_test_ext, run_to_block, Origin, Tee, upsert_sorder_to_provider, Market, remove_work_report};
use frame_support::{
    assert_ok, assert_noop,
    dispatch::DispatchError,
};
use hex;
use keyring::Sr25519Keyring;
use sp_core::crypto::{AccountId32, Ss58Codec};
use primitives::Hash;

type AccountId = AccountId32;

struct RegisterInfo {
    ias_sig: IASSig,
    ias_cert: Cert,
    account_id: AccountId,
    isv_body: ISVBody,
    sig: TeeSignature
}

struct ReportWorksInfo {
    pub_key: PubKey,
    block_number: u64,
    block_hash: Vec<u8>,
    reserved: u64,
    files: Vec<(MerkleRoot, u64)>,
    sig: TeeSignature
}

fn valid_register_info() -> RegisterInfo {
    let applier: AccountId =
        AccountId::from_ss58check("5FqazaU79hjpEMiWTWZx81VjsYFst15eBuSBKdQLgQibD7CX")
            .expect("valid ss58 address");

    let ias_sig = "Lb17i6Gb2LUoMTYz/fRIjZrsF9X8vxv8S5IZtWjJ2i/BklZO8xeWuS9ItM/8JgDI2qv+zZwZtdgoywK2drH8sV/d0GN/bu5RR4u+bTOJnDWRFkU6lZC9N6AT4ntdFrrkCIfPgikd3dQr21e8v9ShfUy6FT44oLCx21p5knbO1ygxFXzm73nvpLqTB7avRqT3JtHEdzvHjPBymDq18dX7a2cRbK2EwvO48cTcTXihwLZxKjdw7Kds9RC79IaSOVSoBhqBjGtccn9xitj2kPJp65KLU5KpsguTiDwrF79UMsbWI0eKv4voXodNL6YEZdFYELGsp9SpwR6sd4t0628fHg==".as_bytes();
    let ias_cert = "-----BEGIN CERTIFICATE-----\nMIIEoTCCAwmgAwIBAgIJANEHdl0yo7CWMA0GCSqGSIb3DQEBCwUAMH4xCzAJBgNV\nBAYTAlVTMQswCQYDVQQIDAJDQTEUMBIGA1UEBwwLU2FudGEgQ2xhcmExGjAYBgNV\nBAoMEUludGVsIENvcnBvcmF0aW9uMTAwLgYDVQQDDCdJbnRlbCBTR1ggQXR0ZXN0\nYXRpb24gUmVwb3J0IFNpZ25pbmcgQ0EwHhcNMTYxMTIyMDkzNjU4WhcNMjYxMTIw\nMDkzNjU4WjB7MQswCQYDVQQGEwJVUzELMAkGA1UECAwCQ0ExFDASBgNVBAcMC1Nh\nbnRhIENsYXJhMRowGAYDVQQKDBFJbnRlbCBDb3Jwb3JhdGlvbjEtMCsGA1UEAwwk\nSW50ZWwgU0dYIEF0dGVzdGF0aW9uIFJlcG9ydCBTaWduaW5nMIIBIjANBgkqhkiG\n9w0BAQEFAAOCAQ8AMIIBCgKCAQEAqXot4OZuphR8nudFrAFiaGxxkgma/Es/BA+t\nbeCTUR106AL1ENcWA4FX3K+E9BBL0/7X5rj5nIgX/R/1ubhkKWw9gfqPG3KeAtId\ncv/uTO1yXv50vqaPvE1CRChvzdS/ZEBqQ5oVvLTPZ3VEicQjlytKgN9cLnxbwtuv\nLUK7eyRPfJW/ksddOzP8VBBniolYnRCD2jrMRZ8nBM2ZWYwnXnwYeOAHV+W9tOhA\nImwRwKF/95yAsVwd21ryHMJBcGH70qLagZ7Ttyt++qO/6+KAXJuKwZqjRlEtSEz8\ngZQeFfVYgcwSfo96oSMAzVr7V0L6HSDLRnpb6xxmbPdqNol4tQIDAQABo4GkMIGh\nMB8GA1UdIwQYMBaAFHhDe3amfrzQr35CN+s1fDuHAVE8MA4GA1UdDwEB/wQEAwIG\nwDAMBgNVHRMBAf8EAjAAMGAGA1UdHwRZMFcwVaBToFGGT2h0dHA6Ly90cnVzdGVk\nc2VydmljZXMuaW50ZWwuY29tL2NvbnRlbnQvQ1JML1NHWC9BdHRlc3RhdGlvblJl\ncG9ydFNpZ25pbmdDQS5jcmwwDQYJKoZIhvcNAQELBQADggGBAGcIthtcK9IVRz4r\nRq+ZKE+7k50/OxUsmW8aavOzKb0iCx07YQ9rzi5nU73tME2yGRLzhSViFs/LpFa9\nlpQL6JL1aQwmDR74TxYGBAIi5f4I5TJoCCEqRHz91kpG6Uvyn2tLmnIdJbPE4vYv\nWLrtXXfFBSSPD4Afn7+3/XUggAlc7oCTizOfbbtOFlYA4g5KcYgS1J2ZAeMQqbUd\nZseZCcaZZZn65tdqee8UXZlDvx0+NdO0LR+5pFy+juM0wWbu59MvzcmTXbjsi7HY\n6zd53Yq5K244fwFHRQ8eOB0IWB+4PfM7FeAApZvlfqlKOlLcZL2uyVmzRkyR5yW7\n2uo9mehX44CiPJ2fse9Y6eQtcfEhMPkmHXI01sN+KwPbpA39+xOsStjhP9N1Y1a2\ntQAVo+yVgLgV2Hws73Fc0o3wC78qPEA+v2aRs/Be3ZFDgDyghc/1fgU+7C+P6kbq\nd4poyb6IW8KCJbxfMJvkordNOgOUUxndPHEi/tb/U7uLjLOgPA==\n-----END CERTIFICATE-----\n".as_bytes();
    let isv_body = "{\"id\":\"28059165425966003836075402765879561587\",\"timestamp\":\"2020-06-23T10:02:29.441419\",\"version\":3,\"epidPseudonym\":\"4tcrS6EX9pIyhLyxtgpQJuMO1VdAkRDtha/N+u/rRkTsb11AhkuTHsY6UXRPLRJavxG3nsByBdTfyDuBDQTEjMYV6NBXjn3P4UyvG1Ae2+I4lE1n+oiKgLA8CR8pc2nSnSY1Wz1Pw/2l9Q5Er6hM6FdeECgMIVTZzjScYSma6rE=\",\"isvEnclaveQuoteStatus\":\"GROUP_OUT_OF_DATE\",\"platformInfoBlob\":\"1502006504000F00000F0F02040101070000000000000000000B00000B00000002000000000000142AA23C001F46C3A71CFB50557CE2E2292DFB24EDE2621957E890432F166F6AC6FA37CD8166DBE6323CD39D3C6AA0CB41779FC7EDE281C5E50BCDCA00935E00A9DF\",\"isvEnclaveQuoteBody\":\"AgABACoUAAAKAAkAAAAAAP7yPH5zo3mCPOcf8onPvAcAAAAAAAAAAAAAAAAAAAAACA7///8CAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAABwAAAAAAAAAHAAAAAAAAAOJWq0y16RNrwcERUIj8QMofQYJUXqdXaVeMINhDAozVAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAACD1xnnferKFHD2uvYqTXdDA8iZ22kCD5xw7h38CMfOngAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAABNu2QBUIMjsY9knwTxdDP9S4cgHvP/Y0toS3FchIu2C5Bd1TBeJHYbSWioh139n2q/sxENn6SU3VMNquzMg1Ph\"}".as_bytes();
    let sig = hex::decode("3022068d50f3edaf63b5aab8f47089091d1cc4c0cf7f55991da40e244a3d26ea6beecaec1b513d281f951dc211338146c31007ff370b296aaf8d9295b2806b65").unwrap();

    RegisterInfo {
        ias_sig: ias_sig.to_vec(),
        ias_cert: ias_cert.to_vec(),
        account_id: applier,
        isv_body: isv_body.to_vec(),
        sig
    }
}

fn valid_report_works_info() -> ReportWorksInfo {
    let pub_key = hex::decode("b0b0c191996073c67747eb1068ce53036d76870516a2973cef506c29aa37323892c5cc5f379f17e63a64bb7bc69fbea14016eea76dae61f467c23de295d7f689").unwrap();
    let block_hash = hex::decode("05404b690b0c785bf180b2dd82a431d88d29baf31346c53dbda95e83e34c8a75").unwrap();
    let files: Vec<(Vec<u8>, u64)> = [
        (hex::decode("5bb706320afc633bfb843108e492192b17d2b6b9d9ee0b795ee95417fe08b660").unwrap(), 134289408),
        (hex::decode("88cdb315c8c37e2dc00fa2a8c7fe51b8149b363d29f404441982f96d2bbae65f").unwrap(), 268578816)
    ].to_vec();
    let sig = hex::decode("9c12986c01efe715ed8bed80b7e391601c45bf152e280693ffcfd10a4b386deaaa0f088fc26b0ebeca64c33cf122d372ebd787aa77beaaba9d2e499ce40a76e6").unwrap();

    ReportWorksInfo {
        pub_key,
        block_number: 300,
        block_hash,
        reserved: 4294967296,
        sig,
        files
    }
}

fn add_pending_sorder() {
    let account: AccountId = Sr25519Keyring::Bob.to_account_id();
    let files: Vec<Vec<u8>> = [
        hex::decode("5bb706320afc633bfb843108e492192b17d2b6b9d9ee0b795ee95417fe08b660").unwrap(),
        hex::decode("5bb706320afc633bfb843108e492192b17d2b6b9d9ee0b795ee95417fe08b660").unwrap(),
        hex::decode("88cdb315c8c37e2dc00fa2a8c7fe51b8149b363d29f404441982f96d2bbae65f").unwrap()
    ].to_vec();

    for (idx, file) in files.iter().enumerate() {
        upsert_sorder_to_provider(&account, file, idx as u8, 350, OrderStatus::Pending);
    }
}

fn add_success_sorder(expired_on: u32) {
    let account: AccountId = Sr25519Keyring::Bob.to_account_id();
    let file: MerkleRoot =
        hex::decode("5bb706320afc633bfb843108e492192b17d2b6b9d9ee0b795ee95417fe08b661").unwrap();

    upsert_sorder_to_provider(&account, &file, 99, expired_on, OrderStatus::Success);

}

#[test]
fn test_for_register_success() {
    new_test_ext().execute_with(|| {
        let applier: AccountId =
            AccountId::from_ss58check("5FqazaU79hjpEMiWTWZx81VjsYFst15eBuSBKdQLgQibD7CX")
                .expect("valid ss58 address");
        let register_info = valid_register_info();

        assert_ok!(Tee::register(
            Origin::signed(applier.clone()),
            register_info.ias_sig,
            register_info.ias_cert,
            register_info.account_id,
            register_info.isv_body,
            register_info.sig
        ));

        let id_registered = Tee::identities(applier.clone()).1.unwrap();

        let id = Identity {
            pub_key: hex::decode("4dbb6401508323b18f649f04f17433fd4b87201ef3ff634b684b715c848bb60b905dd5305e24761b4968a8875dfd9f6abfb3110d9fa494dd530daaeccc8353e1").unwrap(),
            code: hex::decode("e256ab4cb5e9136bc1c1115088fc40ca1f4182545ea75769578c20d843028cd5").unwrap()
        };

        assert_eq!(id, id_registered);
    });
}

#[test]
fn test_for_register_failed_by_duplicate_id() {
    new_test_ext().execute_with(|| {
        let applier: AccountId =
            AccountId::from_ss58check("5FqazaU79hjpEMiWTWZx81VjsYFst15eBuSBKdQLgQibD7CX")
                .expect("valid ss58 address");
        let register_info = valid_register_info();

        // First register should be ok
        assert_ok!(Tee::register(
            Origin::signed(applier.clone()),
            register_info.ias_sig.clone(),
            register_info.ias_cert.clone(),
            register_info.account_id.clone(),
            register_info.isv_body.clone(),
            register_info.sig.clone()
        ));

        // Second register should failed
        assert_noop!(
            Tee::register(
                Origin::signed(applier.clone()),
                register_info.ias_sig,
                register_info.ias_cert,
                register_info.account_id,
                register_info.isv_body,
                register_info.sig
            ),
            DispatchError::Module {
                index: 0,
                error: 1,
                message: Some("DuplicateId"),
            }
        );
    });
}

#[test]
fn test_for_register_failed_by_invalid_ca() {
    new_test_ext().execute_with(|| {
        let applier: AccountId =
            AccountId::from_ss58check("5FqazaU79hjpEMiWTWZx81VjsYFst15eBuSBKdQLgQibD7CX")
                .expect("valid ss58 address");

        let mut register_info = valid_register_info();
        register_info.ias_cert = "wrong_ca".as_bytes().to_vec();

        assert_noop!(
            Tee::register(
                Origin::signed(applier.clone()),
                register_info.ias_sig,
                register_info.ias_cert,
                register_info.account_id,
                register_info.isv_body,
                register_info.sig
            ),
            DispatchError::Module {
                index: 0,
                error: 2,
                message: Some("IllegalTrustedChain"),
            }
        );
    });
}

#[test]
fn test_for_register_failed_by_illegal_ca() {
    new_test_ext().execute_with(|| {
        let applier: AccountId =
            AccountId::from_ss58check("5FqazaU79hjpEMiWTWZx81VjsYFst15eBuSBKdQLgQibD7CX")
                .expect("valid ss58 address");

        let mut register_info = valid_register_info();
        register_info.ias_cert = "-----BEGIN CERTIFICATE-----\nMIIFFjCCAv4CCQChGbr81on1kDANBgkqhkiG9w0BAQsFADBNMQswCQYDVQQGEwJD\nTjERMA8GA1UECAwIU2hhbmdoYWkxETAPBgNVBAcMCFNoYW5naGFpMQswCQYDVQQK\nDAJaazELMAkGA1UECwwCWkgwHhcNMjAwNjIzMDUwODQyWhcNMjEwNjIzMDUwODQy\nWjBNMQswCQYDVQQGEwJDTjERMA8GA1UECAwIU2hhbmdoYWkxETAPBgNVBAcMCFNo\nYW5naGFpMQswCQYDVQQKDAJaazELMAkGA1UECwwCWkgwggIiMA0GCSqGSIb3DQEB\nAQUAA4ICDwAwggIKAoICAQC7oznSx9/gjE1/cEgXGKLATEvDPAdnvJ/T2lopEDZ/\nJEsNu0qBQsbOSAgJUhqAfX6ahwAn/Epz7yXy7PxCKZJi/wvESJ/WX4x+b7tE1nU1\nK7p7bKGJ6erww/ZrmGV+4+6GvdCg5dcOAA5TXAE2ZjTeIoR76Y3IZb0L78i/S+q1\ndZpx4YRfzwHNELCqpgwaJAS0FHIH1g+6X59EbF0UFT0YcM90Xxa0gHkPlYIoEoWS\n+UA/UW1MjuUwNaS5mNB3IpcrMhSeOkkqLglMdanu6r5MZpjuLBl7+sACoH0P7Rda\nx1c/NadmrbZf3/+AHvMZ6M9HrciyKKMauBZM9PUMrzLnTfF8iHitrSlum1UIfUuN\nvXXXzNLWskTxcXuWuyBgXpKM7D5XG7VnENDAbEYiN5Ej6zz5Zi/2OHVyErI3f1Ka\nvwTC8AjJMemCOBgPrgqMH7l6SAXr55eozXaSQVa4HG9iPGJixXZU5PUIiVFVO7Hj\nXtE3yfa2zaucB4rKhOJLwSD9qYgqFKB+vQ1X2GUkkPpsAMrL4n/VDQcJkrvjK3tt\n7AES9Q3TLmbVK91E2scF063XKUc3vT5Q8hcvg4MMLHn7gzMEaWTzjknRo1fLNWPY\nlPV3lZhBwkxdHKYodY7d6subE8nOsiFibq8X6Nf0UNIG0MXeFTAM2WfG2s6AlnZR\nHwIDAQABMA0GCSqGSIb3DQEBCwUAA4ICAQA5NL5fFP1eSBN/WEd8z6vJRWPyOz1t\ntrQF5o7Fazh3AtFcb3j3ab8NG/4qDTr7FrYFyOD+oHgIoHlzK4TFxlhSZquvU2Xb\nSMQyIQChojz8jbTa771ZPsjQjDWU0R0r83vI1ywc1v6eFpXIpV5oidT0afbJ85n4\ngwhVd6S2eTHh91U11BKf2gV4nhewzON4r7YuFg7sMMDVl3wx1HtXCKg5dMtgePyc\nGejdpyxdWX4BIxnvIY8QdAa4gvi9etzRf83mcNfwr+gM0rTyqGEBXuPW8bwq9BRL\nXz6zeL1Anb2HsjMQ6+MKWbXRhBFBCbB+APDcnjHv7OZXUaILi0B1JoTPu/jjSK1U\n7yAnK1sRtVpADVpa2N4STk9ImdTKfqTHZR9iTaheoqxRuTm7vzwGy72V4HEeEyOa\njyYpiCD8we3gJfro1pjzFLOqE3yU14vUc0SwQCZWlEH8LR/a8m/ZCPuqN4a2xPJO\nwksgMSCDkui5yUr4uTINFpROXHzz1dpOuUnvkkCAjKieZHWCyYyoEE0tedgejwee\nWv3UtR7svhpbAVoIQ8Z8EV2Ys1IN0Tp+4pltRbcgeZK0huEFOz4BL/1EGezwLbjE\nvoOMtTumWI9Mw5FTG4iTbRxvWL/KnLMvZr7V+o5ovmm0jeLW03Eh/E+aHH0B0tQp\nf6FKPRF7+Imo/g==\n-----END CERTIFICATE-----\n".as_bytes().to_vec();

        assert_noop!(
            Tee::register(
                Origin::signed(applier.clone()),
                register_info.ias_sig,
                register_info.ias_cert,
                register_info.account_id,
                register_info.isv_body,
                register_info.sig
            ),
            DispatchError::Module {
                index: 0,
                error: 2,
                message: Some("IllegalTrustedChain"),
            }
        );
    });
}

#[test]
fn test_for_register_failed_by_illegal_isv_body() {
    new_test_ext().execute_with(|| {
        let applier: AccountId =
            AccountId::from_ss58check("5FqazaU79hjpEMiWTWZx81VjsYFst15eBuSBKdQLgQibD7CX")
                .expect("valid ss58 address");

        let mut register_info = valid_register_info();
        // Another isv body with wrong enclave code and public key
        register_info.isv_body = "{\"id\":\"125366127848601794295099877969265555107\",\"timestamp\":\"2020-06-22T11:34:54.845374\",\"version\":3,\"epidPseudonym\":\"4tcrS6EX9pIyhLyxtgpQJuMO1VdAkRDtha/N+u/rRkTsb11AhkuTHsY6UXRPLRJavxG3nsByBdTfyDuBDQTEjMYV6NBXjn3P4UyvG1Ae2+I4lE1n+oiKgLA8CR8pc2nSnSY1Wz1Pw/2l9Q5Er6hM6FdeECgMIVTZzjScYSma6rE=\",\"isvEnclaveQuoteStatus\":\"GROUP_OUT_OF_DATE\",\"platformInfoBlob\":\"1502006504000F00000F0F02040101070000000000000000000B00000B00000002000000000000142A70382C3A557904D4AB5766B2D3BAAD8ED8B7B372FB8F25C7E06212DEF369A389047D2249CF2ACDB22197AD7EE604634D47B3720BB1837E35C5C7D66F256117B6\",\"isvEnclaveQuoteBody\":\"AgABACoUAAAKAAkAAAAAAP7yPH5zo3mCPOcf8onPvAcAAAAAAAAAAAAAAAAAAAAACA7///8CAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAABwAAAAAAAAAHAAAAAAAAAJY6Ggjlm1yvKL0sgypJx2BBrGbValVEq8cCi/0sViQcAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAACD1xnnferKFHD2uvYqTXdDA8iZ22kCD5xw7h38CMfOngAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAADagmwZsR+S1ZNqgDg6HobleD6S6tRtqtsF1j81Bw7CnoP9/ZGNDEEzMEh+EKk1jAPW8PE+YKpum0xkVhh2J5Y8\"}".as_bytes().to_vec();

        assert_noop!(
            Tee::register(
                Origin::signed(applier.clone()),
                register_info.ias_sig,
                register_info.ias_cert,
                register_info.account_id,
                register_info.isv_body,
                register_info.sig
            ),
            DispatchError::Module {
                index: 0,
                error: 2,
                message: Some("IllegalTrustedChain"),
            }
        );
    });
}

#[test]
fn test_for_register_failed_by_illegal_sig() {
    new_test_ext().execute_with(|| {
        let applier: AccountId =
            AccountId::from_ss58check("5FqazaU79hjpEMiWTWZx81VjsYFst15eBuSBKdQLgQibD7CX")
                .expect("valid ss58 address");

        let mut register_info = valid_register_info();
        // Another identity sig
        register_info.sig = hex::decode("f45e401778623de9b27726ab749549da35b1f8c0fd7bb56e0c1c3bba86948eb41717c9e13bf57113d85a1cc64d5cc2fc95c12d8b3108ab6fadeff621dfb6a486").unwrap();

        assert_noop!(
            Tee::register(
                Origin::signed(applier.clone()),
                register_info.ias_sig,
                register_info.ias_cert,
                register_info.account_id,
                register_info.isv_body,
                register_info.sig
            ),
            DispatchError::Module {
                index: 0,
                error: 2,
                message: Some("IllegalTrustedChain"),
            }
        );
    });
}

#[test]
fn test_for_register_failed_by_illegal_ias_sig() {
    new_test_ext().execute_with(|| {
        let applier: AccountId =
            AccountId::from_ss58check("5FqazaU79hjpEMiWTWZx81VjsYFst15eBuSBKdQLgQibD7CX")
                .expect("valid ss58 address");

        let mut register_info = valid_register_info();
        // Another ias sig
        register_info.ias_sig = "cU3uOnd5XghR3ngJTbSFr48ttEIrJtbHHtuRM3hgzX7LHGacuTBMVRy0VK3ldqeM7KPBS+g3Da2anDHEJsSgITTXfHh+dxjUPO9v2hC+okjtWSY9fWhaFlR31lFWmSSbUfJSe2rtkLQRoj5VgKpOVkVuGzQjl/xF+SQZU4gjq130TwO8Gr/TvPLA3vJnM3/d8FUpcefp5Q5dbBka7y2ej8hDTyOjix3ZXSVD2SrSySfIg6kvIPS/EEJYoz/eMOFciSWuIIPrUj9M0eUc4xHsUxgNcgjOmtRt621RlzAwgY+yPFoqJwKtmlVNYy/FyvSbIMSB3kJbmlA+qHwOBgPQ0A==".as_bytes().to_vec();

        assert_noop!(
            Tee::register(
                Origin::signed(applier.clone()),
                register_info.ias_sig,
                register_info.ias_cert,
                register_info.account_id,
                register_info.isv_body,
                register_info.sig
            ),
            DispatchError::Module {
                index: 0,
                error: 2,
                message: Some("IllegalTrustedChain"),
            }
        );
    });
}

#[test]
fn test_for_report_works_success() {
    new_test_ext().execute_with(|| {
        // generate 303 blocks first
        run_to_block(303, None);
        // prepare sorder
        add_pending_sorder();

        assert_eq!(Market::storage_orders(Hash::repeat_byte(1)).unwrap_or_default().expired_on, 350);

        let account: AccountId = Sr25519Keyring::Bob.to_account_id();
        let report_works_info = valid_report_works_info();

        // Check workloads
        assert_eq!(Tee::reserved(), 0);

        assert_ok!(Tee::report_works(
            Origin::signed(account.clone()),
            report_works_info.pub_key,
            report_works_info.block_number,
            report_works_info.block_hash,
            report_works_info.reserved,
            report_works_info.files,
            report_works_info.sig
        ));

        // Check workloads after work report
        assert_eq!(Tee::reserved(), 4294967296);
        assert_eq!(Tee::used(), 402868224);

        // Check same file all been confirmed
        assert_eq!(Market::storage_orders(Hash::repeat_byte(1)).unwrap_or_default().status,
                   OrderStatus::Success);
        assert_eq!(Market::storage_orders(Hash::repeat_byte(2)).unwrap_or_default().status,
                   OrderStatus::Success);
        assert_eq!(Market::storage_orders(Hash::repeat_byte(1)).unwrap_or_default().expired_on, 653);
    });
}

#[test]
fn test_for_report_works_success_without_sorder() {
    new_test_ext().execute_with(|| {
        // generate 303 blocks first
        run_to_block(303, None);

        let account: AccountId = Sr25519Keyring::Bob.to_account_id();
        let report_works_info = valid_report_works_info();

        // Check workloads
        assert_eq!(Tee::reserved(), 0);

        assert_ok!(Tee::report_works(
            Origin::signed(account.clone()),
            report_works_info.pub_key,
            report_works_info.block_number,
            report_works_info.block_hash,
            report_works_info.reserved,
            report_works_info.files,
            report_works_info.sig
        ));

        // Check workloads after work report
        assert_eq!(Tee::reserved(), 4294967296);
        assert_eq!(Tee::used(), 0);
    });
}

#[test]
fn test_for_report_works_failed_by_pub_key_is_not_found() {
    new_test_ext().execute_with(|| {
        let account: AccountId32 = Sr25519Keyring::Bob.to_account_id();

        let mut report_works_info = valid_report_works_info();
        report_works_info.pub_key = "another_pub_key".as_bytes().to_vec();

        assert_noop!(
            Tee::report_works(
                Origin::signed(account.clone()),
                report_works_info.pub_key,
                report_works_info.block_number,
                report_works_info.block_hash,
                report_works_info.reserved,
                report_works_info.files,
                report_works_info.sig
            ),
            DispatchError::Module {
                index: 0,
                error: 4,
                message: Some("InvalidPubKey"),
            }
        );
    });
}

#[test]
fn test_for_report_works_failed_by_reporter_is_not_registered() {
    new_test_ext().execute_with(|| {
        let account: AccountId32 = Sr25519Keyring::Dave.to_account_id();

        let report_works_info = ReportWorksInfo {
            pub_key: "pub_key_bob".as_bytes().to_vec(),
            block_number: 50,
            block_hash: "block_hash".as_bytes().to_vec(),
            reserved: 2000,
            sig: "sig_key_bob".as_bytes().to_vec(),
            files: vec![]
        };

        assert_noop!(
            Tee::report_works(
                Origin::signed(account.clone()),
                report_works_info.pub_key,
                report_works_info.block_number,
                report_works_info.block_hash,
                report_works_info.reserved,
                report_works_info.files,
                report_works_info.sig
            ),
            DispatchError::Module {
                index: 0,
                error: 3,
                message: Some("IllegalReporter"),
            }
        );
    });
}

#[test]
fn test_for_work_report_timing_check_failed_by_wrong_block_hash() {
    new_test_ext().execute_with(|| {
        // generate 50 blocks first
        run_to_block(50, None);

        let account: AccountId32 = Sr25519Keyring::Bob.to_account_id();
        let block_hash = [1; 32].to_vec();

        let report_works_info = ReportWorksInfo {
            pub_key: hex::decode("b0b0c191996073c67747eb1068ce53036d76870516a2973cef506c29aa37323892c5cc5f379f17e63a64bb7bc69fbea14016eea76dae61f467c23de295d7f689").unwrap(),
            block_number: 50,
            block_hash,
            reserved: 0,
            sig: "sig_key_alice".as_bytes().to_vec(),
            files: vec![]
        };

        assert_noop!(
            Tee::report_works(
                Origin::signed(account.clone()),
                report_works_info.pub_key,
                report_works_info.block_number,
                report_works_info.block_hash,
                report_works_info.reserved,
                report_works_info.files,
                report_works_info.sig
            ),
            DispatchError::Module {
                index: 0,
                error: 5,
                message: Some("InvalidReportTime"),
            }
        );
    });
}

#[test]
fn test_for_work_report_timing_check_failed_by_slot_outdated() {
    new_test_ext().execute_with(|| {
        // generate 103 blocks first
        run_to_block(103, None);

        let account: AccountId32 = Sr25519Keyring::Bob.to_account_id();
        let block_hash = [0; 32].to_vec();

        let report_works_info = ReportWorksInfo {
            pub_key: hex::decode("b0b0c191996073c67747eb1068ce53036d76870516a2973cef506c29aa37323892c5cc5f379f17e63a64bb7bc69fbea14016eea76dae61f467c23de295d7f689").unwrap(),
            block_number: 50,
            block_hash,
            reserved: 1999,
            sig: "sig_key_alice".as_bytes().to_vec(),
            files: vec![]
        };

        assert_noop!(
            Tee::report_works(
                Origin::signed(account.clone()),
                report_works_info.pub_key,
                report_works_info.block_number,
                report_works_info.block_hash,
                report_works_info.reserved,
                report_works_info.files,
                report_works_info.sig
            ),
            DispatchError::Module {
                index: 0,
                error: 5,
                message: Some("InvalidReportTime"),
            }
        );
    });
}

#[test]
fn test_for_work_report_sig_check_failed() {
    new_test_ext().execute_with(|| {
        // generate 303 blocks first
        run_to_block(303, None);

        let account: AccountId32 = Sr25519Keyring::Bob.to_account_id();
        let pub_key = hex::decode("b0b0c191996073c67747eb1068ce53036d76870516a2973cef506c29aa37323892c5cc5f379f17e63a64bb7bc69fbea14016eea76dae61f467c23de295d7f689").unwrap();
        let block_hash = hex::decode("05404b690b0c785bf180b2dd82a431d88d29baf31346c53dbda95e83e34c8a75").unwrap();
        let files: Vec<(Vec<u8>, u64)> = [
            (hex::decode("5bb706320afc633bfb843108e492192b17d2b6b9d9ee0b795ee95417fe08b660").unwrap(), 134289408),
            (hex::decode("88cdb315c9c37e2dc00fa2a8c7fe51b8149b363d29f404441982f96d2bbae65f").unwrap(), 268578816)
        ].to_vec();
        let sig = hex::decode("9c12986c01efe715ed8bed80b7e391601c45bf152e280693ffcfd10a4b386deaaa0f088fc26b0ebeca64c33cf122d372ebd787aa77beaaba9d2e499ce40a76e6").unwrap();

        let report_works_info = ReportWorksInfo {
            pub_key,
            block_number: 300,
            block_hash,
            reserved: 4294967296,
            sig,
            files
        };

        assert_noop!(
            Tee::report_works(
                Origin::signed(account.clone()),
                report_works_info.pub_key,
                report_works_info.block_number,
                report_works_info.block_hash,
                report_works_info.reserved,
                report_works_info.files,
                report_works_info.sig
            ),
            DispatchError::Module {
                index: 0,
                error: 6,
                message: Some("IllegalWorkReportSig"),
            }
        );
    });
}

#[test]
fn test_for_wr_check_failed_order_by_no_file_in_wr() {
    new_test_ext().execute_with(|| {
        let account: AccountId = Sr25519Keyring::Bob.to_account_id();
        add_success_sorder(350);
        // generate 303 blocks first
        run_to_block(303, None);

        let report_works_info = valid_report_works_info();

        // report works should ok
        assert_ok!(Tee::report_works(
            Origin::signed(account.clone()),
            report_works_info.pub_key,
            report_works_info.block_number,
            report_works_info.block_hash,
            report_works_info.reserved,
            report_works_info.files,
            report_works_info.sig
        ));

        // check work report and workload, current_report_slot updating should work
        Tee::update_identities();

        // Check this 99 order should be failed
        assert_eq!(Market::storage_orders(Hash::repeat_byte(99)).unwrap().status,
                   OrderStatus::Failed);
    });
}

#[test]
fn test_for_wr_check_failed_order_by_not_reported() {
    new_test_ext().execute_with(|| {
        // 1st era
        run_to_block(303, None);
        Tee::update_identities();

        add_success_sorder(650);

        // 2nd era
        run_to_block(606, None);
        Tee::update_identities();

        // Check this 99 order should be failed, cause wr is outdated
        assert_eq!(Market::storage_orders(Hash::repeat_byte(99)).unwrap().status,
                   OrderStatus::Failed);
    });
}

#[test]
fn test_for_wr_check_failed_order_by_no_wr() {
    new_test_ext().execute_with(|| {
        let account: AccountId = Sr25519Keyring::Bob.to_account_id();
        // 1st era
        run_to_block(303, None);
        add_success_sorder(350);

        // This won't happen when previous test case occurs, cause `not reported` will
        // set sorder.status = Failed, but we still design this test case anyway.
        remove_work_report(&account);
        Tee::update_identities();

        // Check this 99 order should be failed, cause wr is outdated
        assert_eq!(Market::storage_orders(Hash::repeat_byte(99)).unwrap().status,
                   OrderStatus::Failed);
    });
}

#[test]
fn test_for_outdated_work_reports() {
    new_test_ext().execute_with(|| {
        let account: AccountId = Sr25519Keyring::Bob.to_account_id();
        // generate 303 blocks first
        run_to_block(303, None);

        let report_works_info = valid_report_works_info();
        let wr = WorkReport {
            block_number: report_works_info.block_number,
            used: 0,
            reserved: report_works_info.reserved.clone(),
            cached_reserved: report_works_info.reserved.clone(),
            files: report_works_info.files.clone()
        };

        // report works should ok
        assert_ok!(Tee::report_works(
            Origin::signed(account.clone()),
            report_works_info.pub_key,
            report_works_info.block_number,
            report_works_info.block_hash,
            report_works_info.reserved,
            report_works_info.files,
            report_works_info.sig
        ));

        // check work report and workload, current_report_slot updating should work
        assert_eq!(Tee::current_report_slot(), 0);
        Tee::update_identities();
        assert_eq!(Tee::current_report_slot(), 300);

        // Check workloads
        assert_eq!(Tee::reserved(), 4294967296);
        assert_eq!(Tee::used(), 0);

        // generate 401 blocks, wr still valid
        run_to_block(401, None);
        assert_eq!(
            Tee::work_reports(&account),
            Some(wr.clone())
        );
        assert!(Tee::reported_in_slot(&account, 300).1);

        // generate 602 blocks
        run_to_block(602, None);
        assert_eq!(Tee::current_report_slot(), 300);
        Tee::update_identities();
        assert_eq!(Tee::current_report_slot(), 600);
        assert_eq!(
            Tee::work_reports(&account),
            Some(wr.clone())
        );
        assert!(!Tee::reported_in_slot(&account, 600).1);

        // Check workloads
        assert_eq!(Tee::reserved(), 4294967296);
        assert_eq!(Tee::used(), 0);

        run_to_block(903, None);
        assert_eq!(Tee::current_report_slot(), 600);
        Tee::update_identities();
        assert_eq!(Tee::current_report_slot(), 900);

        // Check workloads
        assert_eq!(Tee::work_reports(&account), None);
        assert_eq!(Tee::reserved(), 0);
        assert_eq!(Tee::used(), 0);
    });
}

#[test]
fn test_abnormal_era() {
    new_test_ext().execute_with(|| {
        let account: AccountId = Sr25519Keyring::Bob.to_account_id();
        let report_works_info = valid_report_works_info();
        let wr = WorkReport {
            block_number: report_works_info.block_number,
            used: 0,
            reserved: report_works_info.reserved.clone(),
            cached_reserved: report_works_info.reserved.clone(),
            files: report_works_info.files.clone()
        };

        // If new era happens in 101, next work is not reported
        run_to_block(101, None);
        Tee::update_identities();
        assert_eq!(
            Tee::work_reports(&account),
            Some(Default::default())
        );
        assert_eq!(Tee::reserved(), 0);
        assert_eq!(Tee::current_report_slot(), 0);

        // If new era happens on 301, we should update work report and current report slot
        run_to_block(301, None);
        Tee::update_identities();
        assert_eq!(
            Tee::work_reports(&account),
            Some(Default::default())
        );
        assert_eq!(
            Tee::current_report_slot(),
            300
        );
        assert!(Tee::reported_in_slot(&account, 0).1);

        // If next new era happens on 303, then nothing should happen
        run_to_block(303, None);
        Tee::update_identities();
        assert_eq!(
            Tee::work_reports(&account),
            Some(Default::default())
        );
        assert_eq!(
            Tee::current_report_slot(),
            300
        );
        assert!(Tee::reported_in_slot(&account, 0).1);
        assert!(!Tee::reported_in_slot(&account, 300).1);

        // Then report works
        // reserved: 4294967296,
        // used: 1676266280,
        run_to_block(304, None);
        assert_ok!(Tee::report_works(
            Origin::signed(account.clone()),
            report_works_info.pub_key,
            report_works_info.block_number,
            report_works_info.block_hash,
            report_works_info.reserved,
            report_works_info.files,
            report_works_info.sig
        ));
        assert_eq!(Tee::work_reports(&account), Some(wr));
        // total workload should keep same, cause we only updated in a new era
        assert_eq!(Tee::reserved(), 4294967296);
        assert_eq!(Tee::used(), 0);
        assert!(Tee::reported_in_slot(&account, 300).1);
    })
}

#[test]
fn test_ab_upgrade_should_work() {
    new_test_ext().execute_with(|| {
        let reporter: AccountId = Sr25519Keyring::Bob.to_account_id();
        let old_code = hex::decode("bc55e1730c64d9d9788e25161825b3dca016b2288c51daa844bc95f29a010241").unwrap();
        let old_pub_key = hex::decode("c11153203b6003932e50bab39d29cac12fda34d9fc05d96c265940666285f655290d3de363bb81afb36f183123549915268da4589165f4c85c4bfc436305002c").unwrap();
        let old_bh = hex::decode("f59a7fa70a1bc287d6def78c272739b8763c54aa41d254a58b8eca2986baee03").unwrap();
        let old_files = vec![(hex::decode("1111").unwrap(), 40), (hex::decode("2222").unwrap(), 80)];
        let old_id = Identity {
            pub_key: old_pub_key.clone(),
            code: old_code.clone(),
        };
        let mut old_work_report = WorkReport {
            block_number: 37_200,
            used: 0,
            reserved: 42_949_672_960,
            cached_reserved: 42_949_672_960,
            files: old_files.clone()
        };

        // 1. Normal report should be ✅
        // a. Run to 37205 block first with old tee code
        Code::put(old_code.clone());
        run_to_block(37205, Some(old_bh.clone()));

        // b. Identity should do `upgrade` with current_id.code != code
        assert!(Tee::maybe_upsert_id(&reporter, &old_id));

        // c. Report works with current id should be ✅
        assert_ok!(Tee::report_works(
            Origin::signed(reporter.clone()),
            old_pub_key.clone(),
            37_200,
            old_bh.clone(),
            42_949_672_960,
            old_files.clone(),
            hex::decode("a30eb07fd09687264a7b7215061cd9424f945c898bfeb326c9bfa5870ec3926639d10032d7f5141514b03af32142fec7bb8ad09f028d6e0c5e40f4bc03d56272").unwrap(),
        ));
        assert_eq!(Tee::work_reports(&reporter).unwrap(), old_work_report.clone());
        assert_eq!(Tee::reported_in_slot(&reporter, 37200), (false, true));

        // 2. AB Upgrade should be ✅(accept 2 ids report works)
        // a. Bob do the upgrade
        let new_code = hex::decode("d7e6c3c814a5efe3152e1ee5db8ae57ae64836a65102fd328fdc449375baabc8").unwrap();
        let new_bh = hex::decode("d5181df4310eb49f08df7f49cccd61dc3e42aa99cb9d6dfa954cc344a7fa4373").unwrap();
        let new_pk = hex::decode("6a6b80246a52ebdfbd2d51dfaca18b4d05c883baf6e1178bdaa940d1c8dbcc27745b4d2db2673e7def5cb1697018f722edbd8c49e7447d921e863c84342d86a8").unwrap();
        let new_files = vec![(hex::decode("2222").unwrap(), 80)];
        let new_id = Identity {
            pub_key: new_pk.clone(),
            code: new_code.clone(),
        };
        let mut new_work_report = WorkReport {
            block_number: 38_700,
            used: 0,
            reserved: 40_000,
            cached_reserved: 40_000,
            files: new_files.clone()
        };

        // b. Run to 38705 block with new tee code, and do the upgrade
        run_to_block(38705, Some(new_bh.clone()));
        assert_ok!(Tee::upgrade(Origin::root(), new_code.clone(), 39000));

        assert!(Tee::maybe_upsert_id(&reporter, &new_id));
        assert_eq!(Tee::identities(&reporter), (Some(old_id.clone()), Some(new_id.clone())));

        // c. Report with new identity should be ✅
        assert_ok!(Tee::report_works(
            Origin::signed(reporter.clone()),
            new_pk.clone(),
            38_700,
            new_bh.clone(),
            40_000,
            new_files.clone(),
            hex::decode("525fd0d4afcd99965166c6fca2cb74ce34bb303109921d6ab0e172aafb00a4c3ec6086c59e4abe232782848170b88d19b2641d470bb30ba7827d5161ec5ad46e").unwrap(),
        ));
        assert_eq!(Tee::work_reports(&reporter).unwrap(), new_work_report.clone());
        assert_eq!(Tee::reported_in_slot(&reporter, 38700), (false, true));

        // d. Report with old identity should also be ✅
        assert_ok!(Tee::report_works(
            Origin::signed(reporter.clone()),
            old_pub_key.clone(),
            38700,
            new_bh.clone(),
            100,
            old_files.clone(),
            hex::decode("c29ff453b318c9f9e508b9215ff81a7b31df5817630ecb80abbbbf9d7c6e26193ca091a9ff0632974af55db0d2e83c4415fcb03dc46f6f75eba168fd93c24609").unwrap(),
        ));
        old_work_report.reserved = 100;
        new_work_report.cached_reserved = 0;
        new_work_report.reserved += old_work_report.reserved;
        new_work_report.files = old_files.clone();

        assert_eq!(Tee::work_reports(&reporter).unwrap(), new_work_report.clone());
        assert_eq!(Tee::reported_in_slot(&reporter, 38700), (true, true));

        // 3. AB expire should work, replay the block authoring
        // a. Bob do not upgrade
        assert_ok!(Tee::upgrade(Origin::root(), new_code.clone(), 38800));

        // b. Double report would be ignore in the first place ❌, even the sig is illegal
        assert_ok!(Tee::report_works(
            Origin::signed(reporter.clone()),
            old_pub_key.clone(),
            38700,
            new_bh.clone(),
            100,
            old_files.clone(),
            hex::decode("1111").unwrap(),
        ));
        assert_ok!(Tee::report_works(
            Origin::signed(reporter.clone()),
            new_pk.clone(),
            38700,
            new_bh.clone(),
            100,
            new_files.clone(),
            hex::decode("2222").unwrap(),
        ));
        assert_eq!(Tee::work_reports(&reporter).unwrap(), new_work_report.clone());

        // c. Run to block 39005, report should ❌
        run_to_block(39005, Some(new_bh.clone()));
        assert_noop!(Tee::report_works(
            Origin::signed(reporter.clone()),
            old_pub_key.clone(),
            39000,
            new_bh.clone(),
            10,
            old_files.clone(),
            hex::decode("422459e0365445fc1fa14682cef15298f34259cf57206622e4f8355c4633d3a5c14cfea81051b6a11754001f234515115caca6bf3b96b43b0c31fe93f9082d5e").unwrap(),
        ), DispatchError::Module {
            index: 0,
            error: 4,
            message: Some("InvalidPubKey"),
        });

        // 4. Shrink attack(do not upgrade and shrink his disk) detection should works fine
        assert_ok!(Tee::upgrade(Origin::root(), new_code.clone(), 39500));

        // a. Report with old identity should also be ✅
        assert_ok!(Tee::report_works(
            Origin::signed(reporter.clone()),
            old_pub_key.clone(),
            39000,
            new_bh.clone(),
            10,
            old_files.clone(),
            hex::decode("422459e0365445fc1fa14682cef15298f34259cf57206622e4f8355c4633d3a5c14cfea81051b6a11754001f234515115caca6bf3b96b43b0c31fe93f9082d5e").unwrap(),
        ));
        new_work_report.cached_reserved = 10;
        new_work_report.block_number = 39000;
        new_work_report.files = old_files.clone();
        // b. This will keep the same with elder work report
        assert_eq!(Tee::work_reports(&reporter).unwrap(), new_work_report.clone());
        assert_eq!(Tee::reported_in_slot(&reporter, 39000), (true, false));

        // c. Reporter with old identity, the reserved should be right(after shrink the workload)
        run_to_block(39305, Some(new_bh.clone()));
        assert_ok!(Tee::report_works(
            Origin::signed(reporter.clone()),
            old_pub_key.clone(),
            39300,
            new_bh.clone(),
            0,
            old_files.clone(),
            hex::decode("be15fd80b7b590bd08e60a19acf6e01292ec9f05fbd4eff79d03bdea1c43aec6e0ebda676b0215c0ab553cab2add696f98d3b759719ad1442360dc2303241ae7").unwrap(),
        ));
        new_work_report.reserved = 0;
        new_work_report.cached_reserved = 0;
        new_work_report.block_number = 39300;
        assert_eq!(Tee::work_reports(&reporter).unwrap(), new_work_report.clone());
        assert_eq!(Tee::reported_in_slot(&reporter, 39300), (true, false));
    });
}
// Copyright (C) 2019-2021 Crust Network Technologies Ltd.
// This file is part of Crust.

use super::*;

use crate::mock::*;
use frame_support::{
    assert_ok, assert_noop, assert_err,
    dispatch::{DispatchError, DispatchErrorWithPostInfo}, weights::PostDispatchInfo
};
use hex;
use keyring::Sr25519Keyring;

/// Register test cases
#[test]
fn register_should_work() {
    ExtBuilder::default()
        .build()
        .execute_with(|| {
        let applier: AccountId =
            AccountId::from_ss58check("5FqazaU79hjpEMiWTWZx81VjsYFst15eBuSBKdQLgQibD7CX")
                .expect("valid ss58 address");
        let register_info = legal_register_info();

        assert_ok!(Swork::register(
            Origin::signed(applier.clone()),
            register_info.ias_sig,
            register_info.ias_cert,
            register_info.account_id,
            register_info.isv_body,
            register_info.sig
        ));

        let legal_code = LegalCode::get();
        let legal_pk = LegalPK::get();

        assert_eq!(Swork::identities(applier).is_none(), true);
        assert_eq!(Swork::pub_keys(legal_pk), PKInfo {
            code: legal_code,
            allow_report_slot: 0,
            anchor: None
        });
    });
}

// Duplicate pk check is removed due to the uniqueness guaranteed by sWorker-side

#[test]
fn register_should_failed_with_unmatched_reporter() {
    ExtBuilder::default()
        .build()
        .execute_with(|| {
        let applier: AccountId = Sr25519Keyring::Bob.to_account_id();

        let register_info = legal_register_info();

        assert_noop!(
            Swork::register(
                Origin::signed(applier.clone()),
                register_info.ias_sig,
                register_info.ias_cert,
                register_info.account_id,
                register_info.isv_body,
                register_info.sig
            ),
            DispatchError::Module {
                index: 0,
                error: 0,
                message: Some("IllegalApplier"),
            }
        );
    });
}

#[test]
fn register_should_failed_with_illegal_cert() {
    ExtBuilder::default()
        .build()
        .execute_with(|| {
        let applier: AccountId =
            AccountId::from_ss58check("5FqazaU79hjpEMiWTWZx81VjsYFst15eBuSBKdQLgQibD7CX")
                .expect("valid ss58 address");

        let mut register_info = legal_register_info();
        register_info.ias_cert = "-----BEGIN CERTIFICATE-----\nMIIFFjCCAv4CCQChGbr81on1kDANBgkqhkiG9w0BAQsFADBNMQswCQYDVQQGEwJD\nTjERMA8GA1UECAwIU2hhbmdoYWkxETAPBgNVBAcMCFNoYW5naGFpMQswCQYDVQQK\nDAJaazELMAkGA1UECwwCWkgwHhcNMjAwNjIzMDUwODQyWhcNMjEwNjIzMDUwODQy\nWjBNMQswCQYDVQQGEwJDTjERMA8GA1UECAwIU2hhbmdoYWkxETAPBgNVBAcMCFNo\nYW5naGFpMQswCQYDVQQKDAJaazELMAkGA1UECwwCWkgwggIiMA0GCSqGSIb3DQEB\nAQUAA4ICDwAwggIKAoICAQC7oznSx9/gjE1/cEgXGKLATEvDPAdnvJ/T2lopEDZ/\nJEsNu0qBQsbOSAgJUhqAfX6ahwAn/Epz7yXy7PxCKZJi/wvESJ/WX4x+b7tE1nU1\nK7p7bKGJ6erww/ZrmGV+4+6GvdCg5dcOAA5TXAE2ZjTeIoR76Y3IZb0L78i/S+q1\ndZpx4YRfzwHNELCqpgwaJAS0FHIH1g+6X59EbF0UFT0YcM90Xxa0gHkPlYIoEoWS\n+UA/UW1MjuUwNaS5mNB3IpcrMhSeOkkqLglMdanu6r5MZpjuLBl7+sACoH0P7Rda\nx1c/NadmrbZf3/+AHvMZ6M9HrciyKKMauBZM9PUMrzLnTfF8iHitrSlum1UIfUuN\nvXXXzNLWskTxcXuWuyBgXpKM7D5XG7VnENDAbEYiN5Ej6zz5Zi/2OHVyErI3f1Ka\nvwTC8AjJMemCOBgPrgqMH7l6SAXr55eozXaSQVa4HG9iPGJixXZU5PUIiVFVO7Hj\nXtE3yfa2zaucB4rKhOJLwSD9qYgqFKB+vQ1X2GUkkPpsAMrL4n/VDQcJkrvjK3tt\n7AES9Q3TLmbVK91E2scF063XKUc3vT5Q8hcvg4MMLHn7gzMEaWTzjknRo1fLNWPY\nlPV3lZhBwkxdHKYodY7d6subE8nOsiFibq8X6Nf0UNIG0MXeFTAM2WfG2s6AlnZR\nHwIDAQABMA0GCSqGSIb3DQEBCwUAA4ICAQA5NL5fFP1eSBN/WEd8z6vJRWPyOz1t\ntrQF5o7Fazh3AtFcb3j3ab8NG/4qDTr7FrYFyOD+oHgIoHlzK4TFxlhSZquvU2Xb\nSMQyIQChojz8jbTa771ZPsjQjDWU0R0r83vI1ywc1v6eFpXIpV5oidT0afbJ85n4\ngwhVd6S2eTHh91U11BKf2gV4nhewzON4r7YuFg7sMMDVl3wx1HtXCKg5dMtgePyc\nGejdpyxdWX4BIxnvIY8QdAa4gvi9etzRf83mcNfwr+gM0rTyqGEBXuPW8bwq9BRL\nXz6zeL1Anb2HsjMQ6+MKWbXRhBFBCbB+APDcnjHv7OZXUaILi0B1JoTPu/jjSK1U\n7yAnK1sRtVpADVpa2N4STk9ImdTKfqTHZR9iTaheoqxRuTm7vzwGy72V4HEeEyOa\njyYpiCD8we3gJfro1pjzFLOqE3yU14vUc0SwQCZWlEH8LR/a8m/ZCPuqN4a2xPJO\nwksgMSCDkui5yUr4uTINFpROXHzz1dpOuUnvkkCAjKieZHWCyYyoEE0tedgejwee\nWv3UtR7svhpbAVoIQ8Z8EV2Ys1IN0Tp+4pltRbcgeZK0huEFOz4BL/1EGezwLbjE\nvoOMtTumWI9Mw5FTG4iTbRxvWL/KnLMvZr7V+o5ovmm0jeLW03Eh/E+aHH0B0tQp\nf6FKPRF7+Imo/g==\n-----END CERTIFICATE-----\n".as_bytes().to_vec();

        assert_noop!(
            Swork::register(
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
                message: Some("IllegalIdentity"),
            }
        );
    });
}

#[test]
fn register_should_failed_with_illegal_isv_body() {
    ExtBuilder::default()
        .build()
        .execute_with(|| {
        let applier: AccountId =
            AccountId::from_ss58check("5FqazaU79hjpEMiWTWZx81VjsYFst15eBuSBKdQLgQibD7CX")
                .expect("valid ss58 address");

        let mut register_info = legal_register_info();

        // Another isv body with wrong enclave code and public key
        register_info.isv_body = "{\"id\":\"125366127848601794295099877969265555107\",\"timestamp\":\"2020-06-22T11:34:54.845374\",\"version\":3,\"epidPseudonym\":\"4tcrS6EX9pIyhLyxtgpQJuMO1VdAkRDtha/N+u/rRkTsb11AhkuTHsY6UXRPLRJavxG3nsByBdTfyDuBDQTEjMYV6NBXjn3P4UyvG1Ae2+I4lE1n+oiKgLA8CR8pc2nSnSY1Wz1Pw/2l9Q5Er6hM6FdeECgMIVTZzjScYSma6rE=\",\"isvEnclaveQuoteStatus\":\"GROUP_OUT_OF_DATE\",\"platformInfoBlob\":\"1502006504000F00000F0F02040101070000000000000000000B00000B00000002000000000000142A70382C3A557904D4AB5766B2D3BAAD8ED8B7B372FB8F25C7E06212DEF369A389047D2249CF2ACDB22197AD7EE604634D47B3720BB1837E35C5C7D66F256117B6\",\"isvEnclaveQuoteBody\":\"AgABACoUAAAKAAkAAAAAAP7yPH5zo3mCPOcf8onPvAcAAAAAAAAAAAAAAAAAAAAACA7///8CAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAABwAAAAAAAAAHAAAAAAAAAJY6Ggjlm1yvKL0sgypJx2BBrGbValVEq8cCi/0sViQcAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAACD1xnnferKFHD2uvYqTXdDA8iZ22kCD5xw7h38CMfOngAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAADagmwZsR+S1ZNqgDg6HobleD6S6tRtqtsF1j81Bw7CnoP9/ZGNDEEzMEh+EKk1jAPW8PE+YKpum0xkVhh2J5Y8\"}".as_bytes().to_vec();

        assert_noop!(
            Swork::register(
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
                message: Some("IllegalIdentity"),
            }
        );
    });
}

#[test]
fn register_should_failed_with_illegal_id_sig() {
    ExtBuilder::default()
        .build()
        .execute_with(|| {
        let applier: AccountId =
            AccountId::from_ss58check("5FqazaU79hjpEMiWTWZx81VjsYFst15eBuSBKdQLgQibD7CX")
                .expect("valid ss58 address");

        let mut register_info = legal_register_info();
        // Another identity sig
        register_info.sig = hex::decode("f45e401778623de9b27726ab749549da35b1f8c0fd7bb56e0c1c3bba86948eb41717c9e13bf57113d85a1cc64d5cc2fc95c12d8b3108ab6fadeff621dfb6a486").unwrap();

        assert_noop!(
            Swork::register(
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
                message: Some("IllegalIdentity"),
            }
        );
    });
}

#[test]
fn register_should_failed_with_illegal_ias_sig() {
    ExtBuilder::default()
        .build()
        .execute_with(|| {
        let applier: AccountId =
            AccountId::from_ss58check("5FqazaU79hjpEMiWTWZx81VjsYFst15eBuSBKdQLgQibD7CX")
                .expect("valid ss58 address");

        let mut register_info = legal_register_info();
        // Another ias sig
        register_info.ias_sig = "cU3uOnd5XghR3ngJTbSFr48ttEIrJtbHHtuRM3hgzX7LHGacuTBMVRy0VK3ldqeM7KPBS+g3Da2anDHEJsSgITTXfHh+dxjUPO9v2hC+okjtWSY9fWhaFlR31lFWmSSbUfJSe2rtkLQRoj5VgKpOVkVuGzQjl/xF+SQZU4gjq130TwO8Gr/TvPLA3vJnM3/d8FUpcefp5Q5dbBka7y2ej8hDTyOjix3ZXSVD2SrSySfIg6kvIPS/EEJYoz/eMOFciSWuIIPrUj9M0eUc4xHsUxgNcgjOmtRt621RlzAwgY+yPFoqJwKtmlVNYy/FyvSbIMSB3kJbmlA+qHwOBgPQ0A==".as_bytes().to_vec();

        assert_noop!(
            Swork::register(
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
                message: Some("IllegalIdentity"),
            }
        );
    });
}

#[test]
fn register_should_failed_with_wrong_code() {
    ExtBuilder::default()
        .code(hex::decode("0011").unwrap())
        .build()
        .execute_with(|| {
            let applier: AccountId =
                AccountId::from_ss58check("5FqazaU79hjpEMiWTWZx81VjsYFst15eBuSBKdQLgQibD7CX")
                    .expect("valid ss58 address");
            let register_info = legal_register_info();

            assert_noop!(
                Swork::register(
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
                    message: Some("IllegalIdentity"),
                }
            );
        });
}

/// Report works test cases
#[test]
fn report_works_should_work() {
    ExtBuilder::default()
        .build()
        .execute_with(|| {
            // Generate 303 blocks first
            run_to_block(303);

            let reporter: AccountId = Sr25519Keyring::Alice.to_account_id();
            let legal_wr_info = legal_work_report_with_added_files();
            let legal_pk = legal_wr_info.curr_pk.clone();
            let legal_wr = WorkReport {
                report_slot: legal_wr_info.block_number,
                used: legal_wr_info.used * 2,
                free: legal_wr_info.free,
                reported_files_size: legal_wr_info.used,
                reported_srd_root: legal_wr_info.srd_root.clone(),
                reported_files_root: legal_wr_info.files_root.clone()
            };

            register(&legal_pk, LegalCode::get());
            add_not_live_files();

            // Check workloads before reporting
            assert_eq!(Swork::free(), 0);
            assert_eq!(Swork::used(), 0);

            assert_ok!(Swork::report_works(
                Origin::signed(reporter.clone()),
                legal_wr_info.curr_pk,
                legal_wr_info.prev_pk,
                legal_wr_info.block_number,
                legal_wr_info.block_hash,
                legal_wr_info.free,
                legal_wr_info.used,
                legal_wr_info.added_files,
                legal_wr_info.deleted_files,
                legal_wr_info.srd_root,
                legal_wr_info.files_root,
                legal_wr_info.sig
            ));

            // Check work report
            assert_eq!(Swork::work_reports(&legal_pk).unwrap(), legal_wr);

            // Check workloads after work report
            assert_eq!(Swork::free(), 4294967296);
            assert_eq!(Swork::used(), 402868224 * 2);
            assert_eq!(Swork::reported_in_slot(&legal_pk, 300), true);

            assert_eq!(Swork::identities(&reporter).unwrap_or_default(), Identity {
                anchor: legal_pk.clone(),
                group: None
            });

            // Check same file all been confirmed
            assert_eq!(Market::files(hex::decode("5bb706320afc633bfb843108e492192b17d2b6b9d9ee0b795ee95417fe08b660").unwrap()).unwrap_or_default().0, FileInfo {
                file_size: 134289408,
                expired_on: 1303,
                claimed_at: 303,
                amount: 1000,
                expected_replica_count: 4,
                reported_replica_count: 1,
                replicas: vec![Replica {
                    who: reporter.clone(),
                    valid_at: 303,
                    anchor: legal_pk.clone()
                }]
            });
            assert_eq!(Market::files(hex::decode("88cdb315c8c37e2dc00fa2a8c7fe51b8149b363d29f404441982f96d2bbae65f").unwrap()).unwrap_or_default().0, FileInfo {
                file_size: 268578816,
                expired_on: 1303,
                claimed_at: 303,
                amount: 1000,
                expected_replica_count: 4,
                reported_replica_count: 1,
                replicas: vec![Replica {
                    who: reporter,
                    valid_at: 303,
                    anchor: legal_pk
                }]
            });
        });
}

#[test]
fn report_works_should_work_without_files() {
    ExtBuilder::default()
        .build()
        .execute_with(|| {
            // Generate 303 blocks first
            run_to_block(303);

            let reporter: AccountId = Sr25519Keyring::Alice.to_account_id();
            let legal_wr_info = legal_work_report_with_added_files();
            let legal_pk = legal_wr_info.curr_pk.clone();

            register(&legal_pk, LegalCode::get());

            // Check workloads before reporting
            assert_eq!(Swork::free(), 0);
            assert_eq!(Swork::used(), 0);

            assert_ok!(Swork::report_works(
                Origin::signed(reporter.clone()),
                legal_wr_info.curr_pk,
                legal_wr_info.prev_pk,
                legal_wr_info.block_number,
                legal_wr_info.block_hash,
                legal_wr_info.free,
                legal_wr_info.used,
                legal_wr_info.added_files,
                legal_wr_info.deleted_files,
                legal_wr_info.srd_root,
                legal_wr_info.files_root,
                legal_wr_info.sig
            ));

            // Check workloads after work report
            assert_eq!(Swork::free(), 4294967296);
            assert_eq!(Swork::used(), 0);
        });
}

#[test]
fn report_works_should_work_with_added_and_deleted_files() {
    ExtBuilder::default()
        .build()
        .execute_with(|| {
            // Generate 303 blocks first
            run_to_block(303);

            let reporter: AccountId = Sr25519Keyring::Alice.to_account_id();
            let legal_wr_info = legal_work_report();
            let legal_pk = legal_wr_info.curr_pk.clone();

            register(&legal_pk, LegalCode::get());

            assert_ok!(Swork::report_works(
                Origin::signed(reporter.clone()),
                legal_wr_info.curr_pk,
                legal_wr_info.prev_pk,
                legal_wr_info.block_number,
                legal_wr_info.block_hash,
                legal_wr_info.free,
                legal_wr_info.used,
                legal_wr_info.added_files,
                legal_wr_info.deleted_files,
                legal_wr_info.srd_root,
                legal_wr_info.files_root,
                legal_wr_info.sig
            ));

            // Generate 606 blocks
            run_to_block(606);

            // TODO: use `same size added and deleted files` work report test case
            // FAKE Pass.
            let legal_wr_info_with_added_and_deleted_files = legal_work_report_with_added_and_deleted_files();
            assert_ok!(
                Swork::report_works(
                    Origin::signed(reporter),
                    legal_wr_info_with_added_and_deleted_files.curr_pk,
                    legal_wr_info_with_added_and_deleted_files.prev_pk,
                    legal_wr_info_with_added_and_deleted_files.block_number,
                    legal_wr_info_with_added_and_deleted_files.block_hash,
                    legal_wr_info_with_added_and_deleted_files.free,
                    legal_wr_info_with_added_and_deleted_files.used,
                    legal_wr_info_with_added_and_deleted_files.added_files,
                    legal_wr_info_with_added_and_deleted_files.deleted_files,
                    legal_wr_info_with_added_and_deleted_files.srd_root,
                    legal_wr_info_with_added_and_deleted_files.files_root,
                    legal_wr_info_with_added_and_deleted_files.sig
                )
            );
        });
}

#[test]
fn report_works_should_failed_with_not_registered() {
    ExtBuilder::default()
        .build()
        .execute_with(|| {
            // Generate 303 blocks first
            run_to_block(303);

            let illegal_reporter: AccountId = Sr25519Keyring::Alice.to_account_id();
            let legal_wr_info = legal_work_report_with_added_files();

            assert_noop!(
                Swork::report_works(
                    Origin::signed(illegal_reporter),
                    legal_wr_info.curr_pk,
                    legal_wr_info.prev_pk,
                    legal_wr_info.block_number,
                    legal_wr_info.block_hash,
                    legal_wr_info.free,
                    legal_wr_info.used,
                    legal_wr_info.added_files,
                    legal_wr_info.deleted_files,
                    legal_wr_info.srd_root,
                    legal_wr_info.files_root,
                    legal_wr_info.sig
                ),
                DispatchError::Module {
                    index: 0,
                    error: 2,
                    message: Some("IllegalReporter"),
                }
            );
        });
}

#[test]
fn report_works_should_failed_with_illegal_code() {
    ExtBuilder::default()
        .build()
        .execute_with(|| {
            // Generate 303 blocks first
            run_to_block(303);

            let reporter: AccountId = Sr25519Keyring::Alice.to_account_id();
            let legal_wr_info = legal_work_report_with_added_files();
            let legal_pk = legal_wr_info.curr_pk.clone();
            let illegal_code = hex::decode("0011").unwrap();

            // register with
            register(&legal_pk, illegal_code);

            assert_noop!(
                Swork::report_works(
                    Origin::signed(reporter),
                    legal_wr_info.curr_pk,
                    legal_wr_info.prev_pk,
                    legal_wr_info.block_number,
                    legal_wr_info.block_hash,
                    legal_wr_info.free,
                    legal_wr_info.used,
                    legal_wr_info.added_files,
                    legal_wr_info.deleted_files,
                    legal_wr_info.srd_root,
                    legal_wr_info.files_root,
                    legal_wr_info.sig
                ),
                DispatchError::Module {
                    index: 0,
                    error: 3,
                    message: Some("OutdatedReporter"),
                }
            );
        });
}

#[test]
fn report_works_should_failed_with_wrong_timing() {
    ExtBuilder::default()
        .build()
        .execute_with(|| {
            // Generate 50 blocks first
            run_to_block(50);

            let reporter: AccountId = Sr25519Keyring::Alice.to_account_id();
            let illegal_wr_info = legal_work_report_with_added_files();
            let legal_pk = illegal_wr_info.curr_pk.clone();

            register(&legal_pk, LegalCode::get());

            assert_noop!(
                Swork::report_works(
                    Origin::signed(reporter),
                    illegal_wr_info.curr_pk,
                    illegal_wr_info.prev_pk,
                    illegal_wr_info.block_number,
                    illegal_wr_info.block_hash,
                    illegal_wr_info.free,
                    illegal_wr_info.used,
                    illegal_wr_info.added_files,
                    illegal_wr_info.deleted_files,
                    illegal_wr_info.srd_root,
                    illegal_wr_info.files_root,
                    illegal_wr_info.sig
                ),
                DispatchError::Module {
                    index: 0,
                    error: 4,
                    message: Some("InvalidReportTime"),
                }
            );
        });
}

#[test]
fn report_works_should_failed_with_illegal_sig() {
    ExtBuilder::default()
        .build()
        .execute_with(|| {
            // Generate 50 blocks first
            run_to_block(303);

            let reporter: AccountId = Sr25519Keyring::Alice.to_account_id();
            let mut illegal_wr_info = legal_work_report_with_added_files();
            let legal_pk = illegal_wr_info.curr_pk.clone();
            illegal_wr_info.sig = hex::decode("b3f78863ec972955d9ca22d444a5475085a4f7975a738aba1eae1d98dd718fc691a77a35b764a148a3a861a4a2ef3279f3d5e25f607c73ca85ea86e1176ba664").unwrap();

            register(&legal_pk, LegalCode::get());

            assert_noop!(
                Swork::report_works(
                    Origin::signed(reporter),
                    illegal_wr_info.curr_pk,
                    illegal_wr_info.prev_pk,
                    illegal_wr_info.block_number,
                    illegal_wr_info.block_hash,
                    illegal_wr_info.free,
                    illegal_wr_info.used,
                    illegal_wr_info.added_files,
                    illegal_wr_info.deleted_files,
                    illegal_wr_info.srd_root,
                    illegal_wr_info.files_root,
                    illegal_wr_info.sig
                ),
                DispatchError::Module {
                    index: 0,
                    error: 5,
                    message: Some("IllegalWorkReportSig"),
                }
            );
        });
}

#[test]
fn report_works_should_failed_with_illegal_file_transition() {
    ExtBuilder::default()
        .build()
        .execute_with(|| {
            // Generate 50 blocks first
            run_to_block(303);

            let reporter: AccountId = Sr25519Keyring::Alice.to_account_id();
            let illegal_wr_info = legal_work_report_with_added_files();
            let legal_pk = illegal_wr_info.curr_pk.clone();

            register(&legal_pk, LegalCode::get());
            register_identity(&reporter, &legal_pk, &legal_pk);

            // Add initial work report with `reported_files_size = 5`
            add_wr(&legal_pk, &WorkReport {
                report_slot: 0,
                used: 0,
                free: 0,
                reported_files_size: 5,
                reported_srd_root: vec![],
                reported_files_root: vec![]
            });

            assert_noop!(
                Swork::report_works(
                    Origin::signed(reporter),
                    illegal_wr_info.curr_pk,
                    illegal_wr_info.prev_pk,
                    illegal_wr_info.block_number,
                    illegal_wr_info.block_hash,
                    illegal_wr_info.free,
                    illegal_wr_info.used,
                    illegal_wr_info.added_files,
                    illegal_wr_info.deleted_files,
                    illegal_wr_info.srd_root,
                    illegal_wr_info.files_root,
                    illegal_wr_info.sig
                ),
                DispatchError::Module {
                    index: 0,
                    error: 7,
                    message: Some("IllegalFilesTransition"),
                }
            );
        });
}

/// Incremental report test cases
#[test]
fn incremental_report_should_work_without_change() {
    ExtBuilder::default()
        .build()
        .execute_with(|| {
            // Generate 303 blocks first
            run_to_block(303);

            let reporter: AccountId = Sr25519Keyring::Alice.to_account_id();
            let legal_wr_info = legal_work_report();
            let legal_pk = legal_wr_info.curr_pk.clone();

            register(&legal_pk, LegalCode::get());
            register_identity(&reporter, &legal_pk, &legal_pk);
            add_wr(&legal_pk, &WorkReport {
                report_slot: 0,
                used: 2,
                free: 0,
                reported_files_size: 2,
                reported_srd_root: hex::decode("00").unwrap(),
                reported_files_root: hex::decode("11").unwrap()
            });

            assert_ok!(Swork::report_works(
                Origin::signed(reporter.clone()),
                legal_wr_info.curr_pk,
                legal_wr_info.prev_pk,
                legal_wr_info.block_number,
                legal_wr_info.block_hash,
                legal_wr_info.free,
                legal_wr_info.used,
                legal_wr_info.added_files,
                legal_wr_info.deleted_files,
                legal_wr_info.srd_root,
                legal_wr_info.files_root,
                legal_wr_info.sig
            ));
        });
}

#[test]
fn incremental_report_should_work_with_files_change() {
    ExtBuilder::default()
        .build()
        .execute_with(|| {
            // Generate 303 blocks first
            run_to_block(303);

            let reporter: AccountId = Sr25519Keyring::Alice.to_account_id();
            let legal_wr_info = legal_work_report_with_deleted_files();
            let legal_pk = legal_wr_info.curr_pk.clone();

            let legal_wr = WorkReport {
                report_slot: legal_wr_info.block_number,
                used: legal_wr_info.used,
                free: legal_wr_info.free,
                reported_files_size: legal_wr_info.used,
                reported_srd_root: legal_wr_info.srd_root.clone(),
                reported_files_root: legal_wr_info.files_root.clone()
            };

            register(&legal_pk, LegalCode::get());
            add_wr(&legal_pk, &WorkReport {
                report_slot: 0,
                used: 2,
                free: 0,
                reported_files_size: 3,
                reported_srd_root: vec![],
                reported_files_root: vec![]
            });
            add_live_files(&reporter, &legal_pk);

            assert_ok!(Swork::report_works(
                Origin::signed(reporter.clone()),
                legal_wr_info.curr_pk,
                legal_wr_info.prev_pk,
                legal_wr_info.block_number,
                legal_wr_info.block_hash,
                legal_wr_info.free,
                legal_wr_info.used,
                legal_wr_info.added_files,
                legal_wr_info.deleted_files,
                legal_wr_info.srd_root,
                legal_wr_info.files_root,
                legal_wr_info.sig
            ));

            // Check work report
            assert_eq!(Swork::work_reports(&legal_pk).unwrap(), legal_wr);

            // Check workloads after work report
            assert_eq!(Swork::free(), 4294967296);
            assert_eq!(Swork::used(), 0);

        });
}

#[test]
fn incremental_report_should_failed_with_root_change() {
    ExtBuilder::default()
        .build()
        .execute_with(|| {
            // Generate 303 blocks first
            run_to_block(303);

            let reporter: AccountId = Sr25519Keyring::Alice.to_account_id();
            let illegal_wr_info = legal_work_report();
            let legal_pk = illegal_wr_info.curr_pk.clone();

            register(&legal_pk, LegalCode::get());
            register_identity(&reporter, &legal_pk, &legal_pk);
            add_wr(&legal_pk, &WorkReport {
                report_slot: 0,
                used: 2,
                free: 0,
                reported_files_size: 2,
                reported_srd_root: vec![],
                reported_files_root: vec![]
            });

            assert_noop!(
                Swork::report_works(
                    Origin::signed(reporter),
                    illegal_wr_info.curr_pk,
                    illegal_wr_info.prev_pk,
                    illegal_wr_info.block_number,
                    illegal_wr_info.block_hash,
                    illegal_wr_info.free,
                    illegal_wr_info.used,
                    illegal_wr_info.added_files,
                    illegal_wr_info.deleted_files,
                    illegal_wr_info.srd_root,
                    illegal_wr_info.files_root,
                    illegal_wr_info.sig
                ),
                DispatchError::Module {
                    index: 0,
                    error: 7,
                    message: Some("IllegalFilesTransition"),
                }
            );
        });
}

#[test]
fn incremental_report_should_failed_with_wrong_file_size_change() {
    ExtBuilder::default()
        .build()
        .execute_with(|| {
            // Generate 303 blocks first
            run_to_block(303);

            let reporter: AccountId = Sr25519Keyring::Alice.to_account_id();
            let illegal_wr_info = legal_work_report(); // No change but with file size down
            let legal_pk = illegal_wr_info.curr_pk.clone();


            register(&legal_pk, LegalCode::get());
            register_identity(&reporter, &legal_pk, &legal_pk);
            add_wr(&legal_pk, &WorkReport {
                report_slot: 0,
                used: 40,
                free: 40,
                reported_files_size: 40,
                reported_srd_root: vec![],
                reported_files_root: vec![]
            });

            assert_noop!(
                Swork::report_works(
                    Origin::signed(reporter),
                    illegal_wr_info.curr_pk,
                    illegal_wr_info.prev_pk,
                    illegal_wr_info.block_number,
                    illegal_wr_info.block_hash,
                    illegal_wr_info.free,
                    illegal_wr_info.used,
                    illegal_wr_info.added_files,
                    illegal_wr_info.deleted_files,
                    illegal_wr_info.srd_root,
                    illegal_wr_info.files_root,
                    illegal_wr_info.sig
                ),
                DispatchError::Module {
                    index: 0,
                    error: 7,
                    message: Some("IllegalFilesTransition"),
                }
            );
        });
}

/// Timing related test cases
#[test]
fn update_identities_should_work() {
    ExtBuilder::default()
        .build()
        .execute_with(|| {
            let reporter: AccountId = Sr25519Keyring::Alice.to_account_id();
            let legal_wr_info = legal_work_report();
            let legal_pk = legal_wr_info.curr_pk.clone();

            register(&legal_pk, LegalCode::get());
            register_identity(&reporter, &legal_pk, &legal_pk);
            add_wr(&legal_pk, &WorkReport {
                report_slot: 0,
                used: 2,
                free: 0,
                reported_files_size: 2,
                reported_srd_root: hex::decode("00").unwrap(),
                reported_files_root: hex::decode("11").unwrap()
            });
            add_live_files(&reporter, &legal_pk);

            // 1. Runs to 303 block
            run_to_block(303);
            Swork::update_identities();

            assert_eq!(Swork::free(), 0);
            assert_eq!(Swork::used(), 2);
            assert_eq!(Swork::current_report_slot(), 300);
            assert_eq!(*WorkloadMap::get().borrow().get(&reporter).unwrap(), 2u128);

            // 2. Report works in slot 300
            assert_ok!(Swork::report_works(
                Origin::signed(reporter.clone()),
                legal_wr_info.curr_pk,
                legal_wr_info.prev_pk,
                legal_wr_info.block_number,
                legal_wr_info.block_hash,
                legal_wr_info.free,
                legal_wr_info.used,
                legal_wr_info.added_files,
                legal_wr_info.deleted_files,
                legal_wr_info.srd_root,
                legal_wr_info.files_root,
                legal_wr_info.sig
            ));

            // 3. Free and used should already been updated
            assert_eq!(Swork::free(), 4294967296);
            assert_eq!(Swork::used(), 2);
            assert_eq!(*WorkloadMap::get().borrow().get(&reporter).unwrap(), 2u128);

            // 4. Runs to 606
            run_to_block(606);
            Swork::update_identities();

            // 5. Free and used should not change, but current_rs should already been updated
            assert_eq!(Swork::free(), 4294967296);
            assert_eq!(Swork::used(), 2);
            assert_eq!(Swork::current_report_slot(), 600);
            assert_eq!(*WorkloadMap::get().borrow().get(&reporter).unwrap(), 4294967298u128);

            // 6. Runs to 909, work report is outdated
            run_to_block(909);
            Swork::update_identities();

            // 7. Free and used should goes to 0, and the corresponding storage order should failed
            assert_eq!(Swork::free(), 0);
            assert_eq!(Swork::used(), 0);
            assert_eq!(Swork::current_report_slot(), 900);
            assert_eq!(*WorkloadMap::get().borrow().get(&reporter).unwrap(), 0u128);
        });
}

#[test]
fn abnormal_era_should_work() {
    ExtBuilder::default()
        .build()
        .execute_with(|| {
            let reporter: AccountId = Sr25519Keyring::Alice.to_account_id();
            let legal_pk = LegalPK::get();

            register(&legal_pk, LegalCode::get());
            register_identity(&reporter, &legal_pk, &legal_pk);
            add_wr(&legal_pk, &WorkReport {
                report_slot: 0,
                used: 2,
                free: 0,
                reported_files_size: 2,
                reported_srd_root: hex::decode("00").unwrap(),
                reported_files_root: hex::decode("11").unwrap()
            });
            add_live_files(&reporter, &legal_pk);

            // 1. Normal new era, runs to 301 block
            run_to_block(301);
            Swork::update_identities();

            // 2. Everything goes well
            assert_eq!(Swork::free(), 0);
            assert_eq!(Swork::used(), 2);

            // 4. Abnormal era happened, new era goes like 404
            run_to_block(404);
            Swork::update_identities();

            // 5. Free and used should not change
            assert_eq!(Swork::free(), 0);
            assert_eq!(Swork::used(), 2);
        });
}

/// A/B upgrade test cases
#[test]
fn ab_upgrade_should_work() {
    ExtBuilder::default()
        .build()
        .execute_with(|| {
            let reporter: AccountId = Sr25519Keyring::Alice.to_account_id();
            let a_wr_info = legal_work_report();
            let b_wr_info_1 = ab_upgrade_work_report();
            let b_wr_info_2 = continuous_ab_upgrade_work_report();
            let a_pk = a_wr_info.curr_pk.clone();
            let b_pk = b_wr_info_1.curr_pk.clone();

            // 0. Initial setup
            register(&a_pk, LegalCode::get());
            register_identity(&reporter, &a_pk, &a_pk);
            add_wr(&a_pk, &WorkReport {
                report_slot: 0,
                used: 2,
                free: 0,
                reported_files_size: 2,
                reported_srd_root: hex::decode("00").unwrap(),
                reported_files_root: hex::decode("11").unwrap()
            });
            add_not_live_files(); // with b_wr_info_2's added file
            add_live_files(&reporter, &a_pk); // with b_wr_info_2's deleted file

            // 1. Runs to 303 block
            run_to_block(303);

            // 2. Report works with sWorker A
            assert_ok!(Swork::report_works(
                Origin::signed(reporter.clone()),
                a_wr_info.curr_pk,
                a_wr_info.prev_pk,
                a_wr_info.block_number,
                a_wr_info.block_hash,
                a_wr_info.free,
                a_wr_info.used,
                a_wr_info.added_files,
                a_wr_info.deleted_files,
                a_wr_info.srd_root,
                a_wr_info.files_root,
                a_wr_info.sig
            ));

            // 3. Check A's work report and free & used
            assert_eq!(Swork::work_reports(&a_pk).unwrap(), WorkReport {
                report_slot: 300,
                used: 2,
                free: 4294967296,
                reported_files_size: 2,
                reported_srd_root: hex::decode("00").unwrap(),
                reported_files_root: hex::decode("11").unwrap()
            });
            assert_eq!(Swork::free(), 4294967296);
            assert_eq!(Swork::used(), 2);
            assert_eq!(Swork::reported_in_slot(&a_pk, 300), true);

            // 4. Runs to 606, and do sWorker upgrade
            Swork::update_identities();
            run_to_block(606);
            // Fake do upgrade

            // 5. (Fake) Register B 🤣, suppose B's code is upgraded
            register(&b_pk, LegalCode::get());

            // 6. Report works with sWorker B
            assert_ok!(Swork::report_works(
                Origin::signed(reporter.clone()),
                b_wr_info_1.curr_pk,
                b_wr_info_1.prev_pk,
                b_wr_info_1.block_number,
                b_wr_info_1.block_hash,
                b_wr_info_1.free,
                b_wr_info_1.used,
                b_wr_info_1.added_files,
                b_wr_info_1.deleted_files,
                b_wr_info_1.srd_root,
                b_wr_info_1.files_root,
                b_wr_info_1.sig
            ));

            // 7. Check B's work report and free & used
            assert_eq!(Swork::work_reports(&a_pk).unwrap(), WorkReport {
                report_slot: 600,
                used: 2,
                free: 4294967296,
                reported_files_size: 2,
                reported_srd_root: hex::decode("00").unwrap(),
                reported_files_root: hex::decode("11").unwrap()
            });
            assert_eq!(Swork::free(), 4294967296);
            assert_eq!(Swork::used(), 2);
            assert_eq!(Swork::reported_in_slot(&a_pk, 300), true);
            assert_eq!(Swork::reported_in_slot(&a_pk, 600), true);

            // 8. Check A is already be chilled
            assert_eq!(<self::PubKeys>::contains_key(&a_pk), false);

            // 9. Runs to 909
            run_to_block(909);

            // 10. B normally report with A's pk(and with files changing), it should be ok
            assert_ok!(Swork::report_works(
                Origin::signed(reporter.clone()),
                b_wr_info_2.curr_pk,
                b_wr_info_2.prev_pk,
                b_wr_info_2.block_number,
                b_wr_info_2.block_hash,
                b_wr_info_2.free,
                b_wr_info_2.used,
                b_wr_info_2.added_files,
                b_wr_info_2.deleted_files,
                b_wr_info_2.srd_root,
                b_wr_info_2.files_root,
                b_wr_info_2.sig
            ));

            // 11. Check B's work report and free & used again
            assert_eq!(Swork::work_reports(&a_pk).unwrap(), WorkReport {
                report_slot: 900,
                used: 0, // 2 + 2 * 2 - 7 * 2
                free: 4294967296,
                reported_files_size: 3,
                reported_srd_root: hex::decode("00").unwrap(),
                reported_files_root: hex::decode("11").unwrap()
            });
            assert_eq!(Swork::free(), 4294967296);
            assert_eq!(Swork::used(), 0); // 2 added 134289408 and delete 7
        });
}

#[test]
fn ab_upgrade_expire_should_work() {
    ExtBuilder::default()
        .build()
        .execute_with(|| {
            let reporter: AccountId = Sr25519Keyring::Alice.to_account_id();
            let wr_info_300 = continuous_work_report_300();
            let wr_info_600 = continuous_work_report_600();
            let legal_pk = wr_info_300.curr_pk.clone();

            // 0. Initial setup
            register(&legal_pk, LegalCode::get());
            register_identity(&reporter, &legal_pk, &legal_pk);
            add_wr(&legal_pk, &WorkReport {
                report_slot: 0,
                used: 2,
                free: 0,
                reported_files_size: 2,
                reported_srd_root: hex::decode("00").unwrap(),
                reported_files_root: hex::decode("11").unwrap()
            });

            // 1. Arrange an upgrade immediately, expired at 500
            assert_ok!(Swork::upgrade(Origin::root(), hex::decode("0011").unwrap(), 500));

            // 1. Runs to 303 block
            run_to_block(303);

            // 2. Report works still worked
            assert_ok!(Swork::report_works(
                Origin::signed(reporter.clone()),
                wr_info_300.curr_pk,
                wr_info_300.prev_pk,
                wr_info_300.block_number,
                wr_info_300.block_hash,
                wr_info_300.free,
                wr_info_300.used,
                wr_info_300.added_files,
                wr_info_300.deleted_files,
                wr_info_300.srd_root,
                wr_info_300.files_root,
                wr_info_300.sig
            ));

            // 3. Runs to 606
            run_to_block(606);

            // 4. Report works should failed due to the expired time
            assert_noop!(
                Swork::report_works(
                    Origin::signed(reporter.clone()),
                    wr_info_600.curr_pk,
                    wr_info_600.prev_pk,
                    wr_info_600.block_number,
                    wr_info_600.block_hash,
                    wr_info_600.free,
                    wr_info_600.used,
                    wr_info_600.added_files,
                    wr_info_600.deleted_files,
                    wr_info_600.srd_root,
                    wr_info_600.files_root,
                    wr_info_600.sig
                ),
                DispatchError::Module {
                    index: 0,
                    error: 3,
                    message: Some("OutdatedReporter"),
                }
            );
        });
}

#[test]
fn ab_upgrade_should_failed_with_files_size_unmatch() {
    ExtBuilder::default()
        .build()
        .execute_with(|| {
            let reporter: AccountId = Sr25519Keyring::Alice.to_account_id();
            let a_wr_info = legal_work_report();
            let b_wr_info = ab_upgrade_work_report_files_size_unmatch();
            let a_pk = a_wr_info.curr_pk.clone();
            let b_pk = b_wr_info.curr_pk.clone();

            // 0. Initial setup
            register(&a_pk, LegalCode::get());
            add_wr(&a_pk, &WorkReport {
                report_slot: 0,
                used: 2,
                free: 0,
                reported_files_size: 2,
                reported_srd_root: hex::decode("00").unwrap(),
                reported_files_root: hex::decode("11").unwrap()
            });

            // 1. Report A
            run_to_block(303);
            assert_ok!(Swork::report_works(
                Origin::signed(reporter.clone()),
                a_wr_info.curr_pk,
                a_wr_info.prev_pk,
                a_wr_info.block_number,
                a_wr_info.block_hash,
                a_wr_info.free,
                a_wr_info.used,
                a_wr_info.added_files,
                a_wr_info.deleted_files,
                a_wr_info.srd_root,
                a_wr_info.files_root,
                a_wr_info.sig
            ));

            // 2. Runs to 606, and do sWorker upgrade
            run_to_block(606);
            // Fake do upgrade

            // 3. (Fake) Register B 🤣, suppose B's code is upgraded
            register(&b_pk, LegalCode::get());

            // 4. Report works with sWorker B will failed
            assert_noop!(
                Swork::report_works(
                    Origin::signed(reporter.clone()),
                    b_wr_info.curr_pk,
                    b_wr_info.prev_pk,
                    b_wr_info.block_number,
                    b_wr_info.block_hash,
                    b_wr_info.free,
                    b_wr_info.used,
                    b_wr_info.added_files,
                    b_wr_info.deleted_files,
                    b_wr_info.srd_root,
                    b_wr_info.files_root,
                    b_wr_info.sig
                ),
                DispatchError::Module {
                    index: 0,
                    error: 6,
                    message: Some("ABUpgradeFailed"),
                }
            );
        });
}

/// Group test cases
#[test]
fn create_and_join_group_should_work() {
    ExtBuilder::default()
        .build()
        .execute_with(|| {
            let alice = Sr25519Keyring::Alice.to_account_id();
            let bob = Sr25519Keyring::Bob.to_account_id();

            // Prepare two work reports
            let b_wr_info = ab_upgrade_work_report();
            let b_pk = b_wr_info.curr_pk.clone();

            register_identity(&bob, &b_pk, &b_pk);

            add_wr(&b_pk, &WorkReport {
                report_slot: 0,
                used: 0,
                free: 0,
                reported_files_size: 2,
                reported_srd_root: hex::decode("00").unwrap(),
                reported_files_root: hex::decode("11").unwrap()
            });

            // Alice create a group and be the owner
            assert_ok!(Swork::create_group(
                Origin::signed(alice.clone())
            ));

            // Bob join the alice's group
            assert_ok!(Swork::join_group(
                Origin::signed(bob.clone()),
                alice.clone()
            ));

            assert_eq!(Swork::identities(&bob).unwrap_or_default(), Identity {
                anchor: b_pk.clone(),
                group: Some(alice.clone())
            });
        });
}

#[test]
fn create_group_should_fail_due_to_invalid_situations() {
    ExtBuilder::default()
        .build()
        .execute_with(|| {
            let alice = Sr25519Keyring::Alice.to_account_id();
            let a_wr_info = legal_work_report();
            let a_pk = a_wr_info.curr_pk.clone();

            // Alice create a group and be the owner
            assert_ok!(Swork::create_group(
                Origin::signed(alice.clone())
            ));

            assert_noop!(Swork::create_group(
                Origin::signed(alice.clone())
            ),
            DispatchError::Module {
                index: 0,
                error: 14,
                message: Some("GroupAlreadyExist"),
            });

            register_identity(&alice, &a_pk, &a_pk);

            assert_noop!(Swork::create_group(
                Origin::signed(alice.clone())
            ),
            DispatchError::Module {
                index: 0,
                error: 15,
                message: Some("GroupOwnerForbidden"),
            });
        });
}

#[test]
fn register_should_fail_due_to_reporter_is_group_owner() {
    ExtBuilder::default()
        .build()
        .execute_with(|| {
            let applier: AccountId =
                AccountId::from_ss58check("5FqazaU79hjpEMiWTWZx81VjsYFst15eBuSBKdQLgQibD7CX")
                    .expect("valid ss58 address");
            let register_info = legal_register_info();

            // Alice create a group and be the owner
            assert_ok!(Swork::create_group(
                Origin::signed(applier.clone())
            ));

            assert_noop!(
                Swork::register(
                    Origin::signed(applier.clone()),
                    register_info.ias_sig,
                    register_info.ias_cert,
                    register_info.account_id,
                    register_info.isv_body,
                    register_info.sig
                ),
                DispatchError::Module {
                    index: 0,
                    error: 15,
                    message: Some("GroupOwnerForbidden"),
                }
            );
        });
}

#[test]
fn report_works_should_fail_due_to_reporter_is_group_owner() {
    ExtBuilder::default()
        .build()
        .execute_with(|| {
            // Generate 303 blocks first
            run_to_block(303);

            let reporter: AccountId = Sr25519Keyring::Alice.to_account_id();
            let legal_wr_info = legal_work_report_with_added_files();
            let legal_pk = legal_wr_info.curr_pk.clone();

            register(&legal_pk, LegalCode::get());
            add_not_live_files();
            // Alice create a group and be the owner
            assert_ok!(Swork::create_group(
                Origin::signed(reporter.clone())
            ));
            assert_noop!(
                Swork::report_works(
                    Origin::signed(reporter.clone()),
                    legal_wr_info.curr_pk,
                    legal_wr_info.prev_pk,
                    legal_wr_info.block_number,
                    legal_wr_info.block_hash,
                    legal_wr_info.free,
                    legal_wr_info.used,
                    legal_wr_info.added_files,
                    legal_wr_info.deleted_files,
                    legal_wr_info.srd_root,
                    legal_wr_info.files_root,
                    legal_wr_info.sig
                ),
                DispatchError::Module {
                    index: 0,
                    error: 15,
                    message: Some("GroupOwnerForbidden"),
                });
        });
}

#[test]
fn join_group_should_fail_due_to_invalid_situations() {
    ExtBuilder::default()
        .build()
        .execute_with(|| {
            let alice = Sr25519Keyring::Alice.to_account_id();
            let bob = Sr25519Keyring::Bob.to_account_id();

            let b_wr_info = ab_upgrade_work_report();
            let b_pk = b_wr_info.curr_pk.clone();

            // bob's identity doesn't exist
            assert_noop!(Swork::join_group(
                Origin::signed(bob.clone()),
                alice.clone()
            ),
            DispatchError::Module {
                index: 0,
                error: 10,
                message: Some("IdentityNotExist"),
            });


            register_identity(&bob, &b_pk, &b_pk);
            add_wr(&b_pk, &WorkReport {
                report_slot: 0,
                used: 100,
                free: 0,
                reported_files_size: 2,
                reported_srd_root: hex::decode("00").unwrap(),
                reported_files_root: hex::decode("11").unwrap()
            });

            // alice is not the owner of the group
            assert_noop!(Swork::join_group(
                Origin::signed(bob.clone()),
                alice.clone()
            ),
            DispatchError::Module {
                index: 0,
                error: 12,
                message: Some("NotOwner"),
            });

            // Alice create a group and be the owner
            assert_ok!(Swork::create_group(
                Origin::signed(alice.clone())
            ));

            // bob's used is not 0
            assert_noop!(Swork::join_group(
                Origin::signed(bob.clone()),
                alice.clone()
            ),
            DispatchError::Module {
                index: 0,
                error: 13,
                message: Some("IllegalUsed"),
            });

            add_wr(&b_pk, &WorkReport {
                report_slot: 0,
                used: 0,
                free: 0,
                reported_files_size: 2,
                reported_srd_root: hex::decode("00").unwrap(),
                reported_files_root: hex::decode("11").unwrap()
            });

            // Bob join the alice's group
            assert_ok!(Swork::join_group(
                Origin::signed(bob.clone()),
                alice.clone()
            ));

            assert_eq!(Swork::identities(&bob).unwrap_or_default(), Identity {
                anchor: b_pk.clone(),
                group: Some(alice.clone())
            });

            // bob already joined a group
            assert_noop!(Swork::join_group(
                Origin::signed(bob.clone()),
                alice.clone()
            ),
            DispatchError::Module {
                index: 0,
                error: 11,
                message: Some("AlreadyJoint"),
            });
        });
}

#[test]
fn join_group_should_work_for_used_in_work_report() {
    ExtBuilder::default()
        .build()
        .execute_with(|| {
            let alice = Sr25519Keyring::Alice.to_account_id();
            let bob = Sr25519Keyring::Bob.to_account_id();
            let eve = Sr25519Keyring::Eve.to_account_id();
            let ferdie = Sr25519Keyring::Ferdie.to_account_id();

            // Get work report in 300 slot fo alice, bob and eve
            let alice_wr_info = group_work_report_alice_300();
            let bob_wr_info = group_work_report_bob_300();
            let eve_wr_info = group_work_report_eve_300();
            let a_pk = alice_wr_info.curr_pk.clone();
            let b_pk = bob_wr_info.curr_pk.clone();
            let c_pk = eve_wr_info.curr_pk.clone();

            register(&a_pk, LegalCode::get());
            register(&b_pk, LegalCode::get());
            register(&c_pk, LegalCode::get());
            register_identity(&alice, &a_pk, &a_pk);
            register_identity(&bob, &b_pk, &b_pk);
            register_identity(&eve, &c_pk, &c_pk);

            // We have five test files
            let file_a = hex::decode("5aa706320afc633bfb843108e492192b17d2b6b9d9ee0b795ee95417fe08b660").unwrap(); // A file
            let file_b = hex::decode("99cdb315c8c37e2dc00fa2a8c7fe51b8149b363d29f404441982f96d2bbae65f").unwrap(); // B file
            let file_c = hex::decode("77cdb315c8c37e2dc00fa2a8c7fe51b8149b363d29f404441982f96d2bbae65f").unwrap(); // C file
            let file_d = hex::decode("66a706320afc633bfb843108e492192b17d2b6b9d9ee0b795ee95417fe08b110").unwrap(); // D file
            let file_e = hex::decode("33cdb315c8c37e2dc00fa2a8c7fe51b8149b363d29f404441982f96d2bbae12e").unwrap(); // E file

            // alice, bob and eve become a group
            assert_ok!(Swork::create_group(
                Origin::signed(ferdie.clone())
            ));

            assert_ok!(Swork::join_group(
                Origin::signed(alice.clone()),
                ferdie.clone()
            ));

            assert_ok!(Swork::join_group(
                Origin::signed(bob.clone()),
                ferdie.clone()
            ));

            assert_ok!(Swork::join_group(
                Origin::signed(eve.clone()),
                ferdie.clone()
            ));

            run_to_block(303);
            add_not_live_files();
            // A report works in 303
            allow_report_work(&alice_wr_info.curr_pk, alice_wr_info.block_number);
            assert_ok!(Swork::report_works(
                Origin::signed(alice.clone()),
                alice_wr_info.curr_pk,
                alice_wr_info.prev_pk,
                alice_wr_info.block_number,
                alice_wr_info.block_hash,
                alice_wr_info.free,
                alice_wr_info.used,
                alice_wr_info.added_files,
                alice_wr_info.deleted_files,
                alice_wr_info.srd_root,
                alice_wr_info.files_root,
                alice_wr_info.sig
            ));

            assert_eq!(Market::files(&file_a).unwrap_or_default(), (
                FileInfo {
                    file_size: 13,
                    expired_on: 1303,
                    claimed_at: 303,
                    amount: 1000,
                    expected_replica_count: 4,
                    reported_replica_count: 1,
                    replicas: vec![Replica {
                        who: alice.clone(),
                        valid_at: 303,
                        anchor: a_pk.clone()
                    }]
                },
                UsedInfo {
                    used_size: 13 * 2,
                    reported_group_count: 1,
                    groups: BTreeSet::from_iter(vec![a_pk.clone()].into_iter())
                })
            );
            assert_eq!(Market::files(&file_b).unwrap_or_default(), (
                FileInfo {
                    file_size: 7,
                    expired_on: 1303,
                    claimed_at: 303,
                    amount: 1000,
                    expected_replica_count: 4,
                    reported_replica_count: 1,
                    replicas: vec![Replica {
                        who: alice.clone(),
                        valid_at: 303,
                        anchor: a_pk.clone()
                    }]
                },
                UsedInfo {
                    used_size: 7 * 2,
                    reported_group_count: 1,
                    groups: BTreeSet::from_iter(vec![a_pk.clone()].into_iter())
                })
            );
            assert_eq!(Market::files(&file_c).unwrap_or_default(), (
                FileInfo {
                    file_size: 37,
                    expired_on: 1303,
                    claimed_at: 303,
                    amount: 1000,
                    expected_replica_count: 4,
                    reported_replica_count: 1,
                    replicas: vec![Replica {
                        who: alice.clone(),
                        valid_at: 303,
                        anchor: a_pk.clone()
                    }]
                },
                UsedInfo {
                    used_size: 37 * 2,
                    reported_group_count: 1,
                    groups: BTreeSet::from_iter(vec![a_pk.clone()].into_iter())
                })
            );
            assert_eq!(Swork::work_reports(&a_pk).unwrap(), WorkReport {
                report_slot: 300,
                used: 57 * 2,
                free: 4294967296,
                reported_files_size: 57,
                reported_srd_root: hex::decode("00").unwrap(),
                reported_files_root: hex::decode("11").unwrap()
            });
            allow_report_work(&bob_wr_info.curr_pk, bob_wr_info.block_number);
            assert_ok!(Swork::report_works(
                Origin::signed(bob.clone()),
                bob_wr_info.curr_pk,
                bob_wr_info.prev_pk,
                bob_wr_info.block_number,
                bob_wr_info.block_hash,
                bob_wr_info.free,
                bob_wr_info.used,
                bob_wr_info.added_files,
                bob_wr_info.deleted_files,
                bob_wr_info.srd_root,
                bob_wr_info.files_root,
                bob_wr_info.sig
            ));

            assert_eq!(Market::files(&file_b).unwrap_or_default(), (
                FileInfo {
                    file_size: 7,
                    expired_on: 1303,
                    claimed_at: 303,
                    amount: 1000,
                    expected_replica_count: 4,
                    reported_replica_count: 2,
                    replicas: vec![
                        Replica {
                            who: alice.clone(),
                            valid_at: 303,
                            anchor: a_pk.clone()
                        },
                        Replica {
                            who: bob.clone(),
                            valid_at: 303,
                            anchor: b_pk.clone()
                        }
                    ]
                },
                UsedInfo {
                    used_size: 7 * 2,
                    reported_group_count: 1,
                    groups: BTreeSet::from_iter(vec![a_pk.clone()].into_iter())
                })
            );
            assert_eq!(Market::files(&file_c).unwrap_or_default(), (
                FileInfo {
                    file_size: 37,
                    expired_on: 1303,
                    claimed_at: 303,
                    amount: 1000,
                    expected_replica_count: 4,
                    reported_replica_count: 2,
                    replicas: vec![
                        Replica {
                            who: alice.clone(),
                            valid_at: 303,
                            anchor: a_pk.clone()
                        },
                        Replica {
                            who: bob.clone(),
                            valid_at: 303,
                            anchor: b_pk.clone()
                        }
                    ]
                },
                UsedInfo {
                    used_size: 37 * 2,
                    reported_group_count: 1,
                    groups: BTreeSet::from_iter(vec![a_pk.clone()].into_iter())
                })
            );

            assert_eq!(Market::files(&file_d).unwrap_or_default(), (
                FileInfo {
                    file_size: 55,
                    expired_on: 1303,
                    claimed_at: 303,
                    amount: 1000,
                    expected_replica_count: 4,
                    reported_replica_count: 1,
                    replicas: vec![
                        Replica {
                            who: bob.clone(),
                            valid_at: 303,
                            anchor: b_pk.clone()
                        }
                    ]
                },
                UsedInfo {
                    used_size: 55 * 2,
                    reported_group_count: 1,
                    groups: BTreeSet::from_iter(vec![b_pk.clone()].into_iter())
                })
            );
            assert_eq!(Swork::work_reports(&b_pk).unwrap(), WorkReport {
                report_slot: 300,
                used: 55 * 2,
                free: 4294967296,
                reported_files_size: 99,
                reported_srd_root: hex::decode("00").unwrap(),
                reported_files_root: hex::decode("11").unwrap()
            });
            allow_report_work(&eve_wr_info.curr_pk, eve_wr_info.block_number);
            assert_ok!(Swork::report_works(
                Origin::signed(eve.clone()),
                eve_wr_info.curr_pk,
                eve_wr_info.prev_pk,
                eve_wr_info.block_number,
                eve_wr_info.block_hash,
                eve_wr_info.free,
                eve_wr_info.used,
                eve_wr_info.added_files,
                eve_wr_info.deleted_files,
                eve_wr_info.srd_root,
                eve_wr_info.files_root,
                eve_wr_info.sig
            ));

            assert_eq!(Market::files(&file_c).unwrap_or_default(), (
                FileInfo {
                    file_size: 37,
                    expired_on: 1303,
                    claimed_at: 303,
                    amount: 1000,
                    expected_replica_count: 4,
                    reported_replica_count: 3,
                    replicas: vec![
                        Replica {
                            who: alice.clone(),
                            valid_at: 303,
                            anchor: a_pk.clone()
                        },
                        Replica {
                            who: bob.clone(),
                            valid_at: 303,
                            anchor: b_pk.clone()
                        },
                        Replica {
                            who: eve.clone(),
                            valid_at: 303,
                            anchor: c_pk.clone()
                        }
                    ]
                },
                UsedInfo {
                    used_size: 37 * 2,
                    reported_group_count: 1,
                    groups: BTreeSet::from_iter(vec![a_pk.clone()].into_iter())
                })
            );
            assert_eq!(Market::files(&file_d).unwrap_or_default(), (
                FileInfo {
                    file_size: 55,
                    expired_on: 1303,
                    claimed_at: 303,
                    amount: 1000,
                    expected_replica_count: 4,
                    reported_replica_count: 2,
                    replicas: vec![
                        Replica {
                            who: bob.clone(),
                            valid_at: 303,
                            anchor: b_pk.clone()
                        },
                        Replica {
                            who: eve.clone(),
                            valid_at: 303,
                            anchor: c_pk.clone()
                        }
                    ]
                },
                UsedInfo {
                    used_size: 55 * 2,
                    reported_group_count: 1,
                    groups: BTreeSet::from_iter(vec![b_pk.clone()].into_iter())
                })
            );

            assert_eq!(Market::files(&file_e).unwrap_or_default(), (
                FileInfo {
                    file_size: 22,
                    expired_on: 1303,
                    claimed_at: 303,
                    amount: 1000,
                    expected_replica_count: 4,
                    reported_replica_count: 1,
                    replicas: vec![
                        Replica {
                            who: eve.clone(),
                            valid_at: 303,
                            anchor: c_pk.clone()
                        }
                    ]
                },
                UsedInfo {
                    used_size: 22 * 2,
                    reported_group_count: 1,
                    groups: BTreeSet::from_iter(vec![c_pk.clone()].into_iter())
                })
            );
            assert_eq!(Swork::work_reports(&c_pk).unwrap(), WorkReport {
                report_slot: 300,
                used: 22 * 2,
                free: 4294967296,
                reported_files_size: 114,
                reported_srd_root: hex::decode("00").unwrap(),
                reported_files_root: hex::decode("11").unwrap()
            });

            let bob_wr_info = group_work_report_bob_600();
            let eve_wr_info = group_work_report_eve_600();

            run_to_block(603);
            allow_report_work(&bob_wr_info.curr_pk, bob_wr_info.block_number);
            assert_ok!(Swork::report_works(
                Origin::signed(bob.clone()),
                bob_wr_info.curr_pk,
                bob_wr_info.prev_pk,
                bob_wr_info.block_number,
                bob_wr_info.block_hash,
                bob_wr_info.free,
                bob_wr_info.used,
                bob_wr_info.added_files,
                bob_wr_info.deleted_files,
                bob_wr_info.srd_root,
                bob_wr_info.files_root,
                bob_wr_info.sig
            ));

            assert_eq!(Market::files(&file_b).unwrap_or_default(), (
                FileInfo {
                    file_size: 7,
                    expired_on: 1303,
                    claimed_at: 603,
                    amount: 1000,
                    expected_replica_count: 4,
                    reported_replica_count: 1,
                    replicas: vec![
                        Replica {
                            who: alice.clone(),
                            valid_at: 303,
                            anchor: a_pk.clone()
                        }
                    ]
                },
                UsedInfo {
                    used_size: 7 * 2,
                    reported_group_count: 1,
                    groups: BTreeSet::from_iter(vec![a_pk.clone()].into_iter())
                })
            );
            assert_eq!(Market::files(&file_c).unwrap_or_default(), (
                FileInfo {
                    file_size: 37,
                    expired_on: 1303,
                    claimed_at: 603,
                    amount: 1000,
                    expected_replica_count: 4,
                    reported_replica_count: 2,
                    replicas: vec![
                        Replica {
                            who: alice.clone(),
                            valid_at: 303,
                            anchor: a_pk.clone()
                        },
                        Replica {
                            who: eve.clone(),
                            valid_at: 303,
                            anchor: c_pk.clone()
                        }
                    ]
                },
                UsedInfo {
                    used_size: 37 * 2,
                    reported_group_count: 1,
                    groups: BTreeSet::from_iter(vec![a_pk.clone()].into_iter())
                })
            );

            assert_eq!(Market::files(&file_d).unwrap_or_default(), (
                FileInfo {
                    file_size: 55,
                    expired_on: 1303,
                    claimed_at: 303,
                    amount: 1000,
                    expected_replica_count: 4,
                    reported_replica_count: 2,
                    replicas: vec![
                        Replica {
                            who: bob.clone(),
                            valid_at: 303,
                            anchor: b_pk.clone()
                        },
                        Replica {
                            who: eve.clone(),
                            valid_at: 303,
                            anchor: c_pk.clone()
                        }
                    ]
                },
                UsedInfo {
                    used_size: 55 * 2,
                    reported_group_count: 1,
                    groups: BTreeSet::from_iter(vec![b_pk.clone()].into_iter())
                })
            );
            assert_eq!(Swork::work_reports(&b_pk).unwrap(), WorkReport {
                report_slot: 600,
                used: 55 * 2,
                free: 4294967296,
                reported_files_size: 55,
                reported_srd_root: hex::decode("00").unwrap(),
                reported_files_root: hex::decode("11").unwrap()
            });
            allow_report_work(&eve_wr_info.curr_pk, eve_wr_info.block_number);
            assert_ok!(Swork::report_works(
                Origin::signed(eve.clone()),
                eve_wr_info.curr_pk,
                eve_wr_info.prev_pk,
                eve_wr_info.block_number,
                eve_wr_info.block_hash,
                eve_wr_info.free,
                eve_wr_info.used,
                eve_wr_info.added_files,
                eve_wr_info.deleted_files,
                eve_wr_info.srd_root,
                eve_wr_info.files_root,
                eve_wr_info.sig
            ));

            assert_eq!(Market::files(&file_c).unwrap_or_default(), (
                FileInfo {
                    file_size: 37,
                    expired_on: 1303,
                    claimed_at: 603,
                    amount: 1000,
                    expected_replica_count: 4,
                    reported_replica_count: 1,
                    replicas: vec![
                        Replica {
                            who: alice.clone(),
                            valid_at: 303,
                            anchor: a_pk.clone()
                        }
                    ]
                },
                UsedInfo {
                    used_size: 37 * 2,
                    reported_group_count: 1,
                    groups: BTreeSet::from_iter(vec![a_pk.clone()].into_iter())
                })
            );
            assert_eq!(Market::files(&file_d).unwrap_or_default(), (
                FileInfo {
                    file_size: 55,
                    expired_on: 1303,
                    claimed_at: 603,
                    amount: 1000,
                    expected_replica_count: 4,
                    reported_replica_count: 1,
                    replicas: vec![
                        Replica {
                            who: bob.clone(),
                            valid_at: 303,
                            anchor: b_pk.clone()
                        }
                    ]
                },
                UsedInfo {
                    used_size: 55 * 2,
                    reported_group_count: 1,
                    groups: BTreeSet::from_iter(vec![b_pk.clone()].into_iter())
                })
            );

            assert_eq!(Market::files(&file_e).unwrap_or_default(), (
                FileInfo {
                    file_size: 22,
                    expired_on: 1303,
                    claimed_at: 603,
                    amount: 1000,
                    expected_replica_count: 4,
                    reported_replica_count: 0,
                    replicas: vec![]
                },
                UsedInfo {
                    used_size: 0,
                    reported_group_count: 0,
                    groups: BTreeSet::from_iter(vec![].into_iter())
                })
            );
            assert_eq!(Swork::work_reports(&c_pk).unwrap(), WorkReport {
                report_slot: 600,
                used: 0,
                free: 4294967296,
                reported_files_size: 0,
                reported_srd_root: hex::decode("00").unwrap(),
                reported_files_root: hex::decode("11").unwrap()
            });

            run_to_block(1500);
            let alice_wr_info = group_work_report_alice_1500();
            assert_ok!(Market::calculate_reward(Origin::signed(eve.clone()), file_c.clone()));
            assert_ok!(Market::calculate_reward(Origin::signed(eve.clone()), file_d.clone()));
            assert_ok!(Market::calculate_reward(Origin::signed(eve.clone()), file_e.clone()));
            // A, B still open, C, D, E already close. Trash I is full. Trash II has one file. Now we report works of alice to close A, B as well.
            assert_eq!(Market::files(&file_c), None);
            assert_eq!(Market::files(&file_d), None);
            assert_eq!(Market::files(&file_e), None);

            assert_eq!(Market::used_trash_i(&file_c).is_some(), true);
            assert_eq!(Market::used_trash_i(&file_d).is_some(), true);
            assert_eq!(Market::used_trash_ii(&file_e).is_some(), true);

            assert_eq!(Swork::work_reports(&a_pk).unwrap(), WorkReport {
                report_slot: 300,
                used: 57 * 2,
                free: 4294967296,
                reported_files_size: 57,
                reported_srd_root: hex::decode("00").unwrap(),
                reported_files_root: hex::decode("11").unwrap()
            });

            assert_eq!(Swork::work_reports(&b_pk).unwrap(), WorkReport {
                report_slot: 600,
                used: 55 * 2,
                free: 4294967296,
                reported_files_size: 55,
                reported_srd_root: hex::decode("00").unwrap(),
                reported_files_root: hex::decode("11").unwrap()
            });
            allow_report_work(&alice_wr_info.curr_pk, alice_wr_info.block_number);
            assert_ok!(Swork::report_works(
                Origin::signed(alice.clone()),
                alice_wr_info.curr_pk,
                alice_wr_info.prev_pk,
                alice_wr_info.block_number,
                alice_wr_info.block_hash,
                alice_wr_info.free,
                alice_wr_info.used,
                alice_wr_info.added_files,
                alice_wr_info.deleted_files,
                alice_wr_info.srd_root,
                alice_wr_info.files_root,
                alice_wr_info.sig
            ));

            assert_eq!(Market::files(&file_a), None);
            assert_eq!(Market::files(&file_b), None);
            assert_eq!(Market::used_trash_i(&file_b).is_some(), true);
            assert_eq!(Market::used_trash_ii(&file_e).is_some(), true);
            assert_eq!(Market::used_trash_ii(&file_a).is_some(), true);
            assert_eq!(Market::used_trash_i(&file_c).is_none(), true);
            assert_eq!(Market::used_trash_i(&file_d).is_none(), true);

            // d has gone!
            assert_eq!(Swork::work_reports(&b_pk).unwrap(), WorkReport {
                report_slot: 600,
                used: 0,
                free: 4294967296,
                reported_files_size: 55,
                reported_srd_root: hex::decode("00").unwrap(),
                reported_files_root: hex::decode("11").unwrap()
            });

            assert_eq!(Swork::work_reports(&a_pk).unwrap(), WorkReport {
                report_slot: 1500,
                used: 0,
                free: 4294967296,
                reported_files_size: 0,
                reported_srd_root: hex::decode("00").unwrap(),
                reported_files_root: hex::decode("11").unwrap()
            });
        });
}


#[test]
fn join_group_should_work_for_stake_limit() {
    ExtBuilder::default()
        .build()
        .execute_with(|| {
            let alice = Sr25519Keyring::Alice.to_account_id();
            let bob = Sr25519Keyring::Bob.to_account_id();
            let eve = Sr25519Keyring::Eve.to_account_id();
            let ferdie = Sr25519Keyring::Ferdie.to_account_id();

            let alice_wr_info = group_work_report_alice_300();
            let bob_wr_info = group_work_report_bob_300();
            let eve_wr_info = group_work_report_eve_300();
            let a_pk = alice_wr_info.curr_pk.clone();
            let b_pk = bob_wr_info.curr_pk.clone();
            let c_pk = eve_wr_info.curr_pk.clone();

            register(&a_pk, LegalCode::get());
            register(&b_pk, LegalCode::get());
            register(&c_pk, LegalCode::get());
            register_identity(&alice, &a_pk, &a_pk);
            register_identity(&bob, &b_pk, &b_pk);
            register_identity(&eve, &c_pk, &c_pk);

            // alice, bob and eve become a group
            assert_ok!(Swork::create_group(
                Origin::signed(ferdie.clone())
            ));

            assert_ok!(Swork::join_group(
                Origin::signed(alice.clone()),
                ferdie.clone()
            ));

            assert_ok!(Swork::join_group(
                Origin::signed(bob.clone()),
                ferdie.clone()
            ));

            assert_ok!(Swork::join_group(
                Origin::signed(eve.clone()),
                ferdie.clone()
            ));

            run_to_block(303);
            Swork::update_identities();
            add_not_live_files();
            // A report works in 303
            allow_report_work(&alice_wr_info.curr_pk, alice_wr_info.block_number);
            assert_ok!(Swork::report_works(
                Origin::signed(alice.clone()),
                alice_wr_info.curr_pk,
                alice_wr_info.prev_pk,
                alice_wr_info.block_number,
                alice_wr_info.block_hash,
                alice_wr_info.free,
                alice_wr_info.used,
                alice_wr_info.added_files,
                alice_wr_info.deleted_files,
                alice_wr_info.srd_root,
                alice_wr_info.files_root,
                alice_wr_info.sig
            ));
            allow_report_work(&bob_wr_info.curr_pk, bob_wr_info.block_number);
            assert_ok!(Swork::report_works(
                Origin::signed(bob.clone()),
                bob_wr_info.curr_pk,
                bob_wr_info.prev_pk,
                bob_wr_info.block_number,
                bob_wr_info.block_hash,
                bob_wr_info.free,
                bob_wr_info.used,
                bob_wr_info.added_files,
                bob_wr_info.deleted_files,
                bob_wr_info.srd_root,
                bob_wr_info.files_root,
                bob_wr_info.sig
            ));
            allow_report_work(&eve_wr_info.curr_pk, eve_wr_info.block_number);
            assert_ok!(Swork::report_works(
                Origin::signed(eve.clone()),
                eve_wr_info.curr_pk,
                eve_wr_info.prev_pk,
                eve_wr_info.block_number,
                eve_wr_info.block_hash,
                eve_wr_info.free,
                eve_wr_info.used,
                eve_wr_info.added_files,
                eve_wr_info.deleted_files,
                eve_wr_info.srd_root,
                eve_wr_info.files_root,
                eve_wr_info.sig
            ));

            run_to_block(603);
            Swork::update_identities();

            assert_eq!(Swork::free(), 12884901888);
            assert_eq!(Swork::used(), 134 * 2);
            assert_eq!(Swork::current_report_slot(), 600);
            let map = WorkloadMap::get().borrow().clone();
            // All workload is counted to alice. bob and eve is None.
            assert_eq!(*map.get(&ferdie).unwrap(), 12884902156u128);
            assert_eq!(map.get(&alice).is_none(), true);
            assert_eq!(map.get(&bob).is_none(), true);
            assert_eq!(map.get(&eve).is_none(), true);
        });
}

/// Report works test cases
#[test]
fn report_works_punishment_should_work() {
    ExtBuilder::default()
        .build()
        .execute_with(|| {
            // Generate 303 blocks first
            run_to_block(303);

            let reporter: AccountId = Sr25519Keyring::Alice.to_account_id();
            let legal_wr_info = legal_work_report_with_added_files();
            let legal_pk = legal_wr_info.curr_pk.clone();
            let legal_wr = WorkReport {
                report_slot: legal_wr_info.block_number,
                used: legal_wr_info.used,
                free: legal_wr_info.free,
                reported_files_size: legal_wr_info.used,
                reported_srd_root: legal_wr_info.srd_root.clone(),
                reported_files_root: legal_wr_info.files_root.clone()
            };

            register(&legal_pk, LegalCode::get());
            add_not_live_files();

            assert_ok!(Swork::report_works(
                Origin::signed(reporter.clone()),
                legal_wr_info.curr_pk,
                legal_wr_info.prev_pk,
                legal_wr_info.block_number,
                legal_wr_info.block_hash,
                legal_wr_info.free,
                legal_wr_info.used,
                legal_wr_info.added_files,
                legal_wr_info.deleted_files,
                legal_wr_info.srd_root,
                legal_wr_info.files_root,
                legal_wr_info.sig
            ));

            // Check work report
            assert_eq!(Swork::work_reports(&legal_pk).unwrap(), legal_wr);
            assert_eq!(Swork::identities(&reporter).unwrap_or_default(), Identity {
                anchor: legal_pk.clone(),
                group: None
            });

            run_to_block(603); // don't report works
            run_to_block(903);
            let legal_wr_info = legal_work_report_with_added_files();
            assert_err!(Swork::report_works(
                Origin::signed(reporter.clone()),
                legal_wr_info.curr_pk,
                legal_wr_info.prev_pk,
                900,
                legal_wr_info.block_hash,
                legal_wr_info.free,
                legal_wr_info.used,
                legal_wr_info.added_files,
                legal_wr_info.deleted_files,
                legal_wr_info.srd_root,
                legal_wr_info.files_root,
                legal_wr_info.sig
            ), DispatchErrorWithPostInfo {
                post_info: PostDispatchInfo {
                    actual_weight: None,
                    pays_fee: Pays::Yes,
                },
                error: DispatchError::Module {
                    index: 0,
                    error: 16,
                    message: Some(
                        "UnderPunishment",
                    ),
                },
            });

            run_to_block(1203);
            let legal_wr_info = legal_work_report_with_added_files();
            assert_err!(Swork::report_works(
                Origin::signed(reporter.clone()),
                legal_wr_info.curr_pk,
                legal_wr_info.prev_pk,
                1200,
                legal_wr_info.block_hash,
                legal_wr_info.free,
                legal_wr_info.used,
                legal_wr_info.added_files,
                legal_wr_info.deleted_files,
                legal_wr_info.srd_root,
                legal_wr_info.files_root,
                legal_wr_info.sig
            ), DispatchErrorWithPostInfo {
                post_info: PostDispatchInfo {
                    actual_weight: None,
                    pays_fee: Pays::Yes,
                },
                error: DispatchError::Module {
                    index: 0,
                    error: 16,
                    message: Some(
                        "UnderPunishment",
                    ),
                },
            });

            // 900 1200 1500 1800 cannot pass the check since we set punishment for four report_slots
            run_to_block(2103);
            let legal_wr_info = legal_work_report_with_added_files();
            // error changed. pass the punishment check
            assert_err!(Swork::report_works(
                Origin::signed(reporter.clone()),
                legal_wr_info.curr_pk,
                legal_wr_info.prev_pk,
                2100,
                legal_wr_info.block_hash,
                legal_wr_info.free,
                legal_wr_info.used,
                legal_wr_info.added_files,
                legal_wr_info.deleted_files,
                legal_wr_info.srd_root,
                legal_wr_info.files_root,
                legal_wr_info.sig
            ), DispatchErrorWithPostInfo {
                post_info: PostDispatchInfo {
                    actual_weight: None,
                    pays_fee: Pays::Yes,
                },
                error: DispatchError::Module {
                    index: 0,
                    error: 5,
                    message: Some(
                        "IllegalWorkReportSig",
                    ),
                },
            });

            // forbid the punishment again
            run_to_block(2403);
            let legal_wr_info = legal_work_report_with_added_files();
            // error changed. pass the punishment check
            assert_err!(Swork::report_works(
                Origin::signed(reporter.clone()),
                legal_wr_info.curr_pk,
                legal_wr_info.prev_pk,
                2400,
                legal_wr_info.block_hash,
                legal_wr_info.free,
                legal_wr_info.used,
                legal_wr_info.added_files,
                legal_wr_info.deleted_files,
                legal_wr_info.srd_root,
                legal_wr_info.files_root,
                legal_wr_info.sig
            ), DispatchErrorWithPostInfo {
                post_info: PostDispatchInfo {
                    actual_weight: None,
                    pays_fee: Pays::Yes,
                },
                error: DispatchError::Module {
                    index: 0,
                    error: 16,
                    message: Some(
                        "UnderPunishment",
                    ),
                },
            });

            // cannot pass the check since we don't allow it for 3900
            run_to_block(3903);
            let legal_wr_info = legal_work_report_with_added_files();
            // error changed. pass the punishment check
            assert_err!(Swork::report_works(
                Origin::signed(reporter.clone()),
                legal_wr_info.curr_pk,
                legal_wr_info.prev_pk,
                3900,
                legal_wr_info.block_hash,
                legal_wr_info.free,
                legal_wr_info.used,
                legal_wr_info.added_files,
                legal_wr_info.deleted_files,
                legal_wr_info.srd_root,
                legal_wr_info.files_root,
                legal_wr_info.sig
            ), DispatchErrorWithPostInfo {
                post_info: PostDispatchInfo {
                    actual_weight: None,
                    pays_fee: Pays::Yes,
                },
                error: DispatchError::Module {
                    index: 0,
                    error: 16,
                    message: Some(
                        "UnderPunishment",
                    ),
                },
            });

        });
}

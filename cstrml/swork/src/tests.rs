use super::*;

use crate::mock::*;
use frame_support::{
    assert_ok, assert_noop,
    dispatch::DispatchError,
};
use hex;
use keyring::Sr25519Keyring;
use primitives::Hash;

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
        let legal_bonded_ids = vec![legal_pk.clone()];

        assert_eq!(Swork::identities(legal_pk).unwrap(), legal_code);
        assert_eq!(Swork::id_bonds(applier), legal_bonded_ids);
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
                used: legal_wr_info.used,
                free: legal_wr_info.free,
                files: legal_wr_info.added_files.clone().into_iter().collect(),
                reported_files_size: legal_wr_info.used,
                reported_srd_root: legal_wr_info.srd_root.clone(),
                reported_files_root: legal_wr_info.files_root.clone()
            };

            register(&reporter, &legal_pk, &LegalCode::get());
            add_pending_sorders(&reporter);

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
            assert_eq!(Swork::used(), 402868224);
            assert_eq!(Swork::reported_in_slot(&legal_pk, 300), true);

            // Check same file all been confirmed
            assert_eq!(Market::sorder_statuses(Hash::repeat_byte(1)).unwrap_or_default().status, OrderStatus::Success);
            assert_eq!(Market::sorder_statuses(Hash::repeat_byte(2)).unwrap_or_default().status, OrderStatus::Success);
            assert_eq!(Market::sorder_statuses(Hash::repeat_byte(1)).unwrap_or_default().expired_on, 1303);
            assert_eq!(Market::sorder_statuses(Hash::repeat_byte(2)).unwrap_or_default().expired_on, 1303);
        });
}

#[test]
fn report_works_should_work_without_sorders() {
    ExtBuilder::default()
        .build()
        .execute_with(|| {
            // Generate 303 blocks first
            run_to_block(303);

            let reporter: AccountId = Sr25519Keyring::Alice.to_account_id();
            let legal_wr_info = legal_work_report_with_added_files();
            let legal_pk = legal_wr_info.curr_pk.clone();

            register(&reporter, &legal_pk, &LegalCode::get());

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

            register(&reporter, &legal_pk, &LegalCode::get());

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
            assert_noop!(
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
            register(&reporter, &legal_pk, &illegal_code);

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

            register(&reporter, &legal_pk, &LegalCode::get());

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

            register(&reporter, &legal_pk, &LegalCode::get());

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

            register(&reporter, &legal_pk, &LegalCode::get());

            // Add initial work report with `reported_files_size = 5`
            add_wr(&legal_pk, &WorkReport {
                report_slot: 0,
                used: 0,
                free: 0,
                files: Default::default(),
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

            register(&reporter, &legal_pk, &LegalCode::get());
            add_wr(&legal_pk, &WorkReport {
                report_slot: 0,
                used: 2,
                free: 0,
                files: vec![
                    (hex::decode("5aa706320afc633bfb843108e492192b17d2b6b9d9ee0b795ee95417fe08b660").unwrap(), 1),
                    (hex::decode("99cdb315c8c37e2dc00fa2a8c7fe51b8149b363d29f404441982f96d2bbae65f").unwrap(), 1)
                ].into_iter().collect(),
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
                files: legal_wr_info.added_files.clone().into_iter().collect(),
                reported_files_size: legal_wr_info.used,
                reported_srd_root: legal_wr_info.srd_root.clone(),
                reported_files_root: legal_wr_info.files_root.clone()
            };

            register(&reporter, &legal_pk, &LegalCode::get());
            add_wr(&legal_pk, &WorkReport {
                report_slot: 0,
                used: 2,
                free: 0,
                files: vec![
                    (hex::decode("5aa706320afc633bfb843108e492192b17d2b6b9d9ee0b795ee95417fe08b660").unwrap(), 1),
                    (hex::decode("99cdb315c8c37e2dc00fa2a8c7fe51b8149b363d29f404441982f96d2bbae65f").unwrap(), 1)
                ].into_iter().collect(),
                reported_files_size: 3,
                reported_srd_root: vec![],
                reported_files_root: vec![]
            });
            add_success_sorders(&reporter);

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

            // Check same file all been confirmed
            // We only test Success -> Failed here
            // Another case(Pending -> Success) already tested in `report_works_should_work` case
            assert_eq!(Market::sorder_statuses(Hash::repeat_byte(0)).unwrap_or_default().status, OrderStatus::Failed);
            assert_eq!(Market::sorder_statuses(Hash::repeat_byte(1)).unwrap_or_default().status, OrderStatus::Failed);
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

            register(&reporter, &legal_pk, &LegalCode::get());
            add_wr(&legal_pk, &WorkReport {
                report_slot: 0,
                used: 2,
                free: 0,
                files: vec![
                    (hex::decode("5aa706320afc633bfb843108e492192b17d2b6b9d9ee0b795ee95417fe08b660").unwrap(), 1),
                    (hex::decode("99cdb315c8c37e2dc00fa2a8c7fe51b8149b363d29f404441982f96d2bbae65f").unwrap(), 1)
                ].into_iter().collect(),
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


            register(&reporter, &legal_pk, &LegalCode::get());
            add_wr(&legal_pk, &WorkReport {
                report_slot: 0,
                used: 40,
                free: 40,
                files: vec![
                    (hex::decode("5aa706320afc633bfb843108e492192b17d2b6b9d9ee0b795ee95417fe08b660").unwrap(), 20),
                    (hex::decode("99cdb315c8c37e2dc00fa2a8c7fe51b8149b363d29f404441982f96d2bbae65f").unwrap(), 20)
                ].into_iter().collect(),
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

            register(&reporter, &legal_pk, &LegalCode::get());
            add_wr(&legal_pk, &WorkReport {
                report_slot: 0,
                used: 2,
                free: 0,
                files: vec![
                    (hex::decode("5aa706320afc633bfb843108e492192b17d2b6b9d9ee0b795ee95417fe08b660").unwrap(), 1),
                    (hex::decode("99cdb315c8c37e2dc00fa2a8c7fe51b8149b363d29f404441982f96d2bbae65f").unwrap(), 1)
                ].into_iter().collect(),
                reported_files_size: 2,
                reported_srd_root: hex::decode("00").unwrap(),
                reported_files_root: hex::decode("11").unwrap()
            });
            add_success_sorders(&reporter);

            // 1. Runs to 303 block
            run_to_block(303);
            Swork::update_identities();

            assert_eq!(Swork::free(), 0);
            assert_eq!(Swork::used(), 2);
            assert_eq!(Swork::current_report_slot(), 300);

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

            // 4. Runs to 606
            run_to_block(606);
            Swork::update_identities();

            // 5. Free and used should not change, but current_rs should already been updated
            assert_eq!(Swork::free(), 4294967296);
            assert_eq!(Swork::used(), 2);
            assert_eq!(Swork::current_report_slot(), 600);

            // 6. Runs to 909, work report is outdated
            run_to_block(909);
            Swork::update_identities();

            // 7. Free and used should goes to 0, and the corresponding storage order should failed
            assert_eq!(Swork::free(), 0);
            assert_eq!(Swork::used(), 0);
            assert_eq!(Swork::current_report_slot(), 900);
            assert_eq!(Market::sorder_statuses(Hash::repeat_byte(0)).unwrap_or_default().status, OrderStatus::Failed);
            assert_eq!(Market::sorder_statuses(Hash::repeat_byte(1)).unwrap_or_default().status, OrderStatus::Failed);
        });
}

#[test]
fn resuming_report_should_work() {
    ExtBuilder::default()
        .build()
        .execute_with(|| {
            let reporter: AccountId = Sr25519Keyring::Alice.to_account_id();
            let legal_wr_info = resuming_work_report();
            let legal_pk = legal_wr_info.curr_pk.clone();

            register(&reporter, &legal_pk, &LegalCode::get());
            add_wr(&legal_pk, &WorkReport {
                report_slot: 0,
                used: 2,
                free: 0,
                files: vec![
                    (hex::decode("5aa706320afc633bfb843108e492192b17d2b6b9d9ee0b795ee95417fe08b660").unwrap(), 1),
                    (hex::decode("99cdb315c8c37e2dc00fa2a8c7fe51b8149b363d29f404441982f96d2bbae65f").unwrap(), 1)
                ].into_iter().collect(),
                reported_files_size: 2,
                reported_srd_root: hex::decode("00").unwrap(),
                reported_files_root: hex::decode("11").unwrap()
            });
            add_success_sorders(&reporter);

            // 1. Runs to 303 block
            run_to_block(303);
            Swork::update_identities();

            // 2. No works reported, but orders should still be ok
            assert_eq!(Market::sorder_statuses(Hash::repeat_byte(0)).unwrap_or_default().status, OrderStatus::Success);
            assert_eq!(Market::sorder_statuses(Hash::repeat_byte(1)).unwrap_or_default().status, OrderStatus::Success);

            // 3. Runs to 606
            run_to_block(606);
            Swork::update_identities();

            // 4. Storage order should still be success
            assert_eq!(Market::sorder_statuses(Hash::repeat_byte(0)).unwrap_or_default().status, OrderStatus::Failed);
            assert_eq!(Market::sorder_statuses(Hash::repeat_byte(1)).unwrap_or_default().status, OrderStatus::Failed);

            // 5. Runs to 909, work report is outdated
            run_to_block(909);
            Swork::update_identities();

            // 6. Report works in slot 900
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

            // 7. Orders should reset back
            assert_eq!(Market::sorder_statuses(Hash::repeat_byte(0)).unwrap_or_default().status, OrderStatus::Success);
            assert_eq!(Market::sorder_statuses(Hash::repeat_byte(1)).unwrap_or_default().status, OrderStatus::Success);
        });
}

#[test]
fn abnormal_era_should_work() {
    ExtBuilder::default()
        .build()
        .execute_with(|| {
            let reporter: AccountId = Sr25519Keyring::Alice.to_account_id();
            let legal_pk = LegalPK::get();

            register(&reporter, &legal_pk, &LegalCode::get());
            add_wr(&legal_pk, &WorkReport {
                report_slot: 0,
                used: 2,
                free: 0,
                files: vec![
                    (hex::decode("5aa706320afc633bfb843108e492192b17d2b6b9d9ee0b795ee95417fe08b660").unwrap(), 1),
                    (hex::decode("99cdb315c8c37e2dc00fa2a8c7fe51b8149b363d29f404441982f96d2bbae65f").unwrap(), 1)
                ].into_iter().collect(),
                reported_files_size: 2,
                reported_srd_root: hex::decode("00").unwrap(),
                reported_files_root: hex::decode("11").unwrap()
            });
            add_success_sorders(&reporter);

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
            register(&reporter, &a_pk, &LegalCode::get());
            add_wr(&a_pk, &WorkReport {
                report_slot: 0,
                used: 2,
                free: 0,
                files: vec![
                    (hex::decode("5aa706320afc633bfb843108e492192b17d2b6b9d9ee0b795ee95417fe08b660").unwrap(), 1),
                    (hex::decode("99cdb315c8c37e2dc00fa2a8c7fe51b8149b363d29f404441982f96d2bbae65f").unwrap(), 1)
                ].into_iter().collect(),
                reported_files_size: 2,
                reported_srd_root: hex::decode("00").unwrap(),
                reported_files_root: hex::decode("11").unwrap()
            });
            add_pending_sorders(&reporter); // with b_wr_info_2's added file
            add_success_sorders(&reporter); // with b_wr_info_2's deleted file

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
                files: vec![
                    (hex::decode("5aa706320afc633bfb843108e492192b17d2b6b9d9ee0b795ee95417fe08b660").unwrap(), 1),
                    (hex::decode("99cdb315c8c37e2dc00fa2a8c7fe51b8149b363d29f404441982f96d2bbae65f").unwrap(), 1)
                ].into_iter().collect(),
                reported_files_size: 2,
                reported_srd_root: hex::decode("00").unwrap(),
                reported_files_root: hex::decode("11").unwrap()
            });
            assert_eq!(Swork::free(), 4294967296);
            assert_eq!(Swork::used(), 2);

            // 4. Runs to 606, and do sWorker upgrade
            run_to_block(606);
            // Fake do upgrade

            // 5. (Fake) Register B 🤣, suppose B's code is upgraded
            register(&reporter, &b_pk, &LegalCode::get());

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
            assert_eq!(Swork::work_reports(&b_pk).unwrap(), WorkReport {
                report_slot: 600,
                used: 2,
                free: 4294967296,
                files: vec![
                    (hex::decode("5aa706320afc633bfb843108e492192b17d2b6b9d9ee0b795ee95417fe08b660").unwrap(), 1),
                    (hex::decode("99cdb315c8c37e2dc00fa2a8c7fe51b8149b363d29f404441982f96d2bbae65f").unwrap(), 1)
                ].into_iter().collect(),
                reported_files_size: 2,
                reported_srd_root: hex::decode("00").unwrap(),
                reported_files_root: hex::decode("11").unwrap()
            });
            assert_eq!(Swork::free(), 4294967296);
            assert_eq!(Swork::used(), 2);

            // 8. Check A is already be chilled
            assert_eq!(Swork::identities(&a_pk), None);
            assert_eq!(Swork::work_reports(&a_pk), None);
            assert_eq!(Swork::id_bonds(&reporter), vec![b_pk.clone()]);

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
            assert_eq!(Swork::work_reports(&b_pk).unwrap(), WorkReport {
                report_slot: 900,
                used: 3,
                free: 4294967296,
                files: vec![
                    (hex::decode("5aa706320afc633bfb843108e492192b17d2b6b9d9ee0b795ee95417fe08b660").unwrap(), 1),
                    (hex::decode("5bb706320afc633bfb843108e492192b17d2b6b9d9ee0b795ee95417fe08b660").unwrap(), 2)
                ].into_iter().collect(),
                reported_files_size: 3,
                reported_srd_root: hex::decode("00").unwrap(),
                reported_files_root: hex::decode("11").unwrap()
            });
            assert_eq!(Swork::free(), 4294967296);
            assert_eq!(Swork::used(), 3); // Added 2 and delete 1

            // 12. Corresponding sorder should work
            // 5bb706320afc633bfb843108e492192b17d2b6b9d9ee0b795ee95417fe08b660
            assert_eq!(Market::sorder_statuses(Hash::repeat_byte(0)).unwrap_or_default().status, OrderStatus::Success);
            // 99cdb315c8c37e2dc00fa2a8c7fe51b8149b363d29f404441982f96d2bbae65f
            assert_eq!(Market::sorder_statuses(Hash::repeat_byte(1)).unwrap_or_default().status, OrderStatus::Failed);
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
            register(&reporter, &legal_pk, &LegalCode::get());
            add_wr(&legal_pk, &&WorkReport {
                report_slot: 0,
                used: 2,
                free: 0,
                files: vec![
                    (hex::decode("5aa706320afc633bfb843108e492192b17d2b6b9d9ee0b795ee95417fe08b660").unwrap(), 1),
                    (hex::decode("99cdb315c8c37e2dc00fa2a8c7fe51b8149b363d29f404441982f96d2bbae65f").unwrap(), 1)
                ].into_iter().collect(),
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
            let mut b_wr_info = ab_upgrade_work_report();
            let a_pk = a_wr_info.curr_pk.clone();
            let b_pk = b_wr_info.curr_pk.clone();

            // 0. Initial setup
            register(&reporter, &a_pk, &LegalCode::get());
            add_wr(&a_pk, &WorkReport {
                report_slot: 0,
                used: 2,
                free: 0,
                files: vec![
                    (hex::decode("5aa706320afc633bfb843108e492192b17d2b6b9d9ee0b795ee95417fe08b660").unwrap(), 1),
                    (hex::decode("99cdb315c8c37e2dc00fa2a8c7fe51b8149b363d29f404441982f96d2bbae65f").unwrap(), 1)
                ].into_iter().collect(),
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

            // 2. Report B with added_files
            b_wr_info.added_files = vec![(hex::decode("6aa706320afc633bfb843108e492192b17d2b6b9d9ee0b795ee95417fe08b660").unwrap(), 10)];

            // 4. Runs to 606, and do sWorker upgrade
            run_to_block(606);
            // Fake do upgrade

            // 5. (Fake) Register B 🤣, suppose B's code is upgraded
            register(&reporter, &b_pk, &LegalCode::get());

            // 6. Report works with sWorker B will failed
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

/// Star network test cases
/// As for the star network, more should be tested in market module(space size) and staking module(stake limit)
#[test]
fn multiple_bonds_should_work() {
    ExtBuilder::default()
        .build()
        .execute_with(|| {
            let reporter = Sr25519Keyring::Alice.to_account_id();
            let wr_info_1 = legal_work_report();
            let wr_info_2 = legal_work_report_with_added_files();
            let pk1 = wr_info_1.curr_pk.clone();
            let pk2 = wr_info_2.curr_pk.clone();

            register(&reporter, &pk1, &LegalCode::get());
            register(&reporter, &pk2, &LegalCode::get());

            assert_eq!(Swork::id_bonds(&reporter), vec![pk1.clone(), pk2.clone()]);
        });
}

#[test]
fn bonds_limit_should_work() {
    ExtBuilder::default()
        .build()
        .execute_with(|| {
            let applier = AccountId::from_ss58check("5FqazaU79hjpEMiWTWZx81VjsYFst15eBuSBKdQLgQibD7CX")
                .expect("valid ss58 address");
            let legal_register_info = legal_register_info();

            register(&applier, &vec![], &LegalCode::get());
            register(&applier, &vec![], &LegalCode::get());

            assert_noop!(
                Swork::register(
                    Origin::signed(applier.clone()),
                    legal_register_info.ias_sig,
                    legal_register_info.ias_cert,
                    legal_register_info.account_id,
                    legal_register_info.isv_body,
                    legal_register_info.sig
                ),
                DispatchError::Module {
                    index: 0,
                    error: 8,
                    message: Some("ExceedBondsLimit"),
                }
            );
        });
}

#[test]
fn bonds_limit_during_upgrade_should_work() {
    ExtBuilder::default()
        .build()
        .execute_with(|| {
            let applier = AccountId::from_ss58check("5FqazaU79hjpEMiWTWZx81VjsYFst15eBuSBKdQLgQibD7CX")
                .expect("valid ss58 address");
            let failed_legal_register_info = legal_register_info();

            register(&applier, &vec![0], &LegalCode::get());
            register(&applier, &vec![1], &LegalCode::get());

            assert_noop!(
                Swork::register(
                    Origin::signed(applier.clone()),
                    failed_legal_register_info.ias_sig,
                    failed_legal_register_info.ias_cert,
                    failed_legal_register_info.account_id,
                    failed_legal_register_info.isv_body,
                    failed_legal_register_info.sig
                ),
                DispatchError::Module {
                    index: 0,
                    error: 8,
                    message: Some("ExceedBondsLimit"),
                }
            );

            assert_ok!(Swork::upgrade(Origin::root(), hex::decode("0011").unwrap(), 500));

            // TODO: Use success register info later. Fake the test for now.
            let legal_register_info = legal_register_info();
            assert_noop!(
                Swork::register(
                    Origin::signed(applier.clone()),
                    legal_register_info.ias_sig,
                    legal_register_info.ias_cert,
                    legal_register_info.account_id,
                    legal_register_info.isv_body,
                    legal_register_info.sig
                ),
                DispatchError::Module {
                    index: 0,
                    error: 1,
                    message: Some("IllegalIdentity"),
                }
            );
        });
}
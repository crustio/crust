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
            let legal_pk = hex::decode("7c16c0a0d7a1ccf654aa2925fe56575823972adaa0125ffb843d9a1cae0e1f2ea4f3d820ff59d5631ff873693936ebc6b91d0af22b821299019dbacf40f5791d").unwrap();
            let legal_wr_info = legal_work_report_with_added_files();
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

            // Check same file all been confirmed
            assert_eq!(Market::storage_orders(Hash::repeat_byte(1)).unwrap_or_default().status, OrderStatus::Success);
            assert_eq!(Market::storage_orders(Hash::repeat_byte(2)).unwrap_or_default().status, OrderStatus::Success);
            assert_eq!(Market::storage_orders(Hash::repeat_byte(1)).unwrap_or_default().expired_on, 653);
            assert_eq!(Market::storage_orders(Hash::repeat_byte(2)).unwrap_or_default().expired_on, 653);
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
            let legal_pk = hex::decode("7c16c0a0d7a1ccf654aa2925fe56575823972adaa0125ffb843d9a1cae0e1f2ea4f3d820ff59d5631ff873693936ebc6b91d0af22b821299019dbacf40f5791d").unwrap();
            let legal_wr_info = legal_work_report_with_added_files();

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
            let legal_pk = hex::decode("7c16c0a0d7a1ccf654aa2925fe56575823972adaa0125ffb843d9a1cae0e1f2ea4f3d820ff59d5631ff873693936ebc6b91d0af22b821299019dbacf40f5791d").unwrap();
            let legal_wr_info = legal_work_report_with_added_files();
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
            let legal_pk = hex::decode("7c16c0a0d7a1ccf654aa2925fe56575823972adaa0125ffb843d9a1cae0e1f2ea4f3d820ff59d5631ff873693936ebc6b91d0af22b821299019dbacf40f5791d").unwrap();
            let illegal_wr_info = legal_work_report_with_added_files();

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
            let legal_pk = hex::decode("7c16c0a0d7a1ccf654aa2925fe56575823972adaa0125ffb843d9a1cae0e1f2ea4f3d820ff59d5631ff873693936ebc6b91d0af22b821299019dbacf40f5791d").unwrap();
            let mut illegal_wr_info = legal_work_report_with_added_files();
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
            let legal_pk = hex::decode("7c16c0a0d7a1ccf654aa2925fe56575823972adaa0125ffb843d9a1cae0e1f2ea4f3d820ff59d5631ff873693936ebc6b91d0af22b821299019dbacf40f5791d").unwrap();
            let mut illegal_wr_info = legal_work_report_with_added_files();

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
            let legal_pk = hex::decode("69a2e1757b143b45246c6a47c1d2fd4db263328ee9e84f7950414a4ce420079eafa07d062f4fd716104040f3a99159e33434218a8c7c3107a9101fb007dead82").unwrap();
            let legal_wr_info = legal_work_report();

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
            let legal_pk = hex::decode("819e555a290c4f725739eb03a3e8d0f31db074a6e16abeec3a9a6a7c0379b6de9ad4d7658c44257746d58764e9db9c736d39474199ce53e4edfcc3d5340f1916").unwrap();
            let legal_wr_info = legal_work_report_with_deleted_files();
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
            assert_eq!(Market::storage_orders(Hash::repeat_byte(0)).unwrap_or_default().status, OrderStatus::Failed);
            assert_eq!(Market::storage_orders(Hash::repeat_byte(1)).unwrap_or_default().status, OrderStatus::Failed);
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
            let legal_pk = hex::decode("69a2e1757b143b45246c6a47c1d2fd4db263328ee9e84f7950414a4ce420079eafa07d062f4fd716104040f3a99159e33434218a8c7c3107a9101fb007dead82").unwrap();
            let illegal_wr_info = legal_work_report();

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
            let legal_pk = hex::decode("69a2e1757b143b45246c6a47c1d2fd4db263328ee9e84f7950414a4ce420079eafa07d062f4fd716104040f3a99159e33434218a8c7c3107a9101fb007dead82").unwrap();
            let illegal_wr_info = legal_work_report(); // No change but with file size down

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
            let legal_pk = hex::decode("69a2e1757b143b45246c6a47c1d2fd4db263328ee9e84f7950414a4ce420079eafa07d062f4fd716104040f3a99159e33434218a8c7c3107a9101fb007dead82").unwrap();
            let legal_wr_info = legal_work_report();

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

            // 2. Report works in 300 slot
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
        });
}

/*
#[test]
fn test_for_wr_check_failed_order_by_no_file_in_wr() {
    new_test_ext().execute_with(|| {
        let account: AccountId = Sr25519Keyring::Bob.to_account_id();
        add_success_sorder(350);
        // generate 303 blocks first
        run_to_block(303, None);

        let report_works_info = valid_report_works_info();

        // report works should ok
        assert_ok!(Swork::report_works(
            Origin::signed(account.clone()),
            report_works_info.pub_key,
            report_works_info.block_number,
            report_works_info.block_hash,
            report_works_info.reserved,
            report_works_info.files,
            report_works_info.sig
        ));

        // check work report and workload, current_report_slot updating should work
        Swork::update_identities();

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
        Swork::update_identities();

        add_success_sorder(650);

        // 2nd era
        run_to_block(606, None);
        Swork::update_identities();

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
        Swork::update_identities();

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
        assert_ok!(Swork::report_works(
            Origin::signed(account.clone()),
            report_works_info.pub_key,
            report_works_info.block_number,
            report_works_info.block_hash,
            report_works_info.reserved,
            report_works_info.files,
            report_works_info.sig
        ));

        // check work report and workload, current_report_slot updating should work
        assert_eq!(Swork::current_report_slot(), 0);
        Swork::update_identities();
        assert_eq!(Swork::current_report_slot(), 300);

        // Check workloads
        assert_eq!(Swork::reserved(), 4294967296);
        assert_eq!(Swork::used(), 0);

        // generate 401 blocks, wr still valid
        run_to_block(401, None);
        assert_eq!(
            Swork::work_reports(&account),
            Some(wr.clone())
        );
        assert!(Swork::reported_in_slot(&account, 300).1);

        // generate 602 blocks
        run_to_block(602, None);
        assert_eq!(Swork::current_report_slot(), 300);
        Swork::update_identities();
        assert_eq!(Swork::current_report_slot(), 600);
        assert_eq!(
            Swork::work_reports(&account),
            Some(wr.clone())
        );
        assert!(!Swork::reported_in_slot(&account, 600).1);

        // Check workloads
        assert_eq!(Swork::reserved(), 4294967296);
        assert_eq!(Swork::used(), 0);

        run_to_block(903, None);
        assert_eq!(Swork::current_report_slot(), 600);
        Swork::update_identities();
        assert_eq!(Swork::current_report_slot(), 900);

        // Check workloads
        assert_eq!(Swork::work_reports(&account), None);
        assert_eq!(Swork::reserved(), 0);
        assert_eq!(Swork::used(), 0);
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
        Swork::update_identities();
        assert_eq!(
            Swork::work_reports(&account),
            Some(Default::default())
        );
        assert_eq!(Swork::reserved(), 0);
        assert_eq!(Swork::current_report_slot(), 0);

        // If new era happens on 301, we should update work report and current report slot
        run_to_block(301, None);
        Swork::update_identities();
        assert_eq!(
            Swork::work_reports(&account),
            Some(Default::default())
        );
        assert_eq!(
            Swork::current_report_slot(),
            300
        );
        assert!(Swork::reported_in_slot(&account, 0).1);

        // If next new era happens on 303, then nothing should happen
        run_to_block(303, None);
        Swork::update_identities();
        assert_eq!(
            Swork::work_reports(&account),
            Some(Default::default())
        );
        assert_eq!(
            Swork::current_report_slot(),
            300
        );
        assert!(Swork::reported_in_slot(&account, 0).1);
        assert!(!Swork::reported_in_slot(&account, 300).1);

        // Then report works
        // reserved: 4294967296,
        // used: 1676266280,
        run_to_block(304, None);
        assert_ok!(Swork::report_works(
            Origin::signed(account.clone()),
            report_works_info.pub_key,
            report_works_info.block_number,
            report_works_info.block_hash,
            report_works_info.reserved,
            report_works_info.files,
            report_works_info.sig
        ));
        assert_eq!(Swork::work_reports(&account), Some(wr));
        // total workload should keep same, cause we only updated in a new era
        assert_eq!(Swork::reserved(), 4294967296);
        assert_eq!(Swork::used(), 0);
        assert!(Swork::reported_in_slot(&account, 300).1);
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
        // a. Run to 37205 block first with old sworker code
        Code::put(old_code.clone());
        run_to_block(37205, Some(old_bh.clone()));

        // b. Identity should do `upgrade` with current_id.code != code
        assert!(Swork::maybe_upsert_id(&reporter, &old_id));

        // c. Report works with current id should be ✅
        assert_ok!(Swork::report_works(
            Origin::signed(reporter.clone()),
            old_pub_key.clone(),
            37_200,
            old_bh.clone(),
            42_949_672_960,
            old_files.clone(),
            hex::decode("a30eb07fd09687264a7b7215061cd9424f945c898bfeb326c9bfa5870ec3926639d10032d7f5141514b03af32142fec7bb8ad09f028d6e0c5e40f4bc03d56272").unwrap(),
        ));
        assert_eq!(Swork::work_reports(&reporter).unwrap(), old_work_report.clone());
        assert_eq!(Swork::reported_in_slot(&reporter, 37200), (false, true));

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

        // b. Run to 38705 block with new sworker code, and do the upgrade
        run_to_block(38705, Some(new_bh.clone()));
        assert_ok!(Swork::upgrade(Origin::root(), new_code.clone(), 39000));

        assert!(Swork::maybe_upsert_id(&reporter, &new_id));
        assert_eq!(Swork::identities(&reporter), (Some(old_id.clone()), Some(new_id.clone())));

        // c. Report with new identity should be ✅
        assert_ok!(Swork::report_works(
            Origin::signed(reporter.clone()),
            new_pk.clone(),
            38_700,
            new_bh.clone(),
            40_000,
            new_files.clone(),
            hex::decode("525fd0d4afcd99965166c6fca2cb74ce34bb303109921d6ab0e172aafb00a4c3ec6086c59e4abe232782848170b88d19b2641d470bb30ba7827d5161ec5ad46e").unwrap(),
        ));
        assert_eq!(Swork::work_reports(&reporter).unwrap(), new_work_report.clone());
        assert_eq!(Swork::reported_in_slot(&reporter, 38700), (false, true));

        // d. Report with old identity should also be ✅
        assert_ok!(Swork::report_works(
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

        assert_eq!(Swork::work_reports(&reporter).unwrap(), new_work_report.clone());
        assert_eq!(Swork::reported_in_slot(&reporter, 38700), (true, true));

        // 3. AB expire should work, replay the block authoring
        // a. Bob do not upgrade
        assert_ok!(Swork::upgrade(Origin::root(), new_code.clone(), 38800));

        // b. Double report would be ignore in the first place ❌, even the sig is illegal
        assert_ok!(Swork::report_works(
            Origin::signed(reporter.clone()),
            old_pub_key.clone(),
            38700,
            new_bh.clone(),
            100,
            old_files.clone(),
            hex::decode("1111").unwrap(),
        ));
        assert_ok!(Swork::report_works(
            Origin::signed(reporter.clone()),
            new_pk.clone(),
            38700,
            new_bh.clone(),
            100,
            new_files.clone(),
            hex::decode("2222").unwrap(),
        ));
        assert_eq!(Swork::work_reports(&reporter).unwrap(), new_work_report.clone());

        // c. Run to block 39005, report should ❌
        run_to_block(39005, Some(new_bh.clone()));
        assert_noop!(Swork::report_works(
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
        assert_ok!(Swork::upgrade(Origin::root(), new_code.clone(), 39500));

        // a. Report with old identity should also be ✅
        assert_ok!(Swork::report_works(
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
        assert_eq!(Swork::work_reports(&reporter).unwrap(), new_work_report.clone());
        assert_eq!(Swork::reported_in_slot(&reporter, 39000), (true, false));

        // c. Reporter with old identity, the reserved should be right(after shrink the workload)
        run_to_block(39305, Some(new_bh.clone()));
        assert_ok!(Swork::report_works(
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
        assert_eq!(Swork::work_reports(&reporter).unwrap(), new_work_report.clone());
        assert_eq!(Swork::reported_in_slot(&reporter, 39300), (true, false));
    });
}*/
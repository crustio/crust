// Copyright (C) 2019-2021 Crust Network Technologies Ltd.
// This file is part of Crust.

//! Balances pallet benchmarking.
#![cfg_attr(not(feature = "std"), no_std)]
use frame_system::{self as system, RawOrigin};
use frame_benchmarking::{benchmarks, account};
use frame_support::traits::Currency;
use frame_support::storage::StorageMap;
use sp_runtime::traits::{StaticLookup, Zero};
use codec::Decode;
use market::{UsedInfo, FileInfo, Replica};
use primitives::*;
use sp_std::{vec, prelude::*, collections::{btree_set::BTreeSet, btree_map::BTreeMap}, iter::FromIterator};

const SEED: u32 = 0;
const EXPIRE_BLOCK_NUMBER: u32 = 2000;

pub struct Module<T: Config>(swork::Module<T>);
pub trait Config: market::Config + swork::Config {}
pub type Balance = u64;

#[cfg(test)]
mod mock;

struct ReportWorksInfo {
    pub curr_pk: SworkerPubKey,
    pub prev_pk: SworkerPubKey,
    pub block_number: u64,
    pub block_hash: Vec<u8>,
    pub free: u64,
    pub used: u64,
    pub srd_root: MerkleRoot,
    pub files_root: MerkleRoot,
    pub added_files: Vec<(MerkleRoot, u64, u64)>,
    pub deleted_files: Vec<(MerkleRoot, u64, u64)>,
    pub sig: SworkerSignature
}

fn legal_work_report_with_srd() -> ReportWorksInfo {
    let curr_pk = vec![45, 134, 206, 46, 43, 60, 2, 133, 235, 80, 26, 90, 52, 220, 26, 131, 87, 173, 158, 54, 92, 150, 74, 80, 210, 208, 241, 182, 250, 47, 129, 234, 184, 17, 23, 208, 94, 152, 157, 216, 156, 208, 38, 198, 57, 29, 231, 76, 56, 146, 150, 108, 58, 186, 94, 149, 117, 245, 199, 24, 204, 209, 71, 195];
    let prev_pk: Vec<u8> = vec![];
    let block_number = 300;
    let block_hash = vec![0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0];
    let free: u64 = 4294967296;
    let used: u64 = 0;
    let added_files: Vec<(Vec<u8>, u64, u64)> = vec![];
    let deleted_files: Vec<(Vec<u8>, u64, u64)> = vec![];
    let files_root: Vec<u8> = vec![17];
    let srd_root: Vec<u8> = vec![0];
    let sig: Vec<u8> = vec![169, 219, 186, 233, 110, 149, 28, 104, 209, 118, 50, 172, 135, 123, 20, 174, 233, 212, 147, 135, 191, 225, 46, 173, 189, 19, 100, 10, 255, 77, 195, 10, 181, 232, 172, 4, 53, 129, 149, 14, 47, 239, 176, 67, 15, 71, 197, 194, 100, 146, 243, 82, 21, 62, 247, 225, 208, 46, 4, 254, 121, 71, 118, 92];

    ReportWorksInfo {
        curr_pk,
        prev_pk,
        block_number,
        block_hash,
        free,
        used,
        srd_root,
        files_root,
        added_files,
        deleted_files,
        sig
    }
}

fn legal_work_report_with_added_files() -> ReportWorksInfo {
    let curr_pk = vec![105,129,135,206,41,52,134,230,86,57,211,151,97,151,229,104,163,246,160,7,110,85,25,197,107,63,124,60,13,12,92,18,199,28,200,78,79,183,127,80,225,146,152,242,175,40,225,51,49,14,8,194,240,215,71,42,135,144,132,183,35,121,115,177];
    let prev_pk: Vec<u8> = vec![];
    let block_number = 300;
    let block_hash = vec![0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0];
    let free: u64 = 4294967296;
    let used: u64 = 1000;
    let mut added_files: Vec<(Vec<u8>, u64, u64)> = vec![];
    for i in 0..used {
        let upper = (i / 256) as u8;
        let lower = (i % 256) as u8;
        added_files.push((vec![upper, lower], 1, 303));
    }
    let deleted_files: Vec<(Vec<u8>, u64, u64)> = vec![];
    let files_root: Vec<u8> = vec![17];
    let srd_root: Vec<u8> = vec![0];
    let sig: Vec<u8> = vec![66,70,118,81,131,105,13,230,117,199,215,28,17,249,37,63,16,227,29,228,2,135,35,68,52,230,96,110,196,64,228,69,149,246,230,55,137,25,30,65,76,198,179,195,130,49,16,99,167,76,43,160,189,251,30,148,198,64,11,127,28,231,38,8];

    ReportWorksInfo {
        curr_pk,
        prev_pk,
        block_number,
        block_hash,
        free,
        used,
        srd_root,
        files_root,
        added_files,
        deleted_files,
        sig
    }
}

fn legal_work_report_with_deleted_files() -> ReportWorksInfo {
    let curr_pk = vec![105,129,135,206,41,52,134,230,86,57,211,151,97,151,229,104,163,246,160,7,110,85,25,197,107,63,124,60,13,12,92,18,199,28,200,78,79,183,127,80,225,146,152,242,175,40,225,51,49,14,8,194,240,215,71,42,135,144,132,183,35,121,115,177];
    let prev_pk: Vec<u8> = vec![];
    let block_number = 600;
    let block_hash = vec![0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0];
    let free: u64 = 4294967296;
    let used: u64 = 0;
    let added_files: Vec<(Vec<u8>, u64, u64)> = vec![];
    let mut deleted_files: Vec<(Vec<u8>, u64, u64)> = vec![];
    for i in 0..1000 {
        let upper = (i / 256) as u8;
        let lower = (i % 256) as u8;
        deleted_files.push((vec![upper, lower], 1, 603));
    }
    let files_root: Vec<u8> = vec![17];
    let srd_root: Vec<u8> = vec![0];
    let sig: Vec<u8> = vec![159,147,255,189,121,120,74,136,29,153,105,90,25,235,208,54,216,152,193,180,130,219,85,110,231,179,185,183,153,15,200,219,102,209,133,211,160,150,196,215,119,187,27,2,125,60,231,73,11,34,160,88,235,54,204,58,69,206,236,56,231,37,186,211];

    ReportWorksInfo {
        curr_pk,
        prev_pk,
        block_number,
        block_hash,
        free,
        used,
        srd_root,
        files_root,
        added_files,
        deleted_files,
        sig
    }
}

fn add_market_files<T: Config>(files: Vec<(MerkleRoot, u64, u64)>, user: T::AccountId, pub_key: Vec<u8>) {
    for (file, file_size, _) in files.clone().iter() {
        let used_info = UsedInfo {
            used_size: *file_size,
            reported_group_count: 0,
            groups: <BTreeMap<SworkerAnchor, bool>>::new()
        };
        let mut replicas: Vec<Replica<T::AccountId>> = vec![];
        for _ in 0..200 {
            let new_replica = Replica {
                who: user.clone(),
                valid_at: 300,
                anchor: pub_key.clone(),
                is_reported: true
            };
            replicas.push(new_replica);
        }
        let file_info = FileInfo {
            file_size: *file_size,
            expired_on: 1000,
            claimed_at: 400,
            amount: <T as market::Config>::Currency::minimum_balance() * 1000000000u32.into(),
            prepaid: Zero::zero(),
            reported_replica_count: 0,
            replicas
        };
        <market::Files<T>>::insert(file, (file_info, used_info));
    }
    let storage_value = <T as market::Config>::Currency::minimum_balance() * 10000000u32.into();
    <T as market::Config>::Currency::make_free_balance_be(&market::Module::<T>::storage_pot(), storage_value);
}

benchmarks! {
    _{}

    upgrade {
        let code: Vec<u8> = vec![120,27,83,125,61,206,243,157,236,123,139,206,111,223,205,3,45,141,132,102,64,233,181,89,139,74,159,98,113,136,169,8];
    }: {
        swork::Module::<T>::upgrade(RawOrigin::Root.into(), code, EXPIRE_BLOCK_NUMBER.into()).expect("failed to insert code");
    }

    register {
        let code: Vec<u8> = vec![120,27,83,125,61,206,243,157,236,123,139,206,111,223,205,3,45,141,132,102,64,233,181,89,139,74,159,98,113,136,169,8];
        swork::Module::<T>::upgrade(RawOrigin::Root.into(), code.clone(), EXPIRE_BLOCK_NUMBER.into()).expect("failed to insert code");
        let user: Vec<u8> = vec![166,239,163,116,112,15,134,64,183,119,188,146,199,125,52,68,124,85,136,215,235,124,78,201,132,50,60,125,176,152,48,9];
        let caller = T::AccountId::decode(&mut &user[..]).unwrap_or_default();
        let ias_sig = "VWfhb8pfVTHFcwIfFI9fLQPPvScGKwWOtkhYzlIMP5MT/u81VMAJed37p87YyMNwpqopaTP6/QVLkrZFw6fRgONMY+kRyzzkUDB3gRhRh71ZqZe0R+XHsGi6QH0YnMiXtCnD9oP3vSKx8UqhMKRpn4eCUU2jKLkoUOT8fiwozOnrIfYH5aVLcF65Laomj0trgoFbJlm/Yag7HOA3mQMRgCoBzP+xeKZBCWr/Zh6814mnwb8X79KVpM7suiy+g0KuZQpjH9qE32XsBL7lNizqVji9XiAJwN6pbhDmQaRbB8y46mJ1HkII+SFHCyBWAtdiqH9cTsmbsTjAS/TjoXcphQ==".as_bytes();
        let ias_cert = "MIIEoTCCAwmgAwIBAgIJANEHdl0yo7CWMA0GCSqGSIb3DQEBCwUAMH4xCzAJBgNVBAYTAlVTMQswCQYDVQQIDAJDQTEUMBIGA1UEBwwLU2FudGEgQ2xhcmExGjAYBgNVBAoMEUludGVsIENvcnBvcmF0aW9uMTAwLgYDVQQDDCdJbnRlbCBTR1ggQXR0ZXN0YXRpb24gUmVwb3J0IFNpZ25pbmcgQ0EwHhcNMTYxMTIyMDkzNjU4WhcNMjYxMTIwMDkzNjU4WjB7MQswCQYDVQQGEwJVUzELMAkGA1UECAwCQ0ExFDASBgNVBAcMC1NhbnRhIENsYXJhMRowGAYDVQQKDBFJbnRlbCBDb3Jwb3JhdGlvbjEtMCsGA1UEAwwkSW50ZWwgU0dYIEF0dGVzdGF0aW9uIFJlcG9ydCBTaWduaW5nMIIBIjANBgkqhkiG9w0BAQEFAAOCAQ8AMIIBCgKCAQEAqXot4OZuphR8nudFrAFiaGxxkgma/Es/BA+tbeCTUR106AL1ENcWA4FX3K+E9BBL0/7X5rj5nIgX/R/1ubhkKWw9gfqPG3KeAtIdcv/uTO1yXv50vqaPvE1CRChvzdS/ZEBqQ5oVvLTPZ3VEicQjlytKgN9cLnxbwtuvLUK7eyRPfJW/ksddOzP8VBBniolYnRCD2jrMRZ8nBM2ZWYwnXnwYeOAHV+W9tOhAImwRwKF/95yAsVwd21ryHMJBcGH70qLagZ7Ttyt++qO/6+KAXJuKwZqjRlEtSEz8gZQeFfVYgcwSfo96oSMAzVr7V0L6HSDLRnpb6xxmbPdqNol4tQIDAQABo4GkMIGhMB8GA1UdIwQYMBaAFHhDe3amfrzQr35CN+s1fDuHAVE8MA4GA1UdDwEB/wQEAwIGwDAMBgNVHRMBAf8EAjAAMGAGA1UdHwRZMFcwVaBToFGGT2h0dHA6Ly90cnVzdGVkc2VydmljZXMuaW50ZWwuY29tL2NvbnRlbnQvQ1JML1NHWC9BdHRlc3RhdGlvblJlcG9ydFNpZ25pbmdDQS5jcmwwDQYJKoZIhvcNAQELBQADggGBAGcIthtcK9IVRz4rRq+ZKE+7k50/OxUsmW8aavOzKb0iCx07YQ9rzi5nU73tME2yGRLzhSViFs/LpFa9lpQL6JL1aQwmDR74TxYGBAIi5f4I5TJoCCEqRHz91kpG6Uvyn2tLmnIdJbPE4vYvWLrtXXfFBSSPD4Afn7+3/XUggAlc7oCTizOfbbtOFlYA4g5KcYgS1J2ZAeMQqbUdZseZCcaZZZn65tdqee8UXZlDvx0+NdO0LR+5pFy+juM0wWbu59MvzcmTXbjsi7HY6zd53Yq5K244fwFHRQ8eOB0IWB+4PfM7FeAApZvlfqlKOlLcZL2uyVmzRkyR5yW72uo9mehX44CiPJ2fse9Y6eQtcfEhMPkmHXI01sN+KwPbpA39+xOsStjhP9N1Y1a2tQAVo+yVgLgV2Hws73Fc0o3wC78qPEA+v2aRs/Be3ZFDgDyghc/1fgU+7C+P6kbqd4poyb6IW8KCJbxfMJvkordNOgOUUxndPHEi/tb/U7uLjLOgPA==".as_bytes();
        let isv_body = "{\"id\":\"224446224973977124963950294138353548427\",\"timestamp\":\"2020-10-27T07:26:53.412131\",\"version\":3,\"epidPseudonym\":\"4tcrS6EX9pIyhLyxtgpQJuMO1VdAkRDtha/N+u/rRkTsb11AhkuTHsY6UXRPLRJavxG3nsByBdTfyDuBDQTEjMYV6NBXjn3P4UyvG1Ae2+I4lE1n+oiKgLA8CR8pc2nSnSY1Wz1Pw/2l9Q5Er6hM6FdeECgMIVTZzjScYSma6rE=\",\"isvEnclaveQuoteStatus\":\"GROUP_OUT_OF_DATE\",\"platformInfoBlob\":\"1502006504000F00000F0F02040101070000000000000000000B00000B00000002000000000000142ADC0536C0F778E6339B78B7495BDAB064CBC27DA1049CE6739151D0F781995C52276F171A92BE72FDDC4A5602B353742E9DF16256EADC00D3577943656DFEEE1B\",\"isvEnclaveQuoteBody\":\"AgABACoUAAAKAAkAAAAAAP7yPH5zo3mCPOcf8onPvAcAAAAAAAAAAAAAAAAAAAAACBD///8CAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAABwAAAAAAAAAHAAAAAAAAAHgbU309zvOd7HuLzm/fzQMtjYRmQOm1WYtKn2JxiKkIAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAACD1xnnferKFHD2uvYqTXdDA8iZ22kCD5xw7h38CMfOngAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAADLinsnSTdJyTnaS7pyZvFHa7lg50iRgXVEUDISYg3OPJThwmxiLMuahAQViB3u9UErVI8ip9XlwF+0Es/cjlRk\"}".as_bytes();
        let sig: Vec<u8> = vec![153,15,132,203,16,61,189,174,53,69,117,139,125,120,121,86,243,25,28,226,237,230,56,194,238,228,22,182,116,166,245,27,86,43,129,7,122,13,3,143,247,159,97,239,88,200,8,51,238,45,204,71,25,38,46,164,18,85,82,175,13,48,15,190];
    }: {
        swork::Module::<T>::register(RawOrigin::Signed(caller.clone()).into(), ias_sig.to_vec(), ias_cert.to_vec(), caller.clone(), isv_body.to_vec(), sig).expect("Something wrong during registering");
    }

    report_works_with_srd {
        let code: Vec<u8> = vec![120,27,83,125,61,206,243,157,236,123,139,206,111,223,205,3,45,141,132,102,64,233,181,89,139,74,159,98,113,136,169,8];
        swork::Module::<T>::upgrade(RawOrigin::Root.into(), code.clone(), EXPIRE_BLOCK_NUMBER.into()).expect("failed to insert code");

        // Prepare legal work report
        let user: Vec<u8> = vec![212,53,147,199,21,253,211,28,97,20,26,189,4,169,159,214,130,44,133,88,133,76,205,227,154,86,132,231,165,109,162,125]; // Alice
        let caller = T::AccountId::decode(&mut &user[..]).unwrap_or_default();
        let wr = legal_work_report_with_srd();

        // Set block number, system hash and pk in swork
        swork::Module::<T>::insert_pk_info(wr.curr_pk.clone(), code.clone());
        system::Module::<T>::set_block_number(303u32.into());
        let fake_bh:T::Hash = T::Hash::decode(&mut &wr.block_hash[..]).unwrap_or_default();
        let target_block_number:T::BlockNumber = 300u32.into();
        <system::BlockHash<T>>::insert(target_block_number, fake_bh);
    }: {
        swork::Module::<T>::report_works(
            RawOrigin::Signed(caller.clone()).into(),
            wr.curr_pk.clone(),
            wr.prev_pk,
            wr.block_number,
            wr.block_hash,
            wr.free,
            wr.used,
            wr.added_files,
            wr.deleted_files,
            wr.srd_root,
            wr.files_root,
            wr.sig
        ).expect("Something wrong during reporting works");
    } verify {
        assert_eq!(swork::Module::<T>::free(), wr.free as u128);
        assert_eq!(swork::Module::<T>::used(), 0 as u128);
        assert_eq!(swork::Module::<T>::reported_in_slot(&wr.curr_pk, wr.block_number), true);
    }

    report_works_with_added_files {
        let code: Vec<u8> = vec![120,27,83,125,61,206,243,157,236,123,139,206,111,223,205,3,45,141,132,102,64,233,181,89,139,74,159,98,113,136,169,8];
        swork::Module::<T>::upgrade(RawOrigin::Root.into(), code.clone(), EXPIRE_BLOCK_NUMBER.into()).expect("failed to insert code");

        // Prepare legal work report
        let user: Vec<u8> = vec![212,53,147,199,21,253,211,28,97,20,26,189,4,169,159,214,130,44,133,88,133,76,205,227,154,86,132,231,165,109,162,125]; // Alice
        let caller = T::AccountId::decode(&mut &user[..]).unwrap_or_default();
        let wr = legal_work_report_with_added_files();

        // Set block number, system hash and pk in swork
        swork::Module::<T>::insert_pk_info(wr.curr_pk.clone(), code.clone());
        system::Module::<T>::set_block_number(303u32.into());
        let fake_bh:T::Hash = T::Hash::decode(&mut &wr.block_hash[..]).unwrap_or_default();
        let target_block_number:T::BlockNumber = 300u32.into();
        <system::BlockHash<T>>::insert(target_block_number, fake_bh);

        // Prepare Files in market
        add_market_files::<T>(wr.added_files.clone(), caller.clone(), wr.curr_pk.clone());
    }: {
        swork::Module::<T>::report_works(
            RawOrigin::Signed(caller.clone()).into(),
            wr.curr_pk.clone(),
            wr.prev_pk,
            wr.block_number,
            wr.block_hash,
            wr.free,
            wr.used,
            wr.added_files,
            wr.deleted_files,
            wr.srd_root,
            wr.files_root,
            wr.sig
        ).expect("Something wrong during reporting works");
    } verify {
        assert_eq!(swork::Module::<T>::free(), wr.free as u128);
        assert_eq!(swork::Module::<T>::used(), (wr.used * 2) as u128);
        assert_eq!(swork::Module::<T>::reported_in_slot(&wr.curr_pk, wr.block_number), true);
    }

    report_works {
        let code: Vec<u8> = vec![120,27,83,125,61,206,243,157,236,123,139,206,111,223,205,3,45,141,132,102,64,233,181,89,139,74,159,98,113,136,169,8];
        swork::Module::<T>::upgrade(RawOrigin::Root.into(), code.clone(), EXPIRE_BLOCK_NUMBER.into()).expect("failed to insert code");

        // Prepare legal work report
        let user: Vec<u8> = vec![212,53,147,199,21,253,211,28,97,20,26,189,4,169,159,214,130,44,133,88,133,76,205,227,154,86,132,231,165,109,162,125]; // Alice
        let caller = T::AccountId::decode(&mut &user[..]).unwrap_or_default();
        let wr = legal_work_report_with_added_files();

        // Set block number, system hash and pk in swork at 300
        swork::Module::<T>::insert_pk_info(wr.curr_pk.clone(), code.clone());
        system::Module::<T>::set_block_number(303u32.into());
        let fake_bh:T::Hash = T::Hash::decode(&mut &wr.block_hash[..]).unwrap_or_default();
        let target_block_number:T::BlockNumber = 300u32.into();
        <system::BlockHash<T>>::insert(target_block_number, fake_bh);

        // Prepare Files in market
        add_market_files::<T>(wr.added_files.clone(), caller.clone(), wr.curr_pk.clone());

        // Report works at 300
        swork::Module::<T>::report_works(
            RawOrigin::Signed(caller.clone()).into(),
            wr.curr_pk.clone(),
            wr.prev_pk,
            wr.block_number,
            wr.block_hash,
            wr.free,
            wr.used,
            wr.added_files,
            wr.deleted_files,
            wr.srd_root,
            wr.files_root,
            wr.sig
        ).expect("Something wrong during reporting works");

        let wr = legal_work_report_with_deleted_files();
        // Set block number, system hash and pk in swork at 600
        system::Module::<T>::set_block_number(603u32.into());
        let fake_bh:T::Hash = T::Hash::decode(&mut &wr.block_hash[..]).unwrap_or_default();
        let target_block_number:T::BlockNumber = 600u32.into();
        <system::BlockHash<T>>::insert(target_block_number, fake_bh);
    }: {
        swork::Module::<T>::report_works(
            RawOrigin::Signed(caller.clone()).into(),
            wr.curr_pk.clone(),
            wr.prev_pk,
            wr.block_number,
            wr.block_hash,
            wr.free,
            wr.used,
            wr.added_files,
            wr.deleted_files,
            wr.srd_root,
            wr.files_root,
            wr.sig
        ).expect("Something wrong during reporting works");
    } verify {
        assert_eq!(swork::Module::<T>::free(), wr.free as u128);
        assert_eq!(swork::Module::<T>::used(), (wr.used * 2) as u128);
        assert_eq!(swork::Module::<T>::reported_in_slot(&wr.curr_pk, wr.block_number), true);
    }

    create_group {
        let owner: T::AccountId = account("owner", 0, SEED);
    }: {
        swork::Module::<T>::create_group(RawOrigin::Signed(owner.clone()).into()).expect("Something wrong during creating group");
    } verify {
        assert_eq!(<swork::Groups<T>>::contains_key(&owner), true);
    }

    join_group {
        let owner: T::AccountId = account("owner", 0, SEED);
        let member: T::AccountId = account("member", 0, SEED);
        swork::Module::<T>::create_group(RawOrigin::Signed(owner.clone()).into()).expect("Something wrong during creating group");

        let code: Vec<u8> = vec![120,27,83,125,61,206,243,157,236,123,139,206,111,223,205,3,45,141,132,102,64,233,181,89,139,74,159,98,113,136,169,8];
        swork::Module::<T>::upgrade(RawOrigin::Root.into(), code.clone(), EXPIRE_BLOCK_NUMBER.into()).expect("failed to insert code");

        // Prepare legal work report
        let user: Vec<u8> = vec![212,53,147,199,21,253,211,28,97,20,26,189,4,169,159,214,130,44,133,88,133,76,205,227,154,86,132,231,165,109,162,125]; // Alice
        let caller = T::AccountId::decode(&mut &user[..]).unwrap_or_default();
        let wr = legal_work_report_with_deleted_files();

        swork::Module::<T>::insert_pk_info(wr.curr_pk.clone(), code.clone());

        // Set block number, system hash and pk in swork at 600
        system::Module::<T>::set_block_number(603u32.into());
        let fake_bh:T::Hash = T::Hash::decode(&mut &wr.block_hash[..]).unwrap_or_default();
        let target_block_number:T::BlockNumber = 600u32.into();
        <system::BlockHash<T>>::insert(target_block_number, fake_bh);

        swork::Module::<T>::report_works(
            RawOrigin::Signed(member.clone()).into(),
            wr.curr_pk.clone(),
            wr.prev_pk,
            wr.block_number,
            wr.block_hash,
            wr.free,
            wr.used,
            wr.added_files,
            wr.deleted_files,
            wr.srd_root,
            wr.files_root,
            wr.sig
        ).expect("Something wrong during reporting works");
    }: {
        swork::Module::<T>::join_group(RawOrigin::Signed(member.clone()).into(), T::Lookup::unlookup(owner.clone())).expect("Something wrong during joining group");
    } verify {
        assert_eq!(<swork::Groups<T>>::contains_key(&owner), true);
        assert_eq!(swork::Module::<T>::groups(&owner), BTreeSet::from_iter(vec![member.clone()].into_iter()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mock::{ExtBuilder, Test};
    use frame_support::assert_ok;

    #[test]
    fn upgrade() {
        ExtBuilder::default().build().execute_with(|| {
            assert_ok!(test_benchmark_upgrade::<Test>());
        });
    }

    #[test]
    fn report_works() {
        ExtBuilder::default().build().execute_with(|| {
            assert_ok!(test_benchmark_report_works::<Test>());
        });
    }

    #[test]
    fn report_works_with_added_files() {
        ExtBuilder::default().build().execute_with(|| {
            assert_ok!(test_benchmark_report_works_with_added_files::<Test>());
        });
    }

    #[test]
    fn report_works_with_srd() {
        ExtBuilder::default().build().execute_with(|| {
            assert_ok!(test_benchmark_report_works_with_srd::<Test>());
        });
    }

    #[test]
    fn create_group() {
        ExtBuilder::default().build().execute_with(|| {
            assert_ok!(test_benchmark_create_group::<Test>());
        });
    }

    #[test]
    fn join_group() {
        ExtBuilder::default().build().execute_with(|| {
            assert_ok!(test_benchmark_join_group::<Test>());
        });
    }
}



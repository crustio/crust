//! Balances pallet benchmarking.
#![cfg_attr(not(feature = "std"), no_std)]
use sp_std::vec;
use sp_std::prelude::*;
use frame_system::{self as system, RawOrigin};
use frame_benchmarking::{benchmarks, account};
use frame_support::traits::Currency;
use frame_support::storage::StorageMap;
use sp_runtime::traits::StaticLookup;
use codec::Decode;

const SEED: u32 = 0;
const ACCOUNT_BALANCE_RATIO: u32 = 1000000000;
const BLOCK_NUMBER: u32 = 200;

pub struct Module<T: Trait>(swork::Module<T>);
pub trait Trait: market::Trait + swork::Trait {}

fn create_funded_user<T: market::Trait>(string: &'static str, n: u32) -> T::AccountId {
    let user = account(string, n, SEED);
    let balance = T::Currency::minimum_balance() * ACCOUNT_BALANCE_RATIO.into();
    T::Currency::make_free_balance_be(&user, balance);
    user
}

benchmarks! {
    _ {
        let e in 2 .. MAX_EXISTENTIAL_DEPOSIT => ();
        let u in 1 .. MAX_USER_INDEX => ();
    }

     pay_sorders {
        let u in 1..4;

        let code: Vec<u8> = vec![226,86,171,76,181,233,19,107,193,193,17,80,136,252,64,202,31,65,130,84,94,167,87,105,87,140,32,216,67,2,140,213];
        let expire_block: T::BlockNumber = BLOCK_NUMBER.into();
        swork::Module::<T>::upgrade(RawOrigin::Root.into(), code.clone(), expire_block).expect("failed to insert code");
        let user: Vec<u8> = vec![212,53,147,199,21,253,211,28,97,20,26,189,4,169,159,214,130,44,133,88,133,76,205,227,154,86,132,231,165,109,162,125];
        let caller = T::AccountId::decode(&mut &user[..]).unwrap_or_default();
        let pub_key = vec![124,22,192,160,215,161,204,246,84,170,41,37,254,86,87,88,35,151,42,218,160,18,95,251,132,61,154,28,174,14,31,46,164,243,216,32,255,89,213,99,31,248,115,105,57,54,235,198,185,29,10,242,43,130,18,153,1,157,186,207,64,245,121,29];
        let prev_key: Vec<u8> = vec![];
        let block_number = 300;
        // let block_hash = vec![5,64,75,105,11,12,120,91,241,128,178,221,130,164,49,216,141,41,186,243,19,70,197,61,189,169,94,131,227,76,138,117];
        let block_hash = vec![0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0];
        let free = 4294967296;
        let used = 402868224;
        let added_files: Vec<(Vec<u8>, u64)> = vec![
            (vec![91,183,6,50,10,252,99,59,251,132,49,8,228,146,25,43,23,210,182,185,217,238,11,121,94,233,84,23,254,8,182,96],134289408),
            (vec![136,205,179,21,200,195,126,45,192,15,162,168,199,254,81,184,20,155,54,61,41,244,4,68,25,130,249,109,43,186,230,95],268578816)
        ];
        let deleted_files: Vec<(Vec<u8>, u64)> = vec![];
        let sig: Vec<u8> = vec![179,247,136,99,236,151,41,85,217,202,34,212,68,165,71,80,133,164,247,151,90,115,138,186,30,174,29,152,221,113,143,198,145,167,122,53,183,100,161,72,163,168,97,164,162,239,50,121,243,213,226,95,96,124,115,202,133,234,134,225,23,107,166,98];
        let files_root: Vec<u8> = vec![17];
        let srd_root: Vec<u8> = vec![0];
        swork::Module::<T>::maybe_upsert_id(&caller, &pub_key, &code);
        system::Module::<T>::set_block_number(303.into());
        let fake_bh:T::Hash = T::Hash::decode(&mut &block_hash[..]).unwrap_or_default();
        let t_block_number:T::BlockNumber = 300.into();
        <system::BlockHash<T>>::insert(t_block_number, fake_bh);

        let stash = create_funded_user::<T>("stash",u);
        let target_lookup: <T::Lookup as StaticLookup>::Source = <T as frame_system::Trait>::Lookup::unlookup(caller.clone());
        let address_info = "ws://127.0.0.1:8855".as_bytes().to_vec();
        let amount = <T as market::Trait>::Currency::minimum_balance() * 1000000000.into();
        <T as market::Trait>::Currency::make_free_balance_be(&caller, amount.clone() * 2.into());
        market::Module::<T>::pledge(RawOrigin::Signed(caller.clone()).into(), amount).expect("pledge failed");
        market::Module::<T>::register(RawOrigin::Signed(caller.clone()).into(), address_info, <T as market::Trait>::Currency::minimum_balance() * 2.into()).expect("Register failed");
        let file: Vec<u8> = vec![91,183,6,50,10,252,99,59,251,132,49,8,228,146,25,43,23,210,182,185,217,238,11,121,94,233,84,23,254,8,182,96];
        let mut index: u32 = 0;
        for i in 1..10u32.pow(u) {
            index += 1;
            let mut file_alias = "/test/file".as_bytes().to_vec();
            file_alias.extend(index.to_be_bytes().to_vec());
            // file.extend(index.to_be_bytes().to_vec());
            market::Module::<T>::place_storage_order(RawOrigin::Signed(stash.clone()).into(), target_lookup.clone(), file.clone().into(), 1_048_576, 100, file_alias.clone()).expect("Place storage order failed");
        }
        swork::Module::<T>::report_works(RawOrigin::Signed(caller.clone()).into(),
                                         pub_key,
                                         prev_key,
                                         block_number,
                                         block_hash,
                                         free,
                                         used,
                                         added_files,
                                         deleted_files,
                                         srd_root,
                                         files_root,
                                         sig).expect("report work failed");
        let mut file_map = <market::Merchants<T>>::take(caller.clone()).unwrap().file_map;
        let order_ids = file_map.remove(&file).unwrap();
    }: {
        market::Module::<T>::pay_sorders(RawOrigin::Signed(caller.clone()).into(), order_ids.to_vec());
    }

    report_works {
        let u in 1..4;

        let code: Vec<u8> = vec![226,86,171,76,181,233,19,107,193,193,17,80,136,252,64,202,31,65,130,84,94,167,87,105,87,140,32,216,67,2,140,213];
        let expire_block: T::BlockNumber = BLOCK_NUMBER.into();
        swork::Module::<T>::upgrade(RawOrigin::Root.into(), code.clone(), expire_block).expect("failed to insert code");
        let user: Vec<u8> = vec![212,53,147,199,21,253,211,28,97,20,26,189,4,169,159,214,130,44,133,88,133,76,205,227,154,86,132,231,165,109,162,125];
        let caller = T::AccountId::decode(&mut &user[..]).unwrap_or_default();
        let pub_key = vec![124,22,192,160,215,161,204,246,84,170,41,37,254,86,87,88,35,151,42,218,160,18,95,251,132,61,154,28,174,14,31,46,164,243,216,32,255,89,213,99,31,248,115,105,57,54,235,198,185,29,10,242,43,130,18,153,1,157,186,207,64,245,121,29];
        let prev_key: Vec<u8> = vec![];
        let block_number = 300;
        // let block_hash = vec![5,64,75,105,11,12,120,91,241,128,178,221,130,164,49,216,141,41,186,243,19,70,197,61,189,169,94,131,227,76,138,117];
        let block_hash = vec![0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0];
        let free = 4294967296;
        let used = 402868224;
        let added_files: Vec<(Vec<u8>, u64)> = vec![
            (vec![91,183,6,50,10,252,99,59,251,132,49,8,228,146,25,43,23,210,182,185,217,238,11,121,94,233,84,23,254,8,182,96],134289408),
            (vec![136,205,179,21,200,195,126,45,192,15,162,168,199,254,81,184,20,155,54,61,41,244,4,68,25,130,249,109,43,186,230,95],268578816)
        ];
        let deleted_files: Vec<(Vec<u8>, u64)> = vec![];
        let sig: Vec<u8> = vec![179,247,136,99,236,151,41,85,217,202,34,212,68,165,71,80,133,164,247,151,90,115,138,186,30,174,29,152,221,113,143,198,145,167,122,53,183,100,161,72,163,168,97,164,162,239,50,121,243,213,226,95,96,124,115,202,133,234,134,225,23,107,166,98];
        let files_root: Vec<u8> = vec![17];
        let srd_root: Vec<u8> = vec![0];
        swork::Module::<T>::maybe_upsert_id(&caller, &pub_key, &code);
        system::Module::<T>::set_block_number(303.into());
        let fake_bh:T::Hash = T::Hash::decode(&mut &block_hash[..]).unwrap_or_default();
        let t_block_number:T::BlockNumber = 300.into();
        <system::BlockHash<T>>::insert(t_block_number, fake_bh);

        let stash = create_funded_user::<T>("stash",u);
        let target_lookup: <T::Lookup as StaticLookup>::Source = <T as frame_system::Trait>::Lookup::unlookup(caller.clone());
        let address_info = "ws://127.0.0.1:8855".as_bytes().to_vec();
        let amount = <T as market::Trait>::Currency::minimum_balance() * 1000000000.into();
        <T as market::Trait>::Currency::make_free_balance_be(&caller, amount.clone() * 2.into());
        market::Module::<T>::pledge(RawOrigin::Signed(caller.clone()).into(), amount).expect("pledge failed");
        market::Module::<T>::register(RawOrigin::Signed(caller.clone()).into(), address_info, <T as market::Trait>::Currency::minimum_balance() * 2.into()).expect("Register failed");
        let file: Vec<u8> = vec![91,183,6,50,10,252,99,59,251,132,49,8,228,146,25,43,23,210,182,185,217,238,11,121,94,233,84,23,254,8,182,96];
        let mut index: u32 = 0;
        for i in 1..10u32.pow(u) {
            index += 1;
            let mut file_alias = "/test/file".as_bytes().to_vec();
            file_alias.extend(index.to_be_bytes().to_vec());
            // file.extend(index.to_be_bytes().to_vec());
            market::Module::<T>::place_storage_order(RawOrigin::Signed(stash.clone()).into(), target_lookup.clone(), file.clone().into(), 1_048_576, 100, file_alias.clone()).expect("Place storage order failed");
        }
    }: {
        swork::Module::<T>::report_works(RawOrigin::Signed(caller.clone()).into(),
                                         pub_key,
                                         prev_key,
                                         block_number,
                                         block_hash,
                                         free,
                                         used,
                                         added_files,
                                         deleted_files,
                                         srd_root,
                                         files_root,
                                         sig).expect("report work failed");
    }

    update_identities {
        let u in 1..4;

        let code: Vec<u8> = vec![226,86,171,76,181,233,19,107,193,193,17,80,136,252,64,202,31,65,130,84,94,167,87,105,87,140,32,216,67,2,140,213];
        let expire_block: T::BlockNumber = BLOCK_NUMBER.into();
        swork::Module::<T>::upgrade(RawOrigin::Root.into(), code.clone(), expire_block).expect("failed to insert code");
        let user: Vec<u8> = vec![212,53,147,199,21,253,211,28,97,20,26,189,4,169,159,214,130,44,133,88,133,76,205,227,154,86,132,231,165,109,162,125];
        let caller = T::AccountId::decode(&mut &user[..]).unwrap_or_default();
        let pub_key = vec![124,22,192,160,215,161,204,246,84,170,41,37,254,86,87,88,35,151,42,218,160,18,95,251,132,61,154,28,174,14,31,46,164,243,216,32,255,89,213,99,31,248,115,105,57,54,235,198,185,29,10,242,43,130,18,153,1,157,186,207,64,245,121,29];
        let prev_key: Vec<u8> = vec![];
        let block_number = 300;
        // let block_hash = vec![5,64,75,105,11,12,120,91,241,128,178,221,130,164,49,216,141,41,186,243,19,70,197,61,189,169,94,131,227,76,138,117];
        let block_hash = vec![0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0];
        let free = 4294967296;
        let used = 402868224;
        let added_files: Vec<(Vec<u8>, u64)> = vec![
            (vec![91,183,6,50,10,252,99,59,251,132,49,8,228,146,25,43,23,210,182,185,217,238,11,121,94,233,84,23,254,8,182,96],134289408),
            (vec![136,205,179,21,200,195,126,45,192,15,162,168,199,254,81,184,20,155,54,61,41,244,4,68,25,130,249,109,43,186,230,95],268578816)
        ];
        let deleted_files: Vec<(Vec<u8>, u64)> = vec![];
        let sig: Vec<u8> = vec![179,247,136,99,236,151,41,85,217,202,34,212,68,165,71,80,133,164,247,151,90,115,138,186,30,174,29,152,221,113,143,198,145,167,122,53,183,100,161,72,163,168,97,164,162,239,50,121,243,213,226,95,96,124,115,202,133,234,134,225,23,107,166,98];
        let files_root: Vec<u8> = vec![17];
        let srd_root: Vec<u8> = vec![0];
        swork::Module::<T>::maybe_upsert_id(&caller, &pub_key, &code);
        system::Module::<T>::set_block_number(303.into());
        let fake_bh:T::Hash = T::Hash::decode(&mut &block_hash[..]).unwrap_or_default();
        let t_block_number:T::BlockNumber = 300.into();
        <system::BlockHash<T>>::insert(t_block_number, fake_bh);

        let stash = create_funded_user::<T>("stash",u);
        let target_lookup: <T::Lookup as StaticLookup>::Source = <T as frame_system::Trait>::Lookup::unlookup(caller.clone());
        let address_info = "ws://127.0.0.1:8855".as_bytes().to_vec();
        let amount = <T as market::Trait>::Currency::minimum_balance() * 1000000000.into();
        <T as market::Trait>::Currency::make_free_balance_be(&caller, amount.clone() * 2.into());
        market::Module::<T>::pledge(RawOrigin::Signed(caller.clone()).into(), amount).expect("pledge failed");
        market::Module::<T>::register(RawOrigin::Signed(caller.clone()).into(), address_info, <T as market::Trait>::Currency::minimum_balance() * 2.into()).expect("Register failed");
        let file: Vec<u8> = vec![91,183,6,50,10,252,99,59,251,132,49,8,228,146,25,43,23,210,182,185,217,238,11,121,94,233,84,23,254,8,182,96];
        let mut index: u32 = 0;
        for i in 1..10u32.pow(u) {
            index += 1;
            let mut file_alias = "/test/file".as_bytes().to_vec();
            file_alias.extend(index.to_be_bytes().to_vec());
            // file.extend(index.to_be_bytes().to_vec());
            market::Module::<T>::place_storage_order(RawOrigin::Signed(stash.clone()).into(), target_lookup.clone(), file.clone().into(), 1_048_576, 100, file_alias.clone()).expect("Place storage order failed");
        }
        swork::Module::<T>::report_works(RawOrigin::Signed(caller.clone()).into(),
                                         pub_key,
                                         prev_key,
                                         block_number,
                                         block_hash,
                                         free,
                                         used,
                                         added_files,
                                         deleted_files,
                                         srd_root,
                                         files_root,
                                         sig).expect("report work failed");
    }: {
        swork::Module::<T>::update_identities();
    }
    //
    // report_works {
    //     let u in 1..3;
    //     let m in 1..3;
    //
    //     let mut index: u32 = 0;
    //     let code: Vec<u8> = vec![226,86,171,76,181,233,19,107,193,193,17,80,136,252,64,202,31,65,130,84,94,167,87,105,87,140,32,216,67,2,140,213];
    //     let expire_block: T::BlockNumber = BLOCK_NUMBER.into();
    //     swork::Module::<T>::upgrade(RawOrigin::Root.into(), code.clone(), expire_block).expect("failed to insert code");
    //     let user: Vec<u8> = vec![142,175,4,21,22,135,115,99,38,201,254,161,126,37,252,82,135,97,54,147,201,18,144,156,178,38,170,71,148,242,106,72];
    //     let caller = T::AccountId::decode(&mut &user[..]).unwrap_or_default();
    //     let pub_key = vec![176,176,193,145,153,96,115,198,119,71,235,16,104,206,83,3,109,118,135,5,22,162,151,60,239,80,108,41,170,55,50,56,146,197,204,95,55,159,23,230,58,100,187,123,198,159,190,161,64,22,238,167,109,174,97,244,103,194,61,226,149,215,246,137];
    //     let block_number = 300;
    //     let block_hash = vec![5,64,75,105,11,12,120,91,241,128,178,221,130,164,49,216,141,41,186,243,19,70,197,61,189,169,94,131,227,76,138,117];
	// 	// let block_hash = vec![0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0];
	// 	let reserved = 42949672;
    //     let files: Vec<(Vec<u8>, u64)> = vec![
    //         (vec![91,183,6,50,10,252,99,59,251,132,49,8,228,146,25,43,23,210,182,185,217,238,11,121,94,233,84,23,254,8,182,96],134289408),
    //         (vec![136,205,179,21,200,195,126,45,192,15,162,168,199,254,81,184,20,155,54,61,41,244,4,68,25,130,249,109,43,186,230,95],268578816)
    //     ];
    //     let sig: Vec<u8> = vec![156,18,152,108,1,239,231,21,237,139,237,128,183,227,145,96,28,69,191,21,46,40,6,147,255,207,209,10,75,56,109,234,170,15,8,143,194,107,14,190,202,100,195,60,241,34,211,114,235,215,135,170,119,190,170,186,157,46,73,156,228,10,118,230];
    //     let identity = Identity {
    //         pub_key: pub_key.clone(),
    //         code: code
    //     };
    //     swork::Module::<T>::maybe_upsert_id(&caller, &identity);
    //     frame_system::Module::<T>::set_block_number(303.into());
    //     let fake_bh:T::Hash = T::Hash::decode(&mut &block_hash[..]).unwrap_or_default();
    //     let t_block_number:T::BlockNumber = 300.into();
    //     <frame_system::BlockHash<T>>::insert(t_block_number, fake_bh);
    //
    //     let stash = create_funded_user::<T>("stash",u);
    //     let target_lookup: <T::Lookup as StaticLookup>::Source = <T as frame_system::Trait>::Lookup::unlookup(caller.clone());
    //     let address_info = "ws://127.0.0.1:8855".as_bytes().to_vec();
    //     let amount = <T as market::Trait>::Currency::minimum_balance() * 1000000000.into();
    //     <T as market::Trait>::Currency::make_free_balance_be(&caller, amount.clone() * 2.into());
    //     market::Module::<T>::pledge(RawOrigin::Signed(caller.clone()).into(), amount).expect("pledge failed");
    //     market::Module::<T>::register(RawOrigin::Signed(caller.clone()).into(), address_info, <T as market::Trait>::Currency::minimum_balance() * 2.into()).expect("Register failed");
    //     let mut file: Vec<u8> = vec![91,183,6,50,10,252,99,59,251,132,49,8,228,146,25,43,23,210,182,185,217,238,11,121,94,233,84,23,254,8,182,96];
    //     for i in 1..10u32.pow(u) {
    //         index += 1;
    //         let mut file_alias = "/test/file".as_bytes().to_vec();
    //         file_alias.extend(index.to_be_bytes().to_vec());
    //         file.extend(index.to_be_bytes().to_vec());
    //         market::Module::<T>::place_storage_order(RawOrigin::Signed(stash.clone()).into(), target_lookup.clone(), file.clone().into(), 1_048_576, 100, file_alias.clone()).expect("Place storage order failed");
    //     }
    //
    //     for i in 1..10u32.pow(m) {
    //         let stash = create_funded_user::<T>("stash",i);
    //         let target = create_funded_user::<T>("target",i);
    //         let target_lookup: <T::Lookup as StaticLookup>::Source = <T as frame_system::Trait>::Lookup::unlookup(target.clone());
    //         let address_info = "ws://127.0.0.1:8855".as_bytes().to_vec();
    //         let amount = <T as market::Trait>::Currency::minimum_balance() * 1000000000.into();
    //         <T as market::Trait>::Currency::make_free_balance_be(&target, amount.clone() * 2.into());
    //         market::Module::<T>::pledge(RawOrigin::Signed(target.clone()).into(), amount).expect("pledge failed");
    //         market::Module::<T>::register(RawOrigin::Signed(target.clone()).into(), address_info, <T as market::Trait>::Currency::minimum_balance() * 2.into()).expect("Register failed");
    //         let mut file: Vec<u8> = vec![91,183,6,50,10,252,99,59,251,132,49,8,228,146,25,43,23,210,182,185,217,238,11,121,94,233,84,23,254,8,182,96];
    //         for i in 1..10u32.pow(u) {
    //             index += 1;
    //             let mut file_alias = "/test/file".as_bytes().to_vec();
    //             file_alias.extend(index.to_be_bytes().to_vec());
    //             file.extend(index.to_be_bytes().to_vec());
    //             market::Module::<T>::place_storage_order(RawOrigin::Signed(stash.clone()).into(), target_lookup.clone(), file.clone().into(), 1_048_576, 100, file_alias).expect("Place storage order failed");
    //         }
    //     }
    // }: {
    //     // swork::Module::<T>::report_works(RawOrigin::Signed(caller.clone()).into(), pub_key.clone(), block_number.clone(), block_hash.clone(), reserved, files.clone(), sig.clone()).expect("report work failed");
    // }
    //
    // update_identities {
    //     let u in 1..4;
    //
    //     let code: Vec<u8> = vec![226,86,171,76,181,233,19,107,193,193,17,80,136,252,64,202,31,65,130,84,94,167,87,105,87,140,32,216,67,2,140,213];
    //     let expire_block: T::BlockNumber = BLOCK_NUMBER.into();
    //     swork::Module::<T>::upgrade(RawOrigin::Root.into(), code.clone(), expire_block).expect("failed to insert code");
    //     let user: Vec<u8> = vec![142,175,4,21,22,135,115,99,38,201,254,161,126,37,252,82,135,97,54,147,201,18,144,156,178,38,170,71,148,242,106,72];
    //     let caller = T::AccountId::decode(&mut &user[..]).unwrap_or_default();
    //     let pub_key = vec![176,176,193,145,153,96,115,198,119,71,235,16,104,206,83,3,109,118,135,5,22,162,151,60,239,80,108,41,170,55,50,56,146,197,204,95,55,159,23,230,58,100,187,123,198,159,190,161,64,22,238,167,109,174,97,244,103,194,61,226,149,215,246,137];
    //     let block_number = 300;
    //     let block_hash = vec![5,64,75,105,11,12,120,91,241,128,178,221,130,164,49,216,141,41,186,243,19,70,197,61,189,169,94,131,227,76,138,117];
	// 	// let block_hash = vec![0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0];
	// 	let reserved = 42949672;
    //     let files: Vec<(Vec<u8>, u64)> = vec![
    //         (vec![91,183,6,50,10,252,99,59,251,132,49,8,228,146,25,43,23,210,182,185,217,238,11,121,94,233,84,23,254,8,182,96],134289408),
    //         (vec![136,205,179,21,200,195,126,45,192,15,162,168,199,254,81,184,20,155,54,61,41,244,4,68,25,130,249,109,43,186,230,95],268578816)
    //     ];
    //     let sig: Vec<u8> = vec![156,18,152,108,1,239,231,21,237,139,237,128,183,227,145,96,28,69,191,21,46,40,6,147,255,207,209,10,75,56,109,234,170,15,8,143,194,107,14,190,202,100,195,60,241,34,211,114,235,215,135,170,119,190,170,186,157,46,73,156,228,10,118,230];
    //     let identity = Identity {
    //         pub_key: pub_key.clone(),
    //         code: code
    //     };
    //     swork::Module::<T>::maybe_upsert_id(&caller, &identity);
    //     frame_system::Module::<T>::set_block_number(303.into());
    //     let fake_bh:T::Hash = T::Hash::decode(&mut &block_hash[..]).unwrap_or_default();
    //     let t_block_number:T::BlockNumber = 300.into();
    //     <frame_system::BlockHash<T>>::insert(t_block_number, fake_bh);
    //
    //     let stash = create_funded_user::<T>("stash",u);
    //     let target_lookup: <T::Lookup as StaticLookup>::Source = <T as frame_system::Trait>::Lookup::unlookup(caller.clone());
    //     let address_info = "ws://127.0.0.1:8855".as_bytes().to_vec();
    //     let amount = <T as market::Trait>::Currency::minimum_balance() * 1000000000.into();
    //     <T as market::Trait>::Currency::make_free_balance_be(&caller, amount.clone() * 2.into());
    //     market::Module::<T>::pledge(RawOrigin::Signed(caller.clone()).into(), amount).expect("pledge failed");
    //     market::Module::<T>::register(RawOrigin::Signed(caller.clone()).into(), address_info, <T as market::Trait>::Currency::minimum_balance() * 2.into()).expect("Register failed");
    //     let file: Vec<u8> = vec![91,183,6,50,10,252,99,59,251,132,49,8,228,146,25,43,23,210,182,185,217,238,11,121,94,233,84,23,254,8,182,96];
    //     let file_alias = "/test/file1".as_bytes().to_vec();
    //     for i in 1..10u32.pow(u) {
    //         market::Module::<T>::place_storage_order(RawOrigin::Signed(stash.clone()).into(), target_lookup.clone(), file.clone().into(), 1_048_576, 100+i, file_alias.clone()).expect("Place storage order failed");
    //     }
    //     swork::Module::<T>::report_works(RawOrigin::Signed(caller.clone()).into(), pub_key.clone(), block_number.clone(), block_hash.clone(), reserved, files.clone(), sig.clone()).expect("report work failed");
    // }: {
    //     swork::Module::<T>::update_identities();
    // }
}


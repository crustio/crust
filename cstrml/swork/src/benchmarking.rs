// TODO: enable it with new register and report works

use super::*;

use system::{self as frame_system, RawOrigin};
use frame_benchmarking::benchmarks;

use crate::Module as Swork;

const MAX_EXISTENTIAL_DEPOSIT: u32 = 1000;
const MAX_USER_INDEX: u32 = 1000;
const BLOCK_NUMBER: u32 = 200;

benchmarks! {
    _ {
        let e in 2 .. MAX_EXISTENTIAL_DEPOSIT => ();
        let u in 1 .. MAX_USER_INDEX => ();
    }

    upgrade {
        let u in ...;
        let code: Vec<u8> = vec![226,86,171,76,181,233,19,107,193,193,17,80,136,252,64,202,31,65,130,84,94,167,87,105,87,140,32,216,67,2,140,213];    
        let expire_block: T::BlockNumber = BLOCK_NUMBER.into();
    }: _(RawOrigin::Root, code, expire_block)

    register {
        let u in ...;
        let code: Vec<u8> = vec![110,250,232,109,175,224,97,133,3,210,92,225,200,194,249,25,239,179,181,37,165,180,229,67,240,160,234,33,149,160,152,87];
        let expire_block: T::BlockNumber = BLOCK_NUMBER.into();
        Swork::<T>::upgrade(RawOrigin::Root.into(), code, expire_block).expect("failed to insert code");
        let user: Vec<u8> = vec![166,239,163,116,112,15,134,64,183,119,188,146,199,125,52,68,124,85,136,215,235,124,78,201,132,50,60,125,176,152,48,9];
        let caller = T::AccountId::decode(&mut &user[..]).unwrap_or_default();
        let ias_sig = "WldVLYucTO0y8urD2VPnCQncXZglM1MRCiOLwjvBXHWQO7JvoRZfS9tHMJmw1kYIoQkzG2tqxmax90vaNwTli15Nc8umCE6tpNZWxhaV7PIke+6CSRxPu/ttPQ+0ZRpbIhqTaiL0cnhvDTwX8ZSU65gx8nme04Aa+X2RZGlKIvkPa+xlioKwCkTfFO5RcIgQ0qY9bWGa3Dz/JydbsqVTEAXymYmRBji5iK4NA/BbBN5mMFPHdfEnTFV/SyB6oi6NLaIxlqTRQmdw38H8Y5fxtrwAZSzmITITsICoset0YIOj5/BRLcvkyUtNxmKVaeiuAkrShlJgvanOuLW0KEr8oA==".as_bytes();
        let ias_cert = "MIIEoTCCAwmgAwIBAgIJANEHdl0yo7CWMA0GCSqGSIb3DQEBCwUAMH4xCzAJBgNVBAYTAlVTMQswCQYDVQQIDAJDQTEUMBIGA1UEBwwLU2FudGEgQ2xhcmExGjAYBgNVBAoMEUludGVsIENvcnBvcmF0aW9uMTAwLgYDVQQDDCdJbnRlbCBTR1ggQXR0ZXN0YXRpb24gUmVwb3J0IFNpZ25pbmcgQ0EwHhcNMTYxMTIyMDkzNjU4WhcNMjYxMTIwMDkzNjU4WjB7MQswCQYDVQQGEwJVUzELMAkGA1UECAwCQ0ExFDASBgNVBAcMC1NhbnRhIENsYXJhMRowGAYDVQQKDBFJbnRlbCBDb3Jwb3JhdGlvbjEtMCsGA1UEAwwkSW50ZWwgU0dYIEF0dGVzdGF0aW9uIFJlcG9ydCBTaWduaW5nMIIBIjANBgkqhkiG9w0BAQEFAAOCAQ8AMIIBCgKCAQEAqXot4OZuphR8nudFrAFiaGxxkgma/Es/BA+tbeCTUR106AL1ENcWA4FX3K+E9BBL0/7X5rj5nIgX/R/1ubhkKWw9gfqPG3KeAtIdcv/uTO1yXv50vqaPvE1CRChvzdS/ZEBqQ5oVvLTPZ3VEicQjlytKgN9cLnxbwtuvLUK7eyRPfJW/ksddOzP8VBBniolYnRCD2jrMRZ8nBM2ZWYwnXnwYeOAHV+W9tOhAImwRwKF/95yAsVwd21ryHMJBcGH70qLagZ7Ttyt++qO/6+KAXJuKwZqjRlEtSEz8gZQeFfVYgcwSfo96oSMAzVr7V0L6HSDLRnpb6xxmbPdqNol4tQIDAQABo4GkMIGhMB8GA1UdIwQYMBaAFHhDe3amfrzQr35CN+s1fDuHAVE8MA4GA1UdDwEB/wQEAwIGwDAMBgNVHRMBAf8EAjAAMGAGA1UdHwRZMFcwVaBToFGGT2h0dHA6Ly90cnVzdGVkc2VydmljZXMuaW50ZWwuY29tL2NvbnRlbnQvQ1JML1NHWC9BdHRlc3RhdGlvblJlcG9ydFNpZ25pbmdDQS5jcmwwDQYJKoZIhvcNAQELBQADggGBAGcIthtcK9IVRz4rRq+ZKE+7k50/OxUsmW8aavOzKb0iCx07YQ9rzi5nU73tME2yGRLzhSViFs/LpFa9lpQL6JL1aQwmDR74TxYGBAIi5f4I5TJoCCEqRHz91kpG6Uvyn2tLmnIdJbPE4vYvWLrtXXfFBSSPD4Afn7+3/XUggAlc7oCTizOfbbtOFlYA4g5KcYgS1J2ZAeMQqbUdZseZCcaZZZn65tdqee8UXZlDvx0+NdO0LR+5pFy+juM0wWbu59MvzcmTXbjsi7HY6zd53Yq5K244fwFHRQ8eOB0IWB+4PfM7FeAApZvlfqlKOlLcZL2uyVmzRkyR5yW72uo9mehX44CiPJ2fse9Y6eQtcfEhMPkmHXI01sN+KwPbpA39+xOsStjhP9N1Y1a2tQAVo+yVgLgV2Hws73Fc0o3wC78qPEA+v2aRs/Be3ZFDgDyghc/1fgU+7C+P6kbqd4poyb6IW8KCJbxfMJvkordNOgOUUxndPHEi/tb/U7uLjLOgPA==".as_bytes();
        let isv_body = "{\"id\":\"64651438372233583807105649415491449065\",\"timestamp\":\"2020-10-27T07:12:43.407712\",\"version\":3,\"epidPseudonym\":\"4tcrS6EX9pIyhLyxtgpQJuMO1VdAkRDtha/N+u/rRkTsb11AhkuTHsY6UXRPLRJavxG3nsByBdTfyDuBDQTEjMYV6NBXjn3P4UyvG1Ae2+I4lE1n+oiKgLA8CR8pc2nSnSY1Wz1Pw/2l9Q5Er6hM6FdeECgMIVTZzjScYSma6rE=\",\"isvEnclaveQuoteStatus\":\"GROUP_OUT_OF_DATE\",\"platformInfoBlob\":\"1502006504000F00000F0F02040101070000000000000000000B00000B00000002000000000000142ADB138EBD5A898B80677040B950F4564980ABE575AA9FC545EC2406DE1583989629804D5A5E857BB1576AF9CB267F95A87C75867B20EC73BEF409015AF4215AC6\",\"isvEnclaveQuoteBody\":\"AgABACoUAAAKAAkAAAAAAP7yPH5zo3mCPOcf8onPvAcAAAAAAAAAAAAAAAAAAAAACBD///8CAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAABwAAAAAAAAAHAAAAAAAAAG766G2v4GGFA9Jc4cjC+Rnvs7UlpbTlQ/Cg6iGVoJhXAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAACD1xnnferKFHD2uvYqTXdDA8iZ22kCD5xw7h38CMfOngAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAZ0yLLgg4yJX4hoY23wWtx3wn9gK5/2LgM+XTIkZZdy85UtSSe8HJOu0q/VmmtNX9+Mig+NBOl0ou1CAvpJs5s\"}".as_bytes();
        let ab_upgrade_pk = vec![238,112,78,155,203,45,48,108,250,14,117,31,46,123,166,159,255,61,42,162,53,148,174,146,118,54,178,12,169,212,240,21,166,190,123,31,88,33,156,85,99,54,223,120,128,182,57,234,240,114,178,32,108,26,80,88,224,61,248,139,85,139,75,59];
        let sig: Vec<u8> = vec![132,162,61,222,96,159,188,195,62,44,158,209,235,128,97,174,67,100,67,169,210,236,245,136,23,9,158,145,26,235,172,17,13,127,57,14,169,171,49,226,251,35,204,55,33,104,215,76,216,6,53,113,216,79,39,169,219,68,231,72,104,61,122,104];
    }: _(RawOrigin::Signed(caller.clone()), ias_sig.to_vec(), ias_cert.to_vec(), caller.clone(), isv_body.to_vec(), ab_upgrade_pk, sig)

    report_works {
        let u in ...;
        let code: Vec<u8> = vec![226,86,171,76,181,233,19,107,193,193,17,80,136,252,64,202,31,65,130,84,94,167,87,105,87,140,32,216,67,2,140,213];    
        let expire_block: T::BlockNumber = BLOCK_NUMBER.into();
        Swork::<T>::upgrade(RawOrigin::Root.into(), code.clone(), expire_block).expect("failed to insert code");
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
        Swork::<T>::maybe_upsert_id(&caller, &pub_key, &code);
        system::Module::<T>::set_block_number(303.into());
        let fake_bh:T::Hash = T::Hash::decode(&mut &block_hash[..]).unwrap_or_default();
        let t_block_number:T::BlockNumber = 300.into();
        <system::BlockHash<T>>::insert(t_block_number, fake_bh);
    }: _(RawOrigin::Signed(caller.clone()),
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
        sig)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mock::{ExtBuilder, Test};
    use frame_support::assert_ok;

    #[test]
    fn register() {
        ExtBuilder::default()
        .build()
        .execute_with(|| {
            assert_ok!(test_benchmark_register::<Test>());
        });
    }
    
    #[test]
    fn report_works() {
        ExtBuilder::default()
        .build()
        .execute_with(|| {
            assert_ok!(test_benchmark_report_works::<Test>());
        });
    }
}

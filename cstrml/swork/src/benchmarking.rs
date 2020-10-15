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

    register {
        let u in ...;
        let code: Vec<u8> = vec![226,86,171,76,181,233,19,107,193,193,17,80,136,252,64,202,31,65,130,84,94,167,87,105,87,140,32,216,67,2,140,213];    
        let expire_block: T::BlockNumber = BLOCK_NUMBER.into();
        Swork::<T>::upgrade(RawOrigin::Root.into(), code, expire_block).expect("failed to insert code");
        let user: Vec<u8> = vec![166,239,163,116,112,15,134,64,183,119,188,146,199,125,52,68,124,85,136,215,235,124,78,201,132,50,60,125,176,152,48,9];
        let caller = T::AccountId::decode(&mut &user[..]).unwrap_or_default();
        let ias_sig = "Lb17i6Gb2LUoMTYz/fRIjZrsF9X8vxv8S5IZtWjJ2i/BklZO8xeWuS9ItM/8JgDI2qv+zZwZtdgoywK2drH8sV/d0GN/bu5RR4u+bTOJnDWRFkU6lZC9N6AT4ntdFrrkCIfPgikd3dQr21e8v9ShfUy6FT44oLCx21p5knbO1ygxFXzm73nvpLqTB7avRqT3JtHEdzvHjPBymDq18dX7a2cRbK2EwvO48cTcTXihwLZxKjdw7Kds9RC79IaSOVSoBhqBjGtccn9xitj2kPJp65KLU5KpsguTiDwrF79UMsbWI0eKv4voXodNL6YEZdFYELGsp9SpwR6sd4t0628fHg==".as_bytes().to_vec();
        let ias_cert = "-----BEGIN CERTIFICATE-----\nMIIEoTCCAwmgAwIBAgIJANEHdl0yo7CWMA0GCSqGSIb3DQEBCwUAMH4xCzAJBgNV\nBAYTAlVTMQswCQYDVQQIDAJDQTEUMBIGA1UEBwwLU2FudGEgQ2xhcmExGjAYBgNV\nBAoMEUludGVsIENvcnBvcmF0aW9uMTAwLgYDVQQDDCdJbnRlbCBTR1ggQXR0ZXN0\nYXRpb24gUmVwb3J0IFNpZ25pbmcgQ0EwHhcNMTYxMTIyMDkzNjU4WhcNMjYxMTIw\nMDkzNjU4WjB7MQswCQYDVQQGEwJVUzELMAkGA1UECAwCQ0ExFDASBgNVBAcMC1Nh\nbnRhIENsYXJhMRowGAYDVQQKDBFJbnRlbCBDb3Jwb3JhdGlvbjEtMCsGA1UEAwwk\nSW50ZWwgU0dYIEF0dGVzdGF0aW9uIFJlcG9ydCBTaWduaW5nMIIBIjANBgkqhkiG\n9w0BAQEFAAOCAQ8AMIIBCgKCAQEAqXot4OZuphR8nudFrAFiaGxxkgma/Es/BA+t\nbeCTUR106AL1ENcWA4FX3K+E9BBL0/7X5rj5nIgX/R/1ubhkKWw9gfqPG3KeAtId\ncv/uTO1yXv50vqaPvE1CRChvzdS/ZEBqQ5oVvLTPZ3VEicQjlytKgN9cLnxbwtuv\nLUK7eyRPfJW/ksddOzP8VBBniolYnRCD2jrMRZ8nBM2ZWYwnXnwYeOAHV+W9tOhA\nImwRwKF/95yAsVwd21ryHMJBcGH70qLagZ7Ttyt++qO/6+KAXJuKwZqjRlEtSEz8\ngZQeFfVYgcwSfo96oSMAzVr7V0L6HSDLRnpb6xxmbPdqNol4tQIDAQABo4GkMIGh\nMB8GA1UdIwQYMBaAFHhDe3amfrzQr35CN+s1fDuHAVE8MA4GA1UdDwEB/wQEAwIG\nwDAMBgNVHRMBAf8EAjAAMGAGA1UdHwRZMFcwVaBToFGGT2h0dHA6Ly90cnVzdGVk\nc2VydmljZXMuaW50ZWwuY29tL2NvbnRlbnQvQ1JML1NHWC9BdHRlc3RhdGlvblJl\ncG9ydFNpZ25pbmdDQS5jcmwwDQYJKoZIhvcNAQELBQADggGBAGcIthtcK9IVRz4r\nRq+ZKE+7k50/OxUsmW8aavOzKb0iCx07YQ9rzi5nU73tME2yGRLzhSViFs/LpFa9\nlpQL6JL1aQwmDR74TxYGBAIi5f4I5TJoCCEqRHz91kpG6Uvyn2tLmnIdJbPE4vYv\nWLrtXXfFBSSPD4Afn7+3/XUggAlc7oCTizOfbbtOFlYA4g5KcYgS1J2ZAeMQqbUd\nZseZCcaZZZn65tdqee8UXZlDvx0+NdO0LR+5pFy+juM0wWbu59MvzcmTXbjsi7HY\n6zd53Yq5K244fwFHRQ8eOB0IWB+4PfM7FeAApZvlfqlKOlLcZL2uyVmzRkyR5yW7\n2uo9mehX44CiPJ2fse9Y6eQtcfEhMPkmHXI01sN+KwPbpA39+xOsStjhP9N1Y1a2\ntQAVo+yVgLgV2Hws73Fc0o3wC78qPEA+v2aRs/Be3ZFDgDyghc/1fgU+7C+P6kbq\nd4poyb6IW8KCJbxfMJvkordNOgOUUxndPHEi/tb/U7uLjLOgPA==\n-----END CERTIFICATE-----\n".as_bytes().to_vec();
        let isv_body = "{\"id\":\"28059165425966003836075402765879561587\",\"timestamp\":\"2020-06-23T10:02:29.441419\",\"version\":3,\"epidPseudonym\":\"4tcrS6EX9pIyhLyxtgpQJuMO1VdAkRDtha/N+u/rRkTsb11AhkuTHsY6UXRPLRJavxG3nsByBdTfyDuBDQTEjMYV6NBXjn3P4UyvG1Ae2+I4lE1n+oiKgLA8CR8pc2nSnSY1Wz1Pw/2l9Q5Er6hM6FdeECgMIVTZzjScYSma6rE=\",\"isvEnclaveQuoteStatus\":\"GROUP_OUT_OF_DATE\",\"platformInfoBlob\":\"1502006504000F00000F0F02040101070000000000000000000B00000B00000002000000000000142AA23C001F46C3A71CFB50557CE2E2292DFB24EDE2621957E890432F166F6AC6FA37CD8166DBE6323CD39D3C6AA0CB41779FC7EDE281C5E50BCDCA00935E00A9DF\",\"isvEnclaveQuoteBody\":\"AgABACoUAAAKAAkAAAAAAP7yPH5zo3mCPOcf8onPvAcAAAAAAAAAAAAAAAAAAAAACA7///8CAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAABwAAAAAAAAAHAAAAAAAAAOJWq0y16RNrwcERUIj8QMofQYJUXqdXaVeMINhDAozVAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAACD1xnnferKFHD2uvYqTXdDA8iZ22kCD5xw7h38CMfOngAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAABNu2QBUIMjsY9knwTxdDP9S4cgHvP/Y0toS3FchIu2C5Bd1TBeJHYbSWioh139n2q/sxENn6SU3VMNquzMg1Ph\"}".as_bytes().to_vec();
        let sig: Vec<u8> = vec![48,34,6,141,80,243,237,175,99,181,170,184,244,112,137,9,29,28,196,192,207,127,85,153,29,164,14,36,74,61,38,234,107,238,202,236,27,81,61,40,31,149,29,194,17,51,129,70,195,16,7,255,55,11,41,106,175,141,146,149,178,128,107,101];
    }: _(RawOrigin::Signed(caller.clone()), ias_sig, ias_cert, caller.clone(), isv_body, sig)

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

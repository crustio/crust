// TODO: enable it with new register and report works

use super::*;

use system::{self as frame_system, RawOrigin};
use frame_benchmarking::benchmarks;

use crate::Module as Swork;

const BLOCK_NUMBER: u32 = 200;

benchmarks! {
    _{}

    upgrade {
        let code: Vec<u8> = vec![226,86,171,76,181,233,19,107,193,193,17,80,136,252,64,202,31,65,130,84,94,167,87,105,87,140,32,216,67,2,140,213];    
        let expire_block: T::BlockNumber = BLOCK_NUMBER.into();
    }: _(RawOrigin::Root, code, expire_block)

    register {
        let code: Vec<u8> = vec![120,27,83,125,61,206,243,157,236,123,139,206,111,223,205,3,45,141,132,102,64,233,181,89,139,74,159,98,113,136,169,8];
        let expire_block: T::BlockNumber = BLOCK_NUMBER.into();
        Swork::<T>::upgrade(RawOrigin::Root.into(), code, expire_block).expect("failed to insert code");
        let user: Vec<u8> = vec![166,239,163,116,112,15,134,64,183,119,188,146,199,125,52,68,124,85,136,215,235,124,78,201,132,50,60,125,176,152,48,9];
        let caller = T::AccountId::decode(&mut &user[..]).unwrap_or_default();
        let ias_sig = "VWfhb8pfVTHFcwIfFI9fLQPPvScGKwWOtkhYzlIMP5MT/u81VMAJed37p87YyMNwpqopaTP6/QVLkrZFw6fRgONMY+kRyzzkUDB3gRhRh71ZqZe0R+XHsGi6QH0YnMiXtCnD9oP3vSKx8UqhMKRpn4eCUU2jKLkoUOT8fiwozOnrIfYH5aVLcF65Laomj0trgoFbJlm/Yag7HOA3mQMRgCoBzP+xeKZBCWr/Zh6814mnwb8X79KVpM7suiy+g0KuZQpjH9qE32XsBL7lNizqVji9XiAJwN6pbhDmQaRbB8y46mJ1HkII+SFHCyBWAtdiqH9cTsmbsTjAS/TjoXcphQ==".as_bytes();
        let ias_cert = "MIIEoTCCAwmgAwIBAgIJANEHdl0yo7CWMA0GCSqGSIb3DQEBCwUAMH4xCzAJBgNVBAYTAlVTMQswCQYDVQQIDAJDQTEUMBIGA1UEBwwLU2FudGEgQ2xhcmExGjAYBgNVBAoMEUludGVsIENvcnBvcmF0aW9uMTAwLgYDVQQDDCdJbnRlbCBTR1ggQXR0ZXN0YXRpb24gUmVwb3J0IFNpZ25pbmcgQ0EwHhcNMTYxMTIyMDkzNjU4WhcNMjYxMTIwMDkzNjU4WjB7MQswCQYDVQQGEwJVUzELMAkGA1UECAwCQ0ExFDASBgNVBAcMC1NhbnRhIENsYXJhMRowGAYDVQQKDBFJbnRlbCBDb3Jwb3JhdGlvbjEtMCsGA1UEAwwkSW50ZWwgU0dYIEF0dGVzdGF0aW9uIFJlcG9ydCBTaWduaW5nMIIBIjANBgkqhkiG9w0BAQEFAAOCAQ8AMIIBCgKCAQEAqXot4OZuphR8nudFrAFiaGxxkgma/Es/BA+tbeCTUR106AL1ENcWA4FX3K+E9BBL0/7X5rj5nIgX/R/1ubhkKWw9gfqPG3KeAtIdcv/uTO1yXv50vqaPvE1CRChvzdS/ZEBqQ5oVvLTPZ3VEicQjlytKgN9cLnxbwtuvLUK7eyRPfJW/ksddOzP8VBBniolYnRCD2jrMRZ8nBM2ZWYwnXnwYeOAHV+W9tOhAImwRwKF/95yAsVwd21ryHMJBcGH70qLagZ7Ttyt++qO/6+KAXJuKwZqjRlEtSEz8gZQeFfVYgcwSfo96oSMAzVr7V0L6HSDLRnpb6xxmbPdqNol4tQIDAQABo4GkMIGhMB8GA1UdIwQYMBaAFHhDe3amfrzQr35CN+s1fDuHAVE8MA4GA1UdDwEB/wQEAwIGwDAMBgNVHRMBAf8EAjAAMGAGA1UdHwRZMFcwVaBToFGGT2h0dHA6Ly90cnVzdGVkc2VydmljZXMuaW50ZWwuY29tL2NvbnRlbnQvQ1JML1NHWC9BdHRlc3RhdGlvblJlcG9ydFNpZ25pbmdDQS5jcmwwDQYJKoZIhvcNAQELBQADggGBAGcIthtcK9IVRz4rRq+ZKE+7k50/OxUsmW8aavOzKb0iCx07YQ9rzi5nU73tME2yGRLzhSViFs/LpFa9lpQL6JL1aQwmDR74TxYGBAIi5f4I5TJoCCEqRHz91kpG6Uvyn2tLmnIdJbPE4vYvWLrtXXfFBSSPD4Afn7+3/XUggAlc7oCTizOfbbtOFlYA4g5KcYgS1J2ZAeMQqbUdZseZCcaZZZn65tdqee8UXZlDvx0+NdO0LR+5pFy+juM0wWbu59MvzcmTXbjsi7HY6zd53Yq5K244fwFHRQ8eOB0IWB+4PfM7FeAApZvlfqlKOlLcZL2uyVmzRkyR5yW72uo9mehX44CiPJ2fse9Y6eQtcfEhMPkmHXI01sN+KwPbpA39+xOsStjhP9N1Y1a2tQAVo+yVgLgV2Hws73Fc0o3wC78qPEA+v2aRs/Be3ZFDgDyghc/1fgU+7C+P6kbqd4poyb6IW8KCJbxfMJvkordNOgOUUxndPHEi/tb/U7uLjLOgPA==".as_bytes();
        let isv_body = "{\"id\":\"224446224973977124963950294138353548427\",\"timestamp\":\"2020-10-27T07:26:53.412131\",\"version\":3,\"epidPseudonym\":\"4tcrS6EX9pIyhLyxtgpQJuMO1VdAkRDtha/N+u/rRkTsb11AhkuTHsY6UXRPLRJavxG3nsByBdTfyDuBDQTEjMYV6NBXjn3P4UyvG1Ae2+I4lE1n+oiKgLA8CR8pc2nSnSY1Wz1Pw/2l9Q5Er6hM6FdeECgMIVTZzjScYSma6rE=\",\"isvEnclaveQuoteStatus\":\"GROUP_OUT_OF_DATE\",\"platformInfoBlob\":\"1502006504000F00000F0F02040101070000000000000000000B00000B00000002000000000000142ADC0536C0F778E6339B78B7495BDAB064CBC27DA1049CE6739151D0F781995C52276F171A92BE72FDDC4A5602B353742E9DF16256EADC00D3577943656DFEEE1B\",\"isvEnclaveQuoteBody\":\"AgABACoUAAAKAAkAAAAAAP7yPH5zo3mCPOcf8onPvAcAAAAAAAAAAAAAAAAAAAAACBD///8CAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAABwAAAAAAAAAHAAAAAAAAAHgbU309zvOd7HuLzm/fzQMtjYRmQOm1WYtKn2JxiKkIAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAACD1xnnferKFHD2uvYqTXdDA8iZ22kCD5xw7h38CMfOngAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAADLinsnSTdJyTnaS7pyZvFHa7lg50iRgXVEUDISYg3OPJThwmxiLMuahAQViB3u9UErVI8ip9XlwF+0Es/cjlRk\"}".as_bytes();
        let sig: Vec<u8> = vec![153,15,132,203,16,61,189,174,53,69,117,139,125,120,121,86,243,25,28,226,237,230,56,194,238,228,22,182,116,166,245,27,86,43,129,7,122,13,3,143,247,159,97,239,88,200,8,51,238,45,204,71,25,38,46,164,18,85,82,175,13,48,15,190];
    }: _(RawOrigin::Signed(caller.clone()), ias_sig.to_vec(), ias_cert.to_vec(), caller.clone(), isv_body.to_vec(), sig)

    report_works {
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

    chill_pk {
        let code: Vec<u8> = vec![120,27,83,125,61,206,243,157,236,123,139,206,111,223,205,3,45,141,132,102,64,233,181,89,139,74,159,98,113,136,169,8];
        let expire_block: T::BlockNumber = BLOCK_NUMBER.into();
        Swork::<T>::upgrade(RawOrigin::Root.into(), code, expire_block).expect("failed to insert code");
        let user: Vec<u8> = vec![166,239,163,116,112,15,134,64,183,119,188,146,199,125,52,68,124,85,136,215,235,124,78,201,132,50,60,125,176,152,48,9];
        let caller = T::AccountId::decode(&mut &user[..]).unwrap_or_default();
        let ias_sig = "VWfhb8pfVTHFcwIfFI9fLQPPvScGKwWOtkhYzlIMP5MT/u81VMAJed37p87YyMNwpqopaTP6/QVLkrZFw6fRgONMY+kRyzzkUDB3gRhRh71ZqZe0R+XHsGi6QH0YnMiXtCnD9oP3vSKx8UqhMKRpn4eCUU2jKLkoUOT8fiwozOnrIfYH5aVLcF65Laomj0trgoFbJlm/Yag7HOA3mQMRgCoBzP+xeKZBCWr/Zh6814mnwb8X79KVpM7suiy+g0KuZQpjH9qE32XsBL7lNizqVji9XiAJwN6pbhDmQaRbB8y46mJ1HkII+SFHCyBWAtdiqH9cTsmbsTjAS/TjoXcphQ==".as_bytes();
        let ias_cert = "MIIEoTCCAwmgAwIBAgIJANEHdl0yo7CWMA0GCSqGSIb3DQEBCwUAMH4xCzAJBgNVBAYTAlVTMQswCQYDVQQIDAJDQTEUMBIGA1UEBwwLU2FudGEgQ2xhcmExGjAYBgNVBAoMEUludGVsIENvcnBvcmF0aW9uMTAwLgYDVQQDDCdJbnRlbCBTR1ggQXR0ZXN0YXRpb24gUmVwb3J0IFNpZ25pbmcgQ0EwHhcNMTYxMTIyMDkzNjU4WhcNMjYxMTIwMDkzNjU4WjB7MQswCQYDVQQGEwJVUzELMAkGA1UECAwCQ0ExFDASBgNVBAcMC1NhbnRhIENsYXJhMRowGAYDVQQKDBFJbnRlbCBDb3Jwb3JhdGlvbjEtMCsGA1UEAwwkSW50ZWwgU0dYIEF0dGVzdGF0aW9uIFJlcG9ydCBTaWduaW5nMIIBIjANBgkqhkiG9w0BAQEFAAOCAQ8AMIIBCgKCAQEAqXot4OZuphR8nudFrAFiaGxxkgma/Es/BA+tbeCTUR106AL1ENcWA4FX3K+E9BBL0/7X5rj5nIgX/R/1ubhkKWw9gfqPG3KeAtIdcv/uTO1yXv50vqaPvE1CRChvzdS/ZEBqQ5oVvLTPZ3VEicQjlytKgN9cLnxbwtuvLUK7eyRPfJW/ksddOzP8VBBniolYnRCD2jrMRZ8nBM2ZWYwnXnwYeOAHV+W9tOhAImwRwKF/95yAsVwd21ryHMJBcGH70qLagZ7Ttyt++qO/6+KAXJuKwZqjRlEtSEz8gZQeFfVYgcwSfo96oSMAzVr7V0L6HSDLRnpb6xxmbPdqNol4tQIDAQABo4GkMIGhMB8GA1UdIwQYMBaAFHhDe3amfrzQr35CN+s1fDuHAVE8MA4GA1UdDwEB/wQEAwIGwDAMBgNVHRMBAf8EAjAAMGAGA1UdHwRZMFcwVaBToFGGT2h0dHA6Ly90cnVzdGVkc2VydmljZXMuaW50ZWwuY29tL2NvbnRlbnQvQ1JML1NHWC9BdHRlc3RhdGlvblJlcG9ydFNpZ25pbmdDQS5jcmwwDQYJKoZIhvcNAQELBQADggGBAGcIthtcK9IVRz4rRq+ZKE+7k50/OxUsmW8aavOzKb0iCx07YQ9rzi5nU73tME2yGRLzhSViFs/LpFa9lpQL6JL1aQwmDR74TxYGBAIi5f4I5TJoCCEqRHz91kpG6Uvyn2tLmnIdJbPE4vYvWLrtXXfFBSSPD4Afn7+3/XUggAlc7oCTizOfbbtOFlYA4g5KcYgS1J2ZAeMQqbUdZseZCcaZZZn65tdqee8UXZlDvx0+NdO0LR+5pFy+juM0wWbu59MvzcmTXbjsi7HY6zd53Yq5K244fwFHRQ8eOB0IWB+4PfM7FeAApZvlfqlKOlLcZL2uyVmzRkyR5yW72uo9mehX44CiPJ2fse9Y6eQtcfEhMPkmHXI01sN+KwPbpA39+xOsStjhP9N1Y1a2tQAVo+yVgLgV2Hws73Fc0o3wC78qPEA+v2aRs/Be3ZFDgDyghc/1fgU+7C+P6kbqd4poyb6IW8KCJbxfMJvkordNOgOUUxndPHEi/tb/U7uLjLOgPA==".as_bytes();
        let isv_body = "{\"id\":\"224446224973977124963950294138353548427\",\"timestamp\":\"2020-10-27T07:26:53.412131\",\"version\":3,\"epidPseudonym\":\"4tcrS6EX9pIyhLyxtgpQJuMO1VdAkRDtha/N+u/rRkTsb11AhkuTHsY6UXRPLRJavxG3nsByBdTfyDuBDQTEjMYV6NBXjn3P4UyvG1Ae2+I4lE1n+oiKgLA8CR8pc2nSnSY1Wz1Pw/2l9Q5Er6hM6FdeECgMIVTZzjScYSma6rE=\",\"isvEnclaveQuoteStatus\":\"GROUP_OUT_OF_DATE\",\"platformInfoBlob\":\"1502006504000F00000F0F02040101070000000000000000000B00000B00000002000000000000142ADC0536C0F778E6339B78B7495BDAB064CBC27DA1049CE6739151D0F781995C52276F171A92BE72FDDC4A5602B353742E9DF16256EADC00D3577943656DFEEE1B\",\"isvEnclaveQuoteBody\":\"AgABACoUAAAKAAkAAAAAAP7yPH5zo3mCPOcf8onPvAcAAAAAAAAAAAAAAAAAAAAACBD///8CAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAABwAAAAAAAAAHAAAAAAAAAHgbU309zvOd7HuLzm/fzQMtjYRmQOm1WYtKn2JxiKkIAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAACD1xnnferKFHD2uvYqTXdDA8iZ22kCD5xw7h38CMfOngAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAADLinsnSTdJyTnaS7pyZvFHa7lg50iRgXVEUDISYg3OPJThwmxiLMuahAQViB3u9UErVI8ip9XlwF+0Es/cjlRk\"}".as_bytes();
        let sig: Vec<u8> = vec![153,15,132,203,16,61,189,174,53,69,117,139,125,120,121,86,243,25,28,226,237,230,56,194,238,228,22,182,116,166,245,27,86,43,129,7,122,13,3,143,247,159,97,239,88,200,8,51,238,45,204,71,25,38,46,164,18,85,82,175,13,48,15,190];
        Swork::<T>::register(RawOrigin::Signed(caller.clone()).into(), ias_sig.to_vec(), ias_cert.to_vec(), caller.clone(), isv_body.to_vec(), sig).expect("failed to register identity");
        let pk: Vec<u8> = vec![203,138,123,39,73,55,73,201,57,218,75,186,114,102,241,71,107,185,96,231,72,145,129,117,68,80,50,18,98,13,206,60,148,225,194,108,98,44,203,154,132,4,21,136,29,238,245,65,43,84,143,34,167,213,229,192,95,180,18,207,220,142,84,100];
    }: _(RawOrigin::Signed(caller.clone()), pk)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mock::{ExtBuilder, Test};
    use frame_support::assert_ok;

    #[test]
    fn upgrade() {
        ExtBuilder::default()
        .build()
        .execute_with(|| {
            assert_ok!(test_benchmark_upgrade::<Test>());
        });
    }
    
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

    #[test]
    fn chill_pk() {
        ExtBuilder::default()
            .build()
            .execute_with(|| {
                assert_ok!(test_benchmark_chill_pk::<Test>());
            });
    }
}

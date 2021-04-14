// Copyright (C) 2019-2021 Crust Network Technologies Ltd.
// This file is part of Crust.

use crate::*;
use crate as swork;

pub use frame_support::{
    parameter_types, assert_ok,
    weights::{Weight, constants::RocksDbWeight},
    traits::{OnInitialize, OnFinalize, Get, TestRandomness}
};
pub use sp_core::{crypto::{AccountId32, Ss58Codec}, H256};
use sp_runtime::{
    testing::Header, ModuleId,
    traits::{BlakeTwo256, IdentityLookup},
    Perbill,
};
pub use market::{Replica, FileInfo, UsedInfo};
use primitives::MerkleRoot;
use balances::AccountData;
pub use std::{cell::RefCell, collections::HashMap, borrow::Borrow, iter::FromIterator};

pub type AccountId = AccountId32;
pub type Balance = u64;

thread_local! {
    static EXISTENTIAL_DEPOSIT: RefCell<u64> = RefCell::new(0);
    static LEGAL_PK: Vec<u8> = hex::decode("cb8a7b27493749c939da4bba7266f1476bb960e74891817544503212620dce3c94e1c26c622ccb9a840415881deef5412b548f22a7d5e5c05fb412cfdc8e5464").unwrap();
    static LEGAL_CODE: Vec<u8> = hex::decode("781b537d3dcef39dec7b8bce6fdfcd032d8d846640e9b5598b4a9f627188a908").unwrap();
    static WORKLOAD_MAP: RefCell<HashMap<AccountId, u128>> = RefCell::new(Default::default());
}

pub struct ExistentialDeposit;
impl Get<u64> for ExistentialDeposit {
    fn get() -> u64 {
        EXISTENTIAL_DEPOSIT.with(|v| *v.borrow())
    }
}

pub struct LegalPK;
impl Get<Vec<u8>> for LegalPK {
    fn get() -> Vec<u8> {
        LEGAL_PK.with(|pk| pk.clone())
    }
}

pub struct LegalCode;
impl Get<Vec<u8>> for LegalCode {
    fn get() -> Vec<u8> {
        LEGAL_CODE.with(|code| code.clone())
    }
}

pub struct WorkloadMap;
impl Get<HashMap<AccountId, u128>> for WorkloadMap {
    fn get() -> HashMap<AccountId, u128> {
        WORKLOAD_MAP.with(|map| map.borrow().clone())
    }
}
impl WorkloadMap {
    fn set(who: &AccountId, own_workload: u128) {
        WORKLOAD_MAP.with(|map| {
            let mut map = map.borrow_mut();
            map.insert(who.clone(), own_workload);
        })
    }
}

pub struct RegisterInfo {
    pub ias_sig: IASSig,
    pub ias_cert: SworkerCert,
    pub account_id: AccountId,
    pub isv_body: ISVBody,
    pub sig: SworkerSignature
}

pub struct ReportWorksInfo {
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

parameter_types! {
	pub const BlockHashCount: u32 = 250;
	pub BlockWeights: frame_system::limits::BlockWeights =
		frame_system::limits::BlockWeights::simple_max(4 * 1024 * 1024);
}

impl system::Config for Test {
    type BaseCallFilter = ();
    type BlockWeights = BlockWeights;
    type BlockLength = ();
    type Origin = Origin;
    type Call = Call;
    type Index = u64;
    type BlockNumber = u64;
    type Hash = H256;
    type Hashing = BlakeTwo256;
    type AccountId = AccountId;
    type Lookup = IdentityLookup<Self::AccountId>;
    type Header = Header;
    type Event = ();
    type BlockHashCount = BlockHashCount;
    type DbWeight = RocksDbWeight;
    type Version = ();
    type PalletInfo = PalletInfo;
    type AccountData = AccountData<u64>;
    type OnNewAccount = ();
    type OnKilledAccount = ();
    type SystemWeightInfo = ();
    type SS58Prefix = ();
}

impl balances::Config for Test {
    type Balance = Balance;
    type DustRemoval = ();
    type Event = ();
    type ExistentialDeposit = ExistentialDeposit;
    type AccountStore = System;
    type WeightInfo = ();
    type MaxLocks = ();
}

parameter_types! {
    /// Unit is pico
    pub const MarketModuleId: ModuleId = ModuleId(*b"crmarket");
    pub const FileDuration: BlockNumber = 1000;
    pub const FileReplica: u32 = 4;
    pub const FileInitPrice: Balance = 1000; // Need align with FileDuration and FileBaseReplica
    pub const StorageReferenceRatio: (u128, u128) = (1, 2);
    pub const StorageIncreaseRatio: Perbill = Perbill::from_percent(1);
    pub const StorageDecreaseRatio: Perbill = Perbill::from_percent(1);
    pub const StakingRatio: Perbill = Perbill::from_percent(72);
    pub const StorageRatio: Perbill = Perbill::from_percent(18);
    pub const UsedTrashMaxSize: u128 = 2;
    pub const MaximumFileSize: u64 = 137_438_953_472; // 128G = 128 * 1024 * 1024 * 1024
    pub const RenewRewardRatio: Perbill = Perbill::from_percent(5);
}

impl market::Config for Test {
    type ModuleId = MarketModuleId;
    type Currency = balances::Module<Self>;
    type CurrencyToBalance = ();
    type SworkerInterface = Swork;
    type Event = ();
    /// File duration.
    type FileDuration = FileDuration;
    type FileReplica = FileReplica;
    type FileInitPrice = FileInitPrice;
    type StorageReferenceRatio = StorageReferenceRatio;
    type StorageIncreaseRatio = StorageIncreaseRatio;
    type StorageDecreaseRatio = StorageDecreaseRatio;
    type StakingRatio = StakingRatio;
    type RenewRewardRatio = RenewRewardRatio;
    type StorageRatio = StorageRatio;
    type UsedTrashMaxSize = UsedTrashMaxSize;
    type MaximumFileSize = MaximumFileSize;
    type WeightInfo = market::weight::WeightInfo<Test>;
}

pub struct TestWorksInterface;

impl Works<AccountId> for TestWorksInterface {
    fn report_works(workload_map: BTreeMap<AccountId, u128>, _: u128) {
        // Disable work report in mock test
        for (who, own_workload) in workload_map.iter() {
            WorkloadMap::set(who, *own_workload);
        }
    }
}

parameter_types! {
    pub const PunishmentSlots: u32 = 4;
    pub const MaxGroupSize: u32 = 4;
}

impl Config for Test {
    type Currency = balances::Module<Self>;
    type Event = ();
    type PunishmentSlots = PunishmentSlots;
    type Works = TestWorksInterface;
    type MarketInterface = Market;
    type MaxGroupSize = MaxGroupSize;
    type WeightInfo = weight::WeightInfo<Test>;
}

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
type Block = frame_system::mocking::MockBlock<Test>;

frame_support::construct_runtime!(
	pub enum Test where
		Block = Block,
		NodeBlock = Block,
		UncheckedExtrinsic = UncheckedExtrinsic
	{
		System: frame_system::{Module, Call, Config, Storage, Event<T>},
		Balances: balances::{Module, Call, Storage, Config<T>, Event<T>},
		Swork: swork::{Module, Call, Storage, Event<T>, Config<T>},
		Market: market::{Module, Call, Storage, Event<T>, Config},
	}
);

pub struct ExtBuilder {
    code: SworkerCode,
    expired_bn: u64
}

impl Default for ExtBuilder {
    fn default() -> Self {
        Self {
            code: LegalCode::get(),
            expired_bn: 3000
        }
    }
}

impl ExtBuilder {
    pub fn code(mut self, code: SworkerCode) -> Self {
        self.code = code;
        self
    }

    pub fn expired_bn(mut self, expired_bn: u64) -> Self {
        self.expired_bn = expired_bn;
        self
    }

    pub fn build(self) -> sp_io::TestExternalities {
        let mut t = system::GenesisConfig::default()
            .build_storage::<Test>()
            .unwrap();

        let fake_code = hex::decode("00").unwrap();
        swork::GenesisConfig::<Test> {
            init_codes: vec![(self.code, self.expired_bn), (fake_code, 10000)],
        }.assimilate_storage(&mut t).unwrap();

        let mut ext: sp_io::TestExternalities = t.into();
        ext.execute_with(|| {
            assert_ok!(Market::set_market_switch(Origin::root(), true));
            assert_ok!(Market::set_base_fee(Origin::root(), 1000));
        });

        ext
    }
}

/// Run until a particular block.
pub fn run_to_block(n: u64) {
    // This block hash is for the valid work report
    // let bh = maybe_bh.unwrap_or(hex::decode("05404b690b0c785bf180b2dd82a431d88d29baf31346c53dbda95e83e34c8a75").unwrap());
    // let fake_bh = H256::from_slice(bh.as_slice());
    while System::block_number() < n {
        // <system::BlockHash<Test>>::insert(System::block_number(), fake_bh.clone());
        if System::block_number() > 1 {
            System::on_finalize(System::block_number());
        }
        System::on_initialize(System::block_number());
        System::set_block_number(System::block_number() + 1);
    }
}

/// Build allllllll fucking kinds of stupid work reports 🤬
/// TODO: move work report generator into this repo
pub fn legal_register_info() -> RegisterInfo {
    let applier: AccountId =
        AccountId::from_ss58check("5FqazaU79hjpEMiWTWZx81VjsYFst15eBuSBKdQLgQibD7CX")
            .expect("valid ss58 address");

    let ias_sig = "VWfhb8pfVTHFcwIfFI9fLQPPvScGKwWOtkhYzlIMP5MT/u81VMAJed37p87YyMNwpqopaTP6/QVLkrZFw6fRgONMY+kRyzzkUDB3gRhRh71ZqZe0R+XHsGi6QH0YnMiXtCnD9oP3vSKx8UqhMKRpn4eCUU2jKLkoUOT8fiwozOnrIfYH5aVLcF65Laomj0trgoFbJlm/Yag7HOA3mQMRgCoBzP+xeKZBCWr/Zh6814mnwb8X79KVpM7suiy+g0KuZQpjH9qE32XsBL7lNizqVji9XiAJwN6pbhDmQaRbB8y46mJ1HkII+SFHCyBWAtdiqH9cTsmbsTjAS/TjoXcphQ==".as_bytes();
    let ias_cert = "MIIEoTCCAwmgAwIBAgIJANEHdl0yo7CWMA0GCSqGSIb3DQEBCwUAMH4xCzAJBgNVBAYTAlVTMQswCQYDVQQIDAJDQTEUMBIGA1UEBwwLU2FudGEgQ2xhcmExGjAYBgNVBAoMEUludGVsIENvcnBvcmF0aW9uMTAwLgYDVQQDDCdJbnRlbCBTR1ggQXR0ZXN0YXRpb24gUmVwb3J0IFNpZ25pbmcgQ0EwHhcNMTYxMTIyMDkzNjU4WhcNMjYxMTIwMDkzNjU4WjB7MQswCQYDVQQGEwJVUzELMAkGA1UECAwCQ0ExFDASBgNVBAcMC1NhbnRhIENsYXJhMRowGAYDVQQKDBFJbnRlbCBDb3Jwb3JhdGlvbjEtMCsGA1UEAwwkSW50ZWwgU0dYIEF0dGVzdGF0aW9uIFJlcG9ydCBTaWduaW5nMIIBIjANBgkqhkiG9w0BAQEFAAOCAQ8AMIIBCgKCAQEAqXot4OZuphR8nudFrAFiaGxxkgma/Es/BA+tbeCTUR106AL1ENcWA4FX3K+E9BBL0/7X5rj5nIgX/R/1ubhkKWw9gfqPG3KeAtIdcv/uTO1yXv50vqaPvE1CRChvzdS/ZEBqQ5oVvLTPZ3VEicQjlytKgN9cLnxbwtuvLUK7eyRPfJW/ksddOzP8VBBniolYnRCD2jrMRZ8nBM2ZWYwnXnwYeOAHV+W9tOhAImwRwKF/95yAsVwd21ryHMJBcGH70qLagZ7Ttyt++qO/6+KAXJuKwZqjRlEtSEz8gZQeFfVYgcwSfo96oSMAzVr7V0L6HSDLRnpb6xxmbPdqNol4tQIDAQABo4GkMIGhMB8GA1UdIwQYMBaAFHhDe3amfrzQr35CN+s1fDuHAVE8MA4GA1UdDwEB/wQEAwIGwDAMBgNVHRMBAf8EAjAAMGAGA1UdHwRZMFcwVaBToFGGT2h0dHA6Ly90cnVzdGVkc2VydmljZXMuaW50ZWwuY29tL2NvbnRlbnQvQ1JML1NHWC9BdHRlc3RhdGlvblJlcG9ydFNpZ25pbmdDQS5jcmwwDQYJKoZIhvcNAQELBQADggGBAGcIthtcK9IVRz4rRq+ZKE+7k50/OxUsmW8aavOzKb0iCx07YQ9rzi5nU73tME2yGRLzhSViFs/LpFa9lpQL6JL1aQwmDR74TxYGBAIi5f4I5TJoCCEqRHz91kpG6Uvyn2tLmnIdJbPE4vYvWLrtXXfFBSSPD4Afn7+3/XUggAlc7oCTizOfbbtOFlYA4g5KcYgS1J2ZAeMQqbUdZseZCcaZZZn65tdqee8UXZlDvx0+NdO0LR+5pFy+juM0wWbu59MvzcmTXbjsi7HY6zd53Yq5K244fwFHRQ8eOB0IWB+4PfM7FeAApZvlfqlKOlLcZL2uyVmzRkyR5yW72uo9mehX44CiPJ2fse9Y6eQtcfEhMPkmHXI01sN+KwPbpA39+xOsStjhP9N1Y1a2tQAVo+yVgLgV2Hws73Fc0o3wC78qPEA+v2aRs/Be3ZFDgDyghc/1fgU+7C+P6kbqd4poyb6IW8KCJbxfMJvkordNOgOUUxndPHEi/tb/U7uLjLOgPA==".as_bytes();
    let isv_body = "{\"id\":\"224446224973977124963950294138353548427\",\"timestamp\":\"2020-10-27T07:26:53.412131\",\"version\":3,\"epidPseudonym\":\"4tcrS6EX9pIyhLyxtgpQJuMO1VdAkRDtha/N+u/rRkTsb11AhkuTHsY6UXRPLRJavxG3nsByBdTfyDuBDQTEjMYV6NBXjn3P4UyvG1Ae2+I4lE1n+oiKgLA8CR8pc2nSnSY1Wz1Pw/2l9Q5Er6hM6FdeECgMIVTZzjScYSma6rE=\",\"isvEnclaveQuoteStatus\":\"GROUP_OUT_OF_DATE\",\"platformInfoBlob\":\"1502006504000F00000F0F02040101070000000000000000000B00000B00000002000000000000142ADC0536C0F778E6339B78B7495BDAB064CBC27DA1049CE6739151D0F781995C52276F171A92BE72FDDC4A5602B353742E9DF16256EADC00D3577943656DFEEE1B\",\"isvEnclaveQuoteBody\":\"AgABACoUAAAKAAkAAAAAAP7yPH5zo3mCPOcf8onPvAcAAAAAAAAAAAAAAAAAAAAACBD///8CAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAABwAAAAAAAAAHAAAAAAAAAHgbU309zvOd7HuLzm/fzQMtjYRmQOm1WYtKn2JxiKkIAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAACD1xnnferKFHD2uvYqTXdDA8iZ22kCD5xw7h38CMfOngAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAADLinsnSTdJyTnaS7pyZvFHa7lg50iRgXVEUDISYg3OPJThwmxiLMuahAQViB3u9UErVI8ip9XlwF+0Es/cjlRk\"}".as_bytes();
    let sig = hex::decode("990f84cb103dbdae3545758b7d787956f3191ce2ede638c2eee416b674a6f51b562b81077a0d038ff79f61ef58c80833ee2dcc4719262ea4125552af0d300fbe").unwrap();

    RegisterInfo {
        ias_sig: ias_sig.to_vec(),
        ias_cert: ias_cert.to_vec(),
        account_id: applier,
        isv_body: isv_body.to_vec(),
        sig
    }
}

pub fn another_legal_register_info() -> RegisterInfo {
    let applier: AccountId =
        AccountId::from_ss58check("5FqazaU79hjpEMiWTWZx81VjsYFst15eBuSBKdQLgQibD7CX")
            .expect("valid ss58 address");

    let ias_sig = "WxX/4Wk6Nj6kBApve41yh/00gmQhinq/HTgb0DQAso4JybPbtw7WURpY/MUrpsXOzTdJrTHGNKhH42hddvU5boa7wGbI/6wXuJ7jT1eU6YHlX0rmAgjVnSVUlFVSi2ExTgWnYPYrxktJfHRNlKgImQF5Cq7VKQ9CIQA/9tqLJk35saibBIy7AoQzyx0Qdi2bInM1OUCPFDrliaOSaHK31ufe6CHX/HJA0LwyvljteeQRduS5EX9rv6aRAMdf9itqWmMwEs6vKVy4bgi9G+86KXMXDyDrDn4u/ZOsG4LnymWGZehiaKctLIodCNx81cBRAecWQ5MU3GX3CMkAIvVhjw==".as_bytes();
    let ias_cert = "MIIEoTCCAwmgAwIBAgIJANEHdl0yo7CWMA0GCSqGSIb3DQEBCwUAMH4xCzAJBgNVBAYTAlVTMQswCQYDVQQIDAJDQTEUMBIGA1UEBwwLU2FudGEgQ2xhcmExGjAYBgNVBAoMEUludGVsIENvcnBvcmF0aW9uMTAwLgYDVQQDDCdJbnRlbCBTR1ggQXR0ZXN0YXRpb24gUmVwb3J0IFNpZ25pbmcgQ0EwHhcNMTYxMTIyMDkzNjU4WhcNMjYxMTIwMDkzNjU4WjB7MQswCQYDVQQGEwJVUzELMAkGA1UECAwCQ0ExFDASBgNVBAcMC1NhbnRhIENsYXJhMRowGAYDVQQKDBFJbnRlbCBDb3Jwb3JhdGlvbjEtMCsGA1UEAwwkSW50ZWwgU0dYIEF0dGVzdGF0aW9uIFJlcG9ydCBTaWduaW5nMIIBIjANBgkqhkiG9w0BAQEFAAOCAQ8AMIIBCgKCAQEAqXot4OZuphR8nudFrAFiaGxxkgma/Es/BA+tbeCTUR106AL1ENcWA4FX3K+E9BBL0/7X5rj5nIgX/R/1ubhkKWw9gfqPG3KeAtIdcv/uTO1yXv50vqaPvE1CRChvzdS/ZEBqQ5oVvLTPZ3VEicQjlytKgN9cLnxbwtuvLUK7eyRPfJW/ksddOzP8VBBniolYnRCD2jrMRZ8nBM2ZWYwnXnwYeOAHV+W9tOhAImwRwKF/95yAsVwd21ryHMJBcGH70qLagZ7Ttyt++qO/6+KAXJuKwZqjRlEtSEz8gZQeFfVYgcwSfo96oSMAzVr7V0L6HSDLRnpb6xxmbPdqNol4tQIDAQABo4GkMIGhMB8GA1UdIwQYMBaAFHhDe3amfrzQr35CN+s1fDuHAVE8MA4GA1UdDwEB/wQEAwIGwDAMBgNVHRMBAf8EAjAAMGAGA1UdHwRZMFcwVaBToFGGT2h0dHA6Ly90cnVzdGVkc2VydmljZXMuaW50ZWwuY29tL2NvbnRlbnQvQ1JML1NHWC9BdHRlc3RhdGlvblJlcG9ydFNpZ25pbmdDQS5jcmwwDQYJKoZIhvcNAQELBQADggGBAGcIthtcK9IVRz4rRq+ZKE+7k50/OxUsmW8aavOzKb0iCx07YQ9rzi5nU73tME2yGRLzhSViFs/LpFa9lpQL6JL1aQwmDR74TxYGBAIi5f4I5TJoCCEqRHz91kpG6Uvyn2tLmnIdJbPE4vYvWLrtXXfFBSSPD4Afn7+3/XUggAlc7oCTizOfbbtOFlYA4g5KcYgS1J2ZAeMQqbUdZseZCcaZZZn65tdqee8UXZlDvx0+NdO0LR+5pFy+juM0wWbu59MvzcmTXbjsi7HY6zd53Yq5K244fwFHRQ8eOB0IWB+4PfM7FeAApZvlfqlKOlLcZL2uyVmzRkyR5yW72uo9mehX44CiPJ2fse9Y6eQtcfEhMPkmHXI01sN+KwPbpA39+xOsStjhP9N1Y1a2tQAVo+yVgLgV2Hws73Fc0o3wC78qPEA+v2aRs/Be3ZFDgDyghc/1fgU+7C+P6kbqd4poyb6IW8KCJbxfMJvkordNOgOUUxndPHEi/tb/U7uLjLOgPA==".as_bytes();
    let isv_body = "{\"id\":\"316236081840382895866143746657929488303\",\"timestamp\":\"2021-03-17T03:35:36.464149\",\"version\":3,\"isvEnclaveQuoteStatus\":\"GROUP_OUT_OF_DATE\",\"platformInfoBlob\":\"150200650400090000111102040101070000000000000000000B00000B00000002000000000000142A8E8FEC586F414E9D9F4CFEF7199A1900A3778FCCB0E08BE38C0DC3133E9D33945EE397E0F541FF76C9D55D2AA73D65846FB1380BDD81E6BDE8A373EC991D4F83\",\"isvEnclaveQuoteBody\":\"AgAAACoUAAALAAoAAAAAAGaNNT9mGXhlXJ1oIM+TtmviJxnXbu/ZNUKvjn4ZHlOqCBH///8CAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAABQAAAAAAAAAHAAAAAAAAADQ8L7V8NMsGynPd0NBFuiDexSnhqYoND37X+RvY9fJhAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAC9G637ogCU4mSdrLTybzFAmba2MemLakS5KMgTEQp4QAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAbxh3LcyKxbGaH7UDBviaQ01ksz8eywd6nqv3cLyp/zZS18T7hBd4qRm6QQL5/vO5U7NffHpfwrqdh0jPJdscY\"}".as_bytes();
    let sig = hex::decode("a174199f7f9882f60a5d0b2030bcd5fa386623ff819ed3c3e4d17f6918b527f6968dbde80a37ad71c98eaeadac26794830599d28e98e759620e83633b6f7411f").unwrap();

    RegisterInfo {
        ias_sig: ias_sig.to_vec(),
        ias_cert: ias_cert.to_vec(),
        account_id: applier,
        isv_body: isv_body.to_vec(),
        sig
    }
}

pub fn legal_work_report() -> ReportWorksInfo {
    let curr_pk = hex::decode("5c7351b8a5098235505c66b4ba42269cb58a1fbf410906235204406a6080d9640d406a8a26d25efd3988250f80cf3dd5b6c22a70d964d08f03a8e618e51b4812").unwrap();
    let prev_pk = hex::decode("").unwrap();
    let block_number: u64 = 300;
    let block_hash = hex::decode("0000000000000000000000000000000000000000000000000000000000000000").unwrap();
    let free: u64 = 4294967296;
    let used: u64 = 2;
    let added_files: Vec<(Vec<u8>, u64, u64)> = vec![];
    let deleted_files: Vec<(Vec<u8>, u64, u64)> = vec![];
    let files_root = hex::decode("11").unwrap();
    let srd_root = hex::decode("00").unwrap();
    let sig = hex::decode("ba7a02712bdde25048624787e4a21882964e5fc911cfe394bff1ff9964434531e33d62b2fe9c3b0bba68b9d110da490178474d073a299968c7e2eb2ca5a0b754").unwrap();

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

pub fn legal_work_report_with_added_and_deleted_files() -> ReportWorksInfo {
    let mut legal_wr = legal_work_report();
    legal_wr.block_number = 600;
    legal_wr.added_files = vec![
        ("QmdwgqZy1MZBfWPi7GcxVsYgJEtmvHg6rsLzbCej3tf3oA".as_bytes().to_vec(), 13, 503),
    ];
    legal_wr.deleted_files = vec![
        ("QmdwgqZy1MZBfWPi7GcxVsYgJEtmvHg6rsLzbCej3tf3oA".as_bytes().to_vec(), 13, 503),
    ];
    legal_wr.files_root = hex::decode("22").unwrap();
    legal_wr.sig = hex::decode("938482733733576dd997d71e2d7baa984fdb03b019b4493f9e6b2f237e9c7b590199973f6dc9bac384ba8cc1601488eab336a5b2e420329112f2b177d5ffdf1b").unwrap();
    legal_wr
}

pub fn continuous_ab_upgrade_work_report() -> ReportWorksInfo {
    let mut legal_wr = ab_upgrade_work_report();
    legal_wr.block_number = 900;
    legal_wr.added_files = vec![("QmdwgqZy1MZBfWPi7GcxVsYgJEtmvHg6rsLzbCej3tf3oC".as_bytes().to_vec(), 37, 903)];
    legal_wr.deleted_files = vec![("QmdwgqZy1MZBfWPi7GcxVsYgJEtmvHg6rsLzbCej3tf3oB".as_bytes().to_vec(), 7, 903)];
    legal_wr.used = 32;
    legal_wr.sig = hex::decode("c5e5ce5a1632afc8c0fb22dc44cb1f25bfa44df8455d6836578c02ffa203480b0a61e465205c893d4cbdcefc9d0742019f075b5c62c7ae0fc580d7d47f3930f5").unwrap();
    legal_wr
}

pub fn ab_upgrade_work_report() -> ReportWorksInfo {
    let mut legal_wr = legal_work_report();
    legal_wr.block_number = 600;
    legal_wr.curr_pk = hex::decode("09a6f382dac3549cba71b27f7c56328e9dc72d3c8ee15601e4d2418c4f04b2008bc3a71b617c684ec35b24a714fbd550908f67ac2d61a841bbd2f79cc8c74bd1").unwrap();
    legal_wr.prev_pk = hex::decode("5c7351b8a5098235505c66b4ba42269cb58a1fbf410906235204406a6080d9640d406a8a26d25efd3988250f80cf3dd5b6c22a70d964d08f03a8e618e51b4812").unwrap();
    legal_wr.sig = hex::decode("e8adb0cfc9067f447ca33392ccac6c9c263f3ce0f91d8fd18bb4c6df32565acd44aae8923d66a20965a0e8e8e3b947a0dbd97f709d24ec0b59d2b24594a64b74").unwrap();
    legal_wr
}

pub fn ab_upgrade_work_report_files_size_unmatch() -> ReportWorksInfo {
    let mut legal_wr = ab_upgrade_work_report();
    legal_wr.added_files = vec![("QmdwgqZy1MZBfWPi7GcxVsYgJEtmvHg6rsLzbCej3tf3oA".as_bytes().to_vec(), 10, 606)];
    legal_wr.sig = hex::decode("e0096948d5b04a312e72f48072f6964ab33a7a3ff787e78df075aa925cc6ec0ad668dee968bcedcfc7df7482549c910153f52f9038c8f5cb019fa988169f5400").unwrap();
    legal_wr
}

pub fn continuous_work_report_300() -> ReportWorksInfo {
    let mut legal_wr = legal_work_report();
    legal_wr.block_number = 300;
    legal_wr.curr_pk = hex::decode("09a6f382dac3549cba71b27f7c56328e9dc72d3c8ee15601e4d2418c4f04b2008bc3a71b617c684ec35b24a714fbd550908f67ac2d61a841bbd2f79cc8c74bd1").unwrap();
    legal_wr.sig = hex::decode("9877fac3c4c3d3d88ae0175dee6159cc183c643a4ed51e46acb681a3bbdf064f6d55147d92f46356128c218f1eeba7d1ea1039631af6b6b1b3f45d15eedff78e").unwrap();

    legal_wr
}

pub fn continuous_work_report_600() -> ReportWorksInfo {
    let mut legal_wr = legal_work_report();
    legal_wr.block_number = 600;
    legal_wr.curr_pk = hex::decode("09a6f382dac3549cba71b27f7c56328e9dc72d3c8ee15601e4d2418c4f04b2008bc3a71b617c684ec35b24a714fbd550908f67ac2d61a841bbd2f79cc8c74bd1").unwrap();
    legal_wr.sig = hex::decode("438064a66afad40dea6332a97c91d7ae20c55ca328f3e56736628618ca25e515d84bf93aad0faea92374bd0adbd330e8d049cb9e6db972a0e00c9c7b291e522b").unwrap();

    legal_wr
}

pub fn legal_work_report_with_added_files() -> ReportWorksInfo {
    let curr_pk = hex::decode("7137dc62f9a8ba82fae62f5306981b7b39a82ff0e730739c9d8998eec0ab37f02e734e65fc518df5e6263d657faac48242ec1972b5dca058d9b78a6844c7a19c").unwrap();
    let prev_pk = hex::decode("").unwrap();
    let block_number: u64 = 300;
    let block_hash = hex::decode("0000000000000000000000000000000000000000000000000000000000000000").unwrap();
    let free: u64 = 4294967296;
    let used: u64 = 402868224;
    let added_files: Vec<(Vec<u8>, u64, u64)> = [
        ("QmdwgqZy1MZBfWPi7GcxVsYgJEtmvHg6rsLzbCej3tf3oF".as_bytes().to_vec(), 134289408, 303),
        ("QmdwgqZy1MZBfWPi7GcxVsYgJEtmvHg6rsLzbCej3tf3oH".as_bytes().to_vec(), 268578816, 303)
    ].to_vec();
    let deleted_files: Vec<(Vec<u8>, u64, u64)> = vec![];
    let files_root = hex::decode("11").unwrap();
    let srd_root = hex::decode("00").unwrap();
    let sig = hex::decode("d254eb42c15384b8019d676b1cf83c11a6cf0121c47381cabfea44844421cc231e244f83c2c4af3140880c534b672196b147e8b63708c871cc87f1230dbca12f").unwrap();

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

pub fn legal_work_report_with_deleted_files() -> ReportWorksInfo {
    let curr_pk = hex::decode("259638261947a43f175285d1bd05a5a5d52db05518e6c07af58e763fd85c3a6ee0adbd3079e46534201189eb33c7f4cfc70046c18d5e79b78808857da5223e8f").unwrap();
    let prev_pk = hex::decode("").unwrap();
    let block_number: u64 = 300;
    let block_hash = hex::decode("0000000000000000000000000000000000000000000000000000000000000000").unwrap();
    let free: u64 = 4294967296;
    let used: u64 = 0;
    let added_files: Vec<(Vec<u8>, u64, u64)> = [].to_vec();
    let deleted_files: Vec<(Vec<u8>, u64, u64)> = vec![
        ("QmdwgqZy1MZBfWPi7GcxVsYgJEtmvHg6rsLzbCej3tf3oI".as_bytes().to_vec(), 1, 303),
        ("QmdwgqZy1MZBfWPi7GcxVsYgJEtmvHg6rsLzbCej3tf3oJ".as_bytes().to_vec(), 1, 303),
        ("QmdwgqZy1MZBfWPi7GcxVsYgJEtmvHg6rsLzbCej3tf3oK".as_bytes().to_vec(), 1, 303)
    ];
    let files_root = hex::decode("11").unwrap();
    let srd_root = hex::decode("00").unwrap();
    let sig = hex::decode("018fa065d97d46e7576d727993e224dc3859a18220c9d542c7cf44ddfc3f289f83f9d13883db7e0a0d664bc953ffcab9558c2ae65d90466690ce9a3f735b0417").unwrap();

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

pub fn group_work_report_alice_300() -> ReportWorksInfo {
    let curr_pk = hex::decode("961ca9175d351fdb6cea3e4bb76761e6a130e4c4e347df7c4727287dc6c364719d8d427a32c37b8c60742d1a4223180cbc956e37e8c6288b7d66ae0869870524").unwrap();
    let prev_pk = hex::decode("").unwrap();
    let block_number: u64 = 300;
    let block_hash = hex::decode("0000000000000000000000000000000000000000000000000000000000000000").unwrap();
    let free: u64 = 4294967296;
    let used: u64 = 57;
    let added_files: Vec<(Vec<u8>, u64, u64)> = vec![
        ("QmdwgqZy1MZBfWPi7GcxVsYgJEtmvHg6rsLzbCej3tf3oA".as_bytes().to_vec(), 13, 303), // A file
        ("QmdwgqZy1MZBfWPi7GcxVsYgJEtmvHg6rsLzbCej3tf3oB".as_bytes().to_vec(), 7, 303),  // B file
        ("QmdwgqZy1MZBfWPi7GcxVsYgJEtmvHg6rsLzbCej3tf3oC".as_bytes().to_vec(), 37, 303)  // C file
    ];
    let deleted_files: Vec<(Vec<u8>, u64, u64)> = [].to_vec();
    let files_root = hex::decode("11").unwrap();
    let srd_root = hex::decode("00").unwrap();
    let sig = hex::decode("31ab80d7a5ed7e790e06442f490c06a8e17db5159e88bb300645399e4e0f17e12e91e6ee37b33e3a265918537972a7cab838d5e53ea6b8c2644cf7d00b86351e").unwrap();

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

pub fn group_work_report_alice_1500() -> ReportWorksInfo {
    let curr_pk = hex::decode("961ca9175d351fdb6cea3e4bb76761e6a130e4c4e347df7c4727287dc6c364719d8d427a32c37b8c60742d1a4223180cbc956e37e8c6288b7d66ae0869870524").unwrap();
    let prev_pk = hex::decode("").unwrap();
    let block_number: u64 = 1500;
    let block_hash = hex::decode("0000000000000000000000000000000000000000000000000000000000000000").unwrap();
    let free: u64 = 4294967296;
    let used: u64 = 0;
    let added_files: Vec<(Vec<u8>, u64, u64)> = [].to_vec();
    let deleted_files: Vec<(Vec<u8>, u64, u64)> = vec![
        ("QmdwgqZy1MZBfWPi7GcxVsYgJEtmvHg6rsLzbCej3tf3oA".as_bytes().to_vec(), 13, 1503), // A file
        ("QmdwgqZy1MZBfWPi7GcxVsYgJEtmvHg6rsLzbCej3tf3oB".as_bytes().to_vec(), 7, 1503),  // B file
        ("QmdwgqZy1MZBfWPi7GcxVsYgJEtmvHg6rsLzbCej3tf3oC".as_bytes().to_vec(), 37, 1503)  // C file
    ];
    let files_root = hex::decode("11").unwrap();
    let srd_root = hex::decode("00").unwrap();
    let sig = hex::decode("de2a1da96911bc3dd3f4363962a42d57c436ef7d3780c0f8de8a6e72095042e5eb56cc3f5b7069c5da24eed9730c77de902ff65a547d8d0ecbdbc1744e80758c").unwrap();

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

pub fn group_work_report_bob_300() -> ReportWorksInfo {
    let curr_pk = hex::decode("601cf4de0cbc88fc94b13d6c6549558813146071690ceab003c9971386a310d5702e94b506891879ae3945f4dd2027b229c456a2acf6299f167a3d222757af27").unwrap();
    let prev_pk = hex::decode("").unwrap();
    let block_number: u64 = 300;
    let block_hash = hex::decode("0000000000000000000000000000000000000000000000000000000000000000").unwrap();
    let free: u64 = 4294967296;
    let used: u64 = 99;
    let added_files: Vec<(Vec<u8>, u64, u64)> = vec![
        ("QmdwgqZy1MZBfWPi7GcxVsYgJEtmvHg6rsLzbCej3tf3oB".as_bytes().to_vec(), 7, 303),  // B file
        ("QmdwgqZy1MZBfWPi7GcxVsYgJEtmvHg6rsLzbCej3tf3oC".as_bytes().to_vec(), 37, 303), // C file
        ("QmdwgqZy1MZBfWPi7GcxVsYgJEtmvHg6rsLzbCej3tf3oD".as_bytes().to_vec(), 55, 303) // D file
    ];
    let deleted_files: Vec<(Vec<u8>, u64, u64)> = [].to_vec();
    let files_root = hex::decode("11").unwrap();
    let srd_root = hex::decode("00").unwrap();
    let sig = hex::decode("3b3471384879564114d0ec3d1613d96669d4ac67409052e1274f854f38bd8737ed4ed094469e32dca2f2faba18945fa06855447ab0e5717d48dc109545b8a8ba").unwrap();

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

pub fn group_work_report_bob_600() -> ReportWorksInfo {
    let curr_pk = hex::decode("601cf4de0cbc88fc94b13d6c6549558813146071690ceab003c9971386a310d5702e94b506891879ae3945f4dd2027b229c456a2acf6299f167a3d222757af27").unwrap();
    let prev_pk = hex::decode("").unwrap();
    let block_number: u64 = 600;
    let block_hash = hex::decode("0000000000000000000000000000000000000000000000000000000000000000").unwrap();
    let free: u64 = 4294967296;
    let used: u64 = 55;
    let added_files: Vec<(Vec<u8>, u64, u64)> = [].to_vec();
    let deleted_files: Vec<(Vec<u8>, u64, u64)> = vec![
        ("QmdwgqZy1MZBfWPi7GcxVsYgJEtmvHg6rsLzbCej3tf3oB".as_bytes().to_vec(), 7, 603),  // B file
        ("QmdwgqZy1MZBfWPi7GcxVsYgJEtmvHg6rsLzbCej3tf3oC".as_bytes().to_vec(), 37, 603)  // C file
    ];
    let files_root = hex::decode("11").unwrap();
    let srd_root = hex::decode("00").unwrap();
    let sig = hex::decode("6dd6696c1077cd9f162f71df0f4ad8e0f30307a1a597f009dfe64ec9fbff96d4cf491480ffa0ccd074c94933d3a974d2c15b1f12866e3b5b201036d3961179de").unwrap();

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

pub fn group_work_report_eve_300() -> ReportWorksInfo {
    let curr_pk = hex::decode("7bd3636438028483665c7be1f263264f45c7174014ded461dc041fe3b6f19f47b126ef87d95a679bca82533af0f22a979805512f4668262b0124756d69bfcd1c").unwrap();
    let prev_pk = hex::decode("").unwrap();
    let block_number: u64 = 300;
    let block_hash = hex::decode("0000000000000000000000000000000000000000000000000000000000000000").unwrap();
    let free: u64 = 4294967296;
    let used: u64 = 114;
    let added_files: Vec<(Vec<u8>, u64, u64)> = vec![
        ("QmdwgqZy1MZBfWPi7GcxVsYgJEtmvHg6rsLzbCej3tf3oC".as_bytes().to_vec(), 37, 303), // C file
        ("QmdwgqZy1MZBfWPi7GcxVsYgJEtmvHg6rsLzbCej3tf3oD".as_bytes().to_vec(), 55, 303), // D file
        ("QmdwgqZy1MZBfWPi7GcxVsYgJEtmvHg6rsLzbCej3tf3oE".as_bytes().to_vec(), 22, 303)  // E file
    ];
    let deleted_files: Vec<(Vec<u8>, u64, u64)> = [].to_vec();
    let files_root = hex::decode("11").unwrap();
    let srd_root = hex::decode("00").unwrap();
    let sig = hex::decode("4fd80889f5e2282a19d74e791d024155e1d511dc22db55f32c356723d323fe579f0ab937fa628a66fbf2b0d52c47391c82c659b594aba8fc28eaeb69a4412fa6").unwrap();

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

pub fn group_work_report_eve_600() -> ReportWorksInfo {
    let curr_pk = hex::decode("7bd3636438028483665c7be1f263264f45c7174014ded461dc041fe3b6f19f47b126ef87d95a679bca82533af0f22a979805512f4668262b0124756d69bfcd1c").unwrap();
    let prev_pk = hex::decode("").unwrap();
    let block_number: u64 = 600;
    let block_hash = hex::decode("0000000000000000000000000000000000000000000000000000000000000000").unwrap();
    let free: u64 = 4294967296;
    let used: u64 = 0;
    let added_files: Vec<(Vec<u8>, u64, u64)> = [].to_vec();
    let deleted_files: Vec<(Vec<u8>, u64, u64)> = vec![
        ("QmdwgqZy1MZBfWPi7GcxVsYgJEtmvHg6rsLzbCej3tf3oC".as_bytes().to_vec(), 37, 303), // C file
        ("QmdwgqZy1MZBfWPi7GcxVsYgJEtmvHg6rsLzbCej3tf3oD".as_bytes().to_vec(), 55, 303), // D file
        ("QmdwgqZy1MZBfWPi7GcxVsYgJEtmvHg6rsLzbCej3tf3oE".as_bytes().to_vec(), 22, 303)  // E file
    ];
    let files_root = hex::decode("11").unwrap();
    let srd_root = hex::decode("00").unwrap();
    let sig = hex::decode("125500b0886d88235825dcb3f6e61909bc48ed9057535428f4daa42a0fc7ec4f9a8304c66a7580efa7ae414ea25a798fb8af104011da2e595c47ab3668cc0f9e").unwrap();

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

pub fn register(pk: &SworkerPubKey, code: SworkerCode) {
    Swork::insert_pk_info(pk.clone(), code);
}

pub fn register_identity(who: &AccountId, pk: &SworkerPubKey, anchor: &SworkerAnchor) {
    <self::PubKeys>::mutate(pk, |pk_info| {
        pk_info.anchor = Some(anchor.clone());
    });
    <self::Identities<Test>>::insert(who, Identity {
        anchor: anchor.clone(),
        punishment_deadline: 0,
        group: None
    });
}

pub fn add_wr(anchor: &SworkerAnchor, wr: &WorkReport) {
    <self::WorkReports>::insert(anchor.clone(), wr.clone());
    <self::ReportedInSlot>::insert(anchor.clone(), wr.report_slot, true);
}

pub fn add_not_live_files() {
    let files: Vec<(Vec<u8>, u64)> = [
        ("QmdwgqZy1MZBfWPi7GcxVsYgJEtmvHg6rsLzbCej3tf3oF".as_bytes().to_vec(), 134289408),
        ("QmdwgqZy1MZBfWPi7GcxVsYgJEtmvHg6rsLzbCej3tf3oH".as_bytes().to_vec(), 268578816),
        ("QmdwgqZy1MZBfWPi7GcxVsYgJEtmvHg6rsLzbCej3tf3oA".as_bytes().to_vec(), 13), // A file
        ("QmdwgqZy1MZBfWPi7GcxVsYgJEtmvHg6rsLzbCej3tf3oB".as_bytes().to_vec(), 7),  // B file
        ("QmdwgqZy1MZBfWPi7GcxVsYgJEtmvHg6rsLzbCej3tf3oC".as_bytes().to_vec(), 37),  // C file
        ("QmdwgqZy1MZBfWPi7GcxVsYgJEtmvHg6rsLzbCej3tf3oD".as_bytes().to_vec(), 55), // D file
        ("QmdwgqZy1MZBfWPi7GcxVsYgJEtmvHg6rsLzbCej3tf3oE".as_bytes().to_vec(), 22)  // E file
    ].to_vec();

    for (file, file_size) in files.iter() {
        let used_info = UsedInfo {
            used_size: 0,
            reported_group_count: 0,
            groups: <BTreeMap<SworkerAnchor, bool>>::new()
        };
        insert_file(file, 1000, 0, 1000, 0, 0, vec![], *file_size, used_info);
    }

    let storage_pot = Market::storage_pot();
    let _ = Balances::make_free_balance_be(&storage_pot, 20000);
}

pub fn add_live_files(who: &AccountId, anchor: &SworkerAnchor) {
    let files: Vec<(Vec<u8>, u64)> = [
        ("QmdwgqZy1MZBfWPi7GcxVsYgJEtmvHg6rsLzbCej3tf3oF".as_bytes().to_vec(), 134289408),
        ("QmdwgqZy1MZBfWPi7GcxVsYgJEtmvHg6rsLzbCej3tf3oB".as_bytes().to_vec(), 7),
        ("QmdwgqZy1MZBfWPi7GcxVsYgJEtmvHg6rsLzbCej3tf3oC".as_bytes().to_vec(), 37),
        ("QmdwgqZy1MZBfWPi7GcxVsYgJEtmvHg6rsLzbCej3tf3oI".as_bytes().to_vec(), 1),
        ("QmdwgqZy1MZBfWPi7GcxVsYgJEtmvHg6rsLzbCej3tf3oJ".as_bytes().to_vec(), 1),
        ("QmdwgqZy1MZBfWPi7GcxVsYgJEtmvHg6rsLzbCej3tf3oK".as_bytes().to_vec(), 1)
    ].to_vec();

    let replica_info = Replica {
        who: who.clone(),
        valid_at: 200,
        anchor: anchor.clone(),
        is_reported: true
    };
    for (file, file_size) in files.iter() {
        let used_info = UsedInfo {
            used_size: *file_size * 2,
            reported_group_count: 1,
            groups: BTreeMap::from_iter(vec![(anchor.clone(), true)].into_iter())
        };
        insert_file(file, 200, 12000, 1000, 0, 0, vec![replica_info.clone()], *file_size, used_info);
    }
}

fn insert_file(f_id: &MerkleRoot, calculated_at: u32, expired_on: u32, amount: Balance, prepaid: Balance,  reported_replica_count: u32, replicas: Vec<Replica<AccountId>>, file_size: u64, used_info: UsedInfo) {
    let file_info = FileInfo {
        file_size,
        expired_on,
        calculated_at,
        amount,
        prepaid,
        reported_replica_count,
        replicas
    };

    <market::Files<Test>>::insert(f_id, (file_info, used_info));
}

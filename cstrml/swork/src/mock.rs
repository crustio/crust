// Copyright (C) 2019-2021 Crust Network Technologies Ltd.
// This file is part of Crust.

use crate::*;

pub use frame_support::{
    impl_outer_origin, parameter_types,
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

impl_outer_origin! {
    pub enum Origin for Test where system = system {}
}

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

// For testing the module, we construct most of a mock runtime. This means
// first constructing a configuration type (`Test`) which `impl`s each of the
// configuration traits of modules we want to use.
#[derive(Clone, Eq, PartialEq, Debug)]
pub struct Test;

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
    type Call = ();
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
    type PalletInfo = ();
    type AccountData = AccountData<u64>;
    type OnNewAccount = ();
    type OnKilledAccount = ();
    type SystemWeightInfo = ();
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
    pub const InitialReplica: u32 = 4;
    pub const FileBaseFee: Balance = 1000;
    pub const FileInitPrice: Balance = 1000; // Need align with FileDuration and FileBaseReplica
    pub const ClaimLimit: u32 = 1000;
    pub const StorageReferenceRatio: (u128, u128) = (1, 2);
    pub const StorageIncreaseRatio: Perbill = Perbill::from_percent(1);
    pub const StorageDecreaseRatio: Perbill = Perbill::from_percent(1);
    pub const StakingRatio: Perbill = Perbill::from_percent(80);
    pub const TaxRatio: Perbill = Perbill::from_percent(10);
    pub const UsedTrashMaxSize: u128 = 2;
}

impl market::Config for Test {
    type ModuleId = MarketModuleId;
    type Currency = balances::Module<Self>;
    type CurrencyToBalance = ();
    type SworkerInterface = Swork;
    type Event = ();
    /// File duration.
    type FileDuration = FileDuration;
    type InitialReplica = InitialReplica;
    type FileBaseFee = FileBaseFee;
    type FileInitPrice = FileInitPrice;
    type ClaimLimit = ClaimLimit;
    type StorageReferenceRatio = StorageReferenceRatio;
    type StorageIncreaseRatio = StorageIncreaseRatio;
    type StorageDecreaseRatio = StorageDecreaseRatio;
    type StakingRatio = StakingRatio;
    type TaxRatio = TaxRatio;
    type UsedTrashMaxSize = UsedTrashMaxSize;
    type WeightInfo = market::weight::WeightInfo<Test>;
}

pub struct TestWorksInterface;

impl Works<AccountId> for TestWorksInterface {
    fn report_works(who: &AccountId, own_workload: u128, _: u128) {
        WorkloadMap::set(who, own_workload);
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

pub type Swork = Module<Test>;
pub type System = system::Module<Test>;
pub type Market = market::Module<Test>;
pub type Balances = balances::Module<Test>;

pub struct ExtBuilder {
    code: SworkerCode
}

impl Default for ExtBuilder {
    fn default() -> Self {
        Self {
            code: LegalCode::get()
        }
    }
}

impl ExtBuilder {
    pub fn code(mut self, code: SworkerCode) -> Self {
        self.code = code;
        self
    }

    pub fn build(self) -> sp_io::TestExternalities {
        let mut t = system::GenesisConfig::default()
            .build_storage::<Test>()
            .unwrap();

        GenesisConfig {
            code: self.code,
        }.assimilate_storage(&mut t).unwrap();

        t.into()
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

pub fn legal_work_report() -> ReportWorksInfo {
    let curr_pk = hex::decode("8c04b7ea70bdae811cb246c846bcce9b76d5fcf142359c41f76477eca5f30088e51e66b963b5f4305f39645e305fee1e50f3693bfbccc757c4253f76a362d296").unwrap();
    let prev_pk = hex::decode("").unwrap();
    let block_number: u64 = 300;
    let block_hash = hex::decode("0000000000000000000000000000000000000000000000000000000000000000").unwrap();
    let free: u64 = 4294967296;
    let used: u64 = 2;
    let added_files: Vec<(Vec<u8>, u64, u64)> = vec![];
    let deleted_files: Vec<(Vec<u8>, u64, u64)> = vec![];
    let files_root = hex::decode("11").unwrap();
    let srd_root = hex::decode("00").unwrap();
    let sig = hex::decode("2b17ff9033173fdebc5f281ba9f7f165e9344560f124eb2ccebd1cc7ea44e295a558f9ad51f3494a40c906f471dab47cc11d3e373250ff861194d123e127dc10").unwrap();

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
        (hex::decode("5aa706320afc633bfb843108e492192b17d2b6b9d9ee0b795ee95417fe08b660").unwrap(), 1, 503),
    ];
    legal_wr.deleted_files = vec![
        (hex::decode("5aa706320afc633bfb843108e492192b17d2b6b9d9ee0b795ee95417fe08b660").unwrap(), 1, 503),
    ];
    legal_wr.files_root = hex::decode("5aa706320afc633bfb843108e492192b17d2b6b9d9ee0b795ee95417fe08b660").unwrap();
    legal_wr.sig = hex::decode("1ba214b90d6814f9c7783b0da9581d2bd081df72d5ebc30719abe7b6c2f2640e642d83dbb25548b71c344dc82e473c8b5441f44ddc457195b350effd162ef499").unwrap();
    legal_wr
}

pub fn continuous_ab_upgrade_work_report() -> ReportWorksInfo {
    let mut legal_wr = ab_upgrade_work_report();
    legal_wr.block_number = 900;
    legal_wr.added_files = vec![(hex::decode("5bb706320afc633bfb843108e492192b17d2b6b9d9ee0b795ee95417fe08b660").unwrap(), 2, 903)];
    legal_wr.deleted_files = vec![(hex::decode("99cdb315c8c37e2dc00fa2a8c7fe51b8149b363d29f404441982f96d2bbae65f").unwrap(), 1, 903)];
    legal_wr.used = 3;
    legal_wr.sig = hex::decode("c431c0f4fa032ab080e3d2a8d1ee4fbd355c0cf18a096b7ee0945af430fb0970f7668411e150674e3c66b96af22dc17bb0d6e6e4f58ebad218e408d8311dcb9e").unwrap();
    legal_wr
}

pub fn ab_upgrade_work_report() -> ReportWorksInfo {
    let mut legal_wr = legal_work_report();
    legal_wr.block_number = 600;
    legal_wr.curr_pk = hex::decode("4aeb5997d0adcd9397a30d123b4f3a55f76c864191eb760058cff78d2ab0b5e865defca96b8d39803c5cf88d73315b7365b85607015cf6bcf7690c863d6e106a").unwrap();
    legal_wr.prev_pk = hex::decode("8c04b7ea70bdae811cb246c846bcce9b76d5fcf142359c41f76477eca5f30088e51e66b963b5f4305f39645e305fee1e50f3693bfbccc757c4253f76a362d296").unwrap();
    legal_wr.sig = hex::decode("ca2d8c7689ffa5645bb6daa5cd45abb9532945627cc1f14bdacfc7e908d38110292718574a1861502e78e0c8ee320671bc0eb38ce612af3c5914fe8fde957463").unwrap();
    legal_wr
}

pub fn ab_upgrade_work_report_files_size_unmatch() -> ReportWorksInfo {
    let mut legal_wr = ab_upgrade_work_report();
    legal_wr.added_files = vec![(hex::decode("6aa706320afc633bfb843108e492192b17d2b6b9d9ee0b795ee95417fe08b660").unwrap(), 10, 606)];
    legal_wr.sig = hex::decode("7de7dec131d3b78e7a88820b214c4da01be255d4f73db3c060e5f46bfb4293bd1f0a81466f9cdc03ce833782b585f8ed436f2056fdfe38cf2a0c590cec6cb3cd").unwrap();
    legal_wr
}

pub fn continuous_work_report_300() -> ReportWorksInfo {
    let mut legal_wr = legal_work_report();
    legal_wr.block_number = 300;
    legal_wr.curr_pk = hex::decode("4aeb5997d0adcd9397a30d123b4f3a55f76c864191eb760058cff78d2ab0b5e865defca96b8d39803c5cf88d73315b7365b85607015cf6bcf7690c863d6e106a").unwrap();
    legal_wr.sig = hex::decode("8d96e41efe97ab7c0006a702769572a5fb6ae008786b08a88df54779ec73b71d86fb49a191996232daaea2c455be01f8fbfad2c452b4218626781290eb3b686f").unwrap();

    legal_wr
}

pub fn continuous_work_report_600() -> ReportWorksInfo {
    let mut legal_wr = legal_work_report();
    legal_wr.block_number = 600;
    legal_wr.curr_pk = hex::decode("4aeb5997d0adcd9397a30d123b4f3a55f76c864191eb760058cff78d2ab0b5e865defca96b8d39803c5cf88d73315b7365b85607015cf6bcf7690c863d6e106a").unwrap();
    legal_wr.sig = hex::decode("30f62121d380858855eba279a58632bd3d9092f8b70dd31c0baef6f2a60b9ea15db6914e7698132c94333524aafe61e74f5a07f20514de86dc6d80bbb9cc898f").unwrap();

    legal_wr
}

pub fn legal_work_report_with_added_files() -> ReportWorksInfo {
    let curr_pk = hex::decode("8b1412c4eed29d29389f8a66aa61f0c0fdea30c7e384ca8086d72cf84c4b96dd7967f5841ba8b784c4de881fe073b1437051db1ee9ab0fe491df0df3792bce5d").unwrap();
    let prev_pk = hex::decode("").unwrap();
    let block_number: u64 = 300;
    let block_hash = hex::decode("0000000000000000000000000000000000000000000000000000000000000000").unwrap();
    let free: u64 = 4294967296;
    let used: u64 = 402868224;
    let added_files: Vec<(Vec<u8>, u64, u64)> = [
        (hex::decode("5bb706320afc633bfb843108e492192b17d2b6b9d9ee0b795ee95417fe08b660").unwrap(), 134289408, 303),
        (hex::decode("88cdb315c8c37e2dc00fa2a8c7fe51b8149b363d29f404441982f96d2bbae65f").unwrap(), 268578816, 303)
    ].to_vec();
    let deleted_files: Vec<(Vec<u8>, u64, u64)> = vec![];
    let files_root = hex::decode("11").unwrap();
    let srd_root = hex::decode("00").unwrap();
    let sig = hex::decode("9b775ad3a8469b7affacd0252bd5fdaa69a2a22dffec9c428faa5c12fce6886accc19446d1c833d9cd46e4f78d31d2544b91a644702308f9c4211448484ef3a9").unwrap();

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
    let curr_pk = hex::decode("f2553682f7b2cab9fa190a6389c3b8b4f415a799209be54bf1e11b6033693adb6a1c2437a24aaa8472b2047b299cd971c51e2a05652f6648b10372751ecea761").unwrap();
    let prev_pk = hex::decode("").unwrap();
    let block_number: u64 = 300;
    let block_hash = hex::decode("0000000000000000000000000000000000000000000000000000000000000000").unwrap();
    let free: u64 = 4294967296;
    let used: u64 = 0;
    let added_files: Vec<(Vec<u8>, u64, u64)> = [].to_vec();
    let deleted_files: Vec<(Vec<u8>, u64, u64)> = vec![
        (hex::decode("5aa706320afc633bfb843108e492192b17d2b6b9d9ee0b795ee95417fe08b660").unwrap(), 1, 303),
        (hex::decode("99cdb315c8c37e2dc00fa2a8c7fe51b8149b363d29f404441982f96d2bbae65f").unwrap(), 1, 303),
        (hex::decode("77cdb315c8c37e2dc00fa2a8c7fe51b8149b363d29f404441982f96d2bbae65f").unwrap(), 1, 303)
    ];
    let files_root = hex::decode("11").unwrap();
    let srd_root = hex::decode("00").unwrap();
    let sig = hex::decode("227fe53e6e5c9d414e7d062a06232928ecc384bced9bd94746bc625c7e2a13429f323c418d15743600e232cbd3878f29b9eb8d5e6224e3e2cc95a88a93d0c705").unwrap();

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
    let curr_pk = hex::decode("2b60e057cc5a2177dee185b260a475ee5573a22275583c8c16661fe9781f7101d7a5455ec5f3b0dc8ea5183858337348151b13d908e45fff96d96629ae99f917").unwrap();
    let prev_pk = hex::decode("").unwrap();
    let block_number: u64 = 300;
    let block_hash = hex::decode("0000000000000000000000000000000000000000000000000000000000000000").unwrap();
    let free: u64 = 4294967296;
    let used: u64 = 57;
    let added_files: Vec<(Vec<u8>, u64, u64)> = vec![
        (hex::decode("5aa706320afc633bfb843108e492192b17d2b6b9d9ee0b795ee95417fe08b660").unwrap(), 13, 303), // A file
        (hex::decode("99cdb315c8c37e2dc00fa2a8c7fe51b8149b363d29f404441982f96d2bbae65f").unwrap(), 7, 303),  // B file
        (hex::decode("77cdb315c8c37e2dc00fa2a8c7fe51b8149b363d29f404441982f96d2bbae65f").unwrap(), 37, 303)  // C file
    ];
    let deleted_files: Vec<(Vec<u8>, u64, u64)> = [].to_vec();
    let files_root = hex::decode("11").unwrap();
    let srd_root = hex::decode("00").unwrap();
    let sig = hex::decode("77e4aefd3996b16d4016a78f9f8583f1ff10d28c198254d846737a4390573b3fc88ae425d9abf3f86abd9a8f42376554eab9b392e6631b154f8b7a2fad404807").unwrap();

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
    let curr_pk = hex::decode("2b60e057cc5a2177dee185b260a475ee5573a22275583c8c16661fe9781f7101d7a5455ec5f3b0dc8ea5183858337348151b13d908e45fff96d96629ae99f917").unwrap();
    let prev_pk = hex::decode("").unwrap();
    let block_number: u64 = 1500;
    let block_hash = hex::decode("0000000000000000000000000000000000000000000000000000000000000000").unwrap();
    let free: u64 = 4294967296;
    let used: u64 = 0;
    let added_files: Vec<(Vec<u8>, u64, u64)> = [].to_vec();
    let deleted_files: Vec<(Vec<u8>, u64, u64)> = vec![
        (hex::decode("5aa706320afc633bfb843108e492192b17d2b6b9d9ee0b795ee95417fe08b660").unwrap(), 13, 1503), // A file
        (hex::decode("99cdb315c8c37e2dc00fa2a8c7fe51b8149b363d29f404441982f96d2bbae65f").unwrap(), 7, 1503),  // B file
        (hex::decode("77cdb315c8c37e2dc00fa2a8c7fe51b8149b363d29f404441982f96d2bbae65f").unwrap(), 37, 1503)  // C file
    ];
    let files_root = hex::decode("11").unwrap();
    let srd_root = hex::decode("00").unwrap();
    let sig = hex::decode("0d56da4b47066e76b83125c155efadf73faa79c07158d956aa137ae5235b9877caa9c40c13ebf073fd8af7c3c16c55476b864e17a00a7068e4bb7d12187d9114").unwrap();

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
    let curr_pk = hex::decode("9585e9b85d4da275029b62757d1a7e7e4129b63e49f4c01e75d0f4e3940b4fa8d04c85179ac20a458dbf0a5e849a523cd3a1af6b7eb834c0d8468014a4eb483a").unwrap();
    let prev_pk = hex::decode("").unwrap();
    let block_number: u64 = 300;
    let block_hash = hex::decode("0000000000000000000000000000000000000000000000000000000000000000").unwrap();
    let free: u64 = 4294967296;
    let used: u64 = 99;
    let added_files: Vec<(Vec<u8>, u64, u64)> = vec![
        (hex::decode("99cdb315c8c37e2dc00fa2a8c7fe51b8149b363d29f404441982f96d2bbae65f").unwrap(), 7, 303),  // B file
        (hex::decode("77cdb315c8c37e2dc00fa2a8c7fe51b8149b363d29f404441982f96d2bbae65f").unwrap(), 37, 303), // C file
        (hex::decode("66a706320afc633bfb843108e492192b17d2b6b9d9ee0b795ee95417fe08b110").unwrap(), 55, 303) // D file
    ];
    let deleted_files: Vec<(Vec<u8>, u64, u64)> = [].to_vec();
    let files_root = hex::decode("11").unwrap();
    let srd_root = hex::decode("00").unwrap();
    let sig = hex::decode("31cbede1b8959257833a594bd426ab6d0831d446cc4b091c42c10d7b7a2d6e12e5ab2f8125fc7cbf65548d7d6a8e0bb96f78331b6cf6b225f864668bf12f78a1").unwrap();

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
    let curr_pk = hex::decode("9585e9b85d4da275029b62757d1a7e7e4129b63e49f4c01e75d0f4e3940b4fa8d04c85179ac20a458dbf0a5e849a523cd3a1af6b7eb834c0d8468014a4eb483a").unwrap();
    let prev_pk = hex::decode("").unwrap();
    let block_number: u64 = 600;
    let block_hash = hex::decode("0000000000000000000000000000000000000000000000000000000000000000").unwrap();
    let free: u64 = 4294967296;
    let used: u64 = 55;
    let added_files: Vec<(Vec<u8>, u64, u64)> = [].to_vec();
    let deleted_files: Vec<(Vec<u8>, u64, u64)> = vec![
        (hex::decode("99cdb315c8c37e2dc00fa2a8c7fe51b8149b363d29f404441982f96d2bbae65f").unwrap(), 7, 603),  // B file
        (hex::decode("77cdb315c8c37e2dc00fa2a8c7fe51b8149b363d29f404441982f96d2bbae65f").unwrap(), 37, 603)  // C file
    ];
    let files_root = hex::decode("11").unwrap();
    let srd_root = hex::decode("00").unwrap();
    let sig = hex::decode("f39b39de264977e0325d17fb015890e3b5c17e8327f899d3e43577e083603b1f5723a35209905053db0f36b650b0896d1ab26d1be22a5ba023d443f1f205ac14").unwrap();

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
    let curr_pk = hex::decode("09a4fcc750b10131abba450907abab25c79802e5ae8a6f0e88bebff899bc4dc505cb984066e9e1c115411f7e7a5095cf1eef6e084c6081ad611a801c26df8c4d").unwrap();
    let prev_pk = hex::decode("").unwrap();
    let block_number: u64 = 300;
    let block_hash = hex::decode("0000000000000000000000000000000000000000000000000000000000000000").unwrap();
    let free: u64 = 4294967296;
    let used: u64 = 114;
    let added_files: Vec<(Vec<u8>, u64, u64)> = vec![
        (hex::decode("77cdb315c8c37e2dc00fa2a8c7fe51b8149b363d29f404441982f96d2bbae65f").unwrap(), 37, 303), // C file
        (hex::decode("66a706320afc633bfb843108e492192b17d2b6b9d9ee0b795ee95417fe08b110").unwrap(), 55, 303), // D file
        (hex::decode("33cdb315c8c37e2dc00fa2a8c7fe51b8149b363d29f404441982f96d2bbae12e").unwrap(), 22, 303)  // E file
    ];
    let deleted_files: Vec<(Vec<u8>, u64, u64)> = [].to_vec();
    let files_root = hex::decode("11").unwrap();
    let srd_root = hex::decode("00").unwrap();
    let sig = hex::decode("8730b5928c4c0c2e74e723266b0afb2923a689c53570943915960442d89ef5ef663f8993bb9792c2a44eb50836380cb0ddbf3d3c24e85eabcc860dcd6f65d75d").unwrap();

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
    let curr_pk = hex::decode("09a4fcc750b10131abba450907abab25c79802e5ae8a6f0e88bebff899bc4dc505cb984066e9e1c115411f7e7a5095cf1eef6e084c6081ad611a801c26df8c4d").unwrap();
    let prev_pk = hex::decode("").unwrap();
    let block_number: u64 = 600;
    let block_hash = hex::decode("0000000000000000000000000000000000000000000000000000000000000000").unwrap();
    let free: u64 = 4294967296;
    let used: u64 = 0;
    let added_files: Vec<(Vec<u8>, u64, u64)> = [].to_vec();
    let deleted_files: Vec<(Vec<u8>, u64, u64)> = vec![
        (hex::decode("77cdb315c8c37e2dc00fa2a8c7fe51b8149b363d29f404441982f96d2bbae65f").unwrap(), 37, 603), // C file
        (hex::decode("66a706320afc633bfb843108e492192b17d2b6b9d9ee0b795ee95417fe08b110").unwrap(), 55, 603), // D file
        (hex::decode("33cdb315c8c37e2dc00fa2a8c7fe51b8149b363d29f404441982f96d2bbae12e").unwrap(), 22, 603)  // E file
    ];
    let files_root = hex::decode("11").unwrap();
    let srd_root = hex::decode("00").unwrap();
    let sig = hex::decode("9270a2c303bd822495cc26ddd10ffc6df85e0c9e741a0504f4452e07b8a69455e5df71fbfd5cf278bb9994c67a3010bc619c92e9020c70b7c882c03b40b06e16").unwrap();

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
        (hex::decode("5bb706320afc633bfb843108e492192b17d2b6b9d9ee0b795ee95417fe08b660").unwrap(), 134289408),
        (hex::decode("5bb706320afc633bfb843108e492192b17d2b6b9d9ee0b795ee95417fe08b660").unwrap(), 134289408),
        (hex::decode("88cdb315c8c37e2dc00fa2a8c7fe51b8149b363d29f404441982f96d2bbae65f").unwrap(), 268578816),
        (hex::decode("5aa706320afc633bfb843108e492192b17d2b6b9d9ee0b795ee95417fe08b660").unwrap(), 13), // A file
        (hex::decode("99cdb315c8c37e2dc00fa2a8c7fe51b8149b363d29f404441982f96d2bbae65f").unwrap(), 7),  // B file
        (hex::decode("77cdb315c8c37e2dc00fa2a8c7fe51b8149b363d29f404441982f96d2bbae65f").unwrap(), 37),  // C file
        (hex::decode("66a706320afc633bfb843108e492192b17d2b6b9d9ee0b795ee95417fe08b110").unwrap(), 55), // D file
        (hex::decode("33cdb315c8c37e2dc00fa2a8c7fe51b8149b363d29f404441982f96d2bbae12e").unwrap(), 22)  // E file
    ].to_vec();

    for (file, file_size) in files.iter() {
        let used_info = UsedInfo {
            used_size: 0,
            reported_group_count: 0,
            groups: <BTreeMap<SworkerAnchor, bool>>::new()
        };
        insert_file(file, 1000, 0, 1000, 4, 0, vec![], *file_size, used_info);
    }

    let storage_pot = Market::storage_pot();
    let _ = Balances::make_free_balance_be(&storage_pot, 20000);
}

pub fn add_live_files(who: &AccountId, anchor: &SworkerAnchor) {
    let files: Vec<(Vec<u8>, u64)> = [
        (hex::decode("5aa706320afc633bfb843108e492192b17d2b6b9d9ee0b795ee95417fe08b660").unwrap(), 134289408),
        (hex::decode("99cdb315c8c37e2dc00fa2a8c7fe51b8149b363d29f404441982f96d2bbae65f").unwrap(), 7)
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
        insert_file(file, 200, 12000, 1000, 4, 0, vec![replica_info.clone()], *file_size, used_info);
    }
}

fn insert_file(f_id: &MerkleRoot, claimed_at: u32, expired_on: u32, amount: Balance, expected_replica_count: u32, reported_replica_count: u32, replicas: Vec<Replica<AccountId>>, file_size: u64, used_info: UsedInfo) {
    let file_info = FileInfo {
        file_size,
        expired_on,
        claimed_at,
        amount,
        expected_replica_count,
        reported_replica_count,
        replicas
    };

    <market::Files<Test>>::insert(f_id, (file_info, used_info));
}

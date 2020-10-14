use crate::*;

use frame_support::{
    impl_outer_origin, parameter_types,
    weights::{Weight, constants::RocksDbWeight},
    traits::{OnInitialize, OnFinalize, Get, TestRandomness}
};
pub use sp_core::{crypto::{AccountId32, Ss58Codec}, H256};
use sp_runtime::{
    testing::Header,
    traits::{BlakeTwo256, IdentityLookup},
    Perbill,
};
use market::{MerchantInfo, SorderStatus, SorderInfo, SorderPunishment};
use primitives::{MerkleRoot, Hash};
use balances::AccountData;
use std::{cell::RefCell};

pub type AccountId = AccountId32;
pub type Balance = u64;

impl_outer_origin! {
    pub enum Origin for Test where system = system {}
}

thread_local! {
    static EXISTENTIAL_DEPOSIT: RefCell<u64> = RefCell::new(0);
    static LEGAL_PK: Vec<u8> = hex::decode("4dbb6401508323b18f649f04f17433fd4b87201ef3ff634b684b715c848bb60b905dd5305e24761b4968a8875dfd9f6abfb3110d9fa494dd530daaeccc8353e1").unwrap();
    static LEGAL_CODE: Vec<u8> = hex::decode("e256ab4cb5e9136bc1c1115088fc40ca1f4182545ea75769578c20d843028cd5").unwrap();
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
    pub added_files: Vec<(MerkleRoot, u64)>,
    pub deleted_files: Vec<(MerkleRoot, u64)>,
    pub sig: SworkerSignature
}

// For testing the module, we construct most of a mock runtime. This means
// first constructing a configuration type (`Test`) which `impl`s each of the
// configuration traits of modules we want to use.
#[derive(Clone, Eq, PartialEq, Debug)]
pub struct Test;

parameter_types! {
    pub const BlockHashCount: u64 = 250;
    pub const MaximumBlockWeight: Weight = 1024;
    pub const MaximumBlockLength: u32 = 2 * 1024;
    pub const AvailableBlockRatio: Perbill = Perbill::from_percent(75);
}

impl system::Trait for Test {
    type BaseCallFilter = ();
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
    type MaximumBlockWeight = MaximumBlockWeight;
    type DbWeight = RocksDbWeight;
    type BlockExecutionWeight = ();
    type ExtrinsicBaseWeight = ();
    type MaximumExtrinsicWeight = MaximumBlockWeight;
    type MaximumBlockLength = MaximumBlockLength;
    type AvailableBlockRatio = AvailableBlockRatio;
    type Version = ();
    type PalletInfo = ();
    type AccountData = AccountData<u64>;
    type OnNewAccount = ();
    type OnKilledAccount = ();
    type SystemWeightInfo = ();
}

impl balances::Trait for Test {
    type Balance = Balance;
    type DustRemoval = ();
    type Event = ();
    type ExistentialDeposit = ExistentialDeposit;
    type AccountStore = system::Module<Test>;
    type WeightInfo = ();
    type MaxLocks = ();
}

parameter_types! {
    pub const ClaimLimit: u32 = 100;
    pub const MaxBondsLimit: u32 = 2;
}

impl market::Trait for Test {
    type Currency = balances::Module<Self>;
    type CurrencyToBalance = ();
    type Event = ();
    type Randomness = TestRandomness;
    type OrderInspector = Swork;
    type MinimumStoragePrice = ();
    type MinimumSorderDuration = ();
    type ClaimLimit = ClaimLimit;
}

impl Trait for Test {
    type Currency = balances::Module<Self>;
    type Event = ();
    type Works = ();
    type MarketInterface = Market;
    type MaxBondsLimit = MaxBondsLimit;
}

pub type Swork = Module<Test>;
pub type System = system::Module<Test>;
pub type Market = market::Module<Test>;

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

    let ias_sig = "Lb17i6Gb2LUoMTYz/fRIjZrsF9X8vxv8S5IZtWjJ2i/BklZO8xeWuS9ItM/8JgDI2qv+zZwZtdgoywK2drH8sV/d0GN/bu5RR4u+bTOJnDWRFkU6lZC9N6AT4ntdFrrkCIfPgikd3dQr21e8v9ShfUy6FT44oLCx21p5knbO1ygxFXzm73nvpLqTB7avRqT3JtHEdzvHjPBymDq18dX7a2cRbK2EwvO48cTcTXihwLZxKjdw7Kds9RC79IaSOVSoBhqBjGtccn9xitj2kPJp65KLU5KpsguTiDwrF79UMsbWI0eKv4voXodNL6YEZdFYELGsp9SpwR6sd4t0628fHg==".as_bytes();
    let ias_cert = "-----BEGIN CERTIFICATE-----\nMIIEoTCCAwmgAwIBAgIJANEHdl0yo7CWMA0GCSqGSIb3DQEBCwUAMH4xCzAJBgNV\nBAYTAlVTMQswCQYDVQQIDAJDQTEUMBIGA1UEBwwLU2FudGEgQ2xhcmExGjAYBgNV\nBAoMEUludGVsIENvcnBvcmF0aW9uMTAwLgYDVQQDDCdJbnRlbCBTR1ggQXR0ZXN0\nYXRpb24gUmVwb3J0IFNpZ25pbmcgQ0EwHhcNMTYxMTIyMDkzNjU4WhcNMjYxMTIw\nMDkzNjU4WjB7MQswCQYDVQQGEwJVUzELMAkGA1UECAwCQ0ExFDASBgNVBAcMC1Nh\nbnRhIENsYXJhMRowGAYDVQQKDBFJbnRlbCBDb3Jwb3JhdGlvbjEtMCsGA1UEAwwk\nSW50ZWwgU0dYIEF0dGVzdGF0aW9uIFJlcG9ydCBTaWduaW5nMIIBIjANBgkqhkiG\n9w0BAQEFAAOCAQ8AMIIBCgKCAQEAqXot4OZuphR8nudFrAFiaGxxkgma/Es/BA+t\nbeCTUR106AL1ENcWA4FX3K+E9BBL0/7X5rj5nIgX/R/1ubhkKWw9gfqPG3KeAtId\ncv/uTO1yXv50vqaPvE1CRChvzdS/ZEBqQ5oVvLTPZ3VEicQjlytKgN9cLnxbwtuv\nLUK7eyRPfJW/ksddOzP8VBBniolYnRCD2jrMRZ8nBM2ZWYwnXnwYeOAHV+W9tOhA\nImwRwKF/95yAsVwd21ryHMJBcGH70qLagZ7Ttyt++qO/6+KAXJuKwZqjRlEtSEz8\ngZQeFfVYgcwSfo96oSMAzVr7V0L6HSDLRnpb6xxmbPdqNol4tQIDAQABo4GkMIGh\nMB8GA1UdIwQYMBaAFHhDe3amfrzQr35CN+s1fDuHAVE8MA4GA1UdDwEB/wQEAwIG\nwDAMBgNVHRMBAf8EAjAAMGAGA1UdHwRZMFcwVaBToFGGT2h0dHA6Ly90cnVzdGVk\nc2VydmljZXMuaW50ZWwuY29tL2NvbnRlbnQvQ1JML1NHWC9BdHRlc3RhdGlvblJl\ncG9ydFNpZ25pbmdDQS5jcmwwDQYJKoZIhvcNAQELBQADggGBAGcIthtcK9IVRz4r\nRq+ZKE+7k50/OxUsmW8aavOzKb0iCx07YQ9rzi5nU73tME2yGRLzhSViFs/LpFa9\nlpQL6JL1aQwmDR74TxYGBAIi5f4I5TJoCCEqRHz91kpG6Uvyn2tLmnIdJbPE4vYv\nWLrtXXfFBSSPD4Afn7+3/XUggAlc7oCTizOfbbtOFlYA4g5KcYgS1J2ZAeMQqbUd\nZseZCcaZZZn65tdqee8UXZlDvx0+NdO0LR+5pFy+juM0wWbu59MvzcmTXbjsi7HY\n6zd53Yq5K244fwFHRQ8eOB0IWB+4PfM7FeAApZvlfqlKOlLcZL2uyVmzRkyR5yW7\n2uo9mehX44CiPJ2fse9Y6eQtcfEhMPkmHXI01sN+KwPbpA39+xOsStjhP9N1Y1a2\ntQAVo+yVgLgV2Hws73Fc0o3wC78qPEA+v2aRs/Be3ZFDgDyghc/1fgU+7C+P6kbq\nd4poyb6IW8KCJbxfMJvkordNOgOUUxndPHEi/tb/U7uLjLOgPA==\n-----END CERTIFICATE-----\n".as_bytes();
    let isv_body = "{\"id\":\"28059165425966003836075402765879561587\",\"timestamp\":\"2020-06-23T10:02:29.441419\",\"version\":3,\"epidPseudonym\":\"4tcrS6EX9pIyhLyxtgpQJuMO1VdAkRDtha/N+u/rRkTsb11AhkuTHsY6UXRPLRJavxG3nsByBdTfyDuBDQTEjMYV6NBXjn3P4UyvG1Ae2+I4lE1n+oiKgLA8CR8pc2nSnSY1Wz1Pw/2l9Q5Er6hM6FdeECgMIVTZzjScYSma6rE=\",\"isvEnclaveQuoteStatus\":\"GROUP_OUT_OF_DATE\",\"platformInfoBlob\":\"1502006504000F00000F0F02040101070000000000000000000B00000B00000002000000000000142AA23C001F46C3A71CFB50557CE2E2292DFB24EDE2621957E890432F166F6AC6FA37CD8166DBE6323CD39D3C6AA0CB41779FC7EDE281C5E50BCDCA00935E00A9DF\",\"isvEnclaveQuoteBody\":\"AgABACoUAAAKAAkAAAAAAP7yPH5zo3mCPOcf8onPvAcAAAAAAAAAAAAAAAAAAAAACA7///8CAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAABwAAAAAAAAAHAAAAAAAAAOJWq0y16RNrwcERUIj8QMofQYJUXqdXaVeMINhDAozVAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAACD1xnnferKFHD2uvYqTXdDA8iZ22kCD5xw7h38CMfOngAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAABNu2QBUIMjsY9knwTxdDP9S4cgHvP/Y0toS3FchIu2C5Bd1TBeJHYbSWioh139n2q/sxENn6SU3VMNquzMg1Ph\"}".as_bytes();
    let sig = hex::decode("3022068d50f3edaf63b5aab8f47089091d1cc4c0cf7f55991da40e244a3d26ea6beecaec1b513d281f951dc211338146c31007ff370b296aaf8d9295b2806b65").unwrap();

    RegisterInfo {
        ias_sig: ias_sig.to_vec(),
        ias_cert: ias_cert.to_vec(),
        account_id: applier,
        isv_body: isv_body.to_vec(),
        sig
    }
}

pub fn legal_work_report() -> ReportWorksInfo {
    let curr_pk = hex::decode("69a2e1757b143b45246c6a47c1d2fd4db263328ee9e84f7950414a4ce420079eafa07d062f4fd716104040f3a99159e33434218a8c7c3107a9101fb007dead82").unwrap();
    let prev_pk = hex::decode("").unwrap();
    let block_number: u64 = 300;
    let block_hash = hex::decode("0000000000000000000000000000000000000000000000000000000000000000").unwrap();
    let free: u64 = 4294967296;
    let used: u64 = 2;
    let added_files: Vec<(Vec<u8>, u64)> = vec![];
    let deleted_files: Vec<(Vec<u8>, u64)> = vec![];
    let files_root = hex::decode("11").unwrap();
    let srd_root = hex::decode("00").unwrap();
    let sig = hex::decode("d537cc3578cdc126934efee55ab43741e4f2fa9430b7c92c00fad4e020810e3790b1661f3885b8479c1b9f8d7d81d03766ccaef60bd85ba663390483d50788d2").unwrap();

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

pub fn legal_work_report_with_added_files() -> ReportWorksInfo {
    let curr_pk = hex::decode("7c16c0a0d7a1ccf654aa2925fe56575823972adaa0125ffb843d9a1cae0e1f2ea4f3d820ff59d5631ff873693936ebc6b91d0af22b821299019dbacf40f5791d").unwrap();
    let prev_pk = hex::decode("").unwrap();
    let block_number: u64 = 300;
    let block_hash = hex::decode("0000000000000000000000000000000000000000000000000000000000000000").unwrap();
    let free: u64 = 4294967296;
    let used: u64 = 402868224;
    let added_files: Vec<(Vec<u8>, u64)> = [
        (hex::decode("5bb706320afc633bfb843108e492192b17d2b6b9d9ee0b795ee95417fe08b660").unwrap(), 134289408),
        (hex::decode("88cdb315c8c37e2dc00fa2a8c7fe51b8149b363d29f404441982f96d2bbae65f").unwrap(), 268578816)
    ].to_vec();
    let deleted_files: Vec<(Vec<u8>, u64)> = vec![];
    let files_root = hex::decode("11").unwrap();
    let srd_root = hex::decode("00").unwrap();
    let sig = hex::decode("b3f78863ec972955d9ca22d444a5475085a4f7975a738aba1eae1d98dd718fc691a77a35b764a148a3a861a4a2ef3279f3d5e25f607c73ca85ea86e1176ba662").unwrap();

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
    let curr_pk = hex::decode("819e555a290c4f725739eb03a3e8d0f31db074a6e16abeec3a9a6a7c0379b6de9ad4d7658c44257746d58764e9db9c736d39474199ce53e4edfcc3d5340f1916").unwrap();
    let prev_pk = hex::decode("").unwrap();
    let block_number: u64 = 300;
    let block_hash = hex::decode("0000000000000000000000000000000000000000000000000000000000000000").unwrap();
    let free: u64 = 4294967296;
    let used: u64 = 0;
    let added_files: Vec<(Vec<u8>, u64)> = [].to_vec();
    let deleted_files: Vec<(Vec<u8>, u64)> = vec![
        (hex::decode("5aa706320afc633bfb843108e492192b17d2b6b9d9ee0b795ee95417fe08b660").unwrap(), 1),
        (hex::decode("99cdb315c8c37e2dc00fa2a8c7fe51b8149b363d29f404441982f96d2bbae65f").unwrap(), 1),
        (hex::decode("77cdb315c8c37e2dc00fa2a8c7fe51b8149b363d29f404441982f96d2bbae65f").unwrap(), 1)
    ];
    let files_root = hex::decode("11").unwrap();
    let srd_root = hex::decode("00").unwrap();
    let sig = hex::decode("3bce32266ddc55a713f67395a75c0cf0ad66aa9d3b102bea0dcd551a374792289e391f1f79a297fa31459c9969b862056840f07b15373f07f43542361b7664b4").unwrap();

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

pub fn resuming_work_report() -> ReportWorksInfo {
    let mut legal_wr = legal_work_report();
    legal_wr.block_number = 900;
    legal_wr.curr_pk = hex::decode("8dfc5c61af8b9acf32e2d0eee52666da84cd8a205527a02c97d57220044982e5592ace42cd5e0ad483a3569d81b793723cd28e9973fddfc6c5ca44c95dc91f33").unwrap();
    legal_wr.sig = hex::decode("577b5c8753cc7ccd8a63604e8b773fdb18b5b82d7926f916d7243f9bfd3bcb12d4b3a1109ee8d1c5d261a39eba8a4869208e14d5e6bd4de6c62e35dbdeb6128f").unwrap();
    legal_wr
}

pub fn ab_upgrade_work_report() -> ReportWorksInfo {
    let mut legal_wr = legal_work_report();
    legal_wr.block_number = 600;
    legal_wr.curr_pk = hex::decode("3dd32a6624d1a39af67620fb9221928f6892907456109167a8230b331f662458263805d7db1598b98ed363b594ab6f1a52f2c66a6524d09fbd19f064f02c0a73").unwrap();
    legal_wr.prev_pk = hex::decode("69a2e1757b143b45246c6a47c1d2fd4db263328ee9e84f7950414a4ce420079eafa07d062f4fd716104040f3a99159e33434218a8c7c3107a9101fb007dead82").unwrap();
    legal_wr.sig = hex::decode("3949297f56d65adacb6f5837b63a050c2aaf2f5674c425792b37823f78a36254a67a259ab5e03bbfab31d8d716db101036cc42cfb1fbb126c04772763c44486d").unwrap();
    legal_wr
}

pub fn continuous_ab_upgrade_work_report() -> ReportWorksInfo {
    let mut legal_wr = ab_upgrade_work_report();
    legal_wr.block_number = 900;
    legal_wr.added_files = vec![(hex::decode("5bb706320afc633bfb843108e492192b17d2b6b9d9ee0b795ee95417fe08b660").unwrap(), 2)];
    legal_wr.deleted_files = vec![(hex::decode("99cdb315c8c37e2dc00fa2a8c7fe51b8149b363d29f404441982f96d2bbae65f").unwrap(), 1)];
    legal_wr.used = 3;
    legal_wr.sig = hex::decode("d0fb8ec28beea243a550a51b99ae92a046b9829d87783cbc70e86d98ac9cf3b47cfa8148ba4ce6e8ed4352f8fa550437db6effe5f31a3ada755c0f783c83f2c3").unwrap();
    legal_wr
}

pub fn continuous_work_report_300() -> ReportWorksInfo {
    let mut legal_wr = legal_work_report();
    legal_wr.block_number = 300;
    legal_wr.curr_pk = hex::decode("8a71e8588914aeaeaebd27fbf315486398d76d4d32c2169b174a022f671e2e5bd7c9acb1d9259edf9f362e2af29f2df148c5c97eb1f2aec616a5d3c899a39a36").unwrap();
    legal_wr.sig = hex::decode("38a4bf8a17b9578c3ac4758e542f10836b7609f698ebadc76fe9d6314270460ed3adaab60f2c08617fc9307c703192c4b831393a714f88dc62013f0123c19ec9").unwrap();

    legal_wr
}

pub fn continuous_work_report_600() -> ReportWorksInfo {
    let mut legal_wr = legal_work_report();
    legal_wr.block_number = 600;
    legal_wr.curr_pk = hex::decode("8a71e8588914aeaeaebd27fbf315486398d76d4d32c2169b174a022f671e2e5bd7c9acb1d9259edf9f362e2af29f2df148c5c97eb1f2aec616a5d3c899a39a36").unwrap();
    legal_wr.sig = hex::decode("e435a3f626c101ed377eea85271cb47f249ab2d90e17a606a2211dd760ee84de6444d9ac200bffc7f11728439ea866881fb3c497b5b8f2a99ce9e91fb69d4373").unwrap();

    legal_wr
}

pub fn register(who: &AccountId, pk: &SworkerPubKey, code: &SworkerCode) {
    Swork::maybe_upsert_id(who, pk, code);
}

pub fn add_wr(pk: &SworkerPubKey, wr: &WorkReport) {
    <self::WorkReports>::insert(pk.clone(), wr.clone());
    <self::ReportedInSlot>::insert(pk.clone(), wr.report_slot, true);
}

pub fn add_pending_sorders(who: &AccountId) {
    let files: Vec<Vec<u8>> = [
        hex::decode("5bb706320afc633bfb843108e492192b17d2b6b9d9ee0b795ee95417fe08b660").unwrap(),
        hex::decode("5bb706320afc633bfb843108e492192b17d2b6b9d9ee0b795ee95417fe08b660").unwrap(),
        hex::decode("88cdb315c8c37e2dc00fa2a8c7fe51b8149b363d29f404441982f96d2bbae65f").unwrap()
    ].to_vec();

    for (idx, file) in files.iter().enumerate() {
        insert_sorder(who, file, idx as u8, 1000, OrderStatus::Pending);
    }
}

pub fn add_success_sorders(who: &AccountId) {
    let files: Vec<Vec<u8>> = [
        hex::decode("5aa706320afc633bfb843108e492192b17d2b6b9d9ee0b795ee95417fe08b660").unwrap(),
        hex::decode("99cdb315c8c37e2dc00fa2a8c7fe51b8149b363d29f404441982f96d2bbae65f").unwrap(),
    ].to_vec();

    for (idx, file) in files.iter().enumerate() {
        insert_sorder(who, file, idx as u8, 1000, OrderStatus::Success);
    }
}

fn insert_sorder(who: &AccountId, f_id: &MerkleRoot, rd: u8, expired_on: u32, os: OrderStatus) {
    let mut file_map = Market::merchants(who).unwrap_or_default().file_map;
    let sorder_id: Hash = Hash::repeat_byte(rd);
    let sorder_info = SorderInfo {
        file_identifier: f_id.clone(),
        file_size: 0,
        created_on: 0,
        merchant: who.clone(),
        client: who.clone(),
        amount: 10,
        duration: 50
    };
    let sorder_status = SorderStatus {
        completed_on: 0,
        expired_on,
        status: os,
        claimed_at: 0
    };
    if let Some(orders) = file_map.get_mut(f_id) {
        orders.push(sorder_id.clone())
    } else {
        file_map.insert(f_id.clone(), vec![sorder_id.clone()]);
    }

    let provision = MerchantInfo {
        address_info: vec![],
        storage_price: 1,
        file_map
    };
    <market::Merchants<Test>>::insert(who, provision);
    <market::SorderInfos<Test>>::insert(sorder_id.clone(), sorder_info);
    <market::SorderStatuses<Test>>::insert(sorder_id.clone(), sorder_status);
    let punishment = SorderPunishment {
        success: 0,
        failed: 0,
        updated_at: 50
    };
    <market::SorderPunishments<Test>>::insert(sorder_id, punishment);
}

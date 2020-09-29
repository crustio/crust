use super::*;

use frame_support::{
    impl_outer_origin, parameter_types, impl_outer_dispatch,
    weights::{Weight, constants::RocksDbWeight},
    traits::{OnFinalize, OnInitialize, Get, TestRandomness}, assert_ok
};
use sp_runtime::{
    testing::Header,
    traits::{BlakeTwo256, IdentityLookup, SaturatedConversion},
    Perbill,
};
use std::{cell::RefCell};
use balances::AccountData;
pub type Balance = u64;

use sp_core::{crypto::AccountId32, H256};
pub type AccountId = AccountId32;

use primitives::{
    constants::time::MINUTES, BlockNumber
};

impl_outer_origin! {
    pub enum Origin for Test where system = system {}
}

impl_outer_dispatch! {
    pub enum Call for Test where origin: Origin {
        system::System,
        balances::Balances,
        payment::Payment,
	}
}

pub struct CurrencyToVoteHandler;
impl Convert<u64, u64> for CurrencyToVoteHandler {
    fn convert(x: u64) -> u64 {
        x
    }
}
impl Convert<u128, u64> for CurrencyToVoteHandler {
    fn convert(x: u128) -> u64 {
        x.saturated_into()
    }
}

impl Convert<u128, u128> for CurrencyToVoteHandler {
    fn convert(x: u128) -> u128 {
        x
    }
}

impl Convert<u64, u128> for CurrencyToVoteHandler {
    fn convert(x: u64) -> u128 {
        x as u128
    }
}

// For testing the module, we construct most of a mock runtime. This means
// first constructing a configuration type (`Test`) which `impl`s each of the
// configuration traits of modules we want to use.
#[derive(Clone, Eq, PartialEq, Debug)]
pub struct Test;

thread_local! {
    static EXISTENTIAL_DEPOSIT: RefCell<u64> = RefCell::new(0);
}

pub struct ExistentialDeposit;
impl Get<u64> for ExistentialDeposit {
    fn get() -> u64 {
        EXISTENTIAL_DEPOSIT.with(|v| *v.borrow())
    }
}

parameter_types! {
    pub const BlockHashCount: u64 = 250;
    pub const MaximumBlockWeight: Weight = 1024;
    pub const MaximumBlockLength: u32 = 2 * 1024;
    pub const AvailableBlockRatio: Perbill = Perbill::from_percent(75);
}

impl system::Trait for Test {
    type BaseCallFilter = ();
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

pub struct TestOrderInspector;

impl market::OrderInspector<AccountId> for TestOrderInspector {
    // file size should smaller than merchant's num
    fn check_works(_provider: &AccountId, _file_size: u64) -> bool {
        true
    }
}

parameter_types! {
    pub const MinimumStoragePrice: Balance = 1;
    pub const MinimumSorderDuration: u32 = 1;
    pub const Frequency: BlockNumber = MINUTES;
}

impl balances::Trait for Test {
    type Balance = Balance;
    type DustRemoval = ();
    type Event = ();
    type ExistentialDeposit = ExistentialDeposit;
    type AccountStore = System;
    type WeightInfo = ();
    type MaxLocks = ();
}

impl swork::Trait for Test {
    type Currency = Balances;
    type Event = ();
    type Works = ();
    type MarketInterface = Market;
}

parameter_types! {
    pub const PunishDuration: market::EraIndex = 100;
}

impl market::Trait for Test {
    type Currency = Balances;
    type CurrencyToBalance = CurrencyToVoteHandler;
    type Event = ();
    type Randomness = TestRandomness;
    type Payment = Payment;
    type OrderInspector = TestOrderInspector;
    type MinimumStoragePrice = MinimumStoragePrice;
    type MinimumSorderDuration = MinimumSorderDuration;
    type PunishDuration = PunishDuration;
}

impl Trait for Test {
    type Proposal = Call;
    type Currency = Balances;
    type Event = ();
    type CurrencyToBalance = CurrencyToVoteHandler;
    // TODO: Bonding with balance module(now we impl inside Market)
    type MarketInterface = Market;
    type Frequency = Frequency;
}

pub type Market = market::Module<Test>;
pub type System = system::Module<Test>;
pub type Swork = swork::Module<Test>;
pub type Payment = Module<Test>;
pub type Balances = balances::Module<Test>;

// This function basically just builds a genesis storage key/value store according to
// our desired mockup.
pub fn new_test_ext() -> sp_io::TestExternalities {
    let mut t = system::GenesisConfig::default()
    .build_storage::<Test>()
    .unwrap();

    let _ = swork::GenesisConfig {
        code: vec![],
    }.assimilate_storage(&mut t);

    t.into()
}

/// Run until a particular block.
// TODO: make it into util?
pub fn run_to_block(n: u64) {
    while System::block_number() < n {
        if System::block_number() > 1 {
            System::on_finalize(System::block_number());
        }
        System::set_block_number(System::block_number() + 1);
        System::on_initialize(System::block_number());
        Payment::on_initialize(System::block_number());
    }
}

pub fn add_work_report(merchant: &AccountId) {
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

    // 1. Register for this merchant
    <swork::Identities>::insert(curr_pk.clone(), hex::decode("").unwrap());
    <swork::IdBonds<Test>>::insert(merchant.clone(), vec![curr_pk.clone()]);

    // 2. Report works
    assert_ok!(Swork::report_works(
        Origin::signed(merchant.clone()),
        curr_pk,
        prev_pk,
        block_number,
        block_hash,
        free,
        used,
        added_files,
        deleted_files,
        srd_root,
        files_root,
        sig
    ));
}

use super::*;

use frame_support::{
    impl_outer_origin, parameter_types,
    weights::{Weight, constants::RocksDbWeight},
    traits::{OnFinalize, OnInitialize, Get}
};
use sp_core::H256;
use sp_runtime::{
    testing::Header,
    traits::{BlakeTwo256, IdentityLookup, SaturatedConversion},
    Perbill,
};
use std::{cell::RefCell};
use primitives::Hash;
use balances::AccountData;

pub type AccountId = u64;
pub type Balance = u64;

impl_outer_origin! {
    pub enum Origin for Test where system = system {}
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

parameter_types! {
    pub const BlockHashCount: u64 = 250;
    pub const MaximumBlockWeight: Weight = 1024;
    pub const MaximumBlockLength: u32 = 2 * 1024;
    pub const AvailableBlockRatio: Perbill = Perbill::from_percent(75);
    pub const MinimumStoragePrice: Balance = 1;
    pub const MinimumSorderDuration: u32 = 1;
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
    type ModuleToIndex = ();
    type AccountData = AccountData<u64>;
    type OnNewAccount = ();
    type OnKilledAccount = ();
}

pub struct TestOrderInspector;

impl OrderInspector<AccountId> for TestOrderInspector {
    // file size should smaller than provider's num
    fn check_works(provider: &AccountId, file_size: u64) -> bool {
        if let Some(wr) = Tee::work_reports(provider) {
            wr.reserved > file_size
        } else {
            false
        }
    }
}

impl balances::Trait for Test {
    type Balance = Balance;
    type DustRemoval = ();
    type Event = ();
    type ExistentialDeposit = ExistentialDeposit;
    type AccountStore = System;
}

impl tee::Trait for Test {
    type Currency = Balances;
    type Event = ();
    type Works = ();
    type MarketInterface = ();
}

impl Payment<<Test as system::Trait>::AccountId,
    <Test as system::Trait>::Hash, BalanceOf<Test>> for Market
{
    fn reserve_sorder(_: &Hash, _: &AccountId, _: Balance) -> bool {
        true
    }

    fn pay_sorder(_: &<Test as system::Trait>::Hash) { }
}

parameter_types! {
    pub const TestPunishDuration: EraIndex = 100;
}

impl Trait for Test {
    type Currency = Balances;
    type CurrencyToBalance = CurrencyToVoteHandler;
    type Event = ();
    type Randomness = ();
    type Payment = Market;
    type OrderInspector = TestOrderInspector;
    type MinimumStoragePrice = MinimumStoragePrice;
    type MinimumSorderDuration = MinimumSorderDuration;
    type PunishDuration = TestPunishDuration;
}

pub type Market = Module<Test>;
pub type System = system::Module<Test>;
pub type Tee = tee::Module<Test>;
pub type Balances = balances::Module<Test>;

// This function basically just builds a genesis storage key/value store according to
// our desired mockup.
pub fn new_test_ext() -> sp_io::TestExternalities {
    let mut t = system::GenesisConfig::default()
    .build_storage::<Test>()
    .unwrap();

    // tee genesis
    let identities: Vec<u64> = vec![0, 100, 200];
    let work_reports: Vec<(u64, tee::WorkReport)> = identities
            .iter()
            .map(|id| {
                (
                    *id,
                    tee::WorkReport {
                        block_number: 0,
                        files: vec![],
                        used: 0,
                        reserved: *id,
                    },
                )
            })
            .collect();

    let _ = tee::GenesisConfig::<Test> {
        current_report_slot: 0,
        code: vec![],
        identities: identities
            .iter()
            .map(|id| (*id, Default::default()))
            .collect(),
        work_reports
    }
    .assimilate_storage(&mut t);

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
    }
}

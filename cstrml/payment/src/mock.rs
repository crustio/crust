use super::*;

use frame_support::{
    impl_outer_origin, parameter_types, impl_outer_dispatch,
    weights::{Weight, constants::RocksDbWeight},
    traits::{OnFinalize, OnInitialize, Get}
};
use sp_runtime::{
    testing::Header,
    traits::{BlakeTwo256, IdentityLookup, SaturatedConversion},
    Perbill,
};
use std::{cell::RefCell};
use balances::AccountData;
pub type Balance = u64;

use keyring::Sr25519Keyring;
use sp_core::{crypto::AccountId32, H256};
pub type AccountId = AccountId32;

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
    type ModuleToIndex = ();
    type AccountData = AccountData<u64>;
    type OnNewAccount = ();
    type OnKilledAccount = ();
}

pub struct TestOrderInspector;

impl market::OrderInspector<AccountId> for TestOrderInspector {
    // file size should smaller than provider's num
    fn check_works(_provider: &AccountId, _file_size: u64) -> bool {
        true
    }
}

parameter_types! {
    pub const MinimumStoragePrice: Balance = 1;
    pub const MinimumSorderDuration: u32 = 1;
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
    type MarketInterface = Market;
}

parameter_types! {
    pub const PunishDuration: market::EraIndex = 100;
}

impl market::Trait for Test {
    type Currency = Balances;
    type CurrencyToBalance = CurrencyToVoteHandler;
    type Event = ();
    type Randomness = ();
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
}

pub type Market = market::Module<Test>;
pub type System = system::Module<Test>;
pub type Tee = tee::Module<Test>;
pub type Payment = Module<Test>;
pub type Balances = balances::Module<Test>;

// This function basically just builds a genesis storage key/value store according to
// our desired mockup.
pub fn new_test_ext() -> sp_io::TestExternalities {
    let mut t = system::GenesisConfig::default()
    .build_storage::<Test>()
    .unwrap();

    // initial authorities: [Alice, Bob]
    let accounts = [
        (
            Sr25519Keyring::Alice.to_account_id(),
            hex::decode("0fb42b36f26b69b7bbd3f60b2e377e66a4dacf0284877731bb59ca2cc9ce2759390dfb4b7023986e238d74df027f0f7f34b51f4b0dbf60e5f0ac90812d977499").unwrap()
        ),
        (
            Sr25519Keyring::Bob.to_account_id(),
            hex::decode("b0b0c191996073c67747eb1068ce53036d76870516a2973cef506c29aa37323892c5cc5f379f17e63a64bb7bc69fbea14016eea76dae61f467c23de295d7f689").unwrap()
        )
    ];

    let identities = accounts
        .iter()
        .map(|(id, pk)| {
            (
                id.clone(),
                tee::Identity {
                    ias_sig: vec![],
                    pub_key: pk.clone(),
                    code: vec![],
                    account_id: id.clone(),
                    sig: vec![],
                    ias_cert: vec![],
                    isv_body: vec![]
                },
            )
        })
        .collect();
    let work_reports = accounts
        .iter()
        .map(|(x, _)| (x.clone(), Default::default()))
        .collect();

    let _ = tee::GenesisConfig::<Test> {
        current_report_slot: 0,
        code: vec![],
        identities,
        work_reports
    }
    .assimilate_storage(&mut t);

    t.into()
}

/// Run until a particular block.
// TODO: make it into util?
pub fn run_to_block(n: u64) {
    let fake_bh = H256::from_slice(hex::decode("05404b690b0c785bf180b2dd82a431d88d29baf31346c53dbda95e83e34c8a75").unwrap().as_slice());
    while System::block_number() < n {
        <system::BlockHash<Test>>::insert(System::block_number(), fake_bh.clone());
        if System::block_number() > 1 {
            System::on_finalize(System::block_number());
        }
        System::set_block_number(System::block_number() + 1);
        System::on_initialize(System::block_number());
        Payment::on_initialize(System::block_number());
    }
}

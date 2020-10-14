use super::*;

use frame_support::{
    impl_outer_origin, parameter_types,
    weights::{Weight, constants::RocksDbWeight},
    traits::{OnFinalize, OnInitialize, Get, TestRandomness}
};
use sp_core::H256;
use sp_runtime::{
    testing::Header,
    traits::{BlakeTwo256, IdentityLookup, SaturatedConversion},
    Perbill,
};
use std::{cell::RefCell};
use balances::AccountData;
pub use primitives::{MerkleRoot, Hash};

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
    type PalletInfo = ();
    type AccountData = AccountData<u64>;
    type OnNewAccount = ();
    type OnKilledAccount = ();
    type SystemWeightInfo = ();
}

pub struct TestOrderInspector;

impl OrderInspector<AccountId> for TestOrderInspector {
    // file size should smaller than merchant's num
    fn check_works(merchant: &AccountId, file_size: u64) -> bool {
        let mut free = 0;

        // Loop and sum all pks
        for pk in Swork::id_bonds(merchant) {
            if let Some(wr) = Swork::work_reports(pk) {
                // Pruning
                if wr.free > file_size { return true }
                free = free + wr.free;
            }
        }

        free > file_size
    }
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
    type MarketInterface = ();
}

parameter_types! {
    pub const TestClaimLimit: u32 = 100;
}

impl Trait for Test {
    type Currency = Balances;
    type CurrencyToBalance = CurrencyToVoteHandler;
    type Event = ();
    type Randomness = TestRandomness;
    type OrderInspector = TestOrderInspector;
    type MinimumStoragePrice = MinimumStoragePrice;
    type MinimumSorderDuration = MinimumSorderDuration;
    type ClaimLimit = TestClaimLimit;
}

pub type Market = Module<Test>;
pub type System = system::Module<Test>;
pub type Swork = swork::Module<Test>;
pub type Balances = balances::Module<Test>;

pub fn new_test_ext() -> sp_io::TestExternalities {
    let mut t = system::GenesisConfig::default()
    .build_storage::<Test>()
    .unwrap();

    let _ = swork::GenesisConfig {
        code: vec![],
    }.assimilate_storage(&mut t);

    let mut ext: sp_io::TestExternalities = t.into();
    ext.execute_with(|| {
        init_swork_setup();
    });

    ext
}

pub fn init_swork_setup() {
    // 1. Register for 0, 100, 200
    let pk1 = hex::decode("11").unwrap();
    let pk2 = hex::decode("22").unwrap();
    let pk3 = hex::decode("33").unwrap();
    let pk4 = hex::decode("44").unwrap();
    let code = hex::decode("").unwrap();

    <swork::Identities>::insert(pk1.clone(), code.clone());
    <swork::Identities>::insert(pk1.clone(), code.clone());
    <swork::Identities>::insert(pk1.clone(), code.clone());

    <swork::IdBonds<Test>>::insert(0, vec![pk1.clone()]);

    // Test star network
    <swork::IdBonds<Test>>::insert(100, vec![pk2.clone(), pk3.clone()]);
    <swork::IdBonds<Test>>::insert(200, vec![pk4.clone()]);

    <swork::WorkReports>::insert(pk1.clone(), swork::WorkReport{
        report_slot: 0,
        used: 0,
        free: 0,
        files: Default::default(),
        reported_files_size: 0,
        reported_srd_root: vec![],
        reported_files_root: vec![]
    });

    // Test star network
    <swork::WorkReports>::insert(pk2.clone(), swork::WorkReport{
        report_slot: 0,
        used: 0,
        free: 50,
        files: Default::default(),
        reported_files_size: 0,
        reported_srd_root: vec![],
        reported_files_root: vec![]
    });
    <swork::WorkReports>::insert(pk3.clone(), swork::WorkReport{
        report_slot: 0,
        used: 0,
        free: 50,
        files: Default::default(),
        reported_files_size: 0,
        reported_srd_root: vec![],
        reported_files_root: vec![]
    });

    <swork::WorkReports>::insert(pk4.clone(), swork::WorkReport{
        report_slot: 0,
        used: 0,
        free: 200,
        files: Default::default(),
        reported_files_size: 0,
        reported_srd_root: vec![],
        reported_files_root: vec![]
    });
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

pub fn insert_sorder(who: &AccountId, f_id: &MerkleRoot, rd: u8, expired_on: u32, os: OrderStatus) {
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
        claimed_at: 50
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
    <Merchants<Test>>::insert(who, provision);
    <SorderInfos<Test>>::insert(sorder_id.clone(), sorder_info);
    <SorderStatuses<Test>>::insert(sorder_id.clone(), sorder_status);
    let punishment = SorderPunishment {
        success: 0,
        failed: 0,
        updated_at: 50
    };
    <SorderPunishments<Test>>::insert(sorder_id, punishment);
}

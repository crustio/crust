// Copyright (C) 2019-2021 Crust Network Technologies Ltd.
// This file is part of Crust.

//! Test utilities

use crate::*;
use crate as csm_locking;
use frame_support::{
    parameter_types,
    traits::{Get, OnInitialize, OnFinalize},
    weights::constants::RocksDbWeight,
};
use sp_core::H256;
use sp_runtime::testing::Header;
use sp_runtime::traits::IdentityLookup;
use std::cell::RefCell;
use balances::AccountData;
use frame_support::traits::StorageMapShim;

/// The AccountId alias in this test module.
pub type AccountId = u128;
pub type BlockNumber = u64;
pub type Balance = u64;

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
}

impl frame_system::Config for Test {
    type BaseCallFilter = ();
    type Origin = Origin;
    type Call = Call;
    type Index = u64;
    type BlockNumber = BlockNumber;
    type Hash = H256;
    type Hashing = ::sp_runtime::traits::BlakeTwo256;
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
    type BlockWeights = ();
    type BlockLength = ();
    type SS58Prefix = ();
}

impl balances::Config for Test {
    type Balance = Balance;
    type DustRemoval = ();
    type Event = ();
    type ExistentialDeposit = ExistentialDeposit;
    type AccountStore = StorageMapShim<
        balances::Account<Test>,
        frame_system::Provider<Test>,
        AccountId,
        balances::AccountData<Balance>,
    >;
    type WeightInfo = ();
    type MaxLocks = ();
}

parameter_types! {
    pub const BondingDuration: u32 = 1200;
}

impl Config for Test {
    type Currency = Balances;
    type Event = ();
    type BondingDuration = BondingDuration;
    type WeightInfo = weight::WeightInfo;
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
		CSMLocking: csm_locking::{Module, Call, Storage, Event<T>},
	}
);

pub struct ExtBuilder {
    existential_deposit: u64,
}

impl Default for ExtBuilder {
    fn default() -> Self {
        Self {
            existential_deposit: 1,
        }
    }
}

impl ExtBuilder {
    pub fn set_associated_consts(&self) {
        EXISTENTIAL_DEPOSIT.with(|v| *v.borrow_mut() = self.existential_deposit);
    }
    pub fn build(self) -> sp_io::TestExternalities {
        self.set_associated_consts();
        let mut storage = frame_system::GenesisConfig::default()
            .build_storage::<Test>()
            .unwrap();
        let balance_factor = if self.existential_deposit > 1 { 256 } else { 1 };

        let _ = balances::GenesisConfig::<Test> {
            balances: vec![
                (1, 10 * balance_factor),
                (2, 20 * balance_factor),
                (3, 300 * balance_factor),
                (4, 400 * balance_factor),
                (10, balance_factor),
                (11, balance_factor * 1000),
                (20, balance_factor),
                (21, balance_factor * 2000),
                (30, balance_factor),
                (31, balance_factor * 2000),
                (40, balance_factor),
                (41, balance_factor * 2000),
                (50, balance_factor),
                (51, balance_factor * 1000),
                (60, balance_factor),
                (61, balance_factor * 2000),
                (70, balance_factor),
                (71, balance_factor * 2000),
                (80, balance_factor),
                (81, balance_factor * 2000),
                (100, 2000 * balance_factor),
                (101, 2000 * balance_factor),
                // This allow us to have a total_payout different from 0.
                (999, 1_000_000_000_000)
            ],
        }.assimilate_storage(&mut storage);

        let ext = sp_io::TestExternalities::from(storage);
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
#![cfg(test)]

use super::*;

use hex_literal::hex;
use frame_support::{ord_parameter_types, parameter_types, weights::Weight};
use frame_system::{self as system};
use sp_core::H256;
use sp_runtime::{
	testing::Header, ModuleId,
	traits::{AccountIdConversion, BlakeTwo256, IdentityLookup},
	Perbill,
};

use crate::{self as bridge_transfer, Config};
pub use balances;
use cstrml_bridge as bridge;

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
		Bridge: bridge::{Module, Call, Storage, Event<T>},
		BridgeTransfer: bridge_transfer::{Module, Call, Storage, Event<T>},
		Timestamp: pallet_timestamp::{Module, Call, Storage, Inherent},
	}
);

parameter_types! {
	pub const BlockHashCount: u64 = 250;
	pub const MaximumBlockWeight: Weight = 1024;
	pub const MaximumBlockLength: u32 = 2 * 1024;
	pub const AvailableBlockRatio: Perbill = Perbill::one();
	pub const MaxLocks: u32 = 100;
	pub const MinimumPeriod: u64 = 1;
}

impl frame_system::Config for Test {
	type BaseCallFilter = ();
	type Origin = Origin;
	type Call = Call;
	type Index = u64;
	type BlockNumber = u64;
	type Hash = H256;
	type Hashing = BlakeTwo256;
	type AccountId = u64;
	type Lookup = IdentityLookup<Self::AccountId>;
	type Header = Header;
	type Event = Event;
	type BlockHashCount = BlockHashCount;
	type DbWeight = ();
	type Version = ();
	type AccountData = balances::AccountData<u64>;
	type OnNewAccount = ();
	type OnKilledAccount = ();
	type SystemWeightInfo = ();
	type PalletInfo = PalletInfo;
	type BlockWeights = ();
	type BlockLength = ();
	type SS58Prefix = ();
}

parameter_types! {
	pub const ExistentialDeposit: u64 = 1;
}

ord_parameter_types! {
	pub const One: u64 = 1;
}

impl balances::Config for Test {
	type Balance = u64;
	type DustRemoval = ();
	type Event = Event;
	type ExistentialDeposit = ExistentialDeposit;
	type AccountStore = System;
	type WeightInfo = ();
	type MaxLocks = ();
}

parameter_types! {
	pub const TestChainId: u8 = 5;
	pub const ProposalLifetime: u64 = 100;
}

impl bridge::Config for Test {
	type Event = Event;
	type BridgeCommitteeOrigin = frame_system::EnsureRoot<Self::AccountId>;
	type Proposal = Call;
	type BridgeChainId = TestChainId;
	type ProposalLifetime = ProposalLifetime;
}

parameter_types! {
	// bridge::derive_resource_id(1, &bridge::hashing::blake2_128(b"CRU"));
	pub const BridgeTokenId: [u8; 32] = hex!("000000000000000000000000000000608d1bc9a2d146ebc94667c336721b2801");
}

impl Config for Test {
	type Event = Event;
	type BridgeOrigin = bridge::EnsureBridge<Test>;
	type Currency = Balances;
	type BridgeTokenId = BridgeTokenId;
}

impl pallet_timestamp::Config for Test {
	type Moment = u64;
	type OnTimestampSet = ();
	type MinimumPeriod = MinimumPeriod;
	type WeightInfo = ();
}

pub const RELAYER_A: u64 = 0x2;
pub const RELAYER_B: u64 = 0x3;
pub const RELAYER_C: u64 = 0x4;
pub const ENDOWED_BALANCE: u64 = 100_000_000;

pub fn new_test_ext() -> sp_io::TestExternalities {
	let bridge_id = ModuleId(*b"crust/bg").into_account();
	let mut t = frame_system::GenesisConfig::default()
		.build_storage::<Test>()
		.unwrap();
	balances::GenesisConfig::<Test> {
		balances: vec![(bridge_id, ENDOWED_BALANCE), (RELAYER_A, ENDOWED_BALANCE)],
	}
	.assimilate_storage(&mut t)
	.unwrap();
	let mut ext = sp_io::TestExternalities::new(t);
	ext.execute_with(|| System::set_block_number(1));
	ext
}

fn last_event() -> Event {
	system::Pallet::<Test>::events()
		.pop()
		.map(|e| e.event)
		.expect("Event expected")
}

pub fn expect_event<E: Into<Event>>(e: E) {
	assert_eq!(last_event(), e.into());
}

// // Asserts that the event was emitted at some point.
// pub fn event_exists<E: Into<Event>>(e: E) {
// 	let actual: Vec<Event> = system::Pallet::<Test>::events()
// 		.iter()
// 		.map(|e| e.event.clone())
// 		.collect();
// 	let e: Event = e.into();
// 	let mut exists = false;
// 	for evt in actual {
// 		if evt == e {
// 			exists = true;
// 			break;
// 		}
// 	}
// 	assert!(exists);
// }

// Checks events against the latest. A contiguous set of events must be provided. They must
// include the most recent event, but do not have to include every past event.
pub fn assert_events(mut expected: Vec<Event>) {
	let mut actual: Vec<Event> = system::Pallet::<Test>::events()
		.iter()
		.map(|e| e.event.clone())
		.collect();

	expected.reverse();

	for evt in expected {
		let next = actual.pop().expect("event expected");
		assert_eq!(next, evt.into(), "Events don't match");
	}
}

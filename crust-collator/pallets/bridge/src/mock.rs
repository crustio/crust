// Copyright 2021 ChainSafe Systems
// SPDX-License-Identifier: GPL-3.0-only

#![deny(warnings)]
use crate as pallet_chainbridge;
use frame_support::{
    assert_ok,
    parameter_types,
    traits::SortedMembers,
    PalletId,
};
use frame_system::EnsureSignedBy;
use pallet_chainbridge::{
    types::ChainId,
    ResourceId,
};
use sp_core::H256;
use sp_runtime::{
    testing::Header,
    traits::{
        BlakeTwo256,
        IdentityLookup,
    },
};

type Balance = u64;
type UncheckedExtrinsic =
    frame_system::mocking::MockUncheckedExtrinsic<MockRuntime>;
type Block = frame_system::mocking::MockBlock<MockRuntime>;

// Constants definition
pub(crate) const RELAYER_A: u64 = 0x2;
pub(crate) const RELAYER_B: u64 = 0x3;
pub(crate) const RELAYER_C: u64 = 0x4;
pub(crate) const ENDOWED_BALANCE: u64 = 100_000_000;
pub(crate) const TEST_THRESHOLD: u32 = 2;

// Configure a mock runtime to test the pallet.
frame_support::construct_runtime!(
    pub enum MockRuntime where
        Block = Block,
        NodeBlock = Block,
        UncheckedExtrinsic = UncheckedExtrinsic,
    {

        System: frame_system::{Pallet, Call, Config, Storage, Event<T>},
        Bridge: pallet_chainbridge::{Pallet, Call, Storage, Event<T>},
        Balances: pallet_balances::{Pallet, Call, Config<T>, Storage, Event<T>},
    }
);

parameter_types! {
    pub const BlockHashCount: u64 = 250;
    pub const SS58Prefix: u8 = 42;
}

// Implement FRAME balances pallet configuration trait for the mock runtime
impl pallet_balances::Config for MockRuntime {
    type AccountStore = System;
    type Balance = Balance;
    type DustRemoval = ();
    type RuntimeEvent = RuntimeEvent;
    type ExistentialDeposit = ExistentialDeposit;
    type MaxLocks = ();
    type MaxReserves = ();
    type ReserveIdentifier = ();
    type WeightInfo = ();
}

impl frame_system::Config for MockRuntime {
    type AccountData = pallet_balances::AccountData<Balance>;
    type AccountId = u64;
    type BaseCallFilter = frame_support::traits::Everything;
    type BlockHashCount = BlockHashCount;
    type BlockLength = ();
    type BlockNumber = u64;
    type BlockWeights = ();
    type Call = Call;
    type DbWeight = ();
    type RuntimeEvent = RuntimeEvent;
    type Hash = H256;
    type Hashing = BlakeTwo256;
    type Header = Header;
    type Index = u64;
    type Lookup = IdentityLookup<Self::AccountId>;
    type OnKilledAccount = ();
    type OnNewAccount = ();
    type OnSetCode = ();
    type RuntimeOrigin = RuntimeOrigin;
    type PalletInfo = PalletInfo;
    type SS58Prefix = SS58Prefix;
    type SystemWeightInfo = ();
    type Version = ();
}

// Parameterize default test user identifier (with id 1)
parameter_types! {
    pub const TestUserId: u64 = 1;
    pub const TestChainId: ChainId = 5;
    pub const ProposalLifetime: u64 = 50;
    pub const ChainBridgePalletId: PalletId = PalletId(*b"chnbrdge");
}

impl SortedMembers<u64> for TestUserId {
    fn sorted_members() -> Vec<u64> {
        vec![1]
    }
}

// Parameterize FRAME balances pallet
parameter_types! {
    pub const ExistentialDeposit: u64 = 1;
}

impl pallet_chainbridge::Config for MockRuntime {
    type BridgeCommitteeOrigin = EnsureSignedBy<TestUserId, u64>;
    type BridgeChainId = TestChainId;
    type RuntimeEvent = RuntimeEvent;
    type PalletId = ChainBridgePalletId;
    type Proposal = Call;
    type ProposalLifetime = ProposalLifetime;
}

pub fn new_test_ext() -> sp_io::TestExternalities {
    let bridge_id = Bridge::account_id();
    let mut t = frame_system::GenesisConfig::default()
        .build_storage::<MockRuntime>()
        .unwrap();
    pallet_balances::GenesisConfig::<MockRuntime> {
        balances: vec![(bridge_id, ENDOWED_BALANCE)],
    }
    .assimilate_storage(&mut t)
    .unwrap();
    let mut ext = sp_io::TestExternalities::new(t);
    // Note: when block_number is not set, the events will not be stored
    ext.execute_with(|| System::set_block_number(1));
    ext
}

pub fn new_test_ext_initialized(
    src_id: ChainId,
    r_id: ResourceId,
    resource: Vec<u8>,
) -> sp_io::TestExternalities {
    let mut t = new_test_ext();
    t.execute_with(|| {
        // Set and check threshold
        assert_ok!(Bridge::set_threshold(
            crate::mock::RuntimeOrigin::root(),
            TEST_THRESHOLD
        ));
        assert_eq!(Bridge::relayer_threshold(), TEST_THRESHOLD);
        // Add relayers
        assert_ok!(Bridge::add_relayer(RuntimeOrigin::root(), RELAYER_A));
        assert_ok!(Bridge::add_relayer(RuntimeOrigin::root(), RELAYER_B));
        assert_ok!(Bridge::add_relayer(RuntimeOrigin::root(), RELAYER_C));
        // Whitelist chain
        assert_ok!(Bridge::whitelist_chain(RuntimeOrigin::root(), src_id));
        // Set and check resource ID mapped to some junk data
        assert_ok!(Bridge::set_resource(RuntimeOrigin::root(), r_id, resource));
        assert_eq!(Bridge::resource_exists(r_id), true);
    });
    t
}

// Checks events against the latest. A contiguous set of events must be provided. They must
// include the most recent event, but do not have to include every past event.
pub fn assert_events(mut expected: Vec<Event>) {
    let mut actual: Vec<Event> = frame_system::Pallet::<MockRuntime>::events()
        .iter()
        .map(|e| e.event.clone())
        .collect();
    dbg!(&actual);

    expected.reverse();

    for evt in expected {
        dbg!(&evt);
        let next = actual.pop().expect("event expected");
        assert_eq!(next, evt.into(), "Events don't match (actual,expected)");
    }
}

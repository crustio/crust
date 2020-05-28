use crate::*;

use frame_support::{
    impl_outer_origin, parameter_types,
    weights::{Weight, constants::RocksDbWeight},
    traits::{ OnInitialize, OnFinalize }
};
use keyring::Sr25519Keyring;
use sp_core::{crypto::AccountId32, H256};
use sp_runtime::{
    testing::Header,
    traits::{BlakeTwo256, IdentityLookup},
    Perbill,
};
use market::{Provision, StorageOrder};
use primitives::{MerkleRoot, Hash};

type AccountId = AccountId32;

impl_outer_origin! {
    pub enum Origin for Test where system = system {}
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
    type AccountData = ();
    type OnNewAccount = ();
    type OnKilledAccount = ();
}

impl market::Trait for Test {
    type Event = ();
    type Randomness = ();
    type Payment = Market;
    type OrderInspector = Tee;
}

impl Trait for Test {
    type Event = ();
    type Works = ();
    type MarketInterface = Market;
}

pub type Tee = Module<Test>;
pub type System = system::Module<Test>;
pub type Market = market::Module<Test>;

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

    let tee_identities = accounts
        .iter()
        .map(|(id, pk)| {
            (
                id.clone(),
                Identity {
                    pub_key: pk.clone(),
                    account_id: id.clone(),
                    validator_pub_key: pk.clone(),
                    validator_account_id: id.clone(),
                    sig: vec![],
                },
            )
        })
        .collect();
    let work_reports = accounts
        .iter()
        .map(|(x, _)| (x.clone(), Default::default()))
        .collect();

    GenesisConfig::<Test> {
        current_report_slot: 0,
        tee_identities,
        work_reports,
    }
    .assimilate_storage(&mut t)
    .unwrap();

    t.into()
}

/// Run until a particular block.
pub fn run_to_block(n: u64) {
    // This block hash is for the valid work report
    let fake_bh = H256::from_slice(hex::decode("05404b690b0c785bf180b2dd82a431d88d29baf31346c53dbda95e83e34c8a75").unwrap().as_slice());
    while System::block_number() < n {
        <system::BlockHash<Test>>::insert(System::block_number(), fake_bh.clone());
        if System::block_number() > 1 {
            System::on_finalize(System::block_number());
        }
        System::on_initialize(System::block_number());
        System::set_block_number(System::block_number() + 1);
    }
}

pub fn upsert_sorder_to_provider(who: &AccountId, f_id: &MerkleRoot, rd: u8, os: OrderStatus) {
    let mut file_map = Market::providers(who).unwrap_or_default().file_map;
    let sorder_id: Hash = Hash::repeat_byte(rd);
    let sorder = StorageOrder {
        file_identifier: f_id.clone(),
        file_size: 0,
        created_on: 0,
        completed_on: 0,
        expired_on: 0,
        provider: who.clone(),
        client: who.clone(),
        order_status: os
    };
    file_map.insert(f_id.clone(), sorder_id.clone());
    let provision = Provision {
        address_info: vec![],
        file_map
    };
    <market::Providers<Test>>::insert(who, provision);
    <market::StorageOrders<Test>>::insert(sorder_id, sorder);
}

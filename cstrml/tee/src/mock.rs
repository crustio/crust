use super::*;

use frame_support::{impl_outer_origin, parameter_types, weights::Weight};
use keyring::Sr25519Keyring;
use sp_core::{crypto::AccountId32, H256};
use sp_runtime::{
    testing::Header,
    traits::{BlakeTwo256, IdentityLookup, OnFinalize, OnInitialize},
    Perbill,
};

type AccountId = AccountId32;

impl_outer_origin! {
    pub enum Origin for Test {}
}

// For testing the module, we construct most of a mock runtime. This means
// first constructing a configuration type (`Test`) which `impl`s each of the
// configuration traits of modules we want to use.
#[derive(Clone, Eq, PartialEq)]
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
    type MaximumBlockLength = MaximumBlockLength;
    type AvailableBlockRatio = AvailableBlockRatio;
    type Version = ();
    type ModuleToIndex = ();
}

impl Trait for Test {
    type Event = ();
    type OnReportWorks = ();
}

pub type Tee = Module<Test>;
pub type System = system::Module<Test>;

// This function basically just builds a genesis storage key/value store according to
// our desired mockup.
pub fn new_test_ext() -> sp_io::TestExternalities {
    let mut t = system::GenesisConfig::default()
        .build_storage::<Test>()
        .unwrap();

    // stash-controller accounts
    let accounts = [Sr25519Keyring::Alice.to_account_id()];

    let pk = hex::decode("8d61578381b5def81a39332a2dfe1afb88c8da1cb45f5322e9b3856cec5fe5b2d1231a1e0f93f3424e2cdf27f23a7e850cd140e8fd79b104a87428988914be62").unwrap();
    let tee_identities = accounts
        .iter()
        .map(|x| {
            (
                x.clone(),
                Identity {
                    pub_key: pk.clone(),
                    account_id: x.clone(),
                    validator_pub_key: pk.clone(),
                    validator_account_id: x.clone(),
                    sig: vec![],
                },
            )
        })
        .collect();
    let work_reports = accounts
        .iter()
        .map(|x| ((x.clone(), 0), Default::default()))
        .collect();

    GenesisConfig::<Test> {
        tee_identities,
        work_reports,
    }
    .assimilate_storage(&mut t)
    .unwrap();

    t.into()
}

/// Run until a particular block.
pub fn run_to_block(n: u64) {
    while System::block_number() < n {
        if System::block_number() > 1 {
            System::on_finalize(System::block_number());
        }
        System::set_block_number(System::block_number() + 1);
        System::on_initialize(System::block_number());
    }
}

use super::*;

use sp_core::{H256, crypto::AccountId32};
use frame_support::{impl_outer_origin, parameter_types, weights::Weight};
use sp_runtime::{
    traits::{BlakeTwo256, IdentityLookup, OnFinalize, OnInitialize},
    testing::{Header, UintAuthorityId},
    Perbill,
    curve::PiecewiseLinear
};
use keyring::Sr25519Keyring;
use cstrml_staking::StakerStatus;
use primitives::{Balance, BlockNumber, constants::currency::CRUS};

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

parameter_types! {
    pub const MinimumPeriod: u64 = 3;
}

impl timestamp::Trait for Test {
    type Moment = u64;
    type OnTimestampSet = ();
    type MinimumPeriod = MinimumPeriod;
}

parameter_types! {
    pub const ExistentialDeposit: Balance = 0;
    pub const TransferFee: Balance = 0;
    pub const CreationFee: Balance = 0;
}

impl balances::Trait for Test {
    type Balance = Balance;
    type OnFreeBalanceZero = ();
    type OnNewAccount = ();
    type TransferPayment = ();
    type DustRemoval = ();
    type Event = ();
    type ExistentialDeposit = ExistentialDeposit;
    type TransferFee = TransferFee;
    type CreationFee = CreationFee;
}

parameter_types! {
	pub const Period: BlockNumber = 1;
	pub const Offset: BlockNumber = 0;
	pub const UncleGenerations: u64 = 0;
	pub const DisabledValidatorsThreshold: Perbill = Perbill::from_percent(25);
}
impl session::Trait for Test {
    type Event = ();
    type ValidatorId = AccountId;
    type ValidatorIdOf = staking::StashOf<Test>;
    type ShouldEndSession = session::PeriodicSessions<Period, Offset>;
    type OnSessionEnding = ();
    type SessionHandler = session::TestSessionHandler;
    type Keys = UintAuthorityId;
    type DisabledValidatorsThreshold = DisabledValidatorsThreshold;
    type SelectInitialValidators = ();
}

impl session::historical::Trait for Test {
    type FullIdentification = staking::Exposure<AccountId, Balance>;
    type FullIdentificationOf = staking::ExposureOf<Test>;
}

pallet_staking_reward_curve::build! {
    const REWARD_CURVE: PiecewiseLinear<'static> = curve!(
        min_inflation: 0_025_000,
        max_inflation: 0_100_000,
        ideal_stake: 0_500_000,
        falloff: 0_050_000,
        max_piece_count: 40,
        test_precision: 0_005_000,
    );
}

parameter_types! {
    pub const SessionsPerEra: sp_staking::SessionIndex = 6;
    pub const BondingDuration: staking::EraIndex = 28;
    pub const SlashDeferDuration: staking::EraIndex = 7;
    pub const AttestationPeriod: BlockNumber = 100;
    pub const RewardCurve: &'static PiecewiseLinear<'static> = &REWARD_CURVE;
}

impl cstrml_staking::Trait for Test {
    type Currency = balances::Module<Test>;
    type Time = timestamp::Module<Test>;
    type CurrencyToVote = ();
    type RewardRemainder = ();
    type Event = ();
    type Slash = ();
    type Reward = ();
    type SessionsPerEra = SessionsPerEra;
    type BondingDuration = BondingDuration;
    type SlashDeferDuration = SlashDeferDuration;
    type SlashCancelOrigin = system::EnsureRoot<Self::AccountId>;
    type SessionInterface = Self;
    type RewardCurve = RewardCurve;
}


impl Trait for Test {
    type Event = ();
}

pub type Tee = Module<Test>;
pub type System = system::Module<Test>;
pub type Staking = cstrml_staking::Module<Test>;

// This function basically just builds a genesis storage key/value store according to
// our desired mockup.
pub fn new_test_ext() -> sp_io::TestExternalities {
    let mut t = system::GenesisConfig::default().build_storage::<Test>().unwrap();

    // stash-controller accounts
    let accounts = [
        (Sr25519Keyring::One.to_account_id(), Sr25519Keyring::Alice.to_account_id())
    ];

    let pk = hex::decode("5c4af2d40f305ce58aed1c6a8019a61d004781396c1feae5784a5f28cc8c40abe4229b13bc803ae9fbe93f589a60220b9b4816a5a199dfdab4a39b36c86a4c37").unwrap();
    let tee_identities = accounts.iter().map(|x|
        (x.1.clone(), Identity {
            pub_key: pk.clone(),
            account_id: x.1.clone(),
            validator_pub_key: pk.clone(),
            validator_account_id: x.1.clone(),
            sig: [0;32].to_vec()
        }))
        .collect();

    let stakers = accounts.iter().map(|i| (
        i.0.clone(),
        i.1.clone(),
        10_000 * CRUS,
        StakerStatus::Validator,
    )).collect();

    let balances = accounts.iter().map(|id|(id.0.clone(), 100000 * CRUS)).collect();

    GenesisConfig::<Test> {
        tee_identities
    }.assimilate_storage(&mut t).unwrap();

    balances::GenesisConfig::<Test> {
        balances,
        vesting: vec![],
    }.assimilate_storage(&mut t).unwrap();

    staking::GenesisConfig::<Test> {
        current_era: 0,
        stakers,
        validator_count: 4,
        minimum_validator_count: 1,
        invulnerables: vec![],
        .. Default::default()
    }.assimilate_storage(&mut t).unwrap();

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
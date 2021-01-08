// Copyright (C) 2019-2021 Crust Network Technologies Ltd.
// This file is part of Crust.

//! Test utilities

use crate::*;
use frame_support::{
    assert_ok, impl_outer_origin, parameter_types,
    StorageValue, IterableStorageMap,
    traits::{Currency, Get, FindAuthor, OnInitialize, OnFinalize, TestRandomness},
    weights::constants::RocksDbWeight,
};
use sp_core::{crypto::key_types, H256};
use sp_io;
use sp_runtime::testing::{Header, UintAuthorityId};
use sp_runtime::traits::{Convert, IdentityLookup, OpaqueKeys, SaturatedConversion};
use sp_runtime::{KeyTypeId, Perbill};
use sp_staking::{
    offence::{OffenceDetails, OnOffenceHandler},
    SessionIndex,
};
use std::{cell::RefCell, collections::HashSet, collections::btree_set::BTreeSet};
use balances::AccountData;
use primitives::{traits::{MarketInterface, StakingPotInterface}, MerkleRoot, SworkerAnchor};

/// The AccountId alias in this test module.
pub type AccountId = u128;
pub type BlockNumber = u64;
pub type Balance = u64;

/// Simple structure that exposes how u64 currency can be represented as... u64.
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

thread_local! {
    static SESSION: RefCell<(Vec<AccountId>, HashSet<AccountId>)> = RefCell::new(Default::default());
    static EXISTENTIAL_DEPOSIT: RefCell<u64> = RefCell::new(0);
    static SLASH_DEFER_DURATION: RefCell<EraIndex> = RefCell::new(0);
    static OWN_WORKLOAD: RefCell<u128> = RefCell::new(0);
    static TOTAL_WORKLOAD: RefCell<u128> = RefCell::new(0);
    static DSM_STAKING_PAYOUT: RefCell<Balance> = RefCell::new(0);
}

pub struct TestSessionHandler;
impl pallet_session::SessionHandler<AccountId> for TestSessionHandler {
    const KEY_TYPE_IDS: &'static [KeyTypeId] = &[key_types::DUMMY];

    fn on_genesis_session<Ks: OpaqueKeys>(_validators: &[(AccountId, Ks)]) {}

    fn on_new_session<Ks: OpaqueKeys>(
        _changed: bool,
        validators: &[(AccountId, Ks)],
        _queued_validators: &[(AccountId, Ks)],
    ) {
        SESSION.with(|x| {
            *x.borrow_mut() = (
                validators.iter().map(|x| x.0.clone()).collect(),
                HashSet::new(),
            )
        });
    }

    fn on_disabled(validator_index: usize) {
        SESSION.with(|d| {
            let mut d = d.borrow_mut();
            let value = d.0[validator_index];
            d.1.insert(value);
        })
    }
}

pub fn is_disabled(controller: AccountId) -> bool {
    let stash = Staking::ledger(&controller).unwrap().stash;
    SESSION.with(|d| d.borrow().1.contains(&stash))
}

pub struct ExistentialDeposit;
impl Get<u64> for ExistentialDeposit {
    fn get() -> u64 {
        EXISTENTIAL_DEPOSIT.with(|v| *v.borrow())
    }
}

pub struct SlashDeferDuration;
impl Get<EraIndex> for SlashDeferDuration {
    fn get() -> EraIndex {
        SLASH_DEFER_DURATION.with(|v| *v.borrow())
    }
}

impl_outer_origin! {
    pub enum Origin for Test where system = frame_system {}
}

/// Author of block is always 11
pub struct Author11;
impl FindAuthor<u128> for Author11 {
    fn find_author<'a, I>(_digests: I) -> Option<u128>
    where
        I: 'a + IntoIterator<Item = (frame_support::ConsensusEngineId, &'a [u8])>,
    {
        Some(11)
    }
}

// Workaround for https://github.com/rust-lang/rust/issues/26925 . Remove when sorted.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Test;
parameter_types! {
    pub const BlockHashCount: u64 = 250;
}

impl frame_system::Config for Test {
    type BaseCallFilter = ();
    type Origin = Origin;
    type Call = ();
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
    type PalletInfo = ();
    type AccountData = AccountData<u64>;
    type OnNewAccount = ();
    type OnKilledAccount = ();
    type SystemWeightInfo = ();
    type BlockWeights = ();
    type BlockLength = ();
}
parameter_types! {
    pub const TransferFee: Balance = 0;
    pub const CreationFee: Balance = 0;
}
impl balances::Config for Test {
    type Balance = Balance;
    type DustRemoval = ();
    type Event = ();
    type ExistentialDeposit = ExistentialDeposit;
    type AccountStore = System;
    type WeightInfo = ();
    type MaxLocks = ();
}
parameter_types! {
    pub const Period: BlockNumber = 1;
    pub const Offset: BlockNumber = 0;
    pub const UncleGenerations: u64 = 0;
    pub const DisabledValidatorsThreshold: Perbill = Perbill::from_percent(25);
}
impl pallet_session::Config for Test {
    type Event = ();
    type ValidatorId = AccountId;
    type ValidatorIdOf = crate::StashOf<Test>;
    type ShouldEndSession = pallet_session::PeriodicSessions<Period, Offset>;
    type NextSessionRotation = pallet_session::PeriodicSessions<Period, Offset>;
    type SessionManager = pallet_session::historical::NoteHistoricalRoot<Test, Staking>;
    type SessionHandler = TestSessionHandler;
    type Keys = UintAuthorityId;
    type DisabledValidatorsThreshold = DisabledValidatorsThreshold;
    type WeightInfo = ();
}

impl pallet_session::historical::Config for Test {
    type FullIdentification = crate::Exposure<AccountId, Balance>;
    type FullIdentificationOf = crate::ExposureOf<Test>;
}
impl pallet_authorship::Config for Test {
    type FindAuthor = Author11;
    type UncleGenerations = UncleGenerations;
    type FilterUncle = ();
    type EventHandler = Module<Test>;
}
parameter_types! {
    pub const MinimumPeriod: u64 = 5;
}
impl pallet_timestamp::Config for Test {
    type Moment = u64;
    type OnTimestampSet = ();
    type MinimumPeriod = MinimumPeriod;
    type WeightInfo = ();
}
pub struct TestStaking;
impl swork::Works<AccountId> for TestStaking {
    fn report_works(controller: &AccountId, _own_workload: u128, _total_workload: u128) {
        // Disable work report in mock test
        Staking::update_stake_limit(controller,
            OWN_WORKLOAD.with(|v| *v.borrow()),
            TOTAL_WORKLOAD.with(|v| *v.borrow()));
    }
}

impl StakingPotInterface<BalanceOf<Test>> for TestStaking {
    fn withdraw_staking_pot() -> BalanceOf<Test> {
        BalanceOf::<Test>::from(DSM_STAKING_PAYOUT.with(|v| *v.borrow()))
    }
}

impl<AID> MarketInterface<AID> for TestStaking {
    fn upsert_replicas(_: &AID, _: &MerkleRoot, _: &SworkerAnchor, _: u32, _: &Option<BTreeSet<AID>>) -> bool { false }
    fn delete_replicas(_: &AID, _: &MerkleRoot, _: &SworkerAnchor, _: u32) -> bool { false }
}

impl swork::Config for Test {
    type Currency = Balances;
    type Event = ();
    type Works = TestStaking;
    type MarketInterface = TestStaking;
    type WeightInfo = swork::weight::WeightInfo;
}

parameter_types! {
    pub const StakingModuleId: ModuleId = ModuleId(*b"cstaking");
    pub const SessionsPerEra: SessionIndex = 3;
    pub const BondingDuration: EraIndex = 3;
    pub const MaxGuarantorRewardedPerValidator: u32 = 4;
    pub const SPowerRatio: u128 = 2_500;
    pub const DSMStakingPotDuration: u32 = 5;
}

impl Config for Test {
    type ModuleId = StakingModuleId;
    type Currency = balances::Module<Self>;
    type UnixTime = pallet_timestamp::Module<Self>;
    type CurrencyToVote = CurrencyToVoteHandler;
    type RewardRemainder = ();
    type Event = ();
    type Slash = ();
    type Reward = ();
    type Randomness = TestRandomness;
    type SessionsPerEra = SessionsPerEra;
    type BondingDuration = BondingDuration;
    type MaxGuarantorRewardedPerValidator = MaxGuarantorRewardedPerValidator;
    type SlashDeferDuration = SlashDeferDuration;
    type SlashCancelOrigin = frame_system::EnsureRoot<Self::AccountId>;
    type SessionInterface = Self;
    type SPowerRatio = SPowerRatio;
    type DSMStakingPot = TestStaking;
    type DSMStakingPotDuration = DSMStakingPotDuration;
    type WeightInfo = weight::WeightInfo;
}

pub struct ExtBuilder {
    existential_deposit: u64,
    validator_pool: bool,
    guarantee: bool,
    validator_count: u32,
    minimum_validator_count: u32,
    slash_defer_duration: EraIndex,
    fair: bool,
    num_validators: Option<u32>,
    invulnerables: Vec<u128>,
    own_workload: u128,
    total_workload: u128,
    staking_pot: Balance,
    dsm_staking_payout: Balance
}

impl Default for ExtBuilder {
    fn default() -> Self {
        Self {
            existential_deposit: 1,
            validator_pool: false,
            guarantee: true,
            validator_count: 2,
            minimum_validator_count: 0,
            slash_defer_duration: 0,
            fair: true,
            num_validators: None,
            invulnerables: vec![],
            own_workload: 3000,
            total_workload: 3000,
            staking_pot: 1_000_000_000_000_000_000,
            dsm_staking_payout: 0
        }
    }
}

impl ExtBuilder {
    pub fn existential_deposit(mut self, existential_deposit: u64) -> Self {
        self.existential_deposit = existential_deposit;
        self
    }
    pub fn own_workload(mut self, own_workload: u128) -> Self {
        self.own_workload = own_workload;
        self
    }
    pub fn total_workload(mut self, total_workload: u128) -> Self {
        self.total_workload = total_workload;
        self
    }
    pub fn validator_pool(mut self, validator_pool: bool) -> Self {
        self.validator_pool = validator_pool;
        self
    }
    pub fn guarantee(mut self, guarantee: bool) -> Self {
        self.guarantee = guarantee;
        self
    }
    pub fn validator_count(mut self, count: u32) -> Self {
        self.validator_count = count;
        self
    }
    pub fn minimum_validator_count(mut self, count: u32) -> Self {
        self.minimum_validator_count = count;
        self
    }
    pub fn slash_defer_duration(mut self, eras: EraIndex) -> Self {
        self.slash_defer_duration = eras;
        self
    }
    pub fn fair(mut self, is_fair: bool) -> Self {
        self.fair = is_fair;
        self
    }
    pub fn num_validators(mut self, num_validators: u32) -> Self {
        self.num_validators = Some(num_validators);
        self
    }
    pub fn invulnerables(mut self, invulnerables: Vec<u128>) -> Self {
        self.invulnerables = invulnerables;
        self
    }
    pub fn staking_pot(mut self, amount: Balance) -> Self {
        self.staking_pot = amount;
        self
    }
    pub fn dsm_staking_payout(mut self, amount: Balance) -> Self {
        self.dsm_staking_payout = amount;
        self
    }
    pub fn set_associated_consts(&self) {
        EXISTENTIAL_DEPOSIT.with(|v| *v.borrow_mut() = self.existential_deposit);
        SLASH_DEFER_DURATION.with(|v| *v.borrow_mut() = self.slash_defer_duration);
        OWN_WORKLOAD.with(|v| *v.borrow_mut() = self.own_workload);
        TOTAL_WORKLOAD.with(|v| *v.borrow_mut() = self.total_workload);
        DSM_STAKING_PAYOUT.with(|v| *v.borrow_mut() = self.dsm_staking_payout);
    }
    pub fn build(self) -> sp_io::TestExternalities {
        self.set_associated_consts();
        let mut storage = frame_system::GenesisConfig::default()
            .build_storage::<Test>()
            .unwrap();
        let balance_factor = if self.existential_deposit > 1 { 256 } else { 1 };

        let num_validators = self.num_validators.unwrap_or(self.validator_count);
        let validators = (0..num_validators)
            .map(|x| ((x + 1) * 10 + 1) as u128)
            .collect::<Vec<_>>();

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
                (100, 2000 * balance_factor),
                (101, 2000 * balance_factor),
                // This allow us to have a total_payout different from 0.
                (999, 1_000_000_000_000),
                (Staking::staking_pot(), self.staking_pot)
            ],
        }.assimilate_storage(&mut storage);

        let stake_21 = if self.fair { 1000 } else { 2000 };
        let stake_31 = if self.validator_pool {
            balance_factor * 1000
        } else {
            1
        };
        let status_41 = if self.validator_pool {
            StakerStatus::<AccountId, Balance>::Validator
        } else {
            StakerStatus::<AccountId, Balance>::Idle
        };
        let guaranteed = if self.guarantee {
            vec![(11, 250), (21, 250)]
        } else {
            vec![]
        };

        let _ = GenesisConfig::<Test> {
            stakers: vec![
                // (stash, controller, staked_amount, status)
                (
                    11,
                    10,
                    balance_factor * 1000,
                    StakerStatus::<AccountId, Balance>::Validator,
                ),
                (
                    21,
                    20,
                    stake_21,
                    StakerStatus::<AccountId, Balance>::Validator,
                ),
                (
                    31,
                    30,
                    stake_31,
                    StakerStatus::<AccountId, Balance>::Validator,
                ),
                (41, 40, balance_factor * 1000, status_41),
                // guarantor
                (
                    101,
                    100,
                    balance_factor * 500,
                    StakerStatus::<AccountId, Balance>::Guarantor(guaranteed),
                ),
            ],
            validator_count: self.validator_count,
            minimum_validator_count: self.minimum_validator_count,
            invulnerables: self.invulnerables,
            slash_reward_fraction: Perbill::from_percent(10),
            ..Default::default()
        }
        .assimilate_storage(&mut storage);

        let _ = pallet_session::GenesisConfig::<Test> {
            keys: validators.iter().map(|x| (
                *x,
                *x,
                UintAuthorityId((*x).try_into().unwrap())
            )).collect(),
        }
        .assimilate_storage(&mut storage);

        let _ = swork::GenesisConfig {
            code: vec![],
        }.assimilate_storage(&mut storage);

        let mut ext = sp_io::TestExternalities::from(storage);
        ext.execute_with(|| {
            init_swork_setup();
            let validators = Session::validators();
            SESSION.with(|x| *x.borrow_mut() = (validators.clone(), HashSet::new()));
        });

        ext
    }
}

pub type System = frame_system::Module<Test>;
pub type Balances = balances::Module<Test>;
pub type Session = pallet_session::Module<Test>;
pub type Timestamp = pallet_timestamp::Module<Test>;
pub type Staking = Module<Test>;
pub type Swork = swork::Module<Test>;

pub fn check_exposure_all() {
    // a check per validator to ensure the exposure struct is always sane.
    let era = Staking::current_era().unwrap_or(0);
    ErasStakers::<Test>::iter_prefix_values(era).for_each(|expo| {
        assert_eq!(
            expo.total as u128,
            expo.own as u128 + expo.others.iter().map(|e| e.value as u128).sum::<u128>(),
            "wrong total exposure.",
        );
    })
}

pub fn check_guarantor_all() {
    <Guarantors<Test>>::iter().for_each(|(acc, _)| check_guarantor_exposure(acc));
}

/// Check that for each guarantor: slashable_balance > sum(used_balance)
/// Note: we might not consume all of a guarantor's balance, but we MUST NOT over spend it.
pub fn check_guarantor_exposure(stash: u128) {
    assert_is_stash(stash);
    let mut sum = 0;
    let current_era = Staking::current_era().unwrap_or(0);
    Staking::current_elected()
        .iter()
        .map(|v| Staking::eras_stakers(current_era, v))
        .for_each(|e| {
            e.others
                .iter()
                .filter(|i| i.who == stash)
                .for_each(|i| sum += i.value)
        });
    let guarantor_stake = Staking::slashable_balance_of(&stash);
    // a guarantor cannot over-spend.
    assert!(
        guarantor_stake >= sum,
        "failed: Guarantor({}) stake({}) >= sum divided({})",
        stash,
        guarantor_stake,
        sum,
    );
}

pub fn assert_is_stash(acc: u128) {
    assert!(Staking::bonded(&acc).is_some(), "Not a stash.");
}

pub fn assert_ledger_consistent(stash: u128) {
    assert_is_stash(stash);
    let ledger = Staking::ledger(stash - 1).unwrap();

    let real_total: Balance = ledger
        .unlocking
        .iter()
        .fold(ledger.active, |a, c| a + c.value);
    assert_eq!(real_total, ledger.total);
}

pub fn bond_validator(acc: u128, val: u64) {
    // a = controller
    // a + 1 = stash
    let _ = Balances::make_free_balance_be(&(acc + 1), val);
    assert_ok!(Staking::bond(
        Origin::signed(acc + 1),
        acc,
        val,
        RewardDestination::Controller
    ));
    Staking::upsert_stake_limit(&(acc + 1), u64::max_value());
    assert_ok!(Staking::validate(Origin::signed(acc), ValidatorPrefs::default()));
}

pub fn bond_guarantor(acc: u128, val: u64, targets: Vec<(u128, u64)>) {
    // a = controller
    // a + 1 = stash
    let _ = Balances::make_free_balance_be(&(acc + 1), val);
    assert_ok!(Staking::bond(
        Origin::signed(acc + 1),
        acc,
        val,
        RewardDestination::Controller
    ));
    for target in targets {
        assert_ok!(Staking::guarantee(Origin::signed(acc), target));
    }
}

pub fn advance_session() {
    let current_index = Session::current_index();
    start_session(current_index + 1, false);
}

pub fn start_session(session_index: SessionIndex, with_reward: bool) {
    // Compensate for session delay
    for i in Session::current_index()..session_index {
        Swork::on_initialize(System::block_number());
        Staking::on_finalize(System::block_number());
        System::set_block_number(((i+1)*100).into());
        if with_reward {
            Timestamp::set_timestamp(System::block_number() * 1000);
        }
        Session::on_initialize(System::block_number());
    }

    assert_eq!(Session::current_index(), session_index);
}

pub fn start_era(era_index: EraIndex, with_reward: bool) {
    start_session((era_index * 3).into(), with_reward);
    assert_eq!(Staking::current_era().unwrap(), era_index);
    assert_eq!(Staking::active_era().unwrap().index, era_index);
}

pub fn reward_all_elected() {
    let rewards = <Module<Test>>::current_elected()
        .iter()
        .map(|v| (*v, 1))
        .collect::<Vec<_>>();

    <Module<Test>>::reward_by_ids(rewards)
}

pub fn validator_controllers() -> Vec<AccountId> {
    Session::validators()
        .into_iter()
        .map(|s| Staking::bonded(&s).expect("no controller for validator"))
        .collect()
}

pub fn on_offence_in_era(
    offenders: &[OffenceDetails<
        AccountId,
        pallet_session::historical::IdentificationTuple<Test>,
    >],
    slash_fraction: &[Perbill],
    era: EraIndex,
) {
    let bonded_eras = crate::BondedEras::get();
    for &(bonded_era, start_session) in bonded_eras.iter() {
        if bonded_era == era {
            let _ = Staking::on_offence(offenders, slash_fraction, start_session);
            return;
        } else if bonded_era > era {
            break;
        }
    }

    if Staking::current_era().unwrap_or(0) == era {
        let _ = Staking::on_offence(
            offenders,
            slash_fraction,
            Staking::eras_start_session_index(era).unwrap(),
        );
    } else {
        panic!("cannot slash in era {}", era);
    }
}

pub fn on_offence_now(
    offenders: &[OffenceDetails<
        AccountId,
        pallet_session::historical::IdentificationTuple<Test>,
    >],
    slash_fraction: &[Perbill],
) {
    let now = Staking::current_era().unwrap_or(0);
    on_offence_in_era(offenders, slash_fraction, now)
}

pub fn set_own_workload(own_workload: u128) {
    OWN_WORKLOAD.with(|v| *v.borrow_mut() = own_workload);
}

pub fn set_total_workload(total_workload: u128) {
    TOTAL_WORKLOAD.with(|v| *v.borrow_mut() = total_workload);
}

pub fn start_era_with_new_workloads(era_index: EraIndex, with_reward: bool, own_workload: u128, total_workload: u128) {
    set_own_workload(own_workload);
    set_total_workload(total_workload);
    start_session((era_index * 3).into(), with_reward);
    assert_eq!(Staking::current_era().unwrap_or(0), era_index);
}

pub fn payout_all_stakers(era_index: EraIndex) {
    Staking::reward_stakers(Origin::signed(10), 11, era_index).unwrap_or_default();
    Staking::reward_stakers(Origin::signed(10), 21, era_index).unwrap_or_default();
    Staking::reward_stakers(Origin::signed(10), 31, era_index).unwrap_or_default();
    Staking::reward_stakers(Origin::signed(10), 41, era_index).unwrap_or_default();
}

fn init_swork_setup() {
    let identities: Vec<u128> = vec![10, 20, 30, 40, 2, 60, 50, 70, 4, 6, 100];
    let id_map: Vec<(u128, Vec<u8>)> = identities.iter().map(|account| (*account, account.to_be_bytes().to_vec())).collect();
    let code: Vec<u8> = vec![];

    for (id, pk) in id_map {
        <swork::PubKeys>::insert(pk.clone(), swork::PKInfo {
            code: code.clone(),
            anchor: Some(pk.clone())
        });
        <swork::Identities<Test>>::insert(id, swork::Identity {
            anchor: pk.clone(),
            group: None
        });
        <swork::WorkReports>::insert(pk.clone(), swork::WorkReport {
            report_slot: 0,
            used: 0,
            free: 20000000000000,
            reported_files_size: 0,
            reported_srd_root: vec![],
            reported_files_root: vec![]
        });
    }
}
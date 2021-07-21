// Copyright (C) 2019-2021 Crust Network Technologies Ltd.
// This file is part of Crust.

use super::*;
use crate as market;

use frame_support::{
    parameter_types, assert_ok,
    weights::constants::RocksDbWeight,
    traits::{OnFinalize, OnInitialize, Get}
};
// use sp_core::H256;
pub use sp_core::{crypto::{AccountId32, Ss58Codec}, H256};
use sp_runtime::{
    testing::Header, DispatchError,
    traits::{BlakeTwo256, IdentityLookup},
    Perbill,
};
use balances::{AccountData, NegativeImbalance};
pub use primitives::{traits::BenefitInterface, *};
use swork::{PKInfo, Identity, NegativeImbalanceOf};
pub use std::{cell::RefCell, collections::HashMap, borrow::Borrow, iter::FromIterator};

pub type AccountId = AccountId32;
pub type Balance = u64;

pub const ALICE: AccountId32 = AccountId32::new([1u8; 32]);
pub const BOB: AccountId32 = AccountId32::new([2u8; 32]);
pub const CHARLIE: AccountId32 = AccountId32::new([3u8; 32]);
pub const EVE: AccountId32 = AccountId32::new([4u8; 32]);
pub const MERCHANT: AccountId32 = AccountId32::new([5u8; 32]);
pub const DAVE: AccountId32 = AccountId32::new([6u8; 32]);
pub const FERDIE: AccountId32 = AccountId32::new([7u8; 32]);
pub const ZIKUN: AccountId32 = AccountId32::new([8u8; 32]);

#[derive(Debug, PartialEq, Eq, Clone, Encode, Decode, Default)]
pub struct MockMerchantLedger {
    pub collateral: Balance,
    pub reward: Balance
}

thread_local! {
    static EXISTENTIAL_DEPOSIT: RefCell<u64> = RefCell::new(1);
    static LEGAL_CODE: Vec<u8> = hex::decode("781b537d3dcef39dec7b8bce6fdfcd032d8d846640e9b5598b4a9f627188a908").unwrap();
    static MERCHANT_LEDGERS: RefCell<HashMap<AccountId, MockMerchantLedger>> = RefCell::new(Default::default());
    static DISCOUNT_RATIO: RefCell<(u64, u64)> = RefCell::new((0, 0));
}

pub struct ExistentialDeposit;
impl Get<u64> for ExistentialDeposit {
    fn get() -> u64 {
        EXISTENTIAL_DEPOSIT.with(|v| *v.borrow())
    }
}

pub struct DiscountRatio;
impl Get<(u64, u64)> for DiscountRatio {
    fn get() -> (u64, u64) {
        DISCOUNT_RATIO.with(|v| *v.borrow())
    }
}

pub struct MerchantLedgers;
impl Get<HashMap<AccountId, MockMerchantLedger>> for MerchantLedgers {
    fn get() -> HashMap<AccountId, MockMerchantLedger> {
        MERCHANT_LEDGERS.with(|map| map.borrow().clone())
    }
}
impl MerchantLedgers {
    fn set_reward(who: &AccountId, reward: Balance) {
        let collateral = MERCHANT_LEDGERS.with(|map_ref| {
            let map = map_ref.borrow();
            (map.get(who).unwrap_or(&mut MockMerchantLedger { collateral: 0, reward: 0})).collateral
        });
        MERCHANT_LEDGERS.with(|map_ref| {
            let mut map = map_ref.borrow_mut();
            map.insert(who.clone(), MockMerchantLedger { collateral, reward});
        })
    }

    fn set_collateral(who: &AccountId, collateral: Balance) {
        let reward = MERCHANT_LEDGERS.with(|map_ref| {
            let map = map_ref.borrow();
            (map.get(who).unwrap_or(&mut MockMerchantLedger { collateral: 0, reward: 0})).reward
        });
        MERCHANT_LEDGERS.with(|map_ref| {
            let mut map = map_ref.borrow_mut();
            map.insert(who.clone(), MockMerchantLedger { collateral, reward});
        })
    }
}

pub struct ReportWorksInfo {
    pub curr_pk: SworkerPubKey,
    pub prev_pk: SworkerPubKey,
    pub block_number: u64,
    pub block_hash: Vec<u8>,
    pub free: u64,
    pub used: u64,
    pub srd_root: MerkleRoot,
    pub files_root: MerkleRoot,
    pub added_files: Vec<(MerkleRoot, u64, u64)>,
    pub deleted_files: Vec<(MerkleRoot, u64, u64)>,
    pub sig: SworkerSignature
}

pub struct LegalCode;
impl Get<Vec<u8>> for LegalCode {
    fn get() -> Vec<u8> {
        LEGAL_CODE.with(|code| code.clone())
    }
}

parameter_types! {
    pub const BlockHashCount: u64 = 250;
}

impl system::Config for Test {
    type BaseCallFilter = ();
    type BlockWeights = ();
    type BlockLength = ();
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
    type DbWeight = RocksDbWeight;
    type Version = ();
    type PalletInfo = PalletInfo;
    type AccountData = AccountData<Balance>;
    type OnNewAccount = ();
    type OnKilledAccount = ();
    type SystemWeightInfo = ();
    type SS58Prefix = ();
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

pub struct TestBenefitInterface;

impl BenefitInterface<AccountId, BalanceOf<Test>, NegativeImbalanceOf<Test>> for TestBenefitInterface {
    fn update_era_benefit(_: EraIndex, _: BalanceOf<Test>) -> BalanceOf<Test> {
        Zero::zero()
    }

    fn update_reward(who: &AccountId, value: BalanceOf<Test>) {
        MerchantLedgers::set_reward(who, value);
    }

    fn maybe_reduce_fee(_: &AccountId, _: BalanceOf<Test>, _: WithdrawReasons) -> Result<NegativeImbalance<Test>, DispatchError> {
        Ok(NegativeImbalance::new(0))
    }

    fn maybe_free_count(_: &AccountId) -> bool {
        return true;
    }

    fn get_collateral_and_reward(who: &AccountId) -> (BalanceOf<Test>, BalanceOf<Test>) {
        let ledgers = MerchantLedgers::get();
        let merchant_ledger = ledgers.get(who).unwrap_or( &MockMerchantLedger { collateral: 0, reward: 0});
        (BalanceOf::<Test>::saturated_from(merchant_ledger.collateral), BalanceOf::<Test>::saturated_from(merchant_ledger.reward))
    }

    fn get_market_funds_ratio(_: &AccountId) -> Perbill {
        let (active_funds, total_funds) = DiscountRatio::get();
        Perbill::from_rational_approximation(active_funds, total_funds)
    }
}

parameter_types! {
    pub const PunishmentSlots: u32 = 1;
    pub const MaxGroupSize: u32 = 100;
}

impl swork::Config for Test {
    type Currency = Balances;
    type Event = ();
    type PunishmentSlots = PunishmentSlots;
    type Works = ();
    type MarketInterface = Market;
    type MaxGroupSize = MaxGroupSize;
    type BenefitInterface = TestBenefitInterface;
    type WeightInfo = swork::weight::WeightInfo<Test>;
}

parameter_types! {
    /// Unit is pico
    pub const MarketModuleId: ModuleId = ModuleId(*b"crmarket");
    pub const FileDuration: BlockNumber = 1000;
    pub const LiquidityDuration: BlockNumber = 1000;
    pub const FileReplica: u32 = 4;
    pub const InitFileByteFee: Balance = 1000; // Need align with FileDuration and FileBaseReplica
    pub const InitFileKeysCountFee: Balance = 0;
    pub const StorageReferenceRatio: (u128, u128) = (1, 2);
    pub const StorageIncreaseRatio: Perbill = Perbill::from_percent(1);
    pub const StorageDecreaseRatio: Perbill = Perbill::from_percent(1);
    pub const StakingRatio: Perbill = Perbill::from_percent(72);
    pub const StorageRatio: Perbill = Perbill::from_percent(18);
    pub const MaximumFileSize: u64 = 137_438_953_472; // 128G = 128 * 1024 * 1024 * 1024
    pub const RenewRewardRatio: Perbill = Perbill::from_percent(5);
}

impl Config for Test {
    type ModuleId = MarketModuleId;
    type Currency = balances::Module<Self>;
    type SworkerInterface = Swork;
    type BenefitInterface = TestBenefitInterface;
    type Event = ();
    type FileDuration = FileDuration;
    type LiquidityDuration = LiquidityDuration;
    type FileReplica = FileReplica;
    type InitFileByteFee = InitFileByteFee;
    type InitFileKeysCountFee = InitFileKeysCountFee;
    type StorageReferenceRatio = StorageReferenceRatio;
    type StorageIncreaseRatio = StorageIncreaseRatio;
    type StorageDecreaseRatio = StorageDecreaseRatio;
    type StakingRatio = StakingRatio;
    type StorageRatio = StorageRatio;
    type MaximumFileSize = MaximumFileSize;
    type WeightInfo = weight::WeightInfo<Test>;
    type RenewRewardRatio = RenewRewardRatio;
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
		Swork: swork::{Module, Call, Storage, Event<T>, Config<T>},
		Market: market::{Module, Call, Storage, Event<T>, Config},
	}
);

pub fn new_test_ext() -> sp_io::TestExternalities {
    let mut t = system::GenesisConfig::default()
    .build_storage::<Test>()
    .unwrap();

    let _ = swork::GenesisConfig::<Test> {
        init_codes: vec![(LegalCode::get(), 100000)],
    }.assimilate_storage(&mut t);

    let mut ext: sp_io::TestExternalities = t.into();
    ext.execute_with(|| {
        init_swork_setup();
        assert_ok!(Market::set_enable_market(Origin::root(), true));
        assert_ok!(Market::set_base_fee(Origin::root(), 1000));
    });

    ext
}

pub fn set_discount_ratio(active_funds: Balance, total_funds: Balance) {
    DISCOUNT_RATIO.with(|v| *v.borrow_mut() = (active_funds, total_funds));
}

pub fn init_swork_setup() {
    let pks = vec![hex::decode("11").unwrap(), hex::decode("22").unwrap(), hex::decode("33").unwrap(), hex::decode("44").unwrap()];
    let whos = vec![ALICE, BOB, CHARLIE, DAVE];
    let frees: Vec<u64> = vec![0, 50, 50, 200];
    let code = LegalCode::get();
    for ((pk, who), free) in pks.iter().zip(whos.iter()).zip(frees.iter()) {
        <swork::PubKeys>::insert(pk.clone(), PKInfo {
            code: code.clone(),
            anchor: Some(pk.clone())
        });
        <swork::Identities<Test>>::insert(who, Identity {
            anchor: pk.clone(),
            punishment_deadline: 0,
            group: None
        });
        <swork::WorkReports>::insert(pk.clone(), swork::WorkReport{
            report_slot: 0,
            spower: 0,
            free: *free,
            reported_files_size: 0,
            reported_srd_root: vec![],
            reported_files_root: vec![]
        });
    }
}

// fake for report_works
pub fn add_who_into_replica(cid: &MerkleRoot, reported_size: u64, who: AccountId, anchor: SworkerAnchor, created_at: Option<u32>, maybe_members: Option<BTreeSet<AccountId>>) -> u64 {
    Market::upsert_replica(&who, cid, reported_size, &anchor, created_at.unwrap_or(TryInto::<u32>::try_into(System::block_number()).ok().unwrap()), &maybe_members).0
}

pub fn legal_work_report_with_added_files() -> ReportWorksInfo {
    let curr_pk = hex::decode("7137dc62f9a8ba82fae62f5306981b7b39a82ff0e730739c9d8998eec0ab37f02e734e65fc518df5e6263d657faac48242ec1972b5dca058d9b78a6844c7a19c").unwrap();
    let prev_pk = hex::decode("").unwrap();
    let block_number: u64 = 300;
    let block_hash = hex::decode("0000000000000000000000000000000000000000000000000000000000000000").unwrap();
    let free: u64 = 4294967296;
    let used: u64 = 402868224;
    let added_files: Vec<(Vec<u8>, u64, u64)> = [
        ("QmdwgqZy1MZBfWPi7GcxVsYgJEtmvHg6rsLzbCej3tf3oF".as_bytes().to_vec(), 134289408, 303),
        ("QmdwgqZy1MZBfWPi7GcxVsYgJEtmvHg6rsLzbCej3tf3oH".as_bytes().to_vec(), 268578816, 303)
    ].to_vec();
    let deleted_files: Vec<(Vec<u8>, u64, u64)> = vec![];
    let files_root = hex::decode("11").unwrap();
    let srd_root = hex::decode("00").unwrap();
    let sig = hex::decode("d254eb42c15384b8019d676b1cf83c11a6cf0121c47381cabfea44844421cc231e244f83c2c4af3140880c534b672196b147e8b63708c871cc87f1230dbca12f").unwrap();

    ReportWorksInfo {
        curr_pk,
        prev_pk,
        block_number,
        block_hash,
        free,
        used,
        srd_root,
        files_root,
        added_files,
        deleted_files,
        sig
    }
}

pub fn register(pk: &SworkerPubKey, code: SworkerCode) {
    <swork::PubKeys>::insert(pk.clone(), PKInfo {
        code: code,
        anchor: None
    });
}

pub fn add_collateral(who: &AccountId, collateral: Balance) {
    MerchantLedgers::set_collateral(who, collateral);
}

pub fn add_reward(who: &AccountId, reward: Balance) {
    MerchantLedgers::set_reward(who, reward);
}

pub fn merchant_ledgers(who: &AccountId) -> MockMerchantLedger {
    MerchantLedgers::get().get(who).unwrap().clone()
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

pub fn update_spower_info() {
    Market::on_initialize(93);
}

pub fn mock_bond_owner(member: &AccountId, owner: &AccountId) {
    <swork::Identities<Test>>::mutate(member, |maybe_identity| match *maybe_identity {
        Some(ref mut identity) => identity.group = Some(owner.clone()),
        None => *maybe_identity = Some(Identity {
            anchor: vec![],
            punishment_deadline: 0,
            group: Some(owner.clone())
        })
    });
}
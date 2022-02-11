// Copyright (C) 2019-2021 Crust Network Technologies Ltd.
// This file is part of Crust.

use super::*;
use crate as claims;

use sp_core::H256;
use frame_support::parameter_types;
use sp_runtime::{
    traits::{BlakeTwo256, IdentityLookup}, testing::Header,
};
use frame_system::EnsureRoot;
use hex_literal::hex;
use primitives::*;

parameter_types! {
    pub const BlockHashCount: u32 = 250;
}
impl frame_system::Config for Test {
    type BaseCallFilter = ();
    type BlockWeights = ();
    type BlockLength = ();
    type Origin = Origin;
    type Call = Call;
    type Index = u64;
    type BlockNumber = u64;
    type Hash = H256;
    type Hashing = BlakeTwo256;
    type AccountId = u64;
    type Lookup = IdentityLookup<u64>;
    type Header = Header;
    type Event = ();
    type BlockHashCount = BlockHashCount;
    type DbWeight = ();
    type Version = ();
    type PalletInfo = PalletInfo;
    type AccountData = balances::AccountData<u64>;
    type OnNewAccount = ();
    type OnKilledAccount = ();
    type SystemWeightInfo = ();
    type SS58Prefix = ();
}

parameter_types! {
    pub const ExistentialDeposit: u64 = 1;
}

impl balances::Config for Test {
    type Balance = u64;
    type DustRemoval = ();
    type Event = ();
    type ExistentialDeposit = ExistentialDeposit;
    type AccountStore = System;
    type WeightInfo = ();
    type MaxLocks = ();
}

parameter_types! {
    pub const UnlockPeriod: BlockNumber = 1000;
}

impl locks::Config for Test {
    type Event = ();
    type Currency = Balances;
    type UnlockPeriod = UnlockPeriod;
    type WeightInfo = locks::weight::WeightInfo<Test>;
}

parameter_types!{
    pub const ClaimModuleId: ModuleId = ModuleId(*b"crclaims");
    pub Prefix: &'static [u8] = b"Pay RUSTs to the TEST account:";
}
impl Config for Test {
    type ModuleId = ClaimModuleId;
    type Event = ();
    type Currency = Balances;
    type Prefix = Prefix;
    type LocksInterface = CrustLocks;
    type CRU18Origin = EnsureRoot<u64>;
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
        CrustClaims: claims::{Module, Call, Storage, Event<T>, ValidateUnsigned},
        CrustLocks: locks::{Module, Call, Storage, Event<T>, Config<T>},
	}
);

// This function basically just builds a genesis storage key/value store according to
// our desired mockup.
pub fn new_test_ext() -> sp_io::TestExternalities {
    frame_system::GenesisConfig::default().build_storage::<Test>().unwrap().into()
}

pub fn get_legal_tx_hash1() -> EthereumTxHash {
    EthereumTxHash(hex!["6543c650337d70c5686e995e47f26c3136218c5d703b190c60a6ee70a5004324"])
}

pub fn get_legal_tx_hash2() -> EthereumTxHash {
    EthereumTxHash(hex!["549aebae25688ef1a391f886edfa90f34fc92aed20d9e8d20b7bbabefd343b3e"])
}

pub fn get_legal_eth_addr() -> EthereumAddress {
    EthereumAddress(hex!["110eA27b24c9E973098A69dd93cf831b7896b81f"])
}

pub fn get_legal_eth_sig() -> EcdsaSignature {
    // `110eA27b24c9E973098A69dd93cf831b7896b81f`'s sig
    // data: Pay RUSTs to the TEST account:01000000000000006543c650337d70c5686e995e47f26c3136218c5d703b190c60a6ee70a5004324
    EcdsaSignature(hex!["41c350cf489a4ea441948f22de088a4fcd4bcb0a726d27fe6cf4e9bef07a27fd6dc4c7a8645c5270c1f249a1c22c512b39be83d972d733660a58d75cfd5de20b1c"])
}

pub fn get_another_account_eth_sig() -> EcdsaSignature {
    // `0xba0d7d9d1cea3276a6e9082026b80f8e75350306`'s sig
    // data: Pay RUSTs to the TEST account:01000000000000006543c650337d70c5686e995e47f26c3136218c5d703b190c60a6ee70a5004324
    EcdsaSignature(hex!["f75fd11f33029fe4a39ea3f0f85dfa188138599e2e39c64a06650729135691707d549f2798695b4ea367929f128015c6bb3362ed9e9a464fabae52fc581edc021c"])
}

pub fn get_wrong_msg_eth_sig() -> EcdsaSignature {
    // `0xba0d7d9d1cea3276a6e9082026b80f8e75350306`'s sig
    // data: wrong message
    EcdsaSignature(hex!["132ffc29ee017b5affa39367b31b66ff47d8db402dbee9c900128728c9b60096401f3126c6748c4f19bb262e80ab5f5d759dbe69c05d84464def96afe6d699ea1b"])

}
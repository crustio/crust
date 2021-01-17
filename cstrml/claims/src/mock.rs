// Copyright (C) 2019-2021 Crust Network Technologies Ltd.
// This file is part of Crust.

use super::*;

use sp_core::H256;
use frame_support::{impl_outer_dispatch, impl_outer_origin, parameter_types};
use sp_runtime::{
    traits::{BlakeTwo256, IdentityLookup}, testing::Header,
};
use hex_literal::hex;

impl_outer_origin! {
		pub enum Origin for Test {}
	}

impl_outer_dispatch! {
    pub enum Call for Test where origin: Origin {
        claims::CrustClaims,
    }
}

// For testing the module, we construct most of a mock runtime. This means
// first constructing a configuration type (`Test`) which `impl`s each of the
// configuration traits of modules we want to use.
#[derive(Clone, Eq, PartialEq)]
pub struct Test;
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
    type PalletInfo = ();
    type AccountData = balances::AccountData<u64>;
    type OnNewAccount = ();
    type OnKilledAccount = Balances;
    type SystemWeightInfo = ();
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

parameter_types!{
    pub Prefix: &'static [u8] = b"Pay RUSTs to the TEST account:";
}
impl Config for Test {
    type Event = ();
    type Currency = Balances;
    type Prefix = Prefix;
}
pub type System = frame_system::Module<Test>;
pub type Balances = balances::Module<Test>;
pub type CrustClaims = Module<Test>;

// This function basically just builds a genesis storage key/value store according to
// our desired mockup.
pub fn new_test_ext() -> sp_io::TestExternalities {
    frame_system::GenesisConfig::default().build_storage::<Test>().unwrap().into()
}

pub fn get_legal_tx_hash() -> EthereumTxHash {
    EthereumTxHash(hex!["6543c650337d70c5686e995e47f26c3136218c5d703b190c60a6ee70a5004324"])
}

pub fn get_legal_eth_addr() -> EthereumAddress {
    EthereumAddress(hex!["110eA27b24c9E973098A69dd93cf831b7896b81f"])
}

pub fn get_legal_eth_sig() -> EcdsaSignature {
    // `110eA27b24c9E973098A69dd93cf831b7896b81f`'s sig
    // data: Pay RUSTs to the TEST account:0100000000000000
    EcdsaSignature(hex!["87f3db67c86ac43b8e1e763b0164333f0dfe0c65917ea032046c99e21cedd0d826ccf0a405e6308ce83a11cff2b26c26c372438ef09c3beb688413ad7c3171da1c"])
}

pub fn get_another_account_eth_sig() -> EcdsaSignature {
    // `0xba0d7d9d1cea3276a6e9082026b80f8e75350306`'s sig
    // data: Pay RUSTs to the TEST account:0100000000000000
    EcdsaSignature(hex!["132ffc29ee017b5affa39367b31b66ff47d8db402dbee9c900128728c9b60096401f3126c6748c4f19bb262e80ab5f5d759dbe69c05d84464def96afe6d699ea1b"])
}

pub fn get_wrong_msg_eth_sig() -> EcdsaSignature {
    // `0xba0d7d9d1cea3276a6e9082026b80f8e75350306`'s sig
    // data: wrong message
    EcdsaSignature(hex!["132ffc29ee017b5affa39367b31b66ff47d8db402dbee9c900128728c9b60096401f3126c6748c4f19bb262e80ab5f5d759dbe69c05d84464def96afe6d699ea1b"])

}
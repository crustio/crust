// Copyright (C) 2019-2021 Crust Network Technologies Ltd.
// This file is part of Crust.

use frame_support::traits::{LockableCurrency, WithdrawReasons};
use crate::{SworkerAnchor, MerkleRoot, BlockNumber, EraIndex};
use sp_std::collections::btree_set::BTreeSet;
use sp_runtime::DispatchError;

/// A currency whose accounts can have liquidity restrictions.
pub trait UsableCurrency<AccountId>: LockableCurrency<AccountId> {
	fn usable_balance(who: &AccountId) -> Self::Balance;
}

/// Means for interacting with a specialized version of the `swork` trait.
pub trait SworkerInterface<AccountId> {
	// Check whether work report was reported in the last report slot according to given block number
	fn is_wr_reported(anchor: &SworkerAnchor, bn: BlockNumber) -> bool;
	// Update the storage_power value in anchor's work report
	fn update_storage_power(anchor: &SworkerAnchor, decreased_storage_power: u64, increased_storage_power: u64);
    // Check whether the who and anchor is consistent with current status
	fn check_anchor(who: &AccountId, anchor: &SworkerAnchor) -> bool;
	// Get total report files size and srd space
	fn get_total_capacity() -> u128;
}

/// Means for interacting with a specialized version of the `market` trait.
pub trait MarketInterface<AccountId, Balance> {
	// used for `added_files`
	// return real storage power of this file
	fn upsert_replica(who: &AccountId, cid: &MerkleRoot, reported_file_size: u64, anchor: &SworkerAnchor, valid_at: BlockNumber, members: &Option<BTreeSet<AccountId>>) -> u64;
	// used for `delete_files`
	// return real storage power of this file
	fn delete_replica(who: &AccountId, cid: &MerkleRoot, anchor: &SworkerAnchor) -> u64;
	// used for distribute market staking payout
	fn withdraw_staking_pot() -> Balance;
}

pub trait BenefitInterface<AccountId, Balance, NegativeImbalance> {
	fn update_era_benefit(next_era: EraIndex, total_benefits: Balance) -> Balance;

	fn maybe_reduce_fee(who: &AccountId, fee: Balance, reasons: WithdrawReasons) -> Result<NegativeImbalance, DispatchError>;

	fn maybe_free_count(who: &AccountId) -> bool;
}
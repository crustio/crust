// Copyright (C) 2019-2021 Crust Network Technologies Ltd.
// This file is part of Crust.

use frame_support::traits::LockableCurrency;
use crate::{SworkerAnchor, MerkleRoot, BlockNumber};
use sp_std::collections::btree_set::BTreeSet;

/// A currency whose accounts can have liquidity restrictions.
pub trait TransferrableCurrency<AccountId>: LockableCurrency<AccountId> {
	fn transfer_balance(who: &AccountId) -> Self::Balance;
}

/// Means for interacting with a specialized version of the `swork` trait.
pub trait SworkerInterface<AccountId> {
	// Check whether work report was reported in the last report slot according to given block number
	fn is_wr_reported(anchor: &SworkerAnchor, bn: BlockNumber) -> bool;
	// Decrease the used value in anchor's work report
	fn decrease_used(anchor: &SworkerAnchor, used: u64);
    // Check whether the who and anchor is consistent with current status
	fn check_anchor(who: &AccountId, anchor: &SworkerAnchor) -> bool;
	// Get total used and free space
	fn get_total_capacity() -> u128;
}

/// Means for interacting with a specialized version of the `market` trait.
pub trait MarketInterface<AccountId, Balance> {
	// used for `added_files`
	// return is_added
	fn upsert_replicas(who: &AccountId, cid: &MerkleRoot, reported_file_size: u64, anchor: &SworkerAnchor, valid_at: BlockNumber, members: &Option<BTreeSet<AccountId>>) -> u64;
	// used for `delete_files`
	// return is_deleted
	fn delete_replicas(who: &AccountId, cid: &MerkleRoot, anchor: &SworkerAnchor, curr_bn: BlockNumber) -> u64;
	// used for distribute market staking payout
	fn withdraw_staking_pot() -> Balance;
}

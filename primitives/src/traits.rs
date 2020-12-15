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
	fn get_free_plus_used() -> u128;
}

/// Means for interacting with a specialized version of the `market` trait.
pub trait MarketInterface<AccountId> {
	// used for `added_files`
	// return is_added
	fn upsert_payouts(who: &AccountId, cid: &MerkleRoot, anchor: &SworkerAnchor, curr_bn: BlockNumber, is_counted: bool) -> bool;
	// used for `delete_files`
	// return is_deleted
	fn delete_payouts(who: &AccountId, cid: &MerkleRoot, anchor: &SworkerAnchor, curr_bn: BlockNumber) -> bool;
	// check group used
	fn check_duplicate_in_group(cid: &MerkleRoot, members: &BTreeSet<AccountId>) -> bool;
}

impl<AId> MarketInterface<AId> for () {
	fn upsert_payouts(_: &AId, _: &MerkleRoot, _: &SworkerAnchor, _: BlockNumber, _: bool) -> bool { false }

	fn delete_payouts(_: &AId, _: &MerkleRoot, _: &SworkerAnchor, _: BlockNumber) -> bool { false }

	fn check_duplicate_in_group(_: &MerkleRoot, _: &BTreeSet<AId>) -> bool { false }
}
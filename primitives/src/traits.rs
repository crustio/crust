// Copyright (C) 2019-2021 Crust Network Technologies Ltd.
// This file is part of Crust.

use frame_support::traits::{LockableCurrency, WithdrawReasons};
use crate::{BlockNumber, EraIndex, MerkleRoot, ReportSlot, SworkerAnchor};
use sp_runtime::{DispatchError, Perbill};
use sp_std::collections::btree_map::BTreeMap;
use sp_std::vec::Vec;
use sp_std::collections::btree_map::BTreeMap;

/// A currency whose accounts can have liquidity restrictions.
pub trait UsableCurrency<AccountId>: LockableCurrency<AccountId> {
	fn usable_balance(who: &AccountId) -> Self::Balance;

	fn frozen_balance(who: &AccountId) -> Self::Balance;
}

/// Means for interacting with a specialized version of the `swork` trait.
pub trait SworkerInterface<AccountId> {
	// Check whether work report was reported in the last report slot according to given block number
	fn is_wr_reported(anchor: &SworkerAnchor, bn: BlockNumber) -> bool;
	// Update the used value in anchor's work report
	fn update_spower(anchor: &SworkerAnchor, decreased_used: u64, increased_used: u64);
    // Check whether the who and anchor is consistent with current status
	fn check_anchor(who: &AccountId, anchor: &SworkerAnchor) -> bool;
	// Get total used and free space
	fn get_files_size_and_free_space() -> (u128, u128);
	// Get the added files count in the past one period and clear the record
	fn get_added_files_count_and_clear_record() -> u32;
	// Get owner of this member
	fn get_owner(who: &AccountId) -> Option<AccountId>;
	// Update the last processed block of work reports
	fn update_last_processed_block_of_work_reports(last_processed_block: BlockNumber);
	// Update changed spower of sworkers
	fn update_sworkers_changed_spower(sworker_spower_changed_map: &BTreeMap<SworkerAnchor, i64>);
	// Update illegal file replicas count
	fn update_illegal_file_replicas_count(illegal_file_replicas_map: &BTreeMap<ReportSlot, u32>);
}

/// Means for interacting with a specialized version of the `market` trait.
pub trait MarketInterface<AccountId, Balance> {
	// used for distribute market staking payout
	fn withdraw_staking_pot() -> Balance;
	// Update files spower in market::FilesV2
	fn update_files_spower(changed_files: &Vec<(MerkleRoot, u64, Vec<(AccountId, AccountId, SworkerAnchor, Option<BlockNumber>)>)>);
}

pub trait BenefitInterface<AccountId, Balance, NegativeImbalance> {
	fn update_era_benefit(next_era: EraIndex, total_benefits: Balance) -> Balance;

	fn update_reward(who: &AccountId, value: Balance);

	fn maybe_reduce_fee(who: &AccountId, fee: Balance, reasons: WithdrawReasons) -> Result<NegativeImbalance, DispatchError>;

	fn maybe_free_count(who: &AccountId) -> bool;

	fn get_collateral_and_reward(who: &AccountId) -> (Balance, Balance);

	fn get_market_funds_ratio(who: &AccountId) -> Perbill;
}

pub trait LocksInterface<AccountId, Balance> {
	fn create_cru18_lock(who: &AccountId, value: Balance);
}
// Copyright 2021 ChainSafe Systems
// SPDX-License-Identifier: GPL-3.0-only

#![deny(warnings)]

use codec::{
    Decode,
    Encode,
    MaxEncodedLen,
};
use frame_support::pallet_prelude::*;
use scale_info::TypeInfo;
use sp_std::prelude::*;
use sp_core::U256;

pub type ChainId = u8;
pub type DepositNonce = u64;
pub type ResourceId = [u8; 32];

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub enum ProposalStatus {
    Initiated,
    Approved,
    Rejected,
}

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub struct ProposalVotes<AccountId, BlockNumber> {
    pub votes_for: Vec<AccountId>,
    pub votes_against: Vec<AccountId>,
    pub status: ProposalStatus,
    pub expiry: BlockNumber,
}

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, TypeInfo)]
pub enum BridgeEvent {
	FungibleTransfer(u8, u64, [u8; 32], U256, Vec<u8>),
	NonFungibleTransfer(
		u8,
		u64,
		[u8; 32],
		Vec<u8>,
		Vec<u8>,
		Vec<u8>,
	),
	GenericTransfer(u8, u64, [u8; 32], Vec<u8>),
}

impl<AccountId, BlockNumber> Default for ProposalVotes<AccountId, BlockNumber>
where
    BlockNumber: Default,
{
    fn default() -> Self {
        Self {
            votes_for: vec![],
            votes_against: vec![],
            status: ProposalStatus::Initiated,
            expiry: BlockNumber::default(),
        }
    }
}

impl<AccountId, BlockNumber> ProposalVotes<AccountId, BlockNumber>
where
    AccountId: PartialEq,
    BlockNumber: PartialOrd,
{
    /// Attempts to mark the proposal as approve or rejected.
    /// Returns true if the status changes from active.
    pub(crate) fn try_to_complete(
        &mut self,
        threshold: u32,
        total: u32,
    ) -> ProposalStatus {
        if self.votes_for.len() >= threshold as usize {
            self.status = ProposalStatus::Approved;
            ProposalStatus::Approved
        } else if total >= threshold
            && (self.votes_against.len() as u32).saturating_add(threshold)
                > total
        {
            self.status = ProposalStatus::Rejected;
            ProposalStatus::Rejected
        } else {
            ProposalStatus::Initiated
        }
    }

    /// Returns true if the proposal has been rejected or approved, otherwise false.
    pub(crate) fn is_complete(&self) -> bool {
        self.status != ProposalStatus::Initiated
    }

    /// Returns true if the `who` has voted for or against the proposal
    pub(crate) fn has_voted(&self, who: &AccountId) -> bool {
        self.votes_for.contains(&who) || self.votes_against.contains(&who)
    }

    /// Returns true if the expiry time has been reached
    pub(crate) fn is_expired(&self, now: BlockNumber) -> bool {
        self.expiry <= now
    }
}

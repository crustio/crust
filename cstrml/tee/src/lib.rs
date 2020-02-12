//! The Substrate Node runtime. This can be compiled with `#[no_std]`, ready for Wasm.

#![cfg_attr(not(feature = "std"), no_std)]

#![feature(option_result_contains)]

use frame_support::{decl_module, decl_storage, decl_event, ensure, dispatch::DispatchResult};
use system::ensure_signed;
use sp_std::{vec::Vec, str};
use sp_std::convert::TryInto;
use codec::{Encode, Decode};

#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};

// Crust runtime modules
use cstrml_staking as staking;

/// Provides crypto and other std functions by implementing `runtime_interface`
pub mod api;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

/// Define TEE basic elements
type PubKey = Vec<u8>;
type Signature = Vec<u8>;
type MerkleRoot = Vec<u8>;

#[derive(Debug, PartialEq, Eq, Clone, Encode, Decode, Default)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct Identity<T> {
    pub pub_key: PubKey,
    pub account_id: T,
    pub validator_pub_key: PubKey,
    pub validator_account_id: T,
    pub sig: Signature,
}

#[derive(Debug, PartialEq, Eq, Clone, Encode, Decode, Default)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
// TODO: change block_number & block_hash to standard data type
pub struct WorkReport{
    pub pub_key: PubKey,
    pub block_number: u64,
    pub block_hash: Vec<u8>,
    pub empty_root: MerkleRoot,
    pub empty_workload: u64,
    pub meaningful_workload: u64,
    pub sig: Signature,
}

/// The module's configuration trait.
pub trait Trait: system::Trait + staking::Trait {
    /// The overarching event type.
    type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
}

// This module's storage items.
decl_storage! {
	trait Store for Module<T: Trait> as Tee {
		pub TeeIdentities get(tee_identities) config(): map T::AccountId => Option<Identity<T::AccountId>>;
		pub WorkReports get(work_reports): map T::AccountId => Option<WorkReport>;
	}
}

// The module's dispatchable functions.
decl_module! {
	/// The module declaration.
	pub struct Module<T: Trait> for enum Call where origin: T::Origin {
		// Initializing events
		// this is needed only if you are using events in your module
		fn deposit_event() = default;

		pub fn register_identity(origin, identity: Identity<T::AccountId>) -> DispatchResult {
		    // TODO: add account_id <-> tee_pub_key validation
		    let who = ensure_signed(origin)?;

            let applier = &identity.account_id;
            let validator = &identity.validator_account_id;
            let applier_pk = &identity.pub_key;
            let validator_pk = &identity.validator_pub_key;

            // 1. Ensure who is applier
            ensure!(&who == applier, "Tee applier must be the extrinsic sender");

            // 2. applier cannot be validator
            ensure!(&applier != &validator, "You cannot verify yourself");
            // TODO: Add pub key verify
//            ensure!(&applier_pk != &validator_pk, "You cannot verify yourself");

            // 3. v_account_id should been validated before
            ensure!(<TeeIdentities<T>>::exists(validator), "Validator needs to be validated before");

            // 4. Verify sig
            ensure!(Self::identity_sig_check(&identity), "Tee report signature is illegal");

            // 5. applier is new add or needs to be updated
            if !<TeeIdentities<T>>::get(applier).contains(&identity) {
                // Store the tee identity
                <TeeIdentities<T>>::insert(applier, &identity);

                // Emit event
                Self::deposit_event(RawEvent::RegisterIdentity(who, identity));
            }

            Ok(())
		}

		pub fn report_works(origin, work_report: WorkReport) -> DispatchResult {
            let who = ensure_signed(origin)?;

            // 1. Ensure reporter is verified
            ensure!(<TeeIdentities<T>>::exists(&who), "Reporter must be registered before");

            // 2. Do timing check
            ensure!(Self::work_report_timing_check(&work_report).is_ok(), "Work report's timing is wrong");

            // 3. Do sig check
            ensure!(Self::work_report_sig_check(&work_report), "Work report signature is illegal");

            // 4. Upsert works
            <WorkReports<T>>::insert(&who, &work_report);

            // 5. Check staking
            let limitation = work_report.empty_workload + work_report.meaningful_workload;
            Self::check_and_set_stake_limitation(&who, limitation);

            // 6. Emit event
            Self::deposit_event(RawEvent::ReportWorks(who, work_report));

            Ok(())
        }
	}
}

impl<T: Trait> Module<T> {
    pub fn identity_sig_check(id: &Identity<T::AccountId>) -> bool {
        let applier_id = id.account_id.encode();
        let validator_id = id.validator_account_id.encode();
        // TODO: concat data inside runtime for saving PassBy params number
        api::crypto::verify_identity_sig(&id.pub_key, &applier_id, &id.validator_pub_key, &validator_id, &id.sig)
    }

    pub fn work_report_timing_check(wr: &WorkReport) -> DispatchResult {
        // 1. Check block hash
        // TODO: move to constants
        const REPORT_SLOT: u64 = 50;
        let wr_block_number: T::BlockNumber = wr.block_number.try_into().ok().unwrap();
        let wr_block_hash = <system::Module<T>>::block_hash(wr_block_number).as_ref().to_vec();
        ensure!(&wr_block_hash == &wr.block_hash, "work report hash is illegal");

        // 2. Check work report timing
        let current_block_number = <system::Module<T>>::block_number();
        let current_block_number_numeric: u64 = TryInto::<u64>::try_into(current_block_number).ok().unwrap();
        let current_report_slot = current_block_number_numeric / REPORT_SLOT;
        // genesis block or must be 50-times number
        ensure!(wr.block_number == 1 || wr.block_number == current_report_slot * REPORT_SLOT, "work report is outdated or beforehand");

        Ok(())
    }

    pub fn work_report_sig_check(wr: &WorkReport) -> bool {
        // TODO: concat data inside runtime for saving PassBy params number
        api::crypto::verify_work_report_sig(&wr.pub_key, wr.block_number, &wr.block_hash, &wr.empty_root,
                                                wr.empty_workload, wr.meaningful_workload, &wr.sig)
    }

    // TODO: change into own staking module
    pub fn check_and_set_stake_limitation(who: &T::AccountId, limitation: u64) {
        /*// 1. Get lockable balances and stash account
        let mut ledger = <staking::Module<T>>::ledger(&who).unwrap();
        let active_lockable_result = ledger.active;
        let stash_account = ledger.stash;

        // 2. Judge limitation
        if active_lockable_result <= limitation { return }
        ledger.active = limitation;
        ledger.total = limitation;

        // 3. [DANGER] If exceed limitation set new
        // TODO: Try another safe way to set stake limit
        <staking::Module<T>>::update_ledger(&stash_account, &ledger);*/
    }
}

decl_event!(
	pub enum Event<T> where AccountId = <T as system::Trait>::AccountId {
		RegisterIdentity(AccountId, Identity<AccountId>),
		ReportWorks(AccountId, WorkReport),
	}
);
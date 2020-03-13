//! The Substrate Node runtime. This can be compiled with `#[no_std]`, ready for Wasm.

#![cfg_attr(not(feature = "std"), no_std)]
#![feature(option_result_contains)]

use codec::{Decode, Encode};
use frame_support::{decl_event, decl_module, decl_storage, dispatch::DispatchResult, ensure};
use sp_std::convert::TryInto;
use sp_std::{str, vec::Vec};
use system::ensure_signed;

#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};

// Crust runtime modules
use primitives::{constants::tee::*, MerkleRoot, PubKey, TeeSignature};

/// Provides crypto and other std functions by implementing `runtime_interface`
pub mod api;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[derive(Debug, PartialEq, Eq, Clone, Encode, Decode, Default)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct Identity<T> {
    pub pub_key: PubKey,
    pub account_id: T,
    pub validator_pub_key: PubKey,
    pub validator_account_id: T,
    pub sig: TeeSignature,
}

#[derive(Debug, PartialEq, Eq, Clone, Encode, Decode, Default)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
// TODO: change block_number & block_hash to standard data type
pub struct WorkReport {
    pub pub_key: PubKey,
    pub block_number: u64,
    pub block_hash: Vec<u8>,
    pub empty_root: MerkleRoot,
    pub empty_workload: u64,
    pub meaningful_workload: u64,
    pub sig: TeeSignature,
}

/// The module's configuration trait.
pub trait Trait: system::Trait {
    /// The overarching event type.
    type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
}

// TODO: add add_extra_genesis to unify chain_spec
// This module's storage items.
decl_storage! {
    trait Store for Module<T: Trait> as Tee {
        pub TeeIdentities get(fn tee_identities) config(): linked_map T::AccountId => Option<Identity<T::AccountId>>;
        pub WorkReports get(fn work_reports) config(): map T::AccountId => Option<WorkReport>;
        pub Workloads get(fn workloads) build(|config: &GenesisConfig<T>| {
            Some(config.work_reports.iter().fold(0, |acc, (_, work_report)|
                acc + (&work_report.empty_workload + &work_report.meaningful_workload) as u128
            ))
        }): Option<u128>;
    }
}

// The module's dispatchable functions.
decl_module! {
    /// The module declaration.
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        // Initializing events
        // this is needed only if you are using events in your module
        fn deposit_event() = default;

        fn register_identity(origin, identity: Identity<T::AccountId>) -> DispatchResult {
            let who = ensure_signed(origin)?;

            // 0. Genesis validators have rights to register themselves
            if let Some(maybe_genesis_validator) = <TeeIdentities<T>>::get(&who) {
                if &maybe_genesis_validator.account_id == &maybe_genesis_validator.validator_account_id {
                    // Store the tee identity
                    <TeeIdentities<T>>::insert(&who, &identity);
                    return Ok(());
                }
            }

            let applier = &identity.account_id;
            let validator = &identity.validator_account_id;
            let applier_pk = &identity.pub_key;
            let validator_pk = &identity.validator_pub_key;

            // 1. Ensure who is applier
            // ensure!(&who == applier, "Tee applier must be the extrinsic sender");

            // 2. applier cannot be validator
            // ensure!(applier != validator, "You cannot verify yourself");
            // ensure!(applier_pk != validator_pk, "You cannot verify yourself");

            // 3. v_account_id should been validated before
            // ensure!(<TeeIdentities<T>>::exists(validator), "Validator needs to be validated before");
            // ensure!(&<TeeIdentities<T>>::get(validator).unwrap().pub_key == validator_pk, "Validator public key not found");

            // 4. Verify sig
            // ensure!(Self::identity_sig_check(&identity), "Tee report signature is illegal");

            // 5. applier is new add or needs to be updated
            if !<TeeIdentities<T>>::get(applier).contains(&identity) {
                // Store the tee identity
                <TeeIdentities<T>>::insert(applier, &identity);

                // Emit tee identity event
                Self::deposit_event(RawEvent::RegisterIdentity(who, identity));
            }

            Ok(())
        }

        fn report_works(origin, work_report: WorkReport) -> DispatchResult {
            let who = ensure_signed(origin)?;

            // 1. Ensure reporter is verified
            // ensure!(<TeeIdentities<T>>::exists(&who), "Reporter must be registered before");
            // ensure!(&<TeeIdentities<T>>::get(&who).unwrap().pub_key == &work_report.pub_key, "Validator public key not found");

            // 2. Do timing check
            // ensure!(Self::work_report_timing_check(&work_report).is_ok(), "Work report's timing is wrong");

            // 3. Do sig check
            // ensure!(Self::work_report_sig_check(&work_report), "Work report signature is illegal");

            // 4. Judge new and old workload
            let old_work_report = <WorkReports<T>>::get(&who).unwrap_or_default();
            let new_workload = (work_report.empty_workload + work_report.meaningful_workload) as u128;
            let old_workload = (old_work_report.empty_workload + old_work_report.meaningful_workload) as u128;

            if &old_work_report != &work_report {
                // 5. Upsert workload
                <WorkReports<T>>::insert(&who, &work_report);

                // 6. Get workloads
                let workloads = Workloads::get().unwrap_or_default();

                // 7. Upsert workloads
                Workloads::put(workloads + new_workload - old_workload);

                // 8. Emit workload event
                Self::deposit_event(RawEvent::ReportWorks(who, work_report));
            }

            Ok(())
        }
    }
}

impl<T: Trait> Module<T> {
    // IMMUTABLE PUBLIC
    /// Get updated workload by controller account
    pub fn get_and_update_workload(controller: &T::AccountId) -> u128 {
        if let Some(wr) = <WorkReports<T>>::get(controller) {
            // 1. Get current block number
            let current_block_number = <system::Module<T>>::block_number();
            let current_block_number_numeric: u64 =
                TryInto::<u64>::try_into(current_block_number).ok().unwrap();
            let workload = (wr.empty_workload + wr.meaningful_workload) as u128;

            // 2. Judge if work report is outdated
            if current_block_number_numeric - wr.block_number <= REPORT_SLOT*3 + 1 {
                return workload;
            } else {
                // 3. Remove outdated work report
                <WorkReports<T>>::remove(controller);

                // 4. Update workloads
                let current_workloads = Workloads::get().unwrap_or_default();
                Workloads::put((current_workloads - workload).max(0));
            }
        }
        0
    }

    // IMMUTABLE PRIVATE
    fn identity_sig_check(id: &Identity<T::AccountId>) -> bool {
        let applier_id = id.account_id.encode();
        let validator_id = id.validator_account_id.encode();
        // TODO: concat data inside runtime for saving PassBy params number
        api::crypto::verify_identity_sig(
            &id.pub_key,
            &applier_id,
            &id.validator_pub_key,
            &validator_id,
            &id.sig,
        )
    }

    fn work_report_timing_check(wr: &WorkReport) -> DispatchResult {
        // 1. Check block hash
        let wr_block_number: T::BlockNumber = wr.block_number.try_into().ok().unwrap();
        let wr_block_hash = <system::Module<T>>::block_hash(wr_block_number)
            .as_ref()
            .to_vec();
        ensure!(
            &wr_block_hash == &wr.block_hash,
            "work report hash is illegal"
        );

        // 2. Check work report timing
        let current_block_number = <system::Module<T>>::block_number();
        let current_block_number_numeric: u64 =
            TryInto::<u64>::try_into(current_block_number).ok().unwrap();
        let current_report_slot = current_block_number_numeric / REPORT_SLOT;
        // genesis block or must be 50-times number
        ensure!(
            wr.block_number == 1 || wr.block_number == current_report_slot * REPORT_SLOT,
            "work report is outdated or beforehand"
        );

        Ok(())
    }

    fn work_report_sig_check(wr: &WorkReport) -> bool {
        // TODO: concat data inside runtime for saving PassBy params number
        api::crypto::verify_work_report_sig(
            &wr.pub_key,
            wr.block_number,
            &wr.block_hash,
            &wr.empty_root,
            wr.empty_workload,
            wr.meaningful_workload,
            &wr.sig,
        )
    }
}

decl_event!(
    pub enum Event<T>
    where
        AccountId = <T as system::Trait>::AccountId,
    {
        RegisterIdentity(AccountId, Identity<AccountId>),
        ReportWorks(AccountId, WorkReport),
    }
);

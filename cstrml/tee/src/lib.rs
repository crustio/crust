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
// TODO: change workload to u128
pub struct WorkReport {
    pub pub_key: PubKey,
    pub block_number: u64,
    pub block_hash: Vec<u8>,
    pub empty_root: MerkleRoot,
    pub empty_workload: u64,
    pub meaningful_workload: u64,
    pub sig: TeeSignature,
}

/// An event handler for reporting works
pub trait OnReportWorks<AccountId> {
    fn on_report_works(controller: &AccountId, own_workload: u128, total_workload: u128);
}

impl<AId> OnReportWorks<AId> for () {
    fn on_report_works(_: &AId, _: u128, _: u128) {}
}

/// The module's configuration trait.
pub trait Trait: system::Trait {
    /// The overarching event type.
    type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;

    /// The handler for reporting works
    type OnReportWorks: OnReportWorks<Self::AccountId>;
}

decl_storage! {
    trait Store for Module<T: Trait> as Tee {
        /// The TEE identities, mapping from controller to optional identity value
        pub TeeIdentities get(fn tee_identities) config(): linked_map
            T::AccountId => Option<Identity<T::AccountId>>;

        /// Node's work report, mapping from (controller, block_number) to optional work_report
        pub WorkReports get(fn work_reports) config(): map
            (T::AccountId, u64)  => Option<WorkReport>;

        /// The old report slot block number.
        pub CurrentReportSlot get(fn current_report_slot) config(): u64;

        /// The meaningful workload, used for calculating stake limit in the end of era
        /// default is 0
        pub MeaningfulWorkload get(fn meaningful_workload) build(|config: &GenesisConfig<T>| {
            config.work_reports.iter().fold(0, |acc, (_, work_report)|
                acc + work_report.meaningful_workload as u128
            )
        }): u128 = 0;

        /// The empty workload, used for calculating stake limit in the end of era
        /// default is 0
        pub EmptyWorkload get(fn empty_workload) build(|config: &GenesisConfig<T>| {
            config.work_reports.iter().fold(0, |acc, (_, work_report)|
                acc + work_report.empty_workload as u128
            )
        }): u128 = 0;
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
            ensure!(&who == applier, "Tee applier must be the extrinsic sender");

            // 2. applier cannot be validator
            ensure!(applier != validator, "You cannot verify yourself");
            ensure!(applier_pk != validator_pk, "You cannot verify yourself");

            // 3. v_account_id should been validated before
            ensure!(<TeeIdentities<T>>::exists(validator), "Validator needs to be validated before");
            ensure!(&<TeeIdentities<T>>::get(validator).unwrap().pub_key == validator_pk, "Validator public key not found");

            // 4. Verify sig
            ensure!(Self::identity_sig_check(&identity), "Tee report signature is illegal");

            // 5. applier is new add or needs to be updated
            if !Self::tee_identities(applier).contains(&identity) {
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
            ensure!(<TeeIdentities<T>>::exists(&who), "Reporter must be registered before");
            ensure!(&<TeeIdentities<T>>::get(&who).unwrap().pub_key == &work_report.pub_key, "Validator public key not found");

            // 2. Do timing check
            ensure!(Self::work_report_timing_check(&work_report).is_ok(), "Work report's timing is wrong");

            // 3. Do sig check
            ensure!(Self::work_report_sig_check(&work_report), "Work report signature is illegal");

            // 4. Maybe upsert work report
            if Self::maybe_upsert_work_report(&who, &work_report) {
                // 5. Emit workload event
                Self::deposit_event(RawEvent::ReportWorks(who, work_report));
            }
            Ok(())
        }
    }
}

impl<T: Trait> Module<T> {
    // PUBLIC MUTABLES

    /// This function is for updating all identities' work report, mainly aimed to check if it is outdated
    /// and it should be called in the start of era.
    ///
    /// TC = O(n)
    /// DB try is 2n+1
    pub fn update_identities() {
        // Ideally, reported_rs should be current_rs + 1
        let reported_rs = Self::get_reported_slot();
        let current_rs = Self::current_report_slot();

        // 1. Report slot did not change, it should not trigger updating
        if current_rs == reported_rs {
            return;
        }

        // 2. Update last report slot be current
        CurrentReportSlot::mutate(|crs| *crs = reported_rs);

        // 3. Slot changed, update all identities
        let ids: Vec<(T::AccountId, Identity<T::AccountId>)> =
            <TeeIdentities<T>>::enumerate().collect();

        // 4. Update id's work report, get id's workload and total workload
        let workload_map: Vec<(&T::AccountId, u128)> = ids.iter().map(|(controller, _)| {
            (controller, Self::update_and_get_workload(&controller, reported_rs))
        }).collect();
        let total_workload = Self::meaningful_workload() + Self::empty_workload();

        // 5. Update stake limit
        for (controller, own_workload) in workload_map {
            T::OnReportWorks::on_report_works(controller, own_workload, total_workload);
        }
    }

    pub fn get_work_report(who: &T::AccountId) -> Option<WorkReport> {
        let current_rs = Self::current_report_slot();
        <WorkReports<T>>::get((who, current_rs))
    }

    // PRIVATE IMMUTABLES
    fn maybe_upsert_work_report(who: &T::AccountId, wr: &WorkReport) -> bool {
        // 1. Current block always be 300*n(n >= 1) + 4(for Alphanet)
        let current_rs = Self::current_report_slot();

        // 2. Judge if wr on current_rs is existed
        let mut old_m_workload: u128 = 0;
        let mut old_e_workload: u128 = 0;

        if let Some(old_wr) = Self::work_reports((who, current_rs)) {
            if &old_wr == wr {
                return false;
            } else {
                old_m_workload = old_wr.meaningful_workload as u128;
                old_e_workload = old_wr.empty_workload as u128;
            }
        }

        // 3. Upsert work report
        <WorkReports<T>>::insert((who, wr.block_number), wr);

        // 4. Upsert workload
        let m_workload = wr.meaningful_workload as u128;
        let e_workload = wr.empty_workload as u128;
        let m_total_workload = Self::meaningful_workload() - old_m_workload + m_workload;
        let e_total_workload = Self::empty_workload() - old_e_workload + e_workload;

        MeaningfulWorkload::put(m_total_workload);
        EmptyWorkload::put(e_total_workload);

        // 5. Call `on_report_works` handler
        T::OnReportWorks::on_report_works(
            &who,
            m_workload + e_workload,
            m_total_workload + e_total_workload,
        );
        true
    }

    /// Get updated workload by controller account,
    /// this function should only be called in the new era
    /// otherwise, it will be an void in this recursive loop
    fn update_and_get_workload(controller: &T::AccountId, current_rs: u64) -> u128 {
        // 1. Get current era
        let last_rs = current_rs - REPORT_SLOT;

        // 2. Judge if this controller reported works in the former era
        if let Some(wr) = Self::work_reports((controller, last_rs)) {
            // a. Remove former era's work report
            <WorkReports<T>>::remove((controller, last_rs));

            // b. Did report works in the last era
            // ...
            // 889(600): (123,300){wr300} ✅ | (123, 300){wr0} ❌
            // 889(600): (123, 300){wr300} -> (123, 600){wr300}
            // 1080(900): (123, 600){wr600} ✅ | (123, 600){wr300} ❌
            // 1080(900): (123, 600){wr600} -> (123, 900){wr600}
            // ...
            // old(old_old) <- new
            // new(old) <- new_new
            // TODO: between the extended current_rs wr and the real current_rs wr contains a void,
            // it may mislead storage order
            if wr.block_number == last_rs {
                // Extend the last work report(if not exist) into this new report slot
                if !<WorkReports<T>>::exists((controller, current_rs)) {
                    // This should already be true
                    <WorkReports<T>>::insert((controller, current_rs), &wr);
                }
                (wr.empty_workload + wr.meaningful_workload) as u128
            // c. Did not report anything in the last era
            } else {
                // Cut workload
                MeaningfulWorkload::mutate(|mw| *mw -= wr.meaningful_workload as u128);
                EmptyWorkload::mutate(|ew| *ew -= wr.empty_workload as u128);
                0
            }
        } else {
            0
        }
    }

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
        ensure!(
            wr.block_number == 1 || wr.block_number == Self::get_reported_slot(),
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

    fn get_reported_slot() -> u64 {
        let current_block_number = <system::Module<T>>::block_number();
        let current_block_numeric = TryInto::<u64>::try_into(current_block_number).ok().unwrap();
        let current_report_index = current_block_numeric / REPORT_SLOT;
        current_report_index * REPORT_SLOT
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

//! The Substrate Node runtime. This can be compiled with `#[no_std]`, ready for Wasm.

#![cfg_attr(not(feature = "std"), no_std)]
#![feature(option_result_contains)]

use codec::{Decode, Encode};
use frame_support::{
    decl_event, decl_module, decl_storage, ensure,
    dispatch::DispatchResult,
    storage::IterableStorageMap
};
use sp_std::convert::TryInto;
use sp_std::{str, vec::Vec};
use system::ensure_signed;

#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};

// Crust runtime modules
use primitives::{constants::tee::*, MerkleRoot, PubKey, TeeSignature, ReportSlot};

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
        pub TeeIdentities get(fn tee_identities) config():
            map hasher(blake2_128_concat) T::AccountId => Option<Identity<T::AccountId>>;

        /// Node's work report, mapping from controller to optional work_report
        pub WorkReports get(fn work_reports) config():
            map hasher(blake2_128_concat) T::AccountId  => Option<WorkReport>;

        /// The current report slot block number, this value should be a multiple of era block
        pub CurrentReportSlot get(fn current_report_slot) config(): ReportSlot;

        /// Recording whether the validator reported works of each era
        /// We leave it keep all era's report info
        /// cause B-tree won't build index on key2(ReportSlot)
        pub ReportedInSlot get(fn reported_in_slot) build(|config: &GenesisConfig<T>| {
            config.work_reports.iter().map(|(account_id, _)|
                (account_id.clone(), 0, true)
            ).collect::<Vec<_>>()
        }): double_map hasher(twox_64_concat) T::AccountId, hasher(twox_64_concat) ReportSlot
        => bool = false;

        /// The meaningful workload, used for calculating stake limit in the end of era
        /// default is 0
        pub MeaningfulWorkload get(fn meaningful_workload): u128 = 0;

        /// The empty workload, used for calculating stake limit in the end of era
        /// default is 0
        pub EmptyWorkload get(fn empty_workload): u128 = 0;
    }
}

// The module's dispatchable functions.
decl_module! {
    /// The module declaration.
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        // Initializing events
        // this is needed only if you are using events in your module
        fn deposit_event() = default;

        #[weight = frame_support::weights::SimpleDispatchInfo::default()]
        // FIXME: issues#58 check bonding relation is unique
        pub fn register_identity(origin, identity: Identity<T::AccountId>) -> DispatchResult {
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
            ensure!(<TeeIdentities<T>>::contains_key(validator), "Validator needs to be validated before");
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

        #[weight = frame_support::weights::SimpleDispatchInfo::default()]
        fn report_works(origin, work_report: WorkReport) -> DispatchResult {
            let who = ensure_signed(origin)?;

            // 1. Ensure reporter is verified
            ensure!(<TeeIdentities<T>>::contains_key(&who), "Reporter must be registered before");
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

        // 2. Update id's work rzeport, get id's workload and total workload
        let mut total_meaningful_workload = 0;
        let mut total_empty_workload = 0;

        let workload_map: Vec<(T::AccountId, u128)> = <TeeIdentities<T>>::iter().map(|(controller, _)| {
            let (e_workload, m_workload) = Self::update_and_get_workload(&controller, current_rs);
            total_meaningful_workload += m_workload;
            total_empty_workload += e_workload;
            (controller.clone(), m_workload + e_workload)
        }).collect();

        MeaningfulWorkload::put(total_meaningful_workload);
        EmptyWorkload::put(total_empty_workload);
        let total_workload = total_meaningful_workload + total_empty_workload;

        // 3. Update current report slot
        CurrentReportSlot::mutate(|crs| *crs = reported_rs);

        // 4. Update stake limit
        for (controller, own_workload) in workload_map {
            T::OnReportWorks::on_report_works(&controller, own_workload, total_workload);
        }
    }

    pub fn get_work_report(who: &T::AccountId) -> Option<WorkReport> {
        <WorkReports<T>>::get(who)
    }

    // PRIVATE IMMUTABLES
    fn maybe_upsert_work_report(who: &T::AccountId, wr: &WorkReport) -> bool {
        let mut old_m_workload: u128 = 0;
        let mut old_e_workload: u128 = 0;
        let rs = Self::get_reported_slot();

        // 1. Judge if wr exists
        if let Some(old_wr) = Self::work_reports(who) {
            if &old_wr == wr {
                return false;
            } else {
                old_m_workload = old_wr.meaningful_workload as u128;
                old_e_workload = old_wr.empty_workload as u128;
            }
        }

        // 2. Upsert work report and mark reported this (report)slot
        <WorkReports<T>>::insert(who, wr);
        <ReportedInSlot<T>>::insert(who, rs, true);

        // 3. Upsert workload
        let m_workload = wr.meaningful_workload as u128;
        let e_workload = wr.empty_workload as u128;
        let m_total_workload = Self::meaningful_workload() - old_m_workload + m_workload;
        let e_total_workload = Self::empty_workload() - old_e_workload + e_workload;

        MeaningfulWorkload::put(m_total_workload);
        EmptyWorkload::put(e_total_workload);

        // 4. Call `on_report_works` handler
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
    fn update_and_get_workload(controller: &T::AccountId, current_rs: u64) -> (u128, u128) {
        // 1. Judge if this controller reported works in this current era
        if let Some(wr) = Self::work_reports(controller) {
            if Self::reported_in_slot(controller, current_rs) {
                (wr.empty_workload as u128, wr.meaningful_workload as u128)
            } else {
                // Remove work report when wr IS outdated
                if wr.block_number < current_rs {
                    <WorkReports<T>>::remove(controller);
                }
                (0, 0)
            }
        } else {
            (0, 0)
        }
    }

    fn identity_sig_check(id: &Identity<T::AccountId>) -> bool {
        let applier_id = id.account_id.encode();
        let validator_id = id.validator_account_id.encode();
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

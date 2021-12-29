// Copyright (C) 2019-2021 Crust Network Technologies Ltd.
// This file is part of Crust.

#![cfg_attr(not(feature = "std"), no_std)]
#![feature(option_result_contains)]

use codec::{Decode, Encode};
use frame_support::{
    decl_event, decl_module, decl_storage, decl_error, ensure,
    dispatch::{DispatchResult, DispatchResultWithPostInfo},
    storage::{IterableStorageMap, generator::StorageMap, unhashed},
    traits::{Currency, ReservableCurrency, Get},
    ReversibleStorageHasher,
    weights::{
        Weight, DispatchClass, Pays
    }
};
pub use frame_support::storage::PrefixIterator;
use sp_runtime::traits::{StaticLookup, Zero};
use sp_std::{str, convert::TryInto, prelude::*, collections::btree_set::BTreeSet};
use frame_system::{self as system, ensure_root, ensure_signed};

#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};

// Crust primitives and runtime modules
use primitives::{
    constants::swork::*,
    MerkleRoot, SworkerPubKey, SworkerSignature,
    ReportSlot, BlockNumber, IASSig,
    ISVBody, SworkerCert, SworkerCode, SworkerAnchor,
    traits::{MarketInterface, SworkerInterface, BenefitInterface}
};
use sp_std::collections::btree_map::BTreeMap;

pub mod weight;

/// Provides util functions
pub mod utils;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

pub type BalanceOf<T> =
    <<T as Config>::Currency as Currency<<T as system::Config>::AccountId>>::Balance;
pub type NegativeImbalanceOf<T> = <<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::NegativeImbalance;

pub(crate) const LOG_TARGET: &'static str = "swork";
const IDENTITY_UPDATE_LENGTH: usize = 500; // Loop 500 identities per block
const SRD_LIMIT: u64 = 2_251_799_813_685_248; // 2 PB <-> 2 * 1024 * 1024 * 1024 * 1024 * 1024.
const FILES_LIMIT: u64 = 9_007_199_254_740_992; // 8 PB <-> 8 * 1024 * 1024 * 1024 * 1024 * 1024.
const FILES_COUNT_LIMIT: usize = 300; // TODO: 300 files for now(will be deleted after completed wr reporting mechanism).
const NEW_IDENTITY: ReportSlot = 1;
const NO_PUNISHMENT: ReportSlot = 0;

#[macro_export]
macro_rules! log {
    ($level:tt, $patter:expr $(, $values:expr)* $(,)?) => {
        frame_support::debug::$level!(
            target: crate::LOG_TARGET,
            $patter $(, $values)*
        )
    };
}

pub trait WeightInfo {
    fn set_code() -> Weight;
    fn register() -> Weight;
    fn report_works(added: u32, deleted: u32) -> Weight;
    fn create_group() -> Weight;
    fn join_group() -> Weight;
    fn quit_group() -> Weight;
    fn kick_out() -> Weight;
}

#[derive(Debug, PartialEq, Eq, Clone, Encode, Decode, Default)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct WorkReport {
    /// Timing judgement
    pub report_slot: u64,

    /// Storage information
    pub spower: u64, // Real file(mapping with sOrder) size
    pub free: u64,

    /// Assist judgement
    pub reported_files_size: u64, // Reported files size
    pub reported_srd_root: MerkleRoot, // Srd hash root
    pub reported_files_root: MerkleRoot, // Reported files hash root
}

#[derive(Debug, PartialEq, Eq, Clone, Encode, Decode, Default)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct PKInfo {
    pub code: SworkerCode,
    pub anchor: Option<SworkerAnchor> // is bonded to an account or not in report work
}

#[derive(Debug, PartialEq, Eq, Clone, Encode, Decode, Default)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct Identity<AccountId> {
    /// The unique identity associated to one account id.
    /// During the AB upgrade, this anchor would keep and won't change.
    pub anchor: SworkerAnchor,
    pub punishment_deadline: ReportSlot,
    pub group: Option<AccountId>
}

#[derive(Debug, PartialEq, Eq, Clone, Encode, Decode, Default)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct Group<AccountId: Ord + PartialOrd> {
    pub members: BTreeSet<AccountId>,
    pub allowlist: BTreeSet<AccountId>,
}

/// An event handler for reporting works
pub trait Works<AccountId> {
    fn report_works(workload_map: BTreeMap<AccountId, u128>, total_workload: u128) -> Weight;
}

impl<AId> Works<AId> for () {
    fn report_works(_: BTreeMap<AId, u128>, _: u128) -> Weight { 0 }
}

/// Implement market's file inspector
impl<T: Config> SworkerInterface<T::AccountId> for Module<T> {
    /// check wr existing or not
    fn is_wr_reported(anchor: &SworkerAnchor, bn: BlockNumber) -> bool {
        let current_rs = Self::convert_bn_to_rs(bn);
        let prev_rs = current_rs.saturating_sub(REPORT_SLOT);
        Self::reported_in_slot(&anchor, prev_rs)
    }

    /// update the spower value due to deleted files, dump trash or calculate_payout
    fn update_spower(anchor: &SworkerAnchor, anchor_decrease_spower: u64, anchor_increase_spower: u64) {
        if anchor_decrease_spower != anchor_increase_spower {
            WorkReports::mutate_exists(anchor, |maybe_wr| match *maybe_wr {
                Some(WorkReport { ref mut spower, .. }) => *spower = spower.saturating_sub(anchor_decrease_spower).saturating_add(anchor_increase_spower),
                ref mut i => *i = None,
            });
        }
    }

    /// check whether the account id and the anchor is valid or not
    fn check_anchor(who: &T::AccountId, anchor: &SworkerAnchor) -> bool {
        if let Some(identity) = Self::identities(who) {
            return identity.anchor == *anchor;
        }
        false
    }

    /// get total reported files size and free space
    fn get_files_size_and_free_space() -> (u128, u128) {
        (Self::reported_files_size(), Self::free())
    }

    /// Get the added files count in the past one period and clear the record
    fn get_added_files_count_and_clear_record() -> u32 {
        let added_files_count = Self::added_files_count();
        AddedFilesCount::put(0);
        added_files_count
    }

    /// Get owner of this member
    fn get_owner(who: &T::AccountId) -> Option<T::AccountId> {
        Self::identities(who).unwrap_or_default().group
    }
}

/// The module's configuration trait.
pub trait Config: system::Config {
    /// The payment balance.
    /// TODO: remove this for abstracting MarketInterface into sWorker self
    type Currency: ReservableCurrency<Self::AccountId>;

    /// The overarching event type.
    type Event: From<Event<Self>> + Into<<Self as system::Config>::Event>;

    /// Punishment duration if someone offline. It's the count of REPORT_SLOT
    type PunishmentSlots: Get<u32>;

    /// The handler for reporting works.
    type Works: Works<Self::AccountId>;

    /// Interface for interacting with a market module.
    type MarketInterface: MarketInterface<Self::AccountId, BalanceOf<Self>>;

    /// Max number of members in one group
    type MaxGroupSize: Get<u32>;

    /// Fee reduction interface
    type BenefitInterface: BenefitInterface<Self::AccountId, BalanceOf<Self>, NegativeImbalanceOf<Self>>;

    /// Weight information for extrinsics in this pallet.
    type WeightInfo: WeightInfo;
}

decl_storage! {
    trait Store for Module<T: Config> as Swork {

        /// The depth of the history of the ReportedInSlot
        HistorySlotDepth get(fn history_slot_depth): ReportSlot = 6 * REPORT_SLOT;

        /// The sWorker enclave codes, this should be managed by sudo/democracy
        pub Codes get (fn codes): map hasher(twox_64_concat) SworkerCode => Option<T::BlockNumber>;

        /// The identity information for each sworker member, which contains the anchor, punishment deadline and group information.
        pub Identities get(fn identities):
            map hasher(blake2_128_concat) T::AccountId => Option<Identity<T::AccountId>>;

        /// The previous key spower to iterate identities
        pub IdentityPreviousKey get(fn identity_previous_key): Option<Vec<u8>>;

        /// The workload information
        pub Workload get(fn workload): Option<(BTreeMap<T::AccountId, u128>, u128, u128, u128)>;

        /// The pub key information, mapping from sWorker public key to an pubkey information, including the sworker enclave code and option anchor.
        pub PubKeys get(fn pub_keys):
            map hasher(twox_64_concat) SworkerPubKey => PKInfo;

        /// The group information
        pub Groups get(fn groups):
            map hasher(blake2_128_concat) T::AccountId => Group<T::AccountId>;

        /// Node's work report, mapping from sWorker anchor to an optional work report.
        /// WorkReport only been replaced, it won't get removed cause we need to check the
        /// status transition from off-chain sWorker
        pub WorkReports get(fn work_reports):
            map hasher(twox_64_concat) SworkerAnchor => Option<WorkReport>;

        /// The current report slot block number, this value should be a multiple of report slot block.
        pub CurrentReportSlot get(fn current_report_slot): ReportSlot = 0;

        /// Recording whether the validator reported works of each report slot.
        /// We keep the last "HistorySlotDepth" length data
        /// cause B-tree won't build index on key2(ReportSlot).
        /// The value represent if reported in this slot
        // TODO: reverse the keys when we launch mainnet
        pub ReportedInSlot get(fn reported_in_slot):
            double_map hasher(twox_64_concat) SworkerAnchor, hasher(twox_64_concat) ReportSlot => bool = false;

        /// The spower workload, used for calculating stake limit in the end of each report slot.
        /// The default value is 0.
        pub Spower get(fn spower): u128 = 0;

        /// The total reported files workload, used for calculating total_capacity for market module
        /// The default value is 0.
        pub ReportedFilesSize get(fn reported_files_size): u128 = 0;

        /// The free workload, used for calculating stake limit in the end of each report slot.
        /// The default value is 0.
        pub Free get(fn free): u128 = 0;

        /// Enable punishment, the default behavior will have punishment.
        pub EnablePunishment get(fn enable_punishment): bool = true;

        /// Added files count in the past one period(one hour)
        pub AddedFilesCount get(fn added_files_count): u32 = 0;
    }
    add_extra_genesis {
        config(init_codes):
            Vec<(SworkerCode, T::BlockNumber)>;
        build(|config: &GenesisConfig<T>| {
            for (code, expired_bn) in &config.init_codes {
                <Codes<T>>::insert(code, expired_bn);
            }
        });
    }
}

decl_error! {
    /// Error for the swork module.
    pub enum Error for Module<T: Config> {
        /// Illegal applier
        IllegalApplier,
        /// Identity check failed
        IllegalIdentity,
        /// Illegal reporter
        IllegalReporter,
        /// Outdated reporter
        OutdatedReporter,
        /// Invalid timing
        InvalidReportTime,
        /// Illegal work report signature
        IllegalWorkReportSig,
        /// A/B Upgrade failed
        ABUpgradeFailed,
        /// Files change not legal
        IllegalFilesTransition,
        /// Identity doesn't exist
        IdentityNotExist,
        /// Already joint one group
        AlreadyJoint,
        /// The target is not a group owner. Please make sure that the target is a group owner.
        NotOwner,
        /// The spower value is not zero and cannot join a group.
        IllegalSpower,
        /// The group already exist.
        GroupAlreadyExist,
        /// The group owner cannot be a sWorker member.
        GroupOwnerForbidden,
        /// The member is not in this group and cannot quit.
        NotJoint,
        /// Exceed the limit of members number in one group.
        ExceedGroupLimit,
        /// Cannot extend the valid duration for an existed enclave code.
        InvalidExpiredBlock,
        /// Who is not in the allowlist. Please ask owner to add you into the allowlist before you join the group.
        NotInAllowlist,
        /// Exceed the limit of allowlist number in one group.
        ExceedAllowlistLimit,
        /// Illegal work report. This should never happen.
        IllegalWorkReport,
        /// Code has not been expired
        CodeNotExpired
    }
}

decl_module! {
    pub struct Module<T: Config> for enum Call where origin: T::Origin {
        type Error = Error<T>;

        /// The punishment duration if someone offline
        const PunishmentSlots: u32 = T::PunishmentSlots::get();

        /// The max number of members in one group
        const MaxGroupSize: u32 = T::MaxGroupSize::get();

        // Initializing events
        // this is needed only if you are using events in your module
        fn deposit_event() = default;


        /// Called when a block is initialized. Will call update_identities to update stake limit
        fn on_initialize(now: T::BlockNumber) -> Weight {
            let now = TryInto::<u32>::try_into(now).ok().unwrap();
            let mut consumed_weight: Weight = 0;
            let mut add_db_reads_writes = |reads, writes, weights| {
                consumed_weight += T::DbWeight::get().reads_writes(reads, writes);
                consumed_weight += weights;
            };
            // At REPORT_SLOT - UPDATE_OFFSET blocks, updating process would start
            // IdentityPreviousKey is used as the switch as well
            // There are three status
            // 1. identity_previous_key is none and workload is none => not started
            // 2. identity_previous_key is some and workload is some => calculating
            // 3. identity_previous_key is none and workload is some => calculation is done and will send workload to staking module
            if ((now + UPDATE_OFFSET) % (REPORT_SLOT as u32)).is_zero() && Self::workload().is_none() && Self::identity_previous_key().is_none()  {
                let prefix = <Identities<T>>::prefix_hash();
                IdentityPreviousKey::put(prefix);
                <Workload<T>>::put((BTreeMap::<T::AccountId, u128>::new(), 0u128, 0u128, 0u128));
                add_db_reads_writes(0, 2, 0);
            }
            add_db_reads_writes(2, 0, 0);
            // If it's not timeout and not finished yet, continue updating process
            if !((now + END_OFFSET) % (REPORT_SLOT as u32)).is_zero() && Self::identity_previous_key().is_some() {
                let previous_key = Self::identity_previous_key().unwrap();
                // Update the workload map in one batch iter, might kill the IdentityPreviousKey
                // which means updating process is finished.
                add_db_reads_writes(0, 0, Self::patial_update_identities(previous_key));
            } else {
                if let Some((workload_map, total_free, total_spower, total_reported_files_size)) = Self::workload() {
                    // Update Free, Spower, ReportedFilesSize and CurrentReportSlot
                    Free::put(total_free);
                    Spower::put(total_spower);
                    ReportedFilesSize::put(total_reported_files_size);
                    CurrentReportSlot::mutate(|crs| *crs = Self::get_current_reported_slot());

                    add_db_reads_writes(0, 4, 0);

                    // Invoke report works to update stake limit
                    let total_workload = total_spower.saturating_add(total_free);
                    add_db_reads_writes(0, 0, T::Works::report_works(workload_map, total_workload));

                    // Kill the IdentityPreviousKey and Workload
                    IdentityPreviousKey::kill();
                    <Workload<T>>::kill();
                    add_db_reads_writes(0, 2, 0);
                }
            }
            consumed_weight
        }

        /// Set code for AB Upgrade, this should only be called by `root` origin
        /// Ruled by `sudo/democracy`
        ///
        /// # <weight>
        /// - O(1)
        /// - 2 DB try
        /// # </weight>
        #[weight = (T::WeightInfo::set_code(), DispatchClass::Operational)]
        pub fn set_code(origin, new_code: SworkerCode, expire_block: T::BlockNumber) {
            // TODO: enable democracy
            ensure_root(origin)?;
            if let Some(old_expired_block) = Self::codes(&new_code) {
                ensure!(expire_block < old_expired_block, Error::<T>::InvalidExpiredBlock);
            }
            <Codes<T>>::insert(&new_code, &expire_block);
            Self::deposit_event(RawEvent::SetCodeSuccess(new_code, expire_block));
        }


        /// clear the expired code
        #[weight = T::WeightInfo::set_code()]
        pub fn clear_expired_code(origin, expired_code: SworkerCode) {
            let _ = ensure_signed(origin)?;
            if let Some(expire_block) = Self::codes(&expired_code) {
                let curr_bn = <system::Module<T>>::block_number();
                ensure!(expire_block < curr_bn, Error::<T>::CodeNotExpired);
                <Codes<T>>::remove(&expired_code);
                Self::deposit_event(RawEvent::RemoveCodeSuccess(expired_code));
            }
        }

        /// Register as new trusted node, can only called from sWorker.
        /// All `inputs` can only be generated from sWorker's enclave
        ///
        /// The dispatch origin for this call must be _Signed_ by the reporter account.
        ///
        /// Emits `RegisterSuccess` if new id has been registered.
        ///
        /// # <weight>
        /// - Independent of the arguments. Moderate complexity.
        /// - TC depends on identities' number.
        /// - DB try depends on identities' number.
        ///
        /// ------------------
        /// DB Weight:
        /// - Read: Identities
        /// - Write: 3
        /// # </weight>
        #[weight = T::WeightInfo::register()]
        pub fn register(
            origin,
            ias_sig: IASSig,
            ias_cert: SworkerCert,
            applier: T::AccountId,
            isv_body: ISVBody,
            sig: SworkerSignature
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;

            // 1. Ensure who is applier
            ensure!(&who == &applier, Error::<T>::IllegalApplier);

            // 2. Ensure who cannot be group owner
            ensure!(!<Groups<T>>::contains_key(&who), Error::<T>::GroupOwnerForbidden);

            // 3. Ensure unparsed_identity trusted chain is legal, including signature and sworker code
            let (maybe_pk, maybe_code) = Self::maybe_get_pk_and_code(&ias_sig, &ias_cert, &applier, &isv_body, &sig);
            ensure!(maybe_pk.is_some() && maybe_code.is_some(), Error::<T>::IllegalIdentity);

            // 4. Insert new pub key info
            let pk = maybe_pk.unwrap();
            let code = maybe_code.unwrap();

            // 5. Insert the pk and code
            Self::insert_pk_info(pk.clone(), code);

            // 6. Emit event
            Self::deposit_event(RawEvent::RegisterSuccess(who, pk));

            Ok(())
        }

        /// Report storage works from sWorker
        /// All `inputs` can only be generated from sWorker's enclave
        ///
        /// The dispatch origin for this call must be _Signed_ by the reporter account.
        ///
        /// Emits `WorksReportSuccess` if new work report has been reported
        ///
        /// # <weight>
        /// - Independent of the arguments. Moderate complexity.
        /// - TC depends on identities' size and market.Merchant.file_map size
        /// - DB try depends on identities and market.Merchant.file_map
        ///
        /// ------------------
        /// DB Weight:
        /// - Read: Identities, ReportedInSlot, Code, market.Merchant, market.SOrder
        /// - Write: WorkReport, ReportedInSlot, market.SOrder
        /// # </weight>
        #[weight = T::WeightInfo::report_works(added_files.len() as u32, deleted_files.len() as u32)]
        pub fn report_works(
            origin,
            curr_pk: SworkerPubKey,
            ab_upgrade_pk: SworkerPubKey,
            slot: u64,
            slot_hash: Vec<u8>,
            reported_srd_size: u64,
            reported_files_size: u64,
            added_files: Vec<(MerkleRoot, u64, u64)>,
            deleted_files: Vec<(MerkleRoot, u64, u64)>,
            reported_srd_root: MerkleRoot,
            reported_files_root: MerkleRoot,
            sig: SworkerSignature
        ) -> DispatchResultWithPostInfo {
            let reporter = ensure_signed(origin)?;
            let mut prev_pk = curr_pk.clone();

            // 1. Basic check
            ensure!(reported_srd_size < SRD_LIMIT && reported_files_size < FILES_LIMIT && added_files.len() <= FILES_COUNT_LIMIT && deleted_files.len() <= FILES_COUNT_LIMIT, Error::<T>::IllegalWorkReport);

            // 2. Ensure reporter is registered
            ensure!(PubKeys::contains_key(&curr_pk), Error::<T>::IllegalReporter);

            // 3. Ensure who cannot be group owner
            ensure!(!<Groups<T>>::contains_key(&reporter), Error::<T>::GroupOwnerForbidden);
            
            // 4. Ensure reporter's code is legal
            ensure!(Self::reporter_code_check(&curr_pk, slot), Error::<T>::OutdatedReporter);

            // 5. Decide which scenario
            let maybe_anchor = Self::pub_keys(&curr_pk).anchor;
            let is_ab_upgrade = maybe_anchor.is_none() && !ab_upgrade_pk.is_empty();
            let is_first_report = maybe_anchor.is_none() && ab_upgrade_pk.is_empty();

            // 6. Unique Check for normal report work for curr pk
            if let Some(anchor) = maybe_anchor {
                // Normally report works.
                // 6.1 Ensure Identity's anchor be same with current pk's anchor
                ensure!(Self::identities(&reporter).unwrap_or_default().anchor == anchor, Error::<T>::IllegalReporter);
                // 6.2 Already reported with same pub key in the same slot, return immediately
                if Self::reported_in_slot(&anchor, slot) {
                    log!(
                        trace,
                        "🔒 Already reported with same pub key {:?} in the same slot {:?}.",
                        curr_pk,
                        slot
                    );
                    // This is weird and might be an attack.
                    return Ok(Some(0 as Weight).into())
                }
            }

            // 7. Timing check
            ensure!(Self::work_report_timing_check(slot, &slot_hash).is_ok(), Error::<T>::InvalidReportTime);

            // 8. Ensure sig is legal
            ensure!(
                Self::work_report_sig_check(
                    &curr_pk,
                    &ab_upgrade_pk,
                    slot,
                    &slot_hash,
                    reported_srd_size,
                    reported_files_size,
                    &reported_srd_root,
                    &reported_files_root,
                    &added_files,
                    &deleted_files,
                    &sig
                ),
                Error::<T>::IllegalWorkReportSig
            );

            // 9. Files storage status transition check
            if is_ab_upgrade {
                // 9.1 Previous pk should already reported works
                ensure!(PubKeys::contains_key(&ab_upgrade_pk), Error::<T>::ABUpgradeFailed);
                // unwrap_or_default is a small tricky solution
                let maybe_prev_wr = Self::work_reports(&Self::pub_keys(&ab_upgrade_pk).anchor.unwrap_or_default());
                ensure!(maybe_prev_wr.is_some(), Error::<T>::ABUpgradeFailed);

                // 9.2 Current work report should NOT be changed at all
                let prev_wr = maybe_prev_wr.unwrap();
                ensure!(added_files.is_empty() &&
                    deleted_files.is_empty() &&
                    prev_wr.reported_files_root == reported_files_root &&
                    prev_wr.reported_srd_root == reported_srd_root,
                    Error::<T>::ABUpgradeFailed);

                // 9.3 Set the real previous public key(contains work report);
                prev_pk = ab_upgrade_pk.clone();
            } else {
                ensure!(
                    Self::files_transition_check(
                        &prev_pk,
                        reported_files_size,
                        &added_files,
                        &deleted_files,
                        &reported_files_root
                    ),
                    Error::<T>::IllegalFilesTransition
                );
            }

            // 10. Finish register
            if is_ab_upgrade {
                // 10.1 Transfer A's status to B and delete old A's storage status
                let prev_pk_info = Self::pub_keys(&prev_pk);
                PubKeys::mutate(&curr_pk, |curr_pk_info| {
                    curr_pk_info.anchor = prev_pk_info.anchor;
                });
                Self::chill_pk(&ab_upgrade_pk);
                Self::deposit_event(RawEvent::ABUpgradeSuccess(reporter.clone(), ab_upgrade_pk, curr_pk.clone()));
            } else if is_first_report {
                let mut pk_info = Self::pub_keys(&curr_pk);
                match Self::identities(&reporter) {
                    // 10.2 re-register scenario
                    Some(mut identity) => {
                        Self::chill_anchor(&identity.anchor);
                        identity.anchor = curr_pk.clone();
                        identity.punishment_deadline = NEW_IDENTITY;
                        <Identities<T>>::insert(&reporter, identity);
                    },
                    // 10.3 first register scenario
                    None => {
                        let identity = Identity {
                            anchor: curr_pk.clone(),
                            punishment_deadline: NEW_IDENTITY,
                            group: None
                        };
                        <Identities<T>>::insert(&reporter, identity);
                    }
                }
                pk_info.anchor = Some(curr_pk.clone());
                PubKeys::insert(&curr_pk, pk_info);
            }

            // 11. 🏋🏻 ‍️Merge work report and update corresponding storages, contains:
            // a. Upsert work report
            // b. Judge if it is resuming reporting(recover all sOrders)
            // c. Update sOrders according to `added_files` and `deleted_files`
            // d. Update `report_in_slot`
            // e. Update total spaces(spower and reserved)
            let anchor = Self::pub_keys(&curr_pk).anchor.unwrap();
            Self::maybe_upsert_work_report(
                &reporter,
                &anchor,
                reported_srd_size,
                reported_files_size,
                &added_files,
                &deleted_files,
                &reported_srd_root,
                &reported_files_root,
                slot,
            );

            // 12. Emit work report event
            Self::deposit_event(RawEvent::WorksReportSuccess(reporter.clone(), curr_pk.clone()));

            // 13. Try to free count limitation
            let id = Self::identities(&reporter).unwrap_or_default();
            let owner = if let Some(group) = id.group { group } else { reporter };
            if T::BenefitInterface::maybe_free_count(&owner) {
               return Ok(Pays::No.into());
            }

            Ok(Pays::Yes.into())
        }

        /// Create a group. One account can only create one group once.
        #[weight = T::WeightInfo::create_group()]
        pub fn create_group(
            origin
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;

            // 1. Ensure who didn't report work before
            ensure!(Self::identities(&who).is_none(), Error::<T>::GroupOwnerForbidden);

            // 2. Ensure who is not a group owner right now
            ensure!(!<Groups<T>>::contains_key(&who), Error::<T>::GroupAlreadyExist);

            // 3. Create the group
            <Groups<T>>::insert(&who, Group::<T::AccountId>::default());

            // 4. Emit event
            Self::deposit_event(RawEvent::CreateGroupSuccess(who));

            Ok(())
        }

        #[weight = T::WeightInfo::create_group()]
        pub fn add_member_into_allowlist(
            origin,
            target: <T::Lookup as StaticLookup>::Source
        ) -> DispatchResult {
            let owner = ensure_signed(origin)?;
            let who = T::Lookup::lookup(target)?;

            // 1. Ensure owner's group exist
            ensure!(<Groups<T>>::contains_key(&owner), Error::<T>::NotOwner);

            // 2. Ensure who doesn't in any group right now
            ensure!(Self::identities(&who).is_none() || Self::identities(&who).unwrap().group.is_none(), Error::<T>::AlreadyJoint);

            // 3. Ensure allowlist has the space
            ensure!(Self::groups(&owner).allowlist.len() < T::MaxGroupSize::get() as usize, Error::<T>::ExceedAllowlistLimit);

            // 3. Add who into allowlist
            <Groups<T>>::mutate(&owner, |group| {
                group.allowlist.insert(who.clone());
            });

            // 4. Emit event
            Self::deposit_event(RawEvent::AddIntoAllowlistSuccess(owner, who));

            Ok(())
        }

        #[weight = T::WeightInfo::create_group()]
        pub fn remove_member_from_allowlist(
            origin,
            target: <T::Lookup as StaticLookup>::Source
        ) -> DispatchResult {
            let owner = ensure_signed(origin)?;
            let who = T::Lookup::lookup(target)?;

            // 1. Ensure owner's group exist
            ensure!(<Groups<T>>::contains_key(&owner), Error::<T>::NotOwner);

            // 2. Add who into allowlist
            <Groups<T>>::mutate(&owner, |group| {
                group.allowlist.remove(&who);
            });

            // 3. Emit event
            Self::deposit_event(RawEvent::RemoveFromAllowlistSuccess(owner, who));

            Ok(())
        }

        /// Join a group. The account should already report works once and cannot have any spower value.
        /// The target must be a group owner.
        #[weight = T::WeightInfo::join_group()]
        pub fn join_group(
            origin,
            target: <T::Lookup as StaticLookup>::Source
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;
            let owner = T::Lookup::lookup(target)?;

            // 1. Ensure who has identity information
            ensure!(Self::identities(&who).is_some(), Error::<T>::IdentityNotExist);
            let identity = Self::identities(&who).unwrap();

            // 2. Ensure who didn't join group right now
            ensure!(identity.group.is_none(), Error::<T>::AlreadyJoint);

            // 3. Ensure owner's group exist
            ensure!(<Groups<T>>::contains_key(&owner), Error::<T>::NotOwner);

            // 4. Ensure owner's group has space
            // TODO: remove this check after onboarding benifits module
            ensure!(Self::groups(&owner).members.len() < T::MaxGroupSize::get() as usize, Error::<T>::ExceedGroupLimit);

            // 5. Ensure who is in the allowlist
            ensure!(Self::groups(&owner).allowlist.contains(&who), Error::<T>::NotInAllowlist);

            // 6. Set who's wr's spower to zero
            WorkReports::mutate_exists(&identity.anchor, |maybe_wr| match *maybe_wr {
                Some(WorkReport { ref mut spower, .. }) => {
                    *spower = 0;
                },
                ref mut i => *i = None,
            });

            // 7. Join the group
            <Groups<T>>::mutate(&owner, |group| {
                group.members.insert(who.clone());
                group.allowlist.remove(&who);
            });

            // 8. Mark the group owner
            <Identities<T>>::mutate(&who, |maybe_i| match *maybe_i {
                Some(Identity { ref mut group, .. }) => *group = Some(owner.clone()),
                None => {},
            });

            // 9. Emit event
            Self::deposit_event(RawEvent::JoinGroupSuccess(who, owner));

            Ok(())
        }

        /// Quit a group.
        #[weight = T::WeightInfo::quit_group()]
        pub fn quit_group(
            origin
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;

            // 1. Ensure who has identity information
            ensure!(Self::identities(&who).is_some(), Error::<T>::IdentityNotExist);
            let identity = Self::identities(&who).unwrap();

            // 2. Ensure who joint group before
            ensure!(identity.group.is_some(), Error::<T>::NotJoint);

            let owner = identity.group.unwrap();
            // 3. Ensure owner's group exist
            ensure!(<Groups<T>>::contains_key(&owner), Error::<T>::NotJoint);

            // 4. Remove the group owner
            <Identities<T>>::mutate(&who, |maybe_i| match *maybe_i {
                Some(Identity { ref mut group, .. }) => *group = None,
                None => {},
            });

            // 5. Quit the group
            <Groups<T>>::mutate(&owner, |group| {
                group.members.remove(&who);
            });

            // 6. Reset the work report to no files
            WorkReports::mutate_exists(&identity.anchor, |maybe_wr| match *maybe_wr {
                Some(WorkReport { ref mut spower, ref mut reported_files_size, ref mut reported_files_root, .. }) => {
                    *spower = 0;
                    *reported_files_size = 0;
                    // The total number of 0 is 32
                    *reported_files_root = [
                        0, 0, 0, 0, 0, 0, 0, 0,
                        0, 0, 0, 0, 0, 0, 0, 0,
                        0, 0, 0, 0, 0, 0, 0, 0,
                        0, 0, 0, 0, 0, 0, 0, 0].to_vec();
                },
                ref mut i => *i = None,
            });

            // 7. Emit event
            Self::deposit_event(RawEvent::QuitGroupSuccess(who, owner));

            Ok(())
        }

        /// Kick someone out of this group.
        #[weight = T::WeightInfo::kick_out()]
        pub fn kick_out(
            origin,
            target: <T::Lookup as StaticLookup>::Source
        ) -> DispatchResult {
            let owner = ensure_signed(origin)?;
            let member = T::Lookup::lookup(target)?;

            // 1. Ensure who is a group owner right now
            ensure!(<Groups<T>>::contains_key(&owner), Error::<T>::NotOwner);

            // 2. Ensure who has identity information
            ensure!(Self::identities(&member).is_some(), Error::<T>::IdentityNotExist);
            let identity = Self::identities(&member).unwrap();

            // 3. Ensure member is in the right group
            ensure!(identity.group.is_some() && owner == identity.group.unwrap(), Error::<T>::NotJoint);

            // 4. Remove the group owner
            <Identities<T>>::mutate(&member, |maybe_i| match *maybe_i {
                Some(Identity { ref mut group, .. }) => *group = None,
                None => {},
            });

            // 5. Quit the group
            <Groups<T>>::mutate(&owner, |group| {
                group.members.remove(&member);
            });

            // 6. Emit event
            Self::deposit_event(RawEvent::KickOutSuccess(member));

            Ok(())
        }

        /// Set the punishment flag
        #[weight = 1000]
        pub fn set_punishment(
            origin,
            is_enabled: bool
        ) -> DispatchResult {
            let _ = ensure_root(origin)?;

            EnablePunishment::put(is_enabled);

            Self::deposit_event(RawEvent::SetPunishmentSuccess(is_enabled));
            Ok(())
        }

        // TODO: chill anchor, identity and pk

    }
}

impl<T: Config> Module<T> {
    // PUBLIC MUTABLES
    /// This function is for updating all identities, in details:
    /// 1. call `update_and_get_workload` for every identity, which will return (reserved, spower)
    /// this also (maybe) remove the `outdated` work report
    /// 2. re-calculate `Spower` and `Free`
    /// 3. update `CurrentReportSlot`
    /// 4. call `Works::report_works` interface for every identity
    ///
    /// TC = O(2n)
    /// DB try is 2n+5+Works_DB_try
    fn patial_update_identities(previous_key: Vec<u8>) -> Weight {
        let mut consumed_weight: Weight = 0;
        let mut add_db_reads_writes = |reads, writes| {
            consumed_weight += T::DbWeight::get().reads_writes(reads, writes);
        };
        let prefix = <Identities<T>>::prefix_hash();
        let current_rs = Self::current_report_slot();
        let mut previous_key = previous_key;
        let maybe_to_removed_slot = current_rs.checked_sub(Self::history_slot_depth());
        let enable_punishment = Self::enable_punishment();
        if let Some((mut workload_map, mut total_free, mut total_spower, mut total_reported_files_size)) = Self::workload() {
            // read workload
            add_db_reads_writes(1, 0);
            for _ in 0..IDENTITY_UPDATE_LENGTH {
                if let Some((reporter, mut id)) = Self::next_identity(&prefix, &mut previous_key) {
                    let (free, spower, reported_files_size) = Self::get_workload(&reporter, &mut id, current_rs, enable_punishment);
                    // read identity, work_report and report_in_slot, write identity
                    add_db_reads_writes(3, 1);
                    total_spower = total_spower.saturating_add(spower);
                    total_free = total_free.saturating_add(free);
                    total_reported_files_size = total_reported_files_size.saturating_add(reported_files_size);
                    let mut owner = reporter;
                    if let Some(group) = id.group {
                        owner = group;
                    }
                    let workload = workload_map.get(&owner).unwrap_or(&0u128).saturating_add(spower).saturating_add(free);
                    workload_map.insert(owner, workload);
                    if let Some(to_removed_slot) = maybe_to_removed_slot {
                        ReportedInSlot::remove(&id.anchor, to_removed_slot);
                        // write report_in_slot
                        add_db_reads_writes(0, 1);
                    }
                } else {
                    IdentityPreviousKey::kill();
                    <Workload<T>>::put((workload_map, total_free, total_spower, total_reported_files_size));
                    // write workload
                    add_db_reads_writes(0, 1);
                    return consumed_weight;
                }
            }
            IdentityPreviousKey::put(previous_key);
            <Workload<T>>::put((workload_map, total_free, total_spower, total_reported_files_size));
            // write workload
            add_db_reads_writes(0, 2);
        }
        return consumed_weight;
    }

    pub fn next_identity(prefix: &Vec<u8>, previous_key: &mut Vec<u8>) -> Option<(T::AccountId, Identity<T::AccountId>)> {
        let maybe_next = sp_io::storage::next_key(previous_key).filter(|n| n.starts_with(prefix));
        match maybe_next {
            Some(next) => {
                *previous_key = next;
                match unhashed::get::<Identity<T::AccountId>>(&previous_key) {
                    Some(value) => {
                        let mut key_material = <Identities<T> as StorageMap<T::AccountId, Identity<T::AccountId>>>::Hasher::reverse(&previous_key[prefix.len()..]);
                        match T::AccountId::decode(&mut key_material) {
                            Ok(key) => Some((key, value)),
                            Err(_) => None,
                        }
                    }
                    None => None,
                }
            }
            None => {
                None
            },
        }
    }

    // PRIVATE MUTABLES
    /// This function will insert a new pk
    pub fn insert_pk_info(pk: SworkerPubKey, code: SworkerCode) {
        let pk_info = PKInfo {
            code,
            anchor: None
        };

        PubKeys::insert(pk, pk_info);
    }

    /// This function will (maybe) remove pub_keys
    fn chill_pk(pk: &SworkerPubKey) {
        // 1. Remove from `pub_keys`
        PubKeys::remove(pk);
    }

    /// This function will chill WorkReports and ReportedInSlot
    fn chill_anchor(anchor: &SworkerAnchor) {
        WorkReports::remove(anchor);
        ReportedInSlot::remove_prefix(anchor);
    }

    /// This function will (maybe) update or insert a work report, in details:
    /// 1. calculate spower from reported files
    /// 2. set `ReportedInSlot`
    /// 3. update `Spower` and `Free`
    /// 4. call `Works::report_works` interface
    fn maybe_upsert_work_report(
        reporter: &T::AccountId,
        anchor: &SworkerAnchor,
        reported_srd_size: u64,
        reported_files_size: u64,
        added_files: &Vec<(MerkleRoot, u64, u64)>,
        deleted_files: &Vec<(MerkleRoot, u64, u64)>,
        reported_srd_root: &MerkleRoot,
        reported_files_root: &MerkleRoot,
        report_slot: u64,
    ) {
        let mut old_spower: u64 = 0;
        let mut old_free: u64 = 0;
        let mut old_reported_files_size: u64 = 0;

        // 1. Mark who has reported in this (report)slot
        ReportedInSlot::insert(&anchor, report_slot, true);

        // 2. Update sOrder and get changed size
        // loop added. if not exist, calculate spower.
        // loop deleted, need to check each key whether we should delete it or not
        let (added_files_size, added_files_count)= Self::update_files(reporter, added_files, &anchor, true);
        let (deleted_files_size, _) = Self::update_files(reporter, deleted_files, &anchor, false);

        AddedFilesCount::mutate(|count| {*count = count.saturating_add(added_files_count)});

        // 3. If contains work report
        if let Some(old_wr) = Self::work_reports(&anchor) {
            old_spower = old_wr.spower;
            old_free = old_wr.free;
            old_reported_files_size = old_wr.reported_files_size;
        }

        // 4. Construct work report
        let spower = old_spower.saturating_add(added_files_size).saturating_sub(deleted_files_size);
        let wr = WorkReport {
            report_slot,
            spower,
            free: reported_srd_size,
            reported_files_size,
            reported_srd_root: reported_srd_root.clone(),
            reported_files_root: reported_files_root.clone()
        };

        // 5. Upsert work report
        WorkReports::insert(anchor, wr);

        // 6. Update workload
        let total_free = Self::free().saturating_sub(old_free as u128).saturating_add(reported_srd_size as u128);
        let total_reported_files_size = Self::reported_files_size().saturating_sub(old_reported_files_size as u128).saturating_add(reported_files_size as u128);

        Free::put(total_free);
        ReportedFilesSize::put(total_reported_files_size);
    }

    /// Update sOrder information based on changed files, return the changed_file_size and changed_file_count
    fn update_files(
        reporter: &T::AccountId,
        changed_files: &Vec<(MerkleRoot, u64, u64)>,
        anchor: &SworkerPubKey,
        is_added: bool) -> (u64, u32) {
        let mut changed_files_size: u64 = 0;
        let mut changed_files_count: u32 = 0;

        // 1. Loop changed files
        if is_added {
            for (cid, size, valid_at) in changed_files {
                let mut members = None;
                if let Some(identity) = Self::identities(reporter) {
                    if let Some(owner) = identity.group {
                        members= Some(Self::groups(owner).members);
                    }
                };
                let (added_file_size, is_valid_cid) = T::MarketInterface::upsert_replica(reporter, cid, *size, anchor, TryInto::<u32>::try_into(*valid_at).ok().unwrap(), &members);
                changed_files_size = changed_files_size.saturating_add(added_file_size);
                if is_valid_cid {
                    changed_files_count += 1;
                }
            }
        } else {
            for (cid, _, _) in changed_files {
                // 2. If mapping to storage orders
                let (deleted_file_size, is_valid_cid) = T::MarketInterface::delete_replica(reporter, cid, anchor);
                changed_files_size = changed_files_size.saturating_add(deleted_file_size);
                if is_valid_cid {
                    changed_files_count += 1;
                }
            }
        }
        (changed_files_size, changed_files_count)
    }

    /// Get workload by reporter account,
    /// this function should only be called in the 2nd last session of new era
    /// otherwise, it will be an void in this recursive loop, it mainly includes:
    /// 1. passive check work report: judge if the work report is outdated
    /// 2. (maybe) set corresponding storage order to failed if wr is outdated
    /// 2. return the (reserved, spower) storage of this reporter account
    fn get_workload(reporter: &T::AccountId, id: &mut Identity<T::AccountId>, current_rs: u64, enable_punishment: bool) -> (u128, u128, u128) {
        // Got work report
        if let Some(wr) = Self::work_reports(&id.anchor) {
            if Self::is_fully_reported(reporter, id, current_rs, enable_punishment) {
                return (wr.free as u128, wr.spower as u128, wr.reported_files_size as u128)
            }
        }
        // Or nope, idk wtf? 🙂
        log!(
            debug,
            "🔒 No workload for anchor {:?} in slot {:?}",
            &id.anchor,
            current_rs
        );
        (0, 0, 0)
    }

    pub fn is_fully_reported(reporter: &T::AccountId, id: &mut Identity<T::AccountId>, current_rs: u64, enable_punishment: bool) -> bool {
        // punishment_deadline == "NEW_IDENTITY" => It's the first time to check report in slot.
        // We should ignore it and set punishment_deadline to "NO_PUNISHMENT".
        if id.punishment_deadline == NEW_IDENTITY {
            id.punishment_deadline = NO_PUNISHMENT;
            <Identities<T>>::insert(reporter, id.clone());
            return true;
        }
        // If punishment is disable
        // check whether it's reported in the last report slot
        if !enable_punishment {
            return Self::reported_in_slot(&id.anchor, current_rs);
        }
        if !Self::reported_in_slot(&id.anchor, current_rs) {
            // it should have wr, otherwise punish it again and refresh the deadline.
            id.punishment_deadline = current_rs + (T::PunishmentSlots::get() as u64 * REPORT_SLOT);
            <Identities<T>>::insert(reporter, id.clone());
        }
        if current_rs < id.punishment_deadline {
            // punish it anyway
            return false;
        }
        return true;
    }

    // PRIVATE IMMUTABLES
    /// This function will check work report files status transition
    fn files_transition_check(
        prev_pk: &SworkerPubKey,
        new_files_size: u64,
        reported_added_files: &Vec<(MerkleRoot, u64, u64)>,
        reported_deleted_files: &Vec<(MerkleRoot, u64, u64)>,
        reported_files_root: &MerkleRoot
    ) -> bool {
        if let Some(prev_wr) = Self::work_reports(&Self::pub_keys(&prev_pk).anchor.unwrap_or_default()) {
            let old_files_size = prev_wr.reported_files_size;
            let added_files_size = reported_added_files.iter().fold(0, |acc, (_, size, _)| acc+*size);
            let deleted_files_size = reported_deleted_files.iter().fold(0, |acc, (_, size, _)| acc+*size);
            // File size change should equal between before and after
            return if added_files_size == 0 && deleted_files_size == 0 {
                reported_files_root == &prev_wr.reported_files_root
            } else {
                old_files_size.saturating_add(added_files_size).saturating_sub(deleted_files_size) == new_files_size
            }
        }
        // Or just return for the baby 👶🏼
        true
    }

    fn maybe_get_pk_and_code(
        ias_sig: &IASSig,
        ias_cert: &SworkerCert,
        account_id: &T::AccountId,
        isv_body: &ISVBody,
        sig: &SworkerSignature
    ) -> (Option<Vec<u8>>, Option<Vec<u8>>) {
        let curr_bn = <system::Module<T>>::block_number();
        let legal_codes = <Codes<T>>::iter().filter_map(
            |(key, bn)| {
                if bn > curr_bn {
                    Some(key)
                } else {
                    None
                }
            }
        ).collect();
        let applier = account_id.encode();

        utils::verify_identity(
            ias_sig,
            ias_cert,
            &applier,
            isv_body,
            sig,
            &legal_codes,
        )
    }

    /// This function is judging if the work report sworker code is legal,
    /// return `is_sworker_code_legal`
    fn reporter_code_check(pk: &SworkerPubKey, block_number: u64) -> bool {
        let maybe_expired_bn = Self::codes(Self::pub_keys(pk).code);
        maybe_expired_bn.is_some() && block_number < TryInto::<u64>::try_into(maybe_expired_bn.unwrap()).ok().unwrap()
    }

    fn work_report_timing_check(
        wr_block_number: u64,
        wr_block_hash: &Vec<u8>
    ) -> DispatchResult {
        // 1. Check block hash
        let block_number: T::BlockNumber = wr_block_number.try_into().ok().unwrap();
        let block_hash = <system::Module<T>>::block_hash(block_number)
            .as_ref()
            .to_vec();
        ensure!(
            &block_hash == wr_block_hash,
            "work report hash is illegal"
        );

        // 2. Check work report timing
        ensure!(
            wr_block_number == 1 || wr_block_number == Self::get_current_reported_slot(),
            "work report is outdated or beforehand"
        );

        Ok(())
    }

    fn work_report_sig_check(
        curr_pk: &SworkerPubKey,
        prev_pk: &SworkerPubKey,
        block_number: u64,
        block_hash: &Vec<u8>,
        reserved: u64,
        spower: u64,
        srd_root: &MerkleRoot,
        files_root: &MerkleRoot,
        added_files: &Vec<(MerkleRoot, u64, u64)>,
        deleted_files: &Vec<(MerkleRoot, u64, u64)>,
        sig: &SworkerSignature
    ) -> bool {
        // 1. Encode
        let block_number_bytes = utils::encode_u64_to_string_to_bytes(block_number);
        let reserved_bytes = utils::encode_u64_to_string_to_bytes(reserved);
        let spower_bytes = utils::encode_u64_to_string_to_bytes(spower);
        let added_files_bytes = utils::encode_files(added_files);
        let deleted_files_bytes = utils::encode_files(deleted_files);

        // 2. Construct work report data
        //{
        //    curr_pk: SworkerPubKey,
        //    prev_pk: SworkerPubKey,
        //    block_number: u64, -> Vec<u8>
        //    block_hash: Vec<u8>,
        //    free: u64, -> Vec<u8>
        //    spower: u64, -> Vec<u8>
        //    free_root: MerkleRoot,
        //    spower_root: MerkleRoot,
        //    added_files: Vec<(MerkleRoot, u64, u64)>, -> Vec<u8>
        //    deleted_files: Vec<(MerkleRoot, u64, u64)>, -> Vec<u8>
        //}
        let data: Vec<u8> = [
            &curr_pk[..],
            &prev_pk[..],
            &block_number_bytes[..],
            &block_hash[..],
            &reserved_bytes[..],
            &spower_bytes[..],
            &srd_root[..],
            &files_root[..],
            &added_files_bytes[..],
            &deleted_files_bytes[..]
        ].concat();

        utils::verify_p256_sig(curr_pk, &data, sig)
    }

    fn get_current_block_number() -> BlockNumber {
        let current_block_number = <system::Module<T>>::block_number();
        TryInto::<u32>::try_into(current_block_number).ok().unwrap()
    }

    fn get_current_reported_slot() -> u64 {
        let current_block_numeric = Self::get_current_block_number() as u64;
        let current_report_index = current_block_numeric / REPORT_SLOT;
        current_report_index * REPORT_SLOT
    }

    fn convert_bn_to_rs(curr_bn: u32) -> u64 {
        let report_index = curr_bn as u64 / REPORT_SLOT;
        report_index * REPORT_SLOT
    }
}

decl_event!(
    pub enum Event<T>
    where
        AccountId = <T as system::Config>::AccountId,
        BlockNumber = <T as system::Config>::BlockNumber,
    {
        /// sWorker registration success.
        /// The first item is the account who try to register.
        /// The second item is the pub key of the sWorker.
        RegisterSuccess(AccountId, SworkerPubKey),
        /// Send the work report success.
        /// The first item is the account who send the work report
        /// The second item is the pub key of the sWorker.
        WorksReportSuccess(AccountId, SworkerPubKey),
        /// AB upgrade success.
        /// The first item is the account who're doing AB upgrade.
        /// The second item is the pub key of the previous(A) sWorker.
        /// The third item is the pub key of the latest(B) sWorker.
        ABUpgradeSuccess(AccountId, SworkerPubKey, SworkerPubKey),
        /// Set code success.
        /// The first item is the enclave code.
        /// The second item is the expired block number.
        SetCodeSuccess(SworkerCode, BlockNumber),
        /// Join the group success.
        /// The first item is the member's account.
        /// The second item is the group owner's account.
        JoinGroupSuccess(AccountId, AccountId),
        /// Quit the group success.
        /// The first item is the member's account.
        /// The second item is the group owner's account.
        QuitGroupSuccess(AccountId, AccountId),
        /// Create the group success.
        /// The first item is the group owner's account.
        CreateGroupSuccess(AccountId),
        /// Kick some one out of the group.
        /// The first item is the member's account.
        KickOutSuccess(AccountId),
        /// Cancel the punishment success.
        CancelPunishmentSuccess(AccountId),
        /// Add who into allowlist success.
        AddIntoAllowlistSuccess(AccountId, AccountId),
        /// Remove who from allowlist success.
        RemoveFromAllowlistSuccess(AccountId, AccountId),
        /// Enable the punishment or disable it.
        SetPunishmentSuccess(bool),
        /// Remove the expired code success
        RemoveCodeSuccess(SworkerCode),
    }
);

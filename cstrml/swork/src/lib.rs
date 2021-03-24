// Copyright (C) 2019-2021 Crust Network Technologies Ltd.
// This file is part of Crust.

#![cfg_attr(not(feature = "std"), no_std)]
#![feature(option_result_contains)]

use codec::{Decode, Encode};
use frame_support::{
    decl_event, decl_module, decl_storage, decl_error, ensure,
    dispatch::{DispatchResult, DispatchResultWithPostInfo},
    storage::IterableStorageMap,
    traits::{Currency, ReservableCurrency, Get},
    weights::{
        Weight, DispatchClass, Pays
    }
};
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
    traits::{MarketInterface, SworkerInterface}
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

pub(crate) const LOG_TARGET: &'static str = "swork";

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
    fn upgrade() -> Weight;
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
    pub used: u64, // Real file(mapping with sOrder) size
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

/// An event handler for reporting works
pub trait Works<AccountId> {
    fn report_works(workload_map: BTreeMap<AccountId, u128>, total_workload: u128);
}

impl<AId> Works<AId> for () {
    fn report_works(_: BTreeMap<AId, u128>, _: u128) {}
}

/// Implement market's file inspector
impl<T: Config> SworkerInterface<T::AccountId> for Module<T> {
    /// check wr existing or not
    fn is_wr_reported(anchor: &SworkerAnchor, bn: BlockNumber) -> bool {
        let current_rs = Self::convert_bn_to_rs(bn);
        let prev_rs = current_rs.saturating_sub(REPORT_SLOT);
        Self::reported_in_slot(&anchor, prev_rs)
    }

    /// update the used value due to deleted files, dump trash or calculate_payout
    fn update_used(anchor: &SworkerAnchor, anchor_decrease_used: u64, anchor_increase_used: u64) {
        WorkReports::mutate_exists(anchor, |maybe_wr| match *maybe_wr {
            Some(WorkReport { ref mut used, .. }) => *used = used.saturating_sub(anchor_decrease_used).saturating_add(anchor_increase_used),
            ref mut i => *i = None,
        });
    }

    /// check whether the account id and the anchor is valid or not
    fn check_anchor(who: &T::AccountId, anchor: &SworkerAnchor) -> bool {
        if let Some(identity) = Self::identities(who) {
            return identity.anchor == *anchor;
        }
        false
    }

    /// get total reported files size and free space
    fn get_total_capacity() -> u128 {
        return Self::reported_files_size().saturating_add(Self::free());
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

    /// Weight information for extrinsics in this pallet.
    type WeightInfo: WeightInfo;
}

decl_storage! {
    trait Store for Module<T: Config> as Swork {

        HistorySlotDepth get(fn history_slot_depth): ReportSlot = 6 * REPORT_SLOT;

        /// The sWorker enclave code, this should be managed by sudo/democracy
        pub Code get(fn code) config(): SworkerCode;

        /// The AB upgrade expired block, this should be managed by sudo/democracy
        pub ABExpire get(fn ab_expire): Option<T::BlockNumber>;

        /// The bond relationship between AccountId <-> Identity
        pub Identities get(fn identities):
            map hasher(blake2_128_concat) T::AccountId => Option<Identity<T::AccountId>>;

        /// The sWorker information, mapping from sWorker public key to an optional pubkey information
        pub PubKeys get(fn pub_keys):
            map hasher(twox_64_concat) SworkerPubKey => PKInfo;

        /// The group information
        pub Groups get(fn groups):
            map hasher(blake2_128_concat) T::AccountId => BTreeSet<T::AccountId>;

        /// Node's work report, mapping from sWorker anchor to an optional work report
        /// WorkReport only been replaced, it won't get removed cause we need to check the
        /// status transition from off-chain sWorker
        pub WorkReports get(fn work_reports):
            map hasher(twox_64_concat) SworkerAnchor => Option<WorkReport>;

        /// The current report slot block number, this value should be a multiple of era block
        pub CurrentReportSlot get(fn current_report_slot): ReportSlot = 0;

        /// Recording whether the validator reported works of each era
        /// We leave it keep all era's report info
        /// cause B-tree won't build index on key2(ReportSlot)
        /// value represent if reported in this slot
        /// TODO: reverse the keys when we launch mainnet
        pub ReportedInSlot get(fn reported_in_slot):
            double_map hasher(twox_64_concat) SworkerAnchor, hasher(twox_64_concat) ReportSlot => bool = false;

        /// The used workload, used for calculating stake limit in the end of era
        /// default is 0
        pub Used get(fn used): u128 = 0;

        /// The total reported files workload, used for calculating total_capacity for market module
        /// default is 0
        pub ReportedFilesSize get(fn reported_files_size): u128 = 0;

        /// The free workload, used for calculating stake limit in the end of era
        /// default is 0
        pub Free get(fn free): u128 = 0;

        /// Enable punishment, the default behavior will have punishment.
        pub EnablePunishment get(fn enable_punishment): bool = true;
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
        /// Exceed the maximum bonding relation per account
        ExceedBondsLimit,
        /// Illegal pubkey
        IllegalPubKey,
        /// Identity doesn't exist
        IdentityNotExist,
        /// Already joint one group
        AlreadyJoint,
        /// Not a owner,
        NotOwner,
        /// Used is not zero,
        IllegalUsed,
        /// Group already exist
        GroupAlreadyExist,
        /// Group owner cannot register
        GroupOwnerForbidden,
        /// Member is not in a group
        NotJoint,
        /// Exceed the limit of members number in one group
        ExceedGroupLimit
    }
}

decl_module! {
    pub struct Module<T: Config> for enum Call where origin: T::Origin {
        type Error = Error<T>;

        /// Punishment duration if someone offline
        const PunishmentSlots: u32 = T::PunishmentSlots::get();

        /// Max number of members in one group
        const MaxGroupSize: u32 = T::MaxGroupSize::get();

        // Initializing events
        // this is needed only if you are using events in your module
        fn deposit_event() = default;

        /// Called when a block is initialized. Will call update_identities to update stake limit
        fn on_initialize(now: T::BlockNumber) -> Weight {
            if (now % <T as frame_system::Config>::BlockNumber::from(REPORT_SLOT as u32)).is_zero()  {
			    Self::update_identities();
            }
            // TODO: Recalculate this weight
            0
        }

        /// AB Upgrade, this should only be called by `root` origin
        /// Ruled by `sudo/democracy`
        ///
        /// # <weight>
        /// - O(1)
        /// - 2 DB try
        /// # </weight>
        #[weight = (T::WeightInfo::upgrade(), DispatchClass::Operational)]
        pub fn upgrade(origin, new_code: SworkerCode, expire_block: T::BlockNumber) {
            ensure_root(origin)?;
            <Code>::put(&new_code);
            <ABExpire<T>>::put(&expire_block);
            Self::deposit_event(RawEvent::SworkerUpgradeSuccess(new_code, expire_block));
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
        #[weight = (T::WeightInfo::register(), DispatchClass::Operational)]
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
            let maybe_pk = Self::check_and_get_pk(&ias_sig, &ias_cert, &applier, &isv_body, &sig);
            ensure!(maybe_pk.is_some(), Error::<T>::IllegalIdentity);

            // 4. Insert new pub key info
            let pk = maybe_pk.unwrap();
            Self::insert_pk_info(pk.clone(), Self::code());

            // 5. Emit event
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
        #[weight = (T::WeightInfo::report_works(added_files.len() as u32, deleted_files.len() as u32), DispatchClass::Operational)]
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

            // 1. Ensure reporter is registered
            ensure!(PubKeys::contains_key(&curr_pk), Error::<T>::IllegalReporter);

            // 2. Ensure who cannot be group owner
            ensure!(!<Groups<T>>::contains_key(&reporter), Error::<T>::GroupOwnerForbidden);
            
            // 3. Ensure reporter's code is legal
            ensure!(Self::reporter_code_check(&curr_pk, slot), Error::<T>::OutdatedReporter);

            // 4. Decide which scenario
            let maybe_anchor = Self::pub_keys(&curr_pk).anchor;
            let is_ab_upgrade = maybe_anchor.is_none() && !ab_upgrade_pk.is_empty();
            let is_first_report = maybe_anchor.is_none() && ab_upgrade_pk.is_empty();

            // 5. Unique Check for normal report work for curr pk
            if let Some(anchor) = maybe_anchor {
                // Normally report works.
                // 5.1 Ensure Identity's anchor be same with current pk's anchor
                ensure!(Self::identities(&reporter).unwrap_or_default().anchor == anchor, Error::<T>::IllegalReporter);
                // 5.2 Already reported with same pub key in the same slot, return immediately
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

            // 6. Timing check
            ensure!(Self::work_report_timing_check(slot, &slot_hash).is_ok(), Error::<T>::InvalidReportTime);

            // 7. Ensure sig is legal
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

            // 8. Files storage status transition check
            if is_ab_upgrade {
                // 8.1 Previous pk should already reported works
                ensure!(PubKeys::contains_key(&ab_upgrade_pk), Error::<T>::ABUpgradeFailed);
                // unwrap_or_default is a small tricky solution
                let maybe_prev_wr = Self::work_reports(&Self::pub_keys(&ab_upgrade_pk).anchor.unwrap_or_default());
                ensure!(maybe_prev_wr.is_some(), Error::<T>::ABUpgradeFailed);

                // 8.2 Current work report should NOT be changed at all
                let prev_wr = maybe_prev_wr.unwrap();
                ensure!(added_files.is_empty() &&
                    deleted_files.is_empty() &&
                    prev_wr.reported_files_root == reported_files_root &&
                    prev_wr.reported_srd_root == reported_srd_root,
                    Error::<T>::ABUpgradeFailed);

                // 8.3 Set the real previous public key(contains work report);
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

            // 9. Finish register
            if is_ab_upgrade {
                // 9.1 Transfer A's status to B and delete old A's storage status
                let prev_pk_info = Self::pub_keys(&prev_pk);
                PubKeys::mutate(&curr_pk, |curr_pk_info| {
                    curr_pk_info.anchor = prev_pk_info.anchor;
                });
                Self::chill_pk(&ab_upgrade_pk);
                Self::deposit_event(RawEvent::ABUpgradeSuccess(reporter.clone(), ab_upgrade_pk, curr_pk.clone()));
            } else if is_first_report {
                let mut pk_info = Self::pub_keys(&curr_pk);
                match Self::identities(&reporter) {
                    // 9.2 re-register scenario
                    Some(mut identity) => {
                        Self::chill_anchor(&identity.anchor);
                        identity.anchor = curr_pk.clone();
                        identity.punishment_deadline = slot;
                        <Identities<T>>::insert(&reporter, identity);
                    },
                    // 9.3 first register scenario
                    None => {
                        let identity = Identity {
                            anchor: curr_pk.clone(),
                            punishment_deadline: slot,
                            group: None
                        };
                        <Identities<T>>::insert(&reporter, identity);
                    }
                }
                pk_info.anchor = Some(curr_pk.clone());
                PubKeys::insert(&curr_pk, pk_info);
            }

            // 10. 🏋🏻 ‍️Merge work report and update corresponding storages, contains:
            // a. Upsert work report
            // b. Judge if it is resuming reporting(recover all sOrders)
            // c. Update sOrders according to `added_files` and `deleted_files`
            // d. Update `report_in_slot`
            // e. Update total spaces(used and reserved)
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

            // 11. Emit work report event
            Self::deposit_event(RawEvent::WorksReportSuccess(reporter.clone(), curr_pk.clone()));

            Ok(Pays::No.into())
        }

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
            <Groups<T>>::insert(&who, <BTreeSet<T::AccountId>>::new());

            // 4. Emit event
            Self::deposit_event(RawEvent::CreateGroupSuccess(who));

            Ok(())
        }

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
            ensure!(Self::groups(&owner).len() < T::MaxGroupSize::get() as usize, Error::<T>::ExceedGroupLimit);

            // 5. Ensure who's wr's used is zero
            ensure!(Self::work_reports(identity.anchor).unwrap_or_default().used == 0, Error::<T>::IllegalUsed);

            // 6. Join the group
            <Groups<T>>::mutate(&owner, |members| {
                members.insert(who.clone());
            });

            // 7. Mark the group owner
            <Identities<T>>::mutate(&who, |maybe_i| match *maybe_i {
                Some(Identity { ref mut group, .. }) => *group = Some(owner.clone()),
                None => {},
            });

            // 8. Emit event
            Self::deposit_event(RawEvent::JoinGroupSuccess(who, owner));

            Ok(())
        }

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
            <Groups<T>>::mutate(&owner, |members| {
                members.remove(&who);
            });

            // 6. Emit event
            Self::deposit_event(RawEvent::QuitGroupSuccess(who, owner));

            Ok(())
        }

        #[weight = T::WeightInfo::kick_out()]
        pub fn kick_out(
            origin,
            target: <T::Lookup as StaticLookup>::Source
        ) -> DispatchResult {
            let owner = ensure_signed(origin)?;
            let member = T::Lookup::lookup(target)?;

            // 1. Ensure who is a group owner right now
            ensure!(<Groups<T>>::contains_key(&owner), Error::<T>::NotOwner);

            // 2. Remove the group owner
            <Identities<T>>::mutate(&member, |maybe_i| match *maybe_i {
                Some(Identity { ref mut group, .. }) => *group = None,
                None => {},
            });

            // 3. Quit the group
            <Groups<T>>::mutate(&owner, |members| {
                members.remove(&member);
            });

            // 4. Emit event
            Self::deposit_event(RawEvent::KickOutSuccess(member));

            Ok(())
        }

        #[weight = 1000]
        pub fn cancel_punishment(
            origin,
            target: <T::Lookup as StaticLookup>::Source
        ) -> DispatchResult {
            let _ = ensure_root(origin)?;
            let who = T::Lookup::lookup(target)?;

            // 1. Ensure who has identity information
            ensure!(Self::identities(&who).is_some(), Error::<T>::IdentityNotExist);

            // 2. Cancel the punishment
            <Identities<T>>::mutate(&who, |maybe_i| match *maybe_i {
                Some(Identity { ref mut punishment_deadline, .. }) => *punishment_deadline = 0,
                None => {},
            });

            // 3. Emit event
            Self::deposit_event(RawEvent::CancelPunishmentSuccess(who));

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
    /// 1. call `update_and_get_workload` for every identity, which will return (reserved, used)
    /// this also (maybe) remove the `outdated` work report
    /// 2. re-calculate `Used` and `Free`
    /// 3. update `CurrentReportSlot`
    /// 4. call `Works::report_works` interface for every identity
    ///
    /// TC = O(2n)
    /// DB try is 2n+5+Works_DB_try
    pub fn update_identities() {
        // Ideally, reported_rs should be current_rs + 1
        let reported_rs = Self::get_current_reported_slot();
        let current_rs = Self::current_report_slot();

        // 1. Report slot did not change, it should not trigger updating
        if current_rs == reported_rs {
            return;
        }

        let mut total_used = 0u128;
        let mut total_free = 0u128;
        let mut total_reported_files_size = 0u128;

        log!(
            trace,
            "🔒 Loop all identities and update the workload map for slot {:?}",
            reported_rs
        );
        // 2. Loop all identities and get the workload map
        let mut workload_map= BTreeMap::new();
        // TODO: add check when we launch mainnet
        let to_removed_slot = current_rs.saturating_sub(Self::history_slot_depth());
        for (reporter, mut id) in <Identities<T>>::iter() {
            let (free, used, reported_files_size) = Self::get_workload(&reporter, &mut id, current_rs);
            total_used = total_used.saturating_add(used);
            total_free = total_free.saturating_add(free);
            total_reported_files_size = total_reported_files_size.saturating_add(reported_files_size);
            let mut owner = reporter;
            if let Some(group) = id.group {
                owner = group;
            }
            // TODO: we may need to deal with free and used seperately in the future
            let workload = workload_map.get(&owner).unwrap_or(&0u128).saturating_add(used).saturating_add(free);
            workload_map.insert(owner, workload);
            ReportedInSlot::remove(&id.anchor, to_removed_slot);
        }

        Used::put(total_used);
        Free::put(total_free);
        ReportedFilesSize::put(total_reported_files_size);
        let total_workload = total_used.saturating_add(total_free);

        // 3. Update current report slot
        CurrentReportSlot::mutate(|crs| *crs = reported_rs);

        // 4. Update stake limit for every reporter
        log!(
            trace,
            "🔒 Update stake limit for all reporters."
        );
        T::Works::report_works(workload_map, total_workload);
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
    /// 1. calculate used from reported files
    /// 2. set `ReportedInSlot`
    /// 3. update `Used` and `Free`
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
        let mut old_used: u64 = 0;
        let mut old_free: u64 = 0;
        let mut old_reported_files_size: u64 = 0;
        // 1. Mark who has reported in this (report)slot
        ReportedInSlot::insert(&anchor, report_slot, true);

        // 2. Update sOrder and get changed size
        // loop added. if not exist, calculate used.
        // loop deleted, need to check each key whether we should delete it or not
        let added_files = Self::update_files(reporter, added_files, &anchor, true);
        let deleted_files = Self::update_files(reporter, deleted_files, &anchor, false);

        // 3. If contains work report
        if let Some(old_wr) = Self::work_reports(&anchor) {
            old_used = old_wr.used;
            old_free = old_wr.free;
            old_reported_files_size = old_wr.reported_files_size;
        }

        // 4. Construct work report
        let used = old_used.saturating_add(added_files.iter().fold(0, |acc, (_, f_size, _)| acc + *f_size)).saturating_sub(deleted_files.iter().fold(0, |acc, (_, f_size, _)| acc + *f_size));
        let wr = WorkReport {
            report_slot,
            used,
            free: reported_srd_size,
            reported_files_size,
            reported_srd_root: reported_srd_root.clone(),
            reported_files_root: reported_files_root.clone()
        };

        // 5. Upsert work report
        WorkReports::insert(anchor, wr);

        // 6. Update workload
        let total_used = Self::used().saturating_sub(old_used as u128).saturating_add(used as u128);
        let total_free = Self::free().saturating_sub(old_free as u128).saturating_add(reported_srd_size as u128);
        let total_reported_files_size = Self::reported_files_size().saturating_sub(old_reported_files_size as u128).saturating_add(reported_files_size as u128);

        Used::put(total_used);
        Free::put(total_free);
        ReportedFilesSize::put(total_reported_files_size);
    }

    /// Update sOrder information based on changed files, return the real changed files
    fn update_files(
        reporter: &T::AccountId,
        changed_files: &Vec<(MerkleRoot, u64, u64)>,
        anchor: &SworkerPubKey,
        is_added: bool) -> Vec<(MerkleRoot, u64, u64)> {

        // 1. Loop changed files
        if is_added {
            changed_files.iter().filter_map(|(cid, size, valid_at)| {
                let mut members = None;
                if let Some(identity) = Self::identities(reporter) {
                    if let Some(owner) = identity.group {
                        members= Some(Self::groups(owner));
                    }
                };
                Some((cid.clone(), T::MarketInterface::upsert_replica(reporter, cid, *size, anchor, TryInto::<u32>::try_into(*valid_at).ok().unwrap(), &members), *valid_at))
            }).collect()
        } else {
            let curr_bn = Self::get_current_block_number();
            changed_files.iter().filter_map(|(cid, _, _)| {
                // 2. If mapping to storage orders
                Some((cid.clone(), T::MarketInterface::delete_replica(reporter, cid, anchor), curr_bn as u64))
            }).collect()
        }
    }

    /// Get workload by reporter account,
    /// this function should only be called in the 2nd last session of new era
    /// otherwise, it will be an void in this recursive loop, it mainly includes:
    /// 1. passive check work report: judge if the work report is outdated
    /// 2. (maybe) set corresponding storage order to failed if wr is outdated
    /// 2. return the (reserved, used) storage of this reporter account
    fn get_workload(reporter: &T::AccountId, id: &mut Identity<T::AccountId>, current_rs: u64) -> (u128, u128, u128) {
        // Got work report
        if let Some(wr) = Self::work_reports(&id.anchor) {
            if Self::is_fully_reported(reporter, id, current_rs) {
                return (wr.free as u128, wr.used as u128, wr.reported_files_size as u128)
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

    fn is_fully_reported(reporter: &T::AccountId, id: &mut Identity<T::AccountId>, current_rs: u64) -> bool {
        // If punishment is disable
        // check whether it's reported in the last report slot
        if !Self::enable_punishment() {
            return Self::reported_in_slot(&id.anchor, current_rs);
        }
        if !Self::reported_in_slot(&id.anchor, current_rs) {
            // should have wr, otherwise punish it again and refresh the deadline.
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

    fn check_and_get_pk(
        ias_sig: &IASSig,
        ias_cert: &SworkerCert,
        account_id: &T::AccountId,
        isv_body: &ISVBody,
        sig: &SworkerSignature
    ) -> Option<Vec<u8>> {
        let enclave_code = Self::code();
        let applier = account_id.encode();

        utils::verify_identity(
            ias_sig,
            ias_cert,
            &applier,
            isv_body,
            sig,
            &enclave_code,
        )
    }

    /// This function is judging if the work report sworker code is legal,
    /// return `is_sworker_code_legal`
    fn reporter_code_check(pk: &SworkerPubKey, block_number: u64) -> bool {
        return Self::pub_keys(pk).code == Self::code() ||
            (Self::ab_expire().is_some() && block_number < TryInto::<u64>::try_into(Self::ab_expire().unwrap()).ok().unwrap())
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
        used: u64,
        srd_root: &MerkleRoot,
        files_root: &MerkleRoot,
        added_files: &Vec<(MerkleRoot, u64, u64)>,
        deleted_files: &Vec<(MerkleRoot, u64, u64)>,
        sig: &SworkerSignature
    ) -> bool {
        // 1. Encode
        let block_number_bytes = utils::encode_u64_to_string_to_bytes(block_number);
        let reserved_bytes = utils::encode_u64_to_string_to_bytes(reserved);
        let used_bytes = utils::encode_u64_to_string_to_bytes(used);
        let added_files_bytes = utils::encode_files(added_files);
        let deleted_files_bytes = utils::encode_files(deleted_files);

        // 2. Construct work report data
        //{
        //    curr_pk: SworkerPubKey,
        //    prev_pk: SworkerPubKey,
        //    block_number: u64, -> Vec<u8>
        //    block_hash: Vec<u8>,
        //    free: u64, -> Vec<u8>
        //    used: u64, -> Vec<u8>
        //    free_root: MerkleRoot,
        //    used_root: MerkleRoot,
        //    added_files: Vec<(MerkleRoot, u64, u64)>, -> Vec<u8>
        //    deleted_files: Vec<(MerkleRoot, u64, u64)>, -> Vec<u8>
        //}
        let data: Vec<u8> = [
            &curr_pk[..],
            &prev_pk[..],
            &block_number_bytes[..],
            &block_hash[..],
            &reserved_bytes[..],
            &used_bytes[..],
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
        RegisterSuccess(AccountId, SworkerPubKey),
        WorksReportSuccess(AccountId, SworkerPubKey),
        ABUpgradeSuccess(AccountId, SworkerPubKey, SworkerPubKey),
        ChillSuccess(AccountId, SworkerPubKey),
        SworkerUpgradeSuccess(SworkerCode, BlockNumber),
        JoinGroupSuccess(AccountId, AccountId),
        QuitGroupSuccess(AccountId, AccountId),
        CreateGroupSuccess(AccountId),
        KickOutSuccess(AccountId),
        CancelPunishmentSuccess(AccountId),
        SetPunishmentSuccess(bool),
    }
);

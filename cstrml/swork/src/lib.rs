#![cfg_attr(not(feature = "std"), no_std)]
#![feature(option_result_contains)]

use codec::{Decode, Encode};
use frame_support::{
    decl_event, decl_module, decl_storage, decl_error, ensure,
    dispatch::DispatchResult,
    storage::IterableStorageMap,
    traits::{Currency, ReservableCurrency, Get},
    weights::{
        DispatchClass, constants::WEIGHT_PER_MICROS
    }
};
use sp_runtime::traits::Saturating;
use sp_std::{str, convert::TryInto, prelude::*};
use frame_system::{self as system, ensure_root, ensure_signed};

#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};

// Crust primitives and runtime modules
use primitives::{
    constants::swork::*,
    MerkleRoot, SworkerPubKey, SworkerSignature,
    ReportSlot, BlockNumber, IASSig,
    ISVBody, SworkerCert, SworkerCode
};
use market::{OrderStatus, MarketInterface, OrderInspector};
use sp_std::collections::btree_map::BTreeMap;

/// Provides crypto and other std functions by implementing `runtime_interface`
pub mod api;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(any(feature = "runtime-benchmarks", test))]
pub mod benchmarking;

pub type BalanceOf<T> =
    <<T as Trait>::Currency as Currency<<T as system::Trait>::AccountId>>::Balance;

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

#[derive(Debug, PartialEq, Eq, Clone, Encode, Decode, Default)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct WorkReport {
    /// Timing judgement
    pub report_slot: u64,

    /// Storage information
    pub used: u64, // Real file(mapping with sOrder) size
    pub free: u64,
    pub files: BTreeMap<MerkleRoot, u64>, // Only recorded the sOrder file

    /// Assist judgement
    pub reported_files_size: u64, // Reported files size
    pub reported_srd_root: MerkleRoot, // Srd hash root
    pub reported_files_root: MerkleRoot, // Reported files hash root
}

/// An event handler for reporting works
pub trait Works<AccountId> {
    fn report_works(reporter: &AccountId, own_workload: u128, total_workload: u128);
}

impl<AId> Works<AId> for () {
    fn report_works(_: &AId, _: u128, _: u128) {}
}

/// Implement market's order inspector, bonding with work report
/// and return if the order is legality
impl<T: Trait> OrderInspector<T::AccountId> for Module<T> {
    fn check_works(merchant: &T::AccountId, file_size: u64) -> bool {
        let mut free = 0;

        // Loop and sum all pks
        for pk in Self::id_bonds(merchant) {
            if let Some(wr) = Self::work_reports(pk) {
                // Pruning
                if wr.free > file_size { return true }
                free = free.saturating_add(wr.free);
            }
        }

        if cfg!(feature = "runtime-benchmarks") {
            true
        } else {
            free > file_size
        }
    }
}

/// The module's configuration trait.
pub trait Trait: system::Trait {
    /// The payment balance.
    /// TODO: remove this for abstracting MarketInterface into sWorker self
    type Currency: ReservableCurrency<Self::AccountId>;

    /// The overarching event type.
    type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;

    /// The handler for reporting works.
    type Works: Works<Self::AccountId>;

    /// Interface for interacting with a market module.
    type MarketInterface: MarketInterface<Self::AccountId, Self::Hash, BalanceOf<Self>>;

    /// Max bonds restriction per account.
    type MaxBondsLimit: Get<u32>;
}

decl_storage! {
    trait Store for Module<T: Trait> as Swork {
        /// The sWorker enclave code, this should be managed by sudo/democracy
        pub Code get(fn code) config(): SworkerCode;

        /// The AB upgrade expired block, this should be managed by sudo/democracy
        pub ABExpire get(fn ab_expire): Option<T::BlockNumber>;

        /// The bond relationship between AccountId <-> SworkerPubKeys
        /// e.g. 5HdZ269vAbuoZRK7GT67px6RmwFw2NrWsAbh2wENDqtb5WMN -> ['0x123', '0x456', ...]
        pub IdBonds get(fn id_bonds):
            map hasher(blake2_128_concat) T::AccountId => Vec<SworkerPubKey>;

        /// The sWorker identities, mapping from sWorker public key to an optional identity tuple
        pub Identities get(fn identities):
            map hasher(twox_64_concat) SworkerPubKey => Option<SworkerCode>;

        /// Node's work report, mapping from sWorker public key to an optional work report
        /// WorkReport only been replaced, it won't get removed cause we need to check the
        /// status transition from off-chain sWorker
        pub WorkReports get(fn work_reports):
            map hasher(twox_64_concat) SworkerPubKey  => Option<WorkReport>;

        /// The current report slot block number, this value should be a multiple of era block
        pub CurrentReportSlot get(fn current_report_slot): ReportSlot = 0;

        /// Recording whether the validator reported works of each era
        /// We leave it keep all era's report info
        /// cause B-tree won't build index on key2(ReportSlot)
        /// value represent if reported in this slot
        pub ReportedInSlot get(fn reported_in_slot) :
            double_map hasher(twox_64_concat) SworkerPubKey, hasher(twox_64_concat) ReportSlot => bool = false;

        /// The used workload, used for calculating stake limit in the end of era
        /// default is 0
        pub Used get(fn used): u128 = 0;

        /// The free workload, used for calculating stake limit in the end of era
        /// default is 0
        pub Free get(fn free): u128 = 0;
    }
}

decl_error! {
    /// Error for the swork module.
    pub enum Error for Module<T: Trait> {
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
    }
}

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        /// The maximum bond limitation per account
        const MaxBondsLimit: u32 = T::MaxBondsLimit::get();

        type Error = Error<T>;

        // Initializing events
        // this is needed only if you are using events in your module
        fn deposit_event() = default;

        /// AB Upgrade, this should only be called by `root` origin
        /// Ruled by `sudo/democracy`
        ///
        /// # <weight>
        /// - O(1)
        /// - 2 DB try
        /// # </weight>
        #[weight = 1_000_000]
        pub fn upgrade(origin, new_code: SworkerCode, expire_block: T::BlockNumber) {
            ensure_root(origin)?;
            <Code>::put(new_code);
            <ABExpire<T>>::put(expire_block);
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
        /// Base Weight: 154.8 µs
        /// DB Weight:
        /// - Read: Identities
        /// - Write: 3
        /// # </weight>
        #[weight = (154 * WEIGHT_PER_MICROS + T::DbWeight::get().reads_writes(13, 3), DispatchClass::Operational)]
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

            // 2. Ensure unparsed_identity trusted chain is legal, including signature and sworker code
            let maybe_pk = Self::check_and_get_pk(&ias_sig, &ias_cert, &applier, &isv_body, &sig);
            ensure!(maybe_pk.is_some(), Error::<T>::IllegalIdentity);

            // 3. Ensure `id_bonds` still available
            ensure!((Self::id_bonds(&who).len() as u32) < T::MaxBondsLimit::get(), Error::<T>::ExceedBondsLimit);

            // 4. Upsert new id
            let pk = maybe_pk.unwrap();
            Self::maybe_upsert_id(&applier, &pk, &Self::code());

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
        /// Base Weight: 212 µs
        /// DB Weight:
        /// - Read: Identities, ReportedInSlot, Code, market.Merchant, market.SOrder
        /// - Write: WorkReport, ReportedInSlot, market.SOrder
        /// # </weight>
        #[weight = (212 * WEIGHT_PER_MICROS + T::DbWeight::get().reads_writes(26, 7), DispatchClass::Operational)]
        pub fn report_works(
            origin,
            curr_pk: SworkerPubKey,
            ab_upgrade_pk: SworkerPubKey,
            slot: u64,
            slot_hash: Vec<u8>,
            reported_srd_size: u64,
            reported_files_size: u64,
            added_files: Vec<(MerkleRoot, u64)>,
            deleted_files: Vec<(MerkleRoot, u64)>,
            reported_srd_root: MerkleRoot,
            reported_files_root: MerkleRoot,
            sig: SworkerSignature
        ) -> DispatchResult {
            let reporter = ensure_signed(origin)?;
            let mut prev_pk = curr_pk.clone();

            // 1. Already reported with same pub key in the same slot, return immediately
            if Self::reported_in_slot(&curr_pk, slot) {
                log!(
                    trace,
                    "🔒 Already reported with same pub key {:?} in the same slot {:?}.",
                    curr_pk,
                    slot
                );
                return Ok(())
            }

            // 2. Ensure reporter is registered
            ensure!(Identities::contains_key(&curr_pk), Error::<T>::IllegalReporter);

            // 3. Ensure reporter's code is legal
            ensure!(Self::reporter_code_check(&curr_pk, slot), Error::<T>::OutdatedReporter);

            // 4. Ensure A/B upgrade is legal
            let is_ab_upgrade = !ab_upgrade_pk.is_empty() && Self::work_reports(&curr_pk).is_none();
            if is_ab_upgrade {
                // 4.1 Previous pk should already reported works
                let maybe_prev_wr = Self::work_reports(&ab_upgrade_pk);
                ensure!(maybe_prev_wr.is_some(), Error::<T>::ABUpgradeFailed);

                // 4.2 Current work report should NOT be changed at all
                let prev_wr = maybe_prev_wr.unwrap();
                ensure!(added_files.is_empty() &&
                    deleted_files.is_empty() &&
                    prev_wr.reported_files_root == reported_files_root &&
                    prev_wr.reported_srd_root == reported_srd_root,
                    Error::<T>::ABUpgradeFailed);

                // 4.3 Set the real previous public key(contains work report);
                prev_pk = ab_upgrade_pk.clone();
            }

            // 5. Timing check
            ensure!(Self::work_report_timing_check(slot, &slot_hash).is_ok(), Error::<T>::InvalidReportTime);

            // 6. Ensure sig is legal
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

            // 7. Files storage status transition check
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

            // 8. 🏋🏻 ‍️Merge work report and update corresponding storages, contains:
            // a. Upsert work report
            // b. Judge if it is resuming reporting(recover all sOrders)
            // c. Update sOrders according to `added_files` and `deleted_files`
            // d. Update `report_in_slot`
            // e. Update total spaces(used and reserved)
            // f. [If A/B] Delete A's identity and work report
            Self::maybe_upsert_work_report(
                &reporter,
                &curr_pk,
                &prev_pk,
                reported_srd_size,
                reported_files_size,
                &added_files,
                &deleted_files,
                &reported_srd_root,
                &reported_files_root,
                slot,
                &ab_upgrade_pk
            );

            // 9. Emit work report event
            Self::deposit_event(RawEvent::WorksReportSuccess(reporter.clone(), curr_pk));

            // 10. Emit A/B upgrade event
            if is_ab_upgrade {
                Self::deposit_event(RawEvent::ABUpgradeSuccess(reporter));
            }

            Ok(())
        }
    }
}

impl<T: Trait> Module<T> {
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

        let mut total_used = 0;
        let mut total_free = 0;

        log!(
            trace,
            "🔒 Loop all identities and update the workload map for slot {:?}",
            reported_rs
        );
        // 2. Loop all identities and get the workload map
        // TODO: avoid iterate all identities
        let workload_map: Vec<(T::AccountId, u128)> = <IdBonds<T>>::iter().map(|(reporter, ids)| {
            let mut workload = 0;
            for id in ids {
                // 2.1 Calculate reporter's own reserved and used space
                let (free, used) = Self::get_workload(&reporter, &id, current_rs);

                // 2.2 Update total
                total_used = total_used.saturating_add(used);
                total_free = total_free.saturating_add(free);
                workload = workload.saturating_add(used).saturating_add(free);
            }

            (reporter.clone(), workload)
        }).collect();

        Used::put(total_used);
        Free::put(total_free);
        let total_workload = total_used.saturating_add(total_free);

        // 3. Update current report slot
        CurrentReportSlot::mutate(|crs| *crs = reported_rs);

        // 4. Update stake limit for every reporter
        log!(
            trace,
            "🔒 Update stake limit for all reporters."
        );
        for (reporter, own_workload) in workload_map {
            T::Works::report_works(&reporter, own_workload, total_workload);
        }
    }

    // PRIVATE MUTABLES
    /// This function will (maybe) insert or update a identity, in details:
    /// 1. Add to `identities`
    /// 2. Add to `id_bond`
    pub fn maybe_upsert_id(who: &T::AccountId, pk: &SworkerPubKey, code: &SworkerCode) {
        // 1. Add to `identities`
        Identities::insert(pk, code);

        // 2. Add to `id_bond`
        <IdBonds<T>>::mutate(
            who,
            move |bonds| bonds.push(pk.clone())
        );
    }

    /// This function will (maybe) remove identity, id_bond and work report in details:
    /// 1. Remove `identities`
    /// 2. Remove `id_bond`
    /// 3. Remove `work_report`
    fn chill(who: &T::AccountId, pk: &SworkerPubKey) {
        // 1. Remove from `identities`
        Identities::remove(&pk);

        // 2. Remove from `work_reports`
        WorkReports::remove(&pk);

        // 3. Remove from `id_bonds`
        <IdBonds<T>>::mutate(
            who,
            move |bonds| bonds.retain(|bond| bond != pk)
        );
        if Self::id_bonds(who).is_empty() {
            <IdBonds<T>>::remove(who);
        }
    }

    /// This function will (maybe) update or insert a work report, in details:
    /// 1. calculate used from reported files
    /// 2. set `ReportedInSlot`
    /// 3. update `Used` and `Free`
    /// 4. call `Works::report_works` interface
    fn maybe_upsert_work_report(
        reporter: &T::AccountId,
        curr_pk: &SworkerPubKey,
        prev_pk: &SworkerPubKey,
        reported_srd_size: u64,
        reported_files_size: u64,
        added_files: &Vec<(MerkleRoot, u64)>,
        deleted_files: &Vec<(MerkleRoot, u64)>,
        reported_srd_root: &MerkleRoot,
        reported_files_root: &MerkleRoot,
        report_slot: u64,
        ab_upgrade_pk: &SworkerPubKey
    ) {
        let mut old_used: u128 = 0;
        let mut old_free: u128 = 0;
        let mut files: BTreeMap<MerkleRoot, u64> = BTreeMap::new();

        // 1. If contains work report
        if let Some(old_wr) = Self::work_reports(prev_pk) {
            old_used = old_wr.used as u128;
            old_free = old_wr.free as u128;
            files = old_wr.files.clone();

            // If this is resuming reporting, set all files storage order status to success
            if old_wr.report_slot < report_slot - REPORT_SLOT {
                let old_files: Vec<(MerkleRoot, u64)> = old_wr.files.into_iter().collect();
                let _ = Self::update_sorder(reporter, &old_files, true);
            }
        }

        // 2. Update sOrder and get changed size
        let added_files = Self::update_sorder(reporter, added_files, true);
        let deleted_files = Self::update_sorder(reporter, deleted_files, false);

        // 3. Update files
        for (added_file_id, added_file_size) in added_files {
            files.insert(added_file_id, added_file_size);
        }
        for (deleted_file_id, _) in deleted_files {
            files.remove(&deleted_file_id);
        }

        // 4. Construct work report
        let used = files.iter().fold(0, |acc, (_, f_size)| acc + *f_size);
        let wr = WorkReport {
            report_slot,
            used,
            free: reported_srd_size,
            files,
            reported_files_size,
            reported_srd_root: reported_srd_root.clone(),
            reported_files_root: reported_files_root.clone()
        };

        // 5. Upsert work report
        WorkReports::insert(curr_pk, wr);

        // 6. Mark who has reported in this (report)slot
        ReportedInSlot::insert(curr_pk, report_slot, true);

        // 7. Update workload
        let total_used = Self::used().saturating_sub(old_used).saturating_add(used as u128);
        let total_free = Self::free().saturating_sub(old_free).saturating_add(reported_srd_size as u128);

        Used::put(total_used);
        Free::put(total_free);

        // 8. Delete old A's storage status
        if !ab_upgrade_pk.is_empty() {
            Self::chill(reporter, ab_upgrade_pk);
        }
    }

    /// Update sOrder information based on changed files, return the real changed files
    fn update_sorder(reporter: &T::AccountId, changed_files: &Vec<(MerkleRoot, u64)>, is_added: bool) -> Vec<(MerkleRoot, u64)> {
        let mut real_files = vec![];
        let current_block_numeric = Self::get_current_block_number();

        if let Some(mi) = T::MarketInterface::merchants(reporter) {
            let file_map = mi.file_map;

            // 1. Loop changed files
            real_files = changed_files.iter().filter_map(|(f_id, size)| {
                // 2. If mapping to storage orders
                if let Some(sorder_ids) = file_map.get(f_id) {
                    // a. Loop storage orders(same file)
                    for sorder_id in sorder_ids {
                        if let Some(mut so_status) = T::MarketInterface::maybe_get_sorder_status(sorder_id) {
                            if so_status.status != OrderStatus::Pending && current_block_numeric > so_status.expired_on {
                                continue;
                            }
                            // b. Change sOrder status
                            if !is_added {
                                so_status.status = OrderStatus::Failed;
                            } else if is_added && so_status.status == OrderStatus::Pending {
                                // go panic if `current_block_numeric` > `created_on`
                                so_status.expired_on += current_block_numeric - so_status.completed_on;
                                so_status.completed_on = current_block_numeric;
                                so_status.claimed_at = current_block_numeric;
                                so_status.status = OrderStatus::Success;
                            } else {
                                so_status.status = OrderStatus::Success;
                            }
                            // c. Set sOrder status
                            T::MarketInterface::maybe_set_sorder_status(sorder_id, &so_status, &current_block_numeric);
                        }
                    }
                    Some((f_id.clone(), *size))
                } else {
                    // 3. Or invalid
                    None
                }
            }).collect();
        }

        real_files
    }

    /// Get workload by reporter account,
    /// this function should only be called in the 2nd last session of new era
    /// otherwise, it will be an void in this recursive loop, it mainly includes:
    /// 1. passive check work report: judge if the work report is outdated
    /// 2. (maybe) set corresponding storage order to failed if wr is outdated
    /// 2. return the (reserved, used) storage of this reporter account
    fn get_workload(reporter: &T::AccountId, pk: &SworkerPubKey, current_rs: u64) -> (u128, u128) {
        // Got work report
        if let Some(wr) = Self::work_reports(pk) {
            if Self::reported_in_slot(pk, current_rs) {
                return (wr.free as u128, wr.used as u128)
            } else {
                // If it is the 1st time failed
                if wr.report_slot == current_rs.saturating_sub(REPORT_SLOT) {
                    let files: Vec<(MerkleRoot, u64)> = wr.files.into_iter().collect();
                    let _ = Self::update_sorder(reporter, &files, false);
                }
                // Or is already fucked
            }
        }
        // Or nope, idk wtf? 🙂
        log!(
            debug,
            "🔒 No workload for reporter {:?} in slot {:?}",
            reporter,
            current_rs
        );
        (0, 0)
    }

    // PRIVATE IMMUTABLES
    /// This function will check work report files status transition
    fn files_transition_check(
        prev_pk: &SworkerPubKey,
        new_files_size: u64,
        reported_added_files: &Vec<(MerkleRoot, u64)>,
        reported_deleted_files: &Vec<(MerkleRoot, u64)>,
        reported_files_root: &MerkleRoot
    ) -> bool {
        if let Some(prev_wr) = Self::work_reports(&prev_pk) {
            let old_files_size = prev_wr.reported_files_size;
            let added_files_size = reported_added_files.iter().fold(0, |acc, (_, size)| acc+*size);
            let deleted_files_size = reported_deleted_files.iter().fold(0, |acc, (_, size)| acc+*size);

            // File size change should equal between before and after
            return if old_files_size == new_files_size {
                reported_files_root == &prev_wr.reported_files_root
            } else {
                old_files_size.saturating_add(added_files_size).saturating_sub(deleted_files_size) == new_files_size
            }
        } else {
            // Or just return for the baby 👶🏼
            true
        }
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

        api::crypto::verify_identity(
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
        if let Some(reporter_code) = Self::identities(pk) {
            return reporter_code == Self::code() ||
                (Self::ab_expire().is_some() && block_number <
                    TryInto::<u64>::try_into(Self::ab_expire().unwrap()).ok().unwrap())
        }

        false
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
        added_files: &Vec<(MerkleRoot, u64)>,
        deleted_files: &Vec<(MerkleRoot, u64)>,
        sig: &SworkerSignature
    ) -> bool {
        api::crypto::verify_work_report_sig(
            curr_pk,
            prev_pk,
            block_number,
            block_hash,
            reserved,
            used,
            srd_root,
            files_root,
            added_files,
            deleted_files,
            sig
        )
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
}

decl_event!(
    pub enum Event<T>
    where
        AccountId = <T as system::Trait>::AccountId,
    {
        RegisterSuccess(AccountId, SworkerPubKey),
        WorksReportSuccess(AccountId, SworkerPubKey),
        ABUpgradeSuccess(AccountId),
    }
);

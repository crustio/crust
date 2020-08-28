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

#[derive(Debug, PartialEq, Eq, Clone, Encode, Decode, Default)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct Identity {
    pub pub_key: SworkerPubKey,
    pub code: SworkerCode,
}

#[derive(Debug, PartialEq, Eq, Clone, Encode, Decode, Default)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct WorkReport {
    pub pub_key: SworkerPubKey,
    pub code: SworkerCode,
    pub block_number: u64,
    pub used: u64,
    pub reserved: u64,
    pub files: Vec<(MerkleRoot, u64)>,
    pub reserved_root: MerkleRoot,
    pub files_root: MerkleRoot,
}

/// An event handler for reporting works
pub trait Works<AccountId> {
    fn report_works(controller: &AccountId, own_workload: u128, total_workload: u128);
}

impl<AId> Works<AId> for () {
    fn report_works(_: &AId, _: u128, _: u128) {}
}

/// Implement market's order inspector, bonding with work report
/// and return if the order is legality
impl<T: Trait> OrderInspector<T::AccountId> for Module<T> {
    fn check_works(merchant: &T::AccountId, file_size: u64) -> bool {
        if let Some(wr) = Self::work_reports(merchant) {
              wr.reserved > file_size
        } else {
            if cfg!(feature = "runtime-benchmarks"){
                true
            } else {
                false
            }
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
}

decl_storage! {
    trait Store for Module<T: Trait> as Swork {
        /// The sWorker enclave code, this should be managed by sudo/democracy
        pub Code get(fn code) config(): SworkerCode;

        /// The AB upgrade expired block, this should be managed by sudo/democracy
        pub ABExpire get(fn ab_expire): Option<T::BlockNumber>;

        /// The sWorker identities, mapping from controller to an optional identity tuple
        pub Identities get(fn identities) config():
            map hasher(blake2_128_concat) T::AccountId => Option<Identity>;

        /// Node's work report, mapping from controller to an optional work report
        pub WorkReports get(fn work_reports) config():
            map hasher(blake2_128_concat) T::AccountId  => Option<WorkReport>;

        /// The current report slot block number, this value should be a multiple of era block
        pub CurrentReportSlot get(fn current_report_slot) config(): ReportSlot;

        /// Recording whether the validator reported works of each era
        /// We leave it keep all era's report info
        /// cause B-tree won't build index on key2(ReportSlot)
        /// value represent if reported in this slot
        pub ReportedInSlot get(fn reported_in_slot) build(|config: &GenesisConfig<T>| {
            config.work_reports.iter().map(|(account_id, _)|
                (account_id.clone(), 0, true)
            ).collect::<Vec<_>>()
        }): double_map hasher(twox_64_concat) T::AccountId, hasher(twox_64_concat) ReportSlot
        => bool = false;

        /// The used workload, used for calculating stake limit in the end of era
        /// default is 0
        pub Used get(fn used): u128 = 0;

        /// The reserved workload, used for calculating stake limit in the end of era
        /// default is 0
        pub Reserved get(fn reserved): u128 = 0;
    }
}

decl_error! {
    /// Error for the swork module.
    pub enum Error for Module<T: Trait> {
        /// Illegal applier
        IllegalApplier,
        /// Duplicate identity
        DuplicateId,
        /// Identity check failed
        IllegalTrustedChain,
        /// Illegal reporter
        IllegalReporter,
        /// Invalid public key
        InvalidPubKey,
        /// Invalid timing
        InvalidReportTime,
        /// Illegal work report signature
        IllegalWorkReportSig,
        /// Illegal upgrade work report
        IllegalUpgradeWorkReport
    }
}

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
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
        /// The dispatch origin for this call must be _Signed_ by the controller account.
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

            // 2. Ensure unparsed_identity trusted chain id legal
            let maybe_pk = Self::check_and_get_pk(&ias_sig, &ias_cert, &applier, &isv_body, &sig);
            ensure!(maybe_pk.is_some(), Error::<T>::IllegalTrustedChain);

            // 3. Ensure public key is unique
            let pk = maybe_pk.unwrap();
            ensure!(Self::id_is_unique(&pk), Error::<T>::DuplicateId);

            // 4. Construct the identity
            let current_code = Self::code();
            let identity = Identity {
                pub_key: pk,
                code: current_code.clone()
            };

            // 5. Upsert applier
            <Identities<T>>::insert(&who, identity);
            Self::deposit_event(RawEvent::RegisterSuccess(who, current_code));

            Ok(())
        }

        /// Report storage works from sWorker
        /// All `inputs` can only be generated from sWorker's enclave
        ///
        /// The dispatch origin for this call must be _Signed_ by the controller account.
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
            pub_key: SworkerPubKey,
            block_number: u64,
            block_hash: Vec<u8>,
            reserved: u64,
            files: Vec<(MerkleRoot, u64)>,
            reserved_root: MerkleRoot,
            files_root: MerkleRoot,
            sig: SworkerSignature
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;

            // 1. Ensure reporter is verified
            ensure!(<Identities<T>>::contains_key(&who), Error::<T>::IllegalReporter);

            // 2. Ensure reporter's id is valid
            ensure!(Self::wr_pub_key_check(&who, &pub_key), Error::<T>::InvalidPubKey);

            // 3. Do timing check
            ensure!(Self::wr_timing_check(block_number, &block_hash).is_ok(), Error::<T>::InvalidReportTime);

            // 4. Do sig check
            ensure!(
                Self::wr_sig_check(&pub_key, block_number, &block_hash, reserved, &files, &reserved_root, &files_root, &sig),
                Error::<T>::IllegalWorkReportSig
            );

            // 5. Check work report upgrade
            let id = Self::identities(who).unwrap();
            let id_code = id.code;
            ensure!(
                Self::wr_upgrade_check(&who, &id_code, &reserved_root, &files_root),
                Error::<T>::IllegalUpgradeWorkReport
            );

            // 6. Construct work report
            // Identity must exist, otherwise it will failed at step2
            let work_report = WorkReport {
                pub_key: pub_key.clone(),
                code: id_code.clone(),
                block_number,
                used: 0,
                reserved,
                files: files.clone(),
                reserved_root: reserved_root.clone(),
                files_root: files_root.clone()
            };

            // 7. Maybe upsert work report
            if Self::maybe_upsert_work_report(&who, &work_report) {
                // Emit report works event
                Self::deposit_event(RawEvent::WorksReportSuccess(who, work_report));
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
    /// 2. re-calculate `Used` and `Reserved`
    /// 3. update `CurrentReportSlot`
    /// 4. call `Works::report_works` interface for every identity
    ///
    /// TC = O(2n)
    /// DB try is 2n+5+Works_DB_try
    pub fn update_identities() {
        // TODO: do this in previous session instead of end era.
        // Ideally, reported_rs should be current_rs + 1
        let reported_rs = Self::get_reported_slot();
        let current_rs = Self::current_report_slot();
        // 1. Report slot did not change, it should not trigger updating
        if current_rs == reported_rs {
            return;
        }

        // 2. Update id's work report, get id's workload and total workload
        let mut total_used = 0;
        let mut total_reserved = 0;

        // TODO: avoid iterate all identities
        let workload_map: Vec<(T::AccountId, u128)> = <Identities<T>>::iter().map(|(controller, _)| {
            // a. calculate this controller's order file map
            // TC = O(nm), `n` is stored files number and `m` is corresponding order ids
            let mut success_sorder_files: Vec<(MerkleRoot, T::Hash)> = vec![];
            let mut ongoing_sorder_ids: Vec<T::Hash> = vec![];
            let mut overdue_sorder_ids: Vec<T::Hash> = vec![];

            if let Some(minfo) = T::MarketInterface::merchants(&controller) {
                for (f_id, order_ids) in minfo.file_map.iter() {
                    for order_id in order_ids {
                        // Get order status(should exist) and (maybe) change the status
                        let sorder =
                            T::MarketInterface::maybe_get_sorder(order_id).unwrap_or_default();
                        if sorder.status == OrderStatus::Success {
                            success_sorder_files.push((f_id.clone(), order_id.clone()))
                        }
                        ongoing_sorder_ids.push(order_id.clone());
                        if Self::get_current_block_number() >= sorder.expired_on {
                            // TODO: add extra punishment logic when we close overdue sorder
                            overdue_sorder_ids.push(order_id.clone());
                        }
                    }
                }
            }

            // b. calculate controller's own reserved and used space
            // We should first update the sorder's status, then do the punishment
            let (reserved, used) = Self::update_and_get_workload(&controller, &success_sorder_files, current_rs);

            // c. do punishment
            for order_id in ongoing_sorder_ids {
                T::MarketInterface::maybe_punish_merchant(&order_id);
            }
            // d. close overdue storage order
            for order_id in overdue_sorder_ids {
                T::MarketInterface::close_sorder(&order_id);
            }

            // e. add to total
            total_used += used;
            total_reserved += reserved;

            // f. return my own to construct workload map
            (controller.clone(), used + reserved)
        }).collect();

        Used::put(total_used);
        Reserved::put(total_reserved);
        let total_workload = total_used + total_reserved;

        // 3. Update current report slot
        CurrentReportSlot::mutate(|crs| *crs = reported_rs);

        // 4. Update stake limit
        for (controller, own_workload) in workload_map {
            T::Works::report_works(&controller, own_workload, total_workload);
        }
    }

    // PRIVATE MUTABLES

    /// This function will (maybe) update or insert a work report, in details:
    /// 1. calculate used from reported files
    /// 2. set `ReportedInSlot`
    /// 3. update `Used` and `Reserved`
    /// 4. call `Works::report_works` interface
    fn maybe_upsert_work_report(who: &T::AccountId, wr: &WorkReport) -> bool {
        let mut old_used: u128 = 0;
        let mut old_reserved: u128 = 0;
        let rs = Self::get_reported_slot();

        // 1. Judge if wr exists
        if let Some(old_wr) = Self::work_reports(who) {
            if &old_wr == wr {
                return false;
            } else {
                old_used = old_wr.used as u128;
                old_reserved = old_wr.reserved as u128;
            }
        }

        // 2. Calculate used space
        // TC = O(M*logN), N is file_map's key number, M is same file's orders number
        // 2M DB try
        let mut updated_wr = wr.clone();
        let file_map = T::MarketInterface::merchants(who).unwrap_or_default().file_map;
        updated_wr.used = wr.files.iter().fold(0, |used, (f_id, f_size)| {
            // TODO: Abstract and make this logic be a separated function
            if let Some(order_ids) = file_map.get(f_id) {
                for order_id in order_ids {
                    // Get order status(should exist) and (maybe) change the status
                    let mut sorder =
                        T::MarketInterface::maybe_get_sorder(order_id).unwrap_or_default();

                    // TODO: we should specially handle `Failed` status
                    if sorder.status != OrderStatus::Success {
                        // 1. Reset `expired_on` and `completed_on` for new order
                        if sorder.status == OrderStatus::Pending {
                            let current_block_numeric = Self::get_current_block_number();
                            // go panic if `current_block_numeric` > `created_on`
                            sorder.expired_on += current_block_numeric - sorder.created_on;
                            sorder.completed_on = current_block_numeric;
                        }

                        // 2. Change order status to `Success`
                        sorder.status = OrderStatus::Success;

                        // 3. Set sorder status and (Maybe) start delay pay
                        T::MarketInterface::maybe_set_sorder(order_id, &sorder);
                    }
                }
                // Only plus once
                return used + *f_size
            }
            used
        });

        // 3. Upsert work report
        <WorkReports<T>>::insert(who, &updated_wr);

        // 4. Mark reported in this slot
        <ReportedInSlot<T>>::insert(who, rs, true);

        // 5. Update workload
        let used = updated_wr.used as u128;
        let reserved = updated_wr.reserved as u128;
        let total_used = Self::used() - old_used + used;
        let total_reserved = Self::reserved() - old_reserved + reserved;

        Used::put(total_used);
        Reserved::put(total_reserved);

        true
    }

    /// Get updated workload by controller account,
    /// this function should only be called in the new era
    /// otherwise, it will be an void in this recursive loop, it mainly includes:
    /// 1. passive check according to market order, it (maybe) update `used` and `order_status`;
    /// 2. (maybe) remove outdated work report
    /// 3. return the (reserved, used) storage of this controller account
    fn update_and_get_workload(controller: &T::AccountId, order_map: &Vec<(MerkleRoot, T::Hash)>, current_rs: u64) -> (u128, u128) {
        let mut wr_files: Vec<MerkleRoot> = vec![];
        let mut works: (u128, u128) = (0, 0);

        // Judge if this controller reported works in this current era
        if let Some(wr) = Self::work_reports(controller) {
            let reported = Self::reported_in_slot(controller, current_rs);
            if reported {
                // 1. Get all work report files
                wr_files = wr.files.iter().map(|(f_id, _)| f_id.clone()).collect();

                // 2. Get reserved and used
                works =  (wr.reserved as u128, wr.used as u128)
            } else {
                // 2. Remove work report when it is outdated
                if wr.block_number < current_rs {
                    <WorkReports<T>>::remove(controller);
                }
            }
        }
        // Or work report not exist at all

        // 3. Check every order files are stored by controller
        for (f_id, order_id) in order_map {
            if !wr_files.contains(f_id) {
                // Set status to failed, sorder.status should be successful before.
                let mut sorder =
                    T::MarketInterface::maybe_get_sorder(order_id).unwrap_or_default();
                sorder.status = OrderStatus::Failed;
                T::MarketInterface::maybe_set_sorder(order_id, &sorder);         
            }
        }

        return works
    }

    // PRIVATE IMMUTABLES

    /// This function will merge SetA(`files_a`) and SetB(`files_b`)
    /// TODO: Uncomment when master-slave mechanism enable
    /*fn merged_files(files_a: &Vec<(MerkleRoot, u64)>,
                    files_b: &Vec<(MerkleRoot, u64)>) -> Vec<(MerkleRoot, u64)> {

        let mut root_hashes: BTreeSet<MerkleRoot> = BTreeSet::new();
        let mut unmerged_files = files_a.clone();
        unmerged_files.extend(files_b.clone());
        let mut merged_files: Vec<(MerkleRoot, u64)> = vec![];

        for (root_hash, size) in unmerged_files {
            if !root_hashes.contains(&root_hash) {
                merged_files.push((root_hash.clone(), size));
                root_hashes.insert(root_hash.clone());
            }
        }

        merged_files
    }*/

    /// This function is judging if the identity already be registered
    /// TC is O(n)
    /// DB try is O(1)
    fn id_is_unique(pk: &SworkerPubKey) -> bool {
        let mut is_unique = true;

        for (_, maybe_id) in <Identities<T>>::iter() {
            if let Some(id) = maybe_id {
                if &id.pub_key == pk {
                    is_unique = false;
                    break
                }
            }
        }

        is_unique
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

    /// This function is judging if the work report's pub key is legal,
    /// Only `true` with:
    /// 1. pub_key matches
    /// 2. code is correct or upgrade not expired
    fn wr_pub_key_check(reporter: &T::AccountId, wr_pk: &SworkerPubKey) -> bool {
        if let Some(id) = Self::identities(reporter) {
            let code: SworkerCode = Self::code();
            let current_bn = <system::Module<T>>::block_number();
            let not_expired = Self::ab_expire().is_some() && current_bn < Self::ab_expire().unwrap();

            return wr_pk == &id.pub_key && (&id.code == &code || not_expired)
        }
        false
    }

    /// This function is judging if the work report's timing is right,
    /// Only `true` with:
    /// 1. block hash matches block number
    /// 2. block number == current report slot
    fn wr_timing_check(
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
            wr_block_number == 1 || wr_block_number == Self::get_reported_slot(),
            "work report is outdated or beforehand"
        );

        Ok(())
    }

    /// This function is judging if the work report's timing is right,
    /// Only `true` with: enclave sig is legal
    fn wr_sig_check(
        pub_key: &SworkerPubKey,
        block_number: u64,
        block_hash: &Vec<u8>,
        reserved: u64,
        files: &Vec<(MerkleRoot, u64)>,
        reserved_root: &MerkleRoot,
        files_root: &MerkleRoot,
        sig: &SworkerSignature
    ) -> bool {
        api::crypto::verify_work_report_sig(
            pub_key,
            block_number,
            block_hash,
            reserved,
            files,
            reserved_root,
            files_root,
            sig,
        )
    }

    /// This function is judging if the work report's timing is right,
    /// Only `false` with: `pub_key` changed and {reserved, files}_root_hash changed
    fn wr_upgrade_check(
        reporter: &T::AccountId,
        code: &SworkerCode,
        reserved_root: &MerkleRoot,
        files_root: &MerkleRoot
    ) -> bool {
        if let Some(wr) = Self::work_reports(reporter) {
            return code == &wr.code || (reserved_root == &wr.reserved_root && files_root == &wr.files_root)
        }
        true
    }

    fn get_current_block_number() -> BlockNumber {
        let current_block_number = <system::Module<T>>::block_number();
        TryInto::<u32>::try_into(current_block_number).ok().unwrap()
    }

    fn get_reported_slot() -> u64 {
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
        RegisterSuccess(AccountId, SworkerCode),
        WorksReportSuccess(AccountId, WorkReport),
    }
);

#![cfg_attr(not(feature = "std"), no_std)]
#![feature(option_result_contains)]

use codec::{Decode, Encode};
use frame_support::{
    decl_event, decl_module, decl_storage, decl_error, ensure,
    dispatch::DispatchResult,
    storage::IterableStorageMap,
    traits::{Currency, ReservableCurrency}
};
use sp_std::{str, convert::TryInto, prelude::*};
use system::{ensure_root, ensure_signed};

#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};

// Crust primitives and runtime modules
use primitives::{
    constants::tee::*,
    MerkleRoot, PubKey, TeeSignature,
    ReportSlot, BlockNumber, IASSig,
    ISVBody, Cert, TeeCode
};
use market::{OrderStatus, MarketInterface, OrderInspector};
use frame_support::storage::unhashed::get_or_else;

/// Provides crypto and other std functions by implementing `runtime_interface`
pub mod api;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

pub type BalanceOf<T> =
    <<T as Trait>::Currency as Currency<<T as system::Trait>::AccountId>>::Balance;

#[derive(Debug, PartialEq, Eq, Clone, Encode, Decode, Default)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct Identity {
    pub pub_key: PubKey,
    pub code: TeeCode,
}

#[derive(Debug, PartialEq, Eq, Clone, Encode, Decode, Default)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct WorkReport {
    pub block_number: u64,
    pub used: u64,
    pub reserved: u64,
    pub cached_reserved: u64,
    pub files: Vec<(MerkleRoot, u64)>,
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
    fn check_works(provider: &T::AccountId, file_size: u64) -> bool {
        if let Some(wr) = Self::work_reports(provider) {
              wr.reserved > file_size
        } else {
            false
        }
    }
}

/// The module's configuration trait.
pub trait Trait: system::Trait {
    /// The payment balance.
    /// TODO: remove this for abstracting MarketInterface into tee self
    type Currency: ReservableCurrency<Self::AccountId>;

    /// The overarching event type.
    type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;

    /// The handler for reporting works.
    type Works: Works<Self::AccountId>;

    /// Interface for interacting with a market module.
    type MarketInterface: MarketInterface<Self::AccountId, Self::Hash, BalanceOf<Self>>;
}

decl_storage! {
    trait Store for Module<T: Trait> as Tee {
        /// The TEE enclave code, this should be managed by sudo/democracy
        pub Code get(fn code) config(): TeeCode;

        /// The AB upgrade expired block, this should be managed by sudo/democracy
        pub ABExpire get(fn ab_expire): Option<T::BlockNumber>;

        /// The TEE identities, mapping from controller to an optional identity tuple
        /// (elder_id, current_id) = (before-upgrade identity, upgraded identity)
        pub Identities get(fn identities) config():
            map hasher(blake2_128_concat) T::AccountId => (Option<Identity>, Option<Identity>);

        /// Node's work report, mapping from controller to an optional work report
        pub WorkReports get(fn work_reports) config():
            map hasher(blake2_128_concat) T::AccountId  => Option<WorkReport>;

        /// The current report slot block number, this value should be a multiple of era block
        pub CurrentReportSlot get(fn current_report_slot) config(): ReportSlot;

        /// Recording whether the validator reported works of each era
        /// We leave it keep all era's report info
        /// cause B-tree won't build index on key2(ReportSlot)
        /// value (bool, bool) represent two id (elder_reported, current_reported)
        pub ReportedInSlot get(fn reported_in_slot) build(|config: &GenesisConfig<T>| {
            config.work_reports.iter().map(|(account_id, _)|
                (account_id.clone(), 0, (false, true))
            ).collect::<Vec<_>>()
        }): double_map hasher(twox_64_concat) T::AccountId, hasher(twox_64_concat) ReportSlot
        => (bool, bool) = (false, false);

        /// The used workload, used for calculating stake limit in the end of era
        /// default is 0
        pub Used get(fn used): u128 = 0;

        /// The reserved workload, used for calculating stake limit in the end of era
        /// default is 0
        pub Reserved get(fn reserved): u128 = 0;
    }
}

decl_error! {
    /// Error for the tee module.
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
        IllegalWorkReportSig
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
        pub fn upgrade(origin, new_code: TeeCode, expire_block: T::BlockNumber) {
            ensure_root(origin)?;

            <Code>::put(new_code);
            <ABExpire<T>>::put(expire_block);
        }

        /// Register as new trusted node
        ///
        /// # <weight>
        /// - O(n)
        /// - 3 DB try
        /// # </weight>
        #[weight = 1_000_000]
        pub fn register(
            origin,
            ias_sig: IASSig,
            ias_cert: Cert,
            applier: T::AccountId,
            isv_body: ISVBody,
            sig: TeeSignature
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
            let identity = Identity {
                pub_key: pk,
                code: Self::code()
            };

            // 5. Applier is new add or needs to be updated
            if Self::maybe_upsert_id(&applier, &identity) {
                // Emit event
                Self::deposit_event(RawEvent::RegisterSuccess(who));
            }

            Ok(())
        }

        /// Register as new trusted tee node
        ///
        /// # <weight>
        /// - O(2n)
        /// - 3n+8 DB try
        /// # </weight>
        #[weight = 1_000_000]
        pub fn report_works(
            origin,
            pub_key: PubKey,
            block_number: u64,
            block_hash: Vec<u8>,
            reserved: u64,
            files: Vec<(MerkleRoot, u64)>,
            sig: TeeSignature
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;

            // 1. Ensure reporter is verified
            ensure!(<Identities<T>>::contains_key(&who), Error::<T>::IllegalReporter);

            // 2. Ensure reporter's id is valid
            let (is_elder_report, is_current_report) = Self::work_report_id_check(&who, &pub_key);
            ensure!(is_elder_report || is_current_report, Error::<T>::InvalidPubKey);

            // 3. Ensure this work report has not been reported before
            let (elder_reported, current_reported) = Self::reported_in_slot(&who, block_number);
            // TODO: Pass the duplicate reported

            // 4. Do timing check
            ensure!(Self::work_report_timing_check(block_number, &block_hash).is_ok(), Error::<T>::InvalidReportTime);

            // 5. Do sig check
            ensure!(
                Self::work_report_sig_check(&pub_key, block_number, &block_hash, reserved, &files, &sig),
                Error::<T>::IllegalWorkReportSig
            );

            // 6. Construct work report
            let work_report = Self:merged_work_report(&who, reserved, &files, block_number, is_elder_report, is_current_report);

            // 7. Maybe upsert work report
            if Self::maybe_upsert_work_report(&who, &work_report) {
                // 8. Emit report works event
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
            let mut order_files: Vec<(MerkleRoot, T::Hash)> = vec![];
            if let Some(provision) = T::MarketInterface::providers(&controller) {
                for (f_id, order_ids) in provision.file_map.iter() {
                    for order_id in order_ids {
                        // Get order status(should exist) and (maybe) change the status
                        let sorder =
                            T::MarketInterface::maybe_get_sorder(order_id).unwrap_or_default();
                        if sorder.status == OrderStatus::Success {
                            order_files.push((f_id.clone(), order_id.clone()))
                        }
                    }
                }
            }

            // b. calculate controller's own reserved and used space
            let (reserved, used) = Self::update_and_get_workload(&controller, &order_files, current_rs);

            // c. add to total
            total_used += used;
            total_reserved += reserved;

            // d. return my own to construct workload map
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
    /// This function will (maybe) insert or update a identity, in details:
    /// 0. Do nothing if `current_id` == `id`
    /// 1. Update `current_id` if `current_id.code` == `id.code`
    /// 2. Update `current_id` and `elder_id` if `current_id.code` != `id.code`
    fn maybe_upsert_id(who: &T::AccountId, id: &Identity) -> bool {
        let maybe_ids = Self::identities(who);
        let upserted = match maybe_ids {
            // New id
            (None, None) => {
                Identities::<T>::insert(who, (None, id.clone()));
                true
            },
            // Update/upgrade id
            (_, Some(current_id)) => {
                // Duplicate identity
                if &current_id == id {
                    false
                } else {
                    if current_id.code == id.code {
                        // Update(enclave code not change)
                        Identity::<T>::mutate(who, |(_, maybe_cid)| {
                            if let Some(cid) = maybe_cid {
                                cid = current_id.clone();
                            }
                        })
                    } else {
                        // Upgrade(new enclave code detected)
                        // current_id -> elder_id
                        // id         -> current_id
                        Identities::<T>::insert(who, (Some(current_id), Some(id)));
                    }
                    true
                }
            },
        };

        upserted
    }

    /// This function will (maybe) update or insert a work report, in details:
    /// 1. calculate used from reported files
    /// 2. set `ReportedInSlot`
    /// 3. update `Used` and `Reserved`
    /// 4. call `Works::report_works` interface
    fn maybe_upsert_work_report(who: &T::AccountId, wr: &WorkReport, elder_reported: bool, current_reported: bool) -> bool {
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
        let file_map = T::MarketInterface::providers(who).unwrap_or_default().file_map;
        updated_wr.used = wr.files.iter().fold(0, |used, (f_id, f_size)| {
            // TODO: Abstract and make this logic be a seperated function
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
                return used + *f_size
            }
            used
        });

        // 3. Upsert work report
        <WorkReports<T>>::insert(who, &updated_wr);

        // 4. Mark who has reported in this (report)slot, and which public key did he/she used
        <ReportedInSlot<T>>::mutate(who, rs, |(e, c)| {
            *e = e || elder_reported;
            *c = c || current_reported;
        });

        // 5. Update workload
        let used = updated_wr.used as u128;
        let reserved = updated_wr.reserved as u128;
        let total_used = Self::used() - old_used + used;
        let total_reserved = Self::reserved() - old_reserved + reserved;

        Used::put(total_used);
        Reserved::put(total_reserved);
        let total_workload = total_used + total_reserved;

        // 6. Update work report for every identity
        // TC = O(N)
        // N DB try
        for (controller, wr) in <WorkReports<T>>::iter() {
            T::Works::report_works(
                &controller,
                (wr.used + wr.reserved) as u128,
                total_workload
            );
        }
        true
    }

    /// Get updated workload by controller account,
    /// this function should only be called in the new era
    /// otherwise, it will be an void in this recursive loop, it mainly includes:
    /// 1. passive check according to market order, it (maybe) update `used` and `order_status`;
    /// 2. (maybe) remove outdated work report
    /// 3. return the (reserved, used) storage of this controller account
    fn update_and_get_workload(controller: &T::AccountId, order_map: &Vec<(MerkleRoot, T::Hash)>, current_rs: u64) -> (u128, u128) {
        // Judge if this controller reported works in this current era
        if let Some(wr) = Self::work_reports(controller) {
            let (elder_reported, current_reported) = Self::reported_in_slot(controller, current_rs);
            if elder_reported || current_reported {
                // 1. Get all work report files
                let wr_files: Vec<MerkleRoot> = wr.files.iter().map(|(f_id, _)| f_id.clone()).collect();

                // 2. Check every order files are stored by controller
                for (f_id, order_id) in order_map {
                    if !wr_files.contains(f_id) {
                        // 3. Set status to failed
                        let mut sorder =
                            T::MarketInterface::maybe_get_sorder(order_id).unwrap_or_default();
                        sorder.status = OrderStatus::Failed;
                        T::MarketInterface::maybe_set_sorder(order_id, &sorder);
                    }
                    T::MarketInterface::maybe_punish_provider(order_id);
                }

                // 3. Return reserved and used
                (wr.reserved as u128, wr.used as u128)
            } else {
                // Remove work report when it is outdated
                if wr.block_number < current_rs {
                    <WorkReports<T>>::remove(controller);
                }
                (0, 0)
            }
        } else {
            (0, 0)
        }
    }

    // PRIVATE IMMUTABLES
    /// This function will generated the merged work report, merging includes:
    /// 1. `files`: merged with same block number, covered with different block number
    /// 2. `cached_reserved`: valued only `elder_reported == true` and `block_number != wr.block_number`
    /// 3. `merged_reserved`: added with `cached_reserved` when `current_reported == true`
    /// 3. `reserved`:
    fn merged_work_report(who: &T::AccountId,
                          reserved: u64,
                          files: &Vec<(MerkleRoot, u64)>,
                          block_number: u64,
                          elder_reported: bool,
                          current_reported: bool) -> WorkReport {
        let mut merged_reserved = reserved;
        let mut merged_files = files;
        let mut cached_reserved: u64 = 0;

        if let Some(wr) = Self::work_reports(who) {
            // I. New report slot round
            if wr.block_number < block_number {
                // 1. Cover the files(aka. do nothing)
                if current_reported {
                    // 2. If the current id reported first: merged_reserved = reserved(aka. update to this slot round)
                } else if elder_reported {
                    // 3. If the elder id reported first: merged_reserved = wr.reserved(aka. keep the last slot round)
                    merged_reserved = wr.reserved;
                }
                // 4. Cached the reserved;
                cached_reserved = reserved;

            // II. Merge the work reports(elder + current)
            // NOTE: NOT permit multiple submit with same public key(current/elder can only report once),
            // otherwise this could lead a BIG trouble.
            } else if wr.block_number == block_number {
                // 1. Merge the files
                merged_files = Self::merge_files(files, wr.files);

                // 2. Sum up the reserved
                merged_reserved = wr.cached_reserved + reserved;
            }
        }

        WorkReport {
            block_number,
            used: 0,
            reserved: merged_reserved,
            cached_reserved,
            files: merged_files.clone()
        }
    }

    /// This function is judging if the identity already be registered
    /// TC is O(n)
    /// DB try is O(1)
    fn id_is_unique(pk: &PubKey) -> bool {
        let mut is_unique = true;

        for (_, (_, current_id)) in <Identities<T>>::iter() {
            if &current_id.pub_key == pk {
                is_unique = false;
                break
            }
        }

        is_unique
    }

    fn check_and_get_pk(
        ias_sig: &IASSig,
        ias_cert: &Cert,
        account_id: &T::AccountId,
        isv_body: &ISVBody,
        sig: &TeeSignature
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

    /// This function is judging if the work report identity is legal,
    /// return (`elder_reported`, `current_reported`), including:
    /// 1. Reported from `current_id` and the public key is match, then return (false, true)
    /// 2. Reported from `elder_id`(ab expire is legal), and the public ket is match, then return (true, false)
    /// 3. Or, return (false, false)
    fn work_report_id_check(reporter: &T::AccountId, wr_pk: &PubKey) -> (bool, bool) {
        let (maybe_eid, maybe_cid): (Option<Identity>, Option<Identity>) = Self::identities(reporter);
        if let Some(cid) = maybe_cid {
            let code: TeeCode = Self::code();

            if &cid.code == &code && wr_pk == &cid.pub_key {
                // Reported with new pk
                return (false, true);
            } else {
                // Reported with old pk, this require current block number < ab_expire block number
                if let Some(eid) = maybe_eid {
                    let current_bn = <system::Module<T>>::block_number();
                    return (
                        wr_pk == &eid.pub_key &&
                            (Self::ab_expire().is_some() && current_bn < Self::ab_expire().unwrap()),
                        false)
                }
            }
        }
        (false, false)
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
            wr_block_number == 1 || wr_block_number == Self::get_reported_slot(),
            "work report is outdated or beforehand"
        );

        Ok(())
    }

    fn work_report_sig_check(
        pub_key: &PubKey,
        block_number: u64,
        block_hash: &Vec<u8>,
        reserved: u64,
        files: &Vec<(MerkleRoot, u64)>,
        sig: &TeeSignature
    ) -> bool {
        api::crypto::verify_work_report_sig(
            pub_key,
            block_number,
            block_hash,
            reserved,
            files,
            sig,
        )
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
        RegisterSuccess(AccountId),
        WorksReportSuccess(AccountId, WorkReport),
    }
);

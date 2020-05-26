#![cfg_attr(not(feature = "std"), no_std)]
#![feature(option_result_contains)]

use codec::{Decode, Encode};
use frame_support::{
    decl_event, decl_module, decl_storage, ensure,
    weights::SimpleDispatchInfo,
    dispatch::DispatchResult,
    storage::IterableStorageMap
};
use sp_std::{str, convert::TryInto, prelude::*};
use system::ensure_signed;

#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};

// Crust runtime modules
use primitives::{constants::tee::*, MerkleRoot, PubKey, TeeSignature, ReportSlot, BlockNumber};
use market::{OrderStatus, MarketInterface, OrderInspector};

/// Provides crypto and other std functions by implementing `runtime_interface`
pub mod api;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[derive(Debug, PartialEq, Eq, Clone, Encode, Decode, Default)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct Identity<AccountId> {
    pub pub_key: PubKey,
    pub account_id: AccountId,
    pub validator_pub_key: PubKey,
    pub validator_account_id: AccountId,
    pub sig: TeeSignature,
}

#[derive(Debug, PartialEq, Eq, Clone, Encode, Decode, Default)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct WorkReport {
    pub pub_key: PubKey,
    pub block_number: u64,
    pub block_hash: Vec<u8>,
    pub used: u64,
    pub reserved: u64,
    pub files: Vec<(MerkleRoot, u64)>,
    pub sig: TeeSignature,
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
    /// The overarching event type.
    type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;

    /// The handler for reporting works.
    type Works: Works<Self::AccountId>;

    /// Interface for interacting with a market module.
    type MarketInterface: MarketInterface<Self::AccountId, Self::Hash>;
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

        /// The used workload, used for calculating stake limit in the end of era
        /// default is 0
        pub Used get(fn used): u128 = 0;

        /// The reserved workload, used for calculating stake limit in the end of era
        /// default is 0
        pub Reserved get(fn reserved): u128 = 0;
    }
}

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        // Initializing events
        // this is needed only if you are using events in your module
        fn deposit_event() = default;

        #[weight = SimpleDispatchInfo::default()]
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

            // 4. Check pub_key is unique
            ensure!(Self::pub_key_is_unique(applier_pk), "Public key already be registered");

            // 5. Verify sig
            ensure!(Self::identity_sig_check(&identity), "Tee report signature is illegal");

            // 6. applier is new add or needs to be updated
            if !Self::tee_identities(applier).contains(&identity) {
                // Store the tee identity
                <TeeIdentities<T>>::insert(applier, &identity);

                // Emit tee identity event
                Self::deposit_event(RawEvent::RegisterIdentity(who, identity));
            }

            Ok(())
        }

        #[weight = SimpleDispatchInfo::default()]
        pub fn report_works(origin, work_report: WorkReport) -> DispatchResult {
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
        let workload_map: Vec<(T::AccountId, u128)> = <TeeIdentities<T>>::iter().map(|(controller, _)| {
            // a. calculate this controller's order file map
            let mut order_files: Vec<(MerkleRoot, T::Hash)> = vec![];
            if let Some(provision) = T::MarketInterface::providers(&controller) {
                order_files = provision.file_map.values()
                    .filter_map(|order_id| {
                        // Get order status(should exist) and (maybe) change the status
                        let sorder =
                            T::MarketInterface::maybe_get_sorder(order_id).unwrap_or_default();
                        if sorder.order_status == OrderStatus::Success {
                            Some((sorder.file_identifier, order_id.clone()))
                        } else {
                            None
                        }
                    })
                    .collect();
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
    /// This function will (maybe) update or insert a work report, in details:
    /// 1. calculate used from reported files
    /// 2. set `ReportedInSlot` to true
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
        let mut updated_wr = wr.clone();
        let file_map = T::MarketInterface::providers(who).unwrap_or_default().file_map;
        updated_wr.used = wr.files.iter().fold(0, |used, (f_id, f_size)| {
            if let Some(order_id) = file_map.get(f_id) {
                // Get order status(should exist) and (maybe) change the status
                let mut sorder =
                    T::MarketInterface::maybe_get_sorder(order_id).unwrap_or_default();

                // TODO: we should specially handle `Failed` status
                if sorder.order_status != OrderStatus::Success {
                    // 1. Reset `expired_on` and `completed_on` for new order
                    if sorder.order_status == OrderStatus::Pending {
                        let current_block_numeric = Self::get_current_block_number();
                        // go panic if `current_block_numeric` > `created_on`
                        sorder.expired_on += current_block_numeric - sorder.created_on;
                        sorder.completed_on = current_block_numeric;
                    }

                    // 2. Change order status to `Success`
                    sorder.order_status = OrderStatus::Success;

                    // 3. (Maybe) set sorder
                    // TODO: we should specially handle `Failed` status
                    T::MarketInterface::maybe_set_sorder(order_id, &sorder);
                }
                return used + *f_size
            }
            used
        });

        // 3. Upsert work report and mark who has reported in this (report)slot
        <WorkReports<T>>::insert(who, &updated_wr);
        <ReportedInSlot<T>>::insert(who, rs, true);

        // 4. Update workload
        let used = updated_wr.used as u128;
        let reserved = updated_wr.reserved as u128;
        let total_used = Self::used() - old_used + used;
        let total_reserved = Self::reserved() - old_reserved + reserved;

        Used::put(total_used);
        Reserved::put(total_reserved);
        let total_workload = total_used + total_reserved;

        // 5. Update work report for every identity
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
            if Self::reported_in_slot(controller, current_rs) {
                // 1. Get all work report files
                let wr_files: Vec<MerkleRoot> = wr.files.iter().map(|(f_id, _)| f_id.clone()).collect();

                // 2. Check every order files are stored by controller
                for (f_id, order_id) in order_map {
                    if !wr_files.contains(f_id) {
                        // 3. Set status to failed
                        let mut sorder =
                            T::MarketInterface::maybe_get_sorder(order_id).unwrap_or_default();
                        sorder.order_status = OrderStatus::Failed;
                        T::MarketInterface::maybe_set_sorder(order_id, &sorder);
                        // TODO: add market slashing here
                    }
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
    /// This function is judging if the pub_key already be registered
    /// TC is O(n)
    /// DB try is O(1)
    fn pub_key_is_unique(pk: &PubKey) -> bool {
        let mut is_unique = true;

        for (_, id) in <TeeIdentities<T>>::iter() {
            if &id.pub_key == pk {
                is_unique = false;
                break
            }
        }

        is_unique
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
            wr.reserved,
            &wr.files,
            &wr.sig,
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
        RegisterIdentity(AccountId, Identity<AccountId>),
        ReportWorks(AccountId, WorkReport),
    }
);

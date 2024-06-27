// Copyright (C) 2019-2021 Crust Network Technologies Ltd.
// This file is part of Crust.

#![cfg_attr(not(feature = "std"), no_std)]
#![feature(option_result_contains)]

use core::option::Option::None;

use codec::{Decode, Encode};
use frame_support::{
    decl_event, decl_module, decl_storage, decl_error,
    dispatch::{DispatchResult, DispatchResultWithPostInfo}, ensure,
    traits::{
        Currency, ReservableCurrency, Get, LockableCurrency, ExistenceRequirement,
        ExistenceRequirement::{AllowDeath, KeepAlive},
        WithdrawReasons, Imbalance
    },
    weights::{Weight, Pays}
};
use sp_std::{prelude::*, convert::TryInto, collections::btree_set::BTreeSet, collections::btree_map::BTreeMap};
use frame_system::{self as system, ensure_signed, ensure_root};
use sp_runtime::{SaturatedConversion, Perbill, ModuleId, traits::{Zero, CheckedMul, AccountIdConversion, Saturating}, DispatchError};

#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};

pub mod weight;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(any(feature = "runtime-benchmarks", test))]
pub mod benchmarking;

use primitives::{
    constants::market::*, traits::{
        BenefitInterface, MarketInterface, SworkerInterface, UsableCurrency
    }, BlockNumber, MerkleRoot, ReportSlot, SworkerAnchor
};

pub(crate) const LOG_TARGET: &'static str = "market";
const MAX_REPLICAS: usize = 200;
// We should change `calculate_reward_amount` if we change the REWARD_PERSON
// Any ratio change should re-design the `calculate_reward_amount` as well
const REWARD_PERSON: u32 = 4;

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
    fn place_storage_order() -> Weight;
    fn calculate_reward() -> Weight;
    fn reward_merchant() -> Weight;
    fn update_replicas() -> Weight;
}

#[derive(Debug, PartialEq, Encode, Decode, Default, Clone)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct FileInfoV2<AccountId: Ord, Balance> {
    // The ordered file size, which declare by user
    pub file_size: u64,
    // The storage power value in MPoW
    pub spower: u64,
    // The block number when the file goes invalid
    pub expired_at: BlockNumber,
    // The last block number when the file's amount is calculated
    pub calculated_at: BlockNumber,
    // The file value
    #[codec(compact)]
    pub amount: Balance,
    // The pre paid pool
    #[codec(compact)]
    pub prepaid: Balance,
    // The count of valid replica each report slot
    pub reported_replica_count: u32,
    // Remaining paid count
    pub remaining_paid_count: u32,
    // The replica map, key is the group owner
    pub replicas: BTreeMap<AccountId, Replica<AccountId>>
}

#[derive(Debug, PartialEq, Eq, Clone, Encode, Decode, Default)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct Replica<AccountId> {
    // Controller account
    pub who: AccountId,
    // The last bloch number when the node reported works
    pub valid_at: BlockNumber,
    // The anchor associated to the node mapping with file
    pub anchor: SworkerAnchor,
    // Is reported in the last check
    pub is_reported: bool,
    // Timestamp which the replica created
    pub created_at: Option<BlockNumber>
}


#[derive(Debug, PartialEq, Eq, Clone, Encode, Decode, Default)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct ReplicaToUpdate<AccountId> {
    pub reporter: AccountId,
    pub owner: AccountId,
    pub sworker_anchor: SworkerAnchor,
    pub report_slot: ReportSlot,
    pub report_block: BlockNumber,
    pub valid_at: BlockNumber,
    pub is_added: bool
}
type ReplicaToUpdateOf<T> = ReplicaToUpdate<<T as system::Config>::AccountId>; 

type BalanceOf<T> = <<T as Config>::Currency as Currency<<T as system::Config>::AccountId>>::Balance;
type PositiveImbalanceOf<T> = <<T as Config>::Currency as Currency<<T as system::Config>::AccountId>>::PositiveImbalance;
type NegativeImbalanceOf<T> = <<T as Config>::Currency as Currency<<T as system::Config>::AccountId>>::NegativeImbalance;

impl<T: Config> MarketInterface<<T as system::Config>::AccountId, BalanceOf<T>> for Module<T>
{
    /// Withdraw market staking pot for distributing staking reward
    fn withdraw_staking_pot() -> BalanceOf<T> {
        let staking_pot = Self::staking_pot();
        if T::Currency::free_balance(&staking_pot) < T::Currency::minimum_balance() {
            log!(
                info,
                "🏢 Market Staking Pot is empty."
            );

            return Zero::zero();
        }
        // Leave the minimum balance to keep this account live.
        let staking_amount = T::Currency::free_balance(&staking_pot) - T::Currency::minimum_balance();
        let mut imbalance = <PositiveImbalanceOf<T>>::zero();
        imbalance.subsume(T::Currency::burn(staking_amount.clone()));
        if let Err(_) = T::Currency::settle(
            &staking_pot,
            imbalance,
            WithdrawReasons::TRANSFER,
            KeepAlive
        ) {
            log!(
                warn,
                "🏢 Something wrong during withdrawing staking pot. Admin/Council should pay attention to it."
            );

            return Zero::zero();
        }
        staking_amount
    }
    
    fn update_files_spower(changed_files: &Vec<(MerkleRoot, u64, Vec<(T::AccountId, T::AccountId, SworkerAnchor, Option<BlockNumber>)>)>) {
        for (cid, new_spower, changed_replicas) in changed_files {
            if let Some(mut file_info) = <FilesV2<T>>::get(&cid) {
                // Update file spower
                file_info.spower = *new_spower;

                // Update the create_at
                for (owner, who, anchor, created_at) in changed_replicas {
                    let maybe_replica = file_info.replicas.get_mut(owner);
                    if let Some(mut replica) = maybe_replica {
                        if replica.who == *who && replica.anchor == *anchor {
                            replica.created_at = *created_at;
                        }
                    }
                }

                // Write back to storage
                <FilesV2<T>>::insert(&cid, file_info);
            }
        }
    }
}

/// The module's configuration trait.
pub trait Config: system::Config {
    /// The market's module id, used for deriving its sovereign account ID.
    type ModuleId: Get<ModuleId>;

    /// The payment balance.
    type Currency: ReservableCurrency<Self::AccountId> + UsableCurrency<Self::AccountId> + LockableCurrency<Self::AccountId>;

    /// used to check work report
    type SworkerInterface: SworkerInterface<Self::AccountId>;

    /// used for reward and discount
    type BenefitInterface: BenefitInterface<Self::AccountId, BalanceOf<Self>, NegativeImbalanceOf<Self>>;

    /// The overarching event type.
    type Event: From<Event<Self>> + Into<<Self as system::Config>::Event>;

    /// File duration.
    type FileDuration: Get<BlockNumber>;

    /// Liquidity duration.
    type LiquidityDuration: Get<BlockNumber>;

    /// File base replica. Use 4 for now
    type FileReplica: Get<u32>;

    /// File Init Byte Fee.
    type InitFileByteFee: Get<BalanceOf<Self>>;

    /// Files Count Init Price.
    type InitFileKeysCountFee: Get<BalanceOf<Self>>;

    /// Storage reference ratio. reported_files_size / total_capacity
    type StorageReferenceRatio: Get<(u128, u128)>;

    /// Storage increase ratio.
    type StorageIncreaseRatio: Get<Perbill>;

    /// Storage decrease ratio.
    type StorageDecreaseRatio: Get<Perbill>;

    /// Storage/Staking ratio.
    type StakingRatio: Get<Perbill>;

    /// Renew reward ratio
    type RenewRewardRatio: Get<Perbill>;

    /// Tax / Storage plus Staking ratio.
    type StorageRatio: Get<Perbill>;

    /// Maximum file size
    type MaximumFileSize: Get<u64>;

    /// Weight information for extrinsics in this pallet.
    type WeightInfo: WeightInfo;
}

// This module's storage items.
decl_storage! {
    trait Store for Module<T: Config> as Market {
        /// New orders count in the past one period(one hour), determinate the FileBaseFee
        OrdersCount get(fn orders_count): u32 = 0;

        /// The file base fee for each storage order.
        pub FileBaseFee get(fn file_base_fee): BalanceOf<T> = Zero::zero();

        /// The minimal file base fee for each storage order.
        pub MinFileBaseFee get(fn min_file_base_fee): BalanceOf<T> = Zero::zero();

        /// The file price per MB.
        /// It's dynamically adjusted and would change according to FilesSize, TotalCapacity and StorageReferenceRatio.
        pub FileByteFee get(fn file_byte_fee): BalanceOf<T> = T::InitFileByteFee::get();

        /// The minimal file price per MB.
        pub MinFileByteFee get(fn min_file_byte_fee): BalanceOf<T> = Zero::zero();

        /// Files count, determinate the FileKeysCountFee
        pub FileKeysCount get(fn files_count): u32 = 0;

        /// The file price by keys
        /// It's dynamically adjusted and would change according to the total keys in files
        pub FileKeysCountFee get(fn file_keys_count_fee): BalanceOf<T> = T::InitFileKeysCountFee::get();

        /// The minimal file price by keys
        pub MinFileKeysCountFee get(fn min_file_keys_count_fee): BalanceOf<T> = Zero::zero();

        /// File V2 information iterated by order id
        pub FilesV2 get(fn filesv2):
        map hasher(twox_64_concat) MerkleRoot => Option<FileInfoV2<T::AccountId, BalanceOf<T>>>;

        /// Has new order in the past blocks, pruning handling of pending files
        HasNewOrder get(fn has_new_order): bool = false;

        /// Wait for updating storage power for all replicas
        pub PendingFiles get(fn pending_files): BTreeSet<MerkleRoot>;

        /// The global market switch to enable place storage order service
        pub EnableMarket get(fn enable_market): bool = false;

        /// The sPower will become valid after this period, default is 3 months
        pub SpowerReadyPeriod get(fn spower_ready_period): BlockNumber = 1_296_000;

        /// The crust-spower service account
        pub SpowerSuperior get(fn spower_superior): Option<T::AccountId>;

        /// The last replicas update block
        pub LastReplicasUpdateBlock get (fn last_replicas_update_block): BlockNumber = 0;
    }
    add_extra_genesis {
		build(|_config| {
			// Create the market accounts
			<Module<T>>::init_pot(<Module<T>>::collateral_pot);
			<Module<T>>::init_pot(<Module<T>>::storage_pot);
			<Module<T>>::init_pot(<Module<T>>::staking_pot);
			<Module<T>>::init_pot(<Module<T>>::reserved_pot);
		});
	}
}

decl_error! {
    /// Error for the market module.
    pub enum Error for Module<T: Config> {
        /// Don't have enough currency(CRU) to finish the extrinsic(transaction).
        /// Please transfer some CRU into this account.
        InsufficientCurrency,
        /// The file size is not correct.
        /// The same file is already on chain and the file size should be same.
        /// Please check the file size again.
        FileSizeNotCorrect,
        /// The file is not in the reward period.
        /// Please wait until the file is expired.
        NotInRewardPeriod,
        /// The reward is not enough.
        NotEnoughReward,
        /// The file is too large. Please check the MaximumFileSize value.
        FileTooLarge,
        /// Place order is not available right now. Please wait for a while.
        PlaceOrderNotAvailable,
        /// The file does not exist. Please check the cid again.
        FileNotExist,
        /// The spower superior account is not set. Please call the set_spower_superior extrinsic first.
        SpowerSuperiorNotSet,
        /// The caller account is not the spower superior account. Please check the caller account again.
        IllegalSpowerSuperior,
    }
}

decl_module! {
    pub struct Module<T: Config> for enum Call where origin: T::Origin {
        type Error = Error<T>;

        // Initializing events
        // this is needed only if you are using events in your module
        fn deposit_event() = default;

        /// The market's module id, used for deriving its sovereign account ID.
        const ModuleId: ModuleId = T::ModuleId::get();

        /// The file duration.
        const FileDuration: BlockNumber = T::FileDuration::get();

        /// The file base replica to get reward.
        const FileReplica: u32 = T::FileReplica::get();

        /// The file init price after the chain start.
        const InitFileByteFee: BalanceOf<T> = T::InitFileByteFee::get();

        /// The storage reference ratio to adjust the file byte fee.
        const StorageReferenceRatio: (u128, u128) = T::StorageReferenceRatio::get();

        /// The storage increase ratio for each file byte&key fee change.
        const StorageIncreaseRatio: Perbill = T::StorageIncreaseRatio::get();

        /// The storage decrease ratio for each file byte&key fee change.
        const StorageDecreaseRatio: Perbill = T::StorageDecreaseRatio::get();

        /// The staking ratio for how much CRU into staking pot.
        const StakingRatio: Perbill = T::StakingRatio::get();

        /// The storage ratio for how much CRU into storage pot.
        const StorageRatio: Perbill = T::StorageRatio::get();

        /// The max file size of a file
        const MaximumFileSize: u64 = T::MaximumFileSize::get();

        /// The renew reward ratio for liquidator.
        const RenewRewardRatio: Perbill = T::RenewRewardRatio::get();

        /// Called when a block is initialized. Will call update_identities to update file price
        fn on_initialize(now: T::BlockNumber) -> Weight {
            let now = TryInto::<u32>::try_into(now).ok().unwrap();
            let mut consumed_weight: Weight = 0;
            let mut add_db_reads_writes = |reads, writes| {
                consumed_weight += T::DbWeight::get().reads_writes(reads, writes);
            };
            if ((now + PRICE_UPDATE_OFFSET) % PRICE_UPDATE_SLOT).is_zero() && Self::has_new_order(){
                Self::update_file_byte_fee();
                Self::update_file_keys_count_fee();
                HasNewOrder::put(false);
                add_db_reads_writes(8, 3);
            }
            if ((now + BASE_FEE_UPDATE_OFFSET) % BASE_FEE_UPDATE_SLOT).is_zero() {
                Self::update_base_fee();
                add_db_reads_writes(3, 3);
            }
            add_db_reads_writes(1, 0);
            consumed_weight
        }

        /// Place a storage order. The cid and file_size of this file should be provided. Extra tips is accepted.
        #[weight = T::WeightInfo::place_storage_order()]
        pub fn place_storage_order(
            origin,
            cid: MerkleRoot,
            reported_file_size: u64,
            #[compact] tips: BalanceOf<T>,
            _memo: Vec<u8>
        ) -> DispatchResult {
            // 1. Service should be available right now.
            ensure!(Self::enable_market(), Error::<T>::PlaceOrderNotAvailable);
            let who = ensure_signed(origin)?;

            // 2. Calculate amount.
            let mut charged_file_size = reported_file_size;
            if let Some(file_info) = Self::filesv2(&cid) {
                if file_info.file_size <= reported_file_size {
                    // Charge user with real file size
                    charged_file_size = file_info.file_size;
                } else {
                    Err(Error::<T>::FileSizeNotCorrect)?
                }
            }
            // 3. charged_file_size should be smaller than 32G
            ensure!(charged_file_size < T::MaximumFileSize::get(), Error::<T>::FileTooLarge);

            let (file_base_fee, amount) = Self::get_file_fee(charged_file_size);

            // 4. Check client can afford the sorder
            ensure!(T::Currency::usable_balance(&who) >= file_base_fee + amount + tips, Error::<T>::InsufficientCurrency);

            // 5. Split into reserved, storage and staking account
            let amount = Self::split_into_reserved_and_storage_and_staking_pot(&who, amount.clone(), file_base_fee, tips, AllowDeath)?;

            let curr_bn = Self::get_current_block_number();

            // 6. three scenarios: new file, extend time(refresh time)
            Self::upsert_new_file_info(&cid, &amount, &curr_bn, charged_file_size);

            // 7. Update new order status.
            HasNewOrder::put(true);
            OrdersCount::mutate(|count| {*count = count.saturating_add(1)});

            Self::deposit_event(RawEvent::FileSuccess(who, cid));

            Ok(())
        }

        /// Add prepaid amount of currency for this file.
        /// If this file has prepaid value and enough for a new storage order, it can be renewed by anyone.
        #[weight = T::WeightInfo::place_storage_order()]
        pub fn add_prepaid(
            origin,
            cid: MerkleRoot,
            #[compact] amount: BalanceOf<T>
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;

            ensure!(T::Currency::usable_balance(&who) >= amount, Error::<T>::InsufficientCurrency);

            if let Some(mut file_info) = Self::filesv2(&cid) {
                T::Currency::transfer(&who, &Self::storage_pot(), amount.clone(), AllowDeath)?;
                file_info.prepaid += amount;
                <FilesV2<T>>::insert(&cid, file_info);
            } else {
                Err(Error::<T>::FileNotExist)?
            }

            Self::deposit_event(RawEvent::AddPrepaidSuccess(who, cid, amount));

            Ok(())
        }

        /// Set the crust-spower service superior account
        #[weight = 1000]
        pub fn set_spower_superior(origin, superior: T::AccountId) -> DispatchResult {
            ensure_root(origin)?;

            SpowerSuperior::<T>::put(superior.clone());

            Self::deposit_event(RawEvent::SetSpowerSuperiorSuccess(superior));
            Ok(())
        }

        /// Update file replicas from crust-spower offchain service
        /// Emits `ReplicasUpdateSuccess` event if the call is success
        /// # params
        ///  - file_infos_Map: file replicas info map with the data structure as belowed:
        ///     Map<(CID, file_size, Vec<(reporter, owner, sworker_anchor, report_slot, report_block, valid_at, is_added))>>
        ///     The key is the file CID, and the value is a vector of file replicas info.
        ///     PS: We're not using the ReplicaToUpdate type in the argument directly, because this would fail traditional apps
        ///         which would need to decode extrinsics, which will then error out with 'Unable to decode on ReplicaToUpdate'.
        ///         So we directly use the raw types and tuple here as the argument
        #[weight = T::WeightInfo::update_replicas()]
        pub fn update_replicas(
            origin,
            file_infos_map: Vec<(MerkleRoot, u64, Vec<(T::AccountId, T::AccountId, SworkerAnchor, ReportSlot, BlockNumber, BlockNumber, bool)>)>,
            last_processed_block_wrs: BlockNumber
        ) -> DispatchResultWithPostInfo {
            let caller = ensure_signed(origin)?;
            let maybe_superior = Self::spower_superior();

            // 1. Check if superior exist
            ensure!(maybe_superior.is_some(), Error::<T>::SpowerSuperiorNotSet);
            // 2. Check if caller is superior
            ensure!(Some(&caller) == maybe_superior.as_ref(), Error::<T>::IllegalSpowerSuperior);

            // 3. Internal update replicas
            let mut file_infos_map_ex: Vec<(MerkleRoot, u64, Vec<ReplicaToUpdateOf<T>>)> = vec![];
            for (cid, file_size, replicas) in file_infos_map {
                let mut replica_list: Vec<ReplicaToUpdateOf<T>> = vec![];
                for (reporter, owner, sworker_anchor, report_slot, report_block, valid_at, is_added) in replicas {
                    let replica_to_update = ReplicaToUpdate {
                        reporter: reporter,
                        owner: owner,
                        sworker_anchor: sworker_anchor,
                        report_slot: report_slot,
                        report_block: report_block,
                        valid_at: valid_at,
                        is_added: is_added
                    };
                    replica_list.push(replica_to_update);
                }
                file_infos_map_ex.push((cid, file_size, replica_list));
            } 
            let (changed_files_count, sworker_changed_spower_map, illegal_file_replicas_map) = Self::internal_update_replicas(file_infos_map_ex);

            // 4. Update the last processed block of work reports in pallet_swork
            T::SworkerInterface::update_last_processed_block_of_work_reports(last_processed_block_wrs);

            // 5. Update the changed spower of sworkers
            T::SworkerInterface::update_sworkers_changed_spower(&sworker_changed_spower_map);

            // 5. Update illegal file replicas count in pallet_swork
            T::SworkerInterface::update_illegal_file_replicas_count(&illegal_file_replicas_map);

            // 6. Update the LastReplicasUpdateBlock
            let curr_bn = Self::get_current_block_number();
            LastReplicasUpdateBlock::put(curr_bn);

            // 7. Emit the event
            Self::deposit_event(RawEvent::UpdateReplicasSuccess(caller, curr_bn, changed_files_count, last_processed_block_wrs)); 

            // Do not charge fee for management extrinsic
            Ok(Pays::No.into())
        }

        /// Calculate the reward for a file
        #[weight = T::WeightInfo::calculate_reward()]
        pub fn calculate_reward(
            origin,
            cid: MerkleRoot,
        ) -> DispatchResult {
            let liquidator = ensure_signed(origin)?;

            // 1. Ensure file exist
            if !<FilesV2<T>>::contains_key(&cid) {
                return Ok(());
            }

            let file_info = Self::filesv2(&cid).unwrap();
            let curr_bn = Self::get_current_block_number();

            // 2. File should be live right now and calculate reward should be after expired_at
            ensure!(file_info.expired_at != 0, Error::<T>::NotInRewardPeriod);

            // 3. Maybe reward liquidator when he try to close outdated file
            Self::maybe_reward_liquidator(&cid, curr_bn, &liquidator)?;

            // 4. Try to renew file if prepaid is not zero
            Self::try_to_renew_file(&cid, curr_bn, &liquidator)?;

            // 5. Try to close file
            Self::try_to_close_file(&cid, curr_bn)?;

            Self::deposit_event(RawEvent::CalculateSuccess(cid));
            Ok(())
        }

        /// Reward a merchant
        #[weight = T::WeightInfo::reward_merchant()]
        pub fn reward_merchant(
            origin
        ) -> DispatchResult {
            let merchant = ensure_signed(origin)?;

            // 1. Ensure reward is larger than some value
            let (_, reward) = T::BenefitInterface::get_collateral_and_reward(&merchant);
            ensure!(reward > Zero::zero(), Error::<T>::NotEnoughReward);

            // 2. Transfer the money
            T::Currency::transfer(&Self::storage_pot(), &merchant, reward, KeepAlive)?;

            // 3. Set the reward to zero and push it back
            T::BenefitInterface::update_reward(&merchant, Zero::zero());

            Self::deposit_event(RawEvent::RewardMerchantSuccess(merchant));
            Ok(())
        }

        /// Open/Close market service
        ///
        /// The dispatch origin for this call must be _Root_.
        #[weight = 1000]
        pub fn set_enable_market(
            origin,
            is_enabled: bool
        ) -> DispatchResult {
            let _ = ensure_root(origin)?;

            EnableMarket::put(is_enabled);

            Self::deposit_event(RawEvent::SetEnableMarketSuccess(is_enabled));
            Ok(())
        }

        /// Set the file base fee
        ///
        /// The dispatch origin for this call must be _Root_.
        #[weight = 1000]
        pub fn set_base_fee(
            origin,
            #[compact] base_fee: BalanceOf<T>
        ) -> DispatchResult {
            let _ = ensure_root(origin)?;

            <FileBaseFee<T>>::put(base_fee);

            Self::deposit_event(RawEvent::SetBaseFeeSuccess(base_fee));
            Ok(())
        }

        /// Set the file byte fee
        ///
        /// The dispatch origin for this call must be _Root_.
        #[weight = 1000]
        pub fn set_byte_fee(
            origin,
            #[compact] byte_fee: BalanceOf<T>
        ) -> DispatchResult {
            let _ = ensure_root(origin)?;
            <FileByteFee<T>>::put(byte_fee);
            Ok(())
        }

        /// Set the file key count fee
        ///
        /// The dispatch origin for this call must be _Root_.
        #[weight = 1000]
        pub fn set_key_count_fee(
            origin,
            #[compact] key_count_fee: BalanceOf<T>
        ) -> DispatchResult {
            let _ = ensure_root(origin)?;
            <FileKeysCountFee<T>>::put(key_count_fee);
            Ok(())
        }

        /// Set the mininal file base fee
        ///
        /// The dispatch origin for this call must be _Root_.
        #[weight = 1000]
        pub fn set_min_base_fee(
            origin,
            #[compact] min_base_fee: BalanceOf<T>
        ) -> DispatchResult {
            let _ = ensure_root(origin)?;
            <MinFileBaseFee<T>>::put(min_base_fee);
            Ok(())
        }

        /// Set the minimal file byte fee
        ///
        /// The dispatch origin for this call must be _Root_.
        #[weight = 1000]
        pub fn set_min_byte_fee(
            origin,
            #[compact] min_byte_fee: BalanceOf<T>
        ) -> DispatchResult {
            let _ = ensure_root(origin)?;
            <MinFileByteFee<T>>::put(min_byte_fee);
            Ok(())
        }

        /// Set the minimal file key count fee
        ///
        /// The dispatch origin for this call must be _Root_.
        #[weight = 1000]
        pub fn set_min_key_count_fee(
            origin,
            #[compact] min_key_count_fee: BalanceOf<T>
        ) -> DispatchResult {
            let _ = ensure_root(origin)?;
            <MinFileKeysCountFee<T>>::put(min_key_count_fee);
            Ok(())
        }
    }
}

impl<T: Config> Module<T> {
    /// The pot of a collateral account
    pub fn collateral_pot() -> T::AccountId {
        // "modl" ++ "crmarket" ++ "coll" is 16 bytes
        T::ModuleId::get().into_sub_account("coll")
    }

    /// The pot of a storage account
    pub fn storage_pot() -> T::AccountId {
        // "modl" ++ "crmarket" ++ "stor" is 16 bytes
        T::ModuleId::get().into_sub_account("stor")
    }

    /// The pot of a staking account
    pub fn staking_pot() -> T::AccountId {
        // "modl" ++ "crmarket" ++ "stak" is 16 bytes
        T::ModuleId::get().into_sub_account("stak")
    }

    /// The pot of a reserved account
    pub fn reserved_pot() -> T::AccountId {
        // "modl" ++ "crmarket" ++ "rese" is 16 bytes
        T::ModuleId::get().into_sub_account("rese")
    }

    /// 
    pub fn internal_update_replicas(
        file_infos_map: Vec<(MerkleRoot, u64, Vec<ReplicaToUpdateOf<T>>)>
    ) -> (u32, BTreeMap<SworkerAnchor, i64>, BTreeMap<ReportSlot, u32>)
    {
        let mut changed_files_count = 0;
        let mut sworker_changed_spower_map: BTreeMap<SworkerAnchor, i64> = BTreeMap::new(); 
        let mut illegal_file_replicas_map: BTreeMap<ReportSlot, u32> = BTreeMap::new();
        'file_loop: for (cid, reported_file_size, file_replicas) in file_infos_map {

            // Split the replicas array into added_replicas and deleted_replicas
            let mut added_replicas: Vec<ReplicaToUpdateOf<T>> = vec![];
            let mut deleted_replicas: Vec<ReplicaToUpdateOf<T>> = vec![];
            for replica in file_replicas {
                if replica.is_added {
                    added_replicas.push(replica);
                } else {
                    deleted_replicas.push(replica);
                }
            }

            // Sort each array by report_block
            added_replicas.sort_by(|a, b| a.report_block.cmp(&b.report_block));
            deleted_replicas.sort_by(|a, b| a.report_block.cmp(&b.report_block));

            // Get the file_info object from storage for 1 time db read
            let maybe_file_info = Self::filesv2(&cid);
            if maybe_file_info.is_none() {
                // If the cid doesn't exist in the market, either this is a non-exist cid, or has been removed by illegal file size, or has been liquidated and closed
                // Since we haven't changed the sworker's spower during the swork.report_works call, so we can just ignore all replicas here without any side-effects

                // Invalid cid's replicas count should be subtracted from Swork::Added_Files_Count
                if added_replicas.len() > 0 {
                    let ReplicaToUpdate { report_slot, ..} = added_replicas[0];
                    if let Some(count) = illegal_file_replicas_map.get_mut(&report_slot) {
                        *count += added_replicas.len() as u32;
                    } else {
                        illegal_file_replicas_map.insert(report_slot, added_replicas.len() as u32);
                    }
                }
                
                // Just continue to next cid
                continue;
            }
            let mut file_info = maybe_file_info.unwrap();

            // ---------------------------------------------------------
            // --- Handle upsert replicas ---
            for file_replica in added_replicas.iter() {

                let ReplicaToUpdate { reporter, owner, sworker_anchor, report_slot, report_block, valid_at, ..} = file_replica;

                // 1. Check if file_info.file_size == reported_file_size or not
                let is_valid_cid = Self::maybe_upsert_file_size(&mut file_info, &reporter, &cid, reported_file_size); 
                if !is_valid_cid {
                    // This is a invalid cid with illegal file size, which has been removed in maybe_upsert_file_size

                    // We simply add all added_replicas count as of the first replica's report_slot, which is almost the case
                    if let Some(count) = illegal_file_replicas_map.get_mut(report_slot) {
                        *count += added_replicas.len() as u32;
                    } else {
                        illegal_file_replicas_map.insert(*report_slot, added_replicas.len() as u32);
                    }

                    changed_files_count += 1;
                    // We don't need to process all subsequent replicas anymore.
                    continue 'file_loop;
                }

                // 2. Add replica data to storage
                let is_replica_added = Self::upsert_replica(&mut file_info, &reporter, &owner, &sworker_anchor, *report_block, *valid_at);
                // If the replica is not added (due to exceed MAX_REPLICA, or same owner reported), just ignore this replica
                if is_replica_added {
                    // Update related sworker's changed spower
                    if let Some(changed_spower) = sworker_changed_spower_map.get_mut(sworker_anchor) {
                        *changed_spower += file_info.file_size as i64;
                    } else {
                        sworker_changed_spower_map.insert(sworker_anchor.clone(), file_info.file_size as i64);
                    }
                }
            }

            // ---------------------------------------------------------
            // --- Handle delete replicas ---
            for file_replica in deleted_replicas.iter() {

                let ReplicaToUpdate { reporter, owner, sworker_anchor, ..} = file_replica;
                
                let (is_replica_deleted, to_delete_spower) = Self::delete_replica(&mut file_info,&reporter, owner, &sworker_anchor);
                if is_replica_deleted {
                    // Update replicated sworker's changed spower
                    if let Some(changed_spower) = sworker_changed_spower_map.get_mut(sworker_anchor) {
                        *changed_spower -= to_delete_spower as i64;
                    } else {
                        sworker_changed_spower_map.insert(sworker_anchor.clone(), 0-(to_delete_spower as i64));
                    }
                }
            }

            // Update the file info with all the above changes in one DB write
            <FilesV2<T>>::insert(cid.clone(), file_info.clone());
            changed_files_count += 1;
        }

        (changed_files_count, sworker_changed_spower_map, illegal_file_replicas_map)
    }

    fn maybe_upsert_file_size(file_info: &mut FileInfoV2<T::AccountId, BalanceOf<T>>, 
                              who: &T::AccountId, cid: &MerkleRoot, reported_file_size: u64) -> bool {
        
        let mut is_valid_cid = true;
        // 1. Judge if file_info.file_size == reported_file_size or not
        if file_info.replicas.len().is_zero() {
            // ordered_file_size == reported_file_size, return it
            if file_info.file_size == reported_file_size {
                return true;
            // ordered_file_size > reported_file_size, correct it
            } else if file_info.file_size > reported_file_size {
                file_info.file_size = reported_file_size;                
            // ordered_file_size < reported_file_size, close it with notification
            } else {
                let total_amount = file_info.amount + file_info.prepaid;
                if !Self::maybe_reward_merchant(who, &total_amount) {
                    // This should not have error => discard the result
                    let _ = T::Currency::transfer(&Self::storage_pot(), &Self::reserved_pot(), total_amount, KeepAlive);
                }
                <FilesV2<T>>::remove(cid);
                FileKeysCount::mutate(|count| *count = count.saturating_sub(1));
                OrdersCount::mutate(|count| {*count = count.saturating_sub(1)});
                Self::deposit_event(RawEvent::IllegalFileClosed(cid.clone()));

                is_valid_cid = false;
            }
        }

        is_valid_cid
    }

    fn upsert_replica(file_info: &mut FileInfoV2<T::AccountId, BalanceOf<T>>, 
                      who: &<T as system::Config>::AccountId,
                      owner: &<T as system::Config>::AccountId,
                      anchor: &SworkerAnchor,
                      _report_block: BlockNumber,
                      valid_at: BlockNumber
                    ) -> bool {

        let mut is_replica_added = false;
        let curr_bn = Self::get_current_block_number();
        // 1. Check if the length of the groups exceed MAX_REPLICAS or not
        if file_info.replicas.len() < MAX_REPLICAS {
            // 2. Check if the file is stored by other members
            if !file_info.replicas.contains_key(&owner) {
                let new_replica = Replica {
                    who: who.clone(),
                    valid_at,
                    anchor: anchor.clone(),
                    is_reported: true,
                    created_at: Some(valid_at) 
                };
                file_info.replicas.insert(owner.clone(), new_replica);
                file_info.reported_replica_count += 1;
                is_replica_added = true;

                // Reward the first 4 merchants which submits the replica report
                if file_info.remaining_paid_count > 0 {
                    let reward_amount = Self::calculate_reward_amount(file_info.remaining_paid_count, &file_info.amount);
                    if let Some(new_reward) = Self::has_enough_collateral(&owner, &reward_amount) {
                        T::BenefitInterface::update_reward(&owner, new_reward);
                        file_info.amount = file_info.amount.saturating_sub(reward_amount);
                        file_info.remaining_paid_count = file_info.remaining_paid_count.saturating_sub(1);
                    }
                }
            }
        }

        // 3. The first join the replicas and file become live(expired_at > calculated_at)
        if file_info.expired_at == 0 {
            file_info.calculated_at = curr_bn;
            file_info.expired_at = curr_bn + T::FileDuration::get();
        }

        is_replica_added
    }

    fn delete_replica(file_info: &mut FileInfoV2<T::AccountId, BalanceOf<T>>,
                      who: &<T as system::Config>::AccountId,
                      owner: &<T as system::Config>::AccountId,
                      anchor: &SworkerAnchor,
                    ) -> (bool, u64) {
        
        let mut spower: u64 = 0;
        let mut is_replica_deleted: bool = false;

        // 1. Delete replica from file_info
        let maybe_replica = file_info.replicas.get(owner);
        if let Some(replica) = maybe_replica {
            if replica.who == *who {
                // Only decreate the spower if it's the same anchor, because for new anchor, the spower has been reset to 0 after re-register
                if replica.anchor == *anchor {
                    if replica.created_at.is_none() { 
                        // It means the replica is already using the spower value, because created_at would be set to None when use the spower value
                        spower = file_info.spower;
                    } else { 
                        spower = file_info.file_size; 
                    };
                }
                // Don't need to check the replica.is_reported here, because we don't use the calculate_rewards->update_replicas right now
                file_info.reported_replica_count = file_info.reported_replica_count.saturating_sub(1);
                file_info.replicas.remove(owner);
                is_replica_deleted = true;
            }
        }

        (is_replica_deleted, spower)
    }

    /// Close file, maybe move into trash
    fn try_to_close_file(cid: &MerkleRoot, curr_bn: BlockNumber) -> DispatchResult {
        if let Some(mut file_info) = <FilesV2<T>>::get(cid) {
            // If it's already expired.
            if file_info.expired_at <= curr_bn && file_info.expired_at == file_info.calculated_at {
                let total_amount = file_info.amount.saturating_add(file_info.prepaid);
                T::Currency::transfer(&Self::storage_pot(), &Self::reserved_pot(), total_amount, KeepAlive)?;

                // Remove all spower from wr
                file_info.reported_replica_count = 0;
                // TODO: add this weight into place_storage_order
                let _ = Self::update_replicas_spower(&mut file_info, None);

                // Remove files
                <FilesV2<T>>::remove(&cid);
                FileKeysCount::mutate(|count| *count = count.saturating_sub(1));
                Self::deposit_event(RawEvent::FileClosed(cid.clone()));
            };
        }
        Ok(())
    }

    fn try_to_renew_file(cid: &MerkleRoot, curr_bn: BlockNumber, liquidator: &T::AccountId) -> DispatchResult {
        if let Some(mut file_info) = <FilesV2<T>>::get(cid) {
            // 0. return if the file is ongoing or pending
            if file_info.expired_at != file_info.calculated_at {
                return Ok(());
            }
            // 1. Calculate total amount
            let (file_base_fee, file_amount) = Self::get_file_fee(file_info.file_size);
            let total_amount = file_base_fee.clone() + file_amount.clone();
            // 2. Check if prepaid pool can afford the price
            if file_info.prepaid >= total_amount {
                file_info.prepaid = file_info.prepaid.saturating_sub(total_amount.clone());
                // 3. Split into reserved, storage and staking account
                let file_amount = Self::split_into_reserved_and_storage_and_staking_pot(&Self::storage_pot(), file_amount.clone(), file_base_fee, Zero::zero(), KeepAlive)?;
                file_info.amount += file_amount;
                if file_info.replicas.len() == 0 {
                    // turn this file into pending status since replicas.len() is zero
                    // we keep the original amount and expected_replica_count
                    file_info.expired_at = 0;
                    file_info.calculated_at = curr_bn;
                    file_info.remaining_paid_count = REWARD_PERSON;
                } else {
                    // Refresh the file to the new file
                    file_info.expired_at = curr_bn + T::FileDuration::get();
                    file_info.calculated_at = curr_bn;
                }
                <FilesV2<T>>::insert(cid, file_info);

                // 5. Update new order status.
                HasNewOrder::put(true);

                Self::deposit_event(RawEvent::RenewFileSuccess(liquidator.clone(), cid.clone()));
            }
        }
        Ok(())
    }

    fn maybe_reward_liquidator(cid: &MerkleRoot, curr_bn: BlockNumber, liquidator: &T::AccountId) -> DispatchResult {
        if let Some(mut file_info) = Self::filesv2(cid) {
            if curr_bn >= file_info.expired_at {
                let reward_liquidator_amount = file_info.amount;
                file_info.amount = Zero::zero();
                T::Currency::transfer(&Self::storage_pot(), liquidator, reward_liquidator_amount, KeepAlive)?;
            }

            file_info.calculated_at = curr_bn.min(file_info.expired_at);
            <FilesV2<T>>::insert(cid, file_info);
        }
        Ok(())
    }

    fn upsert_new_file_info(cid: &MerkleRoot, amount: &BalanceOf<T>, curr_bn: &BlockNumber, file_size: u64) {
        // Extend expired_at
        if let Some(mut file_info) = Self::filesv2(cid) {
            // expired_at > calculated_at => file is ongoing.
            // expired_at == calculated_at => file is ready to be closed(wait to be refreshed).
            // expired_at < calculated_at => file is not live yet. This situation only happen for new file.
            // If it's ready to be closed, refresh the calculated_at to the current bn
            if file_info.expired_at == file_info.calculated_at {
                file_info.calculated_at = *curr_bn;
            }

            if file_info.replicas.len() == 0 {
                // turn this file into pending status since replicas.len() is zero
                // we keep the original amount
                file_info.expired_at = 0;
            } else {
                // Refresh the file to be a new file
                file_info.expired_at = curr_bn + T::FileDuration::get();
            }

            file_info.amount += amount.clone();
            <FilesV2<T>>::insert(cid, file_info);
        } else {
            // New file
            let file_info = FileInfoV2::<T::AccountId, BalanceOf<T>> {
                file_size,
                spower: 0,
                expired_at: 0,
                calculated_at: curr_bn.clone(),
                amount: amount.clone(),
                prepaid: Zero::zero(),
                remaining_paid_count: REWARD_PERSON,
                reported_replica_count: 0u32,
                replicas: BTreeMap::new()
            };
            <FilesV2<T>>::insert(cid, file_info);
            FileKeysCount::mutate(|count| *count = count.saturating_add(1));
        }
    }

    fn init_pot(account: fn() -> T::AccountId) {
        let account_id = account();
        let min = T::Currency::minimum_balance();
        if T::Currency::free_balance(&account_id) < min {
            let _ = T::Currency::make_free_balance_be(
                &account_id,
                min,
            );
        }
    }

    fn has_enough_collateral(who: &T::AccountId, value: &BalanceOf<T>) -> Option<BalanceOf<T>> {
        let (collateral, reward) = T::BenefitInterface::get_collateral_and_reward(who);
        if (reward + *value).saturating_mul(COLLATERAL_RATIO.into()) <= collateral {
            return Some(reward + *value);
        }
        None
    }

    /// Calculate file price
    /// Include the file base fee, file byte price and files count price
    /// return => (file_base_fee, file_byte_price + file_keys_count_fee)
    pub fn get_file_fee(file_size: u64) -> (BalanceOf<T>, BalanceOf<T>) {
        // 1. Calculate file size price
        // Rounded file size from `bytes` to `megabytes`
        let mut rounded_file_size = file_size / 1_048_576;
        if file_size % 1_048_576 != 0 {
            rounded_file_size += 1;
        }
        let price = Self::file_byte_fee();
        // Convert file size into `Currency`
        let amount = price.checked_mul(&BalanceOf::<T>::saturated_from(rounded_file_size));
        let file_bytes_price = match amount {
            Some(value) => value,
            None => Zero::zero(),
        };
        // 2. Get file base fee
        let file_base_fee = Self::file_base_fee();
        // 3. Get files count price
        let file_keys_count_fee = Self::file_keys_count_fee();

        (file_base_fee, file_bytes_price + file_keys_count_fee)
    }

    pub fn update_file_byte_fee() {
        let (files_size, free) = T::SworkerInterface::get_files_size_and_free_space();
        let total_capacity = files_size.saturating_add(free);
        let (numerator, denominator) = T::StorageReferenceRatio::get();
        let min_file_byte_fee = Self::min_file_byte_fee();
        // Too much supply => decrease the price
        if files_size.saturating_mul(denominator) <= total_capacity.saturating_mul(numerator) {
            <FileByteFee<T>>::mutate(|file_byte_fee| {
                let gap = T::StorageDecreaseRatio::get() * file_byte_fee.clone();
                *file_byte_fee = file_byte_fee.saturating_sub(gap).max(min_file_byte_fee);
            });
        } else {
            <FileByteFee<T>>::mutate(|file_byte_fee| {
                let gap = (T::StorageIncreaseRatio::get() * file_byte_fee.clone()).max(BalanceOf::<T>::saturated_from(1u32));
                *file_byte_fee = file_byte_fee.saturating_add(gap).max(min_file_byte_fee);
            });
        }
    }

    pub fn update_file_keys_count_fee() {
        let files_count = Self::files_count();
        let min_file_keys_count_fee = Self::min_file_keys_count_fee();
        if files_count > FILES_COUNT_REFERENCE {
            // TODO: Independent mechanism
            <FileKeysCountFee<T>>::mutate(|file_keys_count_fee| {
                let gap = (T::StorageIncreaseRatio::get() * file_keys_count_fee.clone()).max(BalanceOf::<T>::saturated_from(1u32));
                *file_keys_count_fee = file_keys_count_fee.saturating_add(gap).max(min_file_keys_count_fee);
            })
        } else {
            <FileKeysCountFee<T>>::mutate(|file_keys_count_fee| {
                let gap = T::StorageDecreaseRatio::get() * file_keys_count_fee.clone();
                *file_keys_count_fee = file_keys_count_fee.saturating_sub(gap).max(min_file_keys_count_fee);
            })
        }
    }

    pub fn update_base_fee() {
        // get added files count and clear the record
        let added_files_count = T::SworkerInterface::get_added_files_count_and_clear_record();
        // get orders count and clear the record
        let orders_count = Self::orders_count();
        OrdersCount::put(0);
        // decide what to do
        let (is_to_decrease, ratio) = Self::base_fee_ratio(added_files_count.checked_div(orders_count));
        let min_file_base_fee = Self::min_file_base_fee();
        // update the file base fee
        <FileBaseFee<T>>::mutate(|file_base_fee| {
            let gap = ratio * file_base_fee.clone();
            if is_to_decrease {
                *file_base_fee = file_base_fee.saturating_sub(gap).max(min_file_base_fee);
            } else {
                *file_base_fee = file_base_fee.saturating_add(gap).max(min_file_base_fee);
            }
        })
    }

    /// return (bool, ratio)
    /// true => decrease the price, false => increase the price
    pub fn base_fee_ratio(maybe_alpha: Option<u32>) -> (bool, Perbill) {
        match maybe_alpha {
            // New order => check the alpha
            Some(alpha) => {
                match alpha {
                    0 ..= 1 => (false,Perbill::from_percent(20)),
                    2 => (false,Perbill::from_percent(18)),
                    3 => (false,Perbill::from_percent(15)),
                    4 => (false,Perbill::from_percent(12)),
                    5 => (false,Perbill::from_percent(10)),
                    6 => (false,Perbill::from_percent(8)),
                    7 => (false,Perbill::from_percent(6)),
                    8 => (false,Perbill::from_percent(4)),
                    9 => (false,Perbill::from_percent(2)),
                    10 ..= 30 => (false,Perbill::zero()),
                    31 ..= 50 => (true,Perbill::from_percent(3)),
                    _ => (true, Perbill::from_percent(5))
                }
            },
            // No new order => decrease the price
            None => (true, Perbill::from_percent(5))
        }
    }

    // Split total value into three pot and return the amount in storage pot
    // Currently
    // 10% into reserved pot
    // 72% into staking pot
    // 18% into storage pot
    fn split_into_reserved_and_storage_and_staking_pot(who: &T::AccountId, value: BalanceOf<T>, base_fee: BalanceOf<T>, tips: BalanceOf<T>, liveness: ExistenceRequirement) -> Result<BalanceOf<T>, DispatchError> {
        // Calculate staking amount and storage amount
        // 18% into storage pot
        // 72% into staking pot
        let staking_amount = T::StakingRatio::get() * value;
        let storage_amount = T::StorageRatio::get() * value;

        // Calculate the discount for the total amount
        // discount_amount = total_amount * min(market_funds_ratio, 0.1)
        // reserved_amount = total_amount - staking_amount - storage_amount - discount_amount
        let total_amount = value.saturating_add(base_fee);
        let reserved_amount = total_amount.saturating_sub(staking_amount).saturating_sub(storage_amount);

        // Add the tips into storage amount
        let storage_amount = storage_amount + tips;

        T::Currency::transfer(&who, &Self::reserved_pot(), reserved_amount, liveness)?;
        T::Currency::transfer(&who, &Self::staking_pot(), staking_amount, liveness)?;
        T::Currency::transfer(&who, &Self::storage_pot(), storage_amount.clone(), liveness)?;
        Ok(storage_amount)
    }

    // discount feature is not implemented yet, comment out first to remove the build warning
    // fn get_discount_ratio(who: &T::AccountId) -> Perbill {
    //     let discount_max_ratio = Perbill::one().saturating_sub(T::StakingRatio::get()).saturating_sub(T::StorageRatio::get());
    //     T::BenefitInterface::get_market_funds_ratio(who).min(discount_max_ratio)
    // }


    fn get_current_block_number() -> BlockNumber {
        let current_block_number = <system::Module<T>>::block_number();
        TryInto::<u32>::try_into(current_block_number).ok().unwrap()
    }

    fn maybe_reward_merchant(who: &T::AccountId, amount: &BalanceOf<T>) -> bool {
        if let Some(owner) = T::SworkerInterface::get_owner(who) {
            if let Some(new_reward) = Self::has_enough_collateral(&owner, amount) {
                T::BenefitInterface::update_reward(&owner, new_reward);
                return true;
            }
        }
        false
    }

    fn calculate_reward_amount(remaining_paid_count: u32, amount: &BalanceOf<T>) -> BalanceOf<T> {
        // x = 2.5 / (18 - 2.5 * {0, 1, 2, 3})
        match remaining_paid_count {
            4u32 => Perbill::from_parts(138888888) * *amount, // 2.5 / 18
            3u32 => Perbill::from_parts(161290320) * *amount, // 2.5 / 15.5
            2u32 => Perbill::from_parts(192307690) * *amount, // 2.5 / 13
            1u32 => Perbill::from_parts(238095240) * *amount, // 2.5 / 10.5
            _ => Zero::zero()
        }
    }

    fn update_replicas_spower(file_info: &mut FileInfoV2<T::AccountId, BalanceOf<T>>, curr_bn: Option<BlockNumber>) -> u64 {
        let new_spower = Self::calculate_spower(file_info.file_size, file_info.reported_replica_count);
        let prev_spower = file_info.spower;
        let mut replicas_count = 0;
        for (_onwer, ref mut replica) in &mut file_info.replicas {
            // already begin to use spower
            if replica.created_at.is_none() {
                replicas_count += 1;
                T::SworkerInterface::update_spower(&replica.anchor, prev_spower, new_spower);
            } else {
                if let Some(curr_bn) = curr_bn {
                    // Make it become valid
                    if let Some(created_at) = replica.created_at {
                        if created_at + Self::spower_ready_period() <= curr_bn {
                            replicas_count += 1;
                            T::SworkerInterface::update_spower(&replica.anchor, file_info.file_size, new_spower);
                            replica.created_at = None;
                        }
                    }
                } else {
                    // File is to close
                    replicas_count += 1;
                    T::SworkerInterface::update_spower(&replica.anchor, file_info.file_size, new_spower);
                }
            }
        }
        file_info.spower = new_spower;
        replicas_count
    }

    pub fn calculate_spower(file_size: u64, reported_replica_count: u32) -> u64 {
        let (alpha, multiplier): (f64, u64) = match reported_replica_count {
            0 => (0.0, 1),
            1..=8 => (0.1, 10),
            9..=16 => (1.0, 1),
            17..=24 => (3.0, 1),
            25..=32 => (7.0, 1),
            33..=40 => (9.0, 1),
            41..=48 => (14.0, 1),
            49..=55 => (19.0, 1),
            56..=65 => (49.0, 1),
            66..=74 => (79.0, 1),
            75..=83 => (99.0, 1),
            84..=92 => (119.0, 1),
            93..=100 => (149.0, 1),
            101..=115 => (159.0, 1),
            116..=127 => (169.0, 1),
            128..=142 => (179.0, 1),
            143..=157 => (189.0, 1),
            158..=200 => (199.0, 1),
            _ => (199.0, 1), // larger than 200 => 200
        };

        file_size + file_size * ((alpha * multiplier as f64) as u64) / multiplier
    }
}

decl_event!(
    pub enum Event<T>
    where
        AccountId = <T as system::Config>::AccountId,
        Balance = BalanceOf<T>
    {
        /// Place a storage order success.
        /// The first item is the account who places the storage order.
        /// The second item is the cid of the file.
        FileSuccess(AccountId, MerkleRoot),
        /// Renew an existed file success.
        /// The first item is the account who renew the storage order.
        /// The second item is the cid of the file.
        RenewFileSuccess(AccountId, MerkleRoot),
        /// Add prepaid value for an existed file success.
        /// The first item is the account who add the prepaid.
        /// The second item is the cid of the file.
        /// The third item is the prepaid amount of currency.
        AddPrepaidSuccess(AccountId, MerkleRoot, Balance),
        /// Calculate the reward for a file success.
        /// The first item is the cid of the file.
        CalculateSuccess(MerkleRoot),
        /// A file is closed due to mismatch file size.
        /// The first item is the cid of the file.
        IllegalFileClosed(MerkleRoot),
        /// Reward the merchant success.
        /// The first item is the account of the merchant.
        RewardMerchantSuccess(AccountId),
        /// Set the global market switch success.
        SetEnableMarketSuccess(bool),
        /// Set the file base fee success.
        SetBaseFeeSuccess(Balance),
        /// Set the crust-spower service superior account.
        SetSpowerSuperiorSuccess(AccountId),
        /// Update replicas success
        /// The first item is the account who update the replicas.
        /// The second item is the current block number
        /// The third item is the changed files count
        /// The fourth item is the last processed block of work reports
        UpdateReplicasSuccess(AccountId, BlockNumber, u32, BlockNumber),
        /// A file is closed due to expired
        /// The first item is the cid of the file
        FileClosed(MerkleRoot),
    }
);

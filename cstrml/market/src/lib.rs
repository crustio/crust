// Copyright (C) 2019-2021 Crust Network Technologies Ltd.
// This file is part of Crust.

#![cfg_attr(not(feature = "std"), no_std)]
#![feature(option_result_contains)]

use codec::{Decode, Encode};
use frame_support::{
    decl_event, decl_module, decl_storage, decl_error,
    dispatch::DispatchResult, ensure,
    traits::{
        Currency, ReservableCurrency, Get, LockableCurrency, ExistenceRequirement,
        ExistenceRequirement::{AllowDeath, KeepAlive},
        WithdrawReasons, Imbalance
    },
    weights::Weight
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
    MerkleRoot, BlockNumber, SworkerAnchor,
    constants::market::*,
    traits::{
        UsableCurrency, MarketInterface,
        SworkerInterface, BenefitInterface
    }
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
}

#[derive(Debug, PartialEq, Eq, Clone, Encode, Decode, Default)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct FileInfo<AccountId, Balance> {
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
    // The replica list, distinct by group
    pub replicas: Vec<Replica<AccountId>>
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
    // None: means who += spower
    // Some: means who += file_size
    pub created_at: Option<BlockNumber>
}

type BalanceOf<T> = <<T as Config>::Currency as Currency<<T as system::Config>::AccountId>>::Balance;
type PositiveImbalanceOf<T> = <<T as Config>::Currency as Currency<<T as system::Config>::AccountId>>::PositiveImbalance;
type NegativeImbalanceOf<T> = <<T as Config>::Currency as Currency<<T as system::Config>::AccountId>>::NegativeImbalance;

impl<T: Config> MarketInterface<<T as system::Config>::AccountId, BalanceOf<T>> for Module<T>
{
    /// Upsert new replica
    /// Accept id(who, anchor), reported_file_size, cid, valid_at and maybe_member
    /// Returns the real storage power of this file and whether this file is in the market system
    /// storage power is decided by market
    fn upsert_replica(who: &<T as system::Config>::AccountId,
                      owner: <T as system::Config>::AccountId,
                      cid: &MerkleRoot,
                      reported_file_size: u64,
                      anchor: &SworkerAnchor,
                      valid_at: BlockNumber,
                      maybe_members: &Option<BTreeSet<<T as system::Config>::AccountId>>
    ) -> (u64, bool) {
        // Judge if file_info.file_size == reported_file_size or not
        Self::maybe_upsert_file_size(who, cid, reported_file_size);

        // `is_counted` is a concept in swork-side, which means if this `cid`'s `storage power` size is counted by `(who, anchor)`
        // if the file doesn't exist/exceed-replicas(aka. is_counted == false), return false(doesn't increase storage power) cause it's junk.
        // if the file exist, is_counted == true, will change it later.
        let mut spower: u64 = 0;
        let mut is_valid_cid: bool = false;
        if let Some(mut file_info) = <Files<T>>::get(cid) {
            is_valid_cid = true;
            // 1. Check if the length of the groups exceed MAX_REPLICAS or not
            let mut is_counted = file_info.replicas.len() < MAX_REPLICAS;
            // 2. Check if the file is stored by other members
            if is_counted {
                if let Some(members) = maybe_members {
                    for replica in file_info.replicas.iter() {
                        if members.contains(&replica.who) {
                            if T::SworkerInterface::check_anchor(&replica.who, &replica.anchor) {
                                // duplicated in group and set is_counted to false
                                is_counted = false;
                                break;
                            }
                        }
                    }
                }
            }

            // 3. Prepare new replica info
            if is_counted {
                let new_replica = Replica {
                    who: who.clone(),
                    valid_at,
                    anchor: anchor.clone(),
                    is_reported: true,
                    // set created_at to some
                    created_at: Some(valid_at)
                };
                file_info.replicas.push(new_replica);
                file_info.reported_replica_count += 1;
                // Always return the file size for this [who] reported first time
                spower = file_info.file_size;
            }

            // 4. The first join the replicas and file become live(expired_at > calculated_at)
            if file_info.expired_at == 0 {
                let curr_bn = Self::get_current_block_number();
                file_info.calculated_at = curr_bn;
                file_info.expired_at = curr_bn + T::FileDuration::get();
            }

            // 5. Update files
            <Files<T>>::insert(cid, file_info);
        } else if let Some(mut file_info) = <FilesV2<T>>::get(cid) {
            is_valid_cid = true;
            // 1. Check if the length of the groups exceed MAX_REPLICAS or not
            if file_info.replicas.len() < MAX_REPLICAS {
                // 2. Check if the file is stored by other members
                if !file_info.replicas.contains_key(&owner) {
                    let new_replica = Replica {
                        who: who.clone(),
                        valid_at,
                        anchor: anchor.clone(),
                        is_reported: true,
                        // set created_at to some
                        created_at: Some(valid_at)
                    };
                    file_info.replicas.insert(owner.clone(), new_replica);
                    file_info.reported_replica_count += 1;
                    // Always return the file size for this [who] reported first time
                    spower = file_info.file_size;

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
                let curr_bn = Self::get_current_block_number();
                file_info.calculated_at = curr_bn;
                file_info.expired_at = curr_bn + T::FileDuration::get();
            }

            // 4. Update files
            <FilesV2<T>>::insert(cid, file_info);
        }
        (spower, is_valid_cid)
    }

    /// Node who delete the replica
    /// Accept id(who, anchor), cid and current block number
    /// Returns the real storage power of this file and whether this file is in the market system
    fn delete_replica(who: &<T as system::Config>::AccountId,
                      owner: <T as system::Config>::AccountId,
                      cid: &MerkleRoot,
                      anchor: &SworkerAnchor) -> (u64, bool) {
        let mut spower: u64 = 0;
        let mut is_valid_cid: bool = false;
        // 1. Delete replica from file_info
        if let Some(mut file_info) = <Files<T>>::get(cid) {
            is_valid_cid = true;
            let mut to_decrease_count = 0;
            // None => No such file
            // Some(true) => Already pass the SpowerReadyPeriod, decrease the spower
            // Some(false) => Still in SpowerReadyPeriod, decrease the file_size
            let mut is_spower_counted: Option<bool> = None;
            file_info.replicas.retain(|replica| {
                if replica.who == *who {
                    if replica.anchor == *anchor {
                        // We added it before
                        if replica.created_at.is_none() { is_spower_counted = Some(true); } else { is_spower_counted = Some(false); };
                    }
                    if replica.is_reported {
                        // if this anchor didn't report work, we already decrease the `reported_replica_count` in `update_replicas`
                        to_decrease_count += 1;
                    }
                }
                replica.who != *who
            });

            // 2. Return the original storage power in wr
            if let Some(is_spower_counted) = is_spower_counted {
                if is_spower_counted {
                    spower = file_info.spower;
                } else {
                    spower = file_info.file_size;
                }
            }

            // 3. Decrease the reported_replica_count
            if to_decrease_count != 0 {
                file_info.reported_replica_count = file_info.reported_replica_count.saturating_sub(to_decrease_count);
            }
            <Files<T>>::insert(cid, file_info);
        } else if let Some(mut file_info) = <FilesV2<T>>::get(cid) {
            is_valid_cid = true;
            let mut to_decrease_count = 0;
            // None => No such file
            // Some(true) => Already pass the SpowerReadyPeriod, decrease the spower
            // Some(false) => Still in SpowerReadyPeriod, decrease the file_size
            let mut is_spower_counted: Option<bool> = None;
            let maybe_replica = file_info.replicas.get(&owner);
            if let Some(replica) = maybe_replica {
                if replica.who == *who {
                    if replica.anchor == *anchor {
                        // We added it before
                        if replica.created_at.is_none() { is_spower_counted = Some(true); } else { is_spower_counted = Some(false); };
                    }
                    if replica.is_reported {
                        // if this anchor didn't report work, we already decrease the `reported_replica_count` in `update_replicas`
                        to_decrease_count += 1;
                    }
                    file_info.replicas.remove(&owner);
                }
            }

            // 2. Return the original storage power in wr
            if let Some(is_spower_counted) = is_spower_counted {
                if is_spower_counted {
                    spower = file_info.spower;
                } else {
                    spower = file_info.file_size;
                }
            }

            // 3. Decrease the reported_replica_count
            if to_decrease_count != 0 {
                file_info.reported_replica_count = file_info.reported_replica_count.saturating_sub(to_decrease_count);
            }
            <FilesV2<T>>::insert(cid, file_info);
        }
        (spower, is_valid_cid)
    }

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

        /// The file price per MB.
        /// It's dynamically adjusted and would change according to FilesSize, TotalCapacity and StorageReferenceRatio.
        pub FileByteFee get(fn file_byte_fee): BalanceOf<T> = T::InitFileByteFee::get();

        /// Files count, determinate the FileKeysCountFee
        pub FileKeysCount get(fn files_count): u32 = 0;

        /// The file price by keys
        /// It's dynamically adjusted and would change according to the total keys in files
        pub FileKeysCountFee get(fn file_keys_count_fee): BalanceOf<T> = T::InitFileKeysCountFee::get();

        /// File information iterated by order id
        pub Files get(fn files):
        map hasher(twox_64_concat) MerkleRoot => Option<FileInfo<T::AccountId, BalanceOf<T>>>;

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

            // 4. Refresh the status of the file and calculate the reward for merchants
            Self::update_replicas(&cid, curr_bn);

            // 5. Try to renew file if prepaid is not zero
            Self::try_to_renew_file(&cid, curr_bn, &liquidator)?;

            // 6. Try to close file
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

        /// Migrate the file to file v2
        /// TODO: Need to weight this one!!!!!!!
        #[weight = T::WeightInfo::reward_merchant()]
        pub fn do_file_migration(
            origin,
            files_count: u32
        ) -> DispatchResult {
            let _ = ensure_signed(origin)?;

            let mut count = 0u32;
            for (cid, file) in <Files<T>>::iter() {
                let mut new_replicas = BTreeMap::<T::AccountId, Replica<T::AccountId>>::new();
                for replica in file.replicas {
                    let mut owner = replica.who.clone();
                    if let Some(group_owner) = T::SworkerInterface::get_owner(&replica.who) {
                        owner = group_owner;
                    }
                    new_replicas.insert(owner, replica);
                }
                let file_info = FileInfoV2::<T::AccountId, BalanceOf<T>> {
                    file_size: file.file_size,
                    spower: file.spower,
                    expired_at: file.expired_at,
                    calculated_at: file.calculated_at,
                    amount: file.amount,
                    prepaid: file.prepaid,
                    remaining_paid_count: 0u32,
                    reported_replica_count: file.reported_replica_count,
                    replicas: new_replicas
                };
                <FilesV2<T>>::insert(&cid, file_info);
                <Files<T>>::remove(cid);
                count += 1;
                if count > files_count {
                    break;
                }
            }

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

    /// This function will update replicas
    /// and (maybe) insert file's status(delete file)
    /// input:
    ///   - cid: MerkleRoot
    ///   - curr_bn: BlockNumber
    pub fn update_replicas(cid: &MerkleRoot, curr_bn: BlockNumber)
    {
        // 1. File must exist
        if Self::filesv2(cid).is_none() { return; }
        
        // 2. File must already started
        let mut file_info = Self::filesv2(cid).unwrap_or_default();
        
        // 3. File already expired
        if file_info.expired_at <= file_info.calculated_at { return; }

        let calculated_block = curr_bn.min(file_info.expired_at);
        let mut new_replicas = BTreeMap::<T::AccountId, Replica<T::AccountId>>::new();
        let mut new_reported_replica_count = 0u32;

        // 4. Loop replicas and update reported replica count
        for (owner, replica) in file_info.replicas.iter() {
            if !T::SworkerInterface::is_wr_reported(&replica.anchor, curr_bn) {
                let mut invalid_replica = replica.clone();
                // update the valid_at to the curr_bn
                invalid_replica.valid_at = curr_bn;
                invalid_replica.is_reported = false;
                new_replicas.insert(owner.clone(), invalid_replica);
            } else {
                let mut valid_replica = replica.clone();
                valid_replica.is_reported = true;
                new_replicas.insert(owner.clone(), valid_replica);
                new_reported_replica_count += 1;
            }
        }

        // 5 Update file info
        file_info.reported_replica_count = new_reported_replica_count;
        file_info.replicas = new_replicas;

        // 6. Update spower info
        // TODO: add this weight into place_storage_order
        let _ = Self::update_replicas_spower(&mut file_info, Some(curr_bn));

        // 6. File status might become ready to be closed if calculated_block == expired_at
        file_info.calculated_at = calculated_block;
        // 7. Update files
        <FilesV2<T>>::insert(cid, file_info);
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
                <FilesV2<T>>::insert(cid, file_info);
            }
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
        // Too much supply => decrease the price
        if files_size.saturating_mul(denominator) <= total_capacity.saturating_mul(numerator) {
            <FileByteFee<T>>::mutate(|file_byte_fee| {
                let gap = T::StorageDecreaseRatio::get() * file_byte_fee.clone();
                *file_byte_fee = file_byte_fee.saturating_sub(gap);
            });
        } else {
            <FileByteFee<T>>::mutate(|file_byte_fee| {
                let gap = (T::StorageIncreaseRatio::get() * file_byte_fee.clone()).max(BalanceOf::<T>::saturated_from(1u32));
                *file_byte_fee = file_byte_fee.saturating_add(gap);
            });
        }
    }

    pub fn update_file_keys_count_fee() {
        let files_count = Self::files_count();
        if files_count > FILES_COUNT_REFERENCE {
            // TODO: Independent mechanism
            <FileKeysCountFee<T>>::mutate(|file_keys_count_fee| {
                let gap = (T::StorageIncreaseRatio::get() * file_keys_count_fee.clone()).max(BalanceOf::<T>::saturated_from(1u32));
                *file_keys_count_fee = file_keys_count_fee.saturating_add(gap);
            })
        } else {
            <FileKeysCountFee<T>>::mutate(|file_keys_count_fee| {
                let gap = T::StorageDecreaseRatio::get() * file_keys_count_fee.clone();
                *file_keys_count_fee = file_keys_count_fee.saturating_sub(gap);
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
        // update the file base fee
        <FileBaseFee<T>>::mutate(|file_base_fee| {
            let gap = ratio * file_base_fee.clone();
            if is_to_decrease {
                *file_base_fee = file_base_fee.saturating_sub(gap);
            } else {
                *file_base_fee = file_base_fee.saturating_add(gap);
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

    fn get_discount_ratio(who: &T::AccountId) -> Perbill {
        let discount_max_ratio = Perbill::one().saturating_sub(T::StakingRatio::get()).saturating_sub(T::StorageRatio::get());
        T::BenefitInterface::get_market_funds_ratio(who).min(discount_max_ratio)
    }


    fn get_current_block_number() -> BlockNumber {
        let current_block_number = <system::Module<T>>::block_number();
        TryInto::<u32>::try_into(current_block_number).ok().unwrap()
    }

    fn maybe_upsert_file_size(who: &T::AccountId, cid: &MerkleRoot, reported_file_size: u64) {
        if let Some(mut file_info) = Self::filesv2(cid) {
            if file_info.replicas.len().is_zero() {
                // ordered_file_size == reported_file_size, return it
                if file_info.file_size == reported_file_size {
                    return
                // ordered_file_size > reported_file_size, correct it
                } else if file_info.file_size > reported_file_size {
                    file_info.file_size = reported_file_size;
                    <FilesV2<T>>::insert(cid, file_info);
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
                }
            }
        } else if let Some(mut file_info) = Self::files(cid) {
            if file_info.replicas.len().is_zero() {
                // ordered_file_size == reported_file_size, return it
                if file_info.file_size == reported_file_size {
                    return
                // ordered_file_size > reported_file_size, correct it
                } else if file_info.file_size > reported_file_size {
                    file_info.file_size = reported_file_size;
                    <Files<T>>::insert(cid, file_info);
                // ordered_file_size < reported_file_size, close it with notification
                } else {
                    let total_amount = file_info.amount + file_info.prepaid;
                    if !Self::maybe_reward_merchant(who, &total_amount) {
                        // This should not have error => discard the result
                        let _ = T::Currency::transfer(&Self::storage_pot(), &Self::reserved_pot(), total_amount, KeepAlive);
                    }
                    <Files<T>>::remove(cid);
                    FileKeysCount::mutate(|count| *count = count.saturating_sub(1));
                    OrdersCount::mutate(|count| {*count = count.saturating_sub(1)});
                    Self::deposit_event(RawEvent::IllegalFileClosed(cid.clone()));
                }
            }
        }
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
        let (integer, numerator, denominator): (u64, u64, u64) = match reported_replica_count {
            0 => (0, 0, 1),
            1..=8 => (1, 1, 20),
            9..=16 => (1, 1, 5),
            17..=24 => (1, 1, 2),
            25..=32 => (2, 0, 1),
            33..=40 => (2, 3, 5),
            41..=48 => (3, 3, 10),
            49..=55 => (4, 0, 1),
            56..=65 => (5, 0, 1),
            66..=74 => (6, 0, 1),
            75..=83 => (7, 0, 1),
            84..=92 => (8, 0, 1),
            93..=100 => (8, 1, 2),
            101..=115 => (8, 4, 5),
            116..=127 => (9, 0, 1),
            128..=142 => (9, 1, 5),
            143..=157 => (9, 2, 5),
            158..=167 => (9, 3, 5),
            168..=182 => (9, 4, 5),
            183..=200 => (10, 0, 1),
            _ => (10, 0, 1), // larger than 200 => 200
        };

        integer * file_size + file_size / denominator * numerator
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
    }
);

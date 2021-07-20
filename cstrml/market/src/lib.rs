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
use sp_std::{prelude::*, convert::TryInto, collections::btree_set::BTreeSet};
use frame_system::{self as system, ensure_signed, ensure_root};
use sp_runtime::{SaturatedConversion, Perbill, ModuleId, traits::{Zero, CheckedMul, AccountIdConversion, Saturating, StaticLookup}, DispatchError};

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
    fn bond() -> Weight;
    fn place_storage_order() -> Weight;
    fn calculate_reward() -> Weight;
    fn reward_merchant() -> Weight;
    fn recharge_free_order_pot() -> Weight;
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
    pub amount: Balance,
    // The pre paid pool
    // TODO: useless field, prepared for the future upgrade
    pub prepaid: Balance,
    // The count of valid replica each report slot
    pub reported_replica_count: u32,
    // The replica list, distinct by group
    pub replicas: Vec<Replica<AccountId>>
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
    /// Returns the real storage power of this file
    /// storage power is decided by market
    fn upsert_replica(who: &<T as system::Config>::AccountId,
                      cid: &MerkleRoot,
                      reported_file_size: u64,
                      anchor: &SworkerAnchor,
                      valid_at: BlockNumber,
                      maybe_members: &Option<BTreeSet<<T as system::Config>::AccountId>>
    ) -> u64 {
        // Judge if file_info.file_size == reported_file_size or not
        Self::maybe_upsert_file_size(who, cid, reported_file_size);

        // `is_counted` is a concept in swork-side, which means if this `cid`'s `storage power` size is counted by `(who, anchor)`
        // if the file doesn't exist/exceed-replicas(aka. is_counted == false), return false(doesn't increase storage power) cause it's junk.
        // if the file exist, is_counted == true, will change it later.
        let mut spower: u64 = 0;
        if let Some(mut file_info) = <Files<T>>::get(cid) {
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
                Self::insert_replica(&mut file_info, new_replica);
                PendingFiles::mutate(|files| {
                    files.insert(cid.clone());
                });
                file_info.reported_replica_count += 1;
                // Always return the file size for the first time
                spower = file_info.file_size;
            }

            // 4. The first join the replicas and file become live(expired_at > calculated_at)
            let curr_bn = Self::get_current_block_number();
            if file_info.expired_at == 0 {
                file_info.calculated_at = curr_bn;
                file_info.expired_at = curr_bn + T::FileDuration::get();
            }

            // 5. Update files
            <Files<T>>::insert(cid, file_info);
        }
        spower
    }

    /// Node who delete the replica
    /// Accept id(who, anchor), cid and current block number
    /// Returns the real storage power of this file
    fn delete_replica(who: &<T as system::Config>::AccountId, cid: &MerkleRoot, anchor: &SworkerAnchor) -> u64 {
        let mut spower: u64 = 0;
        // 1. Delete replica from file_info
        if let Some(mut file_info) = <Files<T>>::get(cid) {
            let mut to_decrease_count = 0;
            let mut is_valid: Option<bool> = None;
            file_info.replicas.retain(|replica| {
                if replica.who == *who {
                    if replica.anchor == *anchor {
                        // We added it before
                        if replica.created_at.is_none() { is_valid = Some(true); } else { is_valid = Some(false); };
                    }
                    if replica.is_reported {
                        // if this anchor didn't report work, we already decrease the `reported_replica_count` in `do_calculate_reward`
                        to_decrease_count += 1;
                    }
                }
                replica.who != *who
            });

            // 2. Return the original storage power in wr
            if let Some(is_valid) = is_valid {
                if is_valid {
                    spower = file_info.spower;
                } else {
                    spower = file_info.file_size;
                }
            }

            // 3. Decrease the reported_replica_count
            if to_decrease_count != 0 {
                file_info.reported_replica_count = file_info.reported_replica_count.saturating_sub(to_decrease_count);
                PendingFiles::mutate(|files| {
                    files.insert(cid.clone());
                });
            }
            <Files<T>>::insert(cid, file_info);
        }
        spower
    }

    // withdraw market staking pot for distributing staking reward
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
        /// The file base fee for each storage order.
        pub FileBaseFee get(fn file_base_fee): BalanceOf<T> = Zero::zero();

        /// The file price per MB.
        /// It's dynamically adjusted and would change according to FilesSize, TotalCapacity and StorageReferenceRatio.
        pub FileByteFee get(fn file_byte_fee): BalanceOf<T> = T::InitFileByteFee::get();

        /// The file price by keys
        /// It's dynamically adjusted and would change according to the total keys in files
        pub FileKeysCountFee get(fn file_keys_count_fee): BalanceOf<T> = T::InitFileKeysCountFee::get();

        /// Bonding Information
        pub Bonded get(fn bonded):
        map hasher(blake2_128_concat) T::AccountId => Option<T::AccountId>;

        /// File information iterated by order id
        pub Files get(fn files):
        map hasher(twox_64_concat) MerkleRoot => Option<FileInfo<T::AccountId, BalanceOf<T>>>;

        /// The free space account list
        pub FreeOrderAccounts get(fn free_order_accounts):
        map hasher(twox_64_concat) T::AccountId => Option<u32>;

        /// Files count
        pub FileKeysCount get(fn files_count): u32 = 0;

        /// New order in the past blocks
        NewOrder get(fn new_order): bool = false;

        /// New orders count in the past one period(one hour)
        OrdersCount get(fn orders_count): u32 = 0;

        /// Wait for updating storage power for all replicas
        pub PendingFiles get(fn pending_files): BTreeSet<MerkleRoot>;

        /// The global market switch to enable place storage order
        pub MarketSwitch get(fn market_switch): bool = false;

        /// The used size to become valid duration
        pub ValidDuration get(fn valid_duration): BlockNumber = 1_296_000; // 3 months

        /// The upper limit for free counts
        pub FreeCountsLimit get(fn free_counts_limit): u32 = 1000;

        FreeOrderAdmin get(fn free_order_admin): Option<T::AccountId>;
    }
    add_extra_genesis {
		build(|_config| {
			// Create the market accounts
			<Module<T>>::init_pot(<Module<T>>::collateral_pot);
			<Module<T>>::init_pot(<Module<T>>::storage_pot);
			<Module<T>>::init_pot(<Module<T>>::staking_pot);
			<Module<T>>::init_pot(<Module<T>>::reserved_pot);
			<Module<T>>::init_pot(<Module<T>>::free_order_pot);
		});
	}
}

decl_error! {
    /// Error for the market module.
    pub enum Error for Module<T: Config> {
        /// Don't have enough currency(CRU) to finish the extrinsic(transaction).
        /// Please transfer some CRU into this account.
        InsufficientCurrency,
        /// Can not choose the value less than the minimum balance.
        /// Please increase the value to be larger than the minimu balance.
        InsufficientValue,
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
        /// FreeOrderAdmin not exist or it's illegal.
        IllegalFreeOrderAdmin,
        /// The account already in free accounts
        AlreadyInFreeAccounts,
        /// The free count exceed the upper limit
        ExceedFreeCountsLimit,
        /// Free account cannot assign tips
        InvalidTip
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

        /// The storage reference ratio to adjust the file price.
        const StorageReferenceRatio: (u128, u128) = T::StorageReferenceRatio::get();

        /// The storage increase ratio for each file price change.
        const StorageIncreaseRatio: Perbill = T::StorageIncreaseRatio::get();

        /// The storage decrease ratio for each file price change.
        const StorageDecreaseRatio: Perbill = T::StorageDecreaseRatio::get();

        /// The staking ratio for how much CRU into staking pot.
        const StakingRatio: Perbill = T::StakingRatio::get();

        /// The storage ratio for how much CRU into storage pot.
        const StorageRatio: Perbill = T::StorageRatio::get();

        /// The max file size of a file
        const MaximumFileSize: u64 = T::MaximumFileSize::get();

        /// Called when a block is initialized. Will call update_identities to update file price
        fn on_initialize(now: T::BlockNumber) -> Weight {
            let now = TryInto::<u32>::try_into(now).ok().unwrap();
            let mut consumed_weight: Weight = 0;
            let mut add_db_reads_writes = |reads, writes| {
                consumed_weight += T::DbWeight::get().reads_writes(reads, writes);
            };
            if ((now + PRICE_UPDATE_OFFSET) % PRICE_UPDATE_SLOT).is_zero() && Self::new_order(){
                Self::update_file_byte_fee();
                Self::update_file_keys_count_fee();
                NewOrder::put(false);
                add_db_reads_writes(8, 3);
            }
            if ((now + BASE_FEE_UPDATE_OFFSET) % BASE_FEE_UPDATE_SLOT).is_zero() {
                Self::update_base_fee();
                add_db_reads_writes(3, 3);
            }
            add_db_reads_writes(1, 0);
            if ((now + SPOWER_UPDATE_OFFSET) % SPOWER_UPDATE_SLOT).is_zero() || Self::pending_files().len() >= MAX_PENDING_FILES {
                let files = Self::get_files_to_update();
                for cid in files {
                    if let Some(mut file_info) = Self::files(&cid) {
                        let groups_count = Self::update_spower_info(&mut file_info, Some(now));
                        <Files<T>>::insert(cid, file_info);
                        add_db_reads_writes(groups_count, groups_count + 1);
                    }
                    add_db_reads_writes(1, 0);
                }
            }
            add_db_reads_writes(1, 0);
            consumed_weight
        }

        /// Bond the origin to the owner
        #[weight = T::WeightInfo::bond()]
        pub fn bond(
            origin,
            owner: <T::Lookup as StaticLookup>::Source
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;
            let owner = T::Lookup::lookup(owner)?;
            <Bonded<T>>::insert(&who, &owner);
            Ok(())
        }

        /// Place a storage order. The cid and file_size of this file should be provided. Extra tips is accepted.
        #[weight = T::WeightInfo::place_storage_order()]
        pub fn place_storage_order(
            origin,
            cid: MerkleRoot,
            reported_file_size: u64,
            #[compact] tips: BalanceOf<T>,
            memo: Vec<u8>
        ) -> DispatchResult {
            // 1. Service should be available right now.
            ensure!(Self::market_switch(), Error::<T>::PlaceOrderNotAvailable);
            let who = ensure_signed(origin)?;

            // 2. Calculate amount.
            let mut charged_file_size = reported_file_size;
            if let Some(file_info) = Self::files(&cid) {
                if file_info.file_size <= reported_file_size {
                    // Charge user with real file size
                    charged_file_size = file_info.file_size;
                } else {
                    Err(Error::<T>::FileSizeNotCorrect)?
                }
            }
            // 3. charged_file_size should be smaller than 128G
            ensure!(charged_file_size < T::MaximumFileSize::get(), Error::<T>::FileTooLarge);

            // 4. Check whether the account is free or not
            let is_free = <FreeOrderAccounts<T>>::mutate_exists(&who, |maybe_count| match *maybe_count {
                Some(count) => {
                    if count > 1u32 {
                        *maybe_count = Some(count - 1);
                    } else {
                        *maybe_count = None;
                    }
                    Ok(())
                },
                None => {
                    Err(())
                }
            }).is_ok();

            let payer = if is_free {
                ensure!(tips.is_zero(), Error::<T>::InvalidTip);
                Self::free_order_pot()
            } else {
                who.clone()
            };
            let (file_base_fee, amount) = Self::get_file_fee(charged_file_size);

            // 5. Check client can afford the sorder
            ensure!(T::Currency::usable_balance(&payer) >= file_base_fee + amount + tips, Error::<T>::InsufficientCurrency);

            // 6. Split into reserved, storage and staking account
            let amount = Self::split_into_reserved_and_storage_and_staking_pot(&payer, amount.clone(), file_base_fee, tips, AllowDeath)?;

            let curr_bn = Self::get_current_block_number();

            // 7. do calculate reward. Try to close file and decrease first party storage
            Self::do_calculate_reward(&cid, curr_bn);

            // 8. three scenarios: new file, extend time(refresh time)
            Self::upsert_new_file_info(&cid, &amount, &curr_bn, charged_file_size);

            // 9. Update new order status.
            NewOrder::put(true);
            OrdersCount::mutate(|count| {*count = count.saturating_add(1)});

            Self::deposit_event(RawEvent::FileSuccess(who, cid));

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
            if !<Files<T>>::contains_key(&cid) {
                return Ok(());
            }

            let file_info = Self::files(&cid).unwrap();
            let curr_bn = Self::get_current_block_number();

            // 2. File should be live right now and calculate reward should be after expired_at
            ensure!(file_info.expired_at != 0, Error::<T>::NotInRewardPeriod);

            // 3. Maybe reward liquidator when he try to close outdated file
            Self::maybe_reward_liquidator(&cid, curr_bn, &liquidator)?;

            // 4. Refresh the status of the file and calculate the reward for merchants
            Self::do_calculate_reward(&cid, curr_bn);

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

        /// Set the global switch
        #[weight = 1000]
        pub fn set_market_switch(
            origin,
            is_enabled: bool
        ) -> DispatchResult {
            let _ = ensure_root(origin)?;

            MarketSwitch::put(is_enabled);

            Self::deposit_event(RawEvent::SetMarketSwitchSuccess(is_enabled));
            Ok(())
        }

        /// Set the file base fee
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

        /// Recharge the free space pot
        #[weight = T::WeightInfo::recharge_free_order_pot()]
        pub fn recharge_free_order_pot(origin, #[compact] value: BalanceOf<T>) {
            let who = ensure_signed(origin)?;
            ensure!(value >= T::Currency::minimum_balance(), Error::<T>::InsufficientValue);
            ensure!(T::Currency::free_balance(&who) >= value, Error::<T>::InsufficientCurrency);
            let free_order_pot = Self::free_order_pot();
            T::Currency::transfer(&who, &free_order_pot, value, AllowDeath)?;
        }

        /// Add the account into free space list
        #[weight = 1000]
        pub fn add_into_free_order_accounts(
            origin,
            target: <T::Lookup as StaticLookup>::Source,
            free_counts: u32,
        ) -> DispatchResult {
            let signer = ensure_signed(origin)?;
            let maybe_free_order_admin = Self::free_order_admin();

            // 1. Check if free_order_admin exist
            ensure!(maybe_free_order_admin.is_some(), Error::<T>::IllegalFreeOrderAdmin);

            // 2. Check if signer is free_order_admin
            ensure!(Some(&signer) == maybe_free_order_admin.as_ref(), Error::<T>::IllegalFreeOrderAdmin);

            let new_account = T::Lookup::lookup(target)?;

            // 3. Ensure it's a new account not in free accounts
            ensure!(Self::free_order_accounts(&new_account).is_none(), Error::<T>::AlreadyInFreeAccounts);

            // 4. Ensure free count does not exceed the upper limit and is reasonable
            ensure!(free_counts <= Self::free_counts_limit(), Error::<T>::ExceedFreeCountsLimit);

            // 5 Add into free order accounts
            <FreeOrderAccounts<T>>::insert(&new_account, free_counts);

            Self::deposit_event(RawEvent::NewFreeAccount(new_account));
            Ok(())
        }

        /// Remove the account from free space list
        #[weight = 1000]
        pub fn remove_from_free_order_accounts(
            origin,
            target: <T::Lookup as StaticLookup>::Source
        ) -> DispatchResult {
            let signer = ensure_signed(origin)?;
            let maybe_free_order_admin = Self::free_order_admin();

            // 1. Check if free_order_admin exist
            ensure!(maybe_free_order_admin.is_some(), Error::<T>::IllegalFreeOrderAdmin);

            // 2. Check if signer is free_order_admin
            ensure!(Some(&signer) == maybe_free_order_admin.as_ref(), Error::<T>::IllegalFreeOrderAdmin);

            // 3. Remove this account from free space list
            let old_account = T::Lookup::lookup(target)?;
            <FreeOrderAccounts<T>>::remove(&old_account);

            Self::deposit_event(RawEvent::FreeAccountRemoved(old_account));
            Ok(())
        }

        /// Set free order admin
        ///
        /// The dispatch origin for this call must be _Root_.
        ///
        /// Parameter:
        /// - `new_free_order_admin`: The new free_order_admin's address
        #[weight = 1000]
        pub fn set_free_order_admin(origin, new_free_order_admin: <T::Lookup as StaticLookup>::Source) -> DispatchResult {
            ensure_root(origin)?;

            let new_free_order_admin = T::Lookup::lookup(new_free_order_admin)?;

            FreeOrderAdmin::<T>::put(new_free_order_admin.clone());

            Self::deposit_event(RawEvent::SetFreeOrderAdminSuccess(new_free_order_admin));

            Ok(())
        }

        /// Set free account limit
        ///
        /// The dispatch origin for this call must be _Root_.
        ///
        /// Parameter:
        /// - `new_free_count_limit`: The new free count limit
        #[weight = 1000]
        pub fn set_free_counts_limit(origin, new_free_count_limit: u32) -> DispatchResult {
            ensure_root(origin)?;

            FreeCountsLimit::put(new_free_count_limit);

            Self::deposit_event(RawEvent::SetFreeCountsLimitSuccess(new_free_count_limit));

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

    /// The pot of a free space account
    /// This account pot is allowed to death
    pub fn free_order_pot() -> T::AccountId {
        // "modl" ++ "crmarket" ++ "rese" is 16 bytes
        T::ModuleId::get().into_sub_account("free")
    }

    /// Calculate reward from file's replica
    /// This function will calculate the file's reward, update replicas
    /// and (maybe) insert file's status(delete file)
    /// input:
    ///     cid: MerkleRoot
    ///     curr_bn: BlockNumber
    pub fn do_calculate_reward(cid: &MerkleRoot, curr_bn: BlockNumber)
    {
        // 1. File must exist
        if Self::files(cid).is_none() { return; }
        
        // 2. File must already started
        let mut file_info = Self::files(cid).unwrap_or_default();
        
        // 3. File already expired
        if file_info.expired_at <= file_info.calculated_at { return; }

        let calculated_block = curr_bn.min(file_info.expired_at);
        let target_reward_count = file_info.replicas.len().min(T::FileReplica::get() as usize) as u32;
        
        // 4. Calculate payouts, check replicas and update the file_info
        if target_reward_count > 0 {
            // 4.1 Get 1 payout amount and sub 1 to make sure that we won't get overflow
            let one_payout_amount = (Perbill::from_rational_approximation(calculated_block - file_info.calculated_at,
                                                                          (file_info.expired_at - file_info.calculated_at) * target_reward_count) * file_info.amount).saturating_sub(1u32.into());
            let mut rewarded_amount = Zero::zero();
            let mut rewarded_count = 0u32;
            let mut new_replicas: Vec<Replica<T::AccountId>> = Vec::with_capacity(file_info.replicas.len());
            let mut invalid_replicas: Vec<Replica<T::AccountId>> = Vec::with_capacity(file_info.replicas.len());
            
            // 4.2. Loop replicas
            for replica in file_info.replicas.iter() {
                // a. didn't report in prev slot, push back to the end of replica
                if !T::SworkerInterface::is_wr_reported(&replica.anchor, curr_bn) {
                    let mut invalid_replica = replica.clone();
                    // update the valid_at to the curr_bn
                    invalid_replica.valid_at = curr_bn;
                    invalid_replica.is_reported = false;
                    // move it to the end of replica
                    invalid_replicas.push(invalid_replica);
                    // TODO: kick this anchor out of file info
                // b. keep the replica's sequence
                } else {
                    let mut valid_replica = replica.clone();
                    valid_replica.is_reported = true;
                    new_replicas.push(valid_replica);
                    
                    // if payouts is full, just continue
                    if rewarded_count == target_reward_count {
                        continue;
                    }
                    
                    // if that guy is poor, just pass him ☠️
                    // Only the first member in the groups can accept the storage reward.
                    if Self::maybe_reward_merchant(&replica.who, &one_payout_amount) {
                        rewarded_amount += one_payout_amount.clone();
                        rewarded_count +=1;
                    }
                }
            }

            // 4.3 Update file info
            file_info.amount = file_info.amount.saturating_sub(rewarded_amount);
            file_info.reported_replica_count = new_replicas.len() as u32;
            new_replicas.append(&mut invalid_replicas);
            file_info.replicas = new_replicas;
        }

        // 5. Update spower info
        // TODO: add this weight into place_storage_order
        let _ = Self::update_spower_info(&mut file_info, Some(curr_bn));

        // 6. File status might become ready to be closed if calculated_block == expired_at
        file_info.calculated_at = calculated_block;
        // 7. Update files
        <Files<T>>::insert(cid, file_info);
    }

    /// Close file, maybe move into trash
    fn try_to_close_file(cid: &MerkleRoot, curr_bn: BlockNumber) -> DispatchResult {
        if let Some(mut file_info) = <Files<T>>::get(cid) {
            // If it's already expired.
            if file_info.expired_at <= curr_bn && file_info.expired_at == file_info.calculated_at {
                let total_amount = file_info.amount.saturating_add(file_info.prepaid);
                T::Currency::transfer(&Self::storage_pot(), &Self::reserved_pot(), total_amount, KeepAlive)?;

                // Remove all spower from wr
                file_info.reported_replica_count = 0;
                // TODO: add this weight into place_storage_order
                let _ = Self::update_spower_info(&mut file_info, None);

                // Remove files
                <Files<T>>::remove(&cid);
                FileKeysCount::mutate(|count| *count = count.saturating_sub(1));
            };
        }
        Ok(())
    }

    fn maybe_reward_liquidator(cid: &MerkleRoot, curr_bn: BlockNumber, liquidator: &T::AccountId) -> DispatchResult {
        if let Some(mut file_info) = Self::files(cid) {
            // 1. expired_at <= curr_bn <= expired_at + T::FileDuration::get() => no reward for liquidator
            // 2. expired_at + T::FileDuration::get() < curr_bn <= expired_at + T::FileDuration::get() * 2 => linearly reward liquidator
            // 3. curr_bn > expired_at + T::FileDuration::get() * 2 => all amount would be rewarded to the liquidator
            let reward_liquidator_amount = Perbill::from_rational_approximation(curr_bn.saturating_sub(file_info.expired_at).saturating_sub(T::LiquidityDuration::get()), T::LiquidityDuration::get()) * file_info.amount;
            if !reward_liquidator_amount.is_zero() {
                file_info.amount = file_info.amount.saturating_sub(reward_liquidator_amount);
                T::Currency::transfer(&Self::storage_pot(), liquidator, reward_liquidator_amount, KeepAlive)?;
                <Files<T>>::insert(cid, file_info);
            }
        }
        Ok(())
    }

    fn upsert_new_file_info(cid: &MerkleRoot, amount: &BalanceOf<T>, curr_bn: &BlockNumber, file_size: u64) {
        // Extend expired_at
        if let Some(mut file_info) = Self::files(cid) {
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
            <Files<T>>::insert(cid, file_info);
        } else {
            // New file
            let file_info = FileInfo::<T::AccountId, BalanceOf<T>> {
                file_size,
                spower: 0,
                expired_at: 0,
                calculated_at: curr_bn.clone(),
                amount: amount.clone(),
                prepaid: Zero::zero(),
                reported_replica_count: 0u32,
                replicas: vec![]
            };
            <Files<T>>::insert(cid, file_info);
            FileKeysCount::mutate(|count| *count = count.saturating_add(1));
        }
    }

    fn insert_replica(file_info: &mut FileInfo<T::AccountId, BalanceOf<T>>, new_replica: Replica<T::AccountId>) {
        file_info.replicas.push(new_replica);
        file_info.replicas.sort_by_key(|d| d.valid_at);
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

    /// Calculate file price
    /// Include the file base fee, file size price and files count price
    /// return => (file_base_fee, file_size_price + file_keys_count_fee)
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
        let file_size_price = match amount {
            Some(value) => value,
            None => Zero::zero(),
        };
        // 2. Get file base fee
        let file_base_fee = Self::file_base_fee();
        // 3. Get files count price
        let file_keys_count_fee = Self::file_keys_count_fee();

        (file_base_fee, file_size_price + file_keys_count_fee)
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
                    0 ..= 5 => (false, Perbill::from_percent(30)),
                    6 => (false,Perbill::from_percent(25)),
                    7 => (false,Perbill::from_percent(21)),
                    8 => (false,Perbill::from_percent(18)),
                    9 => (false,Perbill::from_percent(16)),
                    10 => (false,Perbill::from_percent(15)),
                    11 => (false,Perbill::from_percent(13)),
                    12 => (false,Perbill::from_percent(12)),
                    13 => (false,Perbill::from_percent(11)),
                    14 ..= 15 => (false,Perbill::from_percent(10)),
                    16 => (false,Perbill::from_percent(9)),
                    17 ..= 18 => (false,Perbill::from_percent(8)),
                    19 ..= 21 => (false,Perbill::from_percent(7)),
                    22 ..= 25 => (false,Perbill::from_percent(6)),
                    26 ..= 30 => (false,Perbill::from_percent(5)),
                    31 ..= 37 => (false,Perbill::from_percent(4)),
                    38 ..= 49 => (false,Perbill::from_percent(3)),
                    50 ..= 100 => (false,Perbill::zero()),
                    _ => (true, Perbill::from_percent(3))
                }
            },
            // No new order => decrease the price
            None => (true, Perbill::from_percent(3))
        }
    }

    // Split total value into three pot and return the amount in storage pot
    // Currently
    // 10% into reserved pot
    // 72% into staking pot
    // 18% into storage pot
    fn split_into_reserved_and_storage_and_staking_pot(who: &T::AccountId, value: BalanceOf<T>, base_fee: BalanceOf<T>, tips: BalanceOf<T>, liveness: ExistenceRequirement) -> Result<BalanceOf<T>, DispatchError> {
        // Split the original amount into three parts
        let staking_amount = T::StakingRatio::get() * value;
        let storage_amount = T::StorageRatio::get() * value;
        let reserved_amount = value - staking_amount - storage_amount;

        // Add the tips into storage amount
        let storage_amount = storage_amount + tips;

        // Check the discount for the reserved amount, reserved_amount = max(0, reserved_amount - discount_amount)
        let discount_amount = T::BenefitInterface::get_market_funds_ratio(who) * value;
        let reserved_amount = reserved_amount.saturating_sub(discount_amount);
        let reserved_amount = reserved_amount.saturating_add(base_fee);

        T::Currency::transfer(&who, &Self::reserved_pot(), reserved_amount, liveness)?;
        T::Currency::transfer(&who, &Self::staking_pot(), staking_amount, liveness)?;
        T::Currency::transfer(&who, &Self::storage_pot(), storage_amount.clone(), liveness)?;
        Ok(storage_amount)
    }

    fn get_current_block_number() -> BlockNumber {
        let current_block_number = <system::Module<T>>::block_number();
        TryInto::<u32>::try_into(current_block_number).ok().unwrap()
    }

    fn maybe_upsert_file_size(who: &T::AccountId, cid: &MerkleRoot, reported_file_size: u64) {
        if let Some(mut file_info) = Self::files(cid) {
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
                    Self::deposit_event(RawEvent::IllegalFileClosed(cid.clone()));
                }
            }
        }
    }

    fn maybe_reward_merchant(who: &T::AccountId, amount: &BalanceOf<T>) -> bool {
        if let Some(owner) = Self::bonded(who) {
            if let Some(new_reward) = Self::has_enough_collateral(&owner, amount) {
                T::BenefitInterface::update_reward(&owner, new_reward);
                return true;
            }
        }
        false
    }

    fn update_spower_info(file_info: &mut FileInfo<T::AccountId, BalanceOf<T>>, curr_bn: Option<BlockNumber>) -> u64 {
        let new_spower = Self::calculate_spower(file_info.file_size, file_info.reported_replica_count);
        let prev_spower = file_info.spower;
        let mut replicas_count = 0;
        for ref mut replica in &mut file_info.replicas {
            if replica.created_at.is_none() && prev_spower != new_spower {
                replicas_count += 1;
                T::SworkerInterface::update_spower(&replica.anchor, prev_spower, new_spower);
            } else if let Some(curr_bn) = curr_bn {
                // Make it become valid
                let created_at = replica.created_at.unwrap();
                if created_at + Self::valid_duration() < curr_bn {
                    replicas_count += 1;
                    T::SworkerInterface::update_spower(&replica.anchor, file_info.file_size, new_spower);
                    replica.created_at = None;
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

    fn get_files_to_update() -> Vec<MerkleRoot> {
        let mut pending_files = PendingFiles::take();
        let mut files_to_update = Vec::<MerkleRoot>::new();
        let mut count = 0;
        // Loop the MAX_PENDING_FILES files
        for cid in &pending_files {
            if count >= MAX_PENDING_FILES {
                break;
            }
            files_to_update.push(cid.clone());
            count += 1;
        }
        // Remove the MAX_PENDING_FILES files from pending files
        if files_to_update.len() < pending_files.len() {
            for cid in files_to_update.clone() {
                pending_files.remove(&cid);
            }
            PendingFiles::put(pending_files);
        }
        files_to_update
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
        SetMarketSwitchSuccess(bool),
        /// Set the file base fee success.
        SetBaseFeeSuccess(Balance),
        /// Someone be the new Reviewer
        SetFreeOrderAdminSuccess(AccountId),
        /// Create a new free account
        NewFreeAccount(AccountId),
        /// Remove a free account
        FreeAccountRemoved(AccountId),
        /// Set the free counts limit
        SetFreeCountsLimitSuccess(u32),
    }
);

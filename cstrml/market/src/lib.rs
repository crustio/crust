// Copyright (C) 2019-2021 Crust Network Technologies Ltd.
// This file is part of Crust.

#![cfg_attr(not(feature = "std"), no_std)]
#![feature(option_result_contains)]

use codec::{Decode, Encode};
use frame_support::{
    decl_event, decl_module, decl_storage, decl_error,
    dispatch::DispatchResult, ensure,
    storage::migration::remove_storage_prefix,
    traits::{
        Currency, ReservableCurrency, Get,
        ExistenceRequirement::{AllowDeath, KeepAlive},
        WithdrawReasons, Imbalance
    },
    weights::Weight
};
use sp_std::{prelude::*, convert::TryInto, collections::{btree_map::BTreeMap, btree_set::BTreeSet}};
use frame_system::{self as system, ensure_signed, ensure_root};
use sp_runtime::{Perbill, ModuleId, traits::{Zero, CheckedMul, Convert, AccountIdConversion, Saturating}, DispatchError};

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
    traits::{
        UsableCurrency, MarketInterface,
        SworkerInterface
    }
};

pub(crate) const LOG_TARGET: &'static str = "market";

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
    fn register() -> Weight;
    fn add_collateral() -> Weight;
    fn cut_collateral() -> Weight;
    fn place_storage_order() -> Weight;
    fn calculate_reward() -> Weight;
    fn reward_merchant() -> Weight;
}

#[derive(Debug, PartialEq, Eq, Clone, Encode, Decode, Default)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct FileInfo<AccountId, Balance> {
    // The ordered file size, which declare by user
    pub file_size: u64,
    // The block number when the file goes invalide
    pub expired_on: BlockNumber,
    // The last block number when the file's amount is calculated
    pub calculated_at: BlockNumber,
    // The file value
    pub amount: Balance,
    // The pre paid pool
    pub prepaid: Balance,
    // The count of valid replica each report slot
    pub reported_replica_count: u32,
    // The replica list
    // TODO: restrict the length of this replica
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
    pub is_reported: bool
}

/// According to the definition, we should put this one into swork pallet.
/// However, in consideration of performance,
/// we put this in market to avoid too many keys in storage
#[derive(Debug, PartialEq, Eq, Clone, Encode, Decode, Default)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct UsedInfo {
    // The size of used value in MPoW
    pub used_size: u64,
    // The count of valid group in the previous report slot
    pub reported_group_count: u32,
    // Anchors which is counted as contributor for this file in its own group, bool means in the last check the group is calculated as reported_group_count
    pub groups: BTreeMap<SworkerAnchor, bool>
}

#[derive(Debug, PartialEq, Eq, Clone, Encode, Decode, Default)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct MerchantLedger<Balance> {
    // The current reward amount.
    pub reward: Balance,
    // The total collateral amount
    pub collateral: Balance
}

type BalanceOf<T> =
    <<T as Config>::Currency as Currency<<T as system::Config>::AccountId>>::Balance;
type PositiveImbalanceOf<T> =
    <<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::PositiveImbalance;

impl<T: Config> MarketInterface<<T as system::Config>::AccountId, BalanceOf<T>> for Module<T>
{
    /// Upsert new replica
    /// Accept id(who, anchor), reported_file_size, cid, valid_at and maybe_member
    /// Returns the real used size of this file
    /// used size is decided by market
    fn upsert_replica(who: &<T as system::Config>::AccountId,
                      cid: &MerkleRoot,
                      reported_file_size: u64,
                      anchor: &SworkerAnchor,
                      valid_at: BlockNumber,
                      maybe_members: &Option<BTreeSet<<T as system::Config>::AccountId>>
    ) -> u64 {
        // Judge if file_info.file_size == reported_file_size or not
        Self::maybe_upsert_file_size(who, cid, reported_file_size);

        // `is_counted` is a concept in swork-side, which means if this `cid`'s `used` size is counted by `(who, anchor)`
        // if the file doesn't exist(aka. is_counted == false), return false(doesn't increase used size) cause it's junk.
        // if the file exist, is_counted == true, will change it later.
        let mut used_size: u64 = 0;
        if let Some((mut file_info, mut used_info)) = <Files<T>>::get(cid) {
            let mut is_counted = true;
            // 1. Check if the file is stored by other members
            if let Some(members) = maybe_members {
                for replica in file_info.replicas.iter() {
                    if used_info.groups.contains_key(&replica.anchor) && members.contains(&replica.who) {
                        if T::SworkerInterface::check_anchor(&replica.who, &replica.anchor) {
                            // duplicated in group and set is_counted to false
                            is_counted = false;
                        }
                    }
                }
            }

            // 2. Prepare new replica info
            let new_replica = Replica {
                who: who.clone(),
                valid_at,
                anchor: anchor.clone(),
                is_reported: true
            };
            Self::insert_replica(&mut file_info, new_replica);
            file_info.reported_replica_count += 1;

            // 3. Update used_info
            if is_counted {
                used_size = Self::add_used_group(&mut used_info, anchor, file_info.file_size); // need to add the used_size after the update
            };

            // 4. The first join the replicas and file become live(expired_on > calculated_at)
            let curr_bn = Self::get_current_block_number();
            if file_info.replicas.len() == 1 {
                file_info.calculated_at = curr_bn;
                file_info.expired_on = curr_bn + T::FileDuration::get();
            }

            // 5. Update files
            <Files<T>>::insert(cid, (file_info, used_info));
        }
        return used_size
    }

    /// Node who delete the replica
    /// Accept id(who, anchor), cid and current block number
    /// Returns the real used size of this file
    fn delete_replica(who: &<T as system::Config>::AccountId, cid: &MerkleRoot, anchor: &SworkerAnchor) -> u64 {
        // 1. Delete replica from file_info
        if let Some((mut file_info, used_info)) = <Files<T>>::get(cid) {
            let mut is_to_decreased = false;
            file_info.replicas.retain(|replica| {
                if replica.who == *who && replica.is_reported {
                    // if this anchor didn't report work, we already decrease the `reported_replica_count` in `do_calculate_reward`
                    is_to_decreased = true;
                }
                replica.who != *who
            });
            if is_to_decreased {
                file_info.reported_replica_count = file_info.reported_replica_count.saturating_sub(1);
            }
            <Files<T>>::insert(cid, (file_info, used_info));
        }

        // 2. Delete anchor from file_info/file_trash and return whether it is counted
        Self::delete_used_group(cid, anchor)
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
    type Currency: ReservableCurrency<Self::AccountId> + UsableCurrency<Self::AccountId>;

    /// Converter from Currency<u64> to Balance.
    type CurrencyToBalance: Convert<BalanceOf<Self>, u64> + Convert<u64, BalanceOf<Self>>;

    /// used to check work report
    type SworkerInterface: SworkerInterface<Self::AccountId>;

    /// The overarching event type.
    type Event: From<Event<Self>> + Into<<Self as system::Config>::Event>;

    /// File duration.
    type FileDuration: Get<BlockNumber>;

    /// File base replica. Use 4 for now
    type FileReplica: Get<u32>;

    /// File Base Price.
    type FileInitPrice: Get<BalanceOf<Self>>;

    /// Storage reference ratio. files_size / total_capacity
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

    /// UsedTrashMaxSize.
    type UsedTrashMaxSize: Get<u128>;

    /// Maximum file size
    type MaximumFileSize: Get<u64>;

    /// Weight information for extrinsics in this pallet.
    type WeightInfo: WeightInfo;
}

// This module's storage items.
decl_storage! {
    trait Store for Module<T: Config> as Market {
        /// File Base Fee.
        pub FileBaseFee get(fn file_base_fee): BalanceOf<T> = Zero::zero();

        /// Merchant Ledger
        pub MerchantLedgers get(fn merchant_ledgers):
        map hasher(blake2_128_concat) T::AccountId => MerchantLedger<BalanceOf<T>>;

        /// File information iterated by order id
        pub Files get(fn files):
        map hasher(twox_64_concat) MerkleRoot => Option<(FileInfo<T::AccountId, BalanceOf<T>>, UsedInfo)>;

        /// File price. It would change according to First Party Storage, Total Storage and Storage Base Ratio.
        pub FilePrice get(fn file_price): BalanceOf<T> = T::FileInitPrice::get();

        /// First Class Storage
        pub FilesSize get(fn files_size): u128 = 0;

        /// File trash to store second class storage
        pub UsedTrashI get(fn used_trash_i):
        map hasher(twox_64_concat) MerkleRoot => Option<UsedInfo>;

        pub UsedTrashII get(fn used_trash_ii):
        map hasher(twox_64_concat) MerkleRoot => Option<UsedInfo>;

        pub UsedTrashSizeI get(fn used_trash_size_i): u128 = 0;

        pub UsedTrashSizeII get(fn used_trash_size_ii): u128 = 0;

        pub UsedTrashMappingI get(fn used_trash_mapping_i):
        map hasher(blake2_128_concat) SworkerAnchor => u64 = 0;

        pub UsedTrashMappingII get(fn used_trash_mapping_ii):
        map hasher(blake2_128_concat) SworkerAnchor => u64 = 0;

        /// Market switch to enable place storage order
        pub MarketSwitch get(fn market_switch): bool = false;
    }
    add_extra_genesis {
		build(|_config| {
			// Create Market accounts
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
        /// Don't have enough currency
        InsufficientCurrency,
        /// Don't have enough collateral
        InsufficientCollateral,
        /// Can not bond with value less than minimum balance.
        InsufficientValue,
        /// Not Register before
        NotRegister,
        /// Register before
        AlreadyRegistered,
        /// Reward length is too long
        RewardLengthTooLong,
        /// File size is not correct
        FileSizeNotCorrect,
        /// You are not permitted to this function
        /// You are not in the whitelist
        NotPermitted,
        /// File does not exist
        FileNotExist,
        /// File is not in the reward period
        NotInRewardPeriod,
        /// Reward is not enough
        NotEnoughReward,
        /// File is too large
        FileTooLarge,
        /// Place order is not available right now
        PlaceOrderNotAvailable
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

        /// File duration.
        const FileDuration: BlockNumber = T::FileDuration::get();

        /// File base replica.
        const FileReplica: u32 = T::FileReplica::get();

        /// File Init Price.
        const FileInitPrice: BalanceOf<T> = T::FileInitPrice::get();

        /// Storage reference ratio. files_size / total_capacity
        const StorageReferenceRatio: (u128, u128) = T::StorageReferenceRatio::get();

        /// Storage increase ratio.
        const StorageIncreaseRatio: Perbill = T::StorageIncreaseRatio::get();

        /// Storage decrease ratio.
        const StorageDecreaseRatio: Perbill = T::StorageDecreaseRatio::get();

        /// Storage / Staking ratio.
        const StakingRatio: Perbill = T::StakingRatio::get();

        /// Renew reward ratio.
        const RenewRewardRatio: Perbill = T::RenewRewardRatio::get();

        /// Tax / Storage plus Staking ratio.
        const StorageRatio: Perbill = T::StorageRatio::get();

        /// Max size of used trash.
        const UsedTrashMaxSize: u128 = T::UsedTrashMaxSize::get();

        /// Max size of a file
        const MaximumFileSize: u64 = T::MaximumFileSize::get();

        /// Register to be a merchant, you should provide your storage layer's address info
        /// this will require you to collateral first, complexity depends on `Collaterals`(P).
        ///
        /// # <weight>
        /// Complexity: O(logP)
        /// - Read: Collateral
        /// - Write: Collateral
        /// # </weight>
        #[weight = T::WeightInfo::register()]
        pub fn register(
            origin,
            #[compact] collateral: BalanceOf<T>
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;

            // 1. Reject a collateral which is considered to be _dust_.
            ensure!(collateral >= T::Currency::minimum_balance(), Error::<T>::InsufficientValue);

            // 2. Ensure merchant has enough currency.
            ensure!(collateral <= T::Currency::usable_balance(&who), Error::<T>::InsufficientCurrency);

            // 3. Check if merchant has not register before.
            ensure!(!<MerchantLedgers<T>>::contains_key(&who), Error::<T>::AlreadyRegistered);

            // 4. Transfer from origin to collateral account.
            T::Currency::transfer(&who, &Self::collateral_pot(), collateral.clone(), AllowDeath)?;

            // 5. Prepare new ledger
            let ledger = MerchantLedger {
                reward: Zero::zero(),
                collateral: collateral.clone()
            };

            // 6. Upsert collateral.
            <MerchantLedgers<T>>::insert(&who, ledger);

            // 7. Emit success
            Self::deposit_event(RawEvent::RegisterSuccess(who.clone(), collateral));

            Ok(())
        }

        /// Collateral extra amount of currency to accept market order.
        ///
        /// # <weight>
        /// Complexity: O(logP)
        /// - Read: Collateral
        /// - Write: Collateral
        /// # </weight>
        #[weight = T::WeightInfo::add_collateral()]
        pub fn add_collateral(origin, #[compact] value: BalanceOf<T>) -> DispatchResult {
            let who = ensure_signed(origin)?;

            // 1. Reject a collateral which is considered to be _dust_.
            ensure!(value >= T::Currency::minimum_balance(), Error::<T>::InsufficientValue);

            // 2. Check if merchant has collateral or not
            ensure!(<MerchantLedgers<T>>::contains_key(&who), Error::<T>::NotRegister);

            // 3. Ensure merchant has enough currency.
            ensure!(value <= T::Currency::usable_balance(&who), Error::<T>::InsufficientCurrency);

            // 4. Upgrade collateral.
            <MerchantLedgers<T>>::mutate(&who, |ledger| { ledger.collateral += value.clone();});

            // 5. Transfer from origin to collateral account.
            T::Currency::transfer(&who, &Self::collateral_pot(), value.clone(), AllowDeath)?;

            // 6. Emit success
            Self::deposit_event(RawEvent::AddCollateralSuccess(who.clone(), value));

            Ok(())
        }

        /// Decrease collateral amount of currency for market order.
        ///
        /// # <weight>
        /// Complexity: O(logP)
        /// - Read: Collateral
        /// - Write: Collateral
        /// # </weight>
        #[weight = T::WeightInfo::cut_collateral()]
        pub fn cut_collateral(origin, #[compact] value: BalanceOf<T>) -> DispatchResult {
            let who = ensure_signed(origin)?;

            // 1. Reject a collateral which is considered to be _dust_.
            ensure!(value >= T::Currency::minimum_balance(), Error::<T>::InsufficientValue);

            // 2. Check if merchant has collateral or not
            ensure!(<MerchantLedgers<T>>::contains_key(&who), Error::<T>::NotRegister);

            let mut ledger = Self::merchant_ledgers(&who);

            // 3. Ensure value is smaller than unused.
            ensure!(value <= ledger.collateral - ledger.reward, Error::<T>::InsufficientCollateral);

            // 4. Upgrade collateral.
            ledger.collateral -= value.clone();
            <MerchantLedgers<T>>::insert(&who, ledger.clone());

            // 5. Transfer from origin to collateral account.
            T::Currency::transfer(&Self::collateral_pot(), &who, value.clone(), KeepAlive)?;

            // 6. Emit success
            Self::deposit_event(RawEvent::CutCollateralSuccess(who, value));

            Ok(())
        }

        /// Place a storage order
        #[weight = T::WeightInfo::place_storage_order()]
        pub fn place_storage_order(
            origin,
            cid: MerkleRoot,
            reported_file_size: u64,
            #[compact] tips: BalanceOf<T>
        ) -> DispatchResult {
            // 1. Service should be available right now.
            ensure!(Self::market_switch(), Error::<T>::PlaceOrderNotAvailable);
            let who = ensure_signed(origin)?;

            // 2. Calculate amount.
            let mut charged_file_size = reported_file_size;
            if let Some((file_info, _)) = Self::files(&cid) {
                if file_info.file_size <= reported_file_size {
                    // Charge user with real file size
                    charged_file_size = file_info.file_size;
                } else {
                    Err(Error::<T>::FileSizeNotCorrect)?
                }
            }
            // 3. charged_file_size should be smaller than 128G
            ensure!(charged_file_size < T::MaximumFileSize::get(), Error::<T>::FileTooLarge);
            let amount = Self::file_base_fee() + Self::get_file_amount(charged_file_size) + tips;

            // 4. Check client can afford the sorder
            ensure!(T::Currency::usable_balance(&who) >= amount, Error::<T>::InsufficientCurrency);

            // 5. Split into reserved, storage and staking account
            let amount = Self::split_into_reserved_and_storage_and_staking_pot(&who, amount.clone())?;

            let curr_bn = Self::get_current_block_number();

            // 6. do calculate reward. Try to close file and decrease first party storage
            Self::do_calculate_reward(&cid, curr_bn);

            // 7. three scenarios: new file, extend time(refresh time)
            Self::upsert_new_file_info(&cid, &amount, &curr_bn, charged_file_size);

            // 8. Update storage price.
            #[cfg(not(test))]
            Self::update_file_price();

            Self::deposit_event(RawEvent::FileSuccess(who, cid));

            Ok(())
        }

        /// Place a storage order
        #[weight = T::WeightInfo::place_storage_order()]
        pub fn add_prepaid(
            origin,
            cid: MerkleRoot,
            #[compact] amount: BalanceOf<T>
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;

            ensure!(T::Currency::usable_balance(&who) >= amount, Error::<T>::InsufficientCurrency);

            if let Some((mut file_info, used_info)) = Self::files(&cid) {
                T::Currency::transfer(&who, &Self::storage_pot(), amount.clone(), AllowDeath)?;
                file_info.prepaid += amount;
                <Files<T>>::insert(&cid, (file_info, used_info));
            } else {
                Err(Error::<T>::FileNotExist)?
            }

            Self::deposit_event(RawEvent::AddPrepaidSuccess(who, cid, amount));

            Ok(())
        }

        /// Calculate the payout
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

            let file_info = Self::files(&cid).unwrap().0;
            let curr_bn = Self::get_current_block_number();

            // 2. File should be live right now and calculate reward should be after expired_on
            ensure!(file_info.expired_on != 0 && curr_bn >= file_info.expired_on, Error::<T>::NotInRewardPeriod);

            // 3. Maybe reward liquidator when he try to close outdated file
            Self::maybe_reward_liquidator(&cid, curr_bn, &liquidator)?;

            // 4. Refresh the status of the file and calculate the reward for merchants
            Self::do_calculate_reward(&cid, curr_bn);

            // 5. Try to renew file if prepaid is not zero
            Self::try_to_renew_file(&cid, curr_bn, &liquidator)?;

            // 6. Try to close file
            Self::try_to_close_file(&cid, curr_bn)?;

            Self::deposit_event(RawEvent::CalculateSuccess(cid));
            Ok(())
        }

        /// Reward the merchant
        #[weight = T::WeightInfo::reward_merchant()]
        pub fn reward_merchant(
            origin
        ) -> DispatchResult {
            let merchant = ensure_signed(origin)?;

            // 1. Ensure merchant registered before
            ensure!(<MerchantLedgers<T>>::contains_key(&merchant), Error::<T>::NotRegister);

            // 2. Fetch ledger information
            let mut merchant_ledger = Self::merchant_ledgers(&merchant);

            // 3. Ensure reward is larger than some value
            ensure!(merchant_ledger.reward > Zero::zero(), Error::<T>::NotEnoughReward);

            // 4. Transfer the money
            T::Currency::transfer(&Self::storage_pot(), &merchant, merchant_ledger.reward, KeepAlive)?;

            // 5. Set the reward to zero and push it back
            merchant_ledger.reward = Zero::zero();
            <MerchantLedgers<T>>::insert(&merchant, merchant_ledger);

            Self::deposit_event(RawEvent::RewardMerchantSuccess(merchant));
            Ok(())
        }

        #[weight = 1000]
        pub fn show_pots(
            origin
        ) -> DispatchResult {
            let _ = ensure_root(origin)?;
            let collateral_pot = Self::collateral_pot();
            let storage_pot = Self::storage_pot();
            let staking_pot = Self::staking_pot();
            let reserved_pot = Self::reserved_pot();
            Self::deposit_event(RawEvent::PotList(collateral_pot, storage_pot, staking_pot, reserved_pot));
            Ok(())
        }

        /// Set the global switch
        #[weight = T::WeightInfo::reward_merchant()]
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

    /// Calculate reward from file's replica
    /// This function will calculate the file's reward, update replicas
    /// and (maybe) insert file's status(files_size and delete file)
    /// input:
    ///     cid: MerkleRoot
    ///     curr_bn: BlockNumber
    pub fn do_calculate_reward(cid: &MerkleRoot, curr_bn: BlockNumber)
    {
        // 1. File must exist
        if Self::files(cid).is_none() { return; }
        
        // 2. File must already started
        let (mut file_info, mut used_info) = Self::files(cid).unwrap_or_default();
        
        // 3. File already expired
        if file_info.expired_on <= file_info.calculated_at { return; }

        // 4. Update used_info and files_size
        let prev_reported_group_count = used_info.reported_group_count;
        used_info.reported_group_count = Self::count_reported_groups(&mut used_info.groups, curr_bn); // use curr_bn here since we want to check the latest status
        Self::update_groups_used_info(file_info.file_size, &mut used_info);
        Self::update_files_size(file_info.file_size, prev_reported_group_count, used_info.reported_group_count);

        let calculated_block = curr_bn.min(file_info.expired_on);
        let target_reward_count = file_info.replicas.len().min(T::FileReplica::get() as usize) as u32;
        
        // 5. Calculate payouts, check replicas and update the file_info
        if target_reward_count > 0 {
            // 5.1 Get 1 payout amount and sub 1 to make sure that we won't get overflow
            let one_payout_amount = (Perbill::from_rational_approximation(calculated_block - file_info.calculated_at,
                                                                          (file_info.expired_on - file_info.calculated_at) * target_reward_count) * file_info.amount).saturating_sub(1u32.into());
            let mut rewarded_amount = Zero::zero();
            let mut rewarded_count = 0u32;
            let mut new_replicas: Vec<Replica<T::AccountId>> = Vec::with_capacity(file_info.replicas.len());
            let mut invalid_replicas: Vec<Replica<T::AccountId>> = Vec::with_capacity(file_info.replicas.len());
            
            // 5.2. Loop replicas
            for replica in file_info.replicas.iter() {
                // a. didn't report in prev slot, push back to the end of replica
                if !T::SworkerInterface::is_wr_reported(&replica.anchor, curr_bn) {
                    let mut invalid_replica = replica.clone();
                    // update the valid_at to the curr_bn
                    invalid_replica.valid_at = curr_bn;
                    invalid_replica.is_reported = false;
                    // move it to the end of replica
                    invalid_replicas.push(invalid_replica);
                    // TODO: kick this anchor out of used info
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
                    if Self::maybe_reward_merchant(&replica.who, &one_payout_amount, used_info.groups.contains_key(&replica.anchor)) {
                        rewarded_amount += one_payout_amount.clone();
                        rewarded_count +=1;
                    }
                }
            }

            // 5.3 Update file info
            file_info.amount = file_info.amount.saturating_sub(rewarded_amount);
            file_info.reported_replica_count = new_replicas.len() as u32;
            new_replicas.append(&mut invalid_replicas);
            file_info.replicas = new_replicas;
        }

        // 6. File status might become ready to be closed if calculated_block == expired_on
        file_info.calculated_at = calculated_block;
        // 7. Update files
        <Files<T>>::insert(cid, (file_info, used_info));
    }

    /// Update the first class storage's size
    fn update_files_size(file_size: u64, prev_count: u32, curr_count: u32) {
        FilesSize::mutate(|size| {
            *size = size.saturating_sub((file_size * (prev_count as u64)) as u128).saturating_add((file_size * (curr_count as u64)) as u128);
        });
    }

    /// Close file, maybe move into trash
    fn try_to_close_file(cid: &MerkleRoot, curr_bn: BlockNumber) -> DispatchResult {
        if let Some((file_info, used_info)) = <Files<T>>::get(cid) {
            // If it's already expired.
            if file_info.expired_on <= curr_bn && file_info.expired_on == file_info.calculated_at {
                Self::update_files_size(file_info.file_size, used_info.reported_group_count, 0);
                let total_amount = file_info.amount.saturating_add(file_info.prepaid);
                T::Currency::transfer(&Self::storage_pot(), &Self::reserved_pot(), total_amount, KeepAlive)?;
                Self::move_into_trash(cid, used_info, file_info.file_size);
            };
        }
        Ok(())
    }

    /// Trashbag operations
    fn move_into_trash(cid: &MerkleRoot, mut used_info: UsedInfo, file_size: u64) {
        // Update used info
        used_info.reported_group_count = 1;
        Self::update_groups_used_info(file_size, &mut used_info);

        if Self::used_trash_size_i() < T::UsedTrashMaxSize::get() {
            UsedTrashI::insert(cid, used_info.clone());
            UsedTrashSizeI::mutate(|value| {*value += 1;});
            // archive used for each merchant
            for anchor in used_info.groups.keys() {
                UsedTrashMappingI::mutate(&anchor, |value| {
                    *value += used_info.used_size;
                })
            }
            // trash I is full => dump trash II
            if Self::used_trash_size_i() >= T::UsedTrashMaxSize::get() {
                Self::dump_used_trash_ii();
            }
        } else {
            UsedTrashII::insert(cid, used_info.clone());
            UsedTrashSizeII::mutate(|value| {*value += 1;});
            // archive used for each merchant
            for anchor in used_info.groups.keys() {
                UsedTrashMappingII::mutate(&anchor, |value| {
                    *value += used_info.used_size;
                })
            }
            // trash II is full => dump trash I
            if Self::used_trash_size_ii() >= T::UsedTrashMaxSize::get() {
                Self::dump_used_trash_i();
            }
        }
        <Files<T>>::remove(&cid);
    }

    fn dump_used_trash_i() {
        for (anchor, used) in UsedTrashMappingI::iter() {
            T::SworkerInterface::update_used(&anchor, used, 0);
        }
        remove_storage_prefix(UsedTrashMappingI::module_prefix(), UsedTrashMappingI::storage_prefix(), &[]);
        remove_storage_prefix(UsedTrashI::module_prefix(), UsedTrashI::storage_prefix(), &[]);
        UsedTrashSizeI::mutate(|value| {*value = 0;});
    }

    fn dump_used_trash_ii() {
        for (anchor, used) in UsedTrashMappingII::iter() {
            T::SworkerInterface::update_used(&anchor, used, 0);
        }
        remove_storage_prefix(UsedTrashMappingII::module_prefix(), UsedTrashMappingII::storage_prefix(), &[]);
        remove_storage_prefix(UsedTrashII::module_prefix(), UsedTrashII::storage_prefix(), &[]);
        UsedTrashSizeII::mutate(|value| {*value = 0;});
    }

    fn maybe_delete_file_from_used_trash_i(cid: &MerkleRoot) {
        // 1. Delete trashI's anchor
        UsedTrashI::mutate_exists(cid, |maybe_used| {
            match *maybe_used {
                Some(ref mut used_info) => {
                    for anchor in used_info.groups.keys() {
                        UsedTrashMappingI::mutate(anchor, |value| {
                            *value -= used_info.used_size;
                        });
                        T::SworkerInterface::update_used(anchor, used_info.used_size, 0);
                    }
                    UsedTrashSizeI::mutate(|value| {*value -= 1;});
                },
                None => {}
            }
            *maybe_used = None;
        });
    }

    fn maybe_delete_file_from_used_trash_ii(cid: &MerkleRoot) {
        // 1. Delete trashII's anchor
        UsedTrashII::mutate_exists(cid, |maybe_used| {
            match *maybe_used {
                Some(ref mut used_info) => {
                    for anchor in used_info.groups.keys() {
                        UsedTrashMappingII::mutate(anchor, |value| {
                            *value -= used_info.used_size;
                        });
                        T::SworkerInterface::update_used(anchor, used_info.used_size, 0);
                    }
                    UsedTrashSizeII::mutate(|value| {*value -= 1;});
                },
                None => {}
            }
            *maybe_used = None;
        });
    }

    fn maybe_delete_anchor_from_used_trash_i(cid: &MerkleRoot, anchor: &SworkerAnchor) -> u64 {
        let mut used_size = 0;
        UsedTrashI::mutate(cid, |maybe_used| match *maybe_used {
            Some(ref mut used_info) => {
                if used_info.groups.remove(anchor).is_some() {
                    used_size = used_info.used_size;
                    UsedTrashMappingI::mutate(anchor, |value| {
                        *value -= used_info.used_size;
                    });
                }
            },
            None => {}
        });
        used_size
    }

    fn maybe_delete_anchor_from_used_trash_ii(cid: &MerkleRoot, anchor: &SworkerAnchor) -> u64 {
        let mut used_size = 0;
        UsedTrashII::mutate(cid, |maybe_used| match *maybe_used {
            Some(ref mut used_info) => {
                if used_info.groups.remove(anchor).is_some() {
                    used_size = used_info.used_size;
                    UsedTrashMappingII::mutate(anchor, |value| {
                        *value -= used_info.used_size;
                    });
                }
            },
            None => {}
        });
        used_size
    }

    fn maybe_reward_liquidator(cid: &MerkleRoot, curr_bn: BlockNumber, liquidator: &T::AccountId) -> DispatchResult {
        if let Some((mut file_info, used_info)) = Self::files(cid) {
            // 1. expired_on <= curr_bn <= expired_on + T::FileDuration::get() => no reward for liquidator
            // 2. expired_on + T::FileDuration::get() < curr_bn <= expired_on + T::FileDuration::get() * 2 => linearly reward liquidator
            // 3. curr_bn > expired_on + T::FileDuration::get() * 2 => all amount would be rewarded to the liquidator
            let reward_liquidator_amount = Perbill::from_rational_approximation(curr_bn.saturating_sub(file_info.expired_on).saturating_sub(T::FileDuration::get()), T::FileDuration::get()) * file_info.amount;
            if !reward_liquidator_amount.is_zero() {
                file_info.amount = file_info.amount.saturating_sub(reward_liquidator_amount);
                T::Currency::transfer(&Self::storage_pot(), liquidator, reward_liquidator_amount, KeepAlive)?;
                <Files<T>>::insert(cid, (file_info, used_info));
            }
        }
        Ok(())
    }

    fn upsert_new_file_info(cid: &MerkleRoot, amount: &BalanceOf<T>, curr_bn: &BlockNumber, file_size: u64) {
        // Extend expired_on
        if let Some((mut file_info, used_info)) = Self::files(cid) {
            // expired_on < calculated_at => file is not live yet. This situation only happen for new file.
            // expired_on == calculated_at => file is ready to be closed(wait to be put into trash or refreshed).
            // expired_on > calculated_at => file is ongoing.
            if file_info.expired_on > file_info.calculated_at { //if it's already live.
                file_info.expired_on = curr_bn + T::FileDuration::get();
            } else if file_info.expired_on == file_info.calculated_at {
                if file_info.replicas.len() == 0 {
                    // turn this file into pending status since replicas.len() is zero
                    // we keep the original amount
                    file_info.expired_on = 0;
                    file_info.calculated_at = *curr_bn;
                } else {
                    file_info.expired_on = file_info.expired_on + T::FileDuration::get();
                }
            }
            file_info.amount += amount.clone();
            <Files<T>>::insert(cid, (file_info, used_info));
        } else {
            Self::check_file_in_trash(cid);
            // New file
            let file_info = FileInfo::<T::AccountId, BalanceOf<T>> {
                file_size,
                expired_on: 0,
                calculated_at: curr_bn.clone(),
                amount: amount.clone(),
                prepaid: Zero::zero(),
                reported_replica_count: 0u32,
                replicas: vec![]
            };
            let used_info = UsedInfo {
                used_size: 0,
                reported_group_count: 0,
                groups: <BTreeMap<SworkerAnchor, bool>>::new()
            };
            <Files<T>>::insert(cid, (file_info, used_info));
        }
    }

    fn try_to_renew_file(cid: &MerkleRoot, curr_bn: BlockNumber, liquidator: &T::AccountId) -> DispatchResult {
        if let Some((mut file_info, used_info)) = <Files<T>>::get(cid) {
            // 1. Calculate total amount
            let file_amount = Self::file_base_fee() + Self::get_file_amount(file_info.file_size);
            let renew_reward = T::RenewRewardRatio::get() * file_amount.clone();
            let total_amount = file_amount.clone() + renew_reward.clone();
            // 2. Check prepaid pool can afford the price
            if file_info.prepaid >= total_amount {
                file_info.prepaid = file_info.prepaid.saturating_sub(total_amount.clone());
                // 3. Reward liquidator.
                T::Currency::transfer(&Self::storage_pot(), liquidator, renew_reward, KeepAlive)?;
                // 4. Split into reserved, storage and staking account
                let file_amount = Self::split_into_reserved_and_storage_and_staking_pot(&Self::storage_pot(), file_amount.clone())?;
                file_info.amount += file_amount;
                if file_info.replicas.len() == 0 {
                    // turn this file into pending status since replicas.len() is zero
                    // we keep the original amount and expected_replica_count
                    file_info.expired_on = 0;
                    file_info.calculated_at = curr_bn;
                } else {
                    file_info.expired_on = file_info.expired_on + T::FileDuration::get();
                }
                <Files<T>>::insert(cid, (file_info, used_info));

                #[cfg(not(test))]
                Self::update_file_price();

                Self::deposit_event(RawEvent::RenewFileSuccess(liquidator.clone(), cid.clone()));
            }
        }
        Ok(())
    }

    fn check_file_in_trash(cid: &MerkleRoot) {
        // I. Delete trashI's anchor
        Self::maybe_delete_file_from_used_trash_i(cid);
        // 2. Delete trashII's anchor
        Self::maybe_delete_file_from_used_trash_ii(cid);
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

    fn has_enough_collateral(who: &T::AccountId, value: &BalanceOf<T>) -> bool {
        let ledger = Self::merchant_ledgers(who);
        (ledger.reward + *value).saturating_mul(10u32.into()) <= ledger.collateral
    }

    pub fn update_file_price() {
        let total_capacity = T::SworkerInterface::get_total_capacity();
        let (numerator, denominator) = T::StorageReferenceRatio::get();
        let files_size = Self::files_size();
        let mut file_price = Self::file_price();
        if files_size != 0 {
            // Too much supply => decrease the price
            if files_size.saturating_mul(denominator) < total_capacity.saturating_mul(numerator) {
                let gap = T::StorageDecreaseRatio::get() * file_price;
                file_price = file_price.saturating_sub(gap);
            } else {
                let gap = (T::StorageIncreaseRatio::get() * file_price).max(<T::CurrencyToBalance as Convert<u64, BalanceOf<T>>>::convert(1));
                file_price = file_price.saturating_add(gap);
            }
        } else {
            let gap = T::StorageDecreaseRatio::get() * file_price;
            file_price = file_price.saturating_sub(gap);
        }
        <FilePrice<T>>::put(file_price);
    }

    // Calculate file's amount
    fn get_file_amount(file_size: u64) -> BalanceOf<T> {
        // Rounded file size from `bytes` to `megabytes`
        let mut rounded_file_size = file_size / 1_048_576;
        if file_size % 1_048_576 != 0 {
            rounded_file_size += 1;
        }
        let price = Self::file_price();
        // Convert file size into `Currency`
        let amount = price.checked_mul(&<T::CurrencyToBalance as Convert<u64, BalanceOf<T>>>::convert(rounded_file_size));
        match amount {
            Some(value) => value,
            None => Zero::zero(),
        }
    }

    // Split total value into three pot and return the amount in storage pot
    // Currently
    // 10% into reserved pot
    // 72% into staking pot
    // 18% into storage pot
    fn split_into_reserved_and_storage_and_staking_pot(who: &T::AccountId, value: BalanceOf<T>) -> Result<BalanceOf<T>, DispatchError> {
        let staking_amount = T::StakingRatio::get() * value;
        let storage_amount = T::StorageRatio::get() * value;
        let reserved_amount = value - staking_amount - storage_amount;

        T::Currency::transfer(&who, &Self::reserved_pot(), reserved_amount, KeepAlive)?;
        T::Currency::transfer(&who, &Self::staking_pot(), staking_amount, KeepAlive)?;
        T::Currency::transfer(&who, &Self::storage_pot(), storage_amount.clone(), KeepAlive)?;
        Ok(storage_amount)
    }

    fn get_current_block_number() -> BlockNumber {
        let current_block_number = <system::Module<T>>::block_number();
        TryInto::<u32>::try_into(current_block_number).ok().unwrap()
    }

    fn add_used_group(used_info: &mut UsedInfo, anchor: &SworkerAnchor, file_size: u64) -> u64 {
        used_info.reported_group_count += 1;
        Self::update_groups_used_info(file_size, used_info);
        Self::update_files_size(file_size, 0, 1);
        used_info.groups.insert(anchor.clone(), true);
        used_info.used_size
    }

    fn delete_used_group(cid: &MerkleRoot, anchor: &SworkerAnchor) -> u64 {
        let mut used_size: u64 = 0;
        
        // 1. Delete files anchor
        <Files<T>>::mutate(cid, |maybe_f| match *maybe_f {
            Some((ref file_info, ref mut used_info)) => {
                if let Some(is_calculated_as_reported_group_count) = used_info.groups.remove(anchor) {
                    // need to delete the used_size before the update.
                    // we should always return the used_size no matter `is_calculated_as_reported_group_count` is true of false.
                    // `is_calculated_as_reported_group_count` only change the used_size factor.
                    // we should delete the used from wr no matter what's the factor right now.
                    used_size = used_info.used_size;
                    if is_calculated_as_reported_group_count {
                        used_info.reported_group_count = used_info.reported_group_count.saturating_sub(1);
                        Self::update_groups_used_info(file_info.file_size, used_info);
                        Self::update_files_size(file_info.file_size, 1, 0);
                    }
                }
            },
            None => {}
        });

        // 2. Delete trashI's anchor
        used_size = used_size.max(Self::maybe_delete_anchor_from_used_trash_i(cid, anchor));

        // 3. Delete trashII's anchor
        used_size = used_size.max(Self::maybe_delete_anchor_from_used_trash_ii(cid, anchor));

        used_size
    }

    fn maybe_upsert_file_size(who: &T::AccountId, cid: &MerkleRoot, reported_file_size: u64) {
        if let Some((mut file_info, used_info)) = Self::files(cid) {
            if file_info.replicas.len().is_zero() {
                // ordered_file_size == reported_file_size, return it
                if file_info.file_size == reported_file_size {
                    return
                // ordered_file_size > reported_file_size, correct it
                } else if file_info.file_size > reported_file_size {
                    file_info.file_size = reported_file_size;
                    <Files<T>>::insert(cid, (file_info, used_info));
                // ordered_file_size < reported_file_size, close it with notification
                } else {
                    let total_amount = file_info.amount + file_info.prepaid;
                    if !Self::maybe_reward_merchant(who, &total_amount, true) {
                        // This should not have error => discard the result
                        let _ = T::Currency::transfer(&Self::storage_pot(), &Self::reserved_pot(), total_amount, KeepAlive);
                    }
                    <Files<T>>::remove(cid);
                    Self::deposit_event(RawEvent::IllegalFileClosed(cid.clone()));
                }
            }
        }
    }

    fn maybe_reward_merchant(who: &T::AccountId, amount: &BalanceOf<T>, is_legal_payout_target: bool) -> bool {
        if !is_legal_payout_target {
            return false;
        }
        if Self::has_enough_collateral(&who, amount) {
            <MerchantLedgers<T>>::mutate(&who, |ledger| {
                ledger.reward += amount.clone();
            });
            return true;
        }
        false
    }

    fn update_groups_used_info(file_size: u64, used_info: &mut UsedInfo) {
        let new_used_size = Self::calculate_used_size(file_size, used_info.reported_group_count);
        let prev_used_size = used_info.used_size;
        if prev_used_size != new_used_size {
            for anchor in used_info.groups.keys() {
                T::SworkerInterface::update_used(anchor, prev_used_size, new_used_size);
            }
        }
        used_info.used_size = new_used_size;
    }

    fn count_reported_groups(groups: &mut BTreeMap<SworkerAnchor, bool>, curr_bn: BlockNumber) -> u32 {
        let mut count = 0;
        for (anchor, is_calculated_as_reported_group_count) in groups.iter_mut() {
            if T::SworkerInterface::is_wr_reported(anchor, curr_bn) {
                count += 1;
                *is_calculated_as_reported_group_count = true;
            } else { *is_calculated_as_reported_group_count = false; }
        }
        return count;
    }

    fn calculate_used_size(file_size: u64, reported_group_count: u32) -> u64 {
        let used_ratio: u64 = match reported_group_count {
            1..=10 => 2,
            11..=20 => 4,
            21..=30 => 6,
            31..=40 => 8,
            41..=70 => 10,
            71..=80 => 8,
            81..=90 => 6,
            91..=100 => 4,
            101..=200 => 2,
            _ => return 0,
        };

        used_ratio * file_size
    }
}

decl_event!(
    pub enum Event<T>
    where
        AccountId = <T as system::Config>::AccountId,
        Balance = BalanceOf<T>
    {
        FileSuccess(AccountId, MerkleRoot),
        RenewFileSuccess(AccountId, MerkleRoot),
        AddPrepaidSuccess(AccountId, MerkleRoot, Balance),
        RegisterSuccess(AccountId, Balance),
        AddCollateralSuccess(AccountId, Balance),
        CutCollateralSuccess(AccountId, Balance),
        PaysOrderSuccess(AccountId),
        CalculateSuccess(MerkleRoot),
        PotList(AccountId, AccountId, AccountId, AccountId),
        IllegalFileClosed(MerkleRoot),
        RewardMerchantSuccess(AccountId),
        SetMarketSwitchSuccess(bool),
        SetBaseFeeSuccess(Balance),
    }
);

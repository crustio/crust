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
    }
};
use sp_std::{prelude::*, convert::TryInto, collections::{btree_map::BTreeMap, btree_set::BTreeSet}};
use frame_system::{self as system, ensure_signed, ensure_root};
use sp_runtime::{
    Perbill, ModuleId,
    traits::{Zero, CheckedMul, Convert, AccountIdConversion, Saturating, StaticLookup}
};

#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(any(feature = "runtime-benchmarks", test))]
pub mod benchmarking;

use primitives::{
    MerkleRoot, BlockNumber, SworkerAnchor,
    traits::{
        TransferrableCurrency, MarketInterface,
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

#[derive(Debug, PartialEq, Eq, Clone, Encode, Decode, Default)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct FileInfo<AccountId, Balance> {
    // The ordered file size, which declare by user
    pub file_size: u64,
    // The block number when the file goes invalide
    pub expired_on: BlockNumber,
    // The last block number when the file's amount is claimed
    pub claimed_at: BlockNumber,
    // The file value
    pub amount: Balance,
    // The count of replica that user wants
    pub expected_replica_count: u32,
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
    // The total pledge amount
    pub pledge: Balance
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
    fn upsert_replicas(who: &<T as system::Config>::AccountId,
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
                used_info.reported_group_count += 1;
                Self::update_groups_used_info(file_info.file_size, &mut used_info);
                used_info.groups.insert(anchor.clone(), true);
                used_size = used_info.used_size; // need to add the used_size after the update
            };

            // 4. The first join the replicas and file become live(expired_on > claimed_at)
            let curr_bn = Self::get_current_block_number();
            if file_info.replicas.len() == 1 {
                file_info.claimed_at = curr_bn;
                file_info.expired_on = curr_bn + T::FileDuration::get();
            }

            // 5. Update files size
            if file_info.reported_replica_count <= file_info.expected_replica_count {
                Self::update_files_size(file_info.file_size, 0, 1);
            }

            // 6. Update files
            <Files<T>>::insert(cid, (file_info, used_info));
        }
        return used_size
    }

    /// Node who delete the replica
    /// Accept id(who, anchor), cid and current block number
    /// Returns the real used size of this file
    fn delete_replicas(who: &<T as system::Config>::AccountId, cid: &MerkleRoot, anchor: &SworkerAnchor, curr_bn: BlockNumber) -> u64 {
        if <Files<T>>::get(cid).is_some() {
            // 1. Calculate payouts. Try to close file and decrease first party storage(due to no wr)
            Self::calculate_payout(cid, curr_bn);
            Self::try_to_close_file(cid, curr_bn);

            // 2. Delete replica from file_info
            if let Some((mut file_info, used_info)) = <Files<T>>::get(cid) {
                let mut is_to_decreased = false;
                file_info.replicas.retain(|replica| {
                    if replica.who == *who && replica.is_reported {
                        // if this anchor didn't report work, we already decrease the `reported_replica_count` in `calculate_payout`
                        is_to_decreased = true;
                    }
                    replica.who != *who
                });
                if is_to_decreased {
                    file_info.reported_replica_count = file_info.reported_replica_count.saturating_sub(1);
                    if file_info.reported_replica_count < file_info.expected_replica_count {
                        Self::update_files_size(file_info.file_size, 1, 0);
                    }
                }
                <Files<T>>::insert(cid, (file_info, used_info));
            }
        }

        // 3. Delete anchor from file_info/file_trash and return whether it is counted
        Self::delete_used_anchor(cid, anchor)
    }

    // withdraw market staking pot for distributing staking reward
    fn withdraw_staking_pot() -> BalanceOf<T> {
        let staking_pot = Self::staking_pot();
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
                "🏢 Something wrong during withdrawing staking pot. This should never happen!"
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
    type Currency: ReservableCurrency<Self::AccountId> + TransferrableCurrency<Self::AccountId>;

    /// Converter from Currency<u64> to Balance.
    type CurrencyToBalance: Convert<BalanceOf<Self>, u64> + Convert<u64, BalanceOf<Self>>;

    /// used to check work report
    type SworkerInterface: SworkerInterface<Self::AccountId>;

    /// The overarching event type.
    type Event: From<Event<Self>> + Into<<Self as system::Config>::Event>;

    /// File duration.
    type FileDuration: Get<BlockNumber>;

    /// File base replica. Use 4 for now
    type InitialReplica: Get<u32>;

    /// File Base Fee. Use 0.001 CRU for now
    type FileBaseFee: Get<BalanceOf<Self>>;

    /// File Base Price.
    type FileInitPrice: Get<BalanceOf<Self>>;

    /// Max limit for the length of sorders in each payment claim.
    type ClaimLimit: Get<u32>;

    /// Storage reference ratio. files_size / total_capacity
    type StorageReferenceRatio: Get<(u128, u128)>;

    /// Storage increase ratio.
    type StorageIncreaseRatio: Get<Perbill>;

    /// Storage decrease ratio.
    type StorageDecreaseRatio: Get<Perbill>;

    /// Storage/Staking ratio.
    type StakingRatio: Get<Perbill>;

    /// Tax / Storage plus Staking ratio.
    type TaxRatio: Get<Perbill>;

    /// UsedTrashMaxSize.
    type UsedTrashMaxSize: Get<u128>;
}

// This module's storage items.
decl_storage! {
    trait Store for Module<T: Config> as Market {
        /// Allow List
        pub AllowList get(fn allow_list): BTreeSet<T::AccountId>;

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


    }
    add_extra_genesis {
		build(|_config| {
			// Create Market accounts
			<Module<T>>::init_pot(<Module<T>>::pledge_pot);
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
        /// Don't have enough pledge
        InsufficientPledge,
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
        NotPermitted
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

        /// Register to be a merchant, you should provide your storage layer's address info
        /// this will require you to pledge first, complexity depends on `Pledges`(P).
        ///
        /// # <weight>
        /// Complexity: O(logP)
        /// - Read: Pledge
        /// - Write: Pledge
        /// # </weight>
        #[weight = 1000]
        pub fn register(
            origin,
            #[compact] pledge: BalanceOf<T>
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;

            // 1. Reject a pledge which is considered to be _dust_.
            ensure!(pledge >= T::Currency::minimum_balance(), Error::<T>::InsufficientValue);

            // 2. Ensure merchant has enough currency.
            ensure!(pledge <= T::Currency::transfer_balance(&who), Error::<T>::InsufficientCurrency);

            // 3. Check if merchant has not register before.
            ensure!(!<MerchantLedgers<T>>::contains_key(&who), Error::<T>::AlreadyRegistered);

            // 4. Transfer from origin to pledge account.
            T::Currency::transfer(&who, &Self::pledge_pot(), pledge.clone(), AllowDeath).expect("Something wrong during transferring");

            // 5. Prepare new ledger
            let ledger = MerchantLedger {
                reward: Zero::zero(),
                pledge: pledge.clone()
            };

            // 6. Upsert pledge.
            <MerchantLedgers<T>>::insert(&who, ledger);

            // 7. Emit success
            Self::deposit_event(RawEvent::RegisterSuccess(who.clone(), pledge));

            Ok(())
        }

        /// Pledge extra amount of currency to accept market order.
        ///
        /// # <weight>
        /// Complexity: O(logP)
        /// - Read: Pledge
        /// - Write: Pledge
        /// # </weight>
        #[weight = 1000]
        pub fn pledge_extra(origin, #[compact] value: BalanceOf<T>) -> DispatchResult {
            let who = ensure_signed(origin)?;

            // 1. Reject a pledge which is considered to be _dust_.
            ensure!(value >= T::Currency::minimum_balance(), Error::<T>::InsufficientValue);

            // 2. Check if merchant has pledged before
            ensure!(<MerchantLedgers<T>>::contains_key(&who), Error::<T>::NotRegister);

            // 3. Ensure merchant has enough currency.
            ensure!(value <= T::Currency::transfer_balance(&who), Error::<T>::InsufficientCurrency);

            // 4. Upgrade pledge.
            <MerchantLedgers<T>>::mutate(&who, |ledger| { ledger.pledge += value.clone();});

            // 5. Transfer from origin to pledge account.
            T::Currency::transfer(&who, &Self::pledge_pot(), value.clone(), AllowDeath).expect("Something wrong during transferring");

            // 6. Emit success
            Self::deposit_event(RawEvent::PledgeExtraSuccess(who.clone(), value));

            Ok(())
        }

        /// Decrease pledge amount of currency for market order.
        ///
        /// # <weight>
        /// Complexity: O(logP)
        /// - Read: Pledge
        /// - Write: Pledge
        /// # </weight>
        #[weight = 1000]
        pub fn cut_pledge(origin, #[compact] value: BalanceOf<T>) -> DispatchResult {
            let who = ensure_signed(origin)?;

            // 1. Reject a pledge which is considered to be _dust_.
            ensure!(value >= T::Currency::minimum_balance(), Error::<T>::InsufficientValue);

            // 2. Check if merchant has pledged before
            ensure!(<MerchantLedgers<T>>::contains_key(&who), Error::<T>::NotRegister);

            let mut ledger = Self::merchant_ledgers(&who);

            // 3. Ensure value is smaller than unused.
            ensure!(value <= ledger.pledge - ledger.reward, Error::<T>::InsufficientPledge);

            // 4. Upgrade pledge.
            ledger.pledge -= value.clone();
            <MerchantLedgers<T>>::insert(&who, ledger.clone());

            // 5. Transfer from origin to pledge account.
            T::Currency::transfer(&Self::pledge_pot(), &who, value.clone(), AllowDeath).expect("Something wrong during transferring");

            // 6. Emit success
            Self::deposit_event(RawEvent::CutPledgeSuccess(who, value));

            Ok(())
        }

        /// Place a storage order
        /// TODO: Reconsider this weight
        #[weight = 1000]
        pub fn place_storage_order(
            origin,
            cid: MerkleRoot,
            reported_file_size: u64,
            #[compact] tips: BalanceOf<T>,
            extend_replica: bool
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;

            // TODO: Remove this check later
            ensure!(Self::allow_list().contains(&who), Error::<T>::NotPermitted);

            // 1. Calculate amount.
            let mut charged_file_size = reported_file_size;
            if let Some((file_info, _)) = Self::files(&cid) {
                if file_info.file_size <= reported_file_size {
                    // Charge user with real file size
                    charged_file_size = file_info.file_size;
                } else {
                    Err(Error::<T>::FileSizeNotCorrect)?
                }
            }
            let amount = T::FileBaseFee::get() + Self::get_file_amount(charged_file_size) + tips;

            // 2. This should not happen at all
            ensure!(amount >= T::Currency::minimum_balance(), Error::<T>::InsufficientValue);

            // 3. Check client can afford the sorder
            ensure!(T::Currency::transfer_balance(&who) >= amount, Error::<T>::InsufficientCurrency);

            // 4. Split into storage and staking account.
            let amount = Self::split_into_reserved_and_storage_and_staking_pot(&who, amount.clone());

            let curr_bn = Self::get_current_block_number();

            // 5. calculate payouts. Try to close file and decrease first party storage
            Self::calculate_payout(&cid, curr_bn);

            // 6. three scenarios: new file, extend time(refresh time) or extend replica
            Self::upsert_new_file_info(&cid, extend_replica, &amount, &curr_bn, charged_file_size);

            // 7. Update storage price.
            #[cfg(not(test))]
            Self::update_file_price();

            Self::deposit_event(RawEvent::FileSuccess(who, Self::files(cid).unwrap().0));

            Ok(())
        }

        /// Calculate the payout
        /// TODO: Reconsider this weight
        #[weight = 1000]
        pub fn calculate_reward(
            origin,
            cid: MerkleRoot,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;

            // TODO: Remove this check later
            ensure!(Self::allow_list().contains(&who), Error::<T>::NotPermitted);

            let curr_bn = Self::get_current_block_number();
            Self::calculate_payout(&cid, curr_bn);
            Self::try_to_close_file(&cid, curr_bn);
            Self::deposit_event(RawEvent::CalculateSuccess(cid));
            Ok(())
        }

        // TODO: add claim_reward

        /// Add it into allow list
        #[weight = 1000]
        pub fn add_member_into_allow_list(
            origin,
            target: <T::Lookup as StaticLookup>::Source
        ) -> DispatchResult {
            let _ = ensure_root(origin)?;
            let member = T::Lookup::lookup(target)?;

            <AllowList<T>>::mutate(|members| {
                members.insert(member);
            });
            Ok(())
        }
    }
}

impl<T: Config> Module<T> {
    /// The pot of a pledge account
    pub fn pledge_pot() -> T::AccountId {
        // "modl" ++ "crmarket" ++ "pled" is 16 bytes
        T::ModuleId::get().into_sub_account("pled")
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

    /// Calculate payout from file's replica
    /// This function will calculate the file's reward, update replicas
    /// and (maybe) insert file's status(files_size and delete file)
    /// input:
    ///     cid: MerkleRoot
    ///     curr_bn: BlockNumber
    fn calculate_payout(cid: &MerkleRoot, curr_bn: BlockNumber)
    {
        // 1. File must exist
        if Self::files(cid).is_none() { return; }
        
        // 2. File must already started
        let (mut file_info, mut used_info) = Self::files(cid).unwrap_or_default();
        
        // 3. File already expired
        if file_info.expired_on <= file_info.claimed_at { return; }
        
        // TODO: Restrict the frequency of calculate payout(limit the duration of 2 claiming)

        // 4. Update used_info
        used_info.reported_group_count = Self::count_reported_groups(&mut used_info.groups, curr_bn); // use curr_bn here since we want to check the latest status
        Self::update_groups_used_info(file_info.file_size, &mut used_info);

        // Get the previous first class storage count
        let prev_first_class_count = file_info.reported_replica_count.min(file_info.expected_replica_count);
        let claim_block = curr_bn.min(file_info.expired_on);
        let target_reward_count = file_info.replicas.len().min(file_info.expected_replica_count as usize) as u32;
        
        // 5. Calculate payouts, check replicas and update the file_info
        if target_reward_count > 0 {
            // 5.1 Get 1 payout amount and sub 1 to make sure that we won't get overflow
            let one_payout_amount = (Perbill::from_rational_approximation(claim_block - file_info.claimed_at,
                                                                          (file_info.expired_on - file_info.claimed_at) * target_reward_count) * file_info.amount).saturating_sub(1u32.into());
            let mut rewarded_amount = Zero::zero();
            let mut rewarded_count = 0u32;
            let mut new_replicas: Vec<Replica<T::AccountId>> = Vec::with_capacity(file_info.replicas.len());
            let mut invalid_replicas: Vec<Replica<T::AccountId>> = Vec::with_capacity(file_info.replicas.len());
            
            // 5.2. Loop replicas
            for replica in file_info.replicas.iter() {
                // a. didn't report in prev slot, push back to the end of replica
                if !T::SworkerInterface::is_wr_reported(&replica.anchor, claim_block) {
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
                    if Self::maybe_reward_merchant(&replica.who, &one_payout_amount) {
                        rewarded_amount += one_payout_amount.clone();
                        rewarded_count +=1;
                    }
                }
            }

            // 5.3 Update file info
            // file status might become ready to be closed if claim_block == expired_on
            file_info.claimed_at = claim_block;
            file_info.amount = file_info.amount.saturating_sub(rewarded_amount);
            file_info.reported_replica_count = new_replicas.len() as u32;
            new_replicas.append(&mut invalid_replicas);
            file_info.replicas = new_replicas;

            // 5.4 Update first class storage size
            Self::update_files_size(file_info.file_size, prev_first_class_count, file_info.reported_replica_count.min(file_info.expected_replica_count));
        }

        // 6 Update files
        <Files<T>>::insert(cid, (file_info, used_info));
    }

    /// Update the first class storage's size
    fn update_files_size(file_size: u64, prev_count: u32, curr_count: u32) {
        FilesSize::mutate(|size| {
            *size = size.saturating_sub((file_size * (prev_count as u64)) as u128).saturating_add((file_size * (curr_count as u64)) as u128);
        });
    }

    /// Close file, maybe move into trash
    fn try_to_close_file(cid: &MerkleRoot, curr_bn: BlockNumber) {
        if let Some((file_info, used_info)) = <Files<T>>::get(cid) {
            // If it's already expired.
            if file_info.expired_on <= curr_bn && file_info.expired_on >= file_info.claimed_at {
                Self::update_files_size(file_info.file_size, file_info.reported_replica_count.min(file_info.expected_replica_count), 0);
                if file_info.amount != Zero::zero() {
                    // This should rarely happen.
                    T::Currency::transfer(&Self::storage_pot(), &Self::reserved_pot(), file_info.amount, AllowDeath).expect("Something wrong during transferring");
                }
                Self::move_into_trash(cid, used_info, file_info.file_size);
            };
        }
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


    fn upsert_new_file_info(cid: &MerkleRoot, extend_replica: bool, amount: &BalanceOf<T>, curr_bn: &BlockNumber, file_size: u64) {
        // Extend expired_on or expected_replica_count
        if let Some((mut file_info, used_info)) = Self::files(cid) {
            let prev_first_class_count = file_info.reported_replica_count.min(file_info.expected_replica_count);
            // expired_on < claimed_at => file is not live yet. This situation only happen for new file.
            // expired_on == claimed_at => file is ready to be closed(wait to be put into trash or refreshed).
            // expired_on > claimed_at => file is ongoing.
            if file_info.expired_on > file_info.claimed_at { //if it's already live.
                file_info.expired_on = curr_bn + T::FileDuration::get();
            } else if file_info.expired_on == file_info.claimed_at {
                file_info.expired_on = curr_bn + T::FileDuration::get();
                file_info.claimed_at = *curr_bn;
            }
            file_info.amount += amount.clone();
            if extend_replica {
                // TODO: use 2 instead of 4
                file_info.expected_replica_count += T::InitialReplica::get();
                Self::update_files_size(file_info.file_size, prev_first_class_count, file_info.reported_replica_count.min(file_info.expected_replica_count));
            }
            <Files<T>>::insert(cid, (file_info, used_info));
        } else {
            // New file
            let file_info = FileInfo::<T::AccountId, BalanceOf<T>> {
                file_size,
                expired_on: 0,
                claimed_at: curr_bn.clone(),
                amount: amount.clone(),
                expected_replica_count: T::InitialReplica::get(),
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

    fn insert_replica(file_info: &mut FileInfo<T::AccountId, BalanceOf<T>>, new_replica: Replica<T::AccountId>) {
        fn binary_search_index<AID>(replicas: &Vec<Replica<AID>>, new_replica: &Replica<AID>) -> usize {
            let mut start = 0;
            if replicas.len() == 0 { return start; }
            let mut end = replicas.len().saturating_sub(1);
            while start <= end {
                let mid = start + (end - start) / 2;
                let replica = replicas.get(mid).unwrap();
                if new_replica.valid_at >= replica.valid_at {
                    start = mid + 1;
                } else {
                    end = mid - 1;
                }
            }
            return start;
        }
        file_info.replicas.insert(binary_search_index::<T::AccountId>(&file_info.replicas, &new_replica), new_replica);
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

    fn has_enough_pledge(who: &T::AccountId, value: &BalanceOf<T>) -> bool {
        let ledger = Self::merchant_ledgers(who);
        // TODO: 10x pledge value
        ledger.reward + *value <= ledger.pledge
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
    fn split_into_reserved_and_storage_and_staking_pot(who: &T::AccountId, value: BalanceOf<T>) -> BalanceOf<T> {
        let reserved_amount = T::TaxRatio::get() * value;
        let staking_and_storage_amount = value - reserved_amount;
        let staking_amount = T::StakingRatio::get() * staking_and_storage_amount;
        let storage_amount = staking_and_storage_amount - staking_amount;

        T::Currency::transfer(&who, &Self::reserved_pot(), reserved_amount, AllowDeath).expect("Something wrong during transferring");
        T::Currency::transfer(&who, &Self::staking_pot(), staking_amount, AllowDeath).expect("Something wrong during transferring");
        T::Currency::transfer(&who, &Self::storage_pot(), storage_amount.clone(), AllowDeath).expect("Something wrong during transferring");
        storage_amount
    }

    fn get_current_block_number() -> BlockNumber {
        let current_block_number = <system::Module<T>>::block_number();
        TryInto::<u32>::try_into(current_block_number).ok().unwrap()
    }

    fn delete_used_anchor(cid: &MerkleRoot, anchor: &SworkerAnchor) -> u64 {
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
                    }
                }
            },
            None => {}
        });
        

        // 2. Delete trashI's anchor
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

        // 3. Delete trashII's anchor
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

    fn maybe_upsert_file_size(who: &T::AccountId, cid: &MerkleRoot, reported_file_size: u64) {
        if let Some((mut file_info, used_info)) = Self::files(cid) {
            if file_info.replicas.len().is_zero() {
                if file_info.file_size >= reported_file_size {
                    file_info.file_size = reported_file_size;
                    <Files<T>>::insert(cid, (file_info, used_info));
                } else {
                    if !Self::maybe_reward_merchant(who, &file_info.amount){
                        T::Currency::transfer(&Self::storage_pot(), &Self::reserved_pot(), file_info.amount, AllowDeath).expect("Something wrong during transferring");
                    }
                    <Files<T>>::remove(cid);
                }
            }
        }
    }

    fn maybe_reward_merchant(who: &T::AccountId, amount: &BalanceOf<T>) -> bool {
        if Self::has_enough_pledge(&who, amount) {
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
        FileSuccess(AccountId, FileInfo<AccountId, Balance>),
        RegisterSuccess(AccountId, Balance),
        PledgeExtraSuccess(AccountId, Balance),
        CutPledgeSuccess(AccountId, Balance),
        PaysOrderSuccess(AccountId),
        CalculateSuccess(MerkleRoot),
    }
);

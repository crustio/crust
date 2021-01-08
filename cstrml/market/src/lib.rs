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
        Currency, ReservableCurrency, Get, ExistenceRequirement::AllowDeath,
    }
};
use sp_std::{prelude::*, convert::TryInto, collections::btree_set::BTreeSet};
use frame_system::{self as system, ensure_signed};
use sp_runtime::{
    Perbill, ModuleId,
    traits::{Zero, CheckedMul, Convert, AccountIdConversion, Saturating}
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
        SworkerInterface, StakingPotInterface
    }
};


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
}

/// According to the definition, we should put this one into swork pallet.
/// However, in consideration of performance,
/// we put this in market to avoid too many keys in storage
#[derive(Debug, PartialEq, Eq, Clone, Encode, Decode, Default)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct UsedInfo {
    // The size of used value in MPoW
    pub used_size: u64,
    // Anchors which is counted as contributor for this file in its own group
    pub groups: BTreeSet<SworkerAnchor>
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

impl<T: Config> MarketInterface<<T as system::Config>::AccountId> for Module<T>
{
    /// Upsert new replica
    /// Accept id(who, anchor), cid, valid_at and maybe_member
    /// Returns whether this id is counted to a group/itself's stake limit
    fn upsert_replicas(who: &<T as system::Config>::AccountId, cid: &MerkleRoot, anchor: &SworkerAnchor, valid_at: BlockNumber, maybe_members: &Option<BTreeSet<<T as system::Config>::AccountId>>) -> bool {
        // `is_counted` is a concept in swork-side, which means if this `cid`'s `used` size is counted by `(who, anchor)`
        // if the file doesn't exist(aka. is_counted == false), return false(doesn't increase used size) cause it's junk.
        // if the file exist, is_counted == true, will change it later.
        let mut is_counted = <Files<T>>::get(cid).is_some();
        if let Some((mut file_info, mut used_info)) = <Files<T>>::get(cid) {
            
            // 1. Check if the file is stored by other members
            if let Some(members) = maybe_members {
                for replica in file_info.replicas.iter() {
                    if used_info.groups.contains(&replica.anchor) && members.contains(&replica.who) {
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
            };
            file_info.reported_replica_count += 1;
            Self::insert_replica(&mut file_info, new_replica);

            // 3. Update used_info
            if is_counted {
                used_info.groups.insert(anchor.clone());
            };

            // 4. The first join the 
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
        return is_counted
    }

    /// Node who delete the replica
    /// Accept id(who, anchor), cid and current block number
    /// Return whether this id is counted to a group/itself's stake limit
    fn delete_replicas(who: &<T as system::Config>::AccountId, cid: &MerkleRoot, anchor: &SworkerAnchor, curr_bn: BlockNumber) -> bool {
        if <Files<T>>::get(cid).is_some() {
            // 1. Calculate payouts. Try to close file and decrease first party storage(due to no wr)
            let claimed_bn = Self::calculate_payout(cid, curr_bn);
            
            // 2. Delete replica from file_info
            if let Some((mut file_info, used_info)) = <Files<T>>::get(cid) {
                // if this anchor didn't report work, we already decrease the `reported_replica_count` in `calculate_payout`
                if T::SworkerInterface::is_wr_reported(&anchor, claimed_bn) {
                    // decrease it due to deletion
                    file_info.reported_replica_count -= 1;
                    if file_info.reported_replica_count < file_info.expected_replica_count {
                        Self::update_files_size(file_info.file_size, 1, 0);
                    }
                }
                file_info.replicas.retain(|replica| {
                    replica.who != *who
                });
                <Files<T>>::insert(cid, (file_info, used_info));
            }
        }

        // 3. Delete anchor from file_info/file_trash and return whether it is counted
        Self::delete_used_anchor(cid, anchor)
    }
}

impl<T: Config> StakingPotInterface<BalanceOf<T>> for Module<T> {
    fn withdraw_staking_pot() -> BalanceOf<T> {
        // TODO: withdraw the staking pot
        Zero::zero()
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
            file_size: u64,
            #[compact] tips: BalanceOf<T>,
            extend_replica: bool
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;

            // TODO: 10% tax
            // 1. Calculate amount.
            let amount = T::FileBaseFee::get() + Self::get_file_amount(file_size) + tips;

            // 2. This should not happen at all
            ensure!(amount >= T::Currency::minimum_balance(), Error::<T>::InsufficientValue);

            // 3. Check client can afford the sorder
            ensure!(T::Currency::transfer_balance(&who) >= amount, Error::<T>::InsufficientCurrency);

            // 4. Split into storage and staking account.
            let amount = Self::split_into_reserved_and_storage_and_staking_pot(&who, amount.clone());

            let curr_bn = Self::get_current_block_number();

            // 5. calculate payouts. Try to close file and decrease first party storage
            Self::calculate_payout(&cid, curr_bn);

            // 6. three scenarios: new file, extend time or extend replica
            Self::upsert_new_file_info(&cid, extend_replica, &amount, &curr_bn, file_size);

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
            let _ = ensure_signed(origin)?;
            let curr_bn = Self::get_current_block_number();
            Self::calculate_payout(&cid, curr_bn);
            Self::deposit_event(RawEvent::CalculateSuccess(cid));
            Ok(())
        }

        // TODO: add claim_reward
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
    /// return:
    ///     claimed_bn: BlockNumber
    fn calculate_payout(cid: &MerkleRoot, curr_bn: BlockNumber) -> BlockNumber
    {
        // 1. File must exist
        if Self::files(cid).is_none() { return curr_bn; }
        
        // 2. File must already started
        let (mut file_info, used_info) = Self::files(cid).unwrap_or_default();
        
        // 3. File already expired
        if file_info.expired_on <= file_info.claimed_at { return file_info.claimed_at; }
        
        // TODO: Restrict the frequency of calculate payout(limit the duration of 2 claiming)
        // Get the previous first class storage count
        let prev_first_class_count = file_info.reported_replica_count.min(file_info.expected_replica_count);
        let claim_block = curr_bn.min(file_info.expired_on);
        let target_reward_count = file_info.replicas.len().min(file_info.expected_replica_count as usize) as u32;
        
        // 4. Calculate payouts, check replicas and update the file_info
        if target_reward_count > 0 {
            // 4.1 Get 1 payout amount and sub 1 to make sure that we won't get overflow
            let one_payout_amount = (Perbill::from_rational_approximation(claim_block - file_info.claimed_at,
                                                                          (file_info.expired_on - file_info.claimed_at) * target_reward_count) * file_info.amount).saturating_sub(1u32.into());
            let mut rewarded_amount = Zero::zero();
            let mut rewarded_count = 0u32;
            let mut new_replicas: Vec<Replica<T::AccountId>> = Vec::with_capacity(file_info.replicas.len());
            let mut invalid_replicas: Vec<Replica<T::AccountId>> = Vec::with_capacity(file_info.replicas.len());
            
            // 4.2. Loop replicas
            for replica in file_info.replicas.iter() {
                // a. didn't report in prev slot, push back to the end of replica
                if !T::SworkerInterface::is_wr_reported(&replica.anchor, claim_block) {
                    let mut invalid_replica = replica.clone();
                    // update the valid_at to the curr_bn
                    invalid_replica.valid_at = curr_bn;
                    // move it to the end of replica
                    invalid_replicas.push(invalid_replica);
                    // TODO: kick this anchor out of used info
                // b. keep the replica's sequence
                } else {
                    new_replicas.push(replica.clone());
                    
                    // if payouts is full, just continue
                    if rewarded_count == target_reward_count {
                        continue;
                    }
                    
                    // if that guy is poor, just pass him ☠️ 
                    if Self::has_enough_pledge(&replica.who, &one_payout_amount) {
                        <MerchantLedgers<T>>::mutate(&replica.who, |ledger| {
                            ledger.reward += one_payout_amount.clone();
                        });
                        rewarded_amount += one_payout_amount.clone();
                        rewarded_count +=1;
                    }
                }
            }

            // 4.3 Update file info
            file_info.claimed_at = claim_block;
            file_info.amount = file_info.amount.saturating_sub(rewarded_amount);
            file_info.reported_replica_count = new_replicas.len() as u32;
            new_replicas.append(&mut invalid_replicas);
            file_info.replicas = new_replicas;

            // 4.4 Update files
            <Files<T>>::insert(cid, (file_info.clone(), used_info));

            // 4.5 Update first class storage size
            Self::update_files_size(file_info.file_size, prev_first_class_count, file_info.reported_replica_count.min(file_info.expected_replica_count));
        }
                
        // 5. Try to close file, judge if it is outdated
        Self::try_to_close_file(cid, curr_bn);

        // 6. Return the claimed block
        return claim_block;
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
            if file_info.expired_on <= curr_bn {
                Self::update_files_size(file_info.file_size, file_info.reported_replica_count.min(file_info.expected_replica_count), 0);
                if file_info.amount != Zero::zero() {
                    // This should rarely happen.
                    T::Currency::transfer(&Self::storage_pot(), &Self::reserved_pot(), file_info.amount, AllowDeath).expect("Something wrong during transferring");
                }
                Self::move_into_trash(cid, used_info);
            };
        }
    }

    /// Trashbag operations
    fn move_into_trash(cid: &MerkleRoot, used_info: UsedInfo) {
        if Self::used_trash_size_i() < T::UsedTrashMaxSize::get() {
            UsedTrashI::insert(cid, used_info.clone());
            UsedTrashSizeI::mutate(|value| {*value += 1;});
            // archive used for each merchant
            for anchor in used_info.groups.iter() {
                UsedTrashMappingI::mutate(&anchor, |value| {
                    *value += used_info.used_size;
                })
            }
            // trash I is full => dump trash II
            if Self::used_trash_size_i() == T::UsedTrashMaxSize::get() {
                Self::dump_used_trash_ii();
            }
        } else {
            UsedTrashII::insert(cid, used_info.clone());
            UsedTrashSizeII::mutate(|value| {*value += 1;});
            // archive used for each merchant
            for anchor in used_info.groups.iter() {
                UsedTrashMappingII::mutate(&anchor, |value| {
                    *value += used_info.used_size;
                })
            }
            // trash II is full => dump trash I
            if Self::used_trash_size_ii() == T::UsedTrashMaxSize::get() {
                Self::dump_used_trash_i();
            }
        }
        <Files<T>>::remove(&cid);
    }

    fn dump_used_trash_i() {
        for (anchor, used) in UsedTrashMappingI::iter() {
            T::SworkerInterface::decrease_used(&anchor, used);
        }
        remove_storage_prefix(UsedTrashMappingI::module_prefix(), UsedTrashMappingI::storage_prefix(), &[]);
        remove_storage_prefix(UsedTrashI::module_prefix(), UsedTrashI::storage_prefix(), &[]);
        UsedTrashSizeI::mutate(|value| {*value = 0;});
    }

    fn dump_used_trash_ii() {
        for (anchor, used) in UsedTrashMappingII::iter() {
            T::SworkerInterface::decrease_used(&anchor, used);
        }
        remove_storage_prefix(UsedTrashMappingII::module_prefix(), UsedTrashMappingII::storage_prefix(), &[]);
        remove_storage_prefix(UsedTrashII::module_prefix(), UsedTrashII::storage_prefix(), &[]);
        UsedTrashSizeII::mutate(|value| {*value = 0;});
    }

    fn upsert_new_file_info(cid: &MerkleRoot, extend_replica: bool, amount: &BalanceOf<T>, curr_bn: &BlockNumber, file_size: u64) {
        // Extend expired_on or expected_replica_count
        if let Some((mut file_info, used_info)) = Self::files(cid) {
            let prev_first_class_count = file_info.reported_replica_count.min(file_info.expected_replica_count);
            if file_info.expired_on > file_info.claimed_at { //if it's already live.
                file_info.expired_on = curr_bn + T::FileDuration::get();
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
                expired_on: curr_bn.clone(), // Not fixed, this will be changed, when first file is reported
                claimed_at: curr_bn.clone(),
                amount: amount.clone(),
                expected_replica_count: T::InitialReplica::get(),
                reported_replica_count: 0u32,
                replicas: vec![]
            };
            let used_info = UsedInfo {
                used_size: file_size,
                groups: <BTreeSet<SworkerAnchor>>::new()
            };
            <Files<T>>::insert(cid, (file_info, used_info));
        }
    }

    fn insert_replica(file_info: &mut FileInfo<T::AccountId, BalanceOf<T>>, new_replica: Replica<T::AccountId>) {
        let mut insert_index: usize = file_info.replicas.len();
        for (index, replica) in file_info.replicas.iter().enumerate() {
            if new_replica.valid_at < replica.valid_at {
                insert_index = index;
                break;
            }
        }
        file_info.replicas.insert(insert_index, new_replica);
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
        if files_size !=0 {
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

    fn delete_used_anchor(cid: &MerkleRoot, anchor: &SworkerAnchor) -> bool {
        let mut is_counted = false;
        
        // 1. Delete files anchor
        <Files<T>>::mutate(cid, |maybe_f| match *maybe_f {
            Some((_, ref mut used_info)) => {
                if used_info.groups.take(anchor).is_some() {
                    is_counted = true;
                }
            },
            None => {}
        });
        

        // 2. Delete trashI's anchor
        UsedTrashI::mutate(cid, |maybe_used| match *maybe_used {
            Some(ref mut used_info) => {
                if used_info.groups.take(anchor).is_some() {
                    is_counted = true;
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
                if used_info.groups.take(anchor).is_some() {
                    is_counted = true;
                    UsedTrashMappingII::mutate(anchor, |value| {
                        *value -= used_info.used_size;
                    });
                }
            },
            None => {}
        });

        is_counted
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

#![cfg_attr(not(feature = "std"), no_std)]
#![feature(option_result_contains)]

use codec::{Decode, Encode};
use frame_support::{
    decl_event, decl_module, decl_storage, decl_error,
    dispatch::DispatchResult, ensure,
    storage::migration::remove_storage_prefix,
    traits::{
        Currency, ReservableCurrency, Get, ExistenceRequirement::AllowDeath,
    },
    weights::Weight
};
use sp_std::{prelude::*, convert::TryInto, collections::btree_set::BTreeSet};
use frame_system::{self as system, ensure_signed};
use sp_runtime::{
    Perbill, ModuleId,
    traits::{Zero, CheckedMul, Convert, AccountIdConversion}
};

#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};

// Crust runtime modules
use primitives::{
    MerkleRoot, BlockNumber,
    traits::TransferrableCurrency, SworkerAnchor
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
    pub file_size: u64,
    pub expired_on: BlockNumber,
    pub claimed_at: BlockNumber,
    pub amount: Balance,
    pub expected_payouts: u32,
    pub reported_payouts: u32,
    pub payouts: Vec<PayoutInfo<AccountId>>
}

#[derive(Debug, PartialEq, Eq, Clone, Encode, Decode, Default)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct PayoutInfo<AccountId> {
    pub who: AccountId,
    pub reported_at: BlockNumber,
    pub anchor: SworkerAnchor,
    pub is_counted: bool
}

#[derive(Debug, PartialEq, Eq, Clone, Encode, Decode, Default)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct MerchantLedger<Balance> {
    pub reward: Balance,
    pub pledge: Balance
}

/// A trait for SworkerInspector
/// This wanyi is an outer inspector to judge whether one pk reported wr in the last slot or not
pub trait SworkerInspector<AccountId> {
    fn check_wr(anchor: &SworkerAnchor) -> bool;

    fn decrease_used(anchor: &SworkerAnchor, used: u64);

    fn check_anchor(who: &AccountId, anchor: &SworkerAnchor) -> bool;
}

type BalanceOf<T> =
    <<T as Trait>::Currency as Currency<<T as system::Trait>::AccountId>>::Balance;

/// Means for interacting with a specialized version of the `market` trait.
///
/// This is needed because `sWork`
/// 1. updates the `MerchantRecords` of the `market::Trait`
pub trait MarketInterface<AccountId> {
    // used for `added_files`
    // return is_added
    fn upsert_payouts(who: &AccountId, cid: &MerkleRoot, anchor: &SworkerAnchor, curr_bn: BlockNumber, is_counted: bool) -> bool;
    // used for `delete_files`
    // return is_deleted
    fn delete_payouts(who: &AccountId, cid: &MerkleRoot, anchor: &SworkerAnchor, curr_bn: BlockNumber) -> bool;
    // check group used
    fn check_duplicate_in_group(cid: &MerkleRoot, members: &BTreeSet<AccountId>) -> bool;
}

impl<AId> MarketInterface<AId> for () {
    fn upsert_payouts(_: &AId, _: &MerkleRoot, _: &SworkerAnchor, _: BlockNumber, _: bool) -> bool { false }

    fn delete_payouts(_: &AId, _: &MerkleRoot, _: &SworkerAnchor, _: BlockNumber) -> bool {
        false
    }

    fn check_duplicate_in_group(_: &MerkleRoot, _: &BTreeSet<AId>) -> bool {
        false
    }
}

impl<T: Trait> MarketInterface<<T as system::Trait>::AccountId> for Module<T>
{
    fn upsert_payouts(who: &<T as system::Trait>::AccountId, cid: &MerkleRoot, anchor: &SworkerAnchor, curr_bn: BlockNumber, is_counted: bool) -> bool {
        if let Some(mut file_info) = <Files<T>>::get(cid) {
            let new_payout = PayoutInfo {
                who: who.clone(),
                reported_at: curr_bn,
                anchor: anchor.clone(),
                is_counted
            };
            file_info.reported_payouts += 1;
            if file_info.reported_payouts <= file_info.expected_payouts {
                FirstClassStorage::mutate(|fcs| { *fcs = fcs.saturating_add(file_info.file_size as u128); });
            }
            Self::insert_payout(&mut file_info, new_payout);
            // start file life cycle
            if file_info.payouts.len() == 1 {
                file_info.claimed_at = curr_bn;
                file_info.expired_on = curr_bn + T::FileDuration::get();
            }
            <Files<T>>::insert(cid, file_info);
            return true;
        }
        false
    }

    fn delete_payouts(who: &<T as system::Trait>::AccountId, cid: &MerkleRoot, anchor: &SworkerAnchor, curr_bn: BlockNumber) -> bool {
        let mut is_counted = false;
        if let Some(mut file_info) = <Files<T>>::get(cid) {
            // calculate payouts. Try to close file and decrease first party storage(due to no wr)
            let is_closed = Self::calculate_payout(cid, curr_bn);
            if is_closed {
                if T::SworkerInspector::check_wr(&anchor) {
                    // decrease it due to deletion
                    file_info.reported_payouts -= 1;
                    if file_info.reported_payouts < file_info.expected_payouts {
                        FirstClassStorage::mutate(|fcs| { *fcs = fcs.saturating_sub(file_info.file_size as u128); });
                    }
                }
                file_info.payouts.retain(|payout| {
                    // This is a tricky solution
                    if payout.who == *who && payout.anchor == *anchor && payout.is_counted {
                        is_counted = true;
                    }
                    // Do we need check anchor here?
                    payout.who != *who || payout.anchor != *anchor
                });
                <Files<T>>::insert(cid, file_info);
            }
        }
        if let Some(file_info) = <FileTrashI<T>>::get(cid) {
            for payout in file_info.payouts.iter() {
                if payout.who == *who && payout.anchor == *anchor && payout.is_counted {
                    is_counted = true;
                    FileTrashUsedRecordsI::mutate(&anchor, |value| {
                        *value -= file_info.file_size;
                    });
                }
            }
        }
        if let Some(file_info) = <FileTrashII<T>>::get(cid) {
            for payout in file_info.payouts.iter() {
                if payout.who == *who && payout.anchor == *anchor && payout.is_counted {
                    is_counted = true;
                    FileTrashUsedRecordsII::mutate(&anchor, |value| {
                        *value -= file_info.file_size;
                    });
                }
            }
        }
        is_counted
    }

    fn check_duplicate_in_group(cid: &MerkleRoot, members: &BTreeSet<<T as system::Trait>::AccountId>) -> bool {
        if let Some(file_info) = <Files<T>>::get(cid) {
            for payout in file_info.payouts.iter() {
                if payout.is_counted && members.contains(&payout.who) {
                    if T::SworkerInspector::check_anchor(&payout.who, &payout.anchor) {
                        return true;
                    }
                }
            }
            return false;
        }
        // This result is useless
        return true;
    }
}

/// The module's configuration trait.
pub trait Trait: system::Trait {
    /// The market's module id, used for deriving its sovereign account ID.
    type ModuleId: Get<ModuleId>;

    /// The payment balance.
    type Currency: ReservableCurrency<Self::AccountId> + TransferrableCurrency<Self::AccountId>;

    /// Converter from Currency<u64> to Balance.
    type CurrencyToBalance: Convert<BalanceOf<Self>, u64> + Convert<u64, BalanceOf<Self>>;

    /// used to check work report
    type SworkerInspector: SworkerInspector<Self::AccountId>;

    /// The overarching event type.
    type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;

    /// File duration.
    type FileDuration: Get<BlockNumber>;

    /// File base replica. Use 4 for now
    type FileBaseReplica: Get<u32>;

    /// File Base Fee. Use 0.001 CRU for now
    type FileBaseFee: Get<BalanceOf<Self>>;

    /// File Base Price.
    type FileInitPrice: Get<BalanceOf<Self>>;

    /// Max limit for the length of sorders in each payment claim.
    type ClaimLimit: Get<u32>;

    /// Storage reference ratio.
    type StorageReferenceRatio: Get<f64>;

    /// Storage ratio.
    type StorageIncreaseRatio: Get<Perbill>;

    /// Storage/Staking ratio.
    type StakingRatio: Get<Perbill>;

    /// FileTrashMaxSize.
    type FileTrashMaxSize: Get<u128>;
}

// This module's storage items.
decl_storage! {
    trait Store for Module<T: Trait> as Market {
        /// Merchant Ledger
        pub MerchantLedgers get(fn merchant_ledgers):
        map hasher(blake2_128_concat) T::AccountId => MerchantLedger<BalanceOf<T>>;

        /// File information iterated by order id
        pub Files get(fn files):
        map hasher(twox_64_concat) MerkleRoot => Option<FileInfo<T::AccountId, BalanceOf<T>>>;

        /// File trash to store second class storage
        pub FileTrashI get(fn file_trash_i):
        map hasher(twox_64_concat) MerkleRoot => Option<FileInfo<T::AccountId, BalanceOf<T>>>;

        pub FileTrashII get(fn file_trash_ii):
        map hasher(twox_64_concat) MerkleRoot => Option<FileInfo<T::AccountId, BalanceOf<T>>>;

        pub FileTrashSizeI get(fn file_trash_size_i): u128 = 0;

        pub FileTrashSizeII get(fn file_trash_size_ii): u128 = 0;

        pub FileTrashUsedRecordsI get(fn file_trash_used_records_i):
        map hasher(blake2_128_concat) SworkerAnchor => u64 = 0;

        pub FileTrashUsedRecordsII get(fn file_trash_used_records_ii):
        map hasher(blake2_128_concat) SworkerAnchor => u64 = 0;

        /// File price. It would change according to First Party Storage, Total Storage and Storage Base Ratio.
        pub FilePrice get(fn file_price): BalanceOf<T> = T::FileInitPrice::get();

        /// First Class Storage
        pub FirstClassStorage get(fn first_class_storage): u128 = 0;
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
    pub enum Error for Module<T: Trait> {
        /// Don't have enough currency
        InsufficientCurrency,
        /// Don't have enough pledge
        InsufficientPledge,
        /// Can not bond with value less than minimum balance.
        InsufficientValue,
        /// Not Pledged before
        NotPledged,
        /// Pledged before
        AlreadyPledged,
        /// Reward length is too long
        RewardLengthTooLong,
    }
}

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
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
        pub fn pledge(
            origin,
            #[compact] value: BalanceOf<T>
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;

            // 1. Reject a pledge which is considered to be _dust_.
            ensure!(value >= T::Currency::minimum_balance(), Error::<T>::InsufficientValue);

            // 2. Ensure merchant has enough currency.
            ensure!(value <= T::Currency::transfer_balance(&who), Error::<T>::InsufficientCurrency);

            // 3. Check if merchant has not pledged before.
            ensure!(!<MerchantLedgers<T>>::contains_key(&who), Error::<T>::AlreadyPledged);

            // 4. Transfer from origin to pledge account.
            T::Currency::transfer(&who, &Self::pledge_pot(), value.clone(), AllowDeath).expect("Something wrong during transferring");

            // 4. Prepare new ledger
            let ledger = MerchantLedger {
                reward: Zero::zero(),
                pledge: value
            };

            // 5. Upsert pledge.
            <MerchantLedgers<T>>::insert(&who, ledger);

            // 6. Emit success
            Self::deposit_event(RawEvent::PledgeSuccess(who.clone(), Self::merchant_ledgers(&who).pledge));

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
            ensure!(<MerchantLedgers<T>>::contains_key(&who), Error::<T>::NotPledged);

            // 3. Ensure merchant has enough currency.
            ensure!(value <= T::Currency::transfer_balance(&who), Error::<T>::InsufficientCurrency);

            // 4. Upgrade pledge.
            <MerchantLedgers<T>>::mutate(&who, |ledger| { ledger.pledge += value.clone();});

            // 5. Transfer from origin to pledge account.
            T::Currency::transfer(&who, &Self::pledge_pot(), value, AllowDeath).expect("Something wrong during transferring");

            // 6. Emit success
            Self::deposit_event(RawEvent::PledgeSuccess(who.clone(), Self::merchant_ledgers(&who).pledge));

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
            ensure!(<MerchantLedgers<T>>::contains_key(&who), Error::<T>::NotPledged);

            let mut ledger = Self::merchant_ledgers(&who);

            // 3. Ensure value is smaller than unused.
            ensure!(value <= ledger.pledge - ledger.reward, Error::<T>::InsufficientPledge);

            // 4. Upgrade pledge.
            ledger.pledge -= value.clone();
            <MerchantLedgers<T>>::insert(&who, ledger.clone());

            // 5. Transfer from origin to pledge account.
            T::Currency::transfer(&Self::pledge_pot(), &who, value, AllowDeath).expect("Something wrong during transferring");

            // 6. Emit success
            Self::deposit_event(RawEvent::PledgeSuccess(who, ledger.pledge));

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

            // 1. Calculate amount.
            let amount = Self::get_file_amount(file_size, tips);

            // 2. This should not happen at all
            ensure!(amount >= T::Currency::minimum_balance(), Error::<T>::InsufficientValue);

            // 3. Check client can afford the sorder
            ensure!(T::Currency::transfer_balance(&who) >= amount, Error::<T>::InsufficientCurrency);

            // 4. Split into storage and staking account.
            let storage_amount = Self::split_into_storage_and_staking_account(&who, amount.clone());

            let curr_bn = Self::get_current_block_number();

            // 5. calculate payouts. Try to close file and decrease first party storage
            Self::calculate_payout(&cid, curr_bn);

            // 6. three scenarios: new file, extend time or extend replica
            Self::upsert_new_file_info(&cid, extend_replica, &storage_amount, &curr_bn, file_size);

            // 7. Update storage price.
            Self::update_storage_price();

            Self::deposit_event(RawEvent::FileSuccess(who, Self::files(cid).unwrap()));

            Ok(())
        }

        /// Calculate the payout
        /// TODO: Reconsider this weight
        #[weight = 1000]
        pub fn calculate_files(
            origin,
            files: Vec<MerkleRoot>,
        ) -> DispatchResult {
            let _ = ensure_signed(origin)?;
            let curr_bn = Self::get_current_block_number();
            for cid in files.iter() {
                Self::calculate_payout(&cid, curr_bn);
            }
            Self::deposit_event(RawEvent::CalculateSuccess(files));
            Ok(())
        }

        // TODO: add claim_reward
    }
}

impl<T: Trait> Module<T> {
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

    /// Important!!!! @zikunfan Please review review review
    /// input:
    ///     cid: MerkleRoot
    ///     curr_bn: BlockNumber
    /// return:
    ///     is_closed: bool
    fn calculate_payout(cid: &MerkleRoot, curr_bn: BlockNumber) -> bool
    {
        // File must be valid
        if let Some(mut file_info) = Self::files(cid) {
            let mut is_closed = false;
            // Not start yet
            if file_info.expired_on <= file_info.claimed_at {
                return is_closed;
            }
            // Store the previou first class storage count
            let prev_first_class_count = file_info.reported_payouts.min(file_info.expected_payouts);
            let claim_block = curr_bn.min(file_info.expired_on);
            let to_reward_count = file_info.payouts.len().min(file_info.expected_payouts as usize) as u32;
            if to_reward_count == 0 { return is_closed; } // This should never happen. But it's ok to check.
            let one_payout_amount = Perbill::from_rational_approximation(claim_block - file_info.claimed_at,
                                                                        (file_info.expired_on - file_info.claimed_at) * to_reward_count) * file_info.amount;
            // Prepare some vars
            let mut rewarded_amount = Zero::zero();
            let mut rewarded_count = 0u32;
            let mut new_payouts: Vec<PayoutInfo<T::AccountId>> = Vec::with_capacity(file_info.payouts.len());
            let mut invalid_payouts: Vec<PayoutInfo<T::AccountId>> = Vec::with_capacity(file_info.payouts.len());
            // Loop payouts
            for payout in file_info.payouts.iter() {
                // Didn't report in last slot
                if !T::SworkerInspector::check_wr(&payout.anchor) {
                    let mut invalid_payout = payout.clone();
                    // update the reported_at to the curr_bn
                    invalid_payout.reported_at = curr_bn;
                    // move it to the end of payout
                    invalid_payouts.push(invalid_payout);
                    continue;
                }
                // Keep the order
                new_payouts.push(payout.clone());
                if rewarded_count == to_reward_count {
                    continue;
                }
                if Self::has_enough_pledge(&payout.who, &one_payout_amount) {
                    <MerchantLedgers<T>>::mutate(&payout.who, |ledger| {
                        ledger.reward += one_payout_amount.clone();
                    });
                    rewarded_amount += one_payout_amount.clone();
                    rewarded_count +=1;
                }
            }
            // Update file's information
            file_info.claimed_at = claim_block;
            file_info.amount -= rewarded_amount;
            file_info.reported_payouts = new_payouts.len() as u32;
            new_payouts.append(&mut invalid_payouts);
            file_info.payouts = new_payouts;

            // If it's already expired.
            if file_info.expired_on <= curr_bn {
                Self::update_first_class_storage(file_info.file_size, prev_first_class_count, 0);
                Self::move_into_trash(cid, &file_info);
                is_closed = true;
            } else {
                Self::update_first_class_storage(file_info.file_size, prev_first_class_count, file_info.reported_payouts.min(file_info.expected_payouts));
                <Files<T>>::insert(cid, file_info);
            }
            return is_closed;
        }
        return false;
    }

    fn update_first_class_storage(file_size: u64, prev_count: u32, curr_count: u32) {
        FirstClassStorage::mutate(|storage| {
            *storage = storage.saturating_sub((file_size * (prev_count as u64)) as u128).saturating_add((file_size * (curr_count as u64)) as u128);
        });
    }

    fn move_into_trash(cid: &MerkleRoot, file_info: &FileInfo<T::AccountId, BalanceOf<T>>) {
        if file_info.amount != Zero::zero() {
            // This should rarely happen.
            T::Currency::transfer(&Self::storage_pot(), &Self::reserved_pot(), file_info.amount, AllowDeath).expect("Something wrong during transferring");
        }

        if Self::file_trash_size_i() < T::FileTrashMaxSize::get() {
            <FileTrashI<T>>::insert(cid, file_info.clone());
            FileTrashSizeI::mutate(|value| {*value += 1;});
            // archive used for each merchant
            for payout in file_info.payouts.iter() {
                if payout.is_counted {
                    FileTrashUsedRecordsI::mutate(&payout.anchor, |value| {
                        *value += file_info.file_size;
                    })
                }
            }
            // trash I is full => dump trash II
            // Maybe we need do this by root or scheduler? Cannot do it in time
            if Self::file_trash_size_i() == T::FileTrashMaxSize::get() {
                Self::dump_file_trash_ii();
            }
        } else {
            <FileTrashII<T>>::insert(cid, file_info.clone());
            FileTrashSizeII::mutate(|value| {*value += 1;});
            // archive used for each merchant
            for payout in file_info.payouts.iter() {
                if payout.is_counted {
                    FileTrashUsedRecordsII::mutate(&payout.anchor, |value| {
                        *value += file_info.file_size;
                    })
                }
            }
            // trash II is full => dump trash I
            if Self::file_trash_size_ii() == T::FileTrashMaxSize::get() {
                Self::dump_file_trash_i();
            }
        }
        <Files<T>>::remove(&cid);
    }

    fn dump_file_trash_i() {
        for (anchor, used) in FileTrashUsedRecordsI::iter() {
            T::SworkerInspector::decrease_used(&anchor, used);
        }
        remove_storage_prefix(FileTrashUsedRecordsI::module_prefix(), FileTrashUsedRecordsI::storage_prefix(), &[]);
        remove_storage_prefix(<FileTrashI<T>>::module_prefix(), <FileTrashI<T>>::storage_prefix(), &[]);
        FileTrashSizeI::mutate(|value| {*value = 0;});
    }

    fn dump_file_trash_ii() {
        for (anchor, used) in FileTrashUsedRecordsII::iter() {
            T::SworkerInspector::decrease_used(&anchor, used);
        }
        remove_storage_prefix(FileTrashUsedRecordsII::module_prefix(), FileTrashUsedRecordsII::storage_prefix(), &[]);
        remove_storage_prefix(<FileTrashII<T>>::module_prefix(), <FileTrashII<T>>::storage_prefix(), &[]);
        FileTrashSizeII::mutate(|value| {*value = 0;});
    }

    fn upsert_new_file_info(cid: &MerkleRoot, extend_replica: bool, storage_amount: &BalanceOf<T>, curr_bn: &BlockNumber, file_size: u64) {
        // Extend expired_on or expected_payouts
        if let Some(mut file_info) = Self::files(cid) {
            let prev_first_class_count = file_info.reported_payouts.min(file_info.expected_payouts);
            file_info.expired_on = curr_bn + T::FileDuration::get();
            file_info.amount += storage_amount.clone();
            if extend_replica {
                // TODO: use 2 instead of 4
                file_info.expected_payouts += T::FileBaseReplica::get();
                Self::update_first_class_storage(file_info.file_size, prev_first_class_count, file_info.reported_payouts.min(file_info.expected_payouts));
            }
            <Files<T>>::insert(cid, file_info);
        } else {
            // New file
            let file_info = FileInfo::<T::AccountId, BalanceOf<T>> {
                file_size,
                expired_on: curr_bn.clone(), // Not fixed, this will be changed, when first file is reported
                claimed_at: curr_bn.clone(),
                amount: storage_amount.clone(),
                expected_payouts: T::FileBaseReplica::get(),
                reported_payouts: 0u32,
                payouts: vec![]
            };
            <Files<T>>::insert(cid, file_info);
        }
    }

    fn insert_payout(file_info: &mut FileInfo<T::AccountId, BalanceOf<T>>, new_payout: PayoutInfo<T::AccountId>) {
        let mut insert_index: usize = file_info.payouts.len();
        for (index, payout) in file_info.payouts.iter().enumerate() {
            if new_payout.reported_at < payout.reported_at {
                insert_index = index;
                break;
            }
        }
        file_info.payouts.insert(insert_index, new_payout);
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
        ledger.reward + *value <= ledger.pledge
    }

    fn update_storage_price() {
        // TODO: implement here
    }

    // Calculate file's amount
    fn get_file_amount(file_size: u64, tips: BalanceOf<T>) -> BalanceOf<T> {
        // Rounded file size from `bytes` to `megabytes`
        let mut rounded_file_size = file_size / 1_048_576;
        if file_size % 1_048_576 != 0 {
            rounded_file_size += 1;
        }
        let price = Self::file_price();
        // Convert file size into `Currency`
        let amount = price.checked_mul(&<T::CurrencyToBalance as Convert<u64, BalanceOf<T>>>::convert(rounded_file_size));
        match amount {
            Some(value) => T::FileBaseFee::get() + value + tips,
            None => Zero::zero(),
        }
    }

    /// return:
    ///     storage amount: BalanceOf<T>
    fn split_into_storage_and_staking_account(who: &T::AccountId, value: BalanceOf<T>) -> BalanceOf<T> {
        let staking_amount = T::StakingRatio::get() * value;
        let storage_amount = value - staking_amount;
        T::Currency::transfer(&who, &Self::staking_pot(), staking_amount, AllowDeath).expect("Something wrong during transferring");
        T::Currency::transfer(&who, &Self::storage_pot(), storage_amount.clone(), AllowDeath).expect("Something wrong during transferring");
        storage_amount
    }

    fn get_current_block_number() -> BlockNumber {
        let current_block_number = <system::Module<T>>::block_number();
        TryInto::<u32>::try_into(current_block_number).ok().unwrap()
    }
}

decl_event!(
    pub enum Event<T>
    where
        AccountId = <T as system::Trait>::AccountId,
        Balance = BalanceOf<T>
    {
        FileSuccess(AccountId, FileInfo<AccountId, Balance>),
        PledgeSuccess(AccountId, Balance),
        PaysOrderSuccess(AccountId),
        CalculateSuccess(Vec<MerkleRoot>),
    }
);

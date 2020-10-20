#![cfg_attr(not(feature = "std"), no_std)]
#![feature(option_result_contains)]

use codec::{Decode, Encode};
use frame_support::{
    decl_event, decl_module, decl_storage, decl_error,
    dispatch::DispatchResult, ensure,
    traits::{
        Randomness, Currency, ReservableCurrency, LockIdentifier, LockableCurrency,
        WithdrawReasons, Get, ExistenceRequirement
    },
    weights::constants::WEIGHT_PER_MICROS
};
use sp_std::{prelude::*, convert::TryInto, collections::btree_map::BTreeMap};
use frame_system::{self as system, ensure_signed};
use sp_runtime::{
    Perbill,
    traits::{StaticLookup, Zero, CheckedMul, Convert, TrailingZeroInput}
};

#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};

// Crust runtime modules
use primitives::{
    AddressInfo, MerkleRoot, BlockNumber, FileAlias,
    traits::TransferrableCurrency
};

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(any(feature = "runtime-benchmarks", test))]
pub mod benchmarking;

const MARKET_ID: LockIdentifier = *b"market  ";

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

/// Counter for the number of eras that have passed.
pub type EraIndex = u32;

#[derive(Debug, PartialEq, Eq, Clone, Encode, Decode, Default)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct SorderInfo<AccountId, Balance> {
    pub file_identifier: MerkleRoot,
    pub file_size: u64,
    pub created_on: BlockNumber,
    pub merchant: AccountId,
    pub client: AccountId,
    pub amount: Balance,
    pub duration: BlockNumber
}

#[derive(Debug, PartialEq, Eq, Clone, Encode, Decode, Default)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct SorderStatus {
    pub completed_on: BlockNumber,
    pub expired_on: BlockNumber,
    pub status: OrderStatus,
    pub claimed_at: BlockNumber
}

#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub enum OrderStatus {
    Success,
    Failed,
    Pending
}

#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct SorderPunishment {
    pub success: BlockNumber,
    pub failed: BlockNumber,
    pub updated_at: BlockNumber
}

impl Default for SorderPunishment {
    fn default() -> Self {
        SorderPunishment {
            success: 0,
            failed: 0,
            updated_at: 0
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Encode, Decode, Default)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct Pledge<Balance> {
    // total balance of pledge
    pub total: Balance,
    // used balance of pledge
    pub used: Balance
}

impl Default for OrderStatus {
    fn default() -> Self {
        OrderStatus::Pending
    }
}

/// Preference of what happens regarding validation.
#[derive(Debug, PartialEq, Eq, Clone, Encode, Decode, Default)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct MerchantInfo<Hash, Balance> {
    /// Merchant's address
    pub address_info: AddressInfo,
    /// Merchant's storage order's price, unit is CRUs/byte/minute
    pub storage_price: Balance,
    /// Mapping from `file_id` to `sorder_id`s
    /// this mapping only be added when client place a sorder
    pub file_map: BTreeMap<MerkleRoot, Vec<Hash>>,
}

type BalanceOf<T> =
    <<T as Trait>::Currency as Currency<<T as system::Trait>::AccountId>>::Balance;

/// A trait for checking order's legality
/// This wanyi is an outer inspector to judge if s/r order can be accepted 😵
pub trait OrderInspector<AccountId> {
    /// Check if the merchant can take storage order
    fn check_works(merchant: &AccountId, file_size: u64) -> bool;
}

/// Means for interacting with a specialized version of the `market` trait.
///
/// This is needed because `sWork`
/// 1. updates the `Merchants` of the `market::Trait`
/// 2. use `Merchants` to judge work report
pub trait MarketInterface<AccountId, Hash, Balance> {
    /// MerchantInfo{files} will be used for swork module.
    fn merchants(account_id: &AccountId) -> Option<MerchantInfo<Hash, Balance>>;
    /// Get storage order
    fn maybe_get_sorder_status(order_id: &Hash) -> Option<SorderStatus>;
    /// (Maybe) set storage order's status
    fn maybe_set_sorder_status(order_id: &Hash, so_status: &SorderStatus, current_block: &BlockNumber);
}

impl<AId, Hash, Balance> MarketInterface<AId, Hash, Balance> for () {
    fn merchants(_: &AId) -> Option<MerchantInfo<Hash, Balance>> {
        None
    }

    fn maybe_get_sorder_status(_: &Hash) -> Option<SorderStatus> {
        None
    }

    fn maybe_set_sorder_status(_: &Hash, _: &SorderStatus, _: &BlockNumber) {

    }
}

impl<T: Trait> MarketInterface<<T as system::Trait>::AccountId,
    <T as system::Trait>::Hash, BalanceOf<T>> for Module<T>
{
    fn merchants(account_id: &<T as system::Trait>::AccountId)
                 -> Option<MerchantInfo<<T as system::Trait>::Hash, BalanceOf<T>>> {
        Self::merchants(account_id)
    }

    fn maybe_get_sorder_status(order_id: &<T as system::Trait>::Hash)
        -> Option<SorderStatus> {
        Self::sorder_statuses(order_id)
    }

    fn maybe_set_sorder_status(order_id: &<T as system::Trait>::Hash,
                        so_status: &SorderStatus,
                        current_block: &BlockNumber) {
        Self::maybe_set_sorder_status(order_id, so_status, current_block);
    }
}

/// The module's configuration trait.
pub trait Trait: system::Trait {
    /// The payment balance.
    type Currency: ReservableCurrency<Self::AccountId> + TransferrableCurrency<Self::AccountId>;

    /// Converter from Currency<u64> to Balance.
    type CurrencyToBalance: Convert<BalanceOf<Self>, u64> + Convert<u64, BalanceOf<Self>>;

    /// The overarching event type.
    type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;

    /// Something that provides randomness in the runtime.
    type Randomness: Randomness<Self::Hash>;

    /// Connector with swork module
    type OrderInspector: OrderInspector<Self::AccountId>;

    /// Minimum storage order price
    type MinimumStoragePrice: Get<BalanceOf<Self>>;

    /// Minimum storage order duration
    type MinimumSorderDuration: Get<u32>;

    /// Max limit for the length of sorders in each payment claim
    type ClaimLimit: Get<u32>;
}

// This module's storage items.
decl_storage! {
    trait Store for Module<T: Trait> as Market {
        /// A mapping from storage merchant to order id
        pub Merchants get(fn merchants):
        map hasher(twox_64_concat) T::AccountId => Option<MerchantInfo<T::Hash, BalanceOf<T>>>;

        /// A mapping from clients to order id
        pub Clients get(fn clients):
        double_map hasher(twox_64_concat) T::AccountId, hasher(twox_64_concat) FileAlias => Option<Vec<T::Hash>>;

        /// Order basic information iterated by order id
        pub SorderInfos get(fn sorder_infos):
        map hasher(twox_64_concat) T::Hash => Option<SorderInfo<T::AccountId, BalanceOf<T>>>;

        /// Order status iterated by order id
        pub SorderStatuses get(fn sorder_statuses):
        map hasher(twox_64_concat) T::Hash => Option<SorderStatus>;

        /// Order status counts used for punishment
        pub SorderPunishments get(fn sorder_punishments):
        map hasher(twox_64_concat) T::Hash => Option<SorderPunishment>;

        /// Pledge details iterated by merchant id
        pub Pledges get(fn pledges):
        map hasher(twox_64_concat) T::AccountId => Pledge<BalanceOf<T>>;
    }
}

decl_error! {
    /// Error for the market module.
    pub enum Error for Module<T: Trait> {
        /// Failed on generating order id
        GenerateOrderIdFailed,
        /// No workload
        NoWorkload,
        /// Target is not merchant
        NotMerchant,
        /// File duration is too short
        DurationTooShort,
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
        /// Place order to himself
        PlaceSelfOrder,
        /// Storage price is too low
        LowStoragePrice,
        /// Invalid file alias
        InvalidFileAlias,
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

        /// Register to be a merchant, you should provide your storage layer's address info,
        /// this will require you to pledge first, complexity depends on `Pledges`(P) and `swork.WorkReports`(W).
        ///
        /// # <weight>
        /// Complexity: O(logP)
        /// - Base: 30.26 µs
        /// - Read: Pledge
        /// - Write: WorkReports, Merchants
        /// # </weight>
        #[weight = 30 * WEIGHT_PER_MICROS + T::DbWeight::get().reads_writes(7, 3)]
        pub fn register(
            origin,
            address_info: AddressInfo,
            storage_price: BalanceOf<T>
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;

            // 1. Make sure you have works
            ensure!(T::OrderInspector::check_works(&who, 0), Error::<T>::NoWorkload);

            // 2. Check if the register already pledged before
            ensure!(<Pledges<T>>::contains_key(&who), Error::<T>::NotPledged);

            // 3. Make sure the storage price
            ensure!(storage_price >= T::MinimumStoragePrice::get(), Error::<T>::LowStoragePrice);

            // 4. Upsert merchant info
            <Merchants<T>>::mutate(&who, |maybe_minfo| {
                if let Some(minfo) = maybe_minfo {
                    // Update merchant
                    minfo.address_info = address_info;
                    minfo.storage_price = storage_price;
                } else {
                    // New merchant
                    *maybe_minfo = Some(MerchantInfo {
                        address_info,
                        storage_price,
                        file_map: BTreeMap::new()
                    })
                }
            });

            // 5. Emit success
            Self::deposit_event(RawEvent::RegisterSuccess(who));

            Ok(())
        }

        /// Register to be a merchant, you should provide your storage layer's address info
        /// this will require you to pledge first, complexity depends on `Pledges`(P).
        ///
        /// # <weight>
        /// Complexity: O(logP)
        /// - Base: 69.86 µs
        /// - Read: Pledge
        /// - Write: Pledge
        /// # </weight>
        #[weight = 70 * WEIGHT_PER_MICROS + T::DbWeight::get().reads_writes(7, 5)]
        pub fn pledge(
            origin,
            #[compact] value: BalanceOf<T>
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;

            // 1. Reject a pledge which is considered to be _dust_.
            ensure!(value >= T::Currency::minimum_balance(), Error::<T>::InsufficientValue);

            // 2. Ensure merchant has enough currency.
            ensure!(value <= T::Currency::transfer_balance(&who), Error::<T>::InsufficientCurrency);

            // 3. Check if merchant has not pledged before
            ensure!(!<Pledges<T>>::contains_key(&who), Error::<T>::AlreadyPledged);

            // 4. Prepare new pledge
            let pledge = Pledge {
                total: value,
                used: Zero::zero()
            };

            // 5 Upsert pledge
            Self::upsert_pledge(&who, &pledge);

            // 6. Emit success
            Self::deposit_event(RawEvent::PledgeSuccess(who));

            Ok(())
        }

        /// Pledge extra amount of currency to accept market order.
        ///
        /// # <weight>
        /// Complexity: O(logP)
        /// - Base: 66.6 µs
        /// - Read: Pledge
        /// - Write: Pledge
        /// # </weight>
        #[weight = 67 * WEIGHT_PER_MICROS + T::DbWeight::get().reads_writes(7, 5)]
        pub fn pledge_extra(origin, #[compact] value: BalanceOf<T>) -> DispatchResult {
            let who = ensure_signed(origin)?;

            // 1. Reject a pledge which is considered to be _dust_.
            ensure!(value >= T::Currency::minimum_balance(), Error::<T>::InsufficientValue);

            // 2. Check if merchant has pledged before
            ensure!(<Pledges<T>>::contains_key(&who), Error::<T>::NotPledged);

            // 3. Ensure merchant has enough currency.
            ensure!(value <= T::Currency::transfer_balance(&who), Error::<T>::InsufficientCurrency);

            let mut pledge = Self::pledges(&who);
            // 4. Increase total value
            pledge.total += value;

            // 5 Upsert pledge
            Self::upsert_pledge(&who, &pledge);

            // 6. Emit success
            Self::deposit_event(RawEvent::PledgeSuccess(who));

            Ok(())
        }

        /// Decrease pledge amount of currency for market order.
        ///
        /// # <weight>
        /// Complexity: O(logP)
        /// - Base: 73.5 µs
        /// - Read: Pledge
        /// - Write: Pledge
        /// # </weight>
        #[weight = 73 * WEIGHT_PER_MICROS + T::DbWeight::get().reads_writes(7, 5)]
        pub fn cut_pledge(origin, #[compact] value: BalanceOf<T>) -> DispatchResult {
            let who = ensure_signed(origin)?;

            // 1. Reject a pledge which is considered to be _dust_.
            ensure!(value >= T::Currency::minimum_balance(), Error::<T>::InsufficientValue);

            // 2. Check if merchant has pledged before
            ensure!(<Pledges<T>>::contains_key(&who), Error::<T>::NotPledged);

            // 3. Ensure value is smaller than unused.
            let mut pledge = Self::pledges(&who);
            ensure!(value <= pledge.total - pledge.used, Error::<T>::InsufficientPledge);

            // 4. Decrease total value
            pledge.total -= value;

            // 5 Upsert pledge
            if pledge.total.is_zero() {
                <Pledges<T>>::remove(&who);
                // Remove the lock.
                T::Currency::remove_lock(MARKET_ID, &who);
            } else {
                Self::upsert_pledge(&who, &pledge);
            }

            // 6. Emit success
            Self::deposit_event(RawEvent::PledgeSuccess(who));

            Ok(())
        }

        /// Place a storage order
        // TODO: Reconsider this weight
        #[weight = 1_000_000]
        pub fn place_storage_order(
            origin,
            target: <T::Lookup as StaticLookup>::Source,
            file_identifier: MerkleRoot,
            file_size: u64,
            duration: u32,
            file_alias: FileAlias
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;
            let merchant = T::Lookup::lookup(target)?;

            // 1. Cannot place storage order to himself.
            ensure!(who != merchant, Error::<T>::PlaceSelfOrder);

            // 2. Expired should be greater than created
            ensure!(duration > T::MinimumSorderDuration::get(), Error::<T>::DurationTooShort);

            // 3. Check if merchant is registered
            ensure!(<Merchants<T>>::contains_key(&merchant), Error::<T>::NotMerchant);

            // 4. Check merchant has capacity to store file
            ensure!(T::OrderInspector::check_works(&merchant, file_size), Error::<T>::NoWorkload);

            // 5. Check if merchant pledged and has enough unused pledge
            ensure!(<Pledges<T>>::contains_key(&merchant), Error::<T>::InsufficientPledge);

            let pledge = Self::pledges(&merchant);
            let minfo = Self::merchants(&merchant).unwrap();

            // 6. Get amount
            let amount = Self::get_sorder_amount(&minfo.storage_price, file_size, duration).unwrap();

            // 7. Judge if merchant's pledge is enough
            ensure!(amount <= pledge.total - pledge.used, Error::<T>::InsufficientPledge);

            // 8. Check client can afford the sorder
            ensure!(T::Currency::can_reserve(&who, amount.clone()), Error::<T>::InsufficientCurrency);

            // 9. Construct storage order
            let created_on = TryInto::<u32>::try_into(<system::Module<T>>::block_number()).ok().unwrap();
            let expired_on = created_on + duration*10;
            let sorder_info = SorderInfo::<T::AccountId, BalanceOf<T>> {
                file_identifier,
                file_size,
                created_on,
                merchant: merchant.clone(),
                client: who.clone(),
                amount,
                duration: duration * 10
            };

            let sorder_status = SorderStatus {
                completed_on: created_on, // Not fixed, this will be changed, when `status` become `Success`
                expired_on, // Not fixed, this will be changed, when `status` become `Success`
                status: OrderStatus::Pending,
                claimed_at: created_on
            };


            // 10. Pay the order and (maybe) add storage order
            if let Some(order_id) = Self::maybe_insert_sorder(&who, &merchant, amount.clone(), &sorder_info, &sorder_status, &file_alias) {
                // a. update pledge
                <Pledges<T>>::mutate(&merchant, |pledge| {
                        pledge.used += amount;
                });
                // b. Add `order_id` to client orders
                <Clients<T>>::mutate(&who, file_alias, |maybe_client_orders| {
                    if let Some(client_order) = maybe_client_orders {
                        client_order.push(order_id.clone());
                    } else {
                        *maybe_client_orders = Some(vec![order_id.clone()])
                    }
                });
                // c. emit storage order success event
                Self::deposit_event(RawEvent::StorageOrderSuccess(who, sorder_info, sorder_status));
            } else {
                // d. emit error
                Err(Error::<T>::GenerateOrderIdFailed)?
            }

            Ok(())
        }

        /// Rename the file path for a storage order
        #[weight = 1_000_000]
        pub fn set_file_alias(
            origin,
            old_file_alias: FileAlias,
            new_file_alias: FileAlias
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;

            ensure!(<Clients<T>>::contains_key(&who, &old_file_alias), Error::<T>::InvalidFileAlias);
            let order_ids = Self::clients(&who, &old_file_alias).unwrap();
            <Clients<T>>::insert(&who, &new_file_alias, order_ids);
            <Clients<T>>::remove(&who, &old_file_alias);
            Self::deposit_event(RawEvent::SetAliasSuccess(who, old_file_alias, new_file_alias));
            Ok(())
        }

        /// Do storage order payment
        /// Would loop each sorder in the list
        /// For each sorder, the process is 
        /// 1. Update the sorder punishment
        /// 2. Calculate payment ratio and slash ratio
        /// 3. If the slash ratio is not zero, do slash and prepare for closing sorder
        /// 4. Calculate total payment amount, unreserve the payment amount
        /// 5. Transfer the payment ratio * payment amount currency
        /// 6. Close the sorder or update the sorder status
        #[weight = 1_000_000]
        pub fn pay_sorders(
            origin,
            order_ids: Vec<T::Hash>
        ) {
            let who = ensure_signed(origin)?;

            // The length of order_ids cannot be too long.
            ensure!(order_ids.len() <= T::ClaimLimit::get().try_into().unwrap(), Error::<T>::RewardLengthTooLong);

            let current_block_numeric = Self::get_current_block_number();
            for order_id in order_ids.iter() {
                if let Some(mut so_status) = Self::sorder_statuses(order_id) {
                    if let Some(so_info) = Self::sorder_infos(order_id) {
                        if so_status.status == OrderStatus::Pending {
                            continue;
                        }
                        let mut payment_block = current_block_numeric.min(so_status.expired_on);
                        Self::update_sorder_punishment(&order_id, &payment_block, &so_status.status);
    
                        let (payment_ratio, slash_ratio) = Self::get_payment_and_slash_ratio(&order_id);
                        let mut slash_value = Zero::zero();
                        // If we do need slash the merchant of this sorder.
                        if !slash_ratio.is_zero() {
                            slash_value = slash_ratio * so_info.amount;
                            Self::slash_pledge(&so_info.merchant, slash_value);
                            // Set the payment block to the expired on to trigger closing the order
                            payment_block = so_status.expired_on;
                        }
                        // Do the payment
                        let payment_amount = Perbill::from_rational_approximation(payment_block - so_status.claimed_at, so_info.duration) * so_info.amount;
                        T::Currency::unreserve(&so_info.client, payment_amount);
                        if T::Currency::transfer(&so_info.client, &so_info.merchant, payment_ratio * payment_amount, ExistenceRequirement::AllowDeath).is_ok() {
                            if payment_block >= so_status.expired_on {
                                Self::close_sorder(&order_id, so_info.amount - slash_value);
                            } else {
                                so_status.claimed_at = payment_block;
                                <SorderStatuses<T>>::insert(order_id, so_status);
                            }
                        }
                    }
                }
            }
            Self::deposit_event(RawEvent::PaysOrderSuccess(who));
        }
    }
}

impl<T: Trait> Module<T> {
    // MUTABLE PUBLIC
    pub fn maybe_set_sorder_status(order_id: &T::Hash,
                                   so_status: &SorderStatus,
                                   current_block: &BlockNumber) {
        if let Some(old_sorder_status) = Self::sorder_statuses(order_id) {
            if &old_sorder_status != so_status {
                // 1. Update sorder punishment
                Self::update_sorder_punishment(order_id, current_block, &old_sorder_status.status);
                // 2. Update storage order status (`pay_sorders` depends on the newest `completed_on`)
                <SorderStatuses<T>>::insert(order_id, so_status);
            }
        }
    }

    pub fn update_sorder_punishment(order_id: &T::Hash, current_block: &BlockNumber, so_status: &OrderStatus) {
        if let Some(mut p) = Self::sorder_punishments(order_id) {
            match so_status {
                OrderStatus::Success => p.success += current_block - p.updated_at,
                OrderStatus::Failed => p.failed += current_block - p.updated_at,
                OrderStatus::Pending => {}
            };
            p.updated_at = *current_block;
            <SorderPunishments<T>>::insert(order_id, p);
        }
    }

    // Slash pledge value for a merchant
    pub fn slash_pledge(
        merchant: &T::AccountId,
        slash_value: BalanceOf<T>
    ) {
        T::Currency::slash(merchant, slash_value);
        // Update ledger
        let mut pledge = Self::pledges(merchant);
        pledge.total -= slash_value;
        pledge.used -= slash_value;
        Self::upsert_pledge(merchant, &pledge);
        //TODO: Move slash value into treasury
    }

    // MUTABLE PRIVATE
    // Create a new order
    // `sorder` is equal to storage order
    fn maybe_insert_sorder(client: &T::AccountId,
                           merchant: &T::AccountId,
                           amount: BalanceOf<T>,
                           so_info: &SorderInfo<T::AccountId, BalanceOf<T>>,
                           so_status: &SorderStatus,
                           file_alias: &FileAlias,
                          ) -> Option<T::Hash> {
        let order_id = Self::generate_sorder_id(client, merchant, file_alias);

        // This should be false, cause we don't allow duplicated `order_id`
        if <SorderInfos<T>>::contains_key(&order_id) {
            None
        } else {
            // 0. If reserve client's balance failed return error
            // TODO: return different error type
            if !T::Currency::reserve(&so_info.client, amount).is_ok() {
                log!(
                    debug,
                    "🏢 Cannot reserve currency for sorder {:?}",
                    order_id
                );
                return None
            }

            // 1. Add new storage order basic info and status
            <SorderInfos<T>>::insert(&order_id, so_info);
            <SorderStatuses<T>>::insert(&order_id, so_status);

            // 2. Add `file_identifier` -> `order_id`s to merchant's file_map
            <Merchants<T>>::mutate(merchant, |maybe_minfo| {
                // `minfo` cannot be None
                if let Some(minfo) = maybe_minfo {
                    let mut order_ids: Vec::<T::Hash> = vec![];
                    if let Some(o_ids) = minfo.file_map.get(&so_info.file_identifier) {
                        order_ids = o_ids.clone();
                    }

                    order_ids.push(order_id);
                    minfo.file_map.insert(so_info.file_identifier.clone(), order_ids.clone());
                }
            });

            // 3. Add OrderSlashRecord
            let merchant_punishment = SorderPunishment {
                success: 0,
                failed: 0,
                updated_at: so_info.created_on
            };
            <SorderPunishments<T>>::insert(&order_id, merchant_punishment);

            Some(order_id)
        }
    }

    // Remove a sorder
    fn close_sorder(
        order_id: &<T as system::Trait>::Hash,
        free_pledge: BalanceOf<T>
    ) {
        if let Some(so_info) = Self::sorder_infos(order_id) {
            // 1. Remove `file_identifier` -> `order_id`s from merchant's file_map
            <Merchants<T>>::mutate(&so_info.merchant, |maybe_minfo| {
                // `minfo` cannot be None
                if let Some(minfo) = maybe_minfo {
                    let mut sorder_ids: Vec<T::Hash> = minfo
                        .file_map
                        .get(&so_info.file_identifier)
                        .unwrap_or(&vec![])
                        .clone();
                    sorder_ids.retain(|&id| {&id != order_id});

                    if sorder_ids.is_empty() {
                        minfo.file_map.remove(&so_info.file_identifier);
                    } else {
                        minfo.file_map.insert(so_info.file_identifier.clone(), sorder_ids.clone());
                    }
                }
            });

            // 2. Update `Pledge` for merchant
            let mut pledge = Self::pledges(&so_info.merchant);
            // `checked_sub`, prevent overflow
            if free_pledge >= pledge.used {
                pledge.used = Zero::zero();
            } else {
                pledge.used -= free_pledge;
            }
            Self::upsert_pledge(&so_info.merchant, &pledge);

            // 3. Remove Merchant Punishment
            <SorderPunishments<T>>::remove(order_id);

            // 4. Remove storage order info and status
            <SorderInfos<T>>::remove(order_id);
            <SorderStatuses<T>>::remove(order_id);
        }
    }

    fn upsert_pledge(
        merchant: &T::AccountId,
        pledge: &Pledge<BalanceOf<T>>
    ) {
        // 1. Set lock
        T::Currency::set_lock(
            MARKET_ID,
            &merchant,
            pledge.total,
            WithdrawReasons::all(),
        );
        // 2. Update Pledge
        <Pledges<T>>::insert(&merchant, pledge);
    }

    // IMMUTABLE PRIVATE
    // Generate the storage order id by using the on-chain randomness
    fn generate_sorder_id(client: &T::AccountId, merchant: &T::AccountId, file_alias: &FileAlias) -> T::Hash {
        // 1. Construct random seed, 👼 bless the randomness
        // seed = [ block_hash, client_account, merchant_account ]
        let bn = <system::Module<T>>::block_number();
        let bh: T::Hash = <system::Module<T>>::block_hash(bn);
        let seed = [
            &file_alias.encode()[..],
            &client.encode()[..],
            &merchant.encode()[..],
        ].concat();

        // 2. It can cover most cases, for the "real" random
        T::Hash::decode(&mut TrailingZeroInput::new(&seed[..])).unwrap_or_default()
        // T::Randomness::random(seed.as_slice())
    }

    // Calculate storage order's amount
    fn get_sorder_amount(price: &BalanceOf<T>, file_size: u64, duration: u32) -> Option<BalanceOf<T>> {
        // Rounded file size from `bytes` to `megabytes`
        let mut rounded_file_size = file_size / 1_048_576;
        if file_size % 1_048_576 != 0 {
            rounded_file_size += 1;
        }

        // Convert file size into `Currency`
        price.checked_mul(&<T::CurrencyToBalance
        as Convert<u64, BalanceOf<T>>>::convert(rounded_file_size * (duration as u64)))
    }

    fn get_current_block_number() -> BlockNumber {
        let current_block_number = <system::Module<T>>::block_number();
        TryInto::<u32>::try_into(current_block_number).ok().unwrap()
    }

    fn get_payment_and_slash_ratio(order_id: &T::Hash) -> (Perbill, Perbill) {
        let mut payment_ratio: Perbill = Perbill::one();
        let mut slash_ratio: Perbill = Perbill::zero();
        if let Some(punishment) = Self::sorder_punishments(order_id) {
            let punishment_ratio: f64 = punishment.success as f64 / (punishment.success + punishment.failed) as f64;
            if punishment_ratio >= 0.99 {
                payment_ratio = Perbill::one();
            } else if punishment_ratio >= 0.98 {
                payment_ratio = Perbill::from_percent(95);
            } else if punishment_ratio >= 0.95 {
                payment_ratio = Perbill::from_percent(90);
            } else if punishment_ratio >= 0.90 {
                payment_ratio = Perbill::from_percent(80);
            } else if punishment_ratio >= 0.85 {
                payment_ratio = Perbill::from_percent(50);
            } else {
                payment_ratio = Perbill::zero();
                slash_ratio = Perbill::from_percent(50);
            }
        }
        (payment_ratio, slash_ratio)
    }
}

decl_event!(
    pub enum Event<T>
    where
        AccountId = <T as system::Trait>::AccountId,
        Balance = BalanceOf<T>
    {
        StorageOrderSuccess(AccountId, SorderInfo<AccountId, Balance>, SorderStatus),
        RegisterSuccess(AccountId),
        PledgeSuccess(AccountId),
        SetAliasSuccess(AccountId, FileAlias, FileAlias),
        PaysOrderSuccess(AccountId),
    }
);

// Copyright (C) 2019-2021 Crust Network Technologies Ltd.
// This file is part of Crust.

#![recursion_limit = "128"]
#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(test)]
mod mock;

#[cfg(any(feature = "runtime-benchmarks", test))]
pub mod benchmarking;

pub mod total_stake_limit_ratio;
#[cfg(test)]
mod tests;

use codec::{Decode, Encode, HasCompact};
use total_stake_limit_ratio::total_stake_limit_ratio;
use frame_support::{
    decl_module, decl_event, decl_storage, ensure, decl_error, PalletId,
    storage::IterableStorageMap,
    weights::{Weight, constants::{WEIGHT_PER_MICROS, WEIGHT_PER_NANOS}},
    traits::{
        Currency, LockIdentifier, LockableCurrency, WithdrawReasons, OnUnbalanced, Imbalance, Get,
        UnixTime, EnsureOrigin, Randomness, OneSessionHandler
    },
    dispatch::{DispatchResult, DispatchResultWithPostInfo}
};
use pallet_session::historical;
use sp_runtime::{
    Perbill, Permill, RuntimeDebug, SaturatedConversion,
    traits::{
        Convert, Zero, One, StaticLookup, Saturating, AtLeast32Bit,
        CheckedAdd, CheckedSub, AtLeast32BitUnsigned
    },
};
use sp_consensus_aura::sr25519::AuthorityId as AuraId;
use sp_staking::{
    SessionIndex,
    offence::{OnOffenceHandler, OffenceDetails, Offence, ReportOffence, OffenceError},
};

use sp_std::{convert::TryInto, prelude::*, collections::btree_map::BTreeMap};

use frame_system::{ensure_root, ensure_signed};
#[cfg(feature = "std")]
use sp_runtime::{Deserialize, Serialize};

pub mod weight;

// Crust runtime modules
use swork;
use primitives::{
    EraIndex,
    constants::{currency::*, time::*, staking::*},
    traits::{MarketInterface, BenefitInterface}
};

const MAX_UNLOCKING_CHUNKS: usize = 32;
const MAX_GUARANTEE: usize = 16;
const STAKING_ID: LockIdentifier = *b"staking ";

pub(crate) const LOG_TARGET: &'static str = "staking";

#[macro_export]
macro_rules! log {
    ($level:tt, $patter:expr $(, $values:expr)* $(,)?) => {
        frame_support::log::$level!(
            target: crate::LOG_TARGET,
            $patter $(, $values)*
        )
    };
}

pub trait WeightInfo {
    fn bond() -> Weight;
    fn bond_extra() -> Weight;
    fn unbond() -> Weight;
    fn rebond(l: u32, ) -> Weight;
    fn withdraw_unbonded() -> Weight;
    fn validate() -> Weight;
    fn guarantee() -> Weight;
    fn cut_guarantee() -> Weight;
    fn chill() -> Weight;
    fn set_payee() -> Weight;
    fn set_controller() -> Weight;
    // The following two doesn't used to generate weight info
    fn new_era(v: u32, n: u32, m: u32, ) -> Weight;
    fn select_and_update_validators(v: u32, n: u32, m: u32, ) -> Weight;
}

/// Counter for the number of "reward" points earned by a given validator.
pub type RewardPoint = u32;

/// Indicates the initial status of the staker.
#[derive(RuntimeDebug, scale_info::TypeInfo)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub enum StakerStatus<AccountId, Balance: HasCompact> {
    /// Chilling.
    Idle,
    /// Declared desire in validating or already participating in it.
    Validator,
    /// Guaranteeing for a group of other stakers.
    Guarantor(Vec<(AccountId, Balance)>),
}

/// A destination account for payment.
#[derive(PartialEq, Eq, Copy, Clone, Encode, Decode, RuntimeDebug, scale_info::TypeInfo)]
pub enum RewardDestination<AccountId> {
    /// Pay into the stash account, increasing the amount at stake accordingly.
    Staked,
    /// Pay into the stash account, not increasing the amount at stake.
    Stash,
    /// Pay into the controller account.
    Controller,
    /// Pay into a specified account.
    Account(AccountId),
}

impl<AccountId> Default for RewardDestination<AccountId> {
    fn default() -> Self {
        RewardDestination::Staked
    }
}

/// Preference of what happens regarding validation.
#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, scale_info::TypeInfo)]
pub struct ValidatorPrefs {
    /// Reward that validator takes up-front; only the rest is split between themselves and
    /// guarantors.
    #[codec(compact)]
    pub fee: Perbill,
}

impl Default for ValidatorPrefs {
    fn default() -> Self {
        ValidatorPrefs {
            fee: Perbill::one(),
        }
    }
}

/// Information regarding the active era (era in used in session).
#[derive(Encode, Decode, RuntimeDebug, scale_info::TypeInfo)]
pub struct ActiveEraInfo {
    /// Index of era.
    pub index: EraIndex,
    /// Moment of start expressed as millisecond from `$UNIX_EPOCH`.
    ///
    /// Start can be none if start hasn't been set for the era yet,
    /// Start is set on the first on_finalize of the era to guarantee usage of `Time`.
    start: Option<u64>,
}

/// A record of the nominations made by a specific account.
#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, scale_info::TypeInfo)]
pub struct Guarantee<AccountId, Balance: HasCompact> {
    /// The targets(validators), this vector's element is unique.
    pub targets: Vec<IndividualExposure<AccountId, Balance>>,
    /// The total votes of guarantee.
    #[codec(compact)]
    pub total: Balance,
    /// The era the nominations were submitted.
    pub submitted_in: EraIndex,
    /// Whether the nominations have been suppressed.
    pub suppressed: bool,
}

/// Just a Balance/BlockNumber tuple to encode when a chunk of funds will be unlocked.
#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, scale_info::TypeInfo)]
pub struct UnlockChunk<Balance: HasCompact> {
    /// Amount of funds to be unlocked.
    #[codec(compact)]
    value: Balance,
    /// Era number at which point it'll be unlocked.
    #[codec(compact)]
    era: EraIndex,
}

/// The ledger of a (bonded) stash.
#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, scale_info::TypeInfo)]
pub struct StakingLedger<AccountId, Balance: HasCompact> {
    /// The stash account whose balance is actually locked and at stake.
    pub stash: AccountId,
    /// The total amount of the stash's balance that we are currently accounting for.
    /// It's just `active` plus all the `unlocking` balances.
    #[codec(compact)]
    pub total: Balance,
    /// The total amount of the stash's balance that will be at stake in any forthcoming
    /// rounds.
    #[codec(compact)]
    pub active: Balance,
    /// Any balance that is becoming free, which may eventually be transferred out
    /// of the stash (assuming it doesn't get slashed first).
    pub unlocking: Vec<UnlockChunk<Balance>>,
    /// List of eras for which the stakers behind a validator and guarantor have claimed rewards.
    /// Only updated for validators.
    pub claimed_rewards: Vec<EraIndex>,
}

impl<AccountId, Balance: HasCompact + Copy + Saturating + AtLeast32BitUnsigned> StakingLedger<AccountId, Balance> {
    /// Remove entries from `unlocking` that are sufficiently old and reduce the
    /// total by the sum of their balances.
    fn consolidate_unlocked(self, current_era: EraIndex) -> Self {
        let mut total = self.total;
        let unlocking = self
            .unlocking
            .into_iter()
            .filter(|chunk| {
                if chunk.era > current_era {
                    true
                } else {
                    total = total.saturating_sub(chunk.value);
                    false
                }
            })
            .collect();
        Self {
            total,
            active: self.active,
            stash: self.stash,
            unlocking,
            claimed_rewards: self.claimed_rewards
        }
    }

    /// Re-bond funds that were scheduled for unlocking.
    fn rebond(mut self, value: Balance) -> Self {
        let mut unlocking_balance: Balance = Zero::zero();

        while let Some(last) = self.unlocking.last_mut() {
            if unlocking_balance + last.value <= value {
                unlocking_balance += last.value;
                self.active += last.value;
                self.unlocking.pop();
            } else {
                let diff = value - unlocking_balance;

                unlocking_balance += diff;
                self.active += diff;
                last.value -= diff;
            }

            if unlocking_balance >= value {
                break
            }
        }

        self
    }
}

/// The amount of exposure (to slashing) than an individual guarantor has.
#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Encode, Decode, RuntimeDebug, scale_info::TypeInfo)]
pub struct IndividualExposure<AccountId, Balance: HasCompact> {
    /// The stash account of the guarantor/validator in question.
    pub who: AccountId,
    /// Amount of funds exposed.
    #[codec(compact)]
    pub value: Balance,
}

/// A snapshot of the stake backing a single validator in the system.
#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Encode, Decode, Default, RuntimeDebug, scale_info::TypeInfo)]
pub struct Exposure<AccountId, Balance: HasCompact> {
    /// The total balance backing this validator.
    #[codec(compact)]
    pub total: Balance,
    /// The validator's own stash that is exposed.
    #[codec(compact)]
    pub own: Balance,
    /// The portions of guarantors stashes that are exposed.
    pub others: Vec<IndividualExposure<AccountId, Balance>>,
}

pub type BalanceOf<T> =
    <<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;
type PositiveImbalanceOf<T> =
    <<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::PositiveImbalance;
type NegativeImbalanceOf<T> =
    <<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::NegativeImbalance;

pub trait Config: frame_system::Config {
    /// The staking's module id, used for staking pot
    type PalletId: Get<PalletId>;
    /// The staking balance.
    type Currency: LockableCurrency<Self::AccountId, Moment = Self::BlockNumber>;

    /// Time used for computing era duration.
    ///
    /// It is guaranteed to start being called from the first `on_finalize`. Thus value at genesis
    /// is not used.
    type UnixTime: UnixTime;

    /// Convert a balance into a number used for election calculation.
    /// This must fit into a `u64` but is allowed to be sensibly lossy.
    /// TODO: [Substrate]substrate#1377
    /// The backward convert should be removed as the new Phragmen API returns ratio.
    /// The post-processing needs it but will be moved to off-chain. TODO: #2908
    type CurrencyToVote: Convert<u128, BalanceOf<Self>> + Convert<BalanceOf<Self>, u128>;

    /// Tokens have been minted and are unused for validator-reward.
    type RewardRemainder: OnUnbalanced<NegativeImbalanceOf<Self>>;

    /// The overarching event type.
    type Event: From<Event<Self>> + Into<<Self as frame_system::Config>::Event>;

    /// Handler for the unbalanced increment when rewarding a staker.
    type Reward: OnUnbalanced<PositiveImbalanceOf<Self>>;

    /// Something that provides randomness in the runtime.
    type Randomness: Randomness<Self::Hash, Self::BlockNumber>;

    /// Number of eras that staked funds must remain bonded for.
    type BondingDuration: Get<EraIndex>;

    /// The maximum number of guarantors rewarded for each validator.
    ///
    /// For each validator only the `$MaxGuarantorRewardedPerValidator` biggest stakers can claim
    /// their reward. This used to limit the i/o cost for the guarantor payout.
    type MaxGuarantorRewardedPerValidator: Get<u32>;

    /// Storage power ratio for crust network phase 1
    type SPowerRatio: Get<u128>;

    /// Reference to Market staking pot.
    type MarketStakingPot: MarketInterface<Self::AccountId, BalanceOf<Self>>;

    /// Market Staking Pot Duration. Count of EraIndex
    type MarketStakingPotDuration: Get<u32>;

    /// Fee reduction interface
    type BenefitInterface: BenefitInterface<Self::AccountId, BalanceOf<Self>, NegativeImbalanceOf<Self>>;

    /// Used for bonding buffer
    type UncheckedFrozenBondFund: Get<BalanceOf<Self>>;

    /// Weight information for extrinsics in this pallet.
    type WeightInfo: WeightInfo;
}

/// Mode of era-forcing.
#[derive(Copy, Clone, PartialEq, Eq, Encode, Decode, RuntimeDebug, scale_info::TypeInfo)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub enum Forcing {
    /// Not forcing anything - just let whatever happen.
    NotForcing,
    /// Force a new era, then reset to `NotForcing` as soon as it is done.
    ForceNew,
    /// Avoid a new era indefinitely.
    ForceNone,
    /// Force a new era at the end of all sessions indefinitely.
    ForceAlways,
}

impl Default for Forcing {
    fn default() -> Self {
        Forcing::NotForcing
    }
}

decl_storage! {
    trait Store for Module<T: Config> as Staking {
        /// Number of eras to keep in history.
        ///
        /// Information is kept for eras in `[current_era - history_depth; current_era]`.
        HistoryDepth get(fn history_depth) config(): u32 = 84;

        /// Start era for reward curve
        StartRewardEra get(fn start_reward_era) config(): EraIndex = 100000;

        /// Map from all locked "stash" accounts to the controller account.
        pub Bonded get(fn bonded): map hasher(twox_64_concat) T::AccountId => Option<T::AccountId>;

        /// Map from all (unlocked) "controller" accounts to the info regarding the staking.
        pub Ledger get(fn ledger):
            map hasher(blake2_128_concat) T::AccountId
            => Option<StakingLedger<T::AccountId, BalanceOf<T>>>;

        /// Where the reward payment should be made. Keyed by stash.
        pub Payee get(fn payee): map hasher(twox_64_concat) T::AccountId => RewardDestination<T::AccountId>;

        /// The map from (wannabe) validator stash key to the preferences of that validator.
        pub Validators get(fn validators):
            map hasher(twox_64_concat) T::AccountId => ValidatorPrefs;

        /// The map from guarantor stash key to the set of stash keys of all validators to guarantee.
        Guarantors get(fn guarantors):
            map hasher(twox_64_concat) T::AccountId => Option<Guarantee<T::AccountId, BalanceOf<T>>>;

        /// The stake limit, determined all the staking operations
        /// This is keyed by the stash account.
        pub StakeLimit get(fn stake_limit):
            map hasher(twox_64_concat) T::AccountId => Option<BalanceOf<T>>;

        /// Exposure of validator at era.
        ///
        /// This is keyed first by the era index to allow bulk deletion and then the stash account.
        ///
        /// Is it removed after `HISTORY_DEPTH` eras.
        /// If stakers hasn't been set or has been removed then empty exposure is returned.
        pub ErasStakers get(fn eras_stakers):
            double_map hasher(twox_64_concat) EraIndex, hasher(twox_64_concat) T::AccountId
            => Exposure<T::AccountId, BalanceOf<T>>;

        /// Clipped Exposure of validator at era.
        ///
        /// This is similar to [`ErasStakers`] but number of guarantors exposed is reduced to the
        /// `T::MaxGuarantorRewardedPerValidator` biggest stakers.
        /// (Note: the field `total` and `own` of the exposure remains unchanged).
        /// This is used to limit the i/o cost for the guarantor payout.
        ///
        /// This is keyed fist by the era index to allow bulk deletion and then the stash account.
        ///
        /// Is it removed after `HISTORY_DEPTH` eras.
        /// If stakers hasn't been set or has been removed then empty exposure is returned.
        pub ErasStakersClipped get(fn eras_stakers_clipped):
        double_map hasher(twox_64_concat) EraIndex, hasher(twox_64_concat) T::AccountId
        => Exposure<T::AccountId, BalanceOf<T>>;
            
        /// Similar to `ErasStakers`, this holds the preferences of validators.
        ///
        /// This is keyed first by the era index to allow bulk deletion and then the stash account.
        ///
        /// Is it removed after `HISTORY_DEPTH` eras.
        // If prefs hasn't been set or has been removed then 0 fee is returned.
        pub ErasValidatorPrefs get(fn eras_validator_prefs):
            double_map hasher(twox_64_concat) EraIndex, hasher(twox_64_concat) T::AccountId
            => ValidatorPrefs;
        
        /// Total staking payout at era.
        pub ErasStakingPayout get(fn eras_staking_payout):
            map hasher(twox_64_concat) EraIndex => Option<BalanceOf<T>>;

        /// Market staking payout of validator at era.
        pub ErasMarketPayout get(fn eras_market_payout):
            map hasher(twox_64_concat) EraIndex => Option<BalanceOf<T>>;

        /// Authoring payout of validator at era.
        pub ErasAuthoringPayout get(fn eras_authoring_payout):
        double_map hasher(twox_64_concat) EraIndex, hasher(twox_64_concat) T::AccountId
        => Option<BalanceOf<T>>;

        /// The amount of balance actively at stake for each validator slot, currently.
        ///
        /// This is used to derive rewards and punishments.
        pub ErasTotalStakes get(fn eras_total_stakes):
            map hasher(twox_64_concat) EraIndex => BalanceOf<T>;

        /// The ideal number of staking participants.
        pub ValidatorCount get(fn validator_count) config(): u32;

        /// The currently elected validator set keyed by stash account ID.
        pub CurrentElected get(fn current_elected): Vec<T::AccountId>;

        /// The current era index.
        pub CurrentEra get(fn current_era): Option<EraIndex>;

        /// The active era information, it holds index and start.
        ///
        /// The active era is the era currently rewarded.
        /// Validator set of this era must be equal to `SessionInterface::validators`.
        pub ActiveEra get(fn active_era): Option<ActiveEraInfo>;

        /// True if the next session change will be a new era regardless of index.
        pub ForceEra get(fn force_era) config(): Forcing;
    }
    add_extra_genesis {
        config(stakers):
            Vec<(T::AccountId, T::AccountId, BalanceOf<T>, StakerStatus<T::AccountId, BalanceOf<T>>)>;
        build(|config: &GenesisConfig<T>| {
            let mut gensis_total_stakes: BalanceOf<T> = Zero::zero();
            for &(ref stash, ref controller, balance, ref status) in &config.stakers {
                let _ = <Module<T>>::bond(
                    T::Origin::from(Some(stash.clone()).into()),
                    T::Lookup::unlookup(controller.clone()),
                    balance
                );

                gensis_total_stakes += balance;

                <Module<T>>::upsert_stake_limit(stash, balance+balance);
                let _ = match status {
                    StakerStatus::Validator => {
                        <Module<T>>::validate(
                            T::Origin::from(Some(controller.clone()).into()),
                            Default::default(),
                        )
                    },
                    StakerStatus::Guarantor(votes) => {
                        for (target, vote) in votes {
                            <Module<T>>::guarantee(
                                T::Origin::from(Some(controller.clone()).into()),
                                (T::Lookup::unlookup(target.clone()), vote.clone()),
                            ).ok();
                        }
                        Ok(())
                    }, _ => Ok(())
                };
            }
            <ErasTotalStakes<T>>::insert(0, gensis_total_stakes);
        });
    }
}

decl_event!(
    pub enum Event<T> where
        Balance = BalanceOf<T>,
        <T as frame_system::Config>::AccountId
    {
        /// All validators have been rewarded by the first balance; the second is the remainder
        /// from the maximum amount of reward.
        Reward(AccountId, Balance),
        /// Total reward at each era
        EraReward(EraIndex, Balance, Balance),
        /// Staking pot is not enough
        NotEnoughCurrency(EraIndex, Balance, Balance),
        /// An account has bonded this amount. [stash, amount]
        ///
        /// NOTE: This event is only emitted when funds are bonded via a dispatchable. Notably,
        /// it will not be emitted for staking rewards when they are added to stake.
        Bonded(AccountId, Balance),
        /// An account has unbonded this amount. [stash, amount]
        Unbonded(AccountId, Balance),
        /// An account has called `withdraw_unbonded` and removed unbonding chunks worth `Balance`
        /// from the unlocking queue. [stash, amount]
        Withdrawn(AccountId, Balance),
        /// An account has called `validate` and set guarantee fee.
        ValidateSuccess(AccountId, ValidatorPrefs),
        /// An account has called `guarantee` and vote for one validator.
        GuaranteeSuccess(AccountId, AccountId, Balance),
        /// An account has called `cut_guarantee` and cut vote for one validator.
        CutGuaranteeSuccess(AccountId, AccountId, Balance),
        /// An account has been chilled from its stash
        ChillSuccess(AccountId, AccountId),
        /// Update the identities success. The stake limit of each identity would be updated.
        UpdateStakeLimitSuccess(u32),
    }
);

decl_error! {
    /// Error for the staking module.
    pub enum Error for Module<T: Config> {
        /// Not a controller account.
        NotController,
        /// Not a stash account.
        NotStash,
        /// Stash is already bonded.
        AlreadyBonded,
        /// Controller is already paired.
        AlreadyPaired,
        /// All stakes are guaranteed, cut guarantee first
        AllGuaranteed,
        /// Target is invalid.
        InvalidTarget,
        /// Can not bond with value less than minimum balance.
        InsufficientValue,
        /// Can not schedule more unlock chunks.
        NoMoreChunks,
        /// Can not bond with more than limit
        ExceedGuaranteeLimit,
        /// Attempting to target a stash that still has funds.
        FundedTarget,
        /// Invalid era to reward.
        InvalidEraToReward,
        /// Claimed reward twice.
        AlreadyClaimed,
        /// Don't have enough balance to recharge the staking pot
        InsufficientCurrency,
        /// Can not rebond without unlocking chunks.
        NoUnlockChunk,
    }
}

decl_module! {
    pub struct Module<T: Config> for enum Call where origin: T::Origin {
        /// Number of eras that staked funds must remain bonded for.
        const BondingDuration: EraIndex = T::BondingDuration::get();
        
        /// The maximum number of guarantors rewarded for each validator.
        ///
        /// For each validator only the `$MaxGuarantorRewardedPerValidator` biggest stakers can claim
        /// their reward. This used to limit the i/o cost for the guarantor payout.
        const MaxGuarantorRewardedPerValidator: u32 = T::MaxGuarantorRewardedPerValidator::get();

        /// The staking's module id, used for deriving its sovereign account ID.
        const PalletId: PalletId = T::PalletId::get();

        /// Total era duration for once dsm staking pot.
        const MarketStakingPotDuration: u32 = T::MarketStakingPotDuration::get();

        /// Storage power ratio for crust network phase 1
        const SPowerRatio: u128 = T::SPowerRatio::get();

        const UncheckedFrozenBondFund: BalanceOf<T> = T::UncheckedFrozenBondFund::get();

        type Error = Error<T>;

        fn deposit_event() = default;

        fn on_finalize() {
            // Set the start of the first era.
            if let Some(mut active_era) = Self::active_era() {
                if active_era.start.is_none() {
                    let now_as_millis_u64 = T::UnixTime::now().as_millis().saturated_into::<u64>();
                    active_era.start = Some(now_as_millis_u64);
                    // This write only ever happens once, we don't include it in the weight in general
                    ActiveEra::put(active_era);
                }
            }
            // `on_finalize` weight is tracked in `on_initialize`
        }

        /// Take the origin account as a stash and lock up `value` of its balance. `controller` will
        /// be the account that controls it.
        ///
        /// `value` must be more than the `minimum_balance` specified by `T::Currency`.
        ///
        /// The dispatch origin for this call must be _Signed_ by the stash account.
        ///
        /// Emits `Bonded`.
        ///
        /// # <weight>
        /// - Independent of the arguments. Moderate complexity.
        /// - O(1).
        /// - Three extra DB entries.
        ///
        /// NOTE: Two of the storage writes (`Self::bonded`, `Self::payee`) are _never_ cleaned
        /// unless the `origin` falls below _existential deposit_ and gets removed as dust.
        /// ------------------
        /// DB Weight:
        /// - Read: Bonded, Ledger, [Origin Account], Current Era, Locks
        /// - Write: Bonded, Payee, [Origin Account], Ledger, Locks
        /// # </weight>
        #[weight = T::WeightInfo::bond()]
        fn bond(origin,
            controller: <T::Lookup as StaticLookup>::Source,
            #[compact] value: BalanceOf<T>
        ) {
            let stash = ensure_signed(origin)?;

            if <Bonded<T>>::contains_key(&stash) {
                Err(Error::<T>::AlreadyBonded)?
            }

            let controller = T::Lookup::lookup(controller)?;

            if <Ledger<T>>::contains_key(&controller) {
                Err(Error::<T>::AlreadyPaired)?
            }

            // reject a bond which is considered to be _dust_.
            if value < T::Currency::minimum_balance() {
                Err(Error::<T>::InsufficientValue)?
            }

            // You're auto-bonded forever, here. We might improve this by only bonding when
            // you actually validate/guarantee and remove once you unbond __everything__.
            <Bonded<T>>::insert(&stash, &controller);
            <Payee<T>>::insert(&stash, RewardDestination::Staked);

            let current_era = CurrentEra::get().unwrap_or(0);
            let history_depth = Self::history_depth();
            let last_reward_era = current_era.saturating_sub(history_depth);

            let stash_balance = T::Currency::free_balance(&stash);
            let value = value.min(stash_balance);
            Self::deposit_event(RawEvent::Bonded(stash.clone(), value));
            let item = StakingLedger {
                stash,
                total: value,
                active: value,
                unlocking: vec![],
                claimed_rewards: (last_reward_era..current_era).collect(),
            };
            Self::update_ledger(&controller, &item);
        }

        /// Add some extra amount that have appeared in the stash `free_balance` into the balance up
        /// for staking.
        ///
        /// Use this if there are additional funds in your stash account that you wish to bond.
        /// Unlike [`bond`] or [`unbond`] this function does not impose any limitation on the amount
        /// that can be added.
        ///
        /// The dispatch origin for this call must be _Signed_ by the stash, not the controller and
        /// it can be only called when [`EraElectionStatus`] is `Closed`.
        ///
        /// Emits `Bonded`.
        ///
        /// # <weight>
        /// - Independent of the arguments. Insignificant complexity.
        /// - O(1).
        /// - One DB entry.
        /// ------------
        /// DB Weight:
        /// - Read: Bonded, Ledger, [Origin Account], Locks
        /// - Write: [Origin Account], Locks, Ledger
        /// # </weight>
        #[weight = T::WeightInfo::bond_extra()]
        fn bond_extra(origin, #[compact] max_additional: BalanceOf<T>) {
            let stash = ensure_signed(origin)?;

            let controller = Self::bonded(&stash).ok_or(Error::<T>::NotStash)?;
            let mut ledger = Self::ledger(&controller).ok_or(Error::<T>::NotController)?;

            let stash_balance = T::Currency::free_balance(&stash);
            if let Some(extra) = stash_balance.checked_sub(&ledger.total) {
                let extra = extra.min(max_additional);
                ledger.total += extra;
                ledger.active += extra;
                Self::deposit_event(RawEvent::Bonded(stash, extra));
                Self::update_ledger(&controller, &ledger);
            }
        }

        /// Schedule a portion of the stash to be unlocked ready for transfer out after the bond
        /// period ends. If this leaves an amount actively bonded less than
        /// T::Currency::minimum_balance(), then it is increased to the full amount.
        ///
        /// Once the unlock period is done, you can call `withdraw_unbonded` to actually move
        /// the funds out of management ready for transfer.
        ///
        /// No more than a limited number of unlocking chunks (see `MAX_UNLOCKING_CHUNKS`)
        /// can co-exists at the same time. In that case, [`Call::withdraw_unbonded`] need
        /// to be called first to remove some of the chunks (if possible).
        ///
        /// The dispatch origin for this call must be _Signed_ by the controller, not the stash.
        /// And, it can be only called when [`EraElectionStatus`] is `Closed`.
        ///
        /// Emits `Unbonded`.
        ///
        /// See also [`Call::withdraw_unbonded`].
        ///
        /// # <weight>
        /// - Independent of the arguments. Limited but potentially exploitable complexity.
        /// - Contains a limited number of reads.
        /// - Each call (requires the remainder of the bonded balance to be above `minimum_balance`)
        ///   will cause a new entry to be inserted into a vector (`Ledger.unlocking`) kept in storage.
        ///   The only way to clean the aforementioned storage item is also user-controlled via
        ///   `withdraw_unbonded`.
        /// - One DB entry.
        /// ----------
        /// DB Weight:
        /// - Read: Ledger, Current Era, Locks, [Origin Account]
        /// - Write: [Origin Account], Locks, Ledger
        /// </weight>
        #[weight = T::WeightInfo::unbond()]
        fn unbond(origin, #[compact] value: BalanceOf<T>) {
            let controller = ensure_signed(origin)?;
            let mut ledger = Self::ledger(&controller).ok_or(Error::<T>::NotController)?;

            // 0. Judge if exceed MAX_UNLOCKING_CHUNKS
            ensure!(
                ledger.unlocking.len() < MAX_UNLOCKING_CHUNKS,
                Error::<T>::NoMoreChunks,
            );

            // 1. Ensure guarantee's stakes is free
            let mut value = value;
            if let Some(guarantee) = Self::guarantors(&ledger.stash) {
                ensure!(guarantee.total < ledger.active, Error::<T>::AllGuaranteed);
                value = value.min(ledger.active - guarantee.total);
            }

            // 2. Ensure value < ledger.active
            value = value.min(ledger.active);
            if !value.is_zero() {
                ledger.active -= value;

                // Avoid there being a dust balance left in the staking system.
                if ledger.active < T::Currency::minimum_balance() {
                    value += ledger.active;
                    ledger.active = Zero::zero();
                }

                // Note: in case there is no current era it is fine to bond one era more.
                let era = Self::current_era().unwrap_or(0) + T::BondingDuration::get();
                ledger.unlocking.push(UnlockChunk { value, era });
                Self::update_ledger(&controller, &ledger);
                Self::deposit_event(RawEvent::Unbonded(ledger.stash, value));
            }
        }

	    /// Rebond a portion of the stash scheduled to be unlocked.
		///
		/// The dispatch origin must be signed by the controller, and it can be only called when
		/// [`EraElectionStatus`] is `Closed`.
		///
		/// # <weight>
		/// - Time complexity: O(L), where L is unlocking chunks
		/// - Bounded by `MAX_UNLOCKING_CHUNKS`.
		/// - Storage changes: Can't increase storage, only decrease it.
		/// ---------------
		/// - DB Weight:
		///     - Reads: EraElectionStatus, Ledger, Locks, [Origin Account]
		///     - Writes: [Origin Account], Locks, Ledger
		/// # </weight>
		#[weight = T::WeightInfo::rebond(MAX_UNLOCKING_CHUNKS as u32)]
		fn rebond(origin, #[compact] value: BalanceOf<T>) -> DispatchResultWithPostInfo {
			let controller = ensure_signed(origin)?;
			let ledger = Self::ledger(&controller).ok_or(Error::<T>::NotController)?;
			ensure!(!ledger.unlocking.is_empty(), Error::<T>::NoUnlockChunk);

			let ledger = ledger.rebond(value);
			// last check: the new active amount of ledger must be more than ED.
			ensure!(ledger.active >= T::Currency::minimum_balance(), Error::<T>::InsufficientValue);

			Self::update_ledger(&controller, &ledger);
			Ok(Some(
				35 * WEIGHT_PER_MICROS
				+ 50 * WEIGHT_PER_NANOS * (ledger.unlocking.len() as Weight)
				+ T::DbWeight::get().reads_writes(3, 2)
			).into())
		}

        /// Remove any unlocked chunks from the `unlocking` queue from our management.
        ///
        /// This essentially frees up that balance to be used by the stash account to do
        /// whatever it wants.
        ///
        /// The dispatch origin for this call must be _Signed_ by the controller, not the stash.
        /// And, it can be only called when [`EraElectionStatus`] is `Closed`.
        ///
        /// Emits `Withdrawn`.
        ///
        /// See also [`Call::unbond`].
        ///
        /// # <weight>
        /// - Could be dependent on the `origin` argument and how much `unlocking` chunks exist.
        ///  It implies `consolidate_unlocked` which loops over `Ledger.unlocking`, which is
        ///  indirectly user-controlled. See [`unbond`] for more detail.
        /// - Contains a limited number of reads, yet the size of which could be large based on `ledger`.
        /// - Writes are limited to the `origin` account key.
        /// ---------------
        /// Complexity O(S) where S is the number of slashing spans to remove
        /// Update:
        /// - Reads: EraElectionStatus, Ledger, Current Era, Locks, [Origin Account]
        /// - Writes: [Origin Account], Locks, Ledger
        /// Kill:
        /// - Reads: EraElectionStatus, Ledger, Current Era, Bonded, [Origin Account], Locks
        /// - Writes: Bonded, Slashing Spans (if S > 0), Ledger, Payee, Validators, Guarantors, [Origin Account], Locks
        /// - Writes Each: SpanSlash * S
        /// NOTE: Weight annotation is the kill scenario, we refund otherwise.
        /// # </weight>
        #[weight = T::WeightInfo::withdraw_unbonded()]
        fn withdraw_unbonded(origin) {
            let controller = ensure_signed(origin)?;
            let mut ledger = Self::ledger(&controller).ok_or(Error::<T>::NotController)?;
            let (stash, old_total) = (ledger.stash.clone(), ledger.total);
            if let Some(current_era) = Self::current_era() {
                // remove the lock first, update_ledger would add the lock back anyway
                T::Currency::remove_lock(STAKING_ID, &stash);
                ledger = ledger.consolidate_unlocked(current_era);
            }

            if ledger.unlocking.is_empty() && ledger.active.is_zero() {
                // This account must have called `unbond()` with some value that caused the active
                // portion to fall below existential deposit + will have no more unlocking chunks
                // left. We can now safely remove all staking-related information.
                Self::kill_stash(&stash)?;
            } else {
                // This was the consequence of a partial unbond. just update the ledger and move on.
                Self::update_ledger(&controller, &ledger);
            }

            // `old_total` should never be less than the new total because
            // `consolidate_unlocked` strictly subtracts balance.
            if ledger.total < old_total {
                // Already checked that this won't overflow by entry condition.
                let value = old_total - ledger.total;
                Self::deposit_event(RawEvent::Withdrawn(stash, value));
            }
        }

        /// Declare the desire to validate for the origin controller.
        ///
        /// Effects will be felt at the beginning of the next era.
        ///
        /// The dispatch origin for this call must be _Signed_ by the controller, not the stash.
        ///
        /// # <weight>
        /// - Independent of the arguments. Insignificant complexity.
        /// - Contains a limited number of reads.
        /// - Writes are limited to the `origin` account key.
        /// -----------
        /// DB Weight:
        /// - Read: Ledger, StakeLimit
        /// - Write: Guarantors, Validators
        /// # </weight>
        #[weight = T::WeightInfo::validate()]
        fn validate(origin, prefs: ValidatorPrefs) {
            let controller = ensure_signed(origin)?;
            let ledger = Self::ledger(&controller).ok_or(Error::<T>::NotController)?;
            let v_stash = &ledger.stash;
            <Guarantors<T>>::remove(v_stash);
            <Validators<T>>::insert(v_stash, &prefs);
            // Set the validator pref to 100% for the ongoing era as the punishment
            if let Some(active_era) = Self::active_era() {
                if <ErasValidatorPrefs<T>>::get(&active_era.index, &v_stash).fee > prefs.fee {
                    <ErasValidatorPrefs<T>>::insert(&active_era.index, &v_stash, ValidatorPrefs { fee: Perbill::one() });
                }
            }
            Self::deposit_event(RawEvent::ValidateSuccess(controller, prefs));
        }

        /// Declare the desire to guarantee `targets` for the origin controller.
        ///
        /// The dispatch origin for this call must be _Signed_ by the controller, not the stash.
        ///
        /// # <weight>
        /// - The transaction's complexity is proportional to the size of `validators` (N),
        /// `guarantors`, `guarantee_rel`
        /// - Both the reads and writes follow a similar pattern.
        /// ---------
        /// DB Weight:
        /// - Reads: Guarantors, Ledger, Current Era
        /// - Writes: Guarantors
        /// # </weight>
        #[weight = T::WeightInfo::guarantee()]
        fn guarantee(origin, target: (<T::Lookup as StaticLookup>::Source, BalanceOf<T>)) {
            // 1. Get ledger
            let controller = ensure_signed(origin)?;
            let ledger = Self::ledger(&controller).ok_or(Error::<T>::NotController)?;
            let g_stash = &ledger.stash;
            let (target, votes) = target;

            // 2. Target should be legal
            let v_stash = T::Lookup::lookup(target)?;
            ensure!(<Validators<T>>::contains_key(&v_stash), Error::<T>::InvalidTarget);

            // 3. Votes value should greater than the dust
            ensure!(votes > T::Currency::minimum_balance(), Error::<T>::InsufficientValue);

            // 4. Upsert (increased) guarantee
            let guarantee = Self::increase_guarantee(&v_stash, g_stash, ledger.active.clone(), votes.clone());

            // 5. `None` means exceed the guarantee limit(`MAX_GUARANTEE`)
            ensure!(guarantee.is_some(), Error::<T>::ExceedGuaranteeLimit);
            let guarantee = guarantee.unwrap();

            <Validators<T>>::remove(g_stash);
            <Guarantors<T>>::insert(g_stash, guarantee);
            Self::deposit_event(RawEvent::GuaranteeSuccess(controller, v_stash, votes));
        }

        /// Declare the desire to cut guarantee for the origin controller.
        ///
        /// Effects will be felt at the beginning of the next era.
        ///
        /// The dispatch origin for this call must be _Signed_ by the controller, not the stash.
        ///
        /// # <weight>
        /// - The transaction's complexity is proportional to the size of `validators` (N),
        /// `guarantors`, `guarantee_rel`
        /// - Both the reads and writes follow a similar pattern.
        /// ---------
        /// DB Weight:
        /// - Reads: Guarantors, Ledger, Current Era
        /// - Writes: Validators, Guarantors
        /// # </weight>
        #[weight = T::WeightInfo::cut_guarantee()]
        fn cut_guarantee(origin, target: (<T::Lookup as StaticLookup>::Source, BalanceOf<T>)) {
            // 1. Get ledger
            let controller = ensure_signed(origin)?;
            let ledger = Self::ledger(&controller).ok_or(Error::<T>::NotController)?;
            let g_stash = &ledger.stash;
            let (target, votes) = target;

            // 2. Target should be legal
            let v_stash = T::Lookup::lookup(target)?;

            // 3. Votes value should greater than the dust
            ensure!(votes > T::Currency::minimum_balance(), Error::<T>::InsufficientValue);

            // 4. Upsert (decreased) guarantee
            let guarantee = Self::decrease_guarantee(&v_stash, &g_stash, votes.clone());

            // 5. `None` means the target is invalid(cut a void)
            ensure!(guarantee.is_some(), Error::<T>::InvalidTarget);
            let guarantee = guarantee.unwrap();

            <Guarantors<T>>::insert(g_stash, guarantee);
            Self::deposit_event(RawEvent::CutGuaranteeSuccess(controller, v_stash, votes));
        }

        /// Declare no desire to either validate or guarantee.
        ///
        /// Effects will be felt at the beginning of the next era.
        ///
        /// The dispatch origin for this call must be _Signed_ by the controller, not the stash.
        ///
        /// # <weight>
        /// - Independent of the arguments. Insignificant complexity.
        /// - Contains one read.
        /// - Writes are limited to the `origin` account key.
        /// --------
        /// DB Weight:
        /// - Read: Ledger
        /// - Write: Validators, Guarantors
        /// # </weight>
        #[weight = T::WeightInfo::chill()]
        fn chill(origin) {
            let controller = ensure_signed(origin)?;
            let ledger = Self::ledger(&controller).ok_or(Error::<T>::NotController)?;
            Self::chill_stash(&ledger.stash);
            Self::deposit_event(RawEvent::ChillSuccess(controller, ledger.stash));
        }

        /// (Re-)set the controller of a stash.
        ///
        /// Effects will be felt at the beginning of the next era.
        ///
        /// The dispatch origin for this call must be _Signed_ by the stash, not the controller.
        ///
        /// # <weight>
        /// - Independent of the arguments. Insignificant complexity.
        /// - Contains a limited number of reads.
        /// - Writes are limited to the `origin` account key.
        /// ----------
        /// DB Weight:
        /// - Read: Bonded, Ledger New Controller, Ledger Old Controller
        /// - Write: Bonded, Ledger New Controller, Ledger Old Controller
        /// # </weight>
        #[weight = T::WeightInfo::set_controller()]
        fn set_controller(origin, controller: <T::Lookup as StaticLookup>::Source) {
            let stash = ensure_signed(origin)?;
            let old_controller = Self::bonded(&stash).ok_or(Error::<T>::NotStash)?;
            let controller = T::Lookup::lookup(controller)?;
            if <Ledger<T>>::contains_key(&controller) {
                Err(Error::<T>::AlreadyPaired)?
            }
            if controller != old_controller {
                <Bonded<T>>::insert(&stash, &controller);
                if let Some(l) = <Ledger<T>>::take(&old_controller) {
                    <Ledger<T>>::insert(&controller, l);
                }
            }
        }

        /// Pay out all the stakers behind a single validator for a single era.
        ///
        /// - `validator_stash` is the stash account of the validator. Their guarantors, up to
        ///   `T::MaxGuarantorRewardedPerValidator`, will also receive their rewards.
        /// - `era` may be any era between `[current_era - history_depth; current_era]`.
        ///
        /// The origin of this call must be _Signed_. Any account can call this function, even if
        /// it is not one of the stakers.
        /// TODO: Add weight for this one
        #[weight = 120 * WEIGHT_PER_MICROS]
        fn reward_stakers(origin, validator_stash: T::AccountId, era: EraIndex) -> DispatchResult {
            ensure_signed(origin)?;
            Self::do_reward_stakers(validator_stash, era)
        }


        /// Sets the ideal number of validators.
        ///
        /// The dispatch origin must be Root.
        ///
        /// # <weight>
        /// Base Weight: 1.717 µs
        /// Write: Validator Count
        /// # </weight>
        #[weight = 2 * WEIGHT_PER_MICROS + T::DbWeight::get().writes(1)]
        fn set_validator_count(origin, #[compact] new: u32) {
            ensure_root(origin)?;
            ValidatorCount::put(new);
        }

		/// Increments the ideal number of validators.
		///
		/// The dispatch origin must be Root.
		///
		/// # <weight>
		/// Same as [`set_validator_count`].
		/// # </weight>
		#[weight = 2 * WEIGHT_PER_MICROS + T::DbWeight::get().writes(1) + T::DbWeight::get().reads(1)]
		fn increase_validator_count(origin, #[compact] additional: u32) {
			ensure_root(origin)?;
			ValidatorCount::mutate(|n| *n += additional);
		}

        /// Force a current staker to become completely unstaked, immediately.
        ///
        /// The dispatch origin must be Root.
        ///
        /// # <weight>
        /// O(S) where S is the number of slashing spans to be removed
        /// Base Weight: 53.07 µs
        /// Reads: Bonded, Account, Locks
        /// Writes: Bonded, Ledger, Payee, Validators, Guarantors, Account, Locks
        /// # </weight>
        #[weight = T::DbWeight::get().reads_writes(4, 7)
            .saturating_add(53 * WEIGHT_PER_MICROS)]
        fn force_unstake(origin, stash: T::AccountId) {
            ensure_root(origin)?;

            // remove the lock.
            T::Currency::remove_lock(STAKING_ID, &stash);
            // remove all staking-related information.
            Self::kill_stash(&stash)?;
        }

        /// Remove all data structure concerning a staker/stash once its balance is zero.
        /// This is essentially equivalent to `withdraw_unbonded` except it can be called by anyone
        /// and the target `stash` must have no funds left.
        ///
        /// This can be called from any origin.
        ///
        /// - `stash`: The stash account to reap. Its balance must be zero.
        ///
        /// # <weight>
        /// Complexity: O(S) where S is the number of slashing spans on the account.
        /// Base Weight: 75.94 µs
        /// DB Weight:
        /// - Reads: Stash Account, Bonded, Slashing Spans, Locks
        /// - Writes: Bonded, Ledger, Payee, Validators, guarantors, Stash Account, Locks
        /// # </weight>
        #[weight = T::DbWeight::get().reads_writes(4, 7)
            .saturating_add(76 * WEIGHT_PER_MICROS)]
        fn reap_stash(_origin, stash: T::AccountId) {
            let at_minimum = T::Currency::total_balance(&stash) == T::Currency::minimum_balance();
            ensure!(at_minimum, Error::<T>::FundedTarget);
            Self::kill_stash(&stash)?;
            T::Currency::remove_lock(STAKING_ID, &stash);
        }

        // TODO: Remove it after the main net reward start
        #[weight = 1000]
        fn set_start_reward_era(origin, start_reward_era: EraIndex) {
            ensure_root(origin)?;
            StartRewardEra::put(start_reward_era);
        }

        #[weight = 1000]
        fn cancel_era_reward(origin, era_index: EraIndex) {
            ensure_root(origin)?;
            <ErasStakingPayout<T>>::remove(era_index);
        }
    }
}

impl<T: Config> Module<T> {
    // PUBLIC IMMUTABLES

    // PRIVATE IMMUTABLES

    /// Calculate the stake limit by storage workloads, returns the stake limit value
    ///
    /// # <weight>
    /// - Independent of the arguments. Insignificant complexity.
    /// - O(1).
    /// - 0 DB entry.
    /// # </weight>
    pub fn stage_one_stake_limit_of(own_workloads: u128) -> BalanceOf<T> {
        // we treat 1 terabytes as 1_000_000_000_000 for make `mapping_ratio = 1`
        if let Some(storage_stakes) = own_workloads.checked_mul(T::SPowerRatio::get()) {
            storage_stakes.try_into().ok().unwrap()
        } else {
            Zero::zero()
        }
    }

    pub fn update_stage_one_stake_limit(workload_map: BTreeMap<T::AccountId, u128>) -> u64 {
        // In stage one, state limit / own workload is fixed to T::SPowerRatio
        let mut validators_count = 0;
        for (v_stash, _) in <Validators<T>>::iter() {
            validators_count += 1;
            let v_own_workload = workload_map.get(&v_stash).unwrap_or(&0u128);
            Self::upsert_stake_limit(
                &v_stash,
                Self::stage_one_stake_limit_of(*v_own_workload),
            );
        }
        validators_count
    }

    /// Calculate the stake limit by storage workloads, returns the stake limit value
    ///
    /// # <weight>
    /// - Independent of the arguments. Insignificant complexity.
    /// - O(1).
    /// - 0 DB entry.
    /// # </weight>
    pub fn stage_two_stake_limit_of(own_workloads_in_kb: u128, total_workloads_in_kb: u128, total_stake_limit: u128) -> BalanceOf<T> {
        // total_workloads cannot be zero, or system go panic!
        if total_workloads_in_kb == 0 {
            Zero::zero()
        } else {
            let workloads_to_stakes = (own_workloads_in_kb.wrapping_mul(total_stake_limit) / total_workloads_in_kb) as u128;
            workloads_to_stakes.try_into().ok().unwrap()
        }
    }

    pub fn update_stage_two_stake_limit(workload_map: BTreeMap<T::AccountId, u128>, total_workload: u128, total_stake_limit: u128) -> u64 {
        let mut validators_count = 0;
        let byte_to_kilobyte = |workload_in_byte: u128| {
            workload_in_byte / 1024
        };

        // Decrease the precision to kb to avoid overflow
        let total_workload_in_kb = byte_to_kilobyte(total_workload);
        for (v_stash, _) in <Validators<T>>::iter() {
            validators_count += 1;
            let v_own_workload = workload_map.get(&v_stash).unwrap_or(&0u128);
            // Decrease the precision to kb to avoid overflow
            let v_own_workload_in_kb = byte_to_kilobyte(*v_own_workload);
            Self::upsert_stake_limit(
                &v_stash,
                Self::stage_two_stake_limit_of(v_own_workload_in_kb, total_workload_in_kb, total_stake_limit),
            );
        }
        validators_count
    }

    pub fn limit_ratio_according_to_effective_staking(total_issuance: BalanceOf<T>) -> (u128, Perbill) {
        let maybe_effective_stake_ratio = Self::maybe_get_effective_staking_ratio(total_issuance);
        if let Some(effective_stake_ratio) = maybe_effective_stake_ratio {
            let (integer, frac) = total_stake_limit_ratio(effective_stake_ratio);
            return (integer.into(), frac);
        }
        return (0u128, Perbill::zero());
    }

    fn maybe_get_effective_staking_ratio(total_issuance: BalanceOf<T>) -> Option<Permill> {
        let to_num =
            |b: BalanceOf<T>| <T::CurrencyToVote as Convert<BalanceOf<T>, u128>>::convert(b);
        if let Some(active_era) = Self::active_era() {
            let total_effective_stake = <ErasTotalStakes<T>>::get(&active_era.index);
            return Some(Permill::from_rational_approximation(to_num(total_effective_stake), to_num(total_issuance)));
        }
        None
    }

    fn calculate_total_stake_limit() -> u128 {
        let total_issuance = T::Currency::total_issuance();
        // If effective staking ratio is smaller than some value, we should increase the total stake limit
        let (integer, frac) = Self::limit_ratio_according_to_effective_staking(total_issuance.clone());
        let frac = frac * total_issuance;
        let integer = BalanceOf::<T>::saturated_from(integer).saturating_mul(total_issuance);
        // This value can be larger than total issuance.
        let total_stake_limit = TryInto::<u128>::try_into(integer.saturating_add(frac))
            .ok()
            .unwrap();
        total_stake_limit
    }

    /// Get the updated (increased) guarantee relationship
    /// Basically, this function construct an updated edge or insert a new edge,
    /// then returns the updated `Guarantee`
    ///
    /// # <weight>
    /// - Independent of the arguments. Insignificant complexity.
    /// - O(1).
    /// - 1 DB entry.
    /// # </weight>
    fn increase_guarantee(
        v_stash: &T::AccountId,
        g_stash: &T::AccountId,
        bonded: BalanceOf<T>,
        votes: BalanceOf<T>
    ) -> Option<Guarantee<T::AccountId, BalanceOf<T>>> {
        // 1. Already guaranteed
        if let Some(guarantee) = Self::guarantors(g_stash) {
            let remains = bonded.saturating_sub(guarantee.total);
            let real_votes = remains.min(votes);
            let new_total = guarantee.total.saturating_add(real_votes);
            let mut new_targets: Vec<IndividualExposure<T::AccountId, BalanceOf<T>>> = vec![];
            let mut update = false;

            if real_votes <= Zero::zero() {
                log!(
                    debug,
                    "💸 Staking limit of validator {:?} is zero.",
                    v_stash
                );
                return None
            }

            // Fill in `new_targets`, always LOOP the `targets`
            // However, the TC is O(1) due to the `MAX_GUARANTEE` restriction 🤪
            for mut target in guarantee.targets {
                // a. Update an edge
                if &target.who == v_stash {
                    target.value += real_votes;
                    update = true;
                }
                new_targets.push(target.clone());
            }

            if !update {
                if new_targets.len() >= MAX_GUARANTEE {
                    return None
                } else {
                    // b. New an edge
                    new_targets.push(IndividualExposure {
                        who: v_stash.clone(),
                        value: real_votes
                    });
                }
            }

            Some(Guarantee {
                targets: new_targets.clone(),
                total: new_total,
                submitted_in: Self::current_era().unwrap_or(0),
                suppressed: false,
            })

        // 2. New guarantee
        } else {
            let real_votes = bonded.min(votes);
            let new_total = real_votes;

            // No need check with this case, votes and bonded all greater than 0

            let mut new_targets: Vec<IndividualExposure<T::AccountId, BalanceOf<T>>> = vec![];
            new_targets.push(IndividualExposure {
                who: v_stash.clone(),
                value: real_votes
            });

            Some(Guarantee {
                targets: new_targets.clone(),
                total: new_total,
                submitted_in: Self::current_era().unwrap_or(0),
                suppressed: false,
            })
        }
    }

    /// Get the updated (decreased) guarantee relationship
    /// Basically, this function construct an updated edge,
    /// then returns the updated `Guarantee`
    ///
    /// # <weight>
    /// - Independent of the arguments. Insignificant complexity.
    /// - O(1).
    /// - 1 DB entry.
    /// # </weight>
    fn decrease_guarantee(
        v_stash: &T::AccountId,
        g_stash: &T::AccountId,
        votes: BalanceOf<T>,
    ) -> Option<Guarantee<T::AccountId, BalanceOf<T>>> {
        if let Some(guarantee) = Self::guarantors(g_stash) {
            // `decreased_votes` = min(votes, target.value)
            // `new_targets` means the targets after decreased
            // `exists` means the targets contains `v_stash`
            let mut decreased_votes = Zero::zero();
            let mut new_targets: Vec<IndividualExposure<T::AccountId, BalanceOf<T>>> = vec![];
            let mut exists = false;

            // Always LOOP the targets
            // However, the TC is O(1), due to the `MAX_GUARANTEE` restriction 🤪
            for target in guarantee.targets {
                if &target.who == v_stash {
                    // 1. Mark it really exists (BRAVO), and update the decreased votes
                    exists = true;
                    decreased_votes = target.value.min(votes);

                    if target.value <= votes{
                        // 2. Remove this target
                    } else {
                        // 3. Decrease the value
                        let new_target = IndividualExposure {
                            who: v_stash.clone(),
                            value: target.value - votes
                        };
                        new_targets.push(new_target);
                    }
                } else {
                    // 4. Push target with no change
                    new_targets.push(target.clone());
                }
            }

            if exists  {
                // 5. Update `new_total` with saturating sub the decreased_votes
                let new_total = guarantee.total.saturating_sub(decreased_votes);

                // TODO: `submitted_in` and `suppressed` should not be change?
                return Some(Guarantee {
                    targets: new_targets.clone(),
                    total: new_total,
                    submitted_in: guarantee.submitted_in,
                    suppressed: guarantee.suppressed
                })
            }
        }

        None
    }

    /// Insert new or update old stake limit
    pub fn upsert_stake_limit(account_id: &T::AccountId, limit: BalanceOf<T>) {
        <StakeLimit<T>>::insert(account_id, limit);
    }

    /// Update the ledger for a controller. This will also update the stash lock. The lock will
    /// will lock the entire funds except paying for further transactions.
    fn update_ledger(
        controller: &T::AccountId,
        ledger: &StakingLedger<T::AccountId, BalanceOf<T>>,
    ) {
        T::Currency::set_lock(
            STAKING_ID,
            &ledger.stash,
            ledger.total,
            WithdrawReasons::all(),
        );
        <Ledger<T>>::insert(controller, ledger);
    }

    /// Chill a stash account.
    fn chill_stash(stash: &T::AccountId) {
        <StakeLimit<T>>::remove(stash);
        <Validators<T>>::remove(stash);
        <Guarantors<T>>::remove(stash);
    }

    /// Actually make a payment to a staker. This uses the currency's reward function
    /// to pay the right payee for the given staker account.
    fn make_payout(stash: &T::AccountId, amount: BalanceOf<T>) -> Option<PositiveImbalanceOf<T>> {
        let dest = Self::payee(stash);
        match dest {
            RewardDestination::Controller => Self::bonded(stash).and_then(|controller| {
                T::Currency::deposit_into_existing(&controller, amount).ok()
            }),
            RewardDestination::Stash => T::Currency::deposit_into_existing(stash, amount).ok(),
            RewardDestination::Staked => Self::bonded(stash)
                .and_then(|c| Self::ledger(&c).map(|l| (c, l)))
                .and_then(|(controller, mut l)| {
                    l.active += amount;
                    l.total += amount;
                    let r = T::Currency::deposit_into_existing(stash, amount).ok();
                    Self::update_ledger(&controller, &l);
                    r
                }),
            RewardDestination::Account(dest_account) => {
                Some(T::Currency::deposit_creating(&dest_account, amount))
            }
        }
    }

    /// Pay reward to stakers. Two kinds of reward.
    /// One is authoring reward which is paid to validator who are elected.
    /// Another one is staking reward.
    fn do_reward_stakers(
        validator_stash: T::AccountId,
        era: EraIndex,
    ) -> DispatchResult {
        // 1. Validate input data
        let current_era = CurrentEra::get().ok_or(Error::<T>::InvalidEraToReward)?;
        ensure!(era <= current_era, Error::<T>::InvalidEraToReward);
        let history_depth = Self::history_depth();
        ensure!(era >= current_era.saturating_sub(history_depth), Error::<T>::InvalidEraToReward);

        // Note: if era has no reward to be claimed, era may be future. better not to update
        // `ledger.claimed_rewards` in this case.
        let total_era_staking_payout = <ErasStakingPayout<T>>::get(&era)
            .ok_or_else(|| Error::<T>::InvalidEraToReward)?;

        let controller = Self::bonded(&validator_stash).ok_or(Error::<T>::NotStash)?;
        let mut ledger = <Ledger<T>>::get(&controller).ok_or_else(|| Error::<T>::NotController)?;

        ledger.claimed_rewards.retain(|&x| x >= current_era.saturating_sub(history_depth));
        match ledger.claimed_rewards.binary_search(&era) {
            Ok(_) => Err(Error::<T>::AlreadyClaimed)?,
            Err(pos) => ledger.claimed_rewards.insert(pos, era),
        }
        /* Input data seems good, no errors allowed after this point */
        let exposure = <ErasStakersClipped<T>>::get(&era, &ledger.stash);
        <Ledger<T>>::insert(&controller, &ledger);

        // 2. Pay authoring reward
        let mut validator_imbalance = <PositiveImbalanceOf<T>>::zero();
        let mut total_reward: BalanceOf<T> = Zero::zero();
        if let Some(authoring_reward) = <ErasAuthoringPayout<T>>::get(&era, &validator_stash) {
            total_reward = total_reward.saturating_add(authoring_reward);
        }

        let to_num =
        |b: BalanceOf<T>| <T::CurrencyToVote as Convert<BalanceOf<T>, u128>>::convert(b);

        // 3. Retrieve total stakes and total staking reward
        let era_total_stakes = <ErasTotalStakes<T>>::get(&era);
        let staking_reward = Perbill::from_rational_approximation(to_num(exposure.total), to_num(era_total_stakes)) * total_era_staking_payout;
        total_reward = total_reward.saturating_add(staking_reward);
        let total = exposure.total.max(One::one());
        // 4. Calculate guarantee rewards for staking
        let estimated_guarantee_rewards = <ErasValidatorPrefs<T>>::get(&era, &ledger.stash).fee * total_reward;
        let mut guarantee_rewards = Zero::zero();
        // 5. Pay staking reward to guarantors
        for i in &exposure.others {
            let reward_ratio = Perbill::from_rational_approximation(i.value, total);
            // Reward guarantors
            guarantee_rewards += reward_ratio * estimated_guarantee_rewards;
            if let Some(imbalance) = Self::make_payout(
                &i.who,
                reward_ratio * estimated_guarantee_rewards
            ) {
                Self::deposit_event(RawEvent::Reward(i.who.clone(), imbalance.peek()));
            };
        }
        // 6. Pay staking reward to validator
        validator_imbalance.maybe_subsume(Self::make_payout(&ledger.stash, total_reward - guarantee_rewards));
        Self::deposit_event(RawEvent::Reward(ledger.stash, validator_imbalance.peek()));
        Ok(())
    }

    /// Session has just ended. Provide the validator set for the next session if it's an era-end, along
    /// with the exposure of the prior validator set.
    fn new_session() {
        Self::new_era()
    }


    /// The era has changed - enact new staking set.
    ///
    /// NOTE: This always happens immediately before a session change to ensure that new validators
    /// get a chance to set their session keys.
    /// This also checks stake limitation based on work reports
    fn new_era() {
        // Increment or set current era.
        let current_era = CurrentEra::mutate(|s| {
            *s = Some(s.map(|s| s + 1).unwrap_or(0));
            s.unwrap()
        });
        log!(
            trace,
            "💸 Plan a new era {:?}",
            current_era,
        );

        // Clean old era information.
        if let Some(old_era) = current_era.checked_sub(Self::history_depth() + 1) {
            Self::clear_era_information(old_era);
        }

        // Set staking information for new era.
        Self::select_and_update_validators(current_era);
    }

    /// Start a session potentially starting an era.
    fn start_session() {
        Self::start_era();
    }

    /// End a session potentially ending an era.
    fn end_session() {
        if let Some(active_era) = Self::active_era() {
            Self::end_era(active_era);
        }
    }

    /// * Increment `active_era.index`,
    /// * reset `active_era.start`,
    /// * update `BondedEras` and apply slashes.
    fn start_era() {
        let active_era = ActiveEra::mutate(|active_era| {
            let new_index = active_era.as_ref().map(|info| info.index + 1).unwrap_or(0);
            *active_era = Some(ActiveEraInfo {
                index: new_index,
                // Set new active era start in next `on_finalize`. To guarantee usage of `Time`
                start: None,
            });
            new_index
        });
        log!(
            trace,
            "💸 Start the era {:?}",
            active_era,
        );
    }

    /// Compute payout for era.
    fn end_era(active_era: ActiveEraInfo) {
        // Note: active_era_start can be None if end era is called during genesis config.
        log!(
            trace,
            "💸 End the era {:?}",
            active_era.index,
        );
        if let Some(active_era_start) = active_era.start {
            let now_as_millis_u64 = T::UnixTime::now().as_millis().saturated_into::<u64>();

            let era_duration = now_as_millis_u64 - active_era_start;
            if !era_duration.is_zero() {
                let active_era_index = active_era.index.clone();
                let gpos_total_payout = Self::total_rewards_in_era(active_era_index);

                // 1. Market's staking payout
                let market_total_payout = Self::calculate_market_payout(active_era_index);
                let mut total_payout = market_total_payout.saturating_add(gpos_total_payout);

                // 2. decrease the last fee reduction and update the next total fee reduction
                let used_fee = T::BenefitInterface::update_era_benefit(active_era_index + 1, total_payout);
                total_payout = total_payout.saturating_sub(used_fee);

                // 3. Split the payout for staking and authoring
                let num_of_validators = Self::current_elected().len();
                let total_authoring_payout = Self::get_authoring_and_staking_reward_ratio(num_of_validators as u32) * total_payout;
                let total_staking_payout = total_payout.saturating_sub(total_authoring_payout);

                // 4. Block authoring payout
                for v_stash in Self::current_elected() {
                    let authoring_reward = Perbill::from_rational_approximation(1, num_of_validators as u32) * total_authoring_payout;
                    <ErasAuthoringPayout<T>>::insert(&active_era_index, v_stash, authoring_reward);
                }

                // 5. Staking payout
                <ErasStakingPayout<T>>::insert(active_era_index, total_staking_payout);
    
                // 6. Deposit era reward event
                Self::deposit_event(RawEvent::EraReward(active_era_index, total_authoring_payout, total_staking_payout));
    
                // TODO: enable treasury and might bring this back
                // T::Reward::on_unbalanced(total_imbalance);
                // This is not been used
                // T::RewardRemainder::on_unbalanced(T::Currency::issue(rest));
            }
        }
    }

        /// Clear all era information for given era.
    fn clear_era_information(era_index: EraIndex) {
        <ErasStakers<T>>::remove_prefix(era_index, None);
        <ErasStakersClipped<T>>::remove_prefix(era_index, None);
        <ErasValidatorPrefs<T>>::remove_prefix(era_index, None);
        <ErasStakingPayout<T>>::remove(era_index);
        <ErasMarketPayout<T>>::remove(era_index);
        <ErasAuthoringPayout<T>>::remove_prefix(era_index, None);
        <ErasTotalStakes<T>>::remove(era_index);
    }

    fn total_rewards_in_era(active_era: EraIndex) -> BalanceOf<T> {
        // 1. Has not start rewarding yet
        if active_era < Self::start_reward_era() { return Zero::zero(); }
        let mut maybe_rewards_this_year = FIRST_YEAR_REWARDS ;
        let total_issuance = TryInto::<u128>::try_into(T::Currency::total_issuance())
            .ok()
            .unwrap();
        // Milliseconds per year for the Julian year (365.25 days).
        const MILLISECONDS_PER_YEAR: u64 = 1000 * 3600 * 24 * 36525 / 100;
        // 1 Julian year = (365.25d * 24h * 3600s * 1000ms) / (millisecs_in_era = block_time * blocks_num_in_era)
        let year_in_eras = MILLISECONDS_PER_YEAR / MILLISECS_PER_BLOCK / EPOCH_DURATION_IN_BLOCKS as u64;
        let year_num = active_era.saturating_sub(Self::start_reward_era()) as u64 / year_in_eras;
        for _ in 0..year_num {
            maybe_rewards_this_year = maybe_rewards_this_year * REWARD_DECREASE_RATIO.0 / REWARD_DECREASE_RATIO.1;

            // If reward inflation <= 2.8%, stop reduce
            let min_rewards_this_year = total_issuance / MIN_REWARD_RATIO.1 * MIN_REWARD_RATIO.0;
            if maybe_rewards_this_year <= min_rewards_this_year {
                maybe_rewards_this_year = min_rewards_this_year;
                break;
            }
        }

        if year_num >= EXTRA_REWARD_START_YEAR {
            maybe_rewards_this_year = maybe_rewards_this_year.saturating_add(Self::supply_extra_rewards_due_to_low_effective_staking_ratio(total_issuance));
        }

        let reward_this_era = maybe_rewards_this_year / year_in_eras as u128;

        reward_this_era.try_into().ok().unwrap()
    }

    fn supply_extra_rewards_due_to_low_effective_staking_ratio(total_issuance: u128) -> u128 {
        let maybe_effective_staking_ratio = Self::maybe_get_effective_staking_ratio(BalanceOf::<T>::saturated_from(total_issuance));
        if let Some(effective_staking_ratio) = maybe_effective_staking_ratio {
            if effective_staking_ratio < Permill::from_percent(30) {
                // (1 - sr / 0.3) * 0.08 * total_issuance = total_issuance * 8 / 100 - sr * total_issuance * 8 / 30
                return (total_issuance / 100 * 8).saturating_sub(effective_staking_ratio * total_issuance / 30 * 8);
            }
        }
        return 0;
    }

    //     // Milliseconds per year for the Julian year (365.25 days).
    //     const MILLISECONDS_PER_YEAR: u64 = 1000 * 3600 * 24 * 36525 / 100;
    //     // 1 Julian year = (365.25d * 24h * 3600s * 1000ms) / (millisecs_in_era = block_time * blocks_num_in_era)

    fn calculate_market_payout(active_era: EraIndex) -> BalanceOf<T> {
        let total_dsm_staking_payout = T::MarketStakingPot::withdraw_staking_pot();
        let duration = T::MarketStakingPotDuration::get();
        let dsm_staking_payout_per_era = Perbill::from_rational_approximation(1, duration) * total_dsm_staking_payout;
        // Reward starts from this era.
        for i in 0..duration {
            <ErasMarketPayout<T>>::mutate(active_era + i, |payout| match *payout {
                Some(amount) => *payout = Some(amount.saturating_add(dsm_staking_payout_per_era.clone())),
                None => *payout = Some(dsm_staking_payout_per_era.clone())
            });
        }
        Self::eras_market_payout(active_era).unwrap()
    }

    /// Select the new validator set at the end of the era.
    ///
    /// Returns the a set of newly selected _stash_ IDs.
    ///
    /// This should only be called at the end of an era.
    fn select_and_update_validators(current_era: EraIndex) {
        // I. Ensure minimum validator count
        let validator_count = <Validators<T>>::iter().count();

        let to_votes =
            |b: BalanceOf<T>| <T::CurrencyToVote as Convert<BalanceOf<T>, u128>>::convert(b);
        let to_balance = |e: u128| <T::CurrencyToVote as Convert<u128, BalanceOf<T>>>::convert(e);

        // II. Construct and fill in the V/G graph
        // TC is O(V + G*1), V means validator's number, G means guarantor's number
        // DB try is 2

        log!(
            debug,
            "💸 Construct and fill in the V/G graph for the era {:?}.",
            current_era,
        );
        let mut vg_graph: BTreeMap<T::AccountId, Vec<IndividualExposure<T::AccountId, BalanceOf<T>>>> =
            <Validators<T>>::iter().map(|(v_stash, _)|
                (v_stash, Vec::<IndividualExposure<T::AccountId, BalanceOf<T>>>::new())
            ).collect();
        for (guarantor, guarantee) in <Guarantors<T>>::iter() {
            let Guarantee { total: _, submitted_in: _, targets, suppressed: _ } = guarantee;

            for target in targets {
                if let Some(g) = vg_graph.get_mut(&target.who) {
                     g.push(IndividualExposure {
                         who: guarantor.clone(),
                         value: target.value
                     });
                }
            }
        }

        // III. This part will cover
        // 1. Get `ErasStakers` with `stake_limit` and `vg_graph`
        // 2. Get `ErasValidatorPrefs`
        // 3. Get `total_valid_stakes`
        // 4. Fill in `validator_stakes`
        log!(
            debug,
            "💸 Build the erasStakers for the era {:?}.",
            current_era,
        );
        let mut eras_total_stakes: BalanceOf<T> = Zero::zero();
        let mut validators_stakes: Vec<(T::AccountId, u128)> = vec![];
        for (v_stash, voters) in vg_graph.iter() {
            let v_controller = Self::bonded(v_stash).unwrap();
            let v_ledger: StakingLedger<T::AccountId, BalanceOf<T>> =
                Self::ledger(&v_controller).unwrap();

            let stake_limit = Self::stake_limit(v_stash).unwrap_or(Zero::zero());

            // 0. Add to `validator_stakes` but skip adding to `eras_stakers` if stake limit goes 0
            if stake_limit == Zero::zero() {
                validators_stakes.push((v_stash.clone(), 0));
                continue;
            }

            // 1. Calculate the ratio
            let total_stakes = v_ledger.active.saturating_add(
                voters.iter().fold(
                    Zero::zero(),
                    |acc, ie| acc.saturating_add(ie.value)
                ));
            let valid_votes_ratio = Perbill::from_rational_approximation(stake_limit, total_stakes).min(Perbill::one());

            // 2. Calculate validator valid stake
            let own_stake = valid_votes_ratio * v_ledger.active;

            // 3. Construct exposure
            let mut new_exposure = Exposure {
                total: own_stake,
                own: own_stake,
                others: vec![]
            };
            for voter in voters {
                let g_valid_stake = valid_votes_ratio * voter.value;
                new_exposure.total = new_exposure.total.saturating_add(g_valid_stake);
                new_exposure.others.push(IndividualExposure {
                    who: voter.who.clone(),
                    value: g_valid_stake
                });
            }

            // 4. Update snapshots
            <ErasStakers<T>>::insert(&current_era, &v_stash, new_exposure.clone());
            let exposure_total = new_exposure.total;
            let mut exposure_clipped = new_exposure;
            let clipped_max_len = T::MaxGuarantorRewardedPerValidator::get() as usize;
            if exposure_clipped.others.len() > clipped_max_len {
                exposure_clipped.others.sort_by(|a, b| a.value.cmp(&b.value).reverse());
                exposure_clipped.others.truncate(clipped_max_len);
            }
            <ErasStakersClipped<T>>::insert(&current_era, &v_stash, exposure_clipped);

            <ErasValidatorPrefs<T>>::insert(&current_era, &v_stash, Self::validators(&v_stash).clone());
            if let Some(maybe_total_stakes) = eras_total_stakes.checked_add(&exposure_total) {
                eras_total_stakes = maybe_total_stakes;
            } else {
                eras_total_stakes = to_balance(u64::max_value() as u128);
            }

            // 5. Push validator stakes
            validators_stakes.push((v_stash.clone(), to_votes(exposure_total)))
        }

        // Update slot stake.
        <ErasTotalStakes<T>>::insert(&current_era, eras_total_stakes);

        // V. TopDown Election Algorithm with Randomlization
        let to_elect = (Self::validator_count() as usize).min(validators_stakes.len());


        let elected_stashes= Self::do_election(validators_stakes, to_elect);
        log!(
            info,
            "💸 new validator set of size {:?} has been elected via for era {:?}",
            elected_stashes.len(),
            current_era,
        );

        // VI. Update general staking storage
        // Set the new validator set in sessions.
        <CurrentElected<T>>::put(&elected_stashes);
    }

    fn do_election(
        mut validators_stakes: Vec<(T::AccountId, u128)>,
        to_elect: usize) -> Vec<T::AccountId> {
        // Select new validators by top-down their total `valid` stakes
        // then randomly choose some of them from the top validators

        let candidate_to_elect = validators_stakes.len().min(to_elect * 2);
        // sort by 'valid' stakes
        validators_stakes.sort_by(|a, b| b.1.cmp(&a.1));

        // choose top candidate_to_elect number of validators
        let candidate_stashes = validators_stakes[0..candidate_to_elect]
        .iter()
        .map(|(who, stakes)| (who.clone(), *stakes))
        .collect::<Vec<(T::AccountId, u128)>>();

        // TODO: enable it back when the network is stable
        // // shuffle it
        // Self::shuffle_candidates(&mut candidate_stashes);

        // choose elected_stashes number of validators
        let elected_stashes = candidate_stashes[0..to_elect]
        .iter()
        .map(|(who, _stakes)| who.clone())
        .collect::<Vec<T::AccountId>>();
        elected_stashes
    }

    /// Remove all associated data of a stash account from the staking system.
    ///
    /// Assumes storage is upgraded before calling.
    ///
    /// This is called :
    /// - Immediately when an account's balance falls below existential deposit.
    /// - after a `withdraw_unbond()` call that frees all of a stash's bonded balance.
    fn kill_stash(stash: &T::AccountId) -> DispatchResult {
        let controller = <Bonded<T>>::get(stash).ok_or(Error::<T>::NotStash)?;

        <Bonded<T>>::remove(stash);
        <Ledger<T>>::remove(&controller);

        <Payee<T>>::remove(stash);
        <Validators<T>>::remove(stash);
        <Guarantors<T>>::remove(stash);
        <StakeLimit<T>>::remove(stash);

        Ok(())
    }

    pub fn get_authoring_and_staking_reward_ratio(num_of_validators: u32) -> Perbill {
        match num_of_validators {
            0 ..= 500 => Perbill::from_percent(20),
            501 ..= 1000 => Perbill::from_percent(25),
            1001 ..= 2500 => Perbill::from_percent(30),
            2501 ..= 5000 => Perbill::from_percent(40),
            5001 ..= u32::MAX => Perbill::from_percent(50),
        }
    }

    // fn shuffle_candidates(candidates_stakes: &mut Vec<(T::AccountId, u128)>) {
    //     // 1. Construct random seed, 👼 bless the randomness
    //     // seed = [ block_hash, phrase ]
    //     let phrase = b"candidates_shuffle";
    //     let bn = <frame_system::Module<T>>::block_number();
    //     let bh: T::Hash = <frame_system::Module<T>>::block_hash(bn);
    //     let seed = [
    //         &bh.as_ref()[..],
    //         &phrase.encode()[..]
    //     ].concat();
    //
    //     // we'll need a random seed here.
    //     let seed = T::Randomness::random(seed.as_slice());
    //     // seed needs to be guaranteed to be 32 bytes.
    //     let seed = <[u8; 32]>::decode(&mut TrailingZeroInput::new(seed.as_ref()))
    //         .expect("input is padded with zeroes; qed");
    //     let mut rng = ChaChaRng::from_seed(seed);
    //     for i in (0..candidates_stakes.len()).rev() {
    //         let random_index = (rng.next_u32() % (i as u32 + 1)) as usize;
    //         candidates_stakes.swap(random_index, i);
    //     }
    // }
}
impl<T: Config> sp_runtime::BoundToRuntimeAppPublic for Module<T> {
	type Public = AuraId;
}

impl<T: Config> OneSessionHandler<T::AccountId> for Module<T> {
	type Key = AuraId;

	fn on_genesis_session<'a, I: 'a>(_validators: I)
	where
		I: Iterator<Item = (&'a T::AccountId, Self::Key)>,
	{
		// ignore
	}

	fn on_new_session<'a, I: 'a>(_changed: bool, _validators: I, _queued_validators: I)
	where
		I: Iterator<Item = (&'a T::AccountId, Self::Key)>,
	{
		Self::new_session();
	}

	fn on_before_session_ending() {
        Self::end_session();
		Self::start_session();
	}

	fn on_disabled(_i: u32) {
		// ignore
	}
}


impl<T: Config> swork::Works<T::AccountId> for Module<T> {
    fn report_works(workload_map: BTreeMap<T::AccountId, u128>, total_workload: u128) -> Weight {
        let mut consumed_weight: Weight = 0;
        let mut add_db_reads_writes = |reads, writes| {
            consumed_weight += T::DbWeight::get().reads_writes(reads, writes);
        };
        // 1. Calculate total stake limit
        let total_stake_limit = Self::calculate_total_stake_limit();
        let group_counts = workload_map.len() as u32;
        add_db_reads_writes(3, 0);
        // 2. total_workload * SPowerRatio < total_stake_limit => stage one
        let validators_count: u64 = if total_workload.saturating_mul(T::SPowerRatio::get()) < total_stake_limit {
            Self::update_stage_one_stake_limit(workload_map)
        } else {
            Self::update_stage_two_stake_limit(workload_map, total_workload, total_stake_limit)
        };
        add_db_reads_writes(validators_count, validators_count);
        Self::deposit_event(RawEvent::UpdateStakeLimitSuccess(group_counts));
        consumed_weight
    }
}
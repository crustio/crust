#![feature(vec_remove_item)]
#![recursion_limit = "128"]
#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(test)]
mod mock;

#[cfg(any(feature = "runtime-benchmarks", test))]
pub mod benchmarking;

mod slashing;
#[cfg(test)]
mod tests;

use codec::{Decode, Encode, HasCompact};
use frame_support::{
    decl_module, decl_event, decl_storage, ensure, decl_error,
    storage::IterableStorageMap,
    weights::{Weight, constants::{WEIGHT_PER_MICROS, WEIGHT_PER_NANOS}},
    traits::{
        Currency, LockIdentifier, LockableCurrency, WithdrawReasons, OnUnbalanced, Imbalance, Get,
        Time, EnsureOrigin, Randomness
    },
    dispatch::DispatchResult
};
use pallet_session::historical;
use sp_runtime::{
    Perbill, RuntimeDebug,
    traits::{
        Convert, Zero, One, StaticLookup, Saturating, AtLeast32Bit,
        CheckedAdd, TrailingZeroInput
    },
};
use sp_staking::{
    SessionIndex,
    offence::{OnOffenceHandler, OffenceDetails, Offence, ReportOffence, OffenceError},
};

use sp_std::{convert::TryInto, prelude::*, collections::btree_map::BTreeMap};

use frame_system::{ensure_root, ensure_signed};
#[cfg(feature = "std")]
use sp_runtime::{Deserialize, Serialize};

// Crust runtime modules
use swork;
use primitives::{
    constants::{currency::*, time::*},
    traits::TransferrableCurrency
};

use rand_chacha::{rand_core::{RngCore, SeedableRng}, ChaChaRng};

const DEFAULT_MINIMUM_VALIDATOR_COUNT: u32 = 4;
const MAX_UNLOCKING_CHUNKS: usize = 32;
const MAX_GUARANTEE: usize = 16;
const STAKING_ID: LockIdentifier = *b"staking ";

/// Counter for the number of eras that have passed.
pub type EraIndex = u32;

/// Counter for the number of "reward" points earned by a given validator.
pub type Points = u32;

/// Reward points of an era. Used to split era total payout between validators.
#[derive(Encode, Decode, Default)]
// TODO: change to `ErasRewardPoints` for not using index corresponds
pub struct EraPoints {
    /// Total number of points. Equals the sum of reward points for each validator.
    total: Points,
    /// The reward points earned by a given validator. The index of this vec corresponds to the
    /// index into the current validator set.
    individual: Vec<Points>,
}

impl EraPoints {
    /// Add the reward to the validator at the given index. Index must be valid
    /// (i.e. `index < current_elected.len()`).
    fn add_points_to_index(&mut self, index: u32, points: u32) {
        if let Some(new_total) = self.total.checked_add(points) {
            self.total = new_total;
            self.individual
                .resize((index as usize + 1).max(self.individual.len()), 0);
            self.individual[index as usize] += points; // Addition is less than total
        }
    }
}

/// Indicates the initial status of the staker.
#[derive(RuntimeDebug)]
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
#[derive(PartialEq, Eq, Copy, Clone, Encode, Decode, RuntimeDebug)]
pub enum RewardDestination {
    /// Pay into the stash account, increasing the amount at stake accordingly.
    Staked,
    /// Pay into the stash account, not increasing the amount at stake.
    Stash,
    /// Pay into the controller account.
    Controller,
}

impl Default for RewardDestination {
    fn default() -> Self {
        RewardDestination::Staked
    }
}

/// Preference of what happens regarding validation.
#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
pub struct ValidatorPrefs {
    /// Reward that validator takes up-front; only the rest is split between themselves and
    /// nominators.
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

/// A record of the nominations made by a specific account.
#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
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
#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
pub struct UnlockChunk<Balance: HasCompact> {
    /// Amount of funds to be unlocked.
    #[codec(compact)]
    value: Balance,
    /// Era number at which point it'll be unlocked.
    #[codec(compact)]
    era: EraIndex,
}

/// The ledger of a (bonded) stash.
#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
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

impl<AccountId, Balance: HasCompact + Copy + Saturating> StakingLedger<AccountId, Balance> {
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
}

impl<AccountId, Balance> StakingLedger<AccountId, Balance> where
    Balance: AtLeast32Bit + Saturating + Copy,
{
    /// Slash the validator for a given amount of balance. This can grow the value
    /// of the slash in the case that the validator has less than `minimum_balance`
    /// active funds. Returns the amount of funds actually slashed.
    ///
    /// Slashes from `active` funds first, and then `unlocking`, starting with the
    /// chunks that are closest to unlocking.
    fn slash(&mut self, mut value: Balance, minimum_balance: Balance) -> Balance {
        let pre_total = self.total;
        let total = &mut self.total;
        let active = &mut self.active;

        let slash_out_of =
            |total_remaining: &mut Balance, target: &mut Balance, value: &mut Balance| {
                let mut slash_from_target = (*value).min(*target);

                if !slash_from_target.is_zero() {
                    *target -= slash_from_target;

                    // don't leave a dust balance in the staking system.
                    if *target <= minimum_balance {
                        slash_from_target += *target;
                        *value += sp_std::mem::replace(target, Zero::zero());
                    }

                    *total_remaining = total_remaining.saturating_sub(slash_from_target);
                    *value -= slash_from_target;
                }
            };

        slash_out_of(total, active, &mut value);

        let i = self
            .unlocking
            .iter_mut()
            .map(|chunk| {
                slash_out_of(total, &mut chunk.value, &mut value);
                chunk.value
            })
            .take_while(|value| value.is_zero()) // take all fully-consumed chunks out.
            .count();

        // kill all drained chunks.
        let _ = self.unlocking.drain(..i);

        pre_total.saturating_sub(*total)
    }
}

/// The amount of exposure (to slashing) than an individual guarantor has.
#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Encode, Decode, RuntimeDebug)]
pub struct IndividualExposure<AccountId, Balance: HasCompact> {
    /// The stash account of the guarantor/validator in question.
    pub who: AccountId,
    /// Amount of funds exposed.
    #[codec(compact)]
    pub value: Balance,
}

/// A snapshot of the stake backing a single validator in the system.
#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Encode, Decode, Default, RuntimeDebug)]
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

/// A pending slash record. The value of the slash has been computed but not applied yet,
/// rather deferred for several eras.
#[derive(Encode, Decode, Default, RuntimeDebug)]
pub struct UnappliedSlash<AccountId, Balance: HasCompact> {
    /// The stash ID of the offending validator.
    validator: AccountId,
    /// The validator's own slash.
    own: Balance,
    /// All other slashed stakers and amounts.
    others: Vec<(AccountId, Balance)>,
    /// Reporters of the offence; bounty payout recipients.
    reporters: Vec<AccountId>,
    /// The amount of payout.
    payout: Balance,
}

pub type BalanceOf<T> =
    <<T as Trait>::Currency as Currency<<T as frame_system::Trait>::AccountId>>::Balance;
type PositiveImbalanceOf<T> =
    <<T as Trait>::Currency as Currency<<T as frame_system::Trait>::AccountId>>::PositiveImbalance;
type NegativeImbalanceOf<T> =
    <<T as Trait>::Currency as Currency<<T as frame_system::Trait>::AccountId>>::NegativeImbalance;
type MomentOf<T> = <<T as Trait>::Time as Time>::Moment;

/// Means for interacting with a specialized version of the `session` trait.
///
/// This is needed because `Staking` sets the `ValidatorIdOf` of the `pallet_session::Trait`
pub trait SessionInterface<AccountId>: frame_system::Trait {
    /// Disable a given validator by stash ID.
    ///
    /// Returns `true` if new era should be forced at the end of this session.
    /// This allows preventing a situation where there is too many validators
    /// disabled and block production stalls.
    fn disable_validator(validator: &AccountId) -> Result<bool, ()>;
    /// Get the validators from session.
    fn validators() -> Vec<AccountId>;
    /// Prune historical session tries up to but not including the given index.
    fn prune_historical_up_to(up_to: SessionIndex);
}

impl<T: Trait> SessionInterface<<T as frame_system::Trait>::AccountId> for T where
    T: pallet_session::Trait<ValidatorId = <T as frame_system::Trait>::AccountId>,
    T: pallet_session::historical::Trait<
        FullIdentification = Exposure<<T as frame_system::Trait>::AccountId, BalanceOf<T>>,
        FullIdentificationOf = ExposureOf<T>,
    >,
    T::SessionHandler: pallet_session::SessionHandler<<T as frame_system::Trait>::AccountId>,
    T::SessionManager: pallet_session::SessionManager<<T as frame_system::Trait>::AccountId>,
    T::ValidatorIdOf:
    Convert<<T as frame_system::Trait>::AccountId, Option<<T as frame_system::Trait>::AccountId>>,
{
    fn disable_validator(validator: &<T as frame_system::Trait>::AccountId) -> Result<bool, ()> {
        <pallet_session::Module<T>>::disable(validator)
    }

    fn validators() -> Vec<<T as frame_system::Trait>::AccountId> {
        <pallet_session::Module<T>>::validators()
    }

    fn prune_historical_up_to(up_to: SessionIndex) {
        <pallet_session::historical::Module<T>>::prune_up_to(up_to);
    }
}

pub trait SworkInterface: frame_system::Trait {
    fn update_identities();
}

impl<T: Trait> SworkInterface for T where T: swork::Trait {
    fn update_identities() {
        <swork::Module<T>>::update_identities();
    }
}

pub trait Trait: frame_system::Trait {
    /// The staking balance.
    type Currency: TransferrableCurrency<Self::AccountId, Moment = Self::BlockNumber>;

    /// Time used for computing era duration.
    type Time: Time;

    /// Convert a balance into a number used for election calculation.
    /// This must fit into a `u64` but is allowed to be sensibly lossy.
    /// TODO: [Substrate]substrate#1377
    /// The backward convert should be removed as the new Phragmen API returns ratio.
    /// The post-processing needs it but will be moved to off-chain. TODO: #2908
    type CurrencyToVote: Convert<BalanceOf<Self>, u64> + Convert<u128, BalanceOf<Self>>;

    /// Tokens have been minted and are unused for validator-reward.
    type RewardRemainder: OnUnbalanced<NegativeImbalanceOf<Self>>;

    /// The overarching event type.
    type Event: From<Event<Self>> + Into<<Self as frame_system::Trait>::Event>;

    /// Handler for the unbalanced reduction when slashing a staker.
    type Slash: OnUnbalanced<NegativeImbalanceOf<Self>>;

    /// Handler for the unbalanced increment when rewarding a staker.
    type Reward: OnUnbalanced<PositiveImbalanceOf<Self>>;

    /// Something that provides randomness in the runtime.
    type Randomness: Randomness<Self::Hash>;

    /// Number of sessions per era.
    type SessionsPerEra: Get<SessionIndex>;

    /// Number of eras that staked funds must remain bonded for.
    type BondingDuration: Get<EraIndex>;

    /// Number of eras that slashes are deferred by, after computation. This
    /// should be less than the bonding duration. Set to 0 if slashes should be
    /// applied immediately, without opportunity for intervention.
    type SlashDeferDuration: Get<EraIndex>;

    /// The origin which can cancel a deferred slash. Root can always do this.
    type SlashCancelOrigin: EnsureOrigin<Self::Origin>;

    /// Interface for interacting with a session module.
    type SessionInterface: self::SessionInterface<Self::AccountId>;

    /// Interface for interacting with a swork module
    type SworkInterface: self::SworkInterface;

    /// Storage power ratio for crust network phase 1
    type SPowerRatio: Get<u128>;
}

/// Mode of era-forcing.
#[derive(Copy, Clone, PartialEq, Eq, Encode, Decode, RuntimeDebug)]
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
    trait Store for Module<T: Trait> as Staking {
        /// Number of eras to keep in history.
        ///
        /// Information is kept for eras in `[current_era - history_depth; current_era]`.
        HistoryDepth get(fn history_depth) config(): u32 = 84;

        /// Map from all locked "stash" accounts to the controller account.
        pub Bonded get(fn bonded): map hasher(twox_64_concat) T::AccountId => Option<T::AccountId>;

        /// Map from all (unlocked) "controller" accounts to the info regarding the staking.
        pub Ledger get(fn ledger):
            map hasher(blake2_128_concat) T::AccountId
            => Option<StakingLedger<T::AccountId, BalanceOf<T>>>;

        /// Where the reward payment should be made. Keyed by stash.
        pub Payee get(fn payee): map hasher(twox_64_concat) T::AccountId => RewardDestination;

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

        /// Minimum number of staking participants before emergency conditions are imposed.
        pub MinimumValidatorCount get(fn minimum_validator_count) config():
            u32 = DEFAULT_MINIMUM_VALIDATOR_COUNT;

        /// Any validators that may never be slashed or forcibly kicked. It's a Vec since they're
        /// easy to initialize and the performance hit is minimal (we expect no more than four
        /// invulnerables) and restricted to testnets.
        pub Invulnerables get(fn invulnerables) config(): Vec<T::AccountId>;

        /// The currently elected validator set keyed by stash account ID.
        pub CurrentElected get(fn current_elected): Vec<T::AccountId>;

        /// The current era index.
        pub CurrentEra get(fn current_era): Option<EraIndex>;

        /// The start of the current era.
        pub CurrentEraStart get(fn current_era_start): MomentOf<T>;

        /// The session index at which the current era started.
        pub CurrentEraStartSessionIndex get(fn current_era_start_session_index): SessionIndex;

        /// Rewards for the current era. Using indices of current elected set.
        CurrentEraPointsEarned get(fn current_era_reward): EraPoints;

        /// True if the next session change will be a new era regardless of index.
        pub ForceEra get(fn force_era) config(): Forcing;

        /// The percentage of the slash that is distributed to reporters.
        ///
        /// The rest of the slashed value is handled by the `Slash`.
        pub SlashRewardFraction get(fn slash_reward_fraction) config(): Perbill;

        /// The amount of currency given to reporters of a slash event which was
        /// canceled by extraordinary circumstances (e.g. governance).
        pub CanceledSlashPayout get(fn canceled_payout) config(): BalanceOf<T>;

        /// All unapplied slashes that are queued for later.
        pub UnappliedSlashes:
            map hasher(twox_64_concat) EraIndex => Vec<UnappliedSlash<T::AccountId, BalanceOf<T>>>;

        /// A mapping from still-bonded eras to the first session index of that era.
        BondedEras: Vec<(EraIndex, SessionIndex)>;

        /// All slashing events on validators, mapped by era to the highest slash proportion
        /// and slash value of the era.
        ValidatorSlashInEra:
            double_map hasher(twox_64_concat) EraIndex, hasher(twox_64_concat) T::AccountId
            => Option<(Perbill, BalanceOf<T>)>;

        /// All slashing events on guarantors, mapped by era to the highest slash value of the era.
        GuarantorSlashInEra:
            double_map hasher(twox_64_concat) EraIndex, hasher(twox_64_concat) T::AccountId
            => Option<BalanceOf<T>>;

        /// Slashing spans for stash accounts.
        SlashingSpans: map hasher(twox_64_concat) T::AccountId => Option<slashing::SlashingSpans>;

        /// Records information about the maximum slash of a stash within a slashing span,
        /// as well as how much reward has been paid out.
        SpanSlash:
            map hasher(twox_64_concat) (T::AccountId, slashing::SpanIndex)
            => slashing::SpanRecord<BalanceOf<T>>;

        /// The earliest era for which we have a pending, unapplied slash.
        EarliestUnappliedSlash: Option<EraIndex>;

        /// The version of storage for upgrade.
        StorageVersion: u32;
    }
    add_extra_genesis {
        config(stakers):
            Vec<(T::AccountId, T::AccountId, BalanceOf<T>, StakerStatus<T::AccountId, BalanceOf<T>>)>;
        build(|config: &GenesisConfig<T>| {
            let mut gensis_total_stakes: BalanceOf<T> = Zero::zero();
            for &(ref stash, ref controller, balance, ref status) in &config.stakers {
                assert!(
                    T::Currency::transfer_balance(&stash) >= balance,
                    "Stash does not have enough balance to bond."
                );
                let _ = <Module<T>>::bond(
                    T::Origin::from(Some(stash.clone()).into()),
                    T::Lookup::unlookup(controller.clone()),
                    balance,
                    RewardDestination::Staked,
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
    pub enum Event<T> where Balance = BalanceOf<T>, <T as frame_system::Trait>::AccountId {
        /// All validators have been rewarded by the first balance; the second is the remainder
        /// from the maximum amount of reward.
        Reward(AccountId, Balance),
        /// One validator (and its guarantors) has been slashed by the given amount.
        Slash(AccountId, Balance),
        /// An old slashing report from a prior era was discarded because it could
        /// not be processed.
        OldSlashingReportDiscarded(SessionIndex),
        /// Total reward at each era
        EraReward(EraIndex, Balance, Balance),
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
        // TODO: add stake limitation change event
    }
);

decl_error! {
    /// Error for the staking module.
    pub enum Error for Module<T: Trait> {
        /// Not a controller account.
        NotController,
        /// Not a stash account.
        NotStash,
        /// Stash is already bonded.
        AlreadyBonded,
        /// Controller is already paired.
        AlreadyPaired,
        /// Duplicate index.
        DuplicateIndex,
        /// Slash record index out of bounds.
        InvalidSlashIndex,
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
    }
}

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        /// Number of sessions per era.
        const SessionsPerEra: SessionIndex = T::SessionsPerEra::get();

        /// Number of eras that staked funds must remain bonded for.
        const BondingDuration: EraIndex = T::BondingDuration::get();

        type Error = Error<T>;

        fn deposit_event() = default;

        fn on_finalize() {
            // Set the start of the first era.
            if !<CurrentEraStart<T>>::exists() {
                <CurrentEraStart<T>>::put(T::Time::now());
            }
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
        /// Base Weight: 67.87 µs
        /// DB Weight:
        /// - Read: Bonded, Ledger, [Origin Account], Current Era, Locks
        /// - Write: Bonded, Payee, [Origin Account], Ledger, Locks
        /// # </weight>
        #[weight = 67 * WEIGHT_PER_MICROS + T::DbWeight::get().reads_writes(4, 5)]
        fn bond(origin,
            controller: <T::Lookup as StaticLookup>::Source,
            #[compact] value: BalanceOf<T>,
            payee: RewardDestination
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
            <Payee<T>>::insert(&stash, payee);

            let current_era = CurrentEra::get().unwrap_or(0);
            let history_depth = Self::history_depth();
            let last_reward_era = current_era.saturating_sub(history_depth);

            let stash_balance = T::Currency::transfer_balance(&stash);
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
        /// Base Weight: 77 µs
        /// DB Weight:
        /// - Read: Bonded, Ledger, [Origin Account], Locks
        /// - Write: [Origin Account], Locks, Ledger
        /// # </weight>
        #[weight = 77 * WEIGHT_PER_MICROS + T::DbWeight::get().reads_writes(6, 3)]
        fn bond_extra(origin, #[compact] max_additional: BalanceOf<T>) {
            let stash = ensure_signed(origin)?;

			let controller = Self::bonded(&stash).ok_or(Error::<T>::NotStash)?;
			let mut ledger = Self::ledger(&controller).ok_or(Error::<T>::NotController)?;

			let mut extra = T::Currency::transfer_balance(&stash);

			if extra > Zero::zero() {
				extra = extra.min(max_additional);
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
        /// Base Weight: 50.66 µs
        /// DB Weight:
        /// - Read: Ledger, Current Era, Locks, [Origin Account]
        /// - Write: [Origin Account], Locks, Ledger
        /// </weight>
        #[weight = 50 * WEIGHT_PER_MICROS + T::DbWeight::get().reads_writes(5, 3)]
        fn unbond(origin, #[compact] value: BalanceOf<T>) {
			let controller = ensure_signed(origin)?;
			let mut ledger = Self::ledger(&controller).ok_or(Error::<T>::NotController)?;
			ensure!(
				ledger.unlocking.len() < MAX_UNLOCKING_CHUNKS,
				Error::<T>::NoMoreChunks,
			);

			let mut value = value.min(ledger.active);

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
        /// Base Weight:
        /// Update: 50.52 + .028 * S µs
        /// - Reads: EraElectionStatus, Ledger, Current Era, Locks, [Origin Account]
        /// - Writes: [Origin Account], Locks, Ledger
        /// Kill: 79.41 + 2.366 * S µs
        /// - Reads: EraElectionStatus, Ledger, Current Era, Bonded, [Origin Account], Locks
        /// - Writes: Bonded, Slashing Spans (if S > 0), Ledger, Payee, Validators, Guarantors, [Origin Account], Locks
        /// - Writes Each: SpanSlash * S
        /// NOTE: Weight annotation is the kill scenario, we refund otherwise.
        /// # </weight>
        #[weight = T::DbWeight::get().reads_writes(6, 6)
            .saturating_add(80 * WEIGHT_PER_MICROS)
        ]
        fn withdraw_unbonded(origin) {
            let controller = ensure_signed(origin)?;
            let mut ledger = Self::ledger(&controller).ok_or(Error::<T>::NotController)?;
			let (stash, old_total) = (ledger.stash.clone(), ledger.total);
            if let Some(current_era) = Self::current_era() {
                ledger = ledger.consolidate_unlocked(current_era)
            }

            if ledger.unlocking.is_empty() && ledger.active.is_zero() {
                // This account must have called `unbond()` with some value that caused the active
                // portion to fall below existential deposit + will have no more unlocking chunks
                // left. We can now safely remove all staking-related information.
                Self::kill_stash(&stash)?;
                // remove the lock.
                T::Currency::remove_lock(STAKING_ID, &stash);
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
        /// Base Weight: 27.8 µs
        /// DB Weight:
        /// - Read: Ledger, StakeLimit
        /// - Write: Guarantors, Validators
        /// # </weight>
        #[weight = 27 * WEIGHT_PER_MICROS + T::DbWeight::get().reads_writes(4, 1)]
        fn validate(origin, prefs: ValidatorPrefs) {
            let controller = ensure_signed(origin)?;
            let ledger = Self::ledger(&controller).ok_or(Error::<T>::NotController)?;
            let v_stash = &ledger.stash;
            <Guarantors<T>>::remove(v_stash);
			<Validators<T>>::insert(v_stash, prefs);
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
        /// Base Weight: 1260 µs (For 100 validators and for each contains 10 guarantors)
        /// DB Weight:
        /// - Reads: Guarantors, Ledger, Current Era
        /// - Writes: Guarantors
        /// # </weight>
        // TODO: reconsider this weight value for the V_G Graph
        #[weight = 1260 * WEIGHT_PER_MICROS + T::DbWeight::get().reads_writes(8, 4)]
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
        /// Base Weight: 1324 µs (For 100 validators and for each contains 10 guarantors)
        /// DB Weight:
        /// - Reads: Guarantors, Ledger, Current Era
        /// - Writes: Validators, Guarantors
        /// # </weight>
        #[weight = 1324 * WEIGHT_PER_MICROS + T::DbWeight::get().reads_writes(5, 4)]
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
        /// Base Weight: 22.12 µs
        /// DB Weight:
        /// - Read: Ledger
        /// - Write: Validators, Guarantors
        /// # </weight>
        #[weight = 22 * WEIGHT_PER_MICROS + T::DbWeight::get().reads_writes(3, 1)]
        fn chill(origin) {
            let controller = ensure_signed(origin)?;
            let ledger = Self::ledger(&controller).ok_or(Error::<T>::NotController)?;
            Self::chill_stash(&ledger.stash);
        }

        /// (Re-)set the payment target for a controller.
        ///
        /// Effects will be felt at the beginning of the next era.
        ///
        /// The dispatch origin for this call must be _Signed_ by the controller, not the stash.
        ///
        /// # <weight>
        /// - Independent of the arguments. Insignificant complexity.
        /// - Contains a limited number of reads.
        /// - Writes are limited to the `origin` account key.
        /// ---------
        /// - Base Weight: 11.33 µs
        /// - DB Weight:
        ///     - Read: Ledger
        ///     - Write: Payee
        /// # </weight>
        #[weight = 11 * WEIGHT_PER_MICROS + T::DbWeight::get().reads_writes(1, 1)]
        fn set_payee(origin, payee: RewardDestination) {
            let controller = ensure_signed(origin)?;
            let ledger = Self::ledger(&controller).ok_or(Error::<T>::NotController)?;
            let stash = &ledger.stash;
            <Payee<T>>::insert(stash, payee);
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
        /// Base Weight: 36.2 µs
        /// DB Weight:
        /// - Read: Bonded, Ledger New Controller, Ledger Old Controller
        /// - Write: Bonded, Ledger New Controller, Ledger Old Controller
        /// # </weight>
        #[weight = 36 * WEIGHT_PER_MICROS + T::DbWeight::get().reads_writes(3, 3)]
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

        // ----- Root Calls ------

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

        /// Force there to be no new eras indefinitely.
        ///
        /// The dispatch origin must be Root.
        ///
        /// # <weight>
        /// - No arguments.
        /// - Base Weight: 1.857 µs
        /// - Write: ForceEra
        /// # </weight>
        #[weight = 2 * WEIGHT_PER_MICROS + T::DbWeight::get().writes(1)]
        fn force_no_eras(origin) {
            ensure_root(origin)?;
            ForceEra::put(Forcing::ForceNone);
        }

        /// Force there to be a new era at the end of the next session. After this, it will be
        /// reset to normal (non-forced) behaviour.
        ///
        /// The dispatch origin must be Root.
        ///
        /// # <weight>
        /// - No arguments.
        /// - Base Weight: 1.959 µs
        /// - Write ForceEra
        /// # </weight>
        #[weight = 2 * WEIGHT_PER_MICROS + T::DbWeight::get().writes(1)]
        fn force_new_era(origin) {
            ensure_root(origin)?;
            ForceEra::put(Forcing::ForceNew);
        }

        /// Set the validators who cannot be slashed (if any).
        ///
        /// The dispatch origin must be Root.
        ///
        /// # <weight>
        /// - O(V)
        /// - Base Weight: 2.208 + .006 * V µs
        /// - Write: Invulnerables
        /// # </weight>
        #[weight = T::DbWeight::get().writes(1)
            .saturating_add(2 * WEIGHT_PER_MICROS)
            .saturating_add((6 * WEIGHT_PER_NANOS).saturating_mul(validators.len() as Weight))
        ]
        fn set_invulnerables(origin, validators: Vec<T::AccountId>) {
            ensure_root(origin)?;
            <Invulnerables<T>>::put(validators);
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

        /// Force there to be a new era at the end of sessions indefinitely.
        ///
        /// The dispatch origin must be Root.
        ///
        /// # <weight>
        /// - Base Weight: 2.05 µs
        /// - Write: ForceEra
        /// # </weight>
        #[weight = 2 * WEIGHT_PER_MICROS + T::DbWeight::get().writes(1)]
        fn force_new_era_always(origin) {
            ensure_root(origin)?;
            ForceEra::put(Forcing::ForceAlways);
        }

        /// Cancel enactment of a deferred slash.
        ///
        /// Can be called by the `T::SlashCancelOrigin`.
        ///
        /// Parameters: era and indices of the slashes for that era to kill.
        ///
        /// # <weight>
        /// Complexity: O(U + S)
        /// with U unapplied slashes weighted with U=1000
        /// and S is the number of slash indices to be canceled.
        /// - Base: 5870 + 34.61 * S µs
        /// - Read: Unapplied Slashes
        /// - Write: Unapplied Slashes
        /// # </weight>
        #[weight = T::DbWeight::get().reads_writes(1, 1)
            .saturating_add(5_870 * WEIGHT_PER_MICROS)
            .saturating_add((35 * WEIGHT_PER_MICROS).saturating_mul(slash_indices.len() as Weight))
        ]
        fn cancel_deferred_slash(origin, era: EraIndex, slash_indices: Vec<u32>) {
            T::SlashCancelOrigin::try_origin(origin)
                .map(|_| ())
                .or_else(ensure_root)?;

            let mut slash_indices = slash_indices;
            slash_indices.sort_unstable();
            let mut unapplied = <Self as Store>::UnappliedSlashes::get(&era);

            for (removed, index) in slash_indices.into_iter().enumerate() {
                let index = index as usize;

                // if `index` is not duplicate, `removed` must be <= index.
                ensure!(removed <= index, Error::<T>::DuplicateIndex);

                // all prior removals were from before this index, since the
                // list is sorted.
                let index = index - removed;
                ensure!(index < unapplied.len(), Error::<T>::InvalidSlashIndex);

                unapplied.remove(index);
            }

            <Self as Store>::UnappliedSlashes::insert(&era, &unapplied);
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
        /// - Writes: Bonded, Ledger, Payee, Validators, Nominators, Stash Account, Locks
        /// # </weight>
        #[weight = T::DbWeight::get().reads_writes(4, 7)
            .saturating_add(76 * WEIGHT_PER_MICROS)]
        fn reap_stash(_origin, stash: T::AccountId) {
            ensure!(T::Currency::total_balance(&stash).is_zero(), Error::<T>::FundedTarget);
            Self::kill_stash(&stash)?;
            T::Currency::remove_lock(STAKING_ID, &stash);
        }

        /// Pay out all the stakers behind a single validator for a single era.
        ///
        /// - `validator_stash` is the stash account of the validator. Their nominators, up to
        ///   `T::MaxNominatorRewardedPerValidator`, will also receive their rewards.
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
    }
}

impl<T: Trait> Module<T> {
    // PUBLIC IMMUTABLES

    /// The total balance that can be slashed from a stash account as of right now.
    pub fn slashable_balance_of(stash: &T::AccountId) -> BalanceOf<T> {
        Self::bonded(stash)
            .and_then(Self::ledger)
            .map(|l| l.active)
            .unwrap_or_default()
    }

    // PRIVATE IMMUTABLES

    /// Calculate the stake limit by storage workloads, returns the stake limit value
    ///
    /// # <weight>
    /// - Independent of the arguments. Insignificant complexity.
    /// - O(1).
    /// - 0 DB entry.
    /// # </weight>
    fn stake_limit_of(own_workloads: u128, _: u128) -> BalanceOf<T> {
        // TODO: Stake limit calculation, this should be enable and adjust in olympus phase.
        /*let total_issuance = TryInto::<u128>::try_into(T::Currency::total_issuance())
            .ok()
            .unwrap();

        // total_workloads cannot be zero, or system go panic!
        if total_workloads == 0 {
            Zero::zero()
        } else {
            let workloads_to_stakes = (( own_workloads.wrapping_mul(total_issuance) / total_workloads / 2) as u128)
                .min(u64::max_value() as u128);

            workloads_to_stakes.try_into().ok().unwrap()
        }*/

        // Now, we apply directly mapping algorithm for the early stage:
        // 1. Maxwell 1.0: 1 terabytes -> 80,000 CRUs
        // 2. Olympus 1.0: 1 terabytes -> 30 CRUs(tmp)
        // ps: we treat 1 terabytes as 1_000_000_000_000 for make `mapping_ratio = 1`
        if let Some(storage_stakes) = own_workloads.checked_mul(T::SPowerRatio::get()) {
            storage_stakes.try_into().ok().unwrap()
        } else {
            (u64::max_value() as u128).try_into().ok().unwrap()
        }
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

    // PRIVATE MUTABLE (DANGEROUS)

    /// Insert new or update old stake limit
    fn upsert_stake_limit(account_id: &T::AccountId, limit: BalanceOf<T>) {
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
        let era_staking_payout = <ErasStakingPayout<T>>::get(&era)
            .ok_or_else(|| Error::<T>::InvalidEraToReward)?;

        let controller = Self::bonded(&validator_stash).ok_or(Error::<T>::NotStash)?;
        let mut ledger = <Ledger<T>>::get(&controller).ok_or_else(|| Error::<T>::NotController)?;

        ledger.claimed_rewards.retain(|&x| x >= current_era.saturating_sub(history_depth));
        match ledger.claimed_rewards.binary_search(&era) {
            Ok(_) => Err(Error::<T>::AlreadyClaimed)?,
            Err(pos) => ledger.claimed_rewards.insert(pos, era),
        }
        /* Input data seems good, no errors allowed after this point */
        let exposure = <ErasStakers<T>>::get(&era, &ledger.stash);
        <Ledger<T>>::insert(&controller, &ledger);

        // 2. Pay authoring reward
        let mut validator_imbalance = <PositiveImbalanceOf<T>>::zero();
        if let Some(value) = <ErasAuthoringPayout<T>>::get(&era, &validator_stash) {
            validator_imbalance.maybe_subsume(Self::make_payout(&validator_stash, value));
        }

        let to_num =
        |b: BalanceOf<T>| <T::CurrencyToVote as Convert<BalanceOf<T>, u64>>::convert(b);

        // 3. Retrieve total stakes and total staking reward
        let era_total_stakes = <ErasTotalStakes<T>>::get(&era);
        let staking_reward = Perbill::from_rational_approximation(to_num(exposure.total), to_num(era_total_stakes)) * era_staking_payout;
        let total = exposure.total.max(One::one());
        // 4. Calculate total rewards for staking
        let total_rewards = <ErasValidatorPrefs<T>>::get(&era, &ledger.stash).fee * staking_reward;
        let mut guarantee_rewards = Zero::zero();
        // 5. Pay staking reward to guarantors
        for i in &exposure.others {
            let reward_ratio = Perbill::from_rational_approximation(i.value, total);
            // Reward guarantors
            guarantee_rewards += reward_ratio * total_rewards;
            if let Some(imbalance) = Self::make_payout(
                &i.who,
                reward_ratio * total_rewards
            ) {
                Self::deposit_event(RawEvent::Reward(i.who.clone(), imbalance.peek()));
            };
        }
        // 6. Pay staking reward to validator
        validator_imbalance.maybe_subsume(Self::make_payout(&ledger.stash, staking_reward - guarantee_rewards));
        Self::deposit_event(RawEvent::Reward(ledger.stash, validator_imbalance.peek()));
        Ok(())
    }

    /// Session has just ended. Provide the validator set for the next session if it's an era-end, along
    /// with the exposure of the prior validator set.
    fn new_session(
        session_index: SessionIndex,
    ) -> Option<Vec<T::AccountId>> {
        if Self::current_era().is_some() {
            let era_length = session_index
                .checked_sub(Self::current_era_start_session_index())
                .unwrap_or(0);
            // TODO: remove ForceNew? cause this will make work report update invalid
            match ForceEra::get() {
                Forcing::ForceNew => ForceEra::kill(),
                Forcing::ForceAlways => (),
                Forcing::NotForcing if era_length >= T::SessionsPerEra::get() => (),
                _ => return None,
            }
            // New era
            Self::new_era(session_index)
        } else {
            // Set initial era
            Self::new_era(session_index)
        }
    }

    /// End a session potentially ending an era.
    fn end_session(session_index: SessionIndex) {
        if Self::current_era().is_some() {
            let era_length = session_index
                .checked_sub(Self::current_era_start_session_index())
                .unwrap_or(0);
            // End of era
            if era_length == T::SessionsPerEra::get() - 1 {
                Self::end_era();
            }
        }
    }

    /// The era has changed - enact new staking set.
    ///
    /// NOTE: This always happens immediately before a session change to ensure that new validators
    /// get a chance to set their session keys.
    /// This also checks stake limitation based on work reports
    fn new_era(start_session_index: SessionIndex) -> Option<Vec<T::AccountId>> {
        // Increment current era.
        let current_era = CurrentEra::mutate(|s| {
            *s = Some(s.map(|s| s + 1).unwrap_or(0));
            s.unwrap()
        });

        CurrentEraStartSessionIndex::mutate(|v| {
            *v = start_session_index;
        });
        let bonding_duration = T::BondingDuration::get();

        // Clean old era information.
        // TODO: double check the economic mechanism.
        if let Some(old_era) = current_era.checked_sub(Self::history_depth() + 1) {
            Self::clear_era_information(old_era);
        }

        // Slashing
        // TODO: put slashing into start_era(judging at start_session) like Kusama does?
        BondedEras::mutate(|bonded| {
            bonded.push((current_era, start_session_index));

            if current_era > bonding_duration {
                let first_kept = current_era - bonding_duration;

                // prune out everything that's from before the first-kept index.
                let n_to_prune = bonded
                    .iter()
                    .take_while(|&&(era_idx, _)| era_idx < first_kept)
                    .count();

                // kill slashing metadata.
                for (pruned_era, _) in bonded.drain(..n_to_prune) {
                    slashing::clear_era_metadata::<T>(pruned_era);
                }

                if let Some(&(_, first_session)) = bonded.first() {
                    T::SessionInterface::prune_historical_up_to(first_session);
                }
            }
        });

        // Reassign all stakers.
        let maybe_new_validators = Self::select_and_update_validators(current_era);
        Self::apply_unapplied_slashes(current_era);

        maybe_new_validators
    }

    /// Compute payout for era.
    fn end_era() {
        // Payout
        let now = T::Time::now();
        let previous_era_start = <CurrentEraStart<T>>::mutate(|v| sp_std::mem::replace(v, now));
        let era_duration = now - previous_era_start;
        if !era_duration.is_zero() {
            let points = CurrentEraPointsEarned::take();
            let validators = Self::current_elected();
            let total_authoring_payout = Self::authoring_rewards_in_era();
            // let mut total_imbalance = <PositiveImbalanceOf<T>>::zero();
            let current_era = Self::current_era().unwrap_or(0);
            // 1. Block authoring payout
            for (v, p) in validators.iter().zip(points.individual.into_iter()) {
                if p != 0 {
                    let authoring_reward =
                        Perbill::from_rational_approximation(p, points.total) * total_authoring_payout;
                    <ErasAuthoringPayout<T>>::insert(&current_era, v, authoring_reward);
                }
            }

            // 2. Staking payout
            let total_staking_payout = Self::staking_rewards_in_era(current_era);
            <ErasStakingPayout<T>>::insert(&current_era, total_staking_payout);

            // 3. Deposit era reward event
            Self::deposit_event(RawEvent::EraReward(current_era, total_authoring_payout, total_staking_payout));

            // TODO: enable treasury and might bring this back
            // T::Reward::on_unbalanced(total_imbalance);
            // This is not been used
            // T::RewardRemainder::on_unbalanced(T::Currency::issue(rest));
        }
    }

        /// Clear all era information for given era.
    fn clear_era_information(era_index: EraIndex) {
        <ErasStakers<T>>::remove_prefix(era_index);
        <ErasValidatorPrefs<T>>::remove_prefix(era_index);
        <ErasStakingPayout<T>>::remove(era_index);
        <ErasTotalStakes<T>>::remove(era_index);
        <ErasAuthoringPayout<T>>::remove_prefix(era_index);
    }

    /// Block authoring rewards per era, this won't be changed in every era
    fn authoring_rewards_in_era() -> BalanceOf<T> {
        // Milliseconds per year for the Julian year (365.25 days).
        const MILLISECONDS_PER_YEAR: u64 = 1000 * 3600 * 24 * 36525 / 100;
        // Initial with total rewards per year
        let year_in_eras = MILLISECONDS_PER_YEAR / MILLISECS_PER_BLOCK / (EPOCH_DURATION_IN_BLOCKS * T::SessionsPerEra::get()) as u64;

        let reward_this_era = BLOCK_AUTHORING_REWARDS / year_in_eras as u128;

        reward_this_era.try_into().ok().unwrap()
    }

    /// Staking rewards per era
    fn staking_rewards_in_era(current_era: EraIndex) -> BalanceOf<T> {
        let mut maybe_rewards_this_year = FIRST_YEAR_REWARDS ;
        let total_issuance = TryInto::<u128>::try_into(T::Currency::total_issuance())
            .ok()
            .unwrap();

        // Milliseconds per year for the Julian year (365.25 days).
        // TODO: add era duration to calculate each era's rewards
        const MILLISECONDS_PER_YEAR: u64 = 1000 * 3600 * 24 * 36525 / 100;
        // 1 Julian year = (365.25d * 24h * 3600s * 1000ms) / (millisecs_in_era = block_time * blocks_num_in_era)
        let year_in_eras = MILLISECONDS_PER_YEAR / MILLISECS_PER_BLOCK / (EPOCH_DURATION_IN_BLOCKS * T::SessionsPerEra::get()) as u64;
        let year_num = current_era as u64 / year_in_eras;
        for _ in 0..year_num {
            // If inflation <= 1%, stop reduce
            if maybe_rewards_this_year <= total_issuance / 100 {
                maybe_rewards_this_year = total_issuance / 100;
                break;
            }

            maybe_rewards_this_year = maybe_rewards_this_year * 4 / 5;
        }

        let reward_this_era = maybe_rewards_this_year / year_in_eras as u128;

        reward_this_era.try_into().ok().unwrap()
    }

    /// Apply previously-unapplied slashes on the beginning of a new era, after a delay.
    fn apply_unapplied_slashes(current_era: EraIndex) {
        let slash_defer_duration = T::SlashDeferDuration::get();
        <Self as Store>::EarliestUnappliedSlash::mutate(|earliest| {
            if let Some(ref mut earliest) = earliest {
                let keep_from = current_era.saturating_sub(slash_defer_duration);
                for era in (*earliest)..keep_from {
                    let era_slashes = <Self as Store>::UnappliedSlashes::take(&era);
                    for slash in era_slashes {
                        slashing::apply_slash::<T>(slash);
                    }
                }

                *earliest = (*earliest).max(keep_from)
            }
        })
    }

    /// Select the new validator set at the end of the era.
    ///
    /// Returns the a set of newly selected _stash_ IDs.
    ///
    /// This should only be called at the end of an era.
    fn select_and_update_validators(current_era: EraIndex) -> Option<Vec<T::AccountId>> {
        // I. Update all swork identities work report and clear stakers
        // TODO: this actually should already be prepared in the swork module
        T::SworkInterface::update_identities();

        // II. Ensure minimum validator count
        let validator_count = <Validators<T>>::iter().count();
        let minimum_validator_count = Self::minimum_validator_count().max(1) as usize;

        if validator_count < minimum_validator_count {
            // There were not enough validators for even our minimal level of functionality.
            // This is bad🥺.
            // We should probably disable all functionality except for block production
            // and let the chain keep producing blocks until we can decide on a sufficiently
            // substantial set.
            // TODO: [Substrate]substrate#2494
            return None
        }

        let to_votes =
            |b: BalanceOf<T>| <T::CurrencyToVote as Convert<BalanceOf<T>, u64>>::convert(b) as u128;
        let to_balance = |e: u128| <T::CurrencyToVote as Convert<u128, BalanceOf<T>>>::convert(e);

        // III. Construct and fill in the V/G graph
        // TC is O(V + G*1), V means validator's number, G means guarantor's number
        // DB try is 2
        let mut vg_graph: BTreeMap<T::AccountId, Vec<IndividualExposure<T::AccountId, BalanceOf<T>>>> =
            <Validators<T>>::iter().map(|(v_stash, _)|
                (v_stash, Vec::<IndividualExposure<T::AccountId, BalanceOf<T>>>::new())
            ).collect();
        for (guarantor, guarantee) in <Guarantors<T>>::iter() {
            let Guarantee { total: _, submitted_in, mut targets, suppressed: _ } = guarantee;

            // Filter out guarantee targets which were guaranteed before the most recent
            // slashing span.
            targets.retain(|ie| {
                <Self as Store>::SlashingSpans::get(&ie.who).map_or(
                    true,
                    |spans| submitted_in >= spans.last_nonzero_slash(),
                )
            });
            
            for target in targets {
                if let Some(g) = vg_graph.get_mut(&target.who) {
                     g.push(IndividualExposure {
                         who: guarantor.clone(),
                         value: target.value
                     });
                }
            }
        }

        // IV. This part will cover
        // 1. Get `ErasStakers` with `stake_limit` and `vg_graph`
        // 2. Get `ErasValidatorPrefs`
        // 3. Get `total_valid_stakes`
        // 4. Fill in `validator_stakes`
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
            <ErasValidatorPrefs<T>>::insert(&current_era, &v_stash, Self::validators(&v_stash).clone());
            if let Some(maybe_total_stakes) = eras_total_stakes.checked_add(&new_exposure.total) {
                eras_total_stakes = maybe_total_stakes;
            } else {
                eras_total_stakes = to_balance(u64::max_value() as u128);
            }

            // 5. Push validator stakes
            validators_stakes.push((v_stash.clone(), to_votes(new_exposure.total)))
        }

        // V. TopDown Election Algorithm with Randomlization
        let to_elect = (Self::validator_count() as usize).min(validators_stakes.len());

        // 2. If there's no validators, be as same as little validators
        if to_elect < minimum_validator_count {
            return None;
        }

        let elected_stashes= Self::do_election(validators_stakes, to_elect);
        // VI. Update general staking storage
        // Set the new validator set in sessions.
        <CurrentElected<T>>::put(&elected_stashes);

        // Update slot stake.
        <ErasTotalStakes<T>>::insert(&current_era, eras_total_stakes);

        // In order to keep the property required by `n_session_ending`
        // that we must return the new validator set even if it's the same as the old,
        // as long as any underlying economic conditions have changed, we don't attempt
        // to do any optimization where we compare against the prior set.
        Some(elected_stashes)
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

        // TODO: this may update with `num_slashing_spans`?
        slashing::clear_stash_metadata::<T>(stash);

        Ok(())
    }

    /// This function will update the stash's stake limit
    fn update_stake_limit(controller: &T::AccountId, own_workloads: u128, total_workloads: u128) {
        if let Some(ledger) = Self::ledger(&controller) {
            Self::upsert_stake_limit(
                &ledger.stash,
                Self::stake_limit_of(own_workloads, total_workloads),
            );
        }
    }

    /// Add reward points to validators using their stash account ID.
    ///
    /// Validators are keyed by stash account ID and must be in the current elected set.
    ///
    /// For each element in the iterator the given number of points in u32 is added to the
    /// validator, thus duplicates are handled.
    ///
    /// At the end of the era each the total payout will be distributed among validator
    /// relatively to their points.
    ///
    /// COMPLEXITY: Complexity is `number_of_validator_to_reward x current_elected_len`.
    /// If you need to reward lots of validator consider using `reward_by_indices`.
    pub fn reward_by_ids(validators_points: impl IntoIterator<Item = (T::AccountId, u32)>) {
        CurrentEraPointsEarned::mutate(|rewards| {
            let current_elected = <Module<T>>::current_elected();
            for (validator, points) in validators_points.into_iter() {
                if let Some(index) = current_elected
                    .iter()
                    .position(|elected| *elected == validator)
                {
                    rewards.add_points_to_index(index as u32, points);
                }
            }
        });
    }

    /// Add reward points to validators using their validator index.
    ///
    /// For each element in the iterator the given number of points in u32 is added to the
    /// validator, thus duplicates are handled.
    pub fn reward_by_indices(validators_points: impl IntoIterator<Item = (u32, u32)>) {
        let current_elected_len = <Module<T>>::current_elected().len() as u32;

        CurrentEraPointsEarned::mutate(|rewards| {
            for (validator_index, points) in validators_points.into_iter() {
                if validator_index < current_elected_len {
                    rewards.add_points_to_index(validator_index, points);
                }
            }
        });
    }

    /// Ensures that at the end of the current session there will be a new era.
    fn ensure_new_era() {
        match ForceEra::get() {
            Forcing::ForceAlways | Forcing::ForceNew => (),
            _ => ForceEra::put(Forcing::ForceNew),
        }
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
        let mut candidate_stashes = validators_stakes[0..candidate_to_elect]
        .iter()
        .map(|(who, stakes)| (who.clone(), *stakes))
        .collect::<Vec<(T::AccountId, u128)>>();

        // shuffle it
        Self::shuffle_candidates(&mut candidate_stashes);

        // choose elected_stashes number of validators
        let elected_stashes = candidate_stashes[0..to_elect]
        .iter()
        .map(|(who, _stakes)| who.clone())
        .collect::<Vec<T::AccountId>>();
        elected_stashes
    }

    fn shuffle_candidates(candidates_stakes: &mut Vec<(T::AccountId, u128)>) {
        // 1. Construct random seed, 👼 bless the randomness
        // seed = [ block_hash, phrase ]
        let phrase = b"candidates_shuffle";
        let bn = <frame_system::Module<T>>::block_number();
        let bh: T::Hash = <frame_system::Module<T>>::block_hash(bn);
        let seed = [
            &bh.as_ref()[..],
            &phrase.encode()[..]
        ].concat();

        // we'll need a random seed here.
        let seed = T::Randomness::random(seed.as_slice());
        // seed needs to be guaranteed to be 32 bytes.
        let seed = <[u8; 32]>::decode(&mut TrailingZeroInput::new(seed.as_ref()))
            .expect("input is padded with zeroes; qed");
        let mut rng = ChaChaRng::from_seed(seed);
        for i in (0..candidates_stakes.len()).rev() {
            let random_index = (rng.next_u32() % (i as u32 + 1)) as usize;
            candidates_stakes.swap(random_index, i);
        }
    }
}

/// In this implementation `new_session(session)` must be called before `end_session(session-1)`
/// i.e. the new session must be planned before the ending of the previous session.
///
/// Once the first new_session is planned, all session must start and then end in order, though
/// some session can lag in between the newest session planned and the latest session started.
impl<T: Trait> pallet_session::SessionManager<T::AccountId> for Module<T> {
    fn new_session(new_index: SessionIndex) -> Option<Vec<T::AccountId>> {
        let mut idx = new_index;
        if idx > 0 {
            idx -= 1;
        }
        Self::new_session(idx)
    }
    fn end_session(end_index: SessionIndex) {
        // Do nothing
        Self::end_session(end_index);
    }
    fn start_session(_start_index: SessionIndex) {
        // Do nothing
    }
}

impl<T: Trait> historical::SessionManager<T::AccountId, Exposure<T::AccountId, BalanceOf<T>>> for Module<T> {
    fn new_session(new_index: SessionIndex)
                   -> Option<Vec<(T::AccountId, Exposure<T::AccountId, BalanceOf<T>>)>>
    {
        <Self as pallet_session::SessionManager<_>>::new_session(new_index).map(|validators| {
            let current_era = Self::current_era()
                // Must be some as a new era has been created.
                .unwrap_or(0);

            validators.into_iter().map(|v| {
                let exposure = Self::eras_stakers(current_era, &v);
                (v, exposure)
            }).collect()
        })
    }
    fn start_session(start_index: SessionIndex) {
        <Self as pallet_session::SessionManager<_>>::start_session(start_index)
    }
    fn end_session(end_index: SessionIndex) {
        <Self as pallet_session::SessionManager<_>>::end_session(end_index)
    }
}

impl<T: Trait> swork::Works<T::AccountId> for Module<T> {
    fn report_works(controller: &T::AccountId, own_workload: u128, total_workload: u128) {
        Self::update_stake_limit(controller, own_workload, total_workload);
    }
}

/// Add reward points to block authors:
/// * 20 points to the block producer for producing a (non-uncle) block in the relay chain,
/// * 2 points to the block producer for each reference to a previously unreferenced uncle, and
/// * 1 point to the producer of each referenced uncle block.
impl<T: Trait + pallet_authorship::Trait>
    pallet_authorship::EventHandler<T::AccountId, T::BlockNumber> for Module<T>
{
    fn note_author(author: T::AccountId) {
        Self::reward_by_ids(vec![(author, 20)]);
    }
    fn note_uncle(author: T::AccountId, _age: T::BlockNumber) {
        Self::reward_by_ids(vec![
            (<pallet_authorship::Module<T>>::author(), 2),
            (author, 1),
        ])
    }
}

/// A `Convert` implementation that finds the stash of the given controller account,
/// if any.
pub struct StashOf<T>(sp_std::marker::PhantomData<T>);

impl<T: Trait> Convert<T::AccountId, Option<T::AccountId>> for StashOf<T> {
    fn convert(controller: T::AccountId) -> Option<T::AccountId> {
        <Module<T>>::ledger(&controller).map(|l| l.stash)
    }
}

/// A typed conversion from stash account ID to the current exposure of guarantors
/// on that account.
pub struct ExposureOf<T>(sp_std::marker::PhantomData<T>);

impl<T: Trait> Convert<T::AccountId, Option<Exposure<T::AccountId, BalanceOf<T>>>>
    for ExposureOf<T>
{
    fn convert(validator: T::AccountId) -> Option<Exposure<T::AccountId, BalanceOf<T>>> {
        if let Some(current_era) = <Module<T>>::current_era() {
            Some(<Module<T>>::eras_stakers(current_era, &validator))
        } else {
            None
        }
    }
}

/// This is intended to be used with `FilterHistoricalOffences`.
impl <T: Trait> OnOffenceHandler<T::AccountId, pallet_session::historical::IdentificationTuple<T>, Weight> for Module<T> where
    T: pallet_session::Trait<ValidatorId = <T as frame_system::Trait>::AccountId>,
    T: pallet_session::historical::Trait<
        FullIdentification = Exposure<<T as frame_system::Trait>::AccountId, BalanceOf<T>>,
        FullIdentificationOf = ExposureOf<T>,
    >,
    T::SessionHandler: pallet_session::SessionHandler<<T as frame_system::Trait>::AccountId>,
    T::SessionManager: pallet_session::SessionManager<<T as frame_system::Trait>::AccountId>,
    T::ValidatorIdOf: Convert<<T as frame_system::Trait>::AccountId, Option<<T as frame_system::Trait>::AccountId>>
{
    fn on_offence(
        offenders: &[OffenceDetails<
            T::AccountId,
            pallet_session::historical::IdentificationTuple<T>,
        >],
        slash_fraction: &[Perbill],
        slash_session: SessionIndex,
    ) -> Result<Weight, ()> {
        let reward_proportion = SlashRewardFraction::get();

        let era_now = Self::current_era().unwrap_or(0);
        let window_start = era_now.saturating_sub(T::BondingDuration::get());
        let current_era_start_session = CurrentEraStartSessionIndex::get();
        // TODO: calculate with db weights
        let consumed_weight: Weight = 0;

        // fast path for current-era report - most likely.
        let slash_era = if slash_session >= current_era_start_session {
            era_now
        } else {
            let eras = BondedEras::get();

            // reverse because it's more likely to find reports from recent eras.

            match eras
                .iter()
                .rev()
                .filter(|&&(_, ref sesh)| sesh <= &slash_session)
                .next()
            {
                None => return Ok(consumed_weight), // before bonding period. defensive - should be filtered out.
                Some(&(ref slash_era, _)) => *slash_era,
            }
        };

        <Self as Store>::EarliestUnappliedSlash::mutate(|earliest| {
            if earliest.is_none() {
                *earliest = Some(era_now)
            }
        });

        let slash_defer_duration = T::SlashDeferDuration::get();

        for (details, slash_fraction) in offenders.iter().zip(slash_fraction) {
            let stash = &details.offender.0;
            let exposure = &details.offender.1;

            // Skip if the validator is invulnerable.
            if Self::invulnerables().contains(stash) {
                continue;
            }

            let unapplied = slashing::compute_slash::<T>(slashing::SlashParams {
                stash,
                slash: *slash_fraction,
                exposure,
                slash_era,
                window_start,
                now: era_now,
                reward_proportion,
            });

            if let Some(mut unapplied) = unapplied {
                unapplied.reporters = details.reporters.clone();
                if slash_defer_duration == 0 {
                    // apply right away.
                    slashing::apply_slash::<T>(unapplied);
                } else {
                    // defer to end of some `slash_defer_duration` from now.
                    <Self as Store>::UnappliedSlashes::mutate(era_now, move |for_later| {
                        for_later.push(unapplied)
                    });
                }
            }
        }
        Ok(consumed_weight)
    }

    fn can_report() -> bool {
        true
    }
}

/// Filter historical offences out and only allow those from the bonding period.
pub struct FilterHistoricalOffences<T, R> {
    _inner: sp_std::marker::PhantomData<(T, R)>,
}

impl<T, Reporter, Offender, R, O> ReportOffence<Reporter, Offender, O>
for FilterHistoricalOffences<Module<T>, R> where
    T: Trait,
    R: ReportOffence<Reporter, Offender, O>,
    O: Offence<Offender>,
{
    fn report_offence(reporters: Vec<Reporter>, offence: O) -> Result<(), OffenceError> {
        // disallow any slashing from before the current bonding period.
        let offence_session = offence.session_index();
        let bonded_eras = BondedEras::get();

        if bonded_eras.first().filter(|(_, start)| offence_session >= *start).is_some() {
            R::report_offence(reporters, offence)
        } else {
            <Module<T>>::deposit_event(
                RawEvent::OldSlashingReportDiscarded(offence_session)
            );
            Ok(())
        }
    }

    fn is_known_offence(offenders: &[Offender], time_slot: &O::TimeSlot) -> bool {
        R::is_known_offence(offenders, time_slot)
    }
}
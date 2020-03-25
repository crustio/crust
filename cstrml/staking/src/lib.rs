#![feature(vec_remove_item)]
#![recursion_limit = "128"]
#![cfg_attr(not(feature = "std"), no_std)]

mod migration;
#[cfg(test)]
mod mock;
mod slashing;
#[cfg(test)]
mod tests;

pub mod inflation;

use codec::{Decode, Encode, HasCompact};
use frame_support::{
    decl_error, decl_event, decl_module, decl_storage, ensure,
    traits::{
        Currency, Get, Imbalance, LockIdentifier, LockableCurrency, OnFreeBalanceZero,
        OnUnbalanced, Time, WithdrawReasons,
    },
    weights::SimpleDispatchInfo,
};
use pallet_session::{historical::OnSessionEnding, SelectInitialValidators};
use sp_runtime::{
    curve::PiecewiseLinear,
    traits::{
        Bounded, CheckedSub, Convert, EnsureOrigin, One, SaturatedConversion, Saturating,
        SimpleArithmetic, StaticLookup, Zero,
    },
    Perbill, RuntimeDebug,
};
use sp_staking::{
    offence::{Offence, OffenceDetails, OnOffenceHandler, ReportOffence},
    SessionIndex,
};
use sp_std::{convert::TryInto, prelude::*};

use frame_system::{self as system, ensure_root, ensure_signed};
#[cfg(feature = "std")]
use sp_runtime::{Deserialize, Serialize};

// Crust runtime modules
// TODO: using tee passing into `Trait` like Currency?
use tee;
use crate::StakerStatus::{Validator, Guarantor};

const DEFAULT_MINIMUM_VALIDATOR_COUNT: u32 = 4;
const MAX_NOMINATIONS: usize = 16;
const MAX_UNLOCKING_CHUNKS: usize = 32;
const STAKING_ID: LockIdentifier = *b"staking ";

/// Counter for the number of eras that have passed.
pub type EraIndex = u32;

/// Counter for the number of "reward" points earned by a given validator.
pub type Points = u32;

/// Reward points of an era. Used to split era total payout between validators.
#[derive(Encode, Decode, Default)]
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
pub struct Validations<AccountId> {
    /// Reward that validator takes up-front; only the rest is split between themselves and
    /// guarantors.
    #[codec(compact)]
    pub commission: Perbill,
    /// Record who vote me, this is used for guarantors to change their voting behaviour
    pub guarantors: Vec<AccountId>,
}

impl<AccountId> Default for Validations<AccountId> {
    fn default() -> Self {
        Validations {
            commission: Default::default(),
            guarantors: vec![]
        }
    }
}

/// A record of the nominations made by a specific account.
#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
pub struct Nominations<AccountId> {
    /// The targets of nomination.
    pub targets: Vec<AccountId>,
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
    /// The valid amount of the stash's balance that will be used for calculate rewards
    /// by limitation
    #[codec(compact)]
    pub valid: Balance,
    /// Any balance that is becoming free, which may eventually be transferred out
    /// of the stash (assuming it doesn't get slashed first).
    pub unlocking: Vec<UnlockChunk<Balance>>,
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
            valid: self.valid,
            unlocking,
        }
    }
}

impl<AccountId, Balance> StakingLedger<AccountId, Balance>
where
    Balance: SimpleArithmetic + Saturating + Copy,
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
    /// The stash account of the guarantor in question.
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

impl<T: Trait> SessionInterface<<T as frame_system::Trait>::AccountId> for T
where
    T: pallet_session::Trait<ValidatorId = <T as frame_system::Trait>::AccountId>,
    T: pallet_session::historical::Trait<
        FullIdentification = Exposure<<T as frame_system::Trait>::AccountId, BalanceOf<T>>,
        FullIdentificationOf = ExposureOf<T>,
    >,
    T::SessionHandler: pallet_session::SessionHandler<<T as frame_system::Trait>::AccountId>,
    T::OnSessionEnding: pallet_session::OnSessionEnding<<T as frame_system::Trait>::AccountId>,
    T::SelectInitialValidators:
        pallet_session::SelectInitialValidators<<T as frame_system::Trait>::AccountId>,
    T::ValidatorIdOf: Convert<
        <T as frame_system::Trait>::AccountId,
        Option<<T as frame_system::Trait>::AccountId>,
    >,
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

pub trait Trait: frame_system::Trait + tee::Trait {
    /// The staking balance.
    type Currency: LockableCurrency<Self::AccountId, Moment = Self::BlockNumber>;

    /// Time used for computing era duration.
    type Time: Time;

    /// Convert a balance into a number used for election calculation.
    /// This must fit into a `u64` but is allowed to be sensibly lossy.
    /// TODO: #1377
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

    /// The NPoS reward curve to use.
    type RewardCurve: Get<&'static PiecewiseLinear<'static>>;
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

        /// The ideal number of staking participants.
        pub ValidatorCount get(fn validator_count) config(): u32;
        /// Minimum number of staking participants before emergency conditions are imposed.
        pub MinimumValidatorCount get(fn minimum_validator_count) config():
            u32 = DEFAULT_MINIMUM_VALIDATOR_COUNT;

        /// Any validators that may never be slashed or forcibly kicked. It's a Vec since they're
        /// easy to initialize and the performance hit is minimal (we expect no more than four
        /// invulnerables) and restricted to testnets.
        pub Invulnerables get(fn invulnerables) config(): Vec<T::AccountId>;

        /// Map from all locked "stash" accounts to the controller account.
        pub Bonded get(fn bonded): map T::AccountId => Option<T::AccountId>;
        /// Map from all (unlocked) "controller" accounts to the info regarding the staking.
        pub Ledger get(fn ledger):
            map T::AccountId => Option<StakingLedger<T::AccountId, BalanceOf<T>>>;

        /// Where the reward payment should be made. Keyed by stash.
        pub Payee get(fn payee): map T::AccountId => RewardDestination;

        /// The map from {(wannabe) validator}/{candidate} stash key to the validation
        /// relationship of that validator.
        pub Validators get(fn validators): linked_map T::AccountId => Validations<T::AccountId>;

        /// The map from guarantor stash key to the set of stash keys of all
        /// validators to guarantee.
        ///
        /// NOTE: is private so that we can ensure upgraded before all typical accesses.
        /// Direct storage APIs can still bypass this protection.
        Guarantors get(fn guarantors): linked_map T::AccountId => Option<Nominations<T::AccountId>>;

        /// The map from {guarantor, candidate} edge to vote stakes of all guarantors.
        GuaranteeRel get(fn guarantee_rel): map Vec<(T::AccountId, T::AccountId)> => Option<BalanceOf<T>>;

        /// Guarantors for a particular account that is in action right now. You can't iterate
        /// through candidates here, but you can find them using Validators.
        ///
        /// This is keyed by the stash account.
        pub Stakers get(fn stakers): map T::AccountId => Exposure<T::AccountId, BalanceOf<T>>;

        /// The stake limit
        /// This is keyed by the stash account.
        pub StakeLimit get(fn stake_limit): map T::AccountId => Option<BalanceOf<T>>;

        /// The currently elected validator set keyed by stash account ID.
        pub CurrentElected get(fn current_elected): Vec<T::AccountId>;

        /// The current era index.
        pub CurrentEra get(fn current_era) config(): EraIndex;

        /// The start of the current era.
        pub CurrentEraStart get(fn current_era_start): MomentOf<T>;

        /// The session index at which the current era started.
        pub CurrentEraStartSessionIndex get(fn current_era_start_session_index): SessionIndex;

        /// Rewards for the current era. Using indices of current elected set.
        CurrentEraPointsEarned get(fn current_era_reward): EraPoints;

        /// The amount of balance actively at stake for each validator slot, currently.
        ///
        /// This is used to derive rewards and punishments.
        pub SlotStake get(fn slot_stake) build(|config: &GenesisConfig<T>| {
            config.stakers.iter().map(|&(_, _, value, _)| value).min().unwrap_or_default()
        }): BalanceOf<T>;

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
        pub UnappliedSlashes: map EraIndex => Vec<UnappliedSlash<T::AccountId, BalanceOf<T>>>;

        /// A mapping from still-bonded eras to the first session index of that era.
        BondedEras: Vec<(EraIndex, SessionIndex)>;

        /// All slashing events on validators, mapped by era to the highest slash proportion
        /// and slash value of the era.
        ValidatorSlashInEra:
            double_map EraIndex, twox_128(T::AccountId) => Option<(Perbill, BalanceOf<T>)>;

        /// All slashing events on guarantors, mapped by era to the highest slash value of the era.
        GuarantorSlashInEra:
            double_map EraIndex, twox_128(T::AccountId) => Option<BalanceOf<T>>;

        /// Slashing spans for stash accounts.
        SlashingSpans: map T::AccountId => Option<slashing::SlashingSpans>;

        /// Records information about the maximum slash of a stash within a slashing span,
        /// as well as how much reward has been paid out.
        SpanSlash:
            map (T::AccountId, slashing::SpanIndex) => slashing::SpanRecord<BalanceOf<T>>;

        /// The earliest era for which we have a pending, unapplied slash.
        EarliestUnappliedSlash: Option<EraIndex>;

        /// The version of storage for upgrade.
        StorageVersion: u32;
    }
    add_extra_genesis {
        config(stakers):
            Vec<(T::AccountId, T::AccountId, BalanceOf<T>, StakerStatus<T::AccountId, BalanceOf<T>>)>;
        build(|config: &GenesisConfig<T>| {
            for &(ref stash, ref controller, balance, ref status) in &config.stakers {
                assert!(
                    T::Currency::free_balance(&stash) >= balance,
                    "Stash does not have enough balance to bond."
                );
                let _ = <Module<T>>::bond(
                    T::Origin::from(Some(stash.clone()).into()),
                    T::Lookup::unlookup(controller.clone()),
                    balance,
                    RewardDestination::Staked,
                );

                // TODO: make genesis validator's limitation more reasonable
                <Module<T>>::upsert_stake_limit(stash, balance+balance);
                let _ = match status {
                    StakerStatus::Validator => {
                        <Module<T>>::validate(
                            T::Origin::from(Some(controller.clone()).into()),
                            Default::default(),
                        )
                    },
                    StakerStatus::Guarantor(votes) => {
                        <Module<T>>::guarantee(
                            T::Origin::from(Some(controller.clone()).into()),
                            votes.iter().map(|(l, v)| (T::Lookup::unlookup(l.clone()), v.clone())).collect(),
                        )
                    }, _ => Ok(())
                };
            }

            StorageVersion::put(migration::CURRENT_VERSION);
        });
    }
}

decl_event!(
    pub enum Event<T> where Balance = BalanceOf<T>, <T as frame_system::Trait>::AccountId {
        /// All validators have been rewarded by the first balance; the second is the remainder
        /// from the maximum amount of reward.
        Reward(Balance, Balance),
        /// One validator (and its guarantors) has been slashed by the given amount.
        Slash(AccountId, Balance),
        /// An old slashing report from a prior era was discarded because it could
        /// not be processed.
        OldSlashingReportDiscarded(SessionIndex),

        // TODO: add stake limitation check event
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
        /// Candidate is already validated
        AlreadyValidated,
        /// Controller is already paired.
        AlreadyPaired,
        /// Targets cannot be empty.
        EmptyTargets,
        /// Duplicate index.
        DuplicateIndex,
        /// Slash record index out of bounds.
        InvalidSlashIndex,
        /// Can not bond with value less than minimum balance.
        InsufficientValue,
        /// Can not schedule more unlock chunks.
        NoMoreChunks,
        /// Can not bond with more than limit
        ExceedLimit,
        /// Can not validate without workloads
        NoWorkloads
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

        fn on_initialize() {
            Self::ensure_storage_upgraded();
        }

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
        /// # <weight>
        /// - Independent of the arguments. Moderate complexity.
        /// - O(1).
        /// - Three extra DB entries.
        ///
        /// NOTE: Two of the storage writes (`Self::bonded`, `Self::payee`) are _never_ cleaned unless
        /// the `origin` falls below _existential deposit_ and gets removed as dust.
        /// # </weight>
        #[weight = SimpleDispatchInfo::FixedNormal(500_000)]
        fn bond(origin,
            controller: <T::Lookup as StaticLookup>::Source,
            #[compact] value: BalanceOf<T>,
            payee: RewardDestination
        ) {
            let stash = ensure_signed(origin)?;

            if <Bonded<T>>::exists(&stash) {
                Err(Error::<T>::AlreadyBonded)?
            }

            let controller = T::Lookup::lookup(controller)?;

            if <Ledger<T>>::exists(&controller) {
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

            let stash_balance = T::Currency::free_balance(&stash);
            let value = value.min(stash_balance);
            let item = StakingLedger {
                stash,
                total: value,
                active: value,
                valid: Zero::zero(),
                unlocking: vec![]
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
        /// The dispatch origin for this call must be _Signed_ by the stash, not the controller.
        ///
        /// # <weight>
        /// - Independent of the arguments. Insignificant complexity.
        /// - O(1).
        /// - Two DB entry.
        /// # </weight>
        #[weight = SimpleDispatchInfo::FixedNormal(500_000)]
        fn bond_extra(origin, #[compact] max_additional: BalanceOf<T>) {
            let stash = ensure_signed(origin)?;

            let controller = Self::bonded(&stash).ok_or(Error::<T>::NotStash)?;
            let mut ledger = Self::ledger(&controller).ok_or(Error::<T>::NotController)?;

            let stash_balance = T::Currency::free_balance(&stash);

            if let Some(mut extra) = stash_balance.checked_sub(&ledger.total) {
                extra = extra.min(max_additional);
                // [LIMIT ACTIVE CHECK] 1:
                // Candidates should judge its stake limit, this promise candidates' bonded stake
                // won't exceed.
                if <Validators<T>>::exists(&stash) {
                    let limit = Self::stake_limit(&stash).unwrap_or_default();
                    if ledger.active >= limit {
                        Err(Error::<T>::NoWorkloads)?
                    } else {
                        extra = extra.min(limit - ledger.active);
                    }
                }

                ledger.total += extra;
                ledger.active += extra;
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
        ///
        /// See also [`Call::withdraw_unbonded`].
        ///
        /// # <weight>
        /// - Independent of the arguments. Limited but potentially exploitable complexity.
        /// - Contains a limited number of reads.
        /// - Each call (requires the remainder of the bonded balance to be above `minimum_balance`)
        ///   will cause a new entry to be inserted into a vector (`Ledger.unlocking`) kept in storage.
        ///   The only way to clean the aforementioned storage item is also user-controlled via `withdraw_unbonded`.
        /// - One DB entry.
        /// </weight>
        #[weight = SimpleDispatchInfo::FixedNormal(400_000)]
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

                let era = Self::current_era() + T::BondingDuration::get();
                ledger.unlocking.push(UnlockChunk { value, era });
                ledger.valid = ledger.active;

                Self::update_ledger(&controller, &ledger);
            }
        }

        /// Remove any unlocked chunks from the `unlocking` queue from our management.
        ///
        /// This essentially frees up that balance to be used by the stash account to do
        /// whatever it wants.
        ///
        /// The dispatch origin for this call must be _Signed_ by the controller, not the stash.
        ///
        /// See also [`Call::unbond`].
        ///
        /// # <weight>
        /// - Could be dependent on the `origin` argument and how much `unlocking` chunks exist.
        ///  It implies `consolidate_unlocked` which loops over `Ledger.unlocking`, which is
        ///  indirectly user-controlled. See [`unbond`] for more detail.
        /// - Contains a limited number of reads, yet the size of which could be large based on `ledger`.
        /// - Writes are limited to the `origin` account key.
        /// # </weight>
        #[weight = SimpleDispatchInfo::FixedNormal(400_000)]
        fn withdraw_unbonded(origin) {
            let controller = ensure_signed(origin)?;
            let ledger = Self::ledger(&controller).ok_or(Error::<T>::NotController)?;
            let ledger = ledger.consolidate_unlocked(Self::current_era());

            if ledger.unlocking.is_empty() && ledger.active.is_zero() {
                // This account must have called `unbond()` with some value that caused the active
                // portion to fall below existential deposit + will have no more unlocking chunks
                // left. We can now safely remove this.
                let stash = ledger.stash;
                // remove the lock.
                T::Currency::remove_lock(STAKING_ID, &stash);
                // remove all staking-related information.
                Self::kill_stash(&stash);
            } else {
                // This was the consequence of a partial unbond. just update the ledger and move on.
                Self::update_ledger(&controller, &ledger);
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
        /// # </weight>
        #[weight = SimpleDispatchInfo::FixedNormal(750_000)]
        fn validate(origin, prefs: Perbill) {
            Self::ensure_storage_upgraded();

            let controller = ensure_signed(origin)?;
            let ledger = Self::ledger(&controller).ok_or(Error::<T>::NotController)?;
            let stash = &ledger.stash;

            // GPoS do not allow duplicate validate, cause validator needs keep `guarantors` info
            if <Validators<T>>::exists(&stash) {
                Err(Error::<T>::AlreadyValidated)?
            }

            // [LIMIT ACTIVE CHECK] 2:
            // Only limit is 0, GPoS emit error.
            let limit = Self::stake_limit(&stash).unwrap_or_default();

            if limit == Zero::zero() {
                Err(Error::<T>::NoWorkloads)?
            }

            let validations = Validations {
                commission: prefs,
                guarantors: vec![]
            };

            <Guarantors<T>>::remove(stash);
            <Validators<T>>::insert(stash, validations);
        }

        /// Declare the desire to guarantee `targets` for the origin controller.
        ///
        /// Effects will be felt at the beginning of the next era.
        ///
        /// The dispatch origin for this call must be _Signed_ by the controller, not the stash.
        ///
        /// # <weight>
        /// - The transaction's complexity is proportional to the size of `targets`,
        /// which is capped at `MAX_NOMINATIONS`.
        /// - Both the reads and writes follow a similar pattern.
        /// # </weight>
        #[weight = SimpleDispatchInfo::FixedNormal(750_000)]
        fn guarantee(origin, targets: Vec<(<T::Lookup as StaticLookup>::Source, BalanceOf<T>)>) {
            Self::ensure_storage_upgraded();

            let controller = ensure_signed(origin)?;
            let ledger = Self::ledger(&controller).ok_or(Error::<T>::NotController)?;
            let g_stash = &ledger.stash;
            let mut total_stake = ledger.total;
            ensure!(!targets.is_empty(), Error::<T>::EmptyTargets);

            // [LIMIT ACTIVE CHECK] 3:
            let mut old_targets: Vec<T::AccountId> = vec![];
            if let Some(old_nominations) = Self::guarantors(g_stash) {
                old_targets = old_nominations.targets;
            }
            let mut deleted_targets = old_targets.clone();

            // [LIMIT ACTIVE CHECK] 3:
            // 1. Upserting an edge
            let targets = targets.into_iter()
                .take(MAX_NOMINATIONS)
                .filter_map(|(t, s)|
                    if let Ok(v_stash) = T::Lookup::lookup(t) {
                        // Judge if v_stash is old targets
                        for old_target in &old_targets {
                            if old_target == &v_stash {
                                deleted_targets.remove_item(old_target);
                                break
                            }
                        }
                        if total_stake == Zero::zero() {
                            None
                        } else {
                            let g_votes = s.min(total_stake);
                            if total_stake <= s { total_stake = Zero::zero(); }
                            Self::filter_guarantee(&v_stash, &g_stash, g_votes)
                        }
                    } else {
                        None
                    }
                )
                .collect::<Vec<T::AccountId>>();

            // 2. Deleted edge and update Validators
            for deleted_target in deleted_targets {
                let mut validations = Self::validators(&deleted_target);
                let deleted_edge = vec![(g_stash.clone(), deleted_target.clone())];
                validations.guarantors.remove_item(&g_stash);
                <Validators<T>>::insert(&deleted_target, validations);
                <GuaranteeRel<T>>::remove(&deleted_edge);
            }

            ensure!(!targets.is_empty(), Error::<T>::EmptyTargets);

            let nominations = Nominations {
                targets,
                submitted_in: Self::current_era(),
                suppressed: false,
            };

            <Validators<T>>::remove(g_stash);
            <Guarantors<T>>::insert(g_stash, nominations);
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
        /// # </weight>
        #[weight = SimpleDispatchInfo::FixedNormal(500_000)]
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
        /// # </weight>
        #[weight = SimpleDispatchInfo::FixedNormal(500_000)]
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
        /// # </weight>
        #[weight = SimpleDispatchInfo::FixedNormal(750_000)]
        fn set_controller(origin, controller: <T::Lookup as StaticLookup>::Source) {
            let stash = ensure_signed(origin)?;
            let old_controller = Self::bonded(&stash).ok_or(Error::<T>::NotStash)?;
            let controller = T::Lookup::lookup(controller)?;
            if <Ledger<T>>::exists(&controller) {
                Err(Error::<T>::AlreadyPaired)?
            }
            if controller != old_controller {
                <Bonded<T>>::insert(&stash, &controller);
                if let Some(l) = <Ledger<T>>::take(&old_controller) {
                    <Ledger<T>>::insert(&controller, l);
                }
            }
        }

        /// The ideal number of validators.
        #[weight = SimpleDispatchInfo::FreeOperational]
        fn set_validator_count(origin, #[compact] new: u32) {
            ensure_root(origin)?;
            ValidatorCount::put(new);
        }

        // ----- Root calls.

        /// Force there to be no new eras indefinitely.
        ///
        /// # <weight>
        /// - No arguments.
        /// # </weight>
        #[weight = SimpleDispatchInfo::FreeOperational]
        fn force_no_eras(origin) {
            ensure_root(origin)?;
            ForceEra::put(Forcing::ForceNone);
        }

        /// Force there to be a new era at the end of the next session. After this, it will be
        /// reset to normal (non-forced) behaviour.
        ///
        /// # <weight>
        /// - No arguments.
        /// # </weight>
        #[weight = SimpleDispatchInfo::FreeOperational]
        fn force_new_era(origin) {
            ensure_root(origin)?;
            ForceEra::put(Forcing::ForceNew);
        }

        /// Set the validators who cannot be slashed (if any).
        #[weight = SimpleDispatchInfo::FreeOperational]
        fn set_invulnerables(origin, validators: Vec<T::AccountId>) {
            ensure_root(origin)?;
            <Invulnerables<T>>::put(validators);
        }

        /// Force a current staker to become completely unstaked, immediately.
        #[weight = SimpleDispatchInfo::FreeOperational]
        fn force_unstake(origin, stash: T::AccountId) {
            ensure_root(origin)?;

            // remove the lock.
            T::Currency::remove_lock(STAKING_ID, &stash);
            // remove all staking-related information.
            Self::kill_stash(&stash);
        }

        /// Force there to be a new era at the end of sessions indefinitely.
        ///
        /// # <weight>
        /// - One storage write
        /// # </weight>
        #[weight = SimpleDispatchInfo::FreeOperational]
        fn force_new_era_always(origin) {
            ensure_root(origin)?;
            ForceEra::put(Forcing::ForceAlways);
        }

        /// Cancel enactment of a deferred slash. Can be called by either the root origin or
        /// the `T::SlashCancelOrigin`.
        /// passing the era and indices of the slashes for that era to kill.
        ///
        /// # <weight>
        /// - One storage write.
        /// # </weight>
        #[weight = SimpleDispatchInfo::FreeOperational]
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

    fn stake_limit_of(workloads: u128) -> BalanceOf<T> {
        let total_workloads = <tee::Module<T>>::workloads().unwrap();
        let total_issuance = TryInto::<u128>::try_into(T::Currency::total_issuance())
            .ok()
            .unwrap();

        // total_workloads cannot be zero, or system go panic!
        let workloads_to_stakes = ((workloads * total_issuance / total_workloads / 2) as u128)
            .min(u64::max_value() as u128);

        workloads_to_stakes.try_into().ok().unwrap()
    }

    // MUTABLES (DANGEROUS)

    /// Filter guarantor's target and update validator, this function including new and update
    /// edge(voting relationship) and update UWG(Undirected Weighted Graph)'s vertex
    ///
    /// NOTE: this should handle the update of `Validator` and `GuaranteeRel`, then return (maybe)
    /// voted validator
    /// # <weight>
    /// - Independent of the arguments. Insignificant complexity.
    /// - O(n).
    /// - 2n+5 DB entry.
    /// MAX(n) = MAX_NOMINATIONS
    /// # </weight>
    fn filter_guarantee(v_stash: &T::AccountId, g_stash: &T::AccountId, g_votes: BalanceOf<T>)
                        -> Option<T::AccountId> {
        let mut v_current_stakes = Self::slashable_balance_of(v_stash);
        let mut validations: Validations<T::AccountId> = Self::validators(v_stash);
        let guarantors = validations.guarantors;
        let mut new_guarantors = guarantors.clone();

        // Traverse validator -> guarantors
        for guarantor in guarantors {
            // 1. Get weighted edge info
            let edge = vec![(guarantor.clone(), v_stash.clone())];
            let weight: BalanceOf<T> = Self::guarantee_rel(&edge).unwrap_or_default();
            v_current_stakes += weight;

            // 2. If guarantor already voted the validator
            if &guarantor == g_stash {
                if weight > g_votes {
                    // a. Guarantor wanna to reduce his votes, then just update relationship
                    // and return
                    <GuaranteeRel<T>>::insert(&edge, g_votes);
                    return Some(v_stash.clone());
                } else if weight < g_votes {
                    // b. Guarantor wanna to increase his votes, delete
                    // old guarantor from `Validations` as well as update current_stakes
                    new_guarantors.remove_item(&guarantor);
                    v_current_stakes -= weight;
                } else {
                    // c. Safe and sound, do NOTHING 🙂
                }
                break
            }
        }

        // New guarantor or vote_increasing guarantor
        // 1. Add new guarantor into `Validators`
        // 2. Update `GuaranteeRel` by (maybe) cutting the limit
        let v_last_era_limit = Self::stake_limit(&v_stash).unwrap_or_default();

        if v_current_stakes >= v_last_era_limit {
            None
        } else {
            new_guarantors.push(g_stash.clone());
            validations.guarantors = new_guarantors;
            <Validators<T>>::insert(&v_stash, &validations);

            let new_edge = vec![(g_stash.clone(), v_stash.clone())];
            let new_weight = g_votes.min(v_last_era_limit - v_current_stakes);
            <GuaranteeRel<T>>::insert(&new_edge, new_weight);
            Some(v_stash.clone())
        }
    }

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
            T::BlockNumber::max_value(),
            WithdrawReasons::all(),
        );
        <Ledger<T>>::insert(controller, ledger);
    }

    /// Chill a stash account.
    fn chill_stash(stash: &T::AccountId) {
        <Validators<T>>::remove(stash);
        <Guarantors<T>>::remove(stash);
    }

    /// Ensures storage is upgraded to most recent necessary state.
    fn ensure_storage_upgraded() {
        migration::perform_migrations::<T>();
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

    /// Reward a given validator by a specific amount. Add the reward to the validator's, and its
    /// guarantors' balance, pro-rata based on their exposure, after having removed the validator's
    /// pre-payout cut.
    fn reward_validator(stash: &T::AccountId, reward: BalanceOf<T>) -> PositiveImbalanceOf<T> {
        let off_the_table = Self::validators(stash).commission * reward;
        let reward = reward.saturating_sub(off_the_table);
        let mut imbalance = <PositiveImbalanceOf<T>>::zero();
        let validator_cut = if reward.is_zero() {
            Zero::zero()
        } else {
            let exposure = Self::stakers(stash);
            let total = exposure.total.max(One::one());

            for i in &exposure.others {
                let per_u64 = Perbill::from_rational_approximation(i.value, total);
                imbalance.maybe_subsume(Self::make_payout(&i.who, per_u64 * reward));
            }

            let per_u64 = Perbill::from_rational_approximation(exposure.own, total);
            per_u64 * reward
        };

        imbalance.maybe_subsume(Self::make_payout(stash, validator_cut + off_the_table));

        imbalance
    }

    /// Session has just ended. Provide the validator set for the next session if it's an era-end, along
    /// with the exposure of the prior validator set.
    fn new_session(
        session_index: SessionIndex,
    ) -> Option<(
        Vec<T::AccountId>,
        Vec<(T::AccountId, Exposure<T::AccountId, BalanceOf<T>>)>,
    )> {
        let era_length = session_index
            .checked_sub(Self::current_era_start_session_index())
            .unwrap_or(0);
        match ForceEra::get() {
            Forcing::ForceNew => ForceEra::kill(),
            Forcing::ForceAlways => (),
            Forcing::NotForcing if era_length >= T::SessionsPerEra::get() => (),
            _ => return None,
        }
        let validators = T::SessionInterface::validators();
        let prior = validators
            .into_iter()
            .map(|v| {
                let e = Self::stakers(&v);
                (v, e)
            })
            .collect();

        Self::new_era(session_index).map(move |new| (new, prior))
    }

    /// The era has changed - enact new staking set.
    ///
    /// NOTE: This always happens immediately before a session change to ensure that new validators
    /// get a chance to set their session keys.
    /// This also checks stake limitation based on work reports
    fn new_era(start_session_index: SessionIndex) -> Option<Vec<T::AccountId>> {
        // Payout
        let points = CurrentEraPointsEarned::take();
        let now = T::Time::now();
        let previous_era_start = <CurrentEraStart<T>>::mutate(|v| sp_std::mem::replace(v, now));
        let era_duration = now - previous_era_start;
        if !era_duration.is_zero() {
            let validators = Self::current_elected();

            let validator_len: BalanceOf<T> = (validators.len() as u32).into();
            let total_rewarded_stake = Self::slot_stake() * validator_len;

            let (total_payout, max_payout) = inflation::compute_total_payout(
                &T::RewardCurve::get(),
                total_rewarded_stake.clone(),
                T::Currency::total_issuance(),
                // Duration of era; more than u64::MAX is rewarded as u64::MAX.
                era_duration.saturated_into::<u64>(),
            );

            let mut total_imbalance = <PositiveImbalanceOf<T>>::zero();

            for (v, p) in validators.iter().zip(points.individual.into_iter()) {
                if p != 0 {
                    let reward =
                        Perbill::from_rational_approximation(p, points.total) * total_payout;
                    total_imbalance.subsume(Self::reward_validator(v, reward));
                }
            }

            // assert!(total_imbalance.peek() == total_payout)
            let total_payout = total_imbalance.peek();

            let rest = max_payout.saturating_sub(total_payout);
            Self::deposit_event(RawEvent::Reward(total_payout, rest));

            T::Reward::on_unbalanced(total_imbalance);
            T::RewardRemainder::on_unbalanced(T::Currency::issue(rest));
        }

        // Increment current era.
        let current_era = CurrentEra::mutate(|s| {
            *s += 1;
            *s
        });

        CurrentEraStartSessionIndex::mutate(|v| {
            *v = start_session_index;
        });
        let bonding_duration = T::BondingDuration::get();

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

        // Reassign all Stakers.
        let (_slot_stake, maybe_new_validators) = Self::select_validators();
        Self::apply_unapplied_slashes(current_era);

        maybe_new_validators
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

    /// Select a new validator set from the assembled stakers and their role preferences.
    ///
    /// Returns the new `SlotStake` value and a set of newly selected _stash_ IDs.
    ///
    /// Assumes storage is coherent with the declaration.
    fn select_validators() -> (BalanceOf<T>, Option<Vec<T::AccountId>>) {
        // Update all stake limit anyway
        Self::update_all_stake_limit();

        let validators: Vec<(T::AccountId, Validations<T::AccountId>)> =
            <Validators<T>>::enumerate().collect();
        let validator_count = validators.len();
        let minimum_validator_count = Self::minimum_validator_count().max(1) as usize;

        if validator_count < minimum_validator_count {
            // There were not enough validators for even our minimal level of functionality.
            // This is bad.
            // We should probably disable all functionality except for block production
            // and let the chain keep producing blocks until we can decide on a sufficiently
            // substantial set.
            // TODO: #2494(paritytech/substrate)
            return (Self::slot_stake(), None)
        }

        let to_votes =
            |b: BalanceOf<T>| <T::CurrencyToVote as Convert<BalanceOf<T>, u64>>::convert(b) as u128;
        let to_balance =
            |e: u128| <T::CurrencyToVote as Convert<u128, BalanceOf<T>>>::convert(e);

        // I. Traverse validators, get `IndividualExposure` and update guarantors
        for (v_stash, validations) in &validators {
            let v_controller = Self::bonded(v_stash).unwrap();

            let mut v_ledger: StakingLedger<T::AccountId, BalanceOf<T>> =
                Self::ledger(&v_controller).unwrap();
            let v_limit_stakes = Self::stake_limit(v_stash).unwrap_or(Zero::zero());
            let mut v_own_stakes = v_ledger.active;
            let mut others: Vec<IndividualExposure<T::AccountId, BalanceOf<T>>> = vec![];
            let mut v_guarantors_votes = 0;
            let mut new_guarantors: Vec<T::AccountId> = vec![];

            if v_own_stakes >= v_limit_stakes {
                // 1. There has no credit for guarantors
                v_own_stakes = v_limit_stakes;
            } else {
                // TODO: move a separated function
                let mut remains = to_votes(v_limit_stakes - v_own_stakes);
                let guarantors = &validations.guarantors;

                for guarantor in guarantors {
                    let edge = vec![(guarantor.clone(), v_stash.clone())];
                    let votes = to_votes(Self::guarantee_rel(&edge).unwrap_or_default());
                    if remains > 0 {
                        // 2. There still has credit for guarantors
                        // we should using `FILO` rule to calculate others
                        let g_vote_stakes = to_balance(remains.min(votes));

                        // a. preparing new_guarantors for Validators, others for Stakers
                        new_guarantors.push(guarantor.clone());
                        others.push(IndividualExposure {
                            who: guarantor.clone(),
                            value: g_vote_stakes,
                        });

                        // b. update remains and guarantors votes
                        remains -= votes;
                        v_guarantors_votes += votes;

                        // c. UPDATE EDGE: (maybe) update GuaranteeRel
                        <GuaranteeRel<T>>::insert(&edge, g_vote_stakes);
                    } else {
                        // 3. There has no credit for later guarantors, update `Guarantors`
                        // and remove `GuaranteeRel`

                        // a. UPDATE NODE: update `Guarantors`
                        if let Some(mut nominations) = Self::guarantors(guarantor) {
                            nominations.targets.remove_item(v_stash);
                            <Guarantors<T>>::insert(guarantor, nominations);
                        }
                        // b. UPDATE EDGE: delete edge
                        <GuaranteeRel<T>>::remove(&edge);
                    }
                }
            }

            // 3. UPDATE NODE: `Validator`
            let new_validations = Validations {
                commission: validations.commission,
                guarantors: new_guarantors
            };
            <Validators<T>>::insert(v_stash, new_validations);

            v_ledger.valid = v_own_stakes;
            Self::update_ledger(&v_controller, &v_ledger);

            // 4. Update next era's snapshot `Stakers`
            if v_ledger.valid == Zero::zero() {
                // a. remove validator if valid stake goes 0
                <Validators<T>>::remove(v_stash);
            } else {
                let v_own_votes = to_votes(v_own_stakes);
                // b. total_votes should less than balance max value
                let v_total_votes = (v_own_votes + v_guarantors_votes)
                    .min(u64::max_value() as u128);

                // c. build struct `Exposure`
                let exposure = Exposure {
                    own: v_own_stakes,
                    // This might reasonably saturate and we cannot do much about it. The sum of
                    // someone's stake might exceed the balance type if they have the maximum amount
                    // of balance and receive some support. This is super unlikely to happen, yet
                    // we simulate it in some tests.
                    total: to_balance(v_total_votes),
                    others,
                };

                // d. update snapshot
                <Stakers<T>>::insert(v_stash, exposure);
            }
        }

        // II. Traverse guarantors, update guarantor's ledger
        <Guarantors<T>>::enumerate().for_each(|(g_stash, nominations)| {
            let Nominations {
                submitted_in: _,
                targets,
                suppressed: _,
            } = nominations;

            /*// Filter out nomination targets which were guaranteed before the most recent
            // slashing span.
            // TODO: uncomment it when figured out the slash strategy
            targets.retain(|target| {
                <Self as Store>::SlashingSpans::get(&target.who)
                    .map_or(true, |spans| submitted_in >= spans.last_start())
            });*/

            // init all guarantor's valid stakes, set to zero
            let g_controller = Self::bonded(&g_stash).unwrap();
            let mut g_ledger: StakingLedger<T::AccountId, BalanceOf<T>> =
                Self::ledger(&g_controller).unwrap();
            let mut g_valid_stake_votes = Zero::zero();

            for v_stash in targets {
                let edge = vec![(g_stash.clone(), v_stash)];
                g_valid_stake_votes += Self::guarantee_rel(&edge).unwrap_or_default();
            }

            g_ledger.valid = g_valid_stake_votes;
            Self::update_ledger(&g_controller, &g_ledger);
        });

        // III. TopDown Election Algorithm
        // Select new validators by top-down their `valid` stakes
        // - time complex is O(2n)
        // - DB try is n
        // 1. Populate elections and figure out the minimum stake behind a slot.
        let mut validators_stakes = <Validators<T>>::enumerate()
            .map(|(stash, _pref)| {
                let stakers = Self::stakers(&stash);
                (stash, to_votes(stakers.total))
            })
            .collect::<Vec<(T::AccountId, u128)>>();

        validators_stakes.sort_by(|a, b| b.1.cmp(&a.1));

        let to_elect = (Self::validator_count() as usize).min(validators_stakes.len());

        // 2. If there's no validators, be as same as little validators
        if to_elect < 1 {
            return (Self::slot_stake(), None);
        }

        let slot_stake = to_balance(validators_stakes[to_elect - 1].1);
        let elected_stashes = validators_stakes[0..to_elect]
            .iter()
            .map(|(who, _stakes)| who.clone())
            .collect::<Vec<T::AccountId>>();

        // IV. Update general staking storage
        // Set the new validator set in sessions.
        <CurrentElected<T>>::put(&elected_stashes);

        // Update slot stake.
        <SlotStake<T>>::put(&slot_stake);

        // In order to keep the property required by `n_session_ending`
        // that we must return the new validator set even if it's the same as the old,
        // as long as any underlying economic conditions have changed, we don't attempt
        // to do any optimization where we compare against the prior set.
        (slot_stake, Some(elected_stashes))
    }

    /// Remove all associated data of a stash account from the staking system.
    ///
    /// Assumes storage is upgraded before calling.
    ///
    /// This is called :
    /// - Immediately when an account's balance falls below existential deposit.
    /// - after a `withdraw_unbond()` call that frees all of a stash's bonded balance.
    fn kill_stash(stash: &T::AccountId) {
        if let Some(controller) = <Bonded<T>>::take(stash) {
            <Ledger<T>>::remove(&controller);
        }
        <Payee<T>>::remove(stash);
        <Validators<T>>::remove(stash);
        <Guarantors<T>>::remove(stash);

        slashing::clear_stash_metadata::<T>(stash);
    }

    /// This function will update all the work reporters' stake limit
    ///
    /// # <weight>
    /// - Independent of the arguments. Insignificant complexity.
    /// - O(n).
    /// - 2n+1 DB entry.
    /// # </weight>
    fn update_all_stake_limit() {
        // 1. Get all work reports
        let ids = <tee::TeeIdentities<T>>::enumerate().collect::<Vec<_>>();

        for (controller, _) in ids {
            // 2. Get controller's (maybe)ledger
            let maybe_ledger = Self::ledger(&controller);
            if let Some(ledger) = maybe_ledger {
                let workload = <tee::Module<T>>::get_and_update_workload(&controller);

                // 3. Update stake limit anyway
                Self::upsert_stake_limit(&ledger.stash, Self::stake_limit_of(workload));
            }
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
        // TODO: This can be optimised once #3302 is implemented.
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
}

impl<T: Trait> pallet_session::OnSessionEnding<T::AccountId> for Module<T> {
    fn on_session_ending(
        _ending: SessionIndex,
        start_session: SessionIndex,
    ) -> Option<Vec<T::AccountId>> {
        Self::ensure_storage_upgraded();
        Self::new_session(start_session - 1).map(|(new, _old)| new)
    }
}

impl<T: Trait> OnSessionEnding<T::AccountId, Exposure<T::AccountId, BalanceOf<T>>> for Module<T> {
    fn on_session_ending(
        _ending: SessionIndex,
        start_session: SessionIndex,
    ) -> Option<(
        Vec<T::AccountId>,
        Vec<(T::AccountId, Exposure<T::AccountId, BalanceOf<T>>)>,
    )> {
        Self::ensure_storage_upgraded();
        Self::new_session(start_session - 1)
    }
}

impl<T: Trait> OnFreeBalanceZero<T::AccountId> for Module<T> {
    fn on_free_balance_zero(stash: &T::AccountId) {
        Self::ensure_storage_upgraded();
        Self::kill_stash(stash);
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
        Some(<Module<T>>::stakers(&validator))
    }
}

impl<T: Trait> SelectInitialValidators<T::AccountId> for Module<T> {
    fn select_initial_validators() -> Option<Vec<T::AccountId>> {
        <Module<T>>::select_validators().1
    }
}

/// This is intended to be used with `FilterHistoricalOffences`.
impl<T: Trait> OnOffenceHandler<T::AccountId, pallet_session::historical::IdentificationTuple<T>>
    for Module<T>
where
    T: pallet_session::Trait<ValidatorId = <T as frame_system::Trait>::AccountId>,
    T: pallet_session::historical::Trait<
        FullIdentification = Exposure<<T as frame_system::Trait>::AccountId, BalanceOf<T>>,
        FullIdentificationOf = ExposureOf<T>,
    >,
    T::SessionHandler: pallet_session::SessionHandler<<T as frame_system::Trait>::AccountId>,
    T::OnSessionEnding: pallet_session::OnSessionEnding<<T as frame_system::Trait>::AccountId>,
    T::SelectInitialValidators:
        pallet_session::SelectInitialValidators<<T as frame_system::Trait>::AccountId>,
    T::ValidatorIdOf: Convert<
        <T as frame_system::Trait>::AccountId,
        Option<<T as frame_system::Trait>::AccountId>,
    >,
{
    fn on_offence(
        offenders: &[OffenceDetails<
            T::AccountId,
            pallet_session::historical::IdentificationTuple<T>,
        >],
        slash_fraction: &[Perbill],
        slash_session: SessionIndex,
    ) {
        <Module<T>>::ensure_storage_upgraded();

        let reward_proportion = SlashRewardFraction::get();

        let era_now = Self::current_era();
        let window_start = era_now.saturating_sub(T::BondingDuration::get());
        let current_era_start_session = CurrentEraStartSessionIndex::get();

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
                None => return, // before bonding period. defensive - should be filtered out.
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
    }
}

/// Filter historical offences out and only allow those from the bonding period.
pub struct FilterHistoricalOffences<T, R> {
    _inner: sp_std::marker::PhantomData<(T, R)>,
}

impl<T, Reporter, Offender, R, O> ReportOffence<Reporter, Offender, O>
    for FilterHistoricalOffences<Module<T>, R>
where
    T: Trait,
    R: ReportOffence<Reporter, Offender, O>,
    O: Offence<Offender>,
{
    fn report_offence(reporters: Vec<Reporter>, offence: O) {
        <Module<T>>::ensure_storage_upgraded();

        // disallow any slashing from before the current bonding period.
        let offence_session = offence.session_index();
        let bonded_eras = BondedEras::get();

        if bonded_eras
            .first()
            .filter(|(_, start)| offence_session >= *start)
            .is_some()
        {
            R::report_offence(reporters, offence)
        } else {
            <Module<T>>::deposit_event(RawEvent::OldSlashingReportDiscarded(offence_session))
        }
    }
}

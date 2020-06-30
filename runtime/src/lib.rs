//! The Substrate Node runtime. This can be compiled with `#[no_std]`, ready for Wasm.

#![cfg_attr(not(feature = "std"), no_std)]
// `construct_runtime!` does a lot of recursion and requires us to increase the limit to 256.
#![recursion_limit = "256"]

// Make the WASM binary available.
#[cfg(feature = "std")]
include!(concat!(env!("OUT_DIR"), "/wasm_binary.rs"));

use sp_core::OpaqueMetadata;
use sp_runtime::traits::{
    BlakeTwo256, Block as BlockT, Convert, OpaqueKeys, SaturatedConversion,
    IdentityLookup, Saturating
};
use sp_runtime::{
    create_runtime_str, generic, impl_opaque_keys, ApplyExtrinsicResult, Perbill, KeyTypeId,
    transaction_validity::{TransactionValidity, TransactionSource, TransactionPriority}
};
use sp_std::prelude::*;

use grandpa::{AuthorityId as GrandpaId, fg_primitives, AuthorityList as GrandpaAuthorityList};
use sp_api::impl_runtime_apis;
use sp_staking::SessionIndex;

#[cfg(feature = "std")]
use sp_version::NativeVersion;
use sp_version::RuntimeVersion;

// A few exports that help ease life for downstream crates.
pub use authority_discovery_primitives::AuthorityId as AuthorityDiscoveryId;
pub use balances::Call as BalancesCall;
pub use frame_support::{
    construct_runtime, parameter_types,
    traits::{Currency, KeyOwnerProofSystem, Randomness},
    weights::{
        Weight, IdentityFee,
        constants::{BlockExecutionWeight, ExtrinsicBaseWeight, RocksDbWeight, WEIGHT_PER_SECOND},
    },
    StorageValue,
};
pub use im_online::sr25519::AuthorityId as ImOnlineId;
#[cfg(any(feature = "std", test))]
pub use sp_runtime::BuildStorage;
use session::{historical as session_historical};

pub use timestamp::Call as TimestampCall;

/// Crust primitives
use primitives::{constants::{time::*, currency::*}, *};

#[cfg(feature = "std")]
pub use staking::StakerStatus;

/// Opaque types. These are used by the CLI to instantiate machinery that don't need to know
/// the specifics of the runtime. They can then be made to be agnostic over specific formats
/// of data like extrinsics, allowing for them to continue syncing the network through upgrades
/// to even the core datastructures.
pub mod opaque {
    use super::*;

    pub use sp_runtime::OpaqueExtrinsic as UncheckedExtrinsic;

    /// Opaque block header type.
    pub type Header = generic::Header<BlockNumber, BlakeTwo256>;
    /// Opaque block type.
    pub type Block = generic::Block<Header, UncheckedExtrinsic>;
    /// Opaque block identifier type.
    pub type BlockId = generic::BlockId<Block>;
}

impl_opaque_keys! {
    pub struct SessionKeys {
        pub grandpa: Grandpa,
        pub babe: Babe,
        pub im_online: ImOnline,
        pub authority_discovery: AuthorityDiscovery,
    }
}

/// This runtime version.
pub const VERSION: RuntimeVersion = RuntimeVersion {
    spec_name: create_runtime_str!("crust"),
    impl_name: create_runtime_str!("crustio-crust"),
    authoring_version: 1,
    spec_version: 1,
    impl_version: 1,
    apis: RUNTIME_API_VERSIONS,
    transaction_version: 1
};

/// The version information used to identify this runtime when compiled natively.
#[cfg(feature = "std")]
pub fn native_version() -> NativeVersion {
    NativeVersion {
        runtime_version: VERSION,
        can_author_with: Default::default(),
    }
}

parameter_types! {
	pub const BlockHashCount: BlockNumber = 2400;
	pub const MaximumBlockWeight: Weight = 2 * WEIGHT_PER_SECOND;
	pub const AvailableBlockRatio: Perbill = Perbill::from_percent(75);
	pub const MaximumBlockLength: u32 = 5 * 1024 * 1024;
	pub const Version: RuntimeVersion = VERSION;
	pub const MaximumExtrinsicWeight: Weight = AvailableBlockRatio::get()
		.saturating_sub(Perbill::from_percent(10)) * MaximumBlockWeight::get();
}

impl system::Trait for Runtime {
    /// The ubiquitous origin type.
    type Origin = Origin;
    /// The aggregated dispatch type that is available for extrinsics.
    type Call = Call;
    /// The index type for storing how many extrinsics an account has signed.
    type Index = Index;
    /// The index type for blocks.
    type BlockNumber = BlockNumber;
    /// The type for hashing blocks and tries.
    type Hash = Hash;
    /// The hashing algorithm used.
    type Hashing = BlakeTwo256;
    /// The identifier used to distinguish between accounts.
    type AccountId = AccountId;
    /// The lookup mechanism to get account ID from whatever is passed in dispatchers.
    type Lookup = IdentityLookup<AccountId>;
    /// The header type.
    type Header = generic::Header<BlockNumber, BlakeTwo256>;
    /// The ubiquitous event type.
    type Event = Event;
    /// Maximum number of block number to block hash mappings to keep (oldest pruned first).
    type BlockHashCount = BlockHashCount;
    /// Maximum weight of each block.
    type MaximumBlockWeight = MaximumBlockWeight;
    /// The weight of database operations that the runtime can invoke.
    type DbWeight = RocksDbWeight;
    /// The weight of the overhead invoked on the block import process, independent of the
    /// extrinsics included in that block.
    type BlockExecutionWeight = BlockExecutionWeight;
    /// The base weight of any extrinsic processed by the runtime, independent of the
    /// logic of that extrinsic. (Signature verification, nonce increment, fee, etc...)
    type ExtrinsicBaseWeight = ExtrinsicBaseWeight;
    /// The maximum weight that a single extrinsic of `Normal` dispatch class can have,
    /// idependent of the logic of that extrinsics. (Roughly max block weight - average on
    /// initialize cost).
    type MaximumExtrinsicWeight = MaximumExtrinsicWeight;
    /// Maximum size of all encoded transactions (in bytes) that are allowed in one block.
    type MaximumBlockLength = MaximumBlockLength;
    /// Portion of the block weight that is available to all normal transactions.
    type AvailableBlockRatio = AvailableBlockRatio;
    /// Version of the runtime.
    type Version = Version;
    /// Converts a module to the index of the module in `construct_runtime!`.
    ///
    /// This type is being generated by `construct_runtime!`.
    type ModuleToIndex = ModuleToIndex;
    /// The data to be stored in an account.
    type AccountData = balances::AccountData<Balance>;
    /// What to do if a new account is created.
    type OnNewAccount = ();
    /// What to do if an account is fully reaped from the system.
    type OnKilledAccount = ();
}

parameter_types! {
    pub const EpochDuration: u64 = EPOCH_DURATION_IN_BLOCKS as u64;
    pub const ExpectedBlockTime: u64 = MILLISECS_PER_BLOCK;
}

impl babe::Trait for Runtime {
    type EpochDuration = EpochDuration;
    type ExpectedBlockTime = ExpectedBlockTime;

    // session module is the trigger
    type EpochChangeTrigger = babe::ExternalTrigger;
}

parameter_types! {
	pub const IndexDeposit: Balance = 1 * DOLLARS;
}

impl indices::Trait for Runtime {
    type AccountIndex = AccountIndex;
    type Currency = Balances;
    type Deposit = IndexDeposit;
    type Event = Event;
}

impl authority_discovery::Trait for Runtime {}

impl<C> system::offchain::SendTransactionTypes<C> for Runtime where
    Call: From<C>,
{
    type Extrinsic = UncheckedExtrinsic;
    type OverarchingCall = Call;
}

parameter_types! {
    pub const SessionDuration: BlockNumber = EPOCH_DURATION_IN_BLOCKS as _;
}

parameter_types! {
	pub const StakingUnsignedPriority: TransactionPriority = TransactionPriority::max_value() / 2;
	pub const ImOnlineUnsignedPriority: TransactionPriority = TransactionPriority::max_value();
}

impl im_online::Trait for Runtime {
    type AuthorityId = ImOnlineId;
    type Event = Event;
    type SessionDuration = SessionDuration;
    type ReportUnresponsiveness = Offences;
    type UnsignedPriority = ImOnlineUnsignedPriority;
}

parameter_types! {
	pub const MaximumWeight: Weight = 2_000_000;
}

impl scheduler::Trait for Runtime {
    type Event = Event;
    type Origin = Origin;
    type Call = Call;
    type MaximumWeight = MaximumWeight;
}

impl grandpa::Trait for Runtime {
    type Event = Event;
    type Call = Call;

    type KeyOwnerProof =
    <Self::KeyOwnerProofSystem as KeyOwnerProofSystem<(KeyTypeId, GrandpaId)>>::Proof;

    type KeyOwnerIdentification = <Self::KeyOwnerProofSystem as KeyOwnerProofSystem<(
        KeyTypeId,
        GrandpaId,
    )>>::IdentificationTuple;

    type KeyOwnerProofSystem = ();

    type HandleEquivocation = ();
}

parameter_types! {
    pub const WindowSize: BlockNumber = finality_tracker::DEFAULT_WINDOW_SIZE.into();
    pub const ReportLatency: BlockNumber = finality_tracker::DEFAULT_REPORT_LATENCY.into();
}

impl finality_tracker::Trait for Runtime {
    type OnFinalizationStalled = ();
    type WindowSize = WindowSize;
    type ReportLatency = ReportLatency;
}

parameter_types! {
    pub const MinimumPeriod: u64 = SLOT_DURATION / 2;
}

impl timestamp::Trait for Runtime {
    /// A timestamp: milliseconds since the unix epoch.
    type Moment = u64;
    type OnTimestampSet = Babe;
    type MinimumPeriod = MinimumPeriod;
}

parameter_types! {
	pub const UncleGenerations: BlockNumber = 0;
}

impl authorship::Trait for Runtime {
    type FindAuthor = session::FindAccountFromAuthorIndex<Self, Babe>;
    type UncleGenerations = UncleGenerations;
    type FilterUncle = ();
    type EventHandler = (Staking, ImOnline);
}

parameter_types! {
	pub const DisabledValidatorsThreshold: Perbill = Perbill::from_percent(17);
}

impl session::Trait for Runtime {
    type Event = Event;
    type ValidatorId = AccountId;
    type ValidatorIdOf = staking::StashOf<Self>;
    type ShouldEndSession = Babe;
    type NextSessionRotation = Babe;
    type SessionManager = Staking;
    type SessionHandler = <SessionKeys as OpaqueKeys>::KeyTypeIdProviders;
    type Keys = SessionKeys;
    type DisabledValidatorsThreshold = DisabledValidatorsThreshold;
}

impl session::historical::Trait for Runtime {
    type FullIdentification = staking::Exposure<AccountId, Balance>;
    type FullIdentificationOf = staking::ExposureOf<Runtime>;
}

parameter_types! {
    // 3 sessions in an era (30 mins).
    pub const SessionsPerEra: SessionIndex = 3;
    // 28 eras for unbonding (14 hours).
    pub const BondingDuration: staking::EraIndex = 28;
    // 28 eras in which slashes can be cancelled (14 hours).
    pub const SlashDeferDuration: staking::EraIndex = 28;
}

/// Simple structure that exposes how u64 currency can be represented as... u64.
pub struct CurrencyToVoteHandler;
impl Convert<u64, u64> for CurrencyToVoteHandler {
    fn convert(x: u64) -> u64 {
        x
    }
}
impl Convert<u128, u128> for CurrencyToVoteHandler {
    fn convert(x: u128) -> u128 {
        x
    }
}
impl Convert<u128, u64> for CurrencyToVoteHandler {
    fn convert(x: u128) -> u64 {
        x.saturated_into()
    }
}

impl Convert<u64, u128> for CurrencyToVoteHandler {
    fn convert(x: u64) -> u128 {
        x as u128
    }
}

impl staking::Trait for Runtime {
    type Currency = Balances;
    type Time = Timestamp;

    type CurrencyToVote = CurrencyToVoteHandler;
    type RewardRemainder = ();
    type Event = Event;
    type Slash = ();
    type Reward = ();
    type SessionsPerEra = SessionsPerEra;
    type BondingDuration = BondingDuration;
    type SlashDeferDuration = SlashDeferDuration;

    // A majority of the council can cancel the slash.
    type SlashCancelOrigin = system::EnsureRoot<Self::AccountId>;
    type SessionInterface = Self;
    type TeeInterface = Self;
}

parameter_types! {
	pub const OffencesWeightSoftLimit: Weight = Perbill::from_percent(60) * MaximumBlockWeight::get();
}

impl offences::Trait for Runtime {
	type Event = Event;
	type IdentificationTuple = session::historical::IdentificationTuple<Self>;
	type OnOffenceHandler = Staking;
	type WeightSoftLimit = OffencesWeightSoftLimit;
}

parameter_types! {
    pub const ExistentialDeposit: u128 = 1 * CENTS;
}

/// TODO: re-think about this
/*pub type DealWithFees = SplitTwoWays<
    Balance,
    NegativeImbalance<Runtime>,
    _4, Treasury,   // 4 parts (80%) goes to the treasury.
    _1, ToAuthor<Runtime>,   // 1 part (20%) goes to the block author.
>;*/

impl balances::Trait for Runtime {
    type Balance = Balance;
    type DustRemoval = ();
    type Event = Event;
    type ExistentialDeposit = ExistentialDeposit;
    type AccountStore = System;
}

// TODO: better way to deal with fee(s)
parameter_types! {
	pub const TransactionBaseFee: Balance = 1 * CENTS;
	pub const TransactionByteFee: Balance = 10 * MILLICENTS;
}

impl transaction_payment::Trait for Runtime {
    type Currency = balances::Module<Runtime>;
    type OnTransactionPayment = ();
    type TransactionByteFee = TransactionByteFee;
    type WeightToFee = IdentityFee<Balance>;
    // TODO: add `TargetedFeeAdjustment` mechanism
    type FeeMultiplierUpdate = ();
}

impl sudo::Trait for Runtime {
    type Event = Event;
    type Call = Call;
}

impl tee::Trait for Runtime {
    type Currency = Balances;
    type Event = Event;
    type Works = Staking;
    type MarketInterface = Market;
}

impl market::Trait for Runtime {
    type Currency = Balances;
    type Event = Event;
    type Randomness = RandomnessCollectiveFlip;
    // TODO: Bonding with balance module(now we impl inside Market)
    type Payment = Payment;
    type OrderInspector = Tee;
}

impl payment::Trait for Runtime {
    type Proposal = Call;
    type Currency = Balances;
    type Event = Event;
    type CurrencyToBalance = CurrencyToVoteHandler;
    type Scheduler = Scheduler;
    // TODO: Bonding with balance module(now we impl inside Market)
    type MarketInterface = Market;
}

construct_runtime! {
    pub enum Runtime where
        Block = Block,
        NodeBlock = opaque::Block,
        UncheckedExtrinsic = UncheckedExtrinsic
    {
        // Basic stuff; balances is uncallable initially.
        System: system::{Module, Call, Storage, Config, Event<T>},
        RandomnessCollectiveFlip: randomness_collective_flip::{Module, Storage},

        // Must be before session.
        Babe: babe::{Module, Call, Storage, Config, Inherent(Timestamp)},

        Timestamp: timestamp::{Module, Call, Storage, Inherent},
        Indices: indices::{Module, Call, Storage, Config<T>, Event<T>},
		Balances: balances::{Module, Call, Storage, Config<T>, Event<T>},
        TransactionPayment: transaction_payment::{Module, Storage},

        // Consensus support
        Authorship: authorship::{Module, Call, Storage},
        Staking: staking::{Module, Call, Storage, Config<T>, Event<T>},
        Historical: session_historical::{Module},
        Session: session::{Module, Call, Storage, Event, Config<T>},
        FinalityTracker: finality_tracker::{Module, Call, Inherent},
        Grandpa: grandpa::{Module, Call, Storage, Config, Event},
        ImOnline: im_online::{Module, Call, Storage, Event<T>, ValidateUnsigned, Config<T>},
        AuthorityDiscovery: authority_discovery::{Module, Call, Config},
        Offences: offences::{Module, Call, Storage, Event},
        Scheduler: scheduler::{Module, Call, Storage, Event<T>},

        // Crust modules
        Tee: tee::{Module, Call, Storage, Event<T>, Config<T>},
        Market: market::{Module, Call, Storage, Event<T>},
        Payment: payment::{Module, Call, Storage, Event<T>},

        // Sudo. Last module. Usable initially, but removed once governance enabled.
        Sudo: sudo::{Module, Call, Storage, Config<T>, Event<T>},
    }
}

/// The address format for describing accounts.
pub type Address = AccountId;
/// Block header type as expected by this runtime.
pub type Header = generic::Header<BlockNumber, BlakeTwo256>;
/// Block type as expected by this runtime.
pub type Block = generic::Block<Header, UncheckedExtrinsic>;
/// A Block signed with a Justification
pub type SignedBlock = generic::SignedBlock<Block>;
/// BlockId type as expected by this runtime.
pub type BlockId = generic::BlockId<Block>;
/// The SignedExtension to the basic transaction logic.
pub type SignedExtra = (
    system::CheckSpecVersion<Runtime>,
    system::CheckTxVersion<Runtime>,
    system::CheckGenesis<Runtime>,
    system::CheckEra<Runtime>,
    system::CheckNonce<Runtime>,
    system::CheckWeight<Runtime>,
    transaction_payment::ChargeTransactionPayment<Runtime>,
);
/// Unchecked extrinsic type as expected by this runtime.
pub type UncheckedExtrinsic = generic::UncheckedExtrinsic<Address, Call, Signature, SignedExtra>;
/// Extrinsic type that has already been checked.
pub type CheckedExtrinsic = generic::CheckedExtrinsic<AccountId, Call, SignedExtra>;
/// Executive: handles dispatch to the various modules.
pub type Executive =
    frame_executive::Executive<Runtime, Block, system::ChainContext<Runtime>, Runtime, AllModules>;

impl_runtime_apis! {
    impl sp_api::Core<Block> for Runtime {
        fn version() -> RuntimeVersion {
            VERSION
        }

        fn execute_block(block: Block) {
            Executive::execute_block(block)
        }

        fn initialize_block(header: &<Block as BlockT>::Header) {
            Executive::initialize_block(header)
        }
    }

    impl sp_api::Metadata<Block> for Runtime {
        fn metadata() -> OpaqueMetadata {
            Runtime::metadata().into()
        }
    }

    impl sp_block_builder::BlockBuilder<Block> for Runtime {
        fn apply_extrinsic(extrinsic: <Block as BlockT>::Extrinsic) -> ApplyExtrinsicResult {
            Executive::apply_extrinsic(extrinsic)
        }

        fn finalize_block() -> <Block as BlockT>::Header {
            Executive::finalize_block()
        }

        fn inherent_extrinsics(data: sp_inherents::InherentData) -> Vec<<Block as BlockT>::Extrinsic> {
            data.create_extrinsics()
        }

        fn check_inherents(
            block: Block,
            data: sp_inherents::InherentData,
        ) -> sp_inherents::CheckInherentsResult {
            data.check_extrinsics(&block)
        }

        fn random_seed() -> <Block as BlockT>::Hash {
            RandomnessCollectiveFlip::random_seed()
        }
    }

    impl sp_transaction_pool::runtime_api::TaggedTransactionQueue<Block> for Runtime {
		fn validate_transaction(
		    source: TransactionSource,
		    tx: <Block as BlockT>::Extrinsic) -> TransactionValidity {
			Executive::validate_transaction(source, tx)
		}
    }
    
    impl sp_offchain::OffchainWorkerApi<Block> for Runtime {
		fn offchain_worker(header: &<Block as BlockT>::Header) {
			Executive::offchain_worker(header)
		}
	}

    impl babe_primitives::BabeApi<Block> for Runtime {
        fn configuration() -> babe_primitives::BabeGenesisConfiguration {
            // The choice of `c` parameter (where `1 - c` represents the
            // probability of a slot being empty), is done in accordance to the
            // slot duration and expected target block time, for safely
            // resisting network delays of maximum two seconds.
            // <https://research.web3.foundation/en/latest/polkadot/BABE/Babe/#6-practical-results>
            babe_primitives::BabeGenesisConfiguration {
                slot_duration: Babe::slot_duration(),
				epoch_length: EpochDuration::get(),
				c: PRIMARY_PROBABILITY,
				genesis_authorities: Babe::authorities(),
				randomness: Babe::randomness(),
				allowed_slots: babe_primitives::AllowedSlots::PrimaryAndSecondaryPlainSlots
            }
        }
        fn current_epoch_start() -> babe_primitives::SlotNumber {
			Babe::current_epoch_start()
		}
    }

    impl fg_primitives::GrandpaApi<Block> for Runtime {
        fn grandpa_authorities() -> GrandpaAuthorityList {
            Grandpa::grandpa_authorities()
        }

        fn submit_report_equivocation_extrinsic(
			equivocation_proof: fg_primitives::EquivocationProof<
				<Block as BlockT>::Hash,
				sp_runtime::traits::NumberFor<Block>,
			>,
			key_owner_proof: fg_primitives::OpaqueKeyOwnershipProof,
		) -> Option<()> {
			let key_owner_proof = key_owner_proof.decode()?;

			Grandpa::submit_report_equivocation_extrinsic(
				equivocation_proof,
				key_owner_proof,
			)
		}

		fn generate_key_ownership_proof(
			_set_id: fg_primitives::SetId,
			authority_id: fg_primitives::AuthorityId,
		) -> Option<fg_primitives::OpaqueKeyOwnershipProof> {
			use codec::Encode;

			Historical::prove((fg_primitives::KEY_TYPE, authority_id))
				.map(|p| p.encode())
				.map(fg_primitives::OpaqueKeyOwnershipProof::new)
		}
    }

    impl authority_discovery_primitives::AuthorityDiscoveryApi<Block> for Runtime {
        fn authorities() -> Vec<AuthorityDiscoveryId> {
            AuthorityDiscovery::authorities()
        }
    }

    impl sp_session::SessionKeys<Block> for Runtime {
        fn generate_session_keys(seed: Option<Vec<u8>>) -> Vec<u8> {
			SessionKeys::generate(seed)
		}

		fn decode_session_keys(
			encoded: Vec<u8>,
		) -> Option<Vec<(Vec<u8>, sp_core::crypto::KeyTypeId)>> {
			SessionKeys::decode_into_raw_public_keys(&encoded)
		}
    }

    // TODO:  enable `system_rpc_runtime_api` and `transaction_payment_rpc_runtime_api`?
}

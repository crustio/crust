// Copyright 2019-2021 Parity Technologies (UK) Ltd.
// This file is part of Cumulus.

// Cumulus is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Cumulus is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Cumulus.  If not, see <http://www.gnu.org/licenses/>.

#![cfg_attr(not(feature = "std"), no_std)]
// `construct_runtime!` does a lot of recursion and requires us to increase the limit to 256.
#![recursion_limit = "256"]

// Make the WASM binary available.
#[cfg(feature = "std")]
include!(concat!(env!("OUT_DIR"), "/wasm_binary.rs"));

mod impls;
mod weights;
pub use crust_parachain_primitives::{
    constants::{currency::*}, traits::*,
    *
};
use sp_api::impl_runtime_apis;
use codec::{Decode, Encode, MaxEncodedLen};
use sp_core::{crypto::KeyTypeId, OpaqueMetadata};
use sp_runtime::{
	create_runtime_str, generic, impl_opaque_keys,
	traits::{BlakeTwo256, Block as BlockT, AccountIdLookup, Convert, SaturatedConversion, Zero},
	transaction_validity::{TransactionSource, TransactionValidity},
	ApplyExtrinsicResult,
};
use sp_std::{cmp::Ordering, prelude::*};
#[cfg(feature = "std")]
use sp_version::NativeVersion;
use sp_version::RuntimeVersion;

// A few exports that help ease life for downstream crates.
use sp_std::marker::PhantomData;
use sp_std::{prelude::*, convert::TryInto, collections::btree_set::BTreeSet};
pub use frame_support::{
	construct_runtime, parameter_types, PalletId, match_type,
	traits::{Randomness, OriginTrait, IsInVec, Everything, InstanceFilter, EnsureOneOf, PrivilegeCmp, Currency},
	weights::{
		constants::{BlockExecutionWeight, ExtrinsicBaseWeight, RocksDbWeight, WEIGHT_PER_SECOND},
		DispatchClass, IdentityFee, Weight,
	},
	StorageValue, RuntimeDebug,
};
use frame_system::limits::{BlockLength, BlockWeights};
use frame_system::{EnsureRoot};
use sp_std::convert::TryFrom;
pub use pallet_balances::Call as BalancesCall;
pub use pallet_timestamp::Call as TimestampCall;
#[cfg(any(feature = "std", test))]
pub use sp_runtime::BuildStorage;
pub use sp_runtime::{Perbill, Permill};

// XCM imports
use polkadot_parachain::primitives::Sibling;
use xcm::latest::prelude::*;
use xcm_builder::{
	AccountId32Aliases, CurrencyAdapter, LocationInverter, ParentIsDefault, RelayChainAsNative,
	SiblingParachainConvertsVia, SignedAccountId32AsNative,
	SovereignSignedViaLocation, EnsureXcmOrigin, AllowUnpaidExecutionFrom, ParentAsSuperuser,
	AllowTopLevelPaidExecutionFrom, TakeWeightCredit, FixedWeightBounds, IsConcrete, NativeAsset,
	UsingComponents, SignedToAccountId32, SiblingParachainAsNative, AllowKnownQueryResponses,
	AllowSubscriptionsFrom
};
use xcm_executor::{
	traits::ConvertOrigin,
	Config, XcmExecutor
};
use pallet_xcm::{XcmPassthrough, EnsureXcm, IsMajorityOfBody};
use xcm::v0::Xcm;
use frame_support::traits::Contains;
pub use sp_consensus_aura::sr25519::AuthorityId as AuraId;
use xcm::v1::{
	prelude::*,
	AssetId::{Abstract, Concrete},
	Fungibility::Fungible,
	MultiAsset, MultiLocation,
};
use sp_runtime::traits::CheckedConversion;
use xcm_executor::traits::{FilterAssetLocation, MatchesFungible};
use impls::{CurrencyToVoteHandler, OneTenthFee, CurrencyAdapter as TransactionFeeCurrencyAdapter};

type NegativeImbalance = <Balances as Currency<AccountId>>::NegativeImbalance;

pub type SessionHandlers = ();

pub mod opaque {
	use super::*;
	pub use sp_runtime::OpaqueExtrinsic as UncheckedExtrinsic;
	/// Opaque block header type.
	pub type Header = generic::Header<BlockNumber, BlakeTwo256>;
	/// Opaque block type.
	pub type Block = generic::Block<Header, UncheckedExtrinsic>;
	/// Opaque block identifier type.
	pub type BlockId = generic::BlockId<Block>;
	impl_opaque_keys! {
		pub struct SessionKeys {
			pub aura: Aura,
		}
	}
}

/// This runtime version.
pub const VERSION: RuntimeVersion = RuntimeVersion {
	spec_name: create_runtime_str!("crust-collator"),
	impl_name: create_runtime_str!("crust-collator"),
	authoring_version: 1,
	spec_version: 8,
	impl_version: 1,
	apis: RUNTIME_API_VERSIONS,
	transaction_version: 1,
	state_version: 0,
};

pub const MILLISECS_PER_BLOCK: u64 = 6000;

pub const SLOT_DURATION: u64 = MILLISECS_PER_BLOCK;

pub const EPOCH_DURATION_IN_BLOCKS: u32 = 10 * MINUTES;

// These time units are defined in number of blocks.
pub const MINUTES: BlockNumber = 60_000 / (MILLISECS_PER_BLOCK as BlockNumber);
pub const HOURS: BlockNumber = MINUTES * 60;
pub const DAYS: BlockNumber = HOURS * 24;

// 1 in 4 blocks (on average, not counting collisions) will be primary babe blocks.
pub const PRIMARY_PROBABILITY: (u64, u64) = (1, 4);

pub const ROC: Balance = 1_000_000_000_000;
pub const MILLIROC: Balance = 1_000_000_000;
pub const MICROROC: Balance = 1_000_000;

/// The version information used to identify this runtime when compiled natively.
#[cfg(feature = "std")]
pub fn native_version() -> NativeVersion {
	NativeVersion {
		runtime_version: VERSION,
		can_author_with: Default::default(),
	}
}

/// We assume that ~10% of the block weight is consumed by `on_initalize` handlers.
/// This is used to limit the maximal weight of a single extrinsic.
const AVERAGE_ON_INITIALIZE_RATIO: Perbill = Perbill::from_percent(10);
/// We allow `Normal` extrinsics to fill up the block up to 75%, the rest can be used
/// by  Operational  extrinsics.
const NORMAL_DISPATCH_RATIO: Perbill = Perbill::from_percent(75);
/// We allow for 2 seconds of compute with a 6 second average block time.
const MAXIMUM_BLOCK_WEIGHT: Weight = 2 * WEIGHT_PER_SECOND;

parameter_types! {
	pub const BlockHashCount: BlockNumber = 250;
	pub const Version: RuntimeVersion = VERSION;
	pub RuntimeBlockLength: BlockLength =
		BlockLength::max_with_normal_ratio(5 * 1024 * 1024, NORMAL_DISPATCH_RATIO);
	pub RuntimeBlockWeights: BlockWeights = BlockWeights::builder()
		.base_block(BlockExecutionWeight::get())
		.for_class(DispatchClass::all(), |weights| {
			weights.base_extrinsic = ExtrinsicBaseWeight::get();
		})
		.for_class(DispatchClass::Normal, |weights| {
			weights.max_total = Some(NORMAL_DISPATCH_RATIO * MAXIMUM_BLOCK_WEIGHT);
		})
		.for_class(DispatchClass::Operational, |weights| {
			weights.max_total = Some(MAXIMUM_BLOCK_WEIGHT);
			// Operational transactions have some extra reserved space, so that they
			// are included even if block reached `MAXIMUM_BLOCK_WEIGHT`.
			weights.reserved = Some(
				MAXIMUM_BLOCK_WEIGHT - NORMAL_DISPATCH_RATIO * MAXIMUM_BLOCK_WEIGHT
			);
		})
		.avg_block_initialization(AVERAGE_ON_INITIALIZE_RATIO)
		.build_or_panic();
	pub const SS58Prefix: u8 = 66;
}

impl frame_system::Config for Runtime {
	/// The identifier used to distinguish between accounts.
	type AccountId = AccountId;
	/// The aggregated dispatch type that is available for extrinsics.
	type Call = Call;
	/// The lookup mechanism to get account ID from whatever is passed in dispatchers.
	type Lookup = AccountIdLookup<AccountId, ()>;
	/// The index type for storing how many extrinsics an account has signed.
	type Index = crust_parachain_primitives::Index;
	/// The index type for blocks.
	type BlockNumber = BlockNumber;
	/// The type for hashing blocks and tries.
	type Hash = Hash;
	/// The hashing algorithm used.
	type Hashing = BlakeTwo256;
	/// The header type.
	type Header = generic::Header<BlockNumber, BlakeTwo256>;
	/// The ubiquitous event type.
	type Event = Event;
	/// The ubiquitous origin type.
	type Origin = Origin;
	/// Maximum number of block number to block hash mappings to keep (oldest pruned first).
	type BlockHashCount = BlockHashCount;
	/// Runtime version.
	type Version = Version;
	/// Converts a module to an index of this module in the runtime.
	type PalletInfo = PalletInfo;
	type AccountData = pallet_balances::AccountData<Balance>;
	type OnNewAccount = ();
	type OnKilledAccount = ();
	type DbWeight = ();
	type BaseCallFilter = frame_support::traits::Everything;
	type SystemWeightInfo = ();
	type BlockWeights = RuntimeBlockWeights;
	type BlockLength = RuntimeBlockLength;
	type SS58Prefix = SS58Prefix;
	type OnSetCode = cumulus_pallet_parachain_system::ParachainSetCode<Self>;
	type MaxConsumers = frame_support::traits::ConstU32<16>;
}

parameter_types! {
	pub const MinimumPeriod: u64 = SLOT_DURATION / 2;
}

impl pallet_timestamp::Config for Runtime {
	/// A timestamp: milliseconds since the unix epoch.
	type Moment = u64;
	type OnTimestampSet = ();
	type MinimumPeriod = MinimumPeriod;
	type WeightInfo = weights::pallet_timestamp::WeightInfo<Runtime>;
}

parameter_types! {
	pub const ExistentialDeposit: u128 = 10 * CENTS;
	pub const TransferFee: u128 = 0;
	pub const CreationFee: u128 = 0;
	pub const TransactionByteFee: u128 = 1;
	pub const MaxLocks: u32 = 50;
	pub const MaxReserves: u32 = 50;
	pub const OperationalFeeMultiplier: u8 = 5;
}

impl pallet_balances::Config for Runtime {
	/// The type for recording an account's balance.
	type Balance = Balance;
	/// The ubiquitous event type.
	type Event = Event;
	type DustRemoval = ();
	type ExistentialDeposit = ExistentialDeposit;
	type AccountStore = System;
	type WeightInfo = weights::pallet_balances::WeightInfo<Runtime>;
	type MaxLocks = MaxLocks;
	type MaxReserves = MaxReserves;
	type ReserveIdentifier = [u8; 8];
}

impl pallet_transaction_payment::Config for Runtime {
	type OnChargeTransaction = pallet_transaction_payment::CurrencyAdapter<Balances, ()>;
	type TransactionByteFee = TransactionByteFee;
	type WeightToFee = OneTenthFee<Balance>;
	type FeeMultiplierUpdate = ();
	type OperationalFeeMultiplier = OperationalFeeMultiplier;
}

parameter_types! {
	// One storage item; key size is 32; value is size 4+4+16+32 bytes = 56 bytes.
	pub const DepositBase: Balance = 100 * CENTS;
	// Additional storage item size of 32 bytes.
	pub const DepositFactor: Balance = 1 * CENTS;
	pub const MaxSignatories: u16 = 100;
}

impl pallet_multisig::Config for Runtime {
	type Event = Event;
	type Call = Call;
	type Currency = Balances;
	type DepositBase = DepositBase;
	type DepositFactor = DepositFactor;
	type MaxSignatories = MaxSignatories;
	type WeightInfo = weights::pallet_multisig::WeightInfo<Runtime>;
}

impl pallet_utility::Config for Runtime {
	type Event = Event;
	type Call = Call;
	type WeightInfo = weights::pallet_utility::WeightInfo<Runtime>;
	type PalletsOrigin = OriginCaller;
}

parameter_types! {
	// One storage item; key size 32, value size 8; .
	pub const ProxyDepositBase: Balance = 100 * CENTS;
	// Additional storage item size of 33 bytes.
	pub const ProxyDepositFactor: Balance = 1 * CENTS;
	pub const MaxProxies: u16 = 32;
	pub const AnnouncementDepositBase: Balance = 100 * CENTS;
	pub const AnnouncementDepositFactor: Balance = 1 * CENTS;
	pub const MaxPending: u16 = 32;
}

/// The type used to represent the kinds of proxying allowed.
#[derive(
	Copy,
	Clone,
	Eq,
	PartialEq,
	Ord,
	PartialOrd,
	Encode,
	Decode,
	RuntimeDebug,
	MaxEncodedLen,
	scale_info::TypeInfo,
)]
pub enum ProxyType {
	/// Fully permissioned proxy. Can execute any call on behalf of _proxied_.
	Any,
	/// Can execute any call that does not transfer funds or assets.
	NonTransfer,
	/// Proxy with the ability to reject time-delay proxy announcements.
	CancelProxy,
	// Collator selection proxy. Can execute calls related to collator selection mechanism.
	Collator,
}
impl Default for ProxyType {
	fn default() -> Self {
		Self::Any
	}
}
impl InstanceFilter<Call> for ProxyType {
	fn filter(&self, c: &Call) -> bool {
		match self {
			ProxyType::Any => true,
			ProxyType::NonTransfer =>
				!matches!(c, Call::Balances { .. }),
			ProxyType::CancelProxy => matches!(
				c,
				Call::Proxy(pallet_proxy::Call::reject_announcement { .. }) |
					Call::Utility { .. } | Call::Multisig { .. }
			),
			ProxyType::Collator => matches!(
				c,
				Call::CollatorSelection { .. } | Call::Utility { .. } | Call::Multisig { .. }
			),
		}
	}
	fn is_superset(&self, o: &Self) -> bool {
		match (self, o) {
			(x, y) if x == y => true,
			(ProxyType::Any, _) => true,
			(_, ProxyType::Any) => false,
			_ => false,
		}
	}
}

impl pallet_proxy::Config for Runtime {
	type Event = Event;
	type Call = Call;
	type Currency = Balances;
	type ProxyType = ProxyType;
	type ProxyDepositBase = ProxyDepositBase;
	type ProxyDepositFactor = ProxyDepositFactor;
	type MaxProxies = MaxProxies;
	type WeightInfo = weights::pallet_proxy::WeightInfo<Runtime>;
	type MaxPending = MaxPending;
	type CallHasher = BlakeTwo256;
	type AnnouncementDepositBase = AnnouncementDepositBase;
	type AnnouncementDepositFactor = AnnouncementDepositFactor;
}

impl pallet_sudo::Config for Runtime {
	type Call = Call;
	type Event = Event;
}

parameter_types! {
	pub MaximumSchedulerWeight: Weight = Perbill::from_percent(80) * RuntimeBlockWeights::get().max_block;
	pub const MaxScheduledPerBlock: u32 = 50;
	pub const NoPreimagePostponement: Option<u32> = Some(10);
}

/// Used the compare the privilege of an origin inside the scheduler.
pub struct OriginPrivilegeCmp;

impl PrivilegeCmp<OriginCaller> for OriginPrivilegeCmp {
	fn cmp_privilege(left: &OriginCaller, right: &OriginCaller) -> Option<Ordering> {
		None
	}
}

impl pallet_scheduler::Config for Runtime {
    type Event = Event;
    type Origin = Origin;
    type PalletsOrigin = OriginCaller;
    type Call = Call;
    type MaximumWeight = MaximumSchedulerWeight;
    type ScheduleOrigin = frame_system::EnsureRoot<AccountId>;
    type MaxScheduledPerBlock = MaxScheduledPerBlock;
    type WeightInfo = ();
	type OriginPrivilegeCmp = OriginPrivilegeCmp;
	type PreimageProvider = ();
	type NoPreimagePostponement = NoPreimagePostponement;
}

parameter_types! {
	pub ReservedXcmpWeight: Weight = RuntimeBlockWeights::get().max_block / 4;
	pub ReservedDmpWeight: Weight = RuntimeBlockWeights::get().max_block / 4;
}

impl cumulus_pallet_parachain_system::Config for Runtime {
	type Event = Event;
	type OnSystemEvent = ();
	type SelfParaId = parachain_info::Pallet<Runtime>;
	type OutboundXcmpMessageSource = XcmpQueue;
	type DmpMessageHandler = DmpQueue;
	type ReservedDmpWeight = ReservedDmpWeight;
	type XcmpMessageHandler = XcmpQueue;
	type ReservedXcmpWeight = ReservedXcmpWeight;
}

impl parachain_info::Config for Runtime {}

parameter_types! {
	pub const RococoLocation: MultiLocation = MultiLocation::parent();
	pub const LocalTestNetwork: MultiLocation = MultiLocation { parents: 1, interior: X1(Parachain(2012)) };
	pub const RococoNetwork: NetworkId = NetworkId::Kusama;
	pub RelayChainOrigin: Origin = cumulus_pallet_xcm::Origin::Relay.into();
	pub Ancestry: MultiLocation = Parachain(ParachainInfo::parachain_id().into()).into();
}

type LocationToAccountId = (
	ParentIsDefault<AccountId>,
	SiblingParachainConvertsVia<Sibling, AccountId>,
	AccountId32Aliases<RococoNetwork, AccountId>,
);

pub struct AllowedList;

impl AllowedList {
	fn is_allowed(id: u32) -> bool {
		match id {
			2008 => true, // Local testnet
			2012 => true, // Local testnet
			2000 => true, // Acala
			2004 => true, // Phala
			2003 => true, // Reserved
			_ => false
		}
	}
}


pub struct IsFromSiblingParachain;
impl Contains<MultiLocation> for IsFromSiblingParachain {
	fn contains(id: &MultiLocation) -> bool {
		match id {
			MultiLocation { parents: 1, interior: X1(Junction::Parachain(id)) } if AllowedList::is_allowed(id.clone()) => true,
			_ => false
		}
	}
}

pub struct IsSiblingParachainsConcrete<T>(PhantomData<T>);
impl<T: Contains<MultiLocation>, B: TryFrom<u128>> MatchesFungible<B>
	for IsSiblingParachainsConcrete<T>
{
	fn matches_fungible(a: &MultiAsset) -> Option<B> {
		match (&a.id, &a.fun) {
			(Concrete(ref id), Fungible(ref amount)) if T::contains(id) => {
				CheckedConversion::checked_from(*amount)
			}
			_ => None,
		}
	}
}

type LocalAssetTransactor = CurrencyAdapter<
	// Use this currency:
	Balances,
	// Use this currency when it is a fungible asset matching the given location or name:
	IsSiblingParachainsConcrete<IsFromSiblingParachain>,
	// Do a simple punn to convert an AccountId32 MultiLocation into a native chain account ID:
	LocationToAccountId,
	// Our chain's account ID type (we can't get away without mentioning it explicitly):
	AccountId,
	// We don't track any teleports.
	(),
>;

pub struct IsAllowedToCrust<Origin>(PhantomData<Origin>);
impl<
	Origin: OriginTrait
> ConvertOrigin<Origin> for IsAllowedToCrust<Origin> {
	fn convert_origin(origin: impl Into<MultiLocation>, kind: OriginKind) -> Result<Origin, MultiLocation> {
		match (kind, origin.into()) {
			(
				OriginKind::Superuser,
				MultiLocation { parents: 0, interior: X1(Junction::Parachain(id)) },
			) if AllowedList::is_allowed(id.into()) => Ok(Origin::root()),
			(_, origin) => Err(origin),
		}
	}
}


pub type XcmOriginToTransactDispatchOrigin = (
	IsAllowedToCrust<Origin>,
	SovereignSignedViaLocation<LocationToAccountId, Origin>,
	RelayChainAsNative<RelayChainOrigin, Origin>,
	SiblingParachainAsNative<cumulus_pallet_xcm::Origin, Origin>,
	ParentAsSuperuser<Origin>,
	SignedAccountId32AsNative<RococoNetwork, Origin>,
	// Xcm origins can be represented natively under the Xcm pallet's Xcm origin.
	XcmPassthrough<Origin>,
);

parameter_types! {
	// One XCM operation is 1_000_000_000 weight - almost certainly a conservative estimate.
	pub UnitWeightCost: Weight = 1_000_000_000;
	// One ROC buys 1 second of weight.
	pub const WeightPrice: (MultiLocation, u128) = (MultiLocation::parent(), ROC);
	pub const MaxInstructions: u32 = 100;
}

match_type! {
	pub type ParentOrParentsUnitPlurality: impl Contains<MultiLocation> = {
		MultiLocation { parents: 1, interior: Here } |
		MultiLocation { parents: 1, interior: X1(Plurality { id: BodyId::Unit, .. }) }
	};
}

pub type Barrier = (
	TakeWeightCredit,
	AllowTopLevelPaidExecutionFrom<Everything>,
	AllowUnpaidExecutionFrom<ParentOrParentsUnitPlurality>,	// <- Parent gets free execution
	// Expected responses are OK.
	AllowKnownQueryResponses<PolkadotXcm>,
	// Subscriptions for version tracking are OK.
	AllowSubscriptionsFrom<Everything>,
);

pub struct XcmConfig;
impl Config for XcmConfig {
	type Call = Call;
	type XcmSender = XcmRouter;
	// How to withdraw and deposit an asset.
	type AssetTransactor = LocalAssetTransactor;
	type OriginConverter = XcmOriginToTransactDispatchOrigin;
	type IsReserve = NativeAsset;
	type IsTeleporter = NativeAsset;
	type LocationInverter = LocationInverter<Ancestry>;
	type Barrier = Barrier;
	type Weigher = FixedWeightBounds<UnitWeightCost, Call, MaxInstructions>;
	type Trader = UsingComponents<IdentityFee<Balance>, RococoLocation, AccountId, Balances, ()>;
	type ResponseHandler = PolkadotXcm;
	type AssetTrap = PolkadotXcm;
	type AssetClaims = PolkadotXcm;
	type SubscriptionService = PolkadotXcm;
}

pub type LocalOriginToLocation = (
	SignedToAccountId32<Origin, AccountId, RococoNetwork>,
);

/// The means for routing XCM messages which are not for local execution into the right message
/// queues.
pub type XcmRouter = (
	// Two routers - use UMP to communicate with the relay chain:
	cumulus_primitives_utility::ParentAsUmp<ParachainSystem, ()>,
	// ..and XCMP to communicate with the sibling chains.
	XcmpQueue,
);

impl pallet_xcm::Config for Runtime {
	type Event = Event;
	type SendXcmOrigin = EnsureXcmOrigin<Origin, LocalOriginToLocation>;
	type XcmRouter = XcmRouter;
	type ExecuteXcmOrigin = EnsureXcmOrigin<Origin, LocalOriginToLocation>;
	type XcmExecuteFilter = Everything;
	type XcmExecutor = XcmExecutor<XcmConfig>;
	type XcmTeleportFilter = Everything;
	type XcmReserveTransferFilter = Everything;
	type Weigher = FixedWeightBounds<UnitWeightCost, Call, MaxInstructions>;
	type LocationInverter = LocationInverter<Ancestry>;
	type Origin = Origin;
	type Call = Call;
	const VERSION_DISCOVERY_QUEUE_SIZE: u32 = 100;
	type AdvertisedXcmVersion = pallet_xcm::CurrentXcmVersion;
}

impl orml_xcm::Config for Runtime {
	type Event = Event;
	type SovereignOrigin = frame_system::EnsureRoot<AccountId>;
}

impl cumulus_pallet_xcm::Config for Runtime {
	type Event = Event;
	type XcmExecutor = XcmExecutor<XcmConfig>;
}

impl cumulus_pallet_dmp_queue::Config for Runtime {
	type Event = Event;
	type XcmExecutor = XcmExecutor<XcmConfig>;
	type ExecuteOverweightOrigin = frame_system::EnsureRoot<AccountId>;
}

impl cumulus_pallet_xcmp_queue::Config for Runtime {
	type Event = Event;
	type XcmExecutor = XcmExecutor<XcmConfig>;
	type ChannelInfo = ParachainSystem;
	type VersionWrapper = PolkadotXcm;
	type ExecuteOverweightOrigin = frame_system::EnsureRoot<AccountId>;
}

impl cumulus_ping::Config for Runtime {
	type Event = Event;
	type Origin = Origin;
	type Call = Call;
	type XcmSender = XcmRouter;
}

// parameter_types! {
//     /// Unit is pico
//     pub const MarketPalletId: PalletId = PalletId(*b"crmarket");
//     pub const FileDuration: BlockNumber = 30 * DAYS;
//     pub const FileReplica: u32 = 4;
//     pub const FileBaseFee: Balance = MILLICENTS * 1;
//     pub const FileInitPrice: Balance = MILLICENTS / 1000; // Need align with FileDuration and FileReplica
//     pub const StorageReferenceRatio: (u128, u128) = (25, 100); // 25/100 = 25%
//     pub StorageIncreaseRatio: Perbill = Perbill::from_rational_approximation(1u64, 10000);
//     pub StorageDecreaseRatio: Perbill = Perbill::from_rational_approximation(5u64, 10000);
//     pub const StakingRatio: Perbill = Perbill::from_percent(80);
//     pub const TaxRatio: Perbill = Perbill::from_percent(10);
//     pub const UsedTrashMaxSize: u128 = 1_000;
//     pub const MaximumFileSize: u64 = 137_438_953_472; // 128G = 128 * 1024 * 1024 * 1024
//     pub const RenewRewardRatio: Perbill = Perbill::from_percent(5);
// }

// impl market::Config for Runtime {
//     /// The market's module id, used for deriving its sovereign account ID.
//     type PalletId = MarketPalletId;
//     type Currency = Balances;
//     type CurrencyToBalance = CurrencyToVoteHandler;
//     type SworkerInterface = Market;
//     type Event = Event;
//     /// File duration.
//     type FileDuration = FileDuration;
//     type FileReplica = FileReplica;
//     type FileBaseFee = FileBaseFee;
//     type FileInitPrice = FileInitPrice;
//     type StorageReferenceRatio = StorageReferenceRatio;
//     type StorageIncreaseRatio = StorageIncreaseRatio;
//     type StorageDecreaseRatio = StorageDecreaseRatio;
//     type StakingRatio = StakingRatio;
//     type TaxRatio = TaxRatio;
// 	type RenewRewardRatio = RenewRewardRatio;
//     type UsedTrashMaxSize = UsedTrashMaxSize;
//     type WeightInfo = market::weight::WeightInfo<Runtime>;
//     type MaximumFileSize = MaximumFileSize;
// }

// impl xstorage::Config for Runtime {
// 	type XcmpMessageSender = XcmRouter;
// }

parameter_types! {
	pub const UncleGenerations: u32 = 0;
}

impl pallet_authorship::Config for Runtime {
	type FindAuthor = pallet_session::FindAccountFromAuthorIndex<Self, Aura>;
	type UncleGenerations = UncleGenerations;
	type FilterUncle = ();
	type EventHandler = (CollatorSelection,);
}

parameter_types! {
	pub const Period: u32 = 6 * HOURS;
	pub const Offset: u32 = 0;
	pub const MaxAuthorities: u32 = 100_000;
}

impl pallet_session::Config for Runtime {
	type Event = Event;
	type ValidatorId = <Self as frame_system::Config>::AccountId;
	// we don't have stash and controller, thus we don't need the convert as well.
	type ValidatorIdOf = pallet_collator_selection::IdentityCollator;
	type ShouldEndSession = pallet_session::PeriodicSessions<Period, Offset>;
	type NextSessionRotation = pallet_session::PeriodicSessions<Period, Offset>;
	type SessionManager = CollatorSelection;
	// Essentially just Aura, but lets be pedantic.
	type SessionHandler = <opaque::SessionKeys as sp_runtime::traits::OpaqueKeys>::KeyTypeIdProviders;
	type Keys = opaque::SessionKeys;
	type WeightInfo = ();
}

impl pallet_aura::Config for Runtime {
	type AuthorityId = AuraId;
	type DisabledValidators = ();
	type MaxAuthorities = MaxAuthorities;
}

impl cumulus_pallet_aura_ext::Config for Runtime {}

impl pallet_randomness_collective_flip::Config for Runtime {}

parameter_types! {
	pub const PotId: PalletId = PalletId(*b"PotStake");
	pub const MaxCandidates: u32 = 1000;
	pub const SessionLength: BlockNumber = 6 * HOURS;
	pub const MaxInvulnerables: u32 = 100;
	pub const ExecutiveBody: BodyId = BodyId::Executive;
}

/// We allow root and the Relay Chain council to execute privileged collator selection operations.
pub type CollatorSelectionUpdateOrigin = EnsureOneOf<
	EnsureRoot<AccountId>,
	EnsureXcm<IsMajorityOfBody<RococoLocation, ExecutiveBody>>,
>;

impl pallet_collator_selection::Config for Runtime {
	type Event = Event;
	type Currency = Balances;
	type UpdateOrigin = CollatorSelectionUpdateOrigin;
	type PotId = PotId;
	type MaxCandidates = MaxCandidates;
	type MaxInvulnerables = MaxInvulnerables;
	// should be a multiple of session or things will get inconsistent
	type KickThreshold = Period;
	type WeightInfo = weights::pallet_collator_selection::WeightInfo<Runtime>;
}

// pub struct MockMarket;

// impl MarketInterface<AccountId, Balance> for MockMarket {
// 	// used for `added_files`
// 	// return real spower of this file and whether this file is in the market system
// 	fn upsert_replica(_: &AccountId, _: AccountId, _: &MerkleRoot, _: u64, _: &SworkerAnchor, _: BlockNumber, _: &Option<BTreeSet<AccountId>>) -> (u64, bool) {
// 		(0, false)
// 	}
// 	// used for `delete_files`
// 	// return real spower of this file and whether this file is in the market system
// 	fn delete_replica(_: &AccountId, _: AccountId, _: &MerkleRoot, _: &SworkerAnchor) -> (u64, bool) {
// 		(0, false)
// 	}
// 	// used for distribute market staking payout
// 	fn withdraw_staking_pot() -> Balance {
// 		Zero::zero()
// 	}
// }

// parameter_types! {
//     pub const StakingPalletId: PalletId = PalletId(*b"cstaking");
//     // 112 eras for unbonding (28 days).
//     pub const BondingDuration: EraIndex = 28 * 4;
//     // 108 eras in which slashes can be cancelled (slightly less than 28 days).
//     pub const SlashDeferDuration: EraIndex = 27 * 4;
//     // 1 * CRUs / TB, since we treat 1 TB = 1_000_000_000_000, so the ratio = `1`
//     pub const SPowerRatio: u128 = 1;
//     // 64 guarantors for one validator.
//     pub const MaxGuarantorRewardedPerValidator: u32 = 64;
//     // 60 eras means 15 days if era = 6 hours
//     pub const MarketStakingPotDuration: u32 = 60;
//     // free transfer amount for other locks
//     pub const UncheckedFrozenBondFund: Balance = 1 * DOLLARS;
// }

// impl staking::Config for Runtime {
//     type PalletId = StakingPalletId;
//     type Currency = Balances;
//     type UnixTime = Timestamp;

//     type CurrencyToVote = CurrencyToVoteHandler;
//     type RewardRemainder = ();
//     type Event = Event;
//     type Reward = ();
//     type Randomness = RandomnessCollectiveFlip;
//     type BondingDuration = BondingDuration;
//     type MaxGuarantorRewardedPerValidator = MaxGuarantorRewardedPerValidator;

//     // A majority of the council can cancel the slash.
//     type SPowerRatio = SPowerRatio;
//     type MarketStakingPot = MockMarket;
//     type MarketStakingPotDuration = MarketStakingPotDuration;
//     type BenefitInterface = Benefits;
//     type UncheckedFrozenBondFund = UncheckedFrozenBondFund;
//     type WeightInfo = staking::weight::WeightInfo;
// }

// parameter_types! {
//     pub const PunishmentSlots: u32 = 8; // 8 report slot == 8 hours
//     pub const MaxGroupSize: u32 = 1000;
// }

// impl swork::Config for Runtime {
//     type Currency = Balances;
//     type Event = Event;
//     type PunishmentSlots = PunishmentSlots;
//     type Works = Staking;
//     type MarketInterface = MockMarket;
//     type MaxGroupSize = MaxGroupSize;
//     type BenefitInterface = Benefits;
//     type WeightInfo = swork::weight::WeightInfo<Runtime>;
// }

// parameter_types! {
//     pub const BenefitReportWorkCost: Balance = 3 * DOLLARS;
//     pub BenefitsLimitRatio: Perbill = Perbill::from_rational_approximation(2u64, 1000);
//     pub const BenefitMarketCostRatio: Perbill = Perbill::one();
// }

// impl benefits::Config for Runtime {
//     type Event = Event;
//     type Currency = Balances;
//     type BenefitReportWorkCost = BenefitReportWorkCost;
//     type BenefitsLimitRatio = BenefitsLimitRatio;
//     type BenefitMarketCostRatio = BenefitMarketCostRatio;
//     type BondingDuration = BondingDuration;
//     type WeightInfo = benefits::weight::WeightInfo<Runtime>;
// }

parameter_types! {
    pub const BridgeClaimsPalletId: PalletId = PalletId(*b"crclaims");
    pub Prefix: &'static [u8] = b"Pay CSMs to the Crust Shadow account:";
}

impl claims::Config for Runtime {
    type PalletId = BridgeClaimsPalletId;
    type Event = Event;
    type Currency = Balances;
    type Prefix = Prefix;
}

parameter_types! {
    pub const BridgeChainId: u8 = 3;
    pub const ProposalLifetime: BlockNumber = 50400; // ~7 days
}

type MoreThanHalfCouncil = EnsureRoot<AccountId>;

impl bridge::Config for Runtime {
	type PalletId = BridgeClaimsPalletId;
    type Event = Event;
    type BridgeCommitteeOrigin = MoreThanHalfCouncil;
    type Proposal = Call;
    type BridgeChainId = BridgeChainId;
    type ProposalLifetime = ProposalLifetime;
}

parameter_types! {
    // bridge::derive_resource_id(1, &bridge::hashing::blake2_128(b"CSM"));
    pub const BridgeTokenId: [u8; 32] = hex_literal::hex!("00000000000000000000000000000098aef84ac01d96413445cf3dc4d5c44c01");
}

impl bridge_transfer::Config for Runtime {
    type Event = Event;
    type BridgeOrigin = bridge::EnsureBridge<Runtime>;
    type Currency = Balances;
    type BridgeTokenId = BridgeTokenId;
}

construct_runtime! {
	pub enum Runtime where
		Block = Block,
		NodeBlock = crust_parachain_primitives::Block,
		UncheckedExtrinsic = UncheckedExtrinsic,
	{
		System: frame_system::{Pallet, Call, Storage, Config, Event<T>},
		Timestamp: pallet_timestamp::{Pallet, Call, Storage, Inherent},
		Balances: pallet_balances::{Pallet, Call, Storage, Config<T>, Event<T>},
		Sudo: pallet_sudo::{Pallet, Call, Storage, Config<T>, Event<T>},
		RandomnessCollectiveFlip: pallet_randomness_collective_flip::{Pallet, Storage},
		ParachainSystem: cumulus_pallet_parachain_system::{Pallet, Call, Storage, Inherent, Event<T>, ValidateUnsigned, Config},
		TransactionPayment: pallet_transaction_payment::{Pallet, Storage},
		ParachainInfo: parachain_info::{Pallet, Storage, Config, Call},
		Scheduler: pallet_scheduler::{Pallet, Call, Storage, Event<T>},

		// Collator support. the order of these 4 are important and shall not change.
		Authorship: pallet_authorship::{Pallet, Call, Storage},
		CollatorSelection: pallet_collator_selection::{Pallet, Call, Storage, Event<T>, Config<T>},
		Session: pallet_session::{Pallet, Call, Storage, Event, Config<T>},
		Aura: pallet_aura::{Pallet, Storage, Config<T>},
		AuraExt: cumulus_pallet_aura_ext::{Pallet, Storage, Config},

		// XCM helpers.
		XcmpQueue: cumulus_pallet_xcmp_queue::{Pallet, Call, Storage, Event<T>},
		PolkadotXcm: pallet_xcm::{Pallet, Call, Storage, Event<T>, Origin},
		CumulusXcm: cumulus_pallet_xcm::{Pallet, Call, Event<T>, Origin},
		DmpQueue: cumulus_pallet_dmp_queue::{Pallet, Call, Storage, Event<T>},
		// Xstorage: xstorage::{Pallet, Storage, Call},
		OrmlXcm: orml_xcm::{Pallet, Call, Event<T>},

		Utility: pallet_utility::{Pallet, Call, Event},
		Proxy: pallet_proxy::{Pallet, Call, Storage, Event<T>},
		Multisig: pallet_multisig::{Pallet, Call, Storage, Event<T>},

		Spambot: cumulus_ping::{Pallet, Call, Storage, Event<T>} = 99,
		// Market: market::{Pallet, Call, Storage, Event<T>, Config} = 100,

		// // Crust modules
		// Staking: staking::{Pallet, Call, Storage, Event<T>} = 110,
		// Swork: swork::{Pallet, Call, Storage, Event<T>} = 111,
		Claims: claims::{Pallet, Call, Storage, Event<T>, ValidateUnsigned} = 112,
		// Benefits: benefits::{Pallet, Call, Storage, Event<T>} = 113,

		// ChainBridge
		ChainBridge: bridge::{Pallet, Call, Storage, Event<T>} = 114,
		BridgeTransfer: bridge_transfer::{Pallet, Call, Event<T>, Storage} = 115,
	}
}

/// The address format for describing accounts.
pub type Address = sp_runtime::MultiAddress<AccountId, ()>;
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
	frame_system::CheckSpecVersion<Runtime>,
	frame_system::CheckGenesis<Runtime>,
	frame_system::CheckEra<Runtime>,
	frame_system::CheckNonce<Runtime>,
	frame_system::CheckWeight<Runtime>,
	pallet_transaction_payment::ChargeTransactionPayment<Runtime>,
);
/// Unchecked extrinsic type as expected by this runtime.
pub type UncheckedExtrinsic = generic::UncheckedExtrinsic<Address, Call, Signature, SignedExtra>;
/// Extrinsic type that has already been checked.
pub type CheckedExtrinsic = generic::CheckedExtrinsic<AccountId, Call, SignedExtra>;
/// Executive: handles dispatch to the various modules.
pub type Executive = frame_executive::Executive<
	Runtime,
	Block,
	frame_system::ChainContext<Runtime>,
	Runtime,
	AllPalletsWithSystem,
>;

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
			OpaqueMetadata::new(Runtime::metadata().into())
		}
	}

	impl sp_block_builder::BlockBuilder<Block> for Runtime {
		fn apply_extrinsic(
			extrinsic: <Block as BlockT>::Extrinsic,
		) -> ApplyExtrinsicResult {
			Executive::apply_extrinsic(extrinsic)
		}

		fn finalize_block() -> <Block as BlockT>::Header {
			Executive::finalize_block()
		}

		fn inherent_extrinsics(data: sp_inherents::InherentData) -> Vec<<Block as BlockT>::Extrinsic> {
			data.create_extrinsics()
		}

		fn check_inherents(block: Block, data: sp_inherents::InherentData) -> sp_inherents::CheckInherentsResult {
			data.check_extrinsics(&block)
		}
	}

	impl sp_transaction_pool::runtime_api::TaggedTransactionQueue<Block> for Runtime {
		fn validate_transaction(
			source: TransactionSource,
			tx: <Block as BlockT>::Extrinsic,
			block_hash: <Block as BlockT>::Hash,
		) -> TransactionValidity {
			Executive::validate_transaction(source, tx, block_hash)
		}
	}

	impl sp_offchain::OffchainWorkerApi<Block> for Runtime {
		fn offchain_worker(header: &<Block as BlockT>::Header) {
			Executive::offchain_worker(header)
		}
	}

	impl sp_session::SessionKeys<Block> for Runtime {
		fn generate_session_keys(seed: Option<Vec<u8>>) -> Vec<u8> {
			opaque::SessionKeys::generate(seed)
		}

		fn decode_session_keys(
			encoded: Vec<u8>,
		) -> Option<Vec<(Vec<u8>, KeyTypeId)>> {
			opaque::SessionKeys::decode_into_raw_public_keys(&encoded)
		}
	}

	impl sp_consensus_aura::AuraApi<Block, AuraId> for Runtime {
		fn slot_duration() -> sp_consensus_aura::SlotDuration {
			sp_consensus_aura::SlotDuration::from_millis(Aura::slot_duration())
		}

		fn authorities() -> Vec<AuraId> {
			Aura::authorities().into_inner()
		}
	}

	impl cumulus_primitives_core::CollectCollationInfo<Block> for Runtime {
		fn collect_collation_info(header: &<Block as BlockT>::Header) -> cumulus_primitives_core::CollationInfo {
			ParachainSystem::collect_collation_info(header)
		}
	}
}

struct CheckInherents;

impl cumulus_pallet_parachain_system::CheckInherents<Block> for CheckInherents {
	fn check_inherents(
		block: &Block,
		relay_state_proof: &cumulus_pallet_parachain_system::RelayChainStateProof,
	) -> sp_inherents::CheckInherentsResult {
		let relay_chain_slot = relay_state_proof
			.read_slot()
			.expect("Could not read the relay chain slot from the proof");

		let inherent_data =
			cumulus_primitives_timestamp::InherentDataProvider::from_relay_chain_slot_and_duration(
				relay_chain_slot,
				sp_std::time::Duration::from_secs(6),
			)
			.create_inherent_data()
			.expect("Could not create the timestamp inherent data");

		inherent_data.check_extrinsics(&block)
	}
}

cumulus_pallet_parachain_system::register_validate_block! {
	Runtime = Runtime,
	BlockExecutor = cumulus_pallet_aura_ext::BlockExecutor::<Runtime, Executive>,
	CheckInherents = CheckInherents,
}

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
use sp_core::{
    u32_trait::{_1, _2, _3, _4, _5}
};
use sp_api::impl_runtime_apis;
use codec::{Decode, Encode, MaxEncodedLen};
use sp_core::{crypto::KeyTypeId, OpaqueMetadata};
use sp_runtime::{
	create_runtime_str, generic, impl_opaque_keys,
	traits::{BlakeTwo256, Block as BlockT, AccountIdLookup, Convert, SaturatedConversion, Zero},
	transaction_validity::{TransactionSource, TransactionValidity},
	ApplyExtrinsicResult, FixedPointNumber, Perbill, Percent, Permill, Perquintill,
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
	traits::{
		Randomness, OriginTrait, IsInVec, Everything, InstanceFilter, Imbalance,
		LockIdentifier, EnsureOneOf, PrivilegeCmp, Currency, OnUnbalanced
	},
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
use cumulus_primitives_core::ParaId;
use frame_support::pallet_prelude::*;
use pallet_transaction_payment::RuntimeDispatchInfo;

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
	spec_version: 11,
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

pub struct DealWithFees;
impl OnUnbalanced<NegativeImbalance> for DealWithFees {
    fn on_unbalanceds<B>(mut fees_then_tips: impl Iterator<Item = NegativeImbalance>) {
        if let Some(mut fees) = fees_then_tips.next() {
            if let Some(tips) = fees_then_tips.next() {
                tips.merge_into(&mut fees);
            }
            Treasury::on_unbalanced(fees);
        }
    }
}

impl pallet_transaction_payment::Config for Runtime {
	type OnChargeTransaction = pallet_transaction_payment::CurrencyAdapter<Balances, DealWithFees>;
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
	pub const LocalTestNetwork: MultiLocation = MultiLocation::here();
	pub const RococoNetwork: NetworkId = NetworkId::Kusama;
	pub RelayChainOrigin: Origin = cumulus_pallet_xcm::Origin::Relay.into();
	pub Ancestry: MultiLocation = Parachain(ParachainInfo::parachain_id().into()).into();
}

type LocationToAccountId = (
	ParentIsDefault<AccountId>,
	SiblingParachainConvertsVia<Sibling, AccountId>,
	AccountId32Aliases<RococoNetwork, AccountId>,
);

pub trait NativeAssetChecker {
	fn is_native_asset(asset: &MultiAsset) -> bool;
	fn is_native_asset_id(id: &MultiLocation) -> bool;
	fn native_asset_id() -> MultiLocation;
}

pub struct NativeAssetFilter<T>(PhantomData<T>);
impl<T: Get<ParaId>> NativeAssetChecker for NativeAssetFilter<T> {
	fn is_native_asset(asset: &MultiAsset) -> bool {
		match (&asset.id, &asset.fun) {
			// So far our native asset is concrete
			(Concrete(ref id), Fungible(_)) if Self::is_native_asset_id(id) => true,
			_ => false,
		}
	}

	fn is_native_asset_id(id: &MultiLocation) -> bool {
		let native_locations = [
			MultiLocation::here(),
			(1, X1(Parachain(T::get().into()))).into(),
		];
		native_locations.contains(id)
	}

	fn native_asset_id() -> MultiLocation {
		(1, X1(Parachain(T::get().into()))).into()
	}
}


pub struct NativeAssetMatcher<C>(PhantomData<C>);
impl<C: NativeAssetChecker, B: TryFrom<u128>> MatchesFungible<B> for NativeAssetMatcher<C> {
	fn matches_fungible(a: &MultiAsset) -> Option<B> {
		match (&a.id, &a.fun) {
			(Concrete(_), Fungible(ref amount)) if C::is_native_asset(a) => {
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
	NativeAssetMatcher<NativeAssetFilter<ParachainInfo>>,
	// Do a simple punn to convert an AccountId32 MultiLocation into a native chain account ID:
	LocationToAccountId,
	// Our chain's account ID type (we can't get away without mentioning it explicitly):
	AccountId,
	// We don't track any teleports.
	(),
>;


pub type XcmOriginToTransactDispatchOrigin = (
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
	type Trader = UsingComponents<IdentityFee<Balance>, LocalTestNetwork, AccountId, Balances, ()>;
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

type EnsureRootOrHalfCouncil = EnsureOneOf<
    EnsureRoot<AccountId>,
    pallet_collective::EnsureProportionMoreThan<_1, _2, AccountId, CouncilCollective>,
>;

parameter_types! {
    pub const CouncilMotionDuration: BlockNumber = 7 * DAYS;
    pub const CouncilMaxProposals: u32 = 100;
    pub const CouncilMaxMembers: u32 = 100;
}

type CouncilCollective = pallet_collective::Instance1;
impl pallet_collective::Config<CouncilCollective> for Runtime {
    type Origin = Origin;
    type Proposal = Call;
    type Event = Event;
    type MotionDuration = CouncilMotionDuration;
    type MaxProposals = CouncilMaxProposals;
    type MaxMembers = CouncilMaxMembers;
    type DefaultVote = pallet_collective::PrimeDefaultVote;
    type WeightInfo = pallet_collective::weights::SubstrateWeight<Runtime>;
}

parameter_types! {
    pub const CandidacyBond: Balance = 100 * DOLLARS;
    // 1 storage item created, key size is 32 bytes, value size is 16+16.
    pub const VotingBondBase: Balance = 10 * DOLLARS;
    // additional data per vote is 32 bytes (account id).
    pub const VotingBondFactor: Balance = 1 * DOLLARS;
    /// Daily council elections
    pub const TermDuration: BlockNumber = 3 * DAYS;
    pub const DesiredMembers: u32 = 7;
    pub const DesiredRunnersUp: u32 = 6;
    pub const PhragmenElectionPalletId: LockIdentifier = *b"phrelect";
}

// // Make sure that there are no more than MaxMembers members elected via phragmen.
// const_assert!(DesiredMembers::get() <= CouncilMaxMembers::get());

impl pallet_elections_phragmen::Config for Runtime {
    type Event = Event;
    type Currency = Balances;
    type ChangeMembers = Council;
    type InitializeMembers = Council;
    type CurrencyToVote = frame_support::traits::U128CurrencyToVote;
    type CandidacyBond = CandidacyBond;
    type VotingBondBase = VotingBondBase;
    type VotingBondFactor = VotingBondFactor;
    type LoserCandidate = Treasury;
    type KickedMember = Treasury;
    type DesiredMembers = DesiredMembers;
    type DesiredRunnersUp = DesiredRunnersUp;
    type TermDuration = TermDuration;
    type PalletId = PhragmenElectionPalletId;
    type WeightInfo = pallet_elections_phragmen::weights::SubstrateWeight<Runtime>;
}

parameter_types! {
    pub const TechnicalMotionDuration: BlockNumber = 3 * DAYS;
    pub const TechnicalMaxProposals: u32 = 100;
    pub const TechnicalMaxMembers: u32 = 100;
}

type TechnicalCollective = pallet_collective::Instance2;
impl pallet_collective::Config<TechnicalCollective> for Runtime {
    type Origin = Origin;
    type Proposal = Call;
    type Event = Event;
    type MotionDuration = TechnicalMotionDuration;
    type MaxProposals = TechnicalMaxProposals;
    type MaxMembers = TechnicalMaxMembers;
    type DefaultVote = pallet_collective::PrimeDefaultVote;
    type WeightInfo = pallet_collective::weights::SubstrateWeight<Runtime>;
}

impl pallet_membership::Config<pallet_membership::Instance1> for Runtime {
    type Event = Event;
    type AddOrigin = EnsureRootOrHalfCouncil;
    type RemoveOrigin = EnsureRootOrHalfCouncil;
    type SwapOrigin = EnsureRootOrHalfCouncil;
    type ResetOrigin = EnsureRootOrHalfCouncil;
    type PrimeOrigin = EnsureRootOrHalfCouncil;
    type MembershipInitialized = TechnicalCommittee;
    type MembershipChanged = TechnicalCommittee;
    type MaxMembers = TechnicalMaxMembers;
    type WeightInfo = pallet_membership::weights::SubstrateWeight<Runtime>;
}

parameter_types! {
    pub const ProposalBond: Permill = Permill::from_percent(5);
    pub const ProposalBondMinimum: Balance = 100 * DOLLARS;
    pub const SpendPeriod: BlockNumber = 7 * DAYS;
	pub const Burn: Permill = Permill::from_perthousand(2);
    pub const TipCountdown: BlockNumber = 1 * DAYS;
    pub const TipFindersFee: Percent = Percent::from_percent(20);
    pub const TipReportDepositBase: Balance = 10 * DOLLARS;
    pub const DataDepositPerByte: Balance = 10 * CENTS;
    pub const TreasuryPalletId: PalletId = PalletId(*b"py/trsry");
    pub const MaxApprovals: u32 = 100;
	pub const MaximumReasonLength: u32 = 16384;
}

impl pallet_treasury::Config for Runtime {
    type PalletId = TreasuryPalletId;
    type Currency = Balances;
    type ApproveOrigin = EnsureOneOf<
        EnsureRoot<AccountId>,
        pallet_collective::EnsureProportionAtLeast<_3, _5, AccountId, CouncilCollective>,
    >;
    type RejectOrigin = EnsureOneOf<
        EnsureRoot<AccountId>,
        pallet_collective::EnsureProportionMoreThan<_1, _2, AccountId, CouncilCollective>,
    >;
    type Event = Event;
    type OnSlash = ();
    type ProposalBond = ProposalBond;
    type ProposalBondMinimum = ProposalBondMinimum;
    type ProposalBondMaximum = ();
    type SpendPeriod = SpendPeriod;
    type Burn = Burn;
    type BurnDestination = ();
    type SpendFunds = ();
    type WeightInfo = pallet_treasury::weights::SubstrateWeight<Runtime>;
    type MaxApprovals = MaxApprovals;
}

parameter_types! {
    pub const LaunchPeriod: BlockNumber = 7 * DAYS;
    pub const VotingPeriod: BlockNumber = 7 * DAYS;
    pub const FastTrackVotingPeriod: BlockNumber = 3 * HOURS;
    pub const InstantAllowed: bool = true;
    pub const MinimumDeposit: Balance = 10 * DOLLARS;
    pub const EnactmentPeriod: BlockNumber = 8 * DAYS;
    pub const CooloffPeriod: BlockNumber = 7 * DAYS;
    pub const MaxVotes: u32 = 100;
    pub const MaxProposals: u32 = 100;
}

parameter_types! {
	pub const PreimageMaxSize: u32 = 4096 * 1024;
	pub const PreimageBaseDeposit: Balance = 1 * DOLLARS;
	// One cent: $10,000 / MB
	pub const PreimageByteDeposit: Balance = 1 * CENTS;
}

impl pallet_democracy::Config for Runtime {
    type Proposal = Call;
    type Event = Event;
    type Currency = Balances;
    type EnactmentPeriod = EnactmentPeriod;
    type LaunchPeriod = LaunchPeriod;
    type VotingPeriod = VotingPeriod;
    type VoteLockingPeriod = EnactmentPeriod; // Same as EnactmentPeriod
    type MinimumDeposit = MinimumDeposit;
    /// A straight majority of the council can decide what their next motion is.
    type ExternalOrigin = EnsureOneOf<
        pallet_collective::EnsureProportionAtLeast<_1, _2, AccountId, CouncilCollective>,
        frame_system::EnsureRoot<AccountId>,
    >;
    /// A super-majority can have the next scheduled referendum be a straight majority-carries vote.
    type ExternalMajorityOrigin = EnsureOneOf<
        pallet_collective::EnsureProportionAtLeast<_3, _4, AccountId, CouncilCollective>,
        frame_system::EnsureRoot<AccountId>,
    >;
    /// A unanimous council can have the next scheduled referendum be a straight default-carries
    /// (NTB) vote.
    type ExternalDefaultOrigin = EnsureOneOf<
        pallet_collective::EnsureProportionAtLeast<_1, _1, AccountId, CouncilCollective>,
        frame_system::EnsureRoot<AccountId>,
    >;
    /// Two thirds of the technical committee can have an ExternalMajority/ExternalDefault vote
    /// be tabled immediately and with a shorter voting/enactment period.
    type FastTrackOrigin = EnsureOneOf<
        pallet_collective::EnsureProportionAtLeast<_2, _3, AccountId, TechnicalCollective>,
        frame_system::EnsureRoot<AccountId>,
    >;
    type InstantOrigin = EnsureOneOf<
        pallet_collective::EnsureProportionAtLeast<_1, _1, AccountId, TechnicalCollective>,
        frame_system::EnsureRoot<AccountId>,
    >;
    type InstantAllowed = InstantAllowed;
    type FastTrackVotingPeriod = FastTrackVotingPeriod;
    // To cancel a proposal which has been passed, 2/3 of the council must agree to it.
    type CancellationOrigin = EnsureOneOf<
        pallet_collective::EnsureProportionAtLeast<_2, _3, AccountId, CouncilCollective>,
        EnsureRoot<AccountId>,
    >;
    // To cancel a proposal before it has been passed, the technical committee must be unanimous or
    // Root must agree.
    type CancelProposalOrigin = EnsureOneOf<
        pallet_collective::EnsureProportionAtLeast<_1, _1, AccountId, TechnicalCollective>,
        EnsureRoot<AccountId>,
    >;
    type BlacklistOrigin = EnsureRoot<AccountId>;
    // Any single technical committee member may veto a coming council proposal, however they can
    // only do it once and it lasts only for the cooloff period.
    type VetoOrigin = pallet_collective::EnsureMember<AccountId, TechnicalCollective>;
    type CooloffPeriod = CooloffPeriod;
    type PreimageByteDeposit = PreimageByteDeposit;
    type OperationalPreimageOrigin = pallet_collective::EnsureMember<AccountId, CouncilCollective>;
    type Slash = Treasury;
    type Scheduler = Scheduler;
    type PalletsOrigin = OriginCaller;
    type MaxVotes = MaxVotes;
    type WeightInfo = pallet_democracy::weights::SubstrateWeight<Runtime>;
    type MaxProposals = MaxProposals;
}

impl pallet_tips::Config for Runtime {
    type Event = Event;
    type DataDepositPerByte = DataDepositPerByte;
    type MaximumReasonLength = MaximumReasonLength;
    type Tippers = PhragmenElection;
    type TipCountdown = TipCountdown;
    type TipFindersFee = TipFindersFee;
    type TipReportDepositBase = TipReportDepositBase;
    type WeightInfo = pallet_tips::weights::SubstrateWeight<Runtime>;
}

parameter_types! {
    pub const BasicDeposit: Balance = 10 * DOLLARS;       // 258 bytes on-chain
    pub const FieldDeposit: Balance = 250 * CENTS;        // 66 bytes on-chain
    pub const SubAccountDeposit: Balance = 2 * DOLLARS;   // 53 bytes on-chain
    pub const MaxSubAccounts: u32 = 100;
    pub const MaxAdditionalFields: u32 = 100;
    pub const MaxRegistrars: u32 = 20;
}

impl pallet_identity::Config for Runtime {
    type Event = Event;
    type Currency = Balances;
    type BasicDeposit = BasicDeposit;
    type FieldDeposit = FieldDeposit;
    type SubAccountDeposit = SubAccountDeposit;
    type MaxSubAccounts = MaxSubAccounts;
    type MaxAdditionalFields = MaxAdditionalFields;
    type MaxRegistrars = MaxRegistrars;
    type Slashed = Treasury;
    type ForceOrigin = EnsureRootOrHalfCouncil;
    type RegistrarOrigin = EnsureRootOrHalfCouncil;
    type WeightInfo = pallet_identity::weights::SubstrateWeight<Runtime>;
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

		// Governance
		Identity: pallet_identity::{Pallet, Call, Storage, Event<T>} = 116,
		Democracy: pallet_democracy::{Pallet, Call, Storage, Config<T>, Event<T>} = 117,
		Council: pallet_collective::<Instance1>::{Pallet, Call, Storage, Origin<T>, Event<T>, Config<T>} = 118,
		Treasury: pallet_treasury::{Pallet, Call, Storage, Config, Event<T>} = 119,
		TechnicalCommittee: pallet_collective::<Instance2>::{Pallet, Call, Storage, Origin<T>, Event<T>, Config<T>} = 120,
		TechnicalMembership: pallet_membership::<Instance1>::{Pallet, Call, Storage, Event<T>, Config<T>} = 121,
		PhragmenElection: pallet_elections_phragmen::{Pallet, Call, Storage, Event<T>, Config<T>} = 122,
		Tips: pallet_tips::{Pallet, Call, Storage, Event<T>} = 123,
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

	impl pallet_transaction_payment_rpc_runtime_api::TransactionPaymentApi<
		Block,
		Balance,
	> for Runtime {
		fn query_info(uxt: <Block as BlockT>::Extrinsic, len: u32) -> RuntimeDispatchInfo<Balance> {
			TransactionPayment::query_info(uxt, len)
		}

		fn query_fee_details(uxt: <Block as BlockT>::Extrinsic, len: u32) -> pallet_transaction_payment_rpc_runtime_api::FeeDetails<Balance> {
			TransactionPayment::query_fee_details(uxt, len)
		}
	}

	impl frame_system_rpc_runtime_api::AccountNonceApi<Block, AccountId, crust_parachain_primitives::Index> for Runtime {
		fn account_nonce(account: AccountId) -> crust_parachain_primitives::Index {
			System::account_nonce(account)
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

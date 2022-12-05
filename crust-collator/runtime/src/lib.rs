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
use cumulus_pallet_parachain_system::RelayNumberStrictlyIncreases;
pub use crust_parachain_primitives::{
    constants::{currency::*}, traits::*,
    AssetId, *
};
use sp_api::impl_runtime_apis;
use codec::{Decode, Encode, MaxEncodedLen};
use sp_core::{crypto::KeyTypeId, OpaqueMetadata};
use sp_runtime::{
	create_runtime_str, generic, impl_opaque_keys,
	traits::{BlakeTwo256, Block as BlockT, AccountIdLookup, Convert, AccountIdConversion},
	transaction_validity::{TransactionSource, TransactionValidity},
	ApplyExtrinsicResult, Perbill, Percent, Permill,
};
use sp_std::{cmp::Ordering, prelude::*};
#[cfg(feature = "std")]
use sp_version::NativeVersion;
use sp_version::RuntimeVersion;

// A few exports that help ease life for downstream crates.
use sp_std::marker::PhantomData;
use sp_std::{convert::TryInto};
pub use frame_support::{
	construct_runtime, parameter_types, PalletId, match_types,
	traits::{
		Randomness, OriginTrait, IsInVec, Everything, InstanceFilter, Imbalance,
		LockIdentifier, EitherOfDiverse, PrivilegeCmp, Currency, OnUnbalanced, Nothing
	},
	weights::{
		DispatchClass,
		constants::WEIGHT_PER_SECOND,
		IdentityFee, Weight, ConstantMultiplier,
		WeightToFeeCoefficient, WeightToFeeCoefficients, WeightToFeePolynomial,
	},
	StorageValue, RuntimeDebug,
};
use frame_system::limits::{BlockLength, BlockWeights};
use frame_system::{EnsureRoot};
use sp_std::convert::TryFrom;
pub use pallet_balances::Call as BalancesCall;
pub use pallet_timestamp::Call as TimestampCall;
use sp_core::H256;
use sp_runtime::traits::Hash as THash;
#[cfg(any(feature = "std", test))]
pub use sp_runtime::BuildStorage;
use weights::{BlockExecutionWeight, ExtrinsicBaseWeight};

// XCM imports
use polkadot_parachain::primitives::Sibling;
use xcm::latest::prelude::*;
use xcm_builder::{
	AccountId32Aliases, CurrencyAdapter, LocationInverter, ParentIsPreset, RelayChainAsNative,
	SiblingParachainConvertsVia, SignedAccountId32AsNative,
	SovereignSignedViaLocation, EnsureXcmOrigin, AllowUnpaidExecutionFrom, ParentAsSuperuser,
	AllowTopLevelPaidExecutionFrom, TakeWeightCredit, FixedWeightBounds,
	UsingComponents, SignedToAccountId32, SiblingParachainAsNative, AllowKnownQueryResponses,
	AllowSubscriptionsFrom, FungiblesAdapter, ConvertedConcreteAssetId, Account32Hash
};
use xcm_executor::{
	traits::{JustTry, ConvertOrigin},
	Config, XcmExecutor
};
use pallet_xcm::{XcmPassthrough, EnsureXcm, IsMajorityOfBody};
pub use sp_consensus_aura::sr25519::AuthorityId as AuraId;
use xcm::latest::{
	AssetId::Concrete,
	Fungibility::Fungible,
	MultiAsset, MultiLocation,
};
use sp_runtime::traits::CheckedConversion;
use xcm_executor::traits::{MatchesFungible};
use impls::OneTenthFee;
use cumulus_primitives_core::ParaId;
use frame_support::pallet_prelude::*;
use pallet_transaction_payment::RuntimeDispatchInfo;
use smallvec::smallvec;
use orml_xcm_support::MultiNativeAsset;
use orml_traits::location::{RelativeReserveProvider, Reserve};
use orml_traits::parameter_type_with_key;
pub type XcmV2Weight = xcm::v2::Weight;

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
	spec_version: 15,
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
const MAXIMUM_BLOCK_WEIGHT: Weight = WEIGHT_PER_SECOND.saturating_mul(2);

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

pub struct WeightToFee;
impl WeightToFeePolynomial for WeightToFee {
	type Balance = Balance;

	/// Return a vec of coefficients. Here we just use one coefficient and reduce it to a constant
	/// modifier in order to closely match Ethereum-based fees.
	///
	/// Calculation, per the documentation in `frame_support`:
	///
	/// ```ignore
	/// coeff_integer * x^(degree) + coeff_frac * x^(degree)
	/// ```
	fn polynomial() -> WeightToFeeCoefficients<Self::Balance> {
		smallvec![WeightToFeeCoefficient {
			degree: 1,
			coeff_frac: Perbill::zero(),
			coeff_integer: 10u32.into(),
			negative: false,
		}]
	}
}

impl pallet_transaction_payment::Config for Runtime {
	type Event = Event;
	type OnChargeTransaction = pallet_transaction_payment::CurrencyAdapter<Balances, DealWithFees>;
	type LengthToFee = ConstantMultiplier<Balance, TransactionByteFee>;
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
	type WeightInfo = pallet_proxy::weights::SubstrateWeight<Runtime>;
	type MaxPending = MaxPending;
	type CallHasher = BlakeTwo256;
	type AnnouncementDepositBase = AnnouncementDepositBase;
	type AnnouncementDepositFactor = AnnouncementDepositFactor;
}

// impl pallet_sudo::Config for Runtime {
// 	type Call = Call;
// 	type Event = Event;
// }

parameter_types! {
	pub MaximumSchedulerWeight: Weight = Perbill::from_percent(80) * RuntimeBlockWeights::get().max_block;
	pub const MaxScheduledPerBlock: u32 = 50;
	pub const NoPreimagePostponement: Option<u32> = Some(10);
}

/// Used the compare the privilege of an origin inside the scheduler.
pub struct OriginPrivilegeCmp;

impl PrivilegeCmp<OriginCaller> for OriginPrivilegeCmp {
	fn cmp_privilege(_left: &OriginCaller, _right: &OriginCaller) -> Option<Ordering> {
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
	type CheckAssociatedRelayNumber = RelayNumberStrictlyIncreases;
}

impl parachain_info::Config for Runtime {}

parameter_types! {
	pub const RelayLocation: MultiLocation = MultiLocation::parent();
	pub const LocalNetwork: MultiLocation = MultiLocation::here();
	pub const RelayNetwork: NetworkId = NetworkId::Kusama;
	pub RelayChainOrigin: Origin = cumulus_pallet_xcm::Origin::Relay.into();
	pub Ancestry: MultiLocation = Parachain(ParachainInfo::parachain_id().into()).into();

	// Self Reserve location, defines the multilocation identifiying the self-reserve currency
	// This is used to match it also against our Balances pallet when we receive such
	// a MultiLocation: (Self Balances pallet index)
	// We use the RELATIVE multilocation
	pub SelfReserve: MultiLocation = MultiLocation::here();
	pub CheckingAccount: AccountId = PalletId(*b"checking").into_account_truncating();
}
use xcm_executor::traits::Convert as LocationConvert;
pub struct SiblingAccountId32Aliases<Network, AccountId>(PhantomData<(Network, AccountId)>);
impl<Network: Get<NetworkId>, AccountId: From<[u8; 32]> + Into<[u8; 32]> + Clone>
	LocationConvert<MultiLocation, AccountId> for SiblingAccountId32Aliases<Network, AccountId>
{
	fn convert(location: MultiLocation) -> Result<AccountId, MultiLocation> {
		let id = match location {
			MultiLocation {
				parents: 1,
				interior: Junctions::X2(Parachain(_), AccountId32 { id, network: _ })
			} => id,
			_ => return Err(location),
		};
		Ok(id.into())
	}

	fn reverse(account: AccountId) -> Result<MultiLocation, AccountId> {
		Err(account)
	}
}

type LocationToAccountId = (
	ParentIsPreset<AccountId>,
	SiblingParachainConvertsVia<Sibling, AccountId>,
	AccountId32Aliases<RelayNetwork, AccountId>,
	SiblingAccountId32Aliases<RelayNetwork, AccountId>,
	Account32Hash<RelayNetwork, AccountId>,
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

// The non-reserve fungible transactor type
// It will use pallet-assets, and the Id will be matched against AsAssetType
pub type FungiblesTransactor = FungiblesAdapter<
	// Use this fungibles implementation:
	Assets,
	// Use this currency when it is a fungible asset matching the given location or name:
	(
		ConvertedConcreteAssetId<
			AssetId,
			Balance,
			AsAssetType<AssetId, AssetType, AssetManager>,
			JustTry,
		>,
	),
	// Do a simple punn to convert an AccountId20 MultiLocation into a native chain account ID:
	LocationToAccountId,
	// Our chain's account ID type (we can't get away without mentioning it explicitly):
	AccountId,
	// We dont allow teleports.
	Nothing,
	// We dont track any teleports
	CheckingAccount,
>;

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
	CheckingAccount,
>;

pub type CrustAssetTransactors = (LocalAssetTransactor, FungiblesTransactor);

pub struct SiblingSignedAccountId32AsNative<Origin>(PhantomData<Origin>);
impl<Origin: OriginTrait> ConvertOrigin<Origin>
	for SiblingSignedAccountId32AsNative<Origin>
where
	Origin::AccountId: From<[u8; 32]>,
{
	fn convert_origin(
		origin: impl Into<MultiLocation>,
		kind: OriginKind,
	) -> Result<Origin, MultiLocation> {
		let origin = origin.into();
		log::trace!(
			target: "xcm::origin_conversion",
			"SiblingSignedAccountId32AsNative origin: {:?}, kind: {:?}",
			origin, kind,
		);
		match (kind, origin) {
			(
				OriginKind::Native,
				MultiLocation { parents: 1, interior: Junctions::X2(Parachain(_), AccountId32 { id, network: _ })},
			) =>
				Ok(Origin::signed(id.into())),
			(_, origin) => Err(origin),
		}
	}
}

pub type XcmOriginToTransactDispatchOrigin = (
	SovereignSignedViaLocation<LocationToAccountId, Origin>,
	RelayChainAsNative<RelayChainOrigin, Origin>,
	SiblingParachainAsNative<cumulus_pallet_xcm::Origin, Origin>,
	ParentAsSuperuser<Origin>,
	SignedAccountId32AsNative<RelayNetwork, Origin>,
	// Xcm origins can be represented natively under the Xcm pallet's Xcm origin.
	XcmPassthrough<Origin>,
	SiblingSignedAccountId32AsNative<Origin>,
);

parameter_types! {
	// One XCM operation is 1_000_000_000 weight - almost certainly a conservative estimate.
	pub UnitWeightCost: XcmV2Weight = 1_000_000_000;
	// One ROC buys 1 second of weight.
	pub const WeightPrice: (MultiLocation, u128) = (MultiLocation::parent(), ROC);
	pub const MaxInstructions: u32 = 100;
}

/// Xcm Weigher shared between multiple Xcm-related configs.
pub type XcmWeigher = FixedWeightBounds<UnitWeightCost, Call, MaxInstructions>;

match_types! {
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
	crust_parachain_primitives::AllowDescendOriginFromLocal<Everything>,
);

parameter_types! {
	/// Xcm fees will go to the treasury account
	pub XcmFeesAccount: AccountId = Treasury::account_id();
}

/// This is the struct that will handle the revenue from xcm fees
/// We do not burn anything because we want to mimic exactly what
/// the sovereign account has
pub type XcmFeesToAccount = crust_parachain_primitives::XcmFeesToAccount<
	Assets,
	(
		ConvertedConcreteAssetId<
			AssetId,
			Balance,
			AsAssetType<AssetId, AssetType, AssetManager>,
			JustTry,
		>,
	),
	AccountId,
	XcmFeesAccount,
>;

pub struct XcmConfig;
impl Config for XcmConfig {
	type Call = Call;
	type XcmSender = XcmRouter;
	// How to withdraw and deposit an asset.
	type AssetTransactor = CrustAssetTransactors;
	type OriginConverter = XcmOriginToTransactDispatchOrigin;
	// Filter to the reserve withdraw operations
	// Whenever the reserve matches the relative or absolute value
	// of our chain, we always return the relative reserve
	type IsReserve = MultiNativeAsset<AbsoluteAndRelativeReserve<SelfLocationAbsolute>>;
	type IsTeleporter = (); // No teleport
	type LocationInverter = LocationInverter<Ancestry>;
	type Barrier = Barrier;
	type Weigher = XcmWeigher;
	type Trader = (
		UsingComponents<WeightToFee, LocalNetwork, AccountId, Balances, DealWithFees>,
		FirstAssetTrader<AssetType, AssetManager, XcmFeesToAccount>,
	);
	type ResponseHandler = PolkadotXcm;
	type AssetTrap = PolkadotXcm;
	type AssetClaims = PolkadotXcm;
	type SubscriptionService = PolkadotXcm;
}

pub type LocalOriginToLocation = (
	SignedToAccountId32<Origin, AccountId, RelayNetwork>,
);

/// The means for routing XCM messages which are not for local execution into the right message
/// queues.
pub type XcmRouter = (
	// Two routers - use UMP to communicate with the relay chain:
	cumulus_primitives_utility::ParentAsUmp<ParachainSystem, PolkadotXcm>,
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
	type ControllerOrigin = EnsureRoot<AccountId>;
	type ControllerOriginConverter = XcmOriginToTransactDispatchOrigin;
	type WeightInfo = cumulus_pallet_xcmp_queue::weights::SubstrateWeight<Runtime>;
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

impl xstorage::Config for Runtime {
	type Event = Event;
	type XcmpMessageSender = XcmRouter;
	type AssetTransactor = CrustAssetTransactors;
	type CurrencyId = CurrencyId;
	type AccountIdToMultiLocation = AccountIdToMultiLocation;
	type CurrencyIdToMultiLocation =
		CurrencyIdtoMultiLocation<AsAssetType<AssetId, AssetType, AssetManager>>;
	type StorageFeeOwner = EnsureRootOrHalfCouncil;
}

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
pub type CollatorSelectionUpdateOrigin = EitherOfDiverse<
	EnsureRoot<AccountId>,
	EnsureXcm<IsMajorityOfBody<RelayLocation, ExecutiveBody>>,
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

impl bridge::Config for Runtime {
	type PalletId = BridgeClaimsPalletId;
    type Event = Event;
    type BridgeCommitteeOrigin = EnsureRootOrHalfCouncil;
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

type EnsureRootOrHalfCouncil = EitherOfDiverse<
    EnsureRoot<AccountId>,
    pallet_collective::EnsureProportionMoreThan<AccountId, CouncilCollective, 1, 2>,
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
	pub const MaxVoters: u32 = 10 * 1000;
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
	type MaxVoters = MaxVoters;
	type MaxCandidates = MaxCandidates;
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
    type ApproveOrigin = EitherOfDiverse<
        EnsureRoot<AccountId>,
        pallet_collective::EnsureProportionAtLeast<AccountId, CouncilCollective, 3, 5>,
    >;
    type RejectOrigin = EitherOfDiverse<
        EnsureRoot<AccountId>,
        pallet_collective::EnsureProportionMoreThan<AccountId, CouncilCollective, 1, 2>,
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
	type SpendOrigin = frame_support::traits::NeverEnsureOrigin<Balance>;
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
    type ExternalOrigin = EitherOfDiverse<
        pallet_collective::EnsureProportionAtLeast<AccountId, CouncilCollective, 1, 2>,
        frame_system::EnsureRoot<AccountId>,
    >;
    /// A super-majority can have the next scheduled referendum be a straight majority-carries vote.
    type ExternalMajorityOrigin = EitherOfDiverse<
        pallet_collective::EnsureProportionAtLeast<AccountId, CouncilCollective, 1, 2>,
        frame_system::EnsureRoot<AccountId>,
    >;
    /// A unanimous council can have the next scheduled referendum be a straight default-carries
    /// (NTB) vote.
    type ExternalDefaultOrigin = EitherOfDiverse<
        pallet_collective::EnsureProportionAtLeast<AccountId, CouncilCollective, 1, 1>,
        frame_system::EnsureRoot<AccountId>,
    >;
    /// Two thirds of the technical committee can have an ExternalMajority/ExternalDefault vote
    /// be tabled immediately and with a shorter voting/enactment period.
    type FastTrackOrigin = EitherOfDiverse<
        pallet_collective::EnsureProportionAtLeast<AccountId, TechnicalCollective, 2, 3>,
        frame_system::EnsureRoot<AccountId>,
    >;
    type InstantOrigin = EitherOfDiverse<
        pallet_collective::EnsureProportionAtLeast<AccountId, TechnicalCollective, 1, 1>,
        frame_system::EnsureRoot<AccountId>,
    >;
    type InstantAllowed = InstantAllowed;
    type FastTrackVotingPeriod = FastTrackVotingPeriod;
    // To cancel a proposal which has been passed, 2/3 of the council must agree to it.
    type CancellationOrigin = EitherOfDiverse<
        pallet_collective::EnsureProportionAtLeast<AccountId, CouncilCollective, 2, 3>,
        EnsureRoot<AccountId>,
    >;
    // To cancel a proposal before it has been passed, the technical committee must be unanimous or
    // Root must agree.
    type CancelProposalOrigin = EitherOfDiverse<
        pallet_collective::EnsureProportionAtLeast<AccountId, TechnicalCollective, 1, 1>,
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

// These parameters dont matter much as this will only be called by root with the forced arguments
// No deposit is substracted with those methods
parameter_types! {
	pub const AssetDeposit: Balance = 0;
	pub const ApprovalDeposit: Balance = 0;
	pub const AssetsStringLimit: u32 = 50;
	pub const MetadataDepositBase: Balance = 0;
	pub const MetadataDepositPerByte: Balance = 0;
	pub const AssetAccountDeposit: Balance = 1 * DOLLARS;
}

/// We allow root and Chain council to execute privileged asset operations.
pub type AssetsForceOrigin = EitherOfDiverse<
	EnsureRoot<AccountId>,
	pallet_collective::EnsureProportionMoreThan<AccountId, CouncilCollective, 1, 2>,
>;

impl pallet_assets::Config for Runtime {
	type Event = Event;
	type Balance = Balance;
	type AssetId = AssetId;
	type Currency = Balances;
	type ForceOrigin = AssetsForceOrigin;
	type AssetDeposit = AssetDeposit;
	type MetadataDepositBase = MetadataDepositBase;
	type MetadataDepositPerByte = MetadataDepositPerByte;
	type ApprovalDeposit = ApprovalDeposit;
	type StringLimit = AssetsStringLimit;
	type Freezer = ();
	type Extra = ();
	type AssetAccountDeposit = AssetAccountDeposit;
	type WeightInfo = pallet_assets::weights::SubstrateWeight<Runtime>;
}

parameter_types! {
	// Statemint ParaId in Alphanet
	pub StatemintParaId: u32 = 1001;
	// Assets Pallet instance in Statemint alphanet
	pub StatemintAssetPalletInstance: u8 = 50;
}
// Our AssetType. For now we only handle Xcm Assets
#[derive(Clone, Eq, Debug, PartialEq, Ord, PartialOrd, Encode, Decode, TypeInfo)]
pub enum AssetType {
	Xcm(MultiLocation),
}
impl Default for AssetType {
	fn default() -> Self {
		Self::Xcm(MultiLocation::here())
	}
}

impl From<MultiLocation> for AssetType {
	fn from(location: MultiLocation) -> Self {
		match location {
			// Change https://github.com/paritytech/cumulus/pull/831
			// This avoids interrumption once they upgrade
			// We map the previous location to the new one so that the assetId is well retrieved
			MultiLocation {
				parents: 1,
				interior: X2(Parachain(id), GeneralIndex(index)),
			} if id == StatemintParaId::get() => Self::Xcm(MultiLocation {
				parents: 1,
				interior: X3(
					Parachain(id),
					PalletInstance(StatemintAssetPalletInstance::get()),
					GeneralIndex(index),
				),
			}),
			_ => Self::Xcm(location),
		}
	}
}

impl Into<Option<MultiLocation>> for AssetType {
	fn into(self) -> Option<MultiLocation> {
		match self {
			Self::Xcm(location) => Some(location),
		}
	}
}

// Implementation on how to retrieve the AssetId from an AssetType
// We take it
impl From<AssetType> for AssetId {
	fn from(asset: AssetType) -> AssetId {
		match asset {
			AssetType::Xcm(id) => {
				let mut result: [u8; 16] = [0u8; 16];
				let hash: H256 = id.using_encoded(<Runtime as frame_system::Config>::Hashing::hash);
				result.copy_from_slice(&hash.as_fixed_bytes()[0..16]);
				u128::from_le_bytes(result)
			}
		}
	}
}

// We instruct how to register the Assets
// In this case, we tell it to Create an Asset in pallet-assets
pub struct AssetRegistrar;
use frame_support::{pallet_prelude::DispatchResult, transactional};

impl pallet_asset_manager::AssetRegistrar<Runtime> for AssetRegistrar {
	#[transactional]
	fn create_asset(
		asset: AssetId,
		min_balance: Balance,
		metadata: AssetRegistrarMetadata,
		is_sufficient: bool,
	) -> DispatchResult {
		Assets::force_create(
			Origin::root(),
			asset,
			sp_runtime::MultiAddress::Id(AssetManager::account_id()),
			is_sufficient,
			min_balance,
		)?;

		// TODO uncomment when we feel comfortable
		/*
		// The asset has been created. Let's put the revert code in the precompile address
		let precompile_address = Runtime::asset_id_to_account(asset);
		pallet_evm::AccountCodes::<Runtime>::insert(
			precompile_address,
			vec![0x60, 0x00, 0x60, 0x00, 0xfd],
		);*/

		// Lastly, the metadata
		Assets::force_set_metadata(
			Origin::root(),
			asset,
			metadata.name,
			metadata.symbol,
			metadata.decimals,
			metadata.is_frozen,
		)
	}
}

#[derive(Clone, Default, Eq, Debug, PartialEq, Ord, PartialOrd, Encode, Decode, TypeInfo)]
pub struct AssetRegistrarMetadata {
	pub name: Vec<u8>,
	pub symbol: Vec<u8>,
	pub decimals: u8,
	pub is_frozen: bool,
}

impl pallet_asset_manager::Config for Runtime {
	type Event = Event;
	type Balance = Balance;
	type AssetId = AssetId;
	type AssetRegistrarMetadata = AssetRegistrarMetadata;
	type AssetType = AssetType;
	type AssetRegistrar = AssetRegistrar;
	type AssetModifierOrigin = AssetsForceOrigin;
	type WeightInfo = pallet_asset_manager::weights::SubstrateWeight<Runtime>;
}

// Our currencyId. We distinguish for now between SelfReserve, and Others, defined by their Id.
#[derive(Clone, Eq, Debug, PartialEq, Ord, PartialOrd, Encode, Decode, TypeInfo)]
pub enum CurrencyId {
	SelfReserve,
	OtherReserve(AssetId),
}

// How to convert from CurrencyId to MultiLocation
pub struct CurrencyIdtoMultiLocation<AssetXConverter>(sp_std::marker::PhantomData<AssetXConverter>);
impl<AssetXConverter> sp_runtime::traits::Convert<CurrencyId, Option<MultiLocation>>
	for CurrencyIdtoMultiLocation<AssetXConverter>
where
	AssetXConverter: xcm_executor::traits::Convert<MultiLocation, AssetId>,
{
	fn convert(currency: CurrencyId) -> Option<MultiLocation> {
		match currency {
			// For now and until Xtokens is adapted to handle 0.9.16 version we use
			// the old anchoring here
			// This is not a problem in either cases, since the view of the destination
			// chain does not change
			CurrencyId::SelfReserve => {
				let multi: MultiLocation = LocalNetwork::get();
				Some(multi)
			}
			CurrencyId::OtherReserve(asset) => AssetXConverter::reverse_ref(asset).ok(),
		}
	}
}

/// This struct offers uses RelativeReserveProvider to output relative views of multilocations
/// However, additionally accepts a MultiLocation that aims at representing the chain part
/// (parent: 1, Parachain(paraId)) of the absolute representation of our chain.
/// If a token reserve matches against this absolute view, we return  Some(MultiLocation::here())
/// This helps users by preventing errors when they try to transfer a token through xtokens
/// to our chain (either inserting the relative or the absolute value).
pub struct AbsoluteAndRelativeReserve<AbsoluteMultiLocation>(PhantomData<AbsoluteMultiLocation>);
impl<AbsoluteMultiLocation> Reserve for AbsoluteAndRelativeReserve<AbsoluteMultiLocation>
where
	AbsoluteMultiLocation: Get<MultiLocation>,
{
	fn reserve(asset: &MultiAsset) -> Option<MultiLocation> {
		RelativeReserveProvider::reserve(asset).map(|relative_reserve| {
			if relative_reserve == AbsoluteMultiLocation::get() {
				MultiLocation::here()
			} else {
				relative_reserve
			}
		})
	}
}

parameter_types! {
	pub const BaseXcmWeight: XcmV2Weight = 100_000_000;
	pub const MaxAssetsForTransfer: usize = 2;

	// This is how we are going to detect whether the asset is a Reserve asset
	// This however is the chain part only
	pub SelfLocation: MultiLocation = MultiLocation::here();
	// We need this to be able to catch when someone is trying to execute a non-
	// cross-chain transfer in xtokens through the absolute path way
	pub SelfLocationAbsolute: MultiLocation = MultiLocation {
		parents:1,
		interior: Junctions::X1(
			Parachain(ParachainInfo::parachain_id().into())
		)
	};
}

pub struct AccountIdToMultiLocation;
impl Convert<AccountId, MultiLocation> for AccountIdToMultiLocation {
	fn convert(account: AccountId) -> MultiLocation {
		X1(AccountId32 {
			network: NetworkId::Any,
			id: account.into(),
		})
		.into()
	}
}

parameter_type_with_key! {
	pub ParachainMinFee: |_location: MultiLocation| -> Option<u128> {
		Some(u128::MAX)
	};
}

impl orml_xtokens::Config for Runtime {
	type Event = Event;
	type Balance = Balance;
	type CurrencyId = CurrencyId;
	type AccountIdToMultiLocation = AccountIdToMultiLocation;
	type CurrencyIdConvert =
		CurrencyIdtoMultiLocation<AsAssetType<AssetId, AssetType, AssetManager>>;
	type XcmExecutor = XcmExecutor<XcmConfig>;
	type SelfLocation = SelfLocation;
	type Weigher = XcmWeigher;
	type BaseXcmWeight = BaseXcmWeight;
	type LocationInverter = LocationInverter<Ancestry>;
	type MaxAssetsForTransfer = MaxAssetsForTransfer;
	type MinXcmFee = ParachainMinFee;
	type MultiLocationsFilter = Everything;
	type ReserveProvider = AbsoluteAndRelativeReserve<SelfLocationAbsolute>;
}

// parameter_types! {
// 	pub FeePerSecond: u128 = 1_000_000;
// }

// impl xstorage_client::Config for Runtime {
// 	type Event = Event;
// 	type XcmpMessageSender = XcmRouter;
// 	type AssetTransactor = CrustAssetTransactors;
// 	type CurrencyId = CurrencyId;
// 	type AccountIdToMultiLocation = AccountIdToMultiLocation;
// 	type CurrencyIdToMultiLocation =
// 		CurrencyIdtoMultiLocation<AsAssetType<AssetId, AssetType, AssetManager>>;
// 	type LocationInverter = LocationInverter<Ancestry>;
// 	type CrustNativeToken = xstorage_client::primitives::CrustShadowLocation;
// 	type SelfNativeToken = SelfLocation;
// 	type FeePerSecond = FeePerSecond;
// 	type Destination = xstorage_client::primitives::CsmMultiloaction;
// }

construct_runtime! {
	pub enum Runtime where
		Block = Block,
		NodeBlock = crust_parachain_primitives::Block,
		UncheckedExtrinsic = UncheckedExtrinsic,
	{
		System: frame_system::{Pallet, Call, Storage, Config, Event<T>},
		Timestamp: pallet_timestamp::{Pallet, Call, Storage, Inherent},
		Balances: pallet_balances::{Pallet, Call, Storage, Config<T>, Event<T>},
		// Sudo: pallet_sudo::{Pallet, Call, Storage, Config<T>, Event<T>},
		RandomnessCollectiveFlip: pallet_randomness_collective_flip::{Pallet, Storage},
		ParachainSystem: cumulus_pallet_parachain_system::{Pallet, Call, Storage, Inherent, Event<T>, ValidateUnsigned, Config},
		TransactionPayment: pallet_transaction_payment::{Pallet, Storage, Event<T>},
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

		// MultiCurrency
		Assets: pallet_assets::{Pallet, Call, Storage, Event<T>} = 124,
		XTokens: orml_xtokens::{Pallet, Call, Storage, Event<T>} = 125,
		AssetManager: pallet_asset_manager::{Pallet, Call, Storage, Event<T>} = 126,

		// Crust
		Xstorage: xstorage::{Pallet, Storage, Call, Event<T>} = 127,
		// XstorageClient: xstorage_client::{Pallet, Storage, Call, Event<T>} = 128,
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

	impl pallet_transaction_payment_rpc_runtime_api::TransactionPaymentCallApi<Block, Balance, Call>
		for Runtime
	{
		fn query_call_info(
			call: Call,
			len: u32,
		) -> pallet_transaction_payment::RuntimeDispatchInfo<Balance> {
			TransactionPayment::query_call_info(call, len)
		}
		fn query_call_fee_details(
			call: Call,
			len: u32,
		) -> pallet_transaction_payment::FeeDetails<Balance> {
			TransactionPayment::query_call_fee_details(call, len)
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

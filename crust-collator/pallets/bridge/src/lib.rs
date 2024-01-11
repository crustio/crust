// Copyright 2021 ChainSafe Systems
// SPDX-License-Identifier: GPL-3.0-only

//! # Filecoin Governance Pallet
//!
//! This pallet uses a set of AccountIds to identify who
//! can vote on proposals. Relayers may be added, removed.
//! There is no bound on how many members may exist in the committee.
//!
//! For each block addition proposal, relayers can vote on them.
//! The pallet will lazily resolve all the proposals.
//! Admin could also resolve manually.
//!
#![deny(warnings)]
#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::{
    pallet_prelude::EnsureOrigin,
    traits::Get,
};
use sp_runtime::traits::AccountIdConversion;
pub use pallet::{
    Config,
    Error,
    Event,
    Pallet,
    RelayerThreshold,
    *,
};
pub use types::{
    ResourceId,
};
pub mod types;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

pub mod weights;

#[frame_support::pallet]
pub mod pallet {
    use crate::types::{
        ProposalStatus,
        ProposalVotes,
        ResourceId,
        BridgeEvent,
    };
    use codec::EncodeLike;
    use frame_support::{
        dispatch::{Dispatchable, GetDispatchInfo},
        pallet_prelude::*,
        PalletId,
    };
    use frame_system::pallet_prelude::*;
    use scale_info::prelude::boxed::Box;
    use sp_runtime::traits::AccountIdConversion;
    use sp_core::U256;
    use sp_std::vec::Vec;
    use sp_std::convert::TryInto;
    use crate::weights::WeightInfo;

    #[pallet::config]
    pub trait Config: frame_system::Config {
        type RuntimeEvent: From<Event<Self>>
            + IsType<<Self as frame_system::Config>::RuntimeEvent>;
        /// RuntimeOrigin used to administer the pallet
        type BridgeCommitteeOrigin: EnsureOrigin<Self::RuntimeOrigin>;
        /// Proposed dispatchable call
        type Proposal: Parameter
            + Dispatchable<RuntimeOrigin = Self::RuntimeOrigin>
            + EncodeLike
            + GetDispatchInfo;
        /// The identifier for this chain.
        /// This must be unique and must not collide with existing IDs within a set of bridged
        /// chains.
        #[pallet::constant]
        type BridgeChainId: Get<u8>;

        #[pallet::constant]
        type ProposalLifetime: Get<BlockNumberFor<Self>>;

        /// Constant configuration parameter to store the module identifier for the pallet.
        ///
        /// The module identifier may be of the form ```PalletId(*b"chnbrdge")``` and set
        /// using the [`parameter_types`](https://substrate.dev/docs/en/knowledgebase/runtime/macros#parameter_types)
        // macro in the [`runtime/lib.rs`] file.
        #[pallet::constant]
        type PalletId: Get<PalletId>;

        type WeightInfo: WeightInfo;
    }

    #[pallet::pallet]
    #[pallet::without_storage_info]
    pub struct Pallet<T>(_);

    /// If there is not default relayer threshold set yet, return the value of 1
    #[pallet::type_value]
    pub fn DefaultRelayerThreshold() -> u32 {
        1
    }

    #[pallet::storage]
    #[pallet::getter(fn relayer_threshold)]
    /// Number of votes required for a proposal to execute
    pub type RelayerThreshold<T: Config> =
        StorageValue<_, u32, ValueQuery, DefaultRelayerThreshold>;

    /// Utilized by the bridge software to map resource IDs to actual methods
    #[pallet::storage]
    #[pallet::getter(fn resources)]
    pub type Resources<T: Config> =
        StorageMap<_, Blake2_256, [u8; 32], Vec<u8>, OptionQuery>;

    /// All whitelisted chains and their respective transaction counts
    #[pallet::storage]
    #[pallet::getter(fn chains)]
    pub type ChainNonces<T: Config> =
        StorageMap<_, Blake2_256, u8, Option<u64>, ValueQuery>;

    /// Tracks current relayer set
    #[pallet::storage]
    #[pallet::getter(fn relayers)]
    pub type Relayers<T: Config> =
        StorageMap<_, Blake2_256, T::AccountId, (), OptionQuery>;

    #[pallet::storage]
    #[pallet::getter(fn relayer_count)]
    pub type RelayerCount<T: Config> = StorageValue<_, u32, ValueQuery>;

    /// All known proposals.
    /// The key is the hash of the call and the deposit ID, to ensure it's unique.
    #[pallet::storage]
    #[pallet::getter(fn get_votes)]
    pub(super) type Votes<T: Config> = StorageDoubleMap<
        _,
        Blake2_256,
        u8,
        Blake2_256,
        (u64, T::Proposal),
        ProposalVotes<T::AccountId, BlockNumberFor<T>>,
        OptionQuery,
    >;

    /// All whitelisted chains and their respective transaction counts
    #[pallet::storage]
    #[pallet::getter(fn bridge_events)]
    pub type BridgeEvents<T: Config> =
        StorageValue<_, Vec<BridgeEvent>, ValueQuery>;

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// Vote threshold has changed (new_threshold)
        RelayerThresholdChanged(u32),
        /// Chain now available for transfers (chain_id)
        ChainWhitelisted(u8),
        /// Relayer added to set
        RelayerAdded(T::AccountId),
        /// Relayer removed from set
        RelayerRemoved(T::AccountId),
        /// FunglibleTransfer is for relaying fungibles (dest_id, nonce, resource_id, amount, recipient, metadata)
        FungibleTransfer(u8, u64, ResourceId, U256, Vec<u8>),
        /// NonFungibleTransfer is for relaying NFTS (dest_id, nonce, resource_id, token_id, recipient, metadata)
        NonFungibleTransfer(
            u8,
            u64,
            ResourceId,
            Vec<u8>,
            Vec<u8>,
            Vec<u8>,
        ),
        /// GenericTransfer is for a generic data payload (dest_id, nonce, resource_id, metadata)
        GenericTransfer(u8, u64, ResourceId, Vec<u8>),
        /// Vote submitted in favour of proposal
        VoteFor(u8, u64, T::AccountId),
        /// Vot submitted against proposal
        VoteAgainst(u8, u64, T::AccountId),
        /// Voting successful for a proposal
        ProposalApproved(u8, u64),
        /// Voting rejected a proposal
        ProposalRejected(u8, u64),
        /// Execution of call succeeded
        ProposalSucceeded(u8, u64),
        /// Execution of call failed
        ProposalFailed(u8, u64),
    }

    #[pallet::error]
    pub enum Error<T> {
        /// Relayer threshold not set
        ThresholdNotSet,
        /// Provided chain Id is not valid
        InvalidChainId,
        /// Relayer threshold cannot be 0
        InvalidThreshold,
        /// Interactions with this chain is not permitted
        ChainNotWhitelisted,
        /// Chain has already been enabled
        ChainAlreadyWhitelisted,
        /// Resource ID provided isn't mapped to anything
        ResourceDoesNotExist,
        /// Relayer already in set
        RelayerAlreadyExists,
        /// Provided accountId is not a relayer
        RelayerInvalid,
        /// Protected operation, must be performed by relayer
        MustBeRelayer,
        /// Relayer has already submitted some vote for this proposal
        RelayerAlreadyVoted,
        /// A proposal with these parameters has already been submitted
        ProposalAlreadyExists,
        /// No proposal with the ID was found
        ProposalDoesNotExist,
        /// Cannot complete proposal, needs more votes
        ProposalNotComplete,
        /// Proposal has either failed or succeeded
        ProposalAlreadyComplete,
        /// Lifetime of proposal has been exceeded
        ProposalExpired,
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Sets the vote threshold for proposals.
        ///
        /// This threshold is used to determine how many votes are required
        /// before a proposal is executed.
        ///
        /// # <weight>
        /// - O(1) lookup and insert
        /// # </weight>
        #[pallet::call_index(0)]
		#[pallet::weight(T::WeightInfo::default_bridge_weight())]
        pub fn set_threshold(
            origin: OriginFor<T>,
            threshold: u32,
        ) -> DispatchResult {
            Self::ensure_admin(origin)?;
            Self::set_relayer_threshold(threshold)?;
            Ok(())
        }

        /// Stores a method name on chain under an associated resource ID.
        ///
        /// # <weight>
        /// - O(1) write
        /// # </weight>
        #[pallet::call_index(1)]
		#[pallet::weight(T::WeightInfo::default_bridge_weight())]
        pub fn set_resource(
            origin: OriginFor<T>,
            id: [u8; 32],
            method: Vec<u8>,
        ) -> DispatchResult {
            Self::ensure_admin(origin)?;
            Self::register_resource(id, method)?;
            Ok(())
        }

        /// Removes a resource ID from the resource mapping.
        ///
        /// After this call, bridge transfers with the associated resource ID will
        /// be rejected.
        ///
        /// # <weight>
        /// - O(1) removeal
        /// # </weight>
        #[pallet::call_index(2)]
		#[pallet::weight(T::WeightInfo::default_bridge_weight())]
        pub fn remove_resource(
            origin: OriginFor<T>,
            id: [u8; 32],
        ) -> DispatchResult {
            Self::ensure_admin(origin)?;
            Self::unregister_resource(id)?;
            Ok(())
        }

        /// Adds a new relayer to the relayer set.
        ///
        /// # <weight>
        /// - O(1) lookup and insert
        /// # </weight>
        #[pallet::call_index(3)]
		#[pallet::weight(T::WeightInfo::default_bridge_weight())]
        pub fn whitelist_chain(
            origin: OriginFor<T>,
            id: u8,
        ) -> DispatchResult {
            Self::ensure_admin(origin)?;
            Self::whitelist(id)?;
            Ok(())
        }

        /// Adds a new relayer to the relayer set.
        ///
        /// # <weight>
        /// - O(1) lookup and removal
        /// # </weight>
        #[pallet::call_index(4)]
		#[pallet::weight(T::WeightInfo::default_bridge_weight())]
        pub fn add_relayer(
            origin: OriginFor<T>,
            v: T::AccountId,
        ) -> DispatchResult {
            Self::ensure_admin(origin)?;
            Self::register_relayer(v)?;
            Ok(())
        }

        /// Removes an existing relaye to the set.
        ///
        /// # <weight>
        /// - O(1) lookup and removal
        /// # </weight>
        #[pallet::call_index(5)]
		#[pallet::weight(T::WeightInfo::default_bridge_weight())]
        pub fn remove_relayer(
            origin: OriginFor<T>,
            v: T::AccountId,
        ) -> DispatchResult {
            Self::ensure_admin(origin)?;
            Self::unregister_relayer(v)?;
            Ok(())
        }

        /// Commits a vote in favour of the provided proposal.
        ///
        /// If a proposal with the given nonce and source chain ID does not already exist,
        /// it will be created with an initial vote in favour from the caller.
        ///
        /// # <weight>
        /// - weight of proposed call, regardless of whether execution is performed
        /// # </weight>
        #[pallet::call_index(6)]
		#[pallet::weight(T::WeightInfo::default_bridge_weight())]
        pub fn acknowledge_proposal(
            origin: OriginFor<T>,
            nonce: u64,
            src_id: u8,
            r_id: [u8; 32],
            call: Box<<T as Config>::Proposal>,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;
            ensure!(Self::is_relayer(&who), Error::<T>::MustBeRelayer);
            ensure!(
                Self::chain_whitelisted(src_id),
                Error::<T>::ChainNotWhitelisted
            );
            ensure!(
                Self::resource_exists(r_id),
                Error::<T>::ResourceDoesNotExist
            );

            Self::vote_for(who, nonce, src_id, call)
        }

        /// Commits a vote against a provided proposal.
        ///
        /// # <weight>
        /// - Fixed, since execution of proposal should not be included
        /// # </weight>
        #[pallet::call_index(7)]
		#[pallet::weight(T::WeightInfo::default_bridge_weight())]
        pub fn reject_proposal(
            origin: OriginFor<T>,
            nonce: u64,
            src_id: u8,
            r_id: [u8; 32],
            call: Box<<T as Config>::Proposal>,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;
            ensure!(Self::is_relayer(&who), Error::<T>::MustBeRelayer);
            ensure!(
                Self::chain_whitelisted(src_id),
                Error::<T>::ChainNotWhitelisted
            );
            ensure!(
                Self::resource_exists(r_id),
                Error::<T>::ResourceDoesNotExist
            );
            Self::vote_against(who, nonce, src_id, call)
        }

        /// Evaluate the state of a proposal given the current vote threshold.
        ///
        /// A proposal with enough votes will be either executed or cancelled, and the status
        /// will be updated accordingly.
        ///
        /// # <weight>
        /// - weight of proposed call, regardless of whether execution is performed
        /// # </weight>
        #[pallet::call_index(8)]
		#[pallet::weight(T::WeightInfo::default_bridge_weight())]
        pub fn eval_vote_state(
            origin: OriginFor<T>,
            nonce: u64,
            src_id: u8,
            prop: Box<<T as Config>::Proposal>,
        ) -> DispatchResult {
            ensure_signed(origin)?;

            Self::try_resolve_proposal(nonce, src_id, prop)
        }
    }

    impl<T: Config> Pallet<T> {
        // ** Utility methods ***

        pub fn ensure_admin(origin: OriginFor<T>) -> DispatchResult {
            T::BridgeCommitteeOrigin::try_origin(origin)
                .map(|_| ())
                .or_else(ensure_root)?;
            Ok(())
        }

        /// Checks if who is a relayer
        pub fn is_relayer(who: &T::AccountId) -> bool {
            <Relayers<T>>::contains_key(who)
        }

        /// Provides an AccountId for the pallet.
        /// This is used both as an origin check and deposit/withdrawal account.
        pub fn account_id() -> T::AccountId {
            T::PalletId::get().into_account_truncating()
        }

        /// Asserts if a resource is registered
        pub fn resource_exists(id: [u8; 32]) -> bool {
            Self::resources(id) != None
        }

        /// Checks if a chain exists as a whitelisted destination
        pub fn chain_whitelisted(id: u8) -> bool {
            Self::chains(id) != None
        }

        /// Increments the deposit nonce for the specified chain ID
        fn bump_nonce(id: u8) -> u64 {
            let nonce = Self::chains(id).unwrap_or_default().saturating_add(1);
            <ChainNonces<T>>::insert(id, Some(nonce));
            nonce
        }

        // *** Admin methods ****

        /// Set a new voting threshold
        pub fn set_relayer_threshold(threshold: u32) -> DispatchResult {
            ensure!(threshold > 0, Error::<T>::InvalidThreshold);
            <RelayerThreshold<T>>::put(threshold);
            Self::deposit_event(Event::RelayerThresholdChanged(threshold));
            Ok(())
        }

        /// Register a method for a resource Id, enabling associated transfer
        pub fn register_resource(
            id: [u8; 32],
            method: Vec<u8>,
        ) -> DispatchResult {
            <Resources<T>>::insert(id, method);
            Ok(())
        }

        /// Removes a resource ID, disabling associated transfer
        pub fn unregister_resource(id: [u8; 32]) -> DispatchResult {
            <Resources<T>>::remove(id);
            Ok(())
        }

        /// Whitelist a chain ID for transfer
        pub fn whitelist(id: u8) -> DispatchResult {
            // Cannot whitelist this chain
            ensure!(id != T::BridgeChainId::get(), Error::<T>::InvalidChainId);
            // Cannot whitelist with an existing entry
            ensure!(
                !Self::chain_whitelisted(id),
                Error::<T>::ChainAlreadyWhitelisted
            );
            <ChainNonces<T>>::insert(&id, Some(0));
            Self::deposit_event(Event::ChainWhitelisted(id));
            Ok(())
        }

        /// Adds a new relayer to the set
        pub fn register_relayer(relayer: T::AccountId) -> DispatchResult {
            ensure!(
                !Self::is_relayer(&relayer),
                Error::<T>::RelayerAlreadyExists
            );
            <Relayers<T>>::insert(&relayer, ());
            <RelayerCount<T>>::mutate(|i| *i = i.saturating_add(1));
            Self::deposit_event(Event::RelayerAdded(relayer));
            Ok(())
        }

        /// Removes a relayer from the set
        pub fn unregister_relayer(relayer: T::AccountId) -> DispatchResult {
            ensure!(Self::is_relayer(&relayer), Error::<T>::RelayerInvalid);
            <Relayers<T>>::remove(&relayer);
            <RelayerCount<T>>::mutate(|i| *i = i.saturating_sub(1));
            Self::deposit_event(Event::RelayerRemoved(relayer));
            Ok(())
        }

        // *** Proposal voting and execution methods ***

        /// Commits a vote for a proposal. If the proposal doesn't exist it will be created.
        fn commit_vote(
            who: T::AccountId,
            nonce: u64,
            src_id: u8,
            prop: Box<T::Proposal>,
            in_favour: bool,
        ) -> DispatchResult {
            let now = <frame_system::Pallet<T>>::block_number();
            let mut votes = match <Votes<T>>::get(src_id, (nonce, prop.clone()))
            {
                Some(v) => v,
                None => {
                    let mut v = ProposalVotes::default();
                    v.expiry = now + T::ProposalLifetime::get();
                    v
                }
            };

            // Ensure the proposal isn't complete, proposal is not expired and relayer hasn't already votes
            ensure!(!votes.is_complete(), Error::<T>::ProposalAlreadyComplete);
            ensure!(!votes.is_expired(now), Error::<T>::ProposalExpired);
            ensure!(!votes.has_voted(&who), Error::<T>::RelayerAlreadyVoted);

            if in_favour {
                votes.votes_for.push(who.clone());
                Self::deposit_event(Event::VoteFor(src_id, nonce, who.clone()));
            } else {
                votes.votes_against.push(who.clone());
                Self::deposit_event(Event::VoteAgainst(
                    src_id,
                    nonce,
                    who.clone(),
                ));
            }

            <Votes<T>>::insert(src_id, (nonce, prop.clone()), votes.clone());

            Ok(())
        }

        /// Attempts to finalize or cancel the proposal if the vote count allows.
        fn try_resolve_proposal(
            nonce: u64,
            src_id: u8,
            prop: Box<T::Proposal>,
        ) -> DispatchResult {
            if let Some(mut votes) =
                <Votes<T>>::get(src_id, (nonce, prop.clone()))
            {
                let now = <frame_system::Pallet<T>>::block_number();
                ensure!(
                    !votes.is_complete(),
                    Error::<T>::ProposalAlreadyComplete
                );
                ensure!(!votes.is_expired(now), Error::<T>::ProposalExpired);

                let status = votes.try_to_complete(
                    <RelayerThreshold<T>>::get(),
                    <RelayerCount<T>>::get(),
                );
                <Votes<T>>::insert(
                    src_id,
                    (nonce, prop.clone()),
                    votes.clone(),
                );

                match status {
                    ProposalStatus::Approved => {
                        Self::finalize_execution(src_id, nonce, prop)
                    }
                    ProposalStatus::Rejected => {
                        Self::cancel_execution(src_id, nonce)
                    }
                    _ => Ok(()),
                }
            } else {
                Err(Error::<T>::ProposalDoesNotExist)?
            }
        }

        /// Commits a vote in favour of the proposal and executes it if the vote threshold is met.
        fn vote_for(
            who: T::AccountId,
            nonce: u64,
            src_id: u8,
            prop: Box<T::Proposal>,
        ) -> DispatchResult {
            Self::commit_vote(who, nonce, src_id, prop.clone(), true)?;
            Self::try_resolve_proposal(nonce, src_id, prop)
        }

        /// Commits a vote against the proposal and cancels it if more than
        /// (relayers.len() - threshold) votes against exist.
        fn vote_against(
            who: T::AccountId,
            nonce: u64,
            src_id: u8,
            prop: Box<T::Proposal>,
        ) -> DispatchResult {
            Self::commit_vote(who, nonce, src_id, prop.clone(), false)?;
            Self::try_resolve_proposal(nonce, src_id, prop)
        }

        /// Execute the proposal and signals the result as an event
        fn finalize_execution(
            src_id: u8,
            nonce: u64,
            call: Box<T::Proposal>,
        ) -> DispatchResult {
            Self::deposit_event(Event::ProposalApproved(src_id, nonce));
            call.dispatch(
                frame_system::RawOrigin::Signed(Self::account_id()).into(),
            )
            .map(|_| ())
            .map_err(|e| e.error)?;
            Self::deposit_event(Event::ProposalSucceeded(src_id, nonce));
            Ok(())
        }

        /// Cancels a proposal.
        fn cancel_execution(
            src_id: u8,
            nonce: u64,
        ) -> DispatchResult {
            Self::deposit_event(Event::ProposalRejected(src_id, nonce));
            Ok(())
        }

        /// Initiates a transfer of a fungible asset out of the chain. This should be called by
        /// another pallet
        pub fn transfer_fungible(
            dest_id: u8,
            resource_id: [u8; 32],
            to: Vec<u8>,
            amount: U256,
        ) -> DispatchResult {
            ensure!(
                Self::chain_whitelisted(dest_id),
                Error::<T>::ChainNotWhitelisted
            );
            let nonce = Self::bump_nonce(dest_id);
            <BridgeEvents<T>>::append(BridgeEvent::FungibleTransfer(
                dest_id,
                nonce,
                resource_id,
                amount,
                to.clone(),
            ));
            Self::deposit_event(Event::FungibleTransfer(
                dest_id,
                nonce,
                resource_id,
                amount,
                to,
            ));
            Ok(())
        }

        /// Initiates a transfer of a nunfungible asset out of the chain. This should be called by
        /// another pallet
        pub fn transfer_nonfungible(
            dest_id: u8,
            resource_id: [u8; 32],
            token_id: Vec<u8>,
            to: Vec<u8>,
            metadata: Vec<u8>,
        ) -> DispatchResult {
            ensure!(
                Self::chain_whitelisted(dest_id),
                Error::<T>::ChainNotWhitelisted
            );
            let nonce = Self::bump_nonce(dest_id);
            <BridgeEvents<T>>::append(BridgeEvent::NonFungibleTransfer(
                dest_id,
                nonce,
                resource_id,
                token_id.clone(),
                to.clone(),
                metadata.clone(),
            ));
            Self::deposit_event(Event::NonFungibleTransfer(
                dest_id,
                nonce,
                resource_id,
                token_id,
                to,
                metadata,
            ));
            Ok(())
        }

        /// Initiates a transfer of generic data out of the chain. This should be called by
        /// another pallet.
        pub fn transfer_generic(
            dest_id: u8,
            resource_id: [u8; 32],
            metadata: Vec<u8>,
        ) -> DispatchResult {
            ensure!(
                Self::chain_whitelisted(dest_id),
                Error::<T>::ChainNotWhitelisted
            );
            let nonce = Self::bump_nonce(dest_id);
            <BridgeEvents<T>>::append(BridgeEvent::GenericTransfer(
                dest_id,
                nonce,
                resource_id,
                metadata.clone(),
            ));
            Self::deposit_event(Event::GenericTransfer(
                dest_id,
                nonce,
                resource_id,
                metadata,
            ));
            Ok(())
        }
    }
}

/// Helper function to concatenate a chain ID and some bytes to produce a resource ID.
/// The common format is (31 bytes unique ID + 1 byte chain ID).
pub fn derive_resource_id(chain: u8, id: &[u8]) -> [u8; 32] {
    let mut r_id: [u8; 32] = [0; 32];
    r_id[31] = chain; // last byte is chain id
    let range = if id.len() > 31 { 31 } else { id.len() }; // Use at most 31 bytes
    for i in 0..range {
        r_id[30 - i] = id[range - 1 - i]; // Ensure left padding for eth compatibility
    }
    r_id
}

/// Simple ensure origin for the bridge account
pub struct EnsureBridge<T>(sp_std::marker::PhantomData<T>);

impl<T: Config> EnsureOrigin<T::RuntimeOrigin> for EnsureBridge<T> {
    type Success = T::AccountId;

    fn try_origin(o: T::RuntimeOrigin) -> Result<Self::Success, T::RuntimeOrigin> {
        let bridge_id = T::PalletId::get().into_account_truncating();
        o.into().and_then(|o| {
            match o {
                frame_system::RawOrigin::Signed(who) if who == bridge_id => {
                    Ok(bridge_id)
                }
                r => Err(T::RuntimeOrigin::from(r)),
            }
        })
    }

    /*
    /// Returns an outer origin capable of passing `try_origin` check.
    ///
    /// ** Should be used for benchmarking only!!! **
    #[cfg(feature = "runtime-benchmarks")]
    fn successful_origin() -> T::RuntimeOrigin {
        T::RuntimeOrigin::from(frame_system::RuntimeOrigin::Signed(<Pallet<T>>::account_id()))
    }
    */
}

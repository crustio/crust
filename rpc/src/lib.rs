//! Crust-specific RPCs implementation.

use std::sync::Arc;

use crust_runtime::opaque::Block;
use crust_primitives::{BlockNumber, AccountId, Index, Balance, Hash};
use sp_api::ProvideRuntimeApi;
use txpool_api::TransactionPool;
use sp_blockchain::{HeaderBackend, HeaderMetadata, Error as BlockChainError};
use sp_consensus::SelectChain;
use sp_consensus_babe::BabeApi;
use sc_client_api::light::{Fetcher, RemoteBlockchain};
use sc_consensus_babe::Epoch;
use sp_block_builder::BlockBuilder;
use sc_finality_grandpa::FinalityProofProvider;
pub use sc_rpc::{DenyUnsafe, SubscriptionTaskExecutor};

/// A type representing all RPC extensions.
pub type RpcExtension = jsonrpc_core::IoHandler<sc_rpc::Metadata>;

/// Light client extra dependencies.
pub struct LightDeps<C, F, P> {
    /// The client instance to use.
    pub client: Arc<C>,
    /// Transaction pool instance.
    pub pool: Arc<P>,
    /// Remote access to the blockchain (async).
    pub remote_blockchain: Arc<dyn RemoteBlockchain<Block>>,
    /// Fetcher instance.
    pub fetcher: Arc<F>,
}

/// Extra dependencies for BABE.
pub struct BabeDeps {
    /// BABE protocol config.
    pub babe_config: sc_consensus_babe::Config,
    /// BABE pending epoch changes.
    pub shared_epoch_changes: sc_consensus_epochs::SharedEpochChanges<Block, Epoch>,
    /// The keystore that manages the keys of the node.
    pub keystore: sc_keystore::KeyStorePtr,
}

/// Dependencies for GRANDPA
pub struct GrandpaDeps<B> {
    /// Voting round info.
    pub shared_voter_state: sc_finality_grandpa::SharedVoterState,
    /// Authority set info.
    pub shared_authority_set: sc_finality_grandpa::SharedAuthoritySet<Hash, BlockNumber>,
    /// Receives notifications about justification events from Grandpa.
    pub justification_stream: sc_finality_grandpa::GrandpaJustificationStream<Block>,
    /// Executor to drive the subscription manager in the Grandpa RPC handler.
    pub subscription_executor: sc_rpc::SubscriptionTaskExecutor,
    /// Finality proof provider.
    pub finality_provider: Arc<FinalityProofProvider<B, Block>>,
}

/// Full client dependencies
pub struct FullDeps<C, P, SC, B> {
    /// The client instance to use.
    pub client: Arc<C>,
    /// Transaction pool instance.
    pub pool: Arc<P>,
    /// The SelectChain Strategy
    pub select_chain: SC,
    /// Whether to deny unsafe calls
    pub deny_unsafe: DenyUnsafe,
    /// BABE specific dependencies.
    pub babe: BabeDeps,
    /// GRANDPA specific dependencies.
    pub grandpa: GrandpaDeps<B>,
}

/// Instantiate all RPC extensions.
pub fn create_full<C, P, SC, B>(deps: FullDeps<C, P, SC, B>) -> RpcExtension where
    C: ProvideRuntimeApi<Block>,
    C: HeaderBackend<Block> + HeaderMetadata<Block, Error=BlockChainError>,
    C: Send + Sync + 'static,
    C::Api: frame_rpc_system::AccountNonceApi<Block, AccountId, Index>,
    C::Api: pallet_transaction_payment_rpc::TransactionPaymentRuntimeApi<Block, Balance>,
    C::Api: BabeApi<Block>,
    C::Api: BlockBuilder<Block>,
    P: TransactionPool + Sync + Send + 'static,
    SC: SelectChain<Block> + 'static,
    B: sc_client_api::Backend<Block> + Send + Sync + 'static,
    B::State: sc_client_api::StateBackend<sp_runtime::traits::HashFor<Block>>,
{
    use frame_rpc_system::{FullSystem, SystemApi};
    use pallet_transaction_payment_rpc::{TransactionPayment, TransactionPaymentApi};
    use sc_finality_grandpa_rpc::{GrandpaApi, GrandpaRpcHandler};
    use sc_consensus_babe_rpc::BabeRpcHandler;

    let mut io = jsonrpc_core::IoHandler::default();
    let FullDeps {
        client,
        pool,
        select_chain,
        deny_unsafe,
        babe,
        grandpa,
    } = deps;
    let BabeDeps {
        keystore,
        babe_config,
        shared_epoch_changes,
    } = babe;
    let GrandpaDeps {
        shared_voter_state,
        shared_authority_set,
        justification_stream,
        subscription_executor,
        finality_provider
    } = grandpa;

    io.extend_with(
        SystemApi::to_delegate(FullSystem::new(client.clone(), pool, deny_unsafe))
    );
    io.extend_with(
        TransactionPaymentApi::to_delegate(TransactionPayment::new(client.clone()))
    );
    io.extend_with(
        sc_consensus_babe_rpc::BabeApi::to_delegate(
            BabeRpcHandler::new(
                client,
                shared_epoch_changes,
                keystore,
                babe_config,
                select_chain,
                deny_unsafe,
            )
        )
    );
    io.extend_with(
        GrandpaApi::to_delegate(GrandpaRpcHandler::new(
            shared_authority_set,
            shared_voter_state,
            justification_stream,
            subscription_executor,
            finality_provider
        ))
    );
    io
}

/// Instantiate all RPC extensions for light node.
pub fn create_light<C, P, F>(deps: LightDeps<C, F, P>) -> RpcExtension
    where
        C: ProvideRuntimeApi<Block>,
        C: HeaderBackend<Block>,
        C: Send + Sync + 'static,
        C::Api: frame_rpc_system::AccountNonceApi<Block, AccountId, Index>,
        C::Api: pallet_transaction_payment_rpc::TransactionPaymentRuntimeApi<Block, Balance>,
        P: TransactionPool + Sync + Send + 'static,
        F: Fetcher<Block> + 'static,
{
    use frame_rpc_system::{LightSystem, SystemApi};

    let LightDeps {
        client,
        pool,
        remote_blockchain,
        fetcher,
    } = deps;
    let mut io = jsonrpc_core::IoHandler::default();
    io.extend_with(
        SystemApi::<Hash, AccountId, Index>::to_delegate(LightSystem::new(client, remote_blockchain, fetcher, pool))
    );
    io
}

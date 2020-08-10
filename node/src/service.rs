//! Service and ServiceFactory implementation. Specialized wrapper over substrate service.

use std::sync::Arc;
use std::time::Duration;
use sc_client_api::RemoteBackend;
use crust_runtime::{self, opaque::Block, RuntimeApi};
use service::{error::Error as ServiceError, Configuration, ServiceComponents, TaskManager, Role};
use grandpa::{self, FinalityProofProvider as GrandpaFinalityProofProvider, StorageAndProofProvider, SharedVoterState};
use sc_executor::native_executor_instance;
pub use sc_executor::NativeExecutor;
use sp_inherents::InherentDataProviders;
use sc_consensus::LongestChain;

// Our native executor instance.
native_executor_instance!(
    pub Executor,
    crust_runtime::api::dispatch,
    crust_runtime::native_version,
    cstrml_swork::api::crypto::HostFunctions
);

type FullBackend = service::TFullBackend<Block>;
type FullSelectChain = LongestChain<FullBackend, Block>;
type FullClient = service::TFullClient<Block, RuntimeApi, Executor>;
type FullGrandpaBlockImport = grandpa::GrandpaBlockImport<
    FullBackend, Block, FullClient, FullSelectChain
>;

pub fn new_full_params(config: Configuration) -> Result<(
    service::ServiceParams<
        Block, FullClient,
        babe::BabeImportQueue<Block, FullClient>,
        sc_transaction_pool::FullPool<Block, FullClient>,
        (), FullBackend>,
    FullSelectChain,
    InherentDataProviders,
    babe::BabeBlockImport<Block, FullClient, FullGrandpaBlockImport>,
    grandpa::LinkHalf<Block, FullClient, FullSelectChain>,
    babe::BabeLink<Block>,
    ), ServiceError> {

    let inherent_data_providers = InherentDataProviders::new();

    let (client, backend, keystore, task_manager) =
        service::new_full_parts::<Block, RuntimeApi, Executor>(&config)?;
    let client = Arc::new(client);
    let select_chain = sc_consensus::LongestChain::new(backend.clone());

    let pool_api = sc_transaction_pool::FullChainApi::new(client.clone(), config.prometheus_registry());
    let transaction_pool = sc_transaction_pool::BasicPool::new_full(
    config.transaction_pool.clone(),
    std::sync::Arc::new(pool_api),
    config.prometheus_registry(),
    task_manager.spawn_handle(),
    client.clone(),
    );

    let (grandpa_block_import, grandpa_link) =
        grandpa::block_import(client.clone(), &(client.clone() as Arc<_>), select_chain.clone())?;

    let justification_import = grandpa_block_import.clone();

    let (babe_block_import, babe_link) = babe::block_import(
        babe::Config::get_or_compute(&*client)?,
        grandpa_block_import.clone(),
        client.clone(),
    )?;

    let import_queue = babe::import_queue(
        babe_link.clone(),
        babe_block_import.clone(),
        Some(Box::new(justification_import)),
        None,
        client.clone(),
        select_chain.clone(),
        inherent_data_providers.clone(),
        &task_manager.spawn_handle(),
        config.prometheus_registry(),
    )?;

    let provider = client.clone() as Arc<dyn StorageAndProofProvider<_, _>>;
    let finality_proof_provider =
        Arc::new(GrandpaFinalityProofProvider::new(backend.clone(), provider));

    let params = service::ServiceParams {
        backend, client, import_queue, keystore, task_manager, transaction_pool,
        config,
        block_announce_validator_builder: None,
        finality_proof_request_builder: None,
        finality_proof_provider: Some(finality_proof_provider),
        on_demand: None,
        remote_blockchain: None,
        rpc_extensions_builder: Box::new(|_| ()),
    };

    Ok((
        params, select_chain,
        inherent_data_providers,
        babe_block_import, grandpa_link, babe_link
    ))
}

/// Builds a new service for a full client.
pub fn new_full(config: Configuration) -> Result<TaskManager, ServiceError>
{
    use sc_network::Event;
    use sc_client_api::ExecutorProvider;
    use futures::stream::StreamExt;

    let (
        params, select_chain, inherent_data_providers,
        babe_block_import, grandpa_link, babe_link
    ) = new_full_params(config)?;

    let (
        role, force_authoring, name, enable_grandpa, prometheus_registry,
        client, transaction_pool, keystore,
    ) = {
        let service::ServiceParams {
            config, client, transaction_pool, keystore, ..
        } = &params;

        (
            config.role.clone(),
            config.force_authoring,
            config.network.node_name.clone(),
            !config.disable_grandpa,
            config.prometheus_registry().cloned(),
            client.clone(), transaction_pool.clone(), keystore.clone(),
        )
    };
    let ServiceComponents {
        task_manager, network, telemetry_on_connect_sinks, ..
    } = service::build(params)?;

    if role.is_authority() {
        let proposer =
            sc_basic_authorship::ProposerFactory::new(
                client.clone(),
                transaction_pool,
                prometheus_registry.as_ref()
            );

        let can_author_with =
            sp_consensus::CanAuthorWithNativeVersion::new(client.executor().clone());

        let babe_config = babe::BabeParams {
            keystore: keystore.clone(),
            client: client.clone(),
            select_chain,
            env: proposer,
            block_import: babe_block_import,
            sync_oracle: network.clone(),
            inherent_data_providers: inherent_data_providers.clone(),
            force_authoring,
            babe_link,
            can_author_with,
        };

        let babe = babe::start_babe(babe_config)?;

        // the BABE authoring task is considered essential, i.e. if it
        // fails we take down the service with it.
        task_manager.spawn_essential_handle().spawn_blocking("babe", babe);

        // Authority discovery: this module runs to promise authorities' connection
        // TODO: [Substrate]refine sentry mode using updated substrate code
        if matches!(role, Role::Authority{..} | Role::Sentry{..}) {
            let (sentries, authority_discovery_role) = match role {
                Role::Authority { ref sentry_nodes } => (
                    sentry_nodes.clone(),
                    authority_discovery::Role::Authority (
                        keystore.clone(),
                    ),
                ),
                Role::Sentry {..} => (
                    vec![],
                    authority_discovery::Role::Sentry,
                ),
                _ => unreachable!("Due to outer matches! constraint; qed."),
            };
            let network_event_stream = network.event_stream("authority-discovery");
            let dht_event_stream = network_event_stream.filter_map(|e| async move {
                match e {
                    Event::Dht(e) => Some(e),
                    _ => None,
                }
            }).boxed();
            let authority_discovery = authority_discovery::AuthorityDiscovery::new(
                client.clone(),
                network.clone(),
                sentries,
                dht_event_stream,
                authority_discovery_role,
                prometheus_registry.clone(),
            );
            task_manager.spawn_handle().spawn("authority-discovery", authority_discovery);
        }
    }

    // if the node isn't actively participating in consensus then it doesn't
    // need a keystore, regardless of which protocol we use below.
    let keystore = if role.is_authority() {
        Some(keystore as sp_core::traits::BareCryptoStorePtr)
    } else {
        None
    };

    let grandpa_config = grandpa::Config {
        // FIXME: [Substrate]substrate/issues#1578 make this available through chainspec
        gossip_duration: Duration::from_millis(1000),
        justification_period: 512,
        name: Some(name),
        observer_enabled: false,
        keystore,
        is_authority: role.is_network_authority(),
    };

    if enable_grandpa {
        // start the full GRANDPA voter
        // NOTE: unlike in substrate we are currently running the full
        // GRANDPA voter protocol for all full nodes (regardless of whether
        // they're validators or not). at this point the full voter should
        // provide better guarantees of block and vote data availability than
        // the observer.

        // add a custom voting rule to temporarily stop voting for new blocks
        // after the given pause block is finalized and restarting after the
        // given delay.
        let grandpa_config = grandpa::GrandpaParams {
            config: grandpa_config,
            link: grandpa_link,
            network,
            inherent_data_providers,
            telemetry_on_connect: Some(telemetry_on_connect_sinks.on_connect_stream()),
            voting_rule: grandpa::VotingRulesBuilder::default().build(),
            prometheus_registry,
            shared_voter_state: SharedVoterState::empty()
        };

        // the GRANDPA voter task is considered infallible, i.e.
        // if it fails we take down the service with it.
        task_manager.spawn_essential_handle().spawn_blocking(
            "grandpa-voter",
            grandpa::run_grandpa_voter(grandpa_config)?
        );
    } else {
        grandpa::setup_disabled_grandpa(
            client.clone(),
            &inherent_data_providers,
            network,
        )?;
    }

    Ok(task_manager)
}

/// Builds a new service for a light client.
pub fn new_light(config: Configuration) -> Result<TaskManager, ServiceError> {
    let (client, backend, keystore, task_manager, on_demand) =
        service::new_light_parts::<Block, RuntimeApi, Executor>(&config)?;

    let transaction_pool_api = Arc::new(sc_transaction_pool::LightChainApi::new(
        client.clone(), on_demand.clone(),
    ));

    let select_chain = LongestChain::new(backend.clone());
    let transaction_pool = sc_transaction_pool::BasicPool::new_light(
        config.transaction_pool.clone(),
        transaction_pool_api,
        config.prometheus_registry(),
        task_manager.spawn_handle(),
    );

    let grandpa_block_import = grandpa::light_block_import(
        client.clone(), backend.clone(), &(client.clone() as Arc<_>),
        Arc::new(on_demand.checker().clone()) as Arc<_>,
    )?;
    let finality_proof_import = grandpa_block_import.clone();
    let finality_proof_request_builder =
        finality_proof_import.create_finality_proof_request_builder();

    let (babe_block_import, babe_link) = babe::block_import(
        babe::Config::get_or_compute(&*client)?,
        grandpa_block_import,
        client.clone(),
    )?;
    let inherent_data_providers = InherentDataProviders::new();

    // FIXME: pruning task isn't started since light client doesn't do `AuthoritySetup`.
    let import_queue = babe::import_queue(
        babe_link,
        babe_block_import,
        None,
        Some(Box::new(finality_proof_import)),
        client.clone(),
        select_chain,
        inherent_data_providers.clone(),
        &task_manager.spawn_handle(),
        config.prometheus_registry(),
    )?;

    let finality_proof_provider =
        Arc::new(GrandpaFinalityProofProvider::new(backend.clone(), client.clone() as Arc<_>));

    service::build(service::ServiceParams {
        block_announce_validator_builder: None,
        finality_proof_request_builder: Some(finality_proof_request_builder),
        finality_proof_provider: Some(finality_proof_provider),
        on_demand: Some(on_demand),
        remote_blockchain: Some(backend.remote_blockchain()),
        rpc_extensions_builder: Box::new(|_| ()),
        transaction_pool: Arc::new(transaction_pool),
        config, client, import_queue, keystore, backend, task_manager
    }).map(|ServiceComponents { task_manager, .. }| task_manager)
}
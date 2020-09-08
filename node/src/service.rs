//! Service and ServiceFactory implementation. Specialized wrapper over substrate service.

use std::sync::Arc;
use std::time::Duration;
use sc_client_api::{RemoteBackend, ExecutorProvider};
use crust_runtime::{self, opaque::Block, RuntimeApi};
use service::{error::Error as ServiceError, Configuration, TaskManager, Role};
use grandpa::{self, FinalityProofProvider as GrandpaFinalityProofProvider};
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

pub fn new_partial(config: &Configuration) -> Result<
    service::PartialComponents<
        FullClient, FullBackend, FullSelectChain,
        sp_consensus::DefaultImportQueue<Block, FullClient>,
        sc_transaction_pool::FullPool<Block, FullClient>,
        (
            impl Fn(crust_rpc::DenyUnsafe, crust_rpc::SubscriptionManager) -> crust_rpc::RpcExtension,
            (
                babe::BabeBlockImport<Block, FullClient, FullGrandpaBlockImport>,
                grandpa::LinkHalf<Block, FullClient, FullSelectChain>,
                babe::BabeLink<Block>
            ),
            grandpa::SharedVoterState,
        )
    >, ServiceError> {

    let inherent_data_providers = InherentDataProviders::new();

    let (client, backend, keystore, task_manager) =
        service::new_full_parts::<Block, RuntimeApi, Executor>(&config)?;
    let client = Arc::new(client);

    let select_chain = sc_consensus::LongestChain::new(backend.clone());

    let transaction_pool = sc_transaction_pool::BasicPool::new_full(
    config.transaction_pool.clone(),
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
        sp_consensus::CanAuthorWithNativeVersion::new(client.executor().clone()),
    )?;

    let justification_stream = grandpa_link.justification_stream();
    let shared_authority_set = grandpa_link.shared_authority_set().clone();
    let shared_voter_state = grandpa::SharedVoterState::empty();

    let import_setup = (babe_block_import.clone(), grandpa_link, babe_link.clone());
    let rpc_setup = shared_voter_state.clone();

    let babe_config = babe_link.config().clone();
    let shared_epoch_changes = babe_link.epoch_changes().clone();

    let rpc_extensions_builder = {
        let client = client.clone();
        let keystore = keystore.clone();
        let transaction_pool = transaction_pool.clone();
        let select_chain = select_chain.clone();

        move |deny_unsafe, subscriptions| -> crust_rpc::RpcExtension {
            let deps = crust_rpc::FullDeps {
                client: client.clone(),
                pool: transaction_pool.clone(),
                select_chain: select_chain.clone(),
                deny_unsafe,
                babe: crust_rpc::BabeDeps {
                    babe_config: babe_config.clone(),
                    shared_epoch_changes: shared_epoch_changes.clone(),
                    keystore: keystore.clone(),
                },
                grandpa: crust_rpc::GrandpaDeps {
                    shared_voter_state: shared_voter_state.clone(),
                    shared_authority_set: shared_authority_set.clone(),
                    justification_stream: justification_stream.clone(),
                    subscriptions,
                },
            };

            crust_rpc::create_full(deps)
        }
    };

    Ok(service::PartialComponents {
        client,
        backend,
        task_manager,
        keystore,
        select_chain,
        import_queue,
        transaction_pool,
        inherent_data_providers,
        other: (rpc_extensions_builder, import_setup, rpc_setup)
    })
}

/// Builds a new service for a full client.
pub fn new_full(config: Configuration) -> Result<TaskManager, ServiceError>
{
    use sc_network::Event;
    use futures::stream::StreamExt;

    let role = config.role.clone();
    let is_authority = role.is_authority();
    let force_authoring = config.force_authoring;
    let disable_grandpa = config.disable_grandpa;
    let name = config.network.node_name.clone();

    let service::PartialComponents {
        client, backend, mut task_manager, keystore, select_chain, import_queue, transaction_pool,
        inherent_data_providers,
        other: (rpc_extensions_builder, import_setup, rpc_setup)
    } = new_partial(&config)?;

    let prometheus_registry = config.prometheus_registry().cloned();

    let finality_proof_provider =
        GrandpaFinalityProofProvider::new_for_service(backend.clone(), client.clone());

    let (network, network_status_sinks, system_rpc_tx, network_starter) =
        service::build_network(service::BuildNetworkParams {
            config: &config,
            client: client.clone(),
            transaction_pool: transaction_pool.clone(),
            spawn_handle: task_manager.spawn_handle(),
            import_queue,
            on_demand: None,
            block_announce_validator_builder: None,
            finality_proof_request_builder: None,
            finality_proof_provider: Some(finality_proof_provider.clone()),
        })?;

    if config.offchain_worker.enabled {
        service::build_offchain_workers(
            &config, backend.clone(), task_manager.spawn_handle(), client.clone(), network.clone(),
        );
    }

    let telemetry_connection_sinks = service::TelemetryConnectionSinks::default();

    let _ = service::spawn_tasks(service::SpawnTasksParams {
        config,
        backend: backend.clone(),
        client: client.clone(),
        keystore: keystore.clone(),
        network: network.clone(),
        rpc_extensions_builder: Box::new(rpc_extensions_builder),
        transaction_pool: transaction_pool.clone(),
        task_manager: &mut task_manager,
        on_demand: None,
        remote_blockchain: None,
        telemetry_connection_sinks: telemetry_connection_sinks.clone(),
        network_status_sinks, system_rpc_tx,
    })?;

    let (babe_block_import, grandpa_link, babe_link) = import_setup;

    let shared_voter_state = rpc_setup;

    if is_authority {
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
            let (authority_discovery_worker, _service) = authority_discovery::new_worker_and_service(
                client.clone(),
                network.clone(),
                sentries,
                dht_event_stream,
                authority_discovery_role,
                prometheus_registry.clone(),
            );
            task_manager.spawn_handle().spawn("authority-discovery-worker", authority_discovery_worker);
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

    if !disable_grandpa {
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
            telemetry_on_connect: Some(telemetry_connection_sinks.on_connect_stream()),
            voting_rule: grandpa::VotingRulesBuilder::default().build(),
            prometheus_registry,
            shared_voter_state
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

    network_starter.start_network();

    Ok(task_manager)
}

/// Builds a new service for a light client.
pub fn new_light(config: Configuration) -> Result<TaskManager, ServiceError> {
    let (client, backend, keystore, mut task_manager, on_demand) =
        service::new_light_parts::<Block, RuntimeApi, Executor>(&config)?;

    let select_chain = LongestChain::new(backend.clone());

    let transaction_pool = Arc::new(sc_transaction_pool::BasicPool::new_light(
        config.transaction_pool.clone(),
        config.prometheus_registry(),
        task_manager.spawn_handle(),
        client.clone(),
        on_demand.clone(),
    ));

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
        select_chain.clone(),
        inherent_data_providers.clone(),
        &task_manager.spawn_handle(),
        config.prometheus_registry(),
        sp_consensus::NeverCanAuthor,
    )?;

    let finality_proof_provider =
        GrandpaFinalityProofProvider::new_for_service(backend.clone(), client.clone());

    let (network, network_status_sinks, system_rpc_tx, network_starter) =
        service::build_network(service::BuildNetworkParams {
            config: &config,
            client: client.clone(),
            transaction_pool: transaction_pool.clone(),
            spawn_handle: task_manager.spawn_handle(),
            import_queue,
            on_demand: Some(on_demand.clone()),
            block_announce_validator_builder: None,
            finality_proof_request_builder: Some(finality_proof_request_builder),
            finality_proof_provider: Some(finality_proof_provider),
        })?;

    if config.offchain_worker.enabled {
        service::build_offchain_workers(
            &config, backend.clone(), task_manager.spawn_handle(), client.clone(), network.clone(),
        );
    }

    let light_deps = crust_rpc::LightDeps {
        remote_blockchain: backend.remote_blockchain(),
        fetcher: on_demand.clone(),
        client: client.clone(),
        pool: transaction_pool.clone(),
    };

    let rpc_extensions = crust_rpc::create_light(light_deps);

    let _ = service::spawn_tasks(service::SpawnTasksParams {
        on_demand: Some(on_demand),
        remote_blockchain: Some(backend.remote_blockchain()),
        rpc_extensions_builder: Box::new(service::NoopRpcExtensionBuilder(rpc_extensions)),
        task_manager: &mut task_manager,
        telemetry_connection_sinks: service::TelemetryConnectionSinks::default(),
        config, keystore, backend, transaction_pool, client, network, network_status_sinks,
        system_rpc_tx,
    })?;

    network_starter.start_network();

    Ok(task_manager)
}
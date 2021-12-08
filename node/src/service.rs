// Copyright (C) 2019-2021 Crust Network Technologies Ltd.
// This file is part of Crust.

//! Crust service. Specialized wrapper over substrate service.

use std::sync::Arc;
use std::time::Duration;
use sc_client_api::{RemoteBackend, ExecutorProvider};
use crust_runtime::{self, opaque::Block, RuntimeApi};
use sc_service::{error::Error as ServiceError, Configuration, TaskManager};
use sc_finality_grandpa::{self, AuthoritySetHardFork, FinalityProofProvider as GrandpaFinalityProofProvider};
use sc_executor::native_executor_instance;
pub use sc_executor::NativeExecutor;
use sp_inherents::InherentDataProviders;
use sc_consensus::LongestChain;

// Our native executor instance.
// TODO: Bring benchmarks back
native_executor_instance!(
    pub CrustExecutor,
    crust_runtime::api::dispatch,
    crust_runtime::native_version
);

type FullBackend = sc_service::TFullBackend<Block>;
type FullSelectChain = LongestChain<FullBackend, Block>;
type FullClient = sc_service::TFullClient<Block, RuntimeApi, CrustExecutor>;
type FullGrandpaBlockImport = sc_finality_grandpa::GrandpaBlockImport<
    FullBackend, Block, FullClient, FullSelectChain
>;

pub fn new_partial(config: &Configuration) -> Result<
    sc_service::PartialComponents<
        FullClient, FullBackend, FullSelectChain,
        sp_consensus::DefaultImportQueue<Block, FullClient>,
        sc_transaction_pool::FullPool<Block, FullClient>,
        (
            impl Fn(
                crust_rpc::DenyUnsafe,
                crust_rpc::SubscriptionTaskExecutor
            ) -> crust_rpc::RpcExtension,
            (
                sc_consensus_babe::BabeBlockImport<
                    Block, FullClient, FullGrandpaBlockImport
                >,
                sc_finality_grandpa::LinkHalf<Block, FullClient, FullSelectChain>,
                sc_consensus_babe::BabeLink<Block>
            ),
            sc_finality_grandpa::SharedVoterState
        )
    >,
    ServiceError> {

    let inherent_data_providers = InherentDataProviders::new();

    let (client, backend, keystore_container, task_manager) =
        sc_service::new_full_parts::<Block, RuntimeApi, CrustExecutor>(&config)?;
    let client = Arc::new(client);

    let select_chain = sc_consensus::LongestChain::new(backend.clone());

    let transaction_pool = sc_transaction_pool::BasicPool::new_full(
    config.transaction_pool.clone(),
    config.role.is_authority().into(),
    config.prometheus_registry(),
    task_manager.spawn_handle(),
    client.clone(),
    );

    let (grandpa_block_import, grandpa_link) =
        sc_finality_grandpa::block_import_with_authority_set_hard_forks(
            client.clone(),
            &(client.clone() as Arc<_>),
            select_chain.clone(),
            grandpa_mainnet_hard_forks(),
        )?;

    let justification_import = grandpa_block_import.clone();

    let (babe_block_import, babe_link) = sc_consensus_babe::block_import(
        sc_consensus_babe::Config::get_or_compute(&*client)?,
        grandpa_block_import,
        client.clone(),
    )?;

    let import_queue = sc_consensus_babe::import_queue(
        babe_link.clone(),
        babe_block_import.clone(),
        Some(Box::new(justification_import)),
        client.clone(),
        select_chain.clone(),
        inherent_data_providers.clone(),
        &task_manager.spawn_handle(),
        config.prometheus_registry(),
        sp_consensus::CanAuthorWithNativeVersion::new(client.executor().clone()),
    )?;

    let justification_stream = grandpa_link.justification_stream();
    let shared_authority_set = grandpa_link.shared_authority_set().clone();
    let shared_voter_state = sc_finality_grandpa::SharedVoterState::empty();
    let finality_proof_provider =
        GrandpaFinalityProofProvider::new_for_service(backend.clone(), Some(shared_authority_set.clone()));

    let import_setup = (babe_block_import.clone(), grandpa_link, babe_link.clone());
    let rpc_setup = shared_voter_state.clone();

    let babe_config = babe_link.config().clone();
    let shared_epoch_changes = babe_link.epoch_changes().clone();

    let rpc_extensions_builder = {
        let client = client.clone();
        let keystore = keystore_container.sync_keystore();
        let transaction_pool = transaction_pool.clone();
        let select_chain = select_chain.clone();

        move |deny_unsafe, subscription_executor| -> crust_rpc::RpcExtension {
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
                    subscription_executor,
                    finality_provider: finality_proof_provider.clone(),
                },
            };

            crust_rpc::create_full(deps)
        }
    };

    Ok(sc_service::PartialComponents {
        client,
        backend,
        task_manager,
        keystore_container,
        select_chain,
        import_queue,
        transaction_pool,
        inherent_data_providers,
        other: (rpc_extensions_builder, import_setup, rpc_setup)
    })
}

/// Builds a new service for a full client.
pub fn new_full(mut config: Configuration) -> Result<TaskManager, ServiceError>
{
    let sc_service::PartialComponents {
        client,
        backend,
        mut task_manager,
        keystore_container,
        select_chain,
        import_queue,
        transaction_pool,
        inherent_data_providers,
        other: (rpc_extensions_builder, import_setup, rpc_setup)
    } = new_partial(&config)?;

    let role = config.role.clone();
    let force_authoring = config.force_authoring;
    let disable_grandpa = config.disable_grandpa;
    let name = config.network.node_name.clone();
    let backoff_authoring_blocks: Option<()> = None;
    let prometheus_registry = config.prometheus_registry().cloned();

    let shared_voter_state = rpc_setup;

    config.network.extra_sets.push(sc_finality_grandpa::grandpa_peers_set_config());

    let (network, network_status_sinks, system_rpc_tx, network_starter) =
        sc_service::build_network(sc_service::BuildNetworkParams {
            config: &config,
            client: client.clone(),
            transaction_pool: transaction_pool.clone(),
            spawn_handle: task_manager.spawn_handle(),
            import_queue,
            on_demand: None,
            block_announce_validator_builder: None,
        })?;

    if config.offchain_worker.enabled {
        sc_service::build_offchain_workers(
            &config, backend.clone(), task_manager.spawn_handle(), client.clone(), network.clone(),
        );
    }

    let (_rpc_handlers, telemetry_connection_notifier) = sc_service::spawn_tasks(sc_service::SpawnTasksParams {
        config,
        backend: backend.clone(),
        client: client.clone(),
        keystore: keystore_container.sync_keystore(),
        network: network.clone(),
        rpc_extensions_builder: Box::new(rpc_extensions_builder),
        transaction_pool: transaction_pool.clone(),
        task_manager: &mut task_manager,
        on_demand: None,
        remote_blockchain: None,
        network_status_sinks,
        system_rpc_tx,
    })?;

    let (babe_block_import, grandpa_link, babe_link) = import_setup;

    if role.is_authority() {
        let proposer =
            sc_basic_authorship::ProposerFactory::new(
                task_manager.spawn_handle(),
                client.clone(),
                transaction_pool,
                prometheus_registry.as_ref()
            );

        let can_author_with =
            sp_consensus::CanAuthorWithNativeVersion::new(client.executor().clone());

        let babe_config = sc_consensus_babe::BabeParams {
            keystore: keystore_container.sync_keystore(),
            client: client.clone(),
            select_chain,
            env: proposer,
            block_import: babe_block_import,
            sync_oracle: network.clone(),
            inherent_data_providers: inherent_data_providers.clone(),
            force_authoring,
            backoff_authoring_blocks,
            babe_link,
            can_author_with,
        };

        let babe = sc_consensus_babe::start_babe(babe_config)?;

        // the BABE authoring task is considered essential, i.e. if it
        // fails we take down the service with it.
        task_manager.spawn_essential_handle().spawn_blocking("babe", babe);

        // Authority discovery: this module runs to promise authorities' connection
        use sc_network::Event;
        use futures::StreamExt;

        let authority_discovery_role = if role.is_authority() {
            authority_discovery::Role::PublishAndDiscover(
                keystore_container.keystore(),
            )
        } else {
            // don't publish our addresses when we're only a collator
            authority_discovery::Role::Discover
        };
        let dht_event_stream = network.event_stream("authority-discovery")
            .filter_map(|e| async move { match e {
                Event::Dht(e) => Some(e),
                _ => None,
            }});
        let (worker, _service) = authority_discovery::new_worker_and_service(
            client.clone(),
            network.clone(),
            Box::pin(dht_event_stream),
            authority_discovery_role,
            prometheus_registry.clone(),
        );

        task_manager.spawn_handle().spawn("authority-discovery-worker", worker.run());
    }

    // if the node isn't actively participating in consensus then it doesn't
    // need a keystore, regardless of which protocol we use below.
    let keystore = if role.is_authority() {
        Some(keystore_container.sync_keystore())
    } else {
        None
    };

    let grandpa_config = sc_finality_grandpa::Config {
        // FIXME: [Substrate]substrate/issues#1578 make this available through chain spec
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
        let grandpa_config = sc_finality_grandpa::GrandpaParams {
            config: grandpa_config,
            link: grandpa_link,
            network: network.clone(),
            telemetry_on_connect: telemetry_connection_notifier.map(|x| x.on_connect_stream()),
            voting_rule: sc_finality_grandpa::VotingRulesBuilder::default().build(),
            prometheus_registry,
            shared_voter_state
        };

        // the GRANDPA voter task is considered infallible, i.e.
        // if it fails we take down the service with it.
        task_manager.spawn_essential_handle().spawn_blocking(
            "grandpa-voter",
            sc_finality_grandpa::run_grandpa_voter(grandpa_config)?
        );
    }

    network_starter.start_network();

    Ok(task_manager)
}

/// On block #2080276 the network had some issues that led to finality being stalled. In order to
/// recover the network a bunch of `grandpa.noteStalled` extrinsics were pushed, unfortunately this
/// led to an inconsistency in the number of authority set changes that were applied, in particular
/// authorities that were online at the time applied one extra forced change and therefore their set
/// id will be off by one when compared to syncing nodes. This makes it so that e.g. signature
/// verification of justifications fails and syncing nodes cannot get to the latest finalized block.
/// In order to fix this we need to "manually" insert a forced change as a GRANDPA hard fork below.
/// The list of authorities below is taken from block #2080862 (https://crust.subscan.io/extrinsic/2080862-0?event=2080862-1),
/// and the manual forced change is inserted at #2080800.
pub(crate) fn grandpa_mainnet_hard_forks() -> Vec<AuthoritySetHardFork<Block>> {
    use std::str::FromStr;
    use sp_core::crypto::Ss58Codec;

    let authorities = vec![
        // 710a4eb0d7eb4d399fc824134b27055914d26a67a49e929c4d64200f83c2e936 (5EcvL6bL...)
        "5EcvL6bLUYPmr2GGZjkGNpQUCCgGVf8imVwF7D5rvUWXzkSn",
        // 109409eba4571f2a31a2ad61fee8eda976606e953813956ea2d05af5f43a856d (5CSSbkgw...)
        "5CSSbkgwsvotFhFdsy6KbyQXPVux1PHrNcso1dGgyjErnevM",
        // 6a0519ec737e402f92c7802428d3249abdaece67cbe15b5662f3fd3776733e34 (5ETiSxSR...)
        "5ETiSxSRryzBTsqzRJsxyuav4g4FwmZwWPLsPnxz1UgJMS1w",
        // 6c2dd59b305af2edee2d785fb89142d00cd4a4023a9771f3753f3d44a16ae12d (5EWYeKuo...)
        "5EWYeKuo13L56kzb3tCgyq8A1ehLQiL8wX9zAfQdCm9ghRK7",
        // df48d76e5963d94bb3cbf98bafbfd01742d00a76e55b30691e131e3e9f94d4bf (5H7UB3GD...)
        "5H7UB3GDRvaanzGdxFBeEv2qF1cHU87YTpm2NcBp9UZ1NGg3",
        // e310e3d448225456845dd24284555730ce8c7606a25245cd0669f3d4dd975e7d (5HCRk8G2...)
        "5HCRk8G2M3nFGG8VamTkbznfzFCELPx9ekVwtXrGKc3ehJuy",
        // 92d3f200161ca558ae8b4995fd41b2a32524236237fcd1c48468248118475afe (5FPDotXR...)
        "5FPDotXRfXwsDD9pyrooH8wPoDqvPEwUsKqhRD4adBP3MDeD",
        // f82870a932ca87247d7ecb8ef5844a5907fc4e2172f8f2a7e9158e4f09f3f56c (5Hg5kD4R...)
        "5Hg5kD4RMjW1unMutF2JHrNkzcMJL9kUdmTthH98vcJpnwZ1",
        // 4d0be5a393045dbaf170e3a828173e8211bd2f67146c24bdf093736ad8d78812 (5Doj5W7q...)
        "5Doj5W7qVmW7QjCuBS2VNPKgDNdzFU2oKuoAUvdC81Yjj9cK",
        // b07dd100bea10b1497e0e0de5f3a7bd7bf9fda374138be6ebe24e8ac69b8c41b (5G47fF16...)
        "5G47fF16SxQJCDTY3wEvWEjPFAm8rAB4ZAqh8dnBo9zkCfJP"
    ];

    let set_id = 607;
    let block_number = 2080800;
    let block_hash = "0x1967d464a2f26354491c5c00dfcf906234d9689fb72ae4741305a949287604f2";
    let last_finalized = 2080278;

    let block_hash = primitives::Hash::from_str(block_hash)
        .expect("hard fork hashes are static and they should be carefully defined; qed.");

    let authorities = authorities
        .into_iter()
        .map(|address| {
            (
                sp_finality_grandpa::AuthorityId::from_ss58check(address)
                    .expect("hard fork authority addresses are static and they should be carefully defined; qed."),
                1,
            )
        })
        .collect::<Vec<_>>();

    vec![
        AuthoritySetHardFork {
            set_id,
            block: (block_hash, block_number),
            authorities,
            last_finalized: Some(last_finalized),
        }
    ]
}

/// Builds a new service for a light client.
pub fn new_light(mut config: Configuration) -> Result<TaskManager, ServiceError> {
    let (client, backend, keystore, mut task_manager, on_demand) =
        sc_service::new_light_parts::<Block, RuntimeApi, CrustExecutor>(&config)?;

    config.network.extra_sets.push(sc_finality_grandpa::grandpa_peers_set_config());

    let select_chain = LongestChain::new(backend.clone());

    let transaction_pool = Arc::new(sc_transaction_pool::BasicPool::new_light(
        config.transaction_pool.clone(),
        config.prometheus_registry(),
        task_manager.spawn_handle(),
        client.clone(),
        on_demand.clone(),
    ));

    let (grandpa_block_import, _) = sc_finality_grandpa::block_import_with_authority_set_hard_forks(
        client.clone(),
        &(client.clone() as Arc<_>),
        select_chain.clone(),
        grandpa_mainnet_hard_forks(),
    )?;
    let finality_proof_import = grandpa_block_import.clone();

    let (babe_block_import, babe_link) = sc_consensus_babe::block_import(
        sc_consensus_babe::Config::get_or_compute(&*client)?,
        grandpa_block_import,
        client.clone(),
    )?;
    let inherent_data_providers = InherentDataProviders::new();

    // FIXME: pruning task isn't started since light client doesn't do `AuthoritySetup`.
    let import_queue = sc_consensus_babe::import_queue(
        babe_link,
        babe_block_import,
        Some(Box::new(finality_proof_import)),
        client.clone(),
        select_chain.clone(),
        inherent_data_providers.clone(),
        &task_manager.spawn_handle(),
        config.prometheus_registry(),
        sp_consensus::NeverCanAuthor,
    )?;

    let (network, network_status_sinks, system_rpc_tx, network_starter) =
        sc_service::build_network(sc_service::BuildNetworkParams {
            config: &config,
            client: client.clone(),
            transaction_pool: transaction_pool.clone(),
            spawn_handle: task_manager.spawn_handle(),
            import_queue,
            on_demand: Some(on_demand.clone()),
            block_announce_validator_builder: None,
        })?;

    if config.offchain_worker.enabled {
        sc_service::build_offchain_workers(
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

    let _ = sc_service::spawn_tasks(sc_service::SpawnTasksParams {
        on_demand: Some(on_demand),
        remote_blockchain: Some(backend.remote_blockchain()),
        rpc_extensions_builder: Box::new(sc_service::NoopRpcExtensionBuilder(rpc_extensions)),
        task_manager: &mut task_manager,
        config,
        keystore: keystore.sync_keystore(),
        backend,
        transaction_pool,
        client,
        network,
        network_status_sinks,
        system_rpc_tx,
    })?;

    network_starter.start_network();

    Ok(task_manager)
}
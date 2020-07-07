//! Service and ServiceFactory implementation. Specialized wrapper over substrate service.

use std::sync::Arc;
use std::time::Duration;
use crust_runtime::{self, opaque::Block, RuntimeApi};
use grandpa::{
    self,
    FinalityProofProvider as GrandpaFinalityProofProvider, StorageAndProofProvider, SharedVoterState
};
use sc_consensus::LongestChain;
use sc_executor::native_executor_instance;
pub use sc_executor::NativeExecutor;
use sc_service::{error::Error as ServiceError, AbstractService, Configuration, ServiceBuilder, Role};
use sp_inherents::InherentDataProviders;

// Our native executor instance.
native_executor_instance!(
    pub Executor,
    crust_runtime::api::dispatch,
    crust_runtime::native_version,
    cstrml_tee::api::crypto::HostFunctions
);

/// Starts a `ServiceBuilder` for a full service.
///
/// Use this macro if you don't actually need the full service, but just the builder in order to
/// be able to perform chain operations.
macro_rules! new_full_start {
	($config:expr) => {{
		use std::sync::Arc;

		// TODO: add prometheus setting?
		let mut import_setup = None;
		let inherent_data_providers = sp_inherents::InherentDataProviders::new();

		let builder = sc_service::ServiceBuilder::new_full::<
			crust_runtime::opaque::Block, crust_runtime::RuntimeApi, crate::service::Executor
		>($config)?
			.with_select_chain(|_config, backend| {
				Ok(sc_consensus::LongestChain::new(backend.clone()))
			})?
			.with_transaction_pool(|builder| {
				let pool_api = sc_transaction_pool::FullChainApi::new(
					builder.client().clone(),
				);
				Ok(sc_transaction_pool::BasicPool::new(
					builder.config().transaction_pool.clone(),
					std::sync::Arc::new(pool_api),
					builder.prometheus_registry(),
				))
			})?
			.with_import_queue(|
			    _config,
			    client,
			    mut select_chain,
			    _,
			    spawn_task_handle,
			    registry
            | {
				let select_chain = select_chain.take()
					.ok_or_else(|| sc_service::Error::SelectChainRequired)?;

				let (grandpa_block_import, grandpa_link) =
					grandpa::block_import(client.clone(), &(client.clone() as Arc<_>), select_chain)?;

                let justification_import = grandpa_block_import.clone();

				let (babe_block_import, babe_link) = babe::block_import(
                    babe::Config::get_or_compute(&*client)?,
                    grandpa_block_import,
                    client.clone(),
                )?;

				let import_queue = babe::import_queue(
					babe_link.clone(),
					babe_block_import.clone(),
					Some(Box::new(justification_import)),
					None,
					client,
					inherent_data_providers.clone(),
					spawn_task_handle,
					registry
				)?;

				import_setup = Some((babe_block_import, grandpa_link, babe_link));
				Ok(import_queue)
			})?;

        (builder, import_setup, inherent_data_providers)
    }};
}

/// Builds a new service for a full client.
pub fn new_full(config: Configuration)
    -> Result<impl AbstractService, ServiceError>
{
    use sc_network::Event;
    use sc_client_api::ExecutorProvider;
    use futures::stream::StreamExt;
    use sp_core::traits::BareCryptoStorePtr;

    let role = config.role.clone();
    let force_authoring = config.force_authoring;
    let name = config.network.node_name.clone();
    let disable_grandpa = config.disable_grandpa;

    let (builder, mut import_setup, inherent_data_providers) = new_full_start!(config);

    let service = builder
        .with_finality_proof_provider(|client, backend| {
            // GenesisAuthoritySetProvider is implemented for StorageAndProofProvider
            let provider = client as Arc<dyn StorageAndProofProvider<_, _>>;
            Ok(Arc::new(GrandpaFinalityProofProvider::new(backend, provider)) as _)
        })?
        .build_full()?;

    let (block_import, grandpa_link, babe_link) = import_setup.take()
        .expect("Link Half and Block Import are present for Full Services or setup failed before. qed");

    if role.is_authority() {
        let proposer =
            sc_basic_authorship::ProposerFactory::new(
                service.client(),
                service.transaction_pool(),
                service.prometheus_registry().as_ref()
            );

        let client = service.client();
        let select_chain = service.select_chain()
            .ok_or(ServiceError::SelectChainRequired)?;

        let can_author_with =
            sp_consensus::CanAuthorWithNativeVersion::new(client.executor().clone());

        let babe_config = babe::BabeParams {
            keystore: service.keystore(),
            client,
            select_chain,
            env: proposer,
            block_import,
            sync_oracle: service.network(),
            inherent_data_providers: inherent_data_providers.clone(),
            force_authoring,
            babe_link,
            can_author_with,
        };

        let babe = babe::start_babe(babe_config)?;

        // the BABE authoring task is considered essential, i.e. if it
        // fails we take down the service with it.
        service.spawn_essential_task_handle().spawn_blocking("babe", babe);

        // Authority discovery: this module runs to promise authorities' connection
        // TODO: [Substrate]refine sentry mode using updated substrate code
        if matches!(role, Role::Authority{..} | Role::Sentry{..}) {
            let (sentries, authority_discovery_role) = match role {
                Role::Authority { ref sentry_nodes } => (
                    sentry_nodes.clone(),
                    authority_discovery::Role::Authority (
                        service.keystore(),
                    ),
                ),
                Role::Sentry {..} => (
                    vec![],
                    authority_discovery::Role::Sentry,
                ),
                _ => unreachable!("Due to outer matches! constraint; qed."),
            };
            let network = service.network();
            let network_event_stream = network.event_stream("authority-discovery");
            let dht_event_stream = network_event_stream.filter_map(|e| async move {
                match e {
                    Event::Dht(e) => Some(e),
                    _ => None,
                }
            }).boxed();
            let authority_discovery = authority_discovery::AuthorityDiscovery::new(
                service.client(),
                network,
                sentries,
                dht_event_stream,
                authority_discovery_role,
                service.prometheus_registry(),
            );
            service.spawn_task_handle().spawn("authority-discovery", authority_discovery);
        }
    }

    // if the node isn't actively participating in consensus then it doesn't
    // need a keystore, regardless of which protocol we use below.
    let keystore = if role.is_authority() {
        Some(service.keystore() as BareCryptoStorePtr)
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

    let enable_grandpa = !disable_grandpa;
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
            network: service.network(),
            inherent_data_providers: inherent_data_providers.clone(),
            telemetry_on_connect: Some(service.telemetry_on_connect_stream()),
            voting_rule: grandpa::VotingRulesBuilder::default().build(),
            prometheus_registry: service.prometheus_registry(),
            shared_voter_state: SharedVoterState::empty()
        };

        // the GRANDPA voter task is considered infallible, i.e.
        // if it fails we take down the service with it.
        service.spawn_essential_task_handle().spawn_blocking(
            "grandpa-voter",
            grandpa::run_grandpa_voter(grandpa_config)?
        );
    } else {
        grandpa::setup_disabled_grandpa(
            service.client(),
            &inherent_data_providers,
            service.network(),
        )?;
    }

    Ok(service)
}

/// Builds a new service for a light client.
pub fn new_light(config: Configuration)
                 -> Result<impl AbstractService, ServiceError>
{
    let inherent_data_providers = InherentDataProviders::new();

    ServiceBuilder::new_light::<Block, RuntimeApi, Executor>(config)?
        .with_select_chain(|_config, backend| {
            Ok(LongestChain::new(backend.clone()))
        })?
        .with_transaction_pool(|builder| {
            let fetcher = builder.fetcher()
                .ok_or_else(|| "Trying to start light transaction pool without active fetcher")?;

            let pool_api = sc_transaction_pool::LightChainApi::new(
                builder.client().clone(),
                fetcher.clone(),
            );
            let pool = sc_transaction_pool::BasicPool::with_revalidation_type(
                builder.config().transaction_pool.clone(),
                Arc::new(pool_api),
                builder.prometheus_registry(),
                sc_transaction_pool::RevalidationType::Light,
            );
            Ok(pool)
        })?
        .with_import_queue_and_fprb(|
            _config,
            client,
            backend,
            fetcher,
            _select_chain,
            _tx_pool,
            spawn_task_handle,
            prometheus_registry
        | {
            let fetch_checker = fetcher
                .map(|fetcher| fetcher.checker().clone())
                .ok_or_else(|| "Trying to start light import queue without active fetch checker")?;
            let grandpa_block_import = grandpa::light_block_import(
                client.clone(),
                backend,
                &(client.clone() as Arc<_>),
                Arc::new(fetch_checker),
            )?;
            let finality_proof_import = grandpa_block_import.clone();
            let finality_proof_request_builder =
                finality_proof_import.create_finality_proof_request_builder();

            let (babe_block_import, babe_link) = babe::block_import(
                babe::Config::get_or_compute(&*client)?,
                grandpa_block_import,
                client.clone(),
            )?;

            // FIXME: [Substrate]pruning task isn't started since light client doesn't do `AuthoritySetup`.
            // (keep eyes on polkadot service)
            let import_queue = babe::import_queue(
                babe_link,
                babe_block_import,
                None,
                Some(Box::new(finality_proof_import)),
                client,
                inherent_data_providers.clone(),
                spawn_task_handle,
                prometheus_registry,
            )?;

            Ok((import_queue, finality_proof_request_builder))
        })?
        .with_finality_proof_provider(|client, backend| {
            // GenesisAuthoritySetProvider is implemented for StorageAndProofProvider
            let provider = client as Arc<dyn StorageAndProofProvider<_, _>>;
            Ok(Arc::new(GrandpaFinalityProofProvider::new(backend, provider)) as _)
        })?
        .build_light()
}
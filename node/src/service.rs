//! Service and ServiceFactory implementation. Specialized wrapper over substrate service.

use crust_runtime::{self, opaque::Block, RuntimeApi};
use grandpa::{self, FinalityProofProvider as GrandpaFinalityProofProvider, StorageAndProofProvider};
use sc_client::LongestChain;
use sc_executor::native_executor_instance;
pub use sc_executor::NativeExecutor;
use sc_service::{error::Error as ServiceError, AbstractService, Configuration, ServiceBuilder};
use sp_inherents::InherentDataProviders;
use std::sync::Arc;
use std::time::Duration;

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
		let mut import_setup = None;
		let inherent_data_providers = sp_inherents::InherentDataProviders::new();

		let builder = sc_service::ServiceBuilder::new_full::<
			crust_runtime::opaque::Block, crust_runtime::RuntimeApi, crate::service::Executor
		>($config)?
			.with_select_chain(|_config, backend| {
				Ok(sc_client::LongestChain::new(backend.clone()))
			})?
			.with_transaction_pool(|config, client, _fetcher| {
				let pool_api = sc_transaction_pool::FullChainApi::new(client.clone());
				Ok(sc_transaction_pool::BasicPool::new(config, std::sync::Arc::new(pool_api)))
			})?
			.with_import_queue(|_config, client, mut select_chain, _transaction_pool| {
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

    let is_authority = config.roles.is_authority();
    let force_authoring = config.force_authoring;
    let name = config.name.clone();
    let disable_grandpa = config.disable_grandpa;
    let sentry_nodes = config.network.sentry_nodes.clone();

    // sentry nodes announce themselves as authorities to the network
	// and should run the same protocols authorities do, but it should
	// never actively participate in any consensus process.
	let participates_in_consensus = is_authority && !config.sentry_mode;

    let (builder, mut import_setup, inherent_data_providers) = new_full_start!(config);

    let (block_import, grandpa_link, babe_link) = import_setup.take()
            .expect("Link Half and Block Import are present for Full Services or setup failed before. qed");

    let service = builder
        .with_finality_proof_provider(|client, backend| {
            // GenesisAuthoritySetProvider is implemented for StorageAndProofProvider
            let provider = client as Arc<dyn StorageAndProofProvider<_, _>>;
            Ok(Arc::new(GrandpaFinalityProofProvider::new(backend, provider)) as _)
        })?
        .build()?;

    if participates_in_consensus {
        let proposer =
            sc_basic_authorship::ProposerFactory::new(service.client(), service.transaction_pool());

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
        service.spawn_essential_task("babe", babe);

        // Authority discovery: this module runs to promise authorities' connection
        // TODO: 2 modes? enable or not?
        let network = service.network();
        let network_event_stream = network.event_stream();
        let dht_event_stream = network_event_stream.filter_map(|e| async move { match e {
            Event::Dht(e) => Some(e),
            _ => None,
        }}).boxed();
        let authority_discovery = authority_discovery::AuthorityDiscovery::new(
            service.client(),
            network,
            sentry_nodes.clone(),
            service.keystore(),
            dht_event_stream,
            service.prometheus_registry(),
        );
        service.spawn_task("authority-discovery", authority_discovery);
    }

    // if the node isn't actively participating in consensus then it doesn't
    // need a keystore, regardless of which protocol we use below.
    let keystore = if participates_in_consensus {
        Some(service.keystore())
    } else {
        None
    };

    let grandpa_config = grandpa::Config {
        // FIXME substrate/issues#1578 make this available through chainspec
        gossip_duration: Duration::from_millis(333),
        justification_period: 512,
        name: Some(name),
        observer_enabled: false,
        keystore,
        is_authority,
    };

    let enable_grandpa = !disable_grandpa;
    if enable_grandpa {
        // start the full GRANDPA voter
        // NOTE: non-authorities could run the GRANDPA observer protocol, but at
        // this point the full voter should provide better guarantees of block
        // and vote data availability than the observer. The observer has not
        // been tested extensively yet and having most nodes in a network run it
        // could lead to finality stalls.
        // TODO: Aiming the stalls problem, maybe we could refer polkadot solution
        // in (https://github.com/paritytech/polkadot/blob/master/service/src/lib.rs#L507-L521)
        let grandpa_config = grandpa::GrandpaParams {
            config: grandpa_config,
            link: grandpa_link,
            network: service.network(),
            inherent_data_providers: inherent_data_providers.clone(),
            telemetry_on_connect: Some(service.telemetry_on_connect_stream()),
            voting_rule: grandpa::VotingRulesBuilder::default().build(),
            prometheus_registry: service.prometheus_registry()
        };

        // the GRANDPA voter task is considered infallible, i.e.
        // if it fails we take down the service with it.
        service.spawn_essential_task(
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
        .with_transaction_pool(|config, client, fetcher| {
            let fetcher = fetcher
                .ok_or_else(|| "Trying to start light transaction pool without active fetcher")?;

            let pool_api = sc_transaction_pool::LightChainApi::new(client.clone(), fetcher.clone());
            let pool = sc_transaction_pool::BasicPool::with_revalidation_type(
                config, Arc::new(pool_api), sc_transaction_pool::RevalidationType::Light,
            );
            Ok(pool)
        })?
        .with_import_queue_and_fprb(|_config, client, backend, fetcher, _select_chain, _tx_pool| {
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

            // FIXME: pruning task isn't started since light client doesn't do `AuthoritySetup`.
            let import_queue = babe::import_queue(
                babe_link,
                babe_block_import,
                None,
                Some(Box::new(finality_proof_import)),
                client,
                inherent_data_providers.clone(),
            )?;

            Ok((import_queue, finality_proof_request_builder))
        })?
        .with_finality_proof_provider(|client, backend| {
            // GenesisAuthoritySetProvider is implemented for StorageAndProofProvider
            let provider = client as Arc<dyn StorageAndProofProvider<_, _>>;
            Ok(Arc::new(GrandpaFinalityProofProvider::new(backend, provider)) as _)
        })?
        .build()
}
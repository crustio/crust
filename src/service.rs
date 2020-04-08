//! Service and ServiceFactory implementation. Specialized wrapper over substrate service.

use crust_runtime::{self, opaque::Block, GenesisConfig, RuntimeApi};
use grandpa::{self, FinalityProofProvider as GrandpaFinalityProofProvider};
use sc_basic_authority;
use sc_client::LongestChain;
use sc_executor::native_executor_instance;
pub use sc_executor::NativeExecutor;
use sc_network::construct_simple_protocol;
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

construct_simple_protocol! {
    /// Demo protocol attachment for substrate.
    pub struct NodeProtocol where Block = Block { }
}

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
			node_template_runtime::opaque::Block, node_template_runtime::RuntimeApi, crate::service::Executor
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
					.ok_or_else(|| service::Error::SelectChainRequired)?;

				let grandpa_hard_forks = if config.chain_spec.is_kusama() {
					grandpa_support::kusama_hard_forks()
				} else {
					Vec::new()
				};

				let (grandpa_block_import, grandpa_link) =
					grandpa::block_import_with_authority_set_hard_forks(
						client.clone(),
						&(client.clone() as Arc<_>),
						select_chain,
						grandpa_hard_forks,
					)?;

				let justification_import = grandpa_block_import.clone();

				let (block_import, babe_link) = babe::block_import(
					babe::Config::get_or_compute(&*client)?,
					grandpa_block_import,
					client.clone(),
				)?;

				let import_queue = babe::import_queue(
					babe_link.clone(),
					block_import.clone(),
					Some(Box::new(justification_import)),
					None,
					client,
					inherent_data_providers.clone(),
				)?;

				import_setup = Some((block_import, grandpa_link, babe_link));
				Ok(import_queue)
			})?
			.with_rpc_extensions(|builder| -> Result<polkadot_rpc::RpcExtension, _> {
				Ok(polkadot_rpc::create_full(builder.client().clone(), builder.pool()))
			})?;

        (builder, import_setup, inherent_data_providers)
    }}
}

/// Builds a new service for a full client.
pub fn new_full(config: Configuration)
                -> Result<impl AbstractService, ServiceError>
{
    let role = config.role.clone();
    let force_authoring = config.force_authoring;
    let name = config.name.clone();
    let disable_grandpa = config.disable_grandpa;

    let (builder, mut import_setup, inherent_data_providers) = new_full_start!(config);

    let (block_import, grandpa_link) =
        import_setup.take()
            .expect("Link Half and Block Import are present for Full Services or setup failed before. qed");

    let service = builder
        .with_finality_proof_provider(|client, backend| {
            // GenesisAuthoritySetProvider is implemented for StorageAndProofProvider
            let provider = client as Arc<dyn StorageAndProofProvider<_, _>>;
            Ok(Arc::new(GrandpaFinalityProofProvider::new(backend, provider)) as _)
        })?
        .build()?;

    if role.is_authority() {
        let proposer =
            sc_basic_authorship::ProposerFactory::new(service.client(), service.transaction_pool());

        let client = service.client();
        let select_chain = service.select_chain()
            .ok_or(ServiceError::SelectChainRequired)?;

        let can_author_with =
            sp_consensus::CanAuthorWithNativeVersion::new(client.executor().clone());

        let aura = sc_consensus_aura::start_aura::<_, _, _, _, _, AuraPair, _, _, _>(
            sc_consensus_aura::slot_duration(&*client)?,
            client,
            select_chain,
            block_import,
            proposer,
            service.network(),
            inherent_data_providers.clone(),
            force_authoring,
            service.keystore(),
            can_author_with,
        )?;

        // the AURA authoring task is considered essential, i.e. if it
        // fails we take down the service with it.
        service.spawn_essential_task("aura", aura);
    }

    // if the node isn't actively participating in consensus then it doesn't
    // need a keystore, regardless of which protocol we use below.
    let keystore = if role.is_authority() {
        Some(service.keystore())
    } else {
        None
    };

    let grandpa_config = sc_finality_grandpa::Config {
        // FIXME #1578 make this available through chainspec
        gossip_duration: Duration::from_millis(333),
        justification_period: 512,
        name: Some(name),
        observer_enabled: false,
        keystore,
        is_authority: role.is_network_authority(),
    };

    let enable_grandpa = !disable_grandpa;
    if enable_grandpa {
        // start the full GRANDPA voter
        // NOTE: non-authorities could run the GRANDPA observer protocol, but at
        // this point the full voter should provide better guarantees of block
        // and vote data availability than the observer. The observer has not
        // been tested extensively yet and having most nodes in a network run it
        // could lead to finality stalls.
        let grandpa_config = sc_finality_grandpa::GrandpaParams {
            config: grandpa_config,
            link: grandpa_link,
            network: service.network(),
            inherent_data_providers: inherent_data_providers.clone(),
            telemetry_on_connect: Some(service.telemetry_on_connect_stream()),
            voting_rule: sc_finality_grandpa::VotingRulesBuilder::default().build(),
            prometheus_registry: service.prometheus_registry()
        };

        // the GRANDPA voter task is considered infallible, i.e.
        // if it fails we take down the service with it.
        service.spawn_essential_task(
            "grandpa-voter",
            sc_finality_grandpa::run_grandpa_voter(grandpa_config)?
        );
    } else {
        sc_finality_grandpa::setup_disabled_grandpa(
            service.client(),
            &inherent_data_providers,
            service.network(),
        )?;
    }

    Ok(service)
}

/// Builds a new service for a light client.
pub fn new_light<C: Send + Default + 'static>(
    config: Configuration<C, GenesisConfig>,
) -> Result<impl AbstractService, ServiceError> {
    let inherent_data_providers = InherentDataProviders::new();

    ServiceBuilder::new_light::<Block, RuntimeApi, Executor>(config)?
        .with_select_chain(|_config, backend| Ok(LongestChain::new(backend.clone())))?
        .with_transaction_pool(|config, client, fetcher| {
            let fetcher = fetcher
                .ok_or_else(|| "Trying to start light transaction pool without active fetcher")?;
            let pool_api = sc_transaction_pool::LightChainApi::new(client.clone(), fetcher.clone());
            let pool = sc_transaction_pool::BasicPool::new(config, pool_api);
            let maintainer = sc_transaction_pool::LightBasicPoolMaintainer::with_defaults(
                pool.pool().clone(),
                client,
                fetcher,
            );
            let maintainable_pool =
                sp_transaction_pool::MaintainableTransactionPool::new(pool, maintainer);
            Ok(maintainable_pool)
        })?
        .with_import_queue_and_fprb(
            |_config, client, backend, fetcher, _select_chain, _tx_pool| {
                let fetch_checker = fetcher
                    .map(|fetcher| fetcher.checker().clone())
                    .ok_or_else(|| {
                        "Trying to start light import queue without active fetch checker"
                    })?;
                let grandpa_block_import = grandpa::light_block_import::<_, _, _, RuntimeApi>(
                    client.clone(),
                    backend,
                    &*client.clone(),
                    Arc::new(fetch_checker),
                )?;
                let finality_proof_import = grandpa_block_import.clone();
                let finality_proof_request_builder =
                    finality_proof_import.create_finality_proof_request_builder();

                let (babe_block_import, babe_link) = sc_consensus_babe::block_import(
                    sc_consensus_babe::Config::get_or_compute(&*client)?,
                    grandpa_block_import,
                    client.clone(),
                    client.clone(),
                )?;

                // FIXME: pruning task isn't started since light client doesn't do `AuthoritySetup`.
                let import_queue = sc_consensus_babe::import_queue(
                    babe_link,
                    babe_block_import,
                    None,
                    Some(Box::new(finality_proof_import)),
                    client.clone(),
                    client,
                    inherent_data_providers.clone(),
                )?;

                Ok((import_queue, finality_proof_request_builder))
            },
        )?
        .with_network_protocol(|_| Ok(NodeProtocol::new()))?
        .with_finality_proof_provider(|client, backend| {
            Ok(Arc::new(GrandpaFinalityProofProvider::new(backend, client)) as _)
        })?
        .build()
}

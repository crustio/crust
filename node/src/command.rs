// Copyright 2019-2020 Crustio Technologies Ltd.

// You should have received a copy of the GNU General Public License
// along with Substrate.  If not, see <http://www.gnu.org/licenses/>.

use crate::chain_spec;
use codec::Encode;
use crate::cli::{Cli, Subcommand, RelayChainCli};
use crate::service as crust_service;
use crate::executor::Executor;
use crust_service::new_partial;
use sc_service::PartialComponents;
use sc_cli::{Result, SubstrateCli, RuntimeVersion, ChainSpec};
use crust_runtime::{Block, RuntimeApi};
use cumulus_primitives::{genesis::generate_genesis_block, ParaId};
use polkadot_parachain::primitives::AccountIdConversion;
use sc_cli::{
    CliConfiguration, DefaultConfigurationValues, ImportParams, KeystoreParams,
    NetworkParams, SharedParams,
};
use sc_service::config::{BasePath, PrometheusConfig};
use sp_core::hexdisplay::HexDisplay;
use std::{io::Write, net::SocketAddr};
use log::info;
use sp_runtime::traits::Block as BlockT;


fn load_spec(
    id: &str,
    para_id: ParaId,
) -> std::result::Result<Box<dyn sc_service::ChainSpec>, String> {
    Ok(match id {
        "rocky" => Box::new(chain_spec::rocky_config()?),
        "maxwell" => Box::new(chain_spec::maxwell_config()?),
        "maxwell-rococo" => Box::new(chain_spec::maxwell_rococo_config()?),
        "rocky-staging" => Box::new(chain_spec::rocky_staging_config(para_id)?),
        "maxwell-staging" => Box::new(chain_spec::maxwell_staging_config(para_id)?),
        "dev" => Box::new(chain_spec::development_config(para_id)?),
        "" | "local" => Box::new(chain_spec::local_testnet_config(para_id)?),
        path => Box::new(chain_spec::CrustChainSpec::from_json_file(
            std::path::PathBuf::from(path),
        )?),
    })
}

impl SubstrateCli for Cli {
    fn impl_name() -> String { "Crust Collator".into() }

    fn impl_version() -> String { env!("SUBSTRATE_CLI_IMPL_VERSION").into() }

    fn executable_name() -> String { "crust".into() }

    fn description() -> String { env!("CARGO_PKG_DESCRIPTION").into() }

    fn author() -> String { env!("CARGO_PKG_AUTHORS").into() }

    fn support_url() -> String { "https://github.com/crustio/crust/issues/new".into() }

    fn copyright_start_year() -> i32 { 2019 }

    fn load_spec(&self, id: &str) -> std::result::Result<Box<dyn sc_service::ChainSpec>, String> {
        load_spec(id, self.run.parachain_id.unwrap_or(6666).into())
    }

    fn native_runtime_version(_chain_spec: &Box<dyn ChainSpec>) -> &'static RuntimeVersion {
        &crust_runtime::VERSION
    }
}

impl SubstrateCli for RelayChainCli {
    fn impl_name() -> String {
        "Crust Collator".into()
    }

    fn impl_version() -> String {
        env!("SUBSTRATE_CLI_IMPL_VERSION").into()
    }

    fn description() -> String {
        env!("CARGO_PKG_DESCRIPTION").into()
    }

    fn author() -> String {
        env!("CARGO_PKG_AUTHORS").into()
    }

    fn support_url() -> String {
        "https://github.com/paritytech/cumulus/issues/new".into()
    }

    fn copyright_start_year() -> i32 {
        2019
    }

    fn load_spec(&self, id: &str) -> std::result::Result<Box<dyn sc_service::ChainSpec>, String> {
        polkadot_cli::Cli::from_iter([RelayChainCli::executable_name().to_string()].iter())
            .load_spec(id)
    }

    fn native_runtime_version(chain_spec: &Box<dyn ChainSpec>) -> &'static RuntimeVersion {
        polkadot_cli::Cli::native_runtime_version(chain_spec)
    }
}

fn extract_genesis_wasm(chain_spec: &Box<dyn sc_service::ChainSpec>) -> Result<Vec<u8>> {
    let mut storage = chain_spec.build_storage()?;

    storage
        .top
        .remove(sp_core::storage::well_known_keys::CODE)
        .ok_or_else(|| "Could not find wasm file in genesis state!".into())
}


/// Parse and run command line arguments
pub fn run() -> sc_cli::Result<()> {
    let cli = Cli::from_args();

    match &cli.subcommand {
        Some(Subcommand::BuildSpec(cmd)) => {
            let runner = cli.create_runner(cmd)?;
            runner.sync_run(|config| cmd.run(config.chain_spec, config.network))
        },
        Some(Subcommand::CheckBlock(cmd)) => {
            let runner = cli.create_runner(cmd)?;
            runner.async_run(|config| {
                let PartialComponents { client, task_manager, import_queue, .. }
                    = new_partial(&config)?;
                Ok((cmd.run(client, import_queue), task_manager))
            })
        },
        Some(Subcommand::ExportBlocks(cmd)) => {
            let runner = cli.create_runner(cmd)?;
            runner.async_run(|config| {
                let PartialComponents { client, task_manager, ..}
                    = new_partial(&config)?;
                Ok((cmd.run(client, config.database), task_manager))
            })
        },
        Some(Subcommand::ExportState(cmd)) => {
            let runner = cli.create_runner(cmd)?;
            runner.async_run(|config| {
                let PartialComponents { client, task_manager, ..}
                    = new_partial(&config)?;
                Ok((cmd.run(client, config.chain_spec), task_manager))
            })
        },
        Some(Subcommand::ImportBlocks(cmd)) => {
            let runner = cli.create_runner(cmd)?;
            runner.async_run(|config| {
                let PartialComponents { client, task_manager, import_queue, ..}
                    = new_partial(&config)?;
                Ok((cmd.run(client, import_queue), task_manager))
            })
        },
        Some(Subcommand::PurgeChain(cmd)) => {
            let runner = cli.create_runner(cmd)?;
            runner.sync_run(|config| cmd.run(config.database))
        },
        Some(Subcommand::Revert(cmd)) => {
            let runner = cli.create_runner(cmd)?;
            runner.async_run(|config| {
                let PartialComponents { client, task_manager, backend, ..}
                    = new_partial(&config)?;
                Ok((cmd.run(client, backend), task_manager))
            })
        },
        Some(Subcommand::Benchmark(subcommand)) => {
            if cfg!(feature = "runtime-benchmarks") {
                let runner = cli.create_runner(subcommand)?;

                runner.sync_run(|config| subcommand.run::<Block, Executor>(config))
            } else {
                println!("Benchmarking wasn't enabled when building the node. \
                You can enable it with `--features runtime-benchmarks`.");
                Ok(())
            }
        },
        Some(Subcommand::Inspect(cmd)) => {
            let runner = cli.create_runner(cmd)?;

            runner.sync_run(|config| cmd.run::<Block, RuntimeApi, Executor>(config))
        },
        Some(Subcommand::Base(cmd)) => cmd.run(),
        Some(Subcommand::Sign(cmd)) => cmd.run(),
        Some(Subcommand::Verify(cmd)) => cmd.run(),
        Some(Subcommand::Vanity(cmd)) => cmd.run(),
        Some(Subcommand::ExportGenesisState(params)) => {
            sc_cli::init_logger("", sc_tracing::TracingReceiver::Log, None, false)?;

            let block: Block = generate_genesis_block(&load_spec(
                &params.chain.clone().unwrap_or_default(),
                params.parachain_id.into(),
            )?)?;
            let raw_header = block.header().encode();
            let output_buf = if params.raw {
                raw_header
            } else {
                format!("0x{:?}", HexDisplay::from(&block.header().encode())).into_bytes()
            };

            if let Some(output) = &params.output {
                std::fs::write(output, output_buf)?;
            } else {
                std::io::stdout().write_all(&output_buf)?;
            }

            Ok(())
        },
        Some(Subcommand::ExportGenesisWasm(params)) => {
            sc_cli::init_logger("", sc_tracing::TracingReceiver::Log, None, false)?;

            let raw_wasm_blob =
                extract_genesis_wasm(&cli.load_spec(&params.chain.clone().unwrap_or_default())?)?;
            let output_buf = if params.raw {
                raw_wasm_blob
            } else {
                format!("0x{:?}", HexDisplay::from(&raw_wasm_blob)).into_bytes()
            };

            if let Some(output) = &params.output {
                std::fs::write(output, output_buf)?;
            } else {
                std::io::stdout().write_all(&output_buf)?;
            }

            Ok(())
        },
        None => {
            let runner = cli.create_runner(&*cli.run)?;
            runner.run_node_until_exit(|config| async move {
                let key = sp_core::Pair::generate().0;

                let extension = chain_spec::Extensions::try_get(&config.chain_spec);
                let relay_chain_id = extension.map(|e| e.relay_chain.clone());
                let para_id = extension.map(|e| e.para_id);

                let polkadot_cli = RelayChainCli::new(
                    config.base_path.as_ref().map(|x| x.path().join("polkadot")),
                    relay_chain_id,
                    [RelayChainCli::executable_name().to_string()]
                        .iter()
                        .chain(cli.relaychain_args.iter()),
                );

                let id = ParaId::from(cli.run.parachain_id.or(para_id).unwrap_or(6666));

                let parachain_account =
                    AccountIdConversion::<polkadot_primitives::v0::AccountId>::into_account(&id);

                let block: Block =
                    generate_genesis_block(&config.chain_spec).map_err(|e| format!("{:?}", e))?;
                let genesis_state = format!("0x{:?}", HexDisplay::from(&block.header().encode()));

                let task_executor = config.task_executor.clone();
                let polkadot_config =
                    SubstrateCli::create_configuration(&polkadot_cli, &polkadot_cli, task_executor)
                        .map_err(|err| format!("Relay chain argument error: {}", err))?;
                let collator = cli.run.base.validator || cli.collator;

                info!("Parachain id: {:?}", id);
                info!("Parachain Account: {}", parachain_account);
                info!("Parachain genesis state: {}", genesis_state);
                info!("Is collating: {}", if collator { "yes" } else { "no" });

                crate::service::start_node(config, key, polkadot_config, id, collator)
                    .await
                    .map(|r| r.0)
            })
        }
    }
}

impl DefaultConfigurationValues for RelayChainCli {
    fn p2p_listen_port() -> u16 {
        30334
    }

    fn rpc_ws_listen_port() -> u16 {
        9945
    }

    fn rpc_http_listen_port() -> u16 {
        9934
    }

    fn prometheus_listen_port() -> u16 {
        9616
    }
}

impl CliConfiguration<Self> for RelayChainCli {
    fn shared_params(&self) -> &SharedParams {
        self.base.base.shared_params()
    }

    fn import_params(&self) -> Option<&ImportParams> {
        self.base.base.import_params()
    }

    fn network_params(&self) -> Option<&NetworkParams> {
        self.base.base.network_params()
    }

    fn keystore_params(&self) -> Option<&KeystoreParams> {
        self.base.base.keystore_params()
    }

    fn base_path(&self) -> Result<Option<BasePath>> {
        Ok(self
            .shared_params()
            .base_path()
            .or_else(|| self.base_path.clone().map(Into::into)))
    }

    fn rpc_http(&self, default_listen_port: u16) -> Result<Option<SocketAddr>> {
        self.base.base.rpc_http(default_listen_port)
    }

    fn rpc_ipc(&self) -> Result<Option<String>> {
        self.base.base.rpc_ipc()
    }

    fn rpc_ws(&self, default_listen_port: u16) -> Result<Option<SocketAddr>> {
        self.base.base.rpc_ws(default_listen_port)
    }

    fn prometheus_config(&self, default_listen_port: u16) -> Result<Option<PrometheusConfig>> {
        self.base.base.prometheus_config(default_listen_port)
    }

    fn init<C: SubstrateCli>(&self) -> Result<()> {
        unreachable!("PolkadotCli is never initialized; qed");
    }

    fn chain_id(&self, is_dev: bool) -> Result<String> {
        let chain_id = self.base.base.chain_id(is_dev)?;

        Ok(if chain_id.is_empty() {
            self.chain_id.clone().unwrap_or_default()
        } else {
            chain_id
        })
    }

    fn role(&self, is_dev: bool) -> Result<sc_service::Role> {
        self.base.base.role(is_dev)
    }

    fn transaction_pool(&self) -> Result<sc_service::config::TransactionPoolOptions> {
        self.base.base.transaction_pool()
    }

    fn state_cache_child_ratio(&self) -> Result<Option<usize>> {
        self.base.base.state_cache_child_ratio()
    }

    fn rpc_methods(&self) -> Result<sc_service::config::RpcMethods> {
        self.base.base.rpc_methods()
    }

    fn rpc_ws_max_connections(&self) -> Result<Option<usize>> {
        self.base.base.rpc_ws_max_connections()
    }

    fn rpc_cors(&self, is_dev: bool) -> Result<Option<Vec<String>>> {
        self.base.base.rpc_cors(is_dev)
    }

    fn telemetry_external_transport(&self) -> Result<Option<sc_service::config::ExtTransport>> {
        self.base.base.telemetry_external_transport()
    }

    fn default_heap_pages(&self) -> Result<Option<u64>> {
        self.base.base.default_heap_pages()
    }

    fn force_authoring(&self) -> Result<bool> {
        self.base.base.force_authoring()
    }

    fn disable_grandpa(&self) -> Result<bool> {
        self.base.base.disable_grandpa()
    }

    fn max_runtime_instances(&self) -> Result<Option<usize>> {
        self.base.base.max_runtime_instances()
    }

    fn announce_block(&self) -> Result<bool> {
        self.base.base.announce_block()
    }
}

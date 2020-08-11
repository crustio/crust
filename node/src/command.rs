// Copyright 2019-2020 Crustio Technologies Ltd.

// You should have received a copy of the GNU General Public License
// along with Substrate.  If not, see <http://www.gnu.org/licenses/>.

use crate::chain_spec;
use crate::cli::Cli;
use crate::service as crust_service;
use crate::service::new_full_params;
use service::ServiceParams;
use sc_cli::{SubstrateCli, RuntimeVersion, Role, ChainSpec};

impl SubstrateCli for Cli {
	fn impl_name() -> String { "Crust Node".into() }

	fn impl_version() -> String { env!("SUBSTRATE_CLI_IMPL_VERSION").into() }

	fn executable_name() -> String { "crust".into() }

	fn description() -> String { env!("CARGO_PKG_DESCRIPTION").into() }

	fn author() -> String { env!("CARGO_PKG_AUTHORS").into() }

	fn support_url() -> String { "https://github.com/crustio/crust/issues/new".into() }

	fn copyright_start_year() -> i32 { 2019 }

	fn load_spec(&self, id: &str) -> Result<Box<dyn service::ChainSpec>, String> {
		Ok(match id {
			"rocky" => Box::new(chain_spec::rocky_config()?),
			"maxwell" => Box::new(chain_spec::maxwell_config()?),
			"rocky-staging" => Box::new(chain_spec::rocky_staging_config()?),
			"maxwell-staging" => Box::new(chain_spec::maxwell_staging_config()?),
			"dev" => Box::new(chain_spec::development_config()?),
			"" | "local" => Box::new(chain_spec::local_testnet_config()?),
			path => Box::new(chain_spec::CrustChainSpec::from_json_file(
				std::path::PathBuf::from(path),
			)?),
		})
	}

	fn native_runtime_version(_chain_spec: &Box<dyn ChainSpec>) -> &'static RuntimeVersion {
		&crust_runtime::VERSION
	}
}

/// Parse and run command line arguments
pub fn run() -> sc_cli::Result<()> {
	let cli = Cli::from_args();

	match &cli.subcommand {
		Some(subcommand) => {
			let runner = cli.create_runner(subcommand)?;
			runner.run_subcommand(subcommand, |config| {
				let (ServiceParams { client, backend, task_manager, import_queue, .. }, ..)
					= new_full_params(config)?;
				Ok((client, backend, import_queue, task_manager))
			})
		}
		None => {
			let runner = cli.create_runner(&cli.run)?;
			runner.run_node_until_exit( |config| match config.role {
				Role::Light => crust_service::new_light(config),
				_ => crust_service::new_full(config),
			})
		}
	}
}
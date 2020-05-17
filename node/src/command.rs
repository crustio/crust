// Copyright 2019-2020 Crustio Technologies Ltd.

// You should have received a copy of the GNU General Public License
// along with Substrate.  If not, see <http://www.gnu.org/licenses/>.

use crate::chain_spec;
use crate::cli::{Cli, Subcommand};
use crate::service;
use sc_cli::SubstrateCli;
use crust_runtime::opaque::Block;

impl SubstrateCli for Cli {
	fn impl_name() -> &'static str { "Crust Node" }

	fn impl_version() -> &'static str { env!("SUBSTRATE_CLI_IMPL_VERSION") }

	fn executable_name() -> &'static str { "crust" }

	fn description() -> &'static str { env!("CARGO_PKG_DESCRIPTION") }

	fn author() -> &'static str { env!("CARGO_PKG_AUTHORS") }

	fn support_url() -> &'static str { "https://github.com/crustio/crust/issues/new" }

	fn copyright_start_year() -> i32 { 2019 }

	fn load_spec(&self, id: &str) -> Result<Box<dyn sc_service::ChainSpec>, String> {
		Ok(match id {
			"dev" => Box::new(chain_spec::development_config()),
			"" | "local" => Box::new(chain_spec::local_testnet_config()),
			path => Box::new(chain_spec::ChainSpec::from_json_file(
				std::path::PathBuf::from(path),
			)?),
		})
	}
}

/// Parse and run command line arguments
pub fn run() -> sc_cli::Result<()> {
	let cli = Cli::from_args();

	match &cli.subcommand {
		Some(Subcommand::Benchmark(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			runner.sync_run(|config| cmd.run::<Block, service::Executor>(config))
		}
		Some(Subcommand::Base(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			runner.run_subcommand(cmd, |config| Ok(new_full_start!(config).0))
		}
		None => {
			let runner = cli.create_runner(&cli.run)?;
			runner.run_node(
				service::new_light,
				service::new_full,
				crust_runtime::VERSION
			)
		}
	}
}
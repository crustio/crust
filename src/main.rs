//! Substrate Node CLI library.

#![warn(missing_docs)]
#![warn(unused_extern_crates)]

mod chain_spec;
#[macro_use]
mod service;
mod cli;

pub use sc_cli::{VersionInfo, IntoExit, error};

fn main() -> Result<(), cli::error::Error> {
	let version = VersionInfo {
		name: "Crust ALPHA Node",
		commit: env!("VERGEN_SHA_SHORT"),
		version: env!("CARGO_PKG_VERSION"),
		executable_name: "crust",
		author: "crustio",
		description: "crust alpha testnet",
		support_url: "crustcloud.io",
	};

	cli::run(std::env::args(), cli::Exit, version)
}

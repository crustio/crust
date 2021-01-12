// Copyright (C) 2019-2021 Crust Network Technologies Ltd.
// This file is part of Crust.
#![warn(missing_docs)]

mod chain_spec;
#[macro_use]
mod service;
mod cli;
mod command;
mod executor;

fn main() -> sc_cli::Result<()> {
    command::run()
}

# Crust &middot; [![Build Status](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Factions-badge.atrox.dev%2Fcrustio%2Fcrust%2Fbadge&style=flat)](https://github.com/crustio/crust/actions?query=workflow%3A%22Crust+CI%22) [![Substrate version](https://img.shields.io/badge/Substrate-3.0.0-blue?logo=Parity%20Substrate)](https://substrate.dev/) [![GitHub license](https://img.shields.io/github/license/crustio/crust?logo=apache)](LICENSE)

<a href='https://web3.foundation/'><img width='160' alt='Funded by web3 foundation' src='docs/img/web3f_grants_badge.png'></a>&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;<a href='https://builders.parity.io/'><img width='180' src='docs/img/sbp_grants_badge.png'></a>

Implementation of a [Crust Protocol](https://crust.network) node with [substrate](https://github.com/paritytech/substrate).

## Join to play

Please go to [crust wiki](https://wiki.crust.network/docs/en/nodeOverview), refer the node overview.

## Building

### Build from source

#### 1. Install Rust

> If, after installation, running `rustc --version` in the console fails, refer to [it](https://www.rust-lang.org/tools/install) to repair.

```shell
curl https://sh.rustup.rs -sSf | sh
```

#### 2. Initialize your Wasm Build environment

```shell
./scripts/init.sh
```

#### 3. Build Wasm and native code

```bash
cargo build --release
```

### Build from docker

Refer [this](https://github.com/crustio/crust/tree/master/docker#dockerize-crust) to see how to build and run crust with docker.

## Network

### 1. Connect to maxwell

```shell
./target/release/crust --chain maxwell
```

Get the bootnodes from [here](https://raw.githubusercontent.com/crustio/crust/maxwell/node/res/maxwell.json).

### 2. Connect to rocky

> Archived chain for only **internal test** now.

```shell
./target/release/crust --chain rocky
```

### 3. Run as dev

Purge any existing developer chain state:

```bash
./target/release/crust purge-chain --dev
```

Start a development chain with:

```bash
./target/release/crust --dev
```

Detailed logs may be shown by running the node with the following environment variables set: `RUST_LOG=debug RUST_BACKTRACE=1 cargo run -- --dev`.

### 4. Run as local

If you want to see the multi-node consensus algorithm in action locally, then you can create a local testnet with two validator nodes for Alice and Bob, who are the initial authorities of the genesis chain that have been endowed with testnet units.

You'll need two terminal windows open.

We'll start Alice's substrate node first on default TCP port `30333` with her chain database stored locally at `/tmp/alice`. The bootnode ID of her node is `12D3KooWEyoppNCUx8Yx66oV9fJnriXwCcXwDDUA2kj6vnc6iDEp`, which is generated from the `--node-key` value that we specify below:

```bash
./target/release/crust \
  --base-path /tmp/alice \
  --chain local \
  --alice \
  --node-key 0000000000000000000000000000000000000000000000000000000000000001
```

In the second terminal, we'll start Bob's substrate node on a different TCP port of `30334`, and with his chain database stored locally at `/tmp/bob`. We'll specify a value for the `--bootnodes` option that will connect his node to Alice's bootnode ID on TCP port 30333:

```bash
./target/release/crust \
  --base-path /tmp/bob \
  --chain local \
  --bob \
  --port 30334 \
  --bootnodes /ip4/127.0.0.1/tcp/30333/p2p/12D3KooWEyoppNCUx8Yx66oV9fJnriXwCcXwDDUA2kj6vnc6iDEp
```

Additional CLI usage options are available and may be shown by running `cargo run -- --help`.

## Contribution

Please follow the contributions guidelines as outlined in [docs/CONTRIBUTION.md](https://github.com/crustio/crust/blob/master/docs/CONTRIBUTION.md). In all communications and contributions, this project follows the [Contributor Covenant Code of Conduct](https://github.com/paritytech/substrate/blob/master/docs/CODE_OF_CONDUCT.md).

## License

[Apache 2.0](https://github.com/crustio/crust/blob/master/LICENSE)

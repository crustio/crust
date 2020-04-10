# Crust

Implementation of a [Crust protocol](https://curst.network) node with [substrate](https://github.com/paritytech/substrate).

## Join to play

Go to our [Crust Client](https://github.com/crustio/crust-client), follow the README.

## Building

### 1. Install Rust

```shell
curl https://sh.rustup.rs -sSf | sh
```

### 2. Initialize your Wasm Build environment

```shell
./scripts/init.sh
```

### 3. Build Wasm and native code

```bash
cargo build --release
```

### 4. Single Node Development Chain

Purge any existing developer chain state:

```bash
./target/release/crust purge-chain --dev
```

Start a development chain with:

```bash
./target/release/crust --dev
```

Detailed logs may be shown by running the node with the following environment variables set: `RUST_LOG=debug RUST_BACKTRACE=1 cargo run -- --dev`.

### 5. Multi-Node Local Testnet

If you want to see the multi-node consensus algorithm in action locally, then you can create a local testnet with two validator nodes for Alice and Bob, who are the initial authorities of the genesis chain that have been endowed with testnet units.

You'll need two terminal windows open.

We'll start Alice's substrate node first on default TCP port 30333 with her chain database stored locally at `/tmp/alice`. The bootnode ID of her node is `QmRpheLN4JWdAnY7HGJfWFNbfkQCb6tFf4vvA6hgjMZKrR`, which is generated from the `--node-key` value that we specify below:

```bash
./target/release/crust \
  --base-path /tmp/alice \
  --chain local \
  --alice \
  --node-key 0000000000000000000000000000000000000000000000000000000000000001
```

In the second terminal, we'll start Bob's substrate node on a different TCP port of 30334, and with his chain database stored locally at `/tmp/bob`. We'll specify a value for the `--bootnodes` option that will connect his node to Alice's bootnode ID on TCP port 30333:

```bash
./target/release/crust \
  --base-path /tmp/bob \
  --chain local \
  --bob \
  --port 30334 \
  --bootnodes /ip4/127.0.0.1/tcp/30333/p2p/QmRpheLN4JWdAnY7HGJfWFNbfkQCb6tFf4vvA6hgjMZKrR
```

Additional CLI usage options are available and may be shown by running `cargo run -- --help`.

## Contribution

Thank you for considering to help out with the source code! We welcome contributions from anyone on the internet, and are grateful for even the smallest of fixes!

If you'd like to contribute to crust, please **fork, fix, commit and send a pull request for the maintainers to review and merge into the main codebase**.

### Rules

Please make sure your contribution adhere to our coding guideliness:

- **No --force pushes** or modifying the master branch history in any way. If you need to rebase, ensure you do it in your own repo.
- Code must adhere to the [house coding style](https://wiki.parity.io/Substrate-Style-Guide)
- Pull requests need to be based on and opened against the `master branch`.
- A pull-request **must not be merged until CI** has finished successfully.
- Make sure your every `commit` is [signed](https://help.github.com/en/github/authenticating-to-github/about-commit-signature-verification)

### Merge process

Merging pull requests once CI is successful:

- A PR needs to be reviewed and approved by project maintainers;
- Once a PR is ready for review please add the [`pleasereview`](https://github.com/crustio/crust/labels/pleasereview) label. Generally PRs should sit with this label for 48 hours in order to garner feedback. It may be merged before if all relevant parties had a look at it.
- PRs that break the external API must be tagged with [`breaksapi`](https://github.com/crustio/crust/labels/breaksapi), when it changes the FRAME or consensus of running system with [`breaksconsensus`](https://github.com/crustio/crust/labels/breaksconsensus);
- No PR should be merged until **all reviews' comments** are addressed;
- PR merge should use the **Squash Merging**;

## License

[GPL V3.0](https://github.com/crustio/crust/blob/master/LICENSE)

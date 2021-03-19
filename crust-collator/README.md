# Steps to run crust parachains collator locally

## Before start

Grab the Polkadot source code:

```bash
git clone https://github.com/paritytech/polkadot.git
cd polkadot
```

To make relay chain run three validators, modify function at file ```<polkadot root>/service/src/chain_spec.rs```

```sh
fn rococo_local_testnet_genesis(wasm_binary: &[u8]) -> rococo_runtime::GenesisCo
                vec![
                        get_authority_keys_from_seed("Alice"),
                        get_authority_keys_from_seed("Bob"),
+                       get_authority_keys_from_seed("Charlie"),
                ],
```

Compile source code with command ```cargo build --release --features=real-overseer```

After build, export new chain spec json file:

```sh
./target/release/polkadot build-spec --chain rococo-local --raw --disable-default-bootnode > rococo-custom.json
```

Then grab the crust blockchain source code:

```bash
git clone https://github.com/crustio/crust.git
cd crust
git checkout parachain/rococo-v2
```

Compile source code with command ```cargo build --release --package crust-collator```

## Step1: build test spec and export parachain genesis and wasm data

```shell script
./scripts/build_spec_wasm_and_state.sh
```

## Step2: run relay chain

- run Alice

```sh
./target/release/polkadot --validator --chain rococo-custom.json --tmp --node-key 0000000000000000000000000000000000000000000000000000000000000001 --rpc-cors all --ws-port 9944 --port 30333 --alice
```

Got Alice chain identity:
```12D3KooWEyoppNCUx8Yx66oV9fJnriXwCcXwDDUA2kj6vnc6iDEp```

 - run Bob (set Alice as bootnodes)

 ```sh
./target/release/polkadot --validator --chain rococo-custom.json --tmp --rpc-cors all --ws-port 9955 --port 30334 --bob --bootnodes /ip4/127.0.0.1/tcp/30333/p2p/12D3KooWEyoppNCUx8Yx66oV9fJnriXwCcXwDDUA2kj6vnc6iDEp
```

 - run Charlie (set Alice as bootnodes)

 ```sh
./target/release/polkadot --validator --chain rococo-custom.json --tmp --rpc-cors all --ws-port 9966 --port 30335 --charlie --bootnodes /ip4/127.0.0.1/tcp/30333/p2p/12D3KooWEyoppNCUx8Yx66oV9fJnriXwCcXwDDUA2kj6vnc6iDEp
```

## Step3 Run crust parachain collator

Add ```RUST_LOG=debug RUST_BACKTRACE=1``` if you want see more details

 - run the first parachain collator

 ```sh
./target/release/crust-collator --tmp --chain ./crust-collator/res/7777.json --parachain-id 7777 --port 40343 --ws-port 9953 --rpc-cors all --validator -lruntime=debug  -- --chain ../polkadot/rococo-custom.json
```

 - run the second parachain collator for the same parachain

 ```sh
./target/release/crust-collator --tmp --chain ./crust-collator/res/6666.json --parachain-id 6666 --port 40342 --ws-port 9952 --rpc-cors all --validator -lruntime=debug  -- --chain ../polkadot/rococo-custom.json
```

## Step4 Register your parachain into rococo local test
submit the `paraSudoConfig:sudoParaScheduleInit` extrinsic to register para chain to the relay chain

## Step5 Open channel and accept channel
Ref to this [page](https://wiki.acala.network/build/development-guide/composable-chains/open-hrmp-channel)
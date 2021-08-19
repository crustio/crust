# Dockerize Crust

## Scripts

- `build.sh`: Build crust official docker image.
- `build_env.sh`: Build crust's dependencies, including `rust`, `nightly toolchain`, `wasm toolchain` and `llvm`.
- `build_bin.sh`: Build crust native binary on linux.

## Build

Please run the scripts under the ***root*** of this repository. ***DO NOT*** run from `docker` folder!

### Build crust environment

```bash
  docker/build_env.sh
```

### Build crust

1. First build `crust_bin`

  ```bash
  docker/build_bin.sh
  ```

*Hints*

- Use `-m` flag to use a Chinese cargo mirror, cargo package downloads will be much faster for Chinese developers.
- Use `-c` flag to specify a custom cargo cache location,
    it defaults to docker/.cache, you may want to change it if you want share cargo cache between multiple clones.
- Use `-r` to perform a full build (equals to `cargo clean & cargo build`).
- Use `-p` to push to [Docker Hub](https://hub.docker.com/)

2. Then, build and push crust (with `docker push`)

  ```bash
  docker/build.sh
  ```

## Usage

### 1. Connect to mainnet

```shell
docker pull crustio/crust:mainnet
docker run -v /tmp/crust:/tmp/crust --network host crustio/crust:mainnet ./crust --base-path /tmp/chain --chain mainnet [more_options]
```

### 2. Connect to maxwell

```shell
docker pull crustio/crust:maxwell
docker run -v /tmp/crust:/tmp/crust --network host crustio/crust:maxwell ./crust --base-path /tmp/chain --chain maxwell [more_options]
```

### 3. Connect to rocky

```shell
docker pull crustio/crust:rocky
docker run -v /tmp/crust:/tmp/crust --network host crustio/crust:rocky ./crust --base-path /tmp/chain --chain rocky [more_options]
```

**[more_options]** can be:

1. `--rpc-external`: Specify HTTP RPC server TCP port, default is `9933`.
2. `--ws-external`: Specify WebSockets RPC server TCP port, default is `9944`.
3. `--rpc-cors all`: Specify browser Origins allowed to access the HTTP & WS RPC servers.
4. `--bootnodes`: Specify a list of bootnodes.
5. More options can be found with `--help`.

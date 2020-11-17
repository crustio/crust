# Dockerize crust

## Scripts

- `build.sh`: Build crust official docker image.
- `build_env.sh`: Build crust's dependencies, including `rust`, `nightly toolchain`, `wasm toolchain` and `llvm`.
- `build_bin.sh`: Build crust native binary on linux.

## Usage

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

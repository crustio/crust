# Dockerize crust

## Structure

- `build.sh`: Build crust official docker image.
- `build_env.sh`: Build crust's dependencies, including `rust`, `nightly toochain` and `wasm toochain`.
- `build_bin.sh`: Build crust native binary on linux.

## Usage
Run the scripts at the the root of this repository. DO NOT run from the scripts folder!

### Build and push crust environment image
```bash
  docker/build_env.sh -p
```

### Build and push crust

1. First build `crust_bin`
    ```bash
    docker/build_bin.sh
    ```
    > Hints
      - Use `-m` flag to use a Chinese cargo mirror, cargo package downloads will be much faster for Chinese users.
      - Use `-c` flag to specify a custom cargo cache location,
            it defaults to docker/.cache, you may want to change it if you want share cargo cache between multiple clones.
      - Use `-r` to perform a full build (equals to `cargo clean & cargo build`).

2. Then, build and push crust (with `docker push`)
    ```bash
    docker/build.sh
    ```
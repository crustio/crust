# crust build base image
From "rust:latest"

ARG TOOLCHAIN
ENV RUSTUP_TOOLCHAIN ${TOOLCHAIN:-nightly-2020-07-12}
RUN rustup update ${RUSTUP_TOOLCHAIN}
RUN rustup update stable
RUN rustup target add wasm32-unknown-unknown --toolchain ${RUSTUP_TOOLCHAIN}
RUN rustup default ${RUSTUP_TOOLCHAIN}
RUN apt-get update && apt-get install -y llvm clang libclang-dev


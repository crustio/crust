#!/usr/bin/env bash

set -e

echo "*** Initializing WASM build environment"

if [ -z $CI_PROJECT_NAME ] ; then
   rustup update nightly
   rustup update stable
fi

# do not need wasm for the alpha version
rustup target add wasm32-unknown-unknown --toolchain nightly
rustup default nightly
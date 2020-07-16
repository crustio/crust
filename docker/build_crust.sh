#! /usr/bin/env bash

BUILD_DIR="`pwd`"
TOOLCHAIN_VER="`cat docker/rust-toolchain`"
DIST_FILE="target/release/crust"

usage() {
    echo "Usage:"
		echo "    $0 -h                      Display this help message."
		echo "    $0 [options]"
    echo "Options:"
    echo "     -p publish image"
    echo "     -m use Chinese mirror"
    echo "     -c [dir] use cache directory"

	  exit 1;
}

PUBLISH=0
MIRROR=0
CACHEDIR=""

while getopts ":hmpc:" opt; do
    case ${opt} in
        h )
			      usage
            ;;
        p )
            PUBLISH=1
            ;;
        m )
            MIRROR=1
            ;;
        c )
            CACHEDIR=$OPTARG
            ;;
        \? )
            echo "Invalid Option: -$OPTARG" 1>&2
            exit 1
            ;;
    esac
done


function echo_c {
    echo -e "\e[1;$1m$2\e[0m"
}

function log_info {
    echo_c 33 "$1"
}

function log_success {
    echo_c 32 "$1"
}

function log_err {
    echo_c 35 "$1"
}

log_info "using cache dir: $CACHEDIR"
if [ ! -d $CACHEDIR ]; then
    log_err "directory $CACHEDIR doesn't exist!"
    exit 1
fi

if [ -z $CACHEDIR ]; then
    CACHEDIR="${BUILD_DIR}/docker/.cache"
    log_info "using default cache dir: $CACHEDIR"
    log_info "using a custom location for cache directory is recommended"
    mkdir -p $CACHEDIR
fi

function build_crust {
  echo_c 33 "using build dir: $BUILD_DIR"

  log_success "prepare docker build image, run docker pull"
  docker pull crustio/crust-build:${TOOLCHAIN_VER}
  if [ $? -ne 0 ]; then
    echo "failed to pull docker image"
    exit 1
  fi


  if [ $MIRROR -eq "1" ]; then
      echo "config mirror..."
      mkdir -p .cargo
      cp ./docker/cargo.config .cargo/config
  fi

  RUN_OPTS="-v $BUILD_DIR:/opt/crust -v $CACHEDIR:/opt/cache"

  echo_c 33 "using run opts: $RUN_OPTS"
  docker run -i -t --env CARGO_HOME=/opt/cache $RUN_OPTS crustio/crust-build:${TOOLCHAIN_VER} /bin/bash -c "cd /opt/crust; cargo build --release;  echo done building"
  echo_c 33 "build done, validting result"

  if [ ! -f $DIST_FILE ]; then
    echo_c 33 "build failed, $DIST_FILE does not exist"
    exit 1
  else
    log_success "$DIST_FILE exists - passed"
    echo_c 33 "build validation passed"
  fi
}

build_crust

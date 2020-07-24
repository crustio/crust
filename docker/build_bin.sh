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
    echo "     -r rebuild, will do clean and build"

	  exit 1;
}

PUBLISH=0
MIRROR=0
CACHEDIR=""
REBUILD=0

while getopts ":hmrpc:" opt; do
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
        r )
            REBUILD=1
            ;;
        c )
            CACHEDIR=$OPTARG
            ;;
        \? )
            echo "Invalid options: -$OPTARG" 1>&2
            exit 1
            ;;
    esac
done

DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" >/dev/null 2>&1 && pwd )"
source $DIR/utils.sh

log_info "Using cache dir: $CACHEDIR"
if [ ! -d $CACHEDIR ]; then
    log_err "directory $CACHEDIR doesn't exist!"
    exit 1
fi

if [ -z $CACHEDIR ]; then
    CACHEDIR="${BUILD_DIR}/docker/.cache"
    log_info "Using default cache dir: $CACHEDIR"
    log_info "Using a custom location for cache directory is recommended"
    mkdir -p $CACHEDIR
fi

function build_bin {
  echo_c 33 "Using build dir: $BUILD_DIR"

  log_success "Preparing docker build image, running docker pull"
  docker pull crustio/crust-env:${TOOLCHAIN_VER}
  if [ $? -ne 0 ]; then
    echo "Failed to pull docker image."
    exit 1
  fi


  if [ $MIRROR -eq "1" ]; then
      echo "Config mirror..."
      mkdir -p .cargo
      cp ./docker/Cargo.config .cargo/config
  fi

  RUN_OPTS="-v $BUILD_DIR:/opt/crust -v $CACHEDIR:/opt/cache"

  CIDFILE=`mktemp`
  rm $CIDFILE
  echo_c 33 "Using run opts: $RUN_OPTS"
  CMD=""
  if [ $REBUILD -eq "1" ]; then
      CMD="cargo clean --release; "
  fi
  CMD="$CMD cargo build --release;"

  log_info "Building command: $CMD"
  docker run --workdir /opt/crust --cidfile $CIDFILE -i -t --env CARGO_HOME=/opt/cache $RUN_OPTS crustio/crust-env:${TOOLCHAIN_VER} /bin/bash -c "$CMD"
  CID=`cat $CIDFILE`
  log_info "Cleanup temp container $CID"
  docker rm $CID
  echo_c 33 "Build done, checking results"

  if [ ! -f $DIST_FILE ]; then
    log_err "Build failed, $DIST_FILE does not exist"
    exit 1
  else
    log_success "$DIST_FILE exists - passed"
  fi
  log_info "Crust built at: $DIST_FILE"
}

build_bin

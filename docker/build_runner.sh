#! /usr/bin/env bash
# crust runner builder

usage() {
    echo "Usage:"
		echo "    $0 -h                      Display this help message."
		echo "    $0 [options]"
    echo "Options:"
    echo "     -p publish image"

	  exit 1;
}

PUBLISH=0

while getopts ":hp" opt; do
    case ${opt} in
        h )
			      usage
            ;;
        p )
            PUBLISH=1
            ;;
        \? )
            echo "Invalid Option: -$OPTARG" 1>&2
            exit 1
            ;;
    esac
done

source docker/utils.sh

BUILD_DIR="`pwd`"
DIST_FILE="target/release/crust"
CRUST_VER=`head -n 10 runtime/Cargo.toml|awk '/version/{print $3}' |sed  s"/'//g"`
IMAGEID="crustio/crust:${CRUST_VER}"

if [ ! -f "$DIST_FILE" ]; then
    log_err "application $DIST_FILE doesn't exist, please build crust first!"
    exit 1
fi

log_info "building runner image, crust version: ${CRUST_VER}, dist file $DIST_FILE"

cp -f $DIST_FILE docker/crust-runner/crust

docker build docker/crust-runner -t $IMAGEID

if [ $? -eq "0" ]; then
    echo "done building crust runner image, tag: $IMAGEID"
else
    echo "failed build crust runner!"
    exit 1
fi

log_info "build success"
if [ "$PUBLISH" -eq "1" ]; then
    echo "will publish image to $IMAGEID"
    docker push $IMAGEID
fi

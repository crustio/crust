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
DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" >/dev/null 2>&1 && pwd )"
source $DIR/utils.sh

BUILD_DIR="`pwd`"
DIST_FILE="target/release/crust"
CRUST_VER=`head -n 10 node/Cargo.toml|awk '/version/{print $3}' |sed  s"/\"//g"`
IMAGEID="crustio/crust:${CRUST_VER}"

if [ ! -f "$DIST_FILE" ]; then
    log_err "Binary from $DIST_FILE doesn't exist, please build crust binary first."
    exit 1
fi

log_info "Building crust image, version: ${CRUST_VER}, bin file $DIST_FILE"

cp -f $DIST_FILE docker/crust/crust

docker build docker/crust -t $IMAGEID

if [ $? -eq "0" ]; then
    echo "Done building crust image, tag: $IMAGEID"
else
    echo "Failed on building crust."
    exit 1
fi

log_info "Build success"
if [ "$PUBLISH" -eq "1" ]; then
    echo "Publishing image to $IMAGEID"
    docker push $IMAGEID
fi

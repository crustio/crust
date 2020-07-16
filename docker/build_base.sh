#! /usr/bin/env bash


echo "building crust base image"

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


TOOLCHAIN_VER="`cat docker/rust-toolchain`"
IMAGEID="crustio/crust-build:${TOOLCHAIN_VER}"

docker build docker/crust-build --build-arg TOOLCHAIN="${TOOLCHAIN_VER}" -t $IMAGEID

if [ $? -eq "0" ]; then
    echo "done building crust base image, tag: $IMAGEID"
else
    echo "failed build crust base image!"
    exit 1
fi

echo "build success"
if [ "$PUBLISH" -eq "1" ]; then
    echo "will publish image to $IMAGEID"
    docker push $IMAGEID
fi


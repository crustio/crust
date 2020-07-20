#! /usr/bin/env bash

ARGS=${CRUST_ARGS:-""}
echo "Starting crust using args: ${ARGS}"

cd /opt/crust
/opt/crust/crust ${ARGS}
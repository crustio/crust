#! /usr/bin/env bash


RUN_ARGS=${CRUST_ARGS:-""}
echo "starting crust using args: ${RUN_ARGS}"

cd /opt/crust
/opt/crust/crust ${RUN_ARGS}

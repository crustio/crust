#!/usr/bin/env bash
version="0.3.0"

# check if is built
if [ ! -f target/release/crust ]; then
    echo 'No executable crust file, building now...'
    cargo build --release
fi

if [ -d crust/ ]; then rm -Rf crust/; fi

# create crust folder
mkdir crust
mkdir crust/bin

# create and fillin VERSION file
touch crust/VERSION
echo $version >> crust/VERSION

# copy crust
cp target/release/crust crust/bin/

# package
tar -cvf crust.tar crust

# delete crust folder
rm -Rf crust/
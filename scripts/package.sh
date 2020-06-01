#!/usr/bin/env bash
version="0.3.0"

# check if is built
if [ ! -f target/release/crust ]; then
    echo "No executable file, you should run `cargo build --release` first."
else
    if [ -d crust/ ]; then rm -Rf crust/; fi
    
    # create crust folder
    mkdir crust
    mkdir crust/bin
    
    # create and fillin VERSION file
    touch crust/bin/VERSION
    echo $version >> crust/bin/VERSION

    # copy crust
    cp target/release/crust crust/bin/

    # package
    tar -cvf crust.tar crust 
fi

# Crust docker scripts
> Docker scripts for buildign and running

> it includes:
- build_base.sh: build the base image for building crust binaries
- build_crust.sh: build crust binaries
- build_runner.sh: build crust runner docker image 

## Usage
Run the scripts at the the root of this repository. DO NOT run from the scripts folder!
### build base image and publish
```bash
  docker/scripts/build_base.sh -p
```
  
### build crust
run:
```bash
 docker/scripts/build_crust.sh
```

> hints
  - Use -m flag to use a chinese cargo mirror, cargo package downloads will be much faster for some users.
  - Use -c flag to specify a custom cargo cache location,
        it defaults to docker/.cache, you may want to change it if you want share cargo cache between multiple clones.
  - Use -r to perform a full build (clean+build).
  
        
### build crust and the runner image
```bash
 docker/scripts/build_crust.sh && docker/scripts/build_runner.sh
```

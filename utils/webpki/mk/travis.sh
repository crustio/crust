#!/usr/bin/env bash
#
# Copyright 2015 Brian Smith.
#
# Permission to use, copy, modify, and/or distribute this software for any
# purpose with or without fee is hereby granted, provided that the above
# copyright notice and this permission notice appear in all copies.
#
# THE SOFTWARE IS PROVIDED "AS IS" AND AND THE AUTHORS DISCLAIM ALL WARRANTIES
# WITH REGARD TO THIS SOFTWARE INCLUDING ALL IMPLIED WARRANTIES OF
# MERCHANTABILITY AND FITNESS. IN NO EVENT SHALL THE AUTHORS BE LIABLE FOR ANY
# SPECIAL, DIRECT, INDIRECT, OR CONSEQUENTIAL DAMAGES OR ANY DAMAGES
# WHATSOEVER RESULTING FROM LOSS OF USE, DATA OR PROFITS, WHETHER IN AN ACTION
# OF CONTRACT, NEGLIGENCE OR OTHER TORTIOUS ACTION, ARISING OUT OF OR IN
# CONNECTION WITH THE USE OR PERFORMANCE OF THIS SOFTWARE.

set -eux -o pipefail
IFS=$'\n\t'

printenv
$CC_X --version
$CXX_X --version
make --version

cargo version
rustc --version

if [[ "$MODE_X" == "RELWITHDEBINFO" ]]; then mode=--release; fi

# TODO: Add --target $TARGET_X.

CC=$CC_X CXX=$CXX_X cargo build -j2 ${mode-} -vv

# Default features
CC=$CC_X CXX=$CXX_X cargo test -j2 ${mode-} -vv

CC=$CC_X CXX=$CXX_X cargo test -j2 ${mode-} --all-features -vv

CC=$CC_X CXX=$CXX_X cargo test -j2 ${mode-} --no-default-features -vv

CC=$CC_X CXX=$CXX_X cargo test --no-default-features --features=trust_anchor_util -vv

CC=$CC_X CXX=$CXX_X cargo doc --verbose

CC=$CC_X CXX=$CXX_X cargo clean --verbose

echo end of mk/travis.sh

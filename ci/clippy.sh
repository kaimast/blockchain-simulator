#! /bin/bash
rustup component add clippy --toolchain nightly-x86_64-unknown-linux-gnu
cargo clippy -- -Aclippy::needless_return -Dwarnings

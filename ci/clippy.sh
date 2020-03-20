#! /bin/bash
rustup component add clippy --toolchain nightly-x86_64-unknown-linux-gnu
cargo clippy --features="server" -- -Aclippy::needless_return -Dwarnings

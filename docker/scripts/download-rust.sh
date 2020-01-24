#!/bin/bash

set -u

# Install rust through rustup
curl https://sh.rustup.rs -ksSf > $HOME/install_rustup.sh
#curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs > $HOME/install_rustup.sh
#curl -sf -L https://static.rust-lang.org/rustup.sh > $HOME/install_rustup.sh

sh $HOME/install_rustup.sh -y --default-toolchain $RUST_VERSION


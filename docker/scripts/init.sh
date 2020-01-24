#!/bin/bash

DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" >/dev/null 2>&1 && pwd )"

set -e

if [ -x "/webapp/source/update.sh" ]; then
  printf "*** Run the user specified update script ***\n"
  source "/webapp/source/update.sh"
fi

source $HOME/.cargo/env

export CARGO_TARGET_DIR="$HOME/webapp/target"

if [[ $1 == "cargo" ]]; then
    cd /webapp/source
    shift
    exec cargo $@
else
    exec "$@"
fi


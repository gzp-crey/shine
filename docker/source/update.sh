#!/bin/bash

TOKEN="3_ccrwj9fQrX8hcFzN95"
DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" >/dev/null 2>&1 && pwd )"

cd $DIR
if [ -d shine-backend ]; then
    cd shine-backend
    git reset --hard
    git checkout master
    git pull
else     
    git clone https://oauth2:${TOKEN}@gitlab.com/gzp/shine-backend.git
    cd shine-backend
fi

cargo build --release

cp $CARGO_TARGET_DIR/release/shine-auth /webapp/binary
#!/bin/bash

if [ "$GIT_TOKEN" == "" ]; then
    echo "No token provided, skipping clone and build step"
else
    DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" >/dev/null 2>&1 && pwd )"

    cd $DIR
    if [ -d shine-backend ]; then
        cd shine-backend
        git reset --hard
        git checkout master
        git pull
    else     
        git clone https://oauth2:${GIT_TOKEN}@gitlab.com/gzp/shine-backend.git
        cd shine-backend
    fi

    rm -f $CARGO_TARGET_DIR/release/shine-auth
    cargo build --release

    cp $CARGO_TARGET_DIR/release/shine-auth /webapp/binary
fi
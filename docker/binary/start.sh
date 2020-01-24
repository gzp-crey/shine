#!/bin/bash

# execute service only if git clone and build is enabled
if [ "${GIT_TOKEN}" != "" ]; then
    /webapp/binary/shine-app /webapp/binary/secret.config.json
fi    

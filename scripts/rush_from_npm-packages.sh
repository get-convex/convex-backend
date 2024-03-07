#!/bin/bash
set -e
SCRIPT_DIR=$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )
RUSH="$SCRIPT_DIR/node_modules/.bin/rush"
NPM_PACKAGES="$( cd "$SCRIPT_DIR/../npm-packages" && pwd )"

if [[ $(pwd) == *"npm-packages"* ]]; then
  $RUSH "$@"
else
  cd "$NPM_PACKAGES"
  $RUSH "$@"
fi

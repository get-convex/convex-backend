#!/usr/bin/env bash

node -v | grep -q '^v22' || (echo "Node version must be ^22 to lint this package. Run \`nvm use 22\` to switch to the correct version." && exit 1)

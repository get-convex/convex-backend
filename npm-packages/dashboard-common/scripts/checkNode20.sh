#!/bin/bash

node -v | grep -q '^v20' || (echo "Node version must be >= 20 to lint this package. Run \`nvm use 20\` to switch to the correct version." && exit 1)

#! /bin/bash

set -e
mkdir -p /convex/tmp
./convex-local-backend "$@" --instance-name "$INSTANCE_NAME" --instance-secret "$INSTANCE_SECRET"

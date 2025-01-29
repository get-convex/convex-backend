#! /bin/bash

set -e
mkdir -p /tmp/convex
./convex-local-backend "$@" --instance-name "$INSTANCE_NAME" --instance-secret "$INSTANCE_SECRET"

#!/bin/bash

set -e

source ./read_credentials.sh

ADMIN_KEY=$(./generate_key "$INSTANCE_SECRET" "$INSTANCE_NAME")

echo "$ADMIN_KEY"


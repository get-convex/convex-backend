#!/bin/bash

set -e

source ./read_credentials.sh

ADMIN_KEY=$(./generate_key "$INSTANCE_NAME" "$INSTANCE_SECRET")

echo "$ADMIN_KEY"


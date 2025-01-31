#! /bin/bash

export DATA_DIR=${DATA_DIR:-/convex/data}
export TMPDIR=${TMPDIR:-"$DATA_DIR/tmp"}
export STORAGE_DIR=${STORAGE_DIR:-"$DATA_DIR/storage"}
export SQLITE_DB=${SQLITE_DB:-"$DATA_DIR/db.sqlite3"}

set -e
mkdir -p "$TMPDIR" "$STORAGE_DIR"
# TODO: We should not be passing the ports from environment variables, but this is a quick fix for now
exec ./convex-local-backend "$@" \
    --instance-name "$INSTANCE_NAME" \
    --instance-secret "$INSTANCE_SECRET" \
    --local-storage "$STORAGE_DIR" \
    --port "$PORT" \
    --site-proxy-port "$SITE_PROXY_PORT" \
    "$SQLITE_DB"

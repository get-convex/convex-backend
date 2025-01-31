#! /bin/bash

export DATA_DIR=${DATA_DIR:-/convex/data}
export TMPDIR=${TMPDIR:-"$DATA_DIR/tmp"}
export STORAGE_DIR=${STORAGE_DIR:-"$DATA_DIR/storage"}
export SQLITE_DB=${SQLITE_DB:-"$DATA_DIR/db.sqlite3"}
export CREDENTIALS_DIR=${CREDENTIALS_DIR:-"$DATA_DIR/credentials"}

set -e
mkdir -p "$TMPDIR" "$STORAGE_DIR" "$CREDENTIALS_DIR"

# Set INSTANCE_SECRET by checking in order:
# 1. Use existing INSTANCE_SECRET env var if set
# 2. Read from CREDENTIALS_DIR/instance_secret if file exists
# 3. Generate new random secret if neither exists
# Finally, save the secret to disk for persistence
INSTANCE_SECRET=${INSTANCE_SECRET:-$(cat "$CREDENTIALS_DIR/instance_secret" 2>/dev/null || openssl rand -hex 32)}
echo "$INSTANCE_SECRET" > "$CREDENTIALS_DIR/instance_secret"


# Set INSTANCE_NAME by checking in order:
# 1. Use existing INSTANCE_NAME env var if set
# 2. Read from CREDENTIALS_DIR/instance_name if file exists
# 3. Use default name "convex-self-hosted" if neither exists
# Finally, save the name to disk for persistence
INSTANCE_NAME=${INSTANCE_NAME:-$(cat "$CREDENTIALS_DIR/instance_name" 2>/dev/null || echo "convex-self-hosted")}
echo "$INSTANCE_NAME" > "$CREDENTIALS_DIR/instance_name"

# TODO: We should not be passing the ports from environment variables, but this is a quick fix for now
exec ./convex-local-backend "$@" \
    --instance-name "$INSTANCE_NAME" \
    --instance-secret "$INSTANCE_SECRET" \
    --local-storage "$STORAGE_DIR" \
    --port "$PORT" \
    --site-proxy-port "$SITE_PROXY_PORT" \
    "$SQLITE_DB"

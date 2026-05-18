#!/bin/sh
# Runtime override of NEXT_PUBLIC_* URLs baked into the Next.js client
# bundle at image build time. The image ships with placeholder values
# (http://localhost:8050 and http://localhost:6791); this script rewrites
# them to the operator-provided PUBLIC_* env vars before launching the
# server. Skips the rewrite when env equals the baked defaults so a
# no-config run is a fast no-op.
set -eu

BAKED_ORCH="http://localhost:8050"
BAKED_DASH="http://localhost:6791"
NEW_ORCH="${PUBLIC_ORCHESTRATOR_URL:-$BAKED_ORCH}"
NEW_DASH="${PUBLIC_SELF_HOSTED_DASHBOARD_URL:-$BAKED_DASH}"

if [ "$NEW_ORCH" != "$BAKED_ORCH" ] || [ "$NEW_DASH" != "$BAKED_DASH" ]; then
  echo "[dashboard-orchestrator] rewriting baked URLs:"
  echo "  $BAKED_ORCH -> $NEW_ORCH"
  echo "  $BAKED_DASH -> $NEW_DASH"
  find .next -type f \( -name '*.js' -o -name '*.html' -o -name '*.json' \) -print0 \
    | xargs -0 -r sed -i \
        -e "s|$BAKED_ORCH|$NEW_ORCH|g" \
        -e "s|$BAKED_DASH|$NEW_DASH|g"
fi

exec node server.js

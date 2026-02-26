#!/usr/bin/env bash
# Netlify ignore command: exit 0 to skip build, exit 1 to proceed.
# https://docs.netlify.com/build/configure-builds/ignore-builds/
#
# These paths match the docs build inputs defined in docusaurus.config.ts:
# - npm-packages/docs/ (the docs site itself)
# - npm-packages/convex/src/{browser,server,react,react-auth0,react-clerk,nextjs,values}/ (TypeDoc API generation entry points)
# - npm-packages/convex/tsconfig.json (TypeDoc config)
# - npm-packages/@convex-dev/platform/ (OpenAPI specs)

git diff --quiet "$CACHED_COMMIT_REF" "$COMMIT_REF" -- \
  npm-packages/docs/ \
  npm-packages/convex/src/browser/ \
  npm-packages/convex/src/server/ \
  npm-packages/convex/src/react/ \
  npm-packages/convex/src/react-auth0/ \
  npm-packages/convex/src/react-clerk/ \
  npm-packages/convex/src/nextjs/ \
  npm-packages/convex/src/values/ \
  npm-packages/convex/tsconfig.json \
  npm-packages/@convex-dev/platform/

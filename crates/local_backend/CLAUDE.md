# local_backend

## Regenerating API specs after changing HTTP routes

Whenever you add/remove/change an HTTP route or its `#[utoipa::path(...)]`
annotation or docstring (anything under `src/` that feeds the OpenAPI docs —
e.g. `streaming_export.rs`, `dashboard.rs`, `public_api.rs`), the checked-in
OpenAPI JSON specs and the generated TypeScript clients go stale and must be
regenerated. Even a docstring-only edit counts, because descriptions are baked
into the specs.

1. Regenerate the OpenAPI JSON. The `test_api_specs_match` test rewrites the
   spec files on mismatch and then panics, so run it twice — it fails and
   rewrites on the first run, passes on the second:

   ```sh
   cargo test -p local_backend test_api_specs_match  # fails, rewrites specs
   cargo test -p local_backend test_api_specs_match  # passes
   ```

   This updates three files:
   - `npm-packages/dashboard/dashboard-deployment-openapi.json`
   - `npm-packages/@convex-dev/platform/public-deployment-openapi.json`
   - `npm-packages/@convex-dev/platform/deployment-openapi.json`

2. Regenerate the TypeScript clients from the updated JSON (descriptions become
   JSDoc comments):

   ```sh
   cd npm-packages/@convex-dev/platform && npm run generateApiSpec
   ```

   `generateApiSpec` covers the deployment, management, and log-stream specs;
   run just `generateDeploymentApiSpec` if only `deployment-openapi.json`
   changed.

Commit the regenerated JSON and `.ts` files alongside the Rust change.

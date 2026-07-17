# local_backend

## How the OpenAPI docs pipeline works

HTTP handlers annotated with `#[utoipa::path(...)]` are grouped into three
OpenAPI doc roots in `src/router.rs`. A route appears in a spec only if it is
registered with `utoipa_axum::routes!(...)` on an `OpenApiRouter` merged under
that doc root; handler docstrings and doc comments on request/response types
become the spec descriptions.

| Doc root          | Served at                     | Checked-in spec                                                    |
| ----------------- | ----------------------------- | ------------------------------------------------------------------ |
| `PlatformApiDoc`  | `/api/v1/openapi.json`        | `npm-packages/@convex-dev/platform/deployment-openapi.json`        |
| `PublicApiDoc`    | `/api/public_openapi.json`    | `npm-packages/@convex-dev/platform/public-deployment-openapi.json` |
| `DashboardApiDoc` | `/api/dashboard_openapi.json` | `npm-packages/dashboard/dashboard-deployment-openapi.json`         |

The `test_api_specs_match` test in `src/router.rs` asserts the served specs
match the checked-in files, rewriting them on mismatch. Downstream,
`openapi-typescript` generates the TypeScript clients (e.g.
`npm-packages/@convex-dev/platform/src/generatedDeploymentApi.ts`) from the
checked-in JSON. It resolves every `$ref`, so a schema that is referenced but
never registered in `components.schemas` fails there ("Can't resolve $ref"), not
in Rust.

## Regenerating API specs after changing HTTP routes

Whenever you add/remove/change an HTTP route or its `#[utoipa::path(...)]`
annotation or docstring (anything under `src/` that feeds the OpenAPI docs —
e.g. `streaming_export.rs`, `dashboard.rs`, `public_api.rs`), the checked-in
OpenAPI JSON specs and the generated TypeScript clients go stale and must be
regenerated. Even a docstring-only edit counts, because descriptions are baked
into the specs. Regenerate everything with:

```sh
just generate-api-specs
```

This runs `test_api_specs_match` (retrying once, since the first run rewrites
stale spec files and then fails) and then `npm run generateApiSpec` in
`npm-packages/@convex-dev/platform` (which covers the deployment, management,
and log-stream specs; run just `generateDeploymentApiSpec` there if only
`deployment-openapi.json` changed).

Commit the regenerated JSON and `.ts` files alongside the Rust change.

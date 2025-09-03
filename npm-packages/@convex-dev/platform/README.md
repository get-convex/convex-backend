# Convex management APIs

A client wrapping the management APIs available at

https://provision.convex.dev/v1/openapi.json

as well as the deployment APIs available on every Convex backend.

This wrapper is used for integration tests and examples.

It is not currently published to npm.

## Updating the API spec

### Management API specs (big_brain)

Run `cargo test -p big_brain test_api_specs_match` to rebuild the management API
specs, `cargo test -p local_backend test_api_specs_match` to rebuild the
deployment API specs, and `npm run generateApiSpec` to rebuild the clients.

This updates:

- `management-openapi.json` - Platform management API
- `dashboard-management-openapi.json` - Dashboard management API

### Deployment API specs (local_backend)

Run `cargo test -p local_backend test_api_specs_match` to rebuild the deployment
API specs.

This updates:

- `deployment-openapi.json` - Public deployment API (queries, mutations,
  actions)
- `dashboard-deployment-openapi.json` - Dashboard deployment API (admin
  operations)

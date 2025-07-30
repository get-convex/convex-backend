# Convex management APIs

A client wrapping the management APIs available at

https://provision.convex.dev/v1/openapi.json

This wrapper is used for integration tests and examples.

It is not currently published to npm.

## Updating the API spec

run `cargo test -p big_brain test_api_specs_match` to rebuild the spec and
`npm run generateApiSpec` to rebuild the client.

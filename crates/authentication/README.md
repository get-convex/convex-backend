# Authentication Crate

## Purpose and Functionality

The `authentication` crate handles authentication-related functionality for the Convex backend. It provides mechanisms for validating and authorizing access tokens and application authentication.

## Main Modules and Components

### `access_token_auth`
This module defines the `AccessTokenAuth` trait and its implementation `NullAccessTokenAuth`. The `AccessTokenAuth` trait provides a method for checking authorization based on an access token.

### `application_auth`
This module defines the `ApplicationAuth` struct, which encapsulates authentication logic supporting both legacy Deploy Keys and new Convex Access Tokens. It provides a method for checking the validity of an admin key or access token.

### `metrics`
This module defines metrics related to authentication, such as logging the use of deploy keys.

### `lib.rs`
The main entry point of the crate, which includes various functions and static variables related to authentication, such as extracting bearer tokens, validating OpenID Connect ID tokens, and validating access tokens.

## Dependencies and Features

### Dependencies
- `anyhow`: Error handling library.
- `async-trait`: Asynchronous trait support.
- `base64`: Encoding and decoding base64.
- `biscuit`: JWT library.
- `chrono`: Date and time library.
- `common`: Common utilities and components.
- `errors`: Error handling utilities.
- `futures`: Asynchronous programming utilities.
- `http`: HTTP types.
- `keybroker`: Key management utilities.
- `metrics`: Metrics collection and logging.
- `oauth2`: OAuth 2.0 client library.
- `openidconnect`: OpenID Connect client library.
- `serde`: Serialization and deserialization library.
- `serde_json`: JSON serialization and deserialization.
- `sync_types`: Synchronization types.
- `tokio`: Asynchronous runtime.
- `tracing`: Application-level tracing.
- `url`: URL parsing and manipulation.

### Features
- `metrics`: Enables metrics collection and logging.
- `tracy-tracing`: Enables tracing with Tracy.
- `testing`: Enables testing features and dependencies.

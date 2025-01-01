# Common Crate

The `common` crate provides common utilities and components used across various other crates in the Convex backend. It includes modules for authentication, error handling, logging, persistence, and more.

## Purpose and Functionality

The `common` crate serves as a shared library for common functionality that is used across multiple other crates in the Convex backend. It provides utilities and components that are essential for the overall operation of the system.

## Main Modules and Components

- `auth`: Handles authentication-related functionality.
- `bootstrap_model`: Provides utilities for bootstrapping models.
- `components`: Contains reusable components used across the system.
- `document`: Provides functionality for working with documents.
- `errors`: Handles error-related functionality.
- `knobs`: Provides configuration knobs for tuning the system.
- `log_lines`: Handles logging of lines.
- `log_streaming`: Provides functionality for streaming logs.
- `paths`: Handles path-related functionality.
- `pause`: Provides utilities for pausing execution.
- `persistence`: Handles persistence-related functionality.
- `query_journal`: Provides functionality for querying journals.
- `runtime`: Contains runtime-related utilities.
- `schemas`: Handles schema-related functionality.
- `types`: Provides common types used across the system.
- `RequestId`: Represents a unique request identifier.

## Dependencies and Features

The `common` crate has the following dependencies:

- `anyhow`
- `async-trait`
- `axum`
- `bitvec`
- `byteorder`
- `bytes`
- `cmd_util`
- `crossbeam-channel`
- `csf`
- `cstr`
- `derive_more`
- `enum-iterator`
- `errors`
- `event-listener`
- `float_next_after`
- `fnv`
- `futures`
- `futures-async-stream`
- `futures-util`
- `governor`
- `headers`
- `hex`
- `http`
- `http-body-util`
- `hyper`
- `hyper-util`
- `imbl`
- `itertools`
- `maplit`
- `metrics`
- `mime`
- `minitrace`
- `openidconnect`
- `packed_value`
- `parking_lot`
- `pb`
- `pin-project`
- `prometheus`
- `prometheus-hyper`
- `proptest`
- `proptest-derive`
- `proptest-http`
- `prost-types`
- `rand`
- `rand_chacha`
- `regex`
- `reqwest`
- `semver`
- `sentry`
- `serde`
- `serde_json`
- `sha2`
- `shape_inference`
- `sourcemap`
- `strum`
- `sync_types`
- `thiserror`
- `tld`
- `tokio`
- `tokio-metrics`
- `tokio-metrics-collector`
- `tokio-stream`
- `tonic`
- `tonic-health`
- `tower`
- `tower-cookies`
- `tower-http`
- `tracing`
- `tracy-client`
- `tungstenite`
- `tuple_struct`
- `url`
- `utoipa`
- `uuid`
- `value`

The `common` crate also provides the following features:

- `tracy-tracing`: Enables tracing with Tracy.
- `testing`: Enables testing-related functionality.

# Usage Tracking Crate

The `usage_tracking` crate handles usage tracking-related functionality for the Convex backend. It provides mechanisms to track and log various usage metrics, such as function calls, storage ingress and egress, and database bandwidth.

## Purpose and Functionality

The primary purpose of the `usage_tracking` crate is to track and log usage metrics for different components and functions within the Convex backend. It provides a `UsageCounter` struct that can be used to track various types of usage events, such as function calls, storage ingress and egress, and database bandwidth.

## Main Modules and Components

### `lib.rs`

The `lib.rs` file is the main entry point for the `usage_tracking` crate. It defines the `UsageCounter` struct and various methods for tracking different types of usage events. It also includes the `FunctionUsageTracker` struct, which is used to track usage within a transaction.

### `metrics.rs`

The `metrics.rs` file defines various metrics related to storage usage. It provides functions to log storage ingress and egress sizes, as well as the total number of storage calls.

## Dependencies and Features

The `usage_tracking` crate has the following dependencies:

- `anyhow`: A library for error handling.
- `common`: A crate that provides common utilities and components used across various other crates.
- `events`: A crate that handles event-related functionality.
- `headers`: A library for working with HTTP headers.
- `metrics`: A crate that provides metrics-related functionality.
- `parking_lot`: A library for efficient synchronization primitives.
- `pb`: A crate that provides protocol buffer definitions and utilities.
- `proptest`: A property-based testing framework.
- `proptest-derive`: A crate that provides derive macros for `proptest`.
- `tracing`: A library for instrumenting Rust programs to collect structured, event-based diagnostic information.
- `value`: A crate that provides value-related functionality.

The `usage_tracking` crate also has a `testing` feature, which enables additional dependencies and features for testing purposes.

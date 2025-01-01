# `http_client` Crate

The `http_client` crate handles HTTP client-related functionality for the Convex backend. It provides a cached HTTP client that can be used to make HTTP requests with caching support.

## Purpose and Functionality

The primary purpose of the `http_client` crate is to provide a cached HTTP client that can be used to make HTTP requests with caching support. This crate is used by the `crates/application` crate to handle HTTP client-related functionality.

## Main Modules and Components

- `lib.rs`: The main entry point of the crate. It defines the cached HTTP client and its functionality.
- `metrics.rs`: Contains metrics-related functionality for logging HTTP responses and tracking the number of requests made using the cached HTTP client.

## Dependencies and Features

The `http_client` crate has the following dependencies:

- `anyhow`: A library for error handling in Rust.
- `futures`: A library for asynchronous programming in Rust.
- `http-cache`: A library for caching HTTP responses.
- `http-cache-reqwest`: A library for integrating `http-cache` with `reqwest`.
- `metrics`: A library for collecting and reporting metrics.
- `openidconnect`: A library for handling OpenID Connect functionality.
- `reqwest`: A library for making HTTP requests.
- `reqwest-middleware`: A library for adding middleware to `reqwest` clients.
- `strum`: A library for working with enums in Rust.
- `thiserror`: A library for defining custom error types.

The `http_client` crate does not have any optional features.

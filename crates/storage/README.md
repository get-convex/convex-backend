# Storage Crate

The `storage` crate handles storage-related functionality for the Convex backend. It provides an abstraction layer for different storage backends and offers various utilities for managing and interacting with stored data.

## Purpose and Functionality

The primary purpose of the `storage` crate is to provide a unified interface for different storage backends, such as local file storage and cloud storage services. It allows the Convex backend to store and retrieve data in a consistent manner, regardless of the underlying storage implementation.

## Main Modules and Components

The `storage` crate includes the following main modules and components:

- `lib.rs`: The main entry point of the crate, which defines the core storage traits and implementations.
- `LocalDirStorage`: A storage implementation that uses the local file system for storing data.
- `BufferedUpload`: A utility for managing buffered uploads to storage backends.
- `StorageObjectReader`: A utility for reading and seeking objects stored in storage backends.
- `StorageExt`: An extension trait that provides additional helper methods for the `Storage` trait.

## Dependencies and Features

The `storage` crate depends on several other crates within the Convex backend, as well as some external crates. The main dependencies and features are as follows:

- `common`: Provides common utilities and components used across various other crates.
- `runtime`: Provides runtime utilities and abstractions for asynchronous execution.
- `value`: Provides value-related functionality, such as serialization and deserialization.
- `async-trait`: A crate for defining asynchronous traits.
- `bytes`: A crate for working with byte buffers.
- `futures`: A crate for working with asynchronous programming in Rust.
- `serde_json`: A crate for working with JSON serialization and deserialization.
- `tempfile`: A crate for working with temporary files.
- `tokio`: An asynchronous runtime for the Rust programming language.
- `tracing`: A crate for instrumenting Rust programs to collect structured, event-based diagnostic information.

The `storage` crate also includes a `testing` feature, which enables additional testing-related functionality and dependencies.

# `node_executor` Crate

The `node_executor` crate handles node execution-related functionality for the Convex backend. It provides the necessary components and utilities to execute user-defined functions (UDFs) and other tasks within a node environment.

## Purpose and Functionality

The primary purpose of the `node_executor` crate is to manage the execution of UDFs and other tasks within a node environment. It provides the necessary infrastructure to execute these functions, handle errors, and manage dependencies.

## Main Modules and Components

The `node_executor` crate includes the following main modules and components:

- `executor`: This module contains the core logic for executing UDFs and other tasks. It includes the `NodeExecutor` trait, which defines the interface for node executors, and the `Actions` struct, which provides methods for executing actions and managing dependencies.
- `local`: This module provides a local implementation of the `NodeExecutor` trait, which can be used for testing and development purposes.
- `metrics`: This module contains utilities for logging and tracking various metrics related to node execution, such as execution time, download time, and import time.
- `source_package`: This module provides utilities for managing source packages, which contain the code and dependencies required for executing UDFs.

## Dependencies and Features

The `node_executor` crate depends on several other crates within the Convex backend, including:

- `common`: Provides common utilities and components used across various other crates.
- `errors`: Handles error-related functionality.
- `isolate`: Manages isolate-related functionality.
- `keybroker`: Handles key management-related functionality.
- `metrics`: Provides utilities for logging and tracking metrics.
- `model`: Provides data models used across various other crates.
- `storage`: Handles storage-related functionality.
- `value`: Provides value-related functionality.

The `node_executor` crate also includes a `testing` feature, which enables additional functionality and dependencies for testing purposes.

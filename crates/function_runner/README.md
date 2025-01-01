# Function Runner Crate

The `function_runner` crate is responsible for handling function execution-related functionality in the Convex backend. It provides the necessary components and utilities to run functions, manage in-memory indexes, and track function usage metrics.

## Purpose and Functionality

The primary purpose of the `function_runner` crate is to execute functions within the Convex backend. It includes various modules and components that facilitate function execution, in-memory index management, and function usage tracking.

## Main Modules and Components

### `in_memory_indexes`

This module provides functionality for managing in-memory indexes. It includes the `InMemoryIndexCache` struct, which is responsible for caching and loading indexes from persistence. The `FunctionRunnerInMemoryIndexes` struct implements the `InMemoryIndexes` trait and provides methods for range queries on in-memory indexes.

### `in_process_function_runner`

This module provides the `InProcessFunctionRunner` struct, which implements the `FunctionRunner` trait. It is responsible for running functions in-process, analyzing UDF configurations, evaluating app definitions, and managing action callbacks.

### `metrics`

This module provides functionality for tracking function runner metrics. It includes various functions for logging cache metrics, loading index timers, and tracking the time to begin a transaction.

### `module_cache`

This module provides functionality for managing module caches. It includes the `ModuleCache` struct, which is responsible for caching and loading modules from persistence.

### `server`

This module provides the `FunctionRunnerCore` struct, which is responsible for managing the core functionality of the function runner. It includes methods for running functions, analyzing UDF configurations, evaluating app definitions, and managing action callbacks.

## Dependencies and Features

The `function_runner` crate depends on several other crates within the Convex backend, including `common`, `database`, `errors`, `file_storage`, `isolate`, `keybroker`, `metrics`, `model`, `runtime`, `storage`, `sync_types`, `usage_tracking`, and `value`. It also includes optional dependencies for testing, such as `proptest` and `proptest-derive`.

The crate provides a `testing` feature, which enables various testing-related dependencies and features. It also includes a `tracy-tracing` feature, which enables tracing support using the `tracy` crate.

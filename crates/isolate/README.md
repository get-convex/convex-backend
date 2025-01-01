# JS Runtime Environment

There are a few ways user code can interact with our system.

1. There's a global `Convex` object that's created very early in
   `initialization::setup_context` and populated soon after when executing
   `setup.js`. We pass this global object as the first argument to all UDFs.
2. The `Convex.syscall` method, installed in `initialization::setup_context` and
   implemented within `syscalls.rs` provides the API for the user to interact
   with the database.
3. Helpers within `setup.js` provide bindings, like `Convex.get` for interacting
   with the database without having to use `Convex.syscall` directly.
4. The user can also import system modules under `convex:/system` that will
   eventually include code derived from our npm package. For example, we'll
   eventually have a custom `Int64` object that will be available for the user
   to create themselves within UDF execution.

# Argument and return value serialization (as of 2021-11-10)

```
                             Arguments                     Return value

                       ┌───────────────────┐           ┌───────────────────┐
                       │ Convex Value (JS) │           │ Convex Value (JS) │
                       └───────────────────┘           └───────────────────┘
                                 │                               ▲
                          convexReplacer                         │
                                 │                         convexReviver
 Browser                         ▼                               │
                   ┌──────────────────────────┐    ┌──────────────────────────┐
                   │ JSON-serializable object │    │ JSON-serializable object │
                   └──────────────────────────┘    └──────────────────────────┘
                                 │                               ▲
                          JSON.serialize                         │
                                 │                          JSON.parse
                                 ▼                               │
                          ┌─────────────┐                 ┌─────────────┐
─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ┤   String    ├ ─ ─ ─ ─ ─ ─ ─ ─ ┤   String    ├ ─ ─ ─ ─ ─ ─ ─ ─ ─
                          └─────────────┘                 └─────────────┘
                                 │                               ▲
                        serde::Deserialize                       │
                                 │                       serde::Serialize
                                 ▼                               │
                     ┌──────────────────────┐        ┌──────────────────────┐
 Rust                │ Convex Value (Rust)  │        │ Convex Value (Rust)  │
                     └──────────────────────┘        └──────────────────────┘
                                 │                               ▲
                         serde::Serialize                        │
                                 │                      serde::Deserialize
                                 ▼                               │
                        ┌────────────────┐              ┌────────────────┐
─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ┤     String     │─ ─ ─ ─ ─ ─ ─ ┤     String     │─ ─ ─ ─ ─ ─ ─ ─ ─
                        └────────────────┘              └────────────────┘
                                 │                               ▲
                            JSON.parse                           │
                                 │                        JSON.serialize
                                 ▼                               │
                   ┌──────────────────────────┐    ┌──────────────────────────┐
                   │ JSON-serializable object │    │ JSON-serializable object │
                   └──────────────────────────┘    └──────────────────────────┘
                                 │                               ▲
                           convexReviver                         │
 V8                              │                        convexReplacer
                                 ▼                               │
                       ┌───────────────────┐           ┌───────────────────┐
                       │ Convex Value (JS) │           │ Convex Value (JS) │
                       └───────────────────┘           └───────────────────┘
                                 │                               ▲
                                 │                               │
                                 │    ┌─────────────────────┐    │
                                 │    │                     │    │
                                 └───▶│    User UDF code    │────┘
                                      │                     │
                                      └─────────────────────┘
```

# Purpose and Functionality

The `isolate` crate handles isolate-related functionality in the Convex backend. It provides the necessary components and utilities to manage and execute isolated JavaScript environments for user-defined functions (UDFs) and other tasks. The crate ensures that each UDF runs in a secure and isolated environment, preventing interference between different UDFs and maintaining the integrity of the system.

# Main Modules and Components

The `isolate` crate includes the following main modules and components:

- `bundled_js`: Handles bundled JavaScript code.
- `client`: Provides the client implementation for interacting with isolates.
- `concurrency_limiter`: Manages concurrency limits for isolate execution.
- `error`: Defines error types and handling for isolate-related operations.
- `execution_scope`: Manages the execution scope for isolates.
- `helpers`: Provides helper functions and utilities for isolate operations.
- `http_action`: Handles HTTP actions within isolates.
- `http`: Provides HTTP-related functionality for isolates.
- `is_instance_of_error`: Checks if an object is an instance of an error.
- `isolate_worker`: Manages isolate workers for executing UDFs.
- `isolate`: Core module for isolate management and execution.
- `metrics`: Collects and reports metrics related to isolate execution.
- `module_map`: Manages module mappings for isolates.
- `request_scope`: Manages the request scope for isolate execution.
- `strings`: Provides string-related utilities for isolates.
- `termination`: Handles termination of isolate execution.
- `test_helpers`: Provides helper functions for testing isolate functionality.
- `timeout`: Manages timeouts for isolate execution.

# Dependencies and Features

The `isolate` crate depends on several other crates within the Convex backend to provide its functionality. Some of the key dependencies include:

- `common`: Provides common utilities and components used across various other crates.
- `errors`: Defines error types and handling for the Convex backend.
- `metrics`: Collects and reports metrics for the Convex backend.
- `value`: Provides value-related functionality for the Convex backend.

The `isolate` crate also includes optional features for testing purposes, which can be enabled by specifying the `testing` feature in the `Cargo.toml` file.

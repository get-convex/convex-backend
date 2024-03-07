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

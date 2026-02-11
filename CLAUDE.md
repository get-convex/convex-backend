# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Repository Overview

Convex is an open-source reactive database for web apps. This is a polyglot monorepo with a Rust backend (`crates/`) and a TypeScript/JavaScript frontend ecosystem (`npm-packages/`), managed by Cargo and Rush respectively.

## Build & Development Commands

### Prerequisites

- [Just](https://github.com/casey/just) (task runner)
- Node.js (see `.nvmrc`, currently 20.19.5)
- Rust nightly (see `rust-toolchain`, currently nightly-2025-06-28)
- Rush: `npm ci --prefix scripts`, then `just rush install`

### Rust (crates/)

```sh
just format-rust                          # format after changes
just lint-rust                            # lint before committing
cargo build -p <crate>                    # build a specific crate
cargo test -p <crate>                     # test a specific crate
cargo test -p <crate> "test_name"         # run specific test(s)
cargo nextest run --profile ci            # run all tests (CI style, needs Postgres + MySQL)
just run-local-backend                    # run backend on port 3210
just reset-local-backend                  # wipe local data (sqlite + storage)
```

### TypeScript (npm-packages/)

```sh
just format-js                            # format after changes
just lint-js                              # lint before committing
just rush build -t <package>              # build a specific package and its deps
just rush rebuild                         # force rebuild (when Rush misses changes)
just rush install                         # install JS dependencies
just rush update                          # after modifying package.json dependencies
cd npm-packages/<package> && npm run test -- <file>  # run specific test file
```

### Running Locally

```sh
just run-local-backend                    # start backend
just convex dev                           # run CLI against local backend (from app dir)
just convex data                          # list tables
just convex logs                          # stream logs
```

### Integration Tests

```sh
cd npm-packages/js-integration-tests
just test-open-source                     # open-source integration tests (builds first)
just test-conductor                       # conductor integration tests (builds first)
```

### CI

CI runs `cargo nextest run --profile ci` with Postgres and MySQL service containers. JS packages needed by Rust isolate tests must be built first: `just rush build -t component-tests -t convex -t system-udfs -t udf-runtime -t udf-tests`.

## Architecture

### Rust Backend (crates/)

The backend is a Cargo workspace (~70 crates). Key layers:

- **local_backend** - Main binary (`convex-local-backend`). HTTP/WebSocket server entry point (axum-based).
- **application** - High-level application logic, orchestrates queries/mutations/actions.
- **database** - Core database abstraction over SQLite/Postgres storage.
- **isolate** - JavaScript UDF execution environment (uses `deno_core`).
- **function_runner** - UDF execution orchestration.
- **runtime** - Async runtime abstraction (tokio-based).
- **model** - Data model types and metadata tables.
- **value** / **packed_value** - Convex value representation and serialization.
- **sync** - Client sync protocol implementation.
- **search** / **text_search** / **vector** - Search and vector index support (uses forked tantivy and qdrant).
- **indexing** - Index management.
- **common** - Shared utilities across crates.
- **errors** - Error types with protobuf serialization.
- **pb** - Protobuf definitions (prost/tonic).
- **backend_harness** - Integration test harness for spinning up full backends.
- **storage** / **file_storage** - File storage backends (local, S3).
- **metrics** - Metrics collection (forked prometheus).

Many crates expose a `testing` feature for test utilities.

### TypeScript Ecosystem (npm-packages/)

Rush monorepo (pnpm). Key packages:

- **convex/** - Primary SDK: client libraries, React hooks, CLI (`src/cli/`), and server-side utilities.
- **dashboard/** - Convex Cloud dashboard (Next.js).
- **dashboard-self-hosted/** - Self-hosted dashboard build.
- **dashboard-common/** - Shared dashboard code.
- **@convex-dev/design-system/** - UI component library.
- **system-udfs/** - Internal Convex functions callable by CLI/dashboard.
- **udf-runtime/** - JS environment setup for user-defined functions.
- **udf-tests/** - Test utilities for the isolate layer.
- **js-integration-tests/** - End-to-end integration tests (Jest + backend-harness).
- **docs/** - Public documentation site (https://docs.convex.dev/).

### Key Dependencies (forked)

Several upstream crates are forked under `get-convex` GitHub org: tantivy, qdrant, prometheus, rust-postgres, openidconnect, biscuit. These are pinned by git rev in `Cargo.toml`.

## Formatting & Linting Details

### Rust

- `rustfmt.toml` uses nightly features: vertical imports, crate-level import granularity, `StdExternalCrate` grouping.
- Clippy workspace lints in `Cargo.toml`: warns on `await_holding_lock`, `await_holding_refcell_ref`, `unused_extern_crates`. Allows `large_enum_variant`, `too_many_arguments`, `type_complexity`, and others.

### TypeScript

- ESLint with TypeScript parser and react-hooks plugin.
- Prettier: `trailingComma: "all"`, `proseWrap: "always"`.
- Per-package lint/format via `npm run lint` / `npm run format`.

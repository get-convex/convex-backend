# wasm_runtime

A prototype runtime for running JavaScript inside WebAssembly with a Rust host,
using Wasmtime for isolation and fast instantiation. Ported from the standalone
`felix` prototype; it is an experiment aimed at replacing the V8 `isolate`
runtime, not a production runtime.

The execution model:

- bundle user JavaScript ahead of time with `esbuild`
- load and evaluate globals once
- preinitialize the guest with Wizer, so top-level JS runs at build/deploy time.
  The guest binary holds no user code: it reads the bundle from a directory the
  host preopens while `wizer_initialize` runs, so one guest build serves every
  app and the per-app artifact is the snapshot.
- instantiate fresh per-request Wasm instances from a copy-on-write memory image
- expose host functionality (storage, timers, `fetch`) through explicit async
  host ops keyed by op ID, rather than blocking a host thread per request

## Layout

- `src/host.rs` — the `convex_host` ABI: syscall dispatch, async op table,
  Wasmtime linker/store setup
- `src/fetch.rs` — buffered host `fetch`
- `src/benchmark_support.rs`, `src/bin/benchmark.rs` — request-path benchmark
  harness (bundle → guest build → Wizer → request loop), where the guest build
  is shared across the fixtures in a run
- `guest_js/` — the Wasm guest (`rquickjs`), built separately for
  `wasm32-wasip1`. It is its own Cargo workspace: a host-target build of a
  wasm-only cdylib cannot link, and cargo can only pin a per-package target on
  nightly.
- `npm-packages/wasm-runtime-fixtures` — the JS side, as a workspace package:
  `scripts/bundle-fixtures.mjs` is the esbuild pipeline producing
  `guest-bundle.js`, its sourcemap, and a handler manifest, and `fixtures/`
  holds the small apps used by the tests and benchmarks. `build.rs` builds it
  through pnpm/turbo, so the bundles are ready before any test runs.

## Setup

The tests compile the guest with `cargo build --target wasm32-wasip1`, and the
benchmark also shells out to `node` to time bundling, so you need:

```bash
rustup target add wasm32-wasip1
```

## Running

```bash
cargo test -p wasm_runtime --tests
```

Request-only CPU benchmark:

```bash
cargo run --release -p wasm_runtime --bin benchmark -- \
    --iterations 1000 --concurrency 8 --workers 8 --fixture cpu-heavy --cpu-work 500000
```

Prepare artifacts once, then benchmark requests without counting build/preinit:

```bash
cargo run --release -p wasm_runtime --bin benchmark -- --prepare-only --fixture fetch-basic
cargo run --release -p wasm_runtime --bin benchmark -- --use-prepared \
    --iterations 100 --concurrency 8 --workers 8 --fixture fetch-basic \
    --fetch-url https://docs.convex.dev --fetch-fanout 4
```

## Fixtures

- `light-load` — minimal sync handler
- `heavy-globals` — top-level initialization work
- `async-db` — Promise-based host storage ops
- `record-db` — buffered host record tests
- `sleep-host` — async host timer behavior
- `cpu-heavy` — CPU saturation and leak testing
- `fetch-basic` — buffered global `fetch`
- `convex-functions` — unmodified Convex function definitions
  (`query`/`mutation`/`action`, `v` validators, `defineSchema`, `ctx.db`,
  `ctx.runQuery`) running against the real `convex-test` package, which
  implements the database, indexes, and validators in JS. The guest supplies
  only the JS environment: `src/runtime-shims.ts` provides `console`, `global`,
  `setTimeout`/`clearTimeout` (backed by the host sleep op) and
  `MessageChannel`; `scripts/shims/node-async-hooks.mjs` stands in for
  `node:async_hooks`; host `fetch` backs actions. Because `convex-test` keeps
  its data in the QuickJS heap, state is shared across invocations on one
  instance and disappears with the instance.

`convex-functions` doubles as the artifact-size probe for pulling a real backend
framework into the guest: `light-load` preinitializes to 1.33 MB where
`convex-functions` reaches 3.78 MB, so `convex` plus `convex-test` costs roughly
2.5 MB, most of it QuickJS heap for the evaluated module graph rather than
bundle text.

## Known gaps

Top-level JS cannot call host APIs during preinit; `fetch` is buffered only,
with no streaming bodies and no cancellation; there are no resource controls,
quotas, or fairness policies; and the host boundary has had no security review.

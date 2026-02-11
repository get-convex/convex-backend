# How Convex Works

Notes from [How Convex Works](https://stack.convex.dev/how-convex-works) by Sujay Jayakar, mapped to the codebase.

---

## Overview

Convex is a database running in the cloud that executes client-defined API functions as transactions directly within the database. The frontend connects to a Convex deployment over a persistent WebSocket.

![Deployment overview](https://cdn.sanity.io/images/ts10onj4/production/f20ad6be4bbaeb7c469466c2106e3f605f67d8e9-1588x931.svg)

A Convex deployment has three main components:

![High-level architecture](https://cdn.sanity.io/images/ts10onj4/production/2ae339f39a62935266bd1518844dbdb32f5667f3-1252x832.svg)

![Deployment internals](https://cdn.sanity.io/images/ts10onj4/production/080ed291f78449353715fd412f161633ac237078-1412x1627.svg)

```
Cloud Deployment
├── Sync Worker         -- manages WebSocket sessions, tracks query sets
├── Function Runner     -- executes UDFs in V8, caches results, stores code
└── Database            -- schema, tables, indexes, committer, tx log, subscriptions
```

**Codebase mapping:**

| Component | Crate | Key files |
|-----------|-------|-----------|
| Sync Worker | `crates/sync/` | `worker.rs`, `state.rs` |
| Function Runner | `crates/function_runner/` | `server.rs`, `in_memory_indexes.rs` |
| Function Cache | `crates/application/` | `cache/mod.rs` |
| V8 / Isolate Runtime | `crates/isolate/` | `isolate_worker.rs`, `isolate2/runner.rs` |
| Database | `crates/database/` | `database.rs`, `lib.rs` |
| Committer | `crates/database/` | `committer.rs` |
| Transaction Log | `crates/database/` | `write_log.rs`, `snapshot_manager.rs` |
| Subscriptions | `crates/database/` | `subscription.rs` |
| Transactions | `crates/database/` | `transaction.rs`, `reads.rs`, `writes.rs` |
| Indexes | `crates/database/` | `transaction_index.rs`, `index_workers/` |
| Local Backend (entry point) | `crates/local_backend/` | `lib.rs` |
| Application orchestration | `crates/application/` | `lib.rs`, `application_function_runner/mod.rs` |

---

## Convex at Rest

### Functions

Three types: **queries** (read-only), **mutations** (read-write), and **actions** (side effects allowed). Queries and mutations run as transactions within the database. Code is bundled and pushed to Convex on deploy.

- UDF environment setup: `crates/isolate/src/environment/udf/mod.rs`
- Action environment: `crates/isolate/src/environment/action/`
- Syscall interface: `crates/isolate/src/environment/udf/syscall.rs`

### Transaction Log

An append-only data structure storing all versions of documents. Every document revision carries a monotonically increasing timestamp (version number). All tables share the same timestamp sequence. Multiple changes at the same timestamp apply atomically.

Each timestamp *t* defines a snapshot of the database that includes all revisions up to *t*.

![Transaction log](https://cdn.sanity.io/images/ts10onj4/production/46849de92a3a9faa7e393056c428a9a29b993a3a-1124x660.svg)

![Transaction log with updates](https://cdn.sanity.io/images/ts10onj4/production/34f7865d20bbedf57a192279a41cef39eeed1ec0-1124x980.svg)

- Timestamps are Hybrid Logical Clocks (nanoseconds since Unix epoch, 64-bit integer): `crates/common/src/types/timestamp.rs`
- Write log (the in-memory portion of the tx log): `crates/database/src/write_log.rs`
- Snapshot manager (manages views of the database at different timestamps): `crates/database/src/snapshot_manager.rs`
- Persistence layer: `crates/common/src/persistence.rs`

### Indexes

Built on top of the log, mapping each `_id` to its latest value. Uses standard multiversion concurrency control (MVCC) techniques so the index can be queried at any past timestamp. We don't store many copies -- see [CMU's Advanced DB Systems](https://www.cs.cmu.edu/~15721-f25/schedule.html).

![Index mapping](https://cdn.sanity.io/images/ts10onj4/production/731ffc87aab77be756fe915d6fc3fe6f5ecbf45b-1124x980.svg)

![Multiversion index](https://cdn.sanity.io/images/ts10onj4/production/1e78ad935e2b51227b93448cc752f5991152bb8d-1124x980.svg)

- Transaction-level index access: `crates/database/src/transaction_index.rs`
- In-memory indexes in function runner: `crates/function_runner/src/in_memory_indexes.rs`
- Index metadata/bootstrap: `crates/common/src/bootstrap_model/index/`

---

## The Sync Engine

### Transactions & Optimistic Concurrency Control

All transactions are serializable -- behavior is identical to sequential execution. Implemented via optimistic concurrency control: assume conflicts are rare, record reads/writes, check for conflicts at commit time.

Three ingredients per transaction:

1. **Begin timestamp** -- chooses the database snapshot for all reads
2. **Read set** -- precisely records all data the transaction queried (index ranges scanned)
3. **Write set** -- maps each ID to the new value proposed by the transaction

- Transaction struct: `crates/database/src/transaction.rs`
- Read set tracking: `crates/database/src/reads.rs`
- Write set accumulation: `crates/database/src/writes.rs`
- Token (captures the transaction's position for validation): `crates/database/src/token.rs`

### Commit Protocol

The committer is the sole writer to the transaction log. It receives finalized transactions, decides if they're safe to commit, and appends their write sets.

The commit protocol:

1. Assign a commit timestamp larger than all previously committed transactions
2. Check serializability: "Would this transaction have the exact same outcome if it executed at the commit timestamp instead of the begin timestamp?"
3. Walk writes between begin and commit timestamps, check for overlap with the read set
4. If no overlap -> commit (append to log)
5. If overlap -> abort. The function runner retries at a new begin timestamp past the conflict

Similar in design to FoundationDB's and Aria's commit protocols.

![Commit protocol check](https://cdn.sanity.io/images/ts10onj4/production/dc3be57f330069703714473ad266f0693cdda3f7-1124x1172.svg)

![Conflict detection](https://cdn.sanity.io/images/ts10onj4/production/b2c6c8c76eba8c59fe7cd381cf1e97e24debe0dd-1124x1172.svg)

![Transaction commit](https://cdn.sanity.io/images/ts10onj4/production/f3acb829fe53e5bb37cdb58add171bf783a5d8d9-1076x1316.svg)

- Committer implementation: `crates/database/src/committer.rs`
- Database commit entry point: `crates/database/src/database.rs`

### Subscriptions

Read sets also power realtime updates. After running a query, the system keeps its read set in the client's WebSocket session within the sync worker. When new entries appear in the transaction log, the same overlap-detection algorithm determines if the query result might have changed.

The subscription manager aggregates all client sessions, walks the transaction log once, and efficiently finds which subscriptions are invalidated.

![Subscription manager](https://cdn.sanity.io/images/ts10onj4/production/06ad6e04496479c2c5cd7362c27f606e04f0d51e-1268x1300.svg)

![Subscription overlap detection](https://cdn.sanity.io/images/ts10onj4/production/1c717328429e37026eb3c40b09a088357e179d92-1124x996.svg)

- Subscription manager: `crates/database/src/subscription.rs`
- Local backend subscription handling: `crates/local_backend/src/subs/mod.rs`
- Sync worker (manages WebSocket sessions and query sets): `crates/sync/src/worker.rs`

### Function Cache

Serving a cached result out of memory is much faster than spinning up a V8 isolate. Convex automatically caches queries, and the cache is always 100% consistent. It uses the same overlap-detection algorithm as the subscription manager to determine whether a cached result's read set is still valid at a given timestamp.

- Cache implementation: `crates/application/src/cache/mod.rs`

### Sandboxing & Determinism

Mutations must have no external side effects (enforced through sandboxing). Queries must be fully determined by their arguments and database reads. This enables safe retries and precise subscriptions.

- Isolate sandbox environment: `crates/isolate/src/environment/udf/mod.rs`
- Determinism checks: `crates/isolate/src/request_scope.rs`
- Syscall provider (controlled interface to the database): `crates/isolate/src/environment/udf/syscall.rs`
- Crypto RNG (deterministic seeding): `crates/isolate/src/environment/crypto_rng.rs`

---

## Request Flows

### Executing a Query

![Query request flow](https://cdn.sanity.io/images/ts10onj4/production/86bf1fae07efd38abaaafe95a5fb9bf15cef9839-1656x1855.svg)

1. Client mounts a component, React hook opens a WebSocket
2. Query registered with the **sync worker** (`crates/sync/src/worker.rs`)
3. Sync worker delegates to the **function runner** (`crates/function_runner/src/server.rs`)
4. Function runner checks the **function cache** (`crates/application/src/cache/mod.rs`)
5. On cache miss: spins up V8 isolate, executes the query (`crates/isolate/src/isolate_worker.rs`)
6. Result + read set returned; subscription registered with the **subscription manager**
7. Result sent back over WebSocket to client

![WebSocket connection](https://cdn.sanity.io/images/ts10onj4/production/078f5fd47459a45b328460ad72c938f920788820-2084x1636.svg)

![Query execution](https://cdn.sanity.io/images/ts10onj4/production/f6ba1966da6649035496f3532d3ad1a6fa7f50d9-1860x1616.svg)

![Function runner cache](https://cdn.sanity.io/images/ts10onj4/production/0cac02f131fb6655dff43cf842b5f3c762c35265-1732x1781.svg)

### Executing a Mutation

1. Client sends mutation request through WebSocket
2. Sync worker forwards to function runner
3. Function runner executes the mutation in V8, producing read set + write set
4. Read/write sets sent to the **committer** (`crates/database/src/committer.rs`)
5. Committer validates serializability (read set vs. concurrent writes)
6. On success: appends write set to transaction log, returns commit timestamp
7. On conflict: aborts, function runner retries at new timestamp

![Mutation execution](https://cdn.sanity.io/images/ts10onj4/production/f522b5dce500b6b0ac3f4036717e540030614e87-2004x1620.svg)

![Committer processing](https://cdn.sanity.io/images/ts10onj4/production/8d739e25af4e2a66849b248bffa44007a761bfd5-1748x1620.svg)

### Updating a Subscription

1. New entry appears in transaction log after a committed mutation
2. **Subscription manager** walks the log, checks overlap with active read sets
3. If overlap detected: query is re-run by the function runner at the new timestamp
4. Updated result pushed to client over WebSocket

![Subscription updates](https://cdn.sanity.io/images/ts10onj4/production/f93766ddd64152d67436d5e0826cf6a1da628870-1684x1657.svg)

![Updated result propagation](https://cdn.sanity.io/images/ts10onj4/production/aa134b2a50b1298e4e9898ee711a5c3a457f1cde-1692x1620.svg)

---

## Not Covered Here

Actions, auth, end-to-end type-safety, file storage, virtual system tables, scheduling, crons, import/export, text search and vector search indexes, pagination, and more.

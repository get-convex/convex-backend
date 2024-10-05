//! Tunable limits and parameters for backends.
//! These knobs can all be overridden in production by setting a key in Consul
//! at `convex-backend/<instance-name>/knobs/<knob-name>` and the desired value.
//!
//! See go/knobs-playbook for instructions.
//!
//! Every knob here should have a comment explaining what it's for and the
//! upper/lower bounds if applicable so an oncall engineer can adjust these
//! safely for a backend if needed.
//!
//! When running locally, these knobs can all be overridden with an environment
//! variable.
#![deny(missing_docs)]

use std::{
    num::NonZeroU32,
    sync::LazyLock,
    time::Duration,
};

use cmd_util::env::env_config;

use crate::minitrace_helpers::SamplingConfig;

/// This exists solely to allow knobs to have separate defaults for local
/// execution and prod (running in Nomad). Don't export this outside of
/// this module. We assume that if we're running in Nomad, we're in production
/// which for now is always true.
static IS_PROD: LazyLock<bool> = LazyLock::new(|| std::env::var("NOMAD_ALLOC_ID").is_ok());

/// Returns the `local` value if we are running locally, and the `prod` value if
/// we are running in production. Just syntactic sugar to shorten the knob
/// declarations below.
/// Note that it's generally a bad idea to have separate configurations for
/// local development and production as our local test environment won't match
/// what's really running in production, but there are a few knobs where the
/// production default would make local development slower/harder.
fn prod_override<T>(local_value: T, prod_value: T) -> T {
    if *IS_PROD {
        return prod_value;
    }
    local_value
}

/// Set a consistent thread stack size regardless of environment. This is
/// 2x Rust's default: https://doc.rust-lang.org/nightly/std/thread/index.html#stack-size
pub static RUNTIME_STACK_SIZE: LazyLock<usize> =
    LazyLock::new(|| env_config("RUNTIME_STACK_SIZE", 4 * 1024 * 1024));

/// 0 -> default (number of cores)
pub static RUNTIME_WORKER_THREADS: LazyLock<usize> =
    LazyLock::new(|| env_config("RUNTIME_WORKER_THREADS", 0));

/// Disable the Tokio scheduler's LIFO slot optimization, which may
/// help with tail latencies until they improve its implementation.
/// See https://docs.rs/tokio/latest/tokio/runtime/struct.Builder.html#method.disable_lifo_slot.
pub static RUNTIME_DISABLE_LIFO_SLOT: LazyLock<bool> =
    LazyLock::new(|| env_config("RUNTIME_DISABLE_LIFO_SLOT", false));

/// Maximum size of the UDF cache. Default 100MiB.
pub static UDF_CACHE_MAX_SIZE: LazyLock<usize> =
    LazyLock::new(|| env_config("UDF_CACHE_MAX_SIZE", 104857600));

/// How many UDF execution logs to keep in memory.
pub static MAX_UDF_EXECUTION: LazyLock<usize> =
    LazyLock::new(|| env_config("MAX_UDF_EXECUTION", 1000));

/// How often to flush function activity reports to analytics (in seconds).
pub static UDF_ANALYTICS_POLL_TIME: LazyLock<u64> =
    LazyLock::new(|| env_config("UDF_ANALYTICS_POLL_TIME", 60));

/// Enables the heap worker memory report.
pub static HEAP_WORKER_PRINT_REPORT: LazyLock<bool> =
    LazyLock::new(|| env_config("HEAP_WORKER_PRINT_REPORT", false));

/// How often the heap worker prints a report, if enabled.
pub static HEAP_WORKER_REPORT_INTERVAL_SECONDS: LazyLock<Duration> =
    LazyLock::new(|| Duration::from_secs(env_config("HEAP_WORKER_REPORT_INTERVAL_SECONDS", 30)));

/// This is our official action timeout. This is how much the user code
/// should be allowed to run. Note that we buffer some overhead and the actual
/// Node.js process timeout is higher. We also have separate timeout for V8
/// syscalls.
///
/// NOTE: If you update this, make sure to update the actions resource limits in
/// the docs.
pub static ACTION_USER_TIMEOUT: LazyLock<Duration> =
    LazyLock::new(|| Duration::from_secs(env_config("ACTIONS_USER_TIMEOUT_SECS", 600)));

/// Max number of rows we will read when calculating document deltas.
pub static DOCUMENT_DELTAS_LIMIT: LazyLock<usize> =
    LazyLock::new(|| env_config("DOCUMENT_DELTAS_LIMIT", 128));

/// Max number of rows we will read when calculating snapshot pages.
/// Each document can be up to `::value::MAX_USER_SIZE`
/// Note that this is a pro feature, so we can afford more memory.
pub static SNAPSHOT_LIST_LIMIT: LazyLock<usize> =
    LazyLock::new(|| env_config("SNAPSHOT_LIST_LIMIT", 1024));

/// Enables the log streaming worker.
pub static ENABLE_LOG_STREAMING: LazyLock<bool> =
    LazyLock::new(|| env_config("ENABLE_LOG_STREAMING", true));

/// The size of the log manager's event receive buffer.
pub static LOG_MANAGER_EVENT_RECV_BUFFER_SIZE: LazyLock<usize> =
    LazyLock::new(|| env_config("LOG_MANAGER_EVENT_RECV_BUFFER_SIZE", 4096));

/// The aggregation interval at which the log manager empties its event receiver
/// buffer.
pub static LOG_MANAGER_AGGREGATION_INTERVAL_MILLIS: LazyLock<u64> =
    LazyLock::new(|| env_config("LOG_MANAGER_AGGREGATION_INTERVAL", 5000));

/// Max number of times a mutation can retry due to OCC conflicts.
pub static UDF_EXECUTOR_OCC_MAX_RETRIES: LazyLock<usize> =
    LazyLock::new(|| env_config("UDF_EXECUTOR_OCC_MAX_RETRIES", 4));

/// Initial backoff when we encounter an OCC conflict.
pub static UDF_EXECUTOR_OCC_INITIAL_BACKOFF: LazyLock<Duration> =
    LazyLock::new(|| Duration::from_millis(env_config("UDF_EXECUTOR_OCC_INITIAL_BACKOFF_MS", 10)));

/// Maximum expontial backoff when facing repeated OCC conflicts.
pub static UDF_EXECUTOR_OCC_MAX_BACKOFF: LazyLock<Duration> =
    LazyLock::new(|| Duration::from_millis(env_config("UDF_EXECUTOR_OCC_MAX_BACKOFF_MS", 2000)));

/// The time for which a backend will stay around, after getting preempted,
/// answering health checks but not serving traffic.
///
/// This MUST be set to >= the `healthy_deadline` in backend.nomad. Otherwise
/// a pre-emption loop can be triggered. The old backend process dies due to
/// this timeout, and then is restarted by nomad because the new deployment
/// hasn't become healthy yet. The restarted old process then pre-empts the new
/// process, causing it to die and be restarted etc.
pub static LEASE_LOST_COOL_DOWN: LazyLock<Duration> = LazyLock::new(|| {
    Duration::from_secs(env_config(
        "LEASE_LOST_COOL_DOWN_SECS",
        prod_override(5, 130),
    ))
});

/// The time for which we will try to preempt a backend after we know it has
/// lost its lease. Duration of 0 means preemption is disabled, all past serving
/// records will be assumed shut down and deleted without attempting to preempt
/// them. We default to 0 in dev as no preemption is necessary.
///
/// Try to proactively preempt past backends for up to 2 minutes. This can
/// be much lower, but we use higher timeout to protect against transiently
/// unreachable backends. By the time elapses, a network partitioning backend,
/// should attempt to write, discover it has lost its lease and self-preempt.
pub static BACKEND_PREEMPTION_TIMEOUT: LazyLock<Duration> = LazyLock::new(|| {
    Duration::from_secs(env_config(
        "BACKEND_PREEMPTION_TIMEOUT_SECS",
        prod_override(0, 120),
    ))
});

/// How long the queue must be nonempty before we consider traffic to be
/// "congested" and start shedding traffic. When we are idle (not congested) it
/// is how long each request can live in the queue.
pub static CODEL_QUEUE_IDLE_EXPIRATION_MILLIS: LazyLock<Duration> =
    LazyLock::new(|| Duration::from_millis(env_config("CODEL_QUEUE_IDLE_EXPIRATION_MILLIS", 5000)));
/// How long each request can live in the queue when we are congested.
pub static CODEL_QUEUE_CONGESTED_EXPIRATION_MILLIS: LazyLock<Duration> = LazyLock::new(|| {
    Duration::from_millis(env_config("CODEL_QUEUE_CONGESTED_EXPIRATION_MILLIS", 50))
});

/// Default page size (in number of docuemnts) used when loading documents from
/// the database.
pub static DEFAULT_DOCUMENTS_PAGE_SIZE: LazyLock<u32> =
    LazyLock::new(|| env_config("DEFAULT_DOCUMENTS_PAGE_SIZE", 100));

/// Maximum number of documents it's okay to load into memory at once.
/// Note each document can be up to `::value::MAX_SIZE`.
pub static DOCUMENTS_IN_MEMORY: LazyLock<usize> =
    LazyLock::new(|| env_config("DOCUMENTS_IN_MEMORY", 512));

/// Length of the HTTP server TCP backlog.
pub static HTTP_SERVER_TCP_BACKLOG: LazyLock<u32> =
    LazyLock::new(|| env_config("HTTP_SERVER_TCP_BACKLOG", 256));

/// The max concurrent of concurrent HTTP requests. This also limits Node.js
/// action callbacks concurrency since those go over http.
pub static HTTP_SERVER_MAX_CONCURRENT_REQUESTS: LazyLock<usize> =
    LazyLock::new(|| env_config("HTTP_SERVER_MAX_CONCURRENT_REQUESTS", 1024));

/// Max number of user writes in a transaction
pub static TRANSACTION_MAX_NUM_USER_WRITES: LazyLock<usize> =
    LazyLock::new(|| env_config("TRANSACTION_MAX_NUM_USER_WRITES", 8192));

/// Max size of user writes in a transaction, in bytes
pub static TRANSACTION_MAX_USER_WRITE_SIZE_BYTES: LazyLock<usize> = LazyLock::new(|| {
    env_config("TRANSACTION_MAX_USER_WRITE_SIZE_BYTES", 1 << 23) // 8 MiB
});

/// Maximum size in bytes of arguments to a function.
pub static FUNCTION_MAX_ARGS_SIZE: LazyLock<usize> = LazyLock::new(|| {
    env_config("FUNCTION_MAX_ARGS_SIZE", 1 << 23) // 8 MiB
});

/// Maximum size in bytes of the result of a function.
pub static FUNCTION_MAX_RESULT_SIZE: LazyLock<usize> = LazyLock::new(|| {
    env_config("FUNCTION_MAX_RESULT_SIZE", 1 << 23) // 8 MiB
});

/// When a function exceeds FUNCTION_LIMIT_WARNING_RATIO * a corresponding
/// limit value, we add a warning log line.
pub static FUNCTION_LIMIT_WARNING_RATIO: LazyLock<f64> = LazyLock::new(|| {
    env_config("FUNCTION_LIMIT_WARNING_RATIO", 0.8) // 80%
});

/// We might generate a number of system documents for each UDF write. For
/// example, creating 4000 user documents in new tables, might result in adding
/// an additional 8000 system documents. If we hit this error, this is a system
/// error, not a developer one.
pub static TRANSACTION_MAX_SYSTEM_NUM_WRITES: LazyLock<usize> =
    LazyLock::new(|| env_config("TRANSACTION_MAX_SYSTEM_NUM_WRITES", 20000));

/// We write user modules in system tables and those can get quite large.
/// Similar to the above if we hit this limit, we should count this as system
/// error and do a use case specific validation to avoid hitting this.
pub static TRANSACTION_MAX_SYSTEM_WRITE_SIZE_BYTES: LazyLock<usize> = LazyLock::new(|| {
    env_config("TRANSACTION_MAX_SYSTEM_WRITE_SIZE_BYTES", 1 << 26) // 64 MiB
});

/// Maximum number of scheduled transactions.
pub static TRANSACTION_MAX_NUM_SCHEDULED: LazyLock<usize> =
    LazyLock::new(|| env_config("TRANSACTION_MAX_NUM_SCHEDULED", 1000));

/// Maximum number of scheduled jobs to cancel in a single transaction.
pub static MAX_JOBS_CANCEL_BATCH: LazyLock<usize> =
    LazyLock::new(|| env_config("MAX_JOBS_CANCEL_BATCH", 1000));

/// Maximum size of the arguments to a scheduled function.
pub static TRANSACTION_MAX_SCHEDULED_TOTAL_ARGUMENT_SIZE_BYTES: LazyLock<usize> =
    LazyLock::new(|| {
        env_config(
            "TRANSACTION_MAX_SCHEDULED_TOTAL_ARGUMENT_SIZE_BYTES",
            1 << 23,
        ) // 8 MiB
    });

/// Number of scheduled jobs that can execute in parallel.
// Note that the current algorithm for executing ready jobs has up to
// SCHEDULED_JOB_EXECUTION_PARALLELISM overhead for every executed job, so we
// don't want to set this number too high.
pub static SCHEDULED_JOB_EXECUTION_PARALLELISM: LazyLock<usize> =
    LazyLock::new(|| env_config("SCHEDULED_JOB_EXECUTION_PARALLELISM", 10));

/// Initial backoff in milliseconds on a system error from a scheduled job.
pub static SCHEDULED_JOB_INITIAL_BACKOFF: LazyLock<Duration> =
    LazyLock::new(|| Duration::from_millis(env_config("SCHEDULED_JOB_INITIAL_BACKOFF_MS", 10)));

/// Max backoff in seconds on a system error from a scheduled job.
/// Scheduled jobs can hit many OCCs, so we may need to slow them down if they
/// hit errors repeatedly.
pub static SCHEDULED_JOB_MAX_BACKOFF: LazyLock<Duration> = LazyLock::new(|| {
    Duration::from_secs(env_config("SCHEDULED_JOB_MAX_BACKOFF_SECS", 2 * 60 * 60))
});

/// Initial backoff in milliseconds on a system error from the scheduled job
/// garbage collector.
pub static SCHEDULED_JOB_GARBAGE_COLLECTION_INITIAL_BACKOFF: LazyLock<Duration> =
    LazyLock::new(|| {
        Duration::from_millis(env_config(
            "SCHEDULED_JOB_GARBAGE_COLLECTION_INITIAL_BACKOFF_MS",
            10,
        ))
    });

/// Max backoff in seconds on a system error from the scheduled job garbage
/// collector.
pub static SCHEDULED_JOB_GARBAGE_COLLECTION_MAX_BACKOFF: LazyLock<Duration> = LazyLock::new(|| {
    Duration::from_secs(env_config(
        "SCHEDULED_JOB_GARBAGE_COLLECTION_MAX_BACKOFF_SECS",
        30,
    ))
});

/// How long completed scheduled jobs are kept before getting garbage collected.
pub static SCHEDULED_JOB_RETENTION: LazyLock<Duration> = LazyLock::new(|| {
    Duration::from_secs(env_config(
        "SCHEDULED_JOB_RETENTION",
        60 * 60 * 24 * 7, // 1 week
    ))
});

/// Maximum number of scheduled jobs to garbage collect in a single transaction
pub static SCHEDULED_JOB_GARBAGE_COLLECTION_BATCH_SIZE: LazyLock<usize> =
    LazyLock::new(|| env_config("SCHEDULED_JOB_GARBAGE_COLLECTION_BATCH_SIZE", 1000));

/// Maximum number of syscalls that can run in a batch together when
/// awaited in parallel. Higher values improve latency, while lower ones
/// protect one isolate from hogging database connections.
pub static MAX_SYSCALL_BATCH_SIZE: LazyLock<usize> =
    LazyLock::new(|| env_config("MAX_SYSCALL_BATCH_SIZE", 16));

/// Maximum depth of query/mutation -> query/mutation calls within the reactor.
/// We put a low limit on this for now so users with infinite loops won't starve
/// all of the threads on a single node.
pub static MAX_REACTOR_CALL_DEPTH: LazyLock<usize> =
    LazyLock::new(|| env_config("MAX_REACTOR_CALL_DEPTH", 8));

/// Number of rows that can be read in a transaction.
pub static TRANSACTION_MAX_READ_SIZE_ROWS: LazyLock<usize> =
    LazyLock::new(|| env_config("TRANSACTION_MAX_READ_SIZE_ROWS", 16384));

/// Number of bytes that can be read in a transaction.
pub static TRANSACTION_MAX_READ_SIZE_BYTES: LazyLock<usize> = LazyLock::new(|| {
    env_config("TRANSACTION_MAX_READ_SIZE_BYTES", 1 << 23) // 8 MiB
});

/// Maximum number of intervals that can be read in a transcation.
pub static TRANSACTION_MAX_READ_SET_INTERVALS: LazyLock<usize> =
    LazyLock::new(|| env_config("TRANSACTION_MAX_READ_SET_INTERVALS", 4096));

/// Write max_repeatable_ts if there have been no commits for this duration.
pub static MAX_REPEATABLE_TIMESTAMP_IDLE_FREQUENCY: LazyLock<Duration> = LazyLock::new(|| {
    Duration::from_secs(env_config(
        "MAX_REPEATABLE_TIMESTAMP_IDLE_FREQUENCY",
        100 * 60,
    ))
});

/// This is the max duration between a Commit and bumping max_repeatable_ts.
/// When reading from a follower persistence, we can only read commits at
/// timestamps <= max_repeatable_ts (because commits > max_repeatable_ts are
/// actively being written), so this is the delay between a commit and the
/// commit being visible from db-verifier and other follower reads.
pub static MAX_REPEATABLE_TIMESTAMP_COMMIT_DELAY: LazyLock<Duration> =
    LazyLock::new(|| Duration::from_secs(env_config("MAX_REPEATABLE_TIMESTAMP_COMMIT_DELAY", 5)));

/// The maximum delay between runs of retention, this is now only used for error
/// backoff and the initial delay when backend is started.
pub static MAX_RETENTION_DELAY_SECONDS: LazyLock<Duration> =
    LazyLock::new(|| Duration::from_secs(env_config("RETENTION_DELETE_FREQUENCY", 60)));

/// How many parallel threads to use for computing which index entries have
/// expired.
pub static RETENTION_READ_PARALLEL: LazyLock<usize> =
    LazyLock::new(|| env_config("RETENTION_READ_PARALLEL", 4));

/// How many parallel threads to use for deleting index entries that have
/// expired.
pub static INDEX_RETENTION_DELETE_PARALLEL: LazyLock<usize> =
    LazyLock::new(|| env_config("INDEX_RETENTION_DELETE_PARALLEL", 4));

/// How many parallel threads to use for deleting document log entries that have
/// expired.
pub static DOCUMENT_RETENTION_DELETE_PARALLEL: LazyLock<usize> =
    LazyLock::new(|| env_config("DOCUMENT_RETENTION_DELETE_PARALLEL", 1));

/// INDEX_RETENTION_DELAY determines the size of the index retention window.
///
/// Larger window means we keep around old snapshots for longer, which can cause
/// performance problems in UDFs if there are many tombstones.
///
/// Smaller window means we break snapshot reads faster.
pub static INDEX_RETENTION_DELAY: LazyLock<Duration> =
    LazyLock::new(|| Duration::from_secs(env_config("INDEX_RETENTION_DELAY", 4 * 60)));

/// DOCUMENT_RETENTION_DELAY determines the size of the document retention
/// window.
///
/// Larger window means we keep around longer windows for our write ahead log
/// to be valid.
///
/// Smaller window means we keep less historical data around.
pub static DOCUMENT_RETENTION_DELAY: LazyLock<Duration> = LazyLock::new(|| {
    Duration::from_secs(env_config("DOCUMENT_RETENTION_DELAY", 60 * 60 * 24 * 90))
});

/// Resets DocumentRetentionConfirmedDeletedTimestamp to Timestamp::MIN
pub static RESET_DOCUMENT_RETENTION: LazyLock<bool> =
    LazyLock::new(|| env_config("RESET_DOCUMENT_RETENTION", false));

/// The time backend should wait before it acquires the lease. This wait allows
/// for the backend to be added to service discovery, before it renders the
/// previous backends unusable.
///
/// Wait > 5 seconds before acquiring the backend lease, so we are added to
/// traefik before we make the old backend unusable.
pub static BACKEND_STARTUP_DELAY: LazyLock<Duration> = LazyLock::new(|| {
    Duration::from_secs(env_config(
        "BACKEND_STARTUP_DELAY_SECS",
        prod_override(0, 6),
    ))
});

/// When to start rejecting new additions to the search memory index.
pub static TEXT_INDEX_SIZE_HARD_LIMIT: LazyLock<usize> =
    LazyLock::new(|| env_config("SEARCH_INDEX_SIZE_HARD_LIMIT", 100 * (1 << 20))); // 100 MiB

/// When to start rejecting new additions to the vector memory index.
/// Because they're closely related, this is also used by the vector compaction
/// worker to determine the largest size for a "small" segment. Small segments
/// are merged more aggressively by the compaction worker than large segments.
pub static VECTOR_INDEX_SIZE_HARD_LIMIT: LazyLock<usize> =
    LazyLock::new(|| env_config("VECTOR_INDEX_SIZE_HARD_LIMIT", 100 * (1 << 20))); // 100 MiB

/// Whether indexes will be backfilled. Likely only disabled if index backfill
/// is breaking an instance.
pub static ENABLE_INDEX_BACKFILL: LazyLock<bool> =
    LazyLock::new(|| env_config("INDEX_BACKFILL_ENABLE", true));

/// Number of index chunks processed per second during a backfill.
pub static INDEX_BACKFILL_CHUNK_RATE: LazyLock<usize> =
    LazyLock::new(|| env_config("INDEX_BACKFILL_CHUNK_RATE", 8));

/// How many index entries to write within a single database transaction.
/// Value is a tradeoff between grouping work, vs tying up resources on the
/// database, vs holding all entries in memory.
pub static INDEX_BACKFILL_CHUNK_SIZE: LazyLock<usize> =
    LazyLock::new(|| env_config("INDEX_BACKFILL_CHUNK_SIZE", 256));

/// Chunk size of index entries when reading from persistence.
pub static RETENTION_READ_CHUNK: LazyLock<usize> =
    LazyLock::new(|| env_config("RETENTION_READ_CHUNK", 128));

/// Chunk size of index entries for deleting from Persistence.
pub static INDEX_RETENTION_DELETE_CHUNK: LazyLock<usize> =
    LazyLock::new(|| env_config("INDEX_RETENTION_DELETE_CHUNK", 512));

/// Chunk size of documents for deleting from Persistence.
pub static DOCUMENT_RETENTION_DELETE_CHUNK: LazyLock<usize> =
    LazyLock::new(|| env_config("DOCUMENT_RETENTION_DELETE_CHUNK", 128));

/// Batch size of index entries to delete between checkpoints.
pub static RETENTION_DELETE_BATCH: LazyLock<usize> =
    LazyLock::new(|| env_config("RETENTION_DELETE_BATCH", 10000));

/// Whether retention deletes are enabled.
pub static RETENTION_DELETES_ENABLED: LazyLock<bool> =
    LazyLock::new(|| env_config("RETENTION_DELETES_ENABLED", true));

/// Whether retention document deletes are enabled.
pub static RETENTION_DOCUMENT_DELETES_ENABLED: LazyLock<bool> =
    LazyLock::new(|| env_config("RETENTION_DOCUMENT_DELETES_ENABLED", true));

/// Enable or disable failing insert/update/deletes when retention is behind.
pub static RETENTION_FAIL_ENABLED: LazyLock<bool> =
    LazyLock::new(|| env_config("RETENTION_FAIL_ENABLED", false));

/// Insert/update/delete will start to fail if retention is retention window *
/// this value behind (e.g. 4 * 20 = 1 hour 20 minutes)
pub static RETENTION_FAIL_START_MULTIPLIER: LazyLock<usize> =
    LazyLock::new(|| env_config("RETENTION_FAIL_START_MULTIPLIER", 20));

/// All insert/update/deletes will if retention is retention window * this value
/// behind (e.g. 4 * 40 = 2 hours and 4 minutes).
pub static RETENTION_FAIL_ALL_MULTIPLIER: LazyLock<usize> =
    LazyLock::new(|| env_config("RETENTION_FAIL_ALL_MULTIPLIER", 40));

/// Time in between batches of deletes for document retention. This value is
/// also used to jitter document retention on startup to avoid a thundering
/// herd.
pub static DOCUMENT_RETENTION_BATCH_INTERVAL_SECONDS: LazyLock<Duration> = LazyLock::new(|| {
    Duration::from_secs(env_config(
        "DOCUMENT_RETENTION_BATCH_INTERVAL_SECONDS",
        2 * 60,
    ))
});

/// Maximum scanned documents within a single run for document retention unless
/// there are a bunch of writes at single timestamp. Then, we go until there are
/// no more writes at that timestamp.
pub static DOCUMENT_RETENTION_MAX_SCANNED_DOCUMENTS: LazyLock<usize> =
    LazyLock::new(|| env_config("DOCUMENT_RETENTION_MAX_SCANNED_DOCUMENTS", 5000));

/// Size at which a search index will be queued for snapshotting.
pub static SEARCH_INDEX_SIZE_SOFT_LIMIT: LazyLock<usize> =
    LazyLock::new(|| env_config("SEARCH_INDEX_SIZE_SOFT_LIMIT", 10 * (1 << 20))); // 10 MiB

/// Size at which a v1 single segment text search index will be queued for
/// snapshotting.
///
/// We use a larger value here because building single segment text search
/// indexes is exponential, so building indexes infrequently reduces the overall
/// time spent building the index.
pub static TEXT_SEARCH_V1_INDEX_SIZE_SOFT_LIMIT: LazyLock<usize> =
    LazyLock::new(|| env_config("SEARCH_INDEX_SIZE_SOFT_LIMIT", 50 * (1 << 20))); // 50 MiB
/// Configures the search index worker's rate limit on pages processed per
/// second.
pub static SEARCH_INDEX_WORKER_PAGES_PER_SECOND: LazyLock<NonZeroU32> = LazyLock::new(|| {
    env_config(
        "SEARCH_INDEX_WORKER_PAGES_PER_SECOND",
        NonZeroU32::new(2).unwrap(),
    )
});

/// Don't allow database workers to have more than an hour of uncheckpointed
/// data.
///
/// For search/vector index workers - Note that fast-forwarding will keep the
/// index's timestamp up-to-date if its table hasn't had any writes. This isn't
/// perfect since ideally we'd bound the number and total size of log entries
/// read for bootstrapping, but it's good enough until we have better commit
/// statistics that aren't reset at restart.
pub static DATABASE_WORKERS_MAX_CHECKPOINT_AGE: LazyLock<Duration> =
    LazyLock::new(|| Duration::from_secs(env_config("DATABASE_WORKERS_MAX_CHECKPOINT_AGE", 3600)));

/// Don't fast-forward an index less than ten seconds forward so we don't
/// amplify every commit into another write when the system is under heavy load.
pub static DATABASE_WORKERS_POLL_INTERVAL: LazyLock<Duration> =
    LazyLock::new(|| Duration::from_secs(env_config("DATABASE_WORKERS_POLL_INTERVAL", 20)));

/// The minimum time to retain the WriteLog, note that we will never retain for
/// less, even if WRITE_LOG_SOFT_MAX_SIZE_BYTES is exceeded.
pub static WRITE_LOG_MIN_RETENTION_SECS: LazyLock<Duration> =
    LazyLock::new(|| Duration::from_secs(env_config("WRITE_LOG_MIN_RETENTION_SECS", 30)));

/// The maximum time to retain the WriteLog, note that the WriteLog might be
/// trimmed sooner if it size exceeds WRITE_LOG_MAX_SIZE_BYTES.
pub static WRITE_LOG_MAX_RETENTION_SECS: LazyLock<Duration> =
    LazyLock::new(|| Duration::from_secs(env_config("WRITE_LOG_MAX_RETENTION_SECS", 300)));

/// The maximum size of the write log. Notes:
/// - the write log will be trimmed based on WRITE_LOG_MAX_RETENTION_SECS, and
/// thus it might never reach this size.
/// - the write log always retains at least WRITE_LOG_MIN_RETENTION_SECS, and
/// thus we could exceed this limit. The reason we do that is to allow some
/// minimum buffer for queries to refresh after execution.
pub static WRITE_LOG_SOFT_MAX_SIZE_BYTES: LazyLock<usize> =
    LazyLock::new(|| env_config("WRITE_LOG_SOFT_MAX_SIZE_BYTES", 50 * 1024 * 1024));

/// How frequently system tables are cleaned up.
pub static SYSTEM_TABLE_CLEANUP_FREQUENCY: LazyLock<Duration> = LazyLock::new(|| {
    Duration::from_secs(env_config(
        "SYSTEM_TABLE_CLEANUP_FREQUENCY_SECONDS",
        30 * 60,
    ))
});

/// Number of rows fetched and potentially deleted in a single transaction.
pub static SYSTEM_TABLE_CLEANUP_CHUNK_SIZE: LazyLock<usize> =
    LazyLock::new(|| env_config("SYSTEM_TABLE_CLEANUP_CHUNK_SIZE", 256));

/// Maximum number of rows deleted per second.
/// This should not exceed the maximum rate that retention can process
/// tombstones, which is about 300.
/// TODO(lee) increase this back to 128
pub static SYSTEM_TABLE_ROWS_PER_SECOND: LazyLock<NonZeroU32> = LazyLock::new(|| {
    env_config(
        "SYSTEM_TABLE_CLEANUP_ROWS_PER_SECOND",
        NonZeroU32::new(1).unwrap(),
    )
});

/// Default 6 months, which is approximately how often we deprecate npm
/// packages. If the npm package is deprecated, the client can't reconnect with
/// an outstanding mutation. We can potentially reduce this window by changing
/// clients to track how long they have been open and throw an alert after
/// too many days. See go/idempotent-mutations
/// Zero indicates that there is no maximum.
pub static MAX_SESSION_CLEANUP_DURATION: LazyLock<Option<Duration>> = LazyLock::new(|| {
    let hours = env_config("MAX_SESSION_CLEANUP_DURATION_HOURS", 2 * 7 * 24);
    if hours > 0 {
        Some(Duration::from_secs(60 * 60 * hours))
    } else {
        None
    }
});

/// Number of chunks processed per second when calculating table summaries.
pub static TABLE_SUMMARY_CHUNKS_PER_SECOND: LazyLock<NonZeroU32> = LazyLock::new(|| {
    env_config(
        "TABLE_SUMMARY_CHUNKS_PER_SECOND",
        NonZeroU32::new(1000).unwrap(),
    )
});
/// Size at which a vector index will be queued for snapshotting vector indexes.
pub static VECTOR_INDEX_SIZE_SOFT_LIMIT: LazyLock<usize> =
    LazyLock::new(|| env_config("VECTOR_INDEX_SIZE_SOFT_LIMIT", 30 * (1 << 20))); // 30 MiB

/// Max number of threads used to build a disk index.
pub static VECTOR_INDEX_THREADS: LazyLock<usize> =
    LazyLock::new(|| env_config("VECTOR_INDEX_THREADS", 4));

/// Configures the vector and search index workers' rate limit on pages
/// processed per second. This is the default rate limit for anything a user
/// might be waiting on. It's initialized high enough that it effectively does
/// not rate limit. We keep the knob so that in an emergency we can reduce it.
///
/// NOTE: This number is multiplied with DEFAULT_DOCUMENTS_PAGE_SIZE, do not
/// set this value to such a larg number that doing so will overflow!
pub static SEARCH_WORKER_PAGES_PER_SECOND: LazyLock<NonZeroU32> = LazyLock::new(|| {
    env_config(
        "SEARCH_WORKER_PAGES_PER_SECOND",
        NonZeroU32::new(1000).unwrap(),
    )
});

/// Configures the vector and search index workers' rate limit on pages
/// processed per second for non-user facing rebuilds (well mostly non-user
/// facing, a user facing backfill might get stuck behind one of these).
///
/// The default is low so that an inadvertent metadata change or a deliberate
/// index backfill does not cause a thundering herd.
///
/// NOTE: This number is multiplied with DEFAULT_DOCUMENTS_PAGE_SIZE, do not
/// set this value to such a larg number that doing so will overflow!
pub static SEARCH_WORKER_PASSIVE_PAGES_PER_SECOND: LazyLock<NonZeroU32> = LazyLock::new(|| {
    env_config(
        "SEARCH_WORKER_PASSIVE_PAGES_PER_SECOND",
        NonZeroU32::new(10).unwrap(),
    )
});

/// Default page size (in number of docuemnts) used when loading documents from
/// the database for building a vector index.
pub static VECTOR_INDEX_WORKER_PAGE_SIZE: LazyLock<usize> =
    LazyLock::new(|| env_config("VECTOR_INDEX_WORKER_PAGE_SIZE", 128));

/// Timeout on "user time" spent during a UDF.
pub static DATABASE_UDF_USER_TIMEOUT: LazyLock<Duration> =
    LazyLock::new(|| Duration::from_secs(env_config("DATABASE_UDF_USER_TIMEOUT_SECONDS", 1)));

/// Timeout on the "system time" during a UDF -- i.e. syscalls.
// The user limits are not very tight, which requires us to have a high
// syscall timeout. When the database is healthy, we should never have UDF
// run into SYSTEM_TIMEOUT. UDFs are allowed to do up to 4096 unique queries,
// which at 1.6ms average, will take 6.4 seconds to run in sequence. We should
// aim to lower the SYSTEM_TIMEOUT limit over time by adding real parallelism
// and adding limit on number of `awaits` which is lower than the number of
// queries.
pub static DATABASE_UDF_SYSTEM_TIMEOUT: LazyLock<Duration> =
    LazyLock::new(|| Duration::from_secs(env_config("DATABASE_UDF_SYSTEM_TIMEOUT_SECONDS", 15)));

/// Increasing the size of the queue helps us deal with bursty requests. This is
/// a CoDel queue [https://queue.acm.org/detail.cfm?id=2209336], which will
/// switch from FIFO to LIFO queue when overloaded, in order to process as much
/// as possible and avoid a congestion collapse. The primary downside of
/// increase this is memory usage from the UDF arguments.
pub static ISOLATE_QUEUE_SIZE: LazyLock<usize> =
    LazyLock::new(|| env_config("ISOLATE_QUEUE_SIZE", 2000));

/// The size of the pending commits in the committer queue. This is a FIFO
/// queue, so if the queue is too large, we run into a risk of all requests
/// waiting too long and no requests going through during overload. The size of
/// each commit request is also typically larger than a isolate request. For
/// time being, allow 128 slots, which is the maximum number of isolate threads
/// in any process.
pub static COMMITTER_QUEUE_SIZE: LazyLock<usize> =
    LazyLock::new(|| env_config("COMMITTER_QUEUE_SIZE", 128));

/// 0 -> default (number of cores)
pub static V8_THREADS: LazyLock<u32> = LazyLock::new(|| env_config("V8_THREADS", 0));

/// Cap on the number of active isolate execution threads.
pub static UDF_ISOLATE_MAX_EXEC_THREADS: LazyLock<usize> = LazyLock::new(|| {
    env_config(
        "UDF_ISOLATE_MAX_EXEC_THREADS",
        if cfg!(any(test, feature = "testing")) {
            2
        } else {
            16
        },
    )
});

/// If false, each UDF runs in its own isolate with its own heap.
/// If true, each UDF runs in the same isolate in its own context, sharing a
/// heap.
pub static REUSE_ISOLATES: LazyLock<bool> = LazyLock::new(|| env_config("REUSE_ISOLATES", true));

/// Duration in seconds before an idle isolate is recreated
pub static ISOLATE_IDLE_TIMEOUT: LazyLock<Duration> =
    LazyLock::new(|| Duration::from_secs(env_config("ISOLATE_IDLE_TIMEOUT_SECONDS", 600)));

/// The maximum amount of time an isolate can be used before being recreated.
pub static ISOLATE_MAX_LIFETIME: LazyLock<Duration> =
    LazyLock::new(|| Duration::from_secs(env_config("ISOLATE_MAX_LIFETIME_SECONDS", 60 * 60)));

/// System timeout for V8 actions.
/// This doesn't count most syscalls, but it does count module loading.
pub static V8_ACTION_SYSTEM_TIMEOUT: LazyLock<Duration> =
    LazyLock::new(|| Duration::from_secs(env_config("V8_ACTION_SYSTEM_TIMEOUT_SECONDS", 5 * 60)));

/// The maximum amount of time
pub static APPLICATION_FUNCTION_RUNNER_SEMAPHORE_TIMEOUT: LazyLock<Duration> =
    LazyLock::new(|| {
        Duration::from_millis(env_config(
            "APPLICATION_FUNCTION_RUNNER_SEMAPHORE_TIMEOUT",
            5000,
        ))
    });

/// The maximum number of queries that can be run concurrently by an
/// application.
///
/// This is a per backend limit applied before FunctionRunner implementations.
///
/// The value here may be overridden by big brain.
pub static APPLICATION_MAX_CONCURRENT_QUERIES: LazyLock<usize> =
    LazyLock::new(|| env_config("APPLICATION_MAX_CONCURRENT_QUERIES", 16));

/// The maximum number of mutations that can be run concurrently by an
/// application.
///
/// This is a per backend limit applied before FunctionRunner implementations.
///
/// The value here may be overridden by big brain.
pub static APPLICATION_MAX_CONCURRENT_MUTATIONS: LazyLock<usize> =
    LazyLock::new(|| env_config("APPLICATION_MAX_CONCURRENT_MUTATIONS", 16));

/// The maximum number of v8 actions that can be run concurrently by an
/// application.
///
/// This is a higher level limit applied before FunctionRunner implementations.
///
/// This does NOT apply to:
/// 1. Http actions
/// 2. Node actions
///
/// Node actions are limited by the APPLICATION_MAX_CONCURRENT_NODE_ACTIONS
/// knob. Http actions are limited by APPLICATION_MAX_CONCURRENT_HTTP_ACTIONS
/// knob.
///
/// The value here may be overridden by big brain.
pub static APPLICATION_MAX_CONCURRENT_V8_ACTIONS: LazyLock<usize> =
    LazyLock::new(|| env_config("APPLICATION_MAX_CONCURRENT_V8_ACTIONS", 16));

/// The maximum number of node actions that can be run concurrently by an
/// application
///
/// Node actions are not sent through FunctionRunner implementations, so this is
/// a limit on the number of actions sent to AWS. AWS also has a global maximum
/// number of total concurrent actions across all backends. If we hit the AWS
/// limit, we'll see 429 error responses for node actions.
///
/// The value here may be overridden by big brain.
pub static APPLICATION_MAX_CONCURRENT_NODE_ACTIONS: LazyLock<usize> =
    LazyLock::new(|| env_config("APPLICATION_MAX_CONCURRENT_NODE_ACTIONS", 16));

/// Number of threads to execute V8 actions.
///
/// Http actions are not sent through FunctionRunner implementations. This is a
/// maximum on the number of http actions that will be executed in process in a
/// particular backend.
///
/// The value here may be overridden by big brain.
pub static APPLICATION_MAX_CONCURRENT_HTTP_ACTIONS: LazyLock<usize> = LazyLock::new(|| {
    env_config(
        "APPLICATION_MAX_CONCURRENT_HTTP_ACTIONS",
        if cfg!(any(test, feature = "testing")) {
            2
        } else {
            16
        },
    )
});

/// Set a 64MB limit on the heap size.
pub static ISOLATE_MAX_USER_HEAP_SIZE: LazyLock<usize> =
    LazyLock::new(|| env_config("ISOLATE_MAX_USER_HEAP_SIZE", 1 << 26));

/// Allow for some objects to persist between contexts, not necessarily created
/// by the UDF.
pub static ISOLATE_MAX_HEAP_EXTRA_SIZE: LazyLock<usize> =
    LazyLock::new(|| env_config("ISOLATE_MAX_HEAP_EXTRA_SIZE", 1 << 25));

/// Chunk sizes: 1, 2, 3, ..., MAX_DYNAMIC_SMART_CHUNK_SIZE incrementing by 1.
/// These chunk sizes allow small (common) batches to be handled in a single
/// chunk, while limiting the size of a chunk (don't overload the db), and
/// keeping the number of distinct queries small (for query plan caching).
pub static MYSQL_MAX_DYNAMIC_SMART_CHUNK_SIZE: LazyLock<usize> =
    LazyLock::new(|| env_config("MYSQL_MAX_DYNAMIC_SMART_CHUNK_SIZE", 8));
/// More chunks sizes: 1, 2, 4, 8, 16, 32, ..., MYSQL_CHUNK_SIZE doubling.
/// Max packet size is 16MiB.
pub static MYSQL_MAX_CHUNK_BYTES: LazyLock<usize> =
    LazyLock::new(|| env_config("MYSQL_MAX_CHUNK_BYTES", 10 << 20));

/// Timeout for all operations on MySQL connections
pub static MYSQL_TIMEOUT: LazyLock<u64> = LazyLock::new(|| env_config("MYSQL_TIMEOUT_SECONDS", 30));

/// Maximum number of connections to MySQL
pub static MYSQL_MAX_CONNECTIONS: LazyLock<usize> =
    LazyLock::new(|| env_config("MYSQL_MAX_CONNECTIONS", 128));

/// Minimum number of rows to read from MySQL in a single query.
pub static MYSQL_MIN_QUERY_BATCH_SIZE: LazyLock<usize> =
    LazyLock::new(|| env_config("MYSQL_MIN_QUERY_BATCH_SIZE", 1));

/// Maximum number of rows to read from MySQL in a single query.
pub static MYSQL_MAX_QUERY_BATCH_SIZE: LazyLock<usize> =
    LazyLock::new(|| env_config("MYSQL_MAX_QUERY_BATCH_SIZE", 5000));

/// We dynamically increase the batch size up to this threshold if client keeps
/// fetching more results. This helps correct for tombstones, long prefixes and
/// wrong client size estimates.
pub static MYSQL_MAX_QUERY_DYNAMIC_BATCH_SIZE: LazyLock<usize> =
    LazyLock::new(|| env_config("MYSQL_MAX_QUERY_DYNAMIC_BATCH_SIZE", 8));

/// Close a connection after it has been idle for some time. RDS proxy closes
/// connections after idle_client_timeout in mysql.tf, which should be
/// configured to be higher than this.
pub static MYSQL_INACTIVE_CONNECTION_LIFETIME: LazyLock<Duration> = LazyLock::new(|| {
    Duration::from_secs(env_config("MYSQL_INACTIVE_CONNECTION_LIFETIME_SECS", 90))
});

/// Force recycles a database connections after this period. RDS proxy pins
/// connections if the SQL query which exceeded the 16384 byte limit. Having a
/// hard limit on connection lifetime helps us reduce pinning and improve
/// connection reuse.
pub static MYSQL_MAX_CONNECTION_LIFETIME: LazyLock<Duration> =
    LazyLock::new(|| Duration::from_secs(env_config("MYSQL_MAX_CONNECTION_LIFETIME_SECS", 600)));

/// How many rows we fetch for retention and prev rev fetches (used for
/// TableIterator)
pub static MYSQL_CHUNK_SIZE: LazyLock<usize> =
    LazyLock::new(|| env_config("MYSQL_CHUNK_SIZE", 128));

/// How many actions "ops" (e.g. syscalls) can execute concurrently.
pub static MAX_CONCURRENT_ACTION_OPS: LazyLock<usize> =
    LazyLock::new(|| env_config("MAX_CONCURRENT_ACTION_OPS", 8));

/// Maximum count of transitions within the web socket server message buffer.
/// When this limit is reached, the web socket worker will temporary stop
/// computing and sending transition messages to the client.
pub static SYNC_MAX_SEND_TRANSITION_COUNT: LazyLock<usize> =
    LazyLock::new(|| env_config("SYNC_MAX_SEND_TRANSITION_COUNT", 2));

/// Max Axiom sink attributes. This is a knob just in case a user actually hits
/// the limit but has an Enterprise Axiom plan that lets them use more than the
/// limit we've configured.
pub static AXIOM_MAX_ATTRIBUTES: LazyLock<usize> =
    LazyLock::new(|| env_config("AXIOM_MAX_ATTRIBUTES", 1024));

/// If a qdrant Segment's estimated byte is is <= this threshold, we'll build a
/// plain index without HNSW. The default value is 2.5x qdrants. We hope this is
/// ok due to payload filtering, compaction and aiming for 30mb indexes via
/// VECTOR_INDEX_SIZE_SOFT_LIMIT. This is > VECTOR_INDEX_SIZE_SOFT_LIMIT so that
/// have some wiggle room if we build a slightly larger than expected segment.
pub static MULTI_SEGMENT_FULL_SCAN_THRESHOLD_KB: LazyLock<usize> =
    LazyLock::new(|| env_config("MULTI_SEGMENT_FULL_SCAN_THRESHOLD_KB", 50_000));

/// The maximum size that we will compact any given set of segments to.
/// Default to using 3 segments per 1.1 million vectors.
pub static SEGMENT_MAX_SIZE_BYTES: LazyLock<u64> =
    LazyLock::new(|| env_config("SEGMENT_MAX_SIZE_BYTES", (1100000 * 2048 * 4) / 3_u64));
/// The minimum number of segments we will compact in one pass.
pub static MIN_COMPACTION_SEGMENTS: LazyLock<u64> =
    LazyLock::new(|| env_config("MIN_COMPACTION_SEGMENTS", 3));
/// The maximum percentage of a Segment that can be deleted before we will
/// recompact that segment to remove deleted vectors
/// This number must be between 0 and 1.
pub static MAX_SEGMENT_DELETED_PERCENTAGE: LazyLock<f64> =
    LazyLock::new(|| env_config("MAX_SEGMENT_DELETED_PERCENTAGE", 0.2));

/// Whether to run queries, mutations, and v8 actions in Funrun (true) or
/// InProcessFunctionRunner (false).
pub static UDF_USE_FUNRUN: LazyLock<bool> = LazyLock::new(|| env_config("UDF_USE_FUNRUN", true));

/// The amount of time to wait for the primary request to finish before starting
/// a second backup request when running a vector search.
pub static VECTOR_BACKUP_REQUEST_DELAY_MILLIS: LazyLock<Duration> =
    LazyLock::new(|| Duration::from_millis(env_config("VECTOR_BACKUP_REQUEST_DELAY_MILLIS", 30)));

/// Whether to use prepared statements or not in Persistence.
pub static DATABASE_USE_PREPARED_STATEMENTS: LazyLock<bool> =
    LazyLock::new(|| env_config("DATABASE_USE_PREPARED_STATEMENTS", false));

/// The amount of time to allow for downloading a single file in the archive
/// cache on searchlight before timing out. If a fetch times out, no progress is
/// made and a subsequent request for the same file will start from the
/// beginning. If this number is too low relative to our disk throughput,
/// archive sizes and concurrent fetches, it can cause congestion collapse when
/// searchlight is restarted.
pub static ARCHIVE_FETCH_TIMEOUT_SECONDS: LazyLock<Duration> =
    LazyLock::new(|| Duration::from_secs(env_config("ARCHIVE_FETCH_TIMEOUT_SECONDS", 150)));

/// The total number of modules across all versions that will be held in memory
/// at once.
pub static MODULE_CACHE_MAX_SIZE_BYTES: LazyLock<u64> =
    LazyLock::new(|| env_config("MODULE_CACHE_MAX_SIZE_BYTES", 250_000_000));

/// The maximum number of concurrent module fetches we'll allow.
pub static MODULE_CACHE_MAX_CONCURRENCY: LazyLock<usize> =
    LazyLock::new(|| env_config("MODULE_CACHE_MAX_CONCURRENCY", 10));

/// The maximum size of the in memory index cache in Funrun in bytes.
pub static FUNRUN_INDEX_CACHE_SIZE: LazyLock<u64> =
    LazyLock::new(|| env_config("FUNRUN_INDEX_CACHE_SIZE", 10_000_000)); // 10 MB

/// The maximum number of concurrent index cache requests in Funrun.
pub static FUNRUN_INDEX_CACHE_CONCURRENCY: LazyLock<usize> =
    LazyLock::new(|| env_config("FUNRUN_INDEX_CACHE_CONCURRENCY", 100));

/// The maximum size of the module cache in Funrun in bytes.
pub static FUNRUN_MODULE_CACHE_SIZE: LazyLock<u64> =
    LazyLock::new(|| env_config("FUNRUN_MODULE_CACHE_SIZE", 250_000_000));

/// The maximum number of concurrent module cache requests in Funrun.
pub static FUNRUN_MODULE_MAX_CONCURRENCY: LazyLock<usize> =
    LazyLock::new(|| env_config("FUNRUN_MODULE_MAX_CONCURRENCY", 100));

/// The maximum number of fetch clients Funrun would create.
pub static FUNRUN_FETCH_CLIENT_CACHE_SIZE: LazyLock<usize> =
    LazyLock::new(|| env_config("FUNRUN_FETCH_CLIENT_CACHE_SIZE", 100));

/// The maximum number of concurrent requests a single client can make to a
/// single Funrun server.
/// NOTE: When changing this value, ensure that the following parameters
/// for pro satisfy this inequality. This makes sure that if one funrun instance
/// is down, we still have enough capacity to satisfy requests if a backend is
/// maxing out its concurrent function call limits.
///
/// max_concurrent_queries +
/// max_concurrent_mutations + max_concurrent_v8_actions < (number of funrun
/// nodes - 1) * FUNRUN_CLIENT_MAX_REQUESTS_PER_UPSTREAM
pub static FUNRUN_CLIENT_MAX_REQUESTS_PER_UPSTREAM: LazyLock<usize> =
    LazyLock::new(|| env_config("FUNRUN_CLIENT_MAX_REQUESTS_PER_UPSTREAM", 15));

/// The maximum number of retries a Funrun client will perform. The client only
/// retries overloaded errors.
pub static FUNRUN_CLIENT_MAX_RETRIES: LazyLock<usize> =
    LazyLock::new(|| env_config("FUNRUN_CLIENT_MAX_RETRIES", 1));

/// Value between 1 and 100 representing the percent of a the shared scheduler's
/// worker pool that a single client can use.
pub static FUNRUN_SCHEDULER_MAX_PERCENT_PER_CLIENT: LazyLock<usize> =
    LazyLock::new(|| env_config("FUNRUN_SCHEDULER_MAX_PERCENT_PER_CLIENT", 50));

/// Name of the service to discover for when connecting to Funrun (e.g.
/// funrun-default, funrun-staging, etc.)
pub static FUNRUN_CLUSTER_NAME: LazyLock<String> =
    LazyLock::new(|| env_config("FUNRUN_CLUSTER_NAME", String::from("funrun-default")));

/// Name of the service to discover for when connecting to Searchlight. (e.g.
/// searchlight-default, searchlight-staging, etc.)
// cluster is created.
pub static SEARCHLIGHT_CLUSTER_NAME: LazyLock<String> = LazyLock::new(|| {
    env_config(
        "SEARCHLIGHT_CLUSTER_NAME",
        String::from("searchlight-default"),
    )
});

/// The maximum number of CPU cores that can be used simultaneously by the
/// isolates. Zero means no limit.
pub static FUNRUN_ISOLATE_ACTIVE_THREADS: LazyLock<usize> =
    LazyLock::new(|| env_config("FUNRUN_ISOLATE_ACTIVE_THREADS", 0));

/// What percentage of the physical CPU cores can be actively used by the
/// isolate.
///
/// Give 50% of physical cores to v8. Note that we are still oversubscribing
/// the CPU since we run multiple backends per server. This is fine since we
/// are moving js execution to Funrun.
pub static BACKEND_ISOLATE_ACTIVE_THREADS_PERCENT: LazyLock<usize> = LazyLock::new(|| {
    env_config(
        "BACKEND_ISOLATE_ACTIVE_THREADS_PERCENT",
        prod_override(100, 50),
    )
});

/// How long to splay deploying AWS Lambdas due to changes in the backend. This
/// know doesn't delay deploys that are required due to user backends.
pub static AWS_LAMBDA_DEPLOY_SPLAY_SECONDS: LazyLock<Duration> =
    LazyLock::new(|| Duration::from_secs(env_config("AWS_LAMBDA_DEPLOY_SPLAY_SECONDS", 300)));

/// The maximum number of requests to send using a single AWS Lambda client.
/// Empirical tests have shown that AWS servers allows up to 128 concurrent
/// streams over a single http2 connection.
pub static AWS_LAMBDA_CLIENT_MAX_CONCURRENT_REQUESTS: LazyLock<usize> =
    LazyLock::new(|| env_config("AWS_LAMBDA_MAX_CONCURRENT_STREAMS_PER_CONNECTION", 100));

/// The number of seconds backend should wait for requests to drain before
/// shutting down after SIGINT.
pub static BACKEND_REQUEST_DRAIN_TIMEOUT: LazyLock<Duration> =
    LazyLock::new(|| Duration::from_secs(env_config("BACKEND_REQUEST_DRAIN_TIMEOUT", 15)));

/// The kinesis firehose name for streaming usage metrics to the data
// large body of water.
pub static BACKEND_USAGE_FIREHOSE_NAME: LazyLock<Option<String>> = LazyLock::new(|| {
    let result = env_config(
        "BACKEND_USAGE_FIREHOSE_NAME",
        prod_override("", "cvx-firehose-usage-prod").to_string(),
    );
    if !result.is_empty() {
        Some(result.to_string())
    } else {
        None
    }
});

/// The number of events we can accumulate in the buffer that's used to send
/// events from our business logic to our firehose client.
///
/// We use try_send to send data over a channel, so it's possible we send faster
/// than the receiving thread can read the data. The buffer helps mitigate that.
///
/// Each item in the buffer is a Vec of events. If we assume that each event is
/// 100 bytes, each Vec might have 100 events and we keep 2000 of these, then
/// worst case our usage is ~100MB.
pub static FIREHOSE_BUFFER_SIZE_COUNT: LazyLock<usize> =
    LazyLock::new(|| env_config("FIREHOSE_BUFFER_SIZE_COUNT", 2000));

/// The maximum amount of data we'll populate in a single AWS Kinesis Firehose
/// Record.
///
/// Each Record in AWS has a maximum size of 1000 KiB. PutRecordBatch (which
/// we'll use if a single list of events exceeds this buffer size) has a maximum
/// limit of 4 MiB of records. This controls only the size of the Record, not
/// the size of the batch. We do not currently make any attempt to control the
/// batch size.
pub static FIREHOSE_MAX_BATCH_SIZE_BYTES: LazyLock<usize> =
    LazyLock::new(|| env_config("FIREHOSE_MAX_BATCH_SIZE_BYTES", 900 * 1024));

/// The amount of time we'll allow firehose data to sit in memory while it
/// remains under our buffer size. Once this timeout is exceeded for the oldest
/// buffered record, we'll send our entire buffer.
pub static FIREHOSE_TIMEOUT: LazyLock<Duration> =
    LazyLock::new(|| Duration::from_secs(env_config("FIREHOSE_TIMEOUT", 60)));

/// The initial backoff time for index workers when a failure occurs.
pub static INDEX_WORKERS_INITIAL_BACKOFF: LazyLock<Duration> =
    LazyLock::new(|| Duration::from_millis(env_config("INDEX_WORKERS_INITIAL_BACKOFF", 500)));

/// The maximum size for Funrun request messages. This is 8MiB for path and
/// args, plus a buffer for the smaller fields.
pub static MAX_FUNRUN_REQUEST_MESSAGE_SIZE: LazyLock<usize> =
    LazyLock::new(|| env_config("MAX_FUNRUN_REQUEST_MESSAGE_SIZE", (1 << 23) + 2000)); // 8 MiB + buffer

/// The maximum size for Funrun response messages. 8MiB for reads + 8 MiB for
/// writes + 8MiB for function result + 4MiB for log lines and a 4 MiB buffer
/// for the smaller fields.
pub static MAX_FUNRUN_RESPONSE_MESSAGE_SIZE: LazyLock<usize> =
    LazyLock::new(|| env_config("MAX_FUNRUN_RESPONSE_MESSAGE_SIZE", 1 << 25)); // 32 MiB

/// The maximum size for Backend HTTP and GRPC action callbacks. This is 8MiB
/// for path and args, plus a buffer for the smaller fields This should also be
/// enough for vector and text search callbacks.
pub static MAX_BACKEND_RPC_REQUEST_SIZE: LazyLock<usize> =
    LazyLock::new(|| env_config("MAX_BACKEND_RPC_REQUEST_SIZE", (1 << 23) + 2000)); // 8 MiB + buffer

/// The maximum size for Backend HTTP and GRPC action callbacks. This is 8MiB
/// for function result, plus a buffer for the smaller fields.
pub static MAX_BACKEND_RPC_RESPONSE_SIZE: LazyLock<usize> =
    LazyLock::new(|| env_config("MAX_BACKEND_RPC_RESPONSE_SIZE", (1 << 23) + 2000)); // 8 MiB + buffer

/// The maximum size of byte chunks used when transmitting HTTP request/response
/// bodies as part of HTTP Actions.
pub static MAX_BACKEND_RPC_HTTP_CHUNK_SIZE: LazyLock<usize> =
    LazyLock::new(|| env_config("MAX_BACKEND_RPC_RESPONSE_SIZE", 1 << 23)); // 8 MiB

/// The maximum size for requests to the backend public API. Must be at least 8
/// MiB for function arguments.
pub static MAX_BACKEND_PUBLIC_API_REQUEST_SIZE: LazyLock<usize> =
    LazyLock::new(|| env_config("MAX_BACKEND_PUBLIC_API_REQUEST_SIZE", (1 << 23) + 2000)); // 8 MiB

/// Background database workers wake up periodically, check to see if something
/// has changed, then either go back to sleep or do work. Most workers determine
/// if something has changed at least in part by comparing the number of commits
/// since the last time they woke up. This works perfectly if there's exactly
/// one worker. But we actually have N workers, so if each of them wakes up and
/// does work whenever there's a new commit, they're all constantly doing work
/// because of each other.
///
/// So a simple way to avoid this is to require there to at least N commits
/// rather than >=1 commit. As long as N is greater than the number of workers,
/// each worker will wake up, commit once, then go back to sleep until something
/// other than the other workers happens.
///
/// We must also ensure that workers advance periodically to ensure that we can
/// run document retention in the future. Database times are bumped periodically
/// even if no writes occur. So any worker that checks this should always have
/// some maximum period of time after which they checkpoint unconditionally.
pub static DATABASE_WORKERS_MIN_COMMITS: LazyLock<usize> =
    LazyLock::new(|| env_config("DATABASE_WORKERS_MIN_COMMITS", 100));

/// HTTP requests to backend will time out after this duration has passed.
///
/// See https://docs.rs/tower-http/0.5.0/tower_http/timeout/struct.TimeoutLayer.html
pub static HTTP_SERVER_TIMEOUT_DURATION: LazyLock<Duration> =
    LazyLock::new(|| Duration::from_secs(env_config("HTTP_SERVER_TIMEOUT_SECONDS", 300)));

/// The limit on the request size to /push_config.
// Schema and code bundle pushes must be less than this.
pub static MAX_PUSH_BYTES: LazyLock<usize> =
    LazyLock::new(|| env_config("MAX_PUSH_BYTES", 100_000_000));

/// Percentage of request traces that should sampled.
/// Can set a global value, as well as per-route values
///
/// Note that the regexes can't contain commas.
///
/// Enable sampling for 10% of /api/push_config and 0.001% of all requests
/// Use knobs to enable to higher limits for individual instances.
///
/// You can also enable sampling for individual instance_names if applicable,
/// e.g. in Usher.
///
///
/// Examples:
///   REQUEST_TRACE_SAMPLE_CONFIG=0.01
///   REQUEST_TRACE_SAMPLE_CONFIG=/route1=0.50,0.01
///   REQUEST_TRACE_SAMPLE_CONFIG=/route1=0.50,route2=0.50,0.01
///   REQUEST_TRACE_SAMPLE_CONFIG=/http/.*=0.50
///
///   REQUEST_TRACE_SAMPLE_CONFIG=carnitas:/route1=0.5,alpastor:1.0,0.01
///   This configures:
///   - sampling for "/route1" for instance name "carnitas" to 0.5
///   - sampling for all methods for instance name "alpastor" to 1.0
///   - sampling for anything else to 0.01

pub static REQUEST_TRACE_SAMPLE_CONFIG: LazyLock<SamplingConfig> = LazyLock::new(|| {
    env_config(
        "REQUEST_TRACE_SAMPLE_CONFIG",
        prod_override(
            SamplingConfig::default(),
            "/api/push_config=0.1,0.00001".parse().unwrap(),
        ),
    )
});

/// If true, the backend will check the rate limiter service for capacity under
/// the "backend_startup" domain keyed by db cluster name.
pub static STARTUP_RATE_LIMIT_ENABLED: LazyLock<bool> =
    LazyLock::new(|| env_config("STARTUP_RATE_LIMIT_ENABLED", false));

/// Size of the cache for access token authentication
pub static AUTH_CACHE_SIZE: LazyLock<usize> = LazyLock::new(|| env_config("AUTH_CACHE_SIZE", 1000));

/// Length of time an entry to the access authentication cache is valid
pub static AUTH_CACHE_TTL_SECONDS: LazyLock<u64> =
    LazyLock::new(|| env_config("AUTH_CACHE_TTL_SECONDS", 60));

/// Request body limit for airbyte streaming import requests
pub static AIRBYTE_STREAMING_IMPORT_REQUEST_SIZE_LIMIT: LazyLock<usize> = LazyLock::new(|| {
    env_config(
        "AIRBYTE_STREAMING_IMPORT_REQUEST_SIZE_LIMIT",
        10 * (2 << 20),
    )
});

/// The maximum number of backends to keep open connections to.
pub static USHER_BACKEND_CLIENTS_CACHE_SIZE: LazyLock<u64> =
    LazyLock::new(|| env_config("USHER_BACKEND_CLIENTS_CACHE_SIZE", 5000));

/// The maximum number of concurrent streams over a single tonic channel.
/// Providing a limit helps us not run into any implementation limits.
pub static USHER_MAX_CONCURRENT_STREAMS_PER_CHANNEL: LazyLock<usize> =
    LazyLock::new(|| env_config("USHER_MAX_CONCURRENT_STREAMS_PER_CHANNEL", 500));

/// Batch size for migration that rewrites virtual tables.
pub static MIGRATION_REWRITE_BATCH_SIZE: LazyLock<usize> =
    LazyLock::new(|| env_config("MIGRATION_REWRITE_BATCH_SIZE", 100));

/// Fraction that represents the percentage of HTTP actions to execute in FunRun
pub static EXECUTE_HTTP_ACTIONS_IN_FUNRUN: LazyLock<f64> =
    LazyLock::new(|| env_config("EXECUTE_HTTP_ACTIONS_IN_FUNRUN", 0.0));

/// If an import is taking longer than a day, it's a problem (and our fault).
/// But the customer is probably no longer waiting so we should fail the import.
/// If an import takes more than a week, the file may be deleted from S3.
pub static MAX_IMPORT_AGE: LazyLock<Duration> =
    LazyLock::new(|| Duration::from_secs(env_config("MAX_IMPORT_AGE_SECONDS", 7 * 24 * 60 * 60)));

//! Hand-curated classification of knobs into the three buckets the
//! dashboard cares about. Anything not in `CURATED` or `TIER_TUNED` is
//! `Advanced` by default. Keep this file in sync with the curated table in
//! `docs/superpowers/specs/2026-05-20-project-backend-knobs-design.md`.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Exposure {
    Curated,
    TierTuned,
    Advanced,
}

/// Curated knobs surfaced in the main dialog with friendly names + descriptions.
/// Order here is the order rows render.
pub const CURATED: &[(&str, &str)] = &[
    ("ACTIONS_USER_TIMEOUT_SECS", "Action timeout"),
    ("DOCUMENT_RETENTION_DELAY", "Document retention"),
    ("MAX_TRANSACTION_WINDOW_SECONDS", "Transaction window"),
    ("TRANSACTION_MAX_NUM_USER_WRITES", "Max writes per txn"),
    ("TRANSACTION_MAX_USER_WRITE_SIZE_BYTES", "Max write bytes per txn"),
    ("TRANSACTION_MAX_NUM_SCHEDULED", "Max scheduled per txn"),
    ("FUNCTION_MAX_ARGS_SIZE", "Max function args size"),
    ("FUNCTION_MAX_RESULT_SIZE", "Max function result size"),
    ("DEFAULT_DOCUMENTS_PAGE_SIZE", "Default page size"),
    ("UDF_EXECUTOR_OCC_MAX_RETRIES", "OCC retry budget"),
    ("MAX_SCHEDULED_JOB_ARGUMENT_SIZE_BYTES", "Max scheduled arg size"),
    ("AUDIT_LOG_MAX_TOTAL_SIZE_BYTES", "Audit log size cap"),
    ("UDF_USE_FUNRUN", "Use funrun isolate runner"),
    ("FUNRUN_MAX_ISOLATE_WORKERS", "Max isolate workers"),
    ("FUNRUN_TARGET_CPU_USAGE", "Funrun target CPU usage"),
];

/// Knobs the tier ladder sets defaults for. Surfaced in Advanced as
/// "Tier-tuned" so users see the source of the current value.
///
/// Note: `FUNRUN_MAX_ISOLATE_WORKERS` is intentionally promoted to
/// `CURATED` (above) — the tier ladder still owns its default, but it's
/// useful enough to put in the main dialog. `classify` checks `CURATED`
/// first, so listing it here too would be dead code; we omit it to keep
/// the table honest.
pub const TIER_TUNED: &[&str] = &[
    "RUNTIME_WORKER_THREADS",
    "UDF_CACHE_MAX_SIZE",
    "FUNRUN_INDEX_CACHE_SIZE",
    "FUNRUN_MODULE_CACHE_SIZE",
    "FUNRUN_CODE_CACHE_SIZE",
    "HTTP_SERVER_TCP_BACKLOG",
    "HTTP_SERVER_MAX_CONCURRENT_REQUESTS",
    "APPLICATION_MAX_CONCURRENT_QUERIES",
    "APPLICATION_MAX_CONCURRENT_MUTATIONS",
    "APPLICATION_MAX_CONCURRENT_V8_ACTIONS",
    "APPLICATION_MAX_CONCURRENT_NODE_ACTIONS",
    "MAX_CONCURRENT_ACTION_OPS",
    "COMMITTER_QUEUE_SIZE",
    "MAX_BYTES_WRITTEN_PER_SECOND",
    "POSTGRES_MAX_CONNECTIONS",
];

pub fn classify(env_var: &str) -> Exposure {
    if CURATED.iter().any(|(v, _)| *v == env_var) {
        Exposure::Curated
    } else if TIER_TUNED.contains(&env_var) {
        Exposure::TierTuned
    } else {
        Exposure::Advanced
    }
}

pub fn curated_display_name(env_var: &str) -> Option<&'static str> {
    CURATED.iter().find(|(v, _)| *v == env_var).map(|(_, d)| *d)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classification_buckets() {
        assert_eq!(classify("ACTIONS_USER_TIMEOUT_SECS"), Exposure::Curated);
        assert_eq!(classify("UDF_CACHE_MAX_SIZE"), Exposure::TierTuned);
        assert_eq!(
            classify("APPLICATION_MAX_CONCURRENT_MUTATIONS"),
            Exposure::TierTuned
        );
        assert_eq!(classify("COMMITTER_QUEUE_SIZE"), Exposure::TierTuned);
        assert_eq!(
            classify("MAX_BYTES_WRITTEN_PER_SECOND"),
            Exposure::TierTuned
        );
        assert_eq!(classify("POSTGRES_MAX_CONNECTIONS"), Exposure::TierTuned);
        assert_eq!(classify("DOCUMENT_DELTAS_LIMIT"), Exposure::Advanced);
    }

    #[test]
    fn curated_display_names() {
        assert_eq!(
            curated_display_name("ACTIONS_USER_TIMEOUT_SECS"),
            Some("Action timeout"),
        );
        assert_eq!(curated_display_name("NOT_CURATED"), None);
    }
}

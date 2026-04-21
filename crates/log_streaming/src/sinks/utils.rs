use std::sync::{
    atomic::{
        AtomicU64,
        Ordering,
    },
    Arc,
};

use common::log_streaming::{
    LogEvent,
    StructuredLogEvent,
};

/// Shared counter for total log stream egress bytes across all sinks.
/// Sinks atomically increment this; a periodic task drains it and emits
/// a `LogStreamEgress` event.
pub type EgressCounter = Arc<AtomicU64>;

/// This is the log event filter used by Sentry and future exception sinks.
/// Exception sinks don't receive _verification events or any other events for
/// now.
pub fn only_exceptions_log_filter(event: &LogEvent) -> bool {
    matches!(event.event, StructuredLogEvent::Exception { .. })
}

/// This is the log event filter used by the Axiom, Datadog, and Webhook log
/// sinks, the current sinks which are meant to receive non-exception events.
///
/// This uses a match statement without the catch-all `_` pattern to avoid
/// new log topics being added from being automatically routed to this filter.
/// New topics must be manually added to the first match arm.
pub fn default_log_filter(event: &LogEvent) -> bool {
    match event.event {
        StructuredLogEvent::Verification
        | StructuredLogEvent::Console { .. }
        | StructuredLogEvent::FunctionExecution { .. }
        | StructuredLogEvent::DeploymentAuditLog { .. }
        | StructuredLogEvent::SchedulerStats { .. }
        | StructuredLogEvent::ScheduledJobLag { .. }
        | StructuredLogEvent::CurrentStorageUsage { .. }
        | StructuredLogEvent::ConcurrencyStats { .. }
        | StructuredLogEvent::StorageApiBandwidth { .. }
        | StructuredLogEvent::LogStreamEgress { .. }
        | StructuredLogEvent::CustomAudit { .. } => true,
        StructuredLogEvent::Exception { .. } => false,
    }
}

pub fn build_event_batches(
    events: Vec<Arc<LogEvent>>,
    batch_size: usize,
    filter: fn(&LogEvent) -> bool,
) -> Vec<Vec<Arc<LogEvent>>> {
    events
        .into_iter()
        .filter(|ev| filter(ev))
        .collect::<Vec<_>>()
        .chunks(batch_size)
        .map(|arr: &[Arc<LogEvent>]| arr.to_vec())
        .collect()
}

pub fn batch_has_non_egress_events(events: &[Arc<LogEvent>]) -> bool {
    events
        .iter()
        .any(|ev| !matches!(ev.event, StructuredLogEvent::LogStreamEgress { .. }))
}

/// Accumulates log sink network egress bytes. Called by sinks after each
/// successful HTTP send. The accumulated bytes are periodically drained by
/// `LogManager::egress_emission_worker` which emits both a `LogStreamEgress`
/// event and a billing `track_call`.
///
/// Note: This should only be called for non-verification requests.
pub fn track_log_sink_bandwidth(
    num_bytes_egress: u64,
    egress_counter: &EgressCounter,
    metrics_fn: impl FnOnce(u64),
) {
    metrics_fn(num_bytes_egress);
    egress_counter.fetch_add(num_bytes_egress, Ordering::Relaxed);
}

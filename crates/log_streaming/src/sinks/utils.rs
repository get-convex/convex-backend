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
        | StructuredLogEvent::LogStreamEgress { .. } => true,
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

#[cfg(test)]
mod tests {
    use std::{
        assert_matches,
        sync::Arc,
    };

    use common::{
        errors::JsError,
        log_lines::{
            LogLevel,
            LogLineStructured,
        },
        log_streaming::{
            FunctionEventSource,
            LogEvent,
            StructuredLogEvent,
        },
        runtime::{
            testing::TestRuntime,
            Runtime,
        },
    };

    use crate::sinks::utils::{
        build_event_batches,
        default_log_filter,
        only_exceptions_log_filter,
    };

    #[convex_macro::test_runtime]
    async fn event_batching_even(rt: TestRuntime) -> anyhow::Result<()> {
        let mut events = vec![];
        for i in 0..30 {
            events.push(Arc::new(LogEvent {
                timestamp: rt.unix_timestamp(),
                event: StructuredLogEvent::Console {
                    source: FunctionEventSource::new_for_test(),
                    log_line: LogLineStructured::new_developer_log_line(
                        LogLevel::Log,
                        vec![format!("log {i}")],
                        rt.unix_timestamp(),
                    ),
                },
            }));
        }

        let batches = build_event_batches(events, 10, default_log_filter);
        assert_eq!(batches.len(), 3);
        assert_eq!(batches[0].len(), 10);
        assert_eq!(batches[1].len(), 10);
        assert_eq!(batches[2].len(), 10);

        Ok(())
    }

    #[convex_macro::test_runtime]
    async fn event_batching_remainder(rt: TestRuntime) -> anyhow::Result<()> {
        let mut events = vec![];
        for i in 0..23 {
            events.push(Arc::new(LogEvent {
                timestamp: rt.unix_timestamp(),
                event: StructuredLogEvent::Console {
                    source: FunctionEventSource::new_for_test(),
                    log_line: LogLineStructured::new_developer_log_line(
                        LogLevel::Log,
                        vec![format!("log {i}")],
                        rt.unix_timestamp(),
                    ),
                },
            }));
        }

        let batches = build_event_batches(events, 10, default_log_filter);
        assert_eq!(batches.len(), 3);
        assert_eq!(batches[0].len(), 10);
        assert_eq!(batches[1].len(), 10);
        assert_eq!(batches[2].len(), 3);

        Ok(())
    }

    #[convex_macro::test_runtime]
    async fn event_batching_filter(rt: TestRuntime) -> anyhow::Result<()> {
        let mut events = vec![];
        for i in 0..30 {
            events.push(Arc::new(LogEvent {
                timestamp: rt.unix_timestamp(),
                event: StructuredLogEvent::Console {
                    source: FunctionEventSource::new_for_test(),
                    log_line: LogLineStructured::new_developer_log_line(
                        LogLevel::Log,
                        vec![format!("log {i}")],
                        rt.unix_timestamp(),
                    ),
                },
            }));
            events.push(Arc::new(LogEvent {
                timestamp: rt.unix_timestamp(),
                event: StructuredLogEvent::Exception {
                    error: JsError::from_message(format!("error {i}")),
                    user_identifier: None,
                    source: FunctionEventSource::new_for_test(),
                    udf_server_version: None,
                },
            }));
        }

        let batches = build_event_batches(events, 10, only_exceptions_log_filter);
        assert_eq!(batches.len(), 3);
        assert_eq!(batches[0].len(), 10);
        assert_eq!(batches[1].len(), 10);
        assert_eq!(batches[2].len(), 10);
        assert_matches!(batches[0][0].event, StructuredLogEvent::Exception { .. });

        Ok(())
    }
}

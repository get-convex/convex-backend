use std::sync::Arc;

use common::log_streaming::{
    LogEvent,
    StructuredLogEvent,
};

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
        | StructuredLogEvent::ConcurrencyStats { .. } => true,
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

/// Helper function to track bandwidth usage for log sinks.
/// This consolidates the common logic for tracking network egress across
/// Axiom, Datadog, and Webhook sinks.
///
/// Note: This should only be called for non-verification requests.
/// Verification requests should be filtered out by the caller.
pub async fn track_log_sink_bandwidth(
    num_bytes_egress: u64,
    url_label: String,
    execution_id: common::execution_context::ExecutionId,
    request_id: &common::RequestId,
    usage_counter: &usage_tracking::UsageCounter,
    metrics_fn: impl FnOnce(u64),
) {
    metrics_fn(num_bytes_egress);

    // Track fetch egress
    let usage_tracker = usage_tracking::FunctionUsageTracker::new();
    usage_tracker.track_fetch_egress(url_label, num_bytes_egress);

    // Report usage via track_call
    let stats = usage_tracker.gather_user_stats();
    usage_counter
        .track_call(
            common::types::UdfIdentifier::SystemJob("log_stream_payload".to_string()),
            execution_id,
            request_id.clone(),
            usage_tracking::CallType::LogStreamPayload,
            true,
            stats,
        )
        .await;
}

/// Test helper to verify bandwidth tracking events.
/// Asserts that exactly 2 events (FunctionCall + NetworkBandwidth) were
/// captured, and that the NetworkBandwidth event has the correct egress size
/// and URL.
#[cfg(test)]
pub fn assert_bandwidth_events(
    events: Vec<events::usage::UsageEvent>,
    actual_request_size: u64,
    expected_url: &str,
) {
    use events::usage::UsageEvent;

    assert!(!events.is_empty(), "Expected usage events to be recorded");

    // Should have exactly 2 events: FunctionCall + NetworkBandwidth
    assert_eq!(
        events.len(),
        2,
        "Expected exactly 2 events (FunctionCall + NetworkBandwidth), got: {events:?}",
    );

    // Verify we have one FunctionCall event
    let function_calls: Vec<_> = events
        .iter()
        .filter(|e| matches!(e, UsageEvent::FunctionCall { .. }))
        .collect();
    assert_eq!(
        function_calls.len(),
        1,
        "Expected exactly 1 FunctionCall event"
    );

    // Verify we have one NetworkBandwidth event with correct size
    let bandwidth_events: Vec<_> = events
        .iter()
        .filter_map(|e| match e {
            UsageEvent::NetworkBandwidth { egress, url, .. } => Some((*egress, url.clone())),
            _ => None,
        })
        .collect();

    assert_eq!(
        bandwidth_events.len(),
        1,
        "Expected exactly 1 NetworkBandwidth event"
    );

    assert!(
        actual_request_size > 0,
        "Expected actual request size to be non-zero"
    );

    let (egress, url) = &bandwidth_events[0];
    assert_eq!(
        *egress, actual_request_size,
        "Expected egress bytes ({egress}) to match actual request size ({actual_request_size})",
    );
    assert_eq!(
        url, expected_url,
        "Expected URL to be '{expected_url}', got '{url}'",
    );
}

#[cfg(test)]
mod tests {
    use std::{
        assert_matches::assert_matches,
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

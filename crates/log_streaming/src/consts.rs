use std::time::Duration;

// LogManager
pub const MANAGER_INITIAL_BACKOFF: Duration = Duration::from_secs(1);
pub const MANAGER_MAX_BACKOFF: Duration = Duration::from_secs(30);
pub const SINK_STARTUP_TIMEOUT: Duration = Duration::from_secs(15);

// MockSink
#[cfg(any(test, feature = "testing"))]
pub const MOCK_SINK_EVENTS_BUFFER_SIZE: usize = 5;
#[cfg(any(test, feature = "testing"))]
pub const MOCK_SINK_INITIAL_BACKOFF: Duration = Duration::from_secs(1);
#[cfg(any(test, feature = "testing"))]
pub const MOCK_SINK_MAX_BACKOFF: Duration = Duration::from_secs(10);

// LocalSink
pub const LOCAL_SINK_EVENTS_BUFFER_SIZE: usize = 50;
pub const LOCAL_SINK_INITIAL_BACKOFF: Duration = Duration::from_secs(1);
pub const LOCAL_SINK_MAX_BACKOFF: Duration = Duration::from_secs(10);

// Datadog
pub const DD_SINK_EVENTS_BUFFER_SIZE: usize = 8;
pub const DD_SINK_INITIAL_BACKOFF: Duration = Duration::from_millis(500);
pub const DD_SINK_MAX_BACKOFF: Duration = Duration::from_secs(60);
pub const DD_SINK_MAX_REQUEST_ATTEMPTS: usize = 6;
/// We currently limit logs to `MAX_LOG_LINE_LENGTH` (4kb) and Datadog has a 200
/// log and 1MB limit per batch. Thus, this batch size makes the payload size
/// 800kb so we still have 200kb legroom per batch for system fields.
/// https://docs.datadoghq.com/agent/logs/log_transport/?tab=https
pub const DD_SINK_MAX_LOGS_PER_BATCH: usize = 200;

// Axiom
pub const AXIOM_SINK_EVENTS_BUFFER_SIZE: usize = 8;
pub const AXIOM_SINK_INITIAL_BACKOFF: Duration = Duration::from_millis(500);
pub const AXIOM_SINK_MAX_BACKOFF: Duration = Duration::from_secs(60);
pub const AXIOM_SINK_MAX_REQUEST_ATTEMPTS: usize = 6;
/// This is Axiom's hard limit: https://axiom.co/docs/send-data/ingest#limits
/// In practice, this is impossible to hit in one batch since the LogManager
/// aggregation recv buffer size (LOG_MANAGER_EVENT_RECV_BUFFER_SIZE) is
/// controlled by a knob which, by default, is much less than this.
pub const AXIOM_SINK_MAX_LOGS_PER_BATCH: usize = 10000;

// Webhook
pub const WEBHOOK_SINK_EVENTS_BUFFER_SIZE: usize = 8;
pub const WEBHOOK_SINK_INITIAL_BACKOFF: Duration = Duration::from_secs(1);
pub const WEBHOOK_SINK_MAX_BACKOFF: Duration = Duration::from_secs(60);
pub const WEBHOOK_SINK_MAX_REQUEST_ATTEMPTS: usize = 6;
pub const WEBHOOK_SINK_MAX_LOGS_PER_BATCH: usize = 128;

// Sentry
pub const SENTRY_SINK_EVENTS_BUFFER_SIZE: usize = 8;
pub const SENTRY_SINK_INITIAL_BACKOFF: Duration = Duration::from_millis(500);
pub const SENTRY_SINK_MAX_BACKOFF: Duration = Duration::from_secs(60);
pub const SENTRY_SINK_MAX_LOGS_PER_BATCH: usize = 100;

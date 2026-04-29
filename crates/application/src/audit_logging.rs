use std::sync::{
    atomic::{
        AtomicBool,
        Ordering,
    },
    Arc,
};

use common::{
    audit_log_lines::ResolvedAuditLogLines,
    log_streaming::{
        LogEvent,
        LogSender,
        StructuredLogEvent,
    },
};
use log_streaming::LogManagerClient;

/// AuditLogClient implementation that forwards audit logs to log streams.
#[derive(Clone)]
pub struct AuditLogClient {
    log_stream_client: LogManagerClient,
    is_dev_deployment: Arc<AtomicBool>,
}

impl AuditLogClient {
    pub fn new(log_stream_client: LogManagerClient, is_dev_deployment: bool) -> Self {
        let is_dev_deployment = Arc::new(AtomicBool::new(is_dev_deployment));
        Self {
            log_stream_client,
            is_dev_deployment,
        }
    }

    pub fn include_in_log_streams(&self) -> bool {
        // Only include in log streams on dev deployments
        // TODO: return false if a firehose stream is configured
        self.is_dev_deployment.load(Ordering::Relaxed)
    }

    pub fn set_is_dev_deployment(&self, is_dev_deployment: bool) {
        self.is_dev_deployment
            .store(is_dev_deployment, Ordering::Relaxed)
    }

    fn send_to_log_streams(
        &self,
        ResolvedAuditLogLines { logs, timestamp }: ResolvedAuditLogLines,
    ) {
        let events = logs
            .into_iter()
            .map(|b| LogEvent {
                timestamp,
                event: StructuredLogEvent::CustomAudit {
                    body: b.into_value(),
                },
            })
            .collect();
        self.log_stream_client.send_logs(events);
    }

    pub fn send_logs(&self, logs: ResolvedAuditLogLines) {
        if self.include_in_log_streams() {
            self.send_to_log_streams(logs);
        }
        // TODO: send logs to firehose
    }
}

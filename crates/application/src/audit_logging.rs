use std::sync::{
    atomic::{
        AtomicBool,
        Ordering,
    },
    Arc,
};

use aws_sdk_firehose::{
    primitives::Blob,
    types::Record,
};
use aws_utils::firehose::firehose_client;
use common::{
    audit_log_lines::ResolvedAuditLogLines,
    log_streaming::{
        LogEvent,
        LogSender,
        StructuredLogEvent,
    },
};
use errors::ErrorMetadata;
use log_streaming::LogManagerClient;

/// AuditLogClient implementation that forwards audit logs to log streams.
#[derive(Clone)]
pub struct AuditLogClient {
    log_stream_client: LogManagerClient,
    is_dev_deployment: Arc<AtomicBool>,
    firehose_client: Option<Arc<AuditLogFirehoseClient>>,
}

impl AuditLogClient {
    pub async fn new(
        log_stream_client: LogManagerClient,
        is_dev_deployment: bool,
        firehose_stream_name: Option<String>,
        deployment_name: &String,
    ) -> anyhow::Result<Self> {
        let is_dev_deployment = Arc::new(AtomicBool::new(is_dev_deployment));
        let firehose_client = if let Some(firehose_name) = firehose_stream_name {
            let prefix = format!("customer-audit-logs-{deployment_name}");
            anyhow::ensure!(
                firehose_name.starts_with(&prefix),
                format!(
                    "Expected audit log firehose stream name to start with \"{prefix}\" but got \
                     {firehose_name}"
                )
            );
            Some(Arc::new(AuditLogFirehoseClient::new(firehose_name).await?))
        } else {
            None
        };
        Ok(Self {
            log_stream_client,
            is_dev_deployment,
            firehose_client,
        })
    }

    pub fn include_in_log_streams(&self) -> bool {
        if self.firehose_client.is_some() {
            return false;
        }
        // Only include in log streams on dev deployments
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

    #[fastrace::trace]
    pub async fn send_logs(&self, logs: ResolvedAuditLogLines) -> anyhow::Result<()> {
        let Some(firehose_client) = &self.firehose_client else {
            if self.include_in_log_streams() {
                self.send_to_log_streams(logs);
            }
            return Ok(());
        };

        let records = logs
            .logs
            .into_iter()
            .map(|l| serde_json::to_string(&l.into_value()))
            .collect::<Result<Vec<String>, _>>()?;

        firehose_client.send(records).await?;

        Ok(())
    }
}

pub struct AuditLogFirehoseClient {
    client: aws_sdk_firehose::Client,
    firehose_name: String,
}

impl AuditLogFirehoseClient {
    pub async fn new(firehose_name: String) -> anyhow::Result<Self> {
        let client = firehose_client().await?;
        Ok(Self {
            client,
            firehose_name,
        })
    }

    async fn send(&self, records: Vec<String>) -> anyhow::Result<()> {
        let records = records
            .into_iter()
            .map(|record| {
                let bytes = record.into_bytes();
                let data = Blob::new(bytes);
                Record::builder().set_data(Some(data)).build()
            })
            .collect::<Result<Vec<Record>, _>>()?;

        let results = self
            .client
            .put_record_batch()
            .set_delivery_stream_name(Some(self.firehose_name.clone()))
            .set_records(Some(records.clone()))
            .send()
            .await?;

        if results.failed_put_count() == 0 {
            return Ok(());
        }

        for result in results.request_responses().iter().take(5) {
            if let Some(error_code) = result.error_code() {
                // Log error message for first handful of firehose errors.
                tracing::error!(
                    "Firehose error while delivering audit logs: {}: {}",
                    error_code,
                    result.error_message().unwrap_or("")
                );
            }
        }

        anyhow::bail!(ErrorMetadata::bad_request(
            "AuditLogFailed",
            "Failed to deliver audit logs to AWS Firehose"
        ))
    }
}

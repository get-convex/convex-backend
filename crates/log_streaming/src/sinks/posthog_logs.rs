use std::sync::{
    atomic::Ordering,
    Arc,
};

use bytes::Bytes;
use common::{
    backoff::Backoff,
    errors::report_error,
    execution_context::ExecutionId,
    http::{
        categorize_http_response_stream,
        fetch::FetchClient,
        HttpRequestStream,
        APPLICATION_JSON_CONTENT_TYPE,
    },
    log_lines::LogLevel,
    log_streaming::{
        LogEvent,
        LogEventFormatVersion,
        StructuredLogEvent,
    },
    runtime::Runtime,
    RequestId,
};
use errors::{
    ErrorMetadata,
    ErrorMetadataAnyhowExt,
};
use http::{
    header::{
        AUTHORIZATION,
        CONTENT_TYPE,
    },
    HeaderMap,
    HeaderValue,
};
use model::log_sinks::types::posthog_logs::{
    PostHogLogsConfig,
    DEFAULT_POSTHOG_HOST,
};
use parking_lot::Mutex;
use serde_json::{
    json,
    Value as JsonValue,
};
use tokio::sync::mpsc;
use usage_tracking::UsageCounter;

use crate::{
    consts,
    metrics::posthog_logs_sink_network_egress_bytes,
    sinks::utils::{
        self,
        build_event_batches,
        default_log_filter,
    },
    LogSinkClient,
    LoggingDeploymentMetadata,
};

pub struct PostHogLogsSink<RT: Runtime> {
    runtime: RT,
    endpoint_url: reqwest::Url,
    api_key: String,
    service_name: String,
    fetch_client: Arc<dyn FetchClient>,
    events_receiver: mpsc::Receiver<Vec<Arc<LogEvent>>>,
    backoff: Backoff,
    deployment_metadata: Arc<Mutex<LoggingDeploymentMetadata>>,
    usage_counter: UsageCounter,
}

impl<RT: Runtime> PostHogLogsSink<RT> {
    pub async fn start(
        runtime: RT,
        config: PostHogLogsConfig,
        fetch_client: Arc<dyn FetchClient>,
        deployment_metadata: Arc<Mutex<LoggingDeploymentMetadata>>,
        usage_counter: UsageCounter,
        should_verify: bool,
    ) -> anyhow::Result<LogSinkClient> {
        tracing::info!("Starting PostHogLogsSink");
        let (tx, rx) = mpsc::channel(consts::POSTHOG_LOGS_SINK_EVENTS_BUFFER_SIZE);

        let host = config.host.as_deref().unwrap_or(DEFAULT_POSTHOG_HOST);
        let endpoint_url = format!("{host}/i/v1/logs");

        let service_name = config
            .service_name
            .unwrap_or_else(|| deployment_metadata.lock().deployment_name.clone());

        let mut sink = Self {
            runtime: runtime.clone(),
            deployment_metadata,
            endpoint_url: endpoint_url.parse()?,
            api_key: config.api_key.into_value(),
            service_name,
            fetch_client,
            events_receiver: rx,
            backoff: Backoff::new(
                consts::POSTHOG_LOGS_SINK_INITIAL_BACKOFF,
                consts::POSTHOG_LOGS_SINK_MAX_BACKOFF,
            ),
            usage_counter,
        };

        if should_verify {
            sink.verify_creds().await?;
            tracing::info!("PostHogLogsSink verified!");
        }

        let handle = Arc::new(Mutex::new(runtime.spawn("posthog_logs_sink", sink.go())));
        let client = LogSinkClient {
            _handle: handle,
            events_sender: tx,
        };
        Ok(client)
    }

    async fn verify_creds(&mut self) -> anyhow::Result<()> {
        // Send a minimal OTLP request with an empty logRecords array
        let deployment_metadata = self.deployment_metadata.lock().clone();
        let payload = json!({
            "resourceLogs": [{
                "resource": {
                    "attributes": self.build_resource_attributes(&deployment_metadata)
                },
                "scopeLogs": [{
                    "scope": { "name": "convex" },
                    "logRecords": []
                }]
            }]
        });
        self.send_batch(serde_json::to_vec(&payload)?, true).await?;
        Ok(())
    }

    async fn go(mut self) {
        loop {
            match self.events_receiver.recv().await {
                None => {
                    tracing::warn!("Stopping PostHogLogsSink. Sender was closed.");
                    return;
                },
                Some(ev) => {
                    let batches = build_event_batches(
                        ev,
                        consts::POSTHOG_LOGS_SINK_MAX_LOGS_PER_BATCH,
                        default_log_filter,
                    );

                    for batch in batches {
                        if let Err(mut e) = self.process_events(batch).await {
                            tracing::error!(
                                "Error emitting log event batch in PostHogLogsSink: {e:?}."
                            );
                            report_error(&mut e).await;
                        } else {
                            self.backoff.reset();
                        }
                    }
                },
            }
        }
    }

    fn build_resource_attributes(&self, metadata: &LoggingDeploymentMetadata) -> Vec<JsonValue> {
        let mut attrs = vec![
            json!({"key": "service.name", "value": {"stringValue": self.service_name}}),
            json!({"key": "convex.deployment.name", "value": {"stringValue": metadata.deployment_name}}),
        ];
        if let Some(dt) = metadata.deployment_type {
            attrs.push(
                json!({"key": "convex.deployment.type", "value": {"stringValue": dt.to_string()}}),
            );
        }
        if let Some(ref region) = metadata.deployment_region {
            attrs
                .push(json!({"key": "convex.deployment.region", "value": {"stringValue": region}}));
        }
        attrs
    }

    fn log_event_to_otlp_record(event: &LogEvent) -> anyhow::Result<JsonValue> {
        let time_unix_nano = (event.timestamp.as_ms_since_epoch()? as u128 * 1_000_000).to_string();

        let (severity_text, severity_number) = match &event.event {
            StructuredLogEvent::Console { log_line, .. } => match log_line.level {
                LogLevel::Error => ("ERROR", 17),
                LogLevel::Warn => ("WARN", 13),
                LogLevel::Info | LogLevel::Log => ("INFO", 9),
                LogLevel::Debug => ("DEBUG", 5),
            },
            StructuredLogEvent::FunctionExecution { error, .. } => {
                if error.is_some() {
                    ("ERROR", 17)
                } else {
                    ("INFO", 9)
                }
            },
            _ => ("INFO", 9),
        };

        // Serialize the full log event as the body
        let body_map = event.to_json_map(LogEventFormatVersion::V2)?;
        let body_str = serde_json::to_string(&body_map)?;

        let mut attributes = vec![];

        // Add the topic attribute
        let topic = match &event.event {
            StructuredLogEvent::Verification => "verification",
            StructuredLogEvent::Console { .. } => "console",
            StructuredLogEvent::FunctionExecution { .. } => "function_execution",
            StructuredLogEvent::DeploymentAuditLog { .. } => "deployment_audit_log",
            StructuredLogEvent::SchedulerStats { .. } => "scheduler_stats",
            StructuredLogEvent::ScheduledJobLag { .. } => "scheduled_job_lag",
            StructuredLogEvent::CurrentStorageUsage { .. } => "current_storage_usage",
            StructuredLogEvent::ConcurrencyStats { .. } => "concurrency_stats",
            StructuredLogEvent::Exception { .. } => "exception",
        };
        attributes.push(json!({"key": "convex.topic", "value": {"stringValue": topic}}));

        // Add function metadata when available
        match &event.event {
            StructuredLogEvent::Console { source, .. }
            | StructuredLogEvent::FunctionExecution { source, .. } => {
                attributes.push(
                    json!({"key": "convex.function.path", "value": {"stringValue": source.udf_path}}),
                );
                attributes.push(
                    json!({"key": "convex.function.type", "value": {"stringValue": source.udf_type.to_lowercase_string()}}),
                );
            },
            _ => {},
        }

        Ok(json!({
            "timeUnixNano": time_unix_nano,
            "severityText": severity_text,
            "severityNumber": severity_number,
            "body": { "stringValue": body_str },
            "attributes": attributes,
        }))
    }

    async fn process_events(&mut self, events: Vec<Arc<LogEvent>>) -> anyhow::Result<()> {
        crate::metrics::posthog_logs_sink_logs_received(events.len());

        let mut log_records = vec![];
        for event in &events {
            match Self::log_event_to_otlp_record(event) {
                Err(e) => tracing::warn!("Failed to convert log to OTLP record: {:?}", e),
                Ok(record) => log_records.push(record),
            }
        }

        if log_records.is_empty() {
            anyhow::bail!("Skipping an entire batch due to logs that failed to be serialized");
        }
        let batch_size = log_records.len();

        let deployment_metadata = self.deployment_metadata.lock().clone();
        let payload = json!({
            "resourceLogs": [{
                "resource": {
                    "attributes": self.build_resource_attributes(&deployment_metadata)
                },
                "scopeLogs": [{
                    "scope": { "name": "convex" },
                    "logRecords": log_records
                }]
            }]
        });

        self.send_batch(serde_json::to_vec(&payload)?, false)
            .await?;
        crate::metrics::posthog_logs_sink_logs_sent(batch_size);

        Ok(())
    }

    async fn send_batch(
        &mut self,
        batch_json: Vec<u8>,
        is_verification: bool,
    ) -> anyhow::Result<()> {
        let header_map = HeaderMap::from_iter([
            (
                AUTHORIZATION,
                HeaderValue::from_str(&format!("Bearer {}", self.api_key))?,
            ),
            (CONTENT_TYPE, APPLICATION_JSON_CONTENT_TYPE),
        ]);
        let batch_json = Bytes::from(batch_json);

        let request_id = RequestId::new();
        let execution_id = ExecutionId::new();
        for _ in 0..consts::POSTHOG_LOGS_SINK_MAX_REQUEST_ATTEMPTS {
            let batch_json = batch_json.clone();
            let response = self
                .fetch_client
                .fetch(HttpRequestStream {
                    url: self.endpoint_url.clone(),
                    method: http::Method::POST,
                    headers: header_map.clone(),
                    body: Box::pin(futures::stream::once(async { Ok(batch_json) })),
                    signal: Box::pin(futures::future::pending()),
                })
                .await;

            if !is_verification && let Ok(r) = &response {
                let num_bytes_egress = r.request_size.load(Ordering::Relaxed);
                utils::track_log_sink_bandwidth(
                    num_bytes_egress,
                    "posthog_logs".to_string(),
                    execution_id,
                    &request_id,
                    &self.usage_counter,
                    posthog_logs_sink_network_egress_bytes,
                )
                .await;
            }

            match response.and_then(categorize_http_response_stream) {
                Ok(_) => return Ok(()),
                Err(e) => {
                    if e.is_deterministic_user_error() {
                        anyhow::bail!(e.map_error_metadata(|e| ErrorMetadata {
                            code: e.code,
                            short_msg: "PostHogLogsRequestFailed".into(),
                            msg: e.msg,
                            source: None,
                        }));
                    } else {
                        let delay = self.backoff.fail(&mut self.runtime.rng());
                        tracing::warn!(
                            "Failed to send in PostHog Logs sink: {e}. Waiting {delay:?} before \
                             retrying."
                        );
                        self.runtime.wait(delay).await;
                    }
                },
            }
        }

        anyhow::bail!(ErrorMetadata::overloaded(
            "PostHogLogsMaxRetriesExceeded",
            "Exceeded max number of retry requests to PostHog Logs. Please try again later."
        ))
    }
}

#[cfg(test)]
mod tests {
    use std::{
        sync::Arc,
        time::Duration,
    };

    use common::{
        http::{
            fetch::StaticFetchClient,
            HttpRequestStream,
            HttpResponse,
        },
        log_streaming::LogEvent,
        runtime::{
            testing::TestRuntime,
            Runtime,
        },
    };
    use errors::ErrorMetadata;
    use futures::FutureExt;
    use http::{
        header::AUTHORIZATION,
        StatusCode,
    };
    use model::log_sinks::types::posthog_logs::PostHogLogsConfig;
    use parking_lot::Mutex;
    use reqwest::header::HeaderMap;
    use serde_json::Value as JsonValue;
    use usage_tracking::UsageCounter;

    use crate::{
        sinks::{
            posthog_logs::PostHogLogsSink,
            utils,
        },
        LoggingDeploymentMetadata,
    };

    #[convex_macro::test_runtime]
    async fn test_posthog_logs_requests(rt: TestRuntime) -> anyhow::Result<()> {
        let config = PostHogLogsConfig {
            api_key: "phc_test_key".to_string().into(),
            host: Some("https://us.i.posthog.com".to_string()),
            service_name: Some("test-service".to_string()),
        };

        let topic_buffer: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));

        let mut fetch_client = StaticFetchClient::new();
        {
            let buffer = Arc::clone(&topic_buffer);
            let url: reqwest::Url = "https://us.i.posthog.com/i/v1/logs".parse()?;
            let handler = move |request: HttpRequestStream| {
                let buffer = Arc::clone(&buffer);
                async move {
                    let request = request.into_http_request().await.unwrap();
                    let Some(true) = request
                        .headers
                        .get(AUTHORIZATION)
                        .map(|v| v.eq("Bearer phc_test_key"))
                    else {
                        anyhow::bail!(ErrorMetadata::forbidden("NoAuth", "bad api key"));
                    };

                    let json: JsonValue = serde_json::from_slice(&request.body.unwrap()).unwrap();

                    // Verify OTLP structure
                    let resource_logs = json["resourceLogs"].as_array().unwrap();
                    assert_eq!(resource_logs.len(), 1);

                    let resource = &resource_logs[0]["resource"];
                    let attrs = resource["attributes"].as_array().unwrap();
                    // Check service.name attribute exists
                    let service_attr = attrs.iter().find(|a| a["key"] == "service.name").unwrap();
                    assert_eq!(service_attr["value"]["stringValue"], "test-service");

                    let scope_logs = resource_logs[0]["scopeLogs"].as_array().unwrap();
                    let log_records = scope_logs[0]["logRecords"].as_array().unwrap();

                    if !log_records.is_empty() {
                        for record in log_records {
                            let attrs = record["attributes"].as_array().unwrap();
                            let topic_attr =
                                attrs.iter().find(|a| a["key"] == "convex.topic").unwrap();
                            buffer.lock().push(
                                topic_attr["value"]["stringValue"]
                                    .as_str()
                                    .unwrap()
                                    .to_string(),
                            );
                        }
                    } else {
                        buffer.lock().push("empty_verification".to_string());
                    }

                    Ok(HttpResponse {
                        status: StatusCode::OK,
                        headers: HeaderMap::new(),
                        body: Some("success".to_string().into_bytes()),
                        url: None,
                        request_size: "success".len() as u64,
                    }
                    .into())
                }
                .boxed()
            };
            fetch_client.register_http_route(url, reqwest::Method::POST, handler);
        }

        let meta = Arc::new(Mutex::new(LoggingDeploymentMetadata {
            deployment_name: "test-deployment".to_owned(),
            deployment_type: None,
            project_name: None,
            project_slug: None,
            deployment_region: Some("test".to_string()),
        }));

        let usage_counter = UsageCounter::new(Arc::new(events::usage::NoOpUsageEventLogger));
        let sink = PostHogLogsSink::start(
            rt.clone(),
            config,
            Arc::new(fetch_client),
            meta.clone(),
            usage_counter,
            true,
        )
        .await?;

        // Verification sends empty logRecords
        assert_eq!(
            &*topic_buffer.lock(),
            &vec!["empty_verification".to_string()]
        );

        // Send a regular log event (should pass default_log_filter)
        sink.events_sender
            .send(vec![Arc::new(LogEvent::default_for_verification(&rt)?)])
            .await?;
        rt.wait(Duration::from_secs(1)).await;

        // Send an exception event (should be filtered out by default_log_filter)
        sink.events_sender
            .send(vec![Arc::new(LogEvent::sample_exception(&rt)?)])
            .await?;
        rt.wait(Duration::from_secs(1)).await;

        // Only the verification event should have been sent (not the exception)
        assert_eq!(
            &*topic_buffer.lock(),
            &vec!["empty_verification".to_string(), "verification".to_string(),]
        );

        Ok(())
    }

    #[convex_macro::test_runtime]
    async fn test_posthog_logs_bad_api_key(rt: TestRuntime) -> anyhow::Result<()> {
        let config = PostHogLogsConfig {
            api_key: "phc_test_key".to_string().into(),
            host: Some("https://us.i.posthog.com".to_string()),
            service_name: None,
        };

        let mut fetch_client = StaticFetchClient::new();
        let url: reqwest::Url = "https://us.i.posthog.com/i/v1/logs".parse()?;
        let handler = |request: HttpRequestStream| {
            async move {
                let Some(true) = request
                    .headers
                    .get(AUTHORIZATION)
                    .map(|v| v.eq("INCORRECT_api_key"))
                else {
                    anyhow::bail!(ErrorMetadata::forbidden("NoAuth", "bad api key"));
                };
                Ok(HttpResponse {
                    status: StatusCode::OK,
                    headers: HeaderMap::new(),
                    body: Some("success!".to_string().into_bytes()),
                    url: None,
                    request_size: "success!".len() as u64,
                }
                .into())
            }
            .boxed()
        };
        fetch_client.register_http_route(url, reqwest::Method::POST, Box::new(handler));

        let meta = Arc::new(Mutex::new(LoggingDeploymentMetadata {
            deployment_name: "test-deployment".to_owned(),
            deployment_type: None,
            project_name: None,
            project_slug: None,
            deployment_region: Some("test".to_string()),
        }));

        let usage_counter = UsageCounter::new(Arc::new(events::usage::NoOpUsageEventLogger));
        assert!(PostHogLogsSink::start(
            rt.clone(),
            config,
            Arc::new(fetch_client),
            meta,
            usage_counter,
            true,
        )
        .await
        .is_err());

        Ok(())
    }

    #[convex_macro::test_runtime]
    async fn test_posthog_logs_tracks_bandwidth(rt: TestRuntime) -> anyhow::Result<()> {
        let config = PostHogLogsConfig {
            api_key: "phc_test_key".to_string().into(),
            host: Some("https://us.i.posthog.com".to_string()),
            service_name: Some("test-service".to_string()),
        };

        let actual_request_size = Arc::new(Mutex::new(0u64));

        let mut fetch_client = StaticFetchClient::new();
        let url: reqwest::Url = "https://us.i.posthog.com/i/v1/logs".parse()?;
        let size_tracker = actual_request_size.clone();
        let handler = move |request: HttpRequestStream| {
            let size_tracker = size_tracker.clone();
            async move {
                let request = request.into_http_request().await.unwrap();
                let request_size = request.body.as_ref().map(|b| b.len()).unwrap_or(0) as u64;
                *size_tracker.lock() = request_size;

                Ok(HttpResponse {
                    status: StatusCode::OK,
                    headers: HeaderMap::new(),
                    body: Some("success".to_string().into_bytes()),
                    url: None,
                    request_size,
                }
                .into())
            }
            .boxed()
        };
        fetch_client.register_http_route(url, reqwest::Method::POST, Box::new(handler));

        let meta = Arc::new(Mutex::new(LoggingDeploymentMetadata {
            deployment_name: "test-deployment".to_owned(),
            deployment_type: None,
            project_name: None,
            project_slug: None,
            deployment_region: Some("test".to_string()),
        }));

        let usage_logger = events::testing::BasicTestUsageEventLogger::new();
        let usage_counter = UsageCounter::new(Arc::new(usage_logger.clone()));

        let sink = PostHogLogsSink::start(
            rt.clone(),
            config,
            Arc::new(fetch_client),
            meta.clone(),
            usage_counter,
            true,
        )
        .await?;

        sink.events_sender
            .send(vec![Arc::new(LogEvent::default_for_verification(&rt)?)])
            .await?;
        rt.wait(Duration::from_secs(1)).await;

        let events = usage_logger.collect();
        let actual_size = *actual_request_size.lock();
        utils::assert_bandwidth_events(events, actual_size, "posthog_logs");

        Ok(())
    }
}

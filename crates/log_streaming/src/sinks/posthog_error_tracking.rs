use std::sync::{
    atomic::Ordering,
    Arc,
};

use bytes::Bytes;
use chrono::{
    DateTime,
    Utc,
};
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
    log_streaming::{
        LogEvent,
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
    header::CONTENT_TYPE,
    HeaderMap,
};
use model::log_sinks::types::posthog_logs::DEFAULT_POSTHOG_HOST;
use parking_lot::Mutex;
use serde_json::{
    json,
    Value as JsonValue,
};
use tokio::sync::mpsc;
use usage_tracking::UsageCounter;

use crate::{
    consts,
    metrics::posthog_et_sink_network_egress_bytes,
    sinks::utils::{
        self,
        build_event_batches,
        only_exceptions_log_filter,
    },
    LogSinkClient,
    LoggingDeploymentMetadata,
};

pub struct PostHogErrorTrackingSink<RT: Runtime> {
    runtime: RT,
    capture_url: reqwest::Url,
    api_key: String,
    fetch_client: Arc<dyn FetchClient>,
    events_receiver: mpsc::Receiver<Vec<Arc<LogEvent>>>,
    backoff: Backoff,
    deployment_metadata: Arc<Mutex<LoggingDeploymentMetadata>>,
    usage_counter: UsageCounter,
}

impl<RT: Runtime> PostHogErrorTrackingSink<RT> {
    pub async fn start(
        runtime: RT,
        config: model::log_sinks::types::posthog_error_tracking::PostHogErrorTrackingConfig,
        fetch_client: Arc<dyn FetchClient>,
        deployment_metadata: Arc<Mutex<LoggingDeploymentMetadata>>,
        usage_counter: UsageCounter,
        should_verify: bool,
    ) -> anyhow::Result<LogSinkClient> {
        tracing::info!("Starting PostHogErrorTrackingSink");
        let (tx, rx) = mpsc::channel(consts::POSTHOG_ET_SINK_EVENTS_BUFFER_SIZE);

        let host = config.host.as_deref().unwrap_or(DEFAULT_POSTHOG_HOST);
        let capture_url = format!("{host}/i/v0/e/");

        let mut sink = Self {
            runtime: runtime.clone(),
            deployment_metadata,
            capture_url: capture_url.parse()?,
            api_key: config.api_key.into_value(),
            fetch_client,
            events_receiver: rx,
            backoff: Backoff::new(
                consts::POSTHOG_ET_SINK_INITIAL_BACKOFF,
                consts::POSTHOG_ET_SINK_MAX_BACKOFF,
            ),
            usage_counter,
        };

        if should_verify {
            sink.verify_creds().await?;
            tracing::info!("PostHogErrorTrackingSink verified!");
        }

        let handle = Arc::new(Mutex::new(runtime.spawn("posthog_et_sink", sink.go())));
        let client = LogSinkClient {
            _handle: handle,
            events_sender: tx,
        };
        Ok(client)
    }

    async fn verify_creds(&mut self) -> anyhow::Result<()> {
        // Send a minimal $exception event to verify the API key is valid.
        // PostHog rejects empty batches, so we send a single verification event.
        let payload = json!({
            "api_key": self.api_key,
            "batch": [{
                "event": "$exception",
                "distinct_id": "convex-verification",
                "properties": {
                    "$exception_list": [{
                        "type": "ConvexVerification",
                        "value": "Verifying PostHog Error Tracking integration",
                        "mechanism": { "handled": true, "type": "generic" },
                    }],
                    "$lib": "convex",
                }
            }]
        });
        self.send_batch(serde_json::to_vec(&payload)?, true).await?;
        Ok(())
    }

    async fn go(mut self) {
        loop {
            match self.events_receiver.recv().await {
                None => {
                    tracing::warn!("Stopping PostHogErrorTrackingSink. Sender was closed.");
                    return;
                },
                Some(ev) => {
                    let batches = build_event_batches(
                        ev,
                        consts::POSTHOG_ET_SINK_MAX_LOGS_PER_BATCH,
                        only_exceptions_log_filter,
                    );

                    for batch in batches {
                        if let Err(mut e) = self.process_events(batch).await {
                            tracing::error!(
                                "Error emitting log event batch in PostHogErrorTrackingSink: \
                                 {e:?}."
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

    async fn process_events(&mut self, events: Vec<Arc<LogEvent>>) -> anyhow::Result<()> {
        crate::metrics::posthog_et_sink_logs_received(events.len());

        if events.is_empty() {
            return Ok(());
        }

        let deployment_metadata = self.deployment_metadata.lock().clone();
        let mut batch_events = vec![];

        for event in &events {
            let StructuredLogEvent::Exception {
                error,
                user_identifier,
                source,
                udf_server_version,
            } = &event.event
            else {
                continue;
            };

            let distinct_id = user_identifier
                .as_ref()
                .map(|u| u.to_string())
                .unwrap_or_else(|| deployment_metadata.deployment_name.clone());

            let timestamp: DateTime<Utc> = event.timestamp.as_system_time().into();
            let timestamp_str = timestamp.to_rfc3339();

            // Build frames (NOT reversed — PostHog expects oldest-to-newest)
            let frames: Vec<JsonValue> = error
                .frames
                .as_ref()
                .map(|js_frames| {
                    js_frames
                        .0
                        .iter()
                        .map(|frame| {
                            let function = frame
                                .function_name
                                .as_deref()
                                .or(frame.method_name.as_deref())
                                .unwrap_or("<anonymous>");
                            json!({
                                "platform": "custom",
                                "lang": "javascript",
                                "filename": frame.file_name,
                                "function": function,
                                "lineno": frame.line_number,
                                "colno": frame.column_number,
                                "in_app": true,
                            })
                        })
                        .collect()
                })
                .unwrap_or_default();

            let exception_sources: Vec<&str> = error
                .frames
                .as_ref()
                .map(|f| {
                    f.0.iter()
                        .filter_map(|frame| frame.file_name.as_deref())
                        .collect()
                })
                .unwrap_or_default();

            let exception_functions: Vec<&str> = error
                .frames
                .as_ref()
                .map(|f| {
                    f.0.iter()
                        .map(|frame| {
                            frame
                                .function_name
                                .as_deref()
                                .or(frame.method_name.as_deref())
                                .unwrap_or("<anonymous>")
                        })
                        .collect()
                })
                .unwrap_or_default();

            let lib_version = udf_server_version
                .as_ref()
                .map(|v| v.to_string())
                .unwrap_or_else(|| "unknown".to_string());

            let capture_event = json!({
                "event": "$exception",
                "distinct_id": distinct_id,
                "timestamp": timestamp_str,
                "properties": {
                    "$exception_list": [{
                        "type": "Error",
                        "value": error.message,
                        "mechanism": { "handled": false, "type": "generic" },
                        "stacktrace": {
                            "type": "raw",
                            "frames": frames,
                        }
                    }],
                    "$exception_level": "error",
                    "$exception_types": ["Error"],
                    "$exception_values": [error.message],
                    "$exception_sources": exception_sources,
                    "$exception_functions": exception_functions,
                    "$lib": "convex",
                    "$lib_version": lib_version,
                    "convex_function": source.udf_path,
                    "convex_function_type": source.udf_type.to_lowercase_string(),
                    "convex_function_runtime": source.module_environment.as_sentry_tag(),
                    "convex_deployment": deployment_metadata.deployment_name,
                    "convex_deployment_type": deployment_metadata.deployment_type.map(|dt| dt.to_string()),
                    "convex_request_id": source.context.request_id.to_string(),
                }
            });

            batch_events.push(capture_event);
        }

        if batch_events.is_empty() {
            return Ok(());
        }
        let batch_size = batch_events.len();

        let payload = json!({
            "api_key": self.api_key,
            "batch": batch_events,
        });

        self.send_batch(serde_json::to_vec(&payload)?, false)
            .await?;
        crate::metrics::posthog_et_sink_logs_sent(batch_size);

        Ok(())
    }

    async fn send_batch(
        &mut self,
        batch_json: Vec<u8>,
        is_verification: bool,
    ) -> anyhow::Result<()> {
        // PostHog capture API uses api_key in the body, no Authorization header
        let header_map = HeaderMap::from_iter([(CONTENT_TYPE, APPLICATION_JSON_CONTENT_TYPE)]);
        let batch_json = Bytes::from(batch_json);

        let request_id = RequestId::new();
        let execution_id = ExecutionId::new();
        for _ in 0..consts::POSTHOG_ET_SINK_MAX_REQUEST_ATTEMPTS {
            let batch_json = batch_json.clone();
            let response = self
                .fetch_client
                .fetch(HttpRequestStream {
                    url: self.capture_url.clone(),
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
                    "posthog_error_tracking".to_string(),
                    execution_id,
                    &request_id,
                    &self.usage_counter,
                    posthog_et_sink_network_egress_bytes,
                )
                .await;
            }

            match response.and_then(categorize_http_response_stream) {
                Ok(_) => return Ok(()),
                Err(e) => {
                    if e.is_deterministic_user_error() {
                        anyhow::bail!(e.map_error_metadata(|e| ErrorMetadata {
                            code: e.code,
                            short_msg: "PostHogErrorTrackingRequestFailed".into(),
                            msg: e.msg,
                            source: None,
                        }));
                    } else {
                        let delay = self.backoff.fail(&mut self.runtime.rng());
                        tracing::warn!(
                            "Failed to send in PostHog Error Tracking sink: {e}. Waiting \
                             {delay:?} before retrying."
                        );
                        self.runtime.wait(delay).await;
                    }
                },
            }
        }

        anyhow::bail!(ErrorMetadata::overloaded(
            "PostHogETMaxRetriesExceeded",
            "Exceeded max number of retry requests to PostHog Error Tracking. Please try again \
             later."
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
        types::DeploymentType,
    };
    use errors::ErrorMetadata;
    use futures::FutureExt;
    use model::log_sinks::types::posthog_error_tracking::PostHogErrorTrackingConfig;
    use parking_lot::Mutex;
    use reqwest::header::HeaderMap;
    use serde_json::Value as JsonValue;
    use usage_tracking::UsageCounter;

    use crate::{
        sinks::{
            posthog_error_tracking::PostHogErrorTrackingSink,
            utils,
        },
        LoggingDeploymentMetadata,
    };

    #[convex_macro::test_runtime]
    async fn test_posthog_error_tracking_receives_only_exceptions(
        rt: TestRuntime,
    ) -> anyhow::Result<()> {
        let config = PostHogErrorTrackingConfig {
            api_key: "phc_test_key".to_string().into(),
            host: Some("https://us.i.posthog.com".to_string()),
        };

        let captured_events: Arc<Mutex<Vec<JsonValue>>> = Arc::new(Mutex::new(Vec::new()));

        let mut fetch_client = StaticFetchClient::new();
        {
            let events = Arc::clone(&captured_events);
            let url: reqwest::Url = "https://us.i.posthog.com/i/v0/e/".parse()?;
            let handler = move |request: HttpRequestStream| {
                let events = Arc::clone(&events);
                async move {
                    let request = request.into_http_request().await.unwrap();
                    let json: JsonValue = serde_json::from_slice(&request.body.unwrap()).unwrap();

                    let batch = json["batch"].as_array().unwrap().clone();
                    events.lock().extend(batch);

                    Ok(HttpResponse {
                        status: http::StatusCode::OK,
                        headers: HeaderMap::new(),
                        body: Some(r#"{"status": 1}"#.to_string().into_bytes()),
                        url: None,
                        request_size: r#"{"status": 1}"#.len() as u64,
                    }
                    .into())
                }
                .boxed()
            };
            fetch_client.register_http_route(url, reqwest::Method::POST, handler);
        }

        let meta = Arc::new(Mutex::new(LoggingDeploymentMetadata {
            deployment_name: "flying-fish-123".to_string(),
            deployment_type: Some(DeploymentType::Dev),
            project_name: Some("test".to_string()),
            project_slug: Some("test".to_string()),
            deployment_region: Some("test".to_string()),
        }));

        let usage_counter = UsageCounter::new(Arc::new(events::usage::NoOpUsageEventLogger));
        let sink = PostHogErrorTrackingSink::start(
            rt.clone(),
            config,
            Arc::new(fetch_client),
            meta.clone(),
            usage_counter,
            true,
        )
        .await?;

        // Send both a regular event and an exception
        sink.events_sender
            .send(vec![
                Arc::new(LogEvent::default_for_verification(&rt)?),
                Arc::new(LogEvent::sample_exception(&rt)?),
            ])
            .await?;
        rt.wait(Duration::from_secs(1)).await;

        let all_events = captured_events.lock().clone();
        // Filter out the verification event sent during startup
        let events: Vec<_> = all_events
            .iter()
            .filter(|e| {
                e.get("properties")
                    .and_then(|p| p.get("$exception_list"))
                    .and_then(|l| l.as_array())
                    .and_then(|a| a.first())
                    .and_then(|e| e.get("type"))
                    .and_then(|t| t.as_str())
                    != Some("ConvexVerification")
            })
            .collect();
        // Only the exception should be captured (verification event is filtered
        // out, and the non-exception LogEvent is dropped by the filter)
        assert_eq!(events.len(), 1);

        let exception_event = &events[0];
        assert_eq!(exception_event["event"], "$exception");
        assert_eq!(exception_event["distinct_id"], "test|user");
        assert_eq!(
            exception_event["properties"]["convex_deployment"],
            "flying-fish-123"
        );
        assert_eq!(exception_event["properties"]["convex_function"], "test");
        assert_eq!(
            exception_event["properties"]["convex_function_type"],
            "action"
        );
        assert_eq!(exception_event["properties"]["$lib"], "convex");
        assert_eq!(exception_event["properties"]["$lib_version"], "1.5.1");
        assert_eq!(exception_event["properties"]["$exception_level"], "error");

        // Check exception list structure
        let exception_list = exception_event["properties"]["$exception_list"]
            .as_array()
            .unwrap();
        assert_eq!(exception_list.len(), 1);
        assert_eq!(exception_list[0]["type"], "Error");
        assert_eq!(exception_list[0]["value"], "test_message");

        // Verify frames are NOT reversed (oldest-to-newest)
        let frames = exception_list[0]["stacktrace"]["frames"]
            .as_array()
            .unwrap();
        assert_eq!(frames.len(), 2);
        assert_eq!(frames[0]["filename"], "test_frame_1");
        assert_eq!(frames[1]["filename"], "test_frame_2");

        Ok(())
    }

    #[convex_macro::test_runtime]
    async fn test_posthog_error_tracking_bad_api_key(rt: TestRuntime) -> anyhow::Result<()> {
        let config = PostHogErrorTrackingConfig {
            api_key: "phc_test_key".to_string().into(),
            host: Some("https://us.i.posthog.com".to_string()),
        };

        let mut fetch_client = StaticFetchClient::new();
        let url: reqwest::Url = "https://us.i.posthog.com/i/v0/e/".parse()?;
        let handler = |request: HttpRequestStream| {
            async move {
                let request = request.into_http_request().await.unwrap();
                let json: JsonValue = serde_json::from_slice(&request.body.unwrap()).unwrap();
                if json["api_key"] != "CORRECT_KEY" {
                    anyhow::bail!(ErrorMetadata::forbidden("InvalidAPIKey", "Invalid API key"));
                }
                Ok(HttpResponse {
                    status: http::StatusCode::OK,
                    headers: HeaderMap::new(),
                    body: Some(r#"{"status": 1}"#.to_string().into_bytes()),
                    url: None,
                    request_size: r#"{"status": 1}"#.len() as u64,
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
        assert!(PostHogErrorTrackingSink::start(
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
    async fn test_posthog_error_tracking_tracks_bandwidth(rt: TestRuntime) -> anyhow::Result<()> {
        let config = PostHogErrorTrackingConfig {
            api_key: "phc_test_key".to_string().into(),
            host: Some("https://us.i.posthog.com".to_string()),
        };

        let actual_request_size = Arc::new(Mutex::new(0u64));

        let mut fetch_client = StaticFetchClient::new();
        let url: reqwest::Url = "https://us.i.posthog.com/i/v0/e/".parse()?;
        let size_tracker = actual_request_size.clone();
        let handler = move |request: HttpRequestStream| {
            let size_tracker = size_tracker.clone();
            async move {
                let request = request.into_http_request().await.unwrap();
                let request_size = request.body.as_ref().map(|b| b.len()).unwrap_or(0) as u64;
                *size_tracker.lock() = request_size;

                Ok(HttpResponse {
                    status: http::StatusCode::OK,
                    headers: HeaderMap::new(),
                    body: Some(r#"{"status": 1}"#.to_string().into_bytes()),
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

        let sink = PostHogErrorTrackingSink::start(
            rt.clone(),
            config,
            Arc::new(fetch_client),
            meta.clone(),
            usage_counter,
            true,
        )
        .await?;

        // Send an exception event
        sink.events_sender
            .send(vec![Arc::new(LogEvent::sample_exception(&rt)?)])
            .await?;
        rt.wait(Duration::from_secs(1)).await;

        let events = usage_logger.collect();
        let actual_size = *actual_request_size.lock();
        utils::assert_bandwidth_events(events, actual_size, "posthog_error_tracking");

        Ok(())
    }
}

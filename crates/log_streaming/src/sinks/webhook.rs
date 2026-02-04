use std::{
    ops::Deref,
    sync::{
        atomic::Ordering,
        Arc,
    },
};

use aws_lc_rs::hmac::{
    self,
    HMAC_SHA256,
};
use bytes::Bytes;
use common::{
    backoff::Backoff,
    errors::report_error,
    execution_context::ExecutionId,
    http::{
        categorize_http_response_stream,
        fetch::FetchClient,
        HttpRequest,
        APPLICATION_JSON_CONTENT_TYPE,
    },
    log_streaming::{
        LogEvent,
        LogEventFormatVersion,
    },
    runtime::Runtime,
    RequestId,
};
use errors::{
    ErrorMetadata,
    ErrorMetadataAnyhowExt,
};
use hex::ToHex;
use http::{
    header::CONTENT_TYPE,
    HeaderValue,
};
use model::log_sinks::types::webhook::{
    WebhookConfig,
    WebhookFormat,
};
use parking_lot::Mutex;
use reqwest::header::HeaderMap;
use serde::Serialize;
use serde_json::Value as JsonValue;
use tokio::sync::mpsc;
use usage_tracking::UsageCounter;

use crate::{
    consts,
    metrics::webhook_sink_network_egress_bytes,
    sinks::utils::{
        self,
        build_event_batches,
        default_log_filter,
    },
    LogSinkClient,
    LoggingDeploymentMetadata,
};

pub const LOG_EVENT_FORMAT_FOR_WEBHOOK: LogEventFormatVersion = LogEventFormatVersion::V2;

#[derive(Serialize, Debug, Clone)]
struct WebhookLogEvent<'a> {
    #[serde(flatten)]
    event: serde_json::Map<String, JsonValue>,
    convex: &'a LoggingDeploymentMetadata,
}

impl<'a> WebhookLogEvent<'a> {
    fn new(
        event: LogEvent,
        deployment_metadata: &'a LoggingDeploymentMetadata,
    ) -> anyhow::Result<Self> {
        Ok(Self {
            event: event.to_json_map(LOG_EVENT_FORMAT_FOR_WEBHOOK)?,
            convex: deployment_metadata,
        })
    }
}

pub struct WebhookSink<RT: Runtime> {
    runtime: RT,
    config: WebhookConfig,
    fetch_client: Arc<dyn FetchClient>,
    events_receiver: mpsc::Receiver<Vec<Arc<LogEvent>>>,
    backoff: Backoff,
    deployment_metadata: Arc<Mutex<LoggingDeploymentMetadata>>,
    usage_counter: UsageCounter,
}

impl<RT: Runtime> WebhookSink<RT> {
    pub async fn start(
        runtime: RT,
        config: WebhookConfig,
        fetch_client: Arc<dyn FetchClient>,
        deployment_metadata: Arc<Mutex<LoggingDeploymentMetadata>>,
        usage_counter: UsageCounter,
        should_verify: bool,
    ) -> anyhow::Result<LogSinkClient> {
        tracing::info!("Starting WebhookSink");
        let (tx, rx) = mpsc::channel(consts::WEBHOOK_SINK_EVENTS_BUFFER_SIZE);

        let mut sink = Self {
            runtime: runtime.clone(),
            config,
            fetch_client,
            events_receiver: rx,
            backoff: Backoff::new(
                consts::WEBHOOK_SINK_INITIAL_BACKOFF,
                consts::WEBHOOK_SINK_MAX_BACKOFF,
            ),
            deployment_metadata,
            usage_counter,
        };

        if should_verify {
            sink.verify_initial_request().await?;
            tracing::info!("WebhookSink verified!");
        }

        let handle = Arc::new(Mutex::new(runtime.spawn("webhook_sink", sink.go())));

        Ok(LogSinkClient {
            _handle: handle,
            events_sender: tx,
        })
    }

    async fn verify_initial_request(&mut self) -> anyhow::Result<()> {
        let verification_event = LogEvent::default_for_verification(&self.runtime)?;
        let deployment_metadata = self.deployment_metadata.lock().clone();
        let payload = WebhookLogEvent::new(verification_event, &deployment_metadata)?;
        self.send_batch(vec![payload], true).await?;

        Ok(())
    }

    async fn go(mut self) {
        loop {
            match self.events_receiver.recv().await {
                None => {
                    // The sender was closed, event loop should shutdown
                    tracing::warn!("Stopping WebhookSink. Sender was closed.");
                    return;
                },
                Some(ev) => {
                    // Split events into batches
                    let batches = build_event_batches(
                        ev,
                        consts::WEBHOOK_SINK_MAX_LOGS_PER_BATCH,
                        default_log_filter,
                    );

                    // Process each batch and send to Datadog
                    for batch in batches {
                        if let Err(mut e) = self.process_events(batch).await {
                            tracing::error!(
                                "Error emitting log event batch in WebhookSink: {e:?}."
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

    async fn send_batch(
        &mut self,
        batch: Vec<WebhookLogEvent<'_>>,
        is_verification: bool,
    ) -> anyhow::Result<()> {
        let mut batch_json: Vec<JsonValue> = vec![];
        for ev in batch {
            batch_json.push(serde_json::to_value(ev)?);
        }
        let payload = match self.config.format {
            WebhookFormat::Json => serde_json::to_vec(&JsonValue::Array(batch_json))?,
            WebhookFormat::Jsonl => batch_json
                .into_iter()
                .map(|v| Ok(serde_json::to_vec(&v)?))
                .collect::<anyhow::Result<Vec<Vec<u8>>>>()?
                .join("\n".as_bytes()),
        };
        let payload = Bytes::from(payload);

        // Create HMAC-SHA256 signature
        let s_key = hmac::Key::new(HMAC_SHA256, self.config.hmac_secret.as_ref());
        let signature: String = hmac::sign(&s_key, &payload).encode_hex();

        let mut headers = HeaderMap::from_iter([(CONTENT_TYPE, APPLICATION_JSON_CONTENT_TYPE)]);
        headers.append(
            "x-webhook-signature",
            HeaderValue::from_str(format!("sha256={signature}").as_str())?,
        );

        // Make request in a loop that retries on transient errors
        let request_id = RequestId::new();
        let execution_id = ExecutionId::new();
        let mut last_error = None;
        let max_attempts = if is_verification {
            consts::WEBHOOK_SINK_VERIFICATION_MAX_ATTEMPTS
        } else {
            consts::WEBHOOK_SINK_MAX_REQUEST_ATTEMPTS
        };
        for _ in 0..max_attempts {
            let response = self
                .fetch_client
                .fetch(
                    HttpRequest {
                        url: self.config.url.clone(),
                        method: http::Method::POST,
                        headers: headers.clone(),
                        body: Some(payload.clone()),
                    }
                    .into(),
                )
                .await;

            if !is_verification && let Ok(r) = &response {
                let num_bytes_egress = r.request_size.load(Ordering::Relaxed);
                utils::track_log_sink_bandwidth(
                    num_bytes_egress,
                    self.config.url.to_string(),
                    execution_id,
                    &request_id,
                    &self.usage_counter,
                    webhook_sink_network_egress_bytes,
                )
                .await;
            }

            // Only retry on 5xx requests
            match response.and_then(categorize_http_response_stream) {
                Ok(_) => return Ok(()),
                Err(e) => {
                    // Retry on 5xx, uncategorized errors, or any error which is either our or
                    // webhook's fault. Short-circuit for 4xx errors which are
                    // the user's fault.
                    if e.is_deterministic_user_error() {
                        // Just update the short message
                        anyhow::bail!(e.map_error_metadata(|e| ErrorMetadata {
                            code: e.code,
                            short_msg: "WebhookRequestFailed".into(),
                            msg: e.msg,
                            source: None,
                        }));
                    } else {
                        let delay = self.backoff.fail(&mut self.runtime.rng());
                        tracing::warn!(
                            "Failed to send in Webhook sink: {e}. Waiting {delay:?} before \
                             retrying."
                        );
                        // Wrap error with ErrorMetadata if it doesn't have it, so the actual
                        // error message appears in the failure reason
                        let e = if e.downcast_ref::<ErrorMetadata>().is_none() {
                            let error_msg = format!("{e}");
                            anyhow::anyhow!(ErrorMetadata::overloaded(
                                "WebhookRequestFailed",
                                error_msg
                            ))
                        } else {
                            e
                        };
                        last_error = Some(e);
                        self.runtime.wait(delay).await;
                    }
                },
            }
        }

        // If we get here, we've exceeded the max number of requests
        // Return the last error which now has ErrorMetadata
        if let Some(e) = last_error {
            return Err(e);
        }
        anyhow::bail!(ErrorMetadata::overloaded(
            "WebhookMaxRetriesExceeded",
            format!(
                "Exceeded max number of retry requests to webhook {}.",
                self.config.url.as_str()
            )
        ))
    }

    async fn process_events(&mut self, events: Vec<Arc<LogEvent>>) -> anyhow::Result<()> {
        crate::metrics::webhook_sink_logs_received(events.len());

        let mut values_to_send = vec![];
        let deployment_metadata = self.deployment_metadata.lock().clone();
        for event in events {
            match WebhookLogEvent::new(event.deref().clone(), &deployment_metadata) {
                Err(e) => tracing::warn!("failed to convert log to JSON: {:?}", e),
                Ok(v) => values_to_send.push(v),
            }
        }

        if values_to_send.is_empty() {
            anyhow::bail!("skipping an entire batch due to logs that failed to be processed");
        }
        let batch_size = values_to_send.len();

        if let Err(e) = self.send_batch(values_to_send, false).await {
            // We don't report this error to Sentry to prevent misconfigured webhook sinks
            // from overflowing our Sentry logs.
            tracing::error!("could not send batch to WebhookSink: {e}");
        } else {
            crate::metrics::webhook_sink_logs_sent(batch_size);
        }
        Ok(())
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
    use futures::FutureExt;
    use http::{
        HeaderMap,
        StatusCode,
    };
    use model::log_sinks::types::webhook::{
        WebhookConfig,
        WebhookFormat,
    };
    use parking_lot::Mutex;
    use usage_tracking::UsageCounter;

    use crate::{
        sinks::{
            utils,
            webhook::WebhookSink,
        },
        LoggingDeploymentMetadata,
    };

    #[convex_macro::test_runtime]
    async fn test_webhook_tracks_bandwidth(rt: TestRuntime) -> anyhow::Result<()> {
        // Test that verifies webhook sink correctly tracks network egress as billable
        // usage. This ensures that bytes sent to webhook endpoints are
        // properly reported via UsageEvent::NetworkBandwidth events.
        let webhook_url: reqwest::Url = "https://webhook.example.com/endpoint".parse()?;

        let webhook_config = WebhookConfig {
            url: webhook_url.clone(),
            format: WebhookFormat::Json,
            hmac_secret: "test_secret".to_string(),
        };

        // Track the actual request size from the handler
        let actual_request_size = Arc::new(Mutex::new(0u64));

        // Register handler that returns success and tracks request size
        let mut fetch_client = StaticFetchClient::new();
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

        fetch_client.register_http_route(
            webhook_url.clone(),
            reqwest::Method::POST,
            Box::new(handler),
        );

        let meta = Arc::new(Mutex::new(LoggingDeploymentMetadata {
            deployment_name: "".to_owned(),
            deployment_type: None,
            project_name: None,
            project_slug: None,
            deployment_region: Some("test".to_string()),
        }));

        // Use BasicTestUsageEventLogger to capture usage events
        let usage_logger = events::testing::BasicTestUsageEventLogger::new();
        let usage_counter = UsageCounter::new(Arc::new(usage_logger.clone()));

        let webhook_sink = WebhookSink::start(
            rt.clone(),
            webhook_config,
            Arc::new(fetch_client),
            meta.clone(),
            usage_counter,
            false, // Don't verify, so we only track one event
        )
        .await?;

        // Send a log event
        webhook_sink
            .events_sender
            .send(vec![Arc::new(LogEvent::default_for_verification(&rt)?)])
            .await?;
        rt.wait(Duration::from_secs(1)).await;

        // Verify bandwidth tracking
        let events = usage_logger.collect();
        let actual_size = *actual_request_size.lock();
        utils::assert_bandwidth_events(events, actual_size, webhook_url.as_str());

        Ok(())
    }
}

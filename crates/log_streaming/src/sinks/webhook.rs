use std::{
    ops::Deref,
    sync::Arc,
};

use bytes::Bytes;
use common::{
    backoff::Backoff,
    errors::report_error,
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
};
use errors::{
    ErrorMetadata,
    ErrorMetadataAnyhowExt,
};
use http::header::CONTENT_TYPE;
use model::log_sinks::types::webhook::{
    WebhookConfig,
    WebhookFormat,
};
use parking_lot::Mutex;
use reqwest::header::HeaderMap;
use serde::Serialize;
use serde_json::Value as JsonValue;
use tokio::sync::mpsc;

use crate::{
    consts,
    sinks::utils::{
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
}

impl<RT: Runtime> WebhookSink<RT> {
    pub async fn start(
        runtime: RT,
        config: WebhookConfig,
        fetch_client: Arc<dyn FetchClient>,
        deployment_metadata: Arc<Mutex<LoggingDeploymentMetadata>>,
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
        };

        sink.verify_initial_request().await?;
        tracing::info!("WebhookSink verified!");

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
        self.send_batch(vec![payload]).await?;

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

    async fn send_batch(&mut self, batch: Vec<WebhookLogEvent<'_>>) -> anyhow::Result<()> {
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

        // Make request in a loop that retries on transient errors
        let mut headers = HeaderMap::from_iter([(CONTENT_TYPE, APPLICATION_JSON_CONTENT_TYPE)]);
        if let Some(basic_auth) = &self.config.basic_auth {
            // Build Authorization: Basic base64(username:password)
            let creds = format!("{}:{}", basic_auth.username, basic_auth.password.0);
            let encoded = base64::encode(creds);
            headers.append(
                http::header::AUTHORIZATION,
                format!("Basic {}", encoded).parse().unwrap(),
            );
        }

        for _ in 0..consts::WEBHOOK_SINK_MAX_REQUEST_ATTEMPTS {
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
                        self.runtime.wait(delay).await;
                    }
                },
            }
        }

        // If we get here, we've exceed the max number of requests
        anyhow::bail!(ErrorMetadata::overloaded(
            "WebhookMaxRetriesExceeded",
            format!(
                "Exceeded max number of retry requests to webhook {}. Please try again later.",
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

        if let Err(e) = self.send_batch(values_to_send).await {
            // We don't report this error to Sentry to prevent misconfigured webhook sinks
            // from overflowing our Sentry logs.
            tracing::error!("could not send batch to WebhookSink: {e}");
        } else {
            crate::metrics::webhook_sink_logs_sent(batch_size);
        }
        Ok(())
    }
}

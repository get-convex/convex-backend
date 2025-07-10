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
use model::log_sinks::types::datadog::DatadogConfig;
use parking_lot::Mutex;
use reqwest::header::{
    HeaderMap,
    HeaderName,
    HeaderValue,
};
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

const DD_API_KEY_HEADER: &str = "DD-API-KEY";

#[derive(Debug, Clone)]
pub struct DatadogMetadata {
    ddtags: String,
    hostname: String,
    service: Option<String>,
}

impl DatadogMetadata {
    pub fn new(ddtags: Vec<String>, instance_name: String, service: Option<String>) -> Self {
        let ddtags = ddtags.join(",");

        Self {
            ddtags,
            hostname: instance_name,
            service,
        }
    }
}

#[derive(Serialize, Debug, Clone)]
struct DatadogLogEvent<'a> {
    ddsource: String,
    ddtags: String,
    hostname: String,
    service: Option<String>,
    #[serde(flatten)]
    event: serde_json::Map<String, JsonValue>,
    convex: &'a LoggingDeploymentMetadata,
}

impl<'a> DatadogLogEvent<'a> {
    fn new(
        event: LogEvent,
        metadata: &DatadogMetadata,
        format: LogEventFormatVersion,
        deployment_metadata: &'a LoggingDeploymentMetadata,
    ) -> anyhow::Result<Self> {
        Ok(Self {
            ddsource: "convex".to_string(),
            ddtags: metadata.ddtags.clone(),
            hostname: metadata.hostname.clone(),
            service: metadata.service.clone(),
            event: event.to_json_map(format)?,
            convex: deployment_metadata,
        })
    }
}

pub(crate) struct DatadogSink<RT: Runtime> {
    runtime: RT,
    fetch_client: Arc<dyn FetchClient>,
    dd_url: reqwest::Url,
    dd_api_key: String,
    metadata: DatadogMetadata,
    log_event_format: LogEventFormatVersion,
    events_receiver: mpsc::Receiver<Vec<Arc<LogEvent>>>,
    backoff: Backoff,
    deployment_metadata: Arc<Mutex<LoggingDeploymentMetadata>>,
}

impl<RT: Runtime> DatadogSink<RT> {
    pub async fn start(
        runtime: RT,
        fetch_client: Arc<dyn FetchClient>,
        config: DatadogConfig,
        deployment_metadata: Arc<Mutex<LoggingDeploymentMetadata>>,
    ) -> anyhow::Result<LogSinkClient> {
        tracing::info!("Starting DatadogSink");
        let (tx, rx) = mpsc::channel(consts::DD_SINK_EVENTS_BUFFER_SIZE);

        let metadata = DatadogMetadata::new(
            config.dd_tags,
            deployment_metadata.lock().deployment_name.clone(),
            config.service,
        );

        let mut sink = Self {
            runtime: runtime.clone(),
            dd_url: config.site_location.get_logging_endpoint()?,
            dd_api_key: config.dd_api_key.into_value(),
            metadata,
            log_event_format: config.version,
            events_receiver: rx,
            fetch_client,
            backoff: Backoff::new(consts::DD_SINK_INITIAL_BACKOFF, consts::DD_SINK_MAX_BACKOFF),
            deployment_metadata: deployment_metadata.clone(),
        };

        sink.verify_creds().await?;
        tracing::info!("DatadogSink verified!");

        let handle = Arc::new(Mutex::new(runtime.spawn("datadog_sink", sink.go())));
        let client = LogSinkClient {
            _handle: handle,
            events_sender: tx,
        };
        Ok(client)
    }

    async fn go(mut self) {
        loop {
            match self.events_receiver.recv().await {
                None => {
                    // The sender was closed, event loop should shutdown
                    tracing::warn!("Stopping DatadogSink. Sender was closed.");
                    return;
                },
                Some(ev) => {
                    // Split events into batches
                    let batches = build_event_batches(
                        ev,
                        consts::DD_SINK_MAX_LOGS_PER_BATCH,
                        default_log_filter,
                    );

                    // Process each batch and send to Datadog
                    for batch in batches {
                        if let Err(mut e) = self.process_events(batch).await {
                            tracing::error!(
                                "Error emitting log event batch in DatadogSink: {e:?}."
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

    /// Verify that an initial request succeeds
    async fn verify_creds(&mut self) -> anyhow::Result<()> {
        let verification_event = LogEvent::default_for_verification(&self.runtime)?;
        let deployment_metadata = self.deployment_metadata.lock().clone();
        let payload = DatadogLogEvent::new(
            verification_event,
            &self.metadata,
            self.log_event_format,
            &deployment_metadata,
        )?;
        self.send_batch(vec![payload]).await?;

        Ok(())
    }

    async fn send_batch(&mut self, batch: Vec<DatadogLogEvent<'_>>) -> anyhow::Result<()> {
        let mut batch_json: Vec<JsonValue> = vec![];
        for ev in batch {
            batch_json.push(serde_json::to_value(ev)?);
        }
        let payload = JsonValue::Array(batch_json);
        let header_map = HeaderMap::from_iter([
            (
                HeaderName::from_bytes(DD_API_KEY_HEADER.as_bytes())?,
                HeaderValue::from_str(&self.dd_api_key)?,
            ),
            (CONTENT_TYPE, APPLICATION_JSON_CONTENT_TYPE),
        ]);
        let payload = Bytes::from(serde_json::to_vec(&payload)?);

        // Make request in a loop that retries on transient errors
        for _ in 0..consts::DD_SINK_MAX_REQUEST_ATTEMPTS {
            let response = self
                .fetch_client
                .fetch(
                    HttpRequest {
                        url: self.dd_url.clone(),
                        method: http::Method::POST,
                        headers: header_map.clone(),
                        body: Some(payload.clone()),
                    }
                    .into(),
                )
                .await;

            // Retry only on 5xx errors.
            match response.and_then(categorize_http_response_stream) {
                Ok(_) => return Ok(()),
                Err(e) => {
                    // Retry on 5xx, uncategorized errors, or any error which is either our or
                    // Datadog's fault. Short-circuit for 4xx errors which are
                    // the user's fault.
                    if e.is_deterministic_user_error() {
                        anyhow::bail!(e.map_error_metadata(|e| ErrorMetadata {
                            code: e.code,
                            short_msg: "DatadogRequestFailed".into(),
                            msg: e.msg,
                            source: None,
                        }));
                    } else {
                        let delay = self.backoff.fail(&mut self.runtime.rng());
                        tracing::warn!(
                            "Failed to send in Datadog sink: {e}. Waiting {delay:?} before \
                             retrying."
                        );
                        self.runtime.wait(delay).await;
                    }
                },
            }
        }

        // If we get here, we've exceed the max number of requests
        anyhow::bail!(ErrorMetadata::overloaded(
            "DatadogMaxRetriesExceeded",
            "Exceeded max number of retry requests to Datadog. Please try again later."
        ))
    }

    async fn process_events(&mut self, events: Vec<Arc<LogEvent>>) -> anyhow::Result<()> {
        let log_event_format_version = match self.log_event_format {
            LogEventFormatVersion::V1 => "1",
            LogEventFormatVersion::V2 => "2",
        };
        crate::metrics::datadog_sink_logs_received(events.len(), log_event_format_version);

        let mut values_to_send = vec![];
        let deployment_metadata = self.deployment_metadata.lock().clone();
        for event in events {
            match DatadogLogEvent::new(
                event.deref().clone(),
                &self.metadata,
                self.log_event_format,
                &deployment_metadata,
            ) {
                Err(e) => tracing::warn!("failed to convert log to JSON: {:?}", e),
                Ok(v) => values_to_send.push(v),
            }
        }

        if values_to_send.is_empty() {
            anyhow::bail!("skipping an entire batch due to logs that failed to be serialized");
        }
        let batch_size = values_to_send.len();

        self.send_batch(values_to_send).await?;
        crate::metrics::datadog_sink_logs_sent(batch_size, log_event_format_version);

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
        log_streaming::{
            LogEvent,
            LogEventFormatVersion,
        },
        runtime::{
            testing::TestRuntime,
            Runtime,
        },
        types::DeploymentType,
    };
    use errors::ErrorMetadata;
    use futures::FutureExt;
    use http::StatusCode;
    use model::log_sinks::types::datadog::{
        DatadogConfig,
        DatadogSiteLocation,
    };
    use parking_lot::Mutex;
    use reqwest::header::HeaderMap;
    use serde_json::Value as JsonValue;

    use crate::{
        sinks::datadog::{
            DatadogSink,
            DD_API_KEY_HEADER,
        },
        LoggingDeploymentMetadata,
    };

    #[convex_macro::test_runtime]
    async fn test_dd_requests(rt: TestRuntime) -> anyhow::Result<()> {
        let dd_config = DatadogConfig {
            site_location: DatadogSiteLocation::US1,
            dd_api_key: "fake_api_key".to_string().into(),
            dd_tags: vec![],
            version: LogEventFormatVersion::default(),
            service: Some("fake_service".to_owned()),
        };

        let topic_buffer: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));

        // Register handler
        let mut fetch_client = StaticFetchClient::new();
        {
            let buffer = Arc::clone(&topic_buffer);
            let url: reqwest::Url = "https://http-intake.logs.datadoghq.com/api/v2/logs".parse()?;
            let handler = move |request: HttpRequestStream| {
                let buffer = Arc::clone(&buffer);
                async move {
                    let request = request.into_http_request().await.unwrap();
                    let Some(true) = request
                        .headers
                        .get(DD_API_KEY_HEADER)
                        .map(|v| v.eq("fake_api_key"))
                    else {
                        anyhow::bail!(ErrorMetadata::forbidden("NoAuth", "bad api key"));
                    };

                    // Write topic to buffer
                    let mut json =
                        serde_json::from_slice::<JsonValue>(&request.body.unwrap()).unwrap();
                    let batch = json.as_array_mut().unwrap();
                    let obj = batch[0].as_object_mut().unwrap();
                    let topic = obj.remove("topic").unwrap();

                    buffer.lock().push(topic.as_str().unwrap().to_string());

                    Ok(HttpResponse {
                        status: StatusCode::OK,
                        headers: HeaderMap::new(),
                        body: Some("success".to_string().into_bytes()),
                        url: None,
                    }
                    .into())
                }
                .boxed()
            };

            fetch_client.register_http_route(url, reqwest::Method::POST, handler);
        }
        let meta = Arc::new(Mutex::new(LoggingDeploymentMetadata {
            deployment_name: "".to_owned(),
            deployment_type: Some(DeploymentType::Dev),
            project_name: Some("test".to_string()),
            project_slug: Some("test".to_string()),
        }));
        // Assert that verification response succeeded
        let dd_sink =
            DatadogSink::start(rt.clone(), Arc::new(fetch_client), dd_config, meta.clone()).await?;
        assert_eq!(&*topic_buffer.lock(), &vec!["verification".to_string()]);

        dd_sink
            .events_sender
            .send(vec![Arc::new(LogEvent::default_for_verification(&rt)?)])
            .await?;
        rt.wait(Duration::from_secs(1)).await;

        // This log should be filtered out
        dd_sink
            .events_sender
            .send(vec![Arc::new(LogEvent::sample_exception(&rt)?)])
            .await?;
        rt.wait(Duration::from_secs(1)).await;

        assert_eq!(
            &*topic_buffer.lock(),
            &vec!["verification".to_string(), "verification".to_string()]
        );

        Ok(())
    }

    #[convex_macro::test_runtime]
    async fn test_dd_bad_api_key(rt: TestRuntime) -> anyhow::Result<()> {
        let dd_config = DatadogConfig {
            site_location: DatadogSiteLocation::US1,
            dd_api_key: "fake_api_key".to_string().into(),
            dd_tags: vec![],
            version: LogEventFormatVersion::default(),
            service: None,
        };

        // Register handler
        let mut fetch_client = StaticFetchClient::new();
        let url: reqwest::Url = "https://http-intake.logs.datadoghq.com/api/v2/logs".parse()?;
        let handler = |request: HttpRequestStream| {
            async move {
                let Some(true) = request
                    .headers
                    .get(DD_API_KEY_HEADER)
                    .map(|v| v.eq("INCORRECT_api_key"))
                else {
                    anyhow::bail!(ErrorMetadata::forbidden("NoAuth", "bad api key"));
                };
                Ok(HttpResponse {
                    status: StatusCode::OK,
                    headers: HeaderMap::new(),
                    body: Some("success!".to_string().into_bytes()),
                    url: None,
                }
                .into())
            }
            .boxed()
        };
        fetch_client.register_http_route(url.clone(), reqwest::Method::POST, Box::new(handler));

        let meta = Arc::new(Mutex::new(LoggingDeploymentMetadata {
            deployment_name: "".to_owned(),
            deployment_type: Some(DeploymentType::Dev),
            project_name: Some("test".to_string()),
            project_slug: Some("test".to_string()),
        }));
        // Assert that verification response failed
        assert!(
            DatadogSink::start(rt.clone(), Arc::new(fetch_client), dd_config, meta,)
                .await
                .is_err()
        );

        Ok(())
    }
}

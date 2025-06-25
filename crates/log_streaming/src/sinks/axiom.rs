use std::{
    collections::BTreeMap,
    sync::Arc,
};

use bytes::Bytes;
use common::{
    backoff::Backoff,
    errors::report_error,
    http::{
        categorize_http_response_stream,
        fetch::FetchClient,
        HttpRequestStream,
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
use http::{
    header::{
        AUTHORIZATION,
        CONTENT_TYPE,
    },
    HeaderMap,
    HeaderValue,
};
use model::log_sinks::types::axiom::AxiomConfig;
use parking_lot::Mutex;
use serde::{
    Serialize,
    Serializer,
};
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

#[derive(Serialize, Debug, Clone)]
pub struct AxiomLogEvent<'a> {
    _time: u64,
    #[serde(serialize_with = "serialize_log_event")]
    data: (&'a LogEvent, LogEventFormatVersion),
    attributes: &'a BTreeMap<String, String>,
    convex: &'a LoggingDeploymentMetadata,
}

fn serialize_log_event<S: Serializer>(
    &(event, format): &(&LogEvent, LogEventFormatVersion),
    serializer: S,
) -> Result<S::Ok, S::Error> {
    event.to_json_serializer(format, serializer)
}

impl<'a> AxiomLogEvent<'a> {
    fn new(
        event: &'a LogEvent,
        attributes: &'a BTreeMap<String, String>,
        format: LogEventFormatVersion,
        deployment_metadata: &'a LoggingDeploymentMetadata,
    ) -> anyhow::Result<Self> {
        Ok(Self {
            _time: event.timestamp.as_ms_since_epoch()?,
            data: (event, format),
            attributes,
            convex: deployment_metadata,
        })
    }
}

pub struct AxiomSink<RT: Runtime> {
    runtime: RT,
    axiom_url: reqwest::Url,
    api_key: String,
    attributes: BTreeMap<String, String>,
    log_event_format: LogEventFormatVersion,
    fetch_client: Arc<dyn FetchClient>,
    events_receiver: mpsc::Receiver<Vec<Arc<LogEvent>>>,
    backoff: Backoff,
    deployment_metadata: Arc<Mutex<LoggingDeploymentMetadata>>,
}

impl<RT: Runtime> AxiomSink<RT> {
    pub async fn start(
        runtime: RT,
        config: AxiomConfig,
        fetch_client: Arc<dyn FetchClient>,
        deployment_metadata: Arc<Mutex<LoggingDeploymentMetadata>>,
    ) -> anyhow::Result<LogSinkClient> {
        tracing::info!("Starting AxiomSink");
        let (tx, rx) = mpsc::channel(consts::AXIOM_SINK_EVENTS_BUFFER_SIZE);

        let mut sink = Self {
            runtime: runtime.clone(),
            deployment_metadata,
            axiom_url: format!(
                "https://api.axiom.co/v1/datasets/{:}/ingest",
                config.dataset_name.clone()
            )
            .parse()?,
            api_key: config.api_key.into_value(),
            attributes: config
                .attributes
                .into_iter()
                .map(|a| (a.key, a.value))
                .collect(),
            log_event_format: config.version,
            fetch_client,
            events_receiver: rx,
            backoff: Backoff::new(
                consts::AXIOM_SINK_INITIAL_BACKOFF,
                consts::AXIOM_SINK_MAX_BACKOFF,
            ),
        };

        sink.verify_creds().await?;
        tracing::info!("AxiomSink verified!");

        let handle = Arc::new(Mutex::new(runtime.spawn("axiom_sink", sink.go())));
        let client = LogSinkClient {
            _handle: handle,
            events_sender: tx,
        };
        Ok(client)
    }

    async fn verify_creds(&mut self) -> anyhow::Result<()> {
        let verification_event = LogEvent::default_for_verification(&self.runtime)?;
        let deployment_metadata = self.deployment_metadata.lock().clone();
        let payload = AxiomLogEvent::new(
            &verification_event,
            &self.attributes,
            self.log_event_format,
            &deployment_metadata,
        )?;
        self.send_batch(serde_json::to_vec(&vec![payload])?).await?;

        Ok(())
    }

    async fn go(mut self) {
        loop {
            match self.events_receiver.recv().await {
                None => {
                    // The sender was closed, event loop should shutdown
                    tracing::warn!("Stopping AxiomSink. Sender was closed.");
                    return;
                },
                Some(ev) => {
                    // Split events into batches
                    let batches = build_event_batches(
                        ev,
                        consts::AXIOM_SINK_MAX_LOGS_PER_BATCH,
                        default_log_filter,
                    );

                    // Process each batch and send to Axiom
                    for batch in batches {
                        if let Err(mut e) = self.process_events(batch).await {
                            tracing::error!("Error emitting log event batch in AxiomSink: {e:?}.");
                            report_error(&mut e).await;
                        } else {
                            self.backoff.reset();
                        }
                    }
                },
            }
        }
    }

    async fn send_batch(&mut self, batch_json: Vec<u8>) -> anyhow::Result<()> {
        let header_map = HeaderMap::from_iter([
            (
                AUTHORIZATION,
                HeaderValue::from_str(&format!("Bearer {}", self.api_key))?,
            ),
            (CONTENT_TYPE, APPLICATION_JSON_CONTENT_TYPE),
        ]);
        let batch_json = Bytes::from(batch_json);

        // Make request in a loop that retries on transient errors
        for _ in 0..consts::AXIOM_SINK_MAX_REQUEST_ATTEMPTS {
            let batch_json = batch_json.clone();
            let response = self
                .fetch_client
                .fetch(HttpRequestStream {
                    url: self.axiom_url.clone(),
                    method: http::Method::POST,
                    headers: header_map.clone(),
                    body: Box::pin(futures::stream::once(async { Ok(batch_json) })),
                    signal: Box::pin(futures::future::pending()),
                })
                .await;

            // Retry only on 5xx errors.
            match response.and_then(categorize_http_response_stream) {
                Ok(_) => return Ok(()),
                Err(e) => {
                    // Retry on 5xx, uncategorized errors, or any error which is either our or
                    // Axiom's fault. Short-circuit for 4xx errors which are
                    // the user's fault.
                    if e.is_deterministic_user_error() {
                        anyhow::bail!(e.map_error_metadata(|e| ErrorMetadata {
                            code: e.code,
                            short_msg: "AxiomRequestFailed".into(),
                            msg: e.msg,
                            source: None,
                        }));
                    } else {
                        let delay = self.backoff.fail(&mut self.runtime.rng());
                        tracing::warn!(
                            "Failed to send in Axiom sink: {e}. Waiting {delay:?} before retrying."
                        );
                        self.runtime.wait(delay).await;
                    }
                },
            }
        }

        // If we get here, we've exceed the max number of requests
        anyhow::bail!(ErrorMetadata::overloaded(
            "AxiomMaxRetriesExceeded",
            "Exceeded max number of retry requests to Axiom. Please try again later."
        ))
    }

    async fn process_events(&mut self, events: Vec<Arc<LogEvent>>) -> anyhow::Result<()> {
        let log_event_format_version = match self.log_event_format {
            LogEventFormatVersion::V1 => "1",
            LogEventFormatVersion::V2 => "2",
        };
        crate::metrics::axiom_sink_logs_received(events.len(), log_event_format_version);

        let mut values_to_send = vec![];
        let deployment_metadata = self.deployment_metadata.lock().clone();
        for event in &events {
            match AxiomLogEvent::new(
                event,
                &self.attributes,
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

        self.send_batch(serde_json::to_vec(&values_to_send)?)
            .await?;
        crate::metrics::axiom_sink_logs_sent(batch_size, log_event_format_version);

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
    };
    use errors::ErrorMetadata;
    use futures::FutureExt;
    use http::{
        header::AUTHORIZATION,
        StatusCode,
    };
    use model::log_sinks::types::axiom::{
        AxiomAttribute,
        AxiomConfig,
    };
    use parking_lot::Mutex;
    use reqwest::header::HeaderMap;
    use serde_json::Value as JsonValue;

    use crate::{
        sinks::axiom::AxiomSink,
        LoggingDeploymentMetadata,
    };

    #[convex_macro::test_runtime]
    async fn test_axiom_requests(rt: TestRuntime) -> anyhow::Result<()> {
        let axiom_config = AxiomConfig {
            api_key: "test_api_key".to_string().into(),
            dataset_name: "test_dataset".to_string(),
            attributes: vec![AxiomAttribute {
                key: "author".to_string(),
                value: "rakeeb".to_string(),
            }],
            version: LogEventFormatVersion::default(),
        };

        let topic_buffer: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));

        // Register handler
        let mut fetch_client = StaticFetchClient::new();
        {
            let buffer = Arc::clone(&topic_buffer);
            let url: reqwest::Url =
                "https://api.axiom.co/v1/datasets/test_dataset/ingest".parse()?;
            let handler = move |request: HttpRequestStream| {
                let buffer = Arc::clone(&buffer);
                async move {
                    let request = request.into_http_request().await.unwrap();
                    let Some(true) = request
                        .headers
                        .get(AUTHORIZATION)
                        .map(|v| v.eq("Bearer test_api_key"))
                    else {
                        anyhow::bail!(ErrorMetadata::forbidden("NoAuth", "bad api key"));
                    };

                    // Write topic to buffer
                    let mut json =
                        serde_json::from_slice::<JsonValue>(&request.body.unwrap()).unwrap();
                    let batch = json.as_array_mut().unwrap();
                    let obj = batch[0].as_object_mut().unwrap();
                    let mut data = obj.remove("data").unwrap();
                    let data = data.as_object_mut().unwrap();
                    let topic = data.remove("topic").unwrap();

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
            deployment_type: None,
            project_name: None,
            project_slug: None,
        }));
        // Assert that verification response succeeded
        let axiom_sink = AxiomSink::start(
            rt.clone(),
            axiom_config,
            Arc::new(fetch_client),
            meta.clone(),
        )
        .await?;
        assert_eq!(&*topic_buffer.lock(), &vec!["verification".to_string()]);

        axiom_sink
            .events_sender
            .send(vec![Arc::new(LogEvent::default_for_verification(&rt)?)])
            .await?;
        rt.wait(Duration::from_secs(1)).await;

        // This log should be filtered out
        axiom_sink
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
    async fn test_axiom_bad_api_key(rt: TestRuntime) -> anyhow::Result<()> {
        let axiom_config = AxiomConfig {
            api_key: "test_api_key".to_string().into(),
            dataset_name: "test_dataset".to_string(),
            attributes: vec![AxiomAttribute {
                key: "author".to_string(),
                value: "rakeeb".to_string(),
            }],
            version: LogEventFormatVersion::default(),
        };

        // Register handler
        let mut fetch_client = StaticFetchClient::new();
        let url: reqwest::Url = "https://api.axiom.co/v1/datasets/test_dataset/ingest".parse()?;
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
                }
                .into())
            }
            .boxed()
        };
        fetch_client.register_http_route(url.clone(), reqwest::Method::POST, Box::new(handler));

        let meta = Arc::new(Mutex::new(LoggingDeploymentMetadata {
            deployment_name: "".to_owned(),
            deployment_type: None,
            project_name: None,
            project_slug: None,
        }));
        // Assert that verification response failed
        assert!(
            AxiomSink::start(rt.clone(), axiom_config, Arc::new(fetch_client), meta,)
                .await
                .is_err()
        );

        Ok(())
    }
}

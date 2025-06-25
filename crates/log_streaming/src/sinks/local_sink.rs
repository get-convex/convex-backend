use std::{
    fs::OpenOptions,
    io::Write,
    path::PathBuf,
    str::FromStr,
    sync::Arc,
};

use common::{
    backoff::Backoff,
    errors::report_error,
    log_streaming::{
        LogEvent,
        LogEventFormatVersion,
    },
    runtime::Runtime,
};
use parking_lot::Mutex;
use serde_json::Value as JsonValue;
use tokio::sync::mpsc;

use crate::{
    consts,
    LogSinkClient,
};

pub const LOG_EVENT_FORMAT_FOR_LOCAL_SINK: LogEventFormatVersion = LogEventFormatVersion::V2;

pub struct LocalSink<RT: Runtime> {
    runtime: RT,
    events_receiver: mpsc::Receiver<Vec<Arc<LogEvent>>>,
    config: LocalSinkConfig,
}

#[derive(Clone, Debug)]
pub struct LocalSinkConfig {
    path: PathBuf,
}

impl FromStr for LocalSinkConfig {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self { path: s.parse()? })
    }
}

impl<RT: Runtime> LocalSink<RT> {
    pub async fn start(runtime: RT, config: LocalSinkConfig) -> anyhow::Result<LogSinkClient> {
        let (tx, rx) = mpsc::channel(consts::LOCAL_SINK_EVENTS_BUFFER_SIZE);

        let sink = Self {
            runtime: runtime.clone(),
            events_receiver: rx,
            config: config.clone(),
        };

        let handle = Arc::new(Mutex::new(runtime.spawn("local_sink", sink.go())));

        let client = LogSinkClient {
            _handle: handle,
            events_sender: tx,
        };
        tracing::info!("Started LocalSink at {:?}", config.path);
        Ok(client)
    }

    async fn go(mut self) {
        let mut backoff = Backoff::new(
            consts::LOCAL_SINK_INITIAL_BACKOFF,
            consts::LOCAL_SINK_MAX_BACKOFF,
        );

        loop {
            match self.events_receiver.recv().await {
                None => {
                    // The sender was closed, event loop should shutdown
                    tracing::warn!("Stopping LocalSink. Sender was closed.");
                    return;
                },
                Some(events) => {
                    while let Err(mut e) = self.process_events(events.clone()).await {
                        let delay = backoff.fail(&mut self.runtime.rng());
                        tracing::error!(
                            "Error emitting event in LocalSink: {e:?}. Waiting {delay:?}ms before \
                             retrying"
                        );
                        report_error(&mut e).await;
                        self.runtime.wait(delay).await;
                    }
                    backoff.reset();
                },
            }
        }
    }

    async fn process_events(&mut self, events: Vec<Arc<LogEvent>>) -> anyhow::Result<()> {
        let mut file = OpenOptions::new()
            .append(true)
            .create(true)
            .open(self.config.path.clone())?;
        let num_events = events.len();
        for event in events {
            let fields: serde_json::Map<String, JsonValue> =
                event.to_json_map(LOG_EVENT_FORMAT_FOR_LOCAL_SINK)?;
            let mut event = serde_json::to_vec(&fields)?;
            event.extend_from_slice("\n".as_bytes());
            file.write_all(&event)?;
        }
        file.sync_all()?;
        tracing::debug!(
            "Wrote {} events to file: {:?}",
            num_events,
            self.config.path.clone()
        );
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use std::{
        sync::Arc,
        time::Duration,
    };

    use common::{
        log_lines::{
            LogLevel,
            LogLineStructured,
        },
        log_streaming::{
            FunctionEventSource,
            LogEvent,
            StructuredLogEvent,
        },
        runtime::Runtime,
    };
    use runtime::testing::TestRuntime;
    use tempfile::TempDir;

    use crate::sinks::local_sink::{
        LocalSink,
        LocalSinkConfig,
        LOG_EVENT_FORMAT_FOR_LOCAL_SINK,
    };

    #[convex_macro::test_runtime]
    async fn test_local_sink(rt: TestRuntime) -> anyhow::Result<()> {
        let tempdir = TempDir::new()?;
        let path = tempdir.path().join("test_sink.log");
        let test_sink_config = LocalSinkConfig { path };
        let client = LocalSink::start(rt.clone(), test_sink_config.clone()).await?;
        let event = LogEvent {
            timestamp: rt.unix_timestamp(),
            event: StructuredLogEvent::Console {
                source: FunctionEventSource::new_for_test(),
                log_line: LogLineStructured::new_developer_log_line(
                    LogLevel::Log,
                    vec!["This is a test log.".to_string()],
                    rt.unix_timestamp(),
                ),
            },
        };
        client
            .events_sender
            .try_send(vec![Arc::new(event.clone())])?;
        rt.wait(Duration::from_secs(1)).await;
        let contents = std::fs::read_to_string(test_sink_config.path.clone())?;
        let expected_contents =
            serde_json::to_string(&event.to_json_map(LOG_EVENT_FORMAT_FOR_LOCAL_SINK)?)? + "\n";
        assert_eq!(contents, expected_contents);

        // Sending another event should append to the file
        let event = LogEvent {
            timestamp: rt.unix_timestamp(),
            event: StructuredLogEvent::Verification,
        };
        client
            .events_sender
            .try_send(vec![Arc::new(event.clone())])?;
        rt.wait(Duration::from_secs(1)).await;
        let contents = std::fs::read_to_string(test_sink_config.path)?;
        let log_contents_2 =
            serde_json::to_string(&event.to_json_map(LOG_EVENT_FORMAT_FOR_LOCAL_SINK)?)? + "\n";
        assert_eq!(contents, expected_contents + &log_contents_2);
        Ok(())
    }
}

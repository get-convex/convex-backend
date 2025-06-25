use std::sync::{
    Arc,
    LazyLock,
};

use common::{
    backoff::Backoff,
    errors::report_error,
    log_streaming::LogEvent,
    runtime::Runtime,
};
use parking_lot::{
    Mutex,
    RwLock,
};
use tokio::sync::mpsc;

use crate::{
    consts,
    LogSinkClient,
};

/// MockSink directs all logs to a static buffer
pub struct MockSink<RT: Runtime> {
    runtime: RT,
    events_receiver: mpsc::Receiver<Vec<Arc<LogEvent>>>,
}

/// The buffer MockSink writes events to. This is explicitly a static global
/// since there is no effective and simple way to dependency inject a shared
/// vector into the MockSink that would be accessible to test code:
///
/// Passing a shared vector to MockSink::start is ineffective since this is only
/// invoked by LogManager::config_to_log_sink_client which is not directly
/// invoked by test code but instead by a database listener, making it difficult
/// to obtain a reference to the shared vector outside of the sink.
///
/// Creating a shared vector and passing to LogSinkClient would also be
/// ineffective since testing interfaces through an Application instance and
/// Application only owns a LogManagerClient, not a LogManager itself.
/// LogManagerClient does not have direct access to LogManager's LogSinkClients.
///
/// For safety, this module is only compiled in testing.
pub static MOCK_SINK_EVENTS_BUFFER: LazyLock<Arc<RwLock<Vec<Arc<LogEvent>>>>> =
    LazyLock::new(|| Arc::new(RwLock::new(vec![])));

impl<RT: Runtime> MockSink<RT> {
    pub async fn start(runtime: RT) -> anyhow::Result<LogSinkClient> {
        let (tx, rx) = mpsc::channel(consts::MOCK_SINK_EVENTS_BUFFER_SIZE);

        let sink = Self {
            runtime: runtime.clone(),
            events_receiver: rx,
        };

        let handle = Arc::new(Mutex::new(runtime.spawn("mock_sink", sink.go())));

        let client = LogSinkClient {
            _handle: handle,
            events_sender: tx,
        };
        Ok(client)
    }

    async fn go(mut self) {
        let mut backoff = Backoff::new(
            consts::MOCK_SINK_INITIAL_BACKOFF,
            consts::MOCK_SINK_MAX_BACKOFF,
        );

        loop {
            match self.events_receiver.recv().await {
                None => {
                    // The sender was closed, event loop should shutdown
                    tracing::warn!("Stopping MockSink. Sender was closed.");
                    return;
                },
                Some(events) => {
                    while let Err(mut e) = self.process_events(events.clone()).await {
                        let delay = backoff.fail(&mut self.runtime.rng());
                        tracing::error!(
                            "Error emitting event in MockSink: {e:?}. Waiting {delay:?}ms before \
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

    async fn process_events(&mut self, mut events: Vec<Arc<LogEvent>>) -> anyhow::Result<()> {
        let mut buf = MOCK_SINK_EVENTS_BUFFER.write();
        buf.append(&mut events);
        Ok(())
    }
}

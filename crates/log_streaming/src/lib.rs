#![feature(coroutines)]
#![feature(future_join)]
#![feature(let_chains)]
#![feature(never_type)]
#![feature(assert_matches)]
pub mod consts;
mod metrics;
pub mod sinks;

use std::{
    collections::BTreeMap,
    fmt::Formatter,
    sync::{
        atomic::{
            AtomicBool,
            AtomicUsize,
            Ordering,
        },
        Arc,
    },
    time::Duration,
};

use common::{
    backoff::Backoff,
    document::ParsedDocument,
    errors::{
        report_error,
        report_error_sync,
    },
    http::fetch::FetchClient,
    knobs,
    log_streaming::{
        LogEvent,
        LogSender,
    },
    runtime::{
        Runtime,
        SpawnHandle,
        WithTimeout,
    },
    types::DeploymentType,
};
use database::{
    Database,
    SystemMetadataModel,
};
use errors::{
    ErrorMetadata,
    ErrorMetadataAnyhowExt,
};
use futures::{
    pin_mut,
    select_biased,
    FutureExt,
};
use keybroker::Identity;
use model::{
    backend_info::BackendInfoModel,
    log_sinks::{
        types::{
            LogSinksRow,
            SinkConfig,
            SinkState,
            SinkType,
        },
        LogSinksModel,
    },
};
use parking_lot::{
    Mutex,
    RwLock,
};
use serde::Serialize;
#[cfg(any(test, feature = "testing"))]
use sinks::mock_sink::MockSink;
use sinks::{
    local_sink::LocalSink,
    sentry::SentrySink,
};
use tokio::sync::mpsc;

use crate::sinks::{
    axiom::AxiomSink,
    datadog::DatadogSink,
    webhook::WebhookSink,
};

/// Public worker for the LogManager.
///
/// See `log_streaming/README.md` for more info.
#[derive(Clone)]
pub struct LogManagerClient {
    handle: Arc<Mutex<Box<dyn SpawnHandle>>>,
    event_sender: mpsc::Sender<LogEvent>,
    active_sinks_count: Arc<AtomicUsize>,
    entitlement_enabled: Arc<AtomicBool>,
}

impl LogManagerClient {
    fn send_logs_inner(&self, logs: Vec<LogEvent>) -> Result<(), SendLogBatchError> {
        let mut dropped = 0;
        let mut num_left = logs.len();
        let total = logs.len();
        for log in logs.into_iter() {
            match self.event_sender.try_send(log) {
                Err(mpsc::error::TrySendError::Full(_)) => {
                    dropped += 1;
                },
                Err(mpsc::error::TrySendError::Closed(_)) => {
                    return Err(SendLogBatchError {
                        dropped,
                        total,
                        skipped_from_disconnect: Some(num_left),
                    });
                },
                Ok(()) => {},
            }
            num_left -= 1;
        }
        if dropped > 0 {
            Err(SendLogBatchError {
                dropped,
                total,
                skipped_from_disconnect: None,
            })
        } else {
            Ok(())
        }
    }

    pub fn set_entitlement_enabled(&self, enabled: bool) {
        self.entitlement_enabled.store(enabled, Ordering::Relaxed)
    }

    fn get_active_sinks_count(&self) -> usize {
        self.active_sinks_count.load(Ordering::Relaxed)
    }

    fn is_entitlement_enabled(&self) -> bool {
        self.entitlement_enabled.load(Ordering::Relaxed)
    }

    fn is_active(&self) -> bool {
        self.is_entitlement_enabled() && self.get_active_sinks_count() > 0
    }
}

impl LogSender for LogManagerClient {
    fn shutdown(&self) -> anyhow::Result<()> {
        self.handle.lock().shutdown();
        Ok(())
    }

    fn send_logs(&self, logs: Vec<LogEvent>) {
        // Only route to an active log sink
        if !self.is_active() {
            return;
        }
        let total = logs.len();
        let result = self.send_logs_inner(logs);
        if let Err(e) = result {
            if let Some(skipped) = e.skipped_from_disconnect {
                metrics::log_event_dropped_disconnected_error(skipped);
                // Overflows are common so don't pollute Sentry unless something actually bad
                // happened, i.e. the LogManager events chain disconnected.
                report_error_sync(
                    &mut anyhow::Error::from(e.clone())
                        .context(ErrorMetadata::operational_internal_server_error()),
                );
                tracing::error!("Log event(s) skipped from disconnect: {e}");
            }
            metrics::log_event_dropped_overflow_error(e.dropped);
        }

        metrics::log_event_total(total);
    }
}

#[derive(Debug, Clone)]
pub struct SendLogBatchError {
    pub dropped: usize,
    pub total: usize,
    pub skipped_from_disconnect: Option<usize>, // Some iff channel disconnected
}

impl std::error::Error for SendLogBatchError {}

impl std::fmt::Display for SendLogBatchError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if let Some(num_skipped) = self.skipped_from_disconnect {
            write!(
                f,
                "Dropped {:}/{:} logs due to overflow. Skipped {:}/{:} due to disconnect.",
                self.dropped, self.total, num_skipped, self.total
            )
        } else {
            write!(
                f,
                "Dropped {:}/{:} logs due to overflow.",
                self.dropped, self.total
            )
        }
    }
}

/// The central abstraction for handling log events. This is responsible for
/// forwarding logs to LogSinks. Ideally, any shared batching/aggregation logic
/// is done here to deduplicate work at the LogSinks.
///
/// This is also responsible for selective routing of LogEvents. In the future
/// selective routing could be a feature we expose to customers but, for now, we
/// just use it for selectively filtering internal logs.
pub struct LogManager<RT: Runtime> {
    runtime: RT,
    database: Database<RT>,
    fetch_client: Arc<dyn FetchClient>,
    sinks: Arc<RwLock<BTreeMap<SinkType, LogSinkClient>>>,
    event_receiver: mpsc::Receiver<LogEvent>,
    instance_name: String,
    /// How many sinks are active right now?
    active_sinks_count: Arc<AtomicUsize>,
}

#[derive(Serialize, Debug, Clone)]
pub struct LoggingDeploymentMetadata {
    deployment_name: String,
    deployment_type: Option<DeploymentType>,
    project_name: Option<String>,
    project_slug: Option<String>,
}

impl<RT: Runtime> LogManager<RT> {
    pub async fn start(
        runtime: RT,
        database: Database<RT>,
        fetch_client: Arc<dyn FetchClient>,
        instance_name: String,
        entitlement_enabled: bool,
    ) -> LogManagerClient {
        let (req_tx, req_rx) = mpsc::channel(*knobs::LOG_MANAGER_EVENT_RECV_BUFFER_SIZE);

        let active_sinks_count = Arc::new(AtomicUsize::new(0));
        let worker = Self {
            runtime: runtime.clone(),
            database,
            fetch_client,
            // Sinks are populated from the `_log_sinks` system table on startup
            sinks: Arc::new(RwLock::new(BTreeMap::new())),
            event_receiver: req_rx,
            instance_name,
            active_sinks_count: active_sinks_count.clone(),
        };

        let handle = Arc::new(Mutex::new(runtime.spawn("log_manager", worker.go())));
        let entitlement_enabled = Arc::new(AtomicBool::new(entitlement_enabled));
        LogManagerClient {
            handle,
            event_sender: req_tx,
            active_sinks_count,
            entitlement_enabled,
        }
    }

    /// Event loop
    async fn go(mut self) {
        tracing::info!("Starting LogManager");
        let mut backoff =
            Backoff::new(consts::MANAGER_INITIAL_BACKOFF, consts::MANAGER_MAX_BACKOFF);

        // Start listening to logs
        loop {
            let Err(mut e) = self.listen().await;
            let delay = backoff.fail(&mut self.runtime.rng());
            tracing::error!("LogManager failed, sleeping {delay:?}");
            report_error(&mut e).await;
            self.runtime.wait(delay).await;
        }
    }

    async fn listen(&mut self) -> anyhow::Result<!> {
        tracing::info!("Starting listening for logs");
        let sink_startup_worker_fut = Self::sink_startup_worker(
            &self.runtime,
            &self.database,
            self.fetch_client.clone(),
            &self.sinks,
            self.instance_name.clone(),
            self.active_sinks_count.clone(),
        )
        .fuse();
        pin_mut!(sink_startup_worker_fut);

        let log_event_listener_fut =
            Self::log_event_listener(&self.runtime, &mut self.event_receiver, &self.sinks).fuse();
        pin_mut!(log_event_listener_fut);

        // We use select_biased! here to not starve sink_startup_worker.
        // log_event_listener will usually be of a much higher throughput.
        select_biased! {
            r = sink_startup_worker_fut => {
                r?;
            },
            r = log_event_listener_fut => {
                r?;
            },
        }
    }

    /// This aggregates logs by emitting logs to sinks if the receiving channel
    /// is full or if a constant aggregation interval has passed.
    ///
    /// NOTE: aggregation could happen at the sink-level or at the
    /// manager-level, but we've decided to put it here for simplicity and
    /// for slightly less overhead (no need to have timers at each sink).
    /// This can just as easily be moved to sinks if required.
    async fn log_event_listener(
        runtime: &RT,
        rx: &mut mpsc::Receiver<LogEvent>,
        sinks: &Arc<RwLock<BTreeMap<SinkType, LogSinkClient>>>,
    ) -> anyhow::Result<!> {
        loop {
            // Wait aggregation interval
            // TODO: look into mpsc implementations that allow us to check if the channel is
            // full so we don't need to sleep wastefully here.
            runtime
                .wait(Duration::from_millis(
                    *knobs::LOG_MANAGER_AGGREGATION_INTERVAL_MILLIS,
                ))
                .await;

            // Drain the receive buffer up until the max buffer size or until buffer is
            // empty
            let mut drained_events = vec![];
            let mut disconnected = false;

            let curr_buffer_size = *knobs::LOG_MANAGER_EVENT_RECV_BUFFER_SIZE;
            while drained_events.len() < curr_buffer_size {
                match rx.try_recv() {
                    Err(mpsc::error::TryRecvError::Empty) => break,
                    Err(mpsc::error::TryRecvError::Disconnected) => {
                        disconnected = true;
                        break;
                    },
                    Ok(event) => {
                        drained_events.push(Arc::new(event));
                    },
                }
            }

            if disconnected {
                anyhow::bail!("log manager receive channel closed");
            } else if !drained_events.is_empty() {
                // Route events to sinks
                metrics::log_manager_logs_received(drained_events.len());
                Self::route_event_batch(drained_events, sinks)?;
            }
        }
    }

    // Routing logic will eventually go here. For now, this is just a fanout.
    fn route_event_batch(
        events: Vec<Arc<LogEvent>>,
        sinks: &Arc<RwLock<BTreeMap<SinkType, LogSinkClient>>>,
    ) -> anyhow::Result<()> {
        let sinks = sinks.read();
        sinks
            .iter()
            .map(|(ty, v)| (ty.clone(), v.events_sender.clone(), events.clone()))
            .try_for_each(|(ty, tx, events)| {
                match tx.try_send(events) {
                    Err(mpsc::error::TrySendError::Full(_)) => {
                        // No need to report metrics here, can calculate this drop amount by the
                        // number of logs received metric for each sink
                        tracing::info!("Sink receive buffer full for {ty:?}, dropping logs.");
                        Ok(())
                    },
                    Err(mpsc::error::TrySendError::Closed(_)) => {
                        anyhow::bail!("Sink receive buffer for {ty:?} closed unexpectedly.");
                    },
                    Ok(()) => Ok(()),
                }
            })?;

        Ok(())
    }

    /// Subscribes to LOG_SINKS_TABLE and attempts to drive forward the
    /// state machine described in `SinkState`.
    ///
    /// Specifically the following logic is implemented
    /// - A sink in `Pending` state should be started and moved to the `Active`
    ///   state. Starting a sink involves invoking its `start` method and
    ///   storing the resulting LogSinkClient in the LogManager's local sink_map
    ///   so that logs will start routing.
    /// - If startup of a `Pending` sink fails, it sets its state to `Failed`,
    ///   otherwise it is set to `Active`.
    /// - A sink that is `Active` should be started. This can be violated on
    ///   backend restart so this is handled by setting the sink state to
    ///   `Pending` again.
    /// - A sink in `Tombstoned` state should have its corresponding row removed
    async fn sink_startup_worker(
        runtime: &RT,
        database: &Database<RT>,
        fetch_client: Arc<dyn FetchClient>,
        sinks: &Arc<RwLock<BTreeMap<SinkType, LogSinkClient>>>,
        instance_name: String,
        active_sinks_count: Arc<AtomicUsize>,
    ) -> anyhow::Result<!> {
        // Deployment_type is populated within the loop based on a subscription
        // to BackendInfoModel. Since deployment_type can be updated dynamically
        // during the backend's lifetime, this is important (notably from prewarm ->
        // dev|prod)
        let metadata = Arc::new(Mutex::new(LoggingDeploymentMetadata {
            deployment_name: instance_name,
            deployment_type: None,
            project_name: None,
            project_slug: None,
        }));
        loop {
            let mut tx = database.begin(Identity::system()).await?;

            let mut bi_model = BackendInfoModel::new(&mut tx);
            if let Some(bi) = bi_model.get().await? {
                let bi = bi.into_value();
                let mut metadata_guard = metadata.lock();
                metadata_guard.deployment_type = Some(bi.deployment_type);
                metadata_guard.project_name = bi.project_name;
                metadata_guard.project_slug = bi.project_slug;
            }

            let mut log_sinks_model = LogSinksModel::new(&mut tx);
            let sink_rows = log_sinks_model.get_all().await?;
            Self::sink_startup_worker_once(
                runtime,
                database,
                fetch_client.clone(),
                sink_rows,
                sinks,
                active_sinks_count.clone(),
                metadata.clone(),
            )
            .await?;

            // Wait for changes to the table
            let token = tx.into_token()?;
            let subscription = database.subscribe(token).await?;
            subscription.wait_for_invalidation().await;
        }
    }

    async fn sink_startup_worker_once(
        runtime: &RT,
        database: &Database<RT>,
        fetch_client: Arc<dyn FetchClient>,
        sink_rows: Vec<ParsedDocument<LogSinksRow>>,
        sinks: &Arc<RwLock<BTreeMap<SinkType, LogSinkClient>>>,
        active_sinks_count: Arc<AtomicUsize>,
        metadata: Arc<Mutex<LoggingDeploymentMetadata>>,
    ) -> anyhow::Result<()> {
        let (pending_sinks, inactive_sinks, tombstoned_sinks) = {
            let sinks = sinks.read();
            let pending_sinks: Vec<_> = sink_rows
                .iter()
                .filter(|row| matches!(row.status, SinkState::Pending))
                .cloned()
                .collect();
            let inactive_sinks: Vec<_> = sink_rows
                .iter()
                .filter(|row| {
                    matches!(row.status, SinkState::Active)
                        && !sinks.contains_key(&row.config.sink_type())
                })
                .cloned()
                .collect();
            let tombstoned_sinks: Vec<_> = sink_rows
                .iter()
                .filter(|row| matches!(row.status, SinkState::Tombstoned))
                .cloned()
                .collect();
            (pending_sinks, inactive_sinks, tombstoned_sinks)
        };

        let mut tx = database.begin(Identity::system()).await?;

        // Order is important! Handle any `Tombstoned` rows first in case they were
        // replaced with a new LogSink for the same provider that's in state `Pending`.
        //
        // We could potentially drop events between removing the `Tombstoned` and
        // activating the new `Pending` provider, but this would happen
        // with the user deleting + re-adding the provider manually.
        //
        // There also might be multiple `Tombstoned` sinks in the case of multiple
        // updates to a sink before we reach this loop to clean them up.

        // Delete `Tombstoned` rows
        tracing::info!(
            "Found {:} sink(s) in `Tombstoned` state",
            tombstoned_sinks.len()
        );
        for row in tombstoned_sinks {
            tracing::info!("Deleting row {}", row.config);
            SystemMetadataModel::new_global(&mut tx)
                .delete(row.id())
                .await?;

            // Remove locally
            if sinks.write().remove(&row.config.sink_type()).is_some() {
                active_sinks_count.fetch_sub(1, Ordering::Relaxed);
            }
        }

        // Startup `Pending` sinks
        let mut model = LogSinksModel::new(&mut tx);
        tracing::info!("Found {:} sink(s) in Pending state.", pending_sinks.len());
        for row in pending_sinks {
            tracing::info!("Starting log sink {}", row.config);

            let sink_type = row.config.sink_type();
            let sink_id = row.id();
            let sink_config = row.config.clone();
            let timed_startup_result = runtime
                .with_timeout(
                    "sink startup timeout",
                    consts::SINK_STARTUP_TIMEOUT,
                    Self::config_to_log_sink_client(
                        runtime,
                        fetch_client.clone(),
                        sink_config,
                        metadata.clone(),
                    ),
                )
                .await;

            match timed_startup_result {
                Err(mut e) => {
                    let reason = e.user_facing_message();
                    tracing::error!("Moving sink {sink_type:?} to Failed state. Reason: {reason}");
                    report_error(&mut e).await;

                    model
                        .patch_status(sink_id, SinkState::Failed { reason })
                        .await?;
                },
                Ok(sink_client) => {
                    model.patch_status(sink_id, SinkState::Active {}).await?;

                    // Save locally
                    sinks.write().insert(sink_type, sink_client);
                    // Ordering::Relaxed is okay here since logs don't need to be
                    // picked up immediately by an active sink and they can start
                    // a little earlier as well. Reordering isn't a big deal.
                    active_sinks_count.fetch_add(1, Ordering::Relaxed);
                },
            }
        }

        // Set inactive rows to `Pending`
        tracing::info!("Found {:} inactive sink(s).", inactive_sinks.len());
        for row in inactive_sinks {
            tracing::info!(
                "Found log sink with Active status that has not started. Updating to Pending \
                 status and restarting: {}",
                row.config
            );
            model.patch_status(row.id(), SinkState::Pending {}).await?;
        }

        // Commit
        database
            .commit_with_write_source(tx, "log_sink_worker")
            .await?;
        Ok(())
    }

    /// Starts the LogSinkClient for this config
    async fn config_to_log_sink_client(
        runtime: &RT,
        fetch_client: Arc<dyn FetchClient>,
        config: SinkConfig,
        metadata: Arc<Mutex<LoggingDeploymentMetadata>>,
    ) -> anyhow::Result<LogSinkClient> {
        match config {
            SinkConfig::Local(path) => LocalSink::start(runtime.clone(), path.parse()?).await,
            SinkConfig::Datadog(config) => {
                DatadogSink::start(runtime.clone(), fetch_client, config, metadata).await
            },
            SinkConfig::Webhook(config) => {
                WebhookSink::start(runtime.clone(), config, fetch_client, metadata).await
            },
            SinkConfig::Axiom(config) => {
                AxiomSink::start(runtime.clone(), config, fetch_client, metadata).await
            },
            SinkConfig::Sentry(config) => {
                SentrySink::start(runtime.clone(), config, None, metadata.clone()).await
            },
            #[cfg(any(test, feature = "testing"))]
            SinkConfig::Mock | SinkConfig::Mock2 => MockSink::start(runtime.clone()).await,
        }
    }
}

pub struct LogSinkClient {
    _handle: Arc<Mutex<Box<dyn SpawnHandle>>>,
    events_sender: mpsc::Sender<Vec<Arc<LogEvent>>>,
}

#[cfg(test)]
mod tests {
    use std::{
        assert_matches::assert_matches,
        collections::BTreeMap,
        sync::{
            atomic::{
                AtomicUsize,
                Ordering,
            },
            Arc,
        },
    };

    use common::{
        document::ParsedDocument,
        http::fetch::StaticFetchClient,
        types::DeploymentType,
    };
    use database::{
        test_helpers::DbFixtures,
        Database,
    };
    use model::{
        log_sinks::{
            types::{
                LogSinksRow,
                SinkConfig,
                SinkState,
                SinkType,
            },
            LogSinksModel,
        },
        test_helpers::DbFixturesWithModel,
    };
    use parking_lot::{
        Mutex,
        RwLock,
    };
    use runtime::testing::TestRuntime;

    use crate::{
        LogManager,
        LoggingDeploymentMetadata,
    };

    async fn setup_log_sinks(
        db: &Database<TestRuntime>,
        sink_configs: Vec<SinkConfig>,
    ) -> anyhow::Result<Vec<ParsedDocument<LogSinksRow>>> {
        let mut tx = db.begin_system().await?;
        let mut model = LogSinksModel::new(&mut tx);
        for config in sink_configs {
            model.add_or_update(config).await?;
        }
        db.commit(tx).await?;

        let mut tx = db.begin_system().await?;
        let mut model = LogSinksModel::new(&mut tx);
        model.get_all().await
    }

    #[convex_macro::test_runtime]
    async fn test_adds_sinks(rt: TestRuntime) -> anyhow::Result<()> {
        let db = DbFixtures::new_with_model(&rt).await?.db;
        let fetch_client = Arc::new(StaticFetchClient::new());
        let sinks = Arc::new(RwLock::new(BTreeMap::new()));
        let active_sinks_count = Arc::new(AtomicUsize::new(0));
        let metadata = Arc::new(Mutex::new(LoggingDeploymentMetadata {
            deployment_name: "carnitas".to_string(),
            deployment_type: Some(DeploymentType::Dev),
            project_name: Some("test".to_string()),
            project_slug: Some("test".to_string()),
        }));
        let sink_rows = setup_log_sinks(&db, vec![SinkConfig::Mock, SinkConfig::Mock2]).await?;

        LogManager::sink_startup_worker_once(
            &rt,
            &db,
            fetch_client,
            sink_rows,
            &sinks,
            active_sinks_count.clone(),
            metadata,
        )
        .await?;
        assert_eq!(active_sinks_count.load(Ordering::Relaxed), 2);
        let active_sink_types = sinks.read().keys().cloned().collect::<Vec<_>>();
        assert_eq!(active_sink_types.len(), 2);
        assert!(active_sink_types.contains(&SinkType::Mock));
        assert!(active_sink_types.contains(&SinkType::Mock2));
        Ok(())
    }

    #[convex_macro::test_runtime]
    async fn test_update_sink(rt: TestRuntime) -> anyhow::Result<()> {
        let db = DbFixtures::new_with_model(&rt).await?.db;
        let fetch_client = Arc::new(StaticFetchClient::new());
        let sinks = Arc::new(RwLock::new(BTreeMap::new()));
        let active_sinks_count = Arc::new(AtomicUsize::new(0));
        let metadata = Arc::new(Mutex::new(LoggingDeploymentMetadata {
            deployment_name: "carnitas".to_string(),
            deployment_type: Some(DeploymentType::Dev),
            project_name: Some("test".to_string()),
            project_slug: Some("test".to_string()),
        }));
        let sink_rows = setup_log_sinks(&db, vec![SinkConfig::Mock, SinkConfig::Mock2]).await?;

        LogManager::sink_startup_worker_once(
            &rt,
            &db,
            fetch_client.clone(),
            sink_rows,
            &sinks,
            active_sinks_count.clone(),
            metadata.clone(),
        )
        .await?;

        // update Mock
        let sink_rows = setup_log_sinks(&db, vec![SinkConfig::Mock]).await?;
        assert_eq!(sink_rows.len(), 3);

        LogManager::sink_startup_worker_once(
            &rt,
            &db,
            fetch_client,
            sink_rows,
            &sinks,
            active_sinks_count.clone(),
            metadata,
        )
        .await?;

        // We still have two sinks running
        assert_eq!(active_sinks_count.load(Ordering::Relaxed), 2);
        let active_sink_types = sinks.read().keys().cloned().collect::<Vec<_>>();
        assert_eq!(active_sink_types.len(), 2);
        assert!(active_sink_types.contains(&SinkType::Mock));
        assert!(active_sink_types.contains(&SinkType::Mock2));

        let mut tx = db.begin_system().await?;
        let sink_rows = LogSinksModel::new(&mut tx).get_all().await?;
        // Tombstoned row is gone
        assert_eq!(sink_rows.len(), 2);
        assert_matches!(sink_rows[0].status, SinkState::Active);
        assert_matches!(sink_rows[1].status, SinkState::Active);
        Ok(())
    }

    #[convex_macro::test_runtime]
    async fn test_update_sink_multiple(rt: TestRuntime) -> anyhow::Result<()> {
        let db = DbFixtures::new_with_model(&rt).await?.db;
        let fetch_client = Arc::new(StaticFetchClient::new());
        let sinks = Arc::new(RwLock::new(BTreeMap::new()));
        let active_sinks_count = Arc::new(AtomicUsize::new(0));
        let metadata = Arc::new(Mutex::new(LoggingDeploymentMetadata {
            deployment_name: "carnitas".to_string(),
            deployment_type: Some(DeploymentType::Dev),
            project_name: Some("test".to_string()),
            project_slug: Some("test".to_string()),
        }));
        let sink_rows = setup_log_sinks(&db, vec![SinkConfig::Mock, SinkConfig::Mock2]).await?;

        LogManager::sink_startup_worker_once(
            &rt,
            &db,
            fetch_client.clone(),
            sink_rows,
            &sinks,
            active_sinks_count.clone(),
            metadata.clone(),
        )
        .await?;

        // update Mock twice (e.g. user making two updates before LogManager has
        // processed them)
        setup_log_sinks(&db, vec![SinkConfig::Mock]).await?;
        let sink_rows = setup_log_sinks(&db, vec![SinkConfig::Mock]).await?;
        assert_eq!(sink_rows.len(), 4);

        LogManager::sink_startup_worker_once(
            &rt,
            &db,
            fetch_client,
            sink_rows,
            &sinks,
            active_sinks_count.clone(),
            metadata,
        )
        .await?;

        // We still have two sinks running
        assert_eq!(active_sinks_count.load(Ordering::Relaxed), 2);
        let active_sink_types = sinks.read().keys().cloned().collect::<Vec<_>>();
        assert_eq!(active_sink_types.len(), 2);
        assert!(active_sink_types.contains(&SinkType::Mock));
        assert!(active_sink_types.contains(&SinkType::Mock2));

        let mut tx = db.begin_system().await?;
        let sink_rows = LogSinksModel::new(&mut tx).get_all().await?;
        // Tombstoned row is gone
        assert_eq!(sink_rows.len(), 2);
        assert_matches!(sink_rows[0].status, SinkState::Active);
        assert_matches!(sink_rows[1].status, SinkState::Active);
        Ok(())
    }
}

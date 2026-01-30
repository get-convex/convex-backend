#![feature(iterator_try_collect)]

use std::{
    collections::BTreeMap,
    ops::Deref,
    sync::{
        Arc,
        LazyLock,
    },
    time::Duration,
};

use cmd_util::env::env_config;
use common::{
    backoff::Backoff,
    components::ComponentPath,
    errors::report_error,
    fastrace_helpers::get_sampled_span,
    knobs::USAGE_TRACKING_WORKER_SLOW_TRACE_THRESHOLD,
    log_streaming::{
        LogEvent,
        LogSender,
        StructuredLogEvent,
    },
    runtime::{
        shutdown_and_join,
        Runtime,
        SpawnHandle,
    },
};
use database::{
    Database,
    DatabaseSnapshot,
    SearchNotEnabled,
    TablesUsage,
};
use events::usage::{
    TableDocumentCount,
    TableTextStorage,
    TableVectorStorage,
    UsageEvent::{
        CurrentDocumentCounts,
        CurrentFileStorage,
        CurrentTextStorage,
        CurrentVectorStorage,
    },
    UsageEventLogger,
};
use fastrace::future::FutureExt as _;
use itertools::{
    Either,
    Itertools,
};
use keybroker::Identity;
use model::{
    exports::ExportsModel,
    file_storage::get_total_file_storage_size,
    virtual_system_mapping,
};
use parking_lot::Mutex;
use rand::Rng;
use usage_tracking::FunctionUsageTracker;
use value::TableName;

mod metrics;

const INITIAL_BACKOFF: Duration = Duration::from_secs(1);
const MAX_BACKOFF: Duration = Duration::from_secs(900); // 15 minutes
static RUN_PERIOD: LazyLock<Duration> =
    LazyLock::new(|| Duration::from_secs(env_config("USAGE_TRACKING_PERIOD_SECS", 60 * 60)));

#[derive(Clone)]
pub struct UsageGaugesTrackingWorker {
    worker: Arc<Mutex<Option<Box<dyn SpawnHandle>>>>,
}

struct UsageGaugesTrackingWorkerInner<RT: Runtime> {
    runtime: RT,
    database: Database<RT>,
    backoff: Backoff,
    usage_logger: Arc<dyn UsageEventLogger>,
    log_sender: Arc<dyn LogSender>,
    instance_name: String,
}

impl UsageGaugesTrackingWorker {
    pub fn start<RT: Runtime>(
        runtime: RT,
        database: Database<RT>,
        usage_logger: Arc<dyn UsageEventLogger>,
        log_sender: Arc<dyn LogSender>,
        instance_name: String,
    ) -> Self {
        let mut worker = UsageGaugesTrackingWorkerInner {
            runtime: runtime.clone(),
            database,
            backoff: Backoff::new(INITIAL_BACKOFF, MAX_BACKOFF),
            usage_logger,
            log_sender,
            instance_name: instance_name.clone(),
        };
        let worker_handle = Arc::new(Mutex::new(Some(runtime.spawn(
            "usage_gauges_tracking_worker",
            async move {
                tracing::info!("Starting UsageGaugesTrackingWorker");
                loop {
                    if let Err(e) = worker.run_once().await {
                        report_error(&mut e.context("UsageGaugesTrackingWorker died")).await;
                        let delay = worker.backoff.fail(&mut worker.runtime.rng());
                        worker.runtime.wait(delay).await;
                    } else {
                        worker.backoff.reset();
                    }
                }
            },
        ))));
        Self {
            worker: worker_handle,
        }
    }

    pub async fn shutdown(&self) -> anyhow::Result<()> {
        let handle = self.worker.lock().take();
        if let Some(handle) = handle {
            shutdown_and_join(handle).await?;
        }
        Ok(())
    }
}

impl<RT: Runtime> UsageGaugesTrackingWorkerInner<RT> {
    async fn run_once(&mut self) -> anyhow::Result<()> {
        // Splay over RUN_PERIOD to spread load. Splay first to make sure it happens on
        // startup.
        let to_wait = (*RUN_PERIOD / 2) + RUN_PERIOD.mul_f64(self.runtime.rng().random());
        tracing::debug!("UsageGaugesTrackingWorker waiting for {to_wait:?}.");
        self.runtime.wait(to_wait).await;
        tracing::debug!("UsageGaugesTrackingWorker waking up.");
        let root = get_sampled_span(
            &self.instance_name,
            "usage_tracking_worker/send_usage",
            &mut self.runtime.rng(),
        );
        self.send_usage().in_span(root).await?;
        Ok(())
    }

    // We ignore server errors and we do not recover logs on other failures.
    #[fastrace::trace]
    async fn send_usage(&mut self) -> anyhow::Result<()> {
        if !self.database.has_table_summaries_bootstrapped() {
            tracing::warn!("Skipping usage tracking because table summaries are not bootstrapped");
            return Ok(());
        }
        let timer = metrics::usage_gauges_tracking_worker_timer();

        let gauge_metrics = get_gauge_metrics(
            &Identity::system(),
            &self.database.latest_database_snapshot()?,
        )
        .await?;

        self.send_usage_events(gauge_metrics).await;
        let duration = timer.finish();
        if duration > *USAGE_TRACKING_WORKER_SLOW_TRACE_THRESHOLD {
            tracing::warn!("Usage tracking worker took longer than expected: {duration:?}");
        }
        Ok(())
    }

    /// Send usage events as the current state of the world to the firehose.
    #[fastrace::trace]
    async fn send_usage_events(&self, gauge_metrics: GaugeMetrics) {
        // Send to log streams if available
        let log_sender = &self.log_sender;
        let totals = gauge_metrics.compute_totals();
        let log_event = LogEvent {
            timestamp: self.runtime.unix_timestamp(),
            event: StructuredLogEvent::CurrentStorageUsage {
                total_document_size_bytes: totals.total_document_size,
                total_index_size_bytes: totals.total_index_size,
                total_vector_storage_bytes: totals.total_vector_storage,
                total_file_storage_bytes: totals.total_file_storage,
                total_backup_storage_bytes: totals.total_backup_storage,
            },
        };
        log_sender.send_logs(vec![log_event]);

        // Send to usage event logger (firehose)
        let GaugeMetrics {
            document_and_index_storage,
            vector_index_storage,
            text_index_storage,
            storage_total_size,
            cloud_snapshot_total_size,
            document_counts,
        } = gauge_metrics;

        let (user_document_counts, system_document_counts) = document_counts
            .into_iter()
            .partition_map(|(component_path, table_name, num_documents)| {
                let count = TableDocumentCount {
                    component_path: component_path.serialize(),
                    table_name: table_name.deref().into(),
                    num_documents,
                };
                if table_name.is_system() {
                    Either::Right(count)
                } else {
                    Either::Left(count)
                }
            });

        let events = vec![
            document_and_index_storage.into(),
            CurrentVectorStorage {
                tables: vector_index_storage
                    .into_iter()
                    .map(|((component_path, table_name), size)| TableVectorStorage {
                        component_path: component_path.serialize(),
                        table_name: table_name.deref().into(),
                        size,
                    })
                    .collect(),
            },
            CurrentTextStorage {
                tables: text_index_storage
                    .into_iter()
                    .map(|((component_path, table_name), size)| TableTextStorage {
                        component_path: component_path.serialize(),
                        table_name: table_name.deref().into(),
                        size,
                    })
                    .collect(),
            },
            CurrentFileStorage {
                tag: "dummy_tag".to_string(),
                total_size: storage_total_size + cloud_snapshot_total_size,
                total_user_file_size: storage_total_size,
                total_cloud_backup_size: cloud_snapshot_total_size,
                // NOTE: this is no longer used since we merged snapshot exports and cloud backups
                total_snapshot_export_size: 0,
            },
            CurrentDocumentCounts {
                tables: user_document_counts,
                system_tables: system_document_counts,
            },
        ];

        self.usage_logger.record_async(events).await
    }
}

#[derive(Debug)]
pub struct GaugeMetrics {
    document_and_index_storage: TablesUsage,
    vector_index_storage: BTreeMap<(ComponentPath, TableName), u64>,
    text_index_storage: BTreeMap<(ComponentPath, TableName), u64>,
    storage_total_size: u64,
    cloud_snapshot_total_size: u64,
    document_counts: Vec<(ComponentPath, TableName, u64)>,
}

impl GaugeMetrics {
    /// Compute aggregated storage usage totals from the gauge metrics
    /// Only includes user tables, excluding system tables
    pub fn compute_totals(&self) -> AggregatedStorageUsage {
        let TablesUsage {
            user_tables,
            system_tables: _,
            orphaned_tables: _,
            virtual_tables: _,
        } = &self.document_and_index_storage;
        // Aggregate user tables tables for document and index storage
        let (total_document_size, total_index_size) = user_tables.values().fold(
            (0u64, 0u64),
            |(doc_acc, idx_acc), (usage, _component_path)| {
                (doc_acc + usage.document_size, idx_acc + usage.index_size)
            },
        );

        // Aggregate all vector storage (no vector data in system tables)
        let total_vector_storage = self.vector_index_storage.values().sum();

        // Only count user table documents
        let total_document_count = self
            .document_counts
            .iter()
            .filter(|(_, table_name, _)| !table_name.is_system())
            .map(|(_, _, count)| count)
            .sum();

        AggregatedStorageUsage {
            total_document_size,
            total_index_size,
            total_vector_storage,
            total_file_storage: self.storage_total_size,
            total_backup_storage: self.cloud_snapshot_total_size,
            total_document_count,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct AggregatedStorageUsage {
    pub total_document_size: u64,
    pub total_index_size: u64,
    pub total_vector_storage: u64,
    pub total_file_storage: u64,
    pub total_backup_storage: u64,
    pub total_document_count: u64,
}

#[fastrace::trace]
pub async fn get_gauge_metrics<RT: Runtime>(
    identity: &Identity,
    database: &DatabaseSnapshot<RT>,
) -> anyhow::Result<GaugeMetrics> {
    let document_and_index_storage = database.get_document_and_index_storage(identity)?;
    let vector_index_storage = database.get_vector_index_storage(identity)?;
    let text_index_storage = database.get_text_index_storage(identity)?;
    let cloud_snapshot_total_size = fetch_cloud_snapshot_total_size(identity, database).await?;
    let document_counts = database.get_document_counts(identity)?;
    let storage_total_size = get_total_file_storage_size(identity, database).await?;

    Ok(GaugeMetrics {
        document_and_index_storage,
        vector_index_storage,
        text_index_storage,
        storage_total_size,
        cloud_snapshot_total_size,
        document_counts,
    })
}

#[fastrace::trace]
async fn fetch_cloud_snapshot_total_size<RT: Runtime>(
    identity: &Identity,
    database: &DatabaseSnapshot<RT>,
) -> anyhow::Result<u64> {
    let mut tx = database.begin_tx(
        identity.clone(),
        Arc::new(SearchNotEnabled),
        FunctionUsageTracker::new(),
        virtual_system_mapping().clone(),
    )?;
    let mut model = ExportsModel::new(&mut tx);
    let cloud_snapshots = model.list_unexpired_cloud_backups().await?;
    let cloud_snapshot_total_size = cloud_snapshots
        .into_iter()
        .filter_map(|snapshot| match snapshot.into_value() {
            model::exports::types::Export::Completed { size, .. } => Some(size),
            _ => None,
        })
        .sum();
    Ok(cloud_snapshot_total_size)
}

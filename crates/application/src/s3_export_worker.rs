use std::{
    sync::Arc,
    time::{
        Duration,
        SystemTime,
    },
};

use common::{
    backoff::Backoff,
    errors::report_error,
    execution_context::ExecutionId,
    fastrace_helpers::get_sampled_span,
    runtime::Runtime,
    types::UdfIdentifier,
    RequestId,
};
use database::Database;
use exports::interface::ExportProvider;
use fastrace::future::FutureExt as _;
use keybroker::Identity;
use model::{
    log_sinks::{
        types::{
            s3_export::SyncPeriod,
            SinkConfig,
            SinkType,
        },
        LogSinksModel,
    },
    s3_export_state::{
        types::{
            S3ExportCheckpoint,
            S3ExportProgress,
        },
        S3ExportStateModel,
    },
};
use streaming_export::{
    managed::{
        SyncDestination,
        SyncFormat,
    },
    SyncStatus,
};
use usage_tracking::{
    CallType,
    UsageCounter,
};

use crate::metrics::{
    log_s3_export_worker_failed,
    s3_export_sync_timer,
    S3ExportGauges,
};

const INITIAL_BACKOFF: Duration = Duration::from_secs(1);
const MAX_BACKOFF: Duration = Duration::from_secs(60);
const CONTINUOUS_POLL_INTERVAL: Duration = Duration::from_secs(5);

pub struct S3ExportWorker<RT: Runtime> {
    runtime: RT,
    database: Database<RT>,
    instance_name: String,
    export_provider: Arc<dyn ExportProvider<RT>>,
    gauges: S3ExportGauges,
    usage_counter: UsageCounter,
}

impl<RT: Runtime> S3ExportWorker<RT> {
    pub fn new(
        runtime: RT,
        database: Database<RT>,
        instance_name: String,
        export_provider: Arc<dyn ExportProvider<RT>>,
        usage_counter: UsageCounter,
    ) -> Self {
        Self {
            runtime,
            database,
            gauges: S3ExportGauges::new(instance_name.clone()),
            instance_name,
            export_provider,
            usage_counter,
        }
    }

    pub async fn run(self) {
        let mut backoff = Backoff::new(INITIAL_BACKOFF, MAX_BACKOFF);
        loop {
            match self.run_once().await {
                Ok(()) => backoff.reset(),
                Err(error) => {
                    log_s3_export_worker_failed(&error);
                    report_error(&mut error.context("S3ExportWorker failed")).await;
                    let delay = backoff.fail(&mut self.runtime.rng());
                    self.runtime.wait(delay).await;
                },
            }
        }
    }

    async fn run_once(&self) -> anyhow::Result<()> {
        let mut tx = self.database.begin(Identity::system()).await?;
        let export = LogSinksModel::new(&mut tx)
            .get_by_provider(SinkType::S3Export)
            .await?;
        let Some(export) = export else {
            self.gauges.clear();
            let token = tx.into_token()?;
            self.database
                .subscribe_and_wait_for_invalidation(token)
                .await?;
            return Ok(());
        };
        let SinkConfig::S3Export(config) = &export.config else {
            anyhow::bail!("S3 export document has a different configuration type");
        };
        // A page that hangs keeps aging past this.
        self.gauges.start(self.runtime.system_time());
        let state = S3ExportStateModel::new(&mut tx)
            .get(export.developer_id())
            .await?;
        let previous_checkpoint = state.as_ref().map(|state| &state.checkpoint);
        // Keep the configuration read in the checkpoint transaction: OCC also
        // detects edits that restore the original values while the page is in flight.
        let page = {
            let database_snapshot = self.database.latest_database_snapshot()?;
            let timer = s3_export_sync_timer();
            let span = get_sampled_span(
                &self.instance_name,
                "s3_export_worker/sync",
                &mut self.runtime.rng(),
            );
            let page = self
                .export_provider
                .sync(
                    &self.instance_name,
                    &database_snapshot,
                    SyncDestination::ByoAwsBucket {
                        bucket: config.bucket.clone(),
                        region: config.region.clone(),
                        prefix: config.prefix.clone(),
                        endpoint_url: None,
                        access_key_id: config.access_key_id.0.clone(),
                        secret_access_key: config.secret_access_key.0.clone(),
                    },
                    SyncFormat::Iceberg,
                    &config.selection,
                    &export.developer_id().to_string(),
                )
                .in_span(span)
                .await?;
            timer.finish();
            page
        };
        // Porter commits the page in the destination before returning it, so
        // a returned page is a billed page. The progress checkpoint below is
        // for the dashboard and can lose an OCC race without unbilling anything.
        self.usage_counter
            .track_call(
                UdfIdentifier::SystemJob("s3_export".to_string()),
                ExecutionId::new(),
                RequestId::new(),
                CallType::Export,
                true,
                page.usage.clone(),
            )
            .await;
        // The page is already committed in the destination, so losing this
        // progress update to a concurrent configuration edit loses no data.
        let checkpoint = S3ExportCheckpoint {
            progress: export_progress(&page.status)?,
        };
        if !S3ExportStateModel::new(&mut tx)
            .advance(
                export.developer_id(),
                previous_checkpoint,
                checkpoint.clone(),
            )
            .await?
        {
            return Ok(());
        }
        if previous_checkpoint != Some(&checkpoint) {
            self.database
                .commit_with_write_source(tx, "s3_export_worker_record_progress")
                .await?;
        } else {
            drop(tx);
        }
        let interval = sync_interval(config.period);
        self.gauges.set_next_sync_due(next_sync_due(
            &page.status,
            interval,
            self.runtime.system_time(),
        ));
        self.gauges.set_snapshot_tables_synced(match &page.status {
            SyncStatus::Snapshotting { progress } => Some(progress.num_tables_synced),
            SyncStatus::Stale { .. } | SyncStatus::UpToDate { .. } => None,
        });

        if matches!(&page.status, SyncStatus::UpToDate { .. }) {
            let mut tx = self.database.begin(Identity::system()).await?;
            let current = LogSinksModel::new(&mut tx)
                .get_by_provider(SinkType::S3Export)
                .await?;
            if current.is_none_or(|current| {
                current.developer_id() != export.developer_id() || current.config != export.config
            }) {
                return Ok(());
            }
            let current_state = S3ExportStateModel::new(&mut tx)
                .get(export.developer_id())
                .await?;
            if current_state.is_none_or(|current| current.checkpoint != checkpoint) {
                return Ok(());
            }
            let next_write_ts = (*self.database.now_ts_for_reads()).succ()?;
            let token = tx.into_token()?;
            let invalidated = self.database.subscribe_and_wait_for_invalidation(token);
            match interval {
                None => {
                    tokio::select! {
                        result = invalidated => { result?; },
                        _ = self.database.wait_for_write_ts(next_write_ts) => {},
                        _ = self.runtime.wait(CONTINUOUS_POLL_INTERVAL) => {},
                    }
                },
                Some(interval) => {
                    tokio::select! {
                        result = invalidated => { result?; },
                        _ = self.runtime.wait(interval) => {},
                    }
                },
            }
        }
        Ok(())
    }
}

fn sync_interval(period: SyncPeriod) -> Option<Duration> {
    match period {
        SyncPeriod::Continuous => None,
        SyncPeriod::Hourly => Some(Duration::from_secs(60 * 60)),
        SyncPeriod::Daily => Some(Duration::from_secs(24 * 60 * 60)),
    }
}

/// A caught-up export sleeps for the sync interval; every other page is
/// followed by another right away. Wall-clock time, because an idle
/// deployment's snapshot timestamp only moves with the
/// `MAX_REPEATABLE_TIMESTAMP_IDLE_FREQUENCY` heartbeat.
fn next_sync_due(status: &SyncStatus, interval: Option<Duration>, now: SystemTime) -> SystemTime {
    match status {
        SyncStatus::UpToDate { .. } => now + interval.unwrap_or_default(),
        SyncStatus::Snapshotting { .. } | SyncStatus::Stale { .. } => now,
    }
}

fn export_progress(status: &SyncStatus) -> anyhow::Result<S3ExportProgress> {
    Ok(match status {
        SyncStatus::Snapshotting { progress } => S3ExportProgress::Snapshotting {
            num_tables_synced: progress.num_tables_synced.try_into()?,
            total_tables: progress.total_tables.try_into()?,
            current_component: progress.current_component.to_string(),
            current_table: progress.current_table.to_string(),
            num_documents_in_current_table: progress.num_documents_in_current_table.try_into()?,
            total_documents_in_current_table: progress
                .total_documents_in_current_table
                .map(i64::try_from)
                .transpose()?,
            num_documents_synced: progress.num_documents_synced.try_into()?,
            total_documents: progress.total_documents.map(i64::try_from).transpose()?,
        },
        SyncStatus::Stale { ts } => S3ExportProgress::Stale { ts: (*ts).into() },
        SyncStatus::UpToDate { ts } => S3ExportProgress::UpToDate { ts: (*ts).into() },
    })
}

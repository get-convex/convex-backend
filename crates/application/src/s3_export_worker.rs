use std::{
    sync::Arc,
    time::Duration,
};

use common::{
    backoff::Backoff,
    errors::report_error,
    runtime::Runtime,
};
use database::Database;
use exports::interface::ExportProvider;
use keybroker::Identity;
use model::log_sinks::{
    types::{
        s3_export::{
            S3ExportProgress,
            SyncPeriod,
        },
        SinkConfig,
        SinkType,
    },
    LogSinksModel,
};
use streaming_export::{
    managed::{
        SyncDestination,
        SyncFormat,
    },
    SyncStatus,
};

const INITIAL_BACKOFF: Duration = Duration::from_secs(1);
const MAX_BACKOFF: Duration = Duration::from_secs(60);
const CONTINUOUS_POLL_INTERVAL: Duration = Duration::from_secs(5);

pub struct S3ExportWorker<RT: Runtime> {
    runtime: RT,
    database: Database<RT>,
    instance_name: String,
    export_provider: Arc<dyn ExportProvider<RT>>,
}

impl<RT: Runtime> S3ExportWorker<RT> {
    pub fn new(
        runtime: RT,
        database: Database<RT>,
        instance_name: String,
        export_provider: Arc<dyn ExportProvider<RT>>,
    ) -> Self {
        Self {
            runtime,
            database,
            instance_name,
            export_provider,
        }
    }

    pub async fn run(self) {
        let mut backoff = Backoff::new(INITIAL_BACKOFF, MAX_BACKOFF);
        loop {
            match self.run_once().await {
                Ok(()) => backoff.reset(),
                Err(error) => {
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
        let token = tx.into_token()?;
        let Some(export) = export else {
            self.database
                .subscribe_and_wait_for_invalidation(token)
                .await?;
            return Ok(());
        };
        let SinkConfig::S3Export(config) = &export.config else {
            anyhow::bail!("S3 export document has a different configuration type");
        };
        let previous_cursor = config.cursor.as_deref();
        let page = self
            .export_provider
            .sync(
                &self.instance_name,
                &self.database.latest_database_snapshot()?,
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
                config.cursor.clone(),
            )
            .await?;
        anyhow::ensure!(
            !page.cursor.is_empty(),
            "Export provider returned an empty S3 export cursor"
        );
        if page.cursor == previous_cursor.unwrap_or_default()
            && !matches!(&page.status, SyncStatus::UpToDate { .. })
        {
            anyhow::bail!(
                "Export provider returned an unchanged cursor before the export caught up"
            );
        }

        let mut tx = self.database.begin(Identity::system()).await?;
        let progress = export_progress(&page.status)?;
        if !LogSinksModel::new(&mut tx)
            .advance_s3_export(
                export.developer_id(),
                config,
                page.cursor.clone(),
                progress.clone(),
            )
            .await?
        {
            return Ok(());
        }
        if page.cursor != previous_cursor.unwrap_or_default()
            || config.progress.as_ref() != Some(&progress)
        {
            self.database
                .commit_with_write_source(tx, "s3_export_worker_advance_cursor")
                .await?;
        }

        if matches!(&page.status, SyncStatus::UpToDate { .. }) {
            let mut tx = self.database.begin(Identity::system()).await?;
            let current = LogSinksModel::new(&mut tx)
                .get_by_provider(SinkType::S3Export)
                .await?;
            let mut updated_config = config.clone();
            updated_config.cursor = Some(page.cursor.clone());
            updated_config.progress = Some(progress);
            let current_is_same_export = current.is_some_and(|current| {
                current.developer_id() == export.developer_id()
                    && current.config == SinkConfig::S3Export(updated_config)
            });
            if !current_is_same_export {
                return Ok(());
            }
            let next_write_ts = (*self.database.now_ts_for_reads()).succ()?;
            let token = tx.into_token()?;
            let interval = match config.period {
                SyncPeriod::Continuous => None,
                SyncPeriod::Hourly => Some(Duration::from_secs(60 * 60)),
                SyncPeriod::Daily => Some(Duration::from_secs(24 * 60 * 60)),
            };
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

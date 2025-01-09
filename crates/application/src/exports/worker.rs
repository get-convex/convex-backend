use std::{
    sync::Arc,
    time::Duration,
};

use anyhow::Context;
use common::{
    self,
    backoff::Backoff,
    components::ComponentPath,
    document::ParsedDocument,
    errors::report_error,
    execution_context::ExecutionId,
    pause::PauseClient,
    runtime::Runtime,
    types::UdfIdentifier,
    RequestId,
};
use database::{
    Database,
    SystemMetadataModel,
};
use futures::{
    Future,
    FutureExt,
};
use keybroker::Identity;
use model::exports::{
    types::{
        Export,
        ExportRequestor,
    },
    ExportsModel,
};
use storage::Storage;
use usage_tracking::{
    CallType,
    FunctionUsageTracker,
    StorageCallTracker,
    UsageCounter,
};

use crate::{
    exports::{
        export_inner,
        metrics::export_timer,
    },
    metrics::log_worker_starting,
};

const INITIAL_BACKOFF: Duration = Duration::from_secs(1);
const MAX_BACKOFF: Duration = Duration::from_secs(900); // 15 minutes
                                                        //
#[derive(thiserror::Error, Debug)]
#[error("Export canceled")]
struct ExportCanceled;

pub struct ExportWorker<RT: Runtime> {
    pub(super) runtime: RT,
    pub(super) database: Database<RT>,
    pub(super) storage: Arc<dyn Storage>,
    pub(super) file_storage: Arc<dyn Storage>,
    pub(super) backoff: Backoff,
    pub(super) usage_tracking: UsageCounter,
    pub(super) instance_name: String,
}

impl<RT: Runtime> ExportWorker<RT> {
    #[allow(clippy::new_ret_no_self)]
    pub fn new(
        runtime: RT,
        database: Database<RT>,
        storage: Arc<dyn Storage>,
        file_storage: Arc<dyn Storage>,
        usage_tracking: UsageCounter,
        instance_name: String,
    ) -> impl Future<Output = ()> + Send {
        let mut worker = Self {
            runtime,
            database,
            storage,
            file_storage,
            backoff: Backoff::new(INITIAL_BACKOFF, MAX_BACKOFF),
            usage_tracking,
            instance_name,
        };
        async move {
            loop {
                if let Err(e) = worker.run().await {
                    report_error(&mut e.context("ExportWorker died")).await;
                    let delay = worker.backoff.fail(&mut worker.runtime.rng());
                    worker.runtime.wait(delay).await;
                } else {
                    worker.backoff.reset();
                }
            }
        }
    }

    #[cfg(test)]
    pub fn new_test(
        runtime: RT,
        database: Database<RT>,
        storage: Arc<dyn Storage>,
        file_storage: Arc<dyn Storage>,
    ) -> Self {
        use events::usage::NoOpUsageEventLogger;

        Self {
            runtime,
            database,
            storage,
            file_storage,
            backoff: Backoff::new(INITIAL_BACKOFF, MAX_BACKOFF),
            usage_tracking: UsageCounter::new(Arc::new(NoOpUsageEventLogger)),
            instance_name: "carnitas".to_string(),
        }
    }

    // Subscribe to the export table. If there is a requested export, start
    // an export and mark as in_progress. If there's an export job that didn't
    // finish (it's in_progress), restart that export.
    pub async fn run(&mut self) -> anyhow::Result<()> {
        let mut tx = self.database.begin(Identity::system()).await?;
        let mut exports_model = ExportsModel::new(&mut tx);
        let export_requested = exports_model.latest_requested().await?;
        let export_in_progress = exports_model.latest_in_progress().await?;
        match (export_requested, export_in_progress) {
            (Some(_), Some(_)) => {
                anyhow::bail!("Can only have one export requested or in progress at once.")
            },
            (Some(export), None) => {
                tracing::info!("Export requested.");
                let _status = log_worker_starting("ExportWorker");
                let timer = export_timer();
                let ts = self.database.now_ts_for_reads();
                let in_progress_export = (*export).clone().in_progress(*ts)?;
                let in_progress_export_doc = SystemMetadataModel::new_global(&mut tx)
                    .replace(
                        export.id().to_owned(),
                        in_progress_export.clone().try_into()?,
                    )
                    .await?
                    .try_into()?;
                self.database
                    .commit_with_write_source(tx, "export_worker_export_requested")
                    .await?;
                self.export(in_progress_export_doc).await?;
                timer.finish();
                return Ok(());
            },
            (None, Some(export)) => {
                tracing::info!("In progress export restarting...");
                let _status = log_worker_starting("ExportWorker");
                let timer = export_timer();
                self.export(export).await?;
                timer.finish();
                return Ok(());
            },
            (None, None) => {
                tracing::info!("No exports requested or in progress.");
            },
        }
        let token = tx.into_token()?;
        let subscription = self.database.subscribe(token).await?;
        subscription.wait_for_invalidation().await;
        Ok(())
    }

    async fn export(&mut self, export: ParsedDocument<Export>) -> anyhow::Result<()> {
        loop {
            match self.export_and_mark_complete(export.clone()).await {
                Ok(()) => {
                    return Ok(());
                },
                Err(mut e) => {
                    if e.is::<ExportCanceled>() {
                        tracing::info!("Export {} canceled", export.id());
                        return Ok(());
                    }
                    report_error(&mut e).await;
                    let delay = self.backoff.fail(&mut self.runtime.rng());
                    tracing::error!("Export failed, retrying in {delay:?}");
                    self.runtime.wait(delay).await;
                },
            }
        }
    }

    async fn export_and_mark_complete(
        &mut self,
        export: ParsedDocument<Export>,
    ) -> anyhow::Result<()> {
        let id = export.id();
        let format = export.format();
        let requestor = export.requestor();
        drop(export); // Drop this to prevent accidentally using stale state

        tracing::info!("Export {id} beginning...");
        let (snapshot_ts, object_key, usage) = {
            let database_ = self.database.clone();
            let export_future = async {
                let database_ = self.database.clone();

                export_inner(self, format, requestor, |msg| async {
                    tracing::info!("Export {id} progress: {msg}");
                    database_
                        .execute_with_occ_retries(
                            Identity::system(),
                            FunctionUsageTracker::new(),
                            PauseClient::new(),
                            "export_worker_update_progress",
                            move |tx| {
                                let msg = msg.clone();
                                async move {
                                    let export: ParsedDocument<Export> =
                                        tx.get(id).await?.context(ExportCanceled)?.try_into()?;
                                    let export = export.into_value();
                                    if let Export::Canceled { .. } = export {
                                        anyhow::bail!(ExportCanceled);
                                    }
                                    SystemMetadataModel::new_global(tx)
                                        .replace(id, export.update_progress(msg)?.try_into()?)
                                        .await?;
                                    Ok(())
                                }
                                .boxed()
                                .into()
                            },
                        )
                        .await?;
                    Ok(())
                })
                .await
            };
            tokio::pin!(export_future);

            // In parallel, monitor the export document to check for cancellation
            let monitor_export = async move {
                loop {
                    let mut tx = database_.begin_system().await?;
                    let Some(export) = tx.get(id).await? else {
                        tracing::warn!("Export {id} disappeared");
                        return Err(ExportCanceled.into());
                    };
                    let export: ParsedDocument<Export> = export.try_into()?;
                    match *export {
                        Export::InProgress { .. } => (),
                        Export::Canceled { .. } => return Err(ExportCanceled.into()),
                        Export::Requested { .. }
                        | Export::Failed { .. }
                        | Export::Completed { .. } => {
                            anyhow::bail!("Export {id} is in unexpected state: {export:?}");
                        },
                    }
                    let token = tx.into_token()?;
                    let subscription = database_.subscribe(token).await?;
                    subscription.wait_for_invalidation().await;
                }
            };
            tokio::pin!(monitor_export);

            futures::future::select(export_future, monitor_export)
                .await
                .factor_first()
                .0?
        };

        // Export is done; mark it as such.
        tracing::info!("Export {id} completed");
        self.database
            .execute_with_occ_retries(
                Identity::system(),
                FunctionUsageTracker::new(),
                PauseClient::new(),
                "export_worker_mark_complete",
                |tx| {
                    let object_key = object_key.clone();
                    async move {
                        let Some(export) = tx.get(id).await? else {
                            tracing::warn!("Export {id} disappeared");
                            return Err(ExportCanceled.into());
                        };
                        let export: ParsedDocument<Export> = export.try_into()?;
                        if let Export::Canceled { .. } = *export {
                            return Err(ExportCanceled.into());
                        }
                        let completed_export = export.into_value().completed(
                            snapshot_ts,
                            *tx.begin_timestamp(),
                            object_key,
                        )?;
                        SystemMetadataModel::new_global(tx)
                            .replace(id, completed_export.try_into()?)
                            .await?;
                        Ok(())
                    }
                    .boxed()
                    .into()
                },
            )
            .await?;

        let object_attributes = self
            .storage
            .get_object_attributes(&object_key)
            .await?
            .context("error getting export object attributes from S3")?;

        let tag = requestor.usage_tag().to_string();
        let call_type = match requestor {
            ExportRequestor::SnapshotExport => CallType::Export,
            ExportRequestor::CloudBackup => CallType::CloudBackup,
        };
        // Charge file bandwidth for the upload of the snapshot to exports storage
        usage.track_storage_ingress_size(
            ComponentPath::root(),
            tag.clone(),
            object_attributes.size,
        );
        // Charge database bandwidth accumulated during the export
        self.usage_tracking.track_call(
            UdfIdentifier::SystemJob(tag),
            ExecutionId::new(),
            RequestId::new(),
            call_type,
            true,
            usage.gather_user_stats(),
        );
        Ok(())
    }
}

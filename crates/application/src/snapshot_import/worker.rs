use std::{
    future::Future,
    sync::Arc,
    time::Duration,
};

use common::{
    backoff::Backoff,
    errors::report_error,
    pause::PauseClient,
    runtime::Runtime,
};
use database::Database;
use file_storage::FileStorage;
use keybroker::Identity;
use model::snapshot_imports::{
    types::ImportState,
    SnapshotImportModel,
};
use storage::Storage;
use usage_tracking::UsageCounter;

use crate::{
    metrics::{
        log_worker_starting,
        snapshot_import_timer,
    },
    snapshot_import::SnapshotImportExecutor,
};

const INITIAL_BACKOFF: Duration = Duration::from_secs(1);
const MAX_BACKOFF: Duration = Duration::from_secs(60);

pub struct SnapshotImportWorker;

impl SnapshotImportWorker {
    pub fn start<RT: Runtime>(
        runtime: RT,
        database: Database<RT>,
        snapshot_imports_storage: Arc<dyn Storage>,
        file_storage: FileStorage<RT>,
        usage_tracking: UsageCounter,
        pause_client: PauseClient,
    ) -> impl Future<Output = ()> + Send {
        let mut worker = SnapshotImportExecutor {
            runtime,
            database,
            snapshot_imports_storage,
            file_storage,
            usage_tracking,
            pause_client,
            backoff: Backoff::new(INITIAL_BACKOFF, MAX_BACKOFF),
        };
        async move {
            loop {
                if let Err(e) = Self::run_once(&mut worker).await {
                    report_error(&mut e.context("SnapshotImportWorker died"));
                    let delay = worker.backoff.fail(&mut worker.runtime.rng());
                    worker.runtime.wait(delay).await;
                } else {
                    worker.backoff.reset();
                }
            }
        }
    }

    /// Subscribe to the _snapshot_imports table.
    /// If an import has Uploaded, parse it and set to WaitingForConfirmation.
    /// If an import is InProgress, execute it.
    async fn run_once<RT: Runtime>(
        executor: &mut SnapshotImportExecutor<RT>,
    ) -> anyhow::Result<()> {
        let status = log_worker_starting("SnapshotImport");
        let mut tx = executor.database.begin(Identity::system()).await?;
        let mut import_model = SnapshotImportModel::new(&mut tx);
        let import_uploaded = import_model.import_in_state(ImportState::Uploaded).await?;
        let import_in_progress = import_model
            .import_in_state(ImportState::InProgress {
                progress_message: String::new(),
                checkpoint_messages: vec![],
            })
            .await?;
        let token = tx.into_token()?;

        if let Some(import_uploaded) = import_uploaded {
            executor.handle_uploaded_state(import_uploaded).await?;
        } else if let Some(import_in_progress) = import_in_progress {
            tracing::info!("Executing in-progress snapshot import");
            let timer = snapshot_import_timer();
            executor
                .handle_in_progress_state(import_in_progress)
                .await?;
            timer.finish();
        }
        drop(status);
        let subscription = executor.database.subscribe(token).await?;
        subscription.wait_for_invalidation().await;
        Ok(())
    }
}

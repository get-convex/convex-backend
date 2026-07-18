//! Background worker that polls the `_periodic_backup_config` system table
//! and triggers a snapshot export whenever a configured cron run is due.
//!
//! Runs as a separate task next to the existing `ExportWorker`. The two are
//! decoupled: this worker only inserts a "requested" row in `_exports` and
//! records the run in `_periodic_backup_config`. The `ExportWorker` then
//! picks up the requested row and produces the actual zip.
//!
//! The worker holds no UI/HTTP layer; it only depends on `Database`. This
//! mirrors the rest of the worker fleet in `crates/application/src/`.

use std::time::Duration;

use anyhow::Context;
use common::{
    components::ComponentId,
    errors::report_error,
    execution_context::RequestMetadata,
    runtime::Runtime,
};
use database::Database;
use keybroker::Identity;
use model::{
    deployment_audit_log::{
        types::DeploymentAuditLogEvent,
        DeploymentAuditLogModel,
    },
    exports::{
        types::{
            ExportFormat,
            ExportRequestor,
        },
        ExportsModel,
    },
    periodic_backup::PeriodicBackupModel,
};
use sync_types::Timestamp;
use value::DeveloperDocumentId;

/// How long to sleep between config checks. Chosen as a reasonable balance
/// for cron specs — cron's finest granularity is one minute, so anything
/// faster than this is wasted work.
const POLL_INTERVAL: Duration = Duration::from_secs(60);

/// On startup or after errors, wait a little before the first iteration so
/// we don't slam the database during a restart loop.
const INITIAL_BACKOFF: Duration = Duration::from_secs(5);

pub struct PeriodicBackupWorker<RT: Runtime> {
    runtime: RT,
    database: Database<RT>,
}

impl<RT: Runtime> PeriodicBackupWorker<RT> {
    pub fn start(
        runtime: RT,
        database: Database<RT>,
    ) -> impl std::future::Future<Output = ()> + Send {
        let worker = Self { runtime, database };
        async move {
            worker.runtime.wait(INITIAL_BACKOFF).await;
            loop {
                if let Err(e) = worker.tick().await {
                    report_error(&mut e.context("PeriodicBackupWorker tick failed")).await;
                }
                worker.runtime.wait(POLL_INTERVAL).await;
            }
        }
    }

    async fn tick(&self) -> anyhow::Result<()> {
        // First read: is anything due to run?
        let now_ts = self.now_ts().await?;
        let due = {
            let mut tx = self.database.begin(Identity::system()).await?;
            PeriodicBackupModel::new(&mut tx)
                .get()
                .await?
                .filter(|doc| doc.next_run_ts <= now_ts)
                .map(|doc| (doc.cronspec.clone(), doc.include_storage))
        };
        let Some((_cronspec, include_storage)) = due else {
            return Ok(());
        };

        // Second read+write: re-check no export is in flight (so we don't
        // collide with a manual "Back up now"), insert the requested row,
        // and stamp the config's last_run_ts.
        let mut tx = self.database.begin(Identity::system()).await?;
        let mut exports_model = ExportsModel::new(&mut tx);
        if exports_model.latest_requested().await?.is_some()
            || exports_model.latest_in_progress().await?.is_some()
        {
            // A manual export is still queued or running. Skip this tick;
            // we'll re-check next minute.
            return Ok(());
        }
        let snapshot_id = exports_model
            .insert_requested(
                ExportFormat::Zip { include_storage },
                ComponentId::Root,
                ExportRequestor::SnapshotExport,
                None,
            )
            .await?;

        // Stamp config so we don't re-fire next tick.
        PeriodicBackupModel::new(&mut tx).record_run(now_ts).await?;

        // Audit-log: one entry for the periodic trigger that points at the
        // export id, alongside the standard RequestExport event the manual
        // path emits, so the History page surfaces both lineages clearly.
        let export_id_str = DeveloperDocumentId::from(snapshot_id).encode();
        DeploymentAuditLogModel::new(&mut tx)
            .insert(
                vec![
                    DeploymentAuditLogEvent::RequestExport {
                        id: export_id_str.clone(),
                        component_id: None,
                        component: common::components::ComponentPath::root(),
                        format: if include_storage {
                            "zip_with_storage".to_string()
                        } else {
                            "zip".to_string()
                        },
                        requestor: ExportRequestor::SnapshotExport.usage_tag().to_string(),
                    },
                    DeploymentAuditLogEvent::PeriodicBackupTriggered {
                        export_id: export_id_str,
                    },
                ],
                &RequestMetadata::system(),
            )
            .await?;

        self.database
            .commit_with_write_source(tx, "periodic_backup_worker_trigger")
            .await?;
        Ok(())
    }

    async fn now_ts(&self) -> anyhow::Result<Timestamp> {
        // Use the database's wall clock so this aligns with how
        // `_periodic_backup_config.next_run_ts` was computed by the model
        // (which uses `tx.runtime().generate_timestamp()`).
        Ok(self
            .runtime
            .generate_timestamp()
            .context("PeriodicBackupWorker failed to read wall clock")?)
    }
}

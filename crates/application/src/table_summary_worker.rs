use std::{
    sync::Arc,
    time::Duration,
};

use common::{
    errors::report_error,
    knobs::{
        DATABASE_WORKERS_MAX_CHECKPOINT_AGE,
        DATABASE_WORKERS_MIN_COMMITS,
    },
    persistence::Persistence,
    runtime::{
        Runtime,
        SpawnHandle,
        UnixTimestamp,
    },
};
use database::{
    table_summary::write_snapshot,
    Database,
    TableSummaryWriter,
};
use futures::{
    channel::oneshot,
    pin_mut,
    select_biased,
    FutureExt,
};
use parking_lot::Mutex;

use crate::metrics::log_worker_starting;

pub struct TableSummaryWorker<RT: Runtime> {
    runtime: RT,
    database: Database<RT>,
    persistence: Arc<dyn Persistence>,
}

struct Inner<RT: Runtime> {
    handle: RT::Handle,
    cancel_sender: oneshot::Sender<()>,
}

#[derive(Clone)]
pub struct TableSummaryClient<RT: Runtime> {
    inner: Arc<Mutex<Option<Inner<RT>>>>,
}

struct LastWriteInfo {
    ts: UnixTimestamp,
    observed_commits: usize,
}

impl<RT: Runtime> TableSummaryWorker<RT> {
    pub(crate) fn start(
        runtime: RT,
        database: Database<RT>,
        persistence: Arc<dyn Persistence>,
    ) -> TableSummaryClient<RT> {
        let table_summary_worker = Self {
            runtime: runtime.clone(),
            database,
            persistence,
        };
        let (cancel_sender, cancel_receiver) = oneshot::channel();
        let handle = runtime.spawn(
            "table_summary_worker",
            table_summary_worker.go(cancel_receiver),
        );
        let inner = Inner {
            handle,
            cancel_sender,
        };
        TableSummaryClient {
            inner: Arc::new(Mutex::new(Some(inner))),
        }
    }

    async fn checkpoint_table_summaries(
        &self,
        last_write_info: &mut Option<LastWriteInfo>,
        writer: &TableSummaryWriter<RT>,
    ) -> anyhow::Result<()> {
        let _status = log_worker_starting("TableSummaryWorker");
        let commits_since_load = self.database.write_commits_since_load();
        let now = self.runtime.unix_timestamp();
        if let Some(last_write_info) = last_write_info
            && commits_since_load - last_write_info.observed_commits < *DATABASE_WORKERS_MIN_COMMITS
            && now - last_write_info.ts < *DATABASE_WORKERS_MAX_CHECKPOINT_AGE
        {
            return Ok(());
        }
        tracing::info!("Writing table summary");
        let snapshot = writer.compute_from_last_checkpoint().await?;
        write_snapshot(self.persistence.as_ref(), &snapshot).await?;
        *last_write_info = Some(LastWriteInfo {
            observed_commits: commits_since_load,
            ts: now,
        });
        Ok(())
    }

    async fn go(self, cancel_receiver: oneshot::Receiver<()>) {
        tracing::info!("Starting background table summary worker");
        let cancel_fut = cancel_receiver.fuse();
        pin_mut!(cancel_fut);

        let writer = TableSummaryWriter::new(
            self.runtime.clone(),
            self.persistence.clone(),
            self.database.clone(),
            self.database.retention_validator(),
        );

        let mut last_write_info = None;
        loop {
            let wait_fut = self.runtime.wait(Duration::from_secs(10)).fuse();
            pin_mut!(wait_fut);
            select_biased! {
                _ = cancel_fut => {
                    tracing::info!("Shutting down table summary worker...");
                    break;
                }
                _ = wait_fut => {
                    let result = self.checkpoint_table_summaries(
                        &mut last_write_info,
                        &writer,
                    ).await;
                    if let Err(mut err) = result {
                        report_error(&mut err);
                    }
                },
            }
        }
    }
}

impl<RT: Runtime> TableSummaryClient<RT> {
    pub async fn shutdown(&self) -> anyhow::Result<()> {
        let inner = { self.inner.lock().take() };
        if let Some(inner) = inner {
            let _ = inner.cancel_sender.send(());
            // NB: We don't want to use `shutdown_and_join` here since we actually want to
            // block on our flush completing successfully.
            inner.handle.into_join_future().await?;
        }
        Ok(())
    }
}

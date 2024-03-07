use std::sync::Arc;

use async_trait::async_trait;
use common::{
    knobs::DATABASE_WORKERS_POLL_INTERVAL,
    runtime::Runtime,
};
use futures::{
    join,
    pin_mut,
    select_biased,
    FutureExt,
};
use search::Searcher;
use storage::Storage;
use sync_types::backoff::Backoff;

use crate::{
    index_workers::{
        retriable_worker::{
            retry_loop_expect_occs,
            retry_loop_expect_occs_and_overloaded,
            RetriableWorker,
        },
        timeout_with_jitter,
    },
    vector_index_worker::{
        compactor::CompactionConfig,
        writer::VectorMetadataWriter,
    },
    Database,
    SearchIndexFlusher,
    VectorIndexCompactor,
    VectorIndexFlusher,
};

/// Builds and compacts text/vector search indexes.
pub enum SearchIndexWorker<RT: Runtime> {
    VectorFlusher(VectorIndexFlusher<RT>),
    VectorCompactor(VectorIndexCompactor<RT>),
    SearchFlusher(SearchIndexFlusher<RT>),
}

#[async_trait]
impl<RT: Runtime> RetriableWorker<RT> for SearchIndexWorker<RT> {
    async fn work_loop(
        &mut self,
        name: &'static str,
        rt: &RT,
        db: &Database<RT>,
        backoff: &mut Backoff,
    ) -> anyhow::Result<()> {
        self.work_and_wait_for_changes(name, rt, db, backoff).await
    }
}

impl<RT: Runtime> SearchIndexWorker<RT> {
    pub async fn create_and_start(
        runtime: RT,
        database: Database<RT>,
        search_storage: Arc<dyn Storage>,
        searcher: Arc<dyn Searcher>,
    ) {
        let vector_writer =
            VectorMetadataWriter::new(runtime.clone(), database.clone(), search_storage.clone());
        let vector_flush = retry_loop_expect_occs_and_overloaded(
            "VectorFlusher",
            runtime.clone(),
            database.clone(),
            SearchIndexWorker::VectorFlusher(VectorIndexFlusher::new(
                runtime.clone(),
                database.clone(),
                search_storage.clone(),
                vector_writer.clone(),
            )),
        );
        let vector_compact = retry_loop_expect_occs_and_overloaded(
            "VectorCompactor",
            runtime.clone(),
            database.clone(),
            SearchIndexWorker::VectorCompactor(VectorIndexCompactor::new(
                database.clone(),
                searcher,
                search_storage.clone(),
                CompactionConfig::default(),
                vector_writer,
            )),
        );
        let search_flush = retry_loop_expect_occs(
            "SearchFlusher",
            runtime.clone(),
            database.clone(),
            SearchIndexWorker::SearchFlusher(SearchIndexFlusher::new(
                runtime,
                database.clone(),
                search_storage,
            )),
        );

        join!(vector_flush, vector_compact, search_flush);
    }

    async fn work_and_wait_for_changes(
        &mut self,
        name: &'static str,
        rt: &RT,
        db: &Database<RT>,
        backoff: &mut Backoff,
    ) -> anyhow::Result<()> {
        tracing::info!("Starting {name}");

        loop {
            let (metrics, token) = match self {
                SearchIndexWorker::VectorFlusher(flusher) => flusher.step().await?,
                SearchIndexWorker::VectorCompactor(compactor) => compactor.step().await?,
                SearchIndexWorker::SearchFlusher(flusher) => flusher.step().await?,
            };

            if !metrics.is_empty() {
                // We did some useful work this loop iteration that we expect is committed.
                // There's no point in subscribing, as we'd immediately be woken by our own
                // changes.
                backoff.reset();
                continue;
            }

            // We need to wake up for two reasons:
            // 1. A new or updated index needs to be built - Implement via subscription on
            //    indexes
            // 2. Our soft index size is exceeded so we need to flush to disk - Implement
            //    via polling
            let poll = timeout_with_jitter(rt, *DATABASE_WORKERS_POLL_INTERVAL);
            pin_mut!(poll);
            let subscription = db.subscribe(token).await?;
            let subscription_fut = subscription.wait_for_invalidation();
            pin_mut!(subscription_fut);
            select_biased! {
                _ = subscription_fut.fuse() => {
                    tracing::info!(
                        "{name} resuming after index subscription notification"
                    );
                }
                _ = poll.fuse() => {
                    tracing::debug!("{name} starting background checks");
                }
            }
            backoff.reset();
        }
    }
}

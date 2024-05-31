use std::{
    future::Future,
    sync::Arc,
    time::Duration,
};

use async_trait::async_trait;
use common::{
    knobs::{
        BUILD_MULTI_SEGMENT_TEXT_INDEXES,
        DATABASE_WORKERS_POLL_INTERVAL,
    },
    persistence::PersistenceReader,
    runtime::{
        Runtime,
        SpawnHandle,
    },
};
use futures::{
    join,
    pin_mut,
    select_biased,
    FutureExt,
};
use search::{
    searcher::SegmentTermMetadataFetcher,
    Searcher,
};
use storage::Storage;
use sync_types::backoff::Backoff;

use crate::{
    index_workers::{
        retriable_worker::{
            retry_loop_expect_occs_and_overloaded,
            RetriableWorker,
        },
        search_compactor::CompactionConfig,
        timeout_with_jitter,
        writer::SearchIndexMetadataWriter,
    },
    metrics::log_worker_starting,
    text_index_worker::{
        compactor::{
            new_text_compactor,
            TextIndexCompactor,
        },
        flusher2::{
            new_text_flusher,
            TextIndexFlusher2,
        },
        TextIndexMetadataWriter,
    },
    vector_index_worker::{
        compactor::{
            new_vector_compactor,
            VectorIndexCompactor,
        },
        flusher::new_vector_flusher,
    },
    Database,
    TextIndexFlusher,
    VectorIndexFlusher,
};

/// Builds and compacts text/vector search indexes.
pub enum SearchIndexWorker<RT: Runtime> {
    VectorFlusher(VectorIndexFlusher<RT>),
    VectorCompactor(VectorIndexCompactor<RT>),
    TextFlusher(TextIndexFlusher<RT>),
    TextFlusher2(TextIndexFlusher2<RT>),
    TextCompactor(TextIndexCompactor<RT>),
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
        reader: Arc<dyn PersistenceReader>,
        search_storage: Arc<dyn Storage>,
        searcher: Arc<dyn Searcher>,
        segment_term_metadata_fetcher: Arc<dyn SegmentTermMetadataFetcher>,
    ) {
        let vector_index_metadata_writer = SearchIndexMetadataWriter::new(
            runtime.clone(),
            database.clone(),
            search_storage.clone(),
        );
        let text_index_metadata_writer =
            TextIndexMetadataWriter::new(runtime.clone(), database.clone(), search_storage.clone());
        let vector_flush = retry_loop_expect_occs_and_overloaded(
            "VectorFlusher",
            runtime.clone(),
            database.clone(),
            // Wait a bit since vector needs time to bootstrap. Makes startup logs a bit cleaner.
            Duration::from_secs(5),
            SearchIndexWorker::VectorFlusher(new_vector_flusher(
                runtime.clone(),
                database.clone(),
                reader.clone(),
                search_storage.clone(),
                vector_index_metadata_writer.clone(),
            )),
        );
        let vector_compact = retry_loop_expect_occs_and_overloaded(
            "VectorCompactor",
            runtime.clone(),
            database.clone(),
            Duration::ZERO,
            SearchIndexWorker::VectorCompactor(new_vector_compactor(
                database.clone(),
                searcher.clone(),
                search_storage.clone(),
                CompactionConfig::default(),
                vector_index_metadata_writer.clone(),
            )),
        );
        let text_flusher = if *BUILD_MULTI_SEGMENT_TEXT_INDEXES {
            SearchIndexWorker::TextFlusher2(new_text_flusher(
                runtime.clone(),
                database.clone(),
                reader,
                search_storage.clone(),
                segment_term_metadata_fetcher,
                text_index_metadata_writer.clone(),
            ))
        } else {
            SearchIndexWorker::TextFlusher(TextIndexFlusher::new(
                runtime.clone(),
                database.clone(),
                search_storage.clone(),
            ))
        };
        let search_flush = retry_loop_expect_occs_and_overloaded(
            "SearchFlusher",
            runtime.clone(),
            database.clone(),
            Duration::ZERO,
            text_flusher,
        );

        let text_compact = retry_loop_expect_occs_and_overloaded(
            "TextCompactor",
            runtime.clone(),
            database.clone(),
            Duration::ZERO,
            SearchIndexWorker::TextCompactor(new_text_compactor(
                database,
                searcher,
                search_storage,
                CompactionConfig::default(),
                text_index_metadata_writer,
            )),
        );
        let text_compact = async move {
            if *BUILD_MULTI_SEGMENT_TEXT_INDEXES {
                text_compact.await;
            }
        };

        join!(
            Self::spawn_and_join(runtime.clone(), "vector_flush", vector_flush),
            Self::spawn_and_join(runtime.clone(), "vector_compact", vector_compact),
            Self::spawn_and_join(runtime.clone(), "search_flush", search_flush),
            Self::spawn_and_join(runtime, "text_compact", text_compact),
        );
    }

    async fn spawn_and_join(
        rt: RT,
        name: &'static str,
        fut: impl Future<Output = ()> + Send + 'static,
    ) {
        let _ = rt.spawn(name, fut).into_join_future().await;
    }

    async fn work_and_wait_for_changes(
        &mut self,
        name: &'static str,
        rt: &RT,
        db: &Database<RT>,
        backoff: &mut Backoff,
    ) -> anyhow::Result<()> {
        loop {
            let status = log_worker_starting(name);
            let (metrics, token) = match self {
                SearchIndexWorker::VectorFlusher(flusher) => flusher.step().await?,
                SearchIndexWorker::VectorCompactor(compactor) => compactor.step().await?,
                SearchIndexWorker::TextFlusher(flusher) => flusher.step().await?,
                SearchIndexWorker::TextFlusher2(flusher) => flusher.step().await?,
                SearchIndexWorker::TextCompactor(compactor) => compactor.step().await?,
            };
            drop(status);

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

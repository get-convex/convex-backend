use std::{
    collections::BTreeMap,
    sync::Arc,
    time::Duration,
};

use async_trait::async_trait;
use common::{
    knobs::{
        DATABASE_WORKERS_POLL_INTERVAL,
        MULTI_SEGMENT_FULL_SCAN_THRESHOLD_KB,
        SEARCH_COMPACTOR_MAX_BACKOFF,
        SEARCH_INDEX_FLUSHER_MAX_BACKOFF,
    },
    persistence::PersistenceReader,
    runtime::{
        Runtime,
        SpawnHandle,
    },
    types::TabletIndexName,
};
use futures::{
    future::BoxFuture,
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
    text_index_worker::{
        compactor::{
            new_text_compactor,
            TextIndexCompactor,
        },
        flusher::{
            new_text_flusher,
            TextIndexFlusher,
        },
        BuildTextIndexArgs,
        TextIndexMetadataWriter,
    },
    vector_index_worker::{
        compactor::{
            new_vector_compactor,
            VectorIndexCompactor,
        },
        flusher::new_vector_flusher,
        BuildVectorIndexArgs,
    },
    Database,
    Token,
    VectorIndexFlusher,
};

/// Builds and compacts text/vector search indexes.
pub struct SearchIndexWorkers {
    handles: Vec<Box<dyn SpawnHandle>>,
}

enum SearchIndexWorker<RT: Runtime> {
    VectorFlusher(VectorIndexFlusher<RT>),
    VectorCompactor(VectorIndexCompactor<RT>),
    TextFlusher(TextIndexFlusher<RT>),
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

impl SearchIndexWorkers {
    pub fn create_and_start<RT: Runtime>(
        runtime: RT,
        database: Database<RT>,
        reader: Arc<dyn PersistenceReader>,
        search_storage: Arc<dyn Storage>,
        searcher: Arc<dyn Searcher>,
        segment_term_metadata_fetcher: Arc<dyn SegmentTermMetadataFetcher>,
    ) -> Self {
        let vector_index_metadata_writer = SearchIndexMetadataWriter::new(
            runtime.clone(),
            database.clone(),
            reader.clone(),
            search_storage.clone(),
            BuildVectorIndexArgs {
                full_scan_threshold_bytes: *MULTI_SEGMENT_FULL_SCAN_THRESHOLD_KB,
            },
        );
        let text_index_metadata_writer = TextIndexMetadataWriter::new(
            runtime.clone(),
            database.clone(),
            reader.clone(),
            search_storage.clone(),
            BuildTextIndexArgs {
                search_storage: search_storage.clone(),
                segment_term_metadata_fetcher: segment_term_metadata_fetcher.clone(),
            },
        );
        let vector_flush = retry_loop_expect_occs_and_overloaded(
            "VectorFlusher",
            runtime.clone(),
            database.clone(),
            // Wait a bit since vector needs time to bootstrap. Makes startup logs a bit cleaner.
            Duration::from_secs(5),
            *SEARCH_INDEX_FLUSHER_MAX_BACKOFF,
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
            *SEARCH_COMPACTOR_MAX_BACKOFF,
            SearchIndexWorker::VectorCompactor(new_vector_compactor(
                database.clone(),
                searcher.clone(),
                search_storage.clone(),
                CompactionConfig::default(),
                vector_index_metadata_writer,
            )),
        );
        let text_flusher = SearchIndexWorker::TextFlusher(new_text_flusher(
            runtime.clone(),
            database.clone(),
            reader,
            search_storage.clone(),
            segment_term_metadata_fetcher,
            text_index_metadata_writer.clone(),
        ));
        let text_flush = retry_loop_expect_occs_and_overloaded(
            "SearchFlusher",
            runtime.clone(),
            database.clone(),
            Duration::ZERO,
            *SEARCH_INDEX_FLUSHER_MAX_BACKOFF,
            text_flusher,
        );

        let text_compact = retry_loop_expect_occs_and_overloaded(
            "TextCompactor",
            runtime.clone(),
            database.clone(),
            Duration::ZERO,
            *SEARCH_COMPACTOR_MAX_BACKOFF,
            SearchIndexWorker::TextCompactor(new_text_compactor(
                database,
                searcher,
                search_storage,
                CompactionConfig::default(),
                text_index_metadata_writer.clone(),
            )),
        );

        let vector_flush_handle = runtime.spawn("vector_flush", vector_flush);
        let vector_compact_handle = runtime.spawn("vector_compact", vector_compact);
        let text_flush_handle = runtime.spawn("text_flush", text_flush);
        let text_compact_handle = runtime.spawn("text_compact", text_compact);
        Self {
            handles: vec![
                vector_flush_handle,
                vector_compact_handle,
                text_flush_handle,
                text_compact_handle,
            ],
        }
    }

    pub fn shutdown(&mut self) {
        self.handles.iter_mut().for_each(|handle| handle.shutdown())
    }
}

impl<RT: Runtime> SearchIndexWorker<RT> {
    // The actual work future is fairly large, so box it to avoid consuming
    // steady-state memory.
    fn step(&mut self) -> BoxFuture<'_, anyhow::Result<(BTreeMap<TabletIndexName, u64>, Token)>> {
        match self {
            Self::VectorFlusher(flusher) => flusher.step().boxed(),
            Self::VectorCompactor(compactor) => compactor.step().boxed(),
            Self::TextFlusher(flusher) => flusher.step().boxed(),
            Self::TextCompactor(compactor) => compactor.step().boxed(),
        }
    }

    async fn work_and_wait_for_changes(
        &mut self,
        name: &'static str,
        rt: &RT,
        db: &Database<RT>,
        backoff: &mut Backoff,
    ) -> anyhow::Result<()> {
        loop {
            let (metrics, token) = self.step().await?;

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

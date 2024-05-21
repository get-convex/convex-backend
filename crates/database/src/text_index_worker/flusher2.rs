use std::{
    collections::BTreeMap,
    sync::Arc,
};

#[cfg(any(test, feature = "testing"))]
use common::pause::PauseClient;
use common::{
    bootstrap_model::index::{
        text_index::{
            FragmentedTextSegment,
            TextBackfillCursor,
            TextIndexBackfillState,
            TextIndexSnapshot,
            TextIndexSnapshotData,
            TextIndexState,
            TextSnapshotVersion,
        },
        IndexMetadata,
    },
    knobs::{
        MULTI_SEGMENT_FULL_SCAN_THRESHOLD_KB,
        SEARCH_INDEX_SIZE_SOFT_LIMIT,
    },
    persistence::PersistenceReader,
    runtime::Runtime,
    types::TabletIndexName,
};
use keybroker::Identity;
use storage::Storage;
use sync_types::Timestamp;

use crate::{
    index_workers::{
        index_meta::{
            SearchOnDiskState,
            SnapshotData,
        },
        search_flusher::{
            IndexBuild,
            IndexBuildResult,
            SearchFlusher,
        },
        MultiSegmentBackfillResult,
    },
    metrics::search::{
        log_documents_per_index,
        log_documents_per_new_segment,
    },
    text_index_worker::text_meta::{
        TextIndexConfigParser,
        TextSearchIndex,
    },
    Database,
    SystemMetadataModel,
    Token,
};

#[cfg(any(test, feature = "testing"))]
pub(crate) const FLUSH_RUNNING_LABEL: &str = "flush_running";

pub struct TextIndexFlusher2<RT: Runtime> {
    flusher: SearchFlusher<RT, TextIndexConfigParser>,
    database: Database<RT>,

    #[allow(unused)]
    #[cfg(any(test, feature = "testing"))]
    should_terminate: bool,
    #[allow(unused)]
    #[cfg(any(test, feature = "testing"))]
    pause_client: Option<PauseClient>,
}

pub(crate) struct FlusherBuilder<RT: Runtime> {
    runtime: RT,
    database: Database<RT>,
    reader: Arc<dyn PersistenceReader>,
    storage: Arc<dyn Storage>,
    index_size_soft_limit: usize,
    full_scan_threshold_kb: usize,
    incremental_multipart_threshold_bytes: usize,
    #[cfg(any(test, feature = "testing"))]
    should_terminate: bool,
    #[cfg(any(test, feature = "testing"))]
    pause_client: Option<PauseClient>,
}

impl<RT: Runtime> FlusherBuilder<RT> {
    pub(crate) fn new(
        runtime: RT,
        database: Database<RT>,
        reader: Arc<dyn PersistenceReader>,
        storage: Arc<dyn Storage>,
    ) -> Self {
        Self {
            runtime,
            database,
            reader,
            storage,
            index_size_soft_limit: *SEARCH_INDEX_SIZE_SOFT_LIMIT,
            full_scan_threshold_kb: *MULTI_SEGMENT_FULL_SCAN_THRESHOLD_KB,
            incremental_multipart_threshold_bytes: *SEARCH_INDEX_SIZE_SOFT_LIMIT,
            #[cfg(any(test, feature = "testing"))]
            should_terminate: false,
            #[cfg(any(test, feature = "testing"))]
            pause_client: None,
        }
    }

    #[cfg(any(test, feature = "testing"))]
    #[allow(dead_code)]
    fn set_soft_limit(self, limit: usize) -> Self {
        Self {
            index_size_soft_limit: limit,
            ..self
        }
    }

    pub(crate) fn build(self) -> TextIndexFlusher2<RT> {
        let flusher = SearchFlusher::new(
            self.runtime,
            self.database.clone(),
            self.reader,
            self.storage,
            self.index_size_soft_limit,
            self.full_scan_threshold_kb,
            self.incremental_multipart_threshold_bytes,
        );
        TextIndexFlusher2 {
            flusher,
            database: self.database,
            #[cfg(any(test, feature = "testing"))]
            should_terminate: self.should_terminate,
            #[cfg(any(test, feature = "testing"))]
            pause_client: self.pause_client,
        }
    }
}

impl<RT: Runtime> TextIndexFlusher2<RT> {
    pub(crate) fn new(
        runtime: RT,
        database: Database<RT>,
        reader: Arc<dyn PersistenceReader>,
        storage: Arc<dyn Storage>,
    ) -> Self {
        FlusherBuilder::new(runtime, database, reader, storage).build()
    }

    /// Run one step of the IndexFlusher's main loop.
    ///
    /// Returns a map of IndexName to number of documents indexed for each
    /// index that was built.
    pub(crate) async fn step(&mut self) -> anyhow::Result<(BTreeMap<TabletIndexName, u32>, Token)> {
        let mut metrics = BTreeMap::new();

        let (to_build, token) = self.flusher.needs_backfill().await?;
        let num_to_build = to_build.len();
        if num_to_build > 0 {
            tracing::info!("{num_to_build} text indexes to build");
        }

        #[cfg(any(test, feature = "testing"))]
        if let Some(pause_client) = &mut self.pause_client {
            pause_client.wait(FLUSH_RUNNING_LABEL).await;
        }

        for job in to_build {
            let index_name = job.index_name.clone();
            let num_documents_indexed = self.build_one(job).await?;
            metrics.insert(index_name, num_documents_indexed);
        }

        if num_to_build > 0 {
            tracing::info!("built {num_to_build} text indexes");
        }

        Ok((metrics, token))
    }

    async fn build_one(&self, job: IndexBuild<TextSearchIndex>) -> anyhow::Result<u32> {
        let timer = crate::metrics::search::build_one_timer();

        let result = self.flusher.build_multipart_segment(&job).await?;
        tracing::debug!("Built a text segment for: {result:#?}");

        let IndexBuildResult {
            snapshot_ts,
            data,
            total_stats,
            new_segment_stats,
            backfill_result,
            ..
        } = result;

        match data {
            SnapshotData::Unknown => {
                anyhow::bail!("Created an unknown snapshot data type")
            },
            SnapshotData::MultiSegment(segments) => {
                self.write_search_metadata(job, snapshot_ts, segments, backfill_result)
                    .await?;
            },
        }
        log_documents_per_new_segment(new_segment_stats.unwrap_or_default().num_indexed_documents);
        log_documents_per_index(total_stats.num_indexed_documents as usize);
        timer.finish();
        Ok(0)
    }

    fn get_new_disk_state(
        backfill_result: Option<MultiSegmentBackfillResult>,
        backfill_ts: Timestamp,
        segments: Vec<FragmentedTextSegment>,
        on_disk_state: SearchOnDiskState<TextSearchIndex>,
    ) -> TextIndexState {
        if let Some(backfill_result) = backfill_result {
            if backfill_result.is_backfill_complete {
                TextIndexState::Backfilled(TextIndexSnapshot {
                    data: TextIndexSnapshotData::MultiSegment(segments),
                    ts: backfill_ts,
                    version: TextSnapshotVersion::V0,
                })
            } else {
                let cursor = if let Some(cursor) = backfill_result.new_cursor {
                    Some(TextBackfillCursor {
                        cursor: cursor.internal_id(),
                        backfill_snapshot_ts: backfill_ts,
                    })
                } else {
                    None
                };
                TextIndexState::Backfilling(TextIndexBackfillState { segments, cursor })
            }
        } else {
            let snapshot = TextIndexSnapshot {
                data: TextIndexSnapshotData::MultiSegment(segments),
                ts: backfill_ts,
                version: TextSnapshotVersion::V0,
            };
            let is_snapshotted = matches!(on_disk_state, SearchOnDiskState::SnapshottedAt(_));
            if is_snapshotted {
                TextIndexState::SnapshottedAt(snapshot)
            } else {
                TextIndexState::Backfilled(snapshot)
            }
        }
    }

    async fn write_search_metadata(
        &self,
        job: IndexBuild<TextSearchIndex>,
        snapshot_ts: Timestamp,
        segments: Vec<FragmentedTextSegment>,
        backfill_result: Option<MultiSegmentBackfillResult>,
    ) -> anyhow::Result<()> {
        let mut tx = self.database.begin(Identity::system()).await?;

        let new_on_disk_state = Self::get_new_disk_state(
            backfill_result,
            snapshot_ts,
            segments,
            job.index_config.on_disk_state,
        );

        SystemMetadataModel::new_global(&mut tx)
            .replace(
                job.metadata_id,
                IndexMetadata::new_search_index(
                    job.index_name,
                    job.index_config.developer_config,
                    new_on_disk_state,
                )
                .try_into()?,
            )
            .await?;
        self.database
            .commit_with_write_source(tx, "search_index_worker_build_index")
            .await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use common::{
        persistence::PersistenceReader,
        runtime::testing::TestRuntime,
    };
    use storage::{
        LocalDirStorage,
        Storage,
    };
    use value::TableNamespace;

    use crate::{
        test_helpers::DbFixtures,
        tests::search_test_utils::{
            assert_backfilled,
            insert_backfilling_text_index,
        },
        text_index_worker::flusher2::{
            FlusherBuilder,
            TextIndexFlusher2,
        },
        Database,
    };

    fn new_flusher(
        rt: &TestRuntime,
        database: &Database<TestRuntime>,
        reader: Arc<dyn PersistenceReader>,
        storage: Arc<dyn Storage>,
    ) -> TextIndexFlusher2<TestRuntime> {
        FlusherBuilder::new(rt.clone(), database.clone(), reader, storage)
            // Build after every write.
            .set_soft_limit(0)
            .build()
    }

    #[convex_macro::test_runtime]
    async fn backfill_with_no_documents_sets_state_to_backfilled(
        rt: TestRuntime,
    ) -> anyhow::Result<()> {
        let storage = Arc::new(LocalDirStorage::new(rt.clone())?);
        let DbFixtures { db, tp, .. } = DbFixtures::new(&rt).await?;
        let index = insert_backfilling_text_index(&db).await?;
        let mut flusher = new_flusher(&rt, &db, tp.reader(), storage);
        flusher.step().await?;
        assert_backfilled(&db, TableNamespace::Global, &index.name).await?;
        Ok(())
    }
}

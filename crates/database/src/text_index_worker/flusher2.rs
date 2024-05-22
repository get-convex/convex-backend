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
            SearchIndexLimits,
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
    limits: SearchIndexLimits,
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
            limits: SearchIndexLimits {
                index_size_soft_limit: *SEARCH_INDEX_SIZE_SOFT_LIMIT,
                full_scan_segment_max_kb: *MULTI_SEGMENT_FULL_SCAN_THRESHOLD_KB,
                incremental_multipart_threshold_bytes: *SEARCH_INDEX_SIZE_SOFT_LIMIT,
            },
            #[cfg(any(test, feature = "testing"))]
            should_terminate: false,
            #[cfg(any(test, feature = "testing"))]
            pause_client: None,
        }
    }

    #[allow(dead_code)]
    pub fn set_soft_limit(self, limit: usize) -> Self {
        Self {
            limits: SearchIndexLimits {
                index_size_soft_limit: limit,
                ..self.limits
            },
            ..self
        }
    }

    #[allow(dead_code)]
    pub fn set_incremental_multipart_threshold_bytes(self, limit: usize) -> Self {
        Self {
            limits: SearchIndexLimits {
                incremental_multipart_threshold_bytes: limit,
                ..self.limits
            },
            ..self
        }
    }

    pub(crate) fn build(self) -> TextIndexFlusher2<RT> {
        let flusher = SearchFlusher::new(
            self.runtime,
            self.database.clone(),
            self.reader,
            self.storage,
            self.limits,
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
        let num_indexed_documents = new_segment_stats.unwrap_or_default().num_indexed_documents;
        log_documents_per_new_segment(num_indexed_documents);
        log_documents_per_index(total_stats.num_indexed_documents as usize);
        timer.finish();
        Ok(num_indexed_documents)
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
                    version: TextSnapshotVersion::V2UseStringIds,
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
                version: TextSnapshotVersion::V2UseStringIds,
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
    use common::{
        bootstrap_model::index::{
            text_index::TextIndexState,
            IndexConfig,
            IndexMetadata,
        },
        runtime::testing::TestRuntime,
        types::TabletIndexName,
    };
    use maplit::btreemap;
    use must_let::must_let;
    use value::TableNamespace;

    use crate::tests::text_test_utils::{
        IndexData,
        TextFixtures,
    };

    #[convex_macro::test_runtime]
    async fn backfill_with_no_documents_sets_state_to_backfilled(
        rt: TestRuntime,
    ) -> anyhow::Result<()> {
        let fixtures = TextFixtures::new(rt).await?;
        let index = fixtures.insert_backfilling_text_index().await?;
        let mut flusher = fixtures.new_search_flusher2();
        flusher.step().await?;
        fixtures.assert_backfilled(&index.name).await?;
        Ok(())
    }

    #[convex_macro::test_runtime]
    async fn backfill_with_no_documents_returns_index_in_metrics(
        rt: TestRuntime,
    ) -> anyhow::Result<()> {
        let fixtures = TextFixtures::new(rt).await?;
        let index = fixtures.insert_backfilling_text_index().await?;
        let mut tx = fixtures.db.begin_system().await?;
        let table_id = tx
            .table_mapping()
            .namespace(TableNamespace::Global)
            .id(index.name.table())?
            .tablet_id;
        let resolved_index_name = TabletIndexName::new(table_id, index.name.descriptor().clone())?;
        let mut flusher = fixtures.new_search_flusher2();
        let (metrics, _) = flusher.step().await?;
        assert_eq!(metrics, btreemap! { resolved_index_name => 0 });
        Ok(())
    }

    #[convex_macro::test_runtime]
    async fn backfill_with_one_document_sets_state_to_backfilled(
        rt: TestRuntime,
    ) -> anyhow::Result<()> {
        let fixtures = TextFixtures::new(rt).await?;
        let index = fixtures
            .insert_backfilling_text_index_with_document()
            .await?;
        let mut flusher = fixtures.new_search_flusher2();
        flusher.step().await?;
        fixtures.assert_backfilled(&index.index_name).await?;
        Ok(())
    }

    #[convex_macro::test_runtime]
    async fn backfill_with_one_document_returns_metrics(rt: TestRuntime) -> anyhow::Result<()> {
        let fixtures = TextFixtures::new(rt).await?;
        let IndexData {
            resolved_index_name,
            ..
        } = fixtures
            .insert_backfilling_text_index_with_document()
            .await?;
        let mut flusher = fixtures.new_search_flusher2();
        let (metrics, _) = flusher.step().await?;
        assert_eq!(metrics, btreemap! { resolved_index_name => 1 });
        Ok(())
    }

    #[convex_macro::test_runtime]
    async fn backfill_with_one_document_writes_document(rt: TestRuntime) -> anyhow::Result<()> {
        let fixtures = TextFixtures::new(rt).await?;
        let index = fixtures.insert_backfilling_text_index().await?;
        let doc_id = fixtures.add_document("cat").await?;
        let mut flusher = fixtures.new_search_flusher2();
        flusher.step().await?;
        fixtures.enable_index(&index.name).await?;

        let results = fixtures.search(index.name, "cat").await?;
        assert_eq!(results.first().unwrap().id(), doc_id);

        Ok(())
    }

    #[convex_macro::test_runtime]
    async fn backfill_with_two_documents_0_max_segment_size_creates_two_segments(
        rt: TestRuntime,
    ) -> anyhow::Result<()> {
        let fixtures = TextFixtures::new(rt).await?;
        let IndexMetadata { name, .. } = fixtures.insert_backfilling_text_index().await?;

        fixtures.add_document("some text").await?;
        fixtures.add_document("some other text").await?;

        let mut flusher = fixtures
            .new_search_flusher_builder()
            .set_incremental_multipart_threshold_bytes(0)
            .build();
        // Build the first segment, which stops because the document size is > 0
        flusher.step().await?;
        // Build the second segment and finalize the index metadata.
        flusher.step().await?;

        let segments = fixtures.get_segments_metadata(name).await?;
        assert_eq!(segments.len(), 2);
        Ok(())
    }

    #[convex_macro::test_runtime]
    async fn backfill_with_two_documents_leaves_document_backfilling_after_first_flush(
        rt: TestRuntime,
    ) -> anyhow::Result<()> {
        let fixtures = TextFixtures::new(rt).await?;
        let IndexMetadata { name, .. } = fixtures.insert_backfilling_text_index().await?;

        fixtures.add_document("cat").await?;
        fixtures.add_document("dog").await?;

        let mut flusher = fixtures
            .new_search_flusher_builder()
            .set_incremental_multipart_threshold_bytes(0)
            .build();
        // Build the first segment, which stops because the document size is > 0
        flusher.step().await?;
        let metadata = fixtures.get_index_metadata(name).await?;
        must_let!(let IndexConfig::Search { on_disk_state, .. }= &metadata.config);
        must_let!(let TextIndexState::Backfilling(backfilling_meta) = on_disk_state);
        assert_eq!(backfilling_meta.segments.len(), 1);

        Ok(())
    }

    #[convex_macro::test_runtime]
    async fn backfill_with_two_documents_0_max_segment_size_includes_both_documents(
        rt: TestRuntime,
    ) -> anyhow::Result<()> {
        let fixtures = TextFixtures::new(rt).await?;
        let IndexMetadata { name, .. } = fixtures.insert_backfilling_text_index().await?;

        let cat_doc_id = fixtures.add_document("cat").await?;
        let dog_doc_id = fixtures.add_document("dog").await?;

        let mut flusher = fixtures
            .new_search_flusher_builder()
            .set_incremental_multipart_threshold_bytes(0)
            .build();
        // Build the first segment, which stops because the document size is > 0
        flusher.step().await?;
        // Build the second segment and finalize the index metadata.
        flusher.step().await?;

        fixtures.enable_index(&name).await?;

        let cat_results = fixtures.search(name.clone(), "cat").await?;
        assert_eq!(cat_results.first().unwrap().id(), cat_doc_id);

        let dog_results = fixtures.search(name, "dog").await?;
        assert_eq!(dog_results.first().unwrap().id(), dog_doc_id);

        Ok(())
    }

    #[convex_macro::test_runtime]
    async fn backfill_with_empty_index_adds_no_segments(rt: TestRuntime) -> anyhow::Result<()> {
        let fixtures = TextFixtures::new(rt).await?;
        let IndexMetadata { name, .. } = fixtures.insert_backfilling_text_index().await?;
        let mut flusher = fixtures.new_search_flusher2();
        flusher.step().await?;

        let segments = fixtures.get_segments_metadata(name).await?;
        assert_eq!(0, segments.len());

        Ok(())
    }

    #[convex_macro::test_runtime]
    async fn backfill_with_empty_backfilled_index_new_document_adds_document(
        rt: TestRuntime,
    ) -> anyhow::Result<()> {
        let fixtures = TextFixtures::new(rt).await?;
        let IndexMetadata { name, .. } = fixtures.insert_backfilling_text_index().await?;
        let mut flusher = fixtures.new_search_flusher2();
        flusher.step().await?;

        let doc_id = fixtures.add_document("cat").await?;

        flusher.step().await?;

        fixtures.enable_index(&name).await?;
        let results = fixtures.search(name, "cat").await?;
        assert_eq!(doc_id, results.first().unwrap().id());

        Ok(())
    }

    #[convex_macro::test_runtime]
    async fn backfill_with_non_empty_backfilled_index_new_document_adds_document(
        rt: TestRuntime,
    ) -> anyhow::Result<()> {
        let fixtures = TextFixtures::new(rt).await?;
        let IndexMetadata { name, .. } = fixtures.insert_backfilling_text_index().await?;
        fixtures.add_document("dog").await?;
        let mut flusher = fixtures.new_search_flusher2();
        flusher.step().await?;

        let doc_id = fixtures.add_document("cat").await?;

        flusher.step().await?;

        fixtures.enable_index(&name).await?;
        let results = fixtures.search(name, "cat").await?;
        assert_eq!(doc_id, results.first().unwrap().id());

        Ok(())
    }

    #[convex_macro::test_runtime]
    async fn backfill_with_empty_enabled_index_new_document_adds_document(
        rt: TestRuntime,
    ) -> anyhow::Result<()> {
        let fixtures = TextFixtures::new(rt).await?;
        let IndexMetadata { name, .. } = fixtures.insert_backfilling_text_index().await?;
        let mut flusher = fixtures.new_search_flusher2();
        flusher.step().await?;
        fixtures.enable_index(&name).await?;

        let doc_id = fixtures.add_document("cat").await?;

        flusher.step().await?;

        let results = fixtures.search(name, "cat").await?;
        assert_eq!(doc_id, results.first().unwrap().id());

        Ok(())
    }

    #[convex_macro::test_runtime]
    async fn backfill_with_non_empty_enabled_index_new_document_adds_document(
        rt: TestRuntime,
    ) -> anyhow::Result<()> {
        let fixtures = TextFixtures::new(rt).await?;
        let IndexMetadata { name, .. } = fixtures.insert_backfilling_text_index().await?;
        fixtures.add_document("dog").await?;
        let mut flusher = fixtures.new_search_flusher2();
        flusher.step().await?;
        fixtures.enable_index(&name).await?;

        let doc_id = fixtures.add_document("cat").await?;

        flusher.step().await?;

        let results = fixtures.search(name, "cat").await?;
        assert_eq!(doc_id, results.first().unwrap().id());

        Ok(())
    }

    #[convex_macro::test_runtime]
    async fn backfill_with_non_empty_enabled_index_new_document_adds_new_segment(
        rt: TestRuntime,
    ) -> anyhow::Result<()> {
        let fixtures = TextFixtures::new(rt).await?;
        let IndexMetadata { name, .. } = fixtures.insert_backfilling_text_index().await?;
        fixtures.add_document("dog").await?;
        let mut flusher = fixtures.new_search_flusher2();
        flusher.step().await?;
        fixtures.enable_index(&name).await?;

        fixtures.add_document("cat").await?;

        flusher.step().await?;

        let segments = fixtures.get_segments_metadata(name).await?;
        assert_eq!(segments.len(), 2);

        Ok(())
    }
    #[convex_macro::test_runtime]
    async fn backfill_with_non_empty_backfilled_index_new_document_adds_new_segment(
        rt: TestRuntime,
    ) -> anyhow::Result<()> {
        let fixtures = TextFixtures::new(rt).await?;
        let IndexMetadata { name, .. } = fixtures.insert_backfilling_text_index().await?;
        fixtures.add_document("dog").await?;
        let mut flusher = fixtures.new_search_flusher2();
        flusher.step().await?;

        fixtures.add_document("cat").await?;

        flusher.step().await?;

        fixtures.enable_index(&name).await?;
        let segments = fixtures.get_segments_metadata(name).await?;
        assert_eq!(segments.len(), 2);

        Ok(())
    }
}

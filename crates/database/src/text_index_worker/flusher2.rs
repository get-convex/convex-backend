use std::{
    collections::BTreeMap,
    sync::Arc,
};

#[cfg(any(test, feature = "testing"))]
use common::pause::PauseClient;
use common::{
    knobs::SEARCH_INDEX_SIZE_SOFT_LIMIT,
    persistence::PersistenceReader,
    runtime::Runtime,
    types::TabletIndexName,
};
use search::{
    metrics::SearchType,
    searcher::SegmentTermMetadataFetcher,
};
use storage::Storage;

use crate::{
    index_workers::{
        index_meta::SegmentStatistics,
        search_flusher::{
            IndexBuild,
            SearchFlusher,
            SearchIndexLimits,
        },
        writer::{
            SearchIndexMetadataWriter,
            SearchIndexWriteResult,
        },
    },
    metrics::{
        log_documents_per_new_search_segment,
        log_documents_per_search_segment,
        log_non_deleted_documents_per_search_index,
        log_non_deleted_documents_per_search_segment,
    },
    text_index_worker::text_meta::{
        BuildTextIndexArgs,
        TextIndexConfigParser,
        TextSearchIndex,
    },
    Database,
    Token,
};

#[cfg(any(test, feature = "testing"))]
pub(crate) const FLUSH_RUNNING_LABEL: &str = "flush_running";

pub struct TextIndexFlusher2<RT: Runtime> {
    flusher: SearchFlusher<RT, TextIndexConfigParser>,
    storage: Arc<dyn Storage>,
    segment_term_metadata_fetcher: Arc<dyn SegmentTermMetadataFetcher>,
    writer: SearchIndexMetadataWriter<RT, TextSearchIndex>,

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
    segment_term_metadata_fetcher: Arc<dyn SegmentTermMetadataFetcher>,
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
        segment_term_metadata_fetcher: Arc<dyn SegmentTermMetadataFetcher>,
    ) -> Self {
        Self {
            runtime,
            database,
            reader,
            storage,
            segment_term_metadata_fetcher,
            limits: SearchIndexLimits {
                index_size_soft_limit: *SEARCH_INDEX_SIZE_SOFT_LIMIT,
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
            self.runtime.clone(),
            self.database.clone(),
            self.reader,
            self.storage.clone(),
            self.limits,
        );
        let writer: SearchIndexMetadataWriter<RT, TextSearchIndex> = SearchIndexMetadataWriter::new(
            self.runtime,
            self.database,
            self.storage.clone(),
            SearchType::Text,
        );
        TextIndexFlusher2 {
            flusher,
            storage: self.storage,
            segment_term_metadata_fetcher: self.segment_term_metadata_fetcher,
            writer,
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
        segment_metadata_fetcher: Arc<dyn SegmentTermMetadataFetcher>,
    ) -> Self {
        FlusherBuilder::new(runtime, database, reader, storage, segment_metadata_fetcher).build()
    }

    /// Run one step of the IndexFlusher's main loop.
    ///
    /// Returns a map of IndexName to number of documents indexed for each
    /// index that was built.
    pub(crate) async fn step(&mut self) -> anyhow::Result<(BTreeMap<TabletIndexName, u64>, Token)> {
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

    async fn build_one(&self, job: IndexBuild<TextSearchIndex>) -> anyhow::Result<u64> {
        let timer = crate::metrics::search::build_one_timer();

        let build_index_args = BuildTextIndexArgs {
            search_storage: self.storage.clone(),
            segment_term_metadata_fetcher: self.segment_term_metadata_fetcher.clone(),
        };
        let result = self
            .flusher
            .build_multipart_segment(&job, build_index_args)
            .await?;
        tracing::debug!("Built a text segment for: {result:#?}");

        let SearchIndexWriteResult {
            index_stats,
            new_segment_stats,
            per_segment_stats,
        } = self.writer.commit_flush(&job, result).await?;

        let new_segment_stats = new_segment_stats.unwrap_or_default();
        log_documents_per_new_search_segment(new_segment_stats.num_documents(), SearchType::Text);

        per_segment_stats.into_iter().for_each(|stats| {
            log_documents_per_search_segment(stats.num_documents(), SearchType::Text);
            log_non_deleted_documents_per_search_segment(
                stats.num_non_deleted_documents(),
                SearchType::Text,
            );
        });

        log_documents_per_new_search_segment(index_stats.num_documents(), SearchType::Text);
        log_non_deleted_documents_per_search_index(
            index_stats.num_non_deleted_documents(),
            SearchType::Text,
        );
        timer.finish();
        Ok(new_segment_stats.num_documents())
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

    #[convex_macro::test_runtime]
    async fn backfill_one_doc_added_then_deleted_single_build_does_not_include_deleted_document(
        rt: TestRuntime,
    ) -> anyhow::Result<()> {
        let fixtures = TextFixtures::new(rt).await?;
        let IndexMetadata { name, .. } = fixtures.insert_backfilling_text_index().await?;
        let mut flusher = fixtures.new_search_flusher2();
        flusher.step().await?;

        let doc_id = fixtures.add_document("cat").await?;
        let mut tx = fixtures.db.begin_system().await?;
        tx.delete_inner(doc_id).await?;
        fixtures.db.commit(tx).await?;

        flusher.step().await?;
        fixtures.enable_index(&name).await?;

        let results = fixtures.search(name, "cat").await?;
        assert!(results.is_empty());

        Ok(())
    }

    #[convex_macro::test_runtime]
    async fn backfill_one_doc_added_then_deleted_separate_builds_does_not_include_deleted_document(
        rt: TestRuntime,
    ) -> anyhow::Result<()> {
        let fixtures = TextFixtures::new(rt).await?;
        let IndexMetadata { name, .. } = fixtures.insert_backfilling_text_index().await?;
        let mut flusher = fixtures.new_search_flusher2();

        let doc_id = fixtures.add_document("cat").await?;
        flusher.step().await?;

        let mut tx = fixtures.db.begin_system().await?;
        tx.delete_inner(doc_id).await?;
        fixtures.db.commit(tx).await?;

        flusher.step().await?;
        fixtures.enable_index(&name).await?;

        let results = fixtures.search(name, "cat").await?;
        assert!(results.is_empty());

        Ok(())
    }
}

use std::sync::Arc;

#[cfg(any(test, feature = "testing"))]
use common::pause::PauseClient;
use common::{
    knobs::SEARCH_INDEX_SIZE_SOFT_LIMIT,
    persistence::PersistenceReader,
    runtime::Runtime,
};
use search::searcher::SegmentTermMetadataFetcher;
use storage::Storage;

use crate::{
    index_workers::{
        search_flusher::{
            SearchFlusher,
            SearchIndexLimits,
        },
        writer::SearchIndexMetadataWriter,
    },
    text_index_worker::text_meta::{
        BuildTextIndexArgs,
        TextIndexConfigParser,
        TextSearchIndex,
    },
    Database,
};

#[allow(dead_code)]
#[cfg(any(test, feature = "testing"))]
pub async fn backfill_text_indexes<RT: Runtime>(
    runtime: RT,
    database: Database<RT>,
    reader: Arc<dyn PersistenceReader>,
    storage: Arc<dyn Storage>,
    segment_term_metadata_fetcher: Arc<dyn SegmentTermMetadataFetcher>,
) -> anyhow::Result<()> {
    let writer = SearchIndexMetadataWriter::new(runtime.clone(), database.clone(), storage.clone());
    let mut flusher = FlusherBuilder::new(
        runtime,
        database,
        reader,
        storage,
        segment_term_metadata_fetcher,
        writer,
    )
    .set_soft_limit(0)
    .build();
    flusher.step().await?;
    Ok(())
}

pub(crate) struct FlusherBuilder<RT: Runtime> {
    runtime: RT,
    database: Database<RT>,
    reader: Arc<dyn PersistenceReader>,
    storage: Arc<dyn Storage>,
    segment_term_metadata_fetcher: Arc<dyn SegmentTermMetadataFetcher>,
    limits: SearchIndexLimits,
    writer: SearchIndexMetadataWriter<RT, TextSearchIndex>,
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
        writer: SearchIndexMetadataWriter<RT, TextSearchIndex>,
    ) -> Self {
        Self {
            runtime,
            database,
            reader,
            storage,
            segment_term_metadata_fetcher,
            writer,
            limits: SearchIndexLimits {
                index_size_soft_limit: *SEARCH_INDEX_SIZE_SOFT_LIMIT,
                incremental_multipart_threshold_bytes: *SEARCH_INDEX_SIZE_SOFT_LIMIT,
            },
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
        SearchFlusher::new(
            self.runtime,
            self.database,
            self.reader,
            self.storage.clone(),
            self.limits,
            self.writer,
            BuildTextIndexArgs {
                search_storage: self.storage.clone(),
                segment_term_metadata_fetcher: self.segment_term_metadata_fetcher.clone(),
            },
            #[cfg(any(test, feature = "testing"))]
            self.pause_client,
        )
    }
}

pub type TextIndexFlusher2<RT> = SearchFlusher<RT, TextIndexConfigParser>;

pub(crate) fn new_text_flusher<RT: Runtime>(
    runtime: RT,
    database: Database<RT>,
    reader: Arc<dyn PersistenceReader>,
    storage: Arc<dyn Storage>,
    segment_metadata_fetcher: Arc<dyn SegmentTermMetadataFetcher>,
    writer: SearchIndexMetadataWriter<RT, TextSearchIndex>,
) -> TextIndexFlusher2<RT> {
    FlusherBuilder::new(
        runtime,
        database,
        reader,
        storage,
        segment_metadata_fetcher,
        writer,
    )
    .build()
}

#[cfg(test)]
mod tests {
    use common::{
        bootstrap_model::index::{
            text_index::TextIndexState,
            IndexConfig,
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
        let index_data = fixtures.insert_backfilling_text_index().await?;
        let mut flusher = fixtures.new_search_flusher2();
        flusher.step().await?;
        fixtures.assert_backfilled(&index_data.index_name).await?;
        Ok(())
    }

    #[convex_macro::test_runtime]
    async fn backfill_with_no_documents_returns_index_in_metrics(
        rt: TestRuntime,
    ) -> anyhow::Result<()> {
        let fixtures = TextFixtures::new(rt).await?;
        let index_data = fixtures.insert_backfilling_text_index().await?;
        let mut tx = fixtures.db.begin_system().await?;
        let table_id = tx
            .table_mapping()
            .namespace(TableNamespace::Global)
            .id(index_data.index_name.table())?
            .tablet_id;
        let resolved_index_name =
            TabletIndexName::new(table_id, index_data.index_name.descriptor().clone())?;
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
        let index_data = fixtures
            .insert_backfilling_text_index_with_document()
            .await?;
        let mut flusher = fixtures.new_search_flusher2();
        flusher.step().await?;
        fixtures.assert_backfilled(&index_data.index_name).await?;
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
        let index_data = fixtures.insert_backfilling_text_index().await?;
        let doc_id = fixtures.add_document("cat").await?;
        let mut flusher = fixtures.new_search_flusher2();
        flusher.step().await?;
        fixtures.enable_index(&index_data.index_name).await?;

        let results = fixtures.search(index_data.index_name, "cat").await?;
        assert_eq!(results.first().unwrap().id(), doc_id);

        Ok(())
    }

    #[convex_macro::test_runtime]
    async fn backfill_with_two_documents_0_max_segment_size_creates_two_segments(
        rt: TestRuntime,
    ) -> anyhow::Result<()> {
        let fixtures = TextFixtures::new(rt).await?;
        let IndexData { index_name, .. } = fixtures.insert_backfilling_text_index().await?;

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

        let segments = fixtures.get_segments_metadata(index_name).await?;
        assert_eq!(segments.len(), 2);
        Ok(())
    }

    #[convex_macro::test_runtime]
    async fn backfill_with_two_documents_leaves_document_backfilling_after_first_flush(
        rt: TestRuntime,
    ) -> anyhow::Result<()> {
        let fixtures = TextFixtures::new(rt).await?;
        let index_data = fixtures.insert_backfilling_text_index().await?;

        fixtures.add_document("cat").await?;
        fixtures.add_document("dog").await?;

        let mut flusher = fixtures
            .new_search_flusher_builder()
            .set_incremental_multipart_threshold_bytes(0)
            .build();
        // Build the first segment, which stops because the document size is > 0
        flusher.step().await?;
        let metadata = fixtures.get_index_metadata(index_data.index_name).await?;
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
        let IndexData { index_name, .. } = fixtures.insert_backfilling_text_index().await?;

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

        fixtures.enable_index(&index_name).await?;

        let cat_results = fixtures.search(index_name.clone(), "cat").await?;
        assert_eq!(cat_results.first().unwrap().id(), cat_doc_id);

        let dog_results = fixtures.search(index_name, "dog").await?;
        assert_eq!(dog_results.first().unwrap().id(), dog_doc_id);

        Ok(())
    }

    #[convex_macro::test_runtime]
    async fn backfill_with_empty_index_adds_no_segments(rt: TestRuntime) -> anyhow::Result<()> {
        let fixtures = TextFixtures::new(rt).await?;
        let IndexData { index_name, .. } = fixtures.insert_backfilling_text_index().await?;
        let mut flusher = fixtures.new_search_flusher2();
        flusher.step().await?;

        let segments = fixtures.get_segments_metadata(index_name).await?;
        assert_eq!(0, segments.len());

        Ok(())
    }

    #[convex_macro::test_runtime]
    async fn backfill_with_empty_backfilled_index_new_document_adds_document(
        rt: TestRuntime,
    ) -> anyhow::Result<()> {
        let fixtures = TextFixtures::new(rt).await?;
        let IndexData { index_name, .. } = fixtures.insert_backfilling_text_index().await?;
        let mut flusher = fixtures.new_search_flusher2();
        flusher.step().await?;

        let doc_id = fixtures.add_document("cat").await?;

        flusher.step().await?;

        fixtures.enable_index(&index_name).await?;
        let results = fixtures.search(index_name, "cat").await?;
        assert_eq!(doc_id, results.first().unwrap().id());

        Ok(())
    }

    #[convex_macro::test_runtime]
    async fn backfill_with_non_empty_backfilled_index_new_document_adds_document(
        rt: TestRuntime,
    ) -> anyhow::Result<()> {
        let fixtures = TextFixtures::new(rt).await?;
        let IndexData { index_name, .. } = fixtures.insert_backfilling_text_index().await?;
        fixtures.add_document("dog").await?;
        let mut flusher = fixtures.new_search_flusher2();
        flusher.step().await?;

        let doc_id = fixtures.add_document("cat").await?;

        flusher.step().await?;

        fixtures.enable_index(&index_name).await?;
        let results = fixtures.search(index_name, "cat").await?;
        assert_eq!(doc_id, results.first().unwrap().id());

        Ok(())
    }

    #[convex_macro::test_runtime]
    async fn backfill_with_empty_enabled_index_new_document_adds_document(
        rt: TestRuntime,
    ) -> anyhow::Result<()> {
        let fixtures = TextFixtures::new(rt).await?;
        let IndexData { index_name, .. } = fixtures.insert_backfilling_text_index().await?;
        let mut flusher = fixtures.new_search_flusher2();
        flusher.step().await?;
        fixtures.enable_index(&index_name).await?;

        let doc_id = fixtures.add_document("cat").await?;

        flusher.step().await?;

        let results = fixtures.search(index_name, "cat").await?;
        assert_eq!(doc_id, results.first().unwrap().id());

        Ok(())
    }

    #[convex_macro::test_runtime]
    async fn backfill_with_non_empty_enabled_index_new_document_adds_document(
        rt: TestRuntime,
    ) -> anyhow::Result<()> {
        let fixtures = TextFixtures::new(rt).await?;
        let IndexData { index_name, .. } = fixtures.insert_backfilling_text_index().await?;
        fixtures.add_document("dog").await?;
        let mut flusher = fixtures.new_search_flusher2();
        flusher.step().await?;
        fixtures.enable_index(&index_name).await?;

        let doc_id = fixtures.add_document("cat").await?;

        flusher.step().await?;

        let results = fixtures.search(index_name, "cat").await?;
        assert_eq!(doc_id, results.first().unwrap().id());

        Ok(())
    }

    #[convex_macro::test_runtime]
    async fn backfill_with_non_empty_enabled_index_new_document_adds_new_segment(
        rt: TestRuntime,
    ) -> anyhow::Result<()> {
        let fixtures = TextFixtures::new(rt).await?;
        let IndexData { index_name, .. } = fixtures.insert_backfilling_text_index().await?;
        fixtures.add_document("dog").await?;
        let mut flusher = fixtures.new_search_flusher2();
        flusher.step().await?;
        fixtures.enable_index(&index_name).await?;

        fixtures.add_document("cat").await?;

        flusher.step().await?;

        let segments = fixtures.get_segments_metadata(index_name).await?;
        assert_eq!(segments.len(), 2);

        Ok(())
    }
    #[convex_macro::test_runtime]
    async fn backfill_with_non_empty_backfilled_index_new_document_adds_new_segment(
        rt: TestRuntime,
    ) -> anyhow::Result<()> {
        let fixtures = TextFixtures::new(rt).await?;
        let IndexData { index_name, .. } = fixtures.insert_backfilling_text_index().await?;
        fixtures.add_document("dog").await?;
        let mut flusher = fixtures.new_search_flusher2();
        flusher.step().await?;

        fixtures.add_document("cat").await?;

        flusher.step().await?;

        fixtures.enable_index(&index_name).await?;
        let segments = fixtures.get_segments_metadata(index_name).await?;
        assert_eq!(segments.len(), 2);

        Ok(())
    }

    #[convex_macro::test_runtime]
    async fn backfill_one_doc_added_then_deleted_single_build_does_not_include_deleted_document(
        rt: TestRuntime,
    ) -> anyhow::Result<()> {
        let fixtures = TextFixtures::new(rt).await?;
        let IndexData { index_name, .. } = fixtures.insert_backfilling_text_index().await?;
        let mut flusher = fixtures.new_search_flusher2();
        flusher.step().await?;

        let doc_id = fixtures.add_document("cat").await?;
        let mut tx = fixtures.db.begin_system().await?;
        tx.delete_inner(doc_id).await?;
        fixtures.db.commit(tx).await?;

        flusher.step().await?;
        fixtures.enable_index(&index_name).await?;

        let results = fixtures.search(index_name, "cat").await?;
        assert!(results.is_empty());

        Ok(())
    }

    #[convex_macro::test_runtime]
    async fn backfill_one_doc_added_then_deleted_separate_builds_does_not_include_deleted_document(
        rt: TestRuntime,
    ) -> anyhow::Result<()> {
        let fixtures = TextFixtures::new(rt).await?;
        let IndexData { index_name, .. } = fixtures.insert_backfilling_text_index().await?;
        let mut flusher = fixtures.new_search_flusher2();

        let doc_id = fixtures.add_document("cat").await?;
        flusher.step().await?;

        let mut tx = fixtures.db.begin_system().await?;
        tx.delete_inner(doc_id).await?;
        fixtures.db.commit(tx).await?;

        flusher.step().await?;
        fixtures.enable_index(&index_name).await?;

        let results = fixtures.search(index_name, "cat").await?;
        assert!(results.is_empty());

        Ok(())
    }
}

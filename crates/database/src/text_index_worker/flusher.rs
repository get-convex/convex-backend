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
        TextSearchIndex,
    },
    Database,
};

#[cfg(any(test, feature = "testing"))]
pub async fn backfill_text_indexes<RT: Runtime>(
    runtime: RT,
    database: Database<RT>,
    reader: Arc<dyn PersistenceReader>,
    storage: Arc<dyn Storage>,
    segment_term_metadata_fetcher: Arc<dyn SegmentTermMetadataFetcher>,
) -> anyhow::Result<()> {
    let writer = SearchIndexMetadataWriter::new(
        runtime.clone(),
        database.clone(),
        reader.clone(),
        storage.clone(),
        BuildTextIndexArgs {
            search_storage: storage.clone(),
            segment_term_metadata_fetcher: segment_term_metadata_fetcher.clone(),
        },
    );
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

    #[cfg(any(test, feature = "testing"))]
    pub fn set_soft_limit(self, limit: usize) -> Self {
        Self {
            limits: SearchIndexLimits {
                index_size_soft_limit: limit,
                ..self.limits
            },
            ..self
        }
    }

    #[cfg(any(test, feature = "testing"))]
    #[cfg_attr(not(test), expect(dead_code))]
    pub fn set_incremental_multipart_threshold_bytes(self, limit: usize) -> Self {
        Self {
            limits: SearchIndexLimits {
                incremental_multipart_threshold_bytes: limit,
                ..self.limits
            },
            ..self
        }
    }

    pub(crate) fn build(self) -> TextIndexFlusher<RT> {
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

pub type TextIndexFlusher<RT> = SearchFlusher<RT, TextSearchIndex>;

#[allow(unused)]
#[cfg(any(test, feature = "testing"))]
pub fn new_text_flusher_for_tests<RT: Runtime>(
    runtime: RT,
    database: Database<RT>,
    reader: Arc<dyn PersistenceReader>,
    storage: Arc<dyn Storage>,
    segment_metadata_fetcher: Arc<dyn SegmentTermMetadataFetcher>,
) -> TextIndexFlusher<RT> {
    let writer = SearchIndexMetadataWriter::new(
        runtime.clone(),
        database.clone(),
        reader.clone(),
        storage.clone(),
        BuildTextIndexArgs {
            search_storage: storage.clone(),
            segment_term_metadata_fetcher: segment_metadata_fetcher.clone(),
        },
    );
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

pub(crate) fn new_text_flusher<RT: Runtime>(
    runtime: RT,
    database: Database<RT>,
    reader: Arc<dyn PersistenceReader>,
    storage: Arc<dyn Storage>,
    segment_metadata_fetcher: Arc<dyn SegmentTermMetadataFetcher>,
    writer: SearchIndexMetadataWriter<RT, TextSearchIndex>,
) -> TextIndexFlusher<RT> {
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
    use std::time::Duration;

    use anyhow::Context;
    use common::{
        bootstrap_model::index::{
            text_index::{
                TextIndexBackfillState,
                TextIndexSnapshot,
                TextIndexSnapshotData,
                TextIndexState,
                TextSnapshotVersion,
            },
            IndexConfig,
            IndexMetadata,
        },
        runtime::testing::TestRuntime,
        types::{
            IndexName,
            ObjectKey,
            TabletIndexName,
        },
    };
    use maplit::btreemap;
    use must_let::must_let;
    use sync_types::Timestamp;
    use value::{
        assert_obj,
        TableNamespace,
    };

    use crate::{
        tests::text_test_utils::{
            add_document,
            backfilling_text_index,
            IndexData,
            TextFixtures,
        },
        Database,
        IndexModel,
        SystemMetadataModel,
        TestFacingModel,
    };

    async fn assert_snapshotted(
        database: &Database<TestRuntime>,
        namespace: TableNamespace,
        index_name: &IndexName,
    ) -> anyhow::Result<Timestamp> {
        let mut tx = database.begin_system().await?;
        let new_metadata = IndexModel::new(&mut tx)
            .enabled_index_metadata(namespace, index_name)?
            .context("Index missing or in an unexpected state")?
            .into_value();
        must_let!(let IndexMetadata {
            config: IndexConfig::Search {
                on_disk_state: TextIndexState::SnapshottedAt(TextIndexSnapshot { ts, .. }),
                ..
            },
            ..
        } = new_metadata);
        Ok(ts)
    }

    async fn enable_pending_index(
        database: &Database<TestRuntime>,
        namespace: TableNamespace,
        index_name: &IndexName,
    ) -> anyhow::Result<()> {
        let mut tx = database.begin_system().await.unwrap();
        let mut model = IndexModel::new(&mut tx);
        let index = model
            .pending_index_metadata(namespace, index_name)?
            .context(format!("Missing pending index for {index_name:?}"))?;
        model
            .enable_backfilled_indexes(vec![index.into_value()])
            .await?;
        database.commit(tx).await?;
        Ok(())
    }

    #[convex_macro::test_runtime]
    async fn test_build_search_index(rt: TestRuntime) -> anyhow::Result<()> {
        let fixtures = TextFixtures::new(rt.clone()).await?;

        let IndexData {
            index_name,
            resolved_index_name,
            ..
        } = fixtures
            .insert_backfilling_text_index_with_document()
            .await?;
        let mut worker = fixtures.new_search_flusher();

        // Run one interation of the search index worker.
        let (metrics, _) = worker.step().await?;

        // Make sure we actually built this index with one document.
        assert_eq!(metrics, btreemap! {resolved_index_name.clone() => 1});

        // Check that the metadata is updated so it's no longer backfilling.
        fixtures.assert_backfilled(&index_name).await?;

        Ok(())
    }

    #[convex_macro::test_runtime]
    async fn test_rebuild_backfilled_search_index(rt: TestRuntime) -> anyhow::Result<()> {
        let fixtures = TextFixtures::new(rt.clone()).await?;
        let database = &fixtures.db;

        let IndexData {
            index_name,
            resolved_index_name,
            ..
        } = fixtures
            .insert_backfilling_text_index_with_document()
            .await?;
        let mut worker = fixtures.new_search_flusher();

        // Run one interation of the search index worker.
        let (metrics, _) = worker.step().await?;

        // Make sure we actually built this index with one document.
        assert_eq!(metrics, btreemap! {resolved_index_name.clone() => 1});

        // Check that the metadata is updated so it's no longer backfilling.
        let initial_snapshot_ts = fixtures.assert_backfilled(&index_name).await?;

        // Write 10 more documents into the table to trigger a new snapshot.
        let mut tx = database.begin_system().await.unwrap();
        let num_new_documents = 10;
        for _ in 0..num_new_documents {
            add_document(
                &mut tx,
                index_name.table(),
                "hello world, this is a message with more than just a few terms in it",
            )
            .await?;
        }
        database.commit(tx).await?;

        let (metrics, _) = worker.step().await?;
        assert_eq!(
            metrics,
            btreemap! {resolved_index_name.clone() => num_new_documents}
        );

        // Check that the metadata is updated so it's no longer backfilling.
        let new_snapshot_ts = fixtures.assert_backfilled(&index_name).await?;
        assert!(new_snapshot_ts > initial_snapshot_ts);

        Ok(())
    }

    #[convex_macro::test_runtime]
    async fn test_rebuild_enabled_search_index(rt: TestRuntime) -> anyhow::Result<()> {
        let fixtures = TextFixtures::new(rt.clone()).await?;

        let IndexData {
            index_name,
            resolved_index_name,
            namespace,
            ..
        } = fixtures
            .insert_backfilling_text_index_with_document()
            .await?;
        let mut worker = fixtures.new_search_flusher();

        // Run one interation of the search index worker.
        let (metrics, _) = worker.step().await?;

        // Make sure we actually built this index with one document.
        assert_eq!(metrics, btreemap! {resolved_index_name.clone() => 1});
        // Check that the metadata is updated so it's no longer backfilling.
        let initial_snapshot_ts = fixtures.assert_backfilled(&index_name).await?;
        // Enable the index so it's in the Snapshotted state.
        enable_pending_index(&fixtures.db, namespace, &index_name).await?;
        // Write 10 more documents into the table to trigger a new snapshot.
        let mut tx = fixtures.db.begin_system().await.unwrap();
        let num_new_documents = 10;
        for _ in 0..num_new_documents {
            add_document(
                &mut tx,
                index_name.table(),
                "hello world, this is a message with more than just a few terms in it",
            )
            .await?;
        }
        fixtures.db.commit(tx).await?;

        let (metrics, _) = worker.step().await?;
        assert_eq!(
            metrics,
            btreemap! {resolved_index_name.clone() => num_new_documents}
        );

        // Check that the metadata is updated and still enabled.
        let new_snapshot_ts = assert_snapshotted(&fixtures.db, namespace, &index_name).await?;
        assert!(new_snapshot_ts > initial_snapshot_ts);

        Ok(())
    }

    #[convex_macro::test_runtime]
    async fn test_advance_old_snapshot(rt: TestRuntime) -> anyhow::Result<()> {
        common::testing::init_test_logging();
        let fixtures = TextFixtures::new(rt.clone()).await?;
        let mut worker = fixtures.new_search_flusher_with_soft_limit();
        let database = &fixtures.db;

        let IndexData {
            index_name,
            resolved_index_name,
            ..
        } = fixtures
            .insert_backfilling_text_index_with_document()
            .await?;

        let (metrics, _) = worker.step().await?;
        assert_eq!(metrics, btreemap! {resolved_index_name.clone() => 1});
        let initial_snapshot_ts = fixtures.assert_backfilled(&index_name).await?;

        // Write a single document underneath our soft limit and check that we don't
        // snapshot.
        let mut tx = database.begin_system().await?;
        add_document(&mut tx, index_name.table(), "too small to count").await?;
        database.commit(tx).await?;

        let (metrics, _) = worker.step().await?;
        assert!(metrics.is_empty());
        assert_eq!(
            initial_snapshot_ts,
            fixtures.assert_backfilled(&index_name).await?
        );

        // Advance time past the max index age (and do an unrelated commit to bump the
        // repeatable timestamp).
        rt.advance_time(Duration::from_secs(7200)).await;
        let mut tx = database.begin_system().await?;
        let unrelated_document = assert_obj!("wise" => "ambience");
        TestFacingModel::new(&mut tx)
            .insert(&"unrelated".parse()?, unrelated_document)
            .await?;
        database.commit(tx).await?;

        let (metrics, _) = worker.step().await?;
        assert_eq!(metrics, btreemap! {resolved_index_name.clone() => 1});
        assert!(initial_snapshot_ts < fixtures.assert_backfilled(&index_name).await?);

        Ok(())
    }

    #[convex_macro::test_runtime]
    async fn backfill_with_no_documents_sets_state_to_backfilled(
        rt: TestRuntime,
    ) -> anyhow::Result<()> {
        let fixtures = TextFixtures::new(rt).await?;
        let index_data = fixtures.insert_backfilling_text_index().await?;
        let mut flusher = fixtures.new_search_flusher();
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
            .namespace(TableNamespace::test_user())
            .id(index_data.index_name.table())?
            .tablet_id;
        let resolved_index_name =
            TabletIndexName::new(table_id, index_data.index_name.descriptor().clone())?;
        let mut flusher = fixtures.new_search_flusher();
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
        let mut flusher = fixtures.new_search_flusher();
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
        let mut flusher = fixtures.new_search_flusher();
        let (metrics, _) = flusher.step().await?;
        assert_eq!(metrics, btreemap! { resolved_index_name => 1 });
        Ok(())
    }

    #[convex_macro::test_runtime]
    async fn backfill_with_one_document_writes_document(rt: TestRuntime) -> anyhow::Result<()> {
        let fixtures = TextFixtures::new(rt).await?;
        let index_data = fixtures.insert_backfilling_text_index().await?;
        let doc_id = fixtures.add_document("cat").await?;
        let mut flusher = fixtures.new_search_flusher();
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
        let mut flusher = fixtures.new_search_flusher();
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
        let mut flusher = fixtures.new_search_flusher();
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
        let mut flusher = fixtures.new_search_flusher();
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
        let mut flusher = fixtures.new_search_flusher();
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
        let mut flusher = fixtures.new_search_flusher();
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
        let mut flusher = fixtures.new_search_flusher();
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
        let mut flusher = fixtures.new_search_flusher();
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
        let mut flusher = fixtures.new_search_flusher();
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
        let mut flusher = fixtures.new_search_flusher();

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

    #[convex_macro::test_runtime]
    async fn backfill_one_doc_added_then_replaced_separate_builds_does_not_include_first_document(
        rt: TestRuntime,
    ) -> anyhow::Result<()> {
        let fixtures = TextFixtures::new(rt).await?;
        let IndexData { index_name, .. } = fixtures.insert_backfilling_text_index().await?;
        let mut flusher = fixtures.new_search_flusher();

        let doc_id = fixtures.add_document("cat").await?;
        flusher.step().await?;

        let mut tx = fixtures.db.begin_system().await?;
        tx.replace_inner(doc_id, assert_obj!()).await?;
        fixtures.db.commit(tx).await?;

        flusher.step().await?;
        fixtures.enable_index(&index_name).await?;

        let results = fixtures.search(index_name, "cat").await?;
        assert!(results.is_empty());

        Ok(())
    }

    #[convex_macro::test_runtime]
    async fn backfill_insert_replace_one_segment(rt: TestRuntime) -> anyhow::Result<()> {
        let fixtures = TextFixtures::new(rt).await?;
        let IndexData { index_name, .. } = fixtures.insert_backfilling_text_index().await?;
        let mut flusher = fixtures.new_search_flusher();

        let doc_id = fixtures.add_document("cat").await?;
        fixtures.replace_document(doc_id, "new_text").await?;

        flusher.step().await?;
        fixtures.enable_index(&index_name).await?;

        let results = fixtures.search(index_name.clone(), "cat").await?;
        assert!(results.is_empty());
        let results = fixtures.search(index_name, "new_text").await?;
        assert!(!results.is_empty());

        Ok(())
    }
    #[convex_macro::test_runtime]
    async fn backfill_insert_replace_delete_one_segment(rt: TestRuntime) -> anyhow::Result<()> {
        let fixtures = TextFixtures::new(rt).await?;
        let IndexData { index_name, .. } = fixtures.insert_backfilling_text_index().await?;
        let mut flusher = fixtures.new_search_flusher();

        let doc_id = fixtures.add_document("cat").await?;
        fixtures.replace_document(doc_id, "new_text").await?;
        let mut tx = fixtures.db.begin_system().await?;
        tx.delete_inner(doc_id).await?;
        fixtures.db.commit(tx).await?;

        flusher.step().await?;
        fixtures.enable_index(&index_name).await?;

        let results = fixtures.search(index_name.clone(), "cat").await?;
        assert!(results.is_empty());
        let results = fixtures.search(index_name, "new_text").await?;
        assert!(results.is_empty());

        Ok(())
    }

    #[convex_macro::test_runtime]
    async fn backfill_insert_then_replace_delete_separate_segment(
        rt: TestRuntime,
    ) -> anyhow::Result<()> {
        let fixtures = TextFixtures::new(rt).await?;
        let IndexData { index_name, .. } = fixtures.insert_backfilling_text_index().await?;
        let mut flusher = fixtures.new_search_flusher();

        let doc_id = fixtures.add_document("cat").await?;
        flusher.step().await?;
        fixtures.replace_document(doc_id, "new_text").await?;
        let mut tx = fixtures.db.begin_system().await?;
        tx.delete_inner(doc_id).await?;
        fixtures.db.commit(tx).await?;

        fixtures.enable_index(&index_name).await?;

        let results = fixtures.search(index_name.clone(), "cat").await?;
        assert!(results.is_empty());
        let results = fixtures.search(index_name, "new_text").await?;
        assert!(results.is_empty());

        Ok(())
    }

    #[convex_macro::test_runtime]
    async fn backfill_insert_then_replace_delete_separate_segment_many_replaces(
        rt: TestRuntime,
    ) -> anyhow::Result<()> {
        let fixtures = TextFixtures::new(rt).await?;
        let IndexData { index_name, .. } = fixtures.insert_backfilling_text_index().await?;
        let mut flusher = fixtures.new_search_flusher();

        let doc_id = fixtures.add_document("cat").await?;
        fixtures.replace_document(doc_id, "dog").await?;
        flusher.step().await?;
        fixtures.replace_document(doc_id, "newer_text").await?;
        flusher.step().await?;
        fixtures.enable_index(&index_name).await?;

        let results = fixtures.search(index_name.clone(), "cat").await?;
        assert!(results.is_empty());
        let results = fixtures.search(index_name.clone(), "dog").await?;
        assert!(results.is_empty());
        let results = fixtures.search(index_name, "newer_text").await?;
        assert!(!results.is_empty());

        Ok(())
    }

    #[convex_macro::test_runtime]
    async fn backfill_insert_then_replace_delete_second_segment(
        rt: TestRuntime,
    ) -> anyhow::Result<()> {
        let fixtures = TextFixtures::new(rt).await?;
        let IndexData { index_name, .. } = fixtures.insert_backfilling_text_index().await?;
        let mut flusher = fixtures.new_search_flusher();

        let doc_id = fixtures.add_document("cat").await?;
        flusher.step().await?;

        fixtures.replace_document(doc_id, "new_text").await?;

        let mut tx = fixtures.db.begin_system().await?;
        tx.delete_inner(doc_id).await?;
        fixtures.db.commit(tx).await?;

        flusher.step().await?;
        fixtures.enable_index(&index_name).await?;

        let results = fixtures.search(index_name.clone(), "cat").await?;
        assert!(results.is_empty());
        let results = fixtures.search(index_name, "new_text").await?;
        assert!(results.is_empty());

        Ok(())
    }

    #[convex_macro::test_runtime]
    async fn backfill_insert_then_replace_delete_separate_segments(
        rt: TestRuntime,
    ) -> anyhow::Result<()> {
        let fixtures = TextFixtures::new(rt).await?;
        let IndexData { index_name, .. } = fixtures.insert_backfilling_text_index().await?;
        let mut flusher = fixtures.new_search_flusher();

        let doc_id = fixtures.add_document("cat").await?;
        flusher.step().await?;

        fixtures.replace_document(doc_id, "new_text").await?;
        flusher.step().await?;

        let mut tx = fixtures.db.begin_system().await?;
        tx.delete_inner(doc_id).await?;
        fixtures.db.commit(tx).await?;

        flusher.step().await?;
        fixtures.enable_index(&index_name).await?;

        let results = fixtures.search(index_name.clone(), "cat").await?;
        assert!(results.is_empty());
        let results = fixtures.search(index_name, "new_text").await?;
        assert!(results.is_empty());

        Ok(())
    }

    #[convex_macro::test_runtime]
    async fn backfill_insert_replace_replace_delete_single_segment(
        rt: TestRuntime,
    ) -> anyhow::Result<()> {
        let fixtures = TextFixtures::new(rt).await?;
        let IndexData { index_name, .. } = fixtures.insert_backfilling_text_index().await?;
        let mut flusher = fixtures.new_search_flusher();

        let doc_id = fixtures.add_document("cat").await?;
        flusher.step().await?;

        fixtures.replace_document(doc_id, "new_text").await?;
        fixtures.replace_document(doc_id, "really_new_text").await?;

        let mut tx = fixtures.db.begin_system().await?;
        tx.delete_inner(doc_id).await?;
        fixtures.db.commit(tx).await?;

        flusher.step().await?;
        fixtures.enable_index(&index_name).await?;

        let results = fixtures.search(index_name, "really_new_text").await?;
        assert!(results.is_empty());

        Ok(())
    }

    #[convex_macro::test_runtime]
    async fn backfill_insert_replace_replace_delete_different_segments(
        rt: TestRuntime,
    ) -> anyhow::Result<()> {
        let fixtures = TextFixtures::new(rt).await?;
        let IndexData { index_name, .. } = fixtures.insert_backfilling_text_index().await?;
        let mut flusher = fixtures.new_search_flusher();

        let doc_id = fixtures.add_document("cat").await?;
        flusher.step().await?;

        fixtures.replace_document(doc_id, "new_text").await?;
        flusher.step().await?;
        fixtures.replace_document(doc_id, "really_new_text").await?;
        flusher.step().await?;

        let mut tx = fixtures.db.begin_system().await?;
        tx.delete_inner(doc_id).await?;
        fixtures.db.commit(tx).await?;

        flusher.step().await?;
        fixtures.enable_index(&index_name).await?;

        let results = fixtures.search(index_name, "really_new_text").await?;
        assert!(results.is_empty());

        Ok(())
    }

    #[convex_macro::test_runtime]
    async fn backfill_with_backfilled_single_segment_format_backfills_with_multi_segment_format(
        rt: TestRuntime,
    ) -> anyhow::Result<()> {
        let fixtures = TextFixtures::new(rt).await?;
        let index_name = create_backfilled_single_segment_text_index(&fixtures).await?;

        fixtures.add_document("cat").await?;

        let mut flusher = fixtures.new_search_flusher();
        flusher.step().await?;

        fixtures.enable_index(&index_name).await?;

        let results = fixtures.search(index_name.clone(), "cat").await?;
        assert_eq!(results.len(), 1);
        let segments = fixtures.get_segments_metadata(index_name).await?;
        assert_eq!(segments.len(), 1);
        Ok(())
    }

    #[convex_macro::test_runtime]
    async fn backfill_with_snapshotted_at_single_segment_format_backfills_with_multi_segment_format(
        rt: TestRuntime,
    ) -> anyhow::Result<()> {
        let fixtures = TextFixtures::new(rt).await?;

        let index_name = create_backfilled_single_segment_text_index(&fixtures).await?;

        let mut tx = fixtures.db.begin_system().await?;
        IndexModel::new(&mut tx)
            .enable_index_for_testing(TableNamespace::Global, &index_name)
            .await?;
        fixtures.db.commit(tx).await?;

        fixtures.add_document("cat").await?;

        let mut flusher = fixtures.new_search_flusher();
        flusher.step().await?;

        let results = fixtures.search(index_name.clone(), "cat").await?;
        assert_eq!(results.len(), 1);
        let segments = fixtures.get_segments_metadata(index_name).await?;
        assert_eq!(segments.len(), 1);
        Ok(())
    }

    async fn create_backfilled_single_segment_text_index(
        fixtures: &TextFixtures,
    ) -> anyhow::Result<IndexName> {
        let mut tx = fixtures.db.begin_system().await?;
        let metadata = backfilling_text_index()?;
        let on_disk_state = TextIndexState::Backfilling(TextIndexBackfillState::new());
        must_let!(let IndexConfig::Search {
            developer_config,
            ..
        } = metadata.config);
        let doc_id = IndexModel::new(&mut tx)
            .add_application_index(
                TableNamespace::Global,
                IndexMetadata::new_search_index(
                    metadata.name.clone(),
                    developer_config,
                    on_disk_state,
                ),
            )
            .await?;

        fixtures.db.commit(tx).await?;
        let mut tx = fixtures.db.begin_system().await?;
        let indexes = IndexModel::new(&mut tx).get_all_indexes().await?;
        let index = indexes
            .into_iter()
            .find(|index| index.id() == doc_id)
            .unwrap();
        let (id, value) = index.into_id_and_value();
        must_let!(let IndexConfig::Search {
            developer_config,
            ..
        } = value.config);
        let on_disk_state = TextIndexState::Backfilled(TextIndexSnapshot {
            data: TextIndexSnapshotData::SingleSegment(ObjectKey::try_from("Fake".to_string())?),
            ts: *tx.begin_timestamp(),
            version: TextSnapshotVersion::V2UseStringIds,
        });

        SystemMetadataModel::new_global(&mut tx)
            .replace(
                id,
                IndexMetadata::new_search_index(value.name, developer_config, on_disk_state)
                    .try_into()?,
            )
            .await?;
        fixtures.db.commit(tx).await?;
        Ok(metadata.name)
    }
}

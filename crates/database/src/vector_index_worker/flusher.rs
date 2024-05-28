use std::sync::Arc;

#[cfg(any(test, feature = "testing"))]
use common::pause::PauseClient;
use common::{
    knobs::{
        MULTI_SEGMENT_FULL_SCAN_THRESHOLD_KB,
        VECTOR_INDEX_SIZE_SOFT_LIMIT,
    },
    persistence::PersistenceReader,
    runtime::Runtime,
};
use search::metrics::SearchType;
use storage::Storage;

use super::vector_meta::BuildVectorIndexArgs;
use crate::{
    index_workers::{
        search_flusher::{
            SearchFlusher,
            SearchIndexLimits,
        },
        writer::SearchIndexMetadataWriter,
    },
    vector_index_worker::vector_meta::{
        VectorIndexConfigParser,
        VectorSearchIndex,
    },
    Database,
};

pub type VectorIndexFlusher<RT> = SearchFlusher<RT, VectorIndexConfigParser>;

/// Backfills all search indexes that are in a "backfilling" state.
#[cfg(any(test, feature = "testing"))]
pub async fn backfill_vector_indexes<RT: Runtime>(
    runtime: RT,
    database: Database<RT>,
    reader: Arc<dyn PersistenceReader>,
    storage: Arc<dyn Storage>,
) -> anyhow::Result<()> {
    let mut flusher = new_vector_flusher_for_tests(
        runtime,
        database,
        reader,
        storage,
        /* index_size_soft_limit= */ 0,
        *MULTI_SEGMENT_FULL_SCAN_THRESHOLD_KB,
        *VECTOR_INDEX_SIZE_SOFT_LIMIT,
        None,
    );
    flusher.step().await?;
    Ok(())
}

#[allow(unused)]
#[cfg(any(test, feature = "testing"))]
pub(crate) fn new_vector_flusher_for_tests<RT: Runtime>(
    runtime: RT,
    database: Database<RT>,
    reader: Arc<dyn PersistenceReader>,
    storage: Arc<dyn Storage>,
    index_size_soft_limit: usize,
    full_scan_segment_max_kb: usize,
    incremental_multipart_threshold_bytes: usize,
    pause_client: Option<PauseClient>,
) -> VectorIndexFlusher<RT> {
    use search::metrics::SearchType;
    let writer = SearchIndexMetadataWriter::new(
        runtime.clone(),
        database.clone(),
        storage.clone(),
        SearchType::Vector,
    );
    SearchFlusher::new(
        runtime,
        database,
        reader,
        storage,
        SearchIndexLimits {
            index_size_soft_limit,
            incremental_multipart_threshold_bytes,
        },
        writer,
        SearchType::Vector,
        BuildVectorIndexArgs {
            full_scan_threshold_bytes: full_scan_segment_max_kb,
        },
        pause_client,
    )
}

pub(crate) fn new_vector_flusher<RT: Runtime>(
    runtime: RT,
    database: Database<RT>,
    reader: Arc<dyn PersistenceReader>,
    storage: Arc<dyn Storage>,
    writer: SearchIndexMetadataWriter<RT, VectorSearchIndex>,
) -> VectorIndexFlusher<RT> {
    SearchFlusher::new(
        runtime,
        database,
        reader,
        storage,
        SearchIndexLimits {
            index_size_soft_limit: *VECTOR_INDEX_SIZE_SOFT_LIMIT,
            incremental_multipart_threshold_bytes: *VECTOR_INDEX_SIZE_SOFT_LIMIT,
        },
        writer,
        SearchType::Vector,
        BuildVectorIndexArgs {
            full_scan_threshold_bytes: *MULTI_SEGMENT_FULL_SCAN_THRESHOLD_KB,
        },
        #[cfg(any(test, feature = "testing"))]
        None,
    )
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use common::{
        bootstrap_model::index::{
            vector_index::{
                FragmentedVectorSegment,
                VectorIndexSnapshot,
                VectorIndexState,
            },
            IndexConfig,
            IndexMetadata,
        },
        knobs::{
            MULTI_SEGMENT_FULL_SCAN_THRESHOLD_KB,
            VECTOR_INDEX_SIZE_SOFT_LIMIT,
        },
        persistence::PersistenceReader,
        runtime::Runtime,
        types::{
            IndexId,
            IndexName,
        },
    };
    use keybroker::Identity;
    use maplit::{
        btreemap,
        btreeset,
    };
    use must_let::must_let;
    use qdrant_segment::vector_storage::VectorStorage;
    use runtime::testing::TestRuntime;
    use storage::LocalDirStorage;
    use value::{
        assert_obj,
        assert_val,
        ConvexValue,
        GenericDocumentId,
        TabletIdAndTableNumber,
    };
    use vector::{
        PublicVectorSearchQueryResult,
        QdrantExternalId,
        VectorSearch,
    };

    use super::{
        new_vector_flusher_for_tests,
        VectorIndexFlusher,
    };
    use crate::{
        bootstrap_model::index_workers::IndexWorkerMetadataModel,
        test_helpers::DbFixtures,
        tests::vector_test_utils::{
            add_document_vec,
            add_document_with_value,
            backfilling_vector_index_with_doc,
            IndexData,
            VectorFixtures,
        },
        vector_index_worker::compactor::CompactionConfig,
        Database,
        IndexModel,
        SystemMetadataModel,
        UserFacingModel,
    };

    fn new_vector_flusher_with_soft_limit(
        rt: &TestRuntime,
        database: &Database<TestRuntime>,
        reader: Arc<dyn PersistenceReader>,
        soft_limit: usize,
    ) -> anyhow::Result<VectorIndexFlusher<TestRuntime>> {
        let storage = LocalDirStorage::new(rt.clone())?;
        Ok(new_vector_flusher_for_tests(
            rt.clone(),
            database.clone(),
            reader,
            Arc::new(storage),
            soft_limit,
            *MULTI_SEGMENT_FULL_SCAN_THRESHOLD_KB,
            *VECTOR_INDEX_SIZE_SOFT_LIMIT,
            None,
        ))
    }

    fn new_vector_flusher(
        rt: &TestRuntime,
        database: &Database<TestRuntime>,
        reader: Arc<dyn PersistenceReader>,
    ) -> anyhow::Result<VectorIndexFlusher<TestRuntime>> {
        new_vector_flusher_with_soft_limit(rt, database, reader, 1000)
    }

    #[convex_macro::test_runtime]
    async fn worker_does_not_crash_on_documents_with_invalid_vector_dimensions(
        rt: TestRuntime,
    ) -> anyhow::Result<()> {
        let DbFixtures { tp, db, .. } = DbFixtures::new(&rt).await?;

        let IndexData { index_name, .. } = backfilling_vector_index_with_doc(&db).await?;

        let mut tx = db.begin_system().await?;
        let vec = [1f64].into_iter().map(ConvexValue::Float64).collect();
        add_document_vec(&mut tx, index_name.table(), vec).await?;
        db.commit(tx).await?;

        let mut worker = new_vector_flusher(&rt, &db, tp.reader())?;
        worker.step().await?;

        Ok(())
    }

    #[convex_macro::test_runtime]
    async fn worker_does_not_crash_on_documents_with_non_vector(
        rt: TestRuntime,
    ) -> anyhow::Result<()> {
        let DbFixtures { tp, db, .. } = DbFixtures::new(&rt).await?;

        let IndexData {
            index_name,
            resolved_index_name,
            ..
        } = backfilling_vector_index_with_doc(&db).await?;

        let mut tx = db.begin_system().await?;
        add_document_with_value(
            &mut tx,
            index_name.table(),
            ConvexValue::String(value::ConvexString::try_from("test")?),
        )
        .await?;
        db.commit(tx).await?;

        // Use 0 soft limit so that we always reindex documents
        let mut worker = new_vector_flusher_with_soft_limit(&rt, &db, tp.reader(), 0)?;
        let (metrics, _) = worker.step().await?;
        // Make sure we advance past the invalid document.
        assert_eq!(metrics, btreemap! {resolved_index_name.clone() => 1});

        Ok(())
    }

    #[convex_macro::test_runtime]
    async fn worker_with_empty_index_does_not_create_empty_segment(
        rt: TestRuntime,
    ) -> anyhow::Result<()> {
        let fixtures = VectorFixtures::new(rt.clone()).await?;

        let IndexData {
            index_name,
            resolved_index_name,
            ..
        } = fixtures.backfilling_vector_index().await?;

        let mut worker = fixtures.new_index_flusher()?;
        let (metrics, _) = worker.step().await?;
        assert_eq!(metrics, btreemap! {resolved_index_name.clone() => 0});

        let segments = fixtures.get_segments_metadata(index_name).await?;
        assert!(segments.is_empty());

        Ok(())
    }

    #[convex_macro::test_runtime]
    async fn worker_at_or_over_scan_threshold_uses_hnsw(rt: TestRuntime) -> anyhow::Result<()> {
        let fixtures = VectorFixtures::new(rt.clone()).await?;

        let IndexData { index_name, .. } = fixtures.backfilling_vector_index().await?;
        fixtures
            .add_document_vec_array(index_name.table(), [3f64, 4f64])
            .await?;
        let mut worker = fixtures.new_index_flusher_with_full_scan_threshold(0)?;
        worker.step().await?;

        let segments = fixtures.get_segments_metadata(index_name).await?;
        assert_eq!(segments.len(), 1);
        let segment = segments.first().unwrap();

        let segment = fixtures.load_segment(segment).await?;
        assert!(segment.segment_config.is_any_vector_indexed());

        Ok(())
    }

    #[convex_macro::test_runtime]
    async fn incremental_backfill_exceed_part_threshold_builds_multiple_parts(
        rt: TestRuntime,
    ) -> anyhow::Result<()> {
        let fixtures = VectorFixtures::new(rt.clone()).await?;

        let IndexData { index_name, .. } = fixtures.backfilling_vector_index().await?;
        fixtures
            .add_document_vec_array(index_name.table(), [3f64, 4f64])
            .await?;
        fixtures
            .add_document_vec_array(index_name.table(), [5f64, 6f64])
            .await?;
        let mut worker = fixtures.new_index_flusher_with_incremental_part_threshold(8)?;

        // Should be in backfilling state after step
        worker.step().await?;
        let segments = fixtures
            .get_segments_from_backfilling_index(index_name.clone())
            .await?;
        assert_eq!(segments.len(), 1);
        let segment = segments.first().unwrap();
        let segment = fixtures.load_segment(segment).await?;
        assert_eq!(segment.total_point_count(), 1);

        // Should be no longer in backfilling state now after step
        worker.step().await?;
        let segments = fixtures.get_segments_metadata(index_name).await?;
        assert_eq!(segments.len(), 2);
        let segment = segments.get(1).unwrap();
        let segment = fixtures.load_segment(segment).await?;
        assert_eq!(segment.total_point_count(), 1);

        Ok(())
    }

    #[convex_macro::test_runtime]
    async fn worker_under_full_scan_threshold_does_not_use_hnsw(
        rt: TestRuntime,
    ) -> anyhow::Result<()> {
        let fixtures = VectorFixtures::new(rt.clone()).await?;

        let IndexData { index_name, .. } = fixtures.backfilling_vector_index().await?;
        fixtures
            .add_document_vec_array(index_name.table(), [3f64, 4f64])
            .await?;
        let mut worker = fixtures.new_index_flusher_with_full_scan_threshold(1000000)?;
        worker.step().await?;

        let segments = fixtures.get_segments_metadata(index_name).await?;
        assert_eq!(segments.len(), 1);
        let segment = segments.first().unwrap();

        let segment = fixtures.load_segment(segment).await?;
        assert!(!segment.segment_config.is_any_vector_indexed());

        Ok(())
    }

    #[convex_macro::test_runtime]
    async fn worker_with_deleted_vector_does_not_include_deleted_vector_in_segment(
        rt: TestRuntime,
    ) -> anyhow::Result<()> {
        let fixtures = VectorFixtures::new(rt.clone()).await?;

        let IndexData {
            index_name,
            resolved_index_name,
            ..
        } = fixtures.backfilling_vector_index().await?;
        fixtures
            .add_document_vec_array(index_name.table(), [3f64, 4f64])
            .await?;
        let to_delete = fixtures
            .add_document_vec_array(index_name.table(), [4f64, 5f64])
            .await?;

        let mut tx = fixtures.db.begin_system().await?;
        UserFacingModel::new_root_for_test(&mut tx)
            .delete(to_delete.into())
            .await?;
        fixtures.db.commit(tx).await?;

        let mut worker = fixtures.new_index_flusher()?;
        let (metrics, _) = worker.step().await?;
        assert_eq!(metrics, btreemap! {resolved_index_name.clone() => 1});

        let segments = fixtures.get_segments_metadata(index_name).await?;
        assert_eq!(segments.len(), 1);
        let segment = segments.first().unwrap();
        assert_eq!(1, segment.num_vectors);
        assert_eq!(0, segment.num_deleted);

        let segment = fixtures.load_segment(segment).await?;
        assert_eq!(segment.id_tracker.borrow().deleted_point_count(), 0);
        let count = segment
            .vector_data
            .get("default_vector")
            .unwrap()
            .vector_storage
            .borrow()
            .total_vector_count();
        assert_eq!(1, count);

        Ok(())
    }

    #[convex_macro::test_runtime]
    async fn worker_with_segment_no_new_documents_doesnt_append_empty_segment(
        rt: TestRuntime,
    ) -> anyhow::Result<()> {
        let fixtures = VectorFixtures::new(rt.clone()).await?;

        let IndexData {
            index_name,
            resolved_index_name,
            ..
        } = fixtures.backfilling_vector_index().await?;
        fixtures
            .add_document_vec_array(index_name.table(), [3f64, 4f64])
            .await?;
        let mut worker = fixtures.new_index_flusher()?;
        let (metrics, _) = worker.step().await?;
        assert_eq!(metrics, btreemap! {resolved_index_name.clone() => 1});

        let mut worker = fixtures.new_index_flusher()?;
        worker.step().await?;

        let segments = fixtures.get_segments_metadata(index_name).await?;
        assert_eq!(1, segments.len());

        Ok(())
    }

    #[convex_macro::test_runtime]
    async fn backfilled_concurrent_compaction_and_flush(rt: TestRuntime) -> anyhow::Result<()> {
        let config = CompactionConfig::default();
        let min_compaction_segments = config.min_compaction_segments;
        let config = CompactionConfig {
            // Treat everything as a large segment
            small_segment_threshold_bytes: 0,
            ..config
        };
        let fixtures = VectorFixtures::new_with_config(rt.clone(), config).await?;
        // Mark the index backfilled.
        let index_data = fixtures.backfilling_vector_index().await?;
        fixtures.backfill().await?;

        let IndexData { index_name, .. } = index_data;

        // Create enough segments to trigger compaction.
        let mut deleted_doc_ids = vec![];
        for _ in 0..min_compaction_segments {
            deleted_doc_ids.push(
                fixtures
                    .add_document_vec_array(index_name.table(), [3f64, 4f64])
                    .await?,
            );
            fixtures.backfill().await?;
        }
        // Queue up deletes for all existing segments, and one new vector that will
        // cause the flusher to write a new segment.
        let mut tx = fixtures.db.begin_system().await?;
        for doc_id in &deleted_doc_ids {
            UserFacingModel::new_root_for_test(&mut tx)
                .delete((*doc_id).into())
                .await?;
        }
        fixtures.db.commit(tx).await?;
        let non_deleted_id = fixtures
            .add_document_vec_array(index_name.table(), [3f64, 4f64])
            .await?;

        // Run the compactor / flusher concurrently in a way where the compactor
        // wins the race.
        fixtures.run_compaction_during_flush().await?;

        // Verify we propagate the new deletes to the compacted segment and retain our
        // new segment.
        let segments = fixtures.get_segments_metadata(index_name).await?;
        assert_eq!(2, segments.len());

        let (compacted_segment, new_segment): (Vec<_>, Vec<_>) = segments
            .into_iter()
            .partition(|segment| segment.num_deleted > 0);
        assert_eq!(compacted_segment.len(), 1);

        let compacted_segment = compacted_segment.first().unwrap();
        verify_segment_state(&fixtures, compacted_segment, deleted_doc_ids, vec![]).await?;

        assert_eq!(new_segment.len(), 1);
        let new_segment = new_segment.first().unwrap();
        verify_segment_state(&fixtures, new_segment, vec![], vec![non_deleted_id]).await?;

        Ok(())
    }

    #[convex_macro::test_runtime]
    async fn incremental_index_backfill_concurrent_compaction_and_flush(
        rt: TestRuntime,
    ) -> anyhow::Result<()> {
        let config = CompactionConfig::default();
        let min_compaction_segments = config.min_compaction_segments;
        let config = CompactionConfig {
            // Treat everything as a large segment
            small_segment_threshold_bytes: 0,
            ..config
        };
        let fixtures = VectorFixtures::new_with_config(rt.clone(), config).await?;

        // Add 4 vectors and set part threshold to build 4 segments
        let IndexData { index_name, .. } = fixtures.backfilling_vector_index().await?;
        for i in 0..(min_compaction_segments + 1) {
            fixtures
                .add_document_vec_array(index_name.table(), [i as f64, (i + 1) as f64])
                .await?;
        }
        // Do every backfill flush step until last one
        let mut worker = fixtures.new_index_flusher_with_incremental_part_threshold(8)?;
        for _ in 0..min_compaction_segments {
            worker.step().await?;
        }

        // For last iteration, run the compactor / flusher concurrently in a way where
        // the compactor wins the race.
        fixtures.run_compaction_during_flush().await?;

        // There should be 2 segments left: the compacted segment and the new segment
        // from flush
        worker.step().await?;
        let segments = fixtures.get_segments_metadata(index_name).await?;
        assert_eq!(segments.len(), 2);

        let compacted_segment = fixtures.load_segment(segments.first().unwrap()).await?;
        assert_eq!(compacted_segment.total_point_count(), 3);
        let compacted_segment = fixtures.load_segment(segments.get(1).unwrap()).await?;
        assert_eq!(compacted_segment.total_point_count(), 1);

        Ok(())
    }

    #[convex_macro::test_runtime]
    async fn concurrent_compaction_and_flush_new_segment_propagates_deletes(
        rt: TestRuntime,
    ) -> anyhow::Result<()> {
        let config = CompactionConfig::default();
        let min_compaction_segments = config.min_compaction_segments;
        let config = CompactionConfig {
            // Treat everything as a large segment
            small_segment_threshold_bytes: 0,
            ..config
        };
        let fixtures = VectorFixtures::new_with_config(rt.clone(), config).await?;
        let index_data = fixtures.enabled_vector_index().await?;

        let IndexData { index_name, .. } = index_data;

        // Create enough segments to trigger compaction.
        let mut deleted_doc_ids = vec![];
        for _ in 0..min_compaction_segments {
            deleted_doc_ids.push(
                fixtures
                    .add_document_vec_array(index_name.table(), [3f64, 4f64])
                    .await?,
            );
            fixtures.backfill().await?;
        }
        // Queue up deletes for all existing segments, and one new vector that will
        // cause the flusher to write a new segment.
        let mut tx = fixtures.db.begin_system().await?;
        for doc_id in &deleted_doc_ids {
            UserFacingModel::new_root_for_test(&mut tx)
                .delete((*doc_id).into())
                .await?;
        }
        fixtures.db.commit(tx).await?;
        let non_deleted_id = fixtures
            .add_document_vec_array(index_name.table(), [3f64, 4f64])
            .await?;

        // Run the compactor / flusher concurrently in a way where the compactor
        // wins the race.
        fixtures.run_compaction_during_flush().await?;

        // Verify we propagate the new deletes to the compacted segment and retain our
        // new segment.
        let segments = fixtures.get_segments_metadata(index_name).await?;
        assert_eq!(2, segments.len());

        let (compacted_segment, new_segment): (Vec<_>, Vec<_>) = segments
            .into_iter()
            .partition(|segment| segment.num_deleted > 0);
        assert_eq!(compacted_segment.len(), 1);

        let compacted_segment = compacted_segment.first().unwrap();
        verify_segment_state(&fixtures, compacted_segment, deleted_doc_ids, vec![]).await?;

        assert_eq!(new_segment.len(), 1);
        let new_segment = new_segment.first().unwrap();
        verify_segment_state(&fixtures, new_segment, vec![], vec![non_deleted_id]).await?;

        Ok(())
    }

    #[convex_macro::test_runtime]
    async fn concurrent_compaction_and_flush_no_new_segment_propagates_updates_and_deletes(
        rt: TestRuntime,
    ) -> anyhow::Result<()> {
        let config = CompactionConfig::default();
        let min_compaction_segments = config.min_compaction_segments;
        let config = CompactionConfig {
            // Treat everything as a large segment
            small_segment_threshold_bytes: 0,
            ..config
        };
        let fixtures = VectorFixtures::new_with_config(rt.clone(), config).await?;
        let index_data = fixtures.enabled_vector_index().await?;

        let IndexData { index_name, .. } = index_data;

        // Create enough segments to trigger compaction.
        let mut deleted_doc_ids = vec![];
        for _ in 0..min_compaction_segments {
            deleted_doc_ids.push(
                fixtures
                    .add_document_vec_array(index_name.table(), [3f64, 4f64])
                    .await?,
            );
            fixtures.backfill().await?;
        }
        // Queue up updates and deletes for all existing segments and no new vectors so
        // that compaction will just delete all the existing segments without adding a
        // new one.
        let mut tx = fixtures.db.begin_system().await?;
        let patched_object = assert_val!([5f64, 6f64]);
        for doc_id in &deleted_doc_ids {
            UserFacingModel::new_root_for_test(&mut tx)
                .patch(
                    (*doc_id).into(),
                    assert_obj!("vector" => patched_object.clone()).into(),
                )
                .await?;
        }
        fixtures.db.commit(tx).await?;

        let mut tx = fixtures.db.begin_system().await?;
        for doc_id in &deleted_doc_ids {
            UserFacingModel::new_root_for_test(&mut tx)
                .delete((*doc_id).into())
                .await?;
        }
        fixtures.db.commit(tx).await?;

        fixtures.run_compaction_during_flush().await?;

        let segments = fixtures.get_segments_metadata(index_name).await?;
        assert_eq!(1, segments.len());
        let segment = segments.first().unwrap();

        verify_segment_state(&fixtures, segment, deleted_doc_ids, vec![]).await?;

        Ok(())
    }

    #[convex_macro::test_runtime]
    async fn backfilling_and_enabled_version_of_index_writes_to_backfilling(
        rt: TestRuntime,
    ) -> anyhow::Result<()> {
        let fixtures = VectorFixtures::new(rt.clone()).await?;

        fixtures.enabled_vector_index().await?;

        let backfilling_data = fixtures.backfilling_vector_index().await?;
        let IndexData {
            index_id: backfilling_index_id,
            ..
        } = backfilling_data;

        fixtures.new_index_flusher()?.step().await?;

        let mut tx = fixtures.db.begin_system().await?;
        let new_metadata = IndexModel::new(&mut tx)
            .require_index_by_id(backfilling_index_id)
            .await?
            .into_value();
        must_let!(let IndexMetadata {
            config: IndexConfig::Vector {
                on_disk_state: VectorIndexState::Backfilled(VectorIndexSnapshot { .. }),
                ..
            },
            ..
        } = new_metadata);

        Ok(())
    }

    async fn verify_segment_state(
        fixtures: &VectorFixtures,
        segment: &FragmentedVectorSegment,
        expected_deletes: Vec<GenericDocumentId<TabletIdAndTableNumber>>,
        expected_non_deleted: Vec<GenericDocumentId<TabletIdAndTableNumber>>,
    ) -> anyhow::Result<()> {
        assert_eq!(segment.num_deleted as usize, expected_deletes.len());
        assert_eq!(
            segment.num_vectors as usize,
            expected_non_deleted.len() + expected_deletes.len()
        );

        let segment = fixtures.load_segment(segment).await?;

        for doc_id in expected_deletes {
            let external_id = QdrantExternalId::try_from(&doc_id)?;
            let internal_id = segment
                .id_tracker
                .borrow()
                .internal_id(*external_id)
                .unwrap();
            assert!(segment.id_tracker.borrow().is_deleted_point(internal_id));
        }
        for doc_id in expected_non_deleted {
            let external_id = QdrantExternalId::try_from(&doc_id)?;
            let internal_id = segment
                .id_tracker
                .borrow()
                .internal_id(*external_id)
                .unwrap();
            assert!(!segment.id_tracker.borrow().is_deleted_point(internal_id));
        }
        Ok(())
    }

    #[convex_macro::test_runtime]
    async fn multi_segment_worker_with_segment_document_added_then_removed_no_empty_segment(
        rt: TestRuntime,
    ) -> anyhow::Result<()> {
        let fixtures = VectorFixtures::new(rt.clone()).await?;

        let IndexData {
            index_name,
            resolved_index_name,
            ..
        } = fixtures.backfilling_vector_index().await?;
        fixtures
            .add_document_vec_array(index_name.table(), [3f64, 4f64])
            .await?;
        let mut worker = fixtures.new_index_flusher()?;
        let (metrics, _) = worker.step().await?;
        assert_eq!(metrics, btreemap! {resolved_index_name.clone() => 1});

        let id = fixtures
            .add_document_vec_array(index_name.table(), [3f64, 4f64])
            .await?;
        let mut tx = fixtures.db.begin_system().await?;
        UserFacingModel::new_root_for_test(&mut tx)
            .delete(id.into())
            .await?;
        fixtures.db.commit(tx).await?;

        let mut worker = fixtures.new_index_flusher()?;
        let (metrics, _) = worker.step().await?;
        assert_eq!(metrics, btreemap! { resolved_index_name => 0 });

        let segments = fixtures.get_segments_metadata(index_name).await?;
        assert_eq!(1, segments.len());

        Ok(())
    }

    #[convex_macro::test_runtime]
    async fn multi_segment_worker_with_segment_updated_then_deleted_after_written_succeeds(
        rt: TestRuntime,
    ) -> anyhow::Result<()> {
        let fixtures = VectorFixtures::new(rt.clone()).await?;

        let IndexData {
            index_name,
            resolved_index_name,
            ..
        } = fixtures.backfilling_vector_index().await?;
        // Add the document to a segment
        let id = fixtures
            .add_document_vec_array(index_name.table(), [3f64, 4f64])
            .await?;
        let mut worker = fixtures.new_index_flusher()?;
        let (metrics, _) = worker.step().await?;
        assert_eq!(metrics, btreemap! {resolved_index_name.clone() => 1});

        // Update the document in place
        let mut tx = fixtures.db.begin_system().await?;
        let patched_object = assert_val!([5f64, 6f64]);
        UserFacingModel::new_root_for_test(&mut tx)
            .patch(id.into(), assert_obj!("vector" => patched_object).into())
            .await?;
        fixtures.db.commit(tx).await?;

        // Then delete it
        let mut tx = fixtures.db.begin_system().await?;
        UserFacingModel::new_root_for_test(&mut tx)
            .delete(id.into())
            .await?;
        fixtures.db.commit(tx).await?;

        // And flush to ensure that we handle the document showing up repeatedly in the
        // document log for the old instance.
        let mut worker = fixtures.new_index_flusher()?;
        let (metrics, _) = worker.step().await?;
        assert_eq!(metrics, btreemap! { resolved_index_name => 0 });

        let mut segments = fixtures.get_segments_metadata(index_name).await?;
        assert_eq!(1, segments.len());
        let segment = segments.remove(0);
        assert_eq!(segment.num_deleted, 1);
        assert_eq!(segment.num_vectors, 1);

        Ok(())
    }

    // When we're backfilling we read the contents of the table at some snapshot, so
    // we never read deleted documents and our segment should contain exactly
    // the set of vectors in the table at that snapshot timestamp.
    #[convex_macro::test_runtime]
    async fn flush_during_backfill_with_adds_and_deletes_includes_only_non_deleted_in_counts(
        rt: TestRuntime,
    ) -> anyhow::Result<()> {
        let fixtures = VectorFixtures::new(rt.clone()).await?;

        let IndexData {
            index_name,
            resolved_index_name,
            ..
        } = fixtures.backfilling_vector_index().await?;
        for index in 0..10 {
            let id = fixtures
                .add_document_vec_array(index_name.table(), [3f64, 4f64])
                .await?;
            if index >= 5 {
                let mut tx = fixtures.db.begin_system().await?;
                UserFacingModel::new_root_for_test(&mut tx)
                    .delete(id.into())
                    .await?;
                fixtures.db.commit(tx).await?;
            }
        }
        let mut worker = fixtures.new_index_flusher()?;
        let (metrics, _) = worker.step().await?;
        assert_eq!(metrics, btreemap! {resolved_index_name.clone() => 5});

        let segments = fixtures.get_segments_metadata(index_name).await?;
        assert_eq!(1, segments.len());
        let segment = segments.first().unwrap();
        assert_eq!(5, segment.num_vectors);
        assert_eq!(0, segment.num_deleted);

        Ok(())
    }

    // After backfilling finishes, we read incrementally from the document log. So
    // it's perfectly possible that we'll read a document and a subsequent
    // modification or delete all while writing a single segment. As a result we
    // expect to write some number of deleted vectors.
    #[convex_macro::test_runtime]
    async fn flush_after_backfill_with_adds_and_deletes_includes_deleted_vectors_in_counts(
        rt: TestRuntime,
    ) -> anyhow::Result<()> {
        let fixtures = VectorFixtures::new(rt.clone()).await?;

        let IndexData {
            index_name,
            resolved_index_name,
            ..
        } = fixtures.backfilling_vector_index().await?;
        let mut worker = fixtures.new_index_flusher()?;
        let (metrics, _) = worker.step().await?;
        assert_eq!(metrics, btreemap! {resolved_index_name.clone() => 0});

        for index in 0..10 {
            let id = fixtures
                .add_document_vec_array(index_name.table(), [3f64, 4f64])
                .await?;
            if index >= 5 {
                let mut tx = fixtures.db.begin_system().await?;
                UserFacingModel::new_root_for_test(&mut tx)
                    .delete(id.into())
                    .await?;
                fixtures.db.commit(tx).await?;
            }
        }
        let mut worker = fixtures.new_index_flusher()?;
        let (metrics, _) = worker.step().await?;
        assert_eq!(metrics, btreemap! {resolved_index_name.clone() => 10});

        let segments = fixtures.get_segments_metadata(index_name).await?;
        assert_eq!(1, segments.len());
        let segment = segments.first().unwrap();
        assert_eq!(10, segment.num_vectors);
        assert_eq!(5, segment.num_deleted);

        Ok(())
    }

    #[convex_macro::test_runtime]
    async fn flush_after_backfill_with_adds_and_updates_includes_updated_vectors_in_count(
        rt: TestRuntime,
    ) -> anyhow::Result<()> {
        let fixtures = VectorFixtures::new(rt.clone()).await?;

        let IndexData {
            index_name,
            resolved_index_name,
            ..
        } = fixtures.backfilling_vector_index().await?;
        let mut worker = fixtures.new_index_flusher()?;
        let (metrics, _) = worker.step().await?;
        assert_eq!(metrics, btreemap! {resolved_index_name.clone() => 0});

        for index in 0..10 {
            let id = fixtures
                .add_document_vec_array(index_name.table(), [3f64, 4f64])
                .await?;
            if index >= 5 {
                let mut tx = fixtures.db.begin_system().await?;
                let patched_object = assert_val!([5f64, 6f64]);
                UserFacingModel::new_root_for_test(&mut tx)
                    .patch(
                        id.into(),
                        assert_obj!("vector" => patched_object.clone()).into(),
                    )
                    .await?;
                fixtures.db.commit(tx).await?;
            }
        }
        let mut worker = fixtures.new_index_flusher()?;
        let (metrics, _) = worker.step().await?;
        assert_eq!(metrics, btreemap! {resolved_index_name.clone() => 10});

        let segments = fixtures.get_segments_metadata(index_name).await?;
        assert_eq!(1, segments.len());
        let segment = segments.first().unwrap();
        assert_eq!(10, segment.num_vectors);
        assert_eq!(0, segment.num_deleted);

        Ok(())
    }

    #[convex_macro::test_runtime]
    async fn multi_segment_worker_builds_indexes_incrementally(
        rt: TestRuntime,
    ) -> anyhow::Result<()> {
        let fixtures = VectorFixtures::new(rt.clone()).await?;

        let IndexData {
            index_name,
            resolved_index_name,
            ..
        } = fixtures.enabled_vector_index().await?;

        for vector in [[3f64, 4f64], [5f64, 6f64], [6f64, 7f64]] {
            let id = fixtures
                .add_document_vec_array(index_name.table(), vector)
                .await?;
            let mut worker = fixtures.new_index_flusher_with_full_scan_threshold(0)?;
            let (metrics, _) = worker.step().await?;
            assert_eq!(metrics, btreemap! {resolved_index_name.clone() => 1});

            let results = vector_search(&fixtures.db, index_name.clone(), vector).await?;
            assert_eq!(results.first().unwrap().id.internal_id(), id.internal_id());
        }

        Ok(())
    }

    async fn set_fast_forward_time_to_now<RT: Runtime>(
        db: &Database<RT>,
        index_id: IndexId,
    ) -> anyhow::Result<()> {
        let mut tx = db.begin_system().await?;
        let metadata = IndexWorkerMetadataModel::new(&mut tx)
            .get_or_create_vector_search(index_id)
            .await?;
        let (worker_meta_doc_id, mut metadata) = metadata.into_id_and_value();
        *metadata.index_metadata.mut_fast_forward_ts() = *tx.begin_timestamp();
        SystemMetadataModel::new_global(&mut tx)
            .replace(worker_meta_doc_id, metadata.try_into()?)
            .await?;
        db.commit(tx).await?;
        Ok(())
    }

    #[convex_macro::test_runtime]
    async fn multi_segment_with_newer_fast_forward_time_builds_from_fast_forward_time(
        rt: TestRuntime,
    ) -> anyhow::Result<()> {
        let fixtures = VectorFixtures::new(rt.clone()).await?;

        let IndexData {
            index_name,
            index_id,
            resolved_index_name,
            ..
        } = fixtures.enabled_vector_index().await?;

        let vector = [8f64, 9f64];
        fixtures
            .add_document_vec_array(index_name.table(), vector)
            .await?;

        set_fast_forward_time_to_now(&fixtures.db, index_id.internal_id()).await?;

        let mut worker = fixtures.new_index_flusher_with_full_scan_threshold(0)?;
        let (metrics, _) = worker.step().await?;
        assert_eq!(metrics, btreemap! {resolved_index_name.clone() => 0});

        let results = vector_search(&fixtures.db, index_name, vector).await?;

        assert!(results.is_empty());

        Ok(())
    }

    #[convex_macro::test_runtime]
    async fn multi_segment_with_older_fast_forward_time_builds_from_index_time(
        rt: TestRuntime,
    ) -> anyhow::Result<()> {
        let fixtures = VectorFixtures::new(rt.clone()).await?;

        let IndexData {
            index_name,
            index_id,
            resolved_index_name,
            ..
        } = fixtures.enabled_vector_index().await?;

        set_fast_forward_time_to_now(&fixtures.db, index_id.internal_id()).await?;

        let vector = [8f64, 9f64];
        let vector_doc_id = fixtures
            .add_document_vec_array(index_name.table(), vector)
            .await?;

        let mut worker = fixtures.new_index_flusher_with_full_scan_threshold(0)?;
        let (metrics, _) = worker.step().await?;
        assert_eq!(metrics, btreemap! {resolved_index_name.clone() => 1});

        let results = vector_search(&fixtures.db, index_name, vector).await?;

        assert_eq!(
            results.first().unwrap().id.internal_id(),
            vector_doc_id.internal_id()
        );

        Ok(())
    }

    async fn vector_search<RT: Runtime>(
        db: &Database<RT>,
        index_name: IndexName,
        vector: [f64; 2],
    ) -> anyhow::Result<Vec<PublicVectorSearchQueryResult>> {
        Ok(db
            .vector_search(
                Identity::system(),
                VectorSearch {
                    index_name,
                    vector: vector.into_iter().map(|value| value as f32).collect(),
                    limit: Some(1),
                    expressions: btreeset![],
                },
            )
            .await?
            .0)
    }
}

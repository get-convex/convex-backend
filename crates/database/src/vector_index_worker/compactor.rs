use std::sync::Arc;

use common::runtime::Runtime;
use search::searcher::Searcher;
use storage::Storage;

use crate::{
    index_workers::{
        search_compactor::{
            CompactionConfig,
            SearchIndexCompactor,
        },
        writer::SearchIndexMetadataWriter,
    },
    vector_index_worker::vector_meta::VectorSearchIndex,
    Database,
};

pub type VectorIndexCompactor<RT> = SearchIndexCompactor<RT, VectorSearchIndex>;

pub(crate) fn new_vector_compactor<RT: Runtime>(
    database: Database<RT>,
    searcher: Arc<dyn Searcher>,
    search_storage: Arc<dyn Storage>,
    config: CompactionConfig,
    writer: SearchIndexMetadataWriter<RT, VectorSearchIndex>,
) -> VectorIndexCompactor<RT> {
    VectorIndexCompactor::new(database, searcher, search_storage, config, writer)
}

#[cfg(any(test, feature = "testing"))]
pub(crate) fn new_vector_compactor_for_tests<RT: Runtime>(
    runtime: RT,
    database: Database<RT>,
    search_storage: Arc<dyn Storage>,
    searcher: Arc<dyn Searcher>,
    config: CompactionConfig,
) -> VectorIndexCompactor<RT> {
    let writer = SearchIndexMetadataWriter::new(runtime, database.clone(), search_storage.clone());
    SearchIndexCompactor::new(database, searcher, search_storage.clone(), config, writer)
}

#[cfg(any(test, feature = "testing"))]
pub async fn compact_vector_indexes_in_test<RT: Runtime>(
    runtime: RT,
    database: Database<RT>,
    search_storage: Arc<dyn Storage>,
    searcher: Arc<dyn Searcher>,
) -> anyhow::Result<()> {
    let compactor = new_vector_compactor_for_tests(
        runtime,
        database,
        search_storage,
        searcher,
        CompactionConfig::default(),
    );
    compactor.step().await?;
    Ok(())
}

#[cfg(test)]
mod tests {

    use itertools::Itertools;
    use keybroker::Identity;
    use maplit::{
        btreemap,
        btreeset,
    };
    use runtime::testing::TestRuntime;
    use vector::VectorSearch;

    use crate::{
        tests::vector_test_utils::{
            VectorFixtures,
            VECTOR_SIZE_BYTES,
        },
        vector_index_worker::compactor::CompactionConfig,
        UserFacingModel,
    };

    #[convex_macro::test_runtime]
    async fn compact_with_empty_backfilling_index_does_nothing(
        rt: TestRuntime,
    ) -> anyhow::Result<()> {
        let fixtures = VectorFixtures::new(rt.clone()).await?;
        fixtures.backfilling_vector_index_with_doc().await?;

        let compactor = fixtures.new_compactor().await?;
        let (metrics, _) = compactor.step().await?;
        assert!(metrics.is_empty());
        Ok(())
    }

    #[convex_macro::test_runtime]
    async fn compact_with_single_too_small_backfilled_index_does_nothing(
        rt: TestRuntime,
    ) -> anyhow::Result<()> {
        let fixtures = VectorFixtures::new(rt.clone()).await?;
        fixtures.backfilled_vector_index_with_doc().await?;

        let compactor = fixtures.new_compactor().await?;
        let (metrics, _) = compactor.step().await?;
        assert!(metrics.is_empty());
        Ok(())
    }

    #[convex_macro::test_runtime]
    async fn compact_with_single_too_small_enabled_index_does_nothing(
        rt: TestRuntime,
    ) -> anyhow::Result<()> {
        let fixtures = VectorFixtures::new(rt.clone()).await?;
        let index_data = fixtures.enabled_vector_index().await?;
        fixtures
            .add_document_vec_array(index_data.index_name.table(), [3f64, 4f64])
            .await?;
        fixtures.backfill().await?;

        let compactor = fixtures.new_compactor().await?;
        let (metrics, _) = compactor.step().await?;
        assert!(metrics.is_empty());
        Ok(())
    }

    #[convex_macro::test_runtime]
    async fn compact_with_multiple_small_segments_merges_them(
        rt: TestRuntime,
    ) -> anyhow::Result<()> {
        let fixtures = VectorFixtures::new(rt.clone()).await?;
        let index_data = fixtures.enabled_vector_index().await?;
        let min_compaction_segments = CompactionConfig::default().min_compaction_segments;

        for _ in 0..min_compaction_segments {
            fixtures
                .add_document_vec_array(index_data.index_name.table(), [3f64, 4f64])
                .await?;
            fixtures.backfill().await?;
        }

        let compactor = fixtures.new_compactor().await?;
        let (metrics, _) = compactor.step().await?;
        assert_eq!(
            metrics,
            btreemap! { index_data.resolved_index_name => min_compaction_segments}
        );

        let segments = fixtures
            .get_segments_metadata(index_data.index_name)
            .await?;
        assert_eq!(segments.len(), 1);

        Ok(())
    }

    #[convex_macro::test_runtime]
    async fn compact_with_enabled_index_multiple_large_segments_compacts_them(
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

        for _ in 0..min_compaction_segments {
            fixtures
                .add_document_vec_array(index_data.index_name.table(), [3f64, 4f64])
                .await?;
            fixtures.backfill().await?;
        }

        let compactor = fixtures.new_compactor().await?;
        let (metrics, _) = compactor.step().await?;
        assert_eq!(
            metrics,
            btreemap! { index_data.resolved_index_name => min_compaction_segments }
        );

        let segments = fixtures
            .get_segments_metadata(index_data.index_name)
            .await?;
        assert_eq!(segments.len(), 1);

        Ok(())
    }

    #[convex_macro::test_runtime]
    async fn compact_with_backfilled_index_multiple_segments_compacts_them(
        rt: TestRuntime,
    ) -> anyhow::Result<()> {
        let fixtures =
            VectorFixtures::new_with_config(rt.clone(), CompactionConfig::default()).await?;
        let min_compaction_segments = CompactionConfig::default().min_compaction_segments;
        let index_data = fixtures.backfilled_vector_index().await?;

        for _ in 0..min_compaction_segments {
            fixtures
                .add_document_vec_array(index_data.index_name.table(), [3f64, 4f64])
                .await?;
            fixtures.backfill().await?;
        }

        let compactor = fixtures.new_compactor().await?;
        let (metrics, _) = compactor.step().await?;
        assert_eq!(
            metrics,
            btreemap! { index_data.resolved_index_name => min_compaction_segments }
        );

        let segments = fixtures
            .get_segments_metadata(index_data.index_name)
            .await?;
        assert_eq!(segments.len(), 1);

        Ok(())
    }

    #[convex_macro::test_runtime]
    async fn compact_with_multiple_large_segments_over_size_threshold_does_not_compact_them(
        rt: TestRuntime,
    ) -> anyhow::Result<()> {
        let config = CompactionConfig::default();
        let config = CompactionConfig {
            // Treat everything as a large segment
            small_segment_threshold_bytes: 0,
            // Treat everything as being over the size threshold.
            max_segment_size_bytes: 0,
            ..config
        };
        let fixtures = VectorFixtures::new_with_config(rt.clone(), config).await?;
        let index_data = fixtures.enabled_vector_index().await?;

        for _ in 0..CompactionConfig::default().min_compaction_segments {
            fixtures
                .add_document_vec_array(index_data.index_name.table(), [3f64, 4f64])
                .await?;
            fixtures.backfill().await?;
        }

        let compactor = fixtures.new_compactor().await?;
        let (metrics, _) = compactor.step().await?;
        assert_eq!(0, metrics.len());

        Ok(())
    }

    #[convex_macro::test_runtime]
    async fn compact_with_too_few_large_segments_under_size_threshold_does_not_compact_them(
        rt: TestRuntime,
    ) -> anyhow::Result<()> {
        let config = CompactionConfig::default();
        let min_compaction_segments = config.min_compaction_segments;
        let config = CompactionConfig {
            // Treat everything as a large segment
            small_segment_threshold_bytes: 0,
            max_segment_size_bytes: min_compaction_segments * VECTOR_SIZE_BYTES,
            ..config
        };
        let fixtures = VectorFixtures::new_with_config(rt.clone(), config).await?;
        let index_data = fixtures.enabled_vector_index().await?;

        // Merge a large segment that will now be at the size threshold.
        for _ in 0..min_compaction_segments {
            fixtures
                .add_document_vec_array(index_data.index_name.table(), [3f64, 4f64])
                .await?;
            fixtures.backfill().await?;
        }
        let compactor = fixtures.new_compactor().await?;
        compactor.step().await?;

        // Then add N - 1 large segments and ensure they're not merged with our previous
        // at threshold segment.
        for _ in 0..min_compaction_segments - 1 {
            fixtures
                .add_document_vec_array(index_data.index_name.table(), [3f64, 4f64])
                .await?;
            fixtures.backfill().await?;
        }

        let compactor = fixtures.new_compactor().await?;
        let (metrics, _) = compactor.step().await?;
        assert_eq!(0, metrics.len());

        Ok(())
    }

    #[convex_macro::test_runtime]
    async fn compact_small_segments_skips_those_over_size_threshold(
        rt: TestRuntime,
    ) -> anyhow::Result<()> {
        let config = CompactionConfig::default();
        let min_compaction_segments = config.min_compaction_segments;
        let config = CompactionConfig {
            // N 8 byte vectors + slop
            max_segment_size_bytes: (min_compaction_segments * VECTOR_SIZE_BYTES) + 2,
            ..config
        };
        let fixtures = VectorFixtures::new_with_config(rt.clone(), config).await?;
        let index_data = fixtures.enabled_vector_index().await?;

        // Create N + 1 small segments.
        for _ in 0..min_compaction_segments + 1 {
            fixtures
                .add_document_vec_array(index_data.index_name.table(), [3f64, 4f64])
                .await?;
            fixtures.backfill().await?;
        }

        let compactor = fixtures.new_compactor().await?;
        let (metrics, _) = compactor.step().await?;
        assert_eq!(
            metrics,
            btreemap! { index_data.resolved_index_name => min_compaction_segments }
        );
        let segments = fixtures
            .get_segments_metadata(index_data.index_name)
            .await?;
        assert_eq!(2, segments.len());

        Ok(())
    }

    #[convex_macro::test_runtime]
    async fn compact_large_segments_skips_those_over_size_threshold(
        rt: TestRuntime,
    ) -> anyhow::Result<()> {
        let config = CompactionConfig::default();
        let min_compaction_segments = config.min_compaction_segments;
        let config = CompactionConfig {
            // Treat everything as a large segment
            small_segment_threshold_bytes: 0,
            // Set the segment size so that we can create one large segment out
            // N small segments each of which contain 1 vector of 8 bytes.
            max_segment_size_bytes: min_compaction_segments * VECTOR_SIZE_BYTES,
            ..config
        };
        let fixtures = VectorFixtures::new_with_config(rt.clone(), config).await?;
        let index_data = fixtures.enabled_vector_index().await?;

        // Create a large segment that will now be at the size threshold.
        for _ in 0..min_compaction_segments {
            fixtures
                .add_document_vec_array(index_data.index_name.table(), [3f64, 4f64])
                .await?;
        }
        fixtures.backfill().await?;

        // Create N more segments that would make up a second large segment.
        for _ in 0..min_compaction_segments {
            fixtures
                .add_document_vec_array(index_data.index_name.table(), [3f64, 4f64])
                .await?;
            fixtures.backfill().await?;
        }

        // Run compaction and ensure that we only compact segments while the total size
        // is under our threshold.
        let compactor = fixtures.new_compactor().await?;
        let (metrics, _) = compactor.step().await?;
        assert_eq!(
            metrics,
            btreemap! { index_data.resolved_index_name => min_compaction_segments }
        );
        let segments = fixtures
            .get_segments_metadata(index_data.index_name)
            .await?;
        assert_eq!(
            segments
                .into_iter()
                .map(|segment| segment.num_vectors)
                .collect_vec(),
            // 1-1 ratio between segments and vectors
            vec![
                min_compaction_segments as u32,
                min_compaction_segments as u32
            ]
        );

        Ok(())
    }

    #[convex_macro::test_runtime]
    async fn compact_large_segments_does_not_generate_segment_over_max_threshold(
        rt: TestRuntime,
    ) -> anyhow::Result<()> {
        let config = CompactionConfig::default();
        let min_compaction_segments = config.min_compaction_segments;
        let config = CompactionConfig {
            // Treat everything as a large segment
            small_segment_threshold_bytes: 0,
            max_segment_size_bytes: min_compaction_segments * VECTOR_SIZE_BYTES,
            ..config
        };
        let fixtures = VectorFixtures::new_with_config(rt.clone(), config).await?;
        let index_data = fixtures.enabled_vector_index().await?;

        for _ in 0..min_compaction_segments + 1 {
            fixtures
                .add_document_vec_array(index_data.index_name.table(), [3f64, 4f64])
                .await?;
            fixtures.backfill().await?;
        }
        let compactor = fixtures.new_compactor().await?;
        let (metrics, _) = compactor.step().await?;
        assert_eq!(
            metrics,
            btreemap! { index_data.resolved_index_name => min_compaction_segments }
        );
        let segments = fixtures
            .get_segments_metadata(index_data.index_name)
            .await?;
        assert_eq!(
            segments
                .into_iter()
                .map(|segment| segment.num_vectors)
                .sorted()
                .collect_vec(),
            vec![
                // One segment that we couldn't compact because it would have made the compacted
                // segment exceed the max size.
                1,
                // One compacted segment.
                min_compaction_segments as u32
            ]
        );

        Ok(())
    }

    #[convex_macro::test_runtime]
    async fn compact_with_small_segments_over_delete_threshold_does_not_compact_them(
        rt: TestRuntime,
    ) -> anyhow::Result<()> {
        let fixtures = VectorFixtures::new(rt.clone()).await?;
        let index_data = fixtures.enabled_vector_index().await?;

        let mut ids = vec![];
        // Create a segment that's well under the default small segment threshold size.
        for _ in 0..3 {
            ids.push(
                fixtures
                    .add_document_vec_array(index_data.index_name.table(), [3f64, 4f64])
                    .await?,
            );
        }
        fixtures.backfill().await?;

        // Delete all but 1 vector.
        let mut tx = fixtures.db.begin_system().await?;
        for id in &ids[0..ids.len() - 1] {
            UserFacingModel::new_root_for_test(&mut tx)
                .delete((*id).into())
                .await?;
        }
        fixtures.db.commit(tx).await?;
        fixtures.backfill().await?;

        // Make sure we don't recompact the small segment.
        let compactor = fixtures.new_compactor().await?;
        let (metrics, _) = compactor.step().await?;
        assert_eq!(0, metrics.len());

        let segments = fixtures
            .get_segments_metadata(index_data.index_name.clone())
            .await?;
        let total_deletes = segments
            .into_iter()
            .fold(0, |acc, segment| acc + segment.num_deleted);
        assert_ne!(0, total_deletes);

        Ok(())
    }

    #[convex_macro::test_runtime]
    async fn compact_with_large_segments_over_delete_threshold_compacts_away_deletes(
        rt: TestRuntime,
    ) -> anyhow::Result<()> {
        let config = CompactionConfig::default();
        let config = CompactionConfig {
            // treat everything as a large segment
            small_segment_threshold_bytes: 0,
            ..config
        };
        let fixtures = VectorFixtures::new_with_config(rt.clone(), config).await?;
        let index_data = fixtures.enabled_vector_index().await?;

        // Create a 'large' segment.
        let mut ids = vec![];
        for _ in 0..3 {
            ids.push(
                fixtures
                    .add_document_vec_array(index_data.index_name.table(), [3f64, 4f64])
                    .await?,
            );
        }
        fixtures.backfill().await?;

        // Delete all but one vctor.
        let mut tx = fixtures.db.begin_system().await?;
        for id in &ids[0..ids.len() - 1] {
            UserFacingModel::new_root_for_test(&mut tx)
                .delete((*id).into())
                .await?;
        }
        fixtures.db.commit(tx).await?;
        fixtures.backfill().await?;

        // Make sure that we recompact it to remove the deletes.
        let segments = fixtures
            .get_segments_metadata(index_data.index_name.clone())
            .await?;
        let total_deletes = segments
            .into_iter()
            .fold(0, |acc, segment| acc + segment.num_deleted);
        assert_ne!(0, total_deletes);

        let compactor = fixtures.new_compactor().await?;
        let (metrics, _) = compactor.step().await?;
        // It should have compacted just 1 segment
        assert_eq!(metrics, btreemap! { index_data.resolved_index_name => 1 });

        let segments = fixtures
            .get_segments_metadata(index_data.index_name)
            .await?;
        for segment in segments {
            assert_eq!(0, segment.num_deleted);
        }

        Ok(())
    }

    #[convex_macro::test_runtime]
    async fn compact_with_delete_during_compaction_reconciles_delete(
        rt: TestRuntime,
    ) -> anyhow::Result<()> {
        let fixtures = VectorFixtures::new(rt.clone()).await?;
        let min_compaction_segments = CompactionConfig::default().min_compaction_segments;
        let index_data = fixtures.enabled_vector_index().await?;

        let mut last_id = None;
        for _ in 0..min_compaction_segments {
            last_id = Some(
                fixtures
                    .add_document_vec_array(index_data.index_name.table(), [3f64, 4f64])
                    .await?,
            );
            fixtures.backfill().await?;
        }

        let compactor = fixtures
            .new_compactor_delete_on_compact(last_id.unwrap())
            .await?;
        let (metrics, _) = compactor.step().await?;
        assert_eq!(
            metrics,
            btreemap! { index_data.resolved_index_name => min_compaction_segments }
        );

        // Then it should have the vectors from all 4 segments, but one should be
        // marked as deleted.
        let segments = fixtures
            .get_segments_metadata(index_data.index_name.clone())
            .await?;
        assert_eq!(segments.len(), 1);
        let segment = segments.first().unwrap();
        assert_eq!(segment.num_deleted, 1);
        assert_eq!(segment.num_vectors, min_compaction_segments as u32);

        let (results, _usage_stats) = fixtures
            .db
            .vector_search(
                Identity::system(),
                VectorSearch {
                    index_name: index_data.index_name,
                    vector: vec![0f32, 0f32],
                    limit: Some(10),
                    expressions: btreeset![],
                },
            )
            .await?;
        assert!(!results
            .into_iter()
            .map(|result| result.id.internal_id())
            .any(|id| id == last_id.unwrap().internal_id()));

        Ok(())
    }
}

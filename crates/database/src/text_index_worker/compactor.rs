use std::sync::Arc;

use common::runtime::Runtime;
use search::Searcher;
use storage::Storage;

use crate::{
    index_workers::search_compactor::{
        CompactionConfig,
        SearchIndexCompactor,
    },
    text_index_worker::{
        text_meta::TextSearchIndex,
        TextIndexMetadataWriter,
    },
    Database,
};

pub type TextIndexCompactor<RT> = SearchIndexCompactor<RT, TextSearchIndex>;

pub(crate) fn new_text_compactor<RT: Runtime>(
    database: Database<RT>,
    searcher: Arc<dyn Searcher>,
    search_storage: Arc<dyn Storage>,
    config: CompactionConfig,
    writer: TextIndexMetadataWriter<RT>,
) -> TextIndexCompactor<RT> {
    TextIndexCompactor::new(database, searcher, search_storage, config, writer)
}

#[allow(dead_code)]
#[cfg(any(test, feature = "testing"))]
pub(crate) fn new_text_compactor_for_tests<RT: Runtime>(
    runtime: RT,
    database: Database<RT>,
    search_storage: Arc<dyn Storage>,
    searcher: Arc<dyn Searcher>,
    config: CompactionConfig,
) -> TextIndexCompactor<RT> {
    let writer = TextIndexMetadataWriter::new(runtime, database.clone(), search_storage.clone());
    SearchIndexCompactor::new(database, searcher, search_storage.clone(), config, writer)
}

#[allow(dead_code)]
#[cfg(any(test, feature = "testing"))]
pub async fn compact_text_indexes_in_test<RT: Runtime>(
    runtime: RT,
    database: Database<RT>,
    search_storage: Arc<dyn Storage>,
    searcher: Arc<dyn Searcher>,
) -> anyhow::Result<()> {
    let compactor = new_text_compactor_for_tests(
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
    use common::runtime::testing::TestRuntime;
    use maplit::btreemap;

    use crate::{
        index_workers::search_compactor::CompactionConfig,
        tests::text_test_utils::TextFixtures,
    };

    #[convex_macro::test_runtime]
    async fn compact_with_multiple_small_segments_merges_them(
        rt: TestRuntime,
    ) -> anyhow::Result<()> {
        let fixtures = TextFixtures::new(rt.clone()).await?;
        let index_data = fixtures.enabled_text_index().await?;
        let min_compaction_segments = CompactionConfig::default().min_compaction_segments;

        for _ in 0..min_compaction_segments {
            fixtures.add_document("horse").await?;
            fixtures.backfill().await?;
        }

        let compactor = fixtures.new_compactor();
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
    async fn compact_test(rt: TestRuntime) -> anyhow::Result<()> {
        let fixtures = TextFixtures::new(rt.clone()).await?;
        let index_data = fixtures.enabled_text_index().await?;

        let document_id = fixtures.add_document("horse").await?;
        fixtures.backfill().await?;
        fixtures.replace_document(document_id, "cat").await?;
        fixtures.add_document("other").await?;
        fixtures.backfill().await?;
        fixtures.replace_document(document_id, "bat").await?;
        fixtures.backfill().await?;
        fixtures.replace_document(document_id, "rat").await?;
        fixtures.backfill().await?;

        let mut tx = fixtures.db.begin_system().await?;
        tx.delete_inner(document_id).await?;
        fixtures.db.commit(tx).await?;
        fixtures.backfill().await?;

        let compactor = fixtures.new_compactor();
        let (metrics, _) = compactor.step().await?;
        assert_eq!(metrics, btreemap! { index_data.resolved_index_name => 4});

        let segments = fixtures
            .get_segments_metadata(index_data.index_name.clone())
            .await?;
        assert_eq!(segments.len(), 1);

        assert_eq!(
            fixtures
                .search(index_data.index_name.clone(), "other")
                .await?
                .len(),
            1
        );
        assert_eq!(
            fixtures
                .search(index_data.index_name.clone(), "horse")
                .await?
                .len(),
            0
        );
        assert_eq!(
            fixtures
                .search(index_data.index_name.clone(), "cat")
                .await?
                .len(),
            0
        );
        assert_eq!(
            fixtures
                .search(index_data.index_name.clone(), "bat")
                .await?
                .len(),
            0
        );
        assert_eq!(
            fixtures
                .search(index_data.index_name.clone(), "rat")
                .await?
                .len(),
            0
        );

        Ok(())
    }

    #[ignore]
    #[convex_macro::test_runtime]
    async fn compact_when_compaction_removes_data_in_all_segments_should_not_panic(
        rt: TestRuntime,
    ) -> anyhow::Result<()> {
        let fixtures = TextFixtures::new(rt.clone()).await?;
        let index_data = fixtures.enabled_text_index().await?;

        let document_id = fixtures.add_document("horse").await?;
        fixtures.backfill().await?;
        fixtures.replace_document(document_id, "cat").await?;
        fixtures.backfill().await?;
        fixtures.replace_document(document_id, "bat").await?;
        fixtures.backfill().await?;
        fixtures.replace_document(document_id, "rat").await?;
        fixtures.backfill().await?;

        let mut tx = fixtures.db.begin_system().await?;
        tx.delete_inner(document_id).await?;
        fixtures.db.commit(tx).await?;
        fixtures.backfill().await?;

        let compactor = fixtures.new_compactor();
        let (metrics, _) = compactor.step().await?;
        assert_eq!(metrics, btreemap! { index_data.resolved_index_name => 4});

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
        let fixtures = TextFixtures::new_with_config(rt.clone(), config).await?;
        let index_data = fixtures.enabled_text_index().await?;

        for _ in 0..min_compaction_segments {
            fixtures.add_document("goat").await?;
            fixtures.backfill().await?;
        }

        let compactor = fixtures.new_compactor();
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
            TextFixtures::new_with_config(rt.clone(), CompactionConfig::default()).await?;
        let min_compaction_segments = CompactionConfig::default().min_compaction_segments;
        let index_data = fixtures.backfilled_text_index().await?;

        for _ in 0..min_compaction_segments {
            fixtures.add_document("sheep").await?;
            fixtures.backfill().await?;
        }

        let compactor = fixtures.new_compactor();
        let (metrics, _) = compactor.step().await?;
        assert_eq!(
            metrics,
            btreemap! { index_data.resolved_index_name => min_compaction_segments }
        );

        let segments = fixtures
            .get_segments_metadata(index_data.index_name.clone())
            .await?;
        assert_eq!(segments.len(), 1);
        fixtures.enable_index(&index_data.index_name).await?;
        let results = fixtures.search(index_data.index_name, "sheep").await?;
        assert_eq!(results.len() as u64, min_compaction_segments);

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
        let fixtures = TextFixtures::new_with_config(rt.clone(), config).await?;
        fixtures.enabled_text_index().await?;

        for _ in 0..CompactionConfig::default().min_compaction_segments {
            fixtures.add_document("cat").await?;
            fixtures.backfill().await?;
        }

        let compactor = fixtures.new_compactor();
        let (metrics, _) = compactor.step().await?;
        assert_eq!(0, metrics.len());

        Ok(())
    }
}

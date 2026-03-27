/// Unified tests for the incremental backfill algorithm in
/// `search_flusher::build_multipart_segment`. These tests are generic over
/// `SearchIndexTestHarness` so they run for both text and vector indexes.
use async_trait::async_trait;
use runtime::testing::TestRuntime;
use value::DeveloperDocumentId;

/// Simplified view of a segment's document counts, common to both index
/// types.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SegmentInfo {
    pub num_documents: u64,
    pub num_deleted: u64,
}

/// Which high-level state the index is in.
#[derive(Debug)]
pub enum IndexStateVariant {
    Backfilling { num_segments: usize },
    Backfilled { num_segments: usize },
}

/// Abstraction over TextFixtures / vector Scenario so we can write tests
/// once and run them for both index types.
#[async_trait]
pub trait SearchIndexTestHarness: Sized + Send {
    /// Create fixtures and a database, but no index yet.
    async fn new(rt: TestRuntime) -> anyhow::Result<Self>;

    /// Create a backfilling index on the table.
    async fn create_backfilling_index(&self) -> anyhow::Result<()>;

    /// Insert `count` documents and return their IDs.
    async fn add_documents(&self, count: u32) -> anyhow::Result<Vec<DeveloperDocumentId>>;

    /// Delete a document by ID.
    async fn delete_document(&self, id: DeveloperDocumentId) -> anyhow::Result<()>;

    /// Replace a document's content (new random content).
    async fn replace_document(&self, id: DeveloperDocumentId) -> anyhow::Result<()>;

    /// Create a backfill flusher whose incremental threshold yields roughly
    /// `docs_per_segment` documents per segment, and step it once.
    async fn step_backfill(&self, docs_per_segment: u32) -> anyhow::Result<()>;

    /// Get the current on-disk state variant.
    async fn index_state(&self) -> anyhow::Result<IndexStateVariant>;

    /// Get per-segment (num_documents, num_deleted) for all segments.
    async fn segment_stats(&self) -> anyhow::Result<Vec<SegmentInfo>>;

    /// Enable the index so it becomes queryable.
    async fn enable_index(&self) -> anyhow::Result<()>;

    /// Assert that exactly `expected_ids` are returned by a broad search.
    async fn assert_documents_searchable(
        &self,
        expected_ids: &[DeveloperDocumentId],
    ) -> anyhow::Result<()>;
}

// -----------------------------------------------------------------------
// Generic test functions
// -----------------------------------------------------------------------

/// Backfilling a table with multiple segments' worth of data should produce
/// one segment per flusher step, then transition to Backfilled on the final
/// step.
async fn test_backfill_creates_segments_incrementally<H: SearchIndexTestHarness>(
    rt: TestRuntime,
) -> anyhow::Result<()> {
    let harness = H::new(rt).await?;
    let num_parts = 3u32;
    let docs_per_part = 2u32;

    let ids = harness.add_documents(num_parts * docs_per_part).await?;
    harness.create_backfilling_index().await?;

    for i in 0..num_parts {
        harness.step_backfill(docs_per_part).await?;
        let state = harness.index_state().await?;
        if i < num_parts - 1 {
            let IndexStateVariant::Backfilling { num_segments } = state else {
                anyhow::bail!("Expected Backfilling at step {i}, got {state:?}");
            };
            assert_eq!(num_segments, (i + 1) as usize);
        } else {
            let IndexStateVariant::Backfilled { num_segments } = state else {
                anyhow::bail!("Expected Backfilled at final step, got {state:?}");
            };
            assert_eq!(num_segments, num_parts as usize);
        }
    }

    harness.enable_index().await?;
    harness.assert_documents_searchable(&ids).await?;
    Ok(())
}

/// After the first flusher step (with more data remaining), the index
/// should still be in the Backfilling state with one segment.
async fn test_backfill_still_backfilling_after_partial<H: SearchIndexTestHarness>(
    rt: TestRuntime,
) -> anyhow::Result<()> {
    let harness = H::new(rt).await?;
    // Add more docs than fit in one segment.
    harness.add_documents(3).await?;
    harness.create_backfilling_index().await?;

    harness.step_backfill(2).await?;
    let state = harness.index_state().await?;
    let IndexStateVariant::Backfilling { num_segments } = state else {
        anyhow::bail!("Expected Backfilling, got {state:?}");
    };
    assert_eq!(num_segments, 1);
    Ok(())
}

/// Deleting a document after its segment is built should mark it as deleted
/// in the next segment build.
async fn test_backfill_handles_deletes<H: SearchIndexTestHarness>(
    rt: TestRuntime,
) -> anyhow::Result<()> {
    let harness = H::new(rt).await?;
    // Seed enough docs for 1 segment + 1 leftover so backfill doesn't
    // finish on the first step.
    let docs_per_part = 2u32;
    let mut ids = harness.add_documents(docs_per_part + 1).await?;
    // Sort IDs to match the by_id index scan order so we know which docs
    // land in which segment.
    ids.sort();
    harness.create_backfilling_index().await?;

    // First step: builds segment with `docs_per_part` docs (the first
    // `docs_per_part` IDs in sorted order).
    harness.step_backfill(docs_per_part).await?;
    let state = harness.index_state().await?;
    let IndexStateVariant::Backfilling { num_segments } = state else {
        anyhow::bail!("Expected Backfilling after step 1, got {state:?}");
    };
    assert_eq!(num_segments, 1);
    let segments = harness.segment_stats().await?;
    assert_eq!(segments.len(), 1);
    assert_eq!(segments[0].num_documents, docs_per_part as u64);
    assert_eq!(segments[0].num_deleted, 0);

    // Delete a document from the first segment (ids[0] is the first in
    // by_id order, so it's guaranteed to be in the first segment).
    harness.delete_document(ids[0]).await?;

    // Second step: should complete backfill. The first segment should now
    // have 1 delete, and the second segment has the remaining doc.
    harness.step_backfill(docs_per_part).await?;
    let segments = harness.segment_stats().await?;
    assert_eq!(segments.len(), 2);
    assert_eq!(segments[0].num_deleted, 1);

    harness.enable_index().await?;
    harness.assert_documents_searchable(&ids[1..]).await?;
    Ok(())
}

/// Replacing a document after its segment is built should mark the old
/// version as deleted and include the new version in the next segment.
async fn test_backfill_handles_replaces<H: SearchIndexTestHarness>(
    rt: TestRuntime,
) -> anyhow::Result<()> {
    let harness = H::new(rt).await?;
    let docs_per_part = 2u32;
    let mut ids = harness.add_documents(docs_per_part + 1).await?;
    ids.sort();
    harness.create_backfilling_index().await?;

    harness.step_backfill(docs_per_part).await?;
    let segments = harness.segment_stats().await?;
    assert_eq!(segments.len(), 1);

    // Replace a document from the first segment.
    harness.replace_document(ids[0]).await?;

    harness.step_backfill(docs_per_part).await?;
    let segments = harness.segment_stats().await?;
    assert_eq!(segments.len(), 2);
    // First segment: old version marked deleted.
    assert_eq!(segments[0].num_documents, docs_per_part as u64);
    assert_eq!(segments[0].num_deleted, 1);
    // Second segment: remaining doc + replacement.
    assert_eq!(segments[1].num_documents, 2);
    assert_eq!(segments[1].num_deleted, 0);

    harness.enable_index().await?;
    harness.assert_documents_searchable(&ids).await?;
    Ok(())
}

/// Multiple replaces of the same document between segment builds should
/// only include the most recent version.
async fn test_backfill_skips_past_revisions<H: SearchIndexTestHarness>(
    rt: TestRuntime,
) -> anyhow::Result<()> {
    let harness = H::new(rt).await?;
    let docs_per_part = 2u32;
    let mut ids = harness.add_documents(docs_per_part + 1).await?;
    ids.sort();
    harness.create_backfilling_index().await?;

    harness.step_backfill(docs_per_part).await?;
    let segments = harness.segment_stats().await?;
    assert_eq!(segments.len(), 1);

    // Do 2 replaces of the same doc (from first segment).
    harness.replace_document(ids[1]).await?;
    harness.replace_document(ids[1]).await?;

    harness.step_backfill(docs_per_part).await?;
    let segments = harness.segment_stats().await?;
    assert_eq!(segments.len(), 2);
    // First segment: old version deleted.
    assert_eq!(segments[0].num_documents, docs_per_part as u64);
    assert_eq!(segments[0].num_deleted, 1);
    // Second segment: remaining doc + latest replacement (NOT both
    // replacements).
    assert_eq!(segments[1].num_documents, 2);
    assert_eq!(segments[1].num_deleted, 0);

    harness.enable_index().await?;
    harness.assert_documents_searchable(&ids).await?;
    Ok(())
}

/// Documents added after the index is created (but before backfill
/// completes) should be captured in subsequent segments.
async fn test_backfill_captures_new_writes<H: SearchIndexTestHarness>(
    rt: TestRuntime,
) -> anyhow::Result<()> {
    let harness = H::new(rt).await?;
    let docs_per_part = 2u32;

    harness.create_backfilling_index().await?;
    // Add docs after creating the index.
    let mut ids = harness.add_documents(docs_per_part * 2).await?;

    harness.step_backfill(docs_per_part).await?;
    let state = harness.index_state().await?;
    let IndexStateVariant::Backfilling { num_segments: 1 } = state else {
        anyhow::bail!("Expected Backfilling with 1 segment, got {state:?}");
    };

    // Add more docs between steps.
    let new_ids = harness.add_documents(1).await?;
    ids.extend(new_ids);

    // Finish backfill (may take 2 more steps).
    // This is a bit hacky because we don't know whether the new id was before or
    // after the cursor.
    harness.step_backfill(docs_per_part).await?;
    harness.step_backfill(docs_per_part).await?;

    let state = harness.index_state().await?;
    let IndexStateVariant::Backfilled { .. } = state else {
        anyhow::bail!("Expected Backfilled, got {state:?}");
    };

    harness.enable_index().await?;
    harness.assert_documents_searchable(&ids).await?;
    Ok(())
}

// -----------------------------------------------------------------------
// Text harness
// -----------------------------------------------------------------------

mod text {
    use common::bootstrap_model::index::{
        text_index::{
            TextIndexSnapshotData,
            TextIndexState,
        },
        IndexConfig,
    };
    use runtime::testing::TestRuntime;
    use value::{
        DeveloperDocumentId,
        TableNamespace,
    };

    use super::{
        IndexStateVariant,
        SearchIndexTestHarness,
        SegmentInfo,
    };
    use crate::tests::text_test_utils::TextFixtures;

    pub struct TextSearchHarness {
        fixtures: TextFixtures,
    }

    #[async_trait::async_trait]
    impl SearchIndexTestHarness for TextSearchHarness {
        async fn new(rt: TestRuntime) -> anyhow::Result<Self> {
            let fixtures = TextFixtures::new(rt).await?;
            Ok(Self { fixtures })
        }

        async fn create_backfilling_index(&self) -> anyhow::Result<()> {
            self.fixtures.insert_backfilling_text_index().await?;
            Ok(())
        }

        async fn add_documents(&self, count: u32) -> anyhow::Result<Vec<DeveloperDocumentId>> {
            let mut ids = Vec::new();
            for i in 0..count {
                let resolved_id = self
                    .fixtures
                    .add_document(&format!("searchable_text_{i}"))
                    .await?;
                ids.push(resolved_id.developer_id);
            }
            Ok(ids)
        }

        async fn delete_document(&self, id: DeveloperDocumentId) -> anyhow::Result<()> {
            let mut tx = self.fixtures.db.begin_system().await?;
            let resolved = tx.resolve_developer_id(&id, TableNamespace::test_user())?;
            tx.delete_inner(resolved).await?;
            self.fixtures.db.commit(tx).await?;
            Ok(())
        }

        async fn replace_document(&self, id: DeveloperDocumentId) -> anyhow::Result<()> {
            let resolved = {
                let mut tx = self.fixtures.db.begin_system().await?;
                tx.resolve_developer_id(&id, TableNamespace::test_user())?
            };
            self.fixtures
                .replace_document(resolved, "replaced_text")
                .await?;
            Ok(())
        }

        async fn step_backfill(&self, docs_per_segment: u32) -> anyhow::Result<()> {
            // Each text doc is ~26 bytes (text field "searchable_text_N"
            // = 18 chars + filter field "#general" = 8 chars). The
            // threshold must be <= (docs_per_segment * 26) so that the
            // scan stops after exactly docs_per_segment documents.
            let threshold = (docs_per_segment as usize) * 26;
            let flusher = self
                .fixtures
                .new_search_flusher_builder()
                .set_soft_limit(0)
                .set_incremental_multipart_threshold_bytes(threshold)
                .build();
            flusher.step().await?;
            Ok(())
        }

        async fn index_state(&self) -> anyhow::Result<IndexStateVariant> {
            let index_name = "table.search_index".parse()?;
            let metadata = self.fixtures.get_index_metadata(index_name).await?;
            let IndexConfig::Text { on_disk_state, .. } = &metadata.config else {
                anyhow::bail!("Not a text index");
            };
            match on_disk_state {
                TextIndexState::Backfilling(state) => Ok(IndexStateVariant::Backfilling {
                    num_segments: state.segments.len(),
                }),
                TextIndexState::Backfilled { snapshot, .. }
                | TextIndexState::SnapshottedAt(snapshot) => {
                    let num_segments = match &snapshot.data {
                        TextIndexSnapshotData::MultiSegment(segments) => segments.len(),
                        TextIndexSnapshotData::Unknown(_) => {
                            anyhow::bail!("Unknown snapshot data");
                        },
                    };
                    Ok(IndexStateVariant::Backfilled { num_segments })
                },
            }
        }

        async fn segment_stats(&self) -> anyhow::Result<Vec<SegmentInfo>> {
            let index_name = "table.search_index".parse()?;
            let segments = self.fixtures.get_segments_metadata(index_name).await?;
            Ok(segments
                .into_iter()
                .map(|s| SegmentInfo {
                    num_documents: s.num_indexed_documents,
                    num_deleted: s.num_deleted_documents,
                })
                .collect())
        }

        async fn enable_index(&self) -> anyhow::Result<()> {
            let index_name = "table.search_index".parse()?;
            self.fixtures.enable_index(&index_name).await
        }

        async fn assert_documents_searchable(
            &self,
            expected_ids: &[DeveloperDocumentId],
        ) -> anyhow::Result<()> {
            let index_name = "table.search_index".parse()?;
            // Search for a term that all added documents contain.
            let results = self.fixtures.search(index_name, "searchable_text").await?;
            let mut result_ids: Vec<_> = results.iter().map(|r| r.id().developer_id).collect();
            result_ids.sort();
            let mut expected: Vec<_> = expected_ids.to_vec();
            expected.sort();
            assert_eq!(result_ids, expected);
            Ok(())
        }
    }

    #[convex_macro::test_runtime]
    async fn test_text_backfill_creates_segments_incrementally(
        rt: TestRuntime,
    ) -> anyhow::Result<()> {
        super::test_backfill_creates_segments_incrementally::<TextSearchHarness>(rt).await
    }

    #[convex_macro::test_runtime]
    async fn test_text_backfill_still_backfilling_after_partial(
        rt: TestRuntime,
    ) -> anyhow::Result<()> {
        super::test_backfill_still_backfilling_after_partial::<TextSearchHarness>(rt).await
    }

    #[convex_macro::test_runtime]
    async fn test_text_backfill_handles_deletes(rt: TestRuntime) -> anyhow::Result<()> {
        super::test_backfill_handles_deletes::<TextSearchHarness>(rt).await
    }

    #[convex_macro::test_runtime]
    async fn test_text_backfill_handles_replaces(rt: TestRuntime) -> anyhow::Result<()> {
        super::test_backfill_handles_replaces::<TextSearchHarness>(rt).await
    }

    #[convex_macro::test_runtime]
    async fn test_text_backfill_skips_past_revisions(rt: TestRuntime) -> anyhow::Result<()> {
        super::test_backfill_skips_past_revisions::<TextSearchHarness>(rt).await
    }

    #[convex_macro::test_runtime]
    async fn test_text_backfill_captures_new_writes(rt: TestRuntime) -> anyhow::Result<()> {
        super::test_backfill_captures_new_writes::<TextSearchHarness>(rt).await
    }
}

// -----------------------------------------------------------------------
// Vector harness
// -----------------------------------------------------------------------

mod vector {
    use std::sync::Arc;

    use common::{
        bootstrap_model::index::{
            vector_index::{
                VectorIndexSnapshotData,
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
    };
    use itertools::Itertools;
    use keybroker::Identity;
    use qdrant_segment::types::VECTOR_ELEMENT_SIZE;
    use runtime::testing::TestRuntime;
    use search::searcher::InProcessSearcher;
    use storage::Storage;
    use value::{
        assert_obj,
        ConvexValue,
        DeveloperDocumentId,
        TableName,
        TableNamespace,
    };

    use super::{
        IndexStateVariant,
        SearchIndexTestHarness,
        SegmentInfo,
    };
    use crate::{
        search_index_workers::FlusherType,
        test_helpers::{
            vector_utils::{
                random_vector_value,
                DIMENSIONS,
            },
            DbFixtures,
            DbFixturesArgs,
        },
        vector_index_worker::flusher::new_vector_flusher_for_tests,
        Database,
        IndexModel,
        TableModel,
        UserFacingModel,
    };

    const TABLE_NAME: &str = "test";
    const INDEX_NAME: &str = "test.by_embedding";
    const INDEXED_FIELD: &str = "embedding";
    const FILTER_FIELDS: &[&str] = &["A", "B", "C", "D"];

    pub struct VectorSearchHarness {
        rt: TestRuntime,
        database: Database<TestRuntime>,
        reader: Arc<dyn PersistenceReader>,
        search_storage: Arc<dyn Storage>,
    }

    #[async_trait::async_trait]
    impl SearchIndexTestHarness for VectorSearchHarness {
        async fn new(rt: TestRuntime) -> anyhow::Result<Self> {
            let DbFixtures {
                tp,
                db,
                search_storage,
                ..
            } = DbFixtures::new_with_args(
                &rt,
                DbFixturesArgs {
                    searcher: Some(Arc::new(InProcessSearcher::new(rt.clone())?)),
                    ..Default::default()
                },
            )
            .await?;
            let handle = db.start_search_and_vector_bootstrap();
            handle.join().await?;

            Ok(Self {
                rt,
                database: db,
                reader: tp.reader(),
                search_storage,
            })
        }

        async fn create_backfilling_index(&self) -> anyhow::Result<()> {
            let table_name: TableName = TABLE_NAME.parse()?;
            let mut tx = self.database.begin(Identity::system()).await?;
            let namespace = TableNamespace::test_user();
            TableModel::new(&mut tx)
                .insert_table_metadata_for_test(namespace, &table_name)
                .await?;
            let index = IndexMetadata::new_backfilling_vector_index(
                INDEX_NAME.parse()?,
                INDEXED_FIELD.parse()?,
                DIMENSIONS.try_into()?,
                FILTER_FIELDS.iter().map(|f| f.parse()).try_collect()?,
            );
            IndexModel::new(&mut tx)
                .add_application_index(namespace, index)
                .await?;
            self.database.commit(tx).await?;
            Ok(())
        }

        async fn add_documents(&self, count: u32) -> anyhow::Result<Vec<DeveloperDocumentId>> {
            let mut ids = Vec::new();
            for _ in 0..count {
                let mut tx = self.database.begin(Identity::system()).await?;
                let vector = random_vector_value(&mut self.rt.rng());
                let obj = assert_obj!(
                    INDEXED_FIELD => vector,
                    "A" => ConvexValue::Int64(1017)
                );
                let id = UserFacingModel::new_root_for_test(&mut tx)
                    .insert(TABLE_NAME.parse()?, obj)
                    .await?;
                self.database.commit(tx).await?;
                ids.push(id);
            }
            Ok(ids)
        }

        async fn delete_document(&self, id: DeveloperDocumentId) -> anyhow::Result<()> {
            let mut tx = self.database.begin_system().await?;
            let mut model = UserFacingModel::new(&mut tx, TableNamespace::test_user());
            model.delete(id).await?;
            self.database.commit(tx).await?;
            Ok(())
        }

        async fn replace_document(&self, id: DeveloperDocumentId) -> anyhow::Result<()> {
            let mut tx = self.database.begin_system().await?;
            let random_vector = random_vector_value(&mut self.rt.rng());
            let obj = assert_obj!(
                INDEXED_FIELD => random_vector,
                "A" => ConvexValue::Int64(1017)
            );
            let mut model = UserFacingModel::new(&mut tx, TableNamespace::test_user());
            model.replace(id, obj).await?;
            self.database.commit(tx).await?;
            Ok(())
        }

        async fn step_backfill(&self, docs_per_segment: u32) -> anyhow::Result<()> {
            let threshold = (DIMENSIONS * (VECTOR_ELEMENT_SIZE as u32) * docs_per_segment) as usize;
            let flusher = new_vector_flusher_for_tests(
                self.rt.clone(),
                self.database.clone(),
                self.reader.clone(),
                self.search_storage.clone(),
                *VECTOR_INDEX_SIZE_SOFT_LIMIT,
                *MULTI_SEGMENT_FULL_SCAN_THRESHOLD_KB,
                threshold,
                FlusherType::Backfill,
            );
            flusher.step().await?;
            Ok(())
        }

        async fn index_state(&self) -> anyhow::Result<IndexStateVariant> {
            let mut tx = self.database.begin_system().await?;
            let configs: Vec<_> = IndexModel::new(&mut tx)
                .get_all_indexes()?
                .filter_map(|idx| {
                    if let IndexConfig::Vector { on_disk_state, .. } = &idx.config {
                        Some(on_disk_state.clone())
                    } else {
                        None
                    }
                })
                .collect();
            assert_eq!(configs.len(), 1);
            match &configs[0] {
                VectorIndexState::Backfilling(state) => Ok(IndexStateVariant::Backfilling {
                    num_segments: state.segments.len(),
                }),
                VectorIndexState::Backfilled { snapshot, .. }
                | VectorIndexState::SnapshottedAt(snapshot) => {
                    let VectorIndexSnapshotData::MultiSegment(segments) = &snapshot.data else {
                        anyhow::bail!("Expected MultiSegment");
                    };
                    Ok(IndexStateVariant::Backfilled {
                        num_segments: segments.len(),
                    })
                },
            }
        }

        async fn segment_stats(&self) -> anyhow::Result<Vec<SegmentInfo>> {
            let state = {
                let mut tx = self.database.begin_system().await?;
                let configs: Vec<_> = IndexModel::new(&mut tx)
                    .get_all_indexes()?
                    .filter_map(|idx| {
                        if let IndexConfig::Vector { on_disk_state, .. } = &idx.config {
                            Some(on_disk_state.clone())
                        } else {
                            None
                        }
                    })
                    .collect();
                assert_eq!(configs.len(), 1);
                configs.into_iter().next().unwrap()
            };

            let segments = match state {
                VectorIndexState::Backfilling(s) => s.segments,
                VectorIndexState::Backfilled { snapshot, .. }
                | VectorIndexState::SnapshottedAt(snapshot) => {
                    let VectorIndexSnapshotData::MultiSegment(segments) = snapshot.data else {
                        anyhow::bail!("Expected MultiSegment");
                    };
                    segments
                },
            };
            Ok(segments
                .into_iter()
                .map(|s| SegmentInfo {
                    num_documents: s.num_vectors as u64,
                    num_deleted: s.num_deleted as u64,
                })
                .collect())
        }

        async fn enable_index(&self) -> anyhow::Result<()> {
            let mut tx = self.database.begin_system().await?;
            IndexModel::new(&mut tx)
                .enable_index_for_testing(TableNamespace::test_user(), &INDEX_NAME.parse()?)
                .await?;
            self.database.commit(tx).await?;
            Ok(())
        }

        async fn assert_documents_searchable(
            &self,
            expected_ids: &[DeveloperDocumentId],
        ) -> anyhow::Result<()> {
            use common::components::ComponentId;
            use maplit::btreeset;
            use vector::VectorSearch;

            let (results, _) = self
                .database
                .vector_search(
                    Identity::system(),
                    VectorSearch {
                        index_name: INDEX_NAME.parse()?,
                        component_id: ComponentId::Root,
                        vector: vec![0.; DIMENSIONS as usize],
                        limit: Some(256),
                        expressions: btreeset![],
                    },
                )
                .await?;
            let mut result_ids: Vec<DeveloperDocumentId> =
                results.into_iter().map(|r| r.id).collect();
            result_ids.sort();
            let mut expected: Vec<_> = expected_ids.to_vec();
            expected.sort();
            assert_eq!(result_ids.len(), expected.len());
            assert_eq!(result_ids, expected);
            Ok(())
        }
    }

    #[convex_macro::test_runtime]
    async fn test_vector_backfill_creates_segments_incrementally(
        rt: TestRuntime,
    ) -> anyhow::Result<()> {
        super::test_backfill_creates_segments_incrementally::<VectorSearchHarness>(rt).await
    }

    #[convex_macro::test_runtime]
    async fn test_vector_backfill_still_backfilling_after_partial(
        rt: TestRuntime,
    ) -> anyhow::Result<()> {
        super::test_backfill_still_backfilling_after_partial::<VectorSearchHarness>(rt).await
    }

    #[convex_macro::test_runtime]
    async fn test_vector_backfill_handles_deletes(rt: TestRuntime) -> anyhow::Result<()> {
        super::test_backfill_handles_deletes::<VectorSearchHarness>(rt).await
    }

    #[convex_macro::test_runtime]
    async fn test_vector_backfill_handles_replaces(rt: TestRuntime) -> anyhow::Result<()> {
        super::test_backfill_handles_replaces::<VectorSearchHarness>(rt).await
    }

    #[convex_macro::test_runtime]
    async fn test_vector_backfill_skips_past_revisions(rt: TestRuntime) -> anyhow::Result<()> {
        super::test_backfill_skips_past_revisions::<VectorSearchHarness>(rt).await
    }

    #[convex_macro::test_runtime]
    async fn test_vector_backfill_captures_new_writes(rt: TestRuntime) -> anyhow::Result<()> {
        super::test_backfill_captures_new_writes::<VectorSearchHarness>(rt).await
    }
}

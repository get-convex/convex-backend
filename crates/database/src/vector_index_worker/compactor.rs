use std::{
    collections::BTreeMap,
    sync::Arc,
};

use anyhow::Context;
use common::{
    bootstrap_model::index::{
        vector_index::{
            DeveloperVectorIndexConfig,
            FragmentedVectorSegment,
            VectorDimensions,
            VectorIndexBackfillState,
            VectorIndexSnapshot,
            VectorIndexSnapshotData,
            VectorIndexState,
        },
        IndexConfig,
        IndexMetadata,
    },
    knobs::{
        MAX_SEGMENT_DELETED_PERCENTAGE,
        MIN_COMPACTION_SEGMENTS,
        SEARCH_WORKER_PASSIVE_PAGES_PER_SECOND,
        SEGMENT_MAX_SIZE_BYTES,
        VECTOR_INDEX_SIZE_HARD_LIMIT,
    },
    runtime::Runtime,
    types::TabletIndexName,
};
use itertools::Itertools;
use keybroker::Identity;
use search::searcher::Searcher;
use storage::Storage;
use value::ResolvedDocumentId;

use super::writer::VectorMetadataWriter;
use crate::{
    metrics::vector::{
        finish_compaction_timer,
        log_vector_compaction_compacted_segment_num_vectors_total,
        log_vector_compaction_total_segments,
        vector_compaction_build_one_timer,
        CompactionReason,
    },
    Database,
    IndexModel,
    Token,
};

pub struct VectorIndexCompactor<RT: Runtime> {
    database: Database<RT>,
    searcher: Arc<dyn Searcher>,
    search_storage: Arc<dyn Storage>,
    config: CompactionConfig,
    writer: VectorMetadataWriter<RT>,
}

impl<RT: Runtime> VectorIndexCompactor<RT> {
    pub(crate) fn new(
        database: Database<RT>,
        searcher: Arc<dyn Searcher>,
        search_storage: Arc<dyn Storage>,
        config: CompactionConfig,
        writer: VectorMetadataWriter<RT>,
    ) -> Self {
        Self {
            database,
            searcher,
            search_storage,
            config,
            writer,
        }
    }

    #[cfg(any(test, feature = "testing"))]
    pub(crate) fn new_for_tests(
        runtime: RT,
        database: Database<RT>,
        search_storage: Arc<dyn Storage>,
        searcher: Arc<dyn Searcher>,
        config: CompactionConfig,
    ) -> Self {
        let writer = VectorMetadataWriter::new(runtime, database.clone(), search_storage.clone());
        Self::new(database, searcher, search_storage.clone(), config, writer)
    }

    #[cfg(any(test, feature = "testing"))]
    pub async fn process_all_in_test(
        runtime: RT,
        database: Database<RT>,
        search_storage: Arc<dyn Storage>,
        searcher: Arc<dyn Searcher>,
    ) -> anyhow::Result<()> {
        let compactor = Self::new_for_tests(
            runtime,
            database,
            search_storage,
            searcher,
            CompactionConfig::default(),
        );
        compactor.step().await?;
        Ok(())
    }

    pub(crate) async fn step(&self) -> anyhow::Result<(BTreeMap<TabletIndexName, u32>, Token)> {
        let mut metrics: BTreeMap<TabletIndexName, u32> = BTreeMap::new();

        let (to_build, token) = self.needs_compaction().await?;
        let num_to_build = to_build.len();
        if num_to_build > 0 {
            tracing::info!("{num_to_build} vector indexes to build");
        }

        for job in to_build {
            let index_name = job.index_name.clone();
            let total_segments_compacted = self.build_one(job).await?;
            metrics.insert(index_name, total_segments_compacted);
        }

        if num_to_build > 0 {
            tracing::info!("built {num_to_build} vector indexes");
        }

        Ok((metrics, token))
    }

    async fn needs_compaction(&self) -> anyhow::Result<(Vec<VectorCompaction>, Token)> {
        let mut to_build = vec![];
        let mut tx = self.database.begin(Identity::system()).await?;

        for index_doc in IndexModel::new(&mut tx).get_all_indexes().await? {
            let (index_id, index_metadata) = index_doc.into_id_and_value();
            let IndexMetadata {
                name,
                config:
                    IndexConfig::Vector {
                        on_disk_state,
                        developer_config,
                    },
            } = index_metadata
            else {
                continue;
            };
            let needs_compaction = match &on_disk_state {
                VectorIndexState::Backfilling(VectorIndexBackfillState {
                    segments,
                    backfill_snapshot_ts,
                    ..
                }) => {
                    if backfill_snapshot_ts.is_none() {
                        false
                    } else {
                        Self::segments_need_compaction(segments, &developer_config, &self.config)?
                    }
                },
                VectorIndexState::SnapshottedAt(VectorIndexSnapshot {
                    data: VectorIndexSnapshotData::MultiSegment(segments),
                    ..
                })
                | VectorIndexState::Backfilled(VectorIndexSnapshot {
                    data: VectorIndexSnapshotData::MultiSegment(segments),
                    ..
                }) => Self::segments_need_compaction(segments, &developer_config, &self.config)?,
                _ => false,
            };
            if needs_compaction {
                tracing::info!("Queing vector index for compaction: {name:?}");
                let job = VectorCompaction {
                    index_id,
                    index_name: name.clone(),
                    developer_config: developer_config.clone(),
                    on_disk_state,
                };
                to_build.push(job);
            }
        }
        Ok((to_build, tx.into_token()?))
    }

    async fn build_one(&self, job: VectorCompaction) -> anyhow::Result<u32> {
        let timer = vector_compaction_build_one_timer();
        let (segments, snapshot_ts) = match job.on_disk_state {
            VectorIndexState::Backfilling(VectorIndexBackfillState {
                segments,
                backfill_snapshot_ts,
                ..
            }) => {
                let ts = backfill_snapshot_ts
                    .context("Trying to compact backfilling segments with no backfill timestamp")?;
                (segments, ts)
            },
            VectorIndexState::Backfilled(snapshot) | VectorIndexState::SnapshottedAt(snapshot) => {
                let segments = match snapshot.data {
                    VectorIndexSnapshotData::Unknown(_) => {
                        anyhow::bail!("Trying to compact unknown vector snapshot")
                    },
                    VectorIndexSnapshotData::MultiSegment(segments) => segments,
                };
                (segments, snapshot.ts)
            },
        };

        let (segments_to_compact, reason) =
            Self::find_segments_to_compact(&segments, &job.developer_config, &self.config)?;
        anyhow::ensure!(segments_to_compact.len() > 0);

        let total_compacted_segments = segments_to_compact.len();
        log_vector_compaction_total_segments(total_compacted_segments);

        let new_segment = self
            .compact(job.developer_config.dimensions, &segments_to_compact)
            .await?;

        let total_vectors = new_segment.num_vectors - new_segment.num_deleted;
        log_vector_compaction_compacted_segment_num_vectors_total(total_vectors);

        self.writer
            .commit_compaction(
                job.index_id,
                job.index_name,
                snapshot_ts,
                segments_to_compact
                    .clone()
                    .into_iter()
                    .cloned()
                    .collect_vec(),
                new_segment.clone(),
                *SEARCH_WORKER_PASSIVE_PAGES_PER_SECOND,
            )
            .await?;
        let total_compacted_segments = total_compacted_segments as u32;

        tracing::debug!(
            "Compacted {:#?} segments to {:#?}",
            segments_to_compact
                .iter()
                .map(|segment| Self::format(segment, job.developer_config.dimensions))
                .collect::<anyhow::Result<Vec<_>>>()?,
            Self::format(&new_segment, job.developer_config.dimensions)?,
        );

        finish_compaction_timer(timer, reason);
        Ok(total_compacted_segments)
    }

    fn format(
        segment: &FragmentedVectorSegment,
        dimension: VectorDimensions,
    ) -> anyhow::Result<String> {
        Ok(format!(
            "(id: {}, size: {}, vectors: {}, deletes: {})",
            segment.id,
            segment.total_size_bytes(dimension)?,
            segment.num_vectors,
            segment.num_deleted
        ))
    }

    fn max_compactable_segments<'a>(
        segments: Vec<&'a FragmentedVectorSegment>,
        dimensions: VectorDimensions,
        config: &CompactionConfig,
    ) -> anyhow::Result<Option<Vec<&'a FragmentedVectorSegment>>> {
        let mut size: u64 = 0;
        let segments = segments
            .into_iter()
            .sorted_by_key(|segment| segment.num_vectors)
            .map(|segment| Ok((segment, segment.total_size_bytes(dimensions)?)))
            .take_while(|segment| {
                let Ok((_, segment_size_bytes)) = segment else {
                    // Propagate the error to the outer collect.
                    return true;
                };
                // Some extra paranoia to fail loudly if we've misplaced some zeros somewhere.
                size = size
                    .checked_add(*segment_size_bytes)
                    .context("Overflowed size!")
                    .unwrap();
                size <= config.max_segment_size_bytes
            })
            .collect::<anyhow::Result<Vec<_>>>()?;
        if segments.len() >= config.min_compaction_segments {
            Ok(Some(
                segments
                    .into_iter()
                    .map(|(segment, _)| segment)
                    .collect_vec(),
            ))
        } else {
            Ok(None)
        }
    }

    fn find_segments_to_compact<'a>(
        segments: &'a Vec<FragmentedVectorSegment>,
        developer_config: &'a DeveloperVectorIndexConfig,
        config: &CompactionConfig,
    ) -> anyhow::Result<(Vec<&'a FragmentedVectorSegment>, CompactionReason)> {
        let dimensions = developer_config.dimensions;

        let segments_and_sizes = segments
            .iter()
            .map(|segment| Ok((segment, segment.total_size_bytes(dimensions)?)))
            .collect::<anyhow::Result<Vec<_>>>()?;

        let (small_segments, large_segments): (Vec<_>, Vec<_>) = segments_and_sizes
            .into_iter()
            .partition(|(_, segment_size_bytes)| {
                *segment_size_bytes <= config.small_segment_threshold_bytes
            });
        let small_segments = small_segments
            .into_iter()
            .map(|segments| segments.0)
            .collect_vec();

        // Compact small segments first because it's quick and reducing the total number
        // of segments helps us minimize query costs.
        let compact_small = Self::max_compactable_segments(small_segments, dimensions, config)?;
        if let Some(compact_small) = compact_small {
            return Ok((compact_small, CompactionReason::SmallSegments));
        }
        // Next check to see if we have too many larger segments and if so, compact
        // them.
        let compact_large = Self::max_compactable_segments(
            large_segments
                .clone()
                .into_iter()
                .map(|segment| segment.0)
                .collect_vec(),
            dimensions,
            config,
        )?;
        if let Some(compact_large) = compact_large {
            return Ok((compact_large, CompactionReason::LargeSegments));
        }

        // Finally check to see if any individual segment has a large number of deleted
        // vectors and if so compact just that segment.
        let compact_deletes = large_segments
            .into_iter()
            .find(|(segment, segment_size_bytes)| {
                segment.num_deleted as f64 / segment.num_vectors as f64
                         > config.max_deleted_percentage
                        // Below a certain size, don't bother to recompact on deletes because the
                        // whole segment will be compacted as a small segment in the future anyway.
                        && *segment_size_bytes > config.small_segment_threshold_bytes
            })
            .map(|(segment, _)| vec![segment]);
        if let Some(compact_deletes) = compact_deletes {
            return Ok((compact_deletes, CompactionReason::Deletes));
        }
        tracing::trace!(
            "Found no segments to compact, segments: {:#?}",
            segments
                .iter()
                .map(|segment| (&segment.id, segment.num_vectors, segment.num_deleted))
                .collect_vec()
        );
        Ok((vec![], CompactionReason::Unknown))
    }

    fn segments_need_compaction(
        segments: &Vec<FragmentedVectorSegment>,
        developer_config: &DeveloperVectorIndexConfig,
        config: &CompactionConfig,
    ) -> anyhow::Result<bool> {
        Ok(
            !Self::find_segments_to_compact(segments, developer_config, config)?
                .0
                .is_empty(),
        )
    }

    async fn compact(
        &self,
        dimension: VectorDimensions,
        segments: &Vec<&FragmentedVectorSegment>,
    ) -> anyhow::Result<FragmentedVectorSegment> {
        let total_segment_size_bytes: u64 = segments
            .iter()
            .map(|segment| segment.total_size_bytes(dimension))
            .try_fold(0u64, |sum, current| {
                current.and_then(|current| {
                    sum.checked_add(current)
                        .context("Summing vector sizes overflowed")
                })
            })?;
        anyhow::ensure!(
            total_segment_size_bytes <= self.config.max_segment_size_bytes,
            "Trying to compact {} segments with total size {} > our max size of {}, segments: {:?}",
            segments.len(),
            total_segment_size_bytes,
            self.config.max_segment_size_bytes,
            segments
                .iter()
                .map(|segment| Self::format(segment, dimension))
                .collect_vec(),
        );

        tracing::debug!(
            "Found {} segments to compact with an expected size of {}, max allowed size of {}, \
             segments: {:?}",
            segments.len(),
            total_segment_size_bytes,
            self.config.max_segment_size_bytes,
            segments
                .iter()
                .map(|segment| Self::format(segment, dimension))
                .collect::<anyhow::Result<Vec<_>>>()?,
        );

        let protos: Vec<pb::searchlight::FragmentedVectorSegmentPaths> = segments
            .iter()
            .cloned()
            .cloned()
            .map(|segment| segment.to_paths_proto())
            .collect::<anyhow::Result<Vec<_>>>()?;
        self.searcher
            .execute_vector_compaction(self.search_storage.clone(), protos, dimension.into())
            .await
    }
}

#[derive(Clone)]
pub(crate) struct CompactionConfig {
    pub(crate) max_deleted_percentage: f64,
    pub(crate) small_segment_threshold_bytes: u64,
    // Don't allow compacting fewer than N segments. For example, we have N large segments, but
    // only 2 of those can actually be compacted due to the max size restriction, then we don't
    // want to compact yet.
    pub(crate) min_compaction_segments: usize,
    // 1.1 million vectors at the largest size
    pub(crate) max_segment_size_bytes: u64,
}

impl Default for CompactionConfig {
    fn default() -> Self {
        Self {
            max_deleted_percentage: *MAX_SEGMENT_DELETED_PERCENTAGE,
            // Always treat segments created by our index compactor as
            // "small".
            small_segment_threshold_bytes: *VECTOR_INDEX_SIZE_HARD_LIMIT as u64,
            min_compaction_segments: *MIN_COMPACTION_SEGMENTS,
            max_segment_size_bytes: *SEGMENT_MAX_SIZE_BYTES,
        }
    }
}

struct VectorCompaction {
    index_id: ResolvedDocumentId,
    index_name: TabletIndexName,
    developer_config: DeveloperVectorIndexConfig,
    on_disk_state: VectorIndexState,
}

#[derive(PartialEq)]
pub enum MergeResult {
    Success,
    TypeChanged,
    StartedBackfilling,
    VersionChanged,
    SegmentsRemoved,
    IndexRemoved,
    DeveloperConfigChanged,
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
            btreemap! { index_data.resolved_index_name => min_compaction_segments as u32}
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
            btreemap! { index_data.resolved_index_name => min_compaction_segments as u32}
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
            btreemap! { index_data.resolved_index_name => min_compaction_segments as u32}
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
        let min_compaction_segments = config.min_compaction_segments as u64;
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
        let min_compaction_segments = config.min_compaction_segments as u64;
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
            btreemap! { index_data.resolved_index_name => min_compaction_segments as u32 }
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
            max_segment_size_bytes: min_compaction_segments as u64 * VECTOR_SIZE_BYTES,
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
            btreemap! { index_data.resolved_index_name => min_compaction_segments as u32}
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
            max_segment_size_bytes: min_compaction_segments as u64 * VECTOR_SIZE_BYTES,
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
            btreemap! { index_data.resolved_index_name => min_compaction_segments as u32}
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
            UserFacingModel::new(&mut tx).delete((*id).into()).await?;
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
            UserFacingModel::new(&mut tx).delete((*id).into()).await?;
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
            btreemap! { index_data.resolved_index_name => min_compaction_segments as u32}
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

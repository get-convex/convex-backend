use std::{
    collections::BTreeMap,
    sync::Arc,
};

use anyhow::Context;
use common::{
    knobs::{
        MAX_COMPACTION_SEGMENTS,
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
use rand::seq::SliceRandom;
use search::{
    metrics::SearchType,
    Searcher,
};
use storage::Storage;
use tokio::task;
use value::ResolvedDocumentId;

use crate::{
    index_workers::{
        index_meta::{
            BackfillState,
            SearchIndex,
            SearchOnDiskState,
            SearchSnapshot,
            SegmentStatistics,
            SegmentType,
            SnapshotData,
        },
        writer::SearchIndexMetadataWriter,
    },
    metrics::{
        compaction_build_one_timer,
        log_compaction_compacted_segment_num_documents_total,
        log_compaction_total_segments,
        CompactionReason,
    },
    Database,
    IndexModel,
    Token,
};

pub struct SearchIndexCompactor<RT: Runtime, T: SearchIndex> {
    database: Database<RT>,
    searcher: Arc<dyn Searcher>,
    search_storage: Arc<dyn Storage>,
    config: CompactionConfig,
    writer: SearchIndexMetadataWriter<RT, T>,
}

impl<RT: Runtime, T: SearchIndex> SearchIndexCompactor<RT, T> {
    pub(crate) fn new(
        database: Database<RT>,
        searcher: Arc<dyn Searcher>,
        search_storage: Arc<dyn Storage>,
        config: CompactionConfig,
        writer: SearchIndexMetadataWriter<RT, T>,
    ) -> SearchIndexCompactor<RT, T> {
        SearchIndexCompactor {
            database,
            searcher,
            search_storage,
            config,
            writer,
        }
    }

    fn search_type() -> SearchType {
        T::search_type()
    }

    pub(crate) async fn step(&self) -> anyhow::Result<(BTreeMap<TabletIndexName, u64>, Token)> {
        let mut metrics = BTreeMap::new();

        let (to_build, token) = self.needs_compaction().await?;
        let num_to_build = to_build.len();
        if num_to_build > 0 {
            tracing::info!("{num_to_build} {:?} indexes to build", Self::search_type());
        }

        for job in to_build {
            task::consume_budget().await;

            let index_name = job.index_name.clone();
            let total_segments_compacted = self.build_one(job).await?;
            metrics.insert(index_name, total_segments_compacted);
        }

        if num_to_build > 0 {
            tracing::info!("built {num_to_build} {:?} indexes", Self::search_type());
        }

        Ok((metrics, token))
    }

    async fn needs_compaction(&self) -> anyhow::Result<(Vec<CompactionJob<T>>, Token)> {
        let mut to_build = vec![];
        let mut tx = self.database.begin(Identity::system()).await?;

        // Skip compaction on empty tables.
        for index_doc in IndexModel::new(&mut tx)
            .get_all_non_empty_search_indexes()
            .await?
        {
            let (index_id, index_metadata) = index_doc.into_id_and_value();
            let Some(config) = T::get_config(index_metadata.config) else {
                continue;
            };
            let name = index_metadata.name;

            let maybe_segments_to_compact = match &config.on_disk_state {
                SearchOnDiskState::Backfilling(BackfillState {
                    segments,
                    backfill_snapshot_ts,
                    ..
                }) => {
                    if backfill_snapshot_ts.is_none() {
                        continue;
                    } else {
                        Self::find_segments_to_compact(
                            segments,
                            &config.developer_config,
                            &self.config,
                        )?
                    }
                },
                SearchOnDiskState::SnapshottedAt(SearchSnapshot {
                    data: SnapshotData::MultiSegment(segments),
                    ..
                })
                | SearchOnDiskState::Backfilled(SearchSnapshot {
                    data: SnapshotData::MultiSegment(segments),
                    ..
                }) => Self::find_segments_to_compact(
                    segments,
                    &config.developer_config,
                    &self.config,
                )?,
                _ => continue,
            };
            if let Some((mut segments_to_compact, compaction_reason)) = maybe_segments_to_compact {
                tracing::info!(
                    "Queueing {:?} index for compaction: {name:?}",
                    Self::search_type()
                );
                // Choose segments to compact at random.
                segments_to_compact.shuffle(&mut rand::thread_rng());
                tracing::info!(
                    "Compacting {} segments out of {} that need compaction for reason: {:?}",
                    *MAX_COMPACTION_SEGMENTS,
                    segments_to_compact.len(),
                    compaction_reason,
                );
                segments_to_compact.truncate(*MAX_COMPACTION_SEGMENTS);
                let job = CompactionJob {
                    index_id,
                    index_name: name.clone(),
                    developer_config: config.developer_config.clone(),
                    on_disk_state: config.on_disk_state,
                    segments_to_compact,
                    compaction_reason,
                };
                to_build.push(job);
            }
        }
        Ok((to_build, tx.into_token()?))
    }

    async fn build_one(&self, job: CompactionJob<T>) -> anyhow::Result<u64> {
        let timer = compaction_build_one_timer(Self::search_type(), job.compaction_reason);
        let snapshot_ts = match job.on_disk_state {
            SearchOnDiskState::Backfilling(BackfillState {
                backfill_snapshot_ts,
                ..
            }) => backfill_snapshot_ts.with_context(|| {
                format!(
                    "Trying to compact backfilling {:?} segments with no backfill timestamp",
                    Self::search_type()
                )
            })?,
            SearchOnDiskState::Backfilled(snapshot)
            | SearchOnDiskState::SnapshottedAt(snapshot) => snapshot.ts,
        };

        let segments_to_compact = job.segments_to_compact;
        anyhow::ensure!(segments_to_compact.len() > 0);

        let total_compacted_segments = segments_to_compact.len();
        log_compaction_total_segments(total_compacted_segments, Self::search_type());

        let new_segment = self
            .compact(&job.developer_config, segments_to_compact.clone())
            .await?;
        let stats = new_segment.statistics()?;

        let total_documents = stats.num_documents();
        log_compaction_compacted_segment_num_documents_total(total_documents, Self::search_type());

        self.writer
            .commit_compaction(
                job.index_id,
                job.index_name,
                snapshot_ts,
                segments_to_compact.clone(),
                new_segment.clone(),
                *SEARCH_WORKER_PASSIVE_PAGES_PER_SECOND,
                T::new_schema(&job.developer_config),
            )
            .await?;
        let total_compacted_segments = total_compacted_segments as u64;

        tracing::debug!(
            "Compacted {:#?} segments to {:#?}",
            segments_to_compact
                .iter()
                .map(|segment| Self::format(segment, &job.developer_config))
                .collect::<anyhow::Result<Vec<_>>>()?,
            Self::format(&new_segment, &job.developer_config)?,
        );
        timer.finish();
        Ok(total_compacted_segments)
    }

    fn format(
        segment: &T::Segment,
        developer_config: &T::DeveloperConfig,
    ) -> anyhow::Result<String> {
        let stats = segment.statistics()?;
        Ok(format!(
            "(id: {}, size: {}, documents : {}, deletes: {}, type: {:?})",
            segment.id(),
            segment.total_size_bytes(developer_config)?,
            stats.num_documents(),
            stats.num_deleted_documents(),
            Self::search_type(),
        ))
    }

    fn get_compactable_segments<'a>(
        segments: Vec<&'a T::Segment>,
        developer_config: &T::DeveloperConfig,
        compaction_config: &CompactionConfig,
    ) -> anyhow::Result<Option<Vec<&'a T::Segment>>> {
        let mut total_size: u64 = 0;
        let segments_with_size: Vec<_> = segments
            .into_iter()
            .map(|segment| anyhow::Ok((segment, segment.total_size_bytes(developer_config)?)))
            .try_collect()?;
        // Sort segments in ascending size order and take as many as we can fit in the
        // max segment
        let segments = segments_with_size
            .into_iter()
            .sorted_by_key(|(_, size)| *size)
            .take_while(|(_segment, segment_size_bytes)| {
                // Some extra paranoia to fail loudly if we've misplaced some zeros somewhere.
                total_size = total_size
                    .checked_add(*segment_size_bytes)
                    .context("Overflowed size!")
                    .unwrap();
                total_size <= compaction_config.max_segment_size_bytes
            })
            .collect::<Vec<_>>();
        if segments.len() >= compaction_config.min_compaction_segments as usize {
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
        segments: &'a Vec<T::Segment>,
        developer_config: &'a T::DeveloperConfig,
        compaction_config: &CompactionConfig,
    ) -> anyhow::Result<Option<(Vec<T::Segment>, CompactionReason)>> {
        fn to_owned<R: Clone>(borrowed: Vec<&R>) -> Vec<R> {
            borrowed.into_iter().cloned().collect_vec()
        }

        let segments_and_sizes = segments
            .iter()
            .map(|segment| Ok((segment, segment.total_size_bytes(developer_config)?)))
            .collect::<anyhow::Result<Vec<_>>>()?;

        let (small_segments, large_segments): (Vec<_>, Vec<_>) = segments_and_sizes
            .into_iter()
            .partition(|(_, segment_size_bytes)| {
                *segment_size_bytes <= compaction_config.small_segment_threshold_bytes
            });
        let small_segments = small_segments
            .into_iter()
            .map(|segments| segments.0)
            .collect_vec();

        // Compact small segments first because it's quick and reducing the total number
        // of segments helps us minimize query costs.
        let compact_small =
            Self::get_compactable_segments(small_segments, developer_config, compaction_config)?;
        if let Some(compact_small) = compact_small {
            return Ok(Some((
                to_owned(compact_small),
                CompactionReason::SmallSegments,
            )));
        }
        // Next check to see if we have too many larger segments and if so, compact
        // them.
        let compact_large = Self::get_compactable_segments(
            large_segments
                .clone()
                .into_iter()
                .map(|segment| segment.0)
                .collect_vec(),
            developer_config,
            compaction_config,
        )?;
        if let Some(compact_large) = compact_large {
            return Ok(Some((
                to_owned(compact_large),
                CompactionReason::LargeSegments,
            )));
        }

        // Finally check to see if any individual segment has a large number of deleted
        // documents and if so compact just that segment.
        let compact_deletes = large_segments
            .into_iter()
            .try_find(|(segment, segment_size_bytes)| {
                let stats = segment.statistics()?;
                let result: anyhow::Result<bool> = Ok(
                    stats.num_deleted_documents() as f64 / stats.num_documents() as f64
                        > compaction_config.max_deleted_percentage
                        // Below a certain size, don't bother to recompact on deletes because the
                        // whole segment will be compacted as a small segment in the future anyway.
                        && *segment_size_bytes > compaction_config.small_segment_threshold_bytes,
                );
                result
            })?
            .map(|(segment, _)| vec![segment]);
        if let Some(compact_deletes) = compact_deletes {
            return Ok(Some((to_owned(compact_deletes), CompactionReason::Deletes)));
        }
        tracing::trace!(
            "Found no segments to compact, segments: {:#?}",
            segments
                .iter()
                .map(|segment| {
                    // Avoid throwing while logging...
                    let stats = segment.statistics().unwrap_or_default();
                    (
                        segment.id(),
                        stats.num_documents(),
                        stats.num_deleted_documents(),
                    )
                })
                .collect_vec()
        );
        Ok(None)
    }

    async fn compact(
        &self,
        developer_config: &T::DeveloperConfig,
        segments: Vec<T::Segment>,
    ) -> anyhow::Result<T::Segment> {
        let total_segment_size_bytes: u64 = segments
            .iter()
            .map(|segment| segment.total_size_bytes(developer_config))
            .try_fold(0u64, |sum, current| {
                current.and_then(|current| {
                    sum.checked_add(current)
                        .context("Summing document sizes overflowed")
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
                .map(|segment| Self::format(segment, developer_config))
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
                .map(|segment| Self::format(segment, developer_config))
                .collect::<anyhow::Result<Vec<_>>>()?,
        );

        T::execute_compaction(
            self.searcher.clone(),
            self.search_storage.clone(),
            developer_config,
            segments,
        )
        .await
    }
}

#[derive(Clone)]
pub struct CompactionConfig {
    pub max_deleted_percentage: f64,
    pub small_segment_threshold_bytes: u64,
    // Don't allow compacting fewer than N segments. For example, we have N large segments, but
    // only 2 of those can actually be compacted due to the max size restriction, then we don't
    // want to compact yet.
    pub min_compaction_segments: u64,
    pub max_segment_size_bytes: u64,
}

// TODO(sam): These defaults are reasonable for vector, but maybe not text.
// Remove this Default impl and replace it with variants for text an vector
// search.
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

struct CompactionJob<T: SearchIndex> {
    index_id: ResolvedDocumentId,
    index_name: TabletIndexName,
    developer_config: T::DeveloperConfig,
    on_disk_state: SearchOnDiskState<T>,
    segments_to_compact: Vec<T::Segment>,
    compaction_reason: CompactionReason,
}

use std::{
    collections::BinaryHeap,
    path::Path,
    sync::Arc,
    time::{
        Duration,
        Instant,
    },
};

use anyhow::Context;
use async_trait::async_trait;
use bytesize::ByteSize;
use common::{
    bounded_thread_pool::BoundedThreadPool,
    runtime::Runtime,
    types::ObjectKey,
};
use futures::TryStreamExt;
use pb::searchlight::{
    FragmentedVectorSegmentPaths,
    StorageKey,
};
use storage::Storage;
use vector::{
    qdrant_segments::UntarredDiskSegmentPaths,
    CompiledVectorSearch,
    QdrantSchema,
    VectorIndexType,
    VectorSearchQueryResult,
    VectorSearcher,
};

use super::{
    metrics::{
        self,
        vector_compaction_prefetch_timer,
    },
    segment_cache::{
        SegmentCache,
        SizedSegment,
    },
};
use crate::{
    archive::cache::ArchiveCacheManager,
    disk_index::index_reader_for_directory,
    fragmented_segment::{
        FragmentedSegmentCompactor,
        FragmentedSegmentFetcher,
        FragmentedSegmentPrefetcher,
        FragmentedSegmentStorageKeys,
    },
    query::{
        CompiledQuery,
        TermShortlist,
    },
    scoring::Bm25StatisticsDiff,
    searcher::searchlight_knobs::{
        MAX_CONCURRENT_SEGMENT_COMPACTIONS,
        MAX_CONCURRENT_SEGMENT_FETCHES,
        MAX_CONCURRENT_VECTOR_SEARCHES,
        MAX_CONCURRENT_VECTOR_SEGMENT_PREFETCHES,
        MAX_VECTOR_LRU_SIZE,
        QUEUE_SIZE_MULTIPLIER,
    },
    SearchFileType,
    SearchQueryResult,
    TantivySearchIndexSchema,
};

#[async_trait]
pub trait Searcher: VectorSearcher + Send + Sync + 'static {
    async fn execute_query(
        &self,
        search_storage: Arc<dyn Storage>,
        disk_index: &ObjectKey,
        schema: &TantivySearchIndexSchema,
        search: CompiledQuery,
        memory_statistics_diff: Bm25StatisticsDiff,
        shortlisted_terms: TermShortlist,
        limit: usize,
    ) -> anyhow::Result<SearchQueryResult>;
}

pub struct SearcherImpl<RT: Runtime> {
    pub(crate) archive_cache: ArchiveCacheManager<RT>,
    segment_cache: SegmentCache<RT>,
    // A small thread pool whose size is aimed at capping the maximum memory size
    // from concurrent vector loads.
    vector_search_pool: BoundedThreadPool<RT>,
    // A much larger pool for text search where we assume the memory overhead of each query is
    // quite small.
    text_search_pool: BoundedThreadPool<RT>,
    slow_vector_query_threshold_millis: u64,
    require_exact_vector_search: bool,
    fragmented_segment_fetcher: FragmentedSegmentFetcher<RT>,
    fragmented_segment_compactor: FragmentedSegmentCompactor<RT>,
    fragmented_segment_prefetcher: FragmentedSegmentPrefetcher<RT>,
}

impl<RT: Runtime> SearcherImpl<RT> {
    pub async fn new<P: AsRef<Path>>(
        local_storage_path: P,
        max_disk_cache_size: u64,
        slow_vector_query_threshold_millis: u64,
        require_exact_vector_search: bool,
        runtime: RT,
    ) -> anyhow::Result<Self> {
        tracing::info!(
            "Searchlight starting, local_storage_path: {} max_size: {}",
            local_storage_path.as_ref().display(),
            ByteSize(max_disk_cache_size)
        );
        // Tokio uses ~500 threads for blocking tasks
        let blocking_thread_pool = BoundedThreadPool::new(runtime.clone(), 1000, 50, "general");
        let archive_cache = ArchiveCacheManager::new(
            local_storage_path,
            max_disk_cache_size,
            blocking_thread_pool.clone(),
            // This actually sets total concurrency for all files in the pool,
            // but since segments are the largest and most critical to limit, this is a tolerable
            // maximum bound.
            *MAX_CONCURRENT_SEGMENT_FETCHES,
            runtime.clone(),
        )
        .await?;
        let vector_search_pool = BoundedThreadPool::new(
            runtime.clone(),
            *MAX_CONCURRENT_VECTOR_SEARCHES * *QUEUE_SIZE_MULTIPLIER,
            *MAX_CONCURRENT_VECTOR_SEARCHES,
            "vector",
        );
        let fragmented_segment_fetcher =
            FragmentedSegmentFetcher::new(archive_cache.clone(), blocking_thread_pool.clone());
        let fragmented_segment_compactor = FragmentedSegmentCompactor::new(
            runtime.clone(),
            fragmented_segment_fetcher.clone(),
            BoundedThreadPool::new(
                runtime.clone(),
                *MAX_CONCURRENT_SEGMENT_COMPACTIONS * *QUEUE_SIZE_MULTIPLIER,
                *MAX_CONCURRENT_SEGMENT_COMPACTIONS,
                "vector_compactor",
            ),
        );
        let fragmented_segment_prefetcher = FragmentedSegmentPrefetcher::new(
            runtime.clone(),
            fragmented_segment_fetcher.clone(),
            *MAX_CONCURRENT_VECTOR_SEGMENT_PREFETCHES,
        );
        Ok(Self {
            archive_cache,
            segment_cache: SegmentCache::new(
                runtime,
                *MAX_VECTOR_LRU_SIZE,
                vector_search_pool.clone(),
                *MAX_CONCURRENT_VECTOR_SEARCHES,
            ),
            vector_search_pool,
            text_search_pool: blocking_thread_pool,
            slow_vector_query_threshold_millis,
            require_exact_vector_search,
            fragmented_segment_fetcher,
            fragmented_segment_compactor,
            fragmented_segment_prefetcher,
        })
    }

    pub fn queue_prefetch_segments(
        &self,
        search_storage: Arc<dyn Storage>,
        paths: Vec<FragmentedVectorSegmentPaths>,
    ) -> anyhow::Result<()> {
        let paths: Vec<FragmentedSegmentStorageKeys> = paths
            .into_iter()
            .map(|paths| paths.try_into())
            .try_collect()?;
        self.fragmented_segment_prefetcher
            .queue_prefetch(search_storage, paths)
    }

    /// A blocking prefetch for compaction where we explicitly want to stop the
    /// compaction process until the prefetch finishes.
    ///
    /// In contrast queue_prefetch_segments is intended for cases where we
    /// explicitly do not want to block the calling process and instead want the
    /// prefetches to finish asynchronously after the request completes.
    async fn prefetch_segment(
        &self,
        search_storage: Arc<dyn Storage>,
        segment: common::bootstrap_model::index::vector_index::FragmentedVectorSegment,
    ) -> anyhow::Result<()> {
        let timer = vector_compaction_prefetch_timer();
        let paths = FragmentedSegmentStorageKeys {
            segment: segment.segment_key,
            id_tracker: segment.id_tracker_key,
            deleted_bitset: segment.deleted_bitset_key,
        };
        self.fragmented_segment_fetcher
            .stream_fetch_fragmented_segments(search_storage, vec![paths])
            .try_collect::<Vec<_>>()
            .await?;
        timer.finish();
        Ok(())
    }

    async fn load_fragmented_segment(
        &self,
        paths: UntarredDiskSegmentPaths,
    ) -> anyhow::Result<Arc<SizedSegment>> {
        self.segment_cache.get(paths).await
    }

    async fn vector_query_segment(
        &self,
        schema: QdrantSchema,
        query: CompiledVectorSearch,
        overfetch_delta: u32,
        segment: Arc<SizedSegment>,
    ) -> anyhow::Result<Vec<VectorSearchQueryResult>> {
        let slow_query_threshold = self.slow_vector_query_threshold_millis;
        let require_exact = self.require_exact_vector_search;
        let search = move || {
            let timer = metrics::vector_schema_query_timer();
            let start = Instant::now();
            let result = schema.search(
                &segment,
                &query,
                overfetch_delta,
                slow_query_threshold,
                require_exact,
            );
            let query_duration = Instant::now().duration_since(start);
            if query_duration > Duration::from_millis(slow_query_threshold) {
                tracing::warn!(
                    "Slow vector query, duration: {}ms, results: {:?} schema: {:?}, query: {:?}, \
                     overfetch_delta: {overfetch_delta}",
                    query_duration.as_millis(),
                    result.as_ref().map(|value| value.len()),
                    schema,
                    query,
                )
            }
            timer.finish();
            result
        };
        self.vector_search_pool.execute(search).await?
    }
}

#[async_trait]
impl<RT: Runtime> Searcher for SearcherImpl<RT> {
    async fn execute_query(
        &self,
        search_storage: Arc<dyn Storage>,
        disk_index: &ObjectKey,
        schema: &TantivySearchIndexSchema,
        compiled_query: CompiledQuery,
        memory_statistics_diff: Bm25StatisticsDiff,
        memory_shortlisted_terms: TermShortlist,
        limit: usize,
    ) -> anyhow::Result<SearchQueryResult> {
        // Fetch disk index and perform query
        let timer = metrics::query_timer();
        let archive_path = self
            .archive_cache
            .get(search_storage, disk_index, SearchFileType::Text)
            .await?;
        let search_field = schema.search_field;
        let query = move || {
            let reader = index_reader_for_directory(&archive_path)?;
            let searcher = reader.searcher();
            let results = crate::tantivy_query::query_tantivy(
                search_field,
                &compiled_query,
                &searcher,
                memory_statistics_diff,
                memory_shortlisted_terms,
                limit,
            )?;
            Ok::<SearchQueryResult, anyhow::Error>(results)
        };
        let results = self.text_search_pool.execute(query).await??;
        timer.finish();
        Ok(results)
    }
}

#[async_trait]
impl<RT: Runtime> VectorSearcher for SearcherImpl<RT> {
    async fn execute_multi_segment_vector_query(
        &self,
        search_storage: Arc<dyn Storage>,
        fragments: Vec<FragmentedVectorSegmentPaths>,
        schema: QdrantSchema,
        query: CompiledVectorSearch,
        overfetch_delta: u32,
    ) -> anyhow::Result<Vec<VectorSearchQueryResult>> {
        let timer = metrics::vector_query_timer(VectorIndexType::MultiSegment);
        let results: anyhow::Result<Vec<VectorSearchQueryResult>> = try {
            let total_segments = fragments.len();
            let query_capacity = (query.limit + overfetch_delta) as usize;

            // Store results in a min-heap to avoid storing all intermediate
            // results from parallel segment fetches in-memory
            let results_pq = self
                .fragmented_segment_fetcher
                .stream_fetch_fragmented_segments(search_storage, fragments)
                .and_then(|paths| self.load_fragmented_segment(paths))
                .and_then(|segment| {
                    self.vector_query_segment(
                        schema.clone(),
                        query.clone(),
                        overfetch_delta,
                        segment,
                    )
                })
                .try_fold(
                    BinaryHeap::with_capacity(query_capacity + 1),
                    |mut acc_pq, results| async {
                        for result in results {
                            // Store Reverse(result) in the heap so that the heap becomes a min-heap
                            // instead of the default max-heap. This way, we can evict the smallest
                            // element in the heap efficiently once we've
                            // reached query_capacity, leaving us with the top K
                            // results.
                            acc_pq.push(std::cmp::Reverse(result));
                            if acc_pq.len() > query_capacity {
                                acc_pq.pop();
                            }
                        }
                        Ok(acc_pq)
                    },
                )
                .await?;

            // BinaryHeap::into_sorted_vec returns results in ascending order of score,
            // but this is a Vec<Reverse<_>>, so the order is already descending, as
            // desired.
            // Note: this already contains at most query_capacity = query.limit +
            // overfetch_delta results, so no more filtering required.
            let results: Vec<VectorSearchQueryResult> = results_pq
                .into_sorted_vec()
                .into_iter()
                .map(|v| v.0)
                .collect();
            tracing::debug!(
                "Finished querying {} vectors from {total_segments} segment(s) to get {} limit + \
                 overfetch results",
                results.len(),
                query.limit + overfetch_delta
            );
            results
        };

        timer.finish(results.is_ok());
        results
    }

    async fn execute_vector_compaction(
        &self,
        search_storage: Arc<dyn Storage>,
        segments: Vec<FragmentedVectorSegmentPaths>,
        dimension: usize,
    ) -> anyhow::Result<common::bootstrap_model::index::vector_index::FragmentedVectorSegment> {
        let segment = self
            .fragmented_segment_compactor
            .compact(segments, dimension, search_storage.clone())
            .await?;

        self.prefetch_segment(search_storage, segment.clone())
            .await?;
        Ok(segment)
    }
}

impl TryFrom<FragmentedVectorSegmentPaths> for FragmentedSegmentStorageKeys {
    type Error = anyhow::Error;

    fn try_from(value: FragmentedVectorSegmentPaths) -> Result<Self, Self::Error> {
        let from_path = |path: Option<StorageKey>| {
            path.map(|p| p.storage_key)
                .context("Missing path!")?
                .try_into()
        };
        Ok(FragmentedSegmentStorageKeys {
            segment: from_path(value.segment)?,
            id_tracker: from_path(value.id_tracker)?,
            deleted_bitset: from_path(value.deleted_bitset)?,
        })
    }
}

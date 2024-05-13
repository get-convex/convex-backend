use std::{
    cmp::Ordering,
    collections::{
        BTreeMap,
        BTreeSet,
        BinaryHeap,
    },
    ops::{
        Add,
        AddAssign,
    },
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
    document::CreationTime,
    id_tracker::StaticIdTracker,
    runtime::Runtime,
    types::{
        ObjectKey,
        Timestamp,
        WriteTimestamp,
    },
};
use futures::{
    try_join,
    TryStreamExt,
};
use itertools::Itertools;
use pb::searchlight::{
    FragmentedTextSegmentPaths,
    FragmentedVectorSegmentPaths,
    QueryBm25StatsResponse,
    StorageKey,
};
use storage::Storage;
pub use tantivy::Term;
use tantivy::{
    collector::{
        Collector,
        TopDocs,
    },
    query::{
        Bm25StatisticsProvider,
        EnableScoring,
    },
    schema::Field,
    SegmentReader,
};
use text_search::tracker::{
    load_alive_bitset,
    StaticDeletionTracker,
};
use value::InternalId;
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
    aggregation::TokenMatchAggregator,
    archive::cache::ArchiveCacheManager,
    constants::{
        MAX_EDIT_DISTANCE,
        MAX_UNIQUE_QUERY_TERMS,
    },
    convex_query::{
        AliveDocuments,
        ConvexSearchQuery,
        OrTerm,
    },
    disk_index::index_reader_for_directory,
    fragmented_segment::{
        FragmentedSegmentCompactor,
        FragmentedSegmentFetcher,
        FragmentedSegmentPrefetcher,
        FragmentedSegmentStorageKeys,
    },
    levenshtein_dfa::{
        build_fuzzy_dfa,
        LevenshteinDfaWrapper,
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
    CREATION_TIME_FIELD_NAME,
    INTERNAL_ID_FIELD_NAME,
    TS_FIELD_NAME,
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

    async fn query_tokens(
        &self,
        search_storage: Arc<dyn Storage>,
        storage_keys: FragmentedTextSegmentStorageKeys,
        queries: Vec<TokenQuery>,
        max_results: usize,
    ) -> anyhow::Result<Vec<TokenMatch>>;

    async fn query_bm25_stats(
        &self,
        search_storage: Arc<dyn Storage>,
        storage_keys: FragmentedTextSegmentStorageKeys,
        terms: Vec<Term>,
    ) -> anyhow::Result<Bm25Stats>;

    async fn query_posting_lists(
        &self,
        search_storage: Arc<dyn Storage>,
        storage_keys: FragmentedTextSegmentStorageKeys,
        query: PostingListQuery,
    ) -> anyhow::Result<Vec<PostingListMatch>>;
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

    async fn query_tokens(
        &self,
        search_storage: Arc<dyn Storage>,
        storage_keys: FragmentedTextSegmentStorageKeys,
        queries: Vec<TokenQuery>,
        max_results: usize,
    ) -> anyhow::Result<Vec<TokenMatch>> {
        let (alive_bitset_path, deleted_terms_path) = try_join!(
            self.archive_cache.get(
                search_storage.clone(),
                &storage_keys.segment,
                SearchFileType::TextDeletedBitset
            ),
            self.archive_cache.get(
                search_storage.clone(),
                &storage_keys.segment,
                SearchFileType::TextDeletedTerms
            )
        )?;
        let query = move || {
            let reader = index_reader_for_directory(&alive_bitset_path)?;
            let searcher = reader.searcher();
            anyhow::ensure!(searcher.segment_readers().len() == 1);
            let segment = searcher.segment_reader(0);
            let alive_bitset = load_alive_bitset(&alive_bitset_path)?;
            let deletion_tracker = StaticDeletionTracker::load(alive_bitset, &deleted_terms_path)?;
            Self::query_tokens_impl(segment, &deletion_tracker, queries, max_results)
        };
        let resp = self.text_search_pool.execute(query).await??;
        Ok(resp)
    }

    async fn query_bm25_stats(
        &self,
        search_storage: Arc<dyn Storage>,
        storage_keys: FragmentedTextSegmentStorageKeys,
        terms: Vec<Term>,
    ) -> anyhow::Result<Bm25Stats> {
        let (archive_path, alive_bitset_path, deleted_terms_path) = try_join!(
            self.archive_cache.get(
                search_storage.clone(),
                &storage_keys.segment,
                SearchFileType::Text
            ),
            self.archive_cache.get(
                search_storage.clone(),
                &storage_keys.segment,
                SearchFileType::TextDeletedBitset
            ),
            self.archive_cache.get(
                search_storage.clone(),
                &storage_keys.segment,
                SearchFileType::TextDeletedTerms
            )
        )?;
        let query = move || {
            let reader = index_reader_for_directory(&archive_path)?;
            let searcher = reader.searcher();
            anyhow::ensure!(searcher.segment_readers().len() == 1);
            let segment = searcher.segment_reader(0);
            let alive_bitset = load_alive_bitset(&alive_bitset_path)?;
            let deletion_tracker = StaticDeletionTracker::load(alive_bitset, &deleted_terms_path)?;
            Self::query_bm25_stats_impl(segment, &deletion_tracker, terms)
        };
        let resp = self.text_search_pool.execute(query).await??;
        Ok(resp)
    }

    async fn query_posting_lists(
        &self,
        search_storage: Arc<dyn Storage>,
        storage_keys: FragmentedTextSegmentStorageKeys,
        query: PostingListQuery,
    ) -> anyhow::Result<Vec<PostingListMatch>> {
        let (archive_path, id_path, alive_bitset_path, deleted_terms_path) = try_join!(
            self.archive_cache.get(
                search_storage.clone(),
                &storage_keys.segment,
                SearchFileType::Text,
            ),
            self.archive_cache.get(
                search_storage.clone(),
                &storage_keys.segment,
                SearchFileType::TextIdTracker
            ),
            self.archive_cache.get(
                search_storage.clone(),
                &storage_keys.segment,
                SearchFileType::TextDeletedBitset
            ),
            self.archive_cache.get(
                search_storage.clone(),
                &storage_keys.segment,
                SearchFileType::TextDeletedTerms
            )
        )?;
        let query = move || {
            let reader = index_reader_for_directory(&archive_path)?;
            let searcher = reader.searcher();
            anyhow::ensure!(searcher.segment_readers().len() == 1);
            let segment = searcher.segment_reader(0);
            let alive_bitset = load_alive_bitset(&alive_bitset_path)?;
            let id_tracker = StaticIdTracker::load_from_path(id_path)?;
            let deleted_tracker = StaticDeletionTracker::load(alive_bitset, &deleted_terms_path)?;
            Self::query_posting_lists_impl(&searcher, segment, &id_tracker, &deleted_tracker, query)
        };
        let resp = self.text_search_pool.execute(query).await??;
        Ok(resp)
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

impl<RT: Runtime> SearcherImpl<RT> {
    fn query_tokens_impl(
        segment: &SegmentReader,
        deletion_tracker: &StaticDeletionTracker,
        queries: Vec<TokenQuery>,
        max_results: usize,
    ) -> anyhow::Result<Vec<TokenMatch>> {
        anyhow::ensure!(max_results <= MAX_UNIQUE_QUERY_TERMS);

        // The goal of this algorithm is to deterministically choose a set of terms
        // from our database that are the "best" matches for a given set of query
        // tokens. For two strings `q` and `t`, define their score to be the
        // better of fuzzy matching with and without prefix matching:
        // ```rust
        // fn score(q: &str, t: &str) -> (u32, bool) {
        //    let with_prefix_distance = levenshtein_distance_with_prefix(q, t);
        //    let without_prefix_distance = levenshtein_distance(q, t);
        //    if with_prefix_distance < without_prefix_distance {
        //        (with_prefix_distance, true)
        //    else {
        //        (without_prefix_distance, false)
        //    }
        // }
        //
        // // The levenshtein distance with prefix is defined as the minimum edit
        // // distance over all prefixes of the query string.
        // fn levenshtein_distance_with_prefix(q: &str, t: &str) -> u32 {
        //     q.prefixes().map(|prefix| levenshtein_distance(prefix, t)).min()
        // }
        // ```
        // Then, for each query token `q_i`, we can totally order all of the terms in
        // the database by sorting them by `(score(q_i, t_j), t_j, i)`,
        // breaking ties by the term contents and the query token index.
        // ```
        // q_i: (score(q_i, t_1), t_1, i), (score(q_i, t_2), t_2, i), ..., (score(q_i, t_n), t_n, i)
        // ```
        // Note that each query token `q_i` chooses a different order on our terms:
        // ```
        // q_0: (score(q_0, t_1), t_1, 0), (score(q_0, t_2), t_2, 0), ..., (score(q_0, t_n), t_n, 0)
        // q_1: (score(q_1, t_1), t_1, 1), (score(q_1, t_2), t_2, 1), ..., (score(q_1, t_n), t_n, 1)
        // ...
        // q_k: (score(q_k, t_1), t_1, k), (score(q_k, t_2), t_2, k), ..., (score(q_k, t_n), t_n, k)
        // ```
        // Logically, our algorithm merges these `k` streams, resorts them, and then
        // takes some number of the best values. Instead of taking the top
        // `max_results` values, we continue taking these tuples until we
        // have seen `max_results` unique terms.
        //
        // Since `n` may be very large, our implementation pushes down this sorting into
        // each query term. So, we take tuples from each `q_i` until we've seen
        // `max_results` unique terms (yielding at most `max_results * k` tuples), merge
        // and sort the results, and then take the best tuples until we have seen
        // `max_results` unique terms in the merged stream.
        let mut match_aggregator = TokenMatchAggregator::new(max_results);

        for (token_ord, token_query) in queries.into_iter().enumerate() {
            let token_ord = token_ord as u32;
            anyhow::ensure!(token_query.max_distance <= MAX_EDIT_DISTANCE);

            // Query the top scoring tuples for just our query term.
            Self::visit_top_terms_for_query(
                segment,
                deletion_tracker,
                token_ord,
                &token_query,
                &mut match_aggregator,
            )?;
        }

        Ok(match_aggregator.into_results())
    }

    fn visit_top_terms_for_query(
        segment: &SegmentReader,
        deletion_tracker: &StaticDeletionTracker,
        token_ord: u32,
        query: &TokenQuery,
        results: &mut TokenMatchAggregator,
    ) -> anyhow::Result<()> {
        let inverted_index = segment.inverted_index(query.term.field())?;
        let term_dict = inverted_index.terms();
        let mut seen_terms = BTreeSet::new();
        'query: for distance in [0, 1, 2] {
            for prefix in [false, true] {
                if distance > query.max_distance || (!query.prefix && prefix) {
                    continue;
                }
                if distance == 0 && !prefix {
                    if let Some(term_ord) = term_dict.term_ord(query.term.value_bytes())? {
                        if deletion_tracker.doc_frequency(term_dict, term_ord)? == 0 {
                            continue;
                        }
                        anyhow::ensure!(seen_terms.insert(query.term.clone()));
                        let m = TokenMatch {
                            distance,
                            prefix,
                            term: query.term.clone(),
                            token_ord,
                        };
                        if !results.insert(m) {
                            break 'query;
                        }
                    }
                } else {
                    let term_str = query
                        .term
                        .as_str()
                        .context("Non-exact match for non-string field")?;
                    let dfa = build_fuzzy_dfa(term_str, distance as u8, prefix);
                    let dfa_compat = LevenshteinDfaWrapper(&dfa);
                    let mut term_stream = term_dict.search(dfa_compat).into_stream()?;
                    while term_stream.advance() {
                        let match_term_bytes = term_stream.key();
                        let match_str = std::str::from_utf8(match_term_bytes)?;
                        let match_term = Term::from_field_text(query.term.field(), match_str);

                        let term_ord = term_stream.term_ord();
                        if deletion_tracker.doc_frequency(term_dict, term_ord)? == 0 {
                            continue;
                        }

                        // We need to skip terms we've already processed since we perform
                        // overlapping edit distance queries.
                        if seen_terms.contains(&match_term) {
                            continue;
                        }

                        // TODO: extend Tantivy::TermStreamer to a TermStreamerWithState to avoid
                        // recomputing distance again here.
                        // This comment on a Tantivy open issue describes how to approach this:
                        // https://github.com/quickwit-oss/tantivy/issues/563#issuecomment-801444469
                        // TODO: Ideally we could make DFAs that only match a particular
                        // edit distance so we don't have to skip duplicates above.
                        let match_distance = dfa.eval(match_term_bytes).to_u8() as u32;
                        if distance != match_distance {
                            continue;
                        }

                        seen_terms.insert(match_term.clone());
                        let m = TokenMatch {
                            distance,
                            prefix,
                            term: match_term,
                            token_ord,
                        };
                        if !results.insert(m) {
                            break 'query;
                        }
                    }
                }
            }
        }
        Ok(())
    }

    fn query_bm25_stats_impl(
        segment: &SegmentReader,
        deletion_tracker: &StaticDeletionTracker,
        terms: Vec<Term>,
    ) -> anyhow::Result<Bm25Stats> {
        let field = terms
            .iter()
            .map(|t| t.field())
            .dedup()
            .exactly_one()
            .map_err(|_| anyhow::anyhow!("All terms must be in the same field"))?;

        let inverted_index = segment.inverted_index(field)?;
        let term_dict = inverted_index.terms();
        let num_terms = inverted_index
            .total_num_tokens()
            .checked_sub(deletion_tracker.num_terms_deleted() as u64)
            .context("num_terms underflow")?;
        let num_documents = deletion_tracker.num_alive_docs() as u64;
        let mut doc_frequencies = BTreeMap::new();
        for term in terms {
            let Some(term_ord) = term_dict.term_ord(term.value_bytes())? else {
                anyhow::bail!("Term not found: {:?}", term);
            };
            let doc_freq = deletion_tracker.doc_frequency(term_dict, term_ord)?;
            doc_frequencies.insert(term, doc_freq);
        }
        let stats = Bm25Stats {
            num_terms,
            num_documents,
            doc_frequencies,
        };
        Ok(stats)
    }

    fn query_posting_lists_impl(
        searcher: &tantivy::Searcher,
        segment: &SegmentReader,
        id_tracker: &StaticIdTracker,
        deletion_tracker: &StaticDeletionTracker,
        query: PostingListQuery,
    ) -> anyhow::Result<Vec<PostingListMatch>> {
        let search_field = query
            .or_terms
            .iter()
            .map(|t| t.term.field())
            .dedup()
            .exactly_one()
            .map_err(|_| anyhow::anyhow!("All terms must be in the same field"))?;
        let stats_provider = StatsProvider {
            search_field,
            num_terms: query.num_terms,
            num_documents: query.num_documents,
            doc_frequencies: query
                .or_terms
                .iter()
                .map(|t| (t.term.clone(), t.doc_frequency))
                .collect(),
        };

        let mut memory_deleted = BTreeSet::new();
        for internal_id in query.deleted_internal_ids {
            let Some(doc_id) = id_tracker.lookup(internal_id.0) else {
                continue;
            };
            memory_deleted.insert(doc_id);
        }
        let deleted_documents = AliveDocuments {
            memory_deleted,
            segment_alive_bitset: deletion_tracker.alive_bitset().clone(),
        };
        let search_query =
            ConvexSearchQuery::new(query.or_terms, query.and_terms, deleted_documents);
        let enable_scoring =
            EnableScoring::enabled_from_statistics_provider(&stats_provider, searcher);
        let search_weight = search_query.weight(enable_scoring)?;

        let collector = TopDocs::with_limit(query.max_results);
        let segment_results = collector.collect_segment(&*search_weight, 0, segment)?;

        let fast_fields = segment.fast_fields();
        let internal_ids = fast_fields.bytes(INTERNAL_ID_FIELD_NAME)?;
        let timestamps = fast_fields.u64(TS_FIELD_NAME)?;
        let creation_times = fast_fields.f64(CREATION_TIME_FIELD_NAME)?;

        let mut results = Vec::with_capacity(segment_results.len());
        for (bm25_score, doc_address) in segment_results {
            let internal_id = internal_ids.get_bytes(doc_address.doc_id).try_into()?;
            let ts = Timestamp::try_from(timestamps.get_val(doc_address.doc_id))?;
            let creation_time = CreationTime::try_from(creation_times.get_val(doc_address.doc_id))?;
            let posting_list_match = PostingListMatch {
                internal_id,
                ts: WriteTimestamp::Committed(ts),
                creation_time,
                bm25_score,
            };
            results.push(posting_list_match);
        }
        anyhow::ensure!(results.len() <= query.max_results);

        // TODO: The collector sorts only on score, unlike PostingListMatchAggregator,
        // so we have to resort the results here, sweeping this nondeterminism
        // under the rug.
        results.sort_by(|a, b| a.cmp(b).reverse());

        Ok(results)
    }
}

struct StatsProvider {
    search_field: Field,

    num_terms: u64,
    num_documents: u64,
    doc_frequencies: BTreeMap<Term, u64>,
}

impl Bm25StatisticsProvider for StatsProvider {
    fn total_num_tokens(&self, field: Field) -> tantivy::Result<u64> {
        if field != self.search_field {
            return Err(tantivy::TantivyError::InvalidArgument(format!(
                "Invalid field {field:?} (expected {:?})",
                self.search_field
            )));
        }
        Ok(self.num_terms)
    }

    fn total_num_docs(&self) -> tantivy::Result<u64> {
        Ok(self.num_documents)
    }

    fn doc_freq(&self, term: &Term) -> tantivy::Result<u64> {
        self.doc_frequencies.get(term).copied().ok_or_else(|| {
            tantivy::TantivyError::InvalidArgument(format!("Term not found: {:?}", term))
        })
    }
}

#[derive(Clone, Debug)]
pub struct TokenQuery {
    pub term: Term,
    pub max_distance: u32,
    pub prefix: bool,
}

impl TryFrom<pb::searchlight::TokenQuery> for TokenQuery {
    type Error = anyhow::Error;

    fn try_from(value: pb::searchlight::TokenQuery) -> Result<Self, Self::Error> {
        Ok(TokenQuery {
            term: Term::wrap(value.term),
            max_distance: value.max_distance,
            prefix: value.prefix,
        })
    }
}

impl TryFrom<TokenQuery> for pb::searchlight::TokenQuery {
    type Error = anyhow::Error;

    fn try_from(value: TokenQuery) -> Result<Self, Self::Error> {
        Ok(pb::searchlight::TokenQuery {
            term: value.term.as_slice().to_vec(),
            max_distance: value.max_distance,
            prefix: value.prefix,
        })
    }
}

#[derive(Clone, Debug, Ord, PartialOrd, Eq, PartialEq)]
pub struct TokenMatch {
    pub distance: u32,
    pub prefix: bool,
    pub term: Term,
    pub token_ord: u32,
}

impl TryFrom<pb::searchlight::TokenMatch> for TokenMatch {
    type Error = anyhow::Error;

    fn try_from(value: pb::searchlight::TokenMatch) -> Result<Self, Self::Error> {
        Ok(TokenMatch {
            distance: value.distance,
            prefix: value.prefix,
            term: Term::wrap(value.tantivy_bytes),
            token_ord: value.token_ord,
        })
    }
}

impl TryFrom<TokenMatch> for pb::searchlight::TokenMatch {
    type Error = anyhow::Error;

    fn try_from(value: TokenMatch) -> Result<Self, Self::Error> {
        Ok(pb::searchlight::TokenMatch {
            distance: value.distance,
            prefix: value.prefix,
            tantivy_bytes: value.term.as_slice().to_vec(),
            token_ord: value.token_ord,
        })
    }
}

#[derive(Debug, Clone)]
pub struct FragmentedTextSegmentStorageKeys {
    pub segment: ObjectKey,
    pub id_tracker: ObjectKey,
    pub deletions: ObjectKey,
}

impl TryFrom<FragmentedTextSegmentPaths> for FragmentedTextSegmentStorageKeys {
    type Error = anyhow::Error;

    fn try_from(value: FragmentedTextSegmentPaths) -> Result<Self, Self::Error> {
        let from_path = |path: Option<StorageKey>| {
            path.map(|p| p.storage_key)
                .context("Missing path!")?
                .try_into()
        };
        Ok(FragmentedTextSegmentStorageKeys {
            segment: from_path(value.segment)?,
            id_tracker: from_path(value.id_tracker)?,
            deletions: from_path(value.deletions)?,
        })
    }
}

impl TryFrom<FragmentedTextSegmentStorageKeys> for FragmentedTextSegmentPaths {
    type Error = anyhow::Error;

    fn try_from(value: FragmentedTextSegmentStorageKeys) -> Result<Self, Self::Error> {
        Ok(FragmentedTextSegmentPaths {
            segment: Some(StorageKey {
                storage_key: value.segment.into(),
            }),
            id_tracker: Some(StorageKey {
                storage_key: value.id_tracker.into(),
            }),
            deletions: Some(StorageKey {
                storage_key: value.deletions.into(),
            }),
        })
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

#[derive(Debug)]
pub struct Bm25Stats {
    pub num_terms: u64,
    pub num_documents: u64,
    pub doc_frequencies: BTreeMap<Term, u64>,
}

impl Bm25Stats {
    pub fn empty() -> Self {
        Self {
            num_terms: 0,
            num_documents: 0,
            doc_frequencies: BTreeMap::new(),
        }
    }
}

impl Add for Bm25Stats {
    type Output = Self;

    fn add(mut self, rhs: Self) -> Self::Output {
        self.add_assign(rhs);
        self
    }
}

impl AddAssign for Bm25Stats {
    fn add_assign(&mut self, rhs: Self) {
        self.num_terms += rhs.num_terms;
        self.num_documents += rhs.num_documents;
        for (term, count) in rhs.doc_frequencies {
            *self.doc_frequencies.entry(term).or_insert(0) += count;
        }
    }
}

impl TryFrom<Bm25Stats> for QueryBm25StatsResponse {
    type Error = anyhow::Error;

    fn try_from(value: Bm25Stats) -> Result<Self, Self::Error> {
        let doc_frequencies = value
            .doc_frequencies
            .into_iter()
            .map(|(term, frequency)| pb::searchlight::DocFrequency {
                term: term.as_slice().to_vec(),
                frequency,
            })
            .collect();
        Ok(QueryBm25StatsResponse {
            num_terms: value.num_terms,
            num_documents: value.num_documents,
            doc_frequencies,
        })
    }
}

impl TryFrom<QueryBm25StatsResponse> for Bm25Stats {
    type Error = anyhow::Error;

    fn try_from(value: QueryBm25StatsResponse) -> Result<Self, Self::Error> {
        let doc_frequencies = value
            .doc_frequencies
            .into_iter()
            .map(|df| (Term::wrap(df.term), df.frequency))
            .collect();
        Ok(Bm25Stats {
            num_terms: value.num_terms,
            num_documents: value.num_documents,
            doc_frequencies,
        })
    }
}

#[derive(Clone)]
pub struct PostingListQuery {
    pub deleted_internal_ids: BTreeSet<InternalId>,

    pub num_terms: u64,
    pub num_documents: u64,

    pub or_terms: Vec<OrTerm>,
    pub and_terms: Vec<Term>,

    pub max_results: usize,
}

impl TryFrom<pb::searchlight::PostingListQuery> for PostingListQuery {
    type Error = anyhow::Error;

    fn try_from(value: pb::searchlight::PostingListQuery) -> Result<Self, Self::Error> {
        let deleted_internal_ids = value
            .deleted_internal_ids
            .into_iter()
            .map(|b| InternalId::try_from(&b[..]))
            .collect::<anyhow::Result<_>>()?;
        let or_terms = value
            .or_terms
            .into_iter()
            .map(|t| t.try_into())
            .try_collect()?;
        let and_terms = value.and_terms.into_iter().map(Term::wrap).collect();
        Ok(PostingListQuery {
            deleted_internal_ids,
            num_terms: value.num_terms,
            num_documents: value.num_documents,
            or_terms,
            and_terms,
            max_results: value.max_results as usize,
        })
    }
}

impl TryFrom<PostingListQuery> for pb::searchlight::PostingListQuery {
    type Error = anyhow::Error;

    fn try_from(value: PostingListQuery) -> Result<Self, Self::Error> {
        let deleted_internal_ids = value
            .deleted_internal_ids
            .into_iter()
            .map(|id| id.into())
            .collect();
        let or_terms = value
            .or_terms
            .into_iter()
            .map(|t| t.try_into())
            .try_collect()?;
        let and_terms = value
            .and_terms
            .into_iter()
            .map(|t| t.as_slice().to_vec())
            .collect();
        Ok(pb::searchlight::PostingListQuery {
            deleted_internal_ids,
            num_terms: value.num_terms,
            num_documents: value.num_documents,
            or_terms,
            and_terms,
            max_results: value.max_results as u32,
        })
    }
}

#[derive(Debug)]
pub struct PostingListMatch {
    pub internal_id: InternalId,
    pub ts: WriteTimestamp,
    pub creation_time: CreationTime,
    pub bm25_score: f32,
}

impl Ord for PostingListMatch {
    fn cmp(&self, other: &Self) -> Ordering {
        self.bm25_score
            .total_cmp(&other.bm25_score)
            .then(self.creation_time.cmp(&other.creation_time))
            .then(self.internal_id.cmp(&other.internal_id))
            .then(self.ts.cmp(&other.ts))
    }
}

impl PartialOrd for PostingListMatch {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for PostingListMatch {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other).is_eq()
    }
}

impl Eq for PostingListMatch {}

impl TryFrom<pb::searchlight::PostingListMatch> for PostingListMatch {
    type Error = anyhow::Error;

    fn try_from(value: pb::searchlight::PostingListMatch) -> Result<Self, Self::Error> {
        Ok(PostingListMatch {
            internal_id: InternalId::try_from(&value.internal_id[..])?,
            ts: match value.ts {
                Some(pb::searchlight::posting_list_match::Ts::Committed(ts)) => {
                    WriteTimestamp::Committed(ts.try_into()?)
                },
                Some(pb::searchlight::posting_list_match::Ts::Pending(())) => {
                    WriteTimestamp::Pending
                },
                _ => anyhow::bail!("Missing ts field"),
            },
            creation_time: value.creation_time.try_into()?,
            bm25_score: value.bm25_score,
        })
    }
}

impl TryFrom<PostingListMatch> for pb::searchlight::PostingListMatch {
    type Error = anyhow::Error;

    fn try_from(value: PostingListMatch) -> Result<Self, Self::Error> {
        Ok(pb::searchlight::PostingListMatch {
            internal_id: value.internal_id.into(),
            ts: match value.ts {
                WriteTimestamp::Committed(ts) => Some(
                    pb::searchlight::posting_list_match::Ts::Committed(ts.try_into()?),
                ),
                WriteTimestamp::Pending => {
                    Some(pb::searchlight::posting_list_match::Ts::Pending(()))
                },
            },
            creation_time: value.creation_time.into(),
            bm25_score: value.bm25_score,
        })
    }
}

#[cfg(test)]
mod tests {
    use std::{
        cmp,
        collections::{
            BTreeMap,
            BTreeSet,
        },
        env,
        io::{
            BufRead,
            BufReader,
        },
        path::Path,
        sync::LazyLock,
    };

    use common::{
        bootstrap_model::index::search_index::DeveloperSearchIndexConfig,
        document::{
            CreationTime,
            ResolvedDocument,
        },
        id_tracker::StaticIdTracker,
        persistence_helpers::{
            DocumentRevision,
            RevisionPair,
        },
        testing::TestIdGenerator,
        types::Timestamp,
    };
    use futures::StreamExt;
    use runtime::testing::TestRuntime;
    use tantivy::{
        Index,
        Term,
    };
    use tempfile::TempDir;
    use text_search::tracker::{
        load_alive_bitset,
        StaticDeletionTracker,
    };
    use value::{
        assert_obj,
        FieldPath,
        InternalId,
        ResolvedDocumentId,
        TabletIdAndTableNumber,
    };

    use super::PostingListMatch;
    use crate::{
        convex_query::OrTerm,
        disk_index::index_reader_for_directory,
        incremental_index::{
            build_index,
            merge_segments,
            SearchSegment,
            ALIVE_BITSET_PATH,
            DELETED_TERMS_PATH,
            ID_TRACKER_PATH,
        },
        searcher::{
            searcher::{
                PostingListQuery,
                TokenQuery,
            },
            SearcherImpl,
        },
        TantivySearchIndexSchema,
        EXACT_SEARCH_MAX_WORD_LENGTH,
        SINGLE_TYPO_SEARCH_MAX_WORD_LENGTH,
    };

    static TEST_TABLE: LazyLock<TabletIdAndTableNumber> = LazyLock::new(|| {
        let mut id_generator = TestIdGenerator::new();
        let table_name = "test".parse().unwrap();
        id_generator.table_id(&table_name)
    });

    #[tokio::test]
    #[ignore]
    async fn test_incremental_search() -> anyhow::Result<()> {
        let start = std::time::Instant::now();
        let test_dir = TempDir::new()?;

        let dataset_path = std::env::var("DATASET")?;
        let query = std::env::var("QUERY")?;
        let max_terms = env::var("MAX_TERMS")?.parse()?;
        let max_results = env::var("MAX_RESULTS")?.parse()?;

        let mut id_generator = TestIdGenerator::new();
        let field_path: FieldPath = "mySearchField".parse()?;
        let schema = TantivySearchIndexSchema::new(&DeveloperSearchIndexConfig {
            search_field: field_path.clone(),
            filter_fields: BTreeSet::new(),
        });

        #[derive(serde::Deserialize)]
        struct SearchDocument {
            text: String,
        }
        let f = std::fs::File::open(&dataset_path)?;
        let f = BufReader::new(f);
        let mut strings = vec![];
        for line in f.lines() {
            let d: SearchDocument = serde_json::from_str(&line?)?;
            strings.push(d.text);
        }

        let mut strings_by_id = BTreeMap::new();
        let revisions = strings.into_iter().map(|s| {
            let id = ResolvedDocumentId::new(*TEST_TABLE, id_generator.generate_internal());
            strings_by_id.insert(id, s.clone());
            let creation_time = CreationTime::try_from(10.)?;
            let new_doc =
                ResolvedDocument::new(id, creation_time, assert_obj!("mySearchField" => s))?;
            let revision_pair = RevisionPair {
                id: id.into(),
                rev: DocumentRevision {
                    ts: Timestamp::MIN,
                    document: Some(new_doc),
                },
                prev_rev: None,
            };
            Ok(revision_pair)
        });
        let revision_stream = futures::stream::iter(revisions).boxed();
        build_index(revision_stream, schema.clone(), test_dir.path()).await?;
        println!("Indexed {dataset_path} in {:?}", start.elapsed());

        let index_reader = index_reader_for_directory(test_dir.path())?;
        let searcher = index_reader.searcher();

        let mut token_stream = schema.analyzer.token_stream(&query);
        let mut tokens = vec![];
        while let Some(token) = token_stream.next() {
            tokens.push(token.text.clone());
        }
        let num_tokens = tokens.len();
        let mut token_queries = vec![];
        for (i, token) in tokens.into_iter().enumerate() {
            let char_count = token.chars().count();
            let max_distance = if char_count <= EXACT_SEARCH_MAX_WORD_LENGTH {
                0
            } else if char_count <= SINGLE_TYPO_SEARCH_MAX_WORD_LENGTH {
                1
            } else {
                2
            };
            let term = Term::from_field_text(schema.search_field, &token);
            let query = TokenQuery {
                term,
                max_distance,
                prefix: i == num_tokens - 1,
            };
            token_queries.push(query);
        }

        anyhow::ensure!(searcher.segment_readers().len() == 1);
        let segment = &searcher.segment_readers()[0];
        let alive_bitset_path = test_dir.path().join(ALIVE_BITSET_PATH);
        let alive_bitset = load_alive_bitset(&alive_bitset_path)?;
        let deleted_terms_path = test_dir.path().join(DELETED_TERMS_PATH);
        let deletion_tracker = StaticDeletionTracker::load(alive_bitset, &deleted_terms_path)?;
        let start = std::time::Instant::now();
        let results = SearcherImpl::<TestRuntime>::query_tokens_impl(
            segment,
            &deletion_tracker,
            token_queries,
            max_terms,
        )?;

        if results.is_empty() {
            println!("No results found");
            return Ok(());
        }

        println!("{} term results ({:?}):", results.len(), start.elapsed());
        for result in &results {
            println!(
                "{:?}: dist {}, prefix? {}",
                result.term, result.distance, result.prefix
            );
        }

        let start = std::time::Instant::now();
        let terms = results.iter().map(|r| r.term.clone()).collect();
        let stats =
            SearcherImpl::<TestRuntime>::query_bm25_stats_impl(segment, &deletion_tracker, terms)?;
        println!("\nBM25 stats ({:?}): {stats:?}", start.elapsed());

        let mut results_by_term = BTreeMap::new();
        for result in results {
            let sort_key = (result.distance, result.prefix, result.token_ord);
            let existing_key = results_by_term.entry(result.term).or_insert(sort_key);
            *existing_key = cmp::min(*existing_key, sort_key);
        }

        let mut or_terms = vec![];
        for (term, (distance, prefix, _)) in results_by_term {
            let doc_frequency = stats.doc_frequencies[&term];
            // TODO: Come up with a smarter way to boost scores based on edit distance.
            let mut boost = 1. / (1. + distance as f32);
            if prefix {
                boost *= 0.5;
            }
            let or_term = OrTerm {
                term,
                doc_frequency,
                bm25_boost: boost,
            };
            or_terms.push(or_term);
        }

        let start = std::time::Instant::now();
        let query = PostingListQuery {
            deleted_internal_ids: BTreeSet::new(),
            or_terms,
            and_terms: vec![],
            num_terms: stats.num_terms,
            num_documents: stats.num_documents,
            max_results,
        };
        let id_tracker_path = test_dir.path().join(ID_TRACKER_PATH);
        let id_tracker = StaticIdTracker::load_from_path(id_tracker_path)?;
        let posting_list_matches = SearcherImpl::<TestRuntime>::query_posting_lists_impl(
            &searcher,
            segment,
            &id_tracker,
            &deletion_tracker,
            query,
        )?;
        println!(
            "\n{} posting list results ({:?}):",
            posting_list_matches.len(),
            start.elapsed()
        );
        for result in &posting_list_matches {
            println!("{:?} @ {}", result.internal_id, result.bm25_score);
            let id = ResolvedDocumentId::new(*TEST_TABLE, result.internal_id);
            println!("  {}", strings_by_id[&id]);
        }

        Ok(())
    }

    fn test_schema() -> TantivySearchIndexSchema {
        let field_path: FieldPath = "mySearchField".parse().unwrap();
        TantivySearchIndexSchema::new(&DeveloperSearchIndexConfig {
            search_field: field_path.clone(),
            filter_fields: BTreeSet::new(),
        })
    }

    async fn build_test_index(
        revisions: Vec<StringRevision>,
        index_dir: &Path,
    ) -> anyhow::Result<BTreeMap<ResolvedDocumentId, Option<String>>> {
        let mut strings_by_id = BTreeMap::new();
        let revisions = revisions.into_iter().map(
            |StringRevision {
                 id,
                 prev_str,
                 new_str,
             }| {
                let id = ResolvedDocumentId::new(*TEST_TABLE, id);
                strings_by_id.entry(id).or_insert(new_str.clone());
                let creation_time = CreationTime::try_from(10.)?;
                let old_doc = prev_str
                    .map(|s| {
                        ResolvedDocument::new(id, creation_time, assert_obj!("mySearchField" => s))
                    })
                    .transpose()?;
                let new_doc = new_str
                    .map(|s| {
                        ResolvedDocument::new(id, creation_time, assert_obj!("mySearchField" => s))
                    })
                    .transpose()?;
                let revision_pair = RevisionPair {
                    id: id.into(),
                    rev: DocumentRevision {
                        ts: Timestamp::MIN,
                        document: new_doc,
                    },
                    prev_rev: old_doc.map(|d| DocumentRevision {
                        ts: Timestamp::MIN,
                        document: Some(d),
                    }),
                };
                Ok(revision_pair)
            },
        );
        let revision_stream = futures::stream::iter(revisions).boxed();
        let schema = test_schema();
        build_index(revision_stream, schema.clone(), index_dir).await?;
        Ok(strings_by_id)
    }

    struct StringRevision {
        id: InternalId,
        prev_str: Option<String>,
        new_str: Option<String>,
    }
    async fn incremental_search_with_deletions_helper(
        query: &str,
        test_dir: &Path,
        strings_by_id: &BTreeMap<ResolvedDocumentId, Option<String>>,
    ) -> anyhow::Result<Vec<(PostingListMatch, String)>> {
        let index_reader = index_reader_for_directory(test_dir)?;
        let searcher = index_reader.searcher();

        let schema = test_schema();
        let mut token_stream = schema.analyzer.token_stream(query);
        let mut tokens = vec![];
        while let Some(token) = token_stream.next() {
            tokens.push(token.text.clone());
        }
        let num_tokens = tokens.len();
        let mut token_queries = vec![];
        for (i, token) in tokens.into_iter().enumerate() {
            let char_count = token.chars().count();
            let max_distance = if char_count <= EXACT_SEARCH_MAX_WORD_LENGTH {
                0
            } else if char_count <= SINGLE_TYPO_SEARCH_MAX_WORD_LENGTH {
                1
            } else {
                2
            };
            let term = Term::from_field_text(schema.search_field, &token);
            let query = TokenQuery {
                term,
                max_distance,
                prefix: i == num_tokens - 1,
            };
            token_queries.push(query);
        }

        anyhow::ensure!(searcher.segment_readers().len() == 1);
        let segment = &searcher.segment_readers()[0];
        let alive_bitset_path = test_dir.join(ALIVE_BITSET_PATH);
        let alive_bitset = load_alive_bitset(&alive_bitset_path)?;
        let deleted_terms_path = test_dir.join(DELETED_TERMS_PATH);
        let deletion_tracker = StaticDeletionTracker::load(alive_bitset, &deleted_terms_path)?;
        let start = std::time::Instant::now();
        let results = SearcherImpl::<TestRuntime>::query_tokens_impl(
            segment,
            &deletion_tracker,
            token_queries,
            16,
        )?;

        if results.is_empty() {
            println!("No results found");
            return Ok(vec![]);
        }

        println!("{} term results ({:?}):", results.len(), start.elapsed());
        for result in &results {
            println!(
                "{:?}: dist {}, prefix? {}",
                result.term, result.distance, result.prefix
            );
        }

        let start = std::time::Instant::now();
        let terms = results.iter().map(|r| r.term.clone()).collect();
        let stats =
            SearcherImpl::<TestRuntime>::query_bm25_stats_impl(segment, &deletion_tracker, terms)?;
        println!("\nBM25 stats ({:?}): {stats:?}", start.elapsed());

        let mut results_by_term = BTreeMap::new();
        for result in results {
            let sort_key = (result.distance, result.prefix, result.token_ord);
            let existing_key = results_by_term.entry(result.term).or_insert(sort_key);
            *existing_key = cmp::min(*existing_key, sort_key);
        }

        let mut or_terms = vec![];
        for (term, (distance, prefix, _)) in results_by_term {
            let doc_frequency = stats.doc_frequencies[&term];
            // TODO: Come up with a smarter way to boost scores based on edit distance.
            let mut boost = 1. / (1. + distance as f32);
            if prefix {
                boost *= 0.5;
            }
            let or_term = OrTerm {
                term,
                doc_frequency,
                bm25_boost: boost,
            };
            or_terms.push(or_term);
        }

        let start = std::time::Instant::now();
        let max_results = 10;
        let query = PostingListQuery {
            deleted_internal_ids: BTreeSet::new(),
            or_terms,
            and_terms: vec![],
            num_terms: stats.num_terms,
            num_documents: stats.num_documents,
            max_results,
        };
        let id_tracker_path = test_dir.join(ID_TRACKER_PATH);
        let id_tracker = StaticIdTracker::load_from_path(id_tracker_path)?;
        let posting_list_matches = SearcherImpl::<TestRuntime>::query_posting_lists_impl(
            &searcher,
            segment,
            &id_tracker,
            &deletion_tracker,
            query,
        )?;
        println!(
            "\n{} posting list results ({:?}):",
            posting_list_matches.len(),
            start.elapsed()
        );
        let mut posting_list_matches_and_strings = vec![];
        for result in posting_list_matches {
            println!("{:?} @ {}", result.internal_id, result.bm25_score);
            let id = ResolvedDocumentId::new(*TEST_TABLE, result.internal_id);
            let s = strings_by_id[&id].as_ref().unwrap();
            println!("  {s}",);
            posting_list_matches_and_strings.push((result, s.clone()));
        }
        Ok(posting_list_matches_and_strings)
    }

    #[tokio::test]
    async fn test_incremental_search_with_deletion() -> anyhow::Result<()> {
        let query = "emma";
        let mut id_generator = TestIdGenerator::new();
        let id1 = id_generator.generate_internal();
        let id2 = id_generator.generate_internal();
        let revisions = vec![
            (id1, Some("emma works at convex"), None), // Delete
            (id2, None, Some("sujay lives in ny")),
            (id1, None, Some("emma works at convex")),
        ]
        .into_iter()
        .map(|(id, prev_str, new_str)| StringRevision {
            id,
            prev_str: prev_str.map(|s| s.to_string()),
            new_str: new_str.map(|s| s.to_string()),
        })
        .collect();
        let test_dir = TempDir::new()?;
        let strings_by_id = build_test_index(revisions, test_dir.path()).await?;
        let posting_list_matches =
            incremental_search_with_deletions_helper(query, test_dir.path(), &strings_by_id)
                .await?;
        assert!(posting_list_matches.is_empty());
        Ok(())
    }

    #[tokio::test]
    async fn test_incremental_search_with_replace() -> anyhow::Result<()> {
        let query = "emma";
        let mut id_generator = TestIdGenerator::new();
        let id = id_generator.generate_internal();
        let revisions = vec![
            (id, Some("emma is awesome!"), Some("emma is gr8")), // Replace
            (id, None, Some("emma is awesome!")),
        ]
        .into_iter()
        .map(|(id, prev_str, new_str)| StringRevision {
            id,
            prev_str: prev_str.map(|s| s.to_string()),
            new_str: new_str.map(|s| s.to_string()),
        })
        .collect();
        let test_dir = TempDir::new()?;
        let strings_by_id = build_test_index(revisions, test_dir.path()).await?;
        let posting_list_matches =
            incremental_search_with_deletions_helper(query, test_dir.path(), &strings_by_id)
                .await?;
        assert_eq!(posting_list_matches.len(), 1);
        let (posting_list_match, s) = posting_list_matches.first().unwrap();
        assert_eq!(posting_list_match.internal_id, id);
        assert_eq!(s, "emma is gr8");
        Ok(())
    }

    #[tokio::test]
    async fn test_merge_tantivy_segments() -> anyhow::Result<()> {
        let query = "emma";
        let mut id_generator = TestIdGenerator::new();
        let id1 = id_generator.generate_internal();
        let revisions = vec![(id1, None, Some("emma is gr8"))]
            .into_iter()
            .map(|(id, prev_str, new_str)| StringRevision {
                id,
                prev_str: prev_str.map(|s: &str| s.to_string()),
                new_str: new_str.map(|s| s.to_string()),
            })
            .collect::<Vec<_>>();
        let test_dir_1 = TempDir::new()?;
        let mut strings_by_id_1 = build_test_index(revisions, test_dir_1.path()).await?;
        let posting_list_matches =
            incremental_search_with_deletions_helper(query, test_dir_1.path(), &strings_by_id_1)
                .await?;
        assert_eq!(posting_list_matches.len(), 1);
        let (posting_list_match, s) = posting_list_matches.first().unwrap();
        assert_eq!(posting_list_match.internal_id, id1);
        assert_eq!(s, "emma is gr8");
        let id2 = id_generator.generate_internal();
        let revisions = vec![(id2, None, Some("emma is awesome!"))]
            .into_iter()
            .map(|(id, prev_str, new_str)| StringRevision {
                id,
                prev_str: prev_str.map(|s: &str| s.to_string()),
                new_str: new_str.map(|s| s.to_string()),
            })
            .collect::<Vec<_>>();
        let test_dir_2 = TempDir::new()?;
        let mut strings_by_id_2 = build_test_index(revisions, test_dir_2.path()).await?;
        let posting_list_matches =
            incremental_search_with_deletions_helper(query, test_dir_2.path(), &strings_by_id_2)
                .await?;
        assert_eq!(posting_list_matches.len(), 1);
        let (posting_list_match, s) = posting_list_matches.first().unwrap();
        assert_eq!(posting_list_match.internal_id, id2);
        assert_eq!(s, "emma is awesome!");

        let segments = vec![
            search_segment_from_path(test_dir_1.path())?,
            search_segment_from_path(test_dir_2.path())?,
        ];

        let merged_dir = TempDir::new()?;
        merge_segments(segments, merged_dir.path()).await?;

        strings_by_id_1.append(&mut strings_by_id_2);
        let mut posting_list_matches =
            incremental_search_with_deletions_helper(query, merged_dir.path(), &strings_by_id_1)
                .await?;
        assert_eq!(posting_list_matches.len(), 2);
        let (posting_list_match, s) = posting_list_matches.pop().unwrap();
        assert_eq!(posting_list_match.internal_id, id1);
        assert_eq!(s, "emma is gr8");
        let (posting_list_match, s) = posting_list_matches.pop().unwrap();
        assert_eq!(posting_list_match.internal_id, id2);
        assert_eq!(s, "emma is awesome!");

        Ok(())
    }

    fn search_segment_from_path(dir: &Path) -> anyhow::Result<SearchSegment> {
        Ok(SearchSegment {
            segment: Index::open_in_dir(dir)?,
            alive_bitset: load_alive_bitset(&dir.join(ALIVE_BITSET_PATH))?,
            id_tracker: StaticIdTracker::load_from_path(dir.join(ID_TRACKER_PATH))?,
        })
    }
}

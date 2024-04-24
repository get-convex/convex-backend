use std::{
    collections::{
        BTreeMap,
        BTreeSet,
        BinaryHeap,
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
    runtime::Runtime,
    types::{
        ObjectKey,
        Timestamp,
    },
};
use futures::TryStreamExt;
use itertools::Itertools;
use pb::searchlight::{
    FragmentedTextSegmentPaths,
    FragmentedVectorSegmentPaths,
    QueryBm25StatsResponse,
    StorageKey,
};
use storage::Storage;
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
    termdict::{
        TermDictionary,
        TermOrdinal,
    },
    DocId,
    SegmentReader,
    Term,
};
use tantivy_common::{
    BitSet,
    ReadOnlyBitSet,
};
use value::{
    FieldPath,
    InternalId,
};
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
    convex_query::{
        ConvexSearchQuery,
        DeletedDocuments,
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

impl<RT: Runtime> SearcherImpl<RT> {
    pub async fn query_tokens(
        &self,
        search_storage: Arc<dyn Storage>,
        storage_keys: FragmentedTextSegmentStorageKeys,
        schema: TantivySearchIndexSchema,
        queries: Vec<TokenQuery>,
        max_results: usize,
    ) -> anyhow::Result<Vec<TokenMatch>> {
        let archive_path = self
            .archive_cache
            .get(search_storage, &storage_keys.segment, SearchFileType::Text)
            .await?;
        let query = move || {
            let reader = index_reader_for_directory(&archive_path)?;
            let searcher = reader.searcher();
            anyhow::ensure!(searcher.segment_readers().len() == 1);
            let segment = searcher.segment_reader(0);

            let deletion_tracker = DeletionTracker::load(&archive_path)?;

            let mut token_queries = vec![];
            for (token_ord, query) in queries.into_iter().enumerate() {
                // TODO: Use a uniform Tantivy schema for search and filter fields.
                let tantivy_term = if query.field_path == schema.search_field_path {
                    let token_str = std::str::from_utf8(&query.token)?;
                    Term::from_field_text(schema.search_field, token_str)
                } else {
                    let Some(tantivy_field) = schema.filter_fields.get(&query.field_path) else {
                        anyhow::bail!("Field path not found: {:?}", query.field_path);
                    };
                    Term::from_field_bytes(*tantivy_field, &query.token)
                };
                token_queries.push((tantivy_term, token_ord as u32, query));
            }

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

            let mut all_matches = vec![];

            for (tantivy_term, token_ord, token_query) in token_queries {
                // Query the top scoring tuples for just our query term.
                let matches = Self::top_terms_for_query(
                    segment,
                    &deletion_tracker,
                    token_ord,
                    &tantivy_term,
                    &token_query,
                    max_results,
                )?;
                all_matches.extend(matches);
            }

            // Merge the top results from each query term and take the best results,
            // limiting ourselves to `max_results` terms.
            all_matches.sort();

            let mut seen_terms = BTreeSet::new();
            let mut merged_matches = vec![];

            for query_match in all_matches {
                let new_term = !seen_terms.contains(&query_match.term);
                if seen_terms.len() >= max_results && new_term {
                    break;
                }
                if new_term {
                    seen_terms.insert(query_match.term.clone());
                }
                merged_matches.push(query_match);
            }
            Ok(merged_matches)
        };

        let resp = self.text_search_pool.execute(query).await??;
        Ok(resp)
    }

    fn top_terms_for_query(
        segment: &SegmentReader,
        deletion_tracker: &DeletionTracker,
        token_ord: u32,
        query_term: &Term,
        query: &TokenQuery,
        max_results: usize,
    ) -> anyhow::Result<Vec<TokenMatch>> {
        let inverted_index = segment.inverted_index(query_term.field())?;
        let term_dict = inverted_index.terms();

        let mut results = vec![];
        let mut seen_terms = BTreeSet::new();

        'query: for edit_distance in [0, 1, 2] {
            for prefix in [false, true] {
                if edit_distance > query.max_distance || prefix != query.prefix {
                    continue;
                }
                if seen_terms.len() >= max_results {
                    break 'query;
                }
                if edit_distance == 0 && !prefix {
                    if let Some(term_ord) = term_dict.term_ord(query_term.value_bytes())? {
                        if deletion_tracker.doc_frequency(term_dict, term_ord)? == 0 {
                            continue;
                        }
                        anyhow::ensure!(!seen_terms.contains(query_term));
                        let m = TokenMatch {
                            distance: edit_distance,
                            prefix,
                            term: query_term.clone(),
                            token_ord,
                        };
                        results.push(m);
                        seen_terms.insert(query_term.clone());
                    }
                } else {
                    let term_str = query_term
                        .as_str()
                        .context("Non-exact match for non-string field")?;
                    let dfa = build_fuzzy_dfa(term_str, edit_distance as u8, prefix);
                    let dfa_compat = LevenshteinDfaWrapper(&dfa);
                    let mut term_stream = term_dict.search(dfa_compat).into_stream()?;
                    while term_stream.advance() {
                        let matched_term_bytes = term_stream.key();
                        let match_str = std::str::from_utf8(matched_term_bytes)?;
                        let match_term = Term::from_field_text(query_term.field(), match_str);

                        let term_ord = term_stream.term_ord();
                        if deletion_tracker.doc_frequency(term_dict, term_ord)? == 0 {
                            continue;
                        }

                        // We need to skip terms we've already processed so we can
                        // stop after seeing `results_remaining` new terms.
                        if seen_terms.contains(&match_term) {
                            continue;
                        }

                        // TODO: Copy comment from tantivy_query.rs.
                        // TODO: Ideally we could make DFAs that only match a particular
                        // edit distance.
                        // TODO: What does the `to_u8` mean?
                        let matched_distance = dfa.eval(matched_term_bytes).to_u8() as u32;
                        if edit_distance != matched_distance {
                            anyhow::ensure!(matched_distance <= edit_distance);
                            continue;
                        }

                        let m = TokenMatch {
                            distance: edit_distance,
                            prefix,
                            term: match_term.clone(),
                            token_ord,
                        };
                        results.push(m);
                        seen_terms.insert(match_term);

                        if seen_terms.len() >= max_results {
                            break 'query;
                        }
                    }
                }
            }
        }

        anyhow::ensure!(results.is_sorted());

        Ok(results)
    }

    pub async fn query_bm25_stats(
        &self,
        search_storage: Arc<dyn Storage>,
        storage_keys: FragmentedTextSegmentStorageKeys,
        terms: Vec<Term>,
    ) -> anyhow::Result<Bm25Stats> {
        let archive_path = self
            .archive_cache
            .get(search_storage, &storage_keys.segment, SearchFileType::Text)
            .await?;
        let query = move || {
            let reader = index_reader_for_directory(&archive_path)?;
            let searcher = reader.searcher();
            anyhow::ensure!(searcher.segment_readers().len() == 1);
            let segment = searcher.segment_reader(0);

            let deletion_tracker = DeletionTracker::load(&archive_path)?;

            let field = terms
                .iter()
                .map(|t| t.field())
                .dedup()
                .exactly_one()
                .map_err(|_| anyhow::anyhow!("All terms must be in the same field"))?;

            // TODO: Update stats with deletions.
            let inverted_index = segment.inverted_index(field)?;
            let term_dict = inverted_index.terms();
            let num_terms = inverted_index
                .total_num_tokens()
                .checked_sub(deletion_tracker.num_terms_deleted()?)
                .context("num_terms underflow")?;
            let num_documents = (segment.max_doc() as u64)
                .checked_sub(deletion_tracker.num_documents_deleted()?)
                .context("num_documents underflow")?;
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
        };
        let resp = self.text_search_pool.execute(query).await??;
        Ok(resp)
    }

    pub async fn query_posting_lists(
        &self,
        search_storage: Arc<dyn Storage>,
        storage_keys: FragmentedTextSegmentStorageKeys,
        query: PostingListQuery,
    ) -> anyhow::Result<Vec<PostingListMatch>> {
        let archive_path = self
            .archive_cache
            .get(search_storage, &storage_keys.segment, SearchFileType::Text)
            .await?;
        let query = move || {
            let reader = index_reader_for_directory(&archive_path)?;
            let searcher = reader.searcher();
            anyhow::ensure!(searcher.segment_readers().len() == 1);
            let segment = searcher.segment_reader(0);

            let id_tracker = IdTracker::load(&archive_path)?;
            let deleted_tracker = DeletionTracker::load(&archive_path)?;

            let search_field = query
                .or_terms
                .iter()
                .map(|t| t.field())
                .dedup()
                .exactly_one()
                .map_err(|_| anyhow::anyhow!("All terms must be in the same field"))?;
            let stats_provider = StatsProvider {
                search_field,
                num_terms: query.num_terms,
                num_documents: query.num_documents,
                doc_frequencies: query.doc_frequencies,
            };

            let mut memory_deleted = BTreeSet::new();
            for internal_id in query.deleted_internal_ids {
                let Some(doc_id) = id_tracker.lookup_id(&internal_id)? else {
                    continue;
                };
                memory_deleted.insert(doc_id);
            }
            let deleted_documents = DeletedDocuments {
                memory_deleted,
                segment_deleted: deleted_tracker.deleted_documents(),
                num_segment_deleted: deleted_tracker.num_documents_deleted()? as usize,
            };
            let search_query =
                ConvexSearchQuery::new(query.or_terms, query.and_terms, deleted_documents);
            let enable_scoring =
                EnableScoring::enabled_from_statistics_provider(&stats_provider, &searcher);
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
                let creation_time =
                    CreationTime::try_from(creation_times.get_val(doc_address.doc_id))?;
                let posting_list_match = PostingListMatch {
                    internal_id,
                    ts,
                    creation_time,
                    bm25_score,
                };
                results.push(posting_list_match);
            }
            Ok(results)
        };
        let resp = self.text_search_pool.execute(query).await??;
        Ok(resp)
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

pub struct TokenQuery {
    pub field_path: FieldPath,
    pub token: Vec<u8>,
    pub max_distance: u32,
    pub prefix: bool,
}

impl TryFrom<pb::searchlight::TokenQuery> for TokenQuery {
    type Error = anyhow::Error;

    fn try_from(value: pb::searchlight::TokenQuery) -> Result<Self, Self::Error> {
        Ok(TokenQuery {
            field_path: value.field_path.context("Missing field_path")?.try_into()?,
            token: value.token,
            max_distance: value.max_distance,
            prefix: value.prefix,
        })
    }
}

impl TryFrom<TokenQuery> for pb::searchlight::TokenQuery {
    type Error = anyhow::Error;

    fn try_from(value: TokenQuery) -> Result<Self, Self::Error> {
        Ok(pb::searchlight::TokenQuery {
            field_path: Some(value.field_path.into()),
            token: value.token,
            max_distance: value.max_distance,
            prefix: value.prefix,
        })
    }
}

#[derive(Ord, PartialOrd, Eq, PartialEq)]
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

pub struct Bm25Stats {
    pub num_terms: u64,
    pub num_documents: u64,
    pub doc_frequencies: BTreeMap<Term, u64>,
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

pub struct PostingListQuery {
    pub deleted_internal_ids: BTreeSet<InternalId>,

    pub or_terms: Vec<Term>,
    pub and_terms: Vec<Term>,

    pub num_terms: u64,
    pub num_documents: u64,
    pub doc_frequencies: BTreeMap<Term, u64>,

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
        let or_terms = value.or_terms.into_iter().map(Term::wrap).collect();
        let and_terms = value.and_terms.into_iter().map(Term::wrap).collect();
        let doc_frequencies = value
            .doc_frequencies
            .into_iter()
            .map(|df| (Term::wrap(df.term), df.frequency))
            .collect();
        Ok(PostingListQuery {
            deleted_internal_ids,
            or_terms,
            and_terms,
            num_terms: value.num_terms,
            num_documents: value.num_documents,
            doc_frequencies,
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
            .map(|t| t.as_slice().to_vec())
            .collect();
        let and_terms = value
            .and_terms
            .into_iter()
            .map(|t| t.as_slice().to_vec())
            .collect();
        let doc_frequencies = value
            .doc_frequencies
            .into_iter()
            .map(|(term, frequency)| pb::searchlight::DocFrequency {
                term: term.as_slice().to_vec(),
                frequency,
            })
            .collect();
        Ok(pb::searchlight::PostingListQuery {
            deleted_internal_ids,
            or_terms,
            and_terms,
            num_terms: value.num_terms,
            num_documents: value.num_documents,
            doc_frequencies,
            max_results: value.max_results as u32,
        })
    }
}

pub struct PostingListMatch {
    pub internal_id: InternalId,
    pub ts: Timestamp,
    pub creation_time: CreationTime,
    pub bm25_score: f32,
}

impl TryFrom<pb::searchlight::PostingListMatch> for PostingListMatch {
    type Error = anyhow::Error;

    fn try_from(value: pb::searchlight::PostingListMatch) -> Result<Self, Self::Error> {
        Ok(PostingListMatch {
            internal_id: InternalId::try_from(&value.internal_id[..])?,
            ts: value.ts.try_into()?,
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
            ts: value.ts.into(),
            creation_time: value.creation_time.into(),
            bm25_score: value.bm25_score,
        })
    }
}

// Mapping from `InternalId` to `DocId` for all documents in a segment.
// Immutable over the segment's lifetime.
pub struct IdTracker;

impl IdTracker {
    pub fn load(_archive_path: &Path) -> anyhow::Result<Self> {
        anyhow::bail!("Not implemented")
    }

    pub fn lookup_id(&self, _id: &InternalId) -> anyhow::Result<Option<DocId>> {
        anyhow::bail!("Not implemented")
    }
}

pub struct DeletionTracker;

impl DeletionTracker {
    pub fn load(_archive_path: &Path) -> anyhow::Result<Self> {
        // TODO: Load the file's header into memory.
        Ok(Self)
    }

    pub fn doc_frequency(
        &self,
        term_dict: &TermDictionary,
        term_ord: TermOrdinal,
    ) -> anyhow::Result<u64> {
        let term_info = term_dict.term_info_from_ord(term_ord);
        (term_info.doc_freq as u64)
            .checked_sub(self.term_documents_deleted(term_ord)?)
            .context("doc_frequency underflow")
    }

    /// How many terms have been completely deleted from the segment?
    pub fn num_terms_deleted(&self) -> anyhow::Result<u64> {
        // TODO: Read this from the header.
        Ok(0)
    }

    /// How many documents have been deleted from the segment?
    pub fn num_documents_deleted(&self) -> anyhow::Result<u64> {
        // TODO: Read this from the header.
        Ok(0)
    }

    /// How many of a term's documents have been deleted?
    pub fn term_documents_deleted(&self, _term_ord: TermOrdinal) -> anyhow::Result<u64> {
        // TODO: Load this from the perfect hash table, defaulting to zero if missing.
        Ok(0)
    }

    /// Which documents have been deleted in the segment?
    pub fn deleted_documents(&self) -> ReadOnlyBitSet {
        // TODO: Load this from disk.
        let empty = BitSet::with_max_value(0);
        (&empty).into()
    }
}

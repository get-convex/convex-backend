#![feature(iter_from_coroutine, coroutines)]
#![feature(let_chains)]
#![feature(lazy_cell)]
#![feature(try_blocks)]
#![feature(is_sorted)]
#![feature(ptr_metadata)]
#![feature(iterator_try_collect)]
#![feature(async_closure)]
#![feature(assert_matches)]
#![feature(impl_trait_in_assoc_type)]
#![feature(trait_alias)]

mod aggregation;
mod archive;
mod constants;
mod convex_query;
pub mod disk_index;
pub mod fragmented_segment;
mod incremental_index;
mod intersection;
mod levenshtein_dfa;
mod memory_index;
pub mod metrics;
pub mod query;
mod ranking;
pub mod scoring;
mod search_index_manager;
pub mod searcher;
mod tantivy_query;

use std::{
    cmp,
    collections::{
        BTreeMap,
        BTreeSet,
    },
    sync::Arc,
};

use aggregation::PostingListMatchAggregator;
use anyhow::Context;
use common::{
    bootstrap_model::index::{
        text_index::DeveloperTextIndexConfig,
        IndexConfig,
    },
    document::ResolvedDocument,
    index::IndexKeyBytes,
    knobs::{
        SEARCHLIGHT_CLUSTER_NAME,
        USE_MULTI_SEGMENT_SEARCH_QUERY,
    },
    query::{
        search_value_to_bytes,
        InternalSearch,
        InternalSearchFilterExpression,
        SearchVersion,
    },
    runtime::{
        try_join_buffer_unordered,
        Runtime,
    },
    types::{
        IndexName,
        ObjectKey,
        Timestamp,
    },
};
use constants::CONVEX_EN_TOKENIZER;
pub use constants::{
    convex_en,
    EXACT_SEARCH_MAX_WORD_LENGTH,
    MAX_CANDIDATE_REVISIONS,
    MAX_FILTER_CONDITIONS,
    MAX_QUERY_TERMS,
    SINGLE_TYPO_SEARCH_MAX_WORD_LENGTH,
};
use convex_query::OrTerm;
use errors::ErrorMetadata;
use indexing::index_registry::Index;
use itertools::Itertools;
use metrics::log_search_token_limit_exceeded;
pub use query::{
    CandidateRevision,
    FilterConditionRead,
    QueryReads,
    QueryResults,
    TextQueryTermRead,
};
use query::{
    RevisionWithKeys,
    TextQueryTerm,
};
use searcher::TextStorageKeys;
use storage::Storage;
pub use tantivy::Document as TantivyDocument;
use tantivy::{
    schema::{
        BytesOptions,
        Field,
        IndexRecordOption,
        Schema,
        TextFieldIndexing,
        TextOptions,
        FAST,
    },
    tokenizer::{
        TextAnalyzer,
        Token,
    },
    Term,
};
pub use tantivy_query::SearchQueryResult;
use value::{
    sorting::TotalOrdF64,
    values_to_bytes,
    ConvexValue,
    FieldPath,
};

use self::query::{
    CompiledFilterCondition,
    CompiledQuery,
    QueryTerm,
};
pub use self::{
    incremental_index::{
        build_new_segment,
        fetch_term_ordinals_and_remap_deletes,
        NewTextSegment,
        PreviousTextSegments,
        SegmentStatisticsUpdates,
        TextSegmentPaths,
        UpdatableTextSegment,
    },
    memory_index::{
        build_term_weights,
        MemorySearchIndex,
    },
    search_index_manager::{
        DiskIndex,
        SearchIndex,
        SearchIndexManager,
        SearchIndexManagerState,
        SnapshotInfo,
    },
    searcher::{
        Searcher,
        SegmentTermMetadataFetcher,
    },
};
use crate::{
    aggregation::TokenMatchAggregator,
    constants::MAX_UNIQUE_QUERY_TERMS,
    metrics::log_num_segments_searched_total,
    ranking::Ranker,
    searcher::{
        Bm25Stats,
        PostingListQuery,
        TokenQuery,
    },
};

/// The field ID of the search field in tantivy. DON'T CHANGE THIS!
const SEARCH_FIELD_ID: u32 = 3;

/// The field name for the internal ID field. DON'T CHANGE THIS!
pub const INTERNAL_ID_FIELD_NAME: &str = "internal_id";

/// The field name for the timestamp field. DON'T CHANGE THIS!
pub const TS_FIELD_NAME: &str = "ts";

/// The field name for the creation time field. DON'T CHANGE THIS!
pub const CREATION_TIME_FIELD_NAME: &str = "creation_time";

#[derive(Debug, Clone)]
pub enum DocumentTerm {
    Search { term: Term, pos: FieldPosition },
    Filter { term: Term },
}

impl DocumentTerm {
    pub fn term(&self) -> &Term {
        match self {
            Self::Search { term, .. } => term,
            Self::Filter { term } => term,
        }
    }

    pub fn position(&self) -> FieldPosition {
        match self {
            Self::Search { pos, .. } => *pos,
            // Filter fields are given a dummy position of 0
            Self::Filter { .. } => FieldPosition(0),
        }
    }

    pub fn field_id(&self) -> u32 {
        self.term().field().field_id()
    }
}

impl From<DocumentTerm> for Term {
    fn from(doc_term: DocumentTerm) -> Self {
        match doc_term {
            DocumentTerm::Search { term, .. } => term,
            DocumentTerm::Filter { term } => term,
        }
    }
}

pub type EditDistance = u8;

/// Used to represent the position of a term within a document. For now, this
/// position is wrt the document token stream so should only be used internally.
#[derive(Debug, Clone, Copy, Default, PartialOrd, Ord, Eq, PartialEq)]
pub struct FieldPosition(u32);

impl FieldPosition {
    #[cfg(test)]
    pub fn new_for_test(pos: u32) -> Self {
        Self(pos)
    }
}

impl From<FieldPosition> for u32 {
    fn from(value: FieldPosition) -> Self {
        value.0
    }
}

impl TryFrom<&Token> for FieldPosition {
    type Error = anyhow::Error;

    fn try_from(value: &Token) -> Result<Self, Self::Error> {
        Ok(Self(u32::try_from(value.position)?))
    }
}

#[derive(Clone)]
pub struct TantivySearchIndexSchema {
    analyzer: TextAnalyzer,

    internal_id_field: Field,
    ts_field: Field,
    creation_time_field: Field,

    search_field_path: FieldPath,
    pub search_field: Field,

    pub filter_fields: BTreeMap<FieldPath, Field>,

    pub(crate) schema: Schema,
}

impl From<&TantivySearchIndexSchema> for pb::searchlight::SearchIndexConfig {
    fn from(schema: &TantivySearchIndexSchema) -> Self {
        pb::searchlight::SearchIndexConfig {
            search_field_path: Some(schema.search_field_path.clone().into()),
            filter_fields: schema
                .filter_fields
                .keys()
                .cloned()
                .map(|p| p.into())
                .collect::<Vec<_>>(),
        }
    }
}

impl TantivySearchIndexSchema {
    pub fn new(index_config: &DeveloperTextIndexConfig) -> Self {
        let analyzer = convex_en();

        let mut schema_builder = Schema::builder();

        let internal_id_field = schema_builder.add_bytes_field(INTERNAL_ID_FIELD_NAME, FAST);
        let ts_field = schema_builder.add_u64_field(TS_FIELD_NAME, FAST);
        let creation_time_field = schema_builder.add_f64_field(CREATION_TIME_FIELD_NAME, FAST);

        let search_field_path = index_config.search_field.clone();
        let index_opts = TextFieldIndexing::default()
            .set_tokenizer(CONVEX_EN_TOKENIZER)
            .set_fieldnorms(true)
            .set_index_option(IndexRecordOption::WithFreqsAndPositions);
        let field_opts = TextOptions::default().set_indexing_options(index_opts);

        let field_name = format!("user/search/{}", String::from(search_field_path.clone()));
        let search_field = schema_builder.add_text_field(&field_name, field_opts);

        // NB: It's important that we iterate over `index_config.filter_fields` in
        // sorted order since tantivy assigns field ids in declaration order.
        let mut filter_fields = BTreeMap::new();
        for field_path in &index_config.filter_fields {
            // We store filter fields as the SHA256 hash of their index key.
            let field_name = format!("user/filter/{}", String::from(field_path.clone()));
            let field_opts = BytesOptions::default().set_indexed();
            let filter_field = schema_builder.add_bytes_field(&field_name, field_opts);
            filter_fields.insert(field_path.clone(), filter_field);
        }
        let schema = schema_builder.build();
        Self {
            analyzer,
            internal_id_field,
            ts_field,
            creation_time_field,

            search_field_path,
            search_field,

            filter_fields,
            schema,
        }
    }

    pub fn new_for_index(
        index: &Index,
        printable_index_name: &IndexName,
    ) -> anyhow::Result<TantivySearchIndexSchema> {
        let IndexConfig::Search {
            ref developer_config,
            ..
        } = index.metadata().config
        else {
            anyhow::bail!(ErrorMetadata::bad_request(
                "IndexNotASearchIndexError",
                format!("Index {} is not a search index", printable_index_name),
            ));
        };
        Ok(Self::new(developer_config))
    }

    pub fn to_index_config(&self) -> DeveloperTextIndexConfig {
        DeveloperTextIndexConfig {
            search_field: self.search_field_path.clone(),
            filter_fields: self.filter_fields.keys().cloned().collect(),
        }
    }

    fn filter_field_bytes(document: &ResolvedDocument, field_path: &FieldPath) -> Vec<u8> {
        let value = document.value().get_path(field_path);
        search_value_to_bytes(value)
    }

    /// This is a pretty wild over-estimate for documents with lots of shared
    /// terms. But it does at least provide some maximum value we can use
    /// when a super rough estimate is sufficient (e.g. capping the maximum
    /// size of a new segment).
    pub fn estimate_size(&self, document: &ResolvedDocument) -> u64 {
        let document_size = if let Some(ConvexValue::String(ref s)) =
            document.value().get_path(&self.search_field_path)
        {
            s.len()
        } else {
            0
        };
        let mut filter_field_sizes = 0;
        for field_path in self.filter_fields.keys() {
            let value = TantivySearchIndexSchema::filter_field_bytes(document, field_path);
            filter_field_sizes += value.len();
        }
        (document_size + filter_field_sizes) as u64
    }

    pub fn index_into_terms(
        &self,
        document: &ResolvedDocument,
    ) -> anyhow::Result<Vec<DocumentTerm>> {
        let _timer = metrics::index_into_terms_timer();

        let mut doc_terms = vec![];
        if let Some(ConvexValue::String(ref s)) = document.value().get_path(&self.search_field_path)
        {
            let mut token_stream = self.analyzer.token_stream(&s[..]);

            while let Some(token) = token_stream.next() {
                metrics::log_text_term(&token.text);

                doc_terms.push(DocumentTerm::Search {
                    term: Term::from_field_text(self.search_field, &token.text),
                    pos: FieldPosition::try_from(token)?,
                });
            }
        }
        for (field_path, tantivy_field) in &self.filter_fields {
            let value = TantivySearchIndexSchema::filter_field_bytes(document, field_path);
            metrics::log_filter_term(&value);
            doc_terms.push(DocumentTerm::Filter {
                term: Term::from_field_bytes(*tantivy_field, &value),
            });
        }
        Ok(doc_terms)
    }

    pub fn index_into_tantivy_document(
        &self,
        document: &ResolvedDocument,
        ts: Timestamp,
    ) -> TantivyDocument {
        let _timer = metrics::index_into_tantivy_document_timer();
        let mut tantivy_document = TantivyDocument::default();

        let internal_id_bytes = Vec::<u8>::from(document.id().internal_id());
        tantivy_document.add_bytes(self.internal_id_field, internal_id_bytes);

        tantivy_document.add_u64(self.ts_field, ts.into());

        let creation_time = document
            .creation_time()
            .expect("Document should have creation time");
        tantivy_document.add_f64(self.creation_time_field, creation_time.into());

        if let Some(ConvexValue::String(ref s)) = document.value().get_path(&self.search_field_path)
        {
            tantivy_document.add_text(self.search_field, s);
        }
        for (field_path, tantivy_field) in &self.filter_fields {
            let value = TantivySearchIndexSchema::filter_field_bytes(document, field_path);
            tantivy_document.add_bytes(*tantivy_field, value);
        }

        tantivy_document
    }

    pub fn document_lengths(&self, document: &TantivyDocument) -> DocumentLengths {
        let mut search_field = 0;
        if let Some(tantivy::schema::Value::Str(ref s)) = document.get_first(self.search_field) {
            search_field += s.len();
        }
        let mut filter_fields = BTreeMap::new();
        for (field_path, tantivy_field) in &self.filter_fields {
            if let Some(tantivy::schema::Value::Bytes(ref b)) = document.get_first(*tantivy_field) {
                filter_fields.insert(field_path.clone(), b.len());
            }
        }
        DocumentLengths {
            search_field,
            filter_fields,
        }
    }

    #[minitrace::trace]
    pub async fn search2<RT: Runtime>(
        &self,
        runtime: &RT,
        compiled_query: CompiledQuery,
        memory_index: &MemorySearchIndex,
        search_storage: Arc<dyn Storage>,
        segments: Vec<TextStorageKeys>,
        disk_index_ts: Timestamp,
        searcher: Arc<dyn Searcher>,
    ) -> anyhow::Result<RevisionWithKeys> {
        log_num_segments_searched_total(segments.len());

        // Step 1: Map the old `CompiledQuery` struct onto `TokenQuery`s.
        let mut token_queries = vec![];
        let num_text_query_terms = compiled_query.text_query.len() as u32;
        for query_term in compiled_query.text_query {
            let query = TokenQuery {
                max_distance: query_term.max_distance(),
                prefix: query_term.prefix(),
                term: query_term.into_term(),
            };
            token_queries.push(query);
        }
        let mut exist_filter_conditions = false;
        for CompiledFilterCondition::Must(term) in compiled_query.filter_conditions {
            exist_filter_conditions = true;
            let query = TokenQuery {
                term,
                max_distance: 0,
                prefix: false,
            };
            token_queries.push(query);
        }

        // Step 2: Execute the token queries across both the memory and disk indexes,
        // and merge the results to get the top terms.
        let mut match_aggregator = TokenMatchAggregator::new(MAX_UNIQUE_QUERY_TERMS);
        memory_index.query_tokens(&token_queries, &mut match_aggregator)?;
        let searcher_clone = searcher.clone();
        let search_storage_clone = search_storage.clone();
        let token_match_futs = segments.clone().into_iter().map(move |segment| {
            let searcher = searcher_clone.clone();
            let search_storage = search_storage_clone.clone();
            let token_queries = token_queries.clone();
            async move {
                searcher
                    .query_tokens(
                        search_storage,
                        segment,
                        token_queries,
                        MAX_UNIQUE_QUERY_TERMS,
                    )
                    .await
            }
        });
        let token_matches_by_segment: Vec<_> =
            try_join_buffer_unordered(runtime, "query_tokens", token_match_futs).await?;
        for segment_token_matches in token_matches_by_segment {
            anyhow::ensure!(segment_token_matches.is_sorted());
            for m in segment_token_matches {
                // Since each segment returns results in sorted order, we can stop early once we
                // know that we've already seen `MAX_UNIQUE_QUERY_TERMS` better results.
                if !match_aggregator.insert(m) {
                    break;
                }
            }
        }
        // Deduplicate terms, using the best distance for each term.
        let mut results_by_term = BTreeMap::new();
        for token_match in match_aggregator.into_results() {
            let sort_key = (
                token_match.distance,
                token_match.prefix,
                token_match.token_ord,
            );
            let existing_key = results_by_term.entry(token_match.term).or_insert(sort_key);

            // NB: Since OR and AND queries are on different fields, we can assume their
            // terms are disjoint. Assert this condition here since we're deduplicating
            // terms and taking their minimum `token_ord`, which could potentially lose
            // intersection conditions otherwise.
            let (_, _, existing_token_ord) = *existing_key;
            let existing_is_intersection = existing_token_ord >= num_text_query_terms;
            let is_intersection = token_match.token_ord >= num_text_query_terms;
            anyhow::ensure!(existing_is_intersection == is_intersection);

            *existing_key = cmp::min(*existing_key, sort_key);
        }
        let terms = results_by_term.keys().cloned().collect_vec();
        // If there are no matches, short-circuit and return an empty result.
        let no_and_tokens_present = results_by_term
            .iter()
            .filter(|(_, (_, _, token_ord))| *token_ord >= num_text_query_terms)
            .count()
            == 0;
        let no_filter_matches = exist_filter_conditions && no_and_tokens_present;
        if terms.is_empty() || no_filter_matches {
            return Ok(vec![]);
        }

        // Step 3: Given the terms we decided on, query BM25 statistics across all of
        // the indexes and merge their results.
        let terms_original = terms;
        let terms = terms_original.clone();
        let searcher_clone = searcher.clone();
        let search_storage_clone = search_storage.clone();
        let bm25_stats_futs = segments.clone().into_iter().map(move |segment| {
            let searcher = searcher_clone.clone();
            let search_storage = search_storage_clone.clone();
            let terms = terms.clone();
            async move {
                searcher
                    .query_bm25_stats(search_storage.clone(), segment, terms.clone())
                    .await
            }
        });
        let bm25_stats_per_segment: Vec<_> =
            try_join_buffer_unordered(runtime, "query_bm25_stats", bm25_stats_futs).await?;

        let mut bm25_stats =
            bm25_stats_per_segment
                .into_iter()
                .fold(Bm25Stats::empty(), |mut acc, stats| {
                    acc += stats;
                    acc
                });
        bm25_stats = memory_index.update_bm25_stats(disk_index_ts, &terms_original, bm25_stats)?;

        // Step 4: Decide on our posting list queries given the previous results.
        let mut or_terms = vec![];
        let mut and_terms = vec![];
        for (term, (distance, prefix, token_ord)) in results_by_term {
            if token_ord >= num_text_query_terms {
                anyhow::ensure!(distance == 0 && !prefix);
                and_terms.push(term);
            } else {
                let doc_frequency = *bm25_stats
                    .doc_frequencies
                    .get(&term)
                    .context("Missing term frequency")?;
                // TODO: Come up with a smarter way to boost scores based on edit distance.
                // Eventually this will be in user space so developers can tweak
                // it as they desire.
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
        }

        // or_terms is the set of text tokens that matches our query. and_terms only
        // filters these terms further. So if we have no or_terms, our result is
        // empty regardless of any matching and_terms.
        if or_terms.is_empty() {
            return Ok(vec![]);
        }

        // Step 5: Execute the posting list query against the memory index's tombstones
        // to know which `InternalId`s to exclude when querying the disk
        // indexes.
        let prepared_memory_query =
            memory_index.prepare_posting_list_query(&and_terms, &or_terms, &bm25_stats)?;
        let mut deleted_internal_ids = BTreeSet::new();
        if let Some(ref prepared_query) = prepared_memory_query {
            deleted_internal_ids = memory_index.query_tombstones(disk_index_ts, prepared_query)?;
        }
        let query = PostingListQuery {
            deleted_internal_ids,
            num_terms_by_field: bm25_stats.num_terms_by_field,
            num_documents: bm25_stats.num_documents,
            or_terms,
            and_terms,
            max_results: MAX_CANDIDATE_REVISIONS,
        };

        // Step 6: Query the posting lists across the indexes and take the best
        // results.
        let mut match_aggregator = PostingListMatchAggregator::new(MAX_CANDIDATE_REVISIONS);
        if let Some(ref prepared_query) = prepared_memory_query {
            memory_index.query_posting_lists(
                disk_index_ts,
                prepared_query,
                &mut match_aggregator,
            )?;
        }
        let posting_list_futs = segments.into_iter().map(move |segment| {
            let searcher = searcher.clone();
            let search_storage = search_storage.clone();
            let query = query.clone();
            async move {
                searcher
                    .query_posting_lists(search_storage.clone(), segment, query.clone())
                    .await
            }
        });
        let posting_list_results_per_segment: Vec<_> =
            try_join_buffer_unordered(runtime, "query_posting_lists", posting_list_futs).await?;
        for segment_matches in posting_list_results_per_segment {
            for m in segment_matches {
                if !match_aggregator.insert(m) {
                    break;
                }
            }
        }

        // Step 7: Convert the matches into the final result format.
        let mut result = vec![];
        for m in match_aggregator.into_results() {
            let candidate = CandidateRevision {
                score: m.bm25_score,
                id: m.internal_id,
                ts: m.ts,
                creation_time: m.creation_time,
            };
            let index_fields = vec![
                Some(ConvexValue::Float64(-f64::from(m.bm25_score))),
                Some(ConvexValue::Float64(-f64::from(m.creation_time))),
                Some(ConvexValue::Bytes(
                    Vec::<u8>::from(m.internal_id)
                        .try_into()
                        .expect("Could not convert internal ID to value"),
                )),
            ];
            let bytes = values_to_bytes(&index_fields);
            let index_key_bytes = IndexKeyBytes(bytes);
            result.push((candidate, index_key_bytes));
        }
        Ok(result)
    }

    pub async fn search<RT: Runtime>(
        &self,
        runtime: &RT,
        compiled_query: CompiledQuery,
        memory_index: &MemorySearchIndex,
        search_storage: Arc<dyn Storage>,
        disk_index: &ObjectKey,
        disk_index_ts: Timestamp,
        searcher: Arc<dyn Searcher>,
    ) -> anyhow::Result<RevisionWithKeys> {
        if *USE_MULTI_SEGMENT_SEARCH_QUERY {
            let number_of_segments = searcher
                .number_of_segments(search_storage.clone(), disk_index.clone())
                .await?;
            let segments = (0..number_of_segments)
                .map(|i| TextStorageKeys::SingleSegment {
                    storage_key: disk_index.clone(),
                    segment_ord: i as u32,
                })
                .collect();
            return self
                .search2(
                    runtime,
                    compiled_query,
                    memory_index,
                    search_storage,
                    segments,
                    disk_index_ts,
                    searcher,
                )
                .await;
        }
        // 1. Fetch the memory index matches for each QueryTerm in the query and bound.
        let (term_shortlist, term_shortlist_ids) =
            memory_index.bound_and_evaluate_query_terms(&compiled_query.text_query);

        // 2. For the shortlisted terms, get the BM25 statistics for each term in the
        //    memory index.
        let memory_stats_diff =
            memory_index.bm25_statistics_diff(disk_index_ts, &term_shortlist.terms())?;

        // 3. Query memory index tombstones to count overfetch_delta
        //
        // Our goal is to end up with the top MAX_CANDIDATE_REVISIONS.
        // Some of the ones in searchlight will be filtered out if they were edited
        // since disk_index_ts. Count how many that is and fetch extra!
        let tombstoned_matches = {
            let term_list_query = memory_index.build_term_list_bitset_query(
                &compiled_query,
                &term_shortlist,
                &term_shortlist_ids,
            );
            memory_index.tombstoned_matches(disk_index_ts, &term_list_query)?
        };
        let overfetch_delta = tombstoned_matches.len();
        metrics::log_searchlight_overfetch_delta(overfetch_delta);
        let limit = MAX_CANDIDATE_REVISIONS + overfetch_delta;

        // 4. Do disk query
        let search_results = {
            let timer = metrics::searchlight_client_execute_timer(&SEARCHLIGHT_CLUSTER_NAME);
            let results = searcher
                .execute_query(
                    search_storage,
                    disk_index,
                    self,
                    compiled_query.clone(),
                    memory_stats_diff,
                    term_shortlist,
                    limit,
                )
                .await?;
            metrics::finish_searchlight_client_execute(timer, &results);
            results
        };

        // 5. Do memory index query
        let combined_term_shortlist = search_results.combined_shortlisted_terms;
        let combined_term_ids =
            memory_index.evaluate_shortlisted_query_terms(&combined_term_shortlist);
        let memory_revisions = {
            let term_list_query = memory_index.build_term_list_bitset_query(
                &compiled_query,
                &combined_term_shortlist,
                &combined_term_ids,
            );
            let term_weights = build_term_weights(
                &combined_term_shortlist,
                &combined_term_ids,
                &term_list_query,
                search_results.combined_statistics,
            )?;
            memory_index.query(
                disk_index_ts,
                &term_list_query,
                &combined_term_ids,
                &term_weights,
            )?
        };

        // 6. Filter out tombstones
        let current_disk_revisions = search_results
            .results
            .into_iter()
            .filter(|revision| !tombstoned_matches.contains(&revision.revision.id));

        // 7. Use Bm25 to score top retrieval results
        let mut revisions_with_keys: Vec<_> = memory_revisions
            .into_iter()
            .chain(current_disk_revisions)
            .map(|candidate| {
                (
                    (
                        TotalOrdF64::from(-f64::from(candidate.revision.score)),
                        TotalOrdF64::from(-f64::from(candidate.revision.creation_time)),
                        Vec::<u8>::from(candidate.revision.id),
                    ),
                    candidate,
                )
            })
            .collect();
        revisions_with_keys.sort_by_key(|(key, _)| key.clone());
        let original_len = revisions_with_keys.len();
        revisions_with_keys.truncate(MAX_CANDIDATE_REVISIONS);
        metrics::log_num_discarded_revisions(original_len - revisions_with_keys.len());

        // 8. Rank results
        let ranker = Ranker::create(&compiled_query.text_query, &combined_term_shortlist);
        let mut ranked_revisions: Vec<_> = revisions_with_keys
            .into_iter()
            .map(|(_, candidate)| {
                // Search results are in decreasing score order and then tie break
                // with decreasing creation time (newest first).
                //
                // This isn't a true index key -- notably, the last value is not the
                // document ID, but we're just using the index key bytes for sorting
                // and paginating search results within a table.
                let ranking_score = ranker.score(&candidate);

                let index_fields = vec![
                    Some(ConvexValue::Float64(-f64::from(ranking_score))),
                    Some(ConvexValue::Float64(-f64::from(
                        candidate.revision.creation_time,
                    ))),
                    Some(ConvexValue::Bytes(
                        Vec::<u8>::from(candidate.revision.id)
                            .try_into()
                            .expect("Could not convert internal ID to value"),
                    )),
                ];
                let bytes = values_to_bytes(&index_fields);
                let index_key_bytes = IndexKeyBytes(bytes);
                (CandidateRevision::from(candidate), index_key_bytes)
            })
            .collect();
        ranked_revisions.sort_by_key(|(_, key)| key.clone());

        Ok(ranked_revisions)
    }

    fn compile_tokens_with_typo_tolerance(
        search_field: Field,
        tokens: &Vec<String>,
    ) -> anyhow::Result<Vec<QueryTerm>> {
        let mut res = vec![];

        let mut it = tokens.iter().peekable();
        while let Some(text) = it.next() {
            let term = Term::from_field_text(search_field, text);
            anyhow::ensure!(term.as_str().is_some(), "Term was not valid UTF8");

            let char_count = text.chars().count();
            let is_prefix = it.peek().is_none();
            let num_typos = if char_count <= EXACT_SEARCH_MAX_WORD_LENGTH {
                0
            } else if char_count <= SINGLE_TYPO_SEARCH_MAX_WORD_LENGTH {
                1
            } else {
                2
            };

            if num_typos == 0 && !is_prefix {
                res.push(QueryTerm::Exact(term))
            } else {
                res.push(QueryTerm::Fuzzy {
                    term,
                    max_distance: num_typos,
                    prefix: is_prefix,
                })
            }
        }
        Ok(res)
    }

    pub fn compile(
        &self,
        query: &InternalSearch,
        version: SearchVersion,
    ) -> anyhow::Result<(CompiledQuery, QueryReads)> {
        let timer = metrics::compile_timer();

        let mut search_text: Option<&str> = None;
        let mut filter_conditions = Vec::new();
        let mut filter_reads = Vec::new();
        for filter in query.filters.iter() {
            match filter {
                InternalSearchFilterExpression::Search(field_path, text_query) => {
                    if *field_path != self.search_field_path {
                        anyhow::bail!(ErrorMetadata::bad_request(
                            "IncorrectSearchField",
                            format!(
                                "Search query against {} contains a search filter against {:?}, \
                                 which doesn't match the indexed `searchField` {:?}.",
                                query.printable_index_name()?,
                                field_path,
                                self.search_field_path,
                            ),
                        ))
                    }
                    if search_text.is_some() {
                        anyhow::bail!(ErrorMetadata::bad_request(
                            "DuplicateSearchFiltersError",
                            format!(
                                "Search query against {} contains multiple search filters against \
                                 {field_path:?}. Only one is allowed.",
                                query.printable_index_name()?,
                            )
                        ))
                    }
                    search_text = Some(text_query)
                },
                InternalSearchFilterExpression::Eq(field_path, value) => {
                    let Some(field) = self.filter_fields.get(field_path) else {
                        anyhow::bail!(ErrorMetadata::bad_request(
                            "IncorrectFilterFieldError",
                            format!(
                                "Search query against {} contains an equality filter on \
                                 {field_path:?} but that field isn't indexed for filtering in \
                                 `filterFields`.",
                                query.printable_index_name()?,
                            )
                        ))
                    };
                    let term = Term::from_field_bytes(*field, value);
                    filter_conditions.push(CompiledFilterCondition::Must(term));
                    filter_reads.push(FilterConditionRead::Must(field_path.clone(), value.clone()));
                },
            }
        }

        let Some(search_text) = search_text else {
            anyhow::bail!(ErrorMetadata::bad_request(
                "MissingSearchFilterError",
                format!(
                    "Search query against {} does not contain any search filters. You must \
                     include a search filter like `q.search(\"{:?}\", searchText)`.",
                    query.printable_index_name()?,
                    self.search_field_path,
                )
            ))
        };

        let mut token_stream = self.analyzer.token_stream(search_text);
        let mut tokens = vec![];
        // TODO(CX-5693): Consider how/if we should surface this to developers.
        while tokens.len() < MAX_QUERY_TERMS
            && let Some(token) = token_stream.next()
        {
            tokens.push(token.text.clone());
        }
        if tokens.len() == MAX_QUERY_TERMS && token_stream.next().is_some() {
            log_search_token_limit_exceeded();
        }

        let text_query = match version {
            SearchVersion::V1 => tokens
                .iter()
                .map(|text| {
                    let term = Term::from_field_text(self.search_field, text);
                    anyhow::ensure!(term.as_str().is_some(), "Term was not valid UTF8");
                    Ok(QueryTerm::Exact(term))
                })
                .collect::<anyhow::Result<Vec<_>>>()?,
            // Only the V2 search codepath can generate QueryTerm::Fuzzy
            SearchVersion::V2 => {
                Self::compile_tokens_with_typo_tolerance(self.search_field, &tokens)?
            },
        };

        let text_reads = text_query
            .clone()
            .into_iter()
            .map(|t| {
                anyhow::Ok(TextQueryTermRead::new(
                    self.search_field_path.clone(),
                    TextQueryTerm::try_from(t)?,
                ))
            })
            .collect::<anyhow::Result<_>>()?;

        if filter_conditions.len() > MAX_FILTER_CONDITIONS {
            anyhow::bail!(ErrorMetadata::bad_request(
                "TooManyFilterConditionsInSearchQueryError",
                format!(
                    "Search query against {} has too many filter conditions. Max: {} Actual: {}",
                    query.printable_index_name()?,
                    MAX_FILTER_CONDITIONS,
                    filter_conditions.len()
                )
            ))
        }
        let query = CompiledQuery {
            text_query,
            filter_conditions,
        };
        let reads = QueryReads::new(text_reads, filter_reads.into());
        metrics::log_compiled_query(&query);

        timer.finish();
        Ok((query, reads))
    }
}

pub struct DocumentLengths {
    pub search_field: usize,
    pub filter_fields: BTreeMap<FieldPath, usize>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SearchFileType {
    VectorSegment,
    FragmentedVectorSegment,
    VectorDeletedBitset,
    VectorIdTracker,
    Text,
    TextIdTracker,
    TextAliveBitset,
    TextDeletedTerms,
}

#[cfg(test)]
mod test {
    use std::collections::BTreeSet;

    use common::bootstrap_model::index::text_index::DeveloperTextIndexConfig;

    use crate::{
        TantivySearchIndexSchema,
        SEARCH_FIELD_ID,
    };

    /// DO NOT CHANGE CONSTANTS!
    /// This test ensures that we don't accidentally change our field IDs in
    /// tantivy.
    #[test]
    fn test_field_ids_dont_change() -> anyhow::Result<()> {
        let schema = TantivySearchIndexSchema::new(&DeveloperTextIndexConfig {
            search_field: "mySearchField".parse()?,
            filter_fields: BTreeSet::new(),
        });
        assert_eq!(schema.internal_id_field.field_id(), 0);
        assert_eq!(schema.ts_field.field_id(), 1);
        assert_eq!(schema.creation_time_field.field_id(), 2);
        assert_eq!(schema.search_field.field_id(), SEARCH_FIELD_ID);
        Ok(())
    }
}

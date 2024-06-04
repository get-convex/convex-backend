use std::collections::{
    BTreeMap,
    BTreeSet,
    BinaryHeap,
};

use anyhow::Context;
use common::{
    document::CreationTime,
    types::{
        Timestamp,
        WriteTimestamp,
    },
};
use itertools::Itertools;
use pb::searchlight::QueryResponse;
use tantivy::{
    fieldnorm::FieldNormReader,
    postings::SegmentPostings,
    query::{
        Bm25StatisticsProvider,
        Bm25Weight,
        Scorer,
    },
    schema::{
        Field,
        IndexRecordOption,
    },
    DocAddress,
    DocId,
    DocSet,
    Postings,
    Score,
    Searcher,
    SegmentOrdinal,
    Term,
    TERMINATED,
};
use value::{
    sorting::TotalOrdF64,
    InternalId,
};

use crate::{
    constants::{
        MAX_FUZZY_MATCHES_PER_QUERY_TERM,
        MAX_POSITIONS_PER_MATCHED_TERM,
    },
    intersection::go_to_first_doc,
    levenshtein_dfa::{
        build_fuzzy_dfa,
        LevenshteinDfaWrapper,
    },
    metrics,
    query::{
        CandidateRevisionPositions,
        CompiledFilterCondition,
        CompiledQuery,
        QueryTerm,
        ShortlistId,
        TermShortlist,
    },
    scoring::{
        bm25_weight_boost_for_edit_distance,
        Bm25StatisticsDiff,
    },
    CandidateRevision,
    EditDistance,
    CREATION_TIME_FIELD_NAME,
    INTERNAL_ID_FIELD_NAME,
    TS_FIELD_NAME,
};

/// Results from tantivy.
///
/// This includes both the candidates that match the search query along with
/// additional statistics so we can score results in the `MemorySearchIndex`
#[derive(Clone, Debug, PartialEq)]
pub struct SearchQueryResult {
    pub results: Vec<CandidateRevisionPositions>,
    pub combined_statistics: Bm25StatisticsDiff,
    pub combined_shortlisted_terms: TermShortlist,
}

impl SearchQueryResult {
    pub fn empty() -> Self {
        SearchQueryResult {
            results: vec![],
            combined_statistics: Bm25StatisticsDiff {
                term_statistics: BTreeMap::new(),
                num_documents_diff: 0,
                num_search_tokens_diff: 0,
            },
            combined_shortlisted_terms: TermShortlist::new(BTreeMap::new()),
        }
    }

    pub fn try_from_query_response(
        query_response: QueryResponse,
        search_field: Field,
    ) -> anyhow::Result<Self> {
        Ok(Self {
            results: query_response
                .results
                .into_iter()
                .map(|r| {
                    anyhow::Ok(CandidateRevisionPositions {
                        revision: CandidateRevision::try_from(
                            r.revision.context("Revision missing")?,
                        )?,
                        positions: r
                            .positions
                            .into_iter()
                            .map(|p| Ok((ShortlistId::try_from(p.shortlist_id)?, p.positions)))
                            .collect::<anyhow::Result<BTreeMap<_, _>>>()?,
                    })
                })
                .collect::<anyhow::Result<Vec<_>>>()?,
            combined_statistics: query_response
                .combined_statistics
                .ok_or_else(|| anyhow::anyhow!("No BM25 statistics in search QueryResponse"))?
                .into(),
            combined_shortlisted_terms: TermShortlist::try_from_proto(
                query_response
                    .combined_shortlisted_terms
                    .ok_or_else(|| anyhow::anyhow!("No shortlisted terms in QueryResponse"))?,
                search_field,
            )?,
        })
    }
}

impl TryFrom<SearchQueryResult> for QueryResponse {
    type Error = anyhow::Error;

    fn try_from(search_result: SearchQueryResult) -> Result<Self, Self::Error> {
        Ok(QueryResponse {
            results: search_result
                .results
                .into_iter()
                .map(pb::searchlight::CandidateRevisionPositions::from)
                .collect::<Vec<_>>(),
            combined_statistics: Some(search_result.combined_statistics.into()),
            combined_shortlisted_terms: Some(pb::searchlight::TermShortlist::from(
                search_result.combined_shortlisted_terms,
            )),
        })
    }
}

/// Manually retrieving documents using Tantivy
/// 1. Combine the statistics between memory and disk index for consistent BM25
///    scores
/// 2. Fetch all terms in the Tantivy term dictionary that match a query term or
///    filter term
/// 3. Intersect the posting lists of filter terms
/// 4. Union the filter intersection above with query terms
/// 5. [Deferred] Apply Block-Max WAND to efficiently iterate the union (?)
/// 6. For each candidate doc matched by the posting list iteration, fetch the
///    scores and positions of the candidate and add it to Top K.
///
/// To fetch positions AND scores for matched documents, we need to iterate
/// implementors of the `Postings` and `Scorer` traits. The main implementation
/// is `TermPostingsScorer`.

/// This combination trait is needed to return a Box<dyn Postings + Scorer>
/// which rust doesn't allow unless combined by a trait with supertraits.
trait PostingsScorer: Postings + Scorer {
    fn get_positions(&mut self) -> Vec<u32> {
        let mut output = vec![];
        self.positions(&mut output);
        // TODO: is there a way (perhaps requiring a Tantivy fork) to not fetch all the
        // positions
        output.truncate(MAX_POSITIONS_PER_MATCHED_TERM);
        output
    }
}

impl<T: Postings + Scorer> PostingsScorer for T {}

/// The main scorer, also used for Block-MAX WAND
struct TermPostingsScorer {
    postings: SegmentPostings,
    fieldnorm_reader: FieldNormReader,
    bm25_weight: Bm25Weight,
    boost: Score,
}

impl DocSet for TermPostingsScorer {
    fn advance(&mut self) -> DocId {
        self.postings.advance()
    }

    fn seek(&mut self, target: DocId) -> DocId {
        self.postings.seek(target)
    }

    fn doc(&self) -> DocId {
        self.postings.doc()
    }

    fn size_hint(&self) -> u32 {
        self.postings.size_hint()
    }
}

impl Postings for TermPostingsScorer {
    fn term_freq(&self) -> u32 {
        self.postings.term_freq()
    }

    fn positions_with_offset(&mut self, offset: u32, output: &mut Vec<u32>) {
        self.postings.positions_with_offset(offset, output)
    }
}

impl Scorer for TermPostingsScorer {
    fn score(&mut self) -> Score {
        let fieldnorm_id = self.fieldnorm_reader.fieldnorm_id(self.doc());
        let term_freq = self.term_freq();
        self.bm25_weight.score(fieldnorm_id, term_freq) * self.boost
    }
}

/// Intersection where each TDocSet and TPostingsScorer is iterated in
/// intersection, but scores are extractable from `posting_scorer_docset`.
struct IntersectionSingleScorer<TDocSet: DocSet + 'static, TPostingsScorer: Postings + Scorer> {
    required_docsets: Vec<TDocSet>,
    posting_scorer_docset: TPostingsScorer,
}

impl<TDocSet: DocSet + 'static, TPostingsScorer: Postings + Scorer>
    IntersectionSingleScorer<TDocSet, TPostingsScorer>
{
    #[allow(clippy::new_ret_no_self)]
    fn new(
        mut required_docsets: Vec<TDocSet>,
        mut posting_scorer_docset: TPostingsScorer,
    ) -> Box<dyn PostingsScorer> {
        if required_docsets.is_empty() {
            return Box::new(posting_scorer_docset);
        }
        let mut docsets: Vec<&mut dyn DocSet> = Vec::with_capacity(required_docsets.len() + 1);
        for docset in &mut required_docsets {
            docsets.push(docset);
        }
        docsets.push(&mut posting_scorer_docset);

        // Sort docsets by size_hint so, in go_to_first_doc, we always select smaller
        // docsets first, increasing likelihood of element being in
        // intersection.
        docsets.sort_by_key(|ds| ds.size_hint());

        let doc = go_to_first_doc(&mut docsets[..]);
        debug_assert!(docsets.iter().map(DocSet::doc).all(|d| d == doc));

        Box::new(Self {
            required_docsets,
            posting_scorer_docset,
        })
    }

    #[inline]
    fn at(&self, idx: usize) -> &dyn DocSet {
        debug_assert!(idx <= self.required_docsets.len());
        if idx < self.required_docsets.len() {
            &self.required_docsets[idx]
        } else {
            &self.posting_scorer_docset
        }
    }

    #[inline]
    fn at_mut(&mut self, idx: usize) -> &mut dyn DocSet {
        debug_assert!(idx <= self.required_docsets.len());
        if idx < self.required_docsets.len() {
            &mut self.required_docsets[idx]
        } else {
            &mut self.posting_scorer_docset
        }
    }

    fn len(&self) -> usize {
        self.required_docsets.len() + 1
    }
}

impl<TDocSet: DocSet + 'static, TPostingsScorer: Postings + Scorer> DocSet
    for IntersectionSingleScorer<TDocSet, TPostingsScorer>
{
    fn advance(&mut self) -> DocId {
        let mut candidate = self.at_mut(0).advance();
        let mut candidate_idx = 0_usize;
        let num_docsets = self.len();

        'outer: loop {
            for i in 0..num_docsets {
                if i == candidate_idx {
                    continue;
                }
                let doc = self.at_mut(i).seek(candidate);
                if doc != candidate {
                    candidate = doc;
                    candidate_idx = i;
                    continue 'outer;
                }
            }
            // If we get here, everyone is equal!
            break;
        }

        debug_assert!((0..num_docsets)
            .map(|v| self.at(v).doc())
            .all(|d| d == candidate));
        candidate
    }

    fn seek(&mut self, target: DocId) -> DocId {
        self.at_mut(0).seek(target);
        let mut docsets: Vec<&mut dyn DocSet> = Vec::with_capacity(self.len());
        for docset in &mut self.required_docsets {
            docsets.push(docset);
        }
        docsets.push(&mut self.posting_scorer_docset);
        let doc = go_to_first_doc(&mut docsets[..]);
        debug_assert!(docsets.iter().map(DocSet::doc).all(|d| d == doc));
        debug_assert!(doc >= target);
        doc
    }

    fn doc(&self) -> DocId {
        self.posting_scorer_docset.doc()
    }

    fn size_hint(&self) -> u32 {
        self.posting_scorer_docset
            .size_hint()
            .min(self.at(0).size_hint())
    }
}

impl<TDocSet: DocSet + 'static, TPostingsScorer: Postings + Scorer> Scorer
    for IntersectionSingleScorer<TDocSet, TPostingsScorer>
{
    fn score(&mut self) -> Score {
        self.posting_scorer_docset.score()
    }
}

impl<TDocSet: DocSet + 'static, TPostingsScorer: Postings + Scorer> Postings
    for IntersectionSingleScorer<TDocSet, TPostingsScorer>
{
    fn term_freq(&self) -> u32 {
        self.posting_scorer_docset.term_freq()
    }

    fn positions_with_offset(&mut self, offset: u32, output: &mut Vec<u32>) {
        self.posting_scorer_docset
            .positions_with_offset(offset, output)
    }
}

/// Disk index shortlisting terms to consider. This is important to bound the
/// number of fuzzy matches considered.
fn tantivy_bound_and_evaluate_query_terms(
    searcher: &Searcher,
    search_field: Field,
    query: &CompiledQuery,
    memory_shortlisted_terms: TermShortlist,
) -> anyhow::Result<TermShortlist> {
    // For each segment, get it's Top K matching terms for each query term.
    // We don't actually care about per-term matches, we just want the Top K
    // globally so we conceptually just want to store a single
    // BTreeMap<QueryTerm, BTreeSet<(EditDistance, Term)>> where the BTreeSet acts
    // as a heap.
    let mut matches: BTreeMap<QueryTerm, BTreeSet<(EditDistance, Term)>> = memory_shortlisted_terms
        .query_term_shortlist_items
        .clone()
        .into_iter()
        .map(|(query_term, matches)| {
            Ok((
                query_term,
                matches
                    .into_iter()
                    .map(|(dist, id)| Ok((dist, memory_shortlisted_terms.get_term(id)?.clone())))
                    .collect::<anyhow::Result<BTreeSet<_>>>()?,
            ))
        })
        .collect::<anyhow::Result<BTreeMap<_, _>>>()?;

    let deduplicated_query_terms: BTreeSet<&QueryTerm> = query.text_query.iter().collect();

    for segment_reader in searcher.segment_readers() {
        let inverted_index = segment_reader.inverted_index(search_field)?;

        for query_term in &deduplicated_query_terms {
            let match_set = matches.entry((*query_term).clone()).or_default();
            // Exact queries are already populated from memory index, just need to fetch
            // fuzzy matches.
            match query_term {
                QueryTerm::Fuzzy {
                    term,
                    max_distance,
                    prefix,
                } => {
                    // TODO: have types in QueryTerm reflect that they are Term strings so we don't
                    // have to do this check all the time.
                    let term_str = term.as_str().context("FuzzyTerm was not a string")?;
                    let dfa = build_fuzzy_dfa(term_str, *max_distance, *prefix);
                    let dfa_compat = LevenshteinDfaWrapper(&dfa);
                    let term_dict = inverted_index.terms();
                    let mut term_stream = term_dict.search(dfa_compat.clone()).into_stream()?;

                    while term_stream.advance() {
                        let matched_term_bytes = term_stream.key();
                        let term_str = std::str::from_utf8(matched_term_bytes)
                            .context("Tantivy term match was not a valid string")?;
                        let matched_term = Term::from_field_text(search_field, term_str);

                        // TODO: extend Tantivy::TermStreamer to a TermStreamerWithState to avoid
                        // recomputing distance again here.
                        // This comment on a Tantivy open issue describes how to approach this:
                        // https://github.com/quickwit-oss/tantivy/issues/563#issuecomment-801444469
                        let edit_distance = dfa_compat.0.eval(matched_term_bytes).to_u8();

                        match_set.insert((edit_distance, matched_term));
                        if match_set.len() > MAX_FUZZY_MATCHES_PER_QUERY_TERM {
                            match_set.pop_last();
                        }
                    }
                },
                QueryTerm::Exact(term) => {
                    if match_set.is_empty() && inverted_index.get_term_info(term)?.is_some() {
                        match_set.insert((0, term.clone()));
                    }
                },
            }
        }
    }

    Ok(TermShortlist::new(matches))
}

/// Entrypoint for querying Tantivy
#[minitrace::trace]
pub fn query_tantivy(
    search_field: Field,
    query: &CompiledQuery,
    searcher: &Searcher,
    memory_statistics_diff: Bm25StatisticsDiff,
    memory_shortlisted_terms: TermShortlist,
    limit: usize,
) -> anyhow::Result<SearchQueryResult> {
    let timer = metrics::query_tantivy_timer();

    // Gather shortlisted list of terms
    let combined_shortlisted_terms = tantivy_bound_and_evaluate_query_terms(
        searcher,
        search_field,
        query,
        memory_shortlisted_terms,
    )?;

    // Gather term statistics needed for BM25 scoring of documents containing query
    // terms
    let mut term_statistics = BTreeMap::new();
    for (_, text_term) in combined_shortlisted_terms.ids_and_terms() {
        match text_term.as_str() {
            None => anyhow::bail!(
                "Expected text term to have text. Actual type: {:?}",
                text_term.typ()
            ),
            Some(text) => {
                term_statistics.insert(text.to_string(), searcher.doc_freq(text_term)?.try_into()?)
            },
        };
    }

    let combined_statistics = {
        let timer = metrics::query_tantivy_statistics_timer();
        let mut total_num_search_tokens = 0i64;
        let mut total_num_documents = 0i64;
        for segment_reader in searcher.segment_readers() {
            let inverted_index = segment_reader.inverted_index(search_field)?;
            // This includes deleted documents!
            // If documents are deleted in our tantivy disk index, they will still
            // count towards these metrics. This isn't correct but it matches
            // tantivy's behavior. Also as of writing we never delete documents.
            total_num_search_tokens += i64::try_from(inverted_index.total_num_tokens())?;
            total_num_documents += i64::from(segment_reader.max_doc());
        }
        metrics::log_num_segments(searcher.segment_readers().len());

        let disk_statistics_diff = Bm25StatisticsDiff {
            num_documents_diff: total_num_documents,
            num_search_tokens_diff: total_num_search_tokens,
            term_statistics,
        };
        let result = disk_statistics_diff.combine(memory_statistics_diff);
        timer.finish();
        result
    };

    // Do the query itself
    let revisions = query_tantivy_with_stats_diff(
        searcher,
        query,
        &combined_statistics,
        &combined_shortlisted_terms,
        &search_field,
        limit,
    )?;

    metrics::finish_query_tantivy(timer, revisions.len());
    Ok(SearchQueryResult {
        results: revisions,
        combined_statistics,
        combined_shortlisted_terms,
    })
}

/// Identifies a term that matched within a document.
#[derive(Debug, Clone)]
struct Match {
    shortlist_id: ShortlistId,
    score: Score,
    positions: Vec<u32>,
}

/// Query workhorse using the Tantivy low-level interface. This function
/// involves significant disk I/O.
///
/// TODO: refactor to not fetch positions upfront and break out into smaller
/// functions
fn query_tantivy_with_stats_diff(
    searcher: &Searcher,
    query: &CompiledQuery,
    combined_statistics: &Bm25StatisticsDiff,
    combined_shortlisted_terms: &TermShortlist,
    search_field: &Field,
    limit: usize,
) -> anyhow::Result<Vec<CandidateRevisionPositions>> {
    let mut doc_id_matches: BTreeMap<DocAddress, Vec<Match>> = BTreeMap::new();
    let timer = metrics::query_tantivy_search_timer();

    // Global statistics for BM25
    let average_fieldnorm = combined_statistics.total_num_tokens(*search_field)? as Score
        / combined_statistics.total_num_docs()? as Score;
    let total_num_docs = combined_statistics.total_num_docs()?;

    // Compute score boosts mapping
    let mut score_boosts = BTreeMap::<ShortlistId, Score>::new();
    for text_query in &query.text_query {
        let matches = combined_shortlisted_terms.get_shortlisted_terms_for_query_term(text_query);

        for (dist, id) in matches {
            *score_boosts.entry(*id).or_insert(0.) += bm25_weight_boost_for_edit_distance(*dist);
        }
    }

    // Iterate segments and find all documents and the terms they match
    'outer: for (segment_ord, segment_reader) in searcher.segment_readers().iter().enumerate() {
        // Fetch the TermInfos of all terms in the term dictionary that match the query
        let mut term_infos = vec![];
        let inverted_index = segment_reader.inverted_index(*search_field)?;
        for (shortlist_id, term) in combined_shortlisted_terms.ids_and_terms() {
            if let Some(term_info) = inverted_index.get_term_info(term)? {
                term_infos.push((term_info, term, shortlist_id as ShortlistId));
            }
        }

        // Fetch the filter field posting lists
        let mut filter_postings = vec![];
        for CompiledFilterCondition::Must(filter) in &query.filter_conditions {
            let filter_inverted_index = segment_reader.inverted_index(filter.field())?;
            let Some(filter_posting) =
                filter_inverted_index.read_postings(filter, IndexRecordOption::Basic)?
            else {
                // Terminate early - no possible matches in this segment since some filter
                // values don't exist.
                continue 'outer;
            };
            filter_postings.push(filter_posting);
        }

        // Iterate the TermInfos, fetch their posting lists, and score their respective
        // documents
        // TODO(rakeeb): do BlockWAND optimization and filtering here
        for (term_info, term, shortlist_id) in term_infos.into_iter() {
            // Construct TermPostingScorer
            let postings = inverted_index.read_postings_from_terminfo(
                &term_info,
                IndexRecordOption::WithFreqsAndPositions,
            )?;
            let fieldnorm_reader = segment_reader.get_fieldnorms_reader(*search_field)?;
            let bm25_weight = Bm25Weight::for_one_term(
                combined_statistics.doc_freq(term)?,
                total_num_docs,
                average_fieldnorm,
            );
            let term_postings_scorer = TermPostingsScorer {
                postings,
                fieldnorm_reader,
                bm25_weight,
                boost: *score_boosts.get(&shortlist_id).unwrap_or(&1.),
            };

            // Construct IntersectionSingleScorer
            let mut intersection_scorer =
                IntersectionSingleScorer::new(filter_postings.clone(), term_postings_scorer);

            // Iterate intersection and get scores
            let mut doc_id = intersection_scorer.doc();
            while doc_id != TERMINATED {
                // TODO(rakeeb): defer fetching positions until we've settled on the top `limit`
                // results to reduce disk I/O
                let positions = intersection_scorer.get_positions();
                let term_match = Match {
                    shortlist_id,
                    score: intersection_scorer.score(),
                    positions,
                };
                // Add Match to global set of results
                doc_id_matches
                    .entry(DocAddress::new(segment_ord as SegmentOrdinal, doc_id))
                    .or_default()
                    .push(term_match);
                doc_id = intersection_scorer.advance();
            }
        }
    }
    timer.finish();

    // For each document and its matches, fetch a BM25 score for that document.
    // We want to end up with the top `limit` documents by this score.
    let mut docs_heap = BinaryHeap::new();
    for (doc_address, term_matches) in doc_id_matches.into_iter() {
        // BM25 is additive, so we can just sum here
        let score = term_matches.iter().map(|m| m.score).sum::<Score>();
        let matches_by_shortlist_id: BTreeMap<_, _> = term_matches
            .into_iter()
            .map(|m| (m.shortlist_id, m.positions))
            .collect();

        // Push std::cmp::Reverse so that the heap is a min-heap, not a max-heap
        docs_heap.push(std::cmp::Reverse((
            TotalOrdF64::from(score as f64),
            doc_address,
            matches_by_shortlist_id,
        )));

        if docs_heap.len() > limit {
            docs_heap.pop();
        }
    }

    // Top docs sorted in ascending order
    let top_docs = docs_heap
        .into_sorted_vec()
        .into_iter()
        .map(|r| r.0)
        .rev()
        .collect_vec();
    let top_docs_len = top_docs.len();

    let mut top_docs_by_segment_ord = BTreeMap::new();
    for (score, doc, positions) in top_docs {
        top_docs_by_segment_ord
            .entry(doc.segment_ord)
            .or_insert_with(Vec::new)
            .push((doc.doc_id, score, positions));
    }

    // For each candidate, load its `DocumentId` and `Timestamp` from fast fields.
    let mut revisions = Vec::with_capacity(top_docs_len);
    {
        let timer = metrics::query_tantivy_fast_field_timer();

        for (segment_ord, mut revision_ids) in top_docs_by_segment_ord {
            revision_ids.sort_by_key(|(revision_id, ..)| *revision_id);

            let segment_reader = searcher.segment_reader(segment_ord);
            let fast_fields = segment_reader.fast_fields();

            let internal_ids = fast_fields.bytes(INTERNAL_ID_FIELD_NAME)?;
            let timestamps = fast_fields.u64(TS_FIELD_NAME)?;
            let creation_times = fast_fields.f64(CREATION_TIME_FIELD_NAME)?;

            for (revision_id, score, positions) in revision_ids {
                let internal_id = InternalId::try_from(internal_ids.get_bytes(revision_id))?;
                let ts = Timestamp::try_from(timestamps.get_val(revision_id))?;
                let creation_time = CreationTime::try_from(creation_times.get_val(revision_id))?;
                let revision = CandidateRevision {
                    score: f64::from(score) as Score,
                    id: internal_id,
                    ts: WriteTimestamp::Committed(ts),
                    creation_time,
                };
                let revision_with_positions = CandidateRevisionPositions {
                    revision,
                    positions,
                };
                revisions.push(revision_with_positions);
            }
        }
        timer.finish();
    }
    Ok(revisions)
}

#[cfg(test)]
mod tests {
    use itertools::Itertools;
    use tantivy::{
        query::Scorer,
        DocId,
        DocSet,
        Postings,
        Score,
        TERMINATED,
    };

    use crate::tantivy_query::IntersectionSingleScorer;

    /// A non-public DocSet mock in Tantivy tests which we replicate here
    struct VecDocSet {
        doc_ids: Vec<DocId>,
        cursor: usize,
    }

    impl From<Vec<DocId>> for VecDocSet {
        fn from(value: Vec<DocId>) -> Self {
            Self {
                doc_ids: value,
                cursor: 0,
            }
        }
    }

    impl DocSet for VecDocSet {
        fn advance(&mut self) -> DocId {
            self.cursor += 1;
            if self.cursor >= self.doc_ids.len() {
                self.cursor = self.doc_ids.len();
                return TERMINATED;
            }
            self.doc()
        }

        fn doc(&self) -> DocId {
            if self.cursor == self.doc_ids.len() {
                return TERMINATED;
            }
            self.doc_ids[self.cursor]
        }

        fn size_hint(&self) -> u32 {
            self.doc_ids.len() as u32
        }
    }

    struct VecPostingsScorer {
        docset: VecDocSet,
        scores: Vec<Score>,
        positions: Vec<Vec<u32>>,
    }

    impl DocSet for VecPostingsScorer {
        fn advance(&mut self) -> DocId {
            self.docset.advance()
        }

        fn doc(&self) -> DocId {
            self.docset.doc()
        }

        fn size_hint(&self) -> u32 {
            self.docset.size_hint()
        }
    }

    impl Scorer for VecPostingsScorer {
        fn score(&mut self) -> Score {
            self.scores[self.docset.cursor]
        }
    }

    impl Postings for VecPostingsScorer {
        fn term_freq(&self) -> u32 {
            self.positions[self.docset.cursor].len() as u32
        }

        fn positions_with_offset(&mut self, offset: u32, output: &mut Vec<u32>) {
            let pos = &self.positions[self.docset.cursor];
            *output = pos.iter().map(|v| v + offset).collect_vec();
        }
    }

    impl VecPostingsScorer {
        fn new(docset: Vec<DocId>, scores: Vec<Score>, positions: Vec<Vec<u32>>) -> Self {
            assert_eq!(docset.len(), scores.len());
            assert_eq!(docset.len(), positions.len());
            Self {
                docset: docset.into(),
                scores,
                positions,
            }
        }
    }

    #[test]
    fn test_intersection_scorer() {
        {
            let left = VecDocSet::from(vec![1, 3, 9]);
            let right = VecDocSet::from(vec![3, 4, 9, 18]);
            let scorer = VecPostingsScorer::new(
                vec![1, 3, 4, 9, 18],
                vec![1.0, 2.0, 3.0, 4.0, 5.0],
                vec![vec![], vec![0], vec![1, 2], vec![3, 4, 5], vec![6, 7, 8, 9]],
            );
            let mut intersection = IntersectionSingleScorer::new(vec![left, right], scorer);
            assert_eq!(intersection.doc(), 3);
            assert_eq!(intersection.score(), 2.0);
            assert_eq!(intersection.get_positions(), vec![0]);
            assert_eq!(intersection.advance(), 9);
            assert_eq!(intersection.doc(), 9);
            assert_eq!(intersection.score(), 4.0);
            assert_eq!(intersection.get_positions(), vec![3, 4, 5]);
            assert_eq!(intersection.advance(), TERMINATED);
        }
        {
            let left = VecDocSet::from(vec![1, 3, 9]);
            let right = VecDocSet::from(vec![3, 4, 9, 18]);
            let scorer = VecPostingsScorer::new(
                vec![1, 5, 9, 111],
                vec![1.0, 2.0, 3.0, 40.0],
                vec![vec![0]; 4],
            );
            let mut intersection = IntersectionSingleScorer::new(vec![left, right], scorer);
            assert_eq!(intersection.doc(), 9);
            assert_eq!(intersection.score(), 3.0);
            assert_eq!(intersection.get_positions(), vec![0]);
            assert_eq!(intersection.advance(), TERMINATED);
        }
    }

    #[test]
    fn test_intersection_empty_docsets() {
        {
            let left = VecDocSet::from(vec![1, 3, 9]);
            let right = VecDocSet::from(vec![]);
            let scorer = VecPostingsScorer::new(
                vec![1, 3, 4, 9, 18],
                vec![1.0, 2.0, 3.0, 4.0, 5.0],
                vec![vec![], vec![0], vec![1, 2], vec![3, 4, 5], vec![6, 7, 8, 9]],
            );
            let intersection = IntersectionSingleScorer::new(vec![left, right], scorer);
            assert_eq!(intersection.doc(), TERMINATED);
        }
        {
            let left = VecDocSet::from(vec![1, 3, 9]);
            let right = VecDocSet::from(vec![3, 4, 9, 18]);
            let scorer = VecPostingsScorer::new(vec![], vec![], vec![]);
            let intersection = IntersectionSingleScorer::new(vec![left, right], scorer);
            assert_eq!(intersection.doc(), TERMINATED);
        }
    }

    #[test]
    fn test_intersection_no_required_docsets() {
        let scorer = VecPostingsScorer::new(
            vec![1, 3, 4, 9, 18],
            vec![1.0, 2.0, 3.0, 4.0, 5.0],
            vec![vec![], vec![0], vec![1, 2], vec![3, 4, 5], vec![6, 7, 8, 9]],
        );
        let mut intersection = IntersectionSingleScorer::<VecDocSet, _>::new(vec![], scorer);
        assert_eq!(intersection.doc(), 1);
        assert_eq!(intersection.score(), 1.0);
        assert_eq!(intersection.advance(), 3);
        assert_eq!(intersection.score(), 2.0);
        assert_eq!(intersection.advance(), 4);
        assert_eq!(intersection.score(), 3.0);
        assert_eq!(intersection.advance(), 9);
        assert_eq!(intersection.score(), 4.0);
        assert_eq!(intersection.advance(), 18);
        assert_eq!(intersection.score(), 5.0);
        assert_eq!(intersection.advance(), TERMINATED);
    }
}

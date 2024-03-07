pub mod art;
mod iter_set_bits;
mod slab;
mod small_slice;
mod term_list;
mod term_table;

use std::{
    collections::{
        BTreeMap,
        BTreeSet,
        BinaryHeap,
    },
    fmt::Debug,
    mem,
    ops::Bound,
};

use anyhow::Context;
use common::{
    document::CreationTime,
    types::{
        Timestamp,
        WriteTimestamp,
    },
};
use imbl::{
    OrdMap,
    Vector,
};
use tantivy::{
    query::{
        Bm25StatisticsProvider,
        Bm25Weight,
    },
    Score,
    Term,
};
use value::InternalId;

use self::term_list::{
    TermList,
    TermListBytes,
};
pub use crate::memory_index::term_table::TermId;
use crate::{
    constants::MAX_FUZZY_MATCHES_PER_QUERY_TERM,
    memory_index::term_table::TermTable,
    metrics,
    query::{
        shortlist_and_id_mapping,
        CandidateRevisionPositions,
        CompiledFilterCondition,
        CompiledQuery,
        QueryTerm,
        ShortlistId,
        TermListBitsetQuery,
        TermShortlist,
    },
    scoring::{
        bm25_weight_boost_for_edit_distance,
        Bm25StatisticsDiff,
    },
    CandidateRevision,
    DocumentTerm,
    EditDistance,
    SEARCH_FIELD_ID,
};

#[derive(Clone, Debug)]
pub struct Document {
    ts: WriteTimestamp,
    term_list: TermList,
    fieldnorm: u32,
    creation_time: CreationTime,
}

#[derive(Clone, Debug)]
pub struct Tombstone {
    id: InternalId,
    term_list: TermList,
}

#[derive(Clone, Debug)]
pub struct TimestampStatistics {
    // NB: Since we never mutate this field and don't need copy-on-write, it's more memory
    // efficient to store it as a `BTreeMap` than an `OrdMap`.
    term_freq_diffs: BTreeMap<TermId, i32>,
    total_docs_diff: i32,
    total_term_diff: i32,
}

#[derive(Clone, Debug)]
pub struct MemorySearchIndex {
    min_ts: WriteTimestamp,
    max_ts: WriteTimestamp,

    term_table: TermTable,

    documents: OrdMap<InternalId, Document>,
    // sum(d.terms.heap_allocations() for d in documents)
    documents_terms_size: TermListBytes,

    tombstones: Vector<(WriteTimestamp, Tombstone)>,
    // sum(t.terms.heap_allocations() for _, t in tombstones)
    tombstones_terms_size: TermListBytes,

    statistics: OrdMap<WriteTimestamp, TimestampStatistics>,
    // sum(s.term_freq_diffs.len() for s in statistics.values())
    term_freqs_size: usize,
}

impl MemorySearchIndex {
    pub fn new(base_ts: WriteTimestamp) -> Self {
        Self {
            min_ts: base_ts,
            max_ts: base_ts,

            term_table: TermTable::new(),

            documents: OrdMap::new(),
            documents_terms_size: TermListBytes::ZERO,

            tombstones: Vector::new(),
            tombstones_terms_size: TermListBytes::ZERO,

            statistics: OrdMap::new(),
            term_freqs_size: 0,
        }
    }

    pub fn min_ts(&self) -> WriteTimestamp {
        self.min_ts
    }

    pub fn num_transactions(&self) -> usize {
        self.statistics.len()
    }

    pub fn size(&self) -> usize {
        let mut size = 0;

        size += self.term_table.size();

        size += self.documents.len() * mem::size_of::<(InternalId, Document)>();
        size += self.documents_terms_size.bytes();

        size += self.tombstones.len() * mem::size_of::<(WriteTimestamp, Tombstone)>();
        size += self.tombstones_terms_size.bytes();

        size += self.statistics.len() * mem::size_of::<(WriteTimestamp, TimestampStatistics)>();
        size += self.term_freqs_size * mem::size_of::<(TermId, i64)>();

        size
    }

    pub fn truncate(&mut self, new_min_ts: Timestamp) -> anyhow::Result<()> {
        let new_min_ts = WriteTimestamp::Committed(new_min_ts);
        anyhow::ensure!(
            new_min_ts >= self.min_ts,
            "Expected new_min_ts:{new_min_ts:?} >= min_ts:{:?} ",
            self.min_ts
        );

        let to_remove = self
            .documents
            .iter()
            .filter(|(_, document)| document.ts < new_min_ts)
            .map(|(id, _)| *id)
            .collect::<Vec<_>>();

        for id in to_remove {
            let document = self.documents.remove(&id).unwrap();
            for (term_id, term_freq) in document.term_list.iter_term_freqs() {
                self.term_table.decref(term_id, term_freq);
            }
            self.documents_terms_size -= document.term_list.heap_allocations();
        }

        while let Some((ts, _)) = self.tombstones.front()
            && *ts < new_min_ts
        {
            let (_, tombstone) = self.tombstones.pop_front().unwrap();
            for (term_id, term_freq) in tombstone.term_list.iter_term_freqs() {
                self.term_table.decref(term_id, term_freq);
            }
            self.tombstones_terms_size -= tombstone.term_list.heap_allocations();
        }

        while let Some((ts, _)) = self.statistics.get_min()
            && *ts < new_min_ts
        {
            let ts = *ts;
            let stats = self.statistics.remove(&ts).unwrap();
            for &term_id in stats.term_freq_diffs.keys() {
                self.term_table.decref(term_id, 1);
            }
            self.term_freqs_size -= stats.term_freq_diffs.len();
        }

        self.min_ts = new_min_ts;
        self.max_ts = self.max_ts.max(new_min_ts);

        Ok(())
    }

    pub fn update(
        &mut self,
        id: InternalId,
        ts: WriteTimestamp,
        old_value: Option<(Vec<DocumentTerm>, CreationTime)>,
        new_value: Option<(Vec<DocumentTerm>, CreationTime)>,
    ) -> anyhow::Result<()> {
        let timer = metrics::index_update_timer();

        anyhow::ensure!(
            self.min_ts <= ts,
            "Expected min_ts:{:?} <= ts:{ts:?} ",
            self.min_ts
        );
        anyhow::ensure!(
            self.max_ts <= ts,
            "Expected max_ts:{:?} <= ts:{ts:?} ",
            self.max_ts
        );
        self.max_ts = ts;

        // Update the term increments at `ts`.
        {
            if !self.statistics.contains_key(&ts) {
                if let Some((prev_ts, _)) = self.statistics.get_max() {
                    assert!(*prev_ts < ts);
                }
                let base = TimestampStatistics {
                    term_freq_diffs: BTreeMap::new(),
                    total_docs_diff: 0,
                    total_term_diff: 0,
                };
                self.statistics.insert(ts, base);
            }
            let stats = self.statistics.get_mut(&ts).unwrap();

            if let Some((old_terms, _)) = &old_value {
                let term_set = old_terms
                    .iter()
                    .filter(|doc_term| doc_term.field_id() == SEARCH_FIELD_ID)
                    .map(|doc_term| doc_term.term())
                    .collect::<BTreeSet<_>>();
                for term in term_set {
                    let mut inserted = false;
                    if let Some(term_id) = self.term_table.get(term) {
                        if let Some(count) = stats.term_freq_diffs.get_mut(&term_id) {
                            *count = count.checked_sub(1).ok_or_else(|| {
                                anyhow::anyhow!("Underflow on term frequency diff")
                            })?;
                            inserted = true;
                        }
                    }
                    if !inserted {
                        let term_id = self.term_table.incref(term);
                        assert!(stats.term_freq_diffs.insert(term_id, -1).is_none());
                        self.term_freqs_size += 1;
                    }
                }
                let total_term_diff = old_terms
                    .iter()
                    .filter(|doc_term| doc_term.field_id() == SEARCH_FIELD_ID)
                    .count();
                stats.total_term_diff =
                    stats
                        .total_term_diff
                        .checked_sub(total_term_diff as i32)
                        .ok_or_else(|| anyhow::anyhow!("Underflow on total term diff"))?;
                stats.total_docs_diff = stats
                    .total_docs_diff
                    .checked_sub(1)
                    .ok_or_else(|| anyhow::anyhow!("Underflow on total docs diff"))?;
            }
            if let Some((new_terms, _)) = &new_value {
                let term_set = new_terms
                    .iter()
                    .filter(|doc_term| doc_term.field_id() == SEARCH_FIELD_ID)
                    .map(|doc_term| doc_term.term())
                    .collect::<BTreeSet<_>>();
                for term in term_set {
                    let mut inserted = false;
                    if let Some(term_id) = self.term_table.get(term) {
                        if let Some(count) = stats.term_freq_diffs.get_mut(&term_id) {
                            *count = count.checked_add(1).ok_or_else(|| {
                                anyhow::anyhow!("Overflow on term frequency diff")
                            })?;
                            inserted = true;
                        }
                    }
                    if !inserted {
                        let term_id = self.term_table.incref(term);
                        assert!(stats.term_freq_diffs.insert(term_id, 1).is_none());
                        self.term_freqs_size += 1;
                    }
                }
                let total_term_diff = new_terms
                    .iter()
                    .filter(|doc_term| doc_term.field_id() == SEARCH_FIELD_ID)
                    .count();
                stats.total_term_diff =
                    stats
                        .total_term_diff
                        .checked_add(total_term_diff as i32)
                        .ok_or_else(|| anyhow::anyhow!("Overflow on total term diff"))?;
                stats.total_docs_diff = stats
                    .total_docs_diff
                    .checked_add(1)
                    .ok_or_else(|| anyhow::anyhow!("Overflow on total docs diff"))?;
            }
        }

        if let Some((terms, _)) = old_value {
            if let Some((prev_ts, _)) = self.tombstones.last() {
                anyhow::ensure!(*prev_ts <= ts);
            }
            let term_ids = terms
                .iter()
                .map(|doc_term| (self.term_table.incref(doc_term.term()), doc_term.position()))
                .collect::<Vec<_>>();
            let term_list = TermList::new(term_ids)?;
            let tombstone = Tombstone { id, term_list };
            self.tombstones_terms_size += tombstone.term_list.heap_allocations();
            self.tombstones.push_back((ts, tombstone));
        }

        // Remove the old document if present.
        // NB: It's friendlier to `OrdMap` to do a readonly check for existence before
        // removing, since removing nonexistent IDs still has to do an
        // `Arc::make_mut` for the root, which then has to do a clone.
        if self.documents.contains_key(&id) {
            if let Some(prev_document) = self.documents.remove(&id) {
                for (term_id, term_freq) in prev_document.term_list.iter_term_freqs() {
                    self.term_table.decref(term_id, term_freq);
                }
                self.documents_terms_size -= prev_document.term_list.heap_allocations();
            }
        }

        if let Some((terms, creation_time)) = new_value {
            let num_search_tokens = terms
                .iter()
                .filter(|doc_term| doc_term.field_id() == SEARCH_FIELD_ID)
                .count();
            let term_ids = terms
                .iter()
                .map(|doc_term| (self.term_table.incref(doc_term.term()), doc_term.position()))
                .collect::<Vec<_>>();
            let term_list = TermList::new(term_ids)?;
            let document = Document {
                ts,
                term_list,
                creation_time,
                fieldnorm: num_search_tokens.try_into()?,
            };
            self.documents_terms_size += document.term_list.heap_allocations();
            assert!(self.documents.insert(id, document).is_none());
        }

        timer.finish();
        Ok(())
    }

    /// Evaluate the CompiledQuery for matching terms, bounding as necessary.
    pub fn bound_and_evaluate_query_terms(
        &self,
        query: &Vec<QueryTerm>,
    ) -> (TermShortlist, BTreeMap<ShortlistId, TermId>) {
        let mut query_term_matches = BTreeMap::new();

        for query_term in query {
            if query_term_matches.contains_key(query_term) {
                continue;
            }

            let term_matches = match query_term {
                QueryTerm::Exact(term) => {
                    if let Some(term_id) = self.term_table.get(term) {
                        vec![(0, term.clone(), term_id)]
                    } else {
                        vec![]
                    }
                },
                QueryTerm::Fuzzy {
                    term,
                    max_distance,
                    prefix,
                } => {
                    // We want `terms_heap` to be a min-heap where higher distances compare to lower
                    // values. BinaryHeap is already a max-heap that will yield
                    // distances of higher values first, so we can just use
                    // this.
                    let mut terms_heap = BinaryHeap::<(EditDistance, Term, TermId)>::new();
                    for (term_id, dist, match_term) in
                        self.term_table.get_fuzzy(term, *max_distance, *prefix)
                    {
                        terms_heap.push((dist, match_term, term_id));

                        if terms_heap.len() > MAX_FUZZY_MATCHES_PER_QUERY_TERM {
                            terms_heap.pop();
                        }
                    }
                    terms_heap.into_sorted_vec()
                },
            };

            query_term_matches.insert(query_term.clone(), term_matches);
        }
        shortlist_and_id_mapping(query_term_matches)
    }

    pub fn build_term_list_bitset_query(
        &self,
        query: &CompiledQuery,
        term_shortlist: &TermShortlist,
        term_shortlist_ids: &BTreeMap<ShortlistId, TermId>,
    ) -> TermListBitsetQuery {
        let mut term_ids = BTreeSet::new();
        let mut intersection_term_ids = BTreeSet::new();
        let mut union_id_boosts = BTreeMap::new();

        for CompiledFilterCondition::Must(ref filter_term) in &query.filter_conditions {
            let Some(term_id) = self.term_table.get(filter_term) else {
                // If a filter condition's term is entirely missing, no documents match the
                // query.
                return TermListBitsetQuery::NEVER_MATCH;
            };
            term_ids.insert(term_id);
            intersection_term_ids.insert(term_id);
        }
        for query in &query.text_query {
            let term_matches = term_shortlist.get_shortlisted_terms_for_query_term(query);
            for (dist, id) in term_matches {
                // If term_shortlist_ids contains this shortlist ID, this means the memory index
                // contains this shortlisted term. This will only ever evaluate to None when
                // the disk index returns a combined shortlist of results that includes terms
                // that the memory index does not have.
                if let Some(term_id) = term_shortlist_ids.get(id) {
                    term_ids.insert(*term_id);
                    *union_id_boosts.entry(*term_id).or_insert(0.) +=
                        bm25_weight_boost_for_edit_distance(*dist);
                }
            }
        }

        // If none of the text query terms are present, no documents match the query.
        if union_id_boosts.is_empty() {
            return TermListBitsetQuery::NEVER_MATCH;
        }

        TermListBitsetQuery::new(term_ids, intersection_term_ids, union_id_boosts)
    }

    /// Filters out terms not present in memory index and associates with
    /// TermIds
    pub fn evaluate_shortlisted_query_terms(
        &self,
        shortlisted_terms: &TermShortlist,
    ) -> BTreeMap<ShortlistId, TermId> {
        shortlisted_terms
            .ids_and_terms()
            .filter_map(|(id, t)| self.term_table.get(t).map(|term_id| (id, term_id)))
            .collect()
    }

    pub fn tombstoned_matches(
        &self,
        snapshot_ts: Timestamp,
        query: TermListBitsetQuery,
    ) -> anyhow::Result<BTreeSet<InternalId>> {
        let timer = metrics::updated_matches_timer();
        anyhow::ensure!(
            self.min_ts <= WriteTimestamp::Committed(snapshot_ts.succ()?),
            "Timestamps are out of order! min ts:{:?} snapshot_ts:{snapshot_ts}",
            self.min_ts,
        );
        if query.never_match() {
            return Ok(BTreeSet::new());
        }

        let mut results = BTreeSet::new();
        for (ts, tombstone) in self.tombstones.iter() {
            if *ts <= WriteTimestamp::Committed(snapshot_ts) {
                continue;
            }
            if tombstone.term_list.matches(&query) {
                results.insert(tombstone.id);
            }
        }
        timer.finish();
        Ok(results)
    }

    pub fn bm25_statistics_diff(
        &self,
        snapshot_ts: Timestamp,
        terms: &Vec<Term>,
    ) -> anyhow::Result<Bm25StatisticsDiff> {
        let timer = metrics::bm25_statistics_diff_timer();
        anyhow::ensure!(
            self.min_ts <= WriteTimestamp::Committed(snapshot_ts.succ()?),
            "Timestamps are out of order!  min ts:{:?} snapshot_ts:{snapshot_ts}",
            self.min_ts,
        );
        let from_ts = WriteTimestamp::Committed(snapshot_ts);
        let (total_num_documents, total_num_search_tokens) =
            self.total_num_documents_and_tokens(from_ts);

        let mut term_statistics = BTreeMap::new();
        for term in terms {
            let Some(term_str) = term.as_str() else {
                anyhow::bail!(
                    "Expected text term to have text. Actual type: {:?}",
                    term.typ()
                );
            };
            term_statistics.insert(
                term_str.to_string(),
                self.num_documents_with_term(from_ts, term),
            );
        }
        let diff = Bm25StatisticsDiff {
            term_statistics,
            num_documents_diff: total_num_documents,
            num_search_tokens_diff: total_num_search_tokens,
        };
        metrics::log_bm25_statistics_diff(timer, &diff);
        Ok(diff)
    }

    pub fn query(
        &self,
        snapshot_ts: Timestamp,
        query: &TermListBitsetQuery,
        term_ids: &BTreeMap<ShortlistId, TermId>,
        term_weights: &Vec<Bm25Weight>,
    ) -> anyhow::Result<Vec<CandidateRevisionPositions>> {
        let timer = metrics::memory_query_timer();
        anyhow::ensure!(
            self.min_ts <= WriteTimestamp::Committed(snapshot_ts.succ()?),
            "Timestamps are out of order!  min ts:{:?} snapshot_ts:{snapshot_ts}",
            self.min_ts,
        );
        if query.never_match() {
            return Ok(vec![]);
        }

        let mut revisions = vec![];

        let inverted_term_id_index: BTreeMap<_, _> =
            term_ids.iter().map(|(s, t)| (*t, *s)).collect();
        for (id, document) in self.documents.iter() {
            if document.ts <= WriteTimestamp::Committed(snapshot_ts) {
                continue;
            };
            let maybe_score = document.term_list.matches_with_score_and_positions(
                query,
                term_weights,
                document.fieldnorm,
            );
            let Some((score, positions)) = maybe_score else {
                continue;
            };
            let revision = CandidateRevision {
                score,
                id: *id,
                ts: document.ts,
                creation_time: document.creation_time,
            };
            let positions = positions
                .into_iter()
                .map(|(id, pos)| {
                    anyhow::Ok((
                        *inverted_term_id_index
                            .get(&id)
                            .context("Query matched a TermID not in shortlist")?,
                        pos,
                    ))
                })
                .collect::<anyhow::Result<_>>()?;
            let pos_revision = CandidateRevisionPositions {
                revision,
                positions,
            };
            revisions.push(pos_revision);
        }

        metrics::finish_memory_query(timer, revisions.len());
        Ok(revisions)
    }

    fn num_documents_with_term(&self, from_ts: WriteTimestamp, term: &Term) -> i64 {
        let _timer = metrics::num_documents_with_term_timer();
        let mut num_documents = 0;
        if let Some(term_id) = self.term_table.get(term) {
            for (_, stats) in self
                .statistics
                .range((Bound::Excluded(from_ts), Bound::Unbounded))
            {
                if let Some(increment) = stats.term_freq_diffs.get(&term_id) {
                    num_documents += increment;
                }
            }
        }
        num_documents as i64
    }

    fn total_num_documents_and_tokens(&self, from_ts: WriteTimestamp) -> (i64, i64) {
        let _timer = metrics::total_num_documents_and_tokens_timer();
        let mut num_documents = 0i64;
        let mut num_tokens = 0i64;
        for (_, stats) in self
            .statistics
            .range((Bound::Excluded(from_ts), Bound::Unbounded))
        {
            num_documents += stats.total_docs_diff as i64;
            num_tokens += stats.total_term_diff as i64;
        }
        (num_documents, num_tokens)
    }

    pub fn consistency_check(&self) -> anyhow::Result<()> {
        anyhow::ensure!(self.min_ts <= self.max_ts);
        self.term_table.consistency_check()?;

        let mut expected_refcounts = BTreeMap::new();

        let mut expected_document_terms = TermListBytes::ZERO;
        for (_, document) in &self.documents {
            anyhow::ensure!(self.min_ts <= document.ts && document.ts <= self.max_ts);
            for (term_id, term_freq) in document.term_list.iter_term_freqs() {
                *expected_refcounts.entry(term_id).or_insert(0) += term_freq;
            }
            expected_document_terms += document.term_list.heap_allocations();
        }
        anyhow::ensure!(expected_document_terms == self.documents_terms_size);

        let mut prev_ts = None;
        let mut expected_tombstone_terms = TermListBytes::ZERO;
        for (ts, tombstone) in &self.tombstones {
            anyhow::ensure!(prev_ts <= Some(*ts));
            anyhow::ensure!(self.min_ts <= *ts && *ts <= self.max_ts);
            for (term_id, term_freq) in tombstone.term_list.iter_term_freqs() {
                *expected_refcounts.entry(term_id).or_insert(0) += term_freq;
            }
            expected_tombstone_terms += tombstone.term_list.heap_allocations();
            prev_ts = Some(*ts);
        }
        anyhow::ensure!(expected_tombstone_terms == self.tombstones_terms_size);

        let mut expected_term_freqs = 0;
        for stats in self.statistics.values() {
            for &term_id in stats.term_freq_diffs.keys() {
                *expected_refcounts.entry(term_id).or_insert(0) += 1;
            }
            expected_term_freqs += stats.term_freq_diffs.len();
        }
        anyhow::ensure!(expected_term_freqs == self.term_freqs_size);

        for (term_id, expected_refcount) in expected_refcounts {
            anyhow::ensure!(self.term_table.refcount(term_id) == expected_refcount);
        }

        Ok(())
    }
}

pub fn build_term_weights(
    term_shortlist: &TermShortlist,
    term_shortlist_ids: &BTreeMap<ShortlistId, TermId>,
    query: &TermListBitsetQuery,
    combined_bm25_statistics: Bm25StatisticsDiff,
) -> anyhow::Result<Vec<Bm25Weight>> {
    if query.never_match() {
        return Ok(vec![]);
    }

    let total_num_docs = combined_bm25_statistics.num_documents_diff.try_into()?;
    let average_fieldnorm =
        combined_bm25_statistics.num_search_tokens_diff as Score / total_num_docs as Score;

    // Construct a TermId -> ShortlistId mapping so we can search up each sorted
    // term in query to get a term in term_shortlist
    let inverted_term_id_idx: BTreeMap<TermId, ShortlistId> =
        term_shortlist_ids.iter().map(|(s, t)| (*t, *s)).collect();

    let term_weights = query
        .union_terms
        .iter_ones()
        // Need to map union_idx -> TermId -> ShortlistId (using inverted index) -> Term (using TermShortlist)
        .map(|union_idx| {
            let term_id = query.sorted_terms[union_idx];

            let shortlist_id = inverted_term_id_idx
                .get(&term_id)
                .context("TermId missing from shortlist ID mapping")?;
            let term = term_shortlist
                .get_term(*shortlist_id)?;

            let term_stats = combined_bm25_statistics.doc_freq(term)?;
            anyhow::Ok(Bm25Weight::for_one_term(
                term_stats,
                total_num_docs,
                average_fieldnorm,
            ))
        })
        .collect::<anyhow::Result<Vec<Bm25Weight>>>()?;

    Ok(term_weights)
}

#[cfg(test)]
mod tests {
    use common::{
        document::CreationTime,
        types::Timestamp,
    };
    use tantivy::{
        schema::Field,
        Term,
    };
    use value::InternalId;

    use super::MemorySearchIndex;
    use crate::{
        memory_index::WriteTimestamp,
        DocumentTerm,
        FieldPosition,
    };

    #[test]
    fn test_truncation() -> anyhow::Result<()> {
        let ts0 = Timestamp::MIN;
        let mut index = MemorySearchIndex::new(WriteTimestamp::Committed(ts0));

        // Insert 1 document at t=1
        let ts1 = ts0.succ()?;
        let field = Field::from_field_id(0);
        let term = Term::from_field_text(field, "value");
        index.update(
            InternalId::MIN,
            WriteTimestamp::Committed(ts1),
            None,
            Some((
                vec![DocumentTerm::Search {
                    term: term.clone(),
                    pos: FieldPosition::default(),
                }],
                CreationTime::ONE,
            )),
        )?;

        // At t=1 we can see the document and have a size.
        let query_terms = vec![term];
        assert_eq!(
            index
                .bm25_statistics_diff(ts0, &query_terms)?
                .num_documents_diff,
            1
        );
        assert!(index.size() > 0);

        // Truncate the index at t=2.
        let ts2 = ts1.succ()?;
        index.truncate(ts2)?;

        // We can no longer query before t=2.
        assert!(index
            .bm25_statistics_diff(ts0, &query_terms)
            .unwrap_err()
            .to_string()
            .contains("Timestamps are out of order"));

        // The index now has size 0.
        assert_eq!(index.size(), 0);

        Ok(())
    }
}

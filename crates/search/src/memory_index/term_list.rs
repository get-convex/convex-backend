use std::{
    cmp,
    collections::BTreeMap,
    iter,
    mem,
    ops::{
        AddAssign,
        SubAssign,
    },
    sync::Arc,
};

use bitvec::{
    order::Lsb0,
    vec::BitVec,
};
use sucds::{
    int_vectors::{
        Access,
        DacsOpt,
    },
    mii_sequences::{
        EliasFano,
        EliasFanoBuilder,
    },
    Serializable,
};
use tantivy::{
    fieldnorm::FieldNormReader,
    query::Bm25Weight,
    Score,
};
use xorf::{
    BinaryFuse16,
    Filter,
};

use super::{
    bitset64::Bitset64,
    PreparedMemoryPostingListQuery,
};
use crate::{
    constants::MAX_POSITIONS_PER_MATCHED_TERM,
    memory_index::term_table::TermId,
    query::TermListBitsetQuery,
    FieldPosition,
};

/// Memory-efficient structure for storing the terms in a document, including
/// terms within both search and filter fields. This structure is conceptually a
/// `BTreeMap<TermId, Vec<Position>>`.
///
/// Provides efficient methods for accessing terms, their frequencies, and the
/// positions at which they occur at within a field. This is useful for
/// calculating BM25 statistics and other ranking statistics for search.
///
/// # Construction
/// In search, a `Document` is conceptually a `BTreeMap<FieldId, Vec<TermId>>`
/// where each `FieldId` corresponds to either a filter field or search field.
/// We can represent each `Vec<Term>` as a map of positional information, so
/// this becomes a `BTreeMap<FieldId, BTreeMap<TermId, Vec<Positions>>`.
/// Since `TermId`s are unique across `FieldId`s (due to uniqueness of how
/// filter field terms are represented), we can flatten this into one
/// `BTreeMap<TermId, Vec<Positions>>`.
///
/// To represent this `BTreeMap`, we start by splitting it into two parallel
/// lists:
///
/// terms: btree_map.keys().collect();   // sorted by term_id!
/// freqs: btree_map.values().count();   // we store this as a sorted list of
/// cumulative frequencies
///
/// Then, we can compress these two lists, assuming that (1) the difference
/// between adjacent term IDs in `terms` is small and (2) the frequencies are
/// generally small themselves. Both of these arrays are monotonically
/// increasing integer arrays, so we use `sucds`'s Elias Fano array datatype.
///
/// # Representing Positions
/// To represent positions, we also store and compress a positions array which
/// is just the positions of each term in the terms array concatenated together:
///
/// positions: btree_map.values().flatten().collect()
///
/// To fetch the positions for a given term, we need two pieces of information:
/// (1) the length of the term's positions subarray
/// (2) the starting offset of the term's positions subarray
///
/// (1) is just the frequency which we already can calculate. (2) is the
/// cumulative frequency of that term, which is how we store frequencies in the
/// first place. This info lets us efficiently skip through `positions` to read
/// the term's subarray.
///
/// # Queries
/// The simplest queries `TermList::iter_*` iterate over the terms or
/// frequencies, fully decompressing the integer streams.
///
/// The real benefit of this sorted approach comes from doing sorted set
/// intersections between the term list and a list of terms in a query. This
/// algorithm is exactly the same as for posting lists in Tantivy but inverted:
/// Instead of querying a posting list of DocIds, we're querying a term list of
/// TermIds.
///
/// # Term filter acceleration
/// When a query or intersection term is sufficiently rare, it's very useful to
/// use a hash-based approximate set data structure to quickly determine that a
/// document doesn't match. To this end, we layer on top a `BinaryFuse16` data
/// structure (more modern version of a Bloom filter).
///
/// Current benchmarks show this as helpful in addition to the sorted integer
/// lists, but it's a bit surprising to me (sujayakar, 3/4/23) that this is an
/// improvement. At the very minimum, I think we'll want to segment our
/// documents by their termlist size and configure the filter differently
/// to keep a constant false positive rate.
#[derive(Clone, Debug)]
pub struct TermList {
    // The `EliasFano` structures are really large (336 bytes on the stack), so
    // box the `NonemptyTermList` - containing two - to avoid stack overflow.
    // https://github.com/kampersanda/sucds/issues/96
    //
    // Furthermore, since term lists are immutable, we use Arc to make cloning
    // cheaper because we store these in `imbl::OrdMap`s.
    inner: Option<Arc<NonemptyTermList>>,
}

#[derive(Debug)]
struct NonemptyTermList {
    term_filter: BinaryFuse16,

    terms: EliasFano,
    cumulative_freqs: EliasFano,
    positions: DacsOpt,
}

impl TermList {
    pub fn new(mut terms_and_positions: Vec<(TermId, FieldPosition)>) -> anyhow::Result<Self> {
        // Step 1: Accumulate parallel lists of the unique sorted term IDs, their
        // frequencies, and their positions.
        terms_and_positions.sort_unstable();

        let Some((greatest_term_id, _)) = terms_and_positions.last() else {
            return Ok(TermList { inner: None });
        };
        let mut terms_builder =
            EliasFanoBuilder::new(*greatest_term_id as usize + 1, terms_and_positions.len())?;

        let mut cumulative_freqs_builder =
            EliasFanoBuilder::new(terms_and_positions.len() + 1, terms_and_positions.len())?;
        let mut freqs_sum = 0;

        let mut term_u64s = Vec::with_capacity(terms_and_positions.len());

        let mut position_u32s = Vec::with_capacity(terms_and_positions.len());
        let mut prev_term = None;

        for (term_id, position) in terms_and_positions {
            position_u32s.push(u32::from(position));

            if let Some((prev_term, ref mut prev_freq)) = prev_term {
                if prev_term == term_id {
                    *prev_freq += 1;
                    continue;
                }
            }
            if let Some((prev_term, prev_freq)) = prev_term.take() {
                terms_builder.push(prev_term as usize)?;
                term_u64s.push(prev_term as u64);

                freqs_sum += prev_freq;
                cumulative_freqs_builder.push(freqs_sum as usize)?;
            }
            prev_term = Some((term_id, 1));
        }
        if let Some((term_id, term_freq)) = prev_term {
            terms_builder.push(term_id as usize)?;
            term_u64s.push(term_id as u64);

            freqs_sum += term_freq;
            cumulative_freqs_builder.push(freqs_sum as usize)?;
        }

        let term_filter = BinaryFuse16::try_from(&term_u64s).map_err(anyhow::Error::msg)?;
        let terms = terms_builder.build().enable_rank();
        let cumulative_freqs = cumulative_freqs_builder.build();

        // Taken from their docs as a reasonable, bounded compression level.
        let max_levels = Some(2);
        let positions = DacsOpt::from_slice(&position_u32s, max_levels)?;

        let inner = NonemptyTermList {
            term_filter,
            terms,
            cumulative_freqs,
            positions,
        };
        Ok(Self {
            inner: Some(Arc::new(inner)),
        })
    }

    pub fn iter_terms(&self) -> impl Iterator<Item = TermId> + '_ {
        iter::from_coroutine(
            #[coroutine]
            move || {
                let Some(ref inner) = self.inner else {
                    return;
                };
                for t in inner.terms.iter(0) {
                    yield t as TermId;
                }
            },
        )
    }

    pub fn iter_freqs(&self) -> impl Iterator<Item = u32> + '_ {
        iter::from_coroutine(
            #[coroutine]
            move || {
                let Some(ref inner) = self.inner else {
                    return;
                };
                let mut current = 0;
                for cumulative_freq in inner.cumulative_freqs.iter(0) {
                    yield (cumulative_freq - current) as u32;
                    current = cumulative_freq;
                }
            },
        )
    }

    pub fn iter_term_freqs(&self) -> impl Iterator<Item = (TermId, u32)> + '_ {
        self.iter_terms().zip(self.iter_freqs())
    }

    #[cfg(test)]
    pub fn iter_positions(&self) -> impl Iterator<Item = Vec<u32>> + '_ {
        iter::from_coroutine(
            #[coroutine]
            move || {
                let Some(ref inner) = self.inner else {
                    return;
                };
                let mut current_index = 0usize;
                for term_freq in self.iter_freqs() {
                    let term_freq = term_freq as usize;
                    let mut positions = Vec::with_capacity(term_freq);
                    for i in 0..term_freq {
                        positions.push(inner.positions.access(current_index + i).unwrap() as u32);
                    }
                    yield positions;
                    current_index += term_freq;
                }
            },
        )
    }

    pub fn matches(&self, query: &TermListBitsetQuery) -> bool {
        let Some(ref inner) = self.inner else {
            return false;
        };

        let sorted_terms = query.sorted_terms.as_slice();
        let intersection_ids = &query.intersection_terms;
        let union_ids = &query.union_terms;

        if !inner.term_filter_matches(sorted_terms, intersection_ids, union_ids) {
            return false;
        }

        // Build up a bitset of which terms match.
        let mut matches = BitVec::<usize, Lsb0>::repeat(false, sorted_terms.len());
        for (i, _) in inner.term_matches(sorted_terms) {
            matches.set(i, true);
        }

        // Check that all of the intersection bits and any of the union bits are set.
        intersection_ids.iter_ones().all(|i| matches[i])
            && union_ids.iter_ones().any(|i| matches[i])
    }

    pub fn matches2(&self, query: &PreparedMemoryPostingListQuery) -> bool {
        let Some(ref inner) = self.inner else {
            return false;
        };
        if !inner.term_filter_matches2(query) {
            return false;
        }
        // Build up a bitset of which terms match.
        let mut matching_terms = Bitset64::new();
        for (i, _) in inner.term_matches(&query.sorted_terms) {
            matching_terms.insert(i);
        }

        // Check that all of the intersection bits and any of the union bits are set.
        let all_intersection =
            matching_terms.intersect(query.intersection_terms) == query.intersection_terms;
        let any_union = !matching_terms.intersect(query.union_terms).is_empty();
        all_intersection && any_union
    }

    pub fn matches2_with_score(
        &self,
        query: &PreparedMemoryPostingListQuery,
        num_search_tokens: u32,
    ) -> Option<Score> {
        let inner = self.inner.as_ref()?;
        if !inner.term_filter_matches2(query) {
            return None;
        }

        let mut score = 0.;
        let fieldnorm_id = FieldNormReader::fieldnorm_to_id(num_search_tokens);

        // Build up a bitset of which terms match.
        let mut matching_terms = Bitset64::new();
        for (i, pos) in inner.term_matches(&query.sorted_terms) {
            matching_terms.insert(i);
            if query.union_terms.contains(i) {
                let term_freq = inner
                    .cumulative_freqs
                    .delta(pos)
                    .expect("term position missing from cumulative_freqs");
                let union_rank = query.union_terms.rank(i);
                let bm25_weight = &query.union_weights[union_rank];
                score += bm25_weight.score(fieldnorm_id, term_freq as u32);
            }
        }
        // Check that all of the intersection bits and any of the union bits are set.
        let all_intersection =
            matching_terms.intersect(query.intersection_terms) == query.intersection_terms;
        let any_union = !matching_terms.intersect(query.union_terms).is_empty();
        (all_intersection && any_union).then_some(score)
    }

    // Check if a query matches the given document, and compute its BM25 score if
    // so.
    //
    // Arguments:
    // * sorted_terms: Sorted list of all term IDs in the query.
    // * term_weights: `Bm25Weight`s for each union query term.
    //
    // Bitsets of indexes into `sorted_terms`:
    // * is_intersection: Which terms are part of the intersection query?
    // * is_union: Which terms are part of the union query?
    //
    pub fn matches_with_score_and_positions(
        &self,
        query: &TermListBitsetQuery,
        term_weights: &[Bm25Weight],
        fieldnorm: u32,
    ) -> Option<(Score, BTreeMap<TermId, Vec<u32>>)> {
        let inner = self.inner.as_ref()?;

        let sorted_terms = query.sorted_terms.as_slice();
        let intersection_ids = &query.intersection_terms;
        let union_ids = &query.union_terms;

        if !inner.term_filter_matches(sorted_terms, intersection_ids, union_ids) {
            return None;
        }

        let fieldnorm_id = FieldNormReader::fieldnorm_to_id(fieldnorm);
        let mut matches = BitVec::<usize, Lsb0>::repeat(false, sorted_terms.len());
        let mut score = 0.;
        let mut union_idx = 0;
        let mut positions = BTreeMap::new();

        for (i, pos) in inner.term_matches(sorted_terms) {
            matches.set(i, true);
            if !union_ids[i] {
                continue;
            }
            let term_freq = inner.cumulative_freqs.delta(pos).unwrap();
            let positions_end = inner.cumulative_freqs.select(pos).unwrap();
            let positions_start = positions_end - term_freq;

            // Bound number of positions we consider.
            let num_positions = cmp::min(term_freq, MAX_POSITIONS_PER_MATCHED_TERM);
            let mut term_positions = Vec::with_capacity(num_positions);
            for i in 0..num_positions {
                term_positions.push(inner.positions.access(positions_start + i).unwrap() as u32);
            }
            positions.insert(sorted_terms[i], term_positions);

            // Compute which index into `term_weights` we're at by counting the number of
            // bits in `is_union` set before our current position.
            let union_rank = union_ids.as_bitslice()[..i].count_ones();
            let candidate_score = term_weights[union_rank].score(fieldnorm_id, term_freq as u32);

            // Apply the scoring.
            let boost = query.union_id_boosts[union_idx];
            union_idx += 1;
            score += candidate_score * boost;
        }

        // but they're still necessary, especially for very large documents with
        // high false positive rate.
        if intersection_ids.iter_ones().any(|i| !matches[i])
            || !union_ids.iter_ones().any(|i| matches[i])
        {
            return None;
        }
        Some((score, positions))
    }

    pub fn heap_allocations(&self) -> TermListBytes {
        let Some(ref inner) = self.inner else {
            return TermListBytes::ZERO;
        };
        TermListBytes {
            fingerprints_bytes: inner.term_filter.fingerprints.len() * mem::size_of::<u16>(),
            terms_bytes: inner.terms.size_in_bytes(),
            freqs_bytes: inner.cumulative_freqs.size_in_bytes(),
            positions_bytes: inner.positions.size_in_bytes(),
        }
    }
}

impl NonemptyTermList {
    // Check if the query approximately matches the document set with the
    // possibility of false positives.
    fn term_filter_matches(
        &self,
        sorted_terms: &[TermId],
        is_intersection: &BitVec,
        is_union: &BitVec,
    ) -> bool {
        let any_intersection_missing = is_intersection
            .iter_ones()
            .map(|i| sorted_terms[i] as u64)
            .any(|term_id| !self.term_filter.contains(&term_id));
        if any_intersection_missing {
            return false;
        }
        is_union
            .iter_ones()
            .map(|i| sorted_terms[i] as u64)
            .any(|term_id| self.term_filter.contains(&term_id))
    }

    fn term_filter_matches2(&self, query: &PreparedMemoryPostingListQuery) -> bool {
        let any_intersection_missing = query
            .intersection_terms()
            .map(|t| t as u64)
            .any(|term_id| !self.term_filter.contains(&term_id));
        if any_intersection_missing {
            return false;
        }
        query
            .union_terms()
            .map(|t| t as u64)
            .any(|term_id| self.term_filter.contains(&term_id))
    }

    // Iterate over all term IDs in a query set that intersect with the document's
    // termlist.
    //
    // Yields: (index into sorted terms, index into term list)
    //
    fn term_matches<'a>(
        &'a self,
        sorted_terms: &'a [TermId],
    ) -> impl Iterator<Item = (usize, usize)> + 'a {
        iter::from_coroutine(
            #[coroutine]
            move || {
                for (i, term) in sorted_terms.iter().cloned().enumerate() {
                    // Find the position of `term` as its "rank": The number of
                    // terms in the sequence that are less than `term`.
                    let Some(rank) = self.terms.rank(term as usize) else {
                        break;
                    };
                    // If `term` is larger than all other terms in the sequence,
                    // it can be `terms.len()` and then return `None`. here.
                    let Some(current_term) = self.terms.select(rank) else {
                        break;
                    };
                    if current_term == term as usize {
                        yield (i, rank);
                    }
                }
            },
        )
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TermListBytes {
    pub fingerprints_bytes: usize,
    pub terms_bytes: usize,
    pub freqs_bytes: usize,
    pub positions_bytes: usize,
}

impl AddAssign for TermListBytes {
    fn add_assign(&mut self, rhs: Self) {
        self.fingerprints_bytes += rhs.fingerprints_bytes;
        self.terms_bytes += rhs.terms_bytes;
        self.freqs_bytes += rhs.freqs_bytes;
        self.positions_bytes += rhs.freqs_bytes;
    }
}

impl SubAssign for TermListBytes {
    fn sub_assign(&mut self, rhs: Self) {
        self.fingerprints_bytes -= rhs.fingerprints_bytes;
        self.terms_bytes -= rhs.terms_bytes;
        self.freqs_bytes -= rhs.freqs_bytes;
        self.positions_bytes -= rhs.freqs_bytes;
    }
}

impl TermListBytes {
    pub const ZERO: Self = Self {
        fingerprints_bytes: 0,
        terms_bytes: 0,
        freqs_bytes: 0,
        positions_bytes: 0,
    };

    pub fn bytes(&self) -> usize {
        self.fingerprints_bytes + self.terms_bytes + self.freqs_bytes + self.positions_bytes
    }
}

#[cfg(test)]
mod tests {
    use std::collections::{
        BTreeMap,
        BTreeSet,
    };

    use cmd_util::env::env_config;
    #[allow(unused)]
    use itertools::Itertools;
    use proptest::prelude::*;
    use proptest_derive::Arbitrary;
    use tantivy::{
        fieldnorm::FieldNormReader,
        query::Bm25Weight,
    };

    use crate::{
        memory_index::{
            term_list::TermList,
            TermId,
        },
        query::TermListBitsetQuery,
        FieldPosition,
    };

    fn test_term_iter(terms: Vec<(u32, u32)>) {
        if terms.is_empty() {
            return;
        }

        let mut term_freqs = BTreeMap::new();
        let mut term_positions = BTreeMap::new();
        for (term, pos) in &terms {
            *term_freqs.entry(*term).or_insert(0) += 1;
            term_positions
                .entry(*term)
                .or_insert(BTreeSet::new())
                .insert(*pos);
        }
        let terms_and_positions = terms
            .into_iter()
            .map(|(t, pos)| (t, FieldPosition(pos)))
            .collect();
        let term_list = TermList::new(terms_and_positions).unwrap();

        let computed_freqs = term_list
            .iter_terms()
            .zip(term_list.iter_freqs())
            .collect::<BTreeMap<_, _>>();

        let computed_positions = term_list
            .iter_terms()
            .zip(term_list.iter_positions())
            .map(|(t, pos)| (t, pos.into_iter().collect::<BTreeSet<_>>()))
            .collect::<BTreeMap<_, _>>();

        assert_eq!(term_freqs, computed_freqs);
        assert_eq!(term_positions, computed_positions);
    }

    #[derive(Arbitrary, Debug)]
    struct Query {
        #[proptest(strategy = "prop::collection::vec(any::<u32>(), 1..24)")]
        union_terms: Vec<u32>,
        #[proptest(strategy = "prop::collection::vec(any::<u32>(), 0..24)")]
        intersection_terms: Vec<u32>,
    }

    fn test_matches(terms: Vec<(u32, u32)>, queries: Vec<Query>) {
        if terms.is_empty() {
            return;
        }
        let term_set: BTreeSet<_> = terms.iter().map(|(t, _pos)| t).cloned().collect();
        let terms_and_positions = terms
            .into_iter()
            .map(|(t, pos)| (t, FieldPosition(pos)))
            .collect();
        let term_list = TermList::new(terms_and_positions).unwrap();

        for q in queries {
            let mut term_ids = BTreeSet::new();
            let mut intersection_term_ids = BTreeSet::new();
            let mut boosts = BTreeMap::new();
            for &term_id in &q.intersection_terms {
                intersection_term_ids.insert(term_id);
                term_ids.insert(term_id);
            }
            for term_id in q.union_terms.iter() {
                let term_id = TermId::from(*term_id);
                term_ids.insert(term_id);
                boosts.insert(term_id, 1.);
            }
            let term_list_query = TermListBitsetQuery::new(term_ids, intersection_term_ids, boosts);

            let computed = term_list.matches(&term_list_query);
            let expected = q.intersection_terms.iter().all(|t| term_set.contains(t))
                && q.union_terms.iter().any(|t| term_set.contains(t));
            assert_eq!(computed, expected);
        }
    }

    #[derive(Arbitrary, Debug)]
    struct UnionTerm {
        term_id: u32,
        #[proptest(strategy = "1u64..32")]
        term_doc_freq: u64,
    }

    #[derive(Arbitrary, Debug)]
    struct ScoredQuery {
        #[proptest(strategy = "prop::collection::vec(any::<UnionTerm>(), 1..24)")]
        union_terms: Vec<UnionTerm>,
        #[proptest(strategy = "prop::collection::vec(any::<u32>(), 0..24)")]
        intersection_terms: Vec<u32>,
    }

    fn test_matches_with_score(
        terms: Vec<(u32, u32)>,
        queries: Vec<ScoredQuery>,
        fieldnorm: u32,
        total_doc_freq: u64,
        avg_fieldnorm: f32,
    ) {
        if terms.is_empty() {
            return;
        }
        let mut term_positions = BTreeMap::new();
        for (term, position) in &terms {
            term_positions
                .entry(*term)
                .or_insert_with(Vec::new)
                .push(*position);
        }
        for positions in term_positions.values_mut() {
            positions.sort();
        }
        let terms_and_positions = terms
            .into_iter()
            .map(|(t, pos)| (t, FieldPosition(pos)))
            .collect();
        let term_list = TermList::new(terms_and_positions).unwrap();

        for q in queries {
            let mut term_ids = BTreeSet::new();
            let mut intersection_term_ids = BTreeSet::new();
            for &term_id in &q.intersection_terms {
                intersection_term_ids.insert(term_id);
                term_ids.insert(term_id);
            }
            let mut term_weights = vec![];
            let mut term_weights_by_id = BTreeMap::new();
            let mut boosts = BTreeMap::new();
            for term in q.union_terms.iter() {
                let term_id = TermId::from(term.term_id);
                term_ids.insert(term_id);

                let weight =
                    Bm25Weight::for_one_term(term.term_doc_freq, total_doc_freq, avg_fieldnorm);
                term_weights.push(weight.clone());
                term_weights_by_id.insert(term_id, weight);
                boosts.insert(term_id, 1.);
            }
            let term_list_query = TermListBitsetQuery::new(term_ids, intersection_term_ids, boosts);

            let computed = term_list.matches_with_score_and_positions(
                &term_list_query,
                &term_weights,
                fieldnorm,
            );

            let mut expected = None;
            if q.intersection_terms
                .iter()
                .all(|t| term_positions.contains_key(t))
            {
                let mut any = false;
                let mut score = 0.;
                let mut match_positions = BTreeMap::new();
                for term_idx in term_list_query.union_terms.iter_ones() {
                    let term_id = term_list_query.sorted_terms[term_idx];
                    let Some(position_list) = term_positions.get(&term_id) else {
                        continue;
                    };
                    any = true;
                    score += term_weights_by_id[&term_id].score(
                        FieldNormReader::fieldnorm_to_id(fieldnorm),
                        position_list.len() as u32,
                    );
                    match_positions
                        .entry(term_id)
                        .or_insert_with(Vec::new)
                        .extend(position_list.iter().cloned());
                }
                if any {
                    expected = Some((score, match_positions));
                }
            }
            assert_eq!(computed, expected);
        }
    }

    proptest! {
        // It's useful during development to run many more tests in release builds:
        // PROPTEST_CASES=102400 RUSTFLAGS="-C target-cpu=native -C debug-assertions=yes" cargo test --release --lib term_list
        #![proptest_config(
            ProptestConfig { cases: 256 * env_config("CONVEX_PROPTEST_MULTIPLIER", 1), failure_persistence: None, ..ProptestConfig::default() }
        )]

        #[test]
        fn proptest_term_iter_large(terms in prop::collection::vec(any::<(u32, u32)>(), 1..1200)) {
            test_term_iter(terms);
        }

        #[test]
        fn proptest_term_iter_small(term_bits in prop::collection::vec(any::<bool>(), 1..1200)) {
            let terms = term_bits
                .into_iter()
                .enumerate()
                .filter(|(_, b)| *b)
                .map(|(i, _)| (i as u32, i as u32))
                .collect();
            test_term_iter(terms);
        }

        #[test]
        fn proptest_matches_large(
            terms in prop::collection::vec(any::<(u32, u32)>(), 1..1200),
            queries in prop::collection::vec(any::<Query>(), 1..32)
        ) {
            test_matches(terms, queries);
        }

        #[test]
        fn proptest_matches_small(
            term_bits in prop::collection::vec(any::<bool>(), 1..1200),
            mut queries in prop::collection::vec(any::<Query>(), 1..32)
        ) {
            let n = term_bits.len() as u32;
            let terms = term_bits
                .into_iter()
                .enumerate()
                .filter(|(_, b)| *b)
                .map(|(i, _)| (i as u32, i as u32))
                .collect();

            // Adjust each query term to be within [0, n+1]
            for query in &mut queries {
                for term_id in &mut query.union_terms {
                    *term_id %= n + 1;
                }
                for term_id in &mut query.intersection_terms {
                    *term_id %= n + 1;
                }
            }
            test_matches(terms, queries);
        }

        #[test]
        fn proptest_matches_union_small(
            term_bits in prop::collection::vec(any::<bool>(), 1..1200),
        ) {
            let n = term_bits.len();
            let terms = term_bits
                .into_iter()
                .enumerate()
                .filter(|(_, b)| *b)
                .map(|(i, _)| (i as u32, i as u32))
                .collect();

            // Try all terms in the input bitset as well as one term (n + 1) that's definitely not in the termlist.
            let queries = (0..(n as u32 + 1))
                .map(|i| Query { union_terms: vec![i], intersection_terms: vec![] })
                .collect();
            test_matches(terms, queries);
        }

        #[test]
        fn proptest_matches_intersection_small(
            term_bits in prop::collection::vec(any::<bool>(), 1..1200),
        ) {
            let n = term_bits.len();
            let terms: Vec<(u32, u32)> = term_bits
                .iter()
                .cloned()
                .enumerate()
                .filter(|(_, b)| *b)
                .map(|(i, _)| (i as u32, i as u32))
                .collect();
            if let Some((union_term, _)) = terms.first() {
                // Include a single union term that's definitely in the termlist + every possible intersection list of size one.
                let queries = (0..(n as u32 + 1))
                    .map(|i| Query { union_terms: vec![*union_term], intersection_terms: vec![i] })
                    .collect();
                test_matches(terms, queries);
            }
        }

        #[test]
        #[ignore = "Reenable when we port randomized tests to search2"]
        fn proptest_matches_with_score_large(
            terms in prop::collection::vec(any::<(u32, u32)>(), 1..1200),
            queries in prop::collection::vec(any::<ScoredQuery>(), 1..32),
            fieldnorm in any::<u32>(),
            total_doc_freq in 32u64..64,
            avg_fieldnorm in prop::num::f32::POSITIVE
        ) {
            test_matches_with_score(terms, queries, fieldnorm, total_doc_freq, avg_fieldnorm);
        }
    }

    #[test]
    #[ignore = "Reenable when we port randomized tests to search2"]
    fn repro_test_matches_with_score_large() {
        let terms = vec![(2683748363, 0)];
        let queries = vec![ScoredQuery {
            union_terms: vec![
                UnionTerm {
                    term_id: 2683748363,
                    term_doc_freq: 1,
                },
                UnionTerm {
                    term_id: 0,
                    term_doc_freq: 2,
                },
            ],
            intersection_terms: vec![],
        }];
        let fieldnorm = 0;
        let total_doc_freq = 32;
        let avg_fieldnorm = 2.2986883e35;
        test_matches_with_score(terms, queries, fieldnorm, total_doc_freq, avg_fieldnorm);
    }
}

// Relevance heuristics used for ranking text search results
use std::collections::BTreeMap;

use tantivy::Score;

use crate::{
    query::{
        CandidateRevisionPositions,
        QueryTerm,
        ShortlistId,
        TermShortlist,
    },
    EditDistance,
};

/// Ranking generally requires knowing, for a given match (ShortlistId), what
/// query term indices within the original query that this was a match for and
/// at what distance.
///
/// This is essentially the inversion of
/// `TermShortlist::query_term_shortlist_items`. `Ranker` has an
/// `inverted_index` structure which memoizes a structure for answering
/// this question.
pub struct Ranker<'a> {
    query_terms: &'a Vec<QueryTerm>,
    shortlist: &'a TermShortlist,
    // Maps a shortlisted term ID to a collection of query indexes within `query`
    // that this ID belongs to. This collection is a mapping that also stores the
    // edit distance of this shortlisted term from the query term pointed to by the key.
    inverted_index: BTreeMap<ShortlistId, BTreeMap<QueryIndex, EditDistance>>,
}

type QueryIndex = usize;

impl<'a> Ranker<'a> {
    pub fn create(query_terms: &'a Vec<QueryTerm>, shortlist: &'a TermShortlist) -> Ranker<'a> {
        let mut inverted_index = BTreeMap::new();
        for (i, text_query) in query_terms.iter().enumerate() {
            let matches = shortlist.get_shortlisted_terms_for_query_term(text_query);

            for (dist, id) in matches {
                inverted_index
                    .entry(*id)
                    .or_insert_with(BTreeMap::new)
                    .insert(i, *dist);
            }
        }
        Self {
            query_terms,
            shortlist,
            inverted_index,
        }
    }

    /// Default entrypoint for scoring
    pub fn score(&self, candidate: &CandidateRevisionPositions) -> Score {
        score(
            vec![
                self.criterion_to_value::<OrderSensitiveWordOccurrence>(candidate),
                self.criterion_to_value::<Typos>(candidate),
                self.criterion_to_value::<ApproximateProximity>(candidate),
                self.criterion_to_value::<Exactness>(candidate),
            ]
            .into_iter(),
        )
    }

    /// A more expensive scoring that uses exact instead of approximate
    /// proximity
    #[allow(unused)]
    fn score_expensive(&self, candidate: &CandidateRevisionPositions) -> Score {
        score(
            vec![
                self.criterion_to_value::<OrderSensitiveWordOccurrence>(candidate),
                self.criterion_to_value::<Typos>(candidate),
                self.criterion_to_value::<Proximity>(candidate),
                self.criterion_to_value::<Exactness>(candidate),
            ]
            .into_iter(),
        )
    }

    fn criterion_to_value<T: RankCriterion>(
        &self,
        candidate: &CandidateRevisionPositions,
    ) -> RankCriterionValue {
        RankCriterionValue {
            value: T::value(self, &candidate.positions),
            max_value: T::max_value(self.query_terms),
        }
    }
}

/// Intermediate rank values used by `score` below to generate a score
struct RankCriterionValue {
    value: u32,
    max_value: u32,
}

/// Ranking function that generates a ranking score that respects the successive
/// sorting rule:   Document A should score higher than document B iff there
/// exists a criterion for which   all earlier criteria are equal and A's value
/// of this criterion is greater than B's.   If all criteria are equal, the
/// score must also be equal.
///
/// In other words, sorting documents by this ranking score is equivalent to
/// sorting the documents successively for each criterion, in order.
///
/// # Panics: Criteria must be non-empty or this will panic and divide by zero
fn score(criteria: impl Iterator<Item = RankCriterionValue>) -> Score {
    let mut result = RankCriterionValue {
        value: 0,
        max_value: 1,
    };
    for criterion in criteria {
        result.value *= criterion.max_value + 1;
        result.max_value *= criterion.max_value + 1;

        result.value += criterion.value;
    }
    result.max_value -= 1;
    result.value as Score / result.max_value as Score
}

/// A ranking criterion is comprised of a max_value, which must be exclusively a
/// function of the query and is independent of the document under
/// consideration, and a value which is a function of the candidate document,
/// query, and any other memoized state in the Ranker.
///
/// It is very important that the max value is independent of the candidate
/// under consideration (i.e. if it depends on candidate document length)!
/// Otherwise, this would make it useless to compare ranking scores across
/// documents and search indexes.
pub trait RankCriterion {
    fn max_value(query_terms: &Vec<QueryTerm>) -> u32;
    fn value(ranker: &Ranker, positions: &BTreeMap<ShortlistId, Vec<u32>>) -> u32;
}

/// Counts the max prefix of query terms that are matched by this document.
/// This is equivalent to the `Words` rule in
/// [Meillisearch](https://www.meilisearch.com/docs/learn/core_concepts/relevancy#1-words)
///
/// Value is in range [0, num_terms_in_query]
///
/// Examples:
/// - for query = "who is there?" and doc = "there is rakeeb", the
/// query has a max value of 3, but the document would score a value of 0.
/// since the word who is present.
/// - for query = "brown fox" and doc = "fox is brownd", the query has
/// a max value of 2 and the doc scores all 2 points since it contains a (fuzzy)
/// match  for "brown" and "fox"
struct OrderSensitiveWordOccurrence;

impl RankCriterion for OrderSensitiveWordOccurrence {
    fn max_value(query_terms: &Vec<QueryTerm>) -> u32 {
        query_terms.len() as u32
    }

    fn value(ranker: &Ranker, positions: &BTreeMap<ShortlistId, Vec<u32>>) -> u32 {
        let mut indices_with_match = vec![false; ranker.query_terms.len()];
        for (id, _) in positions.iter() {
            for index in ranker
                .inverted_index
                .get(id)
                .expect("Candidate matches a shortlist ID not in inverted index")
                .keys()
            {
                indices_with_match[*index] = true;
            }
        }

        indices_with_match.iter().take_while(|b| **b).count() as u32
    }
}

/// Counts the sum of typos of matches across all query terms. If there are
/// multiple matches for a query term, the minimum is chosen.
/// This is equivalent to the `Typos` rule in
/// [Meillisearch](https://www.meilisearch.com/docs/learn/core_concepts/relevancy#2-typo)
///
/// Value is in range [0, 3 * num_terms_in_query]. A value of 3 is assigned to
/// query terms without matches. A lower term is better in this score so the
/// final result is returned as `3 * num_terms_in_query - value` to respect that
/// higher scores should be better.
///
/// Examples:
/// - For query = "levenshtein distance", the doc = "levenshteiz winstance"
///   would have a max
/// typo value of 6, and achieves value 3: 1 typo in the first term and 2 in the
/// second term
struct Typos;

impl RankCriterion for Typos {
    fn max_value(query_terms: &Vec<QueryTerm>) -> u32 {
        3 * query_terms.len() as u32
    }

    fn value(ranker: &Ranker, positions: &BTreeMap<ShortlistId, Vec<u32>>) -> u32 {
        let mut min_distance_per_text_query: Vec<EditDistance> = vec![3; ranker.query_terms.len()];
        for (id, _) in positions.iter() {
            for (index, dist) in ranker.inverted_index.get(id).unwrap() {
                min_distance_per_text_query[*index] =
                    min_distance_per_text_query[*index].min(*dist);
            }
        }

        let total_distance = min_distance_per_text_query
            .into_iter()
            .fold(0_u32, |acc, v| acc + v as u32);
        Self::max_value(ranker.query_terms) - total_distance
    }
}

/// Quantifies the proximity of matches within the document, preferring matches
/// which respect the ordering of terms within the query. To do this, for each
/// query term in order, we consider each position that matched that query term
/// and its distance which each position of the last query term. We accumulate
/// the distances to get a total minimum distance. We cap distances in the score
/// to PROXIMITY_ORDINAL_MAX.
///
/// To penalize inversions of words in documents (out-of-order matches) with
/// negative distance `d`, they are assigned a distance of `|d| +
/// PROXIMITY_ORDINAL_MAX`.
///
/// The value is in range [0, 2 * PROXIMITY_ORDINAL_MAX * num_query_terms] since
/// distance is capped.
///
/// This metric approximates the `Proximity` rule in [Meillisearch](https://www.meilisearch.com/docs/learn/core_concepts/relevancy#3-proximity).
/// This is, however, not easy since Meillisearch maintains 3 separate disk
/// indexes for this: https://github.com/meilisearch/meilisearch/issues/3118.
struct Proximity;

const PROXIMITY_ORDINAL_MAX: u32 = 32;
const MAX_PROXIMITY_CONTRIBUTION_PER_QUERY_TERM: u32 = PROXIMITY_ORDINAL_MAX * 2;

impl RankCriterion for Proximity {
    fn max_value(query_terms: &Vec<QueryTerm>) -> u32 {
        MAX_PROXIMITY_CONTRIBUTION_PER_QUERY_TERM * query_terms.len() as u32
    }

    fn value(ranker: &Ranker, positions: &BTreeMap<ShortlistId, Vec<u32>>) -> u32 {
        // Vector of positions of matches on previous query term and the best
        // proximity score of all queries considered until this iteration that
        // end with selecting this position of the match.
        let mut dp: Vec<(u32, u32)> = vec![];
        let mut best_score = 0;

        // As of this comment, the total number of iterations here is ~= 16^4 = 2^16.
        for text_query in ranker.query_terms.iter() {
            let shortlisted_terms = ranker
                .shortlist
                .get_shortlisted_terms_for_query_term(text_query);

            let mut new_dp = vec![];
            let mut new_best_score = best_score + MAX_PROXIMITY_CONTRIBUTION_PER_QUERY_TERM;

            // Iterate through all the positions that matched this query term. For each
            // position, we compare the score against the positions and scores
            // of the last query term. This iteration considers at most
            // MAX_FUZZY_MATCHES_PER_QUERY_TERM * MAX_POSITIONS_PER_MATCHED_TERM
            // * MAX_POSITIONS_PER_MATCHED_TERM ~= 2^12 positions.
            for (_, shortlist_id) in shortlisted_terms {
                if let Some(positions) = positions.get(shortlist_id) {
                    for position in positions {
                        let mut score = best_score + MAX_PROXIMITY_CONTRIBUTION_PER_QUERY_TERM;
                        for (prev_pos, prev_score) in &dp {
                            // Can't pick the same position twice
                            if prev_pos == position {
                                continue;
                            }
                            // Calculate distance
                            let dist = *position as i32 - *prev_pos as i32;

                            // Constrain absolute value to PROXIMITY_ORDINAL_MAX
                            let mut capped_absolute_dist =
                                dist.abs().min(PROXIMITY_ORDINAL_MAX as i32) as u32;

                            // Penalize negative scores further
                            if dist < 0 {
                                capped_absolute_dist += PROXIMITY_ORDINAL_MAX;
                            }
                            score = score.min(prev_score + capped_absolute_dist);
                        }
                        new_best_score = new_best_score.min(score);
                        new_dp.push((*position, score));
                    }
                }
            }

            dp = new_dp;
            best_score = new_best_score;
        }

        Self::max_value(ranker.query_terms) - best_score
    }
}

/// Greedy, approximate version of `Proximity`. Much faster to calculate but
/// less accurate results. In best-case, ~MAX_POSITIONS_PER_MATCHED_TERMx faster
/// than `Proximity`.
///
/// This has the added bonus of scoring higher documents with matches at the
/// start of the document.
struct ApproximateProximity;

impl RankCriterion for ApproximateProximity {
    fn max_value(query_terms: &Vec<QueryTerm>) -> u32 {
        MAX_PROXIMITY_CONTRIBUTION_PER_QUERY_TERM * query_terms.len() as u32
    }

    fn value(ranker: &Ranker, positions: &BTreeMap<ShortlistId, Vec<u32>>) -> u32 {
        let mut total_score = 0;
        let mut prev_pos = 0;

        for text_query in ranker.query_terms.iter() {
            let shortlisted_terms = ranker
                .shortlist
                .get_shortlisted_terms_for_query_term(text_query);

            let mut score_pos: Option<(u32, u32)> = None;

            // Iterate through all the positions that matched this query term.
            // There are at most MAX_FUZZY_MATCHES_PER_QUERY_TERM *
            // MAX_POSITIONS_PER_MATCHED_TERM such matches.
            for (_, shortlist_id) in shortlisted_terms {
                if let Some(positions) = positions.get(shortlist_id) {
                    for position in positions {
                        let dist = *position as i32 - prev_pos as i32;

                        // Constrain absolute value to PROXIMITY_ORDINAL_MAX
                        let mut capped_absolute_dist =
                            dist.abs().min(PROXIMITY_ORDINAL_MAX as i32) as u32;

                        // Penalize negative scores further
                        if dist < 0 {
                            capped_absolute_dist += PROXIMITY_ORDINAL_MAX;
                        }
                        if let Some((score, pos)) = &mut score_pos {
                            if *score > capped_absolute_dist {
                                *score = capped_absolute_dist;
                                *pos = *position;
                            }
                        } else {
                            score_pos = Some((capped_absolute_dist, *position));
                        }
                    }
                }
            }

            if let Some((score, pos)) = score_pos {
                total_score += score;
                prev_pos = pos;
            } else {
                total_score += MAX_PROXIMITY_CONTRIBUTION_PER_QUERY_TERM;
            }
        }

        Self::max_value(ranker.query_terms) - total_score
    }
}

/// Counts the number of query terms that with exact matches.
/// Equivalent to the `Exact` rule in [Algolia](https://www.algolia.com/doc/guides/managing-results/relevance-overview/in-depth/ranking-criteria/#exact)
///
/// Value is in [0, num_terms_in_query]
///
/// Examples:
/// - Query = "fuzzy match", Doc = "fizzy match" would score a value of 1 with a
///   max Exact
/// value of 2.
struct Exactness;

impl RankCriterion for Exactness {
    fn max_value(query_terms: &Vec<QueryTerm>) -> u32 {
        query_terms.len() as u32
    }

    fn value(ranker: &Ranker, positions: &BTreeMap<ShortlistId, Vec<u32>>) -> u32 {
        let mut has_exact = vec![false; ranker.query_terms.len()];
        for (id, _) in positions.iter() {
            for (index, dist) in ranker.inverted_index.get(id).unwrap() {
                if *dist == 0 {
                    has_exact[*index] = true;
                }
            }
        }
        has_exact.into_iter().filter(|b| *b).count() as u32
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use common::{
        document::CreationTime,
        types::WriteTimestamp,
    };
    use itertools::Itertools;
    use tantivy::Score;
    use value::InternalId;

    use crate::{
        query::{
            CandidateRevisionPositions,
            QueryTerm,
            ShortlistId,
            TermShortlist,
        },
        ranking::Ranker,
        scoring::term_from_str,
        CandidateRevision,
        DocumentTerm,
        FieldPosition,
        MemorySearchIndex,
    };

    // Builds a term shortlist using the memory index logic. This is useful
    fn build_shortlist(query: &Vec<QueryTerm>, words: Vec<&'static str>) -> TermShortlist {
        let mut memory_index = MemorySearchIndex::new(WriteTimestamp::Pending);

        let doc_with_all_words = words
            .into_iter()
            .enumerate()
            .map(|(i, w)| DocumentTerm::Search {
                term: term_from_str(w),
                pos: FieldPosition::new_for_test(i as u32),
            })
            .collect_vec();

        memory_index
            .update(
                InternalId::MAX,
                WriteTimestamp::Pending,
                None,
                Some((doc_with_all_words, CreationTime::ONE)),
            )
            .unwrap();
        memory_index.bound_and_evaluate_query_terms(query).0
    }

    fn create_query_term(word: &'static str, dist: u8, prefix: bool) -> QueryTerm {
        if dist == 0 && !prefix {
            QueryTerm::Exact(term_from_str(word))
        } else {
            QueryTerm::Fuzzy {
                term: term_from_str(word),
                max_distance: dist,
                prefix,
            }
        }
    }

    fn candidate(
        words: Vec<&'static str>,
        shortlist: &TermShortlist,
    ) -> CandidateRevisionPositions {
        let index: BTreeMap<String, ShortlistId> = shortlist
            .ids_and_terms()
            .map(|(id, term)| (term.as_str().unwrap().to_string(), id))
            .collect();

        let mut positions = BTreeMap::<ShortlistId, Vec<u32>>::new();
        for (i, word) in words.into_iter().enumerate() {
            if let Some(id) = index.get(word) {
                positions.entry(*id).or_default().push(i as u32);
            }
        }

        CandidateRevisionPositions {
            revision: CandidateRevision {
                id: InternalId::MAX,
                creation_time: CreationTime::ONE,
                score: 0.,
                ts: WriteTimestamp::Pending,
            },
            positions,
        }
    }

    fn get_scores(
        query_terms: Vec<QueryTerm>,
        doc1: &'static str,
        doc2: &'static str,
    ) -> (Score, Score) {
        let doc1 = doc1.split(' ').collect_vec();
        let doc2 = doc2.split(' ').collect_vec();

        let dictionary = doc1.clone().into_iter().chain(doc2.clone()).collect_vec();
        let shortlist = build_shortlist(&query_terms, dictionary);
        let ranker = Ranker::create(&query_terms, &shortlist);

        let score1 = ranker.score(&candidate(doc1, &shortlist));
        let score2 = ranker.score(&candidate(doc2, &shortlist));
        (score1, score2)
    }

    #[test]
    fn test_ranking_ordering() {
        {
            let query_terms = vec![
                create_query_term("some", 0, false),
                create_query_term("prefix", 1, true),
            ];
            // The first wins on more order-sensitive word occurrences
            let (first, sec) = get_scores(
                query_terms,
                "some query prefizblahblah",
                "some query random",
            );
            assert!(first > sec);
        }
        {
            let query_terms = vec![
                create_query_term("some", 0, false),
                create_query_term("prefix", 1, true),
            ];
            // The first wins on fewer typos
            let (first, sec) = get_scores(
                query_terms,
                "some query prefixblahblah",
                "some query prefioblahblah",
            );
            assert!(first > sec);
        }
        {
            let query_terms = vec![
                create_query_term("the", 0, false),
                create_query_term("dark", 0, false),
                create_query_term("knight", 0, false),
            ];
            // The second wins on proximity
            let (first, sec) = get_scores(query_terms, "knight the dark", "the  dark knight");
            assert!(first < sec);
        }
        {
            let query_terms = vec![
                create_query_term("test", 0, false),
                create_query_term("123", 0, false),
            ];
            // The second wins on proximity
            let (first, sec) = get_scores(query_terms, "123 test", "test 123");
            assert!(first < sec);
        }
        {
            let query_terms = vec![
                create_query_term("the", 2, false),
                create_query_term("dark", 2, false),
                create_query_term("knight", 2, false),
            ];
            // The second wins on exactness
            let (first, sec) = get_scores(query_terms, "he dank night", "the dank knightlp");
            assert!(first < sec);
        }
    }
}

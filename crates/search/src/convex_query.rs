use std::{
    collections::BTreeSet,
    fmt,
};

use tantivy::{
    fastfield::AliveBitSet,
    query::{
        intersect_scorers,
        BitSetDocSet,
        BooleanQuery,
        BoostQuery,
        EmptyScorer,
        EnableScoring,
        Explanation,
        Occur,
        Query,
        Scorer,
        TermQuery,
        Weight,
    },
    schema::IndexRecordOption,
    DocId,
    DocSet,
    Score,
    SegmentReader,
    Term,
    TERMINATED,
};
use tantivy_common::ReadOnlyBitSet;

/// A query for documents that:
/// 1. Contain at least one of the OR terms.
/// 2. Match all of the AND terms.
///
/// Unlike tantivy's BooleanQuery, this query will be scored only by the or
/// terms.
#[derive(Clone, Debug)]
pub struct ConvexSearchQuery {
    or_query: BooleanQuery,
    and_queries: Vec<TermQuery>,
    alive_documents: AliveDocuments,
}

impl ConvexSearchQuery {
    #[allow(clippy::new_ret_no_self)]
    pub fn new(
        or_terms: Vec<OrTerm>,
        and_terms: Vec<Term>,
        alive_documents: AliveDocuments,
    ) -> Box<dyn Query> {
        let or_queries = or_terms
            .into_iter()
            .map(|t| {
                let term_query = TermQuery::new(t.term, IndexRecordOption::WithFreqs);
                let boosted = BoostQuery::new(Box::new(term_query), t.bm25_boost);
                (Occur::Should, Box::new(boosted) as Box<dyn Query>)
            })
            .collect();
        let or_query = BooleanQuery::new(or_queries);
        let and_queries: Vec<_> = and_terms
            .into_iter()
            .map(|filter_term| TermQuery::new(filter_term, IndexRecordOption::Basic))
            .collect();
        Box::new(Self {
            or_query,
            and_queries,
            alive_documents,
        })
    }
}

impl Query for ConvexSearchQuery {
    fn weight(
        &self,
        enable_scoring: tantivy::query::EnableScoring<'_>,
    ) -> tantivy::Result<Box<dyn tantivy::query::Weight>> {
        let or_weight = self.or_query.weight(enable_scoring)?;
        let and_weights = self
            .and_queries
            .iter()
            .map(|and_query| {
                and_query.weight(EnableScoring::disabled_from_searcher(
                    enable_scoring
                        .searcher()
                        .expect("EnableScoring is missing searcher"),
                ))
            })
            .collect::<tantivy::Result<Vec<_>>>()?;

        Ok(Box::new(ConvexSearchWeight {
            or_weight,
            and_weights,
            alive_documents: self.alive_documents.clone(),
        }))
    }

    fn query_terms<'a>(&'a self, visitor: &mut dyn FnMut(&'a Term, bool)) {
        self.or_query.query_terms(visitor);
        for filter_query in &self.and_queries {
            filter_query.query_terms(visitor);
        }
    }
}

struct ConvexSearchWeight {
    or_weight: Box<dyn Weight>,
    and_weights: Vec<Box<dyn Weight>>,
    alive_documents: AliveDocuments,
}

impl Weight for ConvexSearchWeight {
    fn scorer(&self, reader: &SegmentReader, boost: Score) -> tantivy::Result<Box<dyn Scorer>> {
        let mut and_scorers: Vec<Box<dyn Scorer>> = vec![Box::new(self.alive_documents.scorer())];
        for filter_weight in &self.and_weights {
            and_scorers.push(filter_weight.scorer(reader, boost)?);
        }
        let query_scorer = intersect_scorers_and_use_one_for_scores(
            self.or_weight.scorer(reader, boost)?,
            intersect_scorers(and_scorers),
        );
        Ok(Box::new(query_scorer))
    }

    fn explain(&self, reader: &SegmentReader, doc: DocId) -> tantivy::Result<Explanation> {
        let mut scorer = self.scorer(reader, 1.0)?;
        let mut explanation = Explanation::new("ConvexSearchWeight", scorer.score());
        if let Ok(child_explanation) = self.or_weight.explain(reader, doc) {
            explanation.add_detail(child_explanation);
        }
        Ok(explanation)
    }
}

#[derive(Clone)]
pub struct AliveDocuments {
    pub memory_deleted: BTreeSet<DocId>,
    pub segment_alive_bitset: AliveBitSet,
}

impl fmt::Debug for AliveDocuments {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AliveDocuments")
            .field("memory_deleted", &self.memory_deleted)
            .field(
                "num_segment_alive",
                &self.segment_alive_bitset.num_alive_docs(),
            )
            .finish()
    }
}

impl AliveDocuments {
    pub fn scorer(&self) -> AliveDocumentsScorer {
        AliveDocumentsScorer::new(
            self.memory_deleted.clone(),
            self.segment_alive_bitset.bitset().clone().into(),
        )
    }
}

pub struct AliveDocumentsScorer {
    memory_deleted: BTreeSet<DocId>,
    docs: BitSetDocSet<ReadOnlyBitSet>,
}

impl AliveDocumentsScorer {
    pub fn new(memory_deleted: BTreeSet<DocId>, mut docs: BitSetDocSet<ReadOnlyBitSet>) -> Self {
        let mut candidate = docs.doc();
        loop {
            if candidate == TERMINATED || !memory_deleted.contains(&candidate) {
                break;
            }
            candidate = docs.advance();
        }
        Self {
            memory_deleted,
            docs,
        }
    }
}

impl DocSet for AliveDocumentsScorer {
    fn advance(&mut self) -> DocId {
        loop {
            let candidate = self.docs.advance();
            if candidate == TERMINATED {
                return TERMINATED;
            }
            if !self.memory_deleted.contains(&candidate) {
                return candidate;
            }
        }
    }

    fn seek(&mut self, target: DocId) -> DocId {
        let doc = self.docs.seek(target);
        if self.memory_deleted.contains(&doc) {
            self.advance()
        } else {
            doc
        }
    }

    fn doc(&self) -> DocId {
        self.docs.doc()
    }

    fn size_hint(&self) -> u32 {
        self.docs
            .size_hint()
            .saturating_sub(self.memory_deleted.len() as u32)
    }
}

impl Scorer for AliveDocumentsScorer {
    fn score(&mut self) -> Score {
        1.0
    }
}

/// Intersect two scorers using only one to compute the score.
///
/// This is similar to `tantivy::intersect_scorers` but it only uses one of the
/// scorers for scoring instead of adding the scores.
pub fn intersect_scorers_and_use_one_for_scores(
    scorer_for_scoring: Box<dyn Scorer>,
    scorer_for_filtering: Box<dyn Scorer>,
) -> Box<dyn Scorer> {
    let mut score_left = true;

    let mut scorers = if scorer_for_scoring.size_hint() < scorer_for_filtering.size_hint() {
        vec![scorer_for_scoring, scorer_for_filtering]
    } else {
        score_left = false;
        vec![scorer_for_filtering, scorer_for_scoring]
    };

    let doc = go_to_first_doc(&mut scorers[..]);
    if doc == TERMINATED {
        return Box::new(EmptyScorer);
    }
    let left = scorers.remove(0);
    let right = scorers.remove(0);
    Box::new(Intersection {
        left,
        right,
        score_left,
    })
}

/// Creates a `DocSet` that iterate through the intersection of two `DocSet`s
/// using only one of them for scoring.
///
/// This is similar to tantivy's Intersection but it only uses one scorer to
/// score.
pub struct Intersection<TDocSet: DocSet> {
    left: TDocSet,
    right: TDocSet,
    score_left: bool,
}

fn go_to_first_doc<TDocSet: DocSet>(docsets: &mut [TDocSet]) -> DocId {
    assert!(!docsets.is_empty());
    let mut candidate = docsets.iter().map(TDocSet::doc).max().unwrap();
    'outer: loop {
        for docset in docsets.iter_mut() {
            let seek_doc = docset.seek(candidate);
            if seek_doc > candidate {
                candidate = docset.doc();
                continue 'outer;
            }
        }
        return candidate;
    }
}

impl<TDocSet: DocSet> DocSet for Intersection<TDocSet> {
    fn advance(&mut self) -> DocId {
        let (left, right) = (&mut self.left, &mut self.right);
        let mut candidate = left.advance();

        loop {
            let right_doc = right.seek(candidate);
            candidate = left.seek(right_doc);
            if candidate == right_doc {
                break;
            }
        }

        debug_assert_eq!(candidate, self.left.doc());
        debug_assert_eq!(candidate, self.right.doc());
        candidate
    }

    fn seek(&mut self, target: DocId) -> DocId {
        self.left.seek(target);
        let mut docsets: Vec<&mut dyn DocSet> = vec![&mut self.left, &mut self.right];
        let doc = go_to_first_doc(&mut docsets[..]);
        debug_assert!(docsets.iter().all(|docset| docset.doc() == doc));
        debug_assert!(doc >= target);
        doc
    }

    fn doc(&self) -> DocId {
        self.left.doc()
    }

    fn size_hint(&self) -> u32 {
        self.left.size_hint()
    }
}

impl<TScorer> Scorer for Intersection<TScorer>
where
    TScorer: Scorer,
{
    fn score(&mut self) -> Score {
        if self.score_left {
            self.left.score()
        } else {
            self.right.score()
        }
    }
}

#[derive(Clone)]
pub struct OrTerm {
    pub term: Term,
    pub doc_frequency: u64,
    pub bm25_boost: f32,
}

impl TryFrom<pb::searchlight::OrTerm> for OrTerm {
    type Error = anyhow::Error;

    fn try_from(value: pb::searchlight::OrTerm) -> Result<Self, Self::Error> {
        Ok(OrTerm {
            term: Term::wrap(value.term),
            doc_frequency: value.doc_frequency,
            bm25_boost: value.bm25_boost,
        })
    }
}

impl TryFrom<OrTerm> for pb::searchlight::OrTerm {
    type Error = anyhow::Error;

    fn try_from(value: OrTerm) -> Result<Self, Self::Error> {
        Ok(pb::searchlight::OrTerm {
            term: value.term.as_slice().to_vec(),
            doc_frequency: value.doc_frequency,
            bm25_boost: value.bm25_boost,
        })
    }
}

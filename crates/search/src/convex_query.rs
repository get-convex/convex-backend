use std::{
    collections::BTreeSet,
    fmt,
};

use tantivy::{
    fastfield::AliveBitSet,
    query::{
        intersect_scorers,
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
    deleted_documents: DeletedDocuments,
}

impl ConvexSearchQuery {
    #[allow(clippy::new_ret_no_self)]
    pub fn new(
        or_terms: Vec<OrTerm>,
        and_terms: Vec<Term>,
        deleted_documents: DeletedDocuments,
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
            deleted_documents,
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
            deleted_documents: self.deleted_documents.clone(),
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
    deleted_documents: DeletedDocuments,
}

impl Weight for ConvexSearchWeight {
    fn scorer(&self, reader: &SegmentReader, boost: Score) -> tantivy::Result<Box<dyn Scorer>> {
        let and_scorers = self
            .and_weights
            .iter()
            .map(|filter_weight| filter_weight.scorer(reader, boost))
            .collect::<tantivy::Result<Vec<_>>>()?;
        let query_scorer = if !and_scorers.is_empty() {
            intersect_scorers_and_use_one_for_scores(
                self.or_weight.scorer(reader, boost)?,
                intersect_scorers(and_scorers),
            )
        } else {
            self.or_weight.scorer(reader, boost)?
        };
        Ok(Box::new(ExcludeDeleted::new(
            query_scorer,
            self.deleted_documents.clone(),
        )))
    }

    fn explain(&self, reader: &SegmentReader, doc: DocId) -> tantivy::Result<Explanation> {
        let mut scorer = self.scorer(reader, 1.0)?;
        let mut explanation = Explanation::new("ConvexSearchWeight", scorer.score());
        if let Ok(child_explanation) = self.or_weight.explain(reader, doc) {
            explanation.add_detail(child_explanation);
        }
        Ok(explanation)
    }

    // TODO: Forward call to or_weight for BlockWAND optimization.
    // fn for_each_pruning(
    //     &self,
    //     threshold: Score,
    //     reader: &SegmentReader,
    //     callback: &mut dyn FnMut(DocId, Score) -> Score,
    // ) -> tantivy::Result<()> {
    //     todo!();
    // }
}

#[derive(Clone)]
pub struct DeletedDocuments {
    pub memory_deleted: BTreeSet<DocId>,
    pub segment_alive_bitset: AliveBitSet,
}

impl fmt::Debug for DeletedDocuments {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DeletedDocuments")
            .field("memory_deleted", &self.memory_deleted)
            .field("segment_deleted", &"<bitset>")
            .field(
                "num_segment_alive",
                &self.segment_alive_bitset.num_alive_docs(),
            )
            .finish()
    }
}

impl DeletedDocuments {
    pub fn contains(&self, doc: DocId) -> bool {
        self.memory_deleted.contains(&doc) || self.segment_alive_bitset.is_deleted(doc)
    }

    pub fn approximate_num_alive_docs(&self) -> usize {
        self.segment_alive_bitset
            .num_alive_docs()
            .saturating_sub(self.memory_deleted.len())
    }
}

pub struct ExcludeDeleted<T: DocSet> {
    docset: T,
    deleted_documents: DeletedDocuments,
}

impl<T: DocSet> ExcludeDeleted<T> {
    pub fn new(docset: T, deleted_documents: DeletedDocuments) -> Self {
        let mut s = Self {
            docset,
            deleted_documents,
        };
        while s.docset.doc() != TERMINATED {
            let target = s.docset.doc();
            if !s.deleted_documents.contains(target) {
                break;
            }
            s.docset.advance();
        }
        s
    }
}

impl<T: DocSet> DocSet for ExcludeDeleted<T> {
    fn advance(&mut self) -> DocId {
        loop {
            let candidate = self.docset.advance();
            if candidate == TERMINATED {
                return TERMINATED;
            }
            if !self.deleted_documents.contains(candidate) {
                return candidate;
            }
        }
    }

    fn seek(&mut self, target: DocId) -> DocId {
        let candidate = self.docset.seek(target);
        if candidate == TERMINATED {
            return TERMINATED;
        }
        if !self.deleted_documents.contains(candidate) {
            return candidate;
        }
        self.advance()
    }

    fn doc(&self) -> DocId {
        self.docset.doc()
    }

    fn size_hint(&self) -> u32 {
        self.deleted_documents.approximate_num_alive_docs() as u32
    }
}

impl<T: Scorer> Scorer for ExcludeDeleted<T> {
    fn score(&mut self) -> Score {
        self.docset.score()
    }

    // TODO: Forward call to or_weight for BlockWAND optimization.
    // fn for_each_pruning(
    //     &self,
    //     threshold: Score,
    //     reader: &SegmentReader,
    //     callback: &mut dyn FnMut(DocId, Score) -> Score,
    // ) -> tantivy::Result<()> {
    //     todo!();
    // }
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

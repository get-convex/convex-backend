use std::collections::BTreeMap;

use tantivy::{
    query::Bm25StatisticsProvider,
    schema::Field,
    Score,
    TantivyError,
    Term,
};

use crate::{
    EditDistance,
    SEARCH_FIELD_ID,
};

pub fn term_from_str(term_value: &str) -> Term {
    Term::from_field_text(Field::from_field_id(SEARCH_FIELD_ID), term_value)
}

/// Per-term statistics used to compute BM25 scores.
///
/// Note that this only includes terms for the search field of the query.
/// Filter fields are not included.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(any(test, feature = "testing"), derive(proptest_derive::Arbitrary))]
pub struct Bm25StatisticsDiff {
    pub term_statistics: BTreeMap<String, i64>,
    pub num_documents_diff: i64,
    pub num_search_tokens_diff: i64,
}

impl Bm25StatisticsDiff {
    pub fn combine(mut self, other: Self) -> Self {
        self.num_documents_diff += other.num_documents_diff;
        self.num_search_tokens_diff += other.num_search_tokens_diff;

        for (term, freq) in other.term_statistics {
            *self.term_statistics.entry(term).or_insert(0) += freq;
        }
        self
    }
}

impl Bm25StatisticsProvider for Bm25StatisticsDiff {
    fn total_num_tokens(&self, _field: Field) -> tantivy::Result<u64> {
        u64::try_from(self.num_search_tokens_diff)
            .map_err(|err| TantivyError::InternalError(err.to_string()))
    }

    fn total_num_docs(&self) -> tantivy::Result<u64> {
        let total = u64::try_from(self.num_documents_diff)
            .map_err(|err| TantivyError::InternalError(err.to_string()))?;

        // We can end up with 0 documents if we're querying tantivy and all of the
        // documents have been deleted since the snapshot.

        // Returning 0 here creates a divide by 0 in the BM25 calcuation. Use
        // 1 instead so we'll get scores and then the documents will be filtered
        // out later because they were all deleted.
        Ok(if total == 0 { 1 } else { total })
    }

    fn doc_freq(&self, term: &Term) -> tantivy::Result<u64> {
        match term.as_str() {
            None => Err(TantivyError::InternalError(format!(
                "Expected text term to have text. Actual type: {:?}",
                term.typ()
            ))),
            Some(term_str) => {
                let num_documents_with_term_diff_opt = self.term_statistics.get(term_str);

                match num_documents_with_term_diff_opt {
                    None => Err(TantivyError::InternalError(
                        "Unable to find term statistics for term".to_string(),
                    )),
                    Some(num_documents_with_term_diff) => {
                        u64::try_from(*num_documents_with_term_diff)
                            .map_err(|err| TantivyError::InternalError(err.to_string()))
                    },
                }
            },
        }
    }
}

impl From<pb::searchlight::Bm25StatisticsDiff> for Bm25StatisticsDiff {
    fn from(proto: pb::searchlight::Bm25StatisticsDiff) -> Self {
        Bm25StatisticsDiff {
            term_statistics: proto.term_statistics.into_iter().collect(),
            num_documents_diff: proto.num_documents_diff,
            num_search_tokens_diff: proto.num_search_tokens_diff,
        }
    }
}

impl From<Bm25StatisticsDiff> for pb::searchlight::Bm25StatisticsDiff {
    fn from(stats: Bm25StatisticsDiff) -> Self {
        pb::searchlight::Bm25StatisticsDiff {
            term_statistics: stats.term_statistics.into_iter().collect(),
            num_documents_diff: stats.num_documents_diff,
            num_search_tokens_diff: stats.num_search_tokens_diff,
        }
    }
}

/// TODO: try 1 / (1 + distance) later
pub fn bm25_weight_boost_for_edit_distance(_distance: EditDistance) -> Score {
    1.
}

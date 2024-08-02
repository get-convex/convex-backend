use std::collections::BTreeMap;

use anyhow::Context;
use pb::searchlight::QueryResponse;
use tantivy::schema::Field;

use crate::{
    query::{
        CandidateRevisionPositions,
        ShortlistId,
        TermShortlist,
    },
    scoring::Bm25StatisticsDiff,
    CandidateRevision,
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

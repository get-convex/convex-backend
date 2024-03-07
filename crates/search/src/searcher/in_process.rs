/// Searcher trait and implementations
/// - Stub implementation
/// - InProcessSearcher implementation
use std::sync::Arc;

use async_trait::async_trait;
use common::{
    bootstrap_model::index::vector_index::FragmentedVectorSegment,
    runtime::Runtime,
    types::ObjectKey,
};
use pb::searchlight::FragmentedVectorSegmentPaths;
use storage::Storage;
use tempfile::TempDir;
use vector::{
    CompiledVectorSearch,
    QdrantSchema,
    VectorSearchQueryResult,
    VectorSearcher,
};

use crate::{
    query::{
        CompiledQuery,
        TermShortlist,
    },
    scoring::Bm25StatisticsDiff,
    searcher::SearcherImpl,
    tantivy_query::SearchQueryResult,
    Searcher,
    TantivySearchIndexSchema,
};

/// Dummy client that will always return no results.
#[derive(Copy, Clone)]
pub struct SearcherStub;
#[async_trait]
impl Searcher for SearcherStub {
    async fn execute_query(
        &self,
        _search_storage: Arc<dyn Storage>,
        _disk_index: &ObjectKey,
        _schema: &TantivySearchIndexSchema,
        _search: CompiledQuery,
        _memory_statistics_diff: Bm25StatisticsDiff,
        _shortlisted_terms: TermShortlist,
        _limit: usize,
    ) -> anyhow::Result<SearchQueryResult> {
        Ok(SearchQueryResult::empty())
    }
}

#[async_trait]
impl VectorSearcher for SearcherStub {
    async fn execute_multi_segment_vector_query(
        &self,
        _search_storage: Arc<dyn Storage>,
        _segments: Vec<FragmentedVectorSegmentPaths>,
        _schema: QdrantSchema,
        _search: CompiledVectorSearch,
        _overfetch_delta: u32,
    ) -> anyhow::Result<Vec<VectorSearchQueryResult>> {
        Ok(vec![])
    }

    async fn execute_vector_compaction(
        &self,
        _search_storage: Arc<dyn Storage>,
        _segments: Vec<FragmentedVectorSegmentPaths>,
        _dimension: usize,
    ) -> anyhow::Result<FragmentedVectorSegment> {
        anyhow::bail!("Not implemented!");
    }
}

#[derive(Clone)]
pub struct InProcessSearcher<RT: Runtime> {
    searcher: Arc<SearcherImpl<RT>>,
    _tmpdir: Arc<TempDir>,
}

impl<RT: Runtime> InProcessSearcher<RT> {
    pub async fn new(runtime: RT) -> anyhow::Result<Self> {
        let tmpdir = TempDir::new()?;
        Ok(Self {
            searcher: Arc::new(
                SearcherImpl::new(tmpdir.path(), bytesize::mib(500u64), 100, false, runtime)
                    .await?,
            ),
            _tmpdir: Arc::new(tmpdir),
        })
    }
}

#[async_trait]
impl<RT: Runtime> Searcher for InProcessSearcher<RT> {
    async fn execute_query(
        &self,
        search_storage: Arc<dyn Storage>,
        disk_index: &ObjectKey,
        schema: &TantivySearchIndexSchema,
        search: CompiledQuery,
        memory_statistics_diff: Bm25StatisticsDiff,
        memory_shortlisted_terms: TermShortlist,
        limit: usize,
    ) -> anyhow::Result<SearchQueryResult> {
        self.searcher
            .execute_query(
                search_storage,
                disk_index,
                schema,
                search,
                memory_statistics_diff,
                memory_shortlisted_terms,
                limit,
            )
            .await
    }
}

#[async_trait]
impl<RT: Runtime> VectorSearcher for InProcessSearcher<RT> {
    async fn execute_multi_segment_vector_query(
        &self,
        search_storage: Arc<dyn Storage>,
        segments: Vec<FragmentedVectorSegmentPaths>,
        schema: QdrantSchema,
        search: CompiledVectorSearch,
        overfetch_delta: u32,
    ) -> anyhow::Result<Vec<VectorSearchQueryResult>> {
        self.searcher
            .execute_multi_segment_vector_query(
                search_storage,
                segments,
                schema,
                search,
                overfetch_delta,
            )
            .await
    }

    async fn execute_vector_compaction(
        &self,
        search_storage: Arc<dyn Storage>,
        segments: Vec<FragmentedVectorSegmentPaths>,
        dimension: usize,
    ) -> anyhow::Result<FragmentedVectorSegment> {
        self.searcher
            .execute_vector_compaction(search_storage, segments, dimension)
            .await
    }
}

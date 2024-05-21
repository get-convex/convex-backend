/// Searcher trait and implementations
/// - Stub implementation
/// - InProcessSearcher implementation
use std::{
    collections::BTreeMap,
    sync::Arc,
};

use async_trait::async_trait;
use common::{
    bootstrap_model::index::vector_index::FragmentedVectorSegment,
    runtime::Runtime,
    types::ObjectKey,
};
use pb::searchlight::FragmentedVectorSegmentPaths;
use storage::Storage;
use tantivy::Term;
use tempfile::TempDir;
use vector::{
    CompiledVectorSearch,
    QdrantSchema,
    VectorSearchQueryResult,
    VectorSearcher,
};

use super::searcher::{
    Bm25Stats,
    PostingListMatch,
    PostingListQuery,
    TextStorageKeys,
    TokenMatch,
    TokenQuery,
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

    async fn number_of_segments(
        &self,
        _search_storage: Arc<dyn Storage>,
        _storage_key: ObjectKey,
    ) -> anyhow::Result<usize> {
        Ok(1)
    }

    async fn query_tokens(
        &self,
        _search_storage: Arc<dyn Storage>,
        _storage_keys: TextStorageKeys,
        _queries: Vec<TokenQuery>,
        _max_results: usize,
    ) -> anyhow::Result<Vec<TokenMatch>> {
        Ok(vec![])
    }

    async fn query_bm25_stats(
        &self,
        _search_storage: Arc<dyn Storage>,
        _storage_keys: TextStorageKeys,
        _terms: Vec<Term>,
    ) -> anyhow::Result<Bm25Stats> {
        Ok(Bm25Stats {
            num_terms: 0,
            num_documents: 0,
            doc_frequencies: BTreeMap::new(),
        })
    }

    async fn query_posting_lists(
        &self,
        _search_storage: Arc<dyn Storage>,
        _storage_keys: TextStorageKeys,
        _query: PostingListQuery,
    ) -> anyhow::Result<Vec<PostingListMatch>> {
        Ok(vec![])
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

    async fn number_of_segments(
        &self,
        search_storage: Arc<dyn Storage>,
        storage_key: ObjectKey,
    ) -> anyhow::Result<usize> {
        self.searcher
            .number_of_segments(search_storage, storage_key)
            .await
    }

    async fn query_tokens(
        &self,
        search_storage: Arc<dyn Storage>,
        storage_keys: TextStorageKeys,
        queries: Vec<TokenQuery>,
        max_results: usize,
    ) -> anyhow::Result<Vec<TokenMatch>> {
        self.searcher
            .query_tokens(search_storage, storage_keys, queries, max_results)
            .await
    }

    async fn query_bm25_stats(
        &self,
        search_storage: Arc<dyn Storage>,
        storage_keys: TextStorageKeys,
        terms: Vec<Term>,
    ) -> anyhow::Result<Bm25Stats> {
        self.searcher
            .query_bm25_stats(search_storage, storage_keys, terms)
            .await
    }

    async fn query_posting_lists(
        &self,
        search_storage: Arc<dyn Storage>,
        storage_keys: TextStorageKeys,
        query: PostingListQuery,
    ) -> anyhow::Result<Vec<PostingListMatch>> {
        self.searcher
            .query_posting_lists(search_storage, storage_keys, query)
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

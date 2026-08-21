use std::collections::BTreeMap;
/// Searcher trait and implementations
/// - Stub implementation
/// - InProcessSearcher implementation
use std::sync::Arc;

use anyhow::Context as _;
use async_trait::async_trait;
use common::{
    bootstrap_model::index::{
        text_index::FragmentedTextSegment,
        vector_index::FragmentedVectorSegment,
    },
    runtime::Runtime,
    types::{
        ObjectKey,
        SearchIndexMetricLabels,
    },
};
use pb::searchlight::FragmentedVectorSegmentPaths;
use storage::Storage;
use tantivy::{
    schema::Field,
    termdict::TermOrdinal,
    Term,
};
use tempfile::TempDir;
use vector::{
    CompiledVectorSearch,
    QdrantSchema,
    VectorSearchQueryResult,
    VectorSearcher,
};

use super::{
    searcher::{
        Bm25Stats,
        PostingListMatch,
        PostingListQuery,
        TokenMatch,
        TokenQuery,
    },
    FragmentedTextStorageKeys,
    SearcherImpl,
    TermValue,
};
use crate::{
    Searcher,
    SegmentTermMetadataFetcher,
};

/// Dummy client that will always return no results.
#[derive(Copy, Clone)]
pub struct SearcherStub;
#[async_trait]
impl Searcher for SearcherStub {
    async fn query_tokens(
        &self,
        _search_storage: Arc<dyn Storage>,
        _storage_keys: FragmentedTextStorageKeys,
        _queries: Vec<TokenQuery>,
        _max_results: usize,
        _labels: SearchIndexMetricLabels<'_>,
    ) -> anyhow::Result<Vec<TokenMatch>> {
        Ok(vec![])
    }

    async fn query_bm25_stats(
        &self,
        _search_storage: Arc<dyn Storage>,
        _storage_keys: FragmentedTextStorageKeys,
        _terms: Vec<Term>,
        _labels: SearchIndexMetricLabels<'_>,
    ) -> anyhow::Result<Bm25Stats> {
        Ok(Bm25Stats::empty())
    }

    async fn query_posting_lists(
        &self,
        _search_storage: Arc<dyn Storage>,
        _storage_keys: FragmentedTextStorageKeys,
        _query: PostingListQuery,
        _labels: SearchIndexMetricLabels<'_>,
    ) -> anyhow::Result<Vec<PostingListMatch>> {
        Ok(vec![])
    }

    async fn execute_text_compaction(
        &self,
        _search_storage: Arc<dyn Storage>,
        _segments: Vec<FragmentedTextStorageKeys>,
        _labels: SearchIndexMetricLabels<'_>,
    ) -> anyhow::Result<FragmentedTextSegment> {
        anyhow::bail!("Not implemented");
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
        _labels: SearchIndexMetricLabels<'_>,
    ) -> anyhow::Result<Vec<VectorSearchQueryResult>> {
        Ok(vec![])
    }

    async fn execute_vector_compaction(
        &self,
        _search_storage: Arc<dyn Storage>,
        _segments: Vec<FragmentedVectorSegmentPaths>,
        _dimension: usize,
        _labels: SearchIndexMetricLabels<'_>,
    ) -> anyhow::Result<FragmentedVectorSegment> {
        anyhow::bail!("Not implemented!");
    }
}

#[async_trait]
impl SegmentTermMetadataFetcher for SearcherStub {
    async fn fetch_term_ordinals(
        &self,
        _search_storage: Arc<dyn Storage>,
        _segment: ObjectKey,
        _field_to_term_values: BTreeMap<Field, Vec<TermValue>>,
        _labels: SearchIndexMetricLabels<'_>,
    ) -> anyhow::Result<BTreeMap<Field, Vec<TermOrdinal>>> {
        unimplemented!()
    }
}

#[derive(Clone)]
pub struct InProcessSearcher<RT: Runtime> {
    searcher: Arc<SearcherImpl<RT>>,
    _tmpdir: Arc<TempDir>,
}

/// Resolve MAX_ARCHIVE_CACHE_SIZE_MIB into bytes, rejecting zero (a cache that
/// evicts every segment immediately after fetch) and sizes whose byte count
/// overflows u64.
fn archive_cache_size_bytes(cache_size_mib: u64) -> anyhow::Result<u64> {
    anyhow::ensure!(
        cache_size_mib > 0,
        "MAX_ARCHIVE_CACHE_SIZE_MIB must be greater than zero",
    );
    cache_size_mib.checked_mul(bytesize::MIB).with_context(|| {
        format!("MAX_ARCHIVE_CACHE_SIZE_MIB={cache_size_mib} overflows the u64 byte size")
    })
}

impl<RT: Runtime> InProcessSearcher<RT> {
    pub fn new(runtime: RT) -> anyhow::Result<Self> {
        let tmpdir = TempDir::new()?;
        let max_disk_cache_size =
            archive_cache_size_bytes(*super::searchlight_knobs::MAX_ARCHIVE_CACHE_SIZE_MIB)?;
        Ok(Self {
            searcher: Arc::new(SearcherImpl::new(
                tmpdir.path(),
                max_disk_cache_size,
                100,
                false,
                runtime,
            )?),
            _tmpdir: Arc::new(tmpdir),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::archive_cache_size_bytes;

    #[test]
    fn archive_cache_size_default_is_500_mib() {
        assert_eq!(
            archive_cache_size_bytes(500).unwrap(),
            bytesize::mib(500u64)
        );
    }

    #[test]
    fn archive_cache_size_rejects_zero() {
        assert!(archive_cache_size_bytes(0).is_err());
    }

    #[test]
    fn archive_cache_size_rejects_u64_overflow() {
        assert!(archive_cache_size_bytes(u64::MAX).is_err());
    }
}

#[async_trait]
impl<RT: Runtime> SegmentTermMetadataFetcher for InProcessSearcher<RT> {
    async fn fetch_term_ordinals(
        &self,
        search_storage: Arc<dyn Storage>,
        segment: ObjectKey,
        field_to_term_values: BTreeMap<Field, Vec<TermValue>>,
        labels: SearchIndexMetricLabels<'_>,
    ) -> anyhow::Result<BTreeMap<Field, Vec<TermOrdinal>>> {
        self.searcher
            .fetch_term_ordinals(search_storage, segment, field_to_term_values, labels)
            .await
    }
}

#[async_trait]
impl<RT: Runtime> Searcher for InProcessSearcher<RT> {
    async fn query_tokens(
        &self,
        search_storage: Arc<dyn Storage>,
        storage_keys: FragmentedTextStorageKeys,
        queries: Vec<TokenQuery>,
        max_results: usize,
        labels: SearchIndexMetricLabels<'_>,
    ) -> anyhow::Result<Vec<TokenMatch>> {
        self.searcher
            .query_tokens(search_storage, storage_keys, queries, max_results, labels)
            .await
    }

    async fn query_bm25_stats(
        &self,
        search_storage: Arc<dyn Storage>,
        storage_keys: FragmentedTextStorageKeys,
        terms: Vec<Term>,
        labels: SearchIndexMetricLabels<'_>,
    ) -> anyhow::Result<Bm25Stats> {
        self.searcher
            .query_bm25_stats(search_storage, storage_keys, terms, labels)
            .await
    }

    async fn query_posting_lists(
        &self,
        search_storage: Arc<dyn Storage>,
        storage_keys: FragmentedTextStorageKeys,
        query: PostingListQuery,
        labels: SearchIndexMetricLabels<'_>,
    ) -> anyhow::Result<Vec<PostingListMatch>> {
        self.searcher
            .query_posting_lists(search_storage, storage_keys, query, labels)
            .await
    }

    async fn execute_text_compaction(
        &self,
        search_storage: Arc<dyn Storage>,
        segments: Vec<FragmentedTextStorageKeys>,
        labels: SearchIndexMetricLabels<'_>,
    ) -> anyhow::Result<FragmentedTextSegment> {
        self.searcher
            .execute_text_compaction(search_storage, segments, labels)
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
        labels: SearchIndexMetricLabels<'_>,
    ) -> anyhow::Result<Vec<VectorSearchQueryResult>> {
        self.searcher
            .execute_multi_segment_vector_query(
                search_storage,
                segments,
                schema,
                search,
                overfetch_delta,
                labels,
            )
            .await
    }

    async fn execute_vector_compaction(
        &self,
        search_storage: Arc<dyn Storage>,
        segments: Vec<FragmentedVectorSegmentPaths>,
        dimension: usize,
        labels: SearchIndexMetricLabels<'_>,
    ) -> anyhow::Result<FragmentedVectorSegment> {
        self.searcher
            .execute_vector_compaction(search_storage, segments, dimension, labels)
            .await
    }
}

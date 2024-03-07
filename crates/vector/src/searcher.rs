use std::sync::Arc;

use async_trait::async_trait;
use common::bootstrap_model::index::vector_index::FragmentedVectorSegment;
use storage::Storage;

use crate::{
    qdrant_index::QdrantSchema,
    query::{
        CompiledVectorSearch,
        VectorSearchQueryResult,
    },
};

#[async_trait]
pub trait VectorSearcher: Send + Sync + 'static {
    async fn execute_multi_segment_vector_query(
        &self,
        search_storage: Arc<dyn Storage>,
        segments: Vec<pb::searchlight::FragmentedVectorSegmentPaths>,
        schema: QdrantSchema,
        search: CompiledVectorSearch,
        overfetch_delta: u32,
    ) -> anyhow::Result<Vec<VectorSearchQueryResult>>;

    async fn execute_vector_compaction(
        &self,
        search_storage: Arc<dyn Storage>,
        segments: Vec<pb::searchlight::FragmentedVectorSegmentPaths>,
        dimension: usize,
    ) -> anyhow::Result<FragmentedVectorSegment>;
}

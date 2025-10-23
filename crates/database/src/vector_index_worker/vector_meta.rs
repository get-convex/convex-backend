use std::{
    collections::BTreeMap,
    path::PathBuf,
    sync::Arc,
};

use async_trait::async_trait;
use common::{
    bootstrap_model::index::{
        vector_index::{
            FragmentedVectorSegment,
            VectorIndexBackfillState,
            VectorIndexSnapshot,
            VectorIndexSnapshotData,
            VectorIndexSpec,
            VectorIndexState,
        },
        IndexConfig,
        TabletIndexMetadata,
    },
    document::{
        ParsedDocument,
        ResolvedDocument,
    },
    runtime::{
        try_join_buffer_unordered,
        Runtime,
    },
    types::IndexId,
};
use futures::TryStreamExt;
use search::{
    disk_index::upload_vector_segment,
    fragmented_segment::{
        MutableFragmentedSegmentMetadata,
        PreviousVectorSegments,
    },
    metrics::SearchType,
    Searcher,
};
use storage::Storage;
use sync_types::Timestamp;
use vector::{
    qdrant_segments::VectorDiskSegmentValues,
    QdrantSchema,
};

use crate::{
    search_index_workers::index_meta::{
        BackfillState,
        MakeDocumentStream,
        SearchIndex,
        SearchIndexConfig,
        SearchOnDiskState,
        SearchSnapshot,
        SegmentStatistics,
        SegmentType,
        SnapshotData,
    },
    Snapshot,
};

impl From<VectorIndexState> for SearchOnDiskState<VectorSearchIndex> {
    fn from(value: VectorIndexState) -> Self {
        match value {
            VectorIndexState::Backfilling(backfill_state) => {
                SearchOnDiskState::Backfilling(backfill_state.into())
            },
            VectorIndexState::Backfilled { snapshot, staged } => SearchOnDiskState::Backfilled {
                snapshot: snapshot.into(),
                staged,
            },
            VectorIndexState::SnapshottedAt(snapshot) => {
                SearchOnDiskState::SnapshottedAt(snapshot.into())
            },
        }
    }
}

impl TryFrom<SearchOnDiskState<VectorSearchIndex>> for VectorIndexState {
    type Error = anyhow::Error;

    fn try_from(value: SearchOnDiskState<VectorSearchIndex>) -> anyhow::Result<Self> {
        Ok(match value {
            SearchOnDiskState::Backfilling(state) => Self::Backfilling(state.into()),
            SearchOnDiskState::Backfilled { snapshot, staged } => Self::Backfilled {
                snapshot: snapshot.try_into()?,
                staged,
            },
            SearchOnDiskState::SnapshottedAt(snapshot) => Self::SnapshottedAt(snapshot.try_into()?),
        })
    }
}

impl SegmentType<VectorSearchIndex> for FragmentedVectorSegment {
    fn id(&self) -> &str {
        &self.id
    }

    fn statistics(&self) -> anyhow::Result<VectorStatistics> {
        let non_deleted_vectors = self.non_deleted_vectors()?;
        Ok(VectorStatistics {
            non_deleted_vectors,
            num_vectors: self.num_vectors,
        })
    }

    fn total_size_bytes(
        &self,
        config: &<VectorSearchIndex as SearchIndex>::Spec,
    ) -> anyhow::Result<u64> {
        self.total_size_bytes(config.dimensions)
    }
}

#[derive(Clone, Debug)]
pub struct VectorSearchIndex;

#[derive(Clone)]
pub struct BuildVectorIndexArgs {
    /// The maximum vector segment size at which it's reasonable to search the
    /// segment by simply iterating over every item individually.
    ///
    /// This is only used for vector search where:
    /// 1. We want to avoid the CPU  cost of building an expensive HNSW segment
    ///    for small segments
    /// 2. It's more accurate/efficient to perform a linear scan than use HNSW
    ///    anyway.
    pub full_scan_threshold_bytes: usize,
}

#[async_trait]
impl SearchIndex for VectorSearchIndex {
    type BuildIndexArgs = BuildVectorIndexArgs;
    type NewSegment = VectorDiskSegmentValues;
    type PreviousSegments = PreviousVectorSegments;
    type Schema = QdrantSchema;
    type Segment = FragmentedVectorSegment;
    type Spec = VectorIndexSpec;
    type Statistics = VectorStatistics;

    fn get_config(config: IndexConfig) -> Option<SearchIndexConfig<Self>> {
        let IndexConfig::Vector {
            on_disk_state,
            spec,
        } = config
        else {
            return None;
        };
        Some(SearchIndexConfig {
            spec,
            on_disk_state: SearchOnDiskState::from(on_disk_state),
        })
    }

    fn get_index_sizes(snapshot: Snapshot) -> anyhow::Result<BTreeMap<IndexId, usize>> {
        Ok(snapshot
            .vector_indexes
            .backfilled_and_enabled_index_sizes()?
            .collect())
    }

    fn is_version_current(snapshot: &SearchSnapshot<Self>) -> bool {
        snapshot.data.is_version_current()
    }

    fn new_schema(config: &Self::Spec) -> Self::Schema {
        QdrantSchema::new(config)
    }

    async fn download_previous_segments(
        storage: Arc<dyn Storage>,
        segments: Vec<Self::Segment>,
    ) -> anyhow::Result<Self::PreviousSegments> {
        let segments = try_join_buffer_unordered(
            "upload_vector_metadata",
            segments.into_iter().map(move |segment| {
                MutableFragmentedSegmentMetadata::download(segment, storage.clone())
            }),
        )
        .await?;
        Ok(PreviousVectorSegments(segments))
    }

    async fn upload_previous_segments(
        storage: Arc<dyn Storage>,
        segments: Self::PreviousSegments,
    ) -> anyhow::Result<Vec<Self::Segment>> {
        try_join_buffer_unordered(
            "upload_vector_metadata",
            segments
                .0
                .into_iter()
                .map(move |segment| segment.upload_deleted_bitset(storage.clone())),
        )
        .await
    }

    fn estimate_document_size(schema: &Self::Schema, _doc: &ResolvedDocument) -> u64 {
        schema.estimate_vector_size() as u64
    }

    async fn build_disk_index(
        schema: &Self::Schema,
        index_path: &PathBuf,
        documents: MakeDocumentStream<'_>,
        previous_segments: &mut Self::PreviousSegments,
        _document_log_lower_bound: Option<Timestamp>,
        BuildVectorIndexArgs {
            full_scan_threshold_bytes,
        }: Self::BuildIndexArgs,
    ) -> anyhow::Result<Option<Self::NewSegment>> {
        schema
            .build_disk_index(
                index_path,
                documents.into_document_stream(),
                full_scan_threshold_bytes,
                previous_segments,
            )
            .await
    }

    async fn upload_new_segment<RT: Runtime>(
        rt: &RT,
        storage: Arc<dyn Storage>,
        new_segment: Self::NewSegment,
    ) -> anyhow::Result<Self::Segment> {
        upload_vector_segment(rt, storage, new_segment).await
    }

    fn extract_metadata(
        metadata: ParsedDocument<TabletIndexMetadata>,
    ) -> anyhow::Result<(Self::Spec, SearchOnDiskState<Self>)> {
        let (on_disk_state, spec) = match metadata.into_value().config {
            IndexConfig::Database { .. } | IndexConfig::Text { .. } => {
                anyhow::bail!("Index type changed!");
            },
            IndexConfig::Vector {
                on_disk_state,
                spec,
            } => (on_disk_state, spec),
        };

        Ok((spec, SearchOnDiskState::from(on_disk_state)))
    }

    fn new_index_config(
        spec: Self::Spec,
        new_state: SearchOnDiskState<Self>,
    ) -> anyhow::Result<IndexConfig> {
        let on_disk_state = VectorIndexState::try_from(new_state)?;
        Ok(IndexConfig::Vector {
            on_disk_state,
            spec,
        })
    }

    fn search_type() -> SearchType {
        SearchType::Vector
    }

    async fn execute_compaction(
        searcher: Arc<dyn Searcher>,
        search_storage: Arc<dyn Storage>,
        config: &Self::Spec,
        segments: Vec<Self::Segment>,
    ) -> anyhow::Result<Self::Segment> {
        let protos: Vec<pb::searchlight::FragmentedVectorSegmentPaths> = segments
            .into_iter()
            .map(|segment| segment.to_paths_proto())
            .collect::<anyhow::Result<Vec<_>>>()?;
        searcher
            .execute_vector_compaction(search_storage, protos, config.dimensions.into())
            .await
    }

    async fn merge_deletes(
        previous_segments: &mut Self::PreviousSegments,
        documents: MakeDocumentStream<'_>,
        _build_index_args: Self::BuildIndexArgs,
        _schema: Self::Schema,
        _document_log_lower_bound: Timestamp,
    ) -> anyhow::Result<()> {
        let mut documents = documents.into_document_stream();
        while let Some(entry) = documents.try_next().await? {
            if entry.value.is_none() {
                previous_segments.maybe_delete_convex(entry.id.internal_id())?;
            }
        }
        Ok(())
    }
}

#[derive(Debug, Default)]
pub struct VectorStatistics {
    pub num_vectors: u32,
    pub non_deleted_vectors: u64,
}

impl SegmentStatistics for VectorStatistics {
    fn add(lhs: anyhow::Result<Self>, rhs: anyhow::Result<Self>) -> anyhow::Result<Self> {
        let rhs = rhs?;
        let lhs = lhs?;
        Ok(Self {
            num_vectors: lhs.num_vectors + rhs.num_vectors,
            non_deleted_vectors: lhs.non_deleted_vectors + rhs.non_deleted_vectors,
        })
    }

    fn num_documents(&self) -> u64 {
        self.num_vectors as u64
    }

    fn num_non_deleted_documents(&self) -> u64 {
        self.non_deleted_vectors
    }
}

impl From<VectorIndexBackfillState> for BackfillState<VectorSearchIndex> {
    fn from(value: VectorIndexBackfillState) -> Self {
        Self {
            segments: value.segments,
            cursor: value.cursor,
            backfill_snapshot_ts: value.backfill_snapshot_ts,
            staged: value.staged,
            last_segment_ts: value.last_segment_ts,
        }
    }
}

impl From<BackfillState<VectorSearchIndex>> for VectorIndexBackfillState {
    fn from(value: BackfillState<VectorSearchIndex>) -> Self {
        Self {
            segments: value.segments,
            cursor: value.cursor,
            backfill_snapshot_ts: value.backfill_snapshot_ts,
            staged: value.staged,
            last_segment_ts: value.last_segment_ts,
        }
    }
}

impl From<VectorIndexSnapshot> for SearchSnapshot<VectorSearchIndex> {
    fn from(snapshot: VectorIndexSnapshot) -> Self {
        Self {
            ts: snapshot.ts,
            data: SnapshotData::from(snapshot.data),
        }
    }
}

// TODO(CX-6589): Make this infallible
impl TryFrom<SearchSnapshot<VectorSearchIndex>> for VectorIndexSnapshot {
    type Error = anyhow::Error;

    fn try_from(value: SearchSnapshot<VectorSearchIndex>) -> anyhow::Result<Self> {
        Ok(VectorIndexSnapshot {
            data: value.data.try_into()?,
            ts: value.ts,
        })
    }
}

impl From<VectorIndexSnapshotData> for SnapshotData<FragmentedVectorSegment> {
    fn from(value: VectorIndexSnapshotData) -> Self {
        match value {
            VectorIndexSnapshotData::MultiSegment(values) => SnapshotData::MultiSegment(values),
            VectorIndexSnapshotData::Unknown(obj) => SnapshotData::Unknown(obj),
        }
    }
}

// TODO(CX-6589): Make this infallible
impl TryFrom<SnapshotData<FragmentedVectorSegment>> for VectorIndexSnapshotData {
    type Error = anyhow::Error;

    fn try_from(value: SnapshotData<FragmentedVectorSegment>) -> anyhow::Result<Self> {
        Ok(match value {
            SnapshotData::Unknown(obj) => Self::Unknown(obj),
            SnapshotData::MultiSegment(data) => Self::MultiSegment(data),
        })
    }
}

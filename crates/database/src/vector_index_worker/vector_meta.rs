use std::{
    collections::BTreeMap,
    path::PathBuf,
    sync::Arc,
};

use anyhow::Context;
use async_trait::async_trait;
use common::{
    bootstrap_model::index::{
        vector_index::{
            DeveloperVectorIndexConfig,
            FragmentedVectorSegment,
            VectorIndexBackfillState,
            VectorIndexSnapshot,
            VectorIndexSnapshotData,
            VectorIndexState,
        },
        IndexConfig,
        IndexMetadata,
        TabletIndexMetadata,
    },
    document::{
        ParsedDocument,
        ResolvedDocument,
    },
    persistence::{
        DocumentStream,
        RepeatablePersistence,
    },
    runtime::{
        try_join_buffer_unordered,
        Runtime,
    },
    types::{
        IndexId,
        TabletIndexName,
    },
};
use search::{
    disk_index::upload_vector_segment,
    fragmented_segment::{
        MutableFragmentedSegmentMetadata,
        PreviousVectorSegments,
    },
};
use storage::Storage;
use sync_types::Timestamp;
use value::{
    InternalId,
    TabletId,
};
use vector::{
    qdrant_segments::VectorDiskSegmentValues,
    QdrantSchema,
};

use crate::{
    index_workers::index_meta::{
        BackfillState,
        IndexMetadataState,
        PreviousSegmentsType,
        SearchIndex,
        SearchIndexConfig,
        SearchIndexConfigParser,
        SearchOnDiskState,
        SearchSnapshot,
        SegmentStatistics,
        SegmentType,
        SnapshotData,
    },
    metrics::vector::log_documents_per_segment,
    Snapshot,
};

pub struct VectorIndexConfigParser;

impl SearchIndexConfigParser for VectorIndexConfigParser {
    type IndexType = VectorSearchIndex;

    fn get_config(config: IndexConfig) -> Option<SearchIndexConfig<Self::IndexType>> {
        let IndexConfig::Vector {
            on_disk_state,
            developer_config,
        } = config
        else {
            return None;
        };
        Some(SearchIndexConfig {
            developer_config,
            on_disk_state: match on_disk_state {
                VectorIndexState::Backfilling(backfill_state) => {
                    SearchOnDiskState::Backfilling(backfill_state.into())
                },
                VectorIndexState::Backfilled(snapshot) => {
                    SearchOnDiskState::Backfilled(snapshot.into())
                },
                VectorIndexState::SnapshottedAt(snapshot) => {
                    SearchOnDiskState::SnapshottedAt(snapshot.into())
                },
            },
        })
    }
}

impl SegmentType for FragmentedVectorSegment {
    fn id(&self) -> &str {
        &self.id
    }

    fn num_deleted(&self) -> u32 {
        self.num_deleted
    }
}

#[derive(Clone, Debug)]
pub struct VectorSearchIndex;

impl PreviousSegmentsType for PreviousVectorSegments {
    fn maybe_delete_document(&mut self, convex_id: InternalId) -> anyhow::Result<()> {
        self.maybe_delete_convex(convex_id)
    }
}

#[async_trait]
impl SearchIndex for VectorSearchIndex {
    type DeveloperConfig = DeveloperVectorIndexConfig;
    type NewSegment = VectorDiskSegmentValues;
    type PreviousSegments = PreviousVectorSegments;
    type Schema = QdrantSchema;
    type Segment = FragmentedVectorSegment;
    type Statistics = VectorStatistics;

    fn get_index_sizes(snapshot: Snapshot) -> anyhow::Result<BTreeMap<IndexId, usize>> {
        Ok(snapshot
            .vector_indexes
            .backfilled_and_enabled_index_sizes()?
            .collect())
    }

    fn is_version_current(snapshot: &SearchSnapshot<Self>) -> bool {
        snapshot.data.is_version_current()
    }

    fn new_schema(config: &Self::DeveloperConfig) -> Self::Schema {
        QdrantSchema::new(config)
    }

    async fn download_previous_segments<RT: Runtime>(
        rt: RT,
        storage: Arc<dyn Storage>,
        segments: Vec<Self::Segment>,
    ) -> anyhow::Result<Self::PreviousSegments> {
        let segments = try_join_buffer_unordered(
            rt,
            "upload_vector_metadata",
            segments.into_iter().map(move |segment| {
                MutableFragmentedSegmentMetadata::download(segment, storage.clone())
            }),
        )
        .await?;
        Ok(PreviousVectorSegments(segments))
    }

    async fn upload_previous_segments<RT: Runtime>(
        rt: RT,
        storage: Arc<dyn Storage>,
        segments: Self::PreviousSegments,
    ) -> anyhow::Result<Vec<Self::Segment>> {
        try_join_buffer_unordered(
            rt,
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
        documents: DocumentStream<'_>,
        _reader: RepeatablePersistence,
        full_scan_threshold_bytes: usize,
        previous_segments: &mut Self::PreviousSegments,
    ) -> anyhow::Result<Option<Self::NewSegment>> {
        schema
            .build_disk_index(
                index_path,
                documents,
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

    fn segment_id(segment: &Self::Segment) -> String {
        segment.id.clone()
    }

    fn statistics(segment: &Self::Segment) -> anyhow::Result<Self::Statistics> {
        let non_deleted_vectors = segment.non_deleted_vectors()?;
        Ok(VectorStatistics {
            non_deleted_vectors,
            num_vectors: segment.num_vectors,
        })
    }

    fn extract_flush_metadata(
        metadata: ParsedDocument<TabletIndexMetadata>,
    ) -> anyhow::Result<(
        Option<Timestamp>,
        Vec<Self::Segment>,
        bool,
        Self::DeveloperConfig,
    )> {
        let (on_disk_state, developer_config) = match metadata.into_value().config {
            IndexConfig::Database { .. } | IndexConfig::Search { .. } => {
                anyhow::bail!("Index type changed!");
            },
            IndexConfig::Vector {
                on_disk_state,
                developer_config,
            } => (on_disk_state, developer_config),
        };

        let is_snapshotted = match on_disk_state {
            VectorIndexState::Backfilling(_) | VectorIndexState::Backfilled(_) => false,
            VectorIndexState::SnapshottedAt(_) => true,
        };
        let (snapshot_ts, current_segments) = match on_disk_state {
            VectorIndexState::Backfilling(state) => (state.backfill_snapshot_ts, state.segments),
            VectorIndexState::Backfilled(snapshot) | VectorIndexState::SnapshottedAt(snapshot) => {
                let current_segments = match snapshot.data {
                    VectorIndexSnapshotData::Unknown(_) => {
                        // We might be migrating to the multi segment format, so we have to be
                        // lenient.
                        vec![]
                    },
                    VectorIndexSnapshotData::MultiSegment(segments) => segments,
                };

                (Some(snapshot.ts), current_segments)
            },
        };

        Ok((
            snapshot_ts,
            current_segments,
            is_snapshotted,
            developer_config,
        ))
    }

    fn extract_compaction_metadata(
        metadata: &mut ParsedDocument<TabletIndexMetadata>,
    ) -> anyhow::Result<(&Timestamp, &mut Vec<Self::Segment>)> {
        let on_disk_state = match &mut metadata.config {
            IndexConfig::Database { .. } | IndexConfig::Search { .. } => {
                anyhow::bail!("Index type changed!");
            },
            IndexConfig::Vector {
                ref mut on_disk_state,
                ..
            } => on_disk_state,
        };
        let (segments, snapshot_ts) = match on_disk_state {
            VectorIndexState::Backfilling(VectorIndexBackfillState {
                segments,
                backfill_snapshot_ts,
                ..
            }) => (
                segments,
                backfill_snapshot_ts.as_ref().context(
                    "cannot compact backfilling index without a backfill snapshot set yet",
                )?,
            ),
            VectorIndexState::Backfilled(snapshot) | VectorIndexState::SnapshottedAt(snapshot) => {
                let current_segments = match snapshot.data {
                    VectorIndexSnapshotData::Unknown(_) => {
                        anyhow::bail!("Index version changed!")
                    },
                    VectorIndexSnapshotData::MultiSegment(ref mut segments) => segments,
                };
                (current_segments, &snapshot.ts)
            },
        };
        Ok((snapshot_ts, segments))
    }

    fn new_metadata(
        name: TabletIndexName,
        developer_config: Self::DeveloperConfig,
        new_and_modified_segments: Vec<Self::Segment>,
        new_ts: Timestamp,
        new_state: IndexMetadataState,
    ) -> IndexMetadata<TabletId> {
        let new_on_disk_state = match new_state {
            IndexMetadataState::SnapshottedAt => {
                let snapshot = VectorIndexSnapshot {
                    data: VectorIndexSnapshotData::MultiSegment(new_and_modified_segments),
                    ts: new_ts,
                };
                VectorIndexState::SnapshottedAt(snapshot)
            },
            IndexMetadataState::Backfilled => {
                let snapshot = VectorIndexSnapshot {
                    data: VectorIndexSnapshotData::MultiSegment(new_and_modified_segments),
                    ts: new_ts,
                };
                VectorIndexState::Backfilled(snapshot)
            },
            IndexMetadataState::Backfilling(cursor) => {
                VectorIndexState::Backfilling(VectorIndexBackfillState {
                    segments: new_and_modified_segments,
                    cursor,
                    backfill_snapshot_ts: Some(new_ts),
                })
            },
        };
        IndexMetadata {
            name,
            config: IndexConfig::Vector {
                on_disk_state: new_on_disk_state,
                developer_config: developer_config.clone(),
            },
        }
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

    fn log(&self) {
        log_documents_per_segment(self.non_deleted_vectors);
    }
}

impl From<VectorIndexBackfillState> for BackfillState<VectorSearchIndex> {
    fn from(value: VectorIndexBackfillState) -> Self {
        Self {
            segments: value.segments,
            cursor: value.cursor,
            backfill_snapshot_ts: value.backfill_snapshot_ts,
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

impl From<VectorIndexSnapshotData> for SnapshotData<FragmentedVectorSegment> {
    fn from(value: VectorIndexSnapshotData) -> Self {
        match value {
            VectorIndexSnapshotData::MultiSegment(values) => SnapshotData::MultiSegment(values),
            VectorIndexSnapshotData::Unknown(_) => SnapshotData::Unknown,
        }
    }
}

use std::{
    collections::BTreeMap,
    path::PathBuf,
    sync::Arc,
};

use async_trait::async_trait;
use common::{
    bootstrap_model::index::{
        search_index::{
            DeveloperSearchIndexConfig,
            FragmentedSearchSegment,
            SearchIndexSnapshot,
            SearchIndexState,
            TextIndexBackfillState,
        },
        vector_index::{
            DeveloperVectorIndexConfig,
            FragmentedVectorSegment,
            VectorIndexBackfillState,
            VectorIndexSnapshot,
            VectorIndexSnapshotData,
            VectorIndexState,
        },
        IndexConfig,
    },
    document::ResolvedDocument,
    persistence::DocumentStream,
    runtime::Runtime,
    types::IndexId,
};
use futures::{
    stream::FuturesUnordered,
    TryStreamExt,
};
use search::{
    disk_index::upload_segment,
    fragmented_segment::MutableFragmentedSegmentMetadata,
};
use storage::Storage;
use sync_types::Timestamp;
use value::InternalId;
use vector::{
    qdrant_segments::DiskSegmentValues,
    QdrantSchema,
};

use crate::{
    metrics::vector::log_documents_per_segment,
    Snapshot,
};

pub trait SearchIndexConfigParser {
    type IndexType: SearchIndex;

    /// Returns the generalized `SearchIndexConfig` if it matches the type of
    /// the parser (e.g. Text vs Vector) and `None` otherwise.
    fn get_config(config: IndexConfig) -> Option<SearchIndexConfig<Self::IndexType>>;
}

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

pub struct TextIndexConfigParser;

impl SearchIndexConfigParser for TextIndexConfigParser {
    type IndexType = TextSearchIndex;

    fn get_config(config: IndexConfig) -> Option<SearchIndexConfig<Self::IndexType>> {
        let IndexConfig::Search {
            on_disk_state,
            developer_config,
        } = config
        else {
            return None;
        };
        Some(SearchIndexConfig {
            developer_config,
            on_disk_state: match on_disk_state {
                SearchIndexState::Backfilling(snapshot) => {
                    SearchOnDiskState::Backfilling(snapshot.into())
                },
                SearchIndexState::Backfilled(snapshot) => {
                    SearchOnDiskState::Backfilled(snapshot.into())
                },
                SearchIndexState::SnapshottedAt(snapshot) => {
                    SearchOnDiskState::SnapshottedAt(snapshot.into())
                },
            },
        })
    }
}

#[async_trait]
pub trait SearchIndex {
    type DeveloperConfig: Clone + Send;
    type Segment: Clone + Send + 'static;
    type NewSegment: Send;

    type PreviousSegments;

    type Statistics: SegmentStatistics;

    type Schema: Send + Sync;

    fn statistics(segment: &Self::Segment) -> anyhow::Result<Self::Statistics>;

    async fn upload_new_segment<RT: Runtime>(
        rt: &RT,
        storage: Arc<dyn Storage>,
        new_segment: Self::NewSegment,
    ) -> anyhow::Result<Self::Segment>;

    fn segment_id(segment: &Self::Segment) -> String;

    fn estimate_document_size(schema: &Self::Schema, doc: &ResolvedDocument) -> u64;

    // TODO(sam): Remove full_scan_threshold_bytes, this is vector specific.
    async fn build_disk_index(
        schema: &Self::Schema,
        index_path: &PathBuf,
        documents: DocumentStream<'_>,
        full_scan_threshold_bytes: usize,
        previous_segments: &mut Self::PreviousSegments,
    ) -> anyhow::Result<Option<Self::NewSegment>>;

    fn new_schema(config: &Self::DeveloperConfig) -> Self::Schema;

    fn get_index_sizes(snapshot: Snapshot) -> anyhow::Result<BTreeMap<IndexId, usize>>;

    fn is_version_current(data: &SearchSnapshot<Self>) -> bool
    where
        Self: Sized;

    async fn download_previous_segments(
        storage: Arc<dyn Storage>,
        segment: Vec<Self::Segment>,
    ) -> anyhow::Result<Self::PreviousSegments>;

    async fn upload_previous_segments(
        storage: Arc<dyn Storage>,
        segments: Self::PreviousSegments,
    ) -> anyhow::Result<Vec<Self::Segment>>;
}

pub trait SegmentStatistics: Default {
    fn add(lhs: anyhow::Result<Self>, rhs: anyhow::Result<Self>) -> anyhow::Result<Self>;
    fn log(&self);
}

pub struct TextSearchIndex;
#[async_trait]
impl SearchIndex for TextSearchIndex {
    type DeveloperConfig = DeveloperSearchIndexConfig;
    type NewSegment = ();
    type PreviousSegments = ();
    type Schema = ();
    type Segment = FragmentedSearchSegment;
    type Statistics = TextStatistics;

    fn get_index_sizes(snapshot: Snapshot) -> anyhow::Result<BTreeMap<IndexId, usize>> {
        Ok(snapshot
            .search_indexes
            .backfilled_and_enabled_index_sizes()?
            .collect())
    }

    fn is_version_current(snapshot: &SearchSnapshot<Self>) -> bool {
        // TODO(sam): This doesn't match the current persistence version based check,
        // but it's closer to what vector search does.
        snapshot.data.is_version_current()
    }

    fn new_schema(_config: &Self::DeveloperConfig) -> Self::Schema {}

    async fn download_previous_segments(
        _storage: Arc<dyn Storage>,
        _segment: Vec<Self::Segment>,
    ) -> anyhow::Result<Self::PreviousSegments> {
        anyhow::bail!("Not implemented");
    }

    async fn upload_previous_segments(
        _storage: Arc<dyn Storage>,
        _segments: Self::PreviousSegments,
    ) -> anyhow::Result<Vec<Self::Segment>> {
        anyhow::bail!("Not implemented");
    }

    fn estimate_document_size(_schema: &Self::Schema, _doc: &ResolvedDocument) -> u64 {
        0
    }

    async fn build_disk_index(
        _schema: &Self::Schema,
        _index_path: &PathBuf,
        _documents: DocumentStream<'_>,
        _full_scan_threshold_bytes: usize,
        _previous_segments: &mut Self::PreviousSegments,
    ) -> anyhow::Result<Option<Self::NewSegment>> {
        anyhow::bail!("Not implemented");
    }

    async fn upload_new_segment<RT: Runtime>(
        _rt: &RT,
        _storage: Arc<dyn Storage>,
        _new_segment: Self::NewSegment,
    ) -> anyhow::Result<Self::Segment> {
        anyhow::bail!("Not implemented")
    }

    fn segment_id(_segment: &Self::Segment) -> String {
        "".to_string()
    }

    fn statistics(_segment: &Self::Segment) -> anyhow::Result<Self::Statistics> {
        Ok(TextStatistics)
    }
}

#[derive(Default)]
pub struct TextStatistics;

impl SegmentStatistics for TextStatistics {
    fn add(_: anyhow::Result<Self>, _: anyhow::Result<Self>) -> anyhow::Result<Self> {
        Ok(Self)
    }

    fn log(&self) {}
}

#[derive(Debug)]
pub struct VectorSearchIndex;

#[async_trait]
impl SearchIndex for VectorSearchIndex {
    type DeveloperConfig = DeveloperVectorIndexConfig;
    type NewSegment = DiskSegmentValues;
    type PreviousSegments = Vec<MutableFragmentedSegmentMetadata>;
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

    async fn download_previous_segments(
        storage: Arc<dyn Storage>,
        segments: Vec<Self::Segment>,
    ) -> anyhow::Result<Self::PreviousSegments> {
        segments
            .into_iter()
            .map(|segment| MutableFragmentedSegmentMetadata::download(segment, storage.clone()))
            .collect::<FuturesUnordered<_>>()
            .try_collect::<Vec<_>>()
            .await
    }

    async fn upload_previous_segments(
        storage: Arc<dyn Storage>,
        segments: Self::PreviousSegments,
    ) -> anyhow::Result<Vec<Self::Segment>> {
        segments
            .into_iter()
            .map(|segment| segment.upload_deleted_bitset(storage.clone()))
            .collect::<FuturesUnordered<_>>()
            .try_collect::<Vec<_>>()
            .await
    }

    fn estimate_document_size(schema: &Self::Schema, _doc: &ResolvedDocument) -> u64 {
        schema.estimate_vector_size() as u64
    }

    async fn build_disk_index(
        schema: &Self::Schema,
        index_path: &PathBuf,
        documents: DocumentStream<'_>,
        full_scan_threshold_bytes: usize,
        previous_segments: &mut Self::PreviousSegments,
    ) -> anyhow::Result<Option<Self::NewSegment>> {
        schema
            .build_disk_index(
                index_path,
                documents,
                full_scan_threshold_bytes,
                &mut previous_segments.iter_mut().collect::<Vec<_>>(),
            )
            .await
    }

    async fn upload_new_segment<RT: Runtime>(
        rt: &RT,
        storage: Arc<dyn Storage>,
        new_segment: Self::NewSegment,
    ) -> anyhow::Result<Self::Segment> {
        upload_segment(rt, storage, new_segment).await
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
}
pub struct SearchIndexConfig<T: SearchIndex> {
    pub developer_config: T::DeveloperConfig,
    pub on_disk_state: SearchOnDiskState<T>,
}

pub struct SearchSnapshot<T: SearchIndex> {
    pub ts: Timestamp,
    pub data: SnapshotData<T::Segment>,
}

pub struct BackfillState<T: SearchIndex> {
    pub segments: Vec<T::Segment>,
    pub cursor: Option<InternalId>,
    pub backfill_snapshot_ts: Option<Timestamp>,
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

impl From<TextIndexBackfillState> for BackfillState<TextSearchIndex> {
    fn from(value: TextIndexBackfillState) -> Self {
        Self {
            segments: value.segments,
            cursor: value.cursor.clone().map(|value| value.cursor),
            backfill_snapshot_ts: value.cursor.map(|value| value.backfill_snapshot_ts),
        }
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

pub enum SearchOnDiskState<T: SearchIndex> {
    Backfilling(BackfillState<T>),
    Backfilled(SearchSnapshot<T>),
    SnapshottedAt(SearchSnapshot<T>),
}

impl From<VectorIndexSnapshot> for SearchSnapshot<VectorSearchIndex> {
    fn from(snapshot: VectorIndexSnapshot) -> Self {
        Self {
            ts: snapshot.ts,
            data: SnapshotData::from(snapshot.data),
        }
    }
}

impl From<SearchIndexSnapshot> for SearchSnapshot<TextSearchIndex> {
    fn from(snapshot: SearchIndexSnapshot) -> Self {
        Self {
            ts: snapshot.ts,
            // TODO(sam): Implement this.
            data: SnapshotData::Unknown,
        }
    }
}

#[derive(Debug)]
pub enum SnapshotData<T> {
    Unknown,
    MultiSegment(Vec<T>),
}

impl<T> SnapshotData<T> {
    fn is_version_current(&self) -> bool {
        matches!(self, Self::MultiSegment(_))
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

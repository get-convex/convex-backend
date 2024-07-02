use std::{
    collections::BTreeMap,
    fmt::Debug,
    path::PathBuf,
    sync::Arc,
};

use async_trait::async_trait;
use common::{
    bootstrap_model::index::{
        IndexConfig,
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
    query::Order,
    runtime::Runtime,
    types::IndexId,
};
use search::{
    metrics::SearchType,
    Searcher,
};
use storage::Storage;
use sync_types::Timestamp;
use value::{
    ConvexObject,
    InternalId,
};

use super::search_flusher::MultipartBuildType;
use crate::Snapshot;

pub trait SegmentType<T: SearchIndex> {
    fn id(&self) -> &str;

    fn statistics(&self) -> anyhow::Result<T::Statistics>;

    fn total_size_bytes(&self, config: &T::DeveloperConfig) -> anyhow::Result<u64>;
}

#[async_trait]
pub trait SearchIndex: Clone + Debug {
    type DeveloperConfig: Clone + Send;
    type Segment: SegmentType<Self> + Clone + Debug + Send + 'static;
    type NewSegment: Send;

    type PreviousSegments: Send;

    type Statistics: SegmentStatistics;

    type BuildIndexArgs: Clone + Send + 'static;

    type Schema: Send + Sync + 'static;

    /// Returns the generalized `SearchIndexConfig` if it matches the type of
    /// the parser (e.g. Text vs Vector) and `None` otherwise.
    fn get_config(_config: IndexConfig) -> Option<SearchIndexConfig<Self>>;

    // TODO(CX-6589): Make this infallible
    fn new_index_config(
        developer_config: Self::DeveloperConfig,
        new_state: SearchOnDiskState<Self>,
    ) -> anyhow::Result<IndexConfig>;

    fn extract_metadata(
        metadata: ParsedDocument<TabletIndexMetadata>,
    ) -> anyhow::Result<(Self::DeveloperConfig, SearchOnDiskState<Self>)>;

    /// Determines the order in which we walk the document log when constructing
    /// partial segments that main contain deletes.
    ///
    /// This does NOT impact the order in which we read documents when
    /// performing an initial backfill by walking the table contents using
    /// table iterator. However, in that case we're guaranteed never to
    /// encounter a deleted document.
    fn partial_document_order() -> Order {
        Order::Asc
    }

    fn search_type() -> SearchType;

    async fn upload_new_segment<RT: Runtime>(
        rt: &RT,
        storage: Arc<dyn Storage>,
        new_segment: Self::NewSegment,
    ) -> anyhow::Result<Self::Segment>;

    fn estimate_document_size(schema: &Self::Schema, doc: &ResolvedDocument) -> u64;

    async fn build_disk_index<RT: Runtime>(
        rt: &RT,
        schema: &Self::Schema,
        index_path: &PathBuf,
        documents: DocumentStream<'_>,
        reader: RepeatablePersistence,
        previous_segments: &mut Self::PreviousSegments,
        document_log_lower_bound: Option<Timestamp>,
        build_index_args: Self::BuildIndexArgs,
        multipart_build_type: MultipartBuildType,
    ) -> anyhow::Result<Option<Self::NewSegment>>;

    fn new_schema(config: &Self::DeveloperConfig) -> Self::Schema;

    fn get_index_sizes<RT: Runtime>(
        snapshot: Snapshot<RT>,
    ) -> anyhow::Result<BTreeMap<IndexId, usize>>;

    fn is_version_current(data: &SearchSnapshot<Self>) -> bool
    where
        Self: Sized;

    async fn download_previous_segments<RT: Runtime>(
        rt: &RT,
        storage: Arc<dyn Storage>,
        segment: Vec<Self::Segment>,
    ) -> anyhow::Result<Self::PreviousSegments>;

    async fn upload_previous_segments<RT: Runtime>(
        rt: &RT,
        storage: Arc<dyn Storage>,
        segments: Self::PreviousSegments,
    ) -> anyhow::Result<Vec<Self::Segment>>;

    async fn execute_compaction(
        searcher: Arc<dyn Searcher>,
        search_storage: Arc<dyn Storage>,
        config: &Self::DeveloperConfig,
        segments: Vec<Self::Segment>,
    ) -> anyhow::Result<Self::Segment>;

    async fn merge_deletes<RT: Runtime>(
        runtime: &RT,
        previous_segments: &mut Self::PreviousSegments,
        document_stream: DocumentStream<'_>,
        repeatable_persistence: &RepeatablePersistence,
        build_index_args: Self::BuildIndexArgs,
        schema: Self::Schema,
        document_log_lower_bound: Timestamp,
    ) -> anyhow::Result<()>;
}

pub trait SegmentStatistics: Default + Debug {
    fn add(lhs: anyhow::Result<Self>, rhs: anyhow::Result<Self>) -> anyhow::Result<Self>;

    fn num_documents(&self) -> u64;

    fn num_non_deleted_documents(&self) -> u64;

    fn num_deleted_documents(&self) -> u64 {
        self.num_documents() - self.num_non_deleted_documents()
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

pub enum SearchOnDiskState<T: SearchIndex> {
    Backfilling(BackfillState<T>),
    Backfilled(SearchSnapshot<T>),
    SnapshottedAt(SearchSnapshot<T>),
}

impl<T: SearchIndex> SearchOnDiskState<T> {
    pub fn segments(&self) -> Vec<T::Segment> {
        match self {
            SearchOnDiskState::Backfilling(ref backfill_state) => backfill_state.segments.clone(),
            SearchOnDiskState::Backfilled(ref snapshot)
            | SearchOnDiskState::SnapshottedAt(ref snapshot) => snapshot.data.clone().segments(),
        }
    }

    pub fn ts(&self) -> Option<&Timestamp> {
        match self {
            SearchOnDiskState::Backfilling(ref backfill_state) => {
                backfill_state.backfill_snapshot_ts.as_ref()
            },
            SearchOnDiskState::Backfilled(ref snapshot)
            | SearchOnDiskState::SnapshottedAt(ref snapshot) => Some(&snapshot.ts),
        }
    }

    pub fn with_updated_snapshot(
        self,
        ts: Timestamp,
        segments: Vec<T::Segment>,
    ) -> anyhow::Result<Self> {
        let snapshot = SearchSnapshot {
            ts,
            data: SnapshotData::MultiSegment(segments),
        };
        match self {
            Self::Backfilling(_) => anyhow::bail!("Can't update backfilling index!"),
            Self::Backfilled(_) => Ok(Self::Backfilled(snapshot)),
            Self::SnapshottedAt(_) => Ok(Self::SnapshottedAt(snapshot)),
        }
    }

    pub fn with_updated_segments(self, segments: Vec<T::Segment>) -> anyhow::Result<Self> {
        match self {
            Self::Backfilling(backfill) => Ok(Self::Backfilling(BackfillState {
                segments,
                ..backfill
            })),
            Self::Backfilled(snapshot) => Ok(Self::Backfilled(SearchSnapshot {
                data: SnapshotData::MultiSegment(segments),
                ..snapshot
            })),
            Self::SnapshottedAt(snapshot) => Ok(Self::SnapshottedAt(SearchSnapshot {
                data: SnapshotData::MultiSegment(segments),
                ..snapshot
            })),
        }
    }
}

#[derive(Clone, Debug)]
pub enum SnapshotData<T> {
    /// An unrecognized snapshot, probably from a newer version of backend than
    /// this one that we subsequently rolled back.
    Unknown(ConvexObject),
    MultiSegment(Vec<T>),
}

impl<T> SnapshotData<T> {
    pub fn segments(self) -> Vec<T> {
        match self {
            Self::Unknown(_) => vec![],
            Self::MultiSegment(segments) => segments,
        }
    }

    pub fn require_multi_segment(self) -> anyhow::Result<Vec<T>> {
        let Self::MultiSegment(segments) = self else {
            anyhow::bail!("Not a multi segment type!");
        };
        Ok(segments)
    }
}

impl<T> SnapshotData<T> {
    pub fn is_version_current(&self) -> bool {
        matches!(self, Self::MultiSegment(_))
    }
}

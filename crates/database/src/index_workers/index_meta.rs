use std::{
    collections::BTreeMap,
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
    types::{
        IndexId,
        ObjectKey,
    },
};
use storage::Storage;
use sync_types::Timestamp;
use value::{
    ConvexObject,
    InternalId,
};

use crate::Snapshot;

pub trait SearchIndexConfigParser {
    type IndexType: SearchIndex;

    /// Returns the generalized `SearchIndexConfig` if it matches the type of
    /// the parser (e.g. Text vs Vector) and `None` otherwise.
    fn get_config(config: IndexConfig) -> Option<SearchIndexConfig<Self::IndexType>>;
}

pub trait PreviousSegmentsType: Send {
    fn maybe_delete_document(&mut self, convex_id: InternalId) -> anyhow::Result<()>;
}
pub trait SegmentType {
    fn id(&self) -> &str;

    fn num_deleted(&self) -> u32;
}

#[async_trait]
pub trait SearchIndex: Clone {
    type DeveloperConfig: Clone + Send;
    type Segment: SegmentType + Clone + Send + 'static;
    type NewSegment: Send;

    type PreviousSegments: PreviousSegmentsType;

    type Statistics: SegmentStatistics;

    type BuildIndexArgs: Send;

    type Schema: Send + Sync;

    // TODO(CX-6589): Make this infallible
    fn new_index_config(
        developer_config: Self::DeveloperConfig,
        new_state: SearchOnDiskState<Self>,
    ) -> anyhow::Result<IndexConfig>;

    fn extract_metadata(
        metadata: ParsedDocument<TabletIndexMetadata>,
    ) -> anyhow::Result<(Self::DeveloperConfig, SearchOnDiskState<Self>)>;

    fn statistics(segment: &Self::Segment) -> anyhow::Result<Self::Statistics>;

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

    async fn upload_new_segment<RT: Runtime>(
        rt: &RT,
        storage: Arc<dyn Storage>,
        new_segment: Self::NewSegment,
    ) -> anyhow::Result<Self::Segment>;

    fn segment_id(segment: &Self::Segment) -> String;

    fn estimate_document_size(schema: &Self::Schema, doc: &ResolvedDocument) -> u64;

    async fn build_disk_index(
        schema: &Self::Schema,
        index_path: &PathBuf,
        documents: DocumentStream<'_>,
        reader: RepeatablePersistence,
        previous_segments: &mut Self::PreviousSegments,
        build_index_args: Self::BuildIndexArgs,
    ) -> anyhow::Result<Option<Self::NewSegment>>;

    fn new_schema(config: &Self::DeveloperConfig) -> Self::Schema;

    fn get_index_sizes(snapshot: Snapshot) -> anyhow::Result<BTreeMap<IndexId, usize>>;

    fn is_version_current(data: &SearchSnapshot<Self>) -> bool
    where
        Self: Sized;

    async fn download_previous_segments<RT: Runtime>(
        rt: RT,
        storage: Arc<dyn Storage>,
        segment: Vec<Self::Segment>,
    ) -> anyhow::Result<Self::PreviousSegments>;

    async fn upload_previous_segments<RT: Runtime>(
        rt: RT,
        storage: Arc<dyn Storage>,
        segments: Self::PreviousSegments,
    ) -> anyhow::Result<Vec<Self::Segment>>;
}

pub trait SegmentStatistics: Default {
    fn add(lhs: anyhow::Result<Self>, rhs: anyhow::Result<Self>) -> anyhow::Result<Self>;
    fn log(&self);
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
    SingleSegment(ObjectKey),
    MultiSegment(Vec<T>),
}

impl<T> SnapshotData<T> {
    pub fn segments(self) -> Vec<T> {
        match self {
            SnapshotData::Unknown(_) | SnapshotData::SingleSegment(_) => vec![],
            SnapshotData::MultiSegment(segments) => segments,
        }
    }
}

impl<T> SnapshotData<T> {
    pub fn is_version_current(&self) -> bool {
        matches!(self, Self::MultiSegment(_))
    }
}

use std::{
    collections::BTreeMap,
    path::PathBuf,
    sync::Arc,
};

use async_trait::async_trait;
use common::{
    bootstrap_model::index::IndexConfig,
    document::ResolvedDocument,
    persistence::{
        DocumentStream,
        RepeatablePersistence,
    },
    query::Order,
    runtime::Runtime,
    types::IndexId,
};
use storage::Storage;
use sync_types::Timestamp;
use value::InternalId;

use crate::Snapshot;

pub trait SearchIndexConfigParser {
    type IndexType: SearchIndex;

    /// Returns the generalized `SearchIndexConfig` if it matches the type of
    /// the parser (e.g. Text vs Vector) and `None` otherwise.
    fn get_config(config: IndexConfig) -> Option<SearchIndexConfig<Self::IndexType>>;
}

pub trait PreviousSegmentsType {
    fn maybe_delete_document(&mut self, convex_id: InternalId) -> anyhow::Result<()>;
}

#[async_trait]
pub trait SearchIndex {
    type DeveloperConfig: Clone + Send;
    type Segment: Clone + Send + 'static;
    type NewSegment: Send;

    type PreviousSegments: PreviousSegmentsType;

    type Statistics: SegmentStatistics;

    type Schema: Send + Sync;

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
        large_segment_threshold_bytes: usize,
        previous_segments: &mut Self::PreviousSegments,
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

#[derive(Debug)]
pub enum SnapshotData<T> {
    /// An unrecognized snapshot, probably from a newer version of backend than
    /// this one that we subsequently rolled back.
    Unknown,
    MultiSegment(Vec<T>),
}

impl<T> SnapshotData<T> {
    pub fn is_version_current(&self) -> bool {
        matches!(self, Self::MultiSegment(_))
    }
}

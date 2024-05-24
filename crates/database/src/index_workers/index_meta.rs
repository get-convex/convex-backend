use std::{
    collections::BTreeMap,
    path::PathBuf,
    sync::Arc,
};

use async_trait::async_trait;
use common::{
    bootstrap_model::index::{
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
    query::Order,
    runtime::Runtime,
    types::{
        IndexId,
        TabletIndexName,
    },
};
use storage::Storage;
use sync_types::Timestamp;
use value::{
    InternalId,
    TabletId,
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

    /// Parse metadata from the IndexMetadata required to complete a compaction.
    ///
    /// In contrast to `extract_flush_metadata` below, this method requires that
    /// a timestamp be present in the metadata. This is safe for any
    /// compaction because a timestamp is always present if at least one
    /// segment was ever built. Compaction never runs on empty indexes, so
    /// at least one segment and therefore a timestamp must be available.
    ///
    /// The returned metadata will be mutated and written back to disk.
    fn extract_compaction_metadata(
        metadata: &mut ParsedDocument<TabletIndexMetadata>,
    ) -> anyhow::Result<(&Timestamp, &mut Vec<Self::Segment>)>;

    /// Parse metadata from the IndexMetadata required to complete a flush.
    ///
    /// In contrast to `extract_compaction_metadata`, this method will not
    /// mutate the results but will instead write a new copy of the metadata
    /// to disk. Flushes require a bit more information than compaction.
    ///
    /// A timestamp is required here because this may be the first flush for an
    /// index, in which case a timestamp will not be present.
    fn extract_flush_metadata(
        metadata: ParsedDocument<TabletIndexMetadata>,
    ) -> anyhow::Result<(
        Option<Timestamp>,
        Vec<Self::Segment>,
        bool, // True if the index is snapshotted, false if it's backfilled or backfilling.
        Self::DeveloperConfig,
    )>;

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

    fn new_metadata(
        name: TabletIndexName,
        developer_config: Self::DeveloperConfig,
        new_and_modified_segments: Vec<Self::Segment>,
        new_ts: Timestamp,
        new_state: IndexMetadataState,
    ) -> IndexMetadata<TabletId>;

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

pub enum IndexMetadataState {
    SnapshottedAt,
    Backfilled,
    Backfilling(Option<InternalId>),
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

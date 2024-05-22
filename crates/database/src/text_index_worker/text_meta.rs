use std::{
    collections::BTreeMap,
    path::PathBuf,
    sync::Arc,
};

use async_trait::async_trait;
use common::{
    bootstrap_model::index::{
        text_index::{
            DeveloperSearchIndexConfig,
            FragmentedTextSegment,
            TextIndexBackfillState,
            TextIndexSnapshot,
            TextIndexSnapshotData,
            TextIndexState,
        },
        IndexConfig,
    },
    document::ResolvedDocument,
    persistence::{
        DocumentStream,
        RepeatablePersistence,
    },
    persistence_helpers::stream_revision_pairs,
    query::Order,
    runtime::{
        try_join_buffer_unordered,
        Runtime,
    },
    types::IndexId,
};
use search::{
    build_new_segment,
    disk_index::upload_text_segment,
    NewTextSegment,
    PreviousTextSegments,
    TantivySearchIndexSchema,
    UpdatableTextSegment,
};
use storage::Storage;

use crate::{
    index_workers::index_meta::{
        BackfillState,
        SearchIndex,
        SearchIndexConfig,
        SearchIndexConfigParser,
        SearchOnDiskState,
        SearchSnapshot,
        SegmentStatistics,
        SnapshotData,
    },
    Snapshot,
};

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
                TextIndexState::Backfilling(snapshot) => {
                    SearchOnDiskState::Backfilling(snapshot.into())
                },
                TextIndexState::Backfilled(snapshot) => {
                    SearchOnDiskState::Backfilled(snapshot.into())
                },
                TextIndexState::SnapshottedAt(snapshot) => {
                    SearchOnDiskState::SnapshottedAt(snapshot.into())
                },
            },
        })
    }
}

#[derive(Debug)]
pub struct TextSearchIndex;
#[async_trait]
impl SearchIndex for TextSearchIndex {
    type DeveloperConfig = DeveloperSearchIndexConfig;
    type NewSegment = NewTextSegment;
    type PreviousSegments = PreviousTextSegments;
    type Schema = TantivySearchIndexSchema;
    type Segment = FragmentedTextSegment;
    type Statistics = TextStatistics;

    // When iterating over the document log for partial segments, we must iterate in
    // reverse timestamp order to match assumptions made in build_disk_index
    // that allow for greater efficiency.
    fn partial_document_order() -> Order {
        Order::Desc
    }

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

    fn new_schema(config: &Self::DeveloperConfig) -> Self::Schema {
        TantivySearchIndexSchema::new(config)
    }

    async fn download_previous_segments<RT: Runtime>(
        rt: RT,
        storage: Arc<dyn Storage>,
        segments: Vec<Self::Segment>,
    ) -> anyhow::Result<Self::PreviousSegments> {
        Ok(PreviousTextSegments(
            try_join_buffer_unordered(
                rt,
                "download_text_meta",
                segments
                    .into_iter()
                    .map(move |segment| UpdatableTextSegment::download(segment, storage.clone())),
            )
            .await?,
        ))
    }

    async fn upload_previous_segments<RT: Runtime>(
        rt: RT,
        storage: Arc<dyn Storage>,
        segments: Self::PreviousSegments,
    ) -> anyhow::Result<Vec<Self::Segment>> {
        try_join_buffer_unordered(
            rt,
            "upload_text_metadata",
            segments
                .0
                .into_iter()
                .map(move |segment| segment.upload_metadata(storage.clone())),
        )
        .await
    }

    fn estimate_document_size(schema: &Self::Schema, doc: &ResolvedDocument) -> u64 {
        schema.estimate_size(doc)
    }

    async fn build_disk_index(
        schema: &Self::Schema,
        index_path: &PathBuf,
        documents: DocumentStream<'_>,
        reader: RepeatablePersistence,
        _large_segment_threshold_bytes: usize,
        previous_segments: &mut Self::PreviousSegments,
    ) -> anyhow::Result<Option<Self::NewSegment>> {
        let revision_stream = Box::pin(stream_revision_pairs(documents, &reader));
        build_new_segment(
            revision_stream,
            schema.clone(),
            index_path,
            previous_segments,
        )
        .await
    }

    async fn upload_new_segment<RT: Runtime>(
        rt: &RT,
        storage: Arc<dyn Storage>,
        new_segment: Self::NewSegment,
    ) -> anyhow::Result<Self::Segment> {
        upload_text_segment(rt, storage, new_segment).await
    }

    fn segment_id(segment: &Self::Segment) -> String {
        segment.id.clone()
    }

    fn statistics(segment: &Self::Segment) -> anyhow::Result<Self::Statistics> {
        Ok(TextStatistics {
            num_indexed_documents: segment.num_indexed_documents,
        })
    }
}

#[derive(Debug, Default)]
pub struct TextStatistics {
    pub num_indexed_documents: u32,
}

impl SegmentStatistics for TextStatistics {
    fn add(lhs: anyhow::Result<Self>, rhs: anyhow::Result<Self>) -> anyhow::Result<Self> {
        Ok(Self {
            num_indexed_documents: lhs?.num_indexed_documents + rhs?.num_indexed_documents,
        })
    }

    fn log(&self) {}
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

impl From<TextIndexSnapshot> for SearchSnapshot<TextSearchIndex> {
    fn from(snapshot: TextIndexSnapshot) -> Self {
        Self {
            ts: snapshot.ts,
            data: match snapshot.data {
                TextIndexSnapshotData::SingleSegment(_) | TextIndexSnapshotData::Unknown(_) => {
                    SnapshotData::Unknown
                },
                TextIndexSnapshotData::MultiSegment(segments) => {
                    SnapshotData::MultiSegment(segments)
                },
            },
        }
    }
}

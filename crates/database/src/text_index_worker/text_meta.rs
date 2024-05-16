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
            TextIndexState,
        },
        IndexConfig,
    },
    document::ResolvedDocument,
    persistence::DocumentStream,
    runtime::Runtime,
    types::IndexId,
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
    type NewSegment = ();
    type PreviousSegments = ();
    type Schema = ();
    type Segment = FragmentedTextSegment;
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
            // TODO(sam): Implement this.
            data: SnapshotData::Unknown,
        }
    }
}

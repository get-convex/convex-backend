use std::collections::BTreeMap;

use common::{
    bootstrap_model::index::{
        search_index::{
            DeveloperSearchIndexConfig,
            FragmentedSearchSegment,
            SearchIndexSnapshot,
            SearchIndexSnapshotData,
            SearchIndexState,
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
    types::IndexId,
};
use sync_types::Timestamp;
use value::InternalId;

use crate::Snapshot;

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
                SearchIndexState::Backfilling(_) => {
                    // TODO(sam): Add support for a backfilling partial state to text search
                    SearchOnDiskState::Backfilling(BackfillState {
                        segments: vec![],
                        cursor: None,
                        backfill_snapshot_ts: None,
                    })
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

pub trait SearchIndex {
    type DeveloperConfig;
    type SnapshotData;
    type Segment;

    fn get_index_sizes(snapshot: Snapshot) -> anyhow::Result<BTreeMap<IndexId, usize>>;

    fn is_version_current(data: &SearchSnapshot<Self>) -> bool
    where
        Self: Sized;
}

pub struct TextSearchIndex;
impl SearchIndex for TextSearchIndex {
    type DeveloperConfig = DeveloperSearchIndexConfig;
    type Segment = FragmentedSearchSegment;
    type SnapshotData = SearchIndexSnapshotData;

    fn get_index_sizes(snapshot: Snapshot) -> anyhow::Result<BTreeMap<IndexId, usize>> {
        Ok(snapshot
            .search_indexes
            .backfilled_and_enabled_index_sizes()?
            .collect())
    }

    fn is_version_current(snapshot: &SearchSnapshot<Self>) -> bool {
        // TODO(sam): This doesn't match the current persistence version based check,
        // but it's closer to what vector search does.
        matches!(snapshot.data, SearchIndexSnapshotData::SingleSegment(_))
    }
}

pub struct VectorSearchIndex;

impl SearchIndex for VectorSearchIndex {
    type DeveloperConfig = DeveloperVectorIndexConfig;
    type Segment = FragmentedVectorSegment;
    type SnapshotData = VectorIndexSnapshotData;

    fn get_index_sizes(snapshot: Snapshot) -> anyhow::Result<BTreeMap<IndexId, usize>> {
        Ok(snapshot
            .vector_indexes
            .backfilled_and_enabled_index_sizes()?
            .collect())
    }

    fn is_version_current(snapshot: &SearchSnapshot<Self>) -> bool {
        snapshot.data.is_version_current()
    }
}
pub struct SearchIndexConfig<T: SearchIndex> {
    pub developer_config: T::DeveloperConfig,
    pub on_disk_state: SearchOnDiskState<T>,
}

pub struct SearchSnapshot<T: SearchIndex> {
    pub ts: Timestamp,
    pub data: T::SnapshotData,
}

pub struct BackfillState<T: SearchIndex> {
    pub segments: Vec<T::Segment>,
    pub cursor: Option<InternalId>,
    pub backfill_snapshot_ts: Option<Timestamp>,
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
            data: snapshot.data,
        }
    }
}

impl From<SearchIndexSnapshot> for SearchSnapshot<TextSearchIndex> {
    fn from(snapshot: SearchIndexSnapshot) -> Self {
        Self {
            ts: snapshot.ts,
            data: snapshot.data,
        }
    }
}

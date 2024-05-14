use std::fmt::Debug;

use serde::{
    Deserialize,
    Serialize,
};
use value::codegen_convex_serialization;

use super::{
    database_index::{
        DatabaseIndexState,
        DeveloperDatabaseIndexConfig,
        SerializedDatabaseIndexState,
        SerializedDeveloperDatabaseIndexConfig,
    },
    search_index::{
        DeveloperSearchIndexConfig,
        SearchIndexState,
        SerializedDeveloperSearchIndexConfig,
        SerializedSearchIndexState,
    },
    vector_index::{
        DeveloperVectorIndexConfig,
        SerializedDeveloperVectorIndexConfig,
        SerializedVectorIndexState,
        VectorIndexSnapshotData,
        VectorIndexState,
    },
};

/// Configuration that depends on the type of index.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(any(test, feature = "testing"), derive(proptest_derive::Arbitrary))]
pub enum IndexConfig {
    /// Standard database index.
    Database {
        developer_config: DeveloperDatabaseIndexConfig,

        /// Whether the index is fully backfilled or not on disk.
        on_disk_state: DatabaseIndexState,
    },

    /// Full text search index.
    Search {
        developer_config: DeveloperSearchIndexConfig,

        /// Whether the index is fully backfilled or not on disk.
        on_disk_state: SearchIndexState,
    },

    Vector {
        developer_config: DeveloperVectorIndexConfig,
        on_disk_state: VectorIndexState,
    },
}

impl IndexConfig {
    pub fn is_enabled(&self) -> bool {
        match self {
            IndexConfig::Database { on_disk_state, .. } => {
                matches!(on_disk_state, DatabaseIndexState::Enabled)
            },
            IndexConfig::Search { on_disk_state, .. } => {
                matches!(on_disk_state, SearchIndexState::SnapshottedAt(_))
            },
            IndexConfig::Vector { on_disk_state, .. } => {
                matches!(on_disk_state, VectorIndexState::SnapshottedAt(_))
            },
        }
    }

    pub fn is_backfilling(&self) -> bool {
        match self {
            IndexConfig::Database { on_disk_state, .. } => {
                matches!(on_disk_state, DatabaseIndexState::Backfilling(_))
            },
            IndexConfig::Search { on_disk_state, .. } => {
                matches!(on_disk_state, SearchIndexState::Backfilling(_))
            },
            IndexConfig::Vector { on_disk_state, .. } => {
                matches!(on_disk_state, VectorIndexState::Backfilling(_))
            },
        }
    }

    pub fn same_config(&self, config: &IndexConfig) -> bool {
        match (self, config) {
            (
                IndexConfig::Database {
                    developer_config, ..
                },
                IndexConfig::Database {
                    developer_config: config_to_compare,
                    ..
                },
            ) => developer_config == config_to_compare,
            (
                IndexConfig::Search {
                    developer_config, ..
                },
                IndexConfig::Search {
                    developer_config: config_to_compare,
                    ..
                },
            ) => developer_config == config_to_compare,
            (
                IndexConfig::Vector {
                    developer_config, ..
                },
                IndexConfig::Vector {
                    developer_config: config_to_compare,
                    ..
                },
            ) => developer_config == config_to_compare,
            (..) => false,
        }
    }

    /// Returns the estimated size of the index in bytes in a manner suitable
    /// for usage and pricing.
    ///
    /// The estimate here may not accurately reflect the actual number of
    /// stored bytes and may not be appropriate for estimate resource usage. For
    /// example, small dimension vector indexes may have 20% overhead from
    /// HNSW indexes that won't be reflected here, but would require
    /// additional RAM or disk space to serve.
    ///
    /// This is only implemented for vector indexes for now. Calling this method
    /// on other index types will panic.
    pub fn estimate_pricing_size_bytes(&self) -> anyhow::Result<u64> {
        match self {
            IndexConfig::Database { .. } | IndexConfig::Search { .. } => {
                // TODO(sam): We should support this for all index types in the future. Right
                // now search indexes are free and we estimate the size of
                // database indexes. Both of those could instead track usage in their metadata,
                // similar to vector indexes.
                anyhow::bail!("Only supported for vector indexes!")
            },
            IndexConfig::Vector {
                on_disk_state,
                developer_config,
            } => match on_disk_state {
                VectorIndexState::Backfilling(_) | VectorIndexState::Backfilled(_) => Ok(0),
                VectorIndexState::SnapshottedAt(snapshot) => match &snapshot.data {
                    VectorIndexSnapshotData::MultiSegment(segments) => segments
                        .iter()
                        .map(|segment| segment.non_deleted_size_bytes(developer_config.dimensions))
                        .sum::<anyhow::Result<_>>(),
                    VectorIndexSnapshotData::Unknown(_) => Ok(0),
                },
            },
        }
    }
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum SerializedIndexConfig {
    #[serde(rename_all = "camelCase")]
    Database {
        #[serde(flatten)]
        developer_config: SerializedDeveloperDatabaseIndexConfig,
        on_disk_state: SerializedDatabaseIndexState,
    },
    #[serde(rename_all = "camelCase")]
    Search {
        #[serde(flatten)]
        developer_config: SerializedDeveloperSearchIndexConfig,
        on_disk_state: SerializedSearchIndexState,
    },
    #[serde(rename_all = "camelCase")]
    Vector {
        #[serde(flatten)]
        developer_config: SerializedDeveloperVectorIndexConfig,
        on_disk_state: SerializedVectorIndexState,
    },
}

impl TryFrom<IndexConfig> for SerializedIndexConfig {
    type Error = anyhow::Error;

    fn try_from(config: IndexConfig) -> anyhow::Result<Self> {
        Ok(match config {
            IndexConfig::Database {
                developer_config,
                on_disk_state,
            } => SerializedIndexConfig::Database {
                developer_config: developer_config.try_into()?,
                on_disk_state: on_disk_state.try_into()?,
            },
            IndexConfig::Search {
                developer_config,
                on_disk_state,
            } => SerializedIndexConfig::Search {
                developer_config: developer_config.try_into()?,
                on_disk_state: on_disk_state.try_into()?,
            },
            IndexConfig::Vector {
                developer_config,
                on_disk_state,
            } => SerializedIndexConfig::Vector {
                developer_config: developer_config.try_into()?,
                on_disk_state: on_disk_state.try_into()?,
            },
        })
    }
}

impl TryFrom<SerializedIndexConfig> for IndexConfig {
    type Error = anyhow::Error;

    fn try_from(config: SerializedIndexConfig) -> anyhow::Result<Self> {
        Ok(match config {
            SerializedIndexConfig::Database {
                developer_config,
                on_disk_state,
            } => IndexConfig::Database {
                developer_config: developer_config.try_into()?,
                on_disk_state: on_disk_state.try_into()?,
            },
            SerializedIndexConfig::Search {
                developer_config,
                on_disk_state,
            } => IndexConfig::Search {
                developer_config: developer_config.try_into()?,
                on_disk_state: on_disk_state.try_into()?,
            },
            SerializedIndexConfig::Vector {
                developer_config,
                on_disk_state,
            } => IndexConfig::Vector {
                developer_config: developer_config.try_into()?,
                on_disk_state: on_disk_state.try_into()?,
            },
        })
    }
}

codegen_convex_serialization!(IndexConfig, SerializedIndexConfig, test_cases = 64);

#[cfg(test)]
mod tests {
    use maplit::btreeset;
    use value::{
        obj,
        ConvexValue,
    };

    use crate::bootstrap_model::index::{
        vector_index::{
            DeveloperVectorIndexConfig,
            FragmentedVectorSegment,
            VectorIndexBackfillState,
            VectorIndexState,
        },
        IndexConfig,
    };

    #[test]
    fn test_backwards_compatibility() -> anyhow::Result<()> {
        let serialized = obj!(
            "type" => "vector",
            "onDiskState" => {
                "state" => "backfilling",
                "document_cursor" => ConvexValue::Null,
                "backfill_snapshot_ts" => 10i64,
                "segments" => [
                    {
                        "segment_key" => "abc",
                        "id_tracker_key" => "def",
                        "deleted_bitset_key" => "ghi",
                        "id" => "jkl",
                        "num_vectors" => 11i64,
                        "num_deleted" => 12i64,
                    },
                ],
            },
            "dimensions" => 1536i64,
            "vectorField" => "embedding.field",
            "filterFields" => ["filter1", "filter2"],
        )?;
        let deserialized: IndexConfig = serialized.try_into()?;
        assert_eq!(
            deserialized,
            IndexConfig::Vector {
                developer_config: DeveloperVectorIndexConfig {
                    dimensions: 1536.try_into()?,
                    vector_field: "embedding.field".parse()?,
                    filter_fields: btreeset! { "filter1".parse()?, "filter2".parse()? },
                },
                on_disk_state: VectorIndexState::Backfilling(VectorIndexBackfillState {
                    cursor: None,
                    backfill_snapshot_ts: Some(10i64.try_into()?),
                    segments: vec![FragmentedVectorSegment {
                        segment_key: "abc".to_string().try_into()?,
                        id_tracker_key: "def".to_string().try_into()?,
                        deleted_bitset_key: "ghi".to_string().try_into()?,
                        id: "jkl".to_string(),
                        num_vectors: 11,
                        num_deleted: 12,
                    }]
                }),
            }
        );
        Ok(())
    }
}

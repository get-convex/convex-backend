use std::fmt::Debug;

use serde::{
    Deserialize,
    Serialize,
};
use value::codegen_convex_serialization;

use super::{
    database_index::{
        DatabaseIndexSpec,
        DatabaseIndexState,
        SerializedDatabaseIndexSpec,
        SerializedDatabaseIndexState,
    },
    text_index::{
        SerializedTextIndexSpec,
        SerializedTextIndexState,
        TextIndexSpec,
        TextIndexState,
    },
    vector_index::{
        SerializedVectorIndexSpec,
        SerializedVectorIndexState,
        VectorIndexSnapshotData,
        VectorIndexSpec,
        VectorIndexState,
    },
};

/// Configuration that depends on the type of index.
///
/// Split into two parts:
///   spec: Specification of the identity of index.
///   state: State of index that can change over time.
///
/// If spec changes (eg fields), it's a *different* index.
/// State can change over time (eg. backfill state or staged flag)
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(any(test, feature = "testing"), derive(proptest_derive::Arbitrary))]
pub enum IndexConfig {
    /// Standard database index.
    Database {
        spec: DatabaseIndexSpec,
        on_disk_state: DatabaseIndexState,
    },

    /// Full text search index.
    Text {
        spec: TextIndexSpec,
        on_disk_state: TextIndexState,
    },

    Vector {
        spec: VectorIndexSpec,
        on_disk_state: VectorIndexState,
    },
}

impl IndexConfig {
    pub fn is_staged(&self) -> bool {
        match self {
            Self::Database { on_disk_state, .. } => on_disk_state.is_staged(),
            Self::Text { on_disk_state, .. } => on_disk_state.is_staged(),
            Self::Vector { on_disk_state, .. } => on_disk_state.is_staged(),
        }
    }

    pub fn set_staged(&mut self, staged: bool) {
        match self {
            Self::Database { on_disk_state, .. } => on_disk_state.set_staged(staged),
            Self::Text { on_disk_state, .. } => on_disk_state.set_staged(staged),
            Self::Vector { on_disk_state, .. } => on_disk_state.set_staged(staged),
        }
    }

    pub fn is_enabled(&self) -> bool {
        match self {
            IndexConfig::Database { on_disk_state, .. } => {
                matches!(on_disk_state, DatabaseIndexState::Enabled)
            },
            IndexConfig::Text { on_disk_state, .. } => {
                matches!(on_disk_state, TextIndexState::SnapshottedAt(_))
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
            IndexConfig::Text { on_disk_state, .. } => {
                matches!(on_disk_state, TextIndexState::Backfilling(_))
            },
            IndexConfig::Vector { on_disk_state, .. } => {
                matches!(on_disk_state, VectorIndexState::Backfilling(_))
            },
        }
    }

    pub fn is_backfilled(&self) -> bool {
        match self {
            IndexConfig::Database { on_disk_state, .. } => {
                matches!(on_disk_state, DatabaseIndexState::Backfilled { .. })
            },
            IndexConfig::Text { on_disk_state, .. } => {
                matches!(on_disk_state, TextIndexState::Backfilled { .. })
            },
            IndexConfig::Vector { on_disk_state, .. } => {
                matches!(on_disk_state, VectorIndexState::Backfilled { .. })
            },
        }
    }

    pub fn same_spec(&self, config: &IndexConfig) -> bool {
        match (self, config) {
            (
                IndexConfig::Database { spec, .. },
                IndexConfig::Database {
                    spec: spec_to_compare,
                    ..
                },
            ) => spec == spec_to_compare,
            (
                IndexConfig::Text { spec, .. },
                IndexConfig::Text {
                    spec: spec_to_compare,
                    ..
                },
            ) => spec == spec_to_compare,
            (
                IndexConfig::Vector { spec, .. },
                IndexConfig::Vector {
                    spec: spec_to_compare,
                    ..
                },
            ) => spec == spec_to_compare,
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
            IndexConfig::Database { .. } | IndexConfig::Text { .. } => {
                // TODO(sam): We should support this for all index types in the future. Right
                // now search indexes are free and we estimate the size of
                // database indexes. Both of those could instead track usage in their metadata,
                // similar to vector indexes.
                anyhow::bail!("Only supported for vector indexes!")
            },
            IndexConfig::Vector {
                on_disk_state,
                spec,
            } => match on_disk_state {
                VectorIndexState::Backfilling(_) | VectorIndexState::Backfilled { .. } => Ok(0),
                VectorIndexState::SnapshottedAt(snapshot) => match &snapshot.data {
                    VectorIndexSnapshotData::MultiSegment(segments) => segments
                        .iter()
                        .map(|segment| segment.non_deleted_size_bytes(spec.dimensions))
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
        spec: SerializedDatabaseIndexSpec,
        on_disk_state: SerializedDatabaseIndexState,
    },
    #[serde(rename_all = "camelCase")]
    Search {
        #[serde(flatten)]
        spec: SerializedTextIndexSpec,
        on_disk_state: SerializedTextIndexState,
    },
    #[serde(rename_all = "camelCase")]
    Vector {
        #[serde(flatten)]
        spec: SerializedVectorIndexSpec,
        on_disk_state: SerializedVectorIndexState,
    },
}

impl TryFrom<IndexConfig> for SerializedIndexConfig {
    type Error = anyhow::Error;

    fn try_from(config: IndexConfig) -> anyhow::Result<Self> {
        Ok(match config {
            IndexConfig::Database {
                spec,
                on_disk_state,
            } => SerializedIndexConfig::Database {
                spec: spec.try_into()?,
                on_disk_state: on_disk_state.try_into()?,
            },
            IndexConfig::Text {
                spec,
                on_disk_state,
            } => SerializedIndexConfig::Search {
                spec: spec.try_into()?,
                on_disk_state: on_disk_state.try_into()?,
            },
            IndexConfig::Vector {
                spec,
                on_disk_state,
            } => SerializedIndexConfig::Vector {
                spec: spec.try_into()?,
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
                spec,
                on_disk_state,
            } => IndexConfig::Database {
                spec: spec.try_into()?,
                on_disk_state: on_disk_state.try_into()?,
            },
            SerializedIndexConfig::Search {
                spec,
                on_disk_state,
            } => IndexConfig::Text {
                spec: spec.try_into()?,
                on_disk_state: on_disk_state.try_into()?,
            },
            SerializedIndexConfig::Vector {
                spec,
                on_disk_state,
            } => IndexConfig::Vector {
                spec: spec.try_into()?,
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
            FragmentedVectorSegment,
            VectorIndexBackfillState,
            VectorIndexSpec,
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
                spec: VectorIndexSpec {
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
                    }],
                    staged: false,
                }),
            }
        );
        Ok(())
    }
}

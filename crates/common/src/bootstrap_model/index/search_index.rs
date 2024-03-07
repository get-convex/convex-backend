use std::{
    collections::{
        BTreeMap,
        BTreeSet,
    },
    convert::TryFrom,
};

use value::{
    obj,
    ConvexObject,
    ConvexValue,
    FieldName,
};

use crate::{
    paths::FieldPath,
    types::{
        ObjectKey,
        PersistenceVersion,
        Timestamp,
    },
};

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(any(test, feature = "testing"), derive(proptest_derive::Arbitrary))]
pub struct DeveloperSearchIndexConfig {
    /// The field to index for full text search.
    pub search_field: FieldPath,

    /// Other fields to index for equality filtering.
    pub filter_fields: BTreeSet<FieldPath>,
}

/// The state of a search index.
/// Search indexes begin in `Backfilling`.
/// Once the backfill completes, we'll have a snapshot at a timestamp which
/// continually moves forward.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(any(test, feature = "testing"), derive(proptest_derive::Arbitrary))]
pub enum SearchIndexState {
    Backfilling,
    Backfilled(SearchIndexSnapshot),
    SnapshottedAt(SearchIndexSnapshot),
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(any(test, feature = "testing"), derive(proptest_derive::Arbitrary))]
pub struct SearchIndexSnapshot {
    pub index: ObjectKey,
    pub ts: Timestamp,
    pub version: SearchSnapshotVersion,
}

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
#[cfg_attr(any(test, feature = "testing"), derive(proptest_derive::Arbitrary))]
pub enum SearchSnapshotVersion {
    /// V0 is the original version for search snapshots.
    /// In particular, it interprets missing fields as null.
    V0,
    /// V1 interprets missing fields as undefined.
    V1MissingAsUndefined,
    /// V2 uses string IDs
    V2UseStringIds,
}

impl SearchSnapshotVersion {
    pub fn new(persistence_version: PersistenceVersion) -> Self {
        // Add a new SearchSnapshotVersion if the index key format changes between
        // different persistence versions.
        match persistence_version {
            PersistenceVersion::V5 => Self::V2UseStringIds,
        }
    }

    pub fn to_code(&self) -> i64 {
        match self {
            Self::V0 => 0,
            Self::V1MissingAsUndefined => 1,
            Self::V2UseStringIds => 2,
        }
    }

    pub fn from_code(code: i64) -> anyhow::Result<Self> {
        match code {
            0 => Ok(Self::V0),
            1 => Ok(Self::V1MissingAsUndefined),
            2 => Ok(Self::V2UseStringIds),
            _ => anyhow::bail!("unrecognized search snapshot version {code:?}"),
        }
    }
}

impl TryFrom<SearchIndexState> for ConvexObject {
    type Error = anyhow::Error;

    fn try_from(state: SearchIndexState) -> Result<Self, Self::Error> {
        match state {
            SearchIndexState::Backfilling => obj!(
                "state" => "backfilling",
            ),
            SearchIndexState::Backfilled(snapshot) => snapshot_to_object("backfilled", &snapshot),
            SearchIndexState::SnapshottedAt(snapshot) => {
                snapshot_to_object("snapshotted", &snapshot)
            },
        }
    }
}

pub(crate) fn snapshot_to_object(
    state: &str,
    snapshot: &SearchIndexSnapshot,
) -> anyhow::Result<ConvexObject> {
    // This structure is intentionally flat for backwards compatibility.
    obj!(
        "state" => state,
        "index" => snapshot.index.to_string(),
        "ts" => ConvexValue::Int64(snapshot.ts.into()),
        "version" => snapshot.version.to_code(),
    )
}

pub(crate) fn snapshot_from_object(
    mut object_fields: BTreeMap<FieldName, ConvexValue>,
) -> anyhow::Result<SearchIndexSnapshot> {
    let index: ObjectKey = match object_fields.remove("index") {
        Some(ConvexValue::String(s)) => String::from(s).try_into()?,
        _ => anyhow::bail!(
            "Invalid or missing `index` field for SearchIndexState: {:?}",
            object_fields
        ),
    };
    let ts: Timestamp = match object_fields.remove("ts") {
        Some(ConvexValue::Int64(i)) => i.try_into()?,
        _ => anyhow::bail!(
            "Invalid or missing `ts` field for SearchIndexState: {:?}",
            object_fields
        ),
    };
    let version = match object_fields.remove("version") {
        Some(ConvexValue::Int64(i)) => SearchSnapshotVersion::from_code(i)?,
        _ => anyhow::bail!(
            "Invalid or missing `version` field for SearchIndexState: {:?}",
            object_fields
        ),
    };
    Ok(SearchIndexSnapshot { index, ts, version })
}

impl TryFrom<ConvexObject> for SearchIndexState {
    type Error = anyhow::Error;

    fn try_from(object: ConvexObject) -> Result<Self, Self::Error> {
        let mut object_fields: BTreeMap<_, _> = object.into();
        let state = match object_fields.remove("state") {
            Some(ConvexValue::String(s)) => s,
            _ => anyhow::bail!(
                "Missing `state` field for SearchIndexState: {:?}",
                object_fields
            ),
        };
        Ok(match state.to_string().as_str() {
            "backfilling" => SearchIndexState::Backfilling,
            "backfilled" => {
                let snapshot = snapshot_from_object(object_fields)?;
                SearchIndexState::Backfilled(snapshot)
            },
            "snapshotted" => {
                let snapshot = snapshot_from_object(object_fields)?;
                SearchIndexState::SnapshottedAt(snapshot)
            },
            _ => anyhow::bail!(
                "Invalid `state` field for SearchIndexState: {:?}",
                object_fields
            ),
        })
    }
}

impl TryFrom<pb::searchlight::SearchIndexConfig> for DeveloperSearchIndexConfig {
    type Error = anyhow::Error;

    fn try_from(proto: pb::searchlight::SearchIndexConfig) -> anyhow::Result<Self> {
        Ok(DeveloperSearchIndexConfig {
            search_field: proto
                .search_field_path
                .ok_or_else(|| anyhow::format_err!("Missing search_field_path"))?
                .try_into()?,
            filter_fields: proto
                .filter_fields
                .into_iter()
                .map(|i| i.try_into())
                .collect::<Result<Vec<_>, _>>()?
                .into_iter()
                .collect(),
        })
    }
}

impl From<DeveloperSearchIndexConfig> for pb::searchlight::SearchIndexConfig {
    fn from(config: DeveloperSearchIndexConfig) -> Self {
        pb::searchlight::SearchIndexConfig {
            search_field_path: Some(config.search_field.into()),
            filter_fields: config
                .filter_fields
                .into_iter()
                .map(|f| f.into())
                .collect::<Vec<_>>(),
        }
    }
}

#[cfg(test)]
mod tests {
    use cmd_util::env::env_config;
    use proptest::prelude::*;
    use sync_types::testing::assert_roundtrips;

    use super::*;

    proptest! {
        #![proptest_config(ProptestConfig { cases: 64 * env_config("CONVEX_PROPTEST_MULTIPLIER", 1), failure_persistence: None, .. ProptestConfig::default() })]

        #[test]
        fn test_developer_search_index_config_roundtrips(v in any::<DeveloperSearchIndexConfig>()) {
                assert_roundtrips::<
                DeveloperSearchIndexConfig,
                pb::searchlight::SearchIndexConfig
            >(v);
        }
    }
}

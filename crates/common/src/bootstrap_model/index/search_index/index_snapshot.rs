use std::convert::TryFrom;

use serde::{
    Deserialize,
    Serialize,
};

use crate::types::{
    ObjectKey,
    PersistenceVersion,
    Timestamp,
};

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

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SerializedSearchIndexSnapshot {
    index: String,
    ts: i64,
    version: i64,
}

impl TryFrom<SearchIndexSnapshot> for SerializedSearchIndexSnapshot {
    type Error = anyhow::Error;

    fn try_from(snapshot: SearchIndexSnapshot) -> Result<Self, Self::Error> {
        Ok(Self {
            index: snapshot.index.to_string(),
            ts: snapshot.ts.into(),
            version: snapshot.version.to_code(),
        })
    }
}

impl TryFrom<SerializedSearchIndexSnapshot> for SearchIndexSnapshot {
    type Error = anyhow::Error;

    fn try_from(serialized: SerializedSearchIndexSnapshot) -> Result<Self, Self::Error> {
        Ok(Self {
            index: serialized.index.try_into()?,
            ts: serialized.ts.try_into()?,
            version: SearchSnapshotVersion::from_code(serialized.version)?,
        })
    }
}

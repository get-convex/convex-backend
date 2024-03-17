use std::convert::TryFrom;

use serde::{
    Deserialize,
    Serialize,
};

use super::indexed_fields::IndexedFields;
use crate::paths::FieldPath;

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(any(test, feature = "testing"), derive(proptest_derive::Arbitrary))]
pub struct DeveloperDatabaseIndexConfig {
    /// Ordered field(s) to index. The "unindexed" primary key ordering of
    /// documents by [`DocumentId`] is represented by an empty vector.
    pub fields: IndexedFields,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SerializedDeveloperDatabaseIndexConfig {
    fields: Vec<String>,
}

impl TryFrom<DeveloperDatabaseIndexConfig> for SerializedDeveloperDatabaseIndexConfig {
    type Error = anyhow::Error;

    fn try_from(config: DeveloperDatabaseIndexConfig) -> anyhow::Result<Self> {
        Ok(Self {
            fields: Vec::<FieldPath>::from(config.fields)
                .into_iter()
                .map(String::from)
                .collect(),
        })
    }
}

impl TryFrom<SerializedDeveloperDatabaseIndexConfig> for DeveloperDatabaseIndexConfig {
    type Error = anyhow::Error;

    fn try_from(config: SerializedDeveloperDatabaseIndexConfig) -> anyhow::Result<Self> {
        Ok(Self {
            fields: config
                .fields
                .into_iter()
                .map(|p| p.parse())
                .collect::<anyhow::Result<Vec<FieldPath>>>()?
                .try_into()?,
        })
    }
}

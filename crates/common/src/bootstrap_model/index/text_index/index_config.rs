use std::collections::BTreeSet;

use serde::{
    Deserialize,
    Serialize,
};
use value::codegen_convex_serialization;

use crate::paths::FieldPath;

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(any(test, feature = "testing"), derive(proptest_derive::Arbitrary))]
pub struct DeveloperTextIndexConfig {
    /// The field to index for full text search.
    pub search_field: FieldPath,

    /// Other fields to index for equality filtering.
    pub filter_fields: BTreeSet<FieldPath>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SerializedDeveloperTextIndexConfig {
    search_field: String,
    filter_fields: Vec<String>,
}

impl TryFrom<DeveloperTextIndexConfig> for SerializedDeveloperTextIndexConfig {
    type Error = anyhow::Error;

    fn try_from(config: DeveloperTextIndexConfig) -> anyhow::Result<Self> {
        Ok(Self {
            search_field: config.search_field.into(),
            filter_fields: config.filter_fields.into_iter().map(String::from).collect(),
        })
    }
}

impl TryFrom<SerializedDeveloperTextIndexConfig> for DeveloperTextIndexConfig {
    type Error = anyhow::Error;

    fn try_from(config: SerializedDeveloperTextIndexConfig) -> anyhow::Result<Self> {
        Ok(Self {
            search_field: config.search_field.parse()?,
            filter_fields: config
                .filter_fields
                .into_iter()
                .map(|p| p.parse())
                .collect::<anyhow::Result<BTreeSet<FieldPath>>>()?,
        })
    }
}

codegen_convex_serialization!(DeveloperTextIndexConfig, SerializedDeveloperTextIndexConfig);

impl TryFrom<pb::searchlight::SearchIndexConfig> for DeveloperTextIndexConfig {
    type Error = anyhow::Error;

    fn try_from(proto: pb::searchlight::SearchIndexConfig) -> anyhow::Result<Self> {
        Ok(DeveloperTextIndexConfig {
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

impl From<DeveloperTextIndexConfig> for pb::searchlight::SearchIndexConfig {
    fn from(config: DeveloperTextIndexConfig) -> Self {
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

use std::fmt::Debug;

use anyhow::Ok;
use common::bootstrap_model::index::{
    database_index::{
        DatabaseIndexSpec,
        SerializedDatabaseIndexSpec,
    },
    text_index::{
        SerializedTextIndexSpec,
        TextIndexSpec,
    },
    vector_index::{
        SerializedVectorIndexSpec,
        VectorIndexSpec,
    },
    IndexConfig,
};
use serde::{
    Deserialize,
    Serialize,
};
use value::codegen_convex_serialization;

// Index config that's specified by the developer - including spec + staged
// state
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(any(test, feature = "testing"), derive(proptest_derive::Arbitrary))]
pub struct DeveloperIndexConfig {
    spec: DeveloperIndexSpec,
    staged: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(any(test, feature = "testing"), derive(proptest_derive::Arbitrary))]
pub enum DeveloperIndexSpec {
    /// Standard database index.
    Database(DatabaseIndexSpec),
    /// Full text search index.
    Search(TextIndexSpec),
    /// Vector search index.
    Vector(VectorIndexSpec),
}

impl From<IndexConfig> for DeveloperIndexConfig {
    fn from(value: IndexConfig) -> Self {
        let staged = value.is_staged();
        let spec = match value {
            IndexConfig::Database { spec, .. } => DeveloperIndexSpec::Database(spec),
            IndexConfig::Text { spec, .. } => DeveloperIndexSpec::Search(spec),
            IndexConfig::Vector { spec, .. } => DeveloperIndexSpec::Vector(spec),
        };
        Self { spec, staged }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(any(test, feature = "testing"), derive(proptest_derive::Arbitrary))]
pub struct SerializedNamedDeveloperIndexConfig {
    pub name: String,
    #[serde(flatten)]
    pub index_config: SerializedDeveloperIndexConfig,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type", rename_all = "camelCase")]
#[cfg_attr(any(test, feature = "testing"), derive(proptest_derive::Arbitrary))]
pub struct SerializedDeveloperIndexConfig {
    #[serde(flatten)]
    spec: SerializedDeveloperIndexSpec,
    #[serde(default)]
    staged: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type", rename_all = "camelCase")]
#[cfg_attr(any(test, feature = "testing"), derive(proptest_derive::Arbitrary))]
pub enum SerializedDeveloperIndexSpec {
    Database(SerializedDatabaseIndexSpec),
    Search(SerializedTextIndexSpec),
    Vector(SerializedVectorIndexSpec),
}

impl From<DeveloperIndexConfig> for SerializedDeveloperIndexConfig {
    fn from(index_config: DeveloperIndexConfig) -> Self {
        let spec = match index_config.spec {
            DeveloperIndexSpec::Database(spec) => {
                SerializedDeveloperIndexSpec::Database(spec.into())
            },
            DeveloperIndexSpec::Search(spec) => SerializedDeveloperIndexSpec::Search(spec.into()),
            DeveloperIndexSpec::Vector(spec) => SerializedDeveloperIndexSpec::Vector(spec.into()),
        };
        Self {
            spec,
            staged: index_config.staged,
        }
    }
}

impl TryFrom<SerializedDeveloperIndexConfig> for DeveloperIndexConfig {
    type Error = anyhow::Error;

    fn try_from(index_config: SerializedDeveloperIndexConfig) -> anyhow::Result<Self> {
        let spec = match index_config.spec {
            SerializedDeveloperIndexSpec::Database(spec) => {
                DeveloperIndexSpec::Database(spec.try_into()?)
            },
            SerializedDeveloperIndexSpec::Search(spec) => {
                DeveloperIndexSpec::Search(spec.try_into()?)
            },
            SerializedDeveloperIndexSpec::Vector(spec) => {
                DeveloperIndexSpec::Vector(spec.try_into()?)
            },
        };
        Ok(Self {
            spec,
            staged: index_config.staged,
        })
    }
}

codegen_convex_serialization!(DeveloperIndexConfig, SerializedDeveloperIndexConfig);

#[cfg(test)]
mod tests {
    use common::bootstrap_model::index::database_index::IndexedFields;

    use super::*;

    #[test]
    fn test_developer_index_config_serialization() -> anyhow::Result<()> {
        let config = DeveloperIndexConfig {
            spec: DeveloperIndexSpec::Database(DatabaseIndexSpec {
                fields: IndexedFields::creation_time(),
            }),
            staged: false,
        };
        let serialized = SerializedDeveloperIndexConfig::from(config.clone());
        let json = serde_json::to_value(&serialized)?;
        let expected = serde_json::json!({
            "type": "database",
            "fields": ["_creationTime"],
            "staged": false,
        });
        assert_eq!(json, expected);

        assert_eq!(
            config,
            serde_json::from_value::<SerializedDeveloperIndexConfig>(expected)?.try_into()?,
        );

        let old_format = serde_json::json!({
            "type": "database",
            "fields": ["_creationTime"],
        });
        assert_eq!(
            config,
            serde_json::from_value::<SerializedDeveloperIndexConfig>(old_format)?.try_into()?,
        );

        Ok(())
    }
}

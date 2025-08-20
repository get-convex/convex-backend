use std::fmt::Debug;

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

// Index config that's specified by the developer
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(any(test, feature = "testing"), derive(proptest_derive::Arbitrary))]
pub enum DeveloperIndexConfig {
    /// Standard database index.
    Database(DatabaseIndexSpec),

    /// Full text search index.
    Search(TextIndexSpec),

    Vector(VectorIndexSpec),
}

impl From<IndexConfig> for DeveloperIndexConfig {
    fn from(value: IndexConfig) -> Self {
        match value {
            IndexConfig::Database { spec, .. } => DeveloperIndexConfig::Database(spec),
            IndexConfig::Text { spec, .. } => DeveloperIndexConfig::Search(spec),
            IndexConfig::Vector { spec, .. } => DeveloperIndexConfig::Vector(spec),
        }
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
pub enum SerializedDeveloperIndexConfig {
    Database(SerializedDatabaseIndexSpec),
    Search(SerializedTextIndexSpec),
    Vector(SerializedVectorIndexSpec),
}

impl TryFrom<DeveloperIndexConfig> for SerializedDeveloperIndexConfig {
    type Error = anyhow::Error;

    fn try_from(index_config: DeveloperIndexConfig) -> anyhow::Result<Self> {
        Ok(match index_config {
            DeveloperIndexConfig::Database(config) => Self::Database(config.try_into()?),
            DeveloperIndexConfig::Search(config) => Self::Search(config.try_into()?),
            DeveloperIndexConfig::Vector(config) => Self::Vector(config.try_into()?),
        })
    }
}

impl TryFrom<SerializedDeveloperIndexConfig> for DeveloperIndexConfig {
    type Error = anyhow::Error;

    fn try_from(index_config: SerializedDeveloperIndexConfig) -> anyhow::Result<Self> {
        Ok(match index_config {
            SerializedDeveloperIndexConfig::Database(config) => Self::Database(config.try_into()?),
            SerializedDeveloperIndexConfig::Search(config) => Self::Search(config.try_into()?),
            SerializedDeveloperIndexConfig::Vector(config) => Self::Vector(config.try_into()?),
        })
    }
}

codegen_convex_serialization!(DeveloperIndexConfig, SerializedDeveloperIndexConfig);

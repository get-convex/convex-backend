use std::collections::BTreeSet;

use serde::{
    Deserialize,
    Serialize,
};
use value::{
    codegen_convex_serialization,
    FieldPath,
};

use super::VectorDimensions;

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(any(test, feature = "testing"), derive(proptest_derive::Arbitrary))]
pub struct DeveloperVectorIndexConfig {
    // Dimensions of the vectors
    pub dimensions: VectorDimensions,

    /// The field to index for vector search.
    pub vector_field: FieldPath,

    /// Other fields to index for equality filtering.
    pub filter_fields: BTreeSet<FieldPath>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SerializedDeveloperVectorIndexConfig {
    // Support legacy alpha users with the old dimension field.
    #[serde(alias = "dimension")]
    dimensions: i64,
    vector_field: String,
    filter_fields: Vec<String>,
}

impl TryFrom<DeveloperVectorIndexConfig> for SerializedDeveloperVectorIndexConfig {
    type Error = anyhow::Error;

    fn try_from(config: DeveloperVectorIndexConfig) -> anyhow::Result<Self> {
        Ok(Self {
            dimensions: u32::from(config.dimensions) as i64,
            vector_field: config.vector_field.into(),
            filter_fields: config.filter_fields.into_iter().map(String::from).collect(),
        })
    }
}

impl TryFrom<SerializedDeveloperVectorIndexConfig> for DeveloperVectorIndexConfig {
    type Error = anyhow::Error;

    fn try_from(config: SerializedDeveloperVectorIndexConfig) -> anyhow::Result<Self> {
        Ok(Self {
            dimensions: VectorDimensions::try_from(u32::try_from(config.dimensions)?)?,
            vector_field: config.vector_field.parse()?,
            filter_fields: config
                .filter_fields
                .into_iter()
                .map(|p| p.parse())
                .collect::<anyhow::Result<BTreeSet<FieldPath>>>()?,
        })
    }
}

codegen_convex_serialization!(
    DeveloperVectorIndexConfig,
    SerializedDeveloperVectorIndexConfig
);

impl TryFrom<pb::searchlight::VectorIndexConfig> for DeveloperVectorIndexConfig {
    type Error = anyhow::Error;

    fn try_from(proto: pb::searchlight::VectorIndexConfig) -> anyhow::Result<Self> {
        Ok(DeveloperVectorIndexConfig {
            dimensions: VectorDimensions::try_from(proto.dimension)?,
            vector_field: proto
                .vector_field_path
                .ok_or_else(|| anyhow::format_err!("Missing vector_field_path"))?
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

impl From<DeveloperVectorIndexConfig> for pb::searchlight::VectorIndexConfig {
    fn from(config: DeveloperVectorIndexConfig) -> Self {
        pb::searchlight::VectorIndexConfig {
            dimension: u32::from(config.dimensions),
            vector_field_path: Some(config.vector_field.into()),
            filter_fields: config
                .filter_fields
                .into_iter()
                .map(|f| f.into())
                .collect::<Vec<_>>(),
        }
    }
}

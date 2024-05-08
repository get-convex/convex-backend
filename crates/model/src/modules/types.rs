use common::types::ModuleEnvironment;
use serde::{
    Deserialize,
    Serialize,
};
use sync_types::{
    CanonicalizedModulePath,
    ModulePath,
};
use value::{
    codegen_convex_serialization,
    DeveloperDocumentId,
};

use super::module_versions::{
    AnalyzedModule,
    ModuleVersion,
    SerializedAnalyzedModule,
};
use crate::source_packages::types::SourcePackageId;

/// In-memory representation of a module's metadata.
#[derive(Debug, Clone, Eq, PartialEq)]
#[cfg_attr(any(test, feature = "testing"), derive(proptest_derive::Arbitrary))]
pub struct ModuleMetadata {
    /// Path stored as a "path" field.
    pub path: CanonicalizedModulePath,
    /// What is the latest version of the module?
    pub latest_version: ModuleVersion,

    // Fields previously in ModuleVersionMetadata.
    // In migration phase, fields are duplicated here but not read.
    pub source_package_id: Option<SourcePackageId>,
    pub environment: ModuleEnvironment,
    pub analyze_result: Option<AnalyzedModule>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SerializedModuleMetadata {
    pub path: String,
    pub latest_version: ModuleVersion,
    pub deleted: Option<bool>,
    pub source_package_id: Option<String>,
    pub environment: String,
    pub analyze_result: Option<SerializedAnalyzedModule>,
}

impl TryFrom<SerializedModuleMetadata> for ModuleMetadata {
    type Error = anyhow::Error;

    fn try_from(m: SerializedModuleMetadata) -> anyhow::Result<Self> {
        Ok(Self {
            path: {
                let path: ModulePath = m.path.parse()?;
                // TODO: Remove this canonicalization once we've fully backfilled canonicalized
                // module paths.
                path.canonicalize()
            },
            latest_version: m.latest_version,
            source_package_id: m
                .source_package_id
                .map(|s| DeveloperDocumentId::decode(&s))
                .transpose()?
                .map(|id| id.into()),
            environment: m.environment.parse()?,
            analyze_result: m.analyze_result.map(|s| s.try_into()).transpose()?,
        })
    }
}

impl TryFrom<ModuleMetadata> for SerializedModuleMetadata {
    type Error = anyhow::Error;

    fn try_from(m: ModuleMetadata) -> anyhow::Result<Self> {
        Ok(Self {
            path: String::from(m.path),
            latest_version: m.latest_version,
            deleted: Some(false),
            source_package_id: m
                .source_package_id
                .map(|s| DeveloperDocumentId::from(s).to_string()),
            environment: m.environment.to_string(),
            analyze_result: m.analyze_result.map(|s| s.try_into()).transpose()?,
        })
    }
}

codegen_convex_serialization!(ModuleMetadata, SerializedModuleMetadata);

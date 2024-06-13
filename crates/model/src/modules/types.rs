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
    sha256::Sha256Digest,
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
    pub latest_version: Option<ModuleVersion>,

    pub source_package_id: SourcePackageId,
    pub environment: ModuleEnvironment,
    pub analyze_result: Option<AnalyzedModule>,
    // This is a hash of source + source_map.
    pub sha256: Sha256Digest,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SerializedModuleMetadata {
    pub path: String,
    pub latest_version: Option<ModuleVersion>,
    pub source_package_id: String,
    pub environment: String,
    pub analyze_result: Option<SerializedAnalyzedModule>,
    pub sha256: String,
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
            source_package_id: DeveloperDocumentId::decode(&m.source_package_id)?.into(),
            environment: m.environment.parse()?,
            analyze_result: m.analyze_result.map(|s| s.try_into()).transpose()?,
            sha256: Sha256Digest::from_base64(&m.sha256)?,
        })
    }
}

impl TryFrom<ModuleMetadata> for SerializedModuleMetadata {
    type Error = anyhow::Error;

    fn try_from(m: ModuleMetadata) -> anyhow::Result<Self> {
        Ok(Self {
            path: String::from(m.path),
            latest_version: m.latest_version,
            source_package_id: DeveloperDocumentId::from(m.source_package_id).to_string(),
            environment: m.environment.to_string(),
            analyze_result: m.analyze_result.map(|s| s.try_into()).transpose()?,
            sha256: m.sha256.as_base64(),
        })
    }
}

codegen_convex_serialization!(ModuleMetadata, SerializedModuleMetadata);

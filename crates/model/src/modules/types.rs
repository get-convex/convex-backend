use serde::{
    Deserialize,
    Serialize,
};
use sync_types::{
    CanonicalizedModulePath,
    ModulePath,
};
use value::codegen_convex_serialization;

use super::module_versions::ModuleVersion;

/// In-memory representation of a module's metadata.
#[derive(Debug, Clone, Eq, PartialEq)]
#[cfg_attr(any(test, feature = "testing"), derive(proptest_derive::Arbitrary))]
pub struct ModuleMetadata {
    /// Path stored as a "path" field.
    pub path: CanonicalizedModulePath,
    /// What is the latest version of the module?
    pub latest_version: ModuleVersion,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SerializedModuleMetadata {
    pub path: String,
    pub latest_version: ModuleVersion,
    pub deleted: Option<bool>,
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
        })
    }
}

codegen_convex_serialization!(ModuleMetadata, SerializedModuleMetadata);

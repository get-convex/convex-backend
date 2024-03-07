use std::{
    collections::BTreeMap,
    convert::TryFrom,
};

use common::obj;
use sync_types::{
    CanonicalizedModulePath,
    ModulePath,
};
use value::{
    ConvexObject,
    ConvexValue,
};

use super::module_versions::ModuleVersion;

/// In-memory representation of a module's metadata.
#[derive(Debug, Clone, Eq, PartialEq)]
#[cfg_attr(any(test, feature = "testing"), derive(proptest_derive::Arbitrary))]
pub struct ModuleMetadata {
    /// Path stored as a "path" field.
    pub path: CanonicalizedModulePath,
    /// What is the latest version of the module?
    pub latest_version: ModuleVersion,
    /// Has the module been deleted?
    pub deleted: bool,
}

impl TryFrom<ModuleMetadata> for ConvexObject {
    type Error = anyhow::Error;

    fn try_from(m: ModuleMetadata) -> anyhow::Result<Self> {
        obj!(
            "path" => String::from(m.path),
            "latestVersion" => m.latest_version,
            "deleted" => m.deleted,
        )
    }
}

impl TryFrom<ModuleMetadata> for ConvexValue {
    type Error = anyhow::Error;

    fn try_from(value: ModuleMetadata) -> Result<Self, Self::Error> {
        Ok(ConvexObject::try_from(value)?.into())
    }
}

impl TryFrom<ConvexObject> for ModuleMetadata {
    type Error = anyhow::Error;

    fn try_from(object: ConvexObject) -> Result<Self, Self::Error> {
        let mut fields: BTreeMap<_, _> = object.into();
        let path = match fields.remove("path") {
            Some(ConvexValue::String(s)) => {
                let path: ModulePath = s.parse()?;
                // TODO: Remove this canonicalization once we've fully backfilled canonicalized
                // module paths.
                path.canonicalize()
            },
            v => anyhow::bail!("Invalid path field for ModuleMetadata: {:?}", v),
        };
        let latest_version = match fields.remove("latestVersion") {
            Some(ConvexValue::Int64(i)) => i,
            v => anyhow::bail!("Invalid latest_version field for ModuleMetadata: {:?}", v),
        };
        let deleted = match fields.remove("deleted") {
            Some(ConvexValue::Boolean(s)) => s,
            v => anyhow::bail!("Invalid deleted field for ModuleMetadata: {:?}", v),
        };
        Ok(Self {
            path,
            latest_version,
            deleted,
        })
    }
}

#[cfg(test)]
mod tests {
    use common::testing::assert_roundtrips;
    use proptest::prelude::*;
    use value::ConvexObject;

    use super::ModuleMetadata;

    proptest! {
        #![proptest_config(
            ProptestConfig { failure_persistence: None, ..ProptestConfig::default() }
        )]
        #[test]
        fn test_module_roundtrips(v in any::<ModuleMetadata>()) {
            assert_roundtrips::<ModuleMetadata, ConvexObject>(v);
        }
    }
}

use std::collections::BTreeMap;

use common::types::{
    NodeDependency,
    ObjectKey,
};
use value::{
    id_v6::DeveloperDocumentId,
    obj,
    sha256::Sha256Digest,
    ConvexObject,
    ConvexValue,
};

use crate::source_packages::types::PackageSize;

#[cfg_attr(any(test, feature = "testing"), derive(proptest_derive::Arbitrary))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExternalDepsPackage {
    pub storage_key: ObjectKey,
    pub sha256: Sha256Digest,
    pub deps: Vec<NodeDependency>,
    pub package_size: PackageSize,
}

#[cfg_attr(any(test, feature = "testing"), derive(proptest_derive::Arbitrary))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExternalDepsPackageId(DeveloperDocumentId);

impl From<DeveloperDocumentId> for ExternalDepsPackageId {
    fn from(id: DeveloperDocumentId) -> Self {
        Self(id)
    }
}

impl From<ExternalDepsPackageId> for DeveloperDocumentId {
    fn from(value: ExternalDepsPackageId) -> Self {
        value.0
    }
}

impl From<ExternalDepsPackageId> for ConvexValue {
    fn from(value: ExternalDepsPackageId) -> Self {
        value.0.into()
    }
}

impl TryFrom<ConvexValue> for ExternalDepsPackageId {
    type Error = anyhow::Error;

    fn try_from(value: ConvexValue) -> Result<Self, Self::Error> {
        let id: DeveloperDocumentId = value.try_into()?;
        Ok(Self(id))
    }
}

impl TryFrom<ConvexObject> for ExternalDepsPackage {
    type Error = anyhow::Error;

    fn try_from(value: ConvexObject) -> Result<Self, Self::Error> {
        let mut fields = BTreeMap::from(value);

        let storage_key = match fields.remove("storageKey") {
            Some(ConvexValue::String(key)) => String::from(key).try_into()?,
            _ => anyhow::bail!("Missing or invalid 'storage_key' in {fields:?}"),
        };
        let sha256 = match fields.remove("sha256") {
            Some(sha256) => sha256.try_into()?,
            _ => anyhow::bail!("Missing or invalid 'sha256' in {fields:?}"),
        };
        let deps = match fields.remove("deps") {
            Some(ConvexValue::Array(arr)) => arr
                .into_iter()
                .map(NodeDependency::try_from)
                .collect::<Result<Vec<_>, anyhow::Error>>()?,
            _ => anyhow::bail!("Missing or invalid 'deps' in {fields:?}"),
        };
        let package_size: PackageSize = match fields.remove("packageSize") {
            Some(ConvexValue::Object(o)) => o.try_into()?,
            // Use PackageSize::default for safety on backends that happen to have used
            // external deps before they were released, so we don't break them
            None => PackageSize::default(),
            _ => anyhow::bail!("Invalid 'packageSize' for ExternalDepsPackage in {fields:?}"),
        };
        Ok(Self {
            storage_key,
            sha256,
            deps,
            package_size,
        })
    }
}

impl TryFrom<ExternalDepsPackage> for ConvexObject {
    type Error = anyhow::Error;

    fn try_from(value: ExternalDepsPackage) -> Result<Self, Self::Error> {
        let storage_key: String = value.storage_key.into();
        obj!(
            "storageKey" => storage_key,
            "sha256" => value.sha256,
            "deps" => ConvexValue::Array(
                value.deps
                    .into_iter()
                    .map(ConvexValue::try_from)
                    .collect::<Result<Vec<_>, anyhow::Error>>()?
                    .try_into()?
            ),
            "packageSize" => ConvexValue::Object(value.package_size.try_into()?),
        )
    }
}

#[cfg(test)]
mod tests {
    use common::testing::assert_roundtrips;
    use proptest::prelude::*;
    use value::ConvexObject;

    use super::ExternalDepsPackage;

    proptest! {
        #![proptest_config(
            ProptestConfig { failure_persistence: None, ..ProptestConfig::default() }
        )]
        #[test]
        fn test_external_package_roundtrip(v in any::<ExternalDepsPackage>()) {
            assert_roundtrips::<ExternalDepsPackage, ConvexObject>(v);
        }
    }
}

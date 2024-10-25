use std::{
    collections::BTreeMap,
    fmt::Formatter,
};

use common::{
    obj,
    types::ObjectKey,
};
use errors::ErrorMetadata;
use humansize::{
    FormatSize,
    BINARY,
};
use value::{
    heap_size::HeapSize,
    id_v6::DeveloperDocumentId,
    sha256::Sha256Digest,
    ConvexObject,
    ConvexValue,
};

use crate::external_packages::types::ExternalDepsPackageId;

#[cfg_attr(any(test, feature = "testing"), derive(proptest_derive::Arbitrary))]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SourcePackage {
    pub storage_key: ObjectKey,
    pub sha256: Sha256Digest,
    pub external_deps_package_id: Option<ExternalDepsPackageId>,
    pub package_size: PackageSize,
}

#[cfg_attr(any(test, feature = "testing"), derive(proptest_derive::Arbitrary))]
#[derive(Copy, Clone, Debug, Eq, PartialEq, Default)]
pub struct PackageSize {
    pub zipped_size_bytes: usize,
    pub unzipped_size_bytes: usize,
}

impl std::ops::Add for PackageSize {
    type Output = PackageSize;

    fn add(self, rhs: Self) -> Self::Output {
        Self {
            zipped_size_bytes: self.zipped_size_bytes + rhs.zipped_size_bytes,
            unzipped_size_bytes: self.unzipped_size_bytes + rhs.unzipped_size_bytes,
        }
    }
}

impl std::ops::AddAssign for PackageSize {
    fn add_assign(&mut self, rhs: Self) {
        self.zipped_size_bytes += rhs.zipped_size_bytes;
        self.unzipped_size_bytes += rhs.unzipped_size_bytes;
    }
}

impl std::fmt::Display for PackageSize {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "(Zipped: {}, Unzipped, {})",
            self.zipped_size_bytes, self.unzipped_size_bytes
        )
    }
}

const MAX_ZIPPED_PACKAGES_SIZE: usize = 45_000_000; // 45 MB - Lambda gives us 50 MB so we have 5 MB wiggle room
const MAX_UNZIPPED_PACKAGES_SIZE: usize = 230_000_000; // 230 MB - Lambda gives us 250 MB

impl PackageSize {
    pub fn verify_size(&self) -> anyhow::Result<()> {
        if self.zipped_size_bytes >= MAX_ZIPPED_PACKAGES_SIZE {
            anyhow::bail!(ErrorMetadata::bad_request(
                "ModulesTooLarge",
                format!(
                    "Total module size exceeded the zipped maximum ({} > maximum size {})",
                    self.zipped_size_bytes.format_size(BINARY),
                    MAX_ZIPPED_PACKAGES_SIZE.format_size(BINARY)
                ),
            ),);
        }
        if self.unzipped_size_bytes >= MAX_UNZIPPED_PACKAGES_SIZE {
            anyhow::bail!(ErrorMetadata::bad_request(
                "ModulesTooLarge",
                format!(
                    "Total module size exceeded the unzipped maximum ({} > maximum size {})",
                    self.unzipped_size_bytes.format_size(BINARY),
                    MAX_UNZIPPED_PACKAGES_SIZE.format_size(BINARY)
                ),
            ),);
        }
        Ok(())
    }
}

impl TryFrom<ConvexObject> for PackageSize {
    type Error = anyhow::Error;

    fn try_from(value: ConvexObject) -> Result<Self, Self::Error> {
        let mut fields = BTreeMap::from(value);

        let zipped_size_bytes: usize = match fields.remove("zippedSizeBytes") {
            Some(ConvexValue::Int64(i)) => i as usize,
            _ => anyhow::bail!("Missing or invalid 'zippedSize' in {fields:?}"),
        };
        let unzipped_size_bytes: usize = match fields.remove("unzippedSizeBytes") {
            Some(ConvexValue::Int64(i)) => i as usize,
            _ => anyhow::bail!("Missing or invalid 'unzippedSizeBytes' in {fields:?}"),
        };
        Ok(PackageSize {
            zipped_size_bytes,
            unzipped_size_bytes,
        })
    }
}

impl TryFrom<PackageSize> for ConvexObject {
    type Error = anyhow::Error;

    fn try_from(value: PackageSize) -> Result<Self, Self::Error> {
        obj!(
            "zippedSizeBytes" => ConvexValue::Int64(value.zipped_size_bytes as i64),
            "unzippedSizeBytes" => ConvexValue::Int64(value.unzipped_size_bytes as i64),
        )
    }
}

#[cfg_attr(any(test, feature = "testing"), derive(proptest_derive::Arbitrary))]
#[derive(Debug, Clone, PartialEq, Eq, Copy, PartialOrd, Ord, Hash)]
pub struct SourcePackageId(DeveloperDocumentId);

impl HeapSize for SourcePackageId {
    fn heap_size(&self) -> usize {
        self.0.heap_size()
    }
}

impl From<DeveloperDocumentId> for SourcePackageId {
    fn from(id: DeveloperDocumentId) -> Self {
        Self(id)
    }
}

impl From<SourcePackageId> for ConvexValue {
    fn from(value: SourcePackageId) -> Self {
        let id: DeveloperDocumentId = value.into();
        id.into()
    }
}

impl TryFrom<ConvexValue> for SourcePackageId {
    type Error = anyhow::Error;

    fn try_from(value: ConvexValue) -> Result<Self, Self::Error> {
        let id: DeveloperDocumentId = value.try_into()?;
        Ok(SourcePackageId(id))
    }
}

impl From<SourcePackageId> for DeveloperDocumentId {
    fn from(id: SourcePackageId) -> DeveloperDocumentId {
        id.0
    }
}

impl TryFrom<SourcePackage> for ConvexObject {
    type Error = anyhow::Error;

    fn try_from(
        SourcePackage {
            storage_key,
            sha256,
            external_deps_package_id,
            package_size,
        }: SourcePackage,
    ) -> Result<Self, Self::Error> {
        let storage_key: String = storage_key.into();
        obj!(
            "storageKey" => storage_key,
            "sha256" => sha256,
            "externalPackageId" => external_deps_package_id
                .map(ConvexValue::try_from)
                .transpose()?
                .unwrap_or(ConvexValue::Null),
            "packageSize" => ConvexValue::Object(package_size.try_into()?),
        )
    }
}

impl TryFrom<ConvexObject> for SourcePackage {
    type Error = anyhow::Error;

    fn try_from(value: ConvexObject) -> Result<Self, Self::Error> {
        let mut object_fields: BTreeMap<_, _> = value.into();
        let storage_key = match object_fields.remove("storageKey") {
            Some(ConvexValue::String(key)) => String::from(key).try_into()?,
            _ => anyhow::bail!("Missing 'storageKey' in {object_fields:?}"),
        };
        let sha256 = match object_fields.remove("sha256") {
            Some(sha256) => sha256.try_into()?,
            _ => anyhow::bail!("Missing 'sha256' in {object_fields:?}"),
        };
        let external_package_id = match object_fields.remove("externalPackageId") {
            Some(ConvexValue::Null) | None => None,
            Some(ConvexValue::String(s)) => Some(DeveloperDocumentId::decode(&s)?.into()),
            _ => anyhow::bail!("Invalid 'externalPackageId' in {object_fields:?}"),
        };
        let package_size: PackageSize = match object_fields.remove("packageSize") {
            Some(ConvexValue::Object(o)) => o.try_into()?,
            // Just use default for old source packages
            None => PackageSize::default(),
            _ => anyhow::bail!("Invalid 'packageSize' in {object_fields:?}"),
        };
        Ok(Self {
            storage_key,
            sha256,
            external_deps_package_id: external_package_id,
            package_size,
        })
    }
}

#[cfg(test)]
mod tests {
    use cmd_util::env::env_config;
    use common::testing::assert_roundtrips;
    use proptest::prelude::*;
    use value::ConvexObject;

    use super::SourcePackage;

    proptest! {
        #![proptest_config(
            ProptestConfig { cases: 256 * env_config("CONVEX_PROPTEST_MULTIPLIER", 1), failure_persistence: None, ..ProptestConfig::default() }
        )]
        #[test]
        fn test_source_package_roundtrip(v in any::<SourcePackage>()) {
            assert_roundtrips::<SourcePackage, ConvexObject>(v);
        }
    }
}

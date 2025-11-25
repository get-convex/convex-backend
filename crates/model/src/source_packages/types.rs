use std::{
    fmt::Formatter,
    str::FromStr,
};

use common::types::ObjectKey;
use errors::ErrorMetadata;
use humansize::{
    FormatSize,
    BINARY,
};
use serde::{
    Deserialize,
    Serialize,
};
use serde_bytes::ByteBuf;
use value::{
    codegen_convex_serialization,
    heap_size::HeapSize,
    id_v6::DeveloperDocumentId,
    sha256::Sha256Digest,
    ConvexValue,
};

use crate::external_packages::types::ExternalDepsPackageId;

#[cfg_attr(any(test, feature = "testing"), derive(proptest_derive::Arbitrary))]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum NodeVersion {
    V18x,
    V20x,
    V22x,
}

impl FromStr for NodeVersion {
    type Err = anyhow::Error;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "18" => Ok(NodeVersion::V18x),
            "20" => Ok(NodeVersion::V20x),
            "22" => Ok(NodeVersion::V22x),
            _ => anyhow::bail!("Invalid node version: {value}"),
        }
    }
}

impl From<NodeVersion> for String {
    fn from(value: NodeVersion) -> String {
        match value {
            NodeVersion::V18x => "18".to_string(),
            NodeVersion::V20x => "20".to_string(),
            NodeVersion::V22x => "22".to_string(),
        }
    }
}

#[cfg_attr(any(test, feature = "testing"), derive(proptest_derive::Arbitrary))]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SourcePackage {
    pub storage_key: ObjectKey,
    pub sha256: Sha256Digest,
    pub external_deps_package_id: Option<ExternalDepsPackageId>,
    pub package_size: PackageSize,
    pub node_version: Option<NodeVersion>,
}

#[cfg_attr(any(test, feature = "testing"), derive(proptest_derive::Arbitrary))]
#[derive(Copy, Clone, Debug, Eq, PartialEq, Default)]
pub struct PackageSize {
    #[cfg_attr(
        any(test, feature = "testing"),
        proptest(strategy = "0..=i64::MAX as usize")
    )]
    pub zipped_size_bytes: usize,
    #[cfg_attr(
        any(test, feature = "testing"),
        proptest(strategy = "0..=i64::MAX as usize")
    )]
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

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SerializedPackageSize {
    zipped_size_bytes: i64,
    unzipped_size_bytes: i64,
}

impl TryFrom<SerializedPackageSize> for PackageSize {
    type Error = anyhow::Error;

    fn try_from(value: SerializedPackageSize) -> Result<Self, Self::Error> {
        let zipped_size_bytes: usize = value.zipped_size_bytes.try_into()?;
        let unzipped_size_bytes: usize = value.unzipped_size_bytes.try_into()?;
        Ok(PackageSize {
            zipped_size_bytes,
            unzipped_size_bytes,
        })
    }
}

impl TryFrom<PackageSize> for SerializedPackageSize {
    type Error = anyhow::Error;

    fn try_from(value: PackageSize) -> Result<Self, Self::Error> {
        Ok(SerializedPackageSize {
            zipped_size_bytes: value.zipped_size_bytes.try_into()?,
            unzipped_size_bytes: value.unzipped_size_bytes.try_into()?,
        })
    }
}

codegen_convex_serialization!(PackageSize, SerializedPackageSize);

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

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SerializedSourcePackage {
    storage_key: String,
    sha256: ByteBuf,
    external_package_id: Option<String>,
    package_size: Option<SerializedPackageSize>,
    node_version: Option<String>,
}

impl TryFrom<SourcePackage> for SerializedSourcePackage {
    type Error = anyhow::Error;

    fn try_from(value: SourcePackage) -> anyhow::Result<Self> {
        Ok(SerializedSourcePackage {
            storage_key: value.storage_key.into(),
            sha256: ByteBuf::from(value.sha256.to_vec()),
            external_package_id: value
                .external_deps_package_id
                .map(|id| DeveloperDocumentId::from(id).encode()),
            package_size: Some(value.package_size.try_into()?),
            node_version: value.node_version.map(String::from),
        })
    }
}
impl TryFrom<SerializedSourcePackage> for SourcePackage {
    type Error = anyhow::Error;

    fn try_from(value: SerializedSourcePackage) -> Result<Self, Self::Error> {
        let storage_key = value.storage_key.try_into()?;
        let sha256 = value.sha256.into_vec().try_into()?;
        let external_package_id = match value.external_package_id {
            None => None,
            Some(s) => Some(DeveloperDocumentId::decode(&s)?.into()),
        };
        let package_size: PackageSize = match value.package_size {
            Some(o) => o.try_into()?,
            // Just use default for old source packages
            None => PackageSize::default(),
        };
        let node_version = match value.node_version {
            None => None,
            Some(s) => Some(s.parse()?),
        };
        Ok(Self {
            storage_key,
            sha256,
            external_deps_package_id: external_package_id,
            package_size,
            node_version,
        })
    }
}

codegen_convex_serialization!(SourcePackage, SerializedSourcePackage);

#[cfg(test)]
mod tests {
    use cmd_util::env::env_config;
    use common::testing::assert_roundtrips;
    use proptest::prelude::*;
    use value::{
        sha256::Sha256Digest,
        ConvexObject,
        DeveloperDocumentId,
    };

    use super::SourcePackage;
    use crate::{
        external_packages::types::ExternalDepsPackageId,
        source_packages::types::PackageSize,
    };

    proptest! {
        #![proptest_config(
            ProptestConfig { cases: 256 * env_config("CONVEX_PROPTEST_MULTIPLIER", 1), failure_persistence: None, ..ProptestConfig::default() }
        )]
        #[test]
        fn test_source_package_roundtrip(v in any::<SourcePackage>()) {
            assert_roundtrips::<SourcePackage, ConvexObject>(v);
        }
    }

    #[test]
    fn test_frozen_source_package() {
        let value = value::json_deserialize(r#"{
            "externalPackageId":"k423gp2tq6nsw6ngwhkw72c3z17fkb5t",
            "packageSize":{"unzippedSizeBytes":{"$integer":"SEUHAAAAAAA="},"zippedSizeBytes":{"$integer":"QZgBAAAAAAA="}},
            "sha256":{"$bytes":"7WWCt6Y52N/xQ2e7Tidc6ZPAx6KAUosaxVcVcq5dbWk="},
            "storageKey":"fcf904d2-566c-41fc-a899-870b6a66b274"
        }"#).unwrap();
        let parsed = SourcePackage::try_from(value).unwrap();
        assert_eq!(
            parsed,
            SourcePackage {
                storage_key: "fcf904d2-566c-41fc-a899-870b6a66b274".try_into().unwrap(),
                sha256: Sha256Digest::from(*b"\xed\x65\x82\xb7\xa6\x39\xd8\xdf\xf1\x43\x67\xbb\x4e\x27\x5c\xe9\x93\xc0\xc7\xa2\x80\x52\x8b\x1a\xc5\x57\x15\x72\xae\x5d\x6d\x69"),
                external_deps_package_id: Some(ExternalDepsPackageId::from(
                    "k423gp2tq6nsw6ngwhkw72c3z17fkb5t"
                        .parse::<DeveloperDocumentId>()
                        .unwrap()
                )),
                package_size: PackageSize {
                    zipped_size_bytes: 104513,
                    unzipped_size_bytes: 476488,
                },
                node_version: None,
            }
        );
    }
}

use std::{
    collections::BTreeMap,
    fmt::Formatter,
};

use common::document::ParsedDocument;
use value::{
    id_v6::DeveloperDocumentId,
    obj,
    sha256::Sha256Digest,
    ConvexObject,
    ConvexValue,
    FieldName,
};

use crate::{
    external_packages::types::ExternalDepsPackageId,
    source_packages::types::{
        SourcePackage,
        SourcePackageId,
    },
};

#[cfg_attr(any(test, feature = "testing"), derive(proptest_derive::Arbitrary))]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AwsLambdaConfig {
    pub env: BTreeMap<FieldName, String>, // Env to pass to node
    pub runtime: String,                  // Eg NodeJs16
    pub handler: String,                  // function to call within the source
    pub memory_size_mb: i32,
    pub disk_size_mb: i32,
    pub timeout_sec: i32,
    pub vpc_subnet_ids: Vec<String>,
    pub vpc_security_group_ids: Vec<String>,
}

#[cfg_attr(any(test, feature = "testing"), derive(proptest_derive::Arbitrary))]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AwsLambdaVersion {
    pub lambda_name: String,
    pub lambda_version: String, // Returned from CreateFunction as a string
    pub node_executor_sha256: Sha256Digest, // hash of our node-executor code
    pub lambda_config: AwsLambdaConfig,
    pub package_desc: AwsLambdaPackageDesc,
}

/// Stores the configuration information for the Lambda relevant to the type
/// of Lambda this enum identifies.
#[cfg_attr(any(test, feature = "testing"), derive(proptest_derive::Arbitrary))]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AwsLambdaPackageDesc {
    Static {
        source_package_id: Option<SourcePackageId>,
    },
    Dynamic {
        external_deps_package_id: Option<ExternalDepsPackageId>,
    },
}

#[derive(Clone, Copy, Debug)]
pub enum AwsLambdaType {
    Static,
    Dynamic,
}

impl AwsLambdaType {
    /// Given a source package of code to deploy and an optional fallback
    /// Lambda version, builds an AwsLambdaPackageDesc
    pub fn get_package_desc(
        &self,
        deployed_code: Option<&ParsedDocument<SourcePackage>>,
    ) -> anyhow::Result<AwsLambdaPackageDesc> {
        match self {
            Self::Static => {
                let source_package_id = deployed_code
                    .map(|source_package| DeveloperDocumentId::from(source_package.id()).into());
                Ok(AwsLambdaPackageDesc::Static { source_package_id })
            },
            Self::Dynamic => {
                let external_deps_package_id = deployed_code
                    .map(|source_package| source_package.external_deps_package_id.clone())
                    .unwrap_or(None);
                Ok(AwsLambdaPackageDesc::Dynamic {
                    external_deps_package_id,
                })
            },
        }
    }
}

impl std::fmt::Display for AwsLambdaType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Static => write!(f, "Static"),
            Self::Dynamic => write!(f, "Dynamic"),
        }
    }
}

impl AwsLambdaPackageDesc {
    pub fn as_static(&self) -> anyhow::Result<&Option<SourcePackageId>> {
        let AwsLambdaPackageDesc::Static { source_package_id } = self else {
            anyhow::bail!("Expected AwsLambdaType::Static, but got {:?}", self);
        };
        Ok(source_package_id)
    }

    pub fn as_dynamic(&self) -> anyhow::Result<&Option<ExternalDepsPackageId>> {
        let AwsLambdaPackageDesc::Dynamic {
            external_deps_package_id,
        } = self
        else {
            anyhow::bail!("Expected AwsLambdaType::Static, but got {:?}", self);
        };
        Ok(external_deps_package_id)
    }
}

impl TryFrom<AwsLambdaPackageDesc> for ConvexObject {
    type Error = anyhow::Error;

    fn try_from(value: AwsLambdaPackageDesc) -> Result<Self, Self::Error> {
        match value {
            AwsLambdaPackageDesc::Static { source_package_id } => {
                obj!(
                    "type" => "static",
                    "sourcePackageId" => match source_package_id {
                        None => ConvexValue::Null,
                        Some(source_package_id) => source_package_id.into(),
                    }
                )
            },
            AwsLambdaPackageDesc::Dynamic {
                external_deps_package_id,
            } => {
                obj!(
                    "type" => "dynamic",
                    "externalDepsPackageId" => match external_deps_package_id {
                        None => ConvexValue::Null,
                        Some(external_deps_package_id) => external_deps_package_id.into(),
                    }
                )
            },
        }
    }
}

impl TryFrom<ConvexObject> for AwsLambdaPackageDesc {
    type Error = anyhow::Error;

    fn try_from(obj: ConvexObject) -> Result<Self, Self::Error> {
        let mut fields = BTreeMap::from(obj);

        let lambda_type: String = match fields.remove("type") {
            Some(ConvexValue::String(s)) => s.into(),
            _ => anyhow::bail!("Missing or invalid 'type' field in AwsLambdaType: {fields:?}"),
        };

        match lambda_type.as_str() {
            "static" => {
                let source_package_id: Option<SourcePackageId> =
                    match fields.remove("sourcePackageId") {
                        Some(ConvexValue::Null) => None,
                        Some(value) => Some(value.try_into()?),
                        None => anyhow::bail!("Missing 'sourcePackageId' in {fields:?}"),
                    };
                Ok(AwsLambdaPackageDesc::Static { source_package_id })
            },
            "dynamic" => {
                let external_deps_package_id: Option<ExternalDepsPackageId> =
                    match fields.remove("externalDepsPackageId") {
                        Some(ConvexValue::Null) => None,
                        Some(value) => Some(value.try_into()?),
                        None => anyhow::bail!("Missing 'externalDepsPackageId' in {fields:?}"),
                    };
                Ok(AwsLambdaPackageDesc::Dynamic {
                    external_deps_package_id,
                })
            },
            _ => anyhow::bail!("Unknown AwsLambdaType {lambda_type:}: {fields:?}"),
        }
    }
}

impl TryFrom<AwsLambdaConfig> for ConvexObject {
    type Error = anyhow::Error;

    fn try_from(
        AwsLambdaConfig {
            env,
            runtime,
            handler,
            memory_size_mb,
            disk_size_mb,
            timeout_sec,
            vpc_subnet_ids,
            vpc_security_group_ids,
        }: AwsLambdaConfig,
    ) -> Result<Self, Self::Error> {
        let env: anyhow::Result<BTreeMap<FieldName, ConvexValue>> = env
            .into_iter()
            .map(|(k, v)| Ok((k, v.try_into()?)))
            .collect();
        let vpc_subnet_ids = vpc_subnet_ids
            .into_iter()
            .map(ConvexValue::try_from)
            .collect::<Result<Vec<ConvexValue>, _>>()?;
        let vpc_security_group_ids = vpc_security_group_ids
            .into_iter()
            .map(ConvexValue::try_from)
            .collect::<Result<Vec<ConvexValue>, _>>()?;
        obj!(
            "env" => env?,
            "runtime" => runtime,
            "handler" => handler,
            "memorySizeMb" => (memory_size_mb as i64),
            "diskSizeMb" => (disk_size_mb as i64),
            "timeoutSec" => (timeout_sec as i64),
            "subnetIds" => vpc_subnet_ids,
            "securityGroupIds" => vpc_security_group_ids
        )
    }
}

impl TryFrom<AwsLambdaVersion> for ConvexObject {
    type Error = anyhow::Error;

    fn try_from(
        AwsLambdaVersion {
            lambda_name,
            lambda_version,
            node_executor_sha256,
            lambda_config,
            package_desc,
        }: AwsLambdaVersion,
    ) -> Result<Self, Self::Error> {
        obj!(
            "lambdaName" => lambda_name,
            "lambdaVersion" => lambda_version,
            // Double-write this field for backwards compatability
            "sourcePackageId" => match &package_desc {
                AwsLambdaPackageDesc::Static {
                    source_package_id: Some(id)
                } => (*id).into(),
                _ => ConvexValue::Null,
            },
            "nodeExecutorSha256" => node_executor_sha256,
            "lambdaConfig" => ConvexValue::Object(lambda_config.try_into()?),
            "typeConfig" => ConvexValue::Object(package_desc.try_into()?),
        )
    }
}

impl TryFrom<ConvexObject> for AwsLambdaConfig {
    type Error = anyhow::Error;

    fn try_from(value: ConvexObject) -> Result<Self, Self::Error> {
        let mut object_fields: BTreeMap<_, _> = value.into();
        let env: anyhow::Result<_> = match object_fields.remove("env") {
            Some(ConvexValue::Object(env)) => env
                .into_iter()
                .map(|(k, v)| Ok((k, v.try_into()?)))
                .collect(),
            _ => anyhow::bail!("Missing 'env' in {object_fields:?}"),
        };
        let runtime = match object_fields.remove("runtime") {
            Some(runtime) => runtime.try_into()?,
            _ => anyhow::bail!("Missing 'runtime' in {object_fields:?}"),
        };
        let handler = match object_fields.remove("handler") {
            Some(handler) => handler.try_into()?,
            _ => anyhow::bail!("Missing 'handler' in {object_fields:?}"),
        };
        let memory_size_mb = match object_fields.remove("memorySizeMb") {
            Some(ConvexValue::Int64(memory_size_mb)) => memory_size_mb.try_into()?,
            _ => anyhow::bail!("Missing 'memorySizeMb' in {object_fields:?}"),
        };
        let disk_size_mb = match object_fields.remove("diskSizeMb") {
            Some(ConvexValue::Int64(disk_size_mb)) => disk_size_mb.try_into()?,
            // Old AwsLambdaConfigs do not have this so just use a sensible default of 512 MB
            None => 512i32,
            _ => anyhow::bail!("Invalid 'diskSizeMb' in {object_fields:?}"),
        };
        let timeout_sec = match object_fields.remove("timeoutSec") {
            Some(ConvexValue::Int64(timeout_sec)) => timeout_sec.try_into()?,
            _ => anyhow::bail!("Missing 'timeoutSec' in {object_fields:?}"),
        };
        let vpc_subnet_ids = match object_fields.remove("subnetIds") {
            Some(ConvexValue::Array(arr)) => arr
                .into_iter()
                .map(String::try_from)
                .collect::<Result<Vec<String>, _>>()?,
            None => vec![],
            _ => anyhow::bail!("Invalid 'subnetIds' in {object_fields:?}"),
        };
        let vpc_security_group_ids = match object_fields.remove("securityGroupIds") {
            Some(ConvexValue::Array(arr)) => arr
                .into_iter()
                .map(String::try_from)
                .collect::<Result<Vec<String>, _>>()?,
            None => vec![],
            _ => anyhow::bail!("Invalid 'securityGroupIds' in {object_fields:?}"),
        };
        Ok(Self {
            env: env?,
            runtime,
            handler,
            memory_size_mb,
            disk_size_mb,
            timeout_sec,
            vpc_subnet_ids,
            vpc_security_group_ids,
        })
    }
}

impl TryFrom<ConvexObject> for AwsLambdaVersion {
    type Error = anyhow::Error;

    fn try_from(value: ConvexObject) -> Result<Self, Self::Error> {
        let mut object_fields: BTreeMap<_, _> = value.into();
        let lambda_name = match object_fields.remove("lambdaName") {
            Some(ConvexValue::String(key)) => key.into(),
            _ => anyhow::bail!("Missing 'lambdaName' in {object_fields:?}"),
        };
        let lambda_version = match object_fields.remove("lambdaVersion") {
            Some(lambda_version) => lambda_version.try_into()?,
            _ => anyhow::bail!("Missing 'lambdaVersion' in {object_fields:?}"),
        };
        let node_executor_sha256 = match object_fields.remove("nodeExecutorSha256") {
            Some(node_executor_sha256) => node_executor_sha256.try_into()?,
            _ => anyhow::bail!("Missing 'nodeExecutorSha256' in {object_fields:?}"),
        };
        let lambda_config = match object_fields.remove("lambdaConfig") {
            Some(ConvexValue::Object(lambda_config)) => lambda_config.try_into()?,
            _ => anyhow::bail!("Missing 'lambdaConfig' in {object_fields:?}"),
        };

        // Deprecated, reading this field for now to populate package_desc on old lambda
        // versions
        let source_package_id = match object_fields.remove("sourcePackageId") {
            None | Some(ConvexValue::Null) => None,
            Some(value) => Some(value.try_into()?),
        };
        let package_desc = match object_fields.remove("typeConfig") {
            Some(ConvexValue::Object(package_desc)) => package_desc.try_into()?,
            // This must be an old-style Lambda so this must be a static lambda and we should read
            // the sourcePackageId field above.
            None => AwsLambdaPackageDesc::Static { source_package_id },
            _ => anyhow::bail!("Invalid 'typeConfig' in {object_fields:?}"),
        };

        Ok(Self {
            lambda_name,
            lambda_version,
            node_executor_sha256,
            lambda_config,
            package_desc,
        })
    }
}

#[cfg(test)]
mod tests {
    use cmd_util::env::env_config;
    use common::testing::assert_roundtrips;
    use proptest::prelude::*;
    use value::ConvexObject;

    use super::{
        AwsLambdaConfig,
        AwsLambdaVersion,
    };

    proptest! {
        #![proptest_config(
            ProptestConfig { cases: 256 * env_config("CONVEX_PROPTEST_MULTIPLIER", 1), failure_persistence: None, ..ProptestConfig::default() }
        )]

        #[test]
        fn test_actions_version_roundtrip(v in any::<AwsLambdaVersion>()) {
            assert_roundtrips::<AwsLambdaVersion, ConvexObject>(v);
        }

        #[test]
        fn test_lambda_config_roundtrip(v in any::<AwsLambdaConfig>()) {
            assert_roundtrips::<AwsLambdaConfig, ConvexObject>(v);
        }
    }
}

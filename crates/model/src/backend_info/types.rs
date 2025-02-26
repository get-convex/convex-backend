use std::collections::BTreeMap;

use common::{
    obj,
    types::{
        BackendInfo,
        DeploymentId,
        DeploymentType,
        ProjectId,
        TeamId,
        DEFAULT_PROVISION_CONCURRENCY,
    },
};
use value::{
    ConvexObject,
    ConvexValue,
};

/// Information and configuration about the backend.
#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg_attr(any(test, feature = "testing"), derive(proptest_derive::Arbitrary))]
pub struct BackendInfoPersisted {
    pub team: TeamId,
    pub project: ProjectId,
    pub deployment: DeploymentId,
    pub deployment_type: DeploymentType,

    // Entitlements
    pub streaming_export_enabled: bool,
    pub provision_concurrency: i32,
    pub log_streaming_enabled: bool,
}

impl From<BackendInfoPersisted> for BackendInfo {
    fn from(bi: BackendInfoPersisted) -> BackendInfo {
        Self {
            team_id: bi.team,
            project_id: bi.project,
            deployment_id: bi.deployment,
            deployment_type: bi.deployment_type,
            streaming_export_enabled: Some(bi.streaming_export_enabled),
            provision_concurrency: Some(bi.provision_concurrency),
            log_streaming_enabled: Some(bi.log_streaming_enabled),
        }
    }
}

impl From<BackendInfo> for BackendInfoPersisted {
    fn from(bi: BackendInfo) -> BackendInfoPersisted {
        Self {
            team: bi.team_id,
            project: bi.project_id,
            deployment: bi.deployment_id,
            streaming_export_enabled: bi.streaming_export_enabled.unwrap_or_default(),
            deployment_type: bi.deployment_type,
            provision_concurrency: bi
                .provision_concurrency
                .unwrap_or(DEFAULT_PROVISION_CONCURRENCY),
            log_streaming_enabled: bi.log_streaming_enabled.unwrap_or_default(),
        }
    }
}

#[cfg(any(test, feature = "testing"))]
impl Default for BackendInfoPersisted {
    fn default() -> Self {
        BackendInfo::default().into()
    }
}

impl TryFrom<BackendInfoPersisted> for ConvexObject {
    type Error = anyhow::Error;

    fn try_from(b: BackendInfoPersisted) -> anyhow::Result<Self> {
        let team: u64 = b.team.into();
        let project: u64 = b.project.into();
        let deployment: u64 = b.deployment.into();
        let deployment_type: String = b.deployment_type.to_string();

        obj!(
            "org" => (team as i64),
            "project" => (project as i64),
            "instance" => (deployment as i64),
            "deploymentType" => deployment_type,
            "streamingExportEnabled" => b.streaming_export_enabled,
            "provisionConcurrency" => (b.provision_concurrency as i64),
            "logStreamingEnabled" => b.log_streaming_enabled,
        )
    }
}

impl TryFrom<ConvexObject> for BackendInfoPersisted {
    type Error = anyhow::Error;

    fn try_from(o: ConvexObject) -> Result<Self, Self::Error> {
        let mut object_fields: BTreeMap<_, _> = o.into();
        let team: TeamId = match object_fields.remove("org") {
            Some(ConvexValue::Int64(i)) => TeamId(i as u64),
            _ => anyhow::bail!(
                "Missing or invalid team for BackendInfoPersisted: {:?}",
                object_fields
            ),
        };
        let project: ProjectId = match object_fields.remove("project") {
            Some(ConvexValue::Int64(i)) => ProjectId(i as u64),
            _ => anyhow::bail!(
                "Missing or invalid project for BackendInfoPersisted: {:?}",
                object_fields
            ),
        };
        let deployment: DeploymentId = match object_fields.remove("instance") {
            Some(ConvexValue::Int64(i)) => DeploymentId(i as u64),
            _ => anyhow::bail!(
                "Missing or invalid deployment for BackendInfoPersisted: {:?}",
                object_fields
            ),
        };
        let deployment_type: DeploymentType = match object_fields.remove("deploymentType") {
            Some(ConvexValue::String(s)) => String::from(s).parse()?,
            dt => anyhow::bail!(
                "Missing or invalid deployment type for BackendInfoPersisted: {:?}, {dt:?}",
                object_fields
            ),
        };
        let streaming_export_enabled = matches!(
            object_fields.remove("streamingExportEnabled"),
            Some(ConvexValue::Boolean(true))
        );
        let provision_concurrency = match object_fields.remove("provisionConcurrency") {
            Some(ConvexValue::Int64(i)) => i as i32,
            _ => DEFAULT_PROVISION_CONCURRENCY,
        };
        let log_streaming_enabled = matches!(
            object_fields.remove("logStreamingEnabled"),
            Some(ConvexValue::Boolean(true))
        );
        Ok(Self {
            team,
            project,
            deployment,
            deployment_type,
            streaming_export_enabled,
            provision_concurrency,
            log_streaming_enabled,
        })
    }
}

#[cfg(test)]
mod tests {
    use cmd_util::env::env_config;
    use common::testing::assert_roundtrips;
    use proptest::prelude::*;
    use value::ConvexObject;

    use super::BackendInfoPersisted;

    proptest! {
        #![proptest_config(
            ProptestConfig { cases: 256 * env_config("CONVEX_PROPTEST_MULTIPLIER", 1), failure_persistence: None, ..ProptestConfig::default() }
        )]
        #[test]
        fn test_backend_info_roundtrips(v in any::<BackendInfoPersisted>()) {
            assert_roundtrips::<BackendInfoPersisted, ConvexObject>(v);
        }
    }
}

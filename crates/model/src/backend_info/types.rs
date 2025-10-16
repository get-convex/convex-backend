use common::types::{
    BackendInfo,
    DeploymentId,
    DeploymentType,
    ProjectId,
    TeamId,
    DEFAULT_PROVISION_CONCURRENCY,
};
use serde::{
    Deserialize,
    Serialize,
};
use value::codegen_convex_serialization;

/// Information and configuration about the backend.
#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg_attr(any(test, feature = "testing"), derive(proptest_derive::Arbitrary))]
pub struct BackendInfoPersisted {
    pub team: TeamId,
    pub project: ProjectId,
    pub deployment: DeploymentId,
    pub deployment_type: DeploymentType,
    pub project_name: Option<String>,
    pub project_slug: Option<String>,

    // Entitlements
    pub streaming_export_enabled: bool,
    pub provision_concurrency: i32,
    pub log_streaming_enabled: bool,
    pub audit_log_retention_days: u64,
}

impl From<BackendInfoPersisted> for BackendInfo {
    fn from(bi: BackendInfoPersisted) -> BackendInfo {
        Self {
            team_id: bi.team,
            project_id: bi.project,
            deployment_id: bi.deployment,
            deployment_type: bi.deployment_type,
            project_name: bi.project_name,
            project_slug: bi.project_slug,
            streaming_export_enabled: Some(bi.streaming_export_enabled),
            provision_concurrency: Some(bi.provision_concurrency),
            log_streaming_enabled: Some(bi.log_streaming_enabled),
            audit_log_retention_days: Some(bi.audit_log_retention_days),
        }
    }
}

impl From<BackendInfo> for BackendInfoPersisted {
    fn from(bi: BackendInfo) -> BackendInfoPersisted {
        Self {
            team: bi.team_id,
            project: bi.project_id,
            deployment: bi.deployment_id,
            project_name: bi.project_name,
            project_slug: bi.project_slug,
            streaming_export_enabled: bi.streaming_export_enabled.unwrap_or_default(),
            deployment_type: bi.deployment_type,
            provision_concurrency: bi
                .provision_concurrency
                .unwrap_or(DEFAULT_PROVISION_CONCURRENCY),
            log_streaming_enabled: bi.log_streaming_enabled.unwrap_or_default(),
            audit_log_retention_days: bi.audit_log_retention_days.unwrap_or_default(),
        }
    }
}

#[cfg(any(test, feature = "testing"))]
impl Default for BackendInfoPersisted {
    fn default() -> Self {
        BackendInfo::default().into()
    }
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SerializedBackendInfo {
    org: i64,
    project: i64,
    instance: i64,
    deployment_type: String,
    #[serde(default)]
    streaming_export_enabled: bool,
    #[serde(default)]
    provision_concurrency: Option<i64>,
    #[serde(default)]
    log_streaming_enabled: bool,
    project_name: Option<String>,
    project_slug: Option<String>,
    #[serde(default)]
    audit_log_retention_days: i64,
}

impl From<BackendInfoPersisted> for SerializedBackendInfo {
    fn from(b: BackendInfoPersisted) -> Self {
        let team: u64 = b.team.into();
        let project: u64 = b.project.into();
        let deployment: u64 = b.deployment.into();
        let deployment_type: String = b.deployment_type.to_string();

        SerializedBackendInfo {
            org: (team as i64),
            project: (project as i64),
            instance: (deployment as i64),
            deployment_type,
            streaming_export_enabled: b.streaming_export_enabled,
            provision_concurrency: Some(b.provision_concurrency as i64),
            log_streaming_enabled: b.log_streaming_enabled,
            project_name: b.project_name,
            project_slug: b.project_slug,
            audit_log_retention_days: (b.audit_log_retention_days as i64),
        }
    }
}

impl TryFrom<SerializedBackendInfo> for BackendInfoPersisted {
    type Error = anyhow::Error;

    fn try_from(o: SerializedBackendInfo) -> Result<Self, Self::Error> {
        let team = TeamId(o.org as u64);
        let project = ProjectId(o.project as u64);
        let deployment = DeploymentId(o.instance as u64);
        let deployment_type: DeploymentType = o.deployment_type.parse()?;
        let streaming_export_enabled = o.streaming_export_enabled;
        let provision_concurrency = o
            .provision_concurrency
            .map_or(DEFAULT_PROVISION_CONCURRENCY, |c| c as i32);
        let log_streaming_enabled = o.log_streaming_enabled;
        let project_name = o.project_name;
        let project_slug = o.project_slug;
        let audit_log_retention_days = o.audit_log_retention_days as u64;

        Ok(Self {
            team,
            project,
            deployment,
            deployment_type,
            project_name,
            project_slug,
            streaming_export_enabled,
            provision_concurrency,
            log_streaming_enabled,
            audit_log_retention_days,
        })
    }
}

codegen_convex_serialization!(BackendInfoPersisted, SerializedBackendInfo);

#[cfg(test)]
mod tests {
    use common::types::{
        DeploymentId,
        DeploymentType,
        ProjectId,
        TeamId,
    };
    use value::assert_obj;

    use super::BackendInfoPersisted;

    #[test]
    fn test_frozen_obj() {
        assert_eq!(
            BackendInfoPersisted::try_from(assert_obj! {
                "deploymentType" => "prod",
                "instance" => 1926612683017131100i64,
                "logStreamingEnabled" => true,
                "org" => -667731666772323580i64,
                "project" => 6688466498098154475i64,
                "projectName" => null,
                "projectSlug" => "ayaya",
                "provisionConcurrency" => 1740011963i64,
                "streamingExportEnabled" => false,
            })
            .unwrap(),
            BackendInfoPersisted {
                team: TeamId(17779012406937228036),
                project: ProjectId(6688466498098154475),
                deployment: DeploymentId(1926612683017131100),
                deployment_type: DeploymentType::Prod,
                project_name: None,
                project_slug: Some("ayaya".to_string()),
                streaming_export_enabled: false,
                provision_concurrency: 1740011963,
                log_streaming_enabled: true,
                audit_log_retention_days: 0,
            }
        );
    }
}

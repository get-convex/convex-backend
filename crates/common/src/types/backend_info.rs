use serde::{
    Deserialize,
    Serialize,
};

use crate::types::{
    DeploymentId,
    DeploymentType,
    ProjectId,
    TeamId,
};

pub const DEFAULT_PROVISION_CONCURRENCY: i32 = 0;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct BackendInfo {
    pub team_id: TeamId,
    pub project_id: ProjectId,
    pub project_name: Option<String>,
    pub project_slug: Option<String>,

    pub deployment_id: DeploymentId,
    pub deployment_type: DeploymentType,

    // If None, the value is overwritten with the default value.
    pub streaming_export_enabled: Option<bool>,
    pub provision_concurrency: Option<i32>,
    pub log_streaming_enabled: Option<bool>,
}

#[cfg(any(test, feature = "testing"))]
impl Default for BackendInfo {
    fn default() -> Self {
        Self {
            team_id: TeamId(4),
            project_id: ProjectId(17),
            deployment_id: DeploymentId(2021),
            deployment_type: DeploymentType::Dev,
            project_name: Some("Default Project".to_string()),
            project_slug: Some("default-project".to_string()),
            streaming_export_enabled: Some(false),
            provision_concurrency: Some(DEFAULT_PROVISION_CONCURRENCY),
            log_streaming_enabled: Some(false),
        }
    }
}

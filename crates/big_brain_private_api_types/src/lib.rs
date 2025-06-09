#![feature(impl_trait_in_assoc_type)]
use std::{
    fmt::Display,
    str::FromStr,
};

use common::types::{
    DeploymentId,
    DeploymentType,
    ProjectId,
    TeamId,
};
use keybroker::AdminIdentityPrincipal;
use serde::{
    Deserialize,
    Serialize,
};
use utoipa::ToSchema;

pub use crate::types::{
    CloudBackupId,
    PartitionId,
    PlanId,
    ProjectName,
    ProjectSlug,
    SubscriptionId,
    TeamName,
    TeamSlug,
};

pub mod types;

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
#[serde(deny_unknown_fields)]
pub struct DeploymentAuthArgs {
    project_slug: ProjectSlug,
    team_slug: TeamSlug,
    deployment_type: DeploymentType,
}

impl DeploymentAuthArgs {
    pub fn new(
        project_slug: ProjectSlug,
        team_slug: TeamSlug,
        deployment_type: DeploymentType,
    ) -> Self {
        Self {
            project_slug,
            team_slug,
            deployment_type,
        }
    }

    pub fn project_slug(&self) -> &ProjectSlug {
        &self.project_slug
    }

    pub fn team_slug(&self) -> &TeamSlug {
        &self.team_slug
    }

    pub fn deployment_type(&self) -> &DeploymentType {
        &self.deployment_type
    }
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
#[serde(deny_unknown_fields)]
pub struct DeploymentAuthProdArgs {
    pub deployment_name: String,
    pub partition_id: Option<PartitionId>,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
#[serde(deny_unknown_fields)]
pub struct DeploymentAuthArgsSimple {
    pub deployment_name: String,
    #[serde(default)]
    pub deployment_type: Option<DeploymentType>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeploymentAuthResponse {
    pub deployment_name: String,
    pub admin_key: String,
    pub url: String,
    pub deployment_type: DeploymentType,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AccessTokenDeploymentAuthArgs {
    pub access_token: String,
    pub deployment_name: String,
    #[serde(default)]
    pub deployment_type: Option<DeploymentType>,
    pub action: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct AccessTokenDeploymentAuthResponse {
    pub is_authorized: bool,
    pub authorized_entity: Option<AdminIdentityPrincipal>,
    pub is_read_only: Option<bool>,
}

#[derive(Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
#[serde(deny_unknown_fields)]
#[serde(tag = "kind")]
pub enum ProjectSelectionArgs {
    /// CONVEX_DEPLOYMENT, pointing to dev or prod.
    #[serde(rename_all = "camelCase")]
    DeploymentName {
        deployment_name: String,
        deployment_type: Option<DeploymentType>,
    },

    /// If there is no CONVEX_DEPLOYMENT, select a project with team and
    /// project.
    #[serde(rename_all = "camelCase")]
    TeamAndProjectSlugs {
        team_slug: String,
        project_slug: String,
    },
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
#[serde(deny_unknown_fields)]
pub struct DeploymentAuthPreviewArgs {
    pub project_selection: ProjectSelectionArgs,
    /// --preview-name instance selector.
    pub preview_name: String,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
#[serde(deny_unknown_fields)]
pub struct DeploymentAuthWithinCurrentProjectArgs {
    pub project_selection: ProjectSelectionArgs,
    /// --deployment-name instance selector.
    pub selected_deployment_name: String,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TeamAndProjectForDeploymentResponse {
    pub team: TeamSlug,
    pub project: ProjectSlug,
    pub team_id: TeamId,
    pub project_id: ProjectId,
    pub deployment_id: Option<DeploymentId>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, ToSchema)]
pub struct PreviewDeploymentIdentifier(String);

impl PreviewDeploymentIdentifier {
    pub fn to_deploy_key_prefix(&self) -> String {
        // Replace `:` and `|` with `_` since our deploy keys look like
        // `preview:branch-name|<rest of key>`
        let mut modified_identifier = self.0.replace([':', '|'], "_");
        modified_identifier.truncate(40);
        modified_identifier
    }
}

impl FromStr for PreviewDeploymentIdentifier {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(PreviewDeploymentIdentifier(s.to_string()))
    }
}

impl Display for PreviewDeploymentIdentifier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl PreviewDeploymentIdentifier {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

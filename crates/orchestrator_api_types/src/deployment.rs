//! Deployment-internal credential exchange types.
//!
//! These types match the wire format used by `crates/big_brain_client`
//! and the CLI's `bigBrainAPI` calls under `/api/deployment/*`.

pub use big_brain_private_api_types::{
    AccessTokenDeploymentAuthArgs,
    AccessTokenDeploymentAuthResponse,
    DeploymentAuthArgs,
    DeploymentAuthArgsSimple,
    DeploymentAuthPreviewArgs,
    DeploymentAuthProdArgs,
    DeploymentAuthResponse,
    DeploymentAuthWithinCurrentProjectArgs,
    PreviewDeploymentIdentifier,
    ProjectSelectionArgs,
    TeamAndProjectForDeploymentResponse,
};
use serde::{
    Deserialize,
    Serialize,
};

/// Body of `POST /api/deployment/team_and_project_for_key`.
#[derive(Debug, Clone, Deserialize, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct TeamAndProjectForKeyArgs {
    pub deploy_key: String,
}

/// Body of `POST /api/deployment/url_for_key`.
#[derive(Debug, Clone, Deserialize, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UrlForKeyArgs {
    pub deploy_key: String,
}

/// Response shape for `POST /api/deployment/url_for_key`.
#[derive(Debug, Clone, Deserialize, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UrlForKeyResponse {
    pub url: String,
    pub deployment_name: String,
}

/// Body of `POST /api/deployment/provision_and_authorize`.
#[derive(Debug, Clone, Deserialize, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ProvisionAndAuthorizeArgs {
    pub team_slug: Option<String>,
    pub project_slug: Option<String>,
    pub deployment_type: String,
}

/// Body of `POST /api/claim_preview_deployment`.
#[derive(Deserialize, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ClaimPreviewDeploymentArgs {
    pub project_selection: ProjectSelectionArgs,
    pub identifier: String,
}

/// Response shape for `POST /api/claim_preview_deployment`.
#[derive(Debug, Clone, Deserialize, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ClaimPreviewDeploymentResponse {
    pub deployment_name: String,
    pub admin_key: String,
    pub url: String,
    pub deployment_type: String,
}

/// Response shape for `GET /api/has_projects` — single boolean.
#[derive(Debug, Clone, Deserialize, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct HasProjectsResponse {
    pub has_projects: bool,
}

/// Response shape for `GET /api/teams` (CLI-internal endpoint).
#[derive(Debug, Clone, Deserialize, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct TeamSummary {
    pub id: u64,
    pub name: String,
    pub slug: String,
}

/// Body of `POST /api/create_project` (CLI).
#[derive(Debug, Clone, Deserialize, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateProjectArgs {
    pub team: String,
    pub project_name: String,
    #[serde(default)]
    pub deployment_type: Option<String>,
    #[serde(default)]
    pub region: Option<String>,
}

/// Response shape for `POST /api/create_project` (CLI).
#[derive(Debug, Clone, Deserialize, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateProjectResponse {
    pub project_id: u64,
    pub project_slug: String,
    pub team_slug: String,
    #[serde(default)]
    pub deployment_name: Option<String>,
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub admin_key: Option<String>,
}

//! Types served under `/v1/*`, the public Convex Management API.
//!
//! Mirrors the schema generated into
//! `npm-packages/@convex-dev/platform/src/generatedManagementApi.ts`.

use serde::{
    Deserialize,
    Serialize,
};

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PlatformProjectDetails {
    pub id: u64,
    pub team_id: u64,
    pub name: String,
    pub slug: String,
    pub is_demo: bool,
    pub creation_time: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PlatformCreateProjectArgs {
    pub project_name: String,
    #[serde(default)]
    pub slug: Option<String>,
    #[serde(default)]
    pub deployment_type: Option<String>,
    #[serde(default)]
    pub region: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PlatformCreateProjectResponse {
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

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PlatformDeploymentResponse {
    pub id: u64,
    pub project_id: u64,
    pub name: String,
    pub kind: String,
    pub deployment_class: String,
    pub url: String,
    pub site_url: String,
    pub state: String,
    pub creation_time: f64,
    pub region: Option<String>,
    pub preview_identifier: Option<String>,
    /// Resource tier (S4/S8/S16/S32/S64/S128/S256/max) snapshotted on the
    /// deployment row at provision time. Surfaced so the dashboard can
    /// display the tier badge without an extra request to settings.
    #[serde(default)]
    pub tier: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PlatformCreateDeploymentArgs {
    pub kind: String,
    #[serde(default)]
    pub deployment_class: Option<String>,
    #[serde(default)]
    pub region: Option<String>,
    #[serde(default)]
    pub preview_identifier: Option<String>,
    /// Overrides the project's default tier for this deployment only.
    #[serde(default)]
    pub tier: Option<String>,
    /// Per-deployment knob overrides layered on top of the project's
    /// `knob_overrides`. Empty omitted.
    #[serde(default)]
    pub knob_overrides: Option<std::collections::BTreeMap<String, String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PlatformUpdateDeploymentArgs {
    #[serde(default)]
    pub deployment_class: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PlatformTransferDeploymentArgs {
    pub project_id: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PaginatedDeploymentsResponse {
    pub deployments: Vec<PlatformDeploymentResponse>,
    pub cursor: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ListLocalDeploymentsResponse {
    pub deployments: Vec<PlatformDeploymentResponse>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct DeploymentClass {
    pub name: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ListDeploymentClassesResponse {
    pub classes: Vec<DeploymentClass>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct DeploymentRegion {
    pub name: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ListDeploymentRegionsResponse {
    pub regions: Vec<DeploymentRegion>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PlatformCreateDeployKeyArgs {
    pub name: String,
    #[serde(default)]
    pub expires_at: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PlatformCreateDeployKeyResponse {
    pub key: String,
    pub id: String,
    pub name: String,
    pub creation_time: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PlatformDeployKeyResponse {
    pub id: String,
    pub name: String,
    pub creation_time: f64,
    pub key_suffix: String,
    /// Milliseconds since epoch; `None` for keys that never expire.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PlatformDeleteDeployKeyArgs {
    pub id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PlatformCreatePreviewDeployKeyArgs {
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PlatformCreatePreviewDeployKeyResponse {
    pub key: String,
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PlatformListPreviewDeployKeysResponse {
    pub keys: Vec<PlatformDeployKeyResponse>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PlatformDeletePreviewDeployKeyArgs {
    pub id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreatePersonalAccessTokenArgs {
    pub name: String,
    #[serde(default)]
    pub expires_at: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreatePersonalAccessTokenResponse {
    pub access_token: String,
    pub id: String,
    pub name: String,
    pub creation_time: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct DeletePersonalAccessTokenArgs {
    pub id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PaginatedPersonalAccessTokensResponse {
    pub tokens: Vec<PlatformDeployKeyResponse>,
    pub cursor: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PlatformTokenDetailsResponse {
    pub kind: String,
    pub team_id: Option<u64>,
    pub project_id: Option<u64>,
    pub deployment_name: Option<String>,
    pub member_id: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PlatformCreateTeamArgs {
    pub name: String,
    #[serde(default)]
    pub slug: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PlatformTeamResponse {
    pub id: u64,
    pub name: String,
    pub slug: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PlatformListTeamMembersResponse {
    pub members: Vec<PlatformTeamMember>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PlatformTeamMember {
    pub id: u64,
    pub email: String,
    pub role: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateInvitationArgs {
    pub email: String,
    pub role: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateTeamAccessTokenResponse {
    pub access_token: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PlatformCustomDomainArgs {
    pub domain: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PlatformDeleteCustomDomainArgs {
    pub domain: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PlatformCustomDomain {
    pub domain: String,
    pub state: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PlatformListCustomDomainsResponse {
    pub domains: Vec<PlatformCustomDomain>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PlatformDefaultEnvVar {
    pub name: String,
    pub value: String,
    pub deployment_types: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PaginatedDefaultEnvironmentVariablesResponse {
    pub variables: Vec<PlatformDefaultEnvVar>,
    pub cursor: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UpdateDefaultEnvironmentVariablesArgs {
    pub variables: Vec<PlatformDefaultEnvVar>,
}

#[derive(Debug, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ProjectSettingsResponse {
    pub tier: String,
    /// Flat map from env var name to canonical string value. Empty when no
    /// overrides have been set (the deployment will use tier defaults).
    pub knob_overrides: std::collections::BTreeMap<String, String>,
}

#[derive(Debug, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UpdateProjectSettingsArgs {
    /// Set to switch tier. Omit to leave unchanged.
    pub tier: Option<String>,
    /// Replace the override map entirely. Omit to leave unchanged. A value
    /// of `null` for any individual knob clears that override (so the
    /// effective value falls back to tier default).
    pub knob_overrides: Option<std::collections::BTreeMap<String, Option<String>>>,
}

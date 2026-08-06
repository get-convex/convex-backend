//! Types served under `/api/dashboard/*`, mirroring
//! `npm-packages/dashboard/dashboard-management-openapi.json`.

use serde::{
    Deserialize,
    Serialize,
};

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct MemberResponse {
    pub id: u64,
    pub email: String,
    pub name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct MemberDataResponse {
    pub member: MemberResponse,
    pub teams: Vec<TeamResponse>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ProfileEmailResponse {
    pub id: u64,
    pub email: String,
    pub is_verified: bool,
    pub is_primary: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ProfileEmailArgs {
    pub email: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UpdateProfileNameArgs {
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct TeamResponse {
    pub id: u64,
    pub name: String,
    pub slug: String,
    pub creator: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateTeamArgs {
    pub name: String,
    #[serde(default)]
    pub slug: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UpdateTeamArgs {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub slug: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct TeamMember {
    pub id: u64,
    pub email: String,
    pub name: Option<String>,
    pub role: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct RemoveMemberArgs {
    pub member_id: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UpdateMemberRoleArgs {
    pub member_id: u64,
    pub role: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct InvitationArgs {
    pub email: String,
    pub role: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct InvitationResponse {
    pub id: u64,
    pub email: String,
    pub role: String,
    pub code: String,
    pub created_at: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ProjectResponse {
    pub id: u64,
    pub team_id: u64,
    pub name: String,
    pub slug: String,
    pub is_demo: bool,
    pub creation_time: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UpdateProjectArgs {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub slug: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct DeploymentResponse {
    pub id: u64,
    pub project_id: u64,
    pub name: String,
    pub deployment_type: String,
    pub deployment_class: String,
    pub url: String,
    pub site_url: String,
    pub state: String,
    pub creation_time: f64,
    pub region: Option<String>,
    pub preview_identifier: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct GetDeploymentAuthDashboardResponse {
    pub admin_key: String,
    pub url: String,
}

/// Body of `POST /api/dashboard/deployments/register` — used by operators in
/// `--provisioner external` mode to tell the orchestrator about a backend
/// they pre-started. The orchestrator stores the URL + an admin-key hash so
/// the dashboard and CLI can look it up later.
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct RegisterDeploymentArgs {
    pub deployment_name: String,
    pub project_id: u64,
    /// `prod`, `dev`, or `preview`.
    pub deployment_type: String,
    pub url: String,
    pub site_url: String,
    /// Full admin key the operator generated when starting their backend.
    /// Stored hashed; only the last few characters are retained in the clear
    /// (as `keySuffix`) for UI display.
    pub admin_key: String,
    #[serde(default)]
    pub region: Option<String>,
    /// Required for `preview` deployments — matches the preview branch / id.
    #[serde(default)]
    pub preview_identifier: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct OptIn {
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct GetOptInsResponse {
    pub opt_ins_to_accept: Vec<OptIn>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AccessTokenResponse {
    pub id: String,
    pub kind: String,
    pub name: String,
    pub creation_time: f64,
    pub key_suffix: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateAccessTokenArgs {
    pub name: String,
    #[serde(default)]
    pub expires_at: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateAccessTokenResponse {
    pub access_token: String,
    pub id: String,
    pub name: String,
    pub creation_time: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AuditLogEvent {
    pub id: u64,
    pub team_id: u64,
    pub member_id: Option<u64>,
    pub action: String,
    pub metadata: serde_json::Value,
    pub creation_time: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AuditLogPage {
    pub events: Vec<AuditLogEvent>,
    pub cursor: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct EnvironmentVariable {
    pub name: String,
    pub value: String,
    pub deployment_types: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ListEnvironmentVariables {
    pub variables: Vec<EnvironmentVariable>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UpdateDefaultEnvVarsArgs {
    pub variables: Vec<EnvironmentVariable>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct DeviceAuthorizeArgs {
    pub device_name: String,
    #[serde(default)]
    pub email: Option<String>,
    #[serde(default)]
    pub password: Option<String>,
    #[serde(default)]
    pub bootstrap_token: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct DeviceAuthorizeResponse {
    pub access_token: String,
    pub member_id: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CustomDomain {
    pub id: i64,
    pub deployment_id: i64,
    pub domain: String,
    /// `pending` -> `issuing` -> `active`, or `failed`. Only ever set from an
    /// observed outcome: `active` means an HTTPS request to the domain
    /// actually succeeded, not that issuance was requested.
    pub cert_state: String,
    pub created_at: i64,
    /// `http-01` (no credentials needed) or `dns-01` (needs a DNS provider
    /// credential, and is the only option for wildcards).
    pub challenge_type: String,
    pub dns_credential_id: Option<i64>,
    /// Verbatim reason the last issuance failed, if it did.
    pub last_error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct DnsProviderField {
    pub key: String,
    pub label: String,
    pub help: String,
}

/// A DNS provider the orchestrator can drive, plus the secret fields its
/// credential form needs. Sent to the dashboard so adding a provider doesn't
/// require a matching dashboard change.
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct DnsProviderInfo {
    pub provider: String,
    pub fields: Vec<DnsProviderField>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct DnsCredential {
    pub id: i64,
    pub name: String,
    pub provider: String,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ListDnsCredentials {
    pub credentials: Vec<DnsCredential>,
    pub providers: Vec<DnsProviderInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateDnsCredentialArgs {
    pub name: String,
    pub provider: String,
    /// Provider secrets keyed by the field `key`s advertised in
    /// [`DnsProviderInfo`]. Sealed before storage and never returned.
    pub secrets: std::collections::BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateCustomDomainArgs {
    pub domain: String,
    /// Defaults to `http-01`, which needs no credentials.
    #[serde(default)]
    pub challenge_type: Option<String>,
    #[serde(default)]
    pub dns_credential_id: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ListCustomDomains {
    pub domains: Vec<CustomDomain>,
    /// Providers available for dns-01, with their credential form fields.
    pub providers: Vec<DnsProviderInfo>,
    /// Hostname the operator should CNAME/A-record the custom domain at.
    /// Empty when the orchestrator has no public router host configured.
    pub target_host: String,
    /// False when the orchestrator has no Traefik dynamic directory wired up,
    /// in which case adding a domain would record a row that never routes.
    pub routing_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CustomDomainArgs {
    pub domain: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct VerifyCustomDomainResponse {
    pub domain: String,
    pub cert_state: String,
    /// Why the probe failed, when it did. Surfaced verbatim in the dashboard
    /// because the causes (DNS not pointed here, ACME rate limit) are things
    /// only the operator can fix.
    pub error: Option<String>,
}

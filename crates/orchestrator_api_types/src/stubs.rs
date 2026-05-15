//! JSON-shape stubs for hosted-only endpoints.
//!
//! These keep the dashboard's typed clients happy when self-hosted
//! orchestrators don't implement the full cloud-only feature set
//! (Orb billing, WorkOS, Discord, Vercel marketplace, etc.).

use serde::{
    Deserialize,
    Serialize,
};

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct StubBillingPlan {
    pub id: String,
    pub name: String,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct StubOrbSubscriptionResponse {
    pub plan: StubBillingPlan,
    pub current_period_start: f64,
    pub current_period_end: f64,
    pub cancel_at_period_end: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct StubTeamEntitlementsResponse {
    pub max_projects: u64,
    pub max_teams: u64,
    pub max_team_members: u64,
    pub max_deployments: u64,
    pub max_storage_bytes: u64,
    pub max_function_calls_per_month: u64,
    pub allow_periodic_backups: bool,
    pub allow_audit_log: bool,
    pub allow_custom_domains: bool,
    pub allow_streaming_export: bool,
    pub plan: String,
}

impl StubTeamEntitlementsResponse {
    pub fn unlimited() -> Self {
        Self {
            max_projects: u64::MAX,
            max_teams: u64::MAX,
            max_team_members: u64::MAX,
            max_deployments: u64::MAX,
            max_storage_bytes: u64::MAX,
            max_function_calls_per_month: u64::MAX,
            allow_periodic_backups: true,
            allow_audit_log: true,
            allow_custom_domains: true,
            allow_streaming_export: true,
            plan: "self-hosted".to_string(),
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct StubSpendingLimitsResponse {
    pub disable_threshold_cents: Option<u64>,
    pub warning_threshold_cents: Option<u64>,
    pub current_spend_cents: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct StubUsageRow {
    pub timestamp: f64,
    pub value: f64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct StubUsageQueryResponse {
    pub rows: Vec<StubUsageRow>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct StubReferralState {
    pub code: String,
    pub referred_count: u64,
    pub max_referrals: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct StubValidateReferralCode {
    pub valid: bool,
    pub team_name: Option<String>,
}

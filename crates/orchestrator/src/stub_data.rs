//! Helpers that produce safe stub responses for hosted-only endpoints.

use orchestrator_api_types::stubs::{
    StubBillingPlan,
    StubOrbSubscriptionResponse,
    StubReferralState,
    StubSpendingLimitsResponse,
    StubTeamEntitlementsResponse,
    StubUsageQueryResponse,
    StubValidateReferralCode,
};

use crate::time::now_unix_ms;

pub fn self_hosted_plan() -> StubBillingPlan {
    StubBillingPlan {
        id: "self-hosted".to_string(),
        name: "Self-hosted".to_string(),
        status: "active".to_string(),
    }
}

pub fn orb_subscription_stub() -> StubOrbSubscriptionResponse {
    let now = now_unix_ms() as f64;
    StubOrbSubscriptionResponse {
        plan: self_hosted_plan(),
        current_period_start: now,
        current_period_end: now + 31_536_000_000.0, // 365d
        cancel_at_period_end: false,
    }
}

pub fn entitlements_unlimited() -> StubTeamEntitlementsResponse {
    StubTeamEntitlementsResponse::unlimited()
}

pub fn spending_limits_unbounded() -> StubSpendingLimitsResponse {
    StubSpendingLimitsResponse::default()
}

pub fn empty_usage() -> StubUsageQueryResponse {
    StubUsageQueryResponse::default()
}

pub fn empty_referral(team_slug: &str) -> StubReferralState {
    StubReferralState {
        code: format!("ref-{team_slug}"),
        referred_count: 0,
        max_referrals: 0,
    }
}

pub fn invalid_referral_code() -> StubValidateReferralCode {
    StubValidateReferralCode {
        valid: false,
        team_name: None,
    }
}

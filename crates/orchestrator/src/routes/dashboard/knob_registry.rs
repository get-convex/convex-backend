//! GET /api/dashboard/knob_registry
//!
//! Returns every known knob (build-time extracted from common/src/knobs.rs)
//! plus its exposure classification. Stable per orchestrator binary; the
//! dashboard caches aggressively.

use axum::Json;
use serde::Serialize;

use crate::{
    auth::identity::AuthIdentity,
    errors::ApiResult,
    knob_registry::{
        exposure::{
            classify,
            curated_display_name,
            Exposure,
        },
        KNOWN_KNOBS,
    },
};

#[derive(Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct KnobEntry {
    pub env_var: String,
    pub description: String,
    pub category: String,
    pub exposure: &'static str,
    pub display_name: Option<String>,
    pub default_value: Option<String>,
}

#[derive(Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct KnobRegistryResponse {
    pub knobs: Vec<KnobEntry>,
}

#[utoipa::path(
    get,
    path = "/api/dashboard/knob_registry",
    responses((status = 200, body = KnobRegistryResponse)),
    tag = "dashboard",
)]
pub(crate) async fn knob_registry(
    _auth: AuthIdentity,
) -> ApiResult<Json<KnobRegistryResponse>> {
    let knobs = KNOWN_KNOBS
        .iter()
        .map(|k| KnobEntry {
            env_var: k.env_var.to_string(),
            description: k.description.to_string(),
            category: k.category.to_string(),
            exposure: match classify(k.env_var) {
                Exposure::Curated => "curated",
                Exposure::TierTuned => "tierTuned",
                Exposure::Advanced => "advanced",
            },
            display_name: curated_display_name(k.env_var).map(String::from),
            default_value: k.default_value.map(String::from),
        })
        .collect();
    Ok(Json(KnobRegistryResponse { knobs }))
}

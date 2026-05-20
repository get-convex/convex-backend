//! GET /api/dashboard/host_capacity
//!
//! Returns the host's total memory + CPU plus what's currently allocated
//! by summing the tier of every running deployment. Dashboard uses this
//! to render the capacity strip + warn before tier picks that would
//! exceed the host.

use axum::{
    extract::State,
    Json,
};
use serde::Serialize;

use crate::{
    auth::identity::AuthIdentity,
    errors::{
        ApiError,
        ApiResult,
    },
    state::OrchestratorState,
};

#[derive(Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct HostCapacityResponse {
    pub total_memory_mb: u64,
    pub total_cpus: u32,
    pub allocated_memory_mb: u64,
    pub allocated_cpus: f32,
    pub deployment_count: u32,
}

#[utoipa::path(
    get,
    path = "/api/dashboard/host_capacity",
    responses(
        (status = 200, body = HostCapacityResponse),
    ),
    tag = "dashboard",
)]
pub(crate) async fn host_capacity(
    _auth: AuthIdentity,
    State(state): State<OrchestratorState>,
) -> ApiResult<Json<HostCapacityResponse>> {
    let host = state.host_capacity.read();
    let tiers = state
        .storage
        .list_deployment_tiers()
        .await
        .map_err(ApiError::Internal)?;
    let mut allocated_memory: u64 = 0;
    let mut allocated_cpus: f32 = 0.0;
    for t in &tiers {
        if let Some(tier) = crate::provisioner::tiers::lookup(t) {
            allocated_memory += u64::from(tier.memory_mb);
            allocated_cpus += tier.cpus;
        }
    }
    Ok(Json(HostCapacityResponse {
        total_memory_mb: host.total_memory_mb,
        total_cpus: host.total_cpus,
        allocated_memory_mb: allocated_memory,
        allocated_cpus,
        deployment_count: tiers.len() as u32,
    }))
}

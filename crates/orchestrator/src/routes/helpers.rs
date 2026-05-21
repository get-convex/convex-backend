//! Shared helpers used by route handlers.

use orchestrator_api_types::dashboard::{
    DeploymentResponse,
    ProjectResponse,
    TeamResponse,
};
use orchestrator_api_types::management::{
    PlatformDeploymentResponse,
    PlatformProjectDetails,
};

use crate::{
    state::OrchestratorState,
    storage::{
        DeploymentRecord,
        ProjectRecord,
        TeamRecord,
    },
};

/// Soft-delete a project AND tear down every backend container that
/// belongs to it. Called from each delete-project route (dashboard,
/// deployment_internal, management) so cleanup is uniform.
///
/// Teardown errors are logged but don't block the project delete — once
/// the project row is marked deleted the orphaned container is invisible
/// to the dashboard, and `docker rm -f` is best-effort anyway.
pub async fn cascade_delete_project(
    state: &OrchestratorState,
    project_id: i64,
) -> anyhow::Result<()> {
    let deployments = state.storage.list_deployments(project_id).await?;
    for d in deployments {
        if let Err(e) = state.provisioner.teardown(&d.name).await {
            tracing::warn!(
                project_id,
                deployment = %d.name,
                error = %e,
                "teardown failed during cascade project delete; continuing",
            );
        }
        if let Err(e) = state.storage.delete_deployment(d.id).await {
            tracing::warn!(
                project_id,
                deployment = %d.name,
                error = %e,
                "deployment row delete failed during cascade; continuing",
            );
        }
    }
    state.storage.delete_project(project_id).await?;
    Ok(())
}

pub fn team_to_response(t: &TeamRecord) -> TeamResponse {
    TeamResponse {
        id: t.id as u64,
        name: t.name.clone(),
        slug: t.slug.clone(),
        creator: t.creator_id.map(|c| c as u64),
    }
}

pub fn project_to_response(p: &ProjectRecord) -> ProjectResponse {
    ProjectResponse {
        id: p.id as u64,
        team_id: p.team_id as u64,
        name: p.name.clone(),
        slug: p.slug.clone(),
        is_demo: p.is_demo,
        creation_time: p.creation_time as f64,
    }
}

pub fn project_to_platform(p: &ProjectRecord) -> PlatformProjectDetails {
    PlatformProjectDetails {
        id: p.id as u64,
        team_id: p.team_id as u64,
        name: p.name.clone(),
        slug: p.slug.clone(),
        is_demo: p.is_demo,
        creation_time: p.creation_time as f64,
    }
}

pub fn deployment_to_response(d: &DeploymentRecord) -> DeploymentResponse {
    DeploymentResponse {
        id: d.id as u64,
        project_id: d.project_id as u64,
        name: d.name.clone(),
        deployment_type: d.deployment_type.to_string(),
        deployment_class: d.deployment_class.to_string(),
        url: d.url.clone(),
        site_url: d.site_url.clone(),
        state: d.state.to_string(),
        creation_time: d.creation_time as f64,
        region: d.region.clone(),
        preview_identifier: d.preview_identifier.clone(),
    }
}

pub fn deployment_to_platform(d: &DeploymentRecord) -> PlatformDeploymentResponse {
    PlatformDeploymentResponse {
        id: d.id as u64,
        project_id: d.project_id as u64,
        name: d.name.clone(),
        kind: d.deployment_type.to_string(),
        deployment_class: d.deployment_class.to_string(),
        url: d.url.clone(),
        site_url: d.site_url.clone(),
        state: d.state.to_string(),
        creation_time: d.creation_time as f64,
        region: d.region.clone(),
        preview_identifier: d.preview_identifier.clone(),
        tier: d.tier.clone(),
    }
}

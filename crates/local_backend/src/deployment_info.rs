use axum::{
    extract::FromRef,
    response::IntoResponse,
};
use common::{
    http::{
        extract::{
            Json,
            MtState,
        },
        HttpResponseError,
    },
    types::{
        DeploymentId,
        DeploymentType,
        ProjectId,
        TeamId,
    },
};
use model::backend_info::BackendInfoModel;
use serde::Serialize;
use utoipa::ToSchema;
use utoipa_axum::router::OpenApiRouter;

use crate::{
    admin::must_be_admin,
    authentication::ExtractIdentity,
    LocalAppState,
};

#[derive(Serialize, ToSchema)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum DeploymentInfoResponse {
    #[serde(rename_all = "camelCase")]
    Cloud {
        team_id: TeamId,
        project_id: ProjectId,
        project_name: Option<String>,
        project_slug: Option<String>,
        id: DeploymentId,
        deployment_type: DeploymentType,
        reference: Option<String>,
    },
    #[serde(rename_all = "camelCase")]
    SelfHosted {},
}

/// Get deployment info
///
/// Returns identity information about this deployment.
#[utoipa::path(
    get,
    path = "/deployment_info",
    tag = "Deployment Info",
    operation_id = "get deployment info",
    security(
        ("Deploy Key" = []),
        ("OAuth Team Token" = []),
        ("Team Token" = []),
        ("OAuth Project Token" = []),
    ),
    responses(
        (status = 200, body = DeploymentInfoResponse)
    ),
)]
pub async fn get_deployment_info(
    MtState(st): MtState<LocalAppState>,
    ExtractIdentity(identity): ExtractIdentity,
) -> Result<impl IntoResponse, HttpResponseError> {
    must_be_admin(&identity)?;

    let mut tx = st.application.begin(identity).await?;
    let backend_info_doc = BackendInfoModel::new(&mut tx).get().await?;

    let response = match backend_info_doc {
        Some(doc) => {
            let backend_info = &**doc;
            DeploymentInfoResponse::Cloud {
                team_id: backend_info.team,
                project_id: backend_info.project,
                project_name: backend_info.project_name.clone(),
                project_slug: backend_info.project_slug.clone(),
                id: backend_info.deployment,
                deployment_type: backend_info.deployment_type,
                reference: backend_info.deployment_ref.clone(),
            }
        },
        None => DeploymentInfoResponse::SelfHosted {},
    };

    Ok(Json(response))
}

pub fn platform_router<S>() -> OpenApiRouter<S>
where
    LocalAppState: FromRef<S>,
    S: Clone + Send + Sync + 'static,
{
    OpenApiRouter::new().routes(utoipa_axum::routes!(get_deployment_info))
}

#[cfg(test)]
mod tests {
    use axum_extra::headers::authorization::Credentials;
    use http::Request;
    use keybroker::Identity;
    use model::backend_info::{
        types::BackendInfoPersisted,
        BackendInfoModel,
    };
    use runtime::prod::ProdRuntime;
    use serde_json::json;

    use crate::test_helpers::{
        setup_backend_for_test,
        TestLocalBackend,
    };

    async fn get_deployment_info(backend: &TestLocalBackend) -> anyhow::Result<serde_json::Value> {
        let req = Request::builder()
            .uri("/api/v1/deployment_info")
            .method("GET")
            .header("Authorization", backend.admin_auth_header.0.encode())
            .body(axum::body::Body::empty())?;
        backend.expect_success(req).await
    }

    #[convex_macro::prod_rt_test]
    async fn test_get_deployment_info_self_hosted(rt: ProdRuntime) -> anyhow::Result<()> {
        let backend = setup_backend_for_test(rt).await?;
        let response = get_deployment_info(&backend).await?;
        assert_eq!(response, json!({"kind": "selfHosted"}));
        Ok(())
    }

    #[convex_macro::prod_rt_test]
    async fn test_get_deployment_info_cloud(rt: ProdRuntime) -> anyhow::Result<()> {
        let backend = setup_backend_for_test(rt).await?;

        // Seed backend info
        let mut tx = backend.st.application.begin(Identity::system()).await?;
        BackendInfoModel::new(&mut tx)
            .set(BackendInfoPersisted::default())
            .await?;
        backend.st.application.commit_test(tx).await?;

        let response = get_deployment_info(&backend).await?;
        assert_eq!(response["kind"], "cloud");
        assert_eq!(response["teamId"], 4);
        assert_eq!(response["projectId"], 17);
        assert_eq!(response["id"], 2021);
        assert_eq!(response["deploymentType"], "dev");
        assert_eq!(response["projectName"], "Default Project");
        assert_eq!(response["projectSlug"], "default-project");
        Ok(())
    }
}

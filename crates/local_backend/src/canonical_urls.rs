use axum::{
    extract::{
        FromRef,
        State,
    },
    response::IntoResponse,
};
use common::http::{
    extract::{
        Json,
        MtState,
    },
    HttpResponseError,
    RequestDestination,
};
use http::StatusCode;
use model::{
    canonical_urls::{
        types::CanonicalUrl,
        CanonicalUrlsModel,
    },
    deployment_audit_log::types::DeploymentAuditLogEvent,
};
use serde::{
    Deserialize,
    Serialize,
};
use utoipa::ToSchema;
use utoipa_axum::router::OpenApiRouter;

use crate::{
    admin::{
        must_be_admin,
        must_be_admin_with_write_access,
    },
    authentication::ExtractIdentity,
    LocalAppState,
};

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UpdateCanonicalUrlRequest {
    /// Whether to update the canonical URL for convex.cloud or convex.site
    request_destination: RequestDestination,
    /// The new canonical URL. Omit this to reset the canonical URl to the
    /// default value.
    url: Option<String>,
}

/// Update canonical URL
///
/// Set or unset the canonical URL for a deployment's convex.cloud or
/// convex.site domain. This allows you to customize the  CONVEX_SITE_URL and
/// CONVEX_CLOUD_URL environment variables in your deployment.
#[utoipa::path(
    post,
    path = "/update_canonical_url",
    request_body = UpdateCanonicalUrlRequest,
    responses((status = 200)),
)]
pub async fn update_canonical_url(
    State(st): State<LocalAppState>,
    ExtractIdentity(identity): ExtractIdentity,
    Json(request): Json<UpdateCanonicalUrlRequest>,
) -> Result<impl IntoResponse, HttpResponseError> {
    must_be_admin_with_write_access(&identity)?;

    let mut tx = st.application.begin(identity).await?;

    let mut audit_log_events = vec![];
    if let Some(url) = request.url.clone() {
        let canonical_url = CanonicalUrl {
            request_destination: request.request_destination,
            url: url.clone(),
        };
        st.application
            .set_canonical_url(&mut tx, canonical_url)
            .await?;
        audit_log_events.push(DeploymentAuditLogEvent::UpdateCanonicalUrl {
            request_destination: request.request_destination,
            url,
        });
    } else {
        st.application
            .unset_canonical_url(&mut tx, request.request_destination)
            .await?;
        audit_log_events.push(DeploymentAuditLogEvent::DeleteCanonicalUrl {
            request_destination: request.request_destination,
        });
    }

    st.application
        .commit_with_audit_log_events(tx, audit_log_events, "update_canonical_url")
        .await?;

    Ok(StatusCode::OK)
}

#[derive(Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct GetCanonicalUrlsResponse {
    convex_cloud_url: String,
    convex_site_url: String,
}

/// Get canonical URLs
///
/// Get the canonical URLs for a deployment.
#[utoipa::path(
    get,
    path = "/get_canonical_urls",
    responses(
        (status = 200, body = GetCanonicalUrlsResponse)
    ),
)]
pub async fn get_canonical_urls(
    MtState(st): MtState<LocalAppState>,
    ExtractIdentity(identity): ExtractIdentity,
) -> Result<impl IntoResponse, HttpResponseError> {
    must_be_admin(&identity)?;

    let mut tx = st.application.begin(identity).await?;
    let urls = CanonicalUrlsModel::new(&mut tx)
        .get_canonical_urls()
        .await?;

    let mut convex_cloud_url = None;
    let mut convex_site_url = None;

    for (destination, url) in urls {
        match destination {
            RequestDestination::ConvexCloud => {
                convex_cloud_url = Some(url.into_value().url);
            },
            RequestDestination::ConvexSite => {
                convex_site_url = Some(url.into_value().url);
            },
        }
    }

    // If canonical URLs aren't set, return the default URLs
    let convex_cloud_url = convex_cloud_url.unwrap_or_else(|| st.origin.to_string());
    let convex_site_url = convex_site_url.unwrap_or_else(|| st.site_origin.to_string());

    Ok(Json(GetCanonicalUrlsResponse {
        convex_cloud_url,
        convex_site_url,
    }))
}

pub fn platform_router<S>() -> OpenApiRouter<S>
where
    LocalAppState: FromRef<S>,
    S: Clone + Send + Sync + 'static,
{
    OpenApiRouter::new().routes(utoipa_axum::routes!(
        update_canonical_url,
        get_canonical_urls
    ))
}

#[cfg(test)]
mod tests {
    use axum_extra::headers::authorization::Credentials;
    use common::http::RequestDestination;
    use http::Request;
    use runtime::prod::ProdRuntime;
    use serde_json::json;
    use value::val;

    use crate::{
        canonical_urls::GetCanonicalUrlsResponse,
        test_helpers::{
            setup_backend_for_test,
            TestLocalBackend,
        },
    };

    async fn update_canonical_url(
        backend: &TestLocalBackend,
        request_destination: RequestDestination,
        url: Option<&str>,
    ) -> anyhow::Result<()> {
        let json_body = json!({
            "requestDestination": request_destination,
            "url": url,
        });
        let body = axum::body::Body::from(serde_json::to_vec(&json_body)?);
        let req = Request::builder()
            .uri("/api/update_canonical_url")
            .method("POST")
            .header("Content-Type", "application/json")
            .header("Authorization", backend.admin_auth_header.0.encode())
            .body(body)?;
        let () = backend.expect_success(req).await?;
        Ok(())
    }

    async fn get_canonical_urls_helper(
        backend: &TestLocalBackend,
    ) -> anyhow::Result<GetCanonicalUrlsResponse> {
        let json_body = json!({});
        let body = axum::body::Body::from(serde_json::to_vec(&json_body)?);
        let req = Request::builder()
            .uri("/api/v1/get_canonical_urls")
            .method("GET")
            .header("Content-Type", "application/json")
            .header("Authorization", backend.admin_auth_header.0.encode())
            .body(body)?;
        backend.expect_success(req).await
    }

    #[convex_macro::prod_rt_test]
    async fn test_create_canonical_urls(rt: ProdRuntime) -> anyhow::Result<()> {
        let backend = setup_backend_for_test(rt).await?;
        update_canonical_url(
            &backend,
            RequestDestination::ConvexCloud,
            Some("https://cloud.example.com"),
        )
        .await?;
        update_canonical_url(
            &backend,
            RequestDestination::ConvexSite,
            Some("https://site.example.com"),
        )
        .await?;

        let response = get_canonical_urls_helper(&backend).await?;
        assert_eq!(response.convex_cloud_url, "https://cloud.example.com");
        assert_eq!(response.convex_site_url, "https://site.example.com");
        Ok(())
    }

    #[convex_macro::prod_rt_test]
    async fn test_update_canonical_urls(rt: ProdRuntime) -> anyhow::Result<()> {
        let backend = setup_backend_for_test(rt).await?;
        update_canonical_url(
            &backend,
            RequestDestination::ConvexCloud,
            Some("https://cloud.example.com"),
        )
        .await?;
        update_canonical_url(
            &backend,
            RequestDestination::ConvexSite,
            Some("https://site.example.com"),
        )
        .await?;

        // Update existing URLs
        update_canonical_url(
            &backend,
            RequestDestination::ConvexCloud,
            Some("https://new-cloud.example.com"),
        )
        .await?;
        update_canonical_url(
            &backend,
            RequestDestination::ConvexSite,
            Some("https://new-site.example.com"),
        )
        .await?;

        let response = get_canonical_urls_helper(&backend).await?;
        assert_eq!(response.convex_cloud_url, "https://new-cloud.example.com");
        assert_eq!(response.convex_site_url, "https://new-site.example.com");

        let query_convex_cloud = backend
            .run_query("_system/frontend/convexCloudUrl".parse()?)
            .await?;
        assert_eq!(
            query_convex_cloud.result.map(|v| v.unpack()),
            Ok(val!("https://new-cloud.example.com"))
        );

        let query_convex_site = backend
            .run_query("_system/frontend/convexSiteUrl".parse()?)
            .await?;
        assert_eq!(
            query_convex_site.result.map(|v| v.unpack()),
            Ok(val!("https://new-site.example.com"))
        );

        Ok(())
    }

    #[convex_macro::prod_rt_test]
    async fn test_delete_canonical_urls(rt: ProdRuntime) -> anyhow::Result<()> {
        let backend = setup_backend_for_test(rt).await?;
        update_canonical_url(
            &backend,
            RequestDestination::ConvexCloud,
            Some("https://cloud.example.com"),
        )
        .await?;
        update_canonical_url(
            &backend,
            RequestDestination::ConvexSite,
            Some("https://site.example.com"),
        )
        .await?;

        // Delete URLs
        update_canonical_url(&backend, RequestDestination::ConvexCloud, None).await?;
        update_canonical_url(&backend, RequestDestination::ConvexSite, None).await?;

        // After deletion, should return default URLs
        let response = get_canonical_urls_helper(&backend).await?;
        assert_eq!(response.convex_cloud_url, backend.st.origin.to_string());
        assert_eq!(response.convex_site_url, backend.st.site_origin.to_string());
        Ok(())
    }

    #[convex_macro::prod_rt_test]
    async fn test_get_default_canonical_urls(rt: ProdRuntime) -> anyhow::Result<()> {
        let backend = setup_backend_for_test(rt).await?;

        // Without setting any canonical URLs, should return default URLs
        let response = get_canonical_urls_helper(&backend).await?;
        assert_eq!(response.convex_cloud_url, backend.st.origin.to_string());
        assert_eq!(response.convex_site_url, backend.st.site_origin.to_string());
        Ok(())
    }
}

use axum::{
    extract::State,
    response::IntoResponse,
};
use common::http::{
    extract::Json,
    HttpResponseError,
    RequestDestination,
};
use http::StatusCode;
use model::{
    canonical_urls::types::CanonicalUrl,
    deployment_audit_log::types::DeploymentAuditLogEvent,
};
use serde::Deserialize;

use crate::{
    admin::must_be_admin_with_write_access,
    authentication::ExtractIdentity,
    LocalAppState,
};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateCanonicalUrlRequest {
    request_destination: RequestDestination,
    url: Option<String>,
}

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

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use axum_extra::headers::authorization::Credentials;
    use common::http::RequestDestination;
    use http::Request;
    use keybroker::Identity;
    use model::canonical_urls::CanonicalUrlsModel;
    use runtime::prod::ProdRuntime;
    use serde_json::json;
    use value::val;

    use crate::test_helpers::{
        setup_backend_for_test,
        TestLocalBackend,
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

    async fn list_canonical_urls(
        backend: &TestLocalBackend,
    ) -> anyhow::Result<BTreeMap<RequestDestination, String>> {
        let mut tx = backend.st.application.begin(Identity::system()).await?;
        let urls = CanonicalUrlsModel::new(&mut tx)
            .get_canonical_urls()
            .await?;
        Ok(urls
            .into_iter()
            .map(|(k, v)| (k, v.into_value().url))
            .collect())
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

        let urls = list_canonical_urls(&backend).await?;
        assert_eq!(urls.len(), 2);
        assert_eq!(
            urls.get(&RequestDestination::ConvexCloud)
                .map(|u| u.as_str()),
            Some("https://cloud.example.com")
        );
        assert_eq!(
            urls.get(&RequestDestination::ConvexSite)
                .map(|u| u.as_str()),
            Some("https://site.example.com")
        );
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

        let urls = list_canonical_urls(&backend).await?;
        assert_eq!(urls.len(), 2);
        assert_eq!(
            urls.get(&RequestDestination::ConvexCloud)
                .map(|u| u.as_str()),
            Some("https://new-cloud.example.com")
        );
        assert_eq!(
            urls.get(&RequestDestination::ConvexSite)
                .map(|u| u.as_str()),
            Some("https://new-site.example.com")
        );

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

        let urls = list_canonical_urls(&backend).await?;
        assert!(urls.is_empty());
        Ok(())
    }
}

use std::time::Duration;

use anyhow::Context;
use axum::{
    body::Body,
    response::IntoResponse,
};
use axum_extra::{
    headers::{
        CacheControl,
        ContentLength,
    },
    TypedHeader,
};
use common::{
    components::ComponentId,
    http::{
        extract::{
            Json,
            MtState,
            Path,
            Query,
        },
        HttpResponseError,
    },
    types::SetExportExpirationRequest,
};
use either::Either;
use errors::ErrorMetadata;
use http::StatusCode;
use model::exports::{
    types::{
        ExportFormat,
        ExportRequestor,
    },
    ExportsModel,
};
use serde::Deserialize;
use storage::StorageGetStream;
use sync_types::Timestamp;
use value::DeveloperDocumentId;

use crate::{
    authentication::ExtractIdentity,
    custom_headers::ContentDispositionAttachment,
    LocalAppState,
};

// Export GETs are immutable. Browser can cache for a long time.
const MAX_CACHE_AGE: Duration = Duration::from_secs(60 * 60 * 24 * 30);

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RequestZipExport {
    #[serde(default)]
    pub include_storage: bool,
    pub component: Option<String>,
}

#[fastrace::trace]
pub async fn request_zip_export(
    MtState(st): MtState<LocalAppState>,
    ExtractIdentity(identity): ExtractIdentity,
    Query(RequestZipExport {
        include_storage,
        component,
    }): Query<RequestZipExport>,
) -> Result<impl IntoResponse, HttpResponseError> {
    let component = ComponentId::deserialize_from_string(component.as_deref())?;
    st.application
        .request_export(
            identity,
            ExportFormat::Zip { include_storage },
            component,
            ExportRequestor::SnapshotExport,
            None,
        )
        .await?;
    Ok(StatusCode::OK)
}

#[derive(Deserialize)]
pub struct ZipExportRequest {
    // The ID of the snapshot
    id: String,
}

pub async fn get_zip_export(
    MtState(st): MtState<LocalAppState>,
    ExtractIdentity(identity): ExtractIdentity,
    Path(ZipExportRequest { id }): Path<ZipExportRequest>,
) -> Result<impl IntoResponse, HttpResponseError> {
    identity.require_operation(keybroker::DeploymentOp::DownloadBackups)?;
    let id: Either<DeveloperDocumentId, Timestamp> = match id.parse() {
        Ok(id) => Either::Left(id),
        Err(_) => Either::Right(id.parse().context(ErrorMetadata::bad_request(
            "BadSnapshotId",
            "Snapshot Id did not parse to an ID.",
        ))?),
    };
    let (
        StorageGetStream {
            content_length,
            stream,
        },
        filename,
    ) = st.application.get_zip_export(identity, id).await?;
    let content_length = ContentLength(content_length as u64);
    Ok((
        TypedHeader(content_length),
        // `ContentDisposition::attachment()` is not implemented in the headers library yet!
        // so we handroll it:
        TypedHeader(ContentDispositionAttachment(filename)),
        TypedHeader(
            CacheControl::new()
                .with_private()
                .with_max_age(MAX_CACHE_AGE),
        ),
        Body::from_stream(stream),
    ))
}

#[derive(Deserialize)]
pub struct SetExportExpirationPathArgs {
    snapshot_id: String,
}

#[fastrace::trace]
pub async fn set_export_expiration(
    MtState(st): MtState<LocalAppState>,
    ExtractIdentity(identity): ExtractIdentity,
    Path(SetExportExpirationPathArgs { snapshot_id }): Path<SetExportExpirationPathArgs>,
    Json(SetExportExpirationRequest { expiration_ts_ns }): Json<SetExportExpirationRequest>,
) -> Result<StatusCode, HttpResponseError> {
    identity.require_operation(keybroker::DeploymentOp::DeleteBackups)?;
    let snapshot_id: DeveloperDocumentId = snapshot_id
        .parse::<DeveloperDocumentId>()
        .map_err(|e| anyhow::anyhow!(e))?;
    let mut tx = st.application.begin(identity).await?;
    ExportsModel::new(&mut tx)
        .set_expiration(snapshot_id, expiration_ts_ns)
        .await?;
    st.application.commit(tx, "set_export_expiration").await?;
    Ok(StatusCode::OK)
}

#[fastrace::trace]
pub async fn cancel_export(
    MtState(st): MtState<LocalAppState>,
    ExtractIdentity(identity): ExtractIdentity,
    Path(SetExportExpirationPathArgs { snapshot_id }): Path<SetExportExpirationPathArgs>,
) -> Result<StatusCode, HttpResponseError> {
    identity.require_operation(keybroker::DeploymentOp::ImportBackups)?;
    let snapshot_id: DeveloperDocumentId = snapshot_id
        .parse::<DeveloperDocumentId>()
        .map_err(|e| anyhow::anyhow!(e))?;
    let mut tx = st.application.begin(identity).await?;
    ExportsModel::new(&mut tx).cancel(snapshot_id).await?;
    st.application.commit(tx, "cancel_export").await?;
    Ok(StatusCode::OK)
}

#[cfg(test)]
mod tests {
    use axum_extra::headers::authorization::Credentials;
    use http::Request;
    use runtime::prod::ProdRuntime;

    use crate::test_helpers::setup_backend_for_test;

    #[convex_macro::prod_rt_test]
    async fn test_request_zip_export_denied_for_read_only(rt: ProdRuntime) -> anyhow::Result<()> {
        let backend = setup_backend_for_test(rt).await?;
        let req = Request::builder()
            .uri("/api/export/request/zip")
            .method("POST")
            .header(
                "Authorization",
                backend.read_only_admin_auth_header.0.encode(),
            )
            .body(axum::body::Body::empty())?;
        backend
            .expect_error(req, http::StatusCode::FORBIDDEN, "Unauthorized")
            .await?;
        Ok(())
    }

    #[convex_macro::prod_rt_test]
    async fn test_set_export_expiration_denied_for_read_only(
        rt: ProdRuntime,
    ) -> anyhow::Result<()> {
        let backend = setup_backend_for_test(rt).await?;
        let req = Request::builder()
            .uri("/api/export/set_expiration/fake_id")
            .method("POST")
            .header("Content-Type", "application/json")
            .header(
                "Authorization",
                backend.read_only_admin_auth_header.0.encode(),
            )
            .body(axum::body::Body::from(serde_json::to_vec(
                &serde_json::json!({"expirationTsNs": 0}),
            )?))?;
        backend
            .expect_error(req, http::StatusCode::FORBIDDEN, "Unauthorized")
            .await?;
        Ok(())
    }

    #[convex_macro::prod_rt_test]
    async fn test_cancel_export_denied_for_read_only(rt: ProdRuntime) -> anyhow::Result<()> {
        let backend = setup_backend_for_test(rt).await?;
        let req = Request::builder()
            .uri("/api/export/cancel/fake_id")
            .method("POST")
            .header(
                "Authorization",
                backend.read_only_admin_auth_header.0.encode(),
            )
            .body(axum::body::Body::empty())?;
        backend
            .expect_error(req, http::StatusCode::FORBIDDEN, "Unauthorized")
            .await?;
        Ok(())
    }

    #[convex_macro::prod_rt_test]
    async fn test_get_zip_export_allowed_for_read_only(rt: ProdRuntime) -> anyhow::Result<()> {
        let backend = setup_backend_for_test(rt).await?;
        // Use a fake ID — will fail with a not-found error, but that proves the
        // auth check (DownloadBackups) passed for the read-only key.
        let req = Request::builder()
            .uri("/api/export/zip/0")
            .method("GET")
            .header(
                "Authorization",
                backend.read_only_admin_auth_header.0.encode(),
            )
            .body(axum::body::Body::empty())?;
        // We expect a not-found error (fake export ID), not a 403 forbidden.
        backend
            .expect_error(req, http::StatusCode::NOT_FOUND, "ExportNotFound")
            .await?;
        Ok(())
    }
}

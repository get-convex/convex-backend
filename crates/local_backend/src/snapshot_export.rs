use std::time::Duration;

use anyhow::Context;
use axum::{
    body::Body,
    debug_handler,
    extract::State,
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
    admin::must_be_admin_with_write_access,
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
    State(st): State<LocalAppState>,
    ExtractIdentity(identity): ExtractIdentity,
    Query(RequestZipExport {
        include_storage,
        component,
    }): Query<RequestZipExport>,
) -> Result<impl IntoResponse, HttpResponseError> {
    must_be_admin_with_write_access(&identity)?;
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

#[debug_handler]
pub async fn get_zip_export(
    State(st): State<LocalAppState>,
    ExtractIdentity(identity): ExtractIdentity,
    Path(ZipExportRequest { id }): Path<ZipExportRequest>,
) -> Result<impl IntoResponse, HttpResponseError> {
    must_be_admin_with_write_access(&identity)?;
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

#[debug_handler]
#[fastrace::trace]
pub async fn set_export_expiration(
    State(st): State<LocalAppState>,
    ExtractIdentity(identity): ExtractIdentity,
    Path(SetExportExpirationPathArgs { snapshot_id }): Path<SetExportExpirationPathArgs>,
    Json(SetExportExpirationRequest { expiration_ts_ns }): Json<SetExportExpirationRequest>,
) -> Result<StatusCode, HttpResponseError> {
    if !(identity.is_system() || identity.is_admin()) {
        Err(anyhow::anyhow!(ErrorMetadata::forbidden(
            "SetExportExpirationForbidden",
            "Must have system or admin identity to set export expiration"
        )))?;
    }
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

#[debug_handler]
#[fastrace::trace]
pub async fn cancel_export(
    State(st): State<LocalAppState>,
    ExtractIdentity(identity): ExtractIdentity,
    Path(SetExportExpirationPathArgs { snapshot_id }): Path<SetExportExpirationPathArgs>,
) -> Result<StatusCode, HttpResponseError> {
    // This route is accessed directly from the admin dashboard
    if !(identity.is_system() || identity.is_admin()) {
        Err(anyhow::anyhow!(ErrorMetadata::forbidden(
            "CancelExportForbidden",
            "Must have system or admin identity to cancel cloud export"
        )))?;
    }
    let snapshot_id: DeveloperDocumentId = snapshot_id
        .parse::<DeveloperDocumentId>()
        .map_err(|e| anyhow::anyhow!(e))?;
    let mut tx = st.application.begin(identity).await?;
    ExportsModel::new(&mut tx).cancel(snapshot_id).await?;
    st.application.commit(tx, "cancel_export").await?;
    Ok(StatusCode::OK)
}

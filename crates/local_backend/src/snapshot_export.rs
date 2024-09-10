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
            Path,
            Query,
        },
        HttpResponseError,
    },
};
use errors::ErrorMetadata;
use http::StatusCode;
use model::exports::types::{
    ExportFormat,
    ExportRequestor,
};
use serde::Deserialize;
use storage::StorageGetStream;
use sync_types::Timestamp;

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

#[minitrace::trace]
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
        )
        .await?;
    Ok(StatusCode::OK)
}

#[derive(Deserialize)]
pub struct ZipExportRequest {
    // Timestamp the snapshot export started at
    snapshot_ts: String,
}

#[debug_handler]
pub async fn get_zip_export(
    State(st): State<LocalAppState>,
    ExtractIdentity(identity): ExtractIdentity,
    Path(ZipExportRequest { snapshot_ts }): Path<ZipExportRequest>,
) -> Result<impl IntoResponse, HttpResponseError> {
    must_be_admin_with_write_access(&identity)?;
    let ts: Timestamp = snapshot_ts.parse().context(ErrorMetadata::bad_request(
        "BadSnapshotTimestamp",
        "Snapshot timestamp did not parse to a timestamp.",
    ))?;
    let (
        StorageGetStream {
            content_length,
            stream,
        },
        filename,
    ) = st.application.get_zip_export(identity, ts).await?;
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

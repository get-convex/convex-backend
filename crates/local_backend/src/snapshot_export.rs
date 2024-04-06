use std::time::Duration;

use anyhow::Context;
use axum::{
    body::StreamBody,
    debug_handler,
    extract::State,
    headers::{
        CacheControl,
        ContentLength,
    },
    response::IntoResponse,
    TypedHeader,
};
use common::http::{
    extract::{
        Path,
        Query,
    },
    HttpResponseError,
};
use errors::ErrorMetadata;
use http::StatusCode;
use serde::Deserialize;
use storage::StorageGetStream;
use sync_types::Timestamp;

use crate::{
    admin::must_be_admin,
    authentication::ExtractIdentity,
    custom_headers::ContentDispositionAttachment,
    LocalAppState,
};

// Export GETs are immutable. Browser can cache for a long time.
const MAX_CACHE_AGE: Duration = Duration::from_secs(60 * 60 * 24 * 30);

pub async fn request_export(
    State(st): State<LocalAppState>,
    ExtractIdentity(identity): ExtractIdentity,
) -> Result<impl IntoResponse, HttpResponseError> {
    must_be_admin(&identity)?;
    st.application
        .request_export(identity, false, false)
        .await?;
    Ok(StatusCode::OK)
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RequestZipExport {
    #[serde(default)]
    include_storage: bool,
}

#[minitrace::trace]
pub async fn request_zip_export(
    State(st): State<LocalAppState>,
    ExtractIdentity(identity): ExtractIdentity,
    Query(RequestZipExport { include_storage }): Query<RequestZipExport>,
) -> Result<impl IntoResponse, HttpResponseError> {
    must_be_admin(&identity)?;
    st.application
        .request_export(identity, true, include_storage)
        .await?;
    Ok(StatusCode::OK)
}

#[derive(Deserialize)]
pub struct ExportRequest {
    // Timestamp the snapshot export started at
    snapshot_ts: String,
    // Table to get the export for
    table_name: String,
}

#[debug_handler]
pub async fn get_export(
    State(st): State<LocalAppState>,
    ExtractIdentity(identity): ExtractIdentity,
    Path(ExportRequest {
        snapshot_ts,
        table_name: file_name,
    }): Path<ExportRequest>,
) -> Result<impl IntoResponse, HttpResponseError> {
    must_be_admin(&identity)?;
    let ts: Timestamp = snapshot_ts.parse().context(ErrorMetadata::bad_request(
        "BadSnapshotTimestamp",
        "Snapshot timestamp did not parse to a timestamp.",
    ))?;
    let extension = if file_name.ends_with(".json") {
        Some(".json")
    } else if file_name.ends_with(".jsonl") {
        Some(".jsonl")
    } else {
        None
    }
    .context(ErrorMetadata::bad_request(
        "BadSnapshotFilename",
        "Snapshot filename must be {table}.json(l)",
    ))?;
    let table_name = file_name.strip_suffix(extension).unwrap();
    let StorageGetStream {
        content_length,
        stream,
    } = st
        .application
        .get_export(identity, ts, table_name.parse()?)
        .await?;
    let content_length = ContentLength(content_length as u64);
    Ok((
        TypedHeader(content_length),
        // `ContentDisposition::attachment()` is not implemented in the headers library yet!
        // so we handroll it:
        TypedHeader(ContentDispositionAttachment(
            // It's kinda jank that we rely on client to tell us the
            // file name but it's much easier to implement :)
            file_name,
        )),
        TypedHeader(
            CacheControl::new()
                .with_private()
                .with_max_age(MAX_CACHE_AGE),
        ),
        StreamBody::new(stream),
    ))
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
    must_be_admin(&identity)?;
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
        StreamBody::new(stream),
    ))
}

use std::{
    ops::Bound,
    time::Duration,
};

use anyhow::Context;
use axum::{
    body::StreamBody,
    debug_handler,
    extract::{
        rejection::{
            TypedHeaderRejection,
            TypedHeaderRejectionReason,
        },
        BodyStream,
        State,
    },
    headers::{
        AcceptRanges,
        CacheControl,
        ContentLength,
        ContentType,
        Header,
        Range,
    },
    response::{
        IntoResponse,
        Response,
    },
    TypedHeader,
};
use common::{
    http::{
        extract::{
            Json,
            Path,
            Query,
        },
        HttpResponseError,
    },
    sha256::DigestHeader,
};
use errors::ErrorMetadata;
use file_storage::{
    FileRangeStream,
    FileStream,
};
use futures::StreamExt;
use http::StatusCode;
use model::file_storage::FileStorageId;
use serde::{
    Deserialize,
    Serialize,
};

use crate::LocalAppState;

// Storage GETs are immutable. Browser can cache for a long time.
const MAX_CACHE_AGE: Duration = Duration::from_secs(60 * 60 * 24 * 30);

const STORE_FILE_AUTHORIZATION_VALIDITY: Duration = Duration::from_secs(60 * 60);

fn map_header_err<T: Header>(
    r: Result<TypedHeader<T>, TypedHeaderRejection>,
) -> anyhow::Result<Option<T>> {
    r.map(|t| Some(t.0)).or_else(|e| match e.reason() {
        TypedHeaderRejectionReason::Missing => Ok(None),
        TypedHeaderRejectionReason::Error(e) => anyhow::bail!(ErrorMetadata::bad_request(
            "BadHeader",
            format!("Bad header for {}: {}", T::name(), e),
        )),
        _ => anyhow::bail!("{e:?}"),
    })
}

#[derive(Deserialize)]
pub struct QueryParams {
    token: String,
}

#[debug_handler]
pub async fn storage_upload(
    State(st): State<LocalAppState>,
    Query(QueryParams { token }): Query<QueryParams>,
    content_type: Result<TypedHeader<ContentType>, TypedHeaderRejection>,
    content_length: Result<TypedHeader<ContentLength>, TypedHeaderRejection>,
    sha256: Result<TypedHeader<DigestHeader>, TypedHeaderRejection>,
    body: BodyStream,
) -> Result<impl IntoResponse, HttpResponseError> {
    st.application.key_broker().check_store_file_authorization(
        &st.application.runtime(),
        &token,
        STORE_FILE_AUTHORIZATION_VALIDITY,
    )?;

    let content_length = map_header_err(content_length)?;
    let content_type = map_header_err(content_type)?;
    let sha256 = map_header_err(sha256)?.map(|dh| dh.0);
    let body = body.map(|r| r.context("Error parsing body"));
    let storage_id = st
        .application
        .store_file(content_length, content_type, sha256, body)
        .await?;

    #[derive(Serialize)]
    #[serde(rename_all = "camelCase")]
    struct Response {
        storage_id: String,
    }
    Ok(Json(Response {
        storage_id: storage_id.to_string(),
    }))
}

#[debug_handler]
pub async fn storage_get(
    State(st): State<LocalAppState>,
    Path(uuid): Path<String>,
    // Query(QueryParams { token }): Query<QueryParams>,
    range: Result<TypedHeader<Range>, TypedHeaderRejection>,
) -> Result<Response, HttpResponseError> {
    let storage_uuid = uuid.parse().context(ErrorMetadata::bad_request(
        "InvalidStoragePath",
        format!("Invalid storage path: \"{uuid}\". Please use `storage.getUrl()` to generate a valid URL to retrieve files. See https://docs.convex.dev/file-storage/serve-files for more details"),
    ))?;
    let file_storage_id = FileStorageId::LegacyStorageId(storage_uuid);

    // TODO(CX-3065) figure out deterministic repeatable tokens

    if let Ok(range_header) = range {
        let ranges: Vec<(Bound<u64>, Bound<u64>)> = range_header.iter().collect();
        // Convex only supports a single range because underlying AWS S3 only supports
        // a single range
        if ranges.len() != 1 {
            // It's always allowable to return the whole file
            // so we could do that here too.
            return Ok(StatusCode::RANGE_NOT_SATISFIABLE.into_response());
        }
        let range = ranges[0];

        let FileRangeStream {
            content_length,
            content_range,
            content_type,
            stream,
        } = st
            .application
            .get_file_range(file_storage_id, range)
            .await?;

        return Ok((
            StatusCode::PARTIAL_CONTENT,
            content_type.map(TypedHeader),
            TypedHeader(content_range),
            TypedHeader(content_length),
            TypedHeader(
                CacheControl::new()
                    .with_private()
                    .with_max_age(MAX_CACHE_AGE),
            ),
            TypedHeader(AcceptRanges::bytes()),
            StreamBody::new(stream),
        )
            .into_response());
    }

    let FileStream {
        sha256,
        content_type,
        content_length,
        stream,
    } = st.application.get_file(file_storage_id).await?;
    Ok((
        TypedHeader(DigestHeader(sha256)),
        content_type.map(TypedHeader),
        TypedHeader(content_length),
        TypedHeader(
            CacheControl::new()
                .with_private()
                .with_max_age(MAX_CACHE_AGE),
        ),
        TypedHeader(AcceptRanges::bytes()),
        StreamBody::new(stream),
    )
        .into_response())
}

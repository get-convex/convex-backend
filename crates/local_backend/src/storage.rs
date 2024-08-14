use std::{
    ops::Bound,
    time::Duration,
};

use anyhow::Context;
use axum::{
    body::Body,
    debug_handler,
    extract::{
        Host,
        State,
    },
    response::{
        IntoResponse,
        Response,
    },
};
use axum_extra::{
    headers::{
        AcceptRanges,
        CacheControl,
        ContentLength,
        ContentType,
        Header,
        Range,
    },
    typed_header::{
        TypedHeaderRejection,
        TypedHeaderRejectionReason,
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
        ExtractRequestId,
        ExtractResolvedHost,
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

use crate::RouterState;

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
    State(st): State<RouterState>,
    Query(QueryParams { token }): Query<QueryParams>,
    content_type: Result<TypedHeader<ContentType>, TypedHeaderRejection>,
    content_length: Result<TypedHeader<ContentLength>, TypedHeaderRejection>,
    sha256: Result<TypedHeader<DigestHeader>, TypedHeaderRejection>,
    ExtractResolvedHost(host): ExtractResolvedHost,
    Host(original_host): Host,
    ExtractRequestId(request_id): ExtractRequestId,
    body: Body,
) -> Result<impl IntoResponse, HttpResponseError> {
    let component = st
        .api
        .check_store_file_authorization(
            &host,
            request_id.clone(),
            &token,
            STORE_FILE_AUTHORIZATION_VALIDITY,
        )
        .await?;
    let content_length = map_header_err(content_length)?;
    let content_type = map_header_err(content_type)?;
    let sha256 = map_header_err(sha256)?.map(|dh| dh.0);
    let body = body
        .into_data_stream()
        .map(|r| r.context("Error parsing body"))
        .boxed();
    let origin = original_host.into();
    let storage_id = st
        .api
        .store_file(
            &host,
            request_id,
            origin,
            component,
            content_length,
            content_type,
            sha256,
            body,
        )
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

#[derive(Deserialize)]
pub struct GetQueryParams {
    component: Option<String>,
}

#[debug_handler]
pub async fn storage_get(
    State(st): State<RouterState>,
    Path(uuid): Path<String>,
    Query(GetQueryParams { component }): Query<GetQueryParams>,
    range: Result<TypedHeader<Range>, TypedHeaderRejection>,
    ExtractResolvedHost(host): ExtractResolvedHost,
    Host(original_host): Host,
    ExtractRequestId(request_id): ExtractRequestId,
) -> Result<Response, HttpResponseError> {
    let storage_uuid = uuid.parse().context(ErrorMetadata::bad_request(
        "InvalidStoragePath",
        format!("Invalid storage path: \"{uuid}\". Please use `storage.getUrl()` to generate a valid URL to retrieve files. See https://docs.convex.dev/file-storage/serve-files for more details"),
    ))?;
    let file_storage_id = FileStorageId::LegacyStorageId(storage_uuid);
    let component = ComponentId::deserialize_from_string(component.as_deref())?;
    let origin = original_host.into();

    // TODO(CX-3065) figure out deterministic repeatable tokens

    if let Ok(range_header) = range {
        let ranges: Vec<(Bound<u64>, Bound<u64>)> = range_header
            .satisfiable_ranges(
                u64::MAX, /* technically, we should pass in the length of the file to protect
                           * against inputs that are too large, but it's a
                           * bit tricky to get at this point */
            )
            .collect();
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
            .api
            .get_file_range(&host, request_id, origin, component, file_storage_id, range)
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
            Body::from_stream(stream),
        )
            .into_response());
    }

    let FileStream {
        sha256,
        content_type,
        content_length,
        stream,
    } = st
        .api
        .get_file(&host, request_id, origin, component, file_storage_id)
        .await?;
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
        Body::from_stream(stream),
    )
        .into_response())
}

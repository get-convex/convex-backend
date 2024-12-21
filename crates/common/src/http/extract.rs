use std::time::Instant;

use async_trait::async_trait;
use axum::{
    extract::{
        FromRequest,
        FromRequestParts,
        Request,
    },
    http::request::Parts,
    response::{
        IntoResponse,
        Response,
    },
};
use bytes::Bytes;
use errors::ErrorMetadata;
use http::HeaderMap;
use minitrace::{
    future::FutureExt,
    Span,
};
use serde::{
    de::DeserializeOwned,
    Serialize,
};

use crate::http::HttpResponseError;

pub struct RequestInitTime(pub Instant);

#[async_trait]
impl<S> FromRequestParts<S> for RequestInitTime
where
    S: Send + Sync,
{
    type Rejection = HttpResponseError;

    async fn from_request_parts(_: &mut Parts, _: &S) -> Result<Self, Self::Rejection> {
        Ok(Self(Instant::now()))
    }
}

pub struct Path<T>(pub T);

/// Wrapper type around axum::extract::Path that uses HttpResponseError instead
/// of PathRejection to make sure we get propper logging / error reporting.
#[async_trait]
impl<S, T> FromRequestParts<S> for Path<T>
where
    T: DeserializeOwned + Send,
    S: Send + Sync,
{
    type Rejection = HttpResponseError;

    async fn from_request_parts(req: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        #[allow(clippy::disallowed_types)]
        let t = axum::extract::Path::<T>::from_request_parts(req, state)
            .await
            .map_err(|e| {
                anyhow::anyhow!(ErrorMetadata::bad_request("BadPathArgs", e.to_string()))
            })?;
        Ok(Self(t.0))
    }
}

pub struct Query<T>(pub T);

/// Wrapper type around axum::extract::Query that uses HttpResponseError instead
/// of PathRejection to make sure we get propper logging / error reporting.
#[async_trait]
impl<S, T> FromRequestParts<S> for Query<T>
where
    T: DeserializeOwned + Send,
    S: Send + Sync,
{
    type Rejection = HttpResponseError;

    async fn from_request_parts(req: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        #[allow(clippy::disallowed_types)]
        let t = axum::extract::Query::<T>::from_request_parts(req, state)
            .await
            .map_err(|e| {
                anyhow::anyhow!(ErrorMetadata::bad_request("BadQueryArgs", e.to_string()))
            })?;
        Ok(Self(t.0))
    }
}

pub struct Json<T>(pub T);

/// Fork of axum::Json that uses HttpResponseError instead of JsonRejection to
/// make sure we get propper logging / error reporting and integrates with our
/// tracing framework.
#[async_trait]
impl<S, T> FromRequest<S> for Json<T>
where
    T: DeserializeOwned,
    S: Send + Sync,
{
    type Rejection = HttpResponseError;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        if !json_content_type(req.headers()) {
            let err = anyhow::anyhow!(ErrorMetadata::bad_request(
                "BadJsonBody",
                "Expected request with `Content-Type: application/json`",
            ));
            return Err(err.into());
        }
        let bytes = Bytes::from_request(req, state)
            .in_span(Span::enter_with_local_parent("buffering_body"))
            .await
            .map_err(|e| {
                anyhow::anyhow!(ErrorMetadata::bad_request("BadJsonBody", e.body_text()))
            })?;

        let t = {
            let _span = Span::enter_with_local_parent("parse_json");
            #[allow(clippy::disallowed_types)]
            axum::Json::<T>::from_bytes(&bytes)
                .map_err(|e| {
                    anyhow::anyhow!(ErrorMetadata::bad_request("BadJsonBody", e.body_text()))
                })?
                .0
        };
        Ok(Self(t))
    }
}

fn json_content_type(headers: &HeaderMap) -> bool {
    let Some(content_type) = headers.get(http::header::CONTENT_TYPE) else {
        return false;
    };
    let Ok(content_type) = content_type.to_str() else {
        return false;
    };
    let Ok(mime) = content_type.parse::<::mime::Mime>() else {
        return false;
    };
    mime.type_() == "application"
        && (mime.subtype() == "json" || mime.suffix().is_some_and(|name| name == "json"))
}

impl<T> IntoResponse for Json<T>
where
    T: Serialize,
{
    fn into_response(self) -> Response {
        axum::Json(self.0).into_response()
    }
}

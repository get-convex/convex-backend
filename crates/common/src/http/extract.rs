use std::time::Instant;

use async_trait::async_trait;
use axum::{
    body::HttpBody,
    extract::{
        FromRequest,
        FromRequestParts,
    },
    http::request::{
        Parts,
        Request,
    },
    response::{
        IntoResponse,
        Response,
    },
    BoxError,
};
use errors::ErrorMetadata;
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

/// Wrapper type around axum::Json that uses HttpResponseError instead
/// of JsonRejection to make sure we get propper logging / error reporting.
#[async_trait]
impl<S, B, T> FromRequest<S, B> for Json<T>
where
    T: DeserializeOwned,
    B: HttpBody + Send + 'static,
    B::Data: Send,
    B::Error: Into<BoxError>,
    S: Send + Sync,
{
    type Rejection = HttpResponseError;

    async fn from_request(req: Request<B>, state: &S) -> Result<Self, Self::Rejection> {
        #[allow(clippy::disallowed_types)]
        let t = axum::Json::<T>::from_request(req, state)
            .await
            .map_err(|e| {
                anyhow::anyhow!(ErrorMetadata::bad_request("BadJsonBody", e.body_text()))
            })?;
        Ok(Self(t.0))
    }
}

impl<T> IntoResponse for Json<T>
where
    T: Serialize,
{
    fn into_response(self) -> Response {
        axum::Json(self.0).into_response()
    }
}

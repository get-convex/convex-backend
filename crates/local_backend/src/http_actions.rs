use async_trait::async_trait;
use axum::{
    body::BoxBody,
    debug_handler,
    extract::{
        FromRequest,
        State,
    },
    response::{
        IntoResponse,
        Response,
    },
    routing::{
        get,
        MethodRouter,
    },
    RequestExt,
};
use common::{
    http::{
        ExtractRequestId,
        HttpResponseError,
    },
    types::FunctionCaller,
};
use futures::TryStreamExt;
use http::{
    Method,
    Uri,
};
use isolate::{
    HttpActionRequest,
    HttpActionRequestHead,
};
use keybroker::Identity;
use url::Url;

use crate::{
    authentication::TryExtractIdentity,
    LocalAppState,
};

pub struct ExtractHttpRequestMetadata(pub HttpActionRequest);

#[async_trait]
impl FromRequest<LocalAppState, axum::body::Body> for ExtractHttpRequestMetadata {
    type Rejection = HttpResponseError;

    async fn from_request(
        mut req: axum::http::Request<axum::body::Body>,
        st: &LocalAppState,
    ) -> Result<Self, Self::Rejection> {
        let uri = req
            .extract_parts::<Uri>()
            .await
            .map_err(|e| anyhow::anyhow!(e))?;
        let headers = req.headers().clone();

        let method = req.method().clone();

        let url =
            Url::parse(&format!("{}{}", st.site_origin, uri)).map_err(|e| anyhow::anyhow!(e))?;

        if method == Method::GET || method == Method::OPTIONS || method == Method::HEAD {
            return Ok(ExtractHttpRequestMetadata(HttpActionRequest {
                head: HttpActionRequestHead {
                    headers,
                    url,
                    method,
                },
                body: None,
            }));
        }

        let body = req.into_body();

        Ok(ExtractHttpRequestMetadata(HttpActionRequest {
            head: HttpActionRequestHead {
                headers,
                url,
                method,
            },
            body: Some(Box::pin(body.map_err(|e| e.into()))),
        }))
    }
}

#[debug_handler]
pub async fn http_any_method(
    State(st): State<LocalAppState>,
    TryExtractIdentity(identity_result): TryExtractIdentity,
    ExtractRequestId(request_id): ExtractRequestId,
    ExtractHttpRequestMetadata(http_request_metadata): ExtractHttpRequestMetadata,
) -> Result<impl IntoResponse, HttpResponseError> {
    // All HTTP actions run the default export of the http.js path.
    let udf_path = "http.js".parse()?;
    // The `Authorization` header for the request may contain a token corresponding
    // to Convex auth, or it could be something separate managed by the developer.
    // Try to extract the identity based on the Convex auth, but allow the request
    // to go through if the header does not seem to specify Convex auth.
    let identity = identity_result.unwrap_or(Identity::Unknown);

    let udf_return = st
        .application
        .http_action_udf(
            request_id,
            udf_path,
            http_request_metadata,
            identity,
            FunctionCaller::HttpEndpoint,
        )
        .await?;

    // TODO: For other endpoints, log_lines is conditioned on the `debug` query
    // param, and we likely want something similar here.
    // For now, just omit them.

    Ok(HttpActionResponse(udf_return))
}

pub fn http_action_handler() -> MethodRouter<LocalAppState> {
    get(http_any_method)
        .post(http_any_method)
        .delete(http_any_method)
        .patch(http_any_method)
        .put(http_any_method)
        .options(http_any_method)
}

pub struct HttpActionResponse(isolate::HttpActionResponse);

impl IntoResponse for HttpActionResponse {
    fn into_response(self) -> Response<BoxBody> {
        let response = self.0;
        let status = response.status();
        let headers = response.headers;
        match response.body {
            Some(body) => (status, headers, body).into_response(),
            None => (status, headers).into_response(),
        }
    }
}

use std::sync::Arc;

use anyhow::Context;
use application::api::ApplicationApi;
use async_trait::async_trait;
use axum::{
    body::{
        BoxBody,
        Bytes,
        StreamBody,
    },
    debug_handler,
    extract::{
        FromRequest,
        Host,
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
    RequestId,
};
use futures::{
    channel::mpsc,
    stream::BoxStream,
    FutureExt,
    StreamExt,
    TryStreamExt,
};
use futures_async_stream::try_stream;
use http::{
    header::FORWARDED,
    HeaderMap,
    Method,
    StatusCode,
};
use isolate::{
    HttpActionRequest,
    HttpActionRequestHead,
    HttpActionResponsePart,
    HttpActionResponseStreamer,
};
use keybroker::Identity;
use sync_types::UdfPath;
use url::Url;

use crate::{
    authentication::TryExtractIdentity,
    RouterState,
};

pub struct ExtractHttpRequestMetadata(pub HttpActionRequest);

const X_FORWARDED_PROTO: &str = "x-forwarded-proto";

fn parse_forwarded(headers: &HeaderMap) -> Option<&str> {
    // if there are multiple `Forwarded` `HeaderMap::get` will return the first one
    let forwarded_values = headers.get(FORWARDED)?.to_str().ok()?;

    // get the first set of values
    let first_value = forwarded_values.split(',').next()?;

    // find the value of the `host` field
    first_value.split(';').find_map(|pair| {
        let (key, value) = pair.split_once('=')?;
        key.trim()
            .eq_ignore_ascii_case("proto")
            .then(|| value.trim().trim_matches('"'))
    })
}

#[async_trait]
impl FromRequest<RouterState, axum::body::Body> for ExtractHttpRequestMetadata {
    type Rejection = HttpResponseError;

    async fn from_request(
        mut req: axum::http::Request<axum::body::Body>,
        _: &RouterState,
    ) -> Result<Self, Self::Rejection> {
        // Assume HTTP if neither `X-Forwarded-Proto` nor `Forwarded` headers give us a
        // protocol. This is a reasonable default as the only way to get HTTPS
        // with the backend is via a TLS-terminating proxy which should be
        // setting one of these headers.
        let scheme = req
            .headers()
            .get(X_FORWARDED_PROTO)
            .and_then(|h| h.to_str().ok())
            .or_else(|| parse_forwarded(req.headers()))
            .unwrap_or("http")
            .to_owned();
        let host = req
            .extract_parts::<Host>()
            .await
            .context("Host header not present")?
            .0;
        let uri = req.uri();
        let headers = req.headers().clone();
        let method = req.method().clone();

        // Construct the URL we provide in the HTTP request object.
        let url = Url::parse(&format!("{scheme}://{host}{uri}")).context("Invalid URL")?;

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

#[minitrace::trace(properties = { "udf_type": "http_action"})]
#[debug_handler]
pub async fn http_any_method(
    State(st): State<RouterState>,
    TryExtractIdentity(identity_result): TryExtractIdentity,
    ExtractRequestId(request_id): ExtractRequestId,
    Host(host): Host,
    ExtractHttpRequestMetadata(http_request_metadata): ExtractHttpRequestMetadata,
) -> Result<impl IntoResponse, HttpResponseError> {
    // All HTTP actions run the default export of the http.js path.
    let path = "http.js".parse()?;
    let mut http_response_stream = stream_http_response(
        host,
        path,
        request_id,
        http_request_metadata,
        identity_result,
        st.api.clone(),
    );
    let head = http_response_stream.try_next().await?;
    let Some(HttpActionResponsePart::Head(response_head)) = head else {
        return Err(anyhow::anyhow!("Did not receive HTTP response head first").into());
    };
    let body = http_response_stream.map(|p| match p {
        Ok(HttpActionResponsePart::BodyChunk(bytes)) => Ok(bytes),
        Err(e) => Err(e),
        _ => Err(anyhow::anyhow!(
            "Unexpected element in HTTP response stream"
        )),
    });

    Ok(HttpActionResponse {
        status: response_head.status,
        headers: response_head.headers,
        body: Box::pin(body),
    })
}

#[try_stream(ok=HttpActionResponsePart, error=anyhow::Error, boxed)]
async fn stream_http_response(
    host: String,
    path: UdfPath,
    request_id: RequestId,
    http_request_metadata: HttpActionRequest,
    identity_result: anyhow::Result<Identity>,
    application: Arc<dyn ApplicationApi>,
) {
    // The `Authorization` header for the request may contain a token corresponding
    // to Convex auth, or it could be something separate managed by the developer.
    // Try to extract the identity based on the Convex auth, but allow the request
    // to go through if the header does not seem to specify Convex auth.
    let identity = identity_result.unwrap_or(Identity::Unknown);
    let (http_response_sender, mut http_response_receiver) = mpsc::unbounded();
    let mut run_action_fut = Box::pin(
        application
            .execute_http_action(
                &host,
                request_id,
                path.into(),
                http_request_metadata,
                identity,
                FunctionCaller::HttpEndpoint,
                HttpActionResponseStreamer::new(http_response_sender),
            )
            .fuse(),
    );
    loop {
        let next_part = async {
            let v: Option<HttpActionResponsePart> = futures::select! {
                result = http_response_receiver.select_next_some() => {
                    Some(result)
                },
                func_result = run_action_fut => {
                    match func_result {
                        Ok(_) => None,
                        Err(e) => return Err(e)
                    }
                }
            };
            Ok(v)
        };
        match next_part.await? {
            Some(part) => yield part,
            None => break,
        }
    }
    while let Some(part) = http_response_receiver.next().await {
        yield part
    }
}

pub fn http_action_handler() -> MethodRouter<RouterState> {
    get(http_any_method)
        .post(http_any_method)
        .delete(http_any_method)
        .patch(http_any_method)
        .put(http_any_method)
        .options(http_any_method)
}

pub struct HttpActionResponse {
    pub body: BoxStream<'static, Result<Bytes, anyhow::Error>>,
    pub status: StatusCode,
    pub headers: HeaderMap,
}

impl IntoResponse for HttpActionResponse {
    fn into_response(self) -> Response<BoxBody> {
        let status = self.status;
        let headers = self.headers;
        let body = StreamBody::new(self.body);
        (status, headers, body).into_response()
    }
}

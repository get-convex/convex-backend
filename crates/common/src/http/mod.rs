use std::{
    borrow::Cow,
    convert::Infallible,
    fmt,
    future::Future,
    net::SocketAddr,
    pin::Pin,
    str,
    sync::LazyLock,
    time::{
        Duration,
        Instant,
    },
};

use ::metrics::{
    CONVEX_METRICS_REGISTRY,
    SERVER_VERSION_STR,
    SERVICE_NAME,
};
use anyhow::Context;
use async_trait::async_trait;
#[cfg(any(test, feature = "testing"))]
use axum::body::HttpBody;
use axum::{
    body::{
        Body,
        BoxBody,
    },
    error_handling::HandleErrorLayer,
    extract::{
        connect_info::IntoMakeServiceWithConnectInfo,
        FromRequestParts,
        State,
    },
    http::{
        Response,
        StatusCode,
    },
    response::IntoResponse,
    routing::get,
    BoxError,
    RequestPartsExt,
    Router,
};
use errors::{
    ErrorCode,
    ErrorMetadata,
    ErrorMetadataAnyhowExt,
};
use futures::{
    stream::BoxStream,
    Stream,
    StreamExt,
};
use http::{
    header::{
        HeaderName,
        HeaderValue,
        ACCEPT,
        AUTHORIZATION,
        CONTENT_TYPE,
        REFERER,
        USER_AGENT,
    },
    request::Parts,
    HeaderMap,
    Method,
};
use hyper::server::conn::AddrIncoming;
use itertools::Itertools;
use maplit::btreemap;
use minitrace::future::FutureExt;
use prometheus::TextEncoder;
use sentry::integrations::tower as sentry_tower;
use serde::{
    Deserialize,
    Serialize,
};
use tokio::net::TcpSocket;
use tower::{
    timeout::TimeoutLayer,
    ServiceBuilder,
};
use tower_http::cors::{
    AllowOrigin,
    CorsLayer,
};
use url::Url;

use self::metrics::log_http_request;
use crate::{
    errors::report_error,
    knobs::HTTP_SERVER_TCP_BACKLOG,
    metrics::log_client_version_unsupported,
    minitrace_helpers::get_sampled_span,
    version::{
        ClientVersion,
        ClientVersionState,
    },
    RequestId,
};

pub mod extract;
pub mod fetch;

const MAX_HTTP2_STREAMS: u32 = 1024;

pub use sync_types::headers::{
    DEPRECATION_MSG_HEADER_NAME,
    DEPRECATION_STATE_HEADER_NAME,
};
use value::heap_size::HeapSize;
mod metrics {
    use std::time::Duration;

    use metrics::{
        log_distribution_with_labels,
        register_convex_histogram,
        MetricLabel,
    };

    use crate::version::SERVER_VERSION_STR;

    register_convex_histogram!(
        HTTP_HANDLE_DURATION_SECONDS,
        "Time to handle an HTTP request",
        &[
            "endpoint",
            "method",
            "status",
            "client_version",
            "server_version"
        ]
    );

    pub fn log_http_request(
        client_version: &str,
        route: &str,
        method: &str,
        status: &str,
        duration: Duration,
    ) {
        // There are a lot of labels in here and some (e.g., client_version) are
        // pretty high cardinality. If this gets too expensive we can split out
        // separate logging just for client version.
        let labels = vec![
            MetricLabel::new("endpoint", route),
            MetricLabel::new("method", method),
            MetricLabel::new("status", status),
            MetricLabel::new("client_version", client_version),
            MetricLabel::new("server_version", &*SERVER_VERSION_STR),
        ];
        log_distribution_with_labels(
            &HTTP_HANDLE_DURATION_SECONDS,
            duration.as_secs_f64(),
            labels,
        );
    }
}

#[allow(clippy::declare_interior_mutable_const)]
pub const APPLICATION_JSON_CONTENT_TYPE: HeaderValue = HeaderValue::from_static("application/json");

#[derive(Debug, PartialEq, Eq)]
pub struct HttpRequest {
    pub headers: HeaderMap,
    pub url: Url,
    pub method: Method,
    pub body: Option<Vec<u8>>,
}

impl From<HttpRequest> for HttpRequestStream {
    fn from(value: HttpRequest) -> Self {
        let body: Pin<
            Box<dyn Stream<Item = anyhow::Result<bytes::Bytes>> + Sync + Send + 'static>,
        > = if let Some(b) = value.body {
            Box::pin(futures::stream::once(async move {
                Ok::<_, anyhow::Error>(bytes::Bytes::from(b))
            }))
        } else {
            Box::pin(futures::stream::empty())
        };

        Self {
            headers: value.headers,
            url: value.url,
            method: value.method,
            body,
        }
    }
}

impl HttpRequestStream {
    #[cfg(any(test, feature = "testing"))]
    pub async fn into_http_request(mut self) -> anyhow::Result<HttpRequest> {
        use futures::TryStreamExt;

        let mut body = vec![];
        while let Some(chunk) = self.body.try_next().await? {
            body.append(&mut chunk.to_vec());
        }

        Ok(HttpRequest {
            headers: self.headers,
            url: self.url,
            method: self.method,
            body: Some(body),
        })
    }
}

impl HeapSize for HttpRequest {
    fn heap_size(&self) -> usize {
        // Assume heap size is dominated by body (because the rest is annoying
        // to calculate).
        self.body.as_ref().map_or(0, |body| body.len())
    }
}

#[cfg(any(test, feature = "testing"))]
impl Arbitrary for HttpRequest {
    type Parameters = ();

    type Strategy = impl Strategy<Value = HttpRequest>;

    fn arbitrary_with(_args: Self::Parameters) -> Self::Strategy {
        use proptest::prelude::*;
        use proptest_http::{
            ArbitraryHeaderMap,
            ArbitraryMethod,
            ArbitraryUri,
        };
        prop_compose! {
            fn inner()(
                ArbitraryHeaderMap(headers) in any::<ArbitraryHeaderMap>(),
                ArbitraryMethod(method) in any::<ArbitraryMethod>(),
                ArbitraryUri(uri) in any::<ArbitraryUri>(),
                body in any::<Option<Vec<u8>>>()) -> anyhow::Result<HttpRequest> {
                    let origin: String = "http://example-deployment.convex.site/".to_string();
                    let path_and_query: String =  uri.path_and_query().ok_or_else(|| anyhow::anyhow!("No path and query"))?.to_string();
                    let url: Url = Url::parse(&(origin + &path_and_query))?;
                Ok(HttpRequest {
                    headers,
                    method,
                    url,
                    body
                })
            }
        };
        inner().prop_filter_map("Invalid HttpRequest", |r| r.ok())
    }
}

pub struct HttpRequestStream {
    pub headers: HeaderMap,
    pub url: Url,
    pub method: Method,
    pub body: Pin<Box<dyn Stream<Item = anyhow::Result<bytes::Bytes>> + Sync + Send + 'static>>,
}

impl std::fmt::Debug for HttpRequestStream {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HttpRequestStream")
            .field("headers", &self.headers)
            .field("url", &self.url)
            .field("method", &self.method)
            .finish()
    }
}

#[derive(Debug)]
pub struct HttpResponse {
    pub body: Option<Vec<u8>>,
    pub status: StatusCode,
    pub headers: HeaderMap,
    pub url: Option<Url>,
}

impl HttpResponse {
    pub fn new(
        status: StatusCode,
        headers: HeaderMap,
        body: Option<Vec<u8>>,
        url: Option<Url>,
    ) -> Self {
        Self {
            body,
            status,
            headers,
            url,
        }
    }
}

impl From<HttpResponse> for HttpResponseStream {
    fn from(value: HttpResponse) -> Self {
        Self {
            body: value
                .body
                .map(|b| futures::stream::once(async move { Ok(bytes::Bytes::from(b)) }).boxed()),
            status: value.status,
            headers: value.headers,
            url: value.url,
        }
    }
}

#[cfg(any(test, feature = "testing"))]
impl Arbitrary for HttpResponse {
    type Parameters = ();

    type Strategy = impl Strategy<Value = HttpResponse>;

    fn arbitrary_with(_args: Self::Parameters) -> Self::Strategy {
        use proptest::prelude::*;
        use proptest_http::{
            ArbitraryHeaderMap,
            ArbitraryStatusCode,
            ArbitraryUri,
        };
        prop_compose! {
            fn inner()(
                ArbitraryHeaderMap(headers) in any::<ArbitraryHeaderMap>(),
                ArbitraryStatusCode(status) in any::<ArbitraryStatusCode>(),
                ArbitraryUri(uri) in any::<ArbitraryUri>(),
                body in any::<Option<Vec<u8>>>()) -> anyhow::Result<HttpResponse> {
                    let origin: String = "http://example-deployment.convex.site/".to_string();
                    let path_and_query: String =  uri.path_and_query().ok_or_else(|| anyhow::anyhow!("No path and query"))?.to_string();
                    let url: Url = Url::parse(&(origin + &path_and_query))?;
                Ok(HttpResponse {
                    status,
                    headers,
                    body,
                    url: Some(url),
                })
            }
        };
        inner().prop_filter_map("Invalid HttpEndpoitnRequest", |r| r.ok())
    }
}

pub struct HttpResponseStream {
    pub body: Option<BoxStream<'static, anyhow::Result<bytes::Bytes>>>,
    pub status: StatusCode,
    pub headers: HeaderMap,
    pub url: Option<Url>,
}

impl HttpResponseStream {
    pub async fn into_http_response(self) -> anyhow::Result<HttpResponse> {
        let body = if let Some(mut body) = self.body {
            let mut bytes = vec![];
            while let Some(chunk) = body.next().await.transpose()? {
                bytes.append(&mut chunk.to_vec());
            }
            Some(bytes)
        } else {
            None
        };

        Ok(HttpResponse {
            body,
            status: self.status,
            headers: self.headers,
            url: self.url,
        })
    }
}

/// Transforms a common::http::HttpResponseStream into a
/// anyhow::Result<HttpResponseStream>, categorizing HTTP status code errors
/// into the ErrorMetadata data model. If no such status code is extractable,
/// the error is left uncategorized with ErrorMetadata.
pub fn categorize_http_response_stream(
    response: HttpResponseStream,
) -> anyhow::Result<HttpResponseStream> {
    if !(response.status.is_server_error() || response.status.is_client_error()) {
        return Ok(response);
    };

    let Some(error_code) = ErrorCode::from_http_status_code(response.status) else {
        anyhow::bail!(
            "Http request to {:?} failed with status code {}",
            response.url,
            response.status
        );
    };
    let canonical_reason = response.status.canonical_reason().unwrap_or("Unknown");

    Err(ErrorMetadata {
        code: error_code,
        short_msg: "RequestFailed".into(),
        msg: canonical_reason.into(),
    }
    .into())
}

#[cfg(any(test, feature = "testing"))]
use proptest::prelude::*;

#[cfg(any(test, feature = "testing"))]
fn status_code_strategy() -> impl Strategy<Value = StatusCode> {
    proptest_http::ArbitraryStatusCode::arbitrary().prop_map(|v| v.0)
}

/// `HttpError` is used as a vehicle for getting client facing error messages
/// to clients on the HTTP protocol. Errors that are tagged with ErrorMetadata
/// can be used to build these.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(any(test, feature = "testing"), derive(proptest_derive::Arbitrary))]
pub struct HttpError {
    /// HTTP Status Code
    #[cfg_attr(
        any(test, feature = "testing"),
        proptest(strategy = "status_code_strategy()")
    )]
    status_code: StatusCode,
    /// Human-readable error code sent in HTTP response
    error_code: Cow<'static, str>,
    /// Detailed customer-facing error message sent in HTTP response
    msg: Cow<'static, str>,
}

impl HttpError {
    pub fn new<S, T>(status_code: StatusCode, error_code: S, msg: T) -> Self
    where
        S: Into<Cow<'static, str>>,
        T: Into<Cow<'static, str>>,
    {
        Self {
            status_code,
            error_code: error_code.into(),
            msg: msg.into(),
        }
    }

    pub fn status_code(&self) -> StatusCode {
        self.status_code
    }

    pub fn error_code(&self) -> &str {
        &self.error_code
    }

    pub fn message(&self) -> &str {
        &self.msg
    }

    pub fn into_response(self) -> Response<BoxBody> {
        (
            self.status_code,
            extract::Json(ResponseErrorMessage {
                code: self.error_code,
                message: self.msg,
            }),
        )
            .into_response()
    }

    // Tests might parse a response back into a message
    #[cfg(any(test, feature = "testing"))]
    pub async fn from_response<B>(response: Response<B>) -> Self
    where
        B: HttpBody,
        B::Error: fmt::Debug,
    {
        let (parts, body) = response.into_parts();
        let ResponseErrorMessage { code, message } = serde_json::from_slice(
            &hyper::body::to_bytes(body)
                .await
                .expect("Couldn't convert to bytes"),
        )
        .expect("Couldn't deserialize as json");

        Self {
            status_code: parts.status,
            error_code: code,
            msg: message,
        }
    }
}

/// `HttpResponseError` is used to convert `anyhow::Error` (and
/// `HttpError` inside it if present) into `http::Response` that is returned
/// from the HTTP middleware. All HTTP handlers should return
/// `HttpResponseError`s. Sentry errors are captured in the `IntoResponse` impl,
/// the exit point of the HTTP middleware.
#[derive(Debug)]
pub struct HttpResponseError {
    trace: anyhow::Error,
    http_error: HttpError,
}

impl const From<Infallible> for HttpResponseError {
    fn from(x: Infallible) -> Self {
        match x {}
    }
}

#[derive(Serialize, Deserialize)]
struct ResponseErrorMessage {
    code: Cow<'static, str>,
    message: Cow<'static, str>,
}

impl IntoResponse for HttpResponseError {
    fn into_response(mut self) -> Response<BoxBody> {
        // This is the only place we capture errors to sentry because it is the exit
        // point of the HTTP layer
        report_error(&mut self.trace);
        self.http_error.into_response()
    }
}

impl From<anyhow::Error> for HttpResponseError {
    fn from(err: anyhow::Error) -> HttpResponseError {
        let http_error = HttpError {
            status_code: err.http_status(),
            error_code: err.short_msg().to_string().into(),
            msg: err.msg().to_string().into(),
        };
        let trace = err.last_second_classification();
        Self { trace, http_error }
    }
}

pub trait RouteMapper: Send + Sync + Clone + 'static {
    fn map_route(&self, route: String) -> String;
}

#[derive(Clone)]
pub struct NoopRouteMapper;

impl RouteMapper for NoopRouteMapper {
    fn map_route(&self, route: String) -> String {
        route
    }
}

/// Router + Middleware for a Convex service
pub struct ConvexHttpService {
    router: Router<(), Body>,
}

impl ConvexHttpService {
    pub fn new<RM: RouteMapper>(
        router: Router<(), Body>,
        version: String,
        max_concurrency: usize,
        request_timeout: Duration,
        route_metric_mapper: RM,
    ) -> Self {
        let sentry_layer = ServiceBuilder::new()
            .layer(sentry_tower::NewSentryLayer::<_>::new_from_top())
            .layer(sentry_tower::SentryHttpLayer::new());

        let router = router
            .layer(
                ServiceBuilder::new()
                    // Order important. Log/stats first because they are infallible.
                    .layer(axum::middleware::from_fn(log_middleware))
                    .layer(axum::middleware::from_fn_with_state(
                        route_metric_mapper.clone(),
                        stats_middleware::<RM>,
                    ))
                    .layer(axum::middleware::from_fn(client_version_state_middleware))
                    .concurrency_limit(max_concurrency)
                    .layer(tower_cookies::CookieManagerLayer::new())
                    .layer(HandleErrorLayer::new(|_: BoxError| async {
                        StatusCode::REQUEST_TIMEOUT
                    }))
                    .layer(TimeoutLayer::new(request_timeout)),
            )
            // Middleware needn't apply to these routes
            .route("/version", get(move || async move { version }))
            .route("/metrics", get(metrics))
            .layer(sentry_layer);

        Self { router }
    }

    pub async fn serve<F: Future<Output = ()>>(
        self,
        addr: SocketAddr,
        shutdown: F,
    ) -> anyhow::Result<()> {
        let make_svc = self
            .router
            .into_make_service_with_connect_info::<SocketAddr>();
        serve_http(make_svc, addr, shutdown).await
    }

    #[cfg(any(test, feature = "testing"))]
    pub fn new_for_test(router: Router<(), Body>) -> Self {
        Self { router }
    }

    #[cfg(any(test, feature = "testing"))]
    pub fn router(&self) -> Router<(), Body> {
        self.router.clone()
    }
}

/// Serves an HTTP server using the given service.
pub async fn serve_http<F>(
    service: IntoMakeServiceWithConnectInfo<Router, SocketAddr>,
    addr: SocketAddr,
    shutdown: F,
) -> anyhow::Result<()>
where
    F: Future<Output = ()>,
{
    // Set SO_REUSEADDR and a bounded TCP accept backlog for our server's listening
    // socket.
    let socket = TcpSocket::new_v4()?;
    socket.set_reuseaddr(true)?;
    socket.bind(addr)?;
    let listener = socket.listen(*HTTP_SERVER_TCP_BACKLOG)?;

    let mut incoming_sockets = AddrIncoming::from_listener(listener)?;
    // Set TCP_NODELAY on accepted connections.
    incoming_sockets.set_nodelay(true);
    // This setting is a bit of a `hyper`-specific hack to prevent a DDoS attack
    // from taking down the webserver. See https://github.com/hyperium/hyper/issues/1358 and
    // https://klau.si/blog/crashing-a-rust-hyper-server-with-a-denial-of-service-attack/ for more
    // details.
    incoming_sockets.set_sleep_on_errors(true);
    let addr = incoming_sockets.local_addr();

    tracing::info!("Listening on http://{}", addr);
    hyper::Server::builder(incoming_sockets)
        .http2_max_concurrent_streams(MAX_HTTP2_STREAMS)
        .serve(service)
        .with_graceful_shutdown(shutdown)
        .await?;
    tracing::info!("HTTP server shutdown complete");

    Ok(())
}

async fn client_version_state_middleware(
    ExtractClientVersion(client_version): ExtractClientVersion,
    req: http::request::Request<Body>,
    next: axum::middleware::Next<Body>,
) -> Result<impl IntoResponse, HttpResponseError> {
    let version_state = client_version.current_state();

    let mut resp = match &version_state {
        ClientVersionState::Unsupported(message) => {
            let message = message.lines().join(" ");
            log_client_version_unsupported(client_version.to_string());
            let http_err_resp: HttpResponseError = anyhow::anyhow!(ErrorMetadata::bad_request(
                "ClientVersionUnsupported",
                message,
            ))
            .into();
            http_err_resp.into_response()
        },
        _ => next.run(req).await,
    };

    match &version_state {
        ClientVersionState::Unsupported(message) | ClientVersionState::UpgradeRequired(message) => {
            let message = message.lines().join(" ");
            let headers = resp.headers_mut();
            let state_str = version_state.variant_name();
            headers.insert(
                HeaderName::from_static(DEPRECATION_STATE_HEADER_NAME),
                HeaderValue::from_str(state_str).context(format!(
                    "Failed to create deprecation state header value: {state_str}"
                ))?,
            );
            headers.insert(
                HeaderName::from_static(DEPRECATION_MSG_HEADER_NAME),
                HeaderValue::from_str(message.as_str()).context(format!(
                    "Failed to create deprecation msg header value: {message}"
                ))?,
            );
        },
        ClientVersionState::Supported => (),
    };
    Ok(resp)
}

pub async fn stats_middleware<RM: RouteMapper>(
    State(route_metric_mapper): State<RM>,
    matched_path: Option<axum::extract::MatchedPath>,
    ExtractRequestId(request_id): ExtractRequestId,
    ExtractClientVersion(client_version): ExtractClientVersion,
    req: http::request::Request<Body>,
    next: axum::middleware::Next<Body>,
) -> Result<impl IntoResponse, HttpResponseError> {
    let start = Instant::now();
    let method = req.method().clone();
    // tag with the route. 404s lack matched query path - and the
    // uri is generally unhelpful for metrics aggregation, so leave it out there.
    let route = matched_path
        .map(|r| r.as_str().to_owned())
        .unwrap_or("unknown".to_owned());

    // Configure tracing
    let root = {
        let mut rng = rand::thread_rng();
        get_sampled_span(
            route.as_str(),
            &mut rng,
            btreemap!["request_id".to_owned() => request_id.to_string()],
        )
    };

    let resp = next.run(req).in_span(root).await;

    let client_version_s = client_version.to_string();

    let route = route_metric_mapper.map_route(route);

    // Add the request_id to sentry
    sentry::configure_scope(|scope| scope.set_tag("request_id", request_id.clone()));

    log_http_request(
        &client_version_s,
        &route,
        method.as_str(),
        resp.status().as_str(),
        start.elapsed(),
    );

    Ok::<_, _>(resp)
}

#[allow(clippy::declare_interior_mutable_const)]
pub const CONVEX_CLIENT_HEADER: HeaderName = HeaderName::from_static("convex-client");

// The client version header to use for requests from this service.
pub static CONVEX_CLIENT_HEADER_VALUE: LazyLock<HeaderValue> = LazyLock::new(|| {
    let service_name = &*SERVICE_NAME;
    let server_version = &*SERVER_VERSION_STR;
    HeaderValue::from_str(&format!("{service_name}-{server_version}")).unwrap()
});

pub struct ExtractClientVersion(pub ClientVersion);

async fn client_version_from_req_parts(
    parts: &mut axum::http::request::Parts,
) -> anyhow::Result<ClientVersion> {
    let client_version = if let Some(version_header) = parts
        .headers
        .get(CONVEX_CLIENT_HEADER)
        .and_then(|h| h.to_str().ok().map(|s| s.to_string()))
    {
        version_header.parse::<ClientVersion>()?
    } else {
        #[derive(Deserialize, Default)]
        struct Params {
            client_version: Option<String>,
        }
        let Params { client_version } = parts
            .extract::<extract::Path<_>>()
            .await
            .map(|path| path.0)
            .unwrap_or_default();
        match client_version {
            None => ClientVersion::unknown(),
            Some(version) => ClientVersion::from_path_param(
                version.parse().map_err(|e| {
                    ErrorMetadata::bad_request(
                        "InvalidVersion",
                        format!("Failed to parse client version: {e}"),
                    )
                })?,
                parts.uri.path(),
            ),
        }
    };
    Ok(client_version)
}

#[async_trait]
impl<S> FromRequestParts<S> for ExtractClientVersion
where
    S: Send + Sync,
{
    type Rejection = HttpResponseError;

    async fn from_request_parts(
        parts: &mut axum::http::request::Parts,
        _state: &S,
    ) -> Result<Self, Self::Rejection> {
        let client_version = client_version_from_req_parts(parts).await.map_err(|e| {
            anyhow::anyhow!(ErrorMetadata::bad_request(
                "InvalidClientVersion",
                e.to_string(),
            ))
        })?;
        Ok(Self(client_version))
    }
}

#[allow(clippy::declare_interior_mutable_const)]
pub const CONVEX_REQUEST_ID_HEADER: HeaderName = HeaderName::from_static("convex-request-id");

pub struct ExtractRequestId(pub RequestId);

async fn request_id_from_req_parts(
    parts: &mut axum::http::request::Parts,
) -> anyhow::Result<RequestId> {
    if let Some(request_id_header) = parts
        .headers
        .get(CONVEX_REQUEST_ID_HEADER)
        .and_then(|h| h.to_str().ok().map(|s| s.to_string()))
    {
        request_id_header.parse::<RequestId>()
    } else {
        // Generate a new request_id
        let request_id = RequestId::new();
        parts
            .headers
            .insert(CONVEX_REQUEST_ID_HEADER, request_id.as_str().parse()?);
        Ok(request_id)
    }
}

#[async_trait]
impl<S> FromRequestParts<S> for ExtractRequestId
where
    S: Send + Sync,
{
    type Rejection = HttpResponseError;

    async fn from_request_parts(
        parts: &mut axum::http::request::Parts,
        _state: &S,
    ) -> Result<Self, Self::Rejection> {
        let request_id = request_id_from_req_parts(parts).await.map_err(|e| {
            anyhow::anyhow!(ErrorMetadata::bad_request(
                "InvalidRequestId",
                e.to_string(),
            ))
        })?;
        Ok(Self(request_id))
    }
}

async fn log_middleware<B: Send>(
    remote_addr: Option<axum::extract::ConnectInfo<SocketAddr>>,
    req: http::request::Request<B>,
    next: axum::middleware::Next<B>,
) -> Result<impl IntoResponse, HttpResponseError> {
    let site_id = ::std::env::var("CONVEX_SITE").unwrap_or_default();
    let start = Instant::now();

    let remote_addr = remote_addr.map(|connect_info| connect_info.0);
    let method = req.method().clone();
    let uri = req.uri().clone();
    let version = req.version();
    let get_header = |name: HeaderName| -> Option<String> {
        req.headers()
            .get(name)
            .and_then(|h| h.to_str().ok().map(|s| s.to_string()))
    };
    let referer = get_header(http::header::REFERER);
    let user_agent = get_header(http::header::USER_AGENT);

    let resp = next.run(req).await;

    tracing::info!(
        target: "convex-cloud-http",
        "[{}] {} \"{} {} {:?}\" {} \"{}\" \"{}\" {:?}",
        site_id,
        LogOptFmt(remote_addr),
        method,
        uri,
        version,
        resp.status().as_u16(),
        LogOptFmt(referer),
        LogOptFmt(user_agent),
        start.elapsed(),
    );
    Ok(resp)
}

struct LogOptFmt<T>(Option<T>);

impl<T: fmt::Display> fmt::Display for LogOptFmt<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if let Some(ref t) = self.0 {
            fmt::Display::fmt(t, f)
        } else {
            f.write_str("-")
        }
    }
}

// CLI endpoints can be used from browser IDEs (e.g. StackBlitz), which send
// different headers.
pub async fn cli_cors() -> CorsLayer {
    CorsLayer::new()
        .allow_headers(vec![
            CONTENT_TYPE,
            AUTHORIZATION,
            ACCEPT,
            REFERER,
            USER_AGENT,
            CONVEX_CLIENT_HEADER,
        ])
        .allow_credentials(true)
        .allow_methods(vec![
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::OPTIONS,
            Method::DELETE,
        ])
        // `predicate` of `true` allows all origins without allow-origin *,
        // since that wouldn't allow use of credentials.
        .allow_origin(
            AllowOrigin::predicate(|_origin: &HeaderValue, _request_head: &Parts| {
                true
            }),
        )
        .max_age(Duration::from_secs(86400))
}

/// Collects metrics and returns them in the Prometheus exposition format.
/// Returns an empty response if no metrics have been recorded yet.
/// Note that registered metrics will not show here until recorded at least
/// once.
pub async fn metrics() -> Result<impl IntoResponse, HttpResponseError> {
    let encoder = TextEncoder::new();
    let metrics = CONVEX_METRICS_REGISTRY.gather();
    let output = encoder
        .encode_to_string(&metrics)
        .map_err(anyhow::Error::from)?;
    Ok(output)
}

#[cfg(test)]
mod tests {
    use axum::response::IntoResponse;
    use errors::{
        ErrorMetadata,
        INTERNAL_SERVER_ERROR,
        INTERNAL_SERVER_ERROR_MSG,
    };
    use http::StatusCode;

    use super::HttpResponseError;
    use crate::http::HttpError;

    #[tokio::test]
    async fn test_http_response_error_internal_server_error() {
        let err_text = "some random error";
        let err = anyhow::anyhow!(err_text);
        let err_clone = anyhow::anyhow!(err_text);
        let http_response_err: HttpResponseError = err.into();
        // Check the backtraces are the same
        assert_eq!(http_response_err.trace.to_string(), err_clone.to_string());
        // Check the HttpError is an internal server error
        assert_eq!(
            HttpError::new(
                StatusCode::INTERNAL_SERVER_ERROR,
                INTERNAL_SERVER_ERROR,
                INTERNAL_SERVER_ERROR_MSG,
            ),
            http_response_err.http_error
        );

        // Check the Response contains the ResponseErrorMessage
        let http_response_err: HttpResponseError = err_clone.into();
        let response = http_response_err.into_response();
        let error = HttpError::from_response(response).await;
        assert_eq!(error.status_code(), StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(error.error_code(), "InternalServerError");
        assert_eq!(error.msg, INTERNAL_SERVER_ERROR_MSG);
    }

    #[tokio::test]
    async fn test_http_error_400() {
        let status_code = StatusCode::BAD_REQUEST;
        let error_code = "ErrorCode";
        let msg = "Nice error message!";
        let first_error = "some random error";
        let middle_error = ErrorMetadata::bad_request(error_code, msg);
        let last_error = "another random error";
        let err = anyhow::anyhow!(first_error)
            .context(middle_error.clone())
            .context(last_error);
        let err_clone = anyhow::anyhow!(first_error)
            .context(middle_error)
            .context(last_error);

        let http_response_err: HttpResponseError = err.into();
        // Check the HttpError in the middle of the stack matches the http_error that
        // the anyhow::Error is downcast to
        assert_eq!(
            HttpError::new(status_code, error_code, msg,),
            http_response_err.http_error
        );

        // Check the backtraces are the same - note that the full stack trace including
        // first_error, HttpError, and last_error, is preserved
        assert_eq!(http_response_err.trace.to_string(), err_clone.to_string());

        // Check the Response contains the ResponseErrorMessage
        let http_response_err: HttpResponseError = err_clone.into();
        let response = http_response_err.into_response();
        let error = HttpError::from_response(response).await;
        assert_eq!(error.status_code(), status_code);
        assert_eq!(error.error_code(), error_code);
        assert_eq!(error.message(), msg);
    }
}

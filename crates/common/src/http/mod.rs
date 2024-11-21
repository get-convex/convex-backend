use std::{
    borrow::Cow,
    convert::Infallible,
    fmt,
    future::Future,
    net::SocketAddr,
    ops::Deref,
    pin::Pin,
    str,
    sync::{
        Arc,
        LazyLock,
    },
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
use axum::{
    body::Body,
    error_handling::HandleErrorLayer,
    extract::{
        connect_info::IntoMakeServiceWithConnectInfo,
        FromRequestParts,
        Host,
        State,
    },
    response::{
        IntoResponse,
        Response,
    },
    routing::get,
    BoxError,
    RequestPartsExt,
    Router,
    ServiceExt,
};
use errors::{
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
    StatusCode,
    Uri,
};
use http_body_util::BodyExt;
use itertools::Itertools;
use minitrace::{
    future::FutureExt as _,
    prelude::SpanContext,
    Span,
};
use prometheus::{
    PullingGauge,
    TextEncoder,
};
use regex::Regex;
use sentry::integrations::tower as sentry_tower;
use serde::{
    Deserialize,
    Serialize,
};
use tokio::net::TcpSocket;
use tower::{
    limit::GlobalConcurrencyLimitLayer,
    timeout::TimeoutLayer,
    Layer,
    Service,
    ServiceBuilder,
};
use tower_http::cors::{
    AllowOrigin,
    CorsLayer,
};
use url::Url;
use utoipa::ToSchema;

use self::metrics::log_http_request;
use crate::{
    errors::report_error,
    knobs::HTTP_SERVER_TCP_BACKLOG,
    metrics::log_client_version_unsupported,
    runtime::TaskManager,
    version::{
        ClientVersion,
        ClientVersionState,
    },
    RequestId,
};

pub mod extract;
pub mod fetch;
pub mod fork_of_axum_serve;

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

    register_convex_histogram!(
        HTTP_HANDLE_DURATION_SECONDS,
        "Time to handle an HTTP request",
        &["endpoint", "method", "status", "client_version", "is_test"]
    );

    pub fn log_http_request(
        client_version: &str,
        route: &str,
        method: &str,
        status: &str,
        duration: Duration,
        is_test: bool,
    ) {
        // There are a lot of labels in here and some (e.g., client_version) are
        // pretty high cardinality. If this gets too expensive we can split out
        // separate logging just for client version.
        let labels = vec![
            MetricLabel::new("endpoint", route),
            MetricLabel::new("method", method),
            MetricLabel::new("status", status),
            MetricLabel::new("client_version", client_version),
            MetricLabel::new("is_test", is_test.to_string()),
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

// Components can mount other components' HTTP routers, so a child component's
// HTTP router may receive a different path from the original HTTP request. For
// example, let's say we mount a rate limiter's router at `/rl/` and the rate
// limiter has a route for "/index.html". Then, an incoming request for
// `/rl/index.html` will get passed to the rate limiter's router with the routed
// path of `/index.html`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoutedHttpPath(pub String);

impl Deref for RoutedHttpPath {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
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

    let canonical_reason = response.status.canonical_reason().unwrap_or("Unknown");
    let Some(em) =
        ErrorMetadata::from_http_status_code(response.status, "RequestFailed", canonical_reason)
    else {
        anyhow::bail!(
            "Http request to {:?} failed with status code {} {}",
            response.url,
            response.status,
            canonical_reason,
        );
    };

    Err(em.into())
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

    pub fn into_response(self) -> Response {
        if self.msg.is_empty() && self.error_code.is_empty() {
            self.status_code.into_response()
        } else {
            (
                self.status_code,
                extract::Json(ResponseErrorMessage {
                    code: self.error_code,
                    message: self.msg,
                }),
            )
                .into_response()
        }
    }

    pub async fn error_message_from_bytes(
        bytes: &hyper::body::Bytes,
    ) -> anyhow::Result<(Cow<'static, str>, Cow<'static, str>)> {
        let ResponseErrorMessage { code, message } =
            serde_json::from_slice(bytes).context(format!(
                "Couldn't deserialize as json: {}",
                String::from_utf8_lossy(bytes)
            ))?;

        Ok((code, message))
    }

    pub async fn from_response(response: Response) -> anyhow::Result<Self> {
        let (parts, body) = response.into_parts();
        let (code, message) = Self::error_message_from_bytes(
            &body
                .collect()
                .await
                .expect("Couldn't collect body")
                .to_bytes(),
        )
        .await?;

        Ok(Self {
            status_code: parts.status,
            error_code: code,
            msg: message,
        })
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
    fn into_response(mut self) -> Response {
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

impl From<HttpResponseError> for anyhow::Error {
    fn from(value: HttpResponseError) -> Self {
        value.trace
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
    router: Router,
    meta_routes_enabled: bool,
    version: String,
    service_name: &'static str,
    _concurrency_gauge: Option<PullingGauge>,
}

impl ConvexHttpService {
    pub fn new<RM: RouteMapper>(
        router: Router,
        service_name: &'static str,
        version: String,
        max_concurrency: usize,
        request_timeout: Duration,
        route_metric_mapper: RM,
    ) -> Self {
        let sentry_layer = ServiceBuilder::new()
            .layer(sentry_tower::NewSentryLayer::<_>::new_from_top())
            .layer(sentry_tower::SentryHttpLayer::new());
        let semaphore = Arc::new(tokio::sync::Semaphore::new(max_concurrency));
        let semaphore_ = semaphore.clone();
        let concurrency_gauge = PullingGauge::new(
            format!(
                "{}_http_service_concurrent_requests",
                service_name.replace('-', "_")
            ),
            "The number of currently outstanding requests on the ConvexHttpService",
            Box::new(move || (max_concurrency - semaphore_.available_permits()) as f64),
        )
        .expect("Invalid gauge initialization");
        if let Err(e) = CONVEX_METRICS_REGISTRY.register(Box::new(concurrency_gauge.clone())) {
            tracing::error!("Failed to register request concurrency gauge for {service_name}: {e}");
        }

        let router = router
            .layer(
                ServiceBuilder::new()
                    // Order important. Log/stats first because they are infallible.
                    .layer(axum::middleware::from_fn(tokio_instrumentation_middleware))
                    .layer(axum::middleware::from_fn(log_middleware))
                    .layer(axum::middleware::from_fn_with_state(
                        route_metric_mapper.clone(),
                        stats_middleware::<RM>,
                    ))
                    .layer(axum::middleware::from_fn(client_version_state_middleware))
                    .layer(GlobalConcurrencyLimitLayer::with_semaphore(semaphore))
                    .layer(tower_cookies::CookieManagerLayer::new())
                    .layer(HandleErrorLayer::new(|_: BoxError| async {
                        StatusCode::REQUEST_TIMEOUT
                    }))
                    .layer(TimeoutLayer::new(request_timeout)),
            )
            .layer(sentry_layer);

        Self {
            router,
            version,
            _concurrency_gauge: Some(concurrency_gauge),
            service_name,
            meta_routes_enabled: true,
        }
    }

    pub fn set_meta_routes_enabled(&mut self, enabled: bool) {
        self.meta_routes_enabled = enabled;
    }

    /// Routes not handled by the passed-in router.
    fn meta_routes(&self) -> Router {
        let version = self.version.clone();
        Router::new()
            .route("/version", get(move || async move { version }))
            .route("/metrics", get(metrics))
    }

    pub async fn serve<F: Future<Output = ()> + Send + 'static>(
        self,
        addr: SocketAddr,
        shutdown: F,
    ) -> anyhow::Result<()> {
        let extra = self.meta_routes();
        let mut router = self.router;
        if self.meta_routes_enabled {
            router = router.merge(extra);
        }
        let make_svc = router.into_make_service_with_connect_info::<SocketAddr>();
        tracing::info!("{} listening on {addr}", self.service_name);
        serve_http(make_svc, addr, shutdown).await
    }

    /// Apply `middleware_fn` to incoming requests *before* passing them to
    /// the router. Because the middleware is applied before routing, it is
    /// allowed to change the request URI and affect which route will be
    /// matched.
    pub async fn serve_with_middleware<F, Fut, Rejection>(
        self,
        addr: SocketAddr,
        shutdown: F,
        middleware_fn: impl FnMut(http::Request<Body>) -> Fut + Clone + Send + 'static,
    ) -> anyhow::Result<()>
    where
        F: Future<Output = ()> + Send + 'static,
        Fut: Future<Output = Result<http::Request<Body>, Rejection>> + Send + 'static,
        Rejection: IntoResponse + Send + 'static,
    {
        let middleware = axum::middleware::map_request(middleware_fn);
        let meta_router = self.meta_routes();
        let wrapped_svc = middleware.layer(self.router);

        tracing::info!("{} listening on {addr}", self.service_name);
        if self.meta_routes_enabled {
            // Fall back to the middleware-wrapped service if the request doesn't match the
            // meta router.
            serve_http(
                meta_router
                    .fallback_service(wrapped_svc)
                    .into_make_service_with_connect_info::<SocketAddr>(),
                addr,
                shutdown,
            )
            .await
        } else {
            // If we're not serving meta routes, simply serve the middleware-wrapped service
            serve_http(
                wrapped_svc.into_make_service_with_connect_info::<SocketAddr>(),
                addr,
                shutdown,
            )
            .await
        }
    }

    #[cfg(any(test, feature = "testing"))]
    pub fn new_for_test(router: Router) -> Self {
        Self {
            router,
            version: String::new(),
            meta_routes_enabled: true,
            service_name: "test-service",
            _concurrency_gauge: None,
        }
    }

    #[cfg(any(test, feature = "testing"))]
    pub fn router(&self) -> Router {
        self.router.clone()
    }
}

/// Serves an HTTP server using the given service.
pub async fn serve_http<F, R>(
    make_service: IntoMakeServiceWithConnectInfo<R, SocketAddr>,
    addr: SocketAddr,
    shutdown: F,
) -> anyhow::Result<()>
where
    R: Service<http::Request<Body>, Response = Response, Error = Infallible>
        + Send
        + Clone
        + 'static,
    <R as Service<http::Request<Body>>>::Future: Send,
    F: Future<Output = ()> + Send + 'static,
{
    // Set SO_REUSEADDR and a bounded TCP accept backlog for our server's listening
    // socket.
    let socket = TcpSocket::new_v4()?;
    socket.set_reuseaddr(true)?;
    // Set TCP_NODELAY on accepted connections.
    socket.set_nodelay(true)?;
    socket.bind(addr)?;
    let listener = socket.listen(*HTTP_SERVER_TCP_BACKLOG)?;

    fork_of_axum_serve::serve(listener, make_service)
        .with_graceful_shutdown(shutdown)
        .await?;
    Ok(())
}

async fn client_version_state_middleware(
    ExtractClientVersion(client_version): ExtractClientVersion,
    req: http::request::Request<Body>,
    next: axum::middleware::Next,
) -> Result<Response, HttpResponseError> {
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
    ExtractResolvedHostname(resolved_host): ExtractResolvedHostname,
    ExtractTraceparent(traceparent): ExtractTraceparent,
    req: http::request::Request<Body>,
    next: axum::middleware::Next,
) -> Result<impl IntoResponse, HttpResponseError> {
    let start = Instant::now();
    let method = req.method().clone();
    // tag with the route. 404s lack matched query path - and the
    // uri is generally unhelpful for metrics aggregation, so leave it out there.
    let route = matched_path
        .map(|r| r.as_str().to_owned())
        .unwrap_or("unknown".to_owned());

    // Sampling isn't done here, and should be done upstream
    let root = match traceparent {
        Some(span_ctx) => Span::root(route.to_owned(), span_ctx),
        None => Span::noop(),
    };
    let resp = next.run(req).in_span(root).await;

    let client_version_s = client_version.to_string();

    let route = route_metric_mapper.map_route(route);
    let is_test = resolved_host.instance_name.starts_with("test-");

    // Add the request_id to sentry
    sentry::configure_scope(|scope| scope.set_tag("request_id", request_id.clone()));

    log_http_request(
        &client_version_s,
        &route,
        method.as_str(),
        resp.status().as_str(),
        start.elapsed(),
        is_test,
    );

    Ok::<_, _>(resp)
}

pub struct InstanceNameExt(pub String);

#[derive(ToSchema, Copy, Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RequestDestination {
    ConvexCloud,
    ConvexSite,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ResolvedHostname {
    pub instance_name: String,
    pub destination: RequestDestination,
}

pub const CONVEX_DOMAIN_REGEX_INSTANCE_CAPTURE: &str = "instance";
pub const CONVEX_DOMAIN_REGEX_TLD_CAPTURE: &str = "tld";
pub static CONVEX_DOMAIN_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?<instance>[A-Za-z0-9-]+)(\.[A-Za-z0-9-]+)?\.convex\.(?<tld>cloud|site)$")
        .unwrap()
});

pub fn resolve_convex_domain(uri: &Uri) -> anyhow::Result<Option<ResolvedHostname>> {
    let host = uri.host().context("URI does not have valid host")?;
    if let Some(captures) = CONVEX_DOMAIN_REGEX.captures(host) {
        let instance_name = captures[CONVEX_DOMAIN_REGEX_INSTANCE_CAPTURE].to_string();
        let destination = match &captures[CONVEX_DOMAIN_REGEX_TLD_CAPTURE] {
            "cloud" => RequestDestination::ConvexCloud,
            "site" => RequestDestination::ConvexSite,
            _ => unreachable!("Regex capture only matches cloud or site"),
        };
        return Ok(Some(ResolvedHostname {
            instance_name,
            destination,
        }));
    }
    Ok(None)
}

pub struct ExtractResolvedHostname(pub ResolvedHostname);

#[derive(Clone, Debug)]
pub struct OriginalHttpUri(pub Uri);

#[async_trait]
impl<S> FromRequestParts<S> for ExtractResolvedHostname {
    type Rejection = Infallible;

    async fn from_request_parts(
        parts: &mut axum::http::request::Parts,
        _state: &S,
    ) -> Result<Self, Self::Rejection> {
        // Check if we've already resolved this earlier in the stack.
        // We allow `Extension` here instead of `State` as we specifically want this
        // extraction to be fallible and optional.
        #[allow(clippy::disallowed_types)]
        if let Ok(axum::Extension(resolved)) =
            parts.extract::<axum::Extension<ResolvedHostname>>().await
        {
            return Ok(ExtractResolvedHostname(resolved));
        }
        // Try to parse the Host header as a URI and then resolve it as a Convex domain
        let host = parts.extract::<Host>().await.map_err(anyhow::Error::from);
        if let Ok(Some(resolved)) = host
            .and_then(|Host(host)| Uri::try_from(host).map_err(anyhow::Error::from))
            .and_then(|uri| resolve_convex_domain(&uri))
        {
            return Ok(ExtractResolvedHostname(resolved));
        }

        // No luck -- fall back to `CONVEX_SITE` and assume `convex.cloud` as this is
        // likely a request to localhost.
        Ok(ExtractResolvedHostname(ResolvedHostname {
            instance_name: ::std::env::var("CONVEX_SITE").unwrap_or_default(),
            destination: RequestDestination::ConvexCloud,
        }))
    }
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

pub const TRACEPARENT_HEADER: &str = "traceparent";

pub struct ExtractTraceparent(pub Option<SpanContext>);

#[async_trait]
impl<S> FromRequestParts<S> for ExtractTraceparent
where
    S: Send + Sync,
{
    type Rejection = Infallible;

    async fn from_request_parts(
        parts: &mut axum::http::request::Parts,
        _state: &S,
    ) -> Result<Self, Self::Rejection> {
        let traceparent = parts
            .headers
            .get(HeaderName::from_static(TRACEPARENT_HEADER))
            .and_then(|h| h.to_str().ok())
            .and_then(SpanContext::decode_w3c_traceparent);
        Ok(Self(traceparent))
    }
}

async fn tokio_instrumentation_middleware(
    req: axum::extract::Request,
    next: axum::middleware::Next,
) -> Result<Response, HttpResponseError> {
    let resp = TaskManager::instrument("axum_handler", next.run(req)).await;
    Ok(resp)
}

async fn log_middleware(
    remote_addr: Option<axum::extract::ConnectInfo<SocketAddr>>,
    ExtractResolvedHostname(resolved_host): ExtractResolvedHostname,
    req: axum::extract::Request,
    next: axum::middleware::Next,
) -> Result<Response, HttpResponseError> {
    let site_id = resolved_host.instance_name;
    let start = Instant::now();

    let remote_addr = remote_addr.map(|connect_info| connect_info.0);
    let method = req.method().clone();
    let uri = req.uri().clone();
    let version = req.version();
    let get_header = |headers: &HeaderMap, name: HeaderName| -> Option<String> {
        headers
            .get(name)
            .and_then(|h| h.to_str().ok().map(|s| s.to_string()))
    };
    let referer = get_header(req.headers(), http::header::REFERER);
    let user_agent = get_header(req.headers(), http::header::USER_AGENT);

    let resp = next.run(req).await;

    let content_length = get_header(resp.headers(), http::header::CONTENT_LENGTH);
    let content_type = get_header(resp.headers(), http::header::CONTENT_TYPE);

    let path = uri.path();
    if path == "/instance_version" || path == "/instance_name" || path == "/get_backend_info" {
        // Skip logging for these high volume, less useful endpoints
        return Ok(resp);
    }

    tracing::info!(
        target: "convex-cloud-http",
        "[{}] {} \"{} {} {:?}\" {} \"{}\" \"{}\" {} {} {:.3}ms",
        site_id,
        LogOptFmt(remote_addr),
        method,
        uri,
        version,
        resp.status().as_u16(),
        LogOptFmt(referer),
        LogOptFmt(user_agent),
        LogOptFmt(content_type),
        LogOptFmt(content_length),
        start.elapsed().as_secs_f64() * 1000.0,
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
pub fn cli_cors() -> CorsLayer {
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

/// Converts a [`HeaderMap`] into an iterator of key-value tuples, handling
/// `None` keys by using the last seen `HeaderName`. This is needed as
/// [`HeaderMap::into_iter`](http::header::HeaderMap#method.into_iter) provides
/// an iterator of `(Option<HeaderName>, T)`.
pub fn normalize_header_map<T>(header_map: HeaderMap<T>) -> impl Iterator<Item = (HeaderName, T)>
where
    T: Clone,
{
    let mut last_key: Option<HeaderName> = None;

    header_map.into_iter().map(move |(key, value)| {
        match key {
            Some(ref key) => last_key = Some(key.clone()),
            None => {},
        }

        let key = last_key
            .clone()
            .expect("HeaderMap should not have a None key without a previous Some key");
        (key, value)
    })
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
    async fn test_http_response_error_internal_server_error() -> anyhow::Result<()> {
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
        let error = HttpError::from_response(response).await?;
        assert_eq!(error.status_code(), StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(error.error_code(), "InternalServerError");
        assert_eq!(error.msg, INTERNAL_SERVER_ERROR_MSG);
        Ok(())
    }

    #[tokio::test]
    async fn test_http_error_400() -> anyhow::Result<()> {
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
        let error = HttpError::from_response(response).await?;
        assert_eq!(error.status_code(), status_code);
        assert_eq!(error.error_code(), error_code);
        assert_eq!(error.message(), msg);
        Ok(())
    }
}

//! Reverse proxy that fronts every spawned `convex-local-backend` container
//! on a single host port. Routes by `Host` header subdomain:
//!
//! - `<deployment_name>.<root>`         → backend port 3210 (cloud API)
//! - `<deployment_name>-site.<root>`    → backend port 3211 (HTTP actions)
//!
//! `<root>` defaults to `localhost` (browsers resolve `*.localhost` to the
//! loopback address per RFC 6761) but is configurable so other hostnames
//! work behind a real DNS or a custom dev hosts file. Deployment containers
//! are reached over the docker network via DNS hostname
//! `<container_prefix><deployment_name>` so they don't need host port
//! mappings.
//!
//! Forwarding handles both plain HTTP and WebSocket upgrades — the latter
//! by hijacking the TCP socket on both ends after the 101 handshake and
//! shuffling bytes bidirectionally.

use std::{
    convert::Infallible,
    fmt,
    io,
    net::SocketAddr,
    sync::Arc,
};

use http_body_util::{
    BodyExt,
    Full,
};
use hyper::{
    body::{
        Bytes,
        Incoming,
    },
    header,
    Request,
    Response,
    StatusCode,
};
use hyper_util::{
    client::legacy::{
        connect::HttpConnector,
        Client,
    },
    rt::{
        TokioExecutor,
        TokioIo,
    },
    server::conn::auto,
};
use tokio::{
    io::{
        AsyncReadExt,
        AsyncWriteExt,
    },
    net::{
        TcpListener,
        TcpStream,
    },
};

use crate::state::OrchestratorState;

const WS_UPSTREAM_RETRY_DELAYS_MS: &[u64] = &[25, 50, 100, 200, 400, 800];

/// Settings shared by every proxied request.
#[derive(Clone)]
pub struct ProxyConfig {
    /// Host suffix expected in the request's Host header. Defaults to
    /// `localhost`. Must not include a leading dot — we add one.
    pub host_root: String,
    /// Container name prefix used when resolving the backend's DNS name.
    /// Same prefix the docker provisioner uses on `docker run --name`.
    pub container_prefix: String,
    /// Optional override port the proxy uses to connect upstream. Defaults
    /// to 3210 (cloud) / 3211 (site) per route.
    pub upstream_cloud_port: u16,
    pub upstream_site_port: u16,
}

impl ProxyConfig {
    pub fn new(host_root: String, container_prefix: String) -> Self {
        Self {
            host_root,
            container_prefix,
            upstream_cloud_port: 3210,
            upstream_site_port: 3211,
        }
    }
}

/// Block on the proxy listener. Returns when the listener errors out.
pub async fn serve_proxy(
    state: OrchestratorState,
    cfg: ProxyConfig,
    addr: SocketAddr,
) -> anyhow::Result<()> {
    let listener = TcpListener::bind(addr).await?;
    tracing::info!(%addr, "convex-orchestrator proxy listening");

    let client: Client<HttpConnector, Full<Bytes>> =
        Client::builder(TokioExecutor::new()).build(HttpConnector::new());
    let shared = Arc::new(SharedCtx {
        state,
        cfg,
        client,
    });

    loop {
        let (stream, peer) = match listener.accept().await {
            Ok(v) => v,
            Err(e) => {
                tracing::warn!(error = %e, "proxy accept failed");
                continue;
            },
        };
        let ctx = shared.clone();
        tokio::spawn(async move {
            let io = TokioIo::new(stream);
            let svc = hyper::service::service_fn(move |req| {
                let ctx = ctx.clone();
                async move { Ok::<_, Infallible>(handle_request(ctx, req).await) }
            });
            if let Err(e) = auto::Builder::new(TokioExecutor::new())
                .serve_connection_with_upgrades(io, svc)
                .await
            {
                tracing::debug!(peer = %peer, error = %e, "proxy connection ended with error");
            }
        });
    }
}

struct SharedCtx {
    state: OrchestratorState,
    cfg: ProxyConfig,
    client: Client<HttpConnector, Full<Bytes>>,
}

#[derive(Debug)]
struct RouteTarget {
    deployment_name: String,
    upstream_host: String,
    upstream_port: u16,
}

/// Pull a `<sub>.<root>` Host header out of `req` and turn it into a route.
/// Returns `None` for malformed/wrong-suffix hosts so the caller can 404.
fn route_for(cfg: &ProxyConfig, req: &Request<Incoming>) -> Option<RouteTarget> {
    let host = req
        .headers()
        .get(header::HOST)?
        .to_str()
        .ok()?
        .to_string();
    // Strip any `:port` suffix.
    let host = host.split(':').next().unwrap_or(&host).to_string();
    let suffix = format!(".{}", cfg.host_root);
    let sub = host.strip_suffix(&suffix)?;
    if sub.is_empty() {
        return None;
    }
    let (deployment_name, port) = if let Some(name) = sub.strip_suffix("-site") {
        (name.to_string(), cfg.upstream_site_port)
    } else {
        (sub.to_string(), cfg.upstream_cloud_port)
    };
    let upstream_host = format!("{}{}", cfg.container_prefix, deployment_name);
    Some(RouteTarget {
        deployment_name,
        upstream_host,
        upstream_port: port,
    })
}

async fn handle_request(
    ctx: Arc<SharedCtx>,
    req: Request<Incoming>,
) -> Response<Full<Bytes>> {
    let target = match route_for(&ctx.cfg, &req) {
        Some(t) => t,
        None => return text(StatusCode::BAD_GATEWAY, "proxy: unrecognized Host header"),
    };

    // Verify the deployment row exists; otherwise return 404 instead of a
    // confusing connection-refused.
    match ctx
        .state
        .storage
        .get_deployment_by_name(&target.deployment_name)
        .await
    {
        Ok(Some(_)) => {},
        Ok(None) => return text(StatusCode::NOT_FOUND, "proxy: unknown deployment"),
        Err(e) => {
            tracing::warn!(error = %e, "proxy: storage lookup failed");
            return text(StatusCode::BAD_GATEWAY, "proxy: storage lookup failed");
        },
    }

    let is_upgrade = req
        .headers()
        .get(header::CONNECTION)
        .and_then(|v| v.to_str().ok())
        .map(|v| v.to_ascii_lowercase().contains("upgrade"))
        .unwrap_or(false);

    if is_upgrade {
        upgrade_proxy(target, req).await
    } else {
        forward_http(ctx, target, req).await
    }
}

async fn forward_http(
    ctx: Arc<SharedCtx>,
    target: RouteTarget,
    req: Request<Incoming>,
) -> Response<Full<Bytes>> {
    let (parts, body) = req.into_parts();
    // Buffer the body so we can re-emit it as a `Full` for the client.
    // Per-deployment payloads tend to be small (RPC-shaped); the dashboard
    // uploads use multipart but those go through a different path on the
    // backend's `/upload` endpoints which we still handle here, just at
    // the cost of a buffered copy.
    let body_bytes = match body.collect().await {
        Ok(b) => b.to_bytes(),
        Err(e) => {
            tracing::warn!(error = %e, "proxy: error reading client body");
            return text(StatusCode::BAD_GATEWAY, "proxy: error reading client body");
        },
    };

    let path_and_query = parts
        .uri
        .path_and_query()
        .map(|pq| pq.as_str().to_string())
        .unwrap_or_else(|| "/".into());
    let upstream_uri = match format!(
        "http://{}:{}{}",
        target.upstream_host, target.upstream_port, path_and_query
    )
    .parse::<hyper::Uri>()
    {
        Ok(u) => u,
        Err(e) => {
            tracing::warn!(error = %e, "proxy: bad upstream URI");
            return text(StatusCode::BAD_GATEWAY, "proxy: bad upstream URI");
        },
    };

    let mut builder = Request::builder().method(parts.method).uri(upstream_uri);
    let h = builder.headers_mut().expect("builder has headers");
    for (k, v) in parts.headers.iter() {
        // Hop-by-hop and Host get rewritten/dropped.
        if matches!(k.as_str(), "host" | "connection" | "upgrade")
            || is_hop_by_hop(k.as_str())
        {
            continue;
        }
        h.insert(k.clone(), v.clone());
    }
    h.insert(
        header::HOST,
        format!("{}:{}", target.upstream_host, target.upstream_port)
            .parse()
            .unwrap(),
    );
    let upstream_req = match builder.body(Full::new(body_bytes)) {
        Ok(r) => r,
        Err(e) => {
            tracing::warn!(error = %e, "proxy: building upstream request failed");
            return text(
                StatusCode::BAD_GATEWAY,
                "proxy: building upstream request failed",
            );
        },
    };

    match ctx.client.request(upstream_req).await {
        Ok(resp) => {
            let (parts, body) = resp.into_parts();
            let body_bytes = match body.collect().await {
                Ok(b) => b.to_bytes(),
                Err(e) => {
                    tracing::warn!(error = %e, "proxy: error reading upstream body");
                    return text(
                        StatusCode::BAD_GATEWAY,
                        "proxy: error reading upstream body",
                    );
                },
            };
            let mut out = Response::builder().status(parts.status);
            let h = out.headers_mut().expect("builder has headers");
            for (k, v) in parts.headers.iter() {
                if is_hop_by_hop(k.as_str()) {
                    continue;
                }
                h.insert(k.clone(), v.clone());
            }
            out.body(Full::new(body_bytes)).unwrap_or_else(|_| {
                Response::builder()
                    .status(StatusCode::BAD_GATEWAY)
                    .body(Full::new(Bytes::from_static(
                        b"proxy: response build failed",
                    )))
                    .unwrap()
            })
        },
        Err(e) => {
            tracing::warn!(
                deployment = %target.deployment_name,
                error = %e,
                "proxy: upstream request failed",
            );
            text(StatusCode::BAD_GATEWAY, "proxy: upstream unreachable")
        },
    }
}

/// Proxy a WebSocket / generic HTTP upgrade. We must:
///
///  1. Open TCP to upstream and send the raw upgrade request.
///  2. Read upstream's response status line + headers (those carry
///     `Sec-WebSocket-Accept`, which the browser validates before treating
///     the handshake as successful).
///  3. Forward upstream's exact response (status, headers) back to the
///     client so hyper triggers the connection upgrade.
///  4. Tunnel raw bytes between the upgraded client socket and the
///     upstream socket bidirectionally.
///
/// A naive "return a fake 101 and stream bytes" version (the previous
/// implementation here) doesn't work because the browser computes the
/// expected `Sec-WebSocket-Accept` from its `Sec-WebSocket-Key` and bails
/// out when the proxy's stub doesn't match.
async fn upgrade_proxy(
    target: RouteTarget,
    mut req: Request<Incoming>,
) -> Response<Full<Bytes>> {
    let raw_request = serialize_http1_request(&req, &target);

    let mut attempt = 0usize;
    let (mut upstream, buf, total, header_end) = loop {
        match open_upstream_upgrade(&target, &raw_request).await {
            Ok(handshake) => break handshake,
            Err(e) if e.is_transient() && attempt < WS_UPSTREAM_RETRY_DELAYS_MS.len() => {
                let delay_ms = WS_UPSTREAM_RETRY_DELAYS_MS[attempt];
                attempt += 1;
                tracing::debug!(
                    deployment = %target.deployment_name,
                    attempt,
                    delay_ms,
                    error = %e,
                    "proxy: transient WS upstream handshake failed; retrying",
                );
                tokio::time::sleep(std::time::Duration::from_millis(delay_ms)).await;
            },
            Err(e) => {
                e.log(&target);
                return e.response();
            },
        };
    };
    let leftover = buf[header_end..total].to_vec();

    // Parse the response with httparse so we can rebuild it as a hyper
    // Response (which the framework forwards verbatim and uses to enter
    // upgrade mode if the status is 1xx).
    let mut header_storage = [httparse::EMPTY_HEADER; 48];
    let mut parsed = httparse::Response::new(&mut header_storage);
    let status_code = match parsed.parse(&buf[..header_end]) {
        Ok(httparse::Status::Complete(_)) => parsed.code.unwrap_or(502),
        _ => {
            tracing::warn!("proxy: failed to parse upstream upgrade response");
            return text(StatusCode::BAD_GATEWAY, "proxy: bad upstream response");
        },
    };

    if status_code != 101 {
        // Upstream rejected the upgrade — propagate its body verbatim so
        // the dashboard sees the actual error from convex-local-backend.
        let body_bytes = leftover.clone();
        let mut out = Response::builder().status(status_code);
        let h = out.headers_mut().expect("builder has headers");
        for hdr in parsed.headers.iter() {
            if hdr.name.is_empty() || is_hop_by_hop(hdr.name) {
                continue;
            }
            if let (Ok(name), Ok(val)) = (
                hyper::header::HeaderName::from_bytes(hdr.name.as_bytes()),
                hyper::header::HeaderValue::from_bytes(hdr.value),
            ) {
                h.insert(name, val);
            }
        }
        return out
            .body(Full::new(Bytes::from(body_bytes)))
            .unwrap_or_else(|_| text(StatusCode::BAD_GATEWAY, "proxy: response build failed"));
    }

    // 101 Switching Protocols — build the response with all the upstream
    // headers (Sec-WebSocket-Accept, Sec-WebSocket-Protocol, etc.) so the
    // browser accepts the handshake. Then tunnel bytes.
    let mut builder = Response::builder().status(StatusCode::SWITCHING_PROTOCOLS);
    let h = builder.headers_mut().expect("builder has headers");
    for hdr in parsed.headers.iter() {
        if hdr.name.is_empty() {
            continue;
        }
        if let (Ok(name), Ok(val)) = (
            hyper::header::HeaderName::from_bytes(hdr.name.as_bytes()),
            hyper::header::HeaderValue::from_bytes(hdr.value),
        ) {
            h.insert(name, val);
        }
    }

    tokio::spawn(async move {
        match hyper::upgrade::on(&mut req).await {
            Ok(upgraded) => {
                let mut upgraded = TokioIo::new(upgraded);
                // Any bytes we already read past the header boundary belong
                // to the data stream (e.g., the first WS frame) — flush
                // them to the client first.
                if !leftover.is_empty()
                    && let Err(e) = upgraded.write_all(&leftover).await
                {
                    tracing::debug!(error = %e, "proxy: writing leftover bytes failed");
                    return;
                }
                let _ = tokio::io::copy_bidirectional_with_sizes(
                    &mut upgraded,
                    &mut upstream,
                    64 * 1024,
                    64 * 1024,
                )
                .await;
            },
            Err(e) => {
                tracing::warn!(error = %e, "proxy: client upgrade failed");
            },
        }
    });

    builder.body(Full::default()).unwrap_or_else(|_| {
        text(StatusCode::BAD_GATEWAY, "proxy: upgrade response build failed")
    })
}

enum UpgradeHandshakeError {
    Connect(io::Error),
    Write(io::Error),
    ClosedEarly,
    Read(io::Error),
    HeadersTooLarge,
}

impl UpgradeHandshakeError {
    fn is_transient(&self) -> bool {
        match self {
            Self::Connect(e) | Self::Write(e) | Self::Read(e) => is_transient_upstream_error(e),
            Self::ClosedEarly => true,
            Self::HeadersTooLarge => false,
        }
    }

    fn log(&self, target: &RouteTarget) {
        match self {
            Self::Connect(e) => tracing::warn!(
                deployment = %target.deployment_name,
                error = %e,
                "proxy: WS upstream connect failed",
            ),
            Self::Write(e) => tracing::warn!(
                deployment = %target.deployment_name,
                error = %e,
                "proxy: WS upstream write failed",
            ),
            Self::ClosedEarly => tracing::warn!(
                deployment = %target.deployment_name,
                "proxy: upstream closed during upgrade handshake",
            ),
            Self::Read(e) => tracing::warn!(
                deployment = %target.deployment_name,
                error = %e,
                "proxy: upstream upgrade read failed",
            ),
            Self::HeadersTooLarge => tracing::warn!(
                deployment = %target.deployment_name,
                "proxy: upstream upgrade response headers too large",
            ),
        }
    }

    fn response(&self) -> Response<Full<Bytes>> {
        match self {
            Self::Connect(_) => transient_upstream("proxy: upstream unreachable"),
            Self::Write(_) => transient_upstream("proxy: upstream write failed"),
            Self::ClosedEarly => transient_upstream("proxy: upstream closed early"),
            Self::Read(_) => transient_upstream("proxy: upstream read failed"),
            Self::HeadersTooLarge => {
                text(StatusCode::BAD_GATEWAY, "proxy: upstream upgrade headers too big")
            },
        }
    }
}

impl fmt::Display for UpgradeHandshakeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Connect(e) => write!(f, "connect failed: {e}"),
            Self::Write(e) => write!(f, "write failed: {e}"),
            Self::ClosedEarly => write!(f, "upstream closed during handshake"),
            Self::Read(e) => write!(f, "read failed: {e}"),
            Self::HeadersTooLarge => write!(f, "headers too large"),
        }
    }
}

async fn open_upstream_upgrade(
    target: &RouteTarget,
    raw_request: &[u8],
) -> Result<(TcpStream, Vec<u8>, usize, usize), UpgradeHandshakeError> {
    let mut upstream = TcpStream::connect(format!(
        "{}:{}",
        target.upstream_host, target.upstream_port
    ))
    .await
    .map_err(UpgradeHandshakeError::Connect)?;

    upstream
        .write_all(raw_request)
        .await
        .map_err(UpgradeHandshakeError::Write)?;

    // Read upstream's response headers until the \r\n\r\n terminator. A real
    // WS handshake is small, so 8KB is plenty.
    let mut buf = vec![0u8; 8192];
    let mut total = 0usize;
    loop {
        if total == buf.len() {
            return Err(UpgradeHandshakeError::HeadersTooLarge);
        }
        let n = match upstream.read(&mut buf[total..]).await {
            Ok(0) => return Err(UpgradeHandshakeError::ClosedEarly),
            Ok(n) => n,
            Err(e) => return Err(UpgradeHandshakeError::Read(e)),
        };
        total += n;
        if let Some(idx) = buf[..total].windows(4).position(|w| w == b"\r\n\r\n") {
            return Ok((upstream, buf, total, idx + 4));
        }
    }
}

fn is_transient_upstream_error(e: &io::Error) -> bool {
    if matches!(
        e.kind(),
        io::ErrorKind::ConnectionAborted
            | io::ErrorKind::ConnectionRefused
            | io::ErrorKind::ConnectionReset
            | io::ErrorKind::Interrupted
            | io::ErrorKind::NotConnected
            | io::ErrorKind::TimedOut
            | io::ErrorKind::WouldBlock
            | io::ErrorKind::BrokenPipe
    ) {
        return true;
    }
    let msg = e.to_string().to_ascii_lowercase();
    msg.contains("temporary failure in name resolution")
        || msg.contains("failed to lookup address information")
        || msg.contains("name or service not known")
}

fn serialize_http1_request(req: &Request<Incoming>, target: &RouteTarget) -> Vec<u8> {
    use std::fmt::Write;
    let path_and_query = req
        .uri()
        .path_and_query()
        .map(|pq| pq.as_str().to_string())
        .unwrap_or_else(|| "/".into());
    let mut s = String::new();
    let _ = write!(s, "{} {} HTTP/1.1\r\n", req.method(), path_and_query);
    let _ = write!(
        s,
        "host: {}:{}\r\n",
        target.upstream_host, target.upstream_port
    );
    for (k, v) in req.headers().iter() {
        if matches!(k.as_str(), "host") {
            continue;
        }
        if let Ok(v) = v.to_str() {
            let _ = write!(s, "{}: {}\r\n", k.as_str(), v);
        }
    }
    s.push_str("\r\n");
    s.into_bytes()
}

fn text(status: StatusCode, body: &'static str) -> Response<Full<Bytes>> {
    Response::builder()
        .status(status)
        .header(header::CONTENT_TYPE, "text/plain; charset=utf-8")
        .body(Full::new(Bytes::from_static(body.as_bytes())))
        .unwrap()
}

fn transient_upstream(body: &'static str) -> Response<Full<Bytes>> {
    Response::builder()
        .status(StatusCode::SERVICE_UNAVAILABLE)
        .header(header::CONTENT_TYPE, "text/plain; charset=utf-8")
        .header(header::RETRY_AFTER, "1")
        .body(Full::new(Bytes::from_static(body.as_bytes())))
        .unwrap()
}

fn is_hop_by_hop(name: &str) -> bool {
    matches!(
        name.to_ascii_lowercase().as_str(),
        "connection"
            | "keep-alive"
            | "proxy-authenticate"
            | "proxy-authorization"
            | "te"
            | "trailers"
            | "transfer-encoding"
            | "upgrade"
    )
}

#[cfg(test)]
mod tests {
    use std::io;

    use super::*;

    #[test]
    fn websocket_upstream_transients_match_restart_errors() {
        assert!(is_transient_upstream_error(&io::Error::new(
            io::ErrorKind::ConnectionReset,
            "Connection reset by peer (os error 104)",
        )));
        assert!(is_transient_upstream_error(&io::Error::new(
            io::ErrorKind::ConnectionRefused,
            "Connection refused (os error 111)",
        )));
        assert!(is_transient_upstream_error(&io::Error::new(
            io::ErrorKind::Other,
            "failed to lookup address information: Temporary failure in name resolution",
        )));
        assert!(!is_transient_upstream_error(&io::Error::new(
            io::ErrorKind::PermissionDenied,
            "permission denied",
        )));
    }
}

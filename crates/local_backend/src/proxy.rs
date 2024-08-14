use std::{
    net::SocketAddr,
    time::Duration,
};

use axum::{
    extract::{
        Request,
        State,
    },
    response::IntoResponse,
    routing::get,
    Router,
};
use common::{
    http::{
        ConvexHttpService,
        HttpResponseError,
        NoopRouteMapper,
    },
    types::ConvexOrigin,
};
use hyper_util::rt::TokioExecutor;

/// Routes HTTP actions to the main webserver
pub async fn dev_site_proxy(
    site_bind_addr: Option<([u8; 4], u16)>,
    origin: ConvexOrigin,
    mut shutdown_rx: async_broadcast::Receiver<()>,
) -> anyhow::Result<()> {
    let Some(addr) = site_bind_addr else {
        return Ok(());
    };
    tracing::info!("Starting dev site proxy at {:?}...", SocketAddr::from(addr));

    async fn proxy_method(
        State(st): State<ConvexOrigin>,
        mut request: Request,
    ) -> Result<impl IntoResponse, HttpResponseError> {
        let new_uri = format!("{}/http{}", st, request.uri());
        *request.uri_mut() = new_uri.parse().map_err(anyhow::Error::new)?;
        let resp = hyper_util::client::legacy::Client::builder(TokioExecutor::new())
            .build_http()
            .request(request)
            .await
            .map_err(anyhow::Error::new)?;
        Ok(resp)
    }

    let proxy_handler = get(proxy_method)
        .post(proxy_method)
        .delete(proxy_method)
        .patch(proxy_method)
        .put(proxy_method)
        .options(proxy_method);
    let router = Router::new()
        .route("/*rest", proxy_handler.clone())
        .route("/", proxy_handler)
        .with_state(origin);

    let service = ConvexHttpService::new(
        Router::new().fallback_service(router),
        "backend_http_proxy",
        "unknown".to_string(),
        4,
        Duration::from_secs(125),
        NoopRouteMapper,
    );
    let proxy_server = service.serve(addr.into(), async move {
        let _ = shutdown_rx.recv().await;
        tracing::info!("Shut down proxy");
    });
    proxy_server.await?;
    Ok(())
}

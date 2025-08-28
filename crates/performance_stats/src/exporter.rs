use std::{
    future,
    net::SocketAddr,
    time::Duration,
};

use axum::{
    routing::get,
    Router,
};
use common::{
    backoff::Backoff,
    http::serve_http,
    runtime::{
        Runtime,
        SpawnHandle,
    },
};
use futures::{
    future::{
        BoxFuture,
        Either,
    },
    pin_mut,
    FutureExt,
};

use crate::memory_allocator::heap_profile;

pub type FlushMetrics<RT: Runtime> = impl FnOnce() -> BoxFuture<'static, ()>;

#[define_opaque(FlushMetrics)]
pub fn register_prometheus_exporter<RT: Runtime>(
    rt: RT,
    bind_addr: SocketAddr,
) -> (Box<dyn SpawnHandle>, FlushMetrics<RT>) {
    let rt_ = rt.clone();
    let handle = rt.clone().spawn("prometheus_exporter", async move {
        let mut backoff = Backoff::new(Duration::from_millis(10), Duration::from_secs(10));

        loop {
            let router = Router::new()
                .route("/metrics", get(common::http::metrics))
                .route("/heap_profile", get(heap_profile));

            let make_svc = router.into_make_service_with_connect_info::<SocketAddr>();
            let e = serve_http(make_svc, bind_addr, future::pending()).await;
            let delay = backoff.fail(&mut rt.rng());
            tracing::error!(
                "Prometheus exporter server failed with error {e:?}, restarting after {}ms delay",
                delay.as_millis()
            );
        }
    });
    let flush = || {
        async move {
            // Prometheus scrapes metrics every 30s.
            let shutdown = tokio::signal::ctrl_c();
            let flush_fut = rt_.wait(Duration::from_secs(60));
            pin_mut!(shutdown);
            pin_mut!(flush_fut);
            tracing::info!("Flushing metrics (60s)... Ctrl-C to skip");
            match futures::future::select(shutdown, flush_fut).await {
                Either::Left(_) => tracing::info!("Got another ctrl-C, shutting down"),
                Either::Right(_) => tracing::info!("Finished flushing metrics"),
            }
        }
        .boxed()
    };
    (handle, flush)
}

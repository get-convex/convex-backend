#![feature(let_chains)]

use std::time::Duration;

use anyhow::anyhow;
use clap::Parser;
use cmd_util::env::config_service;
use common::{
    errors::MainError,
    http::ConvexHttpService,
    runtime::Runtime,
    shutdown::ShutdownSignal,
    version::SERVER_VERSION_STR,
};
use futures::{
    future::{
        self,
        Either,
    },
    FutureExt,
};
use local_backend::{
    config::LocalConfig,
    make_app,
    persistence::connect_persistence,
    proxy::dev_site_proxy,
    router::router,
    HttpActionRouteMapper,
    MAX_CONCURRENT_REQUESTS,
};
use runtime::prod::ProdRuntime;
use tokio::signal::{
    self,
};

fn main() -> Result<(), MainError> {
    tracing::info!("Starting a local backend");
    let _guard = config_service();
    let config = LocalConfig::parse();
    tracing::info!("Starting with config {:?}", config);
    let sentry = sentry::init(sentry::ClientOptions {
        release: Some(format!("local-backend@{}", *SERVER_VERSION_STR).into()),
        ..Default::default()
    });
    if sentry.is_enabled() {
        tracing::info!(
            "Sentry is enabled. Errors will be reported to project with ID {}",
            sentry
                .dsn()
                .map(|dsn| dsn.project_id().to_string())
                .unwrap_or("unknown".to_string())
        );
        if let Some(sentry_identifier) = config.sentry_identifier.clone() {
            sentry::configure_scope(|scope| {
                scope.set_user(Some(sentry::User {
                    id: Some(sentry_identifier),
                    ..Default::default()
                }));
            });
        }
    } else {
        tracing::info!("Sentry is not enabled.")
    }

    sodiumoxide::init().map_err(|()| anyhow!("sodiumoxide initialization failed"))?;

    let tokio = ProdRuntime::init_tokio()?;
    let runtime = ProdRuntime::new(&tokio);

    let runtime_ = runtime.clone();
    let server_future = async {
        run_server(runtime_, config).await?;
        Ok(())
    };

    runtime.block_on("main", server_future)
}

async fn run_server(runtime: ProdRuntime, config: LocalConfig) -> anyhow::Result<()> {
    let serve_future = async move { run_server_inner(runtime, config).await }.fuse();
    futures::pin_mut!(serve_future);

    futures::select! {
        r = serve_future => {
            r?;
            tracing::info!("Done")
        },
    };

    Ok(())
}

async fn run_server_inner(runtime: ProdRuntime, config: LocalConfig) -> anyhow::Result<()> {
    // Used to receive fatal errors from the database or /preempt endpoint.
    let (preempt_tx, mut preempt_rx) = async_broadcast::broadcast(1);
    let preempt_signal = ShutdownSignal::new(preempt_tx.clone(), config.name());
    // Use to signal to the http service to stop.
    let (shutdown_tx, shutdown_rx) = async_broadcast::broadcast(1);
    let persistence = connect_persistence(
        config.db,
        &config.db_spec,
        config.do_not_require_ssl,
        &config.name(),
        runtime.clone(),
        preempt_signal.clone(),
    )
    .await?;
    let st = make_app(
        runtime.clone(),
        config.clone(),
        persistence,
        shutdown_rx.clone(),
        preempt_signal.clone(),
    )
    .await?;
    let router = router(st.clone());
    let mut shutdown_rx_ = shutdown_rx.clone();
    let http_service = ConvexHttpService::new(
        router,
        "backend",
        SERVER_VERSION_STR.to_string(),
        MAX_CONCURRENT_REQUESTS,
        Duration::from_secs(125),
        HttpActionRouteMapper,
    );
    let serve_http_future = http_service.serve(config.http_bind_address().into(), async move {
        let _ = shutdown_rx_.recv().await;
    });
    let proxy_future = dev_site_proxy(
        config.site_bind_address(),
        config.convex_origin_url(),
        shutdown_rx,
    );

    let serve_future = future::try_join(serve_http_future, proxy_future).fuse();
    futures::pin_mut!(serve_future);

    let preempt_future = async move { preempt_rx.recv().await }.fuse();
    futures::pin_mut!(preempt_future);

    // Start shutdown when we get a manual shutdown signal or with the first
    // ctrl-c.
    let mut force_exit_duration = None;
    futures::select! {
        r = serve_future => {
            r?;
            panic!("Serve future stopped unexpectedly!")
        },
        _err = preempt_future => {
            // If we fail with a fatal error, we want to exit immediately.
            tracing::info!("Received a fatal error. Shutting down immediately");
            force_exit_duration = Some(Duration::from_secs(0));
            let _: Result<_, _> = shutdown_tx.broadcast(()).await;
        }
        r = signal::ctrl_c().fuse() => {
            tracing::info!("Received Ctrl-C signal!");
            r?;
            let _: Result<_, _> = shutdown_tx.broadcast(()).await;
        },
    }

    let shutdown = async move {
        // First, drain all in-progress requests;
        tracing::info!("Shutdown initiated, draining existing requests...");
        serve_future.await?;

        // Next, shutdown all of our asynchronous workers.
        tracing::info!("Shutting down application...");
        st.shutdown().await?;

        Ok::<_, anyhow::Error>(())
    }
    .fuse();
    futures::pin_mut!(shutdown);

    let mut force_exit_future = match force_exit_duration {
        Some(force_exit_duration) => Either::Left(runtime.wait(force_exit_duration)),
        None => Either::Right(std::future::pending()),
    }
    .fuse();

    loop {
        futures::select! {
            r = shutdown => {
                r?;
                tracing::info!("Server successfully shut down.");
                // If we are not preempted we exit as soon as the requests are
                // drained. Otherwise, we have to wait for the cool down.
                if force_exit_duration.is_none() {
                    break;
                }
            },
            // Forcibly shutdown when the cool down expires
            _ = force_exit_future => {
                tracing::info!("Cool down expired. Shutting down");
                break;
            }
            // Forcibly shutdown with second ctrl-c.
            r = signal::ctrl_c().fuse() => {
                r?;
                tracing::warn!("Forcibly shutting down!");
                break;
            },
        }
    }

    Ok(())
}

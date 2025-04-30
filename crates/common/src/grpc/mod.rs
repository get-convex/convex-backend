use std::{
    convert::Infallible,
    net::SocketAddr,
    sync::Arc,
    task::{
        Context,
        Poll,
    },
};

use fnv::FnvHashMap;
use futures::Future;
use http::Request;
use pb::error_metadata::ErrorMetadataStatusExt;
use pb_extras::ReflectionService;
use sentry::integrations::tower as sentry_tower;
use tokio::net::TcpSocket;
use tokio_metrics::Instrumented;
use tonic::{
    server::NamedService,
    service::Routes,
    Response,
    Status,
};
use tonic_health::{
    server::{
        health_reporter,
        HealthReporter,
    },
    ServingStatus,
};
use tower::{
    Service,
    ServiceBuilder,
};

use crate::{
    knobs::HTTP_SERVER_TCP_BACKLOG,
    runtime::TaskManager,
};

// maps the full route `/service.Service/Method` to just `Method`
type KnownMethods = FnvHashMap<String, &'static str>;
pub struct ConvexGrpcService {
    routes: Routes,
    known_methods: KnownMethods,
    health_reporter: HealthReporter,
    service_names: Vec<&'static str>,
}

impl ConvexGrpcService {
    pub fn new() -> Self {
        let (health_reporter, health_service) = health_reporter();
        let routes = Routes::new(health_service);
        Self {
            routes,
            known_methods: FnvHashMap::default(),
            health_reporter,
            service_names: Vec::new(),
        }
    }

    pub fn add_service<S>(mut self, service: S) -> Self
    where
        S: tower::Service<
                http::Request<tonic::body::Body>,
                Response = http::Response<tonic::body::Body>,
                Error = Infallible,
            > + ReflectionService
            + Clone
            + Send
            + Sync
            + 'static,
        S::Future: Send + 'static,
    {
        self.routes = self.routes.add_service(service);
        // Gather all service names so we can mark them all as healthy and print one
        // line with all names when we start serving.
        let service_name = <S as NamedService>::NAME;
        self.service_names.push(service_name);
        for method_name in S::METHODS {
            self.known_methods
                .insert(format!("/{service_name}/{method_name}"), method_name);
        }
        self
    }

    pub async fn serve<F>(self, addr: SocketAddr, shutdown: F) -> anyhow::Result<()>
    where
        F: Future<Output = ()>,
    {
        let known_methods = Arc::new(self.known_methods);
        let convex_layers = ServiceBuilder::new()
            .layer_fn(move |s| TokioInstrumentationService::new(known_methods.clone(), s))
            .layer(sentry_tower::NewSentryLayer::new_from_top())
            .layer(sentry_tower::SentryHttpLayer::with_transaction());

        tracing::info!(
            "gRPC services {} listening on ipv4://{addr}",
            self.service_names.join(",")
        );
        for service_name in self.service_names {
            self.health_reporter
                .set_service_status(service_name, ServingStatus::Serving)
                .await;
        }
        // Set SO_REUSEADDR and a bounded TCP accept backlog for our server's listening
        // socket.
        let socket = TcpSocket::new_v4()?;
        socket.set_reuseaddr(true)?;
        socket.set_nodelay(true)?;
        socket.bind(addr)?;

        let listener = socket.listen(*HTTP_SERVER_TCP_BACKLOG)?;
        let incoming = tokio_stream::wrappers::TcpListenerStream::new(listener);
        tonic::transport::Server::builder()
            .layer(convex_layers)
            .add_routes(self.routes)
            .serve_with_incoming_shutdown(incoming, shutdown)
            .await?;
        tracing::info!("GRPC server shutdown complete");
        Ok(())
    }
}

pub fn handle_response<T>(response: Result<Response<T>, Status>) -> anyhow::Result<T> {
    match response {
        Ok(response) => Ok(response.into_inner()),
        Err(status) => Err(status.into_anyhow()),
    }
}

#[derive(Clone)]
struct TokioInstrumentationService<S> {
    known_methods: Arc<KnownMethods>,
    inner: S,
}

impl<S> TokioInstrumentationService<S> {
    fn new(known_methods: Arc<KnownMethods>, inner: S) -> Self {
        Self {
            known_methods,
            inner,
        }
    }
}

impl<S, T> Service<Request<T>> for TokioInstrumentationService<S>
where
    S: Service<Request<T>>,
{
    type Error = S::Error;
    type Future = Instrumented<S::Future>;
    type Response = S::Response;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request<T>) -> Self::Future {
        let name = self
            .known_methods
            .get(req.uri().path())
            .copied()
            .unwrap_or("grpc_handler");
        TaskManager::instrument(name, self.inner.call(req))
    }
}

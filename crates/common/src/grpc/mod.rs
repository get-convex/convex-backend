use std::{
    convert::Infallible,
    net::SocketAddr,
};

use futures::Future;
use sentry::integrations::tower as sentry_tower;
use tonic::{
    server::NamedService,
    service::Routes,
};
use tonic_health::{
    server::{
        health_reporter,
        HealthReporter,
    },
    ServingStatus,
};
use tower::ServiceBuilder;

pub struct ConvexGrpcService {
    routes: Routes,
    health_reporter: HealthReporter,
    service_names: Vec<&'static str>,
}

impl ConvexGrpcService {
    pub fn new() -> Self {
        let (health_reporter, health_service) = health_reporter();
        let routes = Routes::new(health_service);
        Self {
            routes,
            health_reporter,
            service_names: Vec::new(),
        }
    }

    pub fn add_service<S>(mut self, service: S) -> Self
    where
        S: tower::Service<
                http::Request<tonic::body::BoxBody>,
                Response = http::Response<tonic::body::BoxBody>,
                Error = Infallible,
            > + NamedService
            + Clone
            + Send
            + 'static,
        S::Future: Send + 'static,
    {
        self.routes = self.routes.add_service(service);
        // Gather all service names so we can mark them all as healthy and print one
        // line with all names when we start serving.
        let service_name = <S as NamedService>::NAME;
        self.service_names.push(service_name);
        self
    }

    pub async fn serve<F>(mut self, addr: SocketAddr, shutdown: F) -> anyhow::Result<()>
    where
        F: Future<Output = ()>,
    {
        let sentry_layer = ServiceBuilder::new()
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
        tonic::transport::Server::builder()
            .layer(sentry_layer)
            .add_routes(self.routes)
            .serve_with_shutdown(addr, shutdown)
            .await?;
        tracing::info!("GRPC server shutdown complete");
        Ok(())
    }
}

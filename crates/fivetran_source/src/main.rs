#![feature(coroutines)]
#![feature(iterator_try_collect)]

mod api_types;
mod connector;
mod convert;
mod convex_api;
mod log;
mod schema;
mod sync;

#[cfg(test)]
mod tests;

use std::net::{
    IpAddr,
    Ipv4Addr,
    SocketAddr,
};

use clap::Parser;
use connector::ConvexConnector;
use convex_fivetran_common::{
    config::AllowAllHosts,
    fivetran_sdk::source_connector_server::SourceConnectorServer,
};
use tonic::{
    codec::CompressionEncoding,
    transport::Server,
};

/// The command-line arguments received by the connector.
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// The port the connector receives gRPC requests from
    #[arg(long, default_value_t = 50051)]
    port: u16,

    /// Whether the connector is allowed to use any host as deployment URL,
    /// instead of only Convex cloud deployments.
    #[arg(long)]
    allow_all_hosts: bool,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), args.port);

    let connector = ConvexConnector {
        allow_all_hosts: AllowAllHosts(args.allow_all_hosts),
    };

    log::log(&format!("Starting the connector on {addr}"));
    Server::builder()
        .add_service(
            SourceConnectorServer::new(connector)
                .accept_compressed(CompressionEncoding::Gzip)
                .send_compressed(CompressionEncoding::Gzip),
        )
        .serve(addr)
        .await?;

    Ok(())
}

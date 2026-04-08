#![feature(impl_trait_in_assoc_type)]
#![feature(coroutines)]
#![feature(iterator_try_collect)]
#![feature(try_blocks)]
#![feature(try_blocks_heterogeneous)]

use std::net::{
    IpAddr,
    Ipv4Addr,
    SocketAddr,
};

use clap::Parser;
use fivetran_common::fivetran_sdk::destination_connector_server::DestinationConnectorServer;
use fivetran_destination::{
    connector::ConvexFivetranDestination,
    log,
};
use tonic::{
    codec::CompressionEncoding,
    transport::Server,
};

/// The command-line arguments received by the destination.
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// The port the destination receives gRPC requests from
    #[arg(long, default_value_t = 50052)]
    port: u16,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), args.port);

    let destination = ConvexFivetranDestination;

    log(&format!("Starting the destination on {addr}"));

    Server::builder()
        .add_service(
            DestinationConnectorServer::new(destination)
                .accept_compressed(CompressionEncoding::Gzip)
                .send_compressed(CompressionEncoding::Gzip),
        )
        .serve(addr)
        .await?;

    Ok(())
}

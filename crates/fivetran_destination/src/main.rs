#![feature(impl_trait_in_assoc_type)]
#![feature(coroutines)]
#![feature(iterator_try_collect)]
#![feature(try_blocks)]

use std::net::{
    IpAddr,
    Ipv4Addr,
    SocketAddr,
};

use clap::Parser;
use connector::ConvexFivetranDestination;
use convex_fivetran_common::fivetran_sdk::destination_connector_server::DestinationConnectorServer;
use serde::Serialize;
use tonic::{
    codec::CompressionEncoding,
    transport::Server,
};

mod aes;
mod application;
pub mod connector;
mod convert;
mod convex_api;
mod error;
mod file_reader;
mod schema;
#[cfg(test)]
mod testing;

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

#[derive(Serialize)]
#[serde(rename_all = "kebab-case")]
struct LogLine<'a> {
    level: &'a str,
    message: &'a str,
    message_origin: &'a str,
}
pub fn log(message: &str) {
    let result = serde_json::to_string(&LogLine {
        level: "INFO",
        message,
        message_origin: "sdk_destination",
    });
    match result {
        Ok(msg) => println!("{msg}"),
        Err(e) => println!("Unable to serialize to json: {message}: {e}"),
    }
}

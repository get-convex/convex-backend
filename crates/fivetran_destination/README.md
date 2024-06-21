# Fivetran Destination Connector

This crate contains a destination connector allowing developers using Convex to
replicate the data they have in other data sources to their Convex deployments.

The connector consists of a gRPC server hosted on the Fivetran infrastructure.
It communicates with the Convex deployment using the HTTP API it provides.

## Installation

Make sure you have Git and Cargo installed. We recommend installing Cargo via
[rustup](https://rustup.rs/).

```
git clone https://github.com/get-convex/convex-backend.git
cd convex-backend
cargo build --release -p convex_fivetran_destination
```

You can then find the executable file in
`convex-backend/target/release/convex_fivetran_destination`.

## Usage

You can start the connector by starting its binary:

```
$ ./convex_fivetran_destination
{"level":"INFO","message":"Starting the destination on 0.0.0.0:50052","message-origin":"sdk_destination"}
```

You can change the port used using the optional `--port` parameter:

```
$ ./convex_fivetran_destination --port 1337
{"level":"INFO","message":"Starting the destination on 0.0.0.0:1337","message-origin":"sdk_destination"}
```

## Implementation

Unlike the
[Fivetran source connector](https://github.com/get-convex/convex-backend/tree/main/crates/fivetran_source),
the destination connector does not manage the state of the synchronization
mechanism. This is done by Fivetran, which will call the relevant gRPC API
endpoints depending on the state of the data source.

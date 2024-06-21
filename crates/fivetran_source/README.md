# Fivetran Source Connector

This crate contains a source connector allowing developers using Convex to
replicate the data they store in Convex to other databases.

The connector consists of a gRPC server hosted on the Fivetran infrastructure.
This server retrieves the data it needs using the HTTP API described
[in the Convex docs](https://docs.convex.dev/http-api/).

## Installation

Make sure you have Git and Cargo installed. We recommend installing Cargo via
[rustup](https://rustup.rs/).

```
git clone https://github.com/get-convex/convex-backend.git
cd convex-backend
cargo build --release -p convex_fivetran_source
```

You can then find the executable file in
`convex-backend/target/release/convex_fivetran_source`.

## Usage

You can start the connector by starting its binary:

```
$ ./convex_fivetran_source
Starting the connector on [::]:50051
```

You can change the port used using the optional `--port` parameter:

```
$ ./convex_fivetran_source --port 1337
Starting the connector on [::]:1337
```

## Sync Mechanism

The data synchronization happens in two steps:

- During the initial synchronization, the connector uses the
  [`list_snapshot`](https://docs.convex.dev/http-api/#get-apilist_snapshot) API
  to copy all documents.
- During subsequent synchronizations, the connector uses the
  [`document_deltas`](https://docs.convex.dev/http-api/#get-apidocument_deltas)
  API to only apply changes from documents that were modified since the last
  synchronization.

![Flowchart showing the synchronization mechanism.](flow.png)

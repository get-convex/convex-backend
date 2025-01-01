# file_storage Crate

The `file_storage` crate handles file storage-related functionality for the Convex backend. It provides the necessary components and utilities to manage file storage, including uploading, retrieving, and deleting files.

## Purpose and Functionality

The primary purpose of the `file_storage` crate is to manage file storage operations within the Convex backend. It provides functionality for:

- Uploading files to storage
- Retrieving files from storage
- Deleting files from storage
- Generating upload URLs
- Tracking storage usage

## Main Modules and Components

The `file_storage` crate includes the following main modules and components:

- `core`: Contains the core functionality for file storage operations, including uploading, retrieving, and deleting files.
- `metrics`: Provides metrics and logging for file storage operations.
- `tests`: Contains tests for the `file_storage` crate.

## Dependencies and Features

The `file_storage` crate depends on several other crates within the Convex backend. The main dependencies are:

- `anyhow`
- `bytes`
- `common`
- `database`
- `errors`
- `futures`
- `headers`
- `keybroker`
- `maplit`
- `metrics`
- `model`
- `storage`
- `tracing`
- `usage_tracking`
- `value`

The `file_storage` crate also includes several development dependencies for testing purposes:

- `convex_macro`
- `events`
- `runtime`

The `file_storage` crate does not have any specific features defined.

# `model` Crate

The `model` crate provides data models used across various other crates in the Convex backend. It defines the authoritative metadata stored in tables with the prefix `METADATA_PREFIX`. This crate ensures that all system metadata is stored as documents that can be read by UDFs and subscribed to, but cannot be mutated through general-purpose APIs.

## Purpose and Functionality

The primary purpose of the `model` crate is to define and manage the system metadata for the Convex backend. It provides special-purpose APIs for restricted modifications to ensure the integrity of the metadata. The core design principle is to present system data as regular documents while restricting mutations to maintain strong invariants.

## Main Modules and Components

The `model` crate includes the following main modules and components:

- `auth`: Handles authentication-related metadata.
- `backend_state`: Manages the backend state metadata.
- `components`: Defines component-related metadata.
- `config`: Provides configuration-related metadata.
- `cron_jobs`: Manages cron job-related metadata.
- `deployment_audit_log`: Handles deployment audit log metadata.
- `environment_variables`: Manages environment variable metadata.
- `exports`: Handles export-related metadata.
- `external_packages`: Manages external package metadata.
- `file_storage`: Handles file storage-related metadata.
- `modules`: Manages module-related metadata.
- `scheduled_jobs`: Handles scheduled job-related metadata.
- `session_requests`: Manages session request metadata.
- `snapshot_imports`: Handles snapshot import-related metadata.
- `source_packages`: Manages source package metadata.
- `udf_config`: Handles UDF configuration metadata.

## Dependencies and Features

The `model` crate depends on various other crates in the Convex backend. Here are some of the key dependencies:

- `common`: Provides common utilities and components.
- `database`: Handles database-related functionality.
- `errors`: Manages error-related functionality.
- `keybroker`: Handles key management-related functionality.
- `metrics`: Provides metrics-related functionality.
- `runtime`: Manages runtime-related functionality.
- `search`: Handles search-related functionality.
- `storage`: Manages storage-related functionality.
- `sync_types`: Provides synchronization types.
- `value`: Handles value-related functionality.

The `model` crate also includes optional features for testing, which enable additional dependencies and functionalities for testing purposes.

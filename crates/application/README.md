# `application` Crate

The `application` crate is the main application logic for Convex. It includes various modules and components that handle different aspects of the application.

## Purpose and Functionality

The `application` crate provides the core functionality for the Convex backend. It includes the main application logic, handles various tasks such as authentication, database operations, file storage, function execution, and more.

## Main Modules and Components

The `application` crate includes the following main modules and components:

- `api`: Provides the backend API for the application.
- `application_function_runner`: Handles the execution of application functions.
- `cache`: Provides caching functionality for queries.
- `cron_jobs`: Handles the execution of scheduled cron jobs.
- `deploy_config`: Manages deployment configuration.
- `export_worker`: Handles the export of data.
- `function_log`: Manages the logging of function executions.
- `log_visibility`: Manages the visibility of logs.
- `metrics`: Provides metrics and monitoring functionality.
- `module_cache`: Manages the caching of modules.
- `redaction`: Handles the redaction of sensitive data.
- `scheduled_jobs`: Manages the execution of scheduled jobs.
- `schema_worker`: Handles schema-related tasks.
- `snapshot_import`: Manages the import of snapshots.
- `system_table_cleanup`: Handles the cleanup of system tables.
- `table_summary_worker`: Provides functionality for summarizing table data.
- `valid_identifier`: Provides utilities for validating identifiers.

## Dependencies and Features

The `application` crate depends on various other crates to provide its functionality. Some of the main dependencies include:

- `authentication`: Handles authentication-related functionality.
- `common`: Provides common utilities and components used across various other crates.
- `database`: Handles database-related functionality.
- `file_storage`: Manages file storage.
- `function_runner`: Handles function execution.
- `http_client`: Provides HTTP client functionality.
- `isolate`: Manages isolate-related tasks.
- `keybroker`: Handles key management.
- `model`: Provides data models used across various other crates.
- `node_executor`: Manages node execution.
- `scheduled_jobs`: Handles scheduled jobs.
- `schema_worker`: Manages schema-related tasks.
- `search`: Provides search functionality.
- `storage`: Manages storage-related tasks.
- `sync_types`: Provides synchronization types.
- `udf_metrics`: Handles UDF metrics.
- `usage_tracking`: Manages usage tracking.
- `value`: Provides value-related functionality.
- `vector`: Manages vector-related tasks.

The `application` crate also includes various features that can be enabled or disabled based on the requirements. Some of the main features include:

- `testing`: Enables testing-related functionality.

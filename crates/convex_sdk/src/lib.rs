//! # Convex Rust SDK
//!
//! This crate provides the Rust SDK for writing Convex backend functions.
//! Functions are compiled to WebAssembly (WASM) and executed in the Convex backend.
//!
//! ## Function Types
//!
//! There are three types of functions you can write:
//!
//! - **[`query`]**: Read-only, deterministic, cached functions
//! - **[`mutation`]**: Read-write, transactional, deterministic functions
//! - **[`action`]**: Side-effect capable, non-deterministic functions
//!
//! ## Quick Example
//!
//! ```ignore
//! use convex_sdk::*;
//! use serde_json::json;
//!
//! #[query]
//! pub async fn get_user(db: Database, id: String) -> Result<Option<Document>> {
//!     db.get(id.into()).await
//! }
//!
//! #[mutation]
//! pub async fn create_user(
//!     db: Database,
//!     name: String,
//!     email: String,
//! ) -> Result<DocumentId> {
//!     db.insert("users", json!({
//!         "name": name,
//!         "email": email,
//!     })).await
//! }
//! ```
//!
//! ## Security Model
//!
//! Functions run in a WebAssembly sandbox with minimal privileges:
//!
//! | Capability | Query | Mutation | Action |
//! |------------|-------|----------|--------|
//! | Database read | ✅ | ✅ | ✅ |
//! | Database write | ❌ | ✅ | ✅ |
//! | HTTP requests | ❌ | ❌ | ✅ |
//! | File storage | ❌ | ✅ | ✅ |
//! | Filesystem access | ❌ | ❌ | ❌ |
//! | Network access | ❌ | ❌ | ❌* |
//!
//! \* Actions can make HTTP requests through the host function API
//!
//! ## Feature Flags
//!
//! - `macros` (default): Enable procedural macros (`#[query]`, `#[mutation]`, `#[action]`)
//! - `wasm-bindgen`: Enable wasm-bindgen support for browser targets
//!
//! ## Determinism
//!
//! Queries and mutations **must be deterministic** for the Convex system to work correctly:
//!
//! - ✅ Deterministic: Database reads, pure computation, seeded random
//! - ❌ Non-deterministic: HTTP requests, random numbers (unseeded), current time, filesystem
//!
//! The SDK enforces this by restricting available operations based on function type.

#![cfg_attr(target_arch = "wasm32", no_std)]
#![warn(missing_docs)]

extern crate alloc;

pub mod auth;
pub mod components;
pub mod cron;
pub mod db;
pub mod http;
pub mod scheduler;
pub mod search;
pub mod storage;
pub mod types;
pub mod schema;
pub mod vector;

/// Testing utilities (available with `testing` feature)
#[cfg(any(feature = "testing", test))]
pub mod testing;

// Re-export core types
pub use types::{ConvexError, ConvexValue, Document, DocumentId, Result};

// Re-export auth types
pub use auth::{get_identity, is_authenticated, require_auth, Identity};

// Re-export database types
pub use db::{BatchQuery, BatchQueryBuilder, BatchQueryResult, Database, PaginatedResult, QueryBuilder, TableReader, TableWriter, TableQueryBuilder, IndexQueryBuilder, FilterBuilder, FilterExpression, FieldRef, FilterExpressionExt, IndexRange, IndexRangeBuilder, RangeBound, ScanDirection, PageStatus};

// Re-export HTTP types
pub use http::{fetch, FetchOptions, HttpResponse};

// Re-export storage types
pub use storage::{get as storage_get, get_metadata as storage_get_metadata, generate_url as storage_generate_url, store as storage_store, delete as storage_delete, generate_upload_url as storage_generate_upload_url, StorageFile, StorageId, StorageMetadata, StorageUrl, UrlOptions, UploadUrl, UploadUrlOptions};

// Re-export schema types
pub use schema::{SchemaDefinition, TableDefinition, IndexDefinition, SearchIndexDefinition, VectorIndexDefinition, SearchIndexConfig, VectorIndexConfig, validators::v};

// Re-export scheduler types
pub use scheduler::{schedule_job, cancel_job, get_job_info, list_jobs, JobId, JobInfo, JobStatus, ScheduleOptions};

// Re-export search types
pub use search::{SearchQueryBuilder, TableSearchQueryBuilder, SearchResult};

// Re-export vector types
pub use vector::{vector_search, VectorSearchQueryBuilder, VectorSearchResult};

// Re-export cron types
pub use cron::{CronJobs, CronJob, CronContext, Schedule, DayOfWeek};

// Re-export component types
pub use components::{ComponentDefinition, ComponentExports, ComponentInstance, ComponentBuilder, FunctionReference, UdfType};

// Re-export macros when the feature is enabled
#[cfg(feature = "macros")]
pub use convex_sdk_macros::{action, mutation, query, internal_action, internal_mutation, internal_query, http_action};

// Re-export serde_json for convenience
pub use serde_json;

// Re-export serde for derive macros
pub use serde;

/// Version of the Convex Rust SDK.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Compile-time check that the SDK is properly configured.
///
/// This function always succeeds at runtime, but ensures
/// the crate is compiled with the correct target.
pub const fn check_target() {
    // This will fail to compile if we're not targeting WASM
    // when the wasm32 feature is enabled
    #[cfg(all(feature = "wasm32", not(target_arch = "wasm32")))]
    compile_error!("The wasm32 feature requires targeting wasm32 architecture");
}

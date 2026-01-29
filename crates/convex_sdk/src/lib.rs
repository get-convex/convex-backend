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

pub mod db;
pub mod http;
pub mod storage;
pub mod types;

// Re-export core types
pub use types::{ConvexError, ConvexValue, Document, DocumentId, Result};

// Re-export database types
pub use db::{Database, QueryBuilder};

// Re-export HTTP types
pub use http::{fetch, FetchOptions, HttpResponse};

// Re-export storage types
pub use storage::{get as storage_get, store as storage_store, StorageFile, StorageId};

// Re-export macros when the feature is enabled
#[cfg(feature = "macros")]
pub use convex_sdk_macros::{action, mutation, query};

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

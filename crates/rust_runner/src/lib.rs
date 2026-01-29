//! Rust Function Runner
//!
//! This crate provides a WASM runtime for executing Convex functions
//! written in Rust and compiled to WebAssembly.

pub mod runner;
pub mod wasi;
pub mod host_functions;
pub mod module;
pub mod limits;
pub mod determinism;
pub mod analyze;
pub mod source_maps;

pub use runner::RustFunctionRunner;
pub use module::{RustModule, RustFunctionMetadata};
pub use limits::{ExecutionLimits, ResourceLimiter};
pub use determinism::{DeterminismContext, TimeProvider, RandomProvider};
pub use host_functions::{DatabaseClient, StorageClient};
pub use analyze::analyze_rust_module;
pub use source_maps::{
    SourceMap, SourceLocation, SourceMapManager, MappedError, StackFrame,
    SourceMapGenerator
};

use std::sync::Arc;
use anyhow::Result;

/// Initialize the WASM runtime
pub async fn init_runtime() -> Result<Arc<wasmtime::Engine>> {
    let mut config = wasmtime::Config::new();
    config.async_support(true);
    config.wasm_backtrace_details(wasmtime::WasmBacktraceDetails::Enable);
    config.wasm_reference_types(true);
    config.wasm_bulk_memory(true);

    // Enable fuel consumption for CPU limiting
    config.consume_fuel(true);

    let engine = wasmtime::Engine::new(&config)?;
    Ok(Arc::new(engine))
}

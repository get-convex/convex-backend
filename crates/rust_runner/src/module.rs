//! Rust module representation

use anyhow::Result;
use serde::{Deserialize, Serialize};

/// Metadata for a Rust function
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RustFunctionMetadata {
    pub name: String,
    pub function_type: RustFunctionType,
    pub export_name: String,
}

/// Type of Rust function
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RustFunctionType {
    Query,
    Mutation,
    Action,
}

/// A compiled Rust module
pub struct RustModule {
    pub id: String,
    wasm_binary: Vec<u8>,
    pub functions: Vec<RustFunctionMetadata>,
}

impl RustModule {
    /// Create a new Rust module from WASM bytes
    pub fn new(id: String, wasm_binary: Vec<u8>, functions: Vec<RustFunctionMetadata>) -> Self {
        Self {
            id,
            wasm_binary,
            functions,
        }
    }

    /// Get the WASM binary
    pub fn wasm_binary(&self) -> &[u8] {
        &self.wasm_binary
    }

    /// Find a function by name
    pub fn find_function(&self, name: &str) -> Option<&RustFunctionMetadata> {
        self.functions.iter().find(|f| f.name == name)
    }
}

/// Parse a Rust module from source package data
pub fn parse_rust_module(id: String, wasm_binary: Vec<u8>, metadata_json: &str) -> Result<RustModule> {
    let functions: Vec<RustFunctionMetadata> = serde_json::from_str(metadata_json)?;

    Ok(RustModule {
        id,
        wasm_binary,
        functions,
    })
}

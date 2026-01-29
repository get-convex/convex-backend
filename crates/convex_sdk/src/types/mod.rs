//! Convex value types and document identifiers

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A Convex value - the type system for Convex documents
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ConvexValue {
    Null,
    Int64(i64),
    Float64(f64),
    Boolean(bool),
    String(String),
    Bytes(Vec<u8>),
    Array(Vec<ConvexValue>),
    Object(HashMap<String, ConvexValue>),
}

impl Default for ConvexValue {
    fn default() -> Self {
        ConvexValue::Null
    }
}

/// A document ID in Convex
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DocumentId(String);

impl DocumentId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<String> for DocumentId {
    fn from(s: String) -> Self {
        Self::new(s)
    }
}

impl From<&str> for DocumentId {
    fn from(s: &str) -> Self {
        Self::new(s)
    }
}

/// A Convex document with ID and value
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Document {
    pub id: DocumentId,
    pub value: ConvexValue,
}

/// Result type for Convex operations
pub type Result<T> = std::result::Result<T, ConvexError>;

/// Errors that can occur in Convex operations
#[derive(Debug, thiserror::Error)]
pub enum ConvexError {
    #[error("database error: {0}")]
    Database(String),
    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("not found")]
    NotFound,
    #[error("permission denied")]
    PermissionDenied,
    #[error("invalid argument: {0}")]
    InvalidArgument(String),
    #[error("unknown error: {0}")]
    Unknown(String),
}

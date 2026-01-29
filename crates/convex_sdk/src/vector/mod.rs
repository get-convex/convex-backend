//! Vector similarity search for Convex
//!
//! This module provides vector similarity search capabilities using vector indexes.
//!
//! # Example
//!
//! ```ignore
//! use convex_sdk::*;
//!
//! #[query]
//! async fn find_similar_embeddings(
//!     db: Database,
//!     embedding: Vec<f64>,
//! ) -> Result<Vec<Document>> {
//!     vector_search("embeddings", "by_embedding", embedding)
//!         .limit(10)
//!         .collect()
//!         .await
//! }
//! ```

use crate::types::{ConvexError, Document, Result};
use crate::db::wasm_helpers;
use alloc::string::String;
use alloc::vec::Vec;
use serde::{Deserialize, Serialize};

// Host function for vector search
extern "C" {
    fn __convex_vector_search(
        table_ptr: i32,
        table_len: i32,
        index_ptr: i32,
        index_len: i32,
        vector_ptr: i32,
        vector_len: i32,
        options_ptr: i32,
        options_len: i32,
    ) -> i32;
}

/// A filter for vector search queries
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorFilter {
    field: String,
    op: String,
    value: serde_json::Value,
}

/// Vector search query specification
#[derive(Debug, Clone, Serialize, Deserialize)]
struct VectorSearchSpec {
    table: String,
    index: String,
    vector: Vec<f64>,
    filters: Vec<VectorFilter>,
    limit: Option<usize>,
}

/// Vector search result with similarity score
#[derive(Debug, Clone, Deserialize)]
pub struct VectorSearchResult {
    /// The matched document
    #[serde(flatten)]
    pub document: Document,
    /// Similarity score (higher is more similar)
    #[serde(rename = "_score")]
    pub score: f64,
}

/// Builder for vector search queries
#[derive(Debug)]
pub struct VectorSearchQueryBuilder {
    table_name: String,
    index_name: String,
    vector: Vec<f64>,
    filters: Vec<VectorFilter>,
    limit: Option<usize>,
}

impl VectorSearchQueryBuilder {
    /// Create a new vector search query builder
    pub fn new(
        table_name: impl Into<String>,
        index_name: impl Into<String>,
        vector: Vec<f64>,
    ) -> Self {
        Self {
            table_name: table_name.into(),
            index_name: index_name.into(),
            vector,
            filters: Vec::new(),
            limit: None,
        }
    }

    /// Add an equality filter to the vector search
    ///
    /// # Example
    ///
    /// ```ignore
    /// let results = vector_search("embeddings", "by_embedding", query_vector)
    ///     .filter("category", "articles")?
    ///     .limit(10)
    ///     .collect()
    ///     .await?;
    /// ```
    pub fn filter(mut self, field: &str, value: impl Serialize) -> Result<Self> {
        let value_json = serde_json::to_value(value)?;
        self.filters.push(VectorFilter {
            field: field.to_string(),
            op: "eq".to_string(),
            value: value_json,
        });
        Ok(self)
    }

    /// Add a disjunctive (OR) filter
    ///
    /// This allows matching documents where ANY of the filters match.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let results = vector_search("embeddings", "by_embedding", query_vector)
    ///     .filter("category", "articles")?
    ///     .or("category", "tutorials")?
    ///     .collect()
    ///     .await?;
    /// ```
    pub fn or(mut self, field: &str, value: impl Serialize) -> Result<Self> {
        let value_json = serde_json::to_value(value)?;
        self.filters.push(VectorFilter {
            field: field.to_string(),
            op: "or".to_string(),
            value: value_json,
        });
        Ok(self)
    }

    /// Limit the number of results
    pub fn limit(mut self, n: usize) -> Self {
        self.limit = Some(n);
        self
    }

    /// Execute the vector search and return all matching documents
    pub async fn collect(self) -> Result<Vec<Document>> {
        let spec = VectorSearchSpec {
            table: self.table_name,
            index: self.index_name,
            vector: self.vector,
            filters: self.filters,
            limit: self.limit,
        };

        let spec_json = serde_json::to_vec(&spec)?;
        let spec_ptr = wasm_helpers::alloc_and_write(&spec_json)?;

        let result_ptr = unsafe {
            __convex_vector_search(
                spec_ptr,
                spec_json.len() as i32,
                0,
                0,
                0,
                0,
                0,
                0,
            )
        };

        wasm_helpers::free_ptr(spec_ptr);

        let host_result = unsafe { wasm_helpers::parse_host_result(result_ptr)? };
        let data = wasm_helpers::handle_host_result(host_result)?;

        match data {
            Some(value) => {
                let docs: Vec<Document> =
                    serde_json::from_value(value).map_err(ConvexError::Serialization)?;
                Ok(docs)
            }
            None => Ok(Vec::new()),
        }
    }

    /// Execute the search and return the first N results
    pub async fn take(self, n: usize) -> Result<Vec<Document>> {
        self.limit(n).collect().await
    }

    /// Execute the search and return the first result
    pub async fn first(mut self) -> Result<Option<Document>> {
        self.limit = Some(1);
        let results = self.collect().await?;
        Ok(results.into_iter().next())
    }

    /// Execute the search and return results with similarity scores
    pub async fn with_scores(self) -> Result<Vec<VectorSearchResult>> {
        let spec = VectorSearchSpec {
            table: self.table_name,
            index: self.index_name,
            vector: self.vector,
            filters: self.filters,
            limit: self.limit,
        };

        let spec_json = serde_json::to_vec(&spec)?;
        let spec_ptr = wasm_helpers::alloc_and_write(&spec_json)?;

        let result_ptr = unsafe {
            __convex_vector_search(
                spec_ptr,
                spec_json.len() as i32,
                0,
                0,
                0,
                0,
                0,
                0,
            )
        };

        wasm_helpers::free_ptr(spec_ptr);

        let host_result = unsafe { wasm_helpers::parse_host_result(result_ptr)? };
        let data = wasm_helpers::handle_host_result(host_result)?;

        match data {
            Some(value) => {
                let results: Vec<VectorSearchResult> =
                    serde_json::from_value(value).map_err(ConvexError::Serialization)?;
                Ok(results)
            }
            None => Ok(Vec::new()),
        }
    }
}

/// Perform a vector similarity search
///
/// # Arguments
///
/// * `table_name` - The name of the table to search
/// * `index_name` - The name of the vector index to use
/// * `vector` - The query vector to search for
///
/// # Returns
///
/// A `VectorSearchQueryBuilder` for configuring and executing the search
///
/// # Example
///
/// ```ignore
/// use convex_sdk::*;
///
/// #[query]
/// async fn find_similar(
///     db: Database,
///     embedding: Vec<f64>,
/// ) -> Result<Vec<Document>> {
///     vector_search("documents", "by_embedding", embedding)
///         .limit(10)
///         .collect()
///         .await
/// }
/// ```
pub fn vector_search(
    table_name: impl Into<String>,
    index_name: impl Into<String>,
    vector: Vec<f64>,
) -> VectorSearchQueryBuilder {
    VectorSearchQueryBuilder::new(table_name, index_name, vector)
}

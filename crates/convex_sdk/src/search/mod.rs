//! Full-text search for Convex
//!
//! This module provides full-text search capabilities using search indexes.
//!
//! # Example
//!
//! ```ignore
//! use convex_sdk::*;
//!
//! #[query]
//! async fn search_posts(db: Database, query: String) -> Result<Vec<Document>> {
//!     db.query("posts")
//!         .with_search_index("search_content", &query)
//!         .collect()
//!         .await
//! }
//! ```

use crate::types::{ConvexError, Document, Result};
use crate::db::wasm_helpers;
use alloc::string::String;
use alloc::vec::Vec;
use serde::{Deserialize, Serialize};

// Host function for search queries
extern "C" {
    fn __convex_search_query(
        table_ptr: i32,
        table_len: i32,
        index_ptr: i32,
        index_len: i32,
        query_ptr: i32,
        query_len: i32,
    ) -> i32;
}

/// A filter condition for search queries
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchFilter {
    field: String,
    op: String,
    value: serde_json::Value,
}

/// Search query specification sent to the host
#[derive(Debug, Clone, Serialize, Deserialize)]
struct SearchQuerySpec {
    table: String,
    index: String,
    search_text: String,
    filters: Vec<SearchFilter>,
    limit: Option<usize>,
}

/// Search result with relevance score
#[derive(Debug, Clone, Deserialize)]
pub struct SearchResult {
    /// The matched document
    #[serde(flatten)]
    pub document: Document,
    /// Relevance score (higher is better)
    #[serde(rename = "_score")]
    pub score: f64,
}

/// Builder for search queries
#[derive(Debug)]
pub struct SearchQueryBuilder {
    table_name: String,
    index_name: String,
    search_text: String,
    filters: Vec<SearchFilter>,
    limit: Option<usize>,
}

impl SearchQueryBuilder {
    /// Create a new search query builder
    pub(crate) fn new(
        table_name: impl Into<String>,
        index_name: impl Into<String>,
        search_text: impl Into<String>,
    ) -> Self {
        Self {
            table_name: table_name.into(),
            index_name: index_name.into(),
            search_text: search_text.into(),
            filters: Vec::new(),
            limit: None,
        }
    }

    /// Add a filter to the search query
    ///
    /// Only documents matching this filter will be searched.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let results = db.query("posts")
    ///     .with_search_index("search_content", "rust")
    ///     .filter("published", true)?
    ///     .collect()
    ///     .await?;
    /// ```
    pub fn filter(mut self, field: &str, value: impl Serialize) -> Result<Self> {
        let value_json = serde_json::to_value(value)?;
        self.filters.push(SearchFilter {
            field: field.to_string(),
            op: "eq".to_string(),
            value: value_json,
        });
        Ok(self)
    }

    /// Limit the number of search results
    pub fn limit(mut self, n: usize) -> Self {
        self.limit = Some(n);
        self
    }

    /// Execute the search and return all matching documents
    pub async fn collect(self) -> Result<Vec<Document>> {
        let spec = SearchQuerySpec {
            table: self.table_name.clone(),
            index: self.index_name,
            search_text: self.search_text,
            filters: self.filters,
            limit: self.limit,
        };

        let spec_json = serde_json::to_vec(&spec)?;
        let spec_ptr = wasm_helpers::alloc_and_write(&spec_json)?;

        let result_ptr = unsafe {
            __convex_search_query(
                spec_ptr,
                spec_json.len() as i32,
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

    /// Execute the search and return exactly one result
    ///
    /// Returns an error if zero or more than one result is found.
    pub async fn unique(self) -> Result<Document> {
        let results = self.collect().await?;
        match results.len() {
            0 => Err(ConvexError::InvalidArgument(
                "Expected exactly one result, found none".into()
            )),
            1 => Ok(results.into_iter().next().unwrap()),
            n => Err(ConvexError::InvalidArgument(
                format!("Expected exactly one result, found {}", n)
            )),
        }
    }

    /// Execute the search and return paginated results with scores
    pub async fn search_with_scores(self) -> Result<Vec<SearchResult>> {
        let spec = SearchQuerySpec {
            table: self.table_name,
            index: self.index_name,
            search_text: self.search_text,
            filters: self.filters,
            limit: self.limit,
        };

        let spec_json = serde_json::to_vec(&spec)?;
        let spec_ptr = wasm_helpers::alloc_and_write(&spec_json)?;

        let result_ptr = unsafe {
            __convex_search_query(
                spec_ptr,
                spec_json.len() as i32,
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
                let results: Vec<SearchResult> =
                    serde_json::from_value(value).map_err(ConvexError::Serialization)?;
                Ok(results)
            }
            None => Ok(Vec::new()),
        }
    }
}

/// Table-scoped search query builder
#[derive(Debug)]
pub struct TableSearchQueryBuilder {
    inner: SearchQueryBuilder,
}

impl TableSearchQueryBuilder {
    /// Create a new table-scoped search query builder
    pub(crate) fn new(
        table_name: impl Into<String>,
        index_name: impl Into<String>,
        search_text: impl Into<String>,
    ) -> Self {
        Self {
            inner: SearchQueryBuilder::new(table_name, index_name, search_text),
        }
    }

    /// Add a filter to the search query
    pub fn filter(self, field: &str, value: impl Serialize) -> Result<Self> {
        Ok(Self {
            inner: self.inner.filter(field, value)?,
        })
    }

    /// Limit the number of search results
    pub fn limit(self, n: usize) -> Self {
        Self {
            inner: self.inner.limit(n),
        }
    }

    /// Execute the search and return all matching documents
    pub async fn collect(self) -> Result<Vec<Document>> {
        self.inner.collect().await
    }

    /// Execute the search and return the first N results
    pub async fn take(self, n: usize) -> Result<Vec<Document>> {
        self.inner.take(n).await
    }

    /// Execute the search and return the first result
    pub async fn first(self) -> Result<Option<Document>> {
        self.inner.first().await
    }

    /// Execute the search and return exactly one result
    pub async fn unique(self) -> Result<Document> {
        self.inner.unique().await
    }

    /// Execute the search and return results with relevance scores
    pub async fn with_scores(self) -> Result<Vec<SearchResult>> {
        self.inner.search_with_scores().await
    }
}

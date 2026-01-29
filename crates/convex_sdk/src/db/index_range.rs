//! Index range queries for efficient database access
//!
//! This module provides `IndexRangeBuilder` for building efficient index-based
//! range queries, matching the TypeScript SDK's index range functionality.
//!
//! Index ranges allow efficient queries that scan a contiguous portion of an index,
//! supporting both ascending and descending order with customizable start/end bounds.
//!
//! # Example
//!
//! ```ignore
//! use convex_sdk::{Database, IndexRangeBuilder};
//!
//! // Query users by age range (18-65)
//! let users = db.table("users")
//!     .query_index_range("by_age")
//!     .gt(18)
//!     .lte(65)
//!     .collect()
//!     .await?;
//! ```

use alloc::string::String;
use alloc::vec::Vec;
use serde::{Deserialize, Serialize};

use crate::types::{ConvexError, Document, Result};
use crate::db::PaginatedResult;

/// A bound for index range queries
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RangeBound {
    /// Inclusive bound (includes the value)
    Inclusive(serde_json::Value),
    /// Exclusive bound (excludes the value)
    Exclusive(serde_json::Value),
    /// Unbounded (no limit in this direction)
    Unbounded,
}

/// Direction for index scanning
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ScanDirection {
    /// Scan in ascending order
    Asc,
    /// Scan in descending order
    Desc,
}

impl Default for ScanDirection {
    fn default() -> Self {
        ScanDirection::Asc
    }
}

/// Specification for an index range query
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexRange {
    /// The index name to query
    pub index_name: String,
    /// Start bound (inclusive/exclusive/unbounded)
    pub start_bound: RangeBound,
    /// End bound (inclusive/exclusive/unbounded)
    pub end_bound: RangeBound,
    /// Scan direction (ascending/descending)
    pub direction: ScanDirection,
    /// Maximum number of results to return
    pub limit: Option<usize>,
    /// Cursor for pagination
    pub cursor: Option<String>,
}

impl IndexRange {
    /// Create a new index range query
    pub fn new(index_name: impl Into<String>) -> Self {
        Self {
            index_name: index_name.into(),
            start_bound: RangeBound::Unbounded,
            end_bound: RangeBound::Unbounded,
            direction: ScanDirection::Asc,
            limit: None,
            cursor: None,
        }
    }

    /// Set the start bound to be inclusive
    pub fn gte(mut self, value: impl Serialize) -> Result<Self> {
        self.start_bound = RangeBound::Inclusive(serde_json::to_value(value)?);
        Ok(self)
    }

    /// Set the start bound to be exclusive
    pub fn gt(mut self, value: impl Serialize) -> Result<Self> {
        self.start_bound = RangeBound::Exclusive(serde_json::to_value(value)?);
        Ok(self)
    }

    /// Set the end bound to be inclusive
    pub fn lte(mut self, value: impl Serialize) -> Result<Self> {
        self.end_bound = RangeBound::Inclusive(serde_json::to_value(value)?);
        Ok(self)
    }

    /// Set the end bound to be exclusive
    pub fn lt(mut self, value: impl Serialize) -> Result<Self> {
        self.end_bound = RangeBound::Exclusive(serde_json::to_value(value)?);
        Ok(self)
    }

    /// Set the scan direction
    pub fn order(mut self, direction: ScanDirection) -> Self {
        self.direction = direction;
        self
    }

    /// Set the maximum number of results
    pub fn limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }

    /// Set a cursor for pagination
    pub fn cursor(mut self, cursor: impl Into<String>) -> Self {
        self.cursor = Some(cursor.into());
        self
    }
}

/// Builder for index range queries
///
/// This builder provides a fluent API for constructing index range queries
/// with support for bounds, ordering, and pagination.
#[derive(Debug)]
pub struct IndexRangeBuilder {
    table_name: String,
    index_name: String,
    start_bound: RangeBound,
    end_bound: RangeBound,
    direction: ScanDirection,
    limit: Option<usize>,
    cursor: Option<String>,
    filter_expression: Option<crate::db::FilterExpression>,
}

impl IndexRangeBuilder {
    /// Create a new index range builder
    pub(crate) fn new(table_name: String, index_name: String) -> Self {
        Self {
            table_name,
            index_name,
            start_bound: RangeBound::Unbounded,
            end_bound: RangeBound::Unbounded,
            direction: ScanDirection::Asc,
            limit: None,
            cursor: None,
            filter_expression: None,
        }
    }

    /// Set the start bound to be inclusive (>= value)
    ///
    /// # Example
    ///
    /// ```ignore
    /// let users = db.table("users")
    ///     .query_index_range("by_age")
    ///     .gte(18)
    ///     .collect()
    ///     .await?;
    /// ```
    pub fn gte(mut self, value: impl Serialize) -> Result<Self> {
        self.start_bound = RangeBound::Inclusive(serde_json::to_value(value)?);
        Ok(self)
    }

    /// Set the start bound to be exclusive (> value)
    ///
    /// # Example
    ///
    /// ```ignore
    /// let users = db.table("users")
    ///     .query_index_range("by_age")
    ///     .gt(18)
    ///     .collect()
    ///     .await?;
    /// ```
    pub fn gt(mut self, value: impl Serialize) -> Result<Self> {
        self.start_bound = RangeBound::Exclusive(serde_json::to_value(value)?);
        Ok(self)
    }

    /// Set the end bound to be inclusive (<= value)
    ///
    /// # Example
    ///
    /// ```ignore
    /// let users = db.table("users")
    ///     .query_index_range("by_age")
    ///     .lte(65)
    ///     .collect()
    ///     .await?;
    /// ```
    pub fn lte(mut self, value: impl Serialize) -> Result<Self> {
        self.end_bound = RangeBound::Inclusive(serde_json::to_value(value)?);
        Ok(self)
    }

    /// Set the end bound to be exclusive (< value)
    ///
    /// # Example
    ///
    /// ```ignore
    /// let users = db.table("users")
    ///     .query_index_range("by_age")
    ///     .lt(65)
    ///     .collect()
    ///     .await?;
    /// ```
    pub fn lt(mut self, value: impl Serialize) -> Result<Self> {
        self.end_bound = RangeBound::Exclusive(serde_json::to_value(value)?);
        Ok(self)
    }

    /// Set the scan direction to ascending
    pub fn ascending(mut self) -> Self {
        self.direction = ScanDirection::Asc;
        self
    }

    /// Set the scan direction to descending
    pub fn descending(mut self) -> Self {
        self.direction = ScanDirection::Desc;
        self
    }

    /// Set the maximum number of results
    pub fn limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }

    /// Set a cursor for pagination
    pub fn cursor(mut self, cursor: impl Into<String>) -> Self {
        self.cursor = Some(cursor.into());
        self
    }

    /// Add a filter expression for additional filtering
    ///
    /// # Example
    ///
    /// ```ignore
    /// use convex_sdk::FilterBuilder;
    ///
    /// let f = FilterBuilder::new();
    /// let users = db.table("users")
    ///     .query_index_range("by_age")
    ///     .gte(18)?
    ///     .lte(65)?
    ///     .filter_expr(f.eq("status", "active"))
    ///     .collect()
    ///     .await?;
    /// ```
    pub fn filter_expr(mut self, expression: crate::db::FilterExpression) -> Self {
        self.filter_expression = Some(expression);
        self
    }

    /// Execute the query and return all results
    pub async fn collect(self) -> Result<Vec<Document>> {
        use crate::db::wasm_helpers;
        use crate::db::__convex_db_query_advanced;
        use crate::db::IndexRangeQuerySpec;

        // Build the index range query specification
        let spec = IndexRangeQuerySpec {
            table: self.table_name,
            index: self.index_name,
            start_bound: self.start_bound,
            end_bound: self.end_bound,
            direction: self.direction,
            limit: self.limit,
            cursor: self.cursor,
            filter_expression: self.filter_expression,
        };

        // Serialize the query specification
        let spec_json = serde_json::to_vec(&spec)?;

        // Allocate memory and write the query spec
        let spec_ptr = wasm_helpers::alloc_and_write(&spec_json)?;

        // Call the host function
        let result_ptr = unsafe {
            __convex_db_query_advanced(spec_ptr, spec_json.len() as i32)
        };

        // Free the input memory
        wasm_helpers::free_ptr(spec_ptr);

        // Parse the result
        let host_result = unsafe { wasm_helpers::parse_host_result(result_ptr)? };
        let data = wasm_helpers::handle_host_result(host_result)?;

        // Deserialize the documents
        match data {
            Some(value) => {
                let docs: Vec<Document> =
                    serde_json::from_value(value).map_err(ConvexError::Serialization)?;
                Ok(docs)
            }
            None => Ok(Vec::new()),
        }
    }

    /// Execute the query and return the first N results
    pub async fn take(self, n: usize) -> Result<Vec<Document>> {
        self.limit(n).collect().await
    }

    /// Execute the query and return the first result
    pub async fn first(mut self) -> Result<Option<Document>> {
        self.limit = Some(1);
        let results = self.collect().await?;
        Ok(results.into_iter().next())
    }

    /// Execute the query and return exactly one result
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

    /// Execute the query and return paginated results
    pub async fn paginate(self) -> Result<PaginatedResult> {
        use crate::db::wasm_helpers;
        use crate::db::__convex_db_query_advanced;
        use crate::db::IndexRangeQuerySpec;

        // Build the index range query specification
        let spec = IndexRangeQuerySpec {
            table: self.table_name,
            index: self.index_name,
            start_bound: self.start_bound,
            end_bound: self.end_bound,
            direction: self.direction,
            limit: self.limit,
            cursor: self.cursor,
            filter_expression: self.filter_expression,
        };

        // Serialize the query specification
        let spec_json = serde_json::to_vec(&spec)?;

        // Allocate memory and write the query spec
        let spec_ptr = wasm_helpers::alloc_and_write(&spec_json)?;

        // Call the host function
        let result_ptr = unsafe {
            __convex_db_query_advanced(spec_ptr, spec_json.len() as i32)
        };

        // Free the input memory
        wasm_helpers::free_ptr(spec_ptr);

        // Parse the result
        let host_result = unsafe { wasm_helpers::parse_host_result(result_ptr)? };
        let data = wasm_helpers::handle_host_result(host_result)?;

        // Deserialize the paginated result
        match data {
            Some(value) => {
                let result: PaginatedResult =
                    serde_json::from_value(value).map_err(ConvexError::Serialization)?;
                Ok(result)
            }
            None => Ok(PaginatedResult {
                documents: Vec::new(),
                next_cursor: None,
                has_more: false,
                page_status: None,
                split_cursor: None,
                rows_read: None,
                continuation_cursor: None,
            }),
        }
    }
}

/// Index range query specification sent to the host
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexRangeQuerySpec {
    table: String,
    index: String,
    #[serde(rename = "startBound")]
    start_bound: RangeBound,
    #[serde(rename = "endBound")]
    end_bound: RangeBound,
    direction: ScanDirection,
    limit: Option<usize>,
    cursor: Option<String>,
    #[serde(rename = "filterExpression", skip_serializing_if = "Option::is_none")]
    filter_expression: Option<crate::db::FilterExpression>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_range_bound_inclusive() {
        let bound = RangeBound::Inclusive(serde_json::json!(42));
        match bound {
            RangeBound::Inclusive(v) => assert_eq!(v, 42),
            _ => panic!("Expected inclusive bound"),
        }
    }

    #[test]
    fn test_range_bound_exclusive() {
        let bound = RangeBound::Exclusive(serde_json::json!("test"));
        match bound {
            RangeBound::Exclusive(v) => assert_eq!(v, "test"),
            _ => panic!("Expected exclusive bound"),
        }
    }

    #[test]
    fn test_scan_direction_default() {
        let dir: ScanDirection = Default::default();
        match dir {
            ScanDirection::Asc => {},
            _ => panic!("Expected Asc as default"),
        }
    }

    #[test]
    fn test_index_range_new() {
        let range = IndexRange::new("by_age");
        assert_eq!(range.index_name, "by_age");
        match range.start_bound {
            RangeBound::Unbounded => {},
            _ => panic!("Expected unbounded start"),
        }
        match range.end_bound {
            RangeBound::Unbounded => {},
            _ => panic!("Expected unbounded end"),
        }
    }

    #[test]
    fn test_index_range_bounds() {
        let range = IndexRange::new("by_age")
            .gt(18).unwrap()
            .lte(65).unwrap();

        match range.start_bound {
            RangeBound::Exclusive(v) => assert_eq!(v, 18),
            _ => panic!("Expected exclusive start"),
        }
        match range.end_bound {
            RangeBound::Inclusive(v) => assert_eq!(v, 65),
            _ => panic!("Expected inclusive end"),
        }
    }

    #[test]
    fn test_index_range_order() {
        let range = IndexRange::new("by_age")
            .order(ScanDirection::Desc);

        match range.direction {
            ScanDirection::Desc => {},
            _ => panic!("Expected Desc direction"),
        }
    }

    #[test]
    fn test_index_range_serialization() {
        let range = IndexRange::new("by_age")
            .gte(18).unwrap()
            .lte(65).unwrap()
            .order(ScanDirection::Asc)
            .limit(10);

        let json = serde_json::to_string(&range).unwrap();
        assert!(json.contains("by_age"));
        assert!(json.contains("inclusive"));
        assert!(json.contains("18"));
        assert!(json.contains("65"));
    }
}

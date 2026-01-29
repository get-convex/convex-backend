//! Database operations for Convex

pub mod filter;
pub mod index_range;
pub mod table;

use crate::types::{ConvexError, Document, DocumentId, Result};
pub use filter::{FilterBuilder, FilterExpression, FieldRef, FilterExpressionExt};
pub use index_range::{IndexRange, IndexRangeBuilder, RangeBound, ScanDirection, IndexRangeQuerySpec};
pub use table::{TableReader, TableWriter, TableQueryBuilder, IndexQueryBuilder};
use alloc::string::String;
use alloc::vec::Vec;
use serde::{Deserialize, Serialize};

// Host function declarations for WASM environment
// These functions are provided by the Convex runtime
extern "C" {
    /// Query documents from a table
    ///
    /// # Safety
    /// - `table_ptr` must be a valid pointer to a UTF-8 encoded table name
    /// - `table_len` must be the length of the table name in bytes
    /// - Returns a pointer to a length-prefixed JSON result, or -1 on error
    fn __convex_db_query(table_ptr: i32, table_len: i32) -> i32;

    /// Get a single document by ID
    ///
    /// # Safety
    /// - `id_ptr` must be a valid pointer to a UTF-8 encoded document ID
    /// - `id_len` must be the length of the ID in bytes
    /// - Returns a pointer to a length-prefixed JSON result, or -1 on error
    fn __convex_db_get(id_ptr: i32, id_len: i32) -> i32;

    /// Insert a new document into a table
    ///
    /// # Safety
    /// - `table_ptr` must be a valid pointer to a UTF-8 encoded table name
    /// - `table_len` must be the length of the table name in bytes
    /// - `value_ptr` must be a valid pointer to a UTF-8 encoded JSON value
    /// - `value_len` must be the length of the JSON value in bytes
    /// - Returns a pointer to a length-prefixed JSON result containing the new ID, or -1 on error
    fn __convex_db_insert(table_ptr: i32, table_len: i32, value_ptr: i32, value_len: i32) -> i32;

    /// Patch an existing document
    ///
    /// # Safety
    /// - `id_ptr` must be a valid pointer to a UTF-8 encoded document ID
    /// - `id_len` must be the length of the ID in bytes
    /// - `value_ptr` must be a valid pointer to a UTF-8 encoded JSON patch value
    /// - `value_len` must be the length of the JSON value in bytes
    fn __convex_db_patch(id_ptr: i32, id_len: i32, value_ptr: i32, value_len: i32);

    /// Delete a document by ID
    ///
    /// # Safety
    /// - `id_ptr` must be a valid pointer to a UTF-8 encoded document ID
    /// - `id_len` must be the length of the ID in bytes
    fn __convex_db_delete(id_ptr: i32, id_len: i32);

    /// Replace an existing document completely
    ///
    /// # Safety
    /// - `id_ptr` must be a valid pointer to a UTF-8 encoded document ID
    /// - `id_len` must be the length of the ID in bytes
    /// - `value_ptr` must be a valid pointer to a UTF-8 encoded JSON value
    /// - `value_len` must be the length of the JSON value in bytes
    fn __convex_db_replace(id_ptr: i32, id_len: i32, value_ptr: i32, value_len: i32);

    /// Query with filters and options
    ///
    /// # Safety
    /// - `query_ptr` must be a valid pointer to a UTF-8 encoded JSON query specification
    /// - `query_len` must be the length of the query in bytes
    /// - Returns a pointer to a length-prefixed JSON result, or -1 on error
    fn __convex_db_query_advanced(query_ptr: i32, query_len: i32) -> i32;

    /// Count documents matching a query
    ///
    /// # Safety
    /// - `table_ptr` must be a valid pointer to a UTF-8 encoded table name
    /// - `table_len` must be the length of the table name in bytes
    /// - Returns a pointer to a length-prefixed JSON result, or -1 on error
    fn __convex_db_count(table_ptr: i32, table_len: i32) -> i32;

    /// Allocate memory in the host
    ///
    /// # Safety
    /// - `size` must be a positive integer representing the number of bytes to allocate
    /// - Returns a pointer to the allocated memory, or 0 on allocation failure
    fn __convex_alloc(size: i32) -> i32;

    /// Free memory allocated by __convex_alloc
    ///
    /// # Safety
    /// - `ptr` must be a pointer previously returned by __convex_alloc
    /// - `ptr` must not have been freed before
    fn __convex_free(ptr: i32);
}

/// Result from a host function call
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct HostResult {
    success: bool,
    data: Option<serde_json::Value>,
    error: Option<String>,
}

/// Helper functions for WASM memory management
pub(crate) mod wasm_helpers {
    use super::*;

    /// Write bytes to WASM memory at the given pointer
    ///
    /// # Safety
    /// The pointer must be valid and point to at least `len` bytes of allocated memory.
    pub unsafe fn write_bytes(ptr: i32, bytes: &[u8]) {
        let dest = ptr as *mut u8;
        for (i, &byte) in bytes.iter().enumerate() {
            dest.add(i).write(byte);
        }
    }

    /// Read bytes from WASM memory at the given pointer with the given length
    ///
    /// # Safety
    /// The pointer must be valid and point to at least `len` bytes of readable memory.
    pub unsafe fn read_bytes(ptr: i32, len: usize) -> Vec<u8> {
        let src = ptr as *const u8;
        let mut result = Vec::with_capacity(len);
        for i in 0..len {
            result.push(src.add(i).read());
        }
        result
    }

    /// Read a null-terminated string from WASM memory
    ///
    /// # Safety
    /// The pointer must be valid and point to a null-terminated string.
    pub unsafe fn read_string(ptr: i32) -> Result<String> {
        let src = ptr as *const u8;
        let mut bytes = Vec::new();
        let mut i = 0;
        loop {
            let byte = src.add(i).read();
            if byte == 0 {
                break;
            }
            bytes.push(byte);
            i += 1;
            // Safety limit to prevent infinite loops
            if i > 10_000_000 {
                return Err(ConvexError::Unknown(
                    "String too long or not null-terminated".into(),
                ));
            }
        }
        String::from_utf8(bytes).map_err(|e| ConvexError::Unknown(e.to_string()))
    }

    /// Allocate memory and write bytes to it
    pub fn alloc_and_write(bytes: &[u8]) -> Result<i32> {
        let ptr = unsafe { __convex_alloc(bytes.len() as i32) };
        if ptr == 0 {
            return Err(ConvexError::Unknown("Failed to allocate memory".into()));
        }
        unsafe {
            write_bytes(ptr, bytes);
        }
        Ok(ptr)
    }

    /// Free memory allocated by the host
    pub fn free_ptr(ptr: i32) {
        unsafe {
            __convex_free(ptr);
        }
    }

    /// Parse a host result pointer into a HostResult
    /// The host returns length-prefixed data: 4-byte little-endian length followed by JSON data
    pub unsafe fn parse_host_result(result_ptr: i32) -> Result<HostResult> {
        if result_ptr == 0 {
            return Err(ConvexError::Unknown("Null result from host".into()));
        }

        // Read the length prefix (4 bytes, little-endian)
        let response_len = core::ptr::read_unaligned(result_ptr as *const u32) as usize;

        // Read the JSON data after the length prefix
        let response_data = core::slice::from_raw_parts(
            (result_ptr + 4) as *const u8,
            response_len,
        );
        let json_str = String::from_utf8(response_data.to_vec())
            .map_err(|e| ConvexError::Unknown(format!("UTF-8 decode error: {}", e)))?;

        free_ptr(result_ptr);

        let result: HostResult =
            serde_json::from_str(&json_str).map_err(ConvexError::Serialization)?;
        Ok(result)
    }

    /// Handle a host result, converting errors to ConvexError
    pub fn handle_host_result(result: HostResult) -> Result<Option<serde_json::Value>> {
        if result.success {
            Ok(result.data)
        } else {
            let error_msg = result.error.unwrap_or_else(|| "Unknown error".into());
            Err(ConvexError::Database(error_msg))
        }
    }
}

/// The database handle for Convex operations
#[derive(Debug)]
pub struct Database {
    // Opaque handle - in WASM this would be a host reference
    _handle: u32,
}

impl Database {
    /// Create a new database handle (internal use only)
    pub fn new(handle: u32) -> Self {
        Self { _handle: handle }
    }

    /// Query a table
    pub async fn query(&self, table: &str) -> QueryBuilder {
        QueryBuilder::new(table)
    }

    /// Create a batch query builder for executing multiple queries in a single round-trip
    ///
    /// # Example
    ///
    /// ```ignore
    /// let batch = db.batch();
    /// batch.add(BatchQuery::new("users", "users").limit(10));
    /// batch.add(BatchQuery::new("posts", "posts").filter("author", "eq", "user123")?);
    ///
    /// let results = batch.execute().await?;
    /// let users = results.results.get("users");
    /// let posts = results.results.get("posts");
    /// ```
    pub fn batch(&self) -> BatchQueryBuilder {
        BatchQueryBuilder::new()
    }

    /// Get a single document by ID
    pub async fn get(&self, id: DocumentId) -> Result<Option<Document>> {
        // Serialize the document ID
        let id_str = id.as_str();
        let id_bytes = id_str.as_bytes();

        // Allocate memory and write the ID
        let id_ptr = wasm_helpers::alloc_and_write(id_bytes)?;

        // Call the host function
        let result_ptr = unsafe { __convex_db_get(id_ptr, id_bytes.len() as i32) };

        // Free the input memory
        wasm_helpers::free_ptr(id_ptr);

        // Parse the result
        let host_result = unsafe { wasm_helpers::parse_host_result(result_ptr)? };
        let data = wasm_helpers::handle_host_result(host_result)?;

        // Deserialize the document
        match data {
            Some(value) => {
                let doc: Document = serde_json::from_value(value).map_err(ConvexError::Serialization)?;
                Ok(Some(doc))
            }
            None => Ok(None),
        }
    }

    /// Insert a new document
    pub async fn insert(
        &self,
        table: &str,
        value: impl serde::Serialize,
    ) -> Result<DocumentId> {
        // Validate table name
        if table.is_empty() {
            return Err(ConvexError::InvalidArgument("Table name cannot be empty".into()));
        }

        // Serialize the value to JSON
        let value_json = serde_json::to_vec(&value)?;

        // Serialize the table name
        let table_bytes = table.as_bytes();

        // Allocate memory for table and value
        let table_ptr = wasm_helpers::alloc_and_write(table_bytes)?;
        let value_ptr = wasm_helpers::alloc_and_write(&value_json)?;

        // Call the host function
        let result_ptr = unsafe {
            __convex_db_insert(table_ptr, table_bytes.len() as i32, value_ptr, value_json.len() as i32)
        };

        // Free the input memory
        wasm_helpers::free_ptr(table_ptr);
        wasm_helpers::free_ptr(value_ptr);

        // Parse the result
        let host_result = unsafe { wasm_helpers::parse_host_result(result_ptr)? };
        let data = wasm_helpers::handle_host_result(host_result)?;

        // Extract the document ID from the result
        match data {
            Some(value) => {
                let id_str = value.as_str()
                    .ok_or_else(|| ConvexError::Database("Invalid document ID returned".into()))?;
                Ok(DocumentId::new(id_str))
            }
            None => Err(ConvexError::Database("No document ID returned".into())),
        }
    }

    /// Patch an existing document
    pub async fn patch(
        &self,
        id: DocumentId,
        value: impl serde::Serialize,
    ) -> Result<()> {
        // Serialize the document ID
        let id_str = id.as_str();
        let id_bytes = id_str.as_bytes();

        // Serialize the patch value
        let value_json = serde_json::to_vec(&value)?;

        // Allocate memory for ID and value
        let id_ptr = wasm_helpers::alloc_and_write(id_bytes)?;
        let value_ptr = wasm_helpers::alloc_and_write(&value_json)?;

        // Call the host function (no return value for patch)
        unsafe {
            __convex_db_patch(id_ptr, id_bytes.len() as i32, value_ptr, value_json.len() as i32);
        }

        // Free the input memory
        wasm_helpers::free_ptr(id_ptr);
        wasm_helpers::free_ptr(value_ptr);

        Ok(())
    }

    /// Replace an existing document completely
    ///
    /// Unlike patch, replace overwrites the entire document with the new value,
    /// preserving only the document ID.
    ///
    /// # Arguments
    ///
    /// * `id` - The document ID to replace
    /// * `value` - The new document value
    ///
    /// # Returns
    ///
    /// Ok(()) on success
    ///
    /// # Example
    ///
    /// ```ignore
    /// #[mutation]
    /// async fn update_user(db: Database, id: String, new_data: UserData) -> Result<()> {
    ///     db.replace(id.into(), new_data).await
    /// }
    /// ```
    pub async fn replace(
        &self,
        id: DocumentId,
        value: impl serde::Serialize,
    ) -> Result<()> {
        use crate::db::__convex_db_replace;

        // Serialize the document ID
        let id_str = id.as_str();
        let id_bytes = id_str.as_bytes();

        // Serialize the replacement value
        let value_json = serde_json::to_vec(&value)?;

        // Allocate memory for ID and value
        let id_ptr = wasm_helpers::alloc_and_write(id_bytes)?;
        let value_ptr = wasm_helpers::alloc_and_write(&value_json)?;

        // Call the host function (no return value for replace)
        unsafe {
            __convex_db_replace(id_ptr, id_bytes.len() as i32, value_ptr, value_json.len() as i32);
        }

        // Free the input memory
        wasm_helpers::free_ptr(id_ptr);
        wasm_helpers::free_ptr(value_ptr);

        Ok(())
    }

    /// Delete a document
    pub async fn delete(&self, id: DocumentId) -> Result<()> {
        // Serialize the document ID
        let id_str = id.as_str();
        let id_bytes = id_str.as_bytes();

        // Allocate memory and write the ID
        let id_ptr = wasm_helpers::alloc_and_write(id_bytes)?;

        // Call the host function (no return value for delete)
        unsafe {
            __convex_db_delete(id_ptr, id_bytes.len() as i32);
        }

        // Free the input memory
        wasm_helpers::free_ptr(id_ptr);

        Ok(())
    }

    /// Get a table-scoped reader for database operations
    ///
    /// This provides a type-safe interface for operations on a specific table.
    /// In queries, this returns a `TableReader` with read-only operations.
    ///
    /// # Arguments
    ///
    /// * `table_name` - The name of the table
    ///
    /// # Returns
    ///
    /// A `TableReader` for the specified table
    ///
    /// # Example
    ///
    /// ```ignore
    /// #[query]
    /// async fn get_user(db: Database, id: String) -> Result<Option<Document>> {
    ///     let users: TableReader = db.table("users");
    ///     users.get(id.into()).await
    /// }
    /// ```
    pub fn table(&self, table_name: impl Into<String>) -> TableReader {
        TableReader::new(table_name)
    }
}

/// Filter condition for queries
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilterCondition {
    field: String,
    op: String,
    value: serde_json::Value,
}

/// Ordering specification for queries
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderSpec {
    field: String,
    ascending: bool,
}

/// Query specification sent to the host
#[derive(Debug, Clone, Serialize, Deserialize)]
struct QuerySpec {
    table: String,
    filters: Vec<FilterCondition>,
    orders: Vec<OrderSpec>,
    limit: Option<usize>,
    skip: Option<usize>,
    cursor: Option<String>,
    /// Optional filter expression for complex queries
    #[serde(rename = "filterExpression", skip_serializing_if = "Option::is_none")]
    filter_expression: Option<FilterExpression>,
    /// Maximum number of rows to read before stopping (not just returned)
    #[serde(rename = "maximumRowsRead", skip_serializing_if = "Option::is_none")]
    maximum_rows_read: Option<usize>,
}

/// Status of a paginated query page
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PageStatus {
    /// There are more results available
    HasMore,
    /// This is the last page of results
    LastPage,
    /// The query was truncated due to maximumRowsRead limit
    Truncated,
}

/// Result of a paginated query
#[derive(Debug, Clone, Deserialize)]
pub struct PaginatedResult {
    /// Documents in the current page
    pub documents: Vec<Document>,
    /// Cursor for fetching the next page (None if no more results)
    pub next_cursor: Option<String>,
    /// Whether there are more results available (legacy field)
    pub has_more: bool,
    /// Detailed status of this page
    #[serde(rename = "pageStatus", default)]
    pub page_status: Option<PageStatus>,
    /// Split cursor for parallel processing of large result sets
    #[serde(rename = "splitCursor", default)]
    pub split_cursor: Option<String>,
    /// Number of rows read (scanned) to produce this result
    #[serde(rename = "rowsRead", default)]
    pub rows_read: Option<usize>,
    /// Continuation cursor for resuming a truncated query
    #[serde(rename = "continuationCursor", default)]
    pub continuation_cursor: Option<String>,
}

/// A query to be executed as part of a batch
#[derive(Debug, Clone, Serialize)]
pub struct BatchQuery {
    /// Unique identifier for this query in the batch
    pub id: String,
    /// Table to query
    pub table: String,
    /// Optional filter conditions
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub filters: Vec<FilterCondition>,
    /// Optional ordering
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub orders: Vec<OrderSpec>,
    /// Pagination limit
    pub limit: Option<usize>,
    /// Pagination skip
    pub skip: Option<usize>,
}

impl BatchQuery {
    /// Create a new batch query
    pub fn new(id: impl Into<String>, table: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            table: table.into(),
            filters: Vec::new(),
            orders: Vec::new(),
            limit: None,
            skip: None,
        }
    }

    /// Add a filter to this batch query
    pub fn filter(mut self, field: &str, op: &str, value: impl Serialize) -> Result<Self> {
        let value_json = serde_json::to_value(value)?;
        self.filters.push(FilterCondition {
            field: field.to_string(),
            op: op.to_string(),
            value: value_json,
        });
        Ok(self)
    }

    /// Set the limit for this batch query
    pub fn limit(mut self, n: usize) -> Self {
        self.limit = Some(n);
        self
    }
}

/// Result of a single query in a batch
#[derive(Debug, Clone, Deserialize)]
pub struct BatchQueryItemResult {
    /// Documents returned by this query
    pub documents: Vec<Document>,
    /// Cursor for the next page
    pub next_cursor: Option<String>,
    /// Whether there are more results
    pub has_more: bool,
}

/// Result of a batch query operation
#[derive(Debug, Clone, Deserialize)]
pub struct BatchQueryResult {
    /// Results keyed by query ID
    #[serde(flatten)]
    pub results: alloc::collections::BTreeMap<String, BatchQueryItemResult>,
}

/// Builder for batch queries
#[derive(Debug)]
pub struct BatchQueryBuilder {
    queries: Vec<BatchQuery>,
}

impl BatchQueryBuilder {
    /// Create a new batch query builder
    pub fn new() -> Self {
        Self {
            queries: Vec::new(),
        }
    }

    /// Add a query to the batch
    pub fn add(&mut self, query: BatchQuery) -> &mut Self {
        self.queries.push(query);
        self
    }

    /// Execute the batch query
    pub async fn execute(&self) -> Result<BatchQueryResult> {
        // For now, this is a placeholder that would call the host function
        // In a real implementation, this would call __convex_db_query_batch
        Err(ConvexError::Unknown(
            "Batch queries not yet implemented in host".into(),
        ))
    }
}

impl Default for BatchQueryBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Builder for database queries
#[derive(Debug)]
pub struct QueryBuilder {
    table: String,
    filters: Vec<FilterCondition>,
    orders: Vec<OrderSpec>,
    limit: Option<usize>,
    skip: Option<usize>,
    cursor: Option<String>,
    filter_expression: Option<FilterExpression>,
    maximum_rows_read: Option<usize>,
}

impl QueryBuilder {
    fn new(table: &str) -> Self {
        Self {
            table: table.to_string(),
            filters: Vec::new(),
            orders: Vec::new(),
            limit: None,
            skip: None,
            cursor: None,
            filter_expression: None,
            maximum_rows_read: None,
        }
    }

    /// Add a filter expression for complex queries
    ///
    /// This method allows building complex filter expressions using the
    /// `FilterBuilder` API, supporting logical operators (and, or, not),
    /// comparison operators (eq, neq, lt, lte, gt, gte), and arithmetic
    /// operations.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use convex_sdk::FilterBuilder;
    ///
    /// let f = FilterBuilder::new();
    /// let users = db.query("users")
    ///     .filter_expr(f.and(
    ///         f.eq("status", "active"),
    ///         f.gt("created_at", "2024-01-01")
    ///     ))
    ///     .collect()
    ///     .await?;
    /// ```
    pub fn filter_expr(mut self, expression: FilterExpression) -> Self {
        self.filter_expression = Some(expression);
        self
    }

    /// Set the maximum number of rows to read before stopping
    ///
    /// This limits the number of rows scanned (read) by the query, not just
    /// the number returned. This is useful for preventing expensive queries
    /// from scanning too much data.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let users = db.query("users")
    ///     .filter("status", "eq", "active")?
    ///     .maximum_rows_read(1000)
    ///     .collect()
    ///     .await?;
    /// ```
    pub fn maximum_rows_read(mut self, max: usize) -> Self {
        self.maximum_rows_read = Some(max);
        self
    }

    /// Execute the query and return all results
    pub async fn collect(&self) -> Result<Vec<Document>> {
        // Build the query specification
        let spec = QuerySpec {
            table: self.table.clone(),
            filters: self.filters.clone(),
            orders: self.orders.clone(),
            limit: self.limit,
            skip: self.skip,
            cursor: self.cursor.clone(),
            filter_expression: self.filter_expression.clone(),
            maximum_rows_read: self.maximum_rows_read,
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

    /// Count the number of matching documents
    pub async fn count(&self) -> Result<usize> {
        // Serialize the table name
        let table_bytes = self.table.as_bytes();

        // Allocate memory and write the table name
        let table_ptr = wasm_helpers::alloc_and_write(table_bytes)?;

        // Call the host function
        let result_ptr = unsafe {
            __convex_db_count(table_ptr, table_bytes.len() as i32)
        };

        // Free the input memory
        wasm_helpers::free_ptr(table_ptr);

        // Parse the result
        let host_result = unsafe { wasm_helpers::parse_host_result(result_ptr)? };
        let data = wasm_helpers::handle_host_result(host_result)?;

        // Extract the count from the result
        match data {
            Some(value) => {
                let count = value.as_u64()
                    .ok_or_else(|| ConvexError::Database("Invalid count returned".into()))?;
                Ok(count as usize)
            }
            None => Ok(0),
        }
    }

    /// Take only the first N results
    pub fn limit(mut self, n: usize) -> Self {
        self.limit = Some(n);
        self
    }

    /// Order results by a field
    pub fn order(mut self, field: &str, ascending: bool) -> Self {
        self.orders.push(OrderSpec {
            field: field.to_string(),
            ascending,
        });
        self
    }

    /// Add a filter condition to the query
    pub fn filter(mut self, field: &str, op: &str, value: impl serde::Serialize) -> Result<Self> {
        let value_json = serde_json::to_value(value)?;
        self.filters.push(FilterCondition {
            field: field.to_string(),
            op: op.to_string(),
            value: value_json,
        });
        Ok(self)
    }

    /// Skip N results (offset-based pagination)
    ///
    /// # Example
    ///
    /// ```ignore
    /// // Get page 2 with 10 items per page
    /// let page = db.query("users")
    ///     .skip(10)
    ///     .limit(10)
    ///     .collect()
    ///     .await?;
    /// ```
    pub fn skip(mut self, n: usize) -> Self {
        self.skip = Some(n);
        self
    }

    /// Set cursor for cursor-based pagination
    ///
    /// Use the `next_cursor` from a previous paginated result.
    ///
    /// # Example
    ///
    /// ```ignore
    /// // Get first page
    /// let result = db.query("users")
    ///     .limit(10)
    ///     .paginate()
    ///     .await?;
    ///
    /// // Get next page using cursor
    /// if result.has_more {
    ///     let next_page = db.query("users")
    ///         .cursor(result.next_cursor.unwrap())
    ///         .limit(10)
    ///         .paginate()
    ///         .await?;
    /// }
    /// ```
    pub fn cursor(mut self, cursor: impl Into<String>) -> Self {
        self.cursor = Some(cursor.into());
        self
    }

    /// Execute the query and return paginated results
    ///
    /// Returns a `PaginatedResult` with documents, next_cursor, and has_more flag.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let result = db.query("users")
    ///     .limit(10)
    ///     .paginate()
    ///     .await?;
    ///
    /// for doc in result.documents {
    ///     println!("{:?}", doc);
    /// }
    ///
    /// if result.has_more {
    ///     println!("More results available");
    /// }
    /// ```
    pub async fn paginate(&self) -> Result<PaginatedResult> {
        // Build the query specification
        let spec = QuerySpec {
            table: self.table.clone(),
            filters: self.filters.clone(),
            orders: self.orders.clone(),
            limit: self.limit,
            skip: self.skip,
            cursor: self.cursor.clone(),
            filter_expression: self.filter_expression.clone(),
            maximum_rows_read: self.maximum_rows_read,
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

    /// Take only the first N results
    ///
    /// This is a convenience method that sets the limit and collects results.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let users = db.query("users")
    ///     .take(5)
    ///     .await?;
    /// ```
    pub async fn take(self, n: usize) -> Result<Vec<Document>> {
        self.limit(n).collect().await
    }

    /// Get the first result, or None if no results
    ///
    /// # Example
    ///
    /// ```ignore
    /// let user = db.query("users")
    ///     .filter("email", "eq", "alice@example.com")?
    ///     .first()
    ///     .await?;
    /// ```
    pub async fn first(mut self) -> Result<Option<Document>> {
        self.limit = Some(1);
        let results = self.collect().await?;
        Ok(results.into_iter().next())
    }

    /// Get exactly one result, or return an error
    ///
    /// Returns an error if zero or more than one result is found.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let user = db.query("users")
    ///     .filter("email", "eq", "alice@example.com")?
    ///     .unique()
    ///     .await?;
    /// ```
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

    /// Query using a search index
    ///
    /// This method performs a full-text search using a defined search index.
    ///
    /// # Arguments
    ///
    /// * `index_name` - The name of the search index to use
    /// * `search_text` - The search query text
    ///
    /// # Example
    ///
    /// ```ignore
    /// let posts = db.query("posts")
    ///     .with_search_index("search_content", "rust tutorial")
    ///     .filter("published", true)?
    ///     .limit(10)
    ///     .collect()
    ///     .await?;
    /// ```
    pub fn with_search_index(
        self,
        index_name: impl Into<String>,
        search_text: impl Into<String>,
    ) -> crate::search::SearchQueryBuilder {
        crate::search::SearchQueryBuilder::new(self.table, index_name, search_text)
    }
}

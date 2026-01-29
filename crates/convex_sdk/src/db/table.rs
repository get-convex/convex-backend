//! Table-scoped database operations
//!
//! This module provides a type-safe interface for database operations
//! scoped to a specific table, similar to `ctx.db.table("name")` in TypeScript.
//!
//! # Example
//!
//! ```ignore
//! use convex_sdk::{Database, TableReader, TableWriter};
//!
//! #[query]
//! async fn get_user(db: Database, id: String) -> Result<Option<Document>> {
//!     let users: TableReader = db.table("users");
//!     users.get(id.into()).await
//! }
//!
//! #[mutation]
//! async fn create_user(db: Database, name: String, email: String) -> Result<DocumentId> {
//!     let users: TableWriter = db.table("users");
//!     users.insert(json!({
//!         "name": name,
//!         "email": email,
//!     })).await
//! }
//! ```

use crate::types::{ConvexError, Document, DocumentId, Result};
use crate::db::{FilterCondition, OrderSpec, PaginatedResult};
use crate::db::index_range::IndexRangeBuilder;
use alloc::string::String;
use alloc::vec::Vec;
use serde::{Deserialize, Serialize};

/// A table reader provides read-only access to a specific table
///
/// This type is returned by `Database::table()` and provides type-safe
/// read operations scoped to a single table.
#[derive(Debug, Clone)]
pub struct TableReader {
    table_name: String,
}

impl TableReader {
    /// Create a new table reader for the given table
    pub(crate) fn new(table_name: impl Into<String>) -> Self {
        Self {
            table_name: table_name.into(),
        }
    }

    /// Get the table name
    pub fn name(&self) -> &str {
        &self.table_name
    }

    /// Get a single document by ID from this table
    ///
    /// # Arguments
    ///
    /// * `id` - The document ID
    ///
    /// # Returns
    ///
    /// The document if found, None if not found
    ///
    /// # Errors
    ///
    /// Returns an error if the ID doesn't belong to this table
    ///
    /// # Example
    ///
    /// ```ignore
    /// let users: TableReader = db.table("users");
    /// let user = users.get(DocumentId::new("users:abc123")).await?;
    /// ```
    pub async fn get(&self, id: DocumentId) -> Result<Option<Document>> {
        // Verify the ID belongs to this table
        let id_str = id.as_str();
        if !id_str.starts_with(&format!("{}:", self.table_name)) {
            return Err(ConvexError::InvalidArgument(
                format!(
                    "Document ID '{}' does not belong to table '{}'",
                    id_str, self.table_name
                )
            ));
        }

        // Get the document
        self.query().with_id_filter(id.clone()).first().await
    }

    /// Start a new query on this table
    ///
    /// # Returns
    ///
    /// A `TableQueryBuilder` for building the query
    ///
    /// # Example
    ///
    /// ```ignore
    /// let users: TableReader = db.table("users");
    /// let active_users = users.query()
    ///     .filter("active", "eq", true)?
    ///     .order("created_at", false)
    ///     .collect()
    ///     .await?;
    /// ```
    pub fn query(&self) -> TableQueryBuilder {
        TableQueryBuilder::new(self.table_name.clone())
    }

    /// Query this table with a specific index
    ///
    /// # Arguments
    ///
    /// * `index_name` - The name of the index to use
    ///
    /// # Returns
    ///
    /// An `IndexQueryBuilder` for building the index query
    ///
    /// # Example
    ///
    /// ```ignore
    /// let users: TableReader = db.table("users");
    /// let user = users.with_index("by_email")
    ///     .eq("email", "user@example.com")?
    ///     .first()
    ///     .await?;
    /// ```
    pub fn with_index(&self, index_name: impl Into<String>) -> IndexQueryBuilder {
        IndexQueryBuilder::new(self.table_name.clone(), index_name.into())
    }

    /// Query this table using an index range scan
    ///
    /// This method provides efficient index-based range queries with support for
    /// start/end bounds, scan direction, and pagination.
    ///
    /// # Arguments
    ///
    /// * `index_name` - The name of the index to use
    ///
    /// # Returns
    ///
    /// An `IndexRangeBuilder` for building the range query
    ///
    /// # Example
    ///
    /// ```ignore
    /// let users: TableReader = db.table("users");
    /// let active_users = users.query_index_range("by_age")
    ///     .gte(18)?
    ///     .lte(65)?
    ///     .ascending()
    ///     .collect()
    ///     .await?;
    /// ```
    pub fn query_index_range(&self, index_name: impl Into<String>) -> IndexRangeBuilder {
        IndexRangeBuilder::new(self.table_name.clone(), index_name.into())
    }

    /// Count all documents in this table
    ///
    /// # Returns
    ///
    /// The number of documents in the table
    pub async fn count(&self) -> Result<usize> {
        self.query().count().await
    }
}

/// A table writer provides read-write access to a specific table
///
/// This type is returned by `Database::table()` in mutations and provides
/// type-safe read and write operations scoped to a single table.
#[derive(Debug, Clone)]
pub struct TableWriter {
    reader: TableReader,
}

impl TableWriter {
    /// Create a new table writer for the given table
    pub(crate) fn new(table_name: impl Into<String>) -> Self {
        Self {
            reader: TableReader::new(table_name),
        }
    }

    /// Get the table name
    pub fn name(&self) -> &str {
        self.reader.name()
    }

    /// Get a single document by ID from this table
    pub async fn get(&self, id: DocumentId) -> Result<Option<Document>> {
        self.reader.get(id).await
    }

    /// Start a new query on this table
    pub fn query(&self) -> TableQueryBuilder {
        self.reader.query()
    }

    /// Query this table with a specific index
    pub fn with_index(&self, index_name: impl Into<String>) -> IndexQueryBuilder {
        self.reader.with_index(index_name)
    }

    /// Query this table using an index range scan
    pub fn query_index_range(&self, index_name: impl Into<String>) -> IndexRangeBuilder {
        self.reader.query_index_range(index_name)
    }

    /// Count all documents in this table
    pub async fn count(&self) -> Result<usize> {
        self.reader.count().await
    }

    /// Insert a new document into this table
    ///
    /// # Arguments
    ///
    /// * `value` - The document value to insert
    ///
    /// # Returns
    ///
    /// The ID of the newly created document
    ///
    /// # Example
    ///
    /// ```ignore
    /// let users: TableWriter = db.table("users");
    /// let id = users.insert(json!({
    ///     "name": "Alice",
    ///     "email": "alice@example.com",
    /// })).await?;
    /// ```
    pub async fn insert(&self, value: impl Serialize) -> Result<DocumentId> {
        use crate::db::wasm_helpers;
        use crate::db::__convex_db_insert;

        // Serialize the value to JSON
        let value_json = serde_json::to_vec(&value)?;
        let table_bytes = self.reader.table_name.as_bytes();

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

    /// Patch an existing document in this table
    ///
    /// # Arguments
    ///
    /// * `id` - The document ID
    /// * `value` - The partial document to merge
    ///
    /// # Returns
    ///
    /// Ok(()) on success
    pub async fn patch(
        &self,
        id: DocumentId,
        value: impl Serialize,
    ) -> Result<()> {
        use crate::db::wasm_helpers;
        use crate::db::__convex_db_patch;

        // Verify the ID belongs to this table
        let id_str = id.as_str();
        if !id_str.starts_with(&format!("{}:", self.reader.table_name)) {
            return Err(ConvexError::InvalidArgument(
                format!(
                    "Document ID '{}' does not belong to table '{}'",
                    id_str, self.reader.table_name
                )
            ));
        }

        // Serialize the document ID and patch value
        let id_bytes = id_str.as_bytes();
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

    /// Replace an existing document in this table
    ///
    /// # Arguments
    ///
    /// * `id` - The document ID
    /// * `value` - The new document value
    ///
    /// # Returns
    ///
    /// Ok(()) on success
    ///
    /// # Note
    ///
    /// Currently this is implemented as a patch operation (merging fields).
    /// In a future version, this will perform a true replace (delete + insert).
    pub async fn replace(
        &self,
        id: DocumentId,
        value: impl Serialize,
    ) -> Result<()> {
        // TODO: Implement true replace semantics (delete + insert atomically)
        // For now, we use patch as a fallback
        self.patch(id, value).await
    }

    /// Delete a document from this table
    ///
    /// # Arguments
    ///
    /// * `id` - The document ID to delete
    ///
    /// # Returns
    ///
    /// Ok(()) on success
    pub async fn delete(&self, id: DocumentId) -> Result<()> {
        use crate::db::wasm_helpers;
        use crate::db::__convex_db_delete;

        // Verify the ID belongs to this table
        let id_str = id.as_str();
        if !id_str.starts_with(&format!("{}:", self.reader.table_name)) {
            return Err(ConvexError::InvalidArgument(
                format!(
                    "Document ID '{}' does not belong to table '{}'",
                    id_str, self.reader.table_name
                )
            ));
        }

        // Serialize the document ID
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
}

/// Query builder scoped to a specific table
#[derive(Debug)]
pub struct TableQueryBuilder {
    table_name: String,
    filters: Vec<FilterCondition>,
    orders: Vec<OrderSpec>,
    limit: Option<usize>,
    filter_expression: Option<crate::db::FilterExpression>,
    maximum_rows_read: Option<usize>,
}

impl TableQueryBuilder {
    /// Create a new table query builder
    fn new(table_name: String) -> Self {
        Self {
            table_name,
            filters: Vec::new(),
            orders: Vec::new(),
            limit: None,
            filter_expression: None,
            maximum_rows_read: None,
        }
    }

    /// Add a filter condition
    pub fn filter(mut self, field: &str, op: &str, value: impl Serialize) -> Result<Self> {
        let value_json = serde_json::to_value(value)?;
        self.filters.push(FilterCondition {
            field: field.to_string(),
            op: op.to_string(),
            value: value_json,
        });
        Ok(self)
    }

    /// Add a filter expression for complex queries
    ///
    /// This method allows building complex filter expressions using the
    /// `FilterBuilder` API with logical operators (and, or, not),
    /// comparison operators, and arithmetic operations.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use convex_sdk::FilterBuilder;
    ///
    /// let f = FilterBuilder::new();
    /// let users = db.table("users")
    ///     .query()
    ///     .filter_expr(f.and(
    ///         f.eq("status", "active"),
    ///         f.gt("created_at", "2024-01-01")
    ///     ))
    ///     .collect()
    ///     .await?;
    /// ```
    pub fn filter_expr(mut self, expression: crate::db::FilterExpression) -> Self {
        self.filter_expression = Some(expression);
        self
    }

    /// Set the maximum number of rows to read before stopping
    pub fn maximum_rows_read(mut self, max: usize) -> Self {
        self.maximum_rows_read = Some(max);
        self
    }

    /// Filter by document ID
    pub(crate) fn with_id_filter(mut self, id: DocumentId) -> Self {
        self.filters.push(FilterCondition {
            field: "_id".to_string(),
            op: "eq".to_string(),
            value: serde_json::Value::String(id.as_str().to_string()),
        });
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

    /// Limit the number of results
    pub fn limit(mut self, n: usize) -> Self {
        self.limit = Some(n);
        self
    }

    /// Execute the query and return all results
    pub async fn collect(self) -> Result<Vec<Document>> {
        use crate::db::wasm_helpers;
        use crate::db::__convex_db_query_advanced;
        use crate::db::QuerySpec;

        // Build the query specification
        let spec = QuerySpec {
            table: self.table_name.clone(),
            filters: self.filters,
            orders: self.orders,
            limit: self.limit,
            skip: None,
            cursor: None,
            filter_expression: self.filter_expression,
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

    /// Count the number of matching documents
    pub async fn count(self) -> Result<usize> {
        use crate::db::wasm_helpers;
        use crate::db::__convex_db_count;

        // Serialize the table name
        let table_bytes = self.table_name.as_bytes();

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

    /// Execute the query and return paginated results
    pub async fn paginate(self) -> Result<PaginatedResult> {
        use crate::db::wasm_helpers;
        use crate::db::__convex_db_query_advanced;
        use crate::db::QuerySpec;

        // Build the query specification
        let spec = QuerySpec {
            table: self.table_name.clone(),
            filters: self.filters,
            orders: self.orders,
            limit: self.limit,
            skip: None,
            cursor: None,
            filter_expression: self.filter_expression,
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
}

/// Query builder for index-based queries
#[derive(Debug)]
pub struct IndexQueryBuilder {
    table_name: String,
    index_name: String,
    filters: Vec<FilterCondition>,
    orders: Vec<OrderSpec>,
    limit: Option<usize>,
    filter_expression: Option<crate::db::FilterExpression>,
}

impl IndexQueryBuilder {
    /// Create a new index query builder
    fn new(table_name: String, index_name: String) -> Self {
        Self {
            table_name,
            index_name,
            filters: Vec::new(),
            orders: Vec::new(),
            limit: None,
            filter_expression: None,
        }
    }

    /// Add an equality filter
    pub fn eq(mut self, field: &str, value: impl Serialize) -> Result<Self> {
        let value_json = serde_json::to_value(value)?;
        self.filters.push(FilterCondition {
            field: field.to_string(),
            op: "eq".to_string(),
            value: value_json,
        });
        Ok(self)
    }

    /// Add a "less than" filter
    pub fn lt(mut self, field: &str, value: impl Serialize) -> Result<Self> {
        let value_json = serde_json::to_value(value)?;
        self.filters.push(FilterCondition {
            field: field.to_string(),
            op: "lt".to_string(),
            value: value_json,
        });
        Ok(self)
    }

    /// Add a "less than or equal" filter
    pub fn lte(mut self, field: &str, value: impl Serialize) -> Result<Self> {
        let value_json = serde_json::to_value(value)?;
        self.filters.push(FilterCondition {
            field: field.to_string(),
            op: "lte".to_string(),
            value: value_json,
        });
        Ok(self)
    }

    /// Add a "greater than" filter
    pub fn gt(mut self, field: &str, value: impl Serialize) -> Result<Self> {
        let value_json = serde_json::to_value(value)?;
        self.filters.push(FilterCondition {
            field: field.to_string(),
            op: "gt".to_string(),
            value: value_json,
        });
        Ok(self)
    }

    /// Add a "greater than or equal" filter
    pub fn gte(mut self, field: &str, value: impl Serialize) -> Result<Self> {
        let value_json = serde_json::to_value(value)?;
        self.filters.push(FilterCondition {
            field: field.to_string(),
            op: "gte".to_string(),
            value: value_json,
        });
        Ok(self)
    }

    /// Add a filter expression for complex queries
    ///
    /// This method allows building complex filter expressions using the
    /// `FilterBuilder` API with logical operators (and, or, not),
    /// comparison operators, and arithmetic operations.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use convex_sdk::FilterBuilder;
    ///
    /// let f = FilterBuilder::new();
    /// let users = db.table("users")
    ///     .with_index("by_status")
    ///     .eq("status", "active")?
    ///     .filter_expr(f.gt("created_at", "2024-01-01"))
    ///     .collect()
    ///     .await?;
    /// ```
    pub fn filter_expr(mut self, expression: crate::db::FilterExpression) -> Self {
        self.filter_expression = Some(expression);
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

    /// Limit the number of results
    pub fn limit(mut self, n: usize) -> Self {
        self.limit = Some(n);
        self
    }

    /// Execute the query and return all results
    pub async fn collect(self) -> Result<Vec<Document>> {
        use crate::db::wasm_helpers;
        use crate::db::__convex_db_query_advanced;

        // Build the index query specification
        let spec = IndexQuerySpec {
            table: self.table_name.clone(),
            index: self.index_name.clone(),
            filters: self.filters,
            orders: self.orders,
            limit: self.limit,
            skip: None,
            cursor: None,
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

        // Build the index query specification
        let spec = IndexQuerySpec {
            table: self.table_name.clone(),
            index: self.index_name.clone(),
            filters: self.filters,
            orders: self.orders,
            limit: self.limit,
            skip: None,
            cursor: None,
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

/// Index query specification sent to the host
#[derive(Debug, Clone, Serialize, Deserialize)]
struct IndexQuerySpec {
    table: String,
    index: String,
    filters: Vec<FilterCondition>,
    orders: Vec<OrderSpec>,
    limit: Option<usize>,
    skip: Option<usize>,
    cursor: Option<String>,
    /// Optional filter expression for complex queries
    #[serde(rename = "filterExpression", skip_serializing_if = "Option::is_none")]
    filter_expression: Option<crate::db::FilterExpression>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_table_reader_name() {
        let table = TableReader::new("users");
        assert_eq!(table.name(), "users");
    }

    #[test]
    fn test_table_writer_name() {
        let table = TableWriter::new("users");
        assert_eq!(table.name(), "users");
    }

    #[test]
    fn test_table_query_builder() {
        let builder = TableQueryBuilder::new("users".to_string());
        // Just verify it compiles and has the right table name
        assert_eq!(builder.table_name, "users");
    }

    #[test]
    fn test_index_query_builder() {
        let builder = IndexQueryBuilder::new("users".to_string(), "by_email".to_string());
        assert_eq!(builder.table_name, "users");
        assert_eq!(builder.index_name, "by_email");
    }
}

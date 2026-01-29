//! Database operations for Convex

use crate::types::{ConvexError, Document, DocumentId, Result};
use alloc::string::String;
use alloc::vec::Vec;
use serde::{Deserialize, Serialize};

// Host function declarations for WASM environment
// These functions are provided by the Convex runtime
extern "C" {
    /// Query documents from a table
    fn __convex_db_query(table_ptr: i32, table_len: i32) -> i32;

    /// Get a single document by ID
    fn __convex_db_get(id_ptr: i32, id_len: i32) -> i32;

    /// Insert a new document into a table
    fn __convex_db_insert(table_ptr: i32, table_len: i32, value_ptr: i32, value_len: i32) -> i32;

    /// Patch an existing document
    fn __convex_db_patch(id_ptr: i32, id_len: i32, value_ptr: i32, value_len: i32);

    /// Delete a document by ID
    fn __convex_db_delete(id_ptr: i32, id_len: i32);

    /// Query with filters and options
    fn __convex_db_query_advanced(query_ptr: i32, query_len: i32) -> i32;

    /// Count documents matching a query
    fn __convex_db_count(table_ptr: i32, table_len: i32) -> i32;

    /// Allocate memory in the host
    fn __convex_alloc(size: i32) -> i32;

    /// Free memory allocated by __convex_alloc
    fn __convex_free(ptr: i32);
}

/// Result from a host function call
#[derive(Debug, Clone, Serialize, Deserialize)]
struct HostResult {
    success: bool,
    data: Option<serde_json::Value>,
    error: Option<String>,
}

/// Helper functions for WASM memory management
mod wasm_helpers {
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
    pub unsafe fn parse_host_result(result_ptr: i32) -> Result<HostResult> {
        if result_ptr == 0 {
            return Err(ConvexError::Unknown("Null result from host".into()));
        }
        let json_str = read_string(result_ptr)?;
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
}

/// Builder for database queries
#[derive(Debug)]
pub struct QueryBuilder {
    table: String,
    filters: Vec<FilterCondition>,
    orders: Vec<OrderSpec>,
    limit: Option<usize>,
}

impl QueryBuilder {
    fn new(table: &str) -> Self {
        Self {
            table: table.to_string(),
            filters: Vec::new(),
            orders: Vec::new(),
            limit: None,
        }
    }

    /// Execute the query and return all results
    pub async fn collect(&self) -> Result<Vec<Document>> {
        // Build the query specification
        let spec = QuerySpec {
            table: self.table.clone(),
            filters: self.filters.clone(),
            orders: self.orders.clone(),
            limit: self.limit,
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
}

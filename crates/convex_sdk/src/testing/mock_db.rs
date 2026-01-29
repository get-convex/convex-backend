//! Mock database implementation for testing
//!
//! This module provides a mock implementation of the Convex database
//! that can be used in unit tests without requiring a live backend.

use alloc::collections::BTreeMap;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use serde::Serialize;
use serde_json::Value;

use crate::types::{ConvexError, ConvexValue, Document, DocumentId, Result};
use crate::db::PaginatedResult;

/// Convert serde_json::Value to ConvexValue
fn json_to_convex(value: Value) -> ConvexValue {
    match value {
        Value::Null => ConvexValue::Null,
        Value::Bool(b) => ConvexValue::Boolean(b),
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                ConvexValue::Int64(i)
            } else if let Some(f) = n.as_f64() {
                ConvexValue::Float64(f)
            } else {
                ConvexValue::Null
            }
        }
        Value::String(s) => ConvexValue::String(s),
        Value::Array(arr) => {
            ConvexValue::Array(arr.into_iter().map(json_to_convex).collect())
        }
        Value::Object(map) => {
            let btree: BTreeMap<String, ConvexValue> = map
                .into_iter()
                .map(|(k, v)| (k, json_to_convex(v)))
                .collect();
            ConvexValue::Object(btree)
        }
    }
}

/// A mock database for testing
///
/// This implements the same interface as the real Database but stores
/// data in memory for fast, deterministic testing.
#[derive(Debug, Default)]
pub struct MockDatabase {
    tables: BTreeMap<String, MockTable>,
    id_counter: u64,
}

impl MockDatabase {
    /// Create a new empty mock database
    pub fn new() -> Self {
        Self {
            tables: BTreeMap::new(),
            id_counter: 1,
        }
    }

    /// Add a document to a table
    ///
    /// # Arguments
    ///
    /// * `table_name` - The name of the table
    /// * `value` - The document value
    ///
    /// # Returns
    ///
    /// The ID of the created document
    pub fn add_document(
        &mut self,
        table_name: impl Into<String>,
        value: impl Serialize,
    ) -> DocumentId {
        let table_name = table_name.into();
        let table = self.tables.entry(table_name.clone()).or_default();

        let id = format!("{}:{}", table_name, self.id_counter);
        self.id_counter += 1;

        let json_value = serde_json::to_value(value).unwrap_or(Value::Null);
        let convex_value = json_to_convex(json_value);
        let doc = Document {
            id: DocumentId::new(&id),
            value: convex_value,
        };
        table.documents.insert(id.clone(), doc);

        DocumentId::new(&id)
    }

    /// Get a document by ID
    pub fn get_document(&self,
        id: &DocumentId,
    ) -> Option<&Document> {
        let id_str = id.as_str();
        let parts: Vec<&str> = id_str.splitn(2, ':').collect();
        if parts.len() != 2 {
            return None;
        }

        let table = self.tables.get(parts[0])?;
        table.documents.get(id_str)
    }

    /// Get all documents in a table
    pub fn get_table_documents(
        &self,
        table_name: &str,
    ) -> Vec<&Document> {
        self.tables
            .get(table_name)
            .map(|t| t.documents.values().collect())
            .unwrap_or_default()
    }

    /// Count documents in a table
    pub fn table_count(&self,
        table_name: &str,
    ) -> usize {
        self.tables
            .get(table_name)
            .map(|t| t.documents.len())
            .unwrap_or(0)
    }

    /// Delete a document
    pub fn delete_document(
        &mut self,
        id: &DocumentId,
    ) -> Result<()> {
        let id_str = id.as_str();
        let parts: Vec<&str> = id_str.splitn(2, ':').collect();
        if parts.len() != 2 {
            return Err(ConvexError::InvalidArgument(
                format!("Invalid document ID: {}", id_str)
            ));
        }

        let table = self.tables
            .get_mut(parts[0])
            .ok_or_else(|| ConvexError::InvalidArgument(
                format!("Table not found: {}", parts[0])
            ))?;

        table.documents
            .remove(id_str)
            .ok_or_else(|| ConvexError::InvalidArgument(
                format!("Document not found: {}", id_str)
            ))?;

        Ok(())
    }

    /// Patch a document
    pub fn patch_document(
        &mut self,
        id: &DocumentId,
        patch: impl Serialize,
    ) -> Result<()> {
        let id_str = id.as_str();
        let parts: Vec<&str> = id_str.splitn(2, ':').collect();
        if parts.len() != 2 {
            return Err(ConvexError::InvalidArgument(
                format!("Invalid document ID: {}", id_str)
            ));
        }

        let table = self.tables
            .get_mut(parts[0])
            .ok_or_else(|| ConvexError::InvalidArgument(
                format!("Table not found: {}", parts[0])
            ))?;

        let doc = table.documents
            .get_mut(id_str)
            .ok_or_else(|| ConvexError::InvalidArgument(
                format!("Document not found: {}", id_str)
            ))?;

        let patch_value = serde_json::to_value(patch)?;
        if let (ConvexValue::Object(existing), Value::Object(updates)) = (
            &mut doc.value,
            patch_value
        ) {
            for (key, value) in updates {
                existing.insert(key, json_to_convex(value));
            }
        }

        Ok(())
    }

    /// Convert this mock database into a real Database handle
    ///
    /// Note: In a real implementation, this would return a Database
    /// that uses mock host functions. For now, this is a placeholder.
    pub fn into_database(self) -> crate::Database {
        // This would set up mock host functions
        crate::Database::new(1)
    }

    /// Query documents in a table with simple filtering
    pub fn query(
        &self,
        table_name: &str,
    ) -> MockQuery {
        MockQuery {
            db: self,
            table_name: table_name.to_string(),
            filters: Vec::new(),
            limit: None,
            skip: None,
        }
    }
}

/// A mock table
#[derive(Debug, Default)]
pub struct MockTable {
    pub(crate) documents: BTreeMap<String, Document>,
}

/// Result of a mock query
#[derive(Debug)]
pub struct MockQueryResult {
    /// Matching documents
    pub documents: Vec<Document>,
    /// Whether there are more results
    pub has_more: bool,
    /// Cursor for next page (if applicable)
    pub next_cursor: Option<String>,
}

impl MockQueryResult {
    /// Convert to a PaginatedResult
    pub fn into_paginated(self) -> PaginatedResult {
        PaginatedResult {
            documents: self.documents,
            next_cursor: self.next_cursor,
            has_more: self.has_more,
            page_status: None,
            split_cursor: None,
            rows_read: None,
            continuation_cursor: None,
        }
    }
}

/// A mock query builder
pub struct MockQuery<'a> {
    db: &'a MockDatabase,
    table_name: String,
    filters: Vec<MockFilter>,
    limit: Option<usize>,
    skip: Option<usize>,
}

#[derive(Debug)]
struct MockFilter {
    field: String,
    op: String,
    value: Value,
}

impl<'a> MockQuery<'a> {
    /// Add a filter condition
    pub fn filter(
        mut self,
        field: &str,
        op: &str,
        value: impl Serialize,
    ) -> Result<Self> {
        self.filters.push(MockFilter {
            field: field.to_string(),
            op: op.to_string(),
            value: serde_json::to_value(value)?,
        });
        Ok(self)
    }

    /// Set the limit
    pub fn limit(mut self, n: usize) -> Self {
        self.limit = Some(n);
        self
    }

    /// Set the skip
    pub fn skip(mut self, n: usize) -> Self {
        self.skip = Some(n);
        self
    }

    /// Execute the query and return results
    pub fn collect(self) -> Result<Vec<Document>> {
        let table = self.db.tables.get(&self.table_name);
        let mut docs: Vec<Document> = match table {
            Some(t) => t.documents.values().cloned().collect(),
            None => Vec::new(),
        };

        // Apply filters
        for filter in &self.filters {
            docs.retain(|doc| self.matches_filter(doc, filter));
        }

        // Apply skip
        if let Some(skip) = self.skip {
            docs = docs.into_iter().skip(skip).collect();
        }

        // Apply limit
        if let Some(limit) = self.limit {
            docs.truncate(limit);
        }

        Ok(docs)
    }

    /// Execute and return paginated results
    pub fn paginate(self) -> Result<PaginatedResult> {
        let limit = self.limit.unwrap_or(100);
        let docs = self.collect()?;
        let has_more = docs.len() > limit;

        Ok(PaginatedResult {
            documents: docs.into_iter().take(limit).collect(),
            next_cursor: if has_more { Some("next".to_string()) } else { None },
            has_more,
            page_status: None,
            split_cursor: None,
            rows_read: None,
            continuation_cursor: None,
        })
    }

    fn matches_filter(&self,
        doc: &Document,
        filter: &MockFilter,
    ) -> bool {
        let doc_value = get_field_from_convex(&doc.value, &filter.field);

        match filter.op.as_str() {
            "eq" => convex_matches_json(doc_value, &filter.value),
            "neq" => !convex_matches_json(doc_value, &filter.value),
            "gt" => convex_gt_json(doc_value, &filter.value),
            "gte" => convex_gte_json(doc_value, &filter.value),
            "lt" => convex_lt_json(doc_value, &filter.value),
            "lte" => convex_lte_json(doc_value, &filter.value),
            _ => true, // Unknown operator matches all
        }
    }
}

/// Get a field from a ConvexValue object
fn get_field_from_convex<'a>(value: &'a ConvexValue, field: &str) -> Option<&'a ConvexValue> {
    match value {
        ConvexValue::Object(map) => map.get(field),
        _ => None,
    }
}

/// Compare a ConvexValue with a JSON value for equality
fn convex_matches_json(convex: Option<&ConvexValue>, json: &Value) -> bool {
    match (convex, json) {
        (None, Value::Null) => true,
        (Some(ConvexValue::Null), Value::Null) => true,
        (Some(ConvexValue::Boolean(b)), Value::Bool(jb)) => b == jb,
        (Some(ConvexValue::Int64(i)), Value::Number(n)) => n.as_i64() == Some(*i),
        (Some(ConvexValue::Float64(f)), Value::Number(n)) => n.as_f64() == Some(*f),
        (Some(ConvexValue::String(s)), Value::String(js)) => s == js,
        _ => false,
    }
}

fn convex_gt_json(convex: Option<&ConvexValue>, json: &Value) -> bool {
    match (convex, json) {
        (Some(ConvexValue::Int64(i)), Value::Number(n)) => {
            n.as_f64().map_or(false, |f| *i as f64 > f)
        }
        (Some(ConvexValue::Float64(f)), Value::Number(n)) => {
            n.as_f64().map_or(false, |nf| *f > nf)
        }
        (Some(ConvexValue::String(s)), Value::String(js)) => s > js,
        _ => false,
    }
}

fn convex_gte_json(convex: Option<&ConvexValue>, json: &Value) -> bool {
    match (convex, json) {
        (Some(ConvexValue::Int64(i)), Value::Number(n)) => {
            n.as_f64().map_or(false, |f| *i as f64 >= f)
        }
        (Some(ConvexValue::Float64(f)), Value::Number(n)) => {
            n.as_f64().map_or(false, |nf| *f >= nf)
        }
        (Some(ConvexValue::String(s)), Value::String(js)) => s >= js,
        _ => false,
    }
}

fn convex_lt_json(convex: Option<&ConvexValue>, json: &Value) -> bool {
    match (convex, json) {
        (Some(ConvexValue::Int64(i)), Value::Number(n)) => {
            n.as_f64().map_or(false, |f| (*i as f64) < f)
        }
        (Some(ConvexValue::Float64(f)), Value::Number(n)) => {
            n.as_f64().map_or(false, |nf| *f < nf)
        }
        (Some(ConvexValue::String(s)), Value::String(js)) => s < js,
        _ => false,
    }
}

fn convex_lte_json(convex: Option<&ConvexValue>, json: &Value) -> bool {
    match (convex, json) {
        (Some(ConvexValue::Int64(i)), Value::Number(n)) => {
            n.as_f64().map_or(false, |f| (*i as f64) <= f)
        }
        (Some(ConvexValue::Float64(f)), Value::Number(n)) => {
            n.as_f64().map_or(false, |nf| *f <= nf)
        }
        (Some(ConvexValue::String(s)), Value::String(js)) => s <= js,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_db() -> MockDatabase {
        let mut db = MockDatabase::new();
        db.add_document("users", serde_json::json!({
            "name": "Alice",
            "age": 30
        }));
        db.add_document("users", serde_json::json!({
            "name": "Bob",
            "age": 25
        }));
        db
    }

    #[test]
    fn test_add_document() {
        let mut db = MockDatabase::new();
        let id = db.add_document("users", serde_json::json!({"name": "Alice"}));
        assert!(id.as_str().starts_with("users:"));
    }

    #[test]
    fn test_get_document() {
        let mut db = MockDatabase::new();
        let id = db.add_document("users", serde_json::json!({"name": "Alice"}));

        let doc = db.get_document(&id).unwrap();
        let name_value = get_field_from_convex(&doc.value, "name");
        assert!(matches!(name_value, Some(ConvexValue::String(s)) if s == "Alice"));
    }

    #[test]
    fn test_delete_document() {
        let mut db = MockDatabase::new();
        let id = db.add_document("users", serde_json::json!({"name": "Alice"}));

        assert!(db.delete_document(&id).is_ok());
        assert!(db.get_document(&id).is_none());
    }

    #[test]
    fn test_patch_document() {
        let mut db = MockDatabase::new();
        let id = db.add_document("users", serde_json::json!({
            "name": "Alice",
            "age": 30
        }));

        db.patch_document(&id, serde_json::json!({"age": 31})).unwrap();

        let doc = db.get_document(&id).unwrap();
        let age_value = get_field_from_convex(&doc.value, "age");
        assert!(matches!(age_value, Some(ConvexValue::Int64(31)) | Some(ConvexValue::Float64(31.0))));
        let name_value = get_field_from_convex(&doc.value, "name");
        assert!(matches!(name_value, Some(ConvexValue::String(s)) if s == "Alice"));
    }

    #[test]
    fn test_query_filter() {
        let db = create_test_db();

        let results = db.query("users")
            .filter("name", "eq", "Alice").unwrap()
            .collect()
            .unwrap();

        assert_eq!(results.len(), 1);
        let name_value = get_field_from_convex(&results[0].value, "name");
        assert!(matches!(name_value, Some(ConvexValue::String(s)) if s == "Alice"));
    }

    #[test]
    fn test_query_limit() {
        let db = create_test_db();

        let results = db.query("users")
            .limit(1)
            .collect()
            .unwrap();

        assert_eq!(results.len(), 1);
    }
}

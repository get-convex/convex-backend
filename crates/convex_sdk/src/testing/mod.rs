//! Testing utilities for Convex Rust SDK
//!
//! This module provides mocking capabilities and test helpers for testing
//! Convex functions without a live backend.
//!
//! # Example
//!
//! ```ignore
//! use convex_sdk::testing::{MockDatabase, mock_env};
//! use convex_sdk::{query, Database, Document};
//!
//! #[query]
//! async fn get_active_users(db: Database) -> Result<Vec<Document>, convex_sdk::ConvexError> {
//!     db.table("users")
//!         .query()
//!         .filter("status", "eq", "active")?
//!         .collect()
//!         .await
//! }
//!
//! #[tokio::test]
//! async fn test_get_active_users() {
//!     let mut mock = MockDatabase::new();
//!     mock.add_document("users", json!({
//!         "name": "Alice",
//!         "status": "active"
//!     }));
//!     mock.add_document("users", json!({
//!         "name": "Bob",
//!         "status": "inactive"
//!     }));
//!
//!     let db = mock.into_database();
//!     let users = get_active_users(db).await.unwrap();
//!     assert_eq!(users.len(), 1);
//!     assert_eq!(users[0].get("name").unwrap(), "Alice");
//! }
//! ```

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use serde_json::Value;

pub mod assertions;
pub mod mock_db;

pub use mock_db::{MockDatabase, MockTable, MockQueryResult};
pub use assertions::*;

/// Convert serde_json::Value to ConvexValue
pub fn json_to_convex(value: Value) -> crate::ConvexValue {
    use crate::ConvexValue;

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

/// Get a field from a ConvexValue object
fn convex_get<'a>(value: &'a crate::ConvexValue, field: &str) -> Option<&'a crate::ConvexValue> {
    match value {
        crate::ConvexValue::Object(map) => map.get(field),
        _ => None,
    }
}

/// Set up a mock testing environment
///
/// This function initializes the mock host functions for testing.
/// It should be called at the beginning of each test.
///
/// # Example
///
/// ```ignore
/// #[tokio::test]
/// async fn my_test() {
///     let _guard = convex_sdk::testing::mock_env();
///     // Test code here
/// }
/// ```
pub fn mock_env() -> MockEnvGuard {
    MockEnvGuard::new()
}

/// Guard that maintains the mock environment
pub struct MockEnvGuard;

impl MockEnvGuard {
    fn new() -> Self {
        // Initialize mock host functions
        Self
    }
}

impl Drop for MockEnvGuard {
    fn drop(&mut self) {
        // Clean up mock environment
    }
}

/// Create a mock document for testing
///
/// # Arguments
///
/// * `table` - The table name
/// * `id` - The document ID (without table prefix)
/// * `data` - The document data
///
/// # Example
///
/// ```ignore
/// let doc = convex_sdk::testing::mock_document(
///     "users",
///     "abc123",
///     json!({"name": "Alice", "email": "alice@example.com"})
/// );
/// ```
pub fn mock_document(table: &str, id: &str, data: Value) -> crate::Document {
    use crate::{Document, DocumentId};

    let full_id = format!("{}:{}", table, id);
    Document {
        id: DocumentId::new(&full_id),
        value: json_to_convex(data),
    }
}

/// Test helper to create a batch of mock documents
///
/// # Example
///
/// ```ignore
/// let docs = convex_sdk::testing::mock_documents("users", vec![
///     ("id1", json!({"name": "Alice"})),
///     ("id2", json!({"name": "Bob"})),
/// ]);
/// ```
pub fn mock_documents(table: &str, docs: Vec<(&str, Value)>) -> Vec<crate::Document> {
    docs.into_iter()
        .map(|(id, data)| mock_document(table, id, data))
        .collect()
}

/// Builder for test scenarios
///
/// This provides a fluent API for setting up complex test scenarios
/// with multiple tables and documents.
///
/// # Example
///
/// ```ignore
/// let scenario = TestScenario::new()
///     .with_table("users", vec![
///         json!({"name": "Alice", "role": "admin"}),
///         json!({"name": "Bob", "role": "user"}),
///     ])
///     .with_table("posts", vec![
///         json!({"title": "Hello World", "author": "Alice"}),
///     ])
///     .build();
/// ```
pub struct TestScenario {
    tables: BTreeMap<String, Vec<Value>>,
}

impl TestScenario {
    /// Create a new empty test scenario
    pub fn new() -> Self {
        Self {
            tables: BTreeMap::new(),
        }
    }

    /// Add a table with documents to the scenario
    pub fn with_table(
        mut self,
        name: impl Into<String>,
        documents: Vec<Value>,
    ) -> Self {
        self.tables.insert(name.into(), documents);
        self
    }

    /// Add a single document to a table
    pub fn with_document(
        mut self,
        table: impl Into<String>,
        document: Value,
    ) -> Self {
        self.tables
            .entry(table.into())
            .or_default()
            .push(document);
        self
    }

    /// Build the mock database from this scenario
    pub fn build(self) -> MockDatabase {
        let mut db = MockDatabase::new();
        for (table_name, documents) in self.tables {
            for doc in documents {
                db.add_document(&table_name, doc);
            }
        }
        db
    }
}

impl Default for TestScenario {
    fn default() -> Self {
        Self::new()
    }
}

/// Macro for asserting that a document matches expected values
///
/// # Example
///
/// ```ignore
/// let doc = db.get(id).await?.unwrap();
/// assert_doc!(doc, {
///     "name": "Alice",
///     "email": "alice@example.com",
/// });
/// ```
#[macro_export]
macro_rules! assert_doc {
    ($doc:expr, { $($key:expr => $value:expr),* $(,)? }) => {
        $(
            assert_eq!(
                $doc.get($key),
                Some(&$value.into()),
                "Field '{}' mismatch",
                $key
            );
        )*
    };
}

/// Macro for asserting pagination results
///
/// # Example
///
/// ```ignore
/// let result = db.query("users").paginate().await?;
/// assert_paginated!(result, has_more = true, len = 10);
/// ```
#[macro_export]
macro_rules! assert_paginated {
    ($result:expr, has_more = $has_more:expr, len = $len:expr) => {
        assert_eq!($result.has_more, $has_more, "has_more mismatch");
        assert_eq!($result.documents.len(), $len, "document count mismatch");
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_mock_document() {
        let doc = mock_document("users", "abc123", json!({"name": "Alice"}));
        assert_eq!(doc.id.as_str(), "users:abc123");
        let name_value = convex_get(&doc.value, "name");
        assert!(matches!(name_value, Some(crate::ConvexValue::String(s)) if s == "Alice"));
    }

    #[test]
    fn test_mock_documents() {
        let docs = mock_documents("users", vec![
            ("id1", json!({"name": "Alice"})),
            ("id2", json!({"name": "Bob"})),
        ]);
        assert_eq!(docs.len(), 2);
        assert_eq!(docs[0].id.as_str(), "users:id1");
        assert_eq!(docs[1].id.as_str(), "users:id2");
    }

    #[test]
    fn test_test_scenario() {
        let scenario = TestScenario::new()
            .with_table("users", vec![
                json!({"name": "Alice"}),
                json!({"name": "Bob"}),
            ])
            .with_document("posts", json!({"title": "Hello"}));

        let db = scenario.build();
        assert_eq!(db.table_count("users"), 2);
        assert_eq!(db.table_count("posts"), 1);
    }
}

//! Assertion helpers for testing Convex functions
//!
//! This module provides convenient assertion macros and functions
//! for testing Convex-specific types.

use alloc::collections::BTreeMap;
use alloc::string::String;

use crate::types::{Document, ConvexValue};
use crate::db::PaginatedResult;

/// Assert that a document has a specific field value
///
/// # Example
///
/// ```ignore
/// let doc = db.get(id).await?.unwrap();
/// assert_field!(doc, "name", "Alice");
/// assert_field!(doc, "age", 30);
/// ```
#[macro_export]
macro_rules! assert_field {
    ($doc:expr, $field:expr, $expected:expr) => {
        {
            let field_value = match &$doc.value {
                $crate::ConvexValue::Object(map) => map.get($field),
                _ => None,
            };
            // Convert expected JSON value to ConvexValue for comparison
            let expected_json = serde_json::json!($expected);
            let expected = $crate::testing::json_to_convex(expected_json);
            assert_eq!(
                field_value,
                Some(&expected),
                "Field '{}' expected {:?}, got {:?}",
                $field,
                $expected,
                field_value
            );
        }
    };
}

/// Assert that a paginated result has specific properties
///
/// # Example
///
/// ```ignore
/// let result = db.query("users").paginate().await?;
/// assert_pagination!(result, has_more = true, count = 10);
/// ```
#[macro_export]
macro_rules! assert_pagination {
    ($result:expr, has_more = $has_more:expr, count = $count:expr) => {
        assert_eq!(
            $result.has_more, $has_more,
            "Expected has_more={}, got {}",
            $has_more, $result.has_more
        );
        assert_eq!(
            $result.documents.len(), $count,
            "Expected {} documents, got {}",
            $count, $result.documents.len()
        );
    };
}

/// Assert that a result is a specific Convex error type
///
/// # Example
///
/// ```ignore
/// let result = db.get(invalid_id).await;
/// assert_convex_error!(result, InvalidArgument);
/// ```
#[macro_export]
macro_rules! assert_convex_error {
    ($result:expr, $error_type:ident) => {
        match $result {
            Err($crate::ConvexError::$error_type(_)) => {},
            Err(other) => panic!(
                "Expected ConvexError::{}, got {:?}",
                stringify!($error_type),
                other
            ),
            Ok(_) => panic!(
                "Expected ConvexError::{}, got Ok",
                stringify!($error_type)
            ),
        }
    };
}

/// Assert that a ConvexValue equals an expected JSON value
///
/// # Example
///
/// ```ignore
/// let value = doc.value();
/// assert_convex_value!(value, {"name": "Alice", "count": 42});
/// ```
#[macro_export]
macro_rules! assert_convex_value {
    ($value:expr, $expected:tt) => {
        let expected = serde_json::json!($expected);
        assert_eq!(
            $value.to_json(),
            expected,
            "ConvexValue mismatch"
        );
    };
}

/// Convert serde_json::Value to ConvexValue
fn json_to_convex(value: serde_json::Value) -> ConvexValue {
    use alloc::collections::BTreeMap;

    match value {
        serde_json::Value::Null => ConvexValue::Null,
        serde_json::Value::Bool(b) => ConvexValue::Boolean(b),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                ConvexValue::Int64(i)
            } else if let Some(f) = n.as_f64() {
                ConvexValue::Float64(f)
            } else {
                ConvexValue::Null
            }
        }
        serde_json::Value::String(s) => ConvexValue::String(s),
        serde_json::Value::Array(arr) => {
            ConvexValue::Array(arr.into_iter().map(json_to_convex).collect())
        }
        serde_json::Value::Object(map) => {
            let btree: BTreeMap<String, ConvexValue> = map
                .into_iter()
                .map(|(k, v)| (k, json_to_convex(v)))
                .collect();
            ConvexValue::Object(btree)
        }
    }
}

/// Get a field from a ConvexValue object
fn get_field<'a>(value: &'a ConvexValue, field: &str) -> Option<&'a ConvexValue> {
    match value {
        ConvexValue::Object(map) => map.get(field),
        _ => None,
    }
}

/// Functions for assertions
pub mod funcs {
    use super::*;

    /// Assert that a document has all expected fields
    ///
    /// # Example
    ///
    /// ```ignore
    /// assert_document_has_fields(&doc, vec![
    ///     ("name", json!("Alice")),
    ///     ("email", json!("alice@example.com")),
    /// ]);
    /// ```
    pub fn assert_document_has_fields(
        doc: &Document,
        expected: Vec<(&str, serde_json::Value)>,
    ) {
        for (field, value) in expected {
            let actual = get_field(&doc.value, field);
            let expected_value = json_to_convex(value);
            assert_eq!(
                actual, Some(&expected_value),
                "Field '{}' mismatch: expected {:?}, got {:?}",
                field, expected_value, actual
            );
        }
    }

    /// Assert that a paginated result is empty
    pub fn assert_pagination_empty(result: &PaginatedResult) {
        assert!(
            result.documents.is_empty(),
            "Expected empty result, got {} documents",
            result.documents.len()
        );
        assert!(
            !result.has_more,
            "Expected has_more=false for empty result"
        );
    }

    /// Assert that a paginated result has exactly N documents
    pub fn assert_pagination_count(result: &PaginatedResult, count: usize) {
        assert_eq!(
            result.documents.len(), count,
            "Expected {} documents, got {}",
            count, result.documents.len()
        );
    }

    /// Assert paginated result has more pages
    pub fn assert_has_more(result: &PaginatedResult) {
        assert!(
            result.has_more,
            "Expected has_more=true, next_cursor={:?}",
            result.next_cursor
        );
    }

    /// Assert paginated result is the last page
    pub fn assert_is_last_page(result: &PaginatedResult) {
        assert!(
            !result.has_more,
            "Expected has_more=false (last page)"
        );
        assert!(
            result.next_cursor.is_none(),
            "Expected no next_cursor on last page"
        );
    }

    /// Assert that two ConvexValue objects are equal
    pub fn assert_convex_value_eq(
        actual: &ConvexValue,
        expected: &ConvexValue,
    ) {
        assert_eq!(
            actual, expected,
            "ConvexValue mismatch:\n  actual: {:?}\n  expected: {:?}",
            actual, expected
        );
    }
}

pub use funcs::*;

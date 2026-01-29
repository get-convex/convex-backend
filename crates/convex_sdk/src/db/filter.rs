//! Expression-based filtering for Convex queries
//!
//! This module provides a `FilterBuilder` that enables complex query expressions
//! similar to the TypeScript SDK. It supports:
//! - Comparison operators: eq, neq, lt, lte, gt, gte
//! - Logical operators: and, or, not
//! - Arithmetic operators: add, sub, mul, div, mod, neg
//! - Field references
//!
//! # Example
//!
//! ```ignore
//! use convex_sdk::db::filter::FilterBuilder;
//!
//! let f = FilterBuilder::new();
//! let filter = f.and(
//!     f.eq("status", "active"),
//!     f.or(
//!         f.gt("createdAt", "2024-01-01"),
//!         f.eq("priority", "high")
//!     )
//! );
//! ```

use alloc::string::String;
use alloc::vec::Vec;
use serde::{Deserialize, Serialize};

/// A filter expression that can be serialized and sent to the Convex backend
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum FilterExpression {
    /// Equality comparison: field == value
    Eq { field: FieldRef, value: serde_json::Value },

    /// Not equal comparison: field != value
    Neq { field: FieldRef, value: serde_json::Value },

    /// Less than comparison: field < value
    Lt { field: FieldRef, value: serde_json::Value },

    /// Less than or equal comparison: field <= value
    Lte { field: FieldRef, value: serde_json::Value },

    /// Greater than comparison: field > value
    Gt { field: FieldRef, value: serde_json::Value },

    /// Greater than or equal comparison: field >= value
    Gte { field: FieldRef, value: serde_json::Value },

    /// Logical AND of multiple expressions
    And { expressions: Vec<FilterExpression> },

    /// Logical OR of multiple expressions
    Or { expressions: Vec<FilterExpression> },

    /// Logical NOT of an expression
    Not { expression: Box<FilterExpression> },

    /// Addition: left + right
    Add { left: Box<FilterExpression>, right: Box<FilterExpression> },

    /// Subtraction: left - right
    Sub { left: Box<FilterExpression>, right: Box<FilterExpression> },

    /// Multiplication: left * right
    Mul { left: Box<FilterExpression>, right: Box<FilterExpression> },

    /// Division: left / right
    Div { left: Box<FilterExpression>, right: Box<FilterExpression> },

    /// Modulo: left % right
    Mod { left: Box<FilterExpression>, right: Box<FilterExpression> },

    /// Negation: -value
    Neg { value: Box<FilterExpression> },

    /// Field reference
    Field { path: FieldRef },

    /// Literal value
    Literal { value: serde_json::Value },
}

/// A reference to a field path
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FieldRef {
    /// The path segments (e.g., ["user", "name"] for "user.name")
    pub segments: Vec<String>,
}

impl FieldRef {
    /// Create a field reference from a string path
    ///
    /// # Example
    ///
    /// ```ignore
    /// let field = FieldRef::from_path("user.name");
    /// assert_eq!(field.segments, vec!["user", "name"]);
    /// ```
    pub fn from_path(path: impl Into<String>) -> Self {
        let path_str = path.into();
        let segments = path_str
            .split('.')
            .map(|s| s.to_string())
            .collect();
        Self { segments }
    }

    /// Create a field reference from an array of segments
    pub fn from_segments(segments: Vec<String>) -> Self {
        Self { segments }
    }

    /// Convert to a string path
    pub fn to_path(&self) -> String {
        self.segments.join(".")
    }
}

/// Builder for creating filter expressions
///
/// This struct provides methods for building complex filter expressions
/// that match the TypeScript SDK's FilterBuilder API.
#[derive(Debug, Clone)]
pub struct FilterBuilder;

impl FilterBuilder {
    /// Create a new filter builder
    pub fn new() -> Self {
        Self
    }

    // ==================== Comparison Operators ====================

    /// Create an equality filter: field == value
    ///
    /// # Example
    ///
    /// ```ignore
    /// let f = FilterBuilder::new();
    /// let filter = f.eq("status", "active");
    /// ```
    pub fn eq(
        &self,
        field: impl Into<String>,
        value: impl Serialize,
    ) -> FilterExpression {
        FilterExpression::Eq {
            field: FieldRef::from_path(field),
            value: serde_json::to_value(value).unwrap_or(serde_json::Value::Null),
        }
    }

    /// Create a not-equal filter: field != value
    ///
    /// # Example
    ///
    /// ```ignore
    /// let f = FilterBuilder::new();
    /// let filter = f.neq("status", "deleted");
    /// ```
    pub fn neq(
        &self,
        field: impl Into<String>,
        value: impl Serialize,
    ) -> FilterExpression {
        FilterExpression::Neq {
            field: FieldRef::from_path(field),
            value: serde_json::to_value(value).unwrap_or(serde_json::Value::Null),
        }
    }

    /// Create a less-than filter: field < value
    pub fn lt(
        &self,
        field: impl Into<String>,
        value: impl Serialize,
    ) -> FilterExpression {
        FilterExpression::Lt {
            field: FieldRef::from_path(field),
            value: serde_json::to_value(value).unwrap_or(serde_json::Value::Null),
        }
    }

    /// Create a less-than-or-equal filter: field <= value
    pub fn lte(
        &self,
        field: impl Into<String>,
        value: impl Serialize,
    ) -> FilterExpression {
        FilterExpression::Lte {
            field: FieldRef::from_path(field),
            value: serde_json::to_value(value).unwrap_or(serde_json::Value::Null),
        }
    }

    /// Create a greater-than filter: field > value
    pub fn gt(
        &self,
        field: impl Into<String>,
        value: impl Serialize,
    ) -> FilterExpression {
        FilterExpression::Gt {
            field: FieldRef::from_path(field),
            value: serde_json::to_value(value).unwrap_or(serde_json::Value::Null),
        }
    }

    /// Create a greater-than-or-equal filter: field >= value
    pub fn gte(
        &self,
        field: impl Into<String>,
        value: impl Serialize,
    ) -> FilterExpression {
        FilterExpression::Gte {
            field: FieldRef::from_path(field),
            value: serde_json::to_value(value).unwrap_or(serde_json::Value::Null),
        }
    }

    // ==================== Logical Operators ====================

    /// Create a logical AND of multiple expressions
    ///
    /// # Example
    ///
    /// ```ignore
    /// let f = FilterBuilder::new();
    /// let filter = f.and(
    ///     f.eq("status", "active"),
    ///     f.gt("createdAt", "2024-01-01")
    /// );
    /// ```
    pub fn and(
        &self,
        first: FilterExpression,
        second: FilterExpression,
    ) -> FilterExpression {
        FilterExpression::And {
            expressions: vec![first, second],
        }
    }

    /// Create a logical AND of many expressions
    pub fn and_many(&self, expressions: Vec<FilterExpression>) -> FilterExpression {
        FilterExpression::And { expressions }
    }

    /// Create a logical OR of multiple expressions
    ///
    /// # Example
    ///
    /// ```ignore
    /// let f = FilterBuilder::new();
    /// let filter = f.or(
    ///     f.eq("priority", "high"),
    ///     f.eq("priority", "urgent")
    /// );
    /// ```
    pub fn or(
        &self,
        first: FilterExpression,
        second: FilterExpression,
    ) -> FilterExpression {
        FilterExpression::Or {
            expressions: vec![first, second],
        }
    }

    /// Create a logical OR of many expressions
    pub fn or_many(&self, expressions: Vec<FilterExpression>) -> FilterExpression {
        FilterExpression::Or { expressions }
    }

    /// Create a logical NOT of an expression
    ///
    /// # Example
    ///
    /// ```ignore
    /// let f = FilterBuilder::new();
    /// let filter = f.not(f.eq("status", "deleted"));
    /// ```
    pub fn not(&self, expression: FilterExpression) -> FilterExpression {
        FilterExpression::Not {
            expression: Box::new(expression),
        }
    }

    // ==================== Arithmetic Operators ====================

    /// Create an addition expression: left + right
    pub fn add(
        &self,
        left: FilterExpression,
        right: FilterExpression,
    ) -> FilterExpression {
        FilterExpression::Add {
            left: Box::new(left),
            right: Box::new(right),
        }
    }

    /// Create a subtraction expression: left - right
    pub fn sub(
        &self,
        left: FilterExpression,
        right: FilterExpression,
    ) -> FilterExpression {
        FilterExpression::Sub {
            left: Box::new(left),
            right: Box::new(right),
        }
    }

    /// Create a multiplication expression: left * right
    pub fn mul(
        &self,
        left: FilterExpression,
        right: FilterExpression,
    ) -> FilterExpression {
        FilterExpression::Mul {
            left: Box::new(left),
            right: Box::new(right),
        }
    }

    /// Create a division expression: left / right
    pub fn div(
        &self,
        left: FilterExpression,
        right: FilterExpression,
    ) -> FilterExpression {
        FilterExpression::Div {
            left: Box::new(left),
            right: Box::new(right),
        }
    }

    /// Create a modulo expression: left % right
    pub fn mod_expr(
        &self,
        left: FilterExpression,
        right: FilterExpression,
    ) -> FilterExpression {
        FilterExpression::Mod {
            left: Box::new(left),
            right: Box::new(right),
        }
    }

    /// Create a negation expression: -value
    pub fn neg(&self, value: FilterExpression) -> FilterExpression {
        FilterExpression::Neg {
            value: Box::new(value),
        }
    }

    // ==================== Field and Literal ====================

    /// Create a field reference expression
    ///
    /// # Example
    ///
    /// ```ignore
    /// let f = FilterBuilder::new();
    /// let filter = f.eq(f.field("status"), "active");
    /// ```
    pub fn field(&self, path: impl Into<String>) -> FilterExpression {
        FilterExpression::Field {
            path: FieldRef::from_path(path),
        }
    }

    /// Create a literal value expression
    pub fn literal(&self, value: impl Serialize) -> FilterExpression {
        FilterExpression::Literal {
            value: serde_json::to_value(value).unwrap_or(serde_json::Value::Null),
        }
    }
}

impl Default for FilterBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Extension trait for chaining filter operations
pub trait FilterExpressionExt {
    /// Combine with another expression using AND
    fn and(self, other: FilterExpression) -> FilterExpression;

    /// Combine with another expression using OR
    fn or(self, other: FilterExpression) -> FilterExpression;

    /// Negate this expression
    fn not(self) -> FilterExpression;
}

impl FilterExpressionExt for FilterExpression {
    fn and(self, other: FilterExpression) -> FilterExpression {
        FilterExpression::And {
            expressions: vec![self, other],
        }
    }

    fn or(self, other: FilterExpression) -> FilterExpression {
        FilterExpression::Or {
            expressions: vec![self, other],
        }
    }

    fn not(self) -> FilterExpression {
        FilterExpression::Not {
            expression: Box::new(self),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_field_ref_from_path() {
        let field = FieldRef::from_path("user.profile.name");
        assert_eq!(field.segments, vec!["user", "profile", "name"]);
    }

    #[test]
    fn test_field_ref_to_path() {
        let field = FieldRef::from_segments(vec!["user".to_string(), "name".to_string()]);
        assert_eq!(field.to_path(), "user.name");
    }

    #[test]
    fn test_eq_filter() {
        let f = FilterBuilder::new();
        let filter = f.eq("status", "active");

        match filter {
            FilterExpression::Eq { field, value } => {
                assert_eq!(field.to_path(), "status");
                assert_eq!(value, serde_json::json!("active"));
            }
            _ => panic!("Expected Eq filter"),
        }
    }

    #[test]
    fn test_neq_filter() {
        let f = FilterBuilder::new();
        let filter = f.neq("deleted", true);

        match filter {
            FilterExpression::Neq { field, value } => {
                assert_eq!(field.to_path(), "deleted");
                assert_eq!(value, serde_json::json!(true));
            }
            _ => panic!("Expected Neq filter"),
        }
    }

    #[test]
    fn test_and_filter() {
        let f = FilterBuilder::new();
        let filter = f.and(f.eq("status", "active"), f.gt("count", 5));

        match filter {
            FilterExpression::And { expressions } => {
                assert_eq!(expressions.len(), 2);
            }
            _ => panic!("Expected And filter"),
        }
    }

    #[test]
    fn test_or_filter() {
        let f = FilterBuilder::new();
        let filter = f.or(f.eq("priority", "high"), f.eq("priority", "urgent"));

        match filter {
            FilterExpression::Or { expressions } => {
                assert_eq!(expressions.len(), 2);
            }
            _ => panic!("Expected Or filter"),
        }
    }

    #[test]
    fn test_not_filter() {
        let f = FilterBuilder::new();
        let filter = f.not(f.eq("status", "deleted"));

        match filter {
            FilterExpression::Not { expression } => {
                match *expression {
                    FilterExpression::Eq { .. } => {}
                    _ => panic!("Expected Eq inside Not"),
                }
            }
            _ => panic!("Expected Not filter"),
        }
    }

    #[test]
    fn test_arithmetic_filters() {
        let f = FilterBuilder::new();

        // Test addition in filter context
        let add_expr = f.add(f.field("a"), f.literal(5));
        match add_expr {
            FilterExpression::Add { .. } => {}
            _ => panic!("Expected Add expression"),
        }

        // Test negation
        let neg_expr = f.neg(f.field("count"));
        match neg_expr {
            FilterExpression::Neg { .. } => {}
            _ => panic!("Expected Neg expression"),
        }
    }

    #[test]
    fn test_filter_chaining() {
        use FilterExpressionExt;

        let f = FilterBuilder::new();
        let filter = f.eq("status", "active").and(f.gt("createdAt", "2024-01-01"));

        match filter {
            FilterExpression::And { expressions } => {
                assert_eq!(expressions.len(), 2);
            }
            _ => panic!("Expected And filter"),
        }
    }

    #[test]
    fn test_serialization() {
        let f = FilterBuilder::new();
        let filter = f.and(
            f.eq("status", "active"),
            f.or(f.gt("count", 10), f.eq("priority", "high")),
        );

        let json = serde_json::to_string(&filter).unwrap();
        assert!(json.contains("and"));
        assert!(json.contains("eq"));
        assert!(json.contains("gt"));
    }

    #[test]
    fn test_complex_filter() {
        let f = FilterBuilder::new();

        // Complex filter: (status == "active" AND (count > 10 OR priority == "high")) AND NOT deleted
        let filter = f.and(
            f.and(
                f.eq("status", "active"),
                f.or(f.gt("count", 10), f.eq("priority", "high")),
            ),
            f.not(f.eq("deleted", true)),
        );

        // Serialize and verify it produces valid JSON
        let json = serde_json::to_string(&filter).unwrap();
        let deserialized: FilterExpression = serde_json::from_str(&json).unwrap();

        match deserialized {
            FilterExpression::And { expressions } => {
                assert_eq!(expressions.len(), 2);
            }
            _ => panic!("Expected And filter"),
        }
    }
}

//! Validators for Convex schema definitions
//!
//! This module provides validators for defining document schemas,
//! equivalent to the `v` object in the TypeScript SDK.
//!
//! # Example
//!
//! ```ignore
//! use convex_sdk::schema::v;
//!
//! let validator = v.object({
//!     name: v.string(),
//!     age: v.optional(v.number()),
//!     tags: v.array(v.string()),
//!     metadata: v.record(v.any()),
//! });
//! ```

use alloc::string::String;
use alloc::vec::Vec;
use alloc::collections::BTreeMap;
use serde::{Deserialize, Serialize};

/// A validator for Convex values
///
/// Validators define the expected structure of documents and are used
/// for runtime validation and TypeScript type generation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum Validator {
    /// String validator
    String,
    /// Number (float64) validator
    Number,
    /// BigInt (int64) validator
    BigInt,
    /// Boolean validator
    Boolean,
    /// Bytes (binary data) validator
    Bytes,
    /// Null validator
    Null,
    /// Any value validator
    Any,
    /// ID validator for table references
    Id {
        /// The table name this ID references
        table_name: String,
    },
    /// Optional wrapper validator
    Optional {
        /// The inner validator
        inner: Box<Validator>,
    },
    /// Array validator
    Array {
        /// The element validator
        elements: Box<Validator>,
    },
    /// Object validator
    Object {
        /// The field validators
        fields: BTreeMap<String, Validator>,
    },
    /// Union validator (one of several options)
    Union {
        /// The possible validators
        variants: Vec<Validator>,
    },
    /// Literal value validator
    Literal {
        /// The literal value
        value: serde_json::Value,
    },
    /// Record validator (string keys, uniform values)
    Record {
        /// The value validator
        values: Box<Validator>,
    },
}

impl Validator {
    /// Validate a value against this validator
    ///
    /// Returns true if the value is valid, false otherwise.
    pub fn validate(&self, value: &serde_json::Value) -> bool {
        match self {
            Validator::String => value.is_string(),
            Validator::Number => value.is_number(),
            Validator::BigInt => value.is_i64() || value.is_u64(),
            Validator::Boolean => value.is_boolean(),
            Validator::Bytes => value.is_array(), // Bytes are represented as array of numbers
            Validator::Null => value.is_null(),
            Validator::Any => true,
            Validator::Id { .. } => {
                // IDs are strings with a specific format
                value.as_str().map_or(false, |s| s.contains(':'))
            }
            Validator::Optional { inner } => {
                value.is_null() || inner.validate(value)
            }
            Validator::Array { elements } => {
                value.as_array()
                    .map_or(false, |arr| {
                        arr.iter().all(|item| elements.validate(item))
                    })
            }
            Validator::Object { fields } => {
                value.as_object()
                    .map_or(false, |obj| {
                        fields.iter().all(|(key, validator)| {
                            obj.get(key)
                                .map_or(false, |val| validator.validate(val))
                        })
                    })
            }
            Validator::Union { variants } => {
                variants.iter().any(|variant| variant.validate(value))
            }
            Validator::Literal { value: expected } => {
                value == expected
            }
            Validator::Record { values } => {
                value.as_object()
                    .map_or(false, |obj| {
                        obj.values().all(|val| values.validate(val))
                    })
            }
        }
    }

    /// Check if this validator allows null values
    pub fn is_optional(&self) -> bool {
        matches!(self, Validator::Optional { .. })
    }

    /// Wrap this validator in an Optional
    pub fn optional(self) -> Self {
        Validator::Optional {
            inner: Box::new(self),
        }
    }

    /// Create an array validator with this validator as elements
    pub fn array(self) -> Self {
        Validator::Array {
            elements: Box::new(self),
        }
    }

    /// Convert to JSON representation for schema export
    pub fn to_json(&self) -> serde_json::Value {
        serde_json::to_value(self).unwrap_or(serde_json::Value::Null)
    }
}

/// The `v` namespace for validators, matching TypeScript API
pub struct V;

impl V {
    /// Create a string validator
    ///
    /// # Example
    ///
    /// ```ignore
    /// use convex_sdk::schema::v;
    ///
    /// let validator = v.string();
    /// assert!(validator.validate(&json!("hello")));
    /// ```
    pub fn string(&self) -> Validator {
        Validator::String
    }

    /// Create a number (float64) validator
    ///
    /// # Example
    ///
    /// ```ignore
    /// use convex_sdk::schema::v;
    ///
    /// let validator = v.number();
    /// assert!(validator.validate(&json!(42.0)));
    /// ```
    pub fn number(&self) -> Validator {
        Validator::Number
    }

    /// Create a float64 validator (alias for number)
    pub fn float64(&self) -> Validator {
        Validator::Number
    }

    /// Create a bigint (int64) validator
    ///
    /// # Example
    ///
    /// ```ignore
    /// use convex_sdk::schema::v;
    ///
    /// let validator = v.int64();
    /// assert!(validator.validate(&json!(9007199254740993i64)));
    /// ```
    pub fn int64(&self) -> Validator {
        Validator::BigInt
    }

    /// Create a boolean validator
    ///
    /// # Example
    ///
    /// ```ignore
    /// use convex_sdk::schema::v;
    ///
    /// let validator = v.boolean();
    /// assert!(validator.validate(&json!(true)));
    /// ```
    pub fn boolean(&self) -> Validator {
        Validator::Boolean
    }

    /// Create a bytes (binary data) validator
    ///
    /// # Example
    ///
    /// ```ignore
    /// use convex_sdk::schema::v;
    ///
    /// let validator = v.bytes();
    /// ```
    pub fn bytes(&self) -> Validator {
        Validator::Bytes
    }

    /// Create a null validator
    ///
    /// # Example
    ///
    /// ```ignore
    /// use convex_sdk::schema::v;
    ///
    /// let validator = v.null();
    /// assert!(validator.validate(&json!(null)));
    /// ```
    pub fn null(&self) -> Validator {
        Validator::Null
    }

    /// Create an "any" validator that accepts any value
    ///
    /// # Example
    ///
    /// ```ignore
    /// use convex_sdk::schema::v;
    ///
    /// let validator = v.any();
    /// assert!(validator.validate(&json!("anything")));
    /// assert!(validator.validate(&json!(42)));
    /// ```
    pub fn any(&self) -> Validator {
        Validator::Any
    }

    /// Create an ID validator for a specific table
    ///
    /// # Arguments
    ///
    /// * `table_name` - The name of the table this ID references
    ///
    /// # Example
    ///
    /// ```ignore
    /// use convex_sdk::schema::v;
    ///
    /// let validator = v.id("users");
    /// ```
    pub fn id(&self, table_name: impl Into<String>) -> Validator {
        Validator::Id {
            table_name: table_name.into(),
        }
    }

    /// Create an optional validator
    ///
    /// # Arguments
    ///
    /// * `inner` - The validator to wrap
    ///
    /// # Example
    ///
    /// ```ignore
    /// use convex_sdk::schema::v;
    ///
    /// let validator = v.optional(v.string());
    /// assert!(validator.validate(&json!("hello")));
    /// assert!(validator.validate(&json!(null)));
    /// ```
    pub fn optional(&self, inner: Validator) -> Validator {
        Validator::Optional {
            inner: Box::new(inner),
        }
    }

    /// Create an array validator
    ///
    /// # Arguments
    ///
    /// * `elements` - The validator for array elements
    ///
    /// # Example
    ///
    /// ```ignore
    /// use convex_sdk::schema::v;
    ///
    /// let validator = v.array(v.string());
    /// assert!(validator.validate(&json!(["a", "b", "c"])));
    /// ```
    pub fn array(&self, elements: Validator) -> Validator {
        Validator::Array {
            elements: Box::new(elements),
        }
    }

    /// Create an object validator
    ///
    /// # Arguments
    ///
    /// * `fields` - A map of field names to validators
    ///
    /// # Example
    ///
    /// ```ignore
    /// use convex_sdk::schema::v;
    /// use std::collections::BTreeMap;
    ///
    /// let mut fields = BTreeMap::new();
    /// fields.insert("name".to_string(), v.string());
    /// fields.insert("age".to_string(), v.number());
    ///
    /// let validator = v.object(fields);
    /// ```
    pub fn object(&self, fields: BTreeMap<String, Validator>) -> Validator {
        Validator::Object { fields }
    }

    /// Create a union validator (one of several options)
    ///
    /// # Arguments
    ///
    /// * `variants` - The possible validators
    ///
    /// # Example
    ///
    /// ```ignore
    /// use convex_sdk::schema::v;
    ///
    /// let validator = v.union(vec![
    ///     v.string(),
    ///     v.number(),
    /// ]);
    /// assert!(validator.validate(&json!("hello")));
    /// assert!(validator.validate(&json!(42)));
    /// ```
    pub fn union(&self, variants: Vec<Validator>) -> Validator {
        Validator::Union { variants }
    }

    /// Create a literal string validator
    ///
    /// # Arguments
    ///
    /// * `value` - The literal string value
    pub fn literal(&self, value: impl Into<LiteralValue>) -> Validator {
        Validator::Literal {
            value: value.into().into(),
        }
    }

    /// Create a record validator (string keys, uniform values)
    ///
    /// # Arguments
    ///
    /// * `values` - The validator for record values
    ///
    /// # Example
    ///
    /// ```ignore
    /// use convex_sdk::schema::v;
    ///
    /// let validator = v.record(v.string());
    /// assert!(validator.validate(&json!({"key1": "value1", "key2": "value2"})));
    /// ```
    pub fn record(&self, values: Validator) -> Validator {
        Validator::Record {
            values: Box::new(values),
        }
    }
}

/// Singleton instance of the V namespace
pub const v: V = V;

/// Helper type for creating literal values
pub enum LiteralValue {
    String(String),
    Number(f64),
    Bool(bool),
    Null,
}

impl From<LiteralValue> for serde_json::Value {
    fn from(val: LiteralValue) -> Self {
        match val {
            LiteralValue::String(s) => serde_json::Value::String(s),
            LiteralValue::Number(n) => serde_json::Value::Number(
                serde_json::Number::from_f64(n).unwrap_or_else(|| serde_json::Number::from(0))
            ),
            LiteralValue::Bool(b) => serde_json::Value::Bool(b),
            LiteralValue::Null => serde_json::Value::Null,
        }
    }
}

impl From<&str> for LiteralValue {
    fn from(s: &str) -> Self {
        LiteralValue::String(s.to_string())
    }
}

impl From<String> for LiteralValue {
    fn from(s: String) -> Self {
        LiteralValue::String(s)
    }
}

impl From<f64> for LiteralValue {
    fn from(n: f64) -> Self {
        LiteralValue::Number(n)
    }
}

impl From<i64> for LiteralValue {
    fn from(n: i64) -> Self {
        LiteralValue::Number(n as f64)
    }
}

impl From<bool> for LiteralValue {
    fn from(b: bool) -> Self {
        LiteralValue::Bool(b)
    }
}

// Convenience re-exports for common validators
pub type VString = Validator;
pub type VNumber = Validator;
pub type VBoolean = Validator;
pub type VId = Validator;
pub type VOptional = Validator;
pub type VArray = Validator;
pub type VObject = Validator;

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_string_validator() {
        let validator = v.string();
        assert!(validator.validate(&json!("hello")));
        assert!(!validator.validate(&json!(42)));
        assert!(!validator.validate(&json!(null)));
    }

    #[test]
    fn test_number_validator() {
        let validator = v.number();
        assert!(validator.validate(&json!(42)));
        assert!(validator.validate(&json!(3.14)));
        assert!(!validator.validate(&json!("hello")));
    }

    #[test]
    fn test_boolean_validator() {
        let validator = v.boolean();
        assert!(validator.validate(&json!(true)));
        assert!(validator.validate(&json!(false)));
        assert!(!validator.validate(&json!("true")));
    }

    #[test]
    fn test_optional_validator() {
        let validator = v.optional(v.string());
        assert!(validator.validate(&json!("hello")));
        assert!(validator.validate(&json!(null)));
        assert!(!validator.validate(&json!(42)));
    }

    #[test]
    fn test_array_validator() {
        let validator = v.array(v.string());
        assert!(validator.validate(&json!(["a", "b", "c"])));
        assert!(!validator.validate(&json!(["a", 1, "c"])));
        assert!(validator.validate(&json!([])));
    }

    #[test]
    fn test_object_validator() {
        let mut fields = BTreeMap::new();
        fields.insert("name".to_string(), v.string());
        fields.insert("age".to_string(), v.number());

        let validator = v.object(fields);
        assert!(validator.validate(&json!({"name": "Alice", "age": 30})));
        assert!(!validator.validate(&json!({"name": "Alice", "age": "thirty"})));
    }

    #[test]
    fn test_id_validator() {
        let validator = v.id("users");
        assert!(validator.validate(&json!("users:abc123")));
        assert!(!validator.validate(&json!("invalid")));
    }

    #[test]
    fn test_union_validator() {
        let validator = v.union(vec![v.string(), v.number()]);
        assert!(validator.validate(&json!("hello")));
        assert!(validator.validate(&json!(42)));
        assert!(!validator.validate(&json!(true)));
    }

    #[test]
    fn test_literal_validator() {
        let validator = v.literal("active");
        assert!(validator.validate(&json!("active")));
        assert!(!validator.validate(&json!("inactive")));
    }

    #[test]
    fn test_record_validator() {
        let validator = v.record(v.string());
        assert!(validator.validate(&json!({"key1": "value1", "key2": "value2"})));
        assert!(!validator.validate(&json!({"key1": "value1", "key2": 42})));
    }

    #[test]
    fn test_any_validator() {
        let validator = v.any();
        assert!(validator.validate(&json!("anything")));
        assert!(validator.validate(&json!(42)));
        assert!(validator.validate(&json!(null)));
        assert!(validator.validate(&json!({"nested": true})));
    }

    #[test]
    fn test_null_validator() {
        let validator = v.null();
        assert!(validator.validate(&json!(null)));
        assert!(!validator.validate(&json!("null")));
    }
}

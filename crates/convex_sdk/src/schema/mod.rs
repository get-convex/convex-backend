//! Schema definition and validation for Convex
//!
//! This module provides a Rust equivalent of the TypeScript schema system,
//! allowing you to define table schemas with validators.
//!
//! # Example
//!
//! ```ignore
//! use convex_sdk::schema::{define_schema, define_table, v};
//!
//! let schema = define_schema({
//!     users: define_table({
//!         name: v.string(),
//!         email: v.string(),
//!         age: v.optional(v.number()),
//!     })
//!     .index("by_email", ["email"]),
//!
//!     posts: define_table({
//!         title: v.string(),
//!         content: v.string(),
//!         author_id: v.id("users"),
//!         published: v.boolean(),
//!     })
//!     .index("by_author", ["author_id"])
//!     .search_index("search_content", { search_field: "content" }),
//! });
//! ```

use alloc::string::String;
use alloc::vec::Vec;
use alloc::collections::BTreeMap;
use serde::{Deserialize, Serialize};

pub mod validators;

pub use validators::{Validator, VString, VNumber, VBoolean, VId, VOptional, VArray, VObject, v};

/// A schema definition containing multiple tables
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaDefinition {
    /// Tables in the schema
    pub tables: BTreeMap<String, TableDefinition>,
    /// Whether to validate documents at runtime
    pub schema_validation: bool,
}

impl SchemaDefinition {
    /// Create a new schema definition
    pub fn new() -> Self {
        Self {
            tables: BTreeMap::new(),
            schema_validation: true,
        }
    }

    /// Add a table to the schema
    pub fn add_table(mut self, name: impl Into<String>, table: TableDefinition) -> Self {
        self.tables.insert(name.into(), table);
        self
    }

    /// Disable runtime schema validation
    pub fn without_validation(mut self) -> Self {
        self.schema_validation = false;
        self
    }

    /// Export the schema as JSON
    pub fn export(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }

    /// Get a table definition by name
    pub fn get_table(&self, name: &str) -> Option<&TableDefinition> {
        self.tables.get(name)
    }
}

impl Default for SchemaDefinition {
    fn default() -> Self {
        Self::new()
    }
}

/// A table definition with fields, indexes, and validators
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableDefinition {
    /// The validator for documents in this table
    pub document_type: Validator,
    /// Database indexes
    pub indexes: Vec<IndexDefinition>,
    /// Search indexes
    pub search_indexes: Vec<SearchIndexDefinition>,
    /// Vector indexes
    pub vector_indexes: Vec<VectorIndexDefinition>,
}

impl TableDefinition {
    /// Create a new table definition
    pub fn new(document_type: Validator) -> Self {
        Self {
            document_type,
            indexes: Vec::new(),
            search_indexes: Vec::new(),
            vector_indexes: Vec::new(),
        }
    }

    /// Add a database index
    ///
    /// # Arguments
    ///
    /// * `name` - The name of the index
    /// * `fields` - The fields to index, in order
    ///
    /// # Example
    ///
    /// ```ignore
    /// let table = define_table({
    ///     name: v.string(),
    ///     email: v.string(),
    /// })
    /// .index("by_email", ["email"]);
    /// ```
    pub fn index(mut self, name: impl Into<String>, fields: Vec<impl Into<String>>) -> Self {
        self.indexes.push(IndexDefinition {
            name: name.into(),
            fields: fields.into_iter().map(|f| f.into()).collect(),
            staged: false,
        });
        self
    }

    /// Add a staged database index
    ///
    /// Staged indexes are not immediately active and don't block deployment.
    pub fn staged_index(mut self, name: impl Into<String>, fields: Vec<impl Into<String>>) -> Self {
        self.indexes.push(IndexDefinition {
            name: name.into(),
            fields: fields.into_iter().map(|f| f.into()).collect(),
            staged: true,
        });
        self
    }

    /// Add a search index
    ///
    /// # Arguments
    ///
    /// * `name` - The name of the search index
    /// * `config` - The search index configuration
    ///
    /// # Example
    ///
    /// ```ignore
    /// let table = define_table({
    ///     title: v.string(),
    ///     content: v.string(),
    /// })
    /// .search_index("search_content", SearchIndexConfig {
    ///     search_field: "content",
    ///     filter_fields: vec!["title"],
    /// });
    /// ```
    pub fn search_index(mut self, name: impl Into<String>, config: SearchIndexConfig) -> Self {
        self.search_indexes.push(SearchIndexDefinition {
            name: name.into(),
            search_field: config.search_field,
            filter_fields: config.filter_fields,
            staged: false,
        });
        self
    }

    /// Add a vector index
    ///
    /// # Arguments
    ///
    /// * `name` - The name of the vector index
    /// * `config` - The vector index configuration
    ///
    /// # Example
    ///
    /// ```ignore
    /// let table = define_table({
    ///     embedding: v.array(v.float64()),
    ///     category: v.string(),
    /// })
    /// .vector_index("by_embedding", VectorIndexConfig {
    ///     vector_field: "embedding",
    ///     dimensions: 1536,
    ///     filter_fields: vec!["category"],
    /// });
    /// ```
    pub fn vector_index(mut self, name: impl Into<String>, config: VectorIndexConfig) -> Self {
        self.vector_indexes.push(VectorIndexDefinition {
            name: name.into(),
            vector_field: config.vector_field,
            dimensions: config.dimensions,
            filter_fields: config.filter_fields,
            staged: false,
        });
        self
    }

    /// Get the document validator
    pub fn document_type(&self) -> &Validator {
        &self.document_type
    }
}

/// A database index definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexDefinition {
    /// The name of the index
    pub name: String,
    /// The fields to index, in order
    pub fields: Vec<String>,
    /// Whether this is a staged index
    pub staged: bool,
}

/// A search index definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchIndexDefinition {
    /// The name of the search index
    pub name: String,
    /// The field to search
    pub search_field: String,
    /// Fields available for filtering
    pub filter_fields: Vec<String>,
    /// Whether this is a staged index
    pub staged: bool,
}

/// A vector index definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorIndexDefinition {
    /// The name of the vector index
    pub name: String,
    /// The field containing the vector
    pub vector_field: String,
    /// The dimensions of the vector
    pub dimensions: u32,
    /// Fields available for filtering
    pub filter_fields: Vec<String>,
    /// Whether this is a staged index
    pub staged: bool,
}

/// Configuration for a search index
#[derive(Debug, Clone)]
pub struct SearchIndexConfig {
    /// The field to search
    pub search_field: String,
    /// Fields available for filtering (optional)
    pub filter_fields: Vec<String>,
}

impl SearchIndexConfig {
    /// Create a new search index config
    pub fn new(search_field: impl Into<String>) -> Self {
        Self {
            search_field: search_field.into(),
            filter_fields: Vec::new(),
        }
    }

    /// Add filter fields
    pub fn with_filter_fields(mut self, fields: Vec<impl Into<String>>) -> Self {
        self.filter_fields = fields.into_iter().map(|f| f.into()).collect();
        self
    }
}

/// Configuration for a vector index
#[derive(Debug, Clone)]
pub struct VectorIndexConfig {
    /// The field containing the vector
    pub vector_field: String,
    /// The dimensions of the vector
    pub dimensions: u32,
    /// Fields available for filtering (optional)
    pub filter_fields: Vec<String>,
}

impl VectorIndexConfig {
    /// Create a new vector index config
    pub fn new(vector_field: impl Into<String>, dimensions: u32) -> Self {
        Self {
            vector_field: vector_field.into(),
            dimensions,
            filter_fields: Vec::new(),
        }
    }

    /// Add filter fields
    pub fn with_filter_fields(mut self, fields: Vec<impl Into<String>>) -> Self {
        self.filter_fields = fields.into_iter().map(|f| f.into()).collect();
        self
    }
}

/// Macro to define a Convex schema
///
/// # Example
///
/// ```ignore
/// use convex_sdk::schema::{define_schema, define_table, v};
///
/// define_schema!({
///     users: define_table!({
///         name: v.string(),
///         email: v.string(),
///     })
///     .index("by_email", ["email"]),
/// });
/// ```
#[macro_export]
macro_rules! define_schema {
    ({ $($table_name:ident: $table_def:expr),* $(,)? }) => {{
        let mut schema = $crate::schema::SchemaDefinition::new();
        $(
            schema = schema.add_table(stringify!($table_name), $table_def);
        )*
        schema
    }};
}

/// Macro to define a Convex table
///
/// # Example
///
/// ```ignore
/// use convex_sdk::schema::{define_table, v};
///
/// define_table!({
///     name: v.string(),
///     email: v.string(),
/// });
/// ```
#[macro_export]
macro_rules! define_table {
    ({ $($field_name:ident: $validator:expr),* $(,)? }) => {{
        let mut fields = ::alloc::collections::BTreeMap::new();
        $(
            fields.insert(stringify!($field_name).to_string(), $validator);
        )*
        let validator = $crate::schema::v.object(fields);
        $crate::schema::TableDefinition::new(validator)
    }};
}

/// Re-export macros
pub use define_schema;
pub use define_table;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_schema_definition() {
        let schema = SchemaDefinition::new()
            .add_table("users", TableDefinition::new(v.object(BTreeMap::new())))
            .add_table("posts", TableDefinition::new(v.object(BTreeMap::new())));

        assert!(schema.tables.contains_key("users"));
        assert!(schema.tables.contains_key("posts"));
    }

    #[test]
    fn test_table_with_index() {
        let table = TableDefinition::new(v.object(BTreeMap::new()))
            .index("by_email", vec!["email"]);

        assert_eq!(table.indexes.len(), 1);
        assert_eq!(table.indexes[0].name, "by_email");
        assert_eq!(table.indexes[0].fields, vec!["email"]);
    }

    #[test]
    fn test_search_index_config() {
        let config = SearchIndexConfig::new("content")
            .with_filter_fields(vec!["category"]);

        assert_eq!(config.search_field, "content");
        assert_eq!(config.filter_fields, vec!["category"]);
    }

    #[test]
    fn test_vector_index_config() {
        let config = VectorIndexConfig::new("embedding", 1536)
            .with_filter_fields(vec!["category"]);

        assert_eq!(config.vector_field, "embedding");
        assert_eq!(config.dimensions, 1536);
        assert_eq!(config.filter_fields, vec!["category"]);
    }
}

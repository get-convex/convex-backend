//! Component system for calling other Convex UDFs
//!
//! This module provides types and utilities for calling other Convex functions
//! (queries, mutations, actions) from within Rust/WASM functions.
//!
//! # Example
//!
//! ```ignore
//! use convex_sdk::*;
//! use convex_sdk::components::{call_query, call_mutation};
//!
//! #[query]
//! pub async fn get_user_with_posts(db: Database, user_id: String) -> Result<ConvexValue> {
//!     // Get the user
//!     let user = db.get(user_id.clone().into()).await?;
//!
//!     // Call another query to get user's posts
//!     let posts = call_query(
//!         "posts/getByAuthor",
//!         json!({ "authorId": user_id }),
//!     ).await?;
//!
//!     Ok(json!({
//!         "user": user,
//!         "posts": posts,
//!     }))
//! }
//! ```

use serde::{Deserialize, Serialize};

/// Reference to a component and function
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionReference {
    /// Component path (empty for root component)
    pub component: Option<String>,
    /// Module path within the component
    pub module: String,
    /// Function name within the module
    pub function: String,
}

impl FunctionReference {
    /// Create a new function reference
    pub fn new(module: impl Into<String>, function: impl Into<String>) -> Self {
        Self {
            component: None,
            module: module.into(),
            function: function.into(),
        }
    }

    /// Create a reference to a function in a specific component
    pub fn in_component(
        component: impl Into<String>,
        module: impl Into<String>,
        function: impl Into<String>,
    ) -> Self {
        Self {
            component: Some(component.into()),
            module: module.into(),
            function: function.into(),
        }
    }

    /// Parse a function reference from a string path
    /// Format: "component/module/function" or "module/function" for root
    pub fn parse(path: &str) -> anyhow::Result<Self> {
        let parts: Vec<&str> = path.split('/').collect();
        match parts.len() {
            2 => Ok(Self::new(parts[0], parts[1])),
            3 => Ok(Self::in_component(parts[0], parts[1], parts[2])),
            _ => anyhow::bail!("Invalid function path: {}. Expected 'module/function' or 'component/module/function'", path),
        }
    }
}

/// Type of UDF being called
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UdfType {
    /// Read-only query
    Query,
    /// Read-write mutation
    Mutation,
    /// Side-effect capable action
    Action,
}

/// Request to call another UDF
#[derive(Debug, Serialize)]
pub struct CallUdfRequest {
    /// The function to call
    #[serde(flatten)]
    pub function: FunctionReference,
    /// Type of UDF
    pub udf_type: UdfType,
    /// Arguments to pass
    pub args: serde_json::Value,
}

/// Response from calling a UDF
#[derive(Debug, Deserialize)]
pub struct CallUdfResponse {
    /// Whether the call was successful
    pub success: bool,
    /// The result value (if successful)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    /// Error message (if failed)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Component client trait for calling other UDFs
///
/// This trait abstracts the ability to call other Convex functions.
/// The actual implementation is provided by the Convex runtime.
pub trait ComponentClient: Send + Sync {
    /// Call another UDF
    fn call_udf(
        &self,
        function: FunctionReference,
        udf_type: UdfType,
        args: serde_json::Value,
    ) -> anyhow::Result<serde_json::Value>;

    /// Call a query function
    fn call_query(
        &self,
        function: FunctionReference,
        args: serde_json::Value,
    ) -> anyhow::Result<serde_json::Value> {
        self.call_udf(function, UdfType::Query, args)
    }

    /// Call a mutation function
    fn call_mutation(
        &self,
        function: FunctionReference,
        args: serde_json::Value,
    ) -> anyhow::Result<serde_json::Value> {
        self.call_udf(function, UdfType::Mutation, args)
    }

    /// Call an action function
    fn call_action(
        &self,
        function: FunctionReference,
        args: serde_json::Value,
    ) -> anyhow::Result<serde_json::Value> {
        self.call_udf(function, UdfType::Action, args)
    }
}

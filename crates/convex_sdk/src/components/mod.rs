//! Component system for Convex
//!
//! This module provides functionality for defining and using components,
//! which are modular, reusable pieces of functionality.
//!
//! # Example
//!
//! ```ignore
//! use convex_sdk::components::{define_component, ComponentDefinition};
//!
//! define_component!({
//!     name: "analytics",
//!     exports: {
//!         queries: ["getStats"],
//!         mutations: ["trackEvent"],
//!     },
//! });
//! ```

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use serde::{Deserialize, Serialize};

/// A component definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentDefinition {
    /// The name of the component
    pub name: String,
    /// Exported functions by type
    pub exports: ComponentExports,
    /// Child components (for apps)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub components: Option<BTreeMap<String, ComponentInstance>>,
}

/// Component exports
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ComponentExports {
    /// Exported query functions
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub queries: Vec<String>,
    /// Exported mutation functions
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub mutations: Vec<String>,
    /// Exported action functions
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub actions: Vec<String>,
    /// Exported HTTP actions
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub http_actions: Vec<String>,
}

/// An instance of a component
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentInstance {
    /// The component path or identifier
    pub component: String,
    /// Child component instances (for nested apps)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub children: Option<BTreeMap<String, ComponentInstance>>,
}

/// Builder for component definitions
#[derive(Debug)]
pub struct ComponentBuilder {
    name: String,
    exports: ComponentExports,
}

impl ComponentBuilder {
    /// Create a new component builder
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            exports: ComponentExports::default(),
        }
    }

    /// Export a query function
    pub fn export_query(mut self, name: impl Into<String>) -> Self {
        self.exports.queries.push(name.into());
        self
    }

    /// Export a mutation function
    pub fn export_mutation(mut self, name: impl Into<String>) -> Self {
        self.exports.mutations.push(name.into());
        self
    }

    /// Export an action function
    pub fn export_action(mut self, name: impl Into<String>) -> Self {
        self.exports.actions.push(name.into());
        self
    }

    /// Export an HTTP action
    pub fn export_http_action(mut self, name: impl Into<String>) -> Self {
        self.exports.http_actions.push(name.into());
        self
    }

    /// Build the component definition
    pub fn build(self) -> ComponentDefinition {
        ComponentDefinition {
            name: self.name,
            exports: self.exports,
            components: None,
        }
    }
}

/// Reference to a component function
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionReference {
    /// Component path (dot-separated for nested components)
    pub component: Option<String>,
    /// Function name
    pub name: String,
    /// Function type
    pub udf_type: UdfType,
}

/// Function type for component references
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UdfType {
    /// Query function
    Query,
    /// Mutation function
    Mutation,
    /// Action function
    Action,
    /// HTTP action
    HttpAction,
}

impl FunctionReference {
    /// Create a reference to a query in the root component
    pub fn query(name: impl Into<String>) -> Self {
        Self {
            component: None,
            name: name.into(),
            udf_type: UdfType::Query,
        }
    }

    /// Create a reference to a mutation in the root component
    pub fn mutation(name: impl Into<String>) -> Self {
        Self {
            component: None,
            name: name.into(),
            udf_type: UdfType::Mutation,
        }
    }

    /// Create a reference to an action in the root component
    pub fn action(name: impl Into<String>) -> Self {
        Self {
            component: None,
            name: name.into(),
            udf_type: UdfType::Action,
        }
    }

    /// Create a reference to a query in a specific component
    pub fn component_query(component: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            component: Some(component.into()),
            name: name.into(),
            udf_type: UdfType::Query,
        }
    }

    /// Create a reference to a mutation in a specific component
    pub fn component_mutation(component: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            component: Some(component.into()),
            name: name.into(),
            udf_type: UdfType::Mutation,
        }
    }

    /// Create a reference to an action in a specific component
    pub fn component_action(component: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            component: Some(component.into()),
            name: name.into(),
            udf_type: UdfType::Action,
        }
    }
}

/// Macro to define a component
///
/// # Example
///
/// ```ignore
/// define_component!({
///     name: "analytics",
///     exports: {
///         queries: ["getStats", "getEvents"],
///         mutations: ["trackEvent"],
///         actions: ["exportData"],
///     },
/// });
/// ```
#[macro_export]
macro_rules! define_component {
    ({
        name: $name:expr,
        exports: {
            $(queries: [$($query:expr),* $(,)?],)?
            $(mutations: [$($mutation:expr),* $(,)?],)?
            $(actions: [$($action:expr),* $(,)?],)?
            $(http_actions: [$($http_action:expr),* $(,)?],)?
        } $(,)?
    }) => {{
        let mut builder = $crate::components::ComponentBuilder::new($name);
        $(
            $(builder = builder.export_query($query);)*
        )?
        $(
            $(builder = builder.export_mutation($mutation);)*
        )?
        $(
            $(builder = builder.export_action($action);)*
        )?
        $(
            $(builder = builder.export_http_action($http_action);)*
        )?
        builder.build()
    }};
}

/// Macro to define an app (root component with child components)
///
/// # Example
///
/// ```ignore
/// define_app!({
///     components: {
///         analytics: "analytics",
///         billing: "billing",
///     },
/// });
/// ```
#[macro_export]
macro_rules! define_app {
    ({
        components: {
            $($name:ident: $component:expr),* $(,)?
        } $(,)?
    }) => {{
        let mut app = $crate::components::ComponentDefinition {
            name: "app".to_string(),
            exports: $crate::components::ComponentExports::default(),
            components: Some(::alloc::collections::BTreeMap::new()),
        };
        $(
            if let Some(ref mut comps) = app.components {
                comps.insert(
                    stringify!($name).to_string(),
                    $crate::components::ComponentInstance {
                        component: $component.to_string(),
                        children: None,
                    }
                );
            }
        )*
        app
    }};
}

// Re-export macros
pub use define_component;
pub use define_app;

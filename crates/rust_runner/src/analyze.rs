//! Rust Module Analyzer
//!
//! This module provides functionality to analyze Rust source code and extract
//! Convex function metadata. It uses the `syn` crate to parse Rust code and
//! identify functions marked with `#[query]`, `#[mutation]`, or `#[action]` attributes.

use std::str::FromStr;

use anyhow::Context;
use common::types::UdfType;
use model::modules::{
    function_validators::{ArgsValidator, ReturnsValidator},
    module_versions::{AnalyzedFunction, AnalyzedModule, Visibility},
};
use syn::{visit::Visit, Attribute, FnArg, ItemFn, Pat, ReturnType, Type};
use convex_sync_types::FunctionName;

/// Analyzes a Rust source module and extracts Convex function metadata.
///
/// This function parses the Rust source code using `syn` and identifies all
/// functions marked with Convex attribute macros (`#[query]`, `#[mutation]`,
/// `#[action]`). It builds an `AnalyzedModule` containing metadata for each
/// exported function.
///
/// # Arguments
///
/// * `source` - The Rust source code to analyze
///
/// # Returns
///
/// Returns `Ok(AnalyzedModule)` containing all extracted function metadata,
/// or an error if parsing fails.
///
/// # Example
///
/// ```rust,ignore
/// use rust_runner::analyze::analyze_rust_module;
///
/// let source = r#"
///     use convex_sdk::prelude::*;
///
///     #[query]
///     pub async fn get_user(ctx: &mut QueryContext, user_id: String) -> Result<User, Error> {
///         // ...
///     }
///
///     #[mutation]
///     pub async fn update_user(ctx: &mut MutationContext, user_id: String, data: UserUpdate) -> Result<(), Error> {
///         // ...
///     }
/// "#;
///
/// let analyzed = analyze_rust_module(source).unwrap();
/// assert_eq!(analyzed.functions.len(), 2);
/// ```
pub fn analyze_rust_module(source: &str) -> anyhow::Result<AnalyzedModule> {
    let file = syn::parse_file(source).context("Failed to parse Rust source code")?;

    let mut visitor = FunctionVisitor::default();
    visitor.visit_file(&file);

    let functions = visitor
        .functions
        .into_iter()
        .map(|func_info| {
            let name = FunctionName::from_str(&func_info.name)?;
            let udf_type = func_info.udf_type;

            // For now, we don't extract detailed argument/return type validators
            // from Rust code - this would require more sophisticated type analysis
            // or additional attribute macros for validation
            let args = ArgsValidator::Unvalidated;
            let returns = ReturnsValidator::Unvalidated;

            // Determine visibility based on function visibility
            let visibility = if func_info.is_public {
                Some(Visibility::Public)
            } else {
                Some(Visibility::Internal)
            };

            // Create the analyzed function without source position
            // (we could add line number extraction from Span if needed)
            let analyzed_func = AnalyzedFunction::new(
                name,
                None, // pos - could extract from Span if needed
                udf_type,
                visibility,
                args,
                returns,
            )?;

            Ok(analyzed_func)
        })
        .collect::<anyhow::Result<Vec<_>>>()?;

    Ok(AnalyzedModule {
        functions: functions.into(),
        http_routes: None,
        cron_specs: None,
        source_index: None,
    })
}

/// Information about a function extracted from the AST.
#[derive(Debug, Clone)]
struct FunctionInfo {
    /// The name of the function
    name: String,
    /// The UDF type (Query, Mutation, Action) determined by the attribute
    udf_type: UdfType,
    /// Whether the function is public
    is_public: bool,
    /// Function parameters (for potential future validation extraction)
    _params: Vec<ParamInfo>,
    /// Return type information
    _return_type: Option<String>,
}

/// Information about a function parameter.
#[derive(Debug, Clone)]
struct ParamInfo {
    name: String,
    ty: String,
}

/// Visitor that traverses the AST to find Convex-exported functions.
#[derive(Default)]
struct FunctionVisitor {
    functions: Vec<FunctionInfo>,
}

impl<'ast> Visit<'ast> for FunctionVisitor {
    fn visit_item_fn(&mut self, item: &'ast ItemFn) {
        // Check if this function has a Convex attribute
        if let Some(udf_type) = extract_udf_type(&item.attrs) {
            let name = item.sig.ident.to_string();
            let is_public = matches!(item.vis, syn::Visibility::Public(_));

            // Extract parameter information
            let params = item
                .sig
                .inputs
                .iter()
                .filter_map(|arg| match arg {
                    FnArg::Typed(pat_type) => {
                        let name = match &*pat_type.pat {
                            Pat::Ident(pat_ident) => pat_ident.ident.to_string(),
                            _ => "unknown".to_string(),
                        };
                        let ty = type_to_string(&pat_type.ty);
                        Some(ParamInfo { name, ty })
                    }
                    FnArg::Receiver(_) => None, // Skip `self` parameters
                })
                .collect();

            // Extract return type
            let return_type = match &item.sig.output {
                ReturnType::Default => None,
                ReturnType::Type(_, ty) => Some(type_to_string(ty)),
            };

            self.functions.push(FunctionInfo {
                name,
                udf_type,
                is_public,
                _params: params,
                _return_type: return_type,
            });
        }

        // Continue visiting nested items
        syn::visit::visit_item_fn(self, item);
    }
}

/// Extracts the UDF type from function attributes.
///
/// Looks for `#[query]`, `#[mutation]`, or `#[action]` attributes and returns
/// the corresponding `UdfType`. Returns `None` if no Convex attribute is found.
fn extract_udf_type(attrs: &[Attribute]) -> Option<UdfType> {
    for attr in attrs {
        let path = &attr.path();

        // Handle both simple paths and paths with segments
        let path_str = if path.leading_colon.is_some() {
            format!(":: {}", path_to_string(path))
        } else {
            path_to_string(path)
        };

        // Check for Convex attribute macros
        // They could be `query`, `mutation`, `action` or paths like `convex_sdk::query`
        if path_str == "query" || path_str.ends_with("::query") {
            return Some(UdfType::Query);
        } else if path_str == "mutation" || path_str.ends_with("::mutation") {
            return Some(UdfType::Mutation);
        } else if path_str == "action" || path_str.ends_with("::action") {
            return Some(UdfType::Action);
        } else if path_str == "http_action" || path_str.ends_with("::http_action") {
            return Some(UdfType::HttpAction);
        }
    }
    None
}

/// Converts a type path to a string representation.
fn path_to_string(path: &syn::Path) -> String {
    path.segments
        .iter()
        .map(|seg| seg.ident.to_string())
        .collect::<Vec<_>>()
        .join("::")
}

/// Converts a type to its string representation.
fn type_to_string(ty: &Type) -> String {
    // Use quote to convert the type back to a string representation
    quote::quote!(#ty).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_analyze_simple_query() {
        let source = r#"
            use convex_sdk::prelude::*;

            #[query]
            pub async fn get_user(ctx: &mut QueryContext, user_id: String) -> Result<JsonValue, Error> {
                Ok(JsonValue::Null)
            }
        "#;

        let result = analyze_rust_module(source).unwrap();
        assert_eq!(result.functions.len(), 1);
        assert_eq!(result.functions[0].name.to_string(), "get_user");
        assert_eq!(result.functions[0].udf_type, UdfType::Query);
    }

    #[test]
    fn test_analyze_mutation() {
        let source = r#"
            #[mutation]
            pub async fn create_user(ctx: &mut MutationContext, name: String) -> Result<ID, Error> {
                Ok(ID::new())
            }
        "#;

        let result = analyze_rust_module(source).unwrap();
        assert_eq!(result.functions.len(), 1);
        assert_eq!(result.functions[0].name.to_string(), "create_user");
        assert_eq!(result.functions[0].udf_type, UdfType::Mutation);
    }

    #[test]
    fn test_analyze_action() {
        let source = r#"
            #[action]
            pub async fn send_email(ctx: &mut ActionContext, to: String, subject: String) -> Result<(), Error> {
                Ok(())
            }
        "#;

        let result = analyze_rust_module(source).unwrap();
        assert_eq!(result.functions.len(), 1);
        assert_eq!(result.functions[0].name.to_string(), "send_email");
        assert_eq!(result.functions[0].udf_type, UdfType::Action);
    }

    #[test]
    fn test_analyze_multiple_functions() {
        let source = r#"
            #[query]
            pub async fn get_user(ctx: &mut QueryContext, id: String) -> Result<User, Error> {
                Ok(User::default())
            }

            #[query]
            pub async fn list_users(ctx: &mut QueryContext) -> Result<Vec<User>, Error> {
                Ok(vec![])
            }

            #[mutation]
            pub async fn update_user(ctx: &mut MutationContext, id: String, data: UserUpdate) -> Result<(), Error> {
                Ok(())
            }

            #[action]
            pub async fn notify_user(ctx: &mut ActionContext, id: String) -> Result<(), Error> {
                Ok(())
            }

            // This function should not be included (no attribute)
            fn helper_function() -> i32 {
                42
            }
        "#;

        let result = analyze_rust_module(source).unwrap();
        assert_eq!(result.functions.len(), 4);

        let names: Vec<_> = result.functions.iter().map(|f| f.name.to_string()).collect();
        assert!(names.contains(&"get_user".to_string()));
        assert!(names.contains(&"list_users".to_string()));
        assert!(names.contains(&"update_user".to_string()));
        assert!(names.contains(&"notify_user".to_string()));
    }

    #[test]
    fn test_analyze_with_module_path() {
        let source = r#"
            #[convex_sdk::query]
            pub async fn get_user(ctx: &mut QueryContext, id: String) -> Result<User, Error> {
                Ok(User::default())
            }

            #[convex_sdk::mutation]
            pub async fn update_user(ctx: &mut MutationContext, id: String, data: UserUpdate) -> Result<(), Error> {
                Ok(())
            }
        "#;

        let result = analyze_rust_module(source).unwrap();
        assert_eq!(result.functions.len(), 2);
    }

    #[test]
    fn test_analyze_empty_file() {
        let source = r#"
            // Just a comment
            use std::collections::HashMap;

            fn internal_helper() {}
        "#;

        let result = analyze_rust_module(source).unwrap();
        assert_eq!(result.functions.len(), 0);
    }

    #[test]
    fn test_analyze_invalid_rust() {
        let source = "this is not valid rust code!@#$";
        let result = analyze_rust_module(source);
        assert!(result.is_err());
    }

    #[test]
    fn test_visibility_public() {
        let source = r#"
            #[query]
            pub fn public_query() -> i32 { 42 }

            #[query]
            fn private_query() -> i32 { 42 }
        "#;

        let result = analyze_rust_module(source).unwrap();
        assert_eq!(result.functions.len(), 2);

        let public_func = result
            .functions
            .iter()
            .find(|f| f.name.to_string() == "public_query")
            .unwrap();
        assert_eq!(public_func.visibility, Some(Visibility::Public));

        let private_func = result
            .functions
            .iter()
            .find(|f| f.name.to_string() == "private_query")
            .unwrap();
        assert_eq!(private_func.visibility, Some(Visibility::Internal));
    }

    #[test]
    fn test_http_action() {
        let source = r#"
            #[http_action]
            pub async fn handle_request(ctx: &mut HttpActionContext, request: Request) -> Response {
                Response::new("Hello")
            }
        "#;

        let result = analyze_rust_module(source).unwrap();
        assert_eq!(result.functions.len(), 1);
        assert_eq!(result.functions[0].udf_type, UdfType::HttpAction);
    }
}

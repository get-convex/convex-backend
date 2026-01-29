//! Proc macros for the Convex Rust SDK
//!
//! This crate provides the `#[query]`, `#[mutation]`, and `#[action]` macros
//! for defining Convex backend functions in Rust.

use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::{format_ident, quote};
use syn::{parse_macro_input, FnArg, Ident, ItemFn, Pat, PatType, ReturnType, Type};

/// Marks a function as a Convex query
///
/// Queries are read-only, deterministic functions that can be cached.
/// They run in a V8 isolate (via WASM) with access to the database.
///
/// # Example
/// ```ignore
/// #[query]
/// async fn get_user(db: Database, id: String) -> Result<Option<Document>, ConvexError> {
///     db.get(id.into()).await
/// }
/// ```
#[proc_macro_attribute]
pub fn query(_args: TokenStream, input: TokenStream) -> TokenStream {
    let input_fn = parse_macro_input!(input as ItemFn);
    generate_function_wrapper(input_fn, FunctionType::Query)
}

/// Marks a function as a Convex mutation
///
/// Mutations are read-write, transactional, deterministic functions.
/// They run in a V8 isolate (via WASM) with full database access.
///
/// # Example
/// ```ignore
/// #[mutation]
/// async fn create_user(
///     db: Database,
///     name: String,
///     email: String,
/// ) -> Result<DocumentId, ConvexError> {
///     db.insert("users", json!({ "name": name, "email": email })).await
/// }
/// ```
#[proc_macro_attribute]
pub fn mutation(_args: TokenStream, input: TokenStream) -> TokenStream {
    let input_fn = parse_macro_input!(input as ItemFn);
    generate_function_wrapper(input_fn, FunctionType::Mutation)
}

/// Marks a function as a Convex action
///
/// Actions can perform side effects and are non-deterministic.
/// They run in a Node.js-compatible environment.
///
/// # Example
/// ```ignore
/// #[action]
/// async fn send_email(to: String, subject: String, body: String) -> Result<(), ConvexError> {
///     // Make HTTP request to email service
///     let response = fetch("https://api.emailservice.com/send", FetchOptions::new()
///         .method("POST")
///         .header("Content-Type", "application/json")
///         .body(json!({ "to": to, "subject": subject, "body": body }).to_string().into_bytes()))
///         .await?;
///
///     if response.status == 200 {
///         Ok(())
///     } else {
///         Err(ConvexError::Unknown("Failed to send email".to_string()))
///     }
/// }
/// ```
#[proc_macro_attribute]
pub fn action(_args: TokenStream, input: TokenStream) -> TokenStream {
    let input_fn = parse_macro_input!(input as ItemFn);
    generate_function_wrapper(input_fn, FunctionType::Action)
}

#[derive(Debug, Clone, Copy)]
enum FunctionType {
    Query,
    Mutation,
    Action,
}

impl FunctionType {
    fn as_str(&self) -> &'static str {
        match self {
            FunctionType::Query => "query",
            FunctionType::Mutation => "mutation",
            FunctionType::Action => "action",
        }
    }
}

/// Extracts the type name as a string for metadata
fn type_to_string(ty: &Type) -> String {
    quote!(#ty).to_string().replace(' ', "")
}

/// Extracts the argument names and types from a function signature
fn extract_arguments(sig: &syn::Signature) -> Vec<(Ident, Box<Type>)> {
    let mut args = Vec::new();

    for (idx, arg) in sig.inputs.iter().enumerate() {
        match arg {
            FnArg::Typed(PatType { pat, ty, .. }) => {
                // Extract the identifier name
                let ident = match pat.as_ref() {
                    Pat::Ident(pat_ident) => pat_ident.ident.clone(),
                    _ => Ident::new(&format!("arg{}", idx), Span::call_site()),
                };
                args.push((ident, ty.clone()));
            }
            FnArg::Receiver(_) => {
                // Skip `self` parameters - they shouldn't appear in Convex functions
                continue;
            }
        }
    }

    args
}

/// Extracts the return type as a string
fn extract_return_type(sig: &syn::Signature) -> String {
    match &sig.output {
        ReturnType::Default => "()".to_string(),
        ReturnType::Type(_, ty) => type_to_string(ty),
    }
}

/// Generates argument deserialization code
fn generate_arg_deserialization(args: &[(Ident, Box<Type>)]) -> proc_macro2::TokenStream {
    let mut tokens = proc_macro2::TokenStream::new();

    for (idx, (ident, ty)) in args.iter().enumerate() {
        let idx_lit = syn::Index::from(idx);
        let deser = quote! {
            let #ident: #ty = match serde_json::from_value(args.get(#idx_lit).cloned().unwrap_or(serde_json::Value::Null)) {
                Ok(v) => v,
                Err(e) => {
                    let error_json = serde_json::json!({
                        "error": format!("Argument deserialization failed for '{}': {}", stringify!(#ident), e)
                    });
                    let error_bytes = serde_json::to_vec(&error_json).unwrap_or_default();
                    let error_ptr = unsafe { __convex_alloc(error_bytes.len() as i32) };
                    if error_ptr != 0 {
                        unsafe {
                            std::ptr::copy_nonoverlapping(
                                error_bytes.as_ptr(),
                                error_ptr as *mut u8,
                                error_bytes.len()
                            );
                        }
                    }
                    return -(error_ptr as i32);
                }
            };
        };
        tokens.extend(deser);
    }

    tokens
}

/// Generates the list of argument names for calling the original function
fn generate_arg_names(args: &[(Ident, Box<Type>)]) -> proc_macro2::TokenStream {
    let idents: Vec<_> = args.iter().map(|(ident, _)| ident).collect();
    quote!(#(#idents),*)
}

/// Generates metadata JSON for the function
fn generate_metadata_json(
    fn_name: &Ident,
    function_type: FunctionType,
    args: &[(Ident, Box<Type>)],
    return_type: &str,
) -> proc_macro2::TokenStream {
    let fn_name_str = fn_name.to_string();
    let fn_type_str = function_type.as_str();

    // Build argument metadata
    let arg_metadata: Vec<_> = args
        .iter()
        .map(|(ident, ty)| {
            let name = ident.to_string();
            let type_str = type_to_string(ty);
            quote!({
                "name": #name,
                "type": #type_str
            })
        })
        .collect();

    quote!({
        "name": #fn_name_str,
        "type": #fn_type_str,
        "args": [#(#arg_metadata),*],
        "returns": #return_type
    })
}

fn generate_function_wrapper(
    input_fn: ItemFn,
    function_type: FunctionType,
) -> TokenStream {
    let fn_name = &input_fn.sig.ident;
    let fn_vis = &input_fn.vis;
    let fn_sig = &input_fn.sig;
    let fn_block = &input_fn.block;
    let fn_attrs = &input_fn.attrs;

    // Generate metadata function name
    let metadata_fn_name = format_ident!("__convex_metadata_{}", fn_name);
    let internal_fn_name = format_ident!("__convex_internal_{}", fn_name);

    // Check if the function is async
    let is_async = fn_sig.asyncness.is_some();

    // Extract function arguments
    let args = extract_arguments(fn_sig);
    let return_type_str = extract_return_type(fn_sig);

    // Generate argument deserialization
    let arg_deserialization = generate_arg_deserialization(&args);
    let arg_names = generate_arg_names(&args);

    // Generate metadata JSON
    let metadata_json = generate_metadata_json(fn_name, function_type, &args, &return_type_str);

    // Determine if this function needs a Database parameter (queries and mutations do)
    let needs_db = matches!(function_type, FunctionType::Query | FunctionType::Mutation);

    // Generate the wrapper that handles WASM ABI
    let wrapper = if is_async {
        if needs_db {
            // Async function with Database parameter (queries and mutations)
            quote! {
                // Original function (kept for internal use)
                #(#fn_attrs)*
                #fn_vis #fn_sig #fn_block

                /// Internal wrapper that handles the async execution
                async fn #internal_fn_name(args: Vec<serde_json::Value>) -> Result<serde_json::Value, convex_sdk::ConvexError> {
                    // First argument is the database handle
                    let db_handle: u32 = serde_json::from_value(args.get(0).cloned().unwrap_or(serde_json::Value::Null))
                        .map_err(|e| convex_sdk::ConvexError::InvalidArgument(format!("Invalid database handle: {}", e)))?;
                    let db = convex_sdk::Database::new(db_handle);

                    // Deserialize remaining arguments
                    #arg_deserialization

                    // Call the original function
                    let result = #fn_name(db, #arg_names).await;

                    // Serialize result
                    serde_json::to_value(result)
                        .map_err(|e| convex_sdk::ConvexError::Serialization(e))
                }

                /// WASM export for the function
                #[no_mangle]
                pub extern "C" fn #fn_name(args_ptr: i32, args_len: i32) -> i32 {
                    // Use catch_unwind to handle panics gracefully
                    let result = std::panic::catch_unwind(|| {
                        // Deserialize arguments from WASM memory
                        let args_bytes = unsafe {
                            std::slice::from_raw_parts(args_ptr as *const u8, args_len as usize)
                        };

                        let args: Vec<serde_json::Value> = match serde_json::from_slice(args_bytes) {
                            Ok(a) => a,
                            Err(e) => {
                                let error_json = serde_json::json!({
                                    "error": format!("Failed to deserialize arguments: {}", e)
                                });
                                return serialize_and_return(error_json);
                            }
                        };

                        // Execute the async function using a simple executor
                        let result = match pollster::block_on(#internal_fn_name(args)) {
                            Ok(v) => v,
                            Err(e) => serde_json::json!({"error": e.to_string()}),
                        };

                        serialize_and_return(result)
                    });

                    match result {
                        Ok(ptr) => ptr,
                        Err(_) => -1,
                    }
                }

                /// Metadata export for the function
                #[no_mangle]
                pub extern "C" fn #metadata_fn_name() -> i32 {
                    let metadata = serde_json::json!(#metadata_json);
                    serialize_and_return(metadata)
                }
            }
        } else {
            // Async function without Database parameter (actions)
            quote! {
                // Original function (kept for internal use)
                #(#fn_attrs)*
                #fn_vis #fn_sig #fn_block

                /// Internal wrapper that handles the async execution
                async fn #internal_fn_name(args: Vec<serde_json::Value>) -> Result<serde_json::Value, convex_sdk::ConvexError> {
                    // Deserialize arguments
                    #arg_deserialization

                    // Call the original function
                    let result = #fn_name(#arg_names).await;

                    // Serialize result
                    serde_json::to_value(result)
                        .map_err(|e| convex_sdk::ConvexError::Serialization(e))
                }

                /// WASM export for the function
                #[no_mangle]
                pub extern "C" fn #fn_name(args_ptr: i32, args_len: i32) -> i32 {
                    // Use catch_unwind to handle panics gracefully
                    let result = std::panic::catch_unwind(|| {
                        // Deserialize arguments from WASM memory
                        let args_bytes = unsafe {
                            std::slice::from_raw_parts(args_ptr as *const u8, args_len as usize)
                        };

                        let args: Vec<serde_json::Value> = match serde_json::from_slice(args_bytes) {
                            Ok(a) => a,
                            Err(e) => {
                                let error_json = serde_json::json!({
                                    "error": format!("Failed to deserialize arguments: {}", e)
                                });
                                return serialize_and_return(error_json);
                            }
                        };

                        // Execute the async function using a simple executor
                        let result = match pollster::block_on(#internal_fn_name(args)) {
                            Ok(v) => v,
                            Err(e) => serde_json::json!({"error": e.to_string()}),
                        };

                        serialize_and_return(result)
                    });

                    match result {
                        Ok(ptr) => ptr,
                        Err(_) => -1,
                    }
                }

                /// Metadata export for the function
                #[no_mangle]
                pub extern "C" fn #metadata_fn_name() -> i32 {
                    let metadata = serde_json::json!(#metadata_json);
                    serialize_and_return(metadata)
                }
            }
        }
    } else {
        // Synchronous function
        if needs_db {
            quote! {
                // Original function (kept for internal use)
                #(#fn_attrs)*
                #fn_vis #fn_sig #fn_block

                /// WASM export for the function
                #[no_mangle]
                pub extern "C" fn #fn_name(args_ptr: i32, args_len: i32) -> i32 {
                    // Use catch_unwind to handle panics gracefully
                    let result = std::panic::catch_unwind(|| {
                        // Deserialize arguments from WASM memory
                        let args_bytes = unsafe {
                            std::slice::from_raw_parts(args_ptr as *const u8, args_len as usize)
                        };

                        let args: Vec<serde_json::Value> = match serde_json::from_slice(args_bytes) {
                            Ok(a) => a,
                            Err(e) => {
                                let error_json = serde_json::json!({
                                    "error": format!("Failed to deserialize arguments: {}", e)
                                });
                                return serialize_and_return(error_json);
                            }
                        };

                        // First argument is the database handle
                        let db_handle: u32 = match serde_json::from_value(args.get(0).cloned().unwrap_or(serde_json::Value::Null)) {
                            Ok(h) => h,
                            Err(e) => {
                                let error_json = serde_json::json!({
                                    "error": format!("Invalid database handle: {}", e)
                                });
                                return serialize_and_return(error_json);
                            }
                        };
                        let db = convex_sdk::Database::new(db_handle);

                        // Deserialize remaining arguments
                        #arg_deserialization

                        // Call the original function
                        let result = #fn_name(db, #arg_names);

                        // Serialize result
                        let result_json = match serde_json::to_value(result) {
                            Ok(v) => v,
                            Err(e) => serde_json::json!({"error": e.to_string()}),
                        };

                        serialize_and_return(result_json)
                    });

                    match result {
                        Ok(ptr) => ptr,
                        Err(_) => -1,
                    }
                }

                /// Metadata export for the function
                #[no_mangle]
                pub extern "C" fn #metadata_fn_name() -> i32 {
                    let metadata = serde_json::json!(#metadata_json);
                    serialize_and_return(metadata)
                }
            }
        } else {
            quote! {
                // Original function (kept for internal use)
                #(#fn_attrs)*
                #fn_vis #fn_sig #fn_block

                /// WASM export for the function
                #[no_mangle]
                pub extern "C" fn #fn_name(args_ptr: i32, args_len: i32) -> i32 {
                    // Use catch_unwind to handle panics gracefully
                    let result = std::panic::catch_unwind(|| {
                        // Deserialize arguments from WASM memory
                        let args_bytes = unsafe {
                            std::slice::from_raw_parts(args_ptr as *const u8, args_len as usize)
                        };

                        let args: Vec<serde_json::Value> = match serde_json::from_slice(args_bytes) {
                            Ok(a) => a,
                            Err(e) => {
                                let error_json = serde_json::json!({
                                    "error": format!("Failed to deserialize arguments: {}", e)
                                });
                                return serialize_and_return(error_json);
                            }
                        };

                        // Deserialize arguments
                        #arg_deserialization

                        // Call the original function
                        let result = #fn_name(#arg_names);

                        // Serialize result
                        let result_json = match serde_json::to_value(result) {
                            Ok(v) => v,
                            Err(e) => serde_json::json!({"error": e.to_string()}),
                        };

                        serialize_and_return(result_json)
                    });

                    match result {
                        Ok(ptr) => ptr,
                        Err(_) => -1,
                    }
                }

                /// Metadata export for the function
                #[no_mangle]
                pub extern "C" fn #metadata_fn_name() -> i32 {
                    let metadata = serde_json::json!(#metadata_json);
                    serialize_and_return(metadata)
                }
            }
        }
    };

    TokenStream::from(wrapper)
}

/// Generates a complete Convex module with metadata
///
/// This macro should be applied to a module containing query, mutation, and action functions.
/// It generates the necessary metadata and exports for the Convex bundler.
#[proc_macro_attribute]
pub fn convex_module(_args: TokenStream, input: TokenStream) -> TokenStream {
    // For now, just pass through - the full implementation would generate
    // module-level metadata
    input
}

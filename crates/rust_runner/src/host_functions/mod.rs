//! Host functions exposed to WASM guest
//!
//! These functions provide the Convex API to Rust/WASM functions.
//! The module is organized into submodules by functionality:
//!
//! - `http`: HTTP fetch operations (actions only)
//! - `storage`: File storage operations
//! - `util`: Utility functions (logging, random, time)
//!
//! # Security Model
//!
//! Host functions enforce security constraints:
//! - HTTP is only available in actions (not queries/mutations)
//! - Random and time are deterministic for queries/mutations
//! - All inputs are validated for size and format
//! - Memory operations are bounds-checked

use std::sync::Arc;

use anyhow::Context;
use common::http::fetch::FetchClient;
use common::runtime::Runtime;
use common::types::UdfType;
use rand::RngCore;
use rand_chacha::ChaCha12Rng;
use rand_chacha::rand_core::SeedableRng;
use serde::{Deserialize, Serialize};
use wasmtime::{Caller, Func, FuncType, Store, Val, ValType};
use wasi_common::WasiCtx;

pub mod http;
pub mod storage;
pub mod util;

pub use http::create_http_functions;
pub use storage::{create_storage_functions, StorageClient};
pub use util::create_util_functions;

/// Maximum number of log lines per function execution
const MAX_LOG_LINES: usize = 1000;

/// Host context passed to WASM functions
///
/// This struct holds all the state needed by host functions,
/// including the WASI context, execution type, and various clients.
pub struct HostContext<RT: Runtime> {
    /// WASI context for the guest
    pub(crate) wasi: WasiCtx,

    /// The type of UDF being executed (Query, Mutation, Action, HttpAction)
    udf_type: UdfType,

    /// Runtime handle for async operations
    _runtime: RT,

    /// HTTP client for fetch operations (only available in actions)
    fetch_client: Option<Arc<dyn FetchClient>>,

    /// Storage client for file operations
    storage_client: Option<Arc<dyn StorageClient>>,

    /// Database client for database operations
    database_client: Option<Arc<dyn DatabaseClient>>,

    /// Deterministic RNG for queries/mutations
    deterministic_rng: Option<ChaCha12Rng>,

    /// Deterministic timestamp for queries/mutations (in milliseconds since Unix epoch)
    deterministic_timestamp_ms: i64,

    /// Log line counter for rate limiting
    log_line_count: usize,
}

impl<RT: Runtime> HostContext<RT> {
    /// Create a new host context
    ///
    /// # Arguments
    /// * `wasi` - The WASI context
    /// * `udf_type` - The type of UDF being executed
    /// * `runtime` - The runtime handle
    /// * `seed` - Random seed for deterministic execution (queries/mutations)
    pub fn new(wasi: WasiCtx, udf_type: UdfType, runtime: RT, seed: u64) -> Self {
        Self {
            wasi,
            udf_type,
            _runtime: runtime,
            fetch_client: None,
            storage_client: None,
            database_client: None,
            deterministic_rng: if Self::is_deterministic_udf_type(&udf_type) {
                Some(ChaCha12Rng::seed_from_u64(seed))
            } else {
                None
            },
            deterministic_timestamp_ms: Self::current_timestamp_ms(),
            log_line_count: 0,
        }
    }

    /// Create a new host context with HTTP client (for actions)
    pub fn with_fetch_client(mut self, client: Arc<dyn FetchClient>) -> Self {
        self.fetch_client = Some(client);
        self
    }

    /// Create a new host context with storage client
    pub fn with_storage_client(mut self, client: Arc<dyn StorageClient>) -> Self {
        self.storage_client = Some(client);
        self
    }

    /// Create a new host context with database client
    pub fn with_database_client(mut self, client: Arc<dyn DatabaseClient>) -> Self {
        self.database_client = Some(client);
        self
    }

    /// Get the UDF type
    pub fn udf_type(&self) -> UdfType {
        self.udf_type
    }

    /// Get the fetch client if available
    pub fn fetch_client(&self) -> Option<Arc<dyn FetchClient>> {
        self.fetch_client.clone()
    }

    /// Get the storage client if available
    pub fn storage_client(&self) -> Option<Arc<dyn StorageClient>> {
        self.storage_client.clone()
    }

    /// Get the database client if available
    pub fn database_client(&self) -> Option<Arc<dyn DatabaseClient>> {
        self.database_client.clone()
    }

    /// Check if this execution should be deterministic
    ///
    /// Queries and mutations must be deterministic for consistency.
    /// Actions can use non-deterministic operations.
    pub fn is_deterministic(&self) -> bool {
        Self::is_deterministic_udf_type(&self.udf_type)
    }

    /// Check if a UDF type requires deterministic execution
    fn is_deterministic_udf_type(udf_type: &UdfType) -> bool {
        matches!(udf_type, UdfType::Query | UdfType::Mutation)
    }

    /// Fill a buffer with deterministic random bytes
    ///
    /// This uses a seeded ChaCha12 RNG for reproducibility.
    /// Returns an error if called in a non-deterministic context.
    pub fn fill_random_bytes_deterministic(&mut self, buf: &mut [u8]) -> anyhow::Result<()> {
        match &mut self.deterministic_rng {
            Some(rng) => {
                rng.fill_bytes(buf);
                Ok(())
            }
            None => Err(anyhow::anyhow!("Deterministic RNG not available in action context")),
        }
    }

    /// Fill a buffer with cryptographically secure random bytes
    ///
    /// This uses the system's CSPRNG.
    pub fn fill_random_bytes_secure(&mut self, buf: &mut [u8]) {
        rand::thread_rng().fill_bytes(buf);
    }

    /// Get the deterministic timestamp in milliseconds
    pub fn deterministic_timestamp_ms(&self) -> i64 {
        self.deterministic_timestamp_ms
    }

    /// Check if logging is allowed (rate limiting)
    pub fn check_log_rate_limit(&mut self) -> bool {
        if self.log_line_count >= MAX_LOG_LINES {
            return false;
        }
        self.log_line_count += 1;
        true
    }

    /// Get the current timestamp in milliseconds
    fn current_timestamp_ms() -> i64 {
        use std::time::{SystemTime, UNIX_EPOCH};
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as i64)
            .unwrap_or(0)
    }
}

impl<RT: Runtime> std::ops::Deref for HostContext<RT> {
    type Target = WasiCtx;

    fn deref(&self) -> &Self::Target {
        &self.wasi
    }
}

impl<RT: Runtime> std::ops::DerefMut for HostContext<RT> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.wasi
    }
}

/// Database client trait for host functions
///
/// This trait abstracts database operations needed by the Rust runner.
/// The actual implementation is provided by the function_runner crate,
/// which has access to the Convex database transaction.
pub trait DatabaseClient: Send + Sync {
    /// Query documents from a table
    fn query(
        &self,
        table: String,
    ) -> anyhow::Result<Vec<(String, serde_json::Value)>>;

    /// Get a single document by ID
    fn get(&self, id: String) -> anyhow::Result<Option<serde_json::Value>>;

    /// Insert a new document into a table
    fn insert(
        &self,
        table: String,
        value: serde_json::Value,
    ) -> anyhow::Result<String>;

    /// Patch (partial update) an existing document
    fn patch(&self, id: String, value: serde_json::Value) -> anyhow::Result<()>;

    /// Delete a document by ID
    fn delete(&self, id: String) -> anyhow::Result<()>;

    /// Count documents in a table
    fn count(&self, table: String) -> anyhow::Result<u64>;
}

/// Create all host functions for the WASM module
///
/// This function creates all the host functions that will be available
/// to the WASM guest. The functions are organized by category:
/// - Database operations
/// - HTTP operations (actions only)
/// - Storage operations
/// - Utility functions (logging, random, time)
pub fn create_host_functions<RT: Runtime>(
    store: &mut Store<HostContext<RT>>,
) -> Vec<(String, Func)> {
    let mut functions = Vec::new();

    // Database operations
    functions.extend(create_db_functions(store));

    // HTTP operations (actions only)
    functions.extend(create_http_functions(store));

    // Storage operations
    functions.extend(create_storage_functions(store));

    // Utility functions (logging, random, time)
    functions.extend(create_util_functions(store));

    functions
}

/// Create database-related host functions
///
/// These functions provide database access to WASM guests:
/// - `__convex_db_query`: Query documents from a table with optional filters
/// - `__convex_db_get`: Get a single document by ID
/// - `__convex_db_insert`: Insert a new document into a table
/// - `__convex_db_patch`: Patch (partial update) an existing document
/// - `__convex_db_delete`: Delete a document by ID
/// - `__convex_db_query_advanced`: Complex query with filters, ordering, and limits
/// - `__convex_db_count`: Count documents matching a query
fn create_db_functions<RT: Runtime>(store: &mut Store<HostContext<RT>>) -> Vec<(String, Func)> {
    vec![
        ("__convex_db_query".to_string(), create_db_query(store)),
        ("__convex_db_get".to_string(), create_db_get(store)),
        ("__convex_db_insert".to_string(), create_db_insert(store)),
        ("__convex_db_patch".to_string(), create_db_patch(store)),
        ("__convex_db_delete".to_string(), create_db_delete(store)),
        (
            "__convex_db_query_advanced".to_string(),
            create_db_query_advanced(store),
        ),
        ("__convex_db_count".to_string(), create_db_count(store)),
    ]
}

/// Database query request
#[derive(Debug, Serialize, Deserialize)]
struct DbQueryRequest {
    table: String,
}

/// Database query with filters request
#[derive(Debug, Serialize, Deserialize)]
struct DbQueryAdvancedRequest {
    table: String,
    filters: Vec<DbFilterCondition>,
    orders: Vec<DbOrderSpec>,
    limit: Option<usize>,
}

/// Filter condition for queries
#[derive(Debug, Serialize, Deserialize)]
struct DbFilterCondition {
    field: String,
    op: String,
    value: serde_json::Value,
}

/// Ordering specification
#[derive(Debug, Serialize, Deserialize)]
struct DbOrderSpec {
    field: String,
    ascending: bool,
}

/// Document representation
#[derive(Debug, Serialize, Deserialize)]
struct DbDocument {
    id: String,
    #[serde(flatten)]
    value: serde_json::Value,
}

/// Database operation result
#[derive(Debug, Serialize, Deserialize)]
struct DbResult {
    success: bool,
    data: Option<serde_json::Value>,
    error: Option<String>,
}

/// Create the db.query host function
fn create_db_query<RT: Runtime>(store: &mut Store<HostContext<RT>>) -> Func {
    let func_type = FuncType::new(
        vec![ValType::I32, ValType::I32], // table_ptr, table_len
        vec![ValType::I32],               // result_ptr
    );

    Func::new(
        store,
        func_type,
        move |mut caller: Caller<'_, HostContext<RT>>, params: &[Val], results: &mut [Val]| {
            let table_ptr = params[0].i32().unwrap_or(0);
            let table_len = params[1].i32().unwrap_or(0);

            // Read table name from WASM memory
            let table_name = match read_memory_string(&mut caller, table_ptr, table_len) {
                Ok(s) => s,
                Err(e) => {
                    let error_result = DbResult {
                        success: false,
                        data: None,
                        error: Some(format!("Failed to read table name: {}", e)),
                    };
                    let ptr = write_json_response(&mut caller, &error_result).unwrap_or(-1);
                    results[0] = Val::I32(ptr);
                    return Ok(());
                },
            };

            // Query the database if a database client is available
            let query_result = if let Some(db_client) = caller.data().database_client() {
                // NOTE: Database operations are async, but host functions are sync.
                // In production, this would need to be handled properly using the runtime
                // to block on the async operation. For now, we return a placeholder.
                // TODO: Use runtime.block_on() or similar to execute async database operations
                match db_client.query(table_name) {
                    Ok(docs) => {
                        let docs_json: Vec<serde_json::Value> = docs
                            .into_iter()
                            .map(|(id, value)| {
                                serde_json::json!({
                                    "id": id,
                                    "value": value
                                })
                            })
                            .collect();
                        DbResult {
                            success: true,
                            data: Some(serde_json::json!(docs_json)),
                            error: None,
                        }
                    },
                    Err(e) => DbResult {
                        success: false,
                        data: None,
                        error: Some(format!("Database query failed: {}", e)),
                    },
                }
            } else {
                // No database client available - return empty array
                DbResult {
                    success: true,
                    data: Some(serde_json::json!([])),
                    error: None,
                }
            };

            let ptr = write_json_response(&mut caller, &query_result).unwrap_or(-1);
            results[0] = Val::I32(ptr);
            Ok(())
        },
    )
}

/// Create the db.get host function
fn create_db_get<RT: Runtime>(store: &mut Store<HostContext<RT>>) -> Func {
    let func_type = FuncType::new(
        vec![ValType::I32, ValType::I32], // id_ptr, id_len
        vec![ValType::I32],               // result_ptr
    );

    Func::new(
        store,
        func_type,
        move |mut caller: Caller<'_, HostContext<RT>>, params: &[Val], results: &mut [Val]| {
            let id_ptr = params[0].i32().unwrap_or(0);
            let id_len = params[1].i32().unwrap_or(0);

            // Read document ID from WASM memory
            let doc_id = match read_memory_string(&mut caller, id_ptr, id_len) {
                Ok(s) => s,
                Err(e) => {
                    let error_result = DbResult {
                        success: false,
                        data: None,
                        error: Some(format!("Failed to read document ID: {}", e)),
                    };
                    let ptr = write_json_response(&mut caller, &error_result).unwrap_or(-1);
                    results[0] = Val::I32(ptr);
                    return Ok(());
                },
            };

            // Use DatabaseClient if available
            let result = if let Some(ref client) = caller.data().database_client() {
                match client.get(doc_id) {
                    Ok(doc) => DbResult {
                        success: true,
                        data: Some(doc.unwrap_or(serde_json::Value::Null)),
                        error: None,
                    },
                    Err(e) => DbResult {
                        success: false,
                        data: None,
                        error: Some(format!("Database get failed: {}", e)),
                    },
                }
            } else {
                // No database client available - return error
                DbResult {
                    success: false,
                    data: None,
                    error: Some("Database client not available".to_string()),
                }
            };

            let ptr = write_json_response(&mut caller, &result).unwrap_or(-1);
            results[0] = Val::I32(ptr);
            Ok(())
        },
    )
}

/// Create the db.insert host function
fn create_db_insert<RT: Runtime>(store: &mut Store<HostContext<RT>>) -> Func {
    let func_type = FuncType::new(
        vec![ValType::I32, ValType::I32, ValType::I32, ValType::I32], // table_ptr, table_len, value_ptr, value_len
        vec![ValType::I32],                                          // result_ptr
    );

    Func::new(
        store,
        func_type,
        move |mut caller: Caller<'_, HostContext<RT>>, params: &[Val], results: &mut [Val]| {
            let table_ptr = params[0].i32().unwrap_or(0);
            let table_len = params[1].i32().unwrap_or(0);
            let value_ptr = params[2].i32().unwrap_or(0);
            let value_len = params[3].i32().unwrap_or(0);

            // Read table name
            let table_name = match read_memory_string(&mut caller, table_ptr, table_len) {
                Ok(s) => s,
                Err(e) => {
                    let error_result = DbResult {
                        success: false,
                        data: None,
                        error: Some(format!("Failed to read table name: {}", e)),
                    };
                    let ptr = write_json_response(&mut caller, &error_result).unwrap_or(-1);
                    results[0] = Val::I32(ptr);
                    return Ok(());
                },
            };

            // Read value JSON
            let value_json = match read_memory(&mut caller, value_ptr, value_len) {
                Ok(data) => data,
                Err(e) => {
                    let error_result = DbResult {
                        success: false,
                        data: None,
                        error: Some(format!("Failed to read value: {}", e)),
                    };
                    let ptr = write_json_response(&mut caller, &error_result).unwrap_or(-1);
                    results[0] = Val::I32(ptr);
                    return Ok(());
                },
            };

            // Parse the JSON value
            let value: serde_json::Value = match serde_json::from_slice(&value_json) {
                Ok(v) => v,
                Err(e) => {
                    let error_result = DbResult {
                        success: false,
                        data: None,
                        error: Some(format!("Failed to parse value JSON: {}", e)),
                    };
                    let ptr = write_json_response(&mut caller, &error_result).unwrap_or(-1);
                    results[0] = Val::I32(ptr);
                    return Ok(());
                }
            };

            // Use DatabaseClient if available
            let result = if let Some(ref client) = caller.data().database_client() {
                match client.insert(table_name, value) {
                    Ok(id) => DbResult {
                        success: true,
                        data: Some(serde_json::json!(id)),
                        error: None,
                    },
                    Err(e) => DbResult {
                        success: false,
                        data: None,
                        error: Some(format!("Database insert failed: {}", e)),
                    },
                }
            } else {
                // No database client available - return error
                DbResult {
                    success: false,
                    data: None,
                    error: Some("Database client not available".to_string()),
                }
            };

            let ptr = write_json_response(&mut caller, &result).unwrap_or(-1);
            results[0] = Val::I32(ptr);
            Ok(())
        },
    )
}

/// Create the db.patch host function
fn create_db_patch<RT: Runtime>(store: &mut Store<HostContext<RT>>) -> Func {
    let func_type = FuncType::new(
        vec![ValType::I32, ValType::I32, ValType::I32, ValType::I32], // id_ptr, id_len, value_ptr, value_len
        vec![ValType::I32],                                          // result_ptr
    );

    Func::new(
        store,
        func_type,
        move |mut caller: Caller<'_, HostContext<RT>>, params: &[Val], results: &mut [Val]| {
            let id_ptr = params[0].i32().unwrap_or(0);
            let id_len = params[1].i32().unwrap_or(0);
            let value_ptr = params[2].i32().unwrap_or(0);
            let value_len = params[3].i32().unwrap_or(0);

            // Read document ID
            let doc_id = match read_memory_string(&mut caller, id_ptr, id_len) {
                Ok(s) => s,
                Err(e) => {
                    let error_result = DbResult {
                        success: false,
                        data: None,
                        error: Some(format!("Failed to read document ID: {}", e)),
                    };
                    let ptr = write_json_response(&mut caller, &error_result).unwrap_or(-1);
                    results[0] = Val::I32(ptr);
                    return Ok(());
                },
            };

            // Read patch value
            let patch_json = match read_memory(&mut caller, value_ptr, value_len) {
                Ok(data) => data,
                Err(e) => {
                    let error_result = DbResult {
                        success: false,
                        data: None,
                        error: Some(format!("Failed to read patch value: {}", e)),
                    };
                    let ptr = write_json_response(&mut caller, &error_result).unwrap_or(-1);
                    results[0] = Val::I32(ptr);
                    return Ok(());
                },
            };

            // Parse the patch JSON
            let patch: serde_json::Value = match serde_json::from_slice(&patch_json) {
                Ok(v) => v,
                Err(e) => {
                    let error_result = DbResult {
                        success: false,
                        data: None,
                        error: Some(format!("Failed to parse patch JSON: {}", e)),
                    };
                    let ptr = write_json_response(&mut caller, &error_result).unwrap_or(-1);
                    results[0] = Val::I32(ptr);
                    return Ok(());
                },
            };

            // Use DatabaseClient if available
            let result = if let Some(ref client) = caller.data().database_client() {
                match client.patch(doc_id, patch) {
                    Ok(()) => DbResult {
                        success: true,
                        data: None,
                        error: None,
                    },
                    Err(e) => DbResult {
                        success: false,
                        data: None,
                        error: Some(format!("Database patch failed: {}", e)),
                    },
                }
            } else {
                // No database client available - return error
                DbResult {
                    success: false,
                    data: None,
                    error: Some("Database client not available".to_string()),
                }
            };

            let ptr = write_json_response(&mut caller, &result).unwrap_or(-1);
            results[0] = Val::I32(ptr);
            Ok(())
        },
    )
}

/// Create the db.delete host function
fn create_db_delete<RT: Runtime>(store: &mut Store<HostContext<RT>>) -> Func {
    let func_type = FuncType::new(
        vec![ValType::I32, ValType::I32], // id_ptr, id_len
        vec![ValType::I32],               // result_ptr
    );

    Func::new(
        store,
        func_type,
        move |mut caller: Caller<'_, HostContext<RT>>, params: &[Val], results: &mut [Val]| {
            let id_ptr = params[0].i32().unwrap_or(0);
            let id_len = params[1].i32().unwrap_or(0);

            // Read document ID
            let doc_id = match read_memory_string(&mut caller, id_ptr, id_len) {
                Ok(s) => s,
                Err(e) => {
                    let error_result = DbResult {
                        success: false,
                        data: None,
                        error: Some(format!("Failed to read document ID: {}", e)),
                    };
                    let ptr = write_json_response(&mut caller, &error_result).unwrap_or(-1);
                    results[0] = Val::I32(ptr);
                    return Ok(());
                },
            };

            // Use DatabaseClient if available
            let result = if let Some(ref client) = caller.data().database_client() {
                match client.delete(doc_id) {
                    Ok(()) => DbResult {
                        success: true,
                        data: None,
                        error: None,
                    },
                    Err(e) => DbResult {
                        success: false,
                        data: None,
                        error: Some(format!("Database delete failed: {}", e)),
                    },
                }
            } else {
                // No database client available - return error
                DbResult {
                    success: false,
                    data: None,
                    error: Some("Database client not available".to_string()),
                }
            };

            let ptr = write_json_response(&mut caller, &result).unwrap_or(-1);
            results[0] = Val::I32(ptr);
            Ok(())
        },
    )
}

/// Create the db.query_advanced host function
fn create_db_query_advanced<RT: Runtime>(store: &mut Store<HostContext<RT>>) -> Func {
    let func_type = FuncType::new(
        vec![ValType::I32, ValType::I32], // query_ptr, query_len
        vec![ValType::I32],               // result_ptr
    );

    Func::new(
        store,
        func_type,
        move |mut caller: Caller<'_, HostContext<RT>>, params: &[Val], results: &mut [Val]| {
            let query_ptr = params[0].i32().unwrap_or(0);
            let query_len = params[1].i32().unwrap_or(0);

            // Read query JSON
            let query_bytes = match read_memory(&mut caller, query_ptr, query_len) {
                Ok(data) => data,
                Err(e) => {
                    let error_result = DbResult {
                        success: false,
                        data: None,
                        error: Some(format!("Failed to read query: {}", e)),
                    };
                    let ptr = write_json_response(&mut caller, &error_result).unwrap_or(-1);
                    results[0] = Val::I32(ptr);
                    return Ok(());
                },
            };

            // Parse query request
            let _query_request: DbQueryAdvancedRequest = match serde_json::from_slice(&query_bytes)
            {
                Ok(q) => q,
                Err(e) => {
                    let error_result = DbResult {
                        success: false,
                        data: None,
                        error: Some(format!("Invalid query JSON: {}", e)),
                    };
                    let ptr = write_json_response(&mut caller, &error_result).unwrap_or(-1);
                    results[0] = Val::I32(ptr);
                    return Ok(());
                },
            };

            // TODO: Integrate with actual Convex database backend
            // For now, return empty array
            let result = DbResult {
                success: true,
                data: Some(serde_json::json!([])),
                error: None,
            };

            let ptr = write_json_response(&mut caller, &result).unwrap_or(-1);
            results[0] = Val::I32(ptr);
            Ok(())
        },
    )
}

/// Create the db.count host function
fn create_db_count<RT: Runtime>(store: &mut Store<HostContext<RT>>) -> Func {
    let func_type = FuncType::new(
        vec![ValType::I32, ValType::I32], // table_ptr, table_len
        vec![ValType::I32],               // result_ptr
    );

    Func::new(
        store,
        func_type,
        move |mut caller: Caller<'_, HostContext<RT>>, params: &[Val], results: &mut [Val]| {
            let table_ptr = params[0].i32().unwrap_or(0);
            let table_len = params[1].i32().unwrap_or(0);

            // Read table name
            let table_name = match read_memory_string(&mut caller, table_ptr, table_len) {
                Ok(s) => s,
                Err(e) => {
                    let error_result = DbResult {
                        success: false,
                        data: None,
                        error: Some(format!("Failed to read table name: {}", e)),
                    };
                    let ptr = write_json_response(&mut caller, &error_result).unwrap_or(-1);
                    results[0] = Val::I32(ptr);
                    return Ok(());
                },
            };

            // Use DatabaseClient if available
            let result = if let Some(ref client) = caller.data().database_client() {
                match client.count(table_name) {
                    Ok(count) => DbResult {
                        success: true,
                        data: Some(serde_json::json!(count)),
                        error: None,
                    },
                    Err(e) => DbResult {
                        success: false,
                        data: None,
                        error: Some(format!("Database count failed: {}", e)),
                    },
                }
            } else {
                // No database client available - return error
                DbResult {
                    success: false,
                    data: None,
                    error: Some("Database client not available".to_string()),
                }
            };

            let ptr = write_json_response(&mut caller, &result).unwrap_or(-1);
            results[0] = Val::I32(ptr);
            Ok(())
        },
    )
}

/// Read data from WASM memory
fn read_memory<RT: Runtime>(
    caller: &mut Caller<'_, HostContext<RT>>,
    ptr: i32,
    len: i32,
) -> anyhow::Result<Vec<u8>> {
    if ptr < 0 || len < 0 {
        anyhow::bail!("Invalid memory pointer or length");
    }

    let ptr = ptr as usize;
    let len = len as usize;

    let memory = caller
        .get_export("memory")
        .and_then(|e| e.into_memory())
        .context("Memory export not found")?;

    let mut data = vec![0u8; len];
    memory.read(caller, ptr, &mut data)?;

    Ok(data)
}

/// Read a string from WASM memory
fn read_memory_string<RT: Runtime>(
    caller: &mut Caller<'_, HostContext<RT>>,
    ptr: i32,
    len: i32,
) -> anyhow::Result<String> {
    let bytes = read_memory(caller, ptr, len)?;
    String::from_utf8(bytes).context("Invalid UTF-8 string")
}

/// Write JSON response to WASM memory
fn write_json_response<RT: Runtime, T: Serialize>(
    caller: &mut Caller<'_, HostContext<RT>>,
    response: &T,
) -> anyhow::Result<i32> {
    let json = serde_json::to_vec(response)?;

    // Get alloc function from WASM module
    let alloc_func = caller
        .get_export("__convex_alloc")
        .and_then(|e| e.into_func())
        .context("alloc function not found")?;

    let typed_alloc = alloc_func.typed::<i32, i32>(&mut *caller)?;
    let total_len = 4 + json.len();
    let ptr = typed_alloc.call(&mut *caller, total_len as i32)?;

    if ptr == 0 {
        anyhow::bail!("Memory allocation failed");
    }

    // Write length prefix (little-endian u32)
    let memory = caller
        .get_export("memory")
        .and_then(|e| e.into_memory())
        .context("Memory export not found")?;

    let len_bytes = (json.len() as u32).to_le_bytes();
    memory.write(&mut *caller, ptr as usize, &len_bytes)?;

    // Write JSON data after length prefix
    memory.write(&mut *caller, ptr as usize + 4, &json)?;

    Ok(ptr)
}

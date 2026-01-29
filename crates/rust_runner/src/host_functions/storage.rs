//! Storage host functions for WASM guest
//!
//! These functions provide file storage capabilities to Rust/WASM functions.
//! Storage operations are available in all function types (queries, mutations, actions).

use anyhow::Context;
use common::runtime::Runtime;
use serde::{Deserialize, Serialize};
use wasmtime::{Caller, Func, FuncType, Store, Val, ValType};

use crate::host_functions::{write_json_response, HostContext};

/// Maximum size for storage upload (100 MB)
const MAX_STORAGE_SIZE: usize = 100 * 1024 * 1024;

/// Storage client trait for file operations
///
/// This trait abstracts over the actual storage backend (S3, GCS, etc.)
pub trait StorageClient: Send + Sync {
    /// Store a file and return a storage ID
    fn store(&self, content_type: String, data: Vec<u8>) -> anyhow::Result<String>;

    /// Retrieve a file by storage ID
    fn get(&self, storage_id: String) -> anyhow::Result<Option<StorageFile>>;
}

/// Storage file metadata and content
#[derive(Debug, Clone)]
pub struct StorageFile {
    pub content_type: String,
    pub data: Vec<u8>,
}

/// Storage request for storing a file
#[derive(Debug, Serialize, Deserialize)]
struct StorageStoreRequest {
    content_type: String,
    data: Vec<u8>,
}

/// Storage response with storage ID
#[derive(Debug, Serialize, Deserialize)]
struct StorageStoreResponse {
    storage_id: String,
}

/// Storage get request
#[derive(Debug, Serialize, Deserialize)]
struct StorageGetRequest {
    storage_id: String,
}

/// Storage get response
#[derive(Debug, Serialize, Deserialize)]
struct StorageGetResponse {
    content_type: String,
    data: Vec<u8>,
}

/// Storage operation result
#[derive(Debug, Serialize, Deserialize)]
struct StorageResult {
    success: bool,
    data: Option<serde_json::Value>,
    error: Option<String>,
}

/// Create all storage-related host functions
pub fn create_storage_functions<RT: Runtime>(
    store: &mut Store<HostContext<RT>>,
) -> Vec<(String, Func)> {
    vec![
        (
            "__convex_storage_store".to_string(),
            create_storage_store(store),
        ),
        ("__convex_storage_get".to_string(), create_storage_get(store)),
    ]
}

/// Create the storage.store host function
///
/// Stores a file and returns a storage ID
fn create_storage_store<RT: Runtime>(store: &mut Store<HostContext<RT>>) -> Func {
    let func_type = FuncType::new(
        vec![ValType::I32, ValType::I32], // request_ptr, request_len
        vec![ValType::I32],               // result_ptr
    );

    Func::new(
        store,
        func_type,
        move |mut caller: Caller<'_, HostContext<RT>>, params: &[Val], results: &mut [Val]| {
            let request_ptr = params[0].i32().unwrap_or(0);
            let request_len = params[1].i32().unwrap_or(0);

            // Read request from WASM memory
            let request_bytes = match read_memory(&mut caller, request_ptr, request_len) {
                Ok(data) => data,
                Err(e) => {
                    let error_result = StorageResult {
                        success: false,
                        data: None,
                        error: Some(format!("Failed to read request: {}", e)),
                    };
                    let ptr = write_json_response(&mut caller, &error_result).unwrap_or(-1);
                    results[0] = Val::I32(ptr);
                    return Ok(());
                },
            };

            // Parse storage request
            let store_request: StorageStoreRequest = match serde_json::from_slice(&request_bytes) {
                Ok(req) => req,
                Err(e) => {
                    let error_result = StorageResult {
                        success: false,
                        data: None,
                        error: Some(format!("Invalid request JSON: {}", e)),
                    };
                    let ptr = write_json_response(&mut caller, &error_result).unwrap_or(-1);
                    results[0] = Val::I32(ptr);
                    return Ok(());
                },
            };

            // Validate size
            if store_request.data.len() > MAX_STORAGE_SIZE {
                let error_result = StorageResult {
                    success: false,
                    data: None,
                    error: Some(format!(
                        "Storage size {} exceeds maximum of {}",
                        store_request.data.len(),
                        MAX_STORAGE_SIZE
                    )),
                };
                let ptr = write_json_response(&mut caller, &error_result).unwrap_or(-1);
                results[0] = Val::I32(ptr);
                return Ok(());
            }

            // Get storage client from context
            let storage_client = match caller.data().storage_client() {
                Some(client) => client,
                None => {
                    // No storage client available - return an error
                    let error_result = StorageResult {
                        success: false,
                        data: None,
                        error: Some("Storage service not available".to_string()),
                    };
                    let ptr = write_json_response(&mut caller, &error_result).unwrap_or(-1);
                    results[0] = Val::I32(ptr);
                    return Ok(());
                },
            };

            // Store the file
            match storage_client.store(store_request.content_type, store_request.data) {
                Ok(storage_id) => {
                    let response = StorageStoreResponse { storage_id };
                    let result = StorageResult {
                        success: true,
                        data: Some(serde_json::to_value(&response).unwrap_or_default()),
                        error: None,
                    };
                    let ptr = write_json_response(&mut caller, &result).unwrap_or(-1);
                    results[0] = Val::I32(ptr);
                },
                Err(e) => {
                    let error_result = StorageResult {
                        success: false,
                        data: None,
                        error: Some(format!("Storage failed: {}", e)),
                    };
                    let ptr = write_json_response(&mut caller, &error_result).unwrap_or(-1);
                    results[0] = Val::I32(ptr);
                },
            }

            Ok(())
        },
    )
}

/// Create the storage.get host function
///
/// Retrieves a file by storage ID
fn create_storage_get<RT: Runtime>(store: &mut Store<HostContext<RT>>) -> Func {
    let func_type = FuncType::new(
        vec![ValType::I32, ValType::I32], // request_ptr, request_len
        vec![ValType::I32],               // result_ptr
    );

    Func::new(
        store,
        func_type,
        move |mut caller: Caller<'_, HostContext<RT>>, params: &[Val], results: &mut [Val]| {
            let request_ptr = params[0].i32().unwrap_or(0);
            let request_len = params[1].i32().unwrap_or(0);

            // Read request from WASM memory
            let request_bytes = match read_memory(&mut caller, request_ptr, request_len) {
                Ok(data) => data,
                Err(e) => {
                    let error_result = StorageResult {
                        success: false,
                        data: None,
                        error: Some(format!("Failed to read request: {}", e)),
                    };
                    let ptr = write_json_response(&mut caller, &error_result).unwrap_or(-1);
                    results[0] = Val::I32(ptr);
                    return Ok(());
                },
            };

            // Parse get request
            let get_request: StorageGetRequest = match serde_json::from_slice(&request_bytes) {
                Ok(req) => req,
                Err(e) => {
                    let error_result = StorageResult {
                        success: false,
                        data: None,
                        error: Some(format!("Invalid request JSON: {}", e)),
                    };
                    let ptr = write_json_response(&mut caller, &error_result).unwrap_or(-1);
                    results[0] = Val::I32(ptr);
                    return Ok(());
                },
            };

            // Get storage client from context
            let storage_client = match caller.data().storage_client() {
                Some(client) => client,
                None => {
                    // No storage client available - return not found
                    let error_result = StorageResult {
                        success: false,
                        data: None,
                        error: Some(format!(
                            "Storage ID '{}' not found",
                            get_request.storage_id
                        )),
                    };
                    let ptr = write_json_response(&mut caller, &error_result).unwrap_or(-1);
                    results[0] = Val::I32(ptr);
                    return Ok(());
                },
            };

            // Retrieve the file
            match storage_client.get(get_request.storage_id) {
                Ok(Some(file)) => {
                    let response = StorageGetResponse {
                        content_type: file.content_type,
                        data: file.data,
                    };
                    let result = StorageResult {
                        success: true,
                        data: Some(serde_json::to_value(&response).unwrap_or_default()),
                        error: None,
                    };
                    let ptr = write_json_response(&mut caller, &result).unwrap_or(-1);
                    results[0] = Val::I32(ptr);
                },
                Ok(None) => {
                    let error_result = StorageResult {
                        success: false,
                        data: None,
                        error: Some("Storage file not found".to_string()),
                    };
                    let ptr = write_json_response(&mut caller, &error_result).unwrap_or(-1);
                    results[0] = Val::I32(ptr);
                },
                Err(e) => {
                    let error_result = StorageResult {
                        success: false,
                        data: None,
                        error: Some(format!("Storage retrieval failed: {}", e)),
                    };
                    let ptr = write_json_response(&mut caller, &error_result).unwrap_or(-1);
                    results[0] = Val::I32(ptr);
                },
            }

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

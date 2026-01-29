//! HTTP host functions for WASM guest
//!
//! These functions provide HTTP fetch capabilities to Rust/WASM functions.
//! HTTP operations are only available in actions (not queries/mutations)
//! for security and determinism reasons.

use std::pin::Pin;
use std::sync::Arc;

use anyhow::Context;
use bytes::Bytes;
use common::http::fetch::FetchClient;
use common::http::{HttpRequestStream, HttpResponseStream};
use common::runtime::Runtime;
use common::types::UdfType;
use futures::stream;
use http::{HeaderMap, Method};
use serde::{Deserialize, Serialize};
use wasmtime::{Caller, Func, FuncType, Val, ValType};

use crate::host_functions::HostContext;

/// Maximum size for HTTP request body (10 MB)
const MAX_REQUEST_BODY_SIZE: usize = 10 * 1024 * 1024;

/// Maximum size for HTTP response body (10 MB)
const MAX_RESPONSE_BODY_SIZE: usize = 10 * 1024 * 1024;

/// HTTP fetch timeout in seconds
const HTTP_FETCH_TIMEOUT_SECS: u64 = 300; // 5 minutes

/// HTTP request options from WASM guest
#[derive(Debug, Serialize, Deserialize)]
struct HttpFetchRequest {
    url: String,
    method: String,
    headers: Vec<(String, String)>,
    body: Option<Vec<u8>>,
}

/// HTTP response to WASM guest
#[derive(Debug, Serialize, Deserialize)]
struct HttpFetchResponse {
    status: u16,
    headers: Vec<(String, String)>,
    body: Vec<u8>,
}

/// Error response to WASM guest
#[derive(Debug, Serialize, Deserialize)]
struct HttpFetchError {
    message: String,
    code: String,
}

/// Result type for HTTP fetch
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
enum HttpFetchResult {
    Success(HttpFetchResponse),
    Error(HttpFetchError),
}

/// Create all HTTP-related host functions
pub fn create_http_functions<RT: Runtime>(
    store: &mut wasmtime::Store<HostContext<RT>>,
) -> Vec<(String, Func)> {
    vec![(
        "__convex_http_fetch".to_string(),
        create_http_fetch(store),
    )]
}

/// Create the HTTP fetch host function
///
/// Parameters (from WASM):
/// - i32: pointer to request JSON in WASM memory
/// - i32: length of request JSON
///
/// Returns (to WASM):
/// - i32: pointer to response JSON in WASM memory (caller must free)
fn create_http_fetch<RT: Runtime>(store: &mut wasmtime::Store<HostContext<RT>>) -> Func {
    let func_type = FuncType::new(
        vec![ValType::I32, ValType::I32], // request_ptr, request_len
        vec![ValType::I32],               // result_ptr
    );

    Func::new(
        store,
        func_type,
        move |mut caller: Caller<'_, HostContext<RT>>, params: &[Val], results: &mut [Val]| {
            // Extract parameters
            let request_ptr = params[0].i32().context("request_ptr must be i32")?;
            let request_len = params[1].i32().context("request_len must be i32")?;

            // Validate this is an action (not query/mutation)
            let udf_type = caller.data().udf_type();
            if !matches!(udf_type, UdfType::Action | UdfType::HttpAction) {
                let error_result = HttpFetchResult::Error(HttpFetchError {
                    message: "HTTP fetch is only available in actions".to_string(),
                    code: "HttpFetchNotAllowed".to_string(),
                });
                let response_ptr =
                    write_json_response(&mut caller, &error_result).unwrap_or(-1);
                results[0] = Val::I32(response_ptr);
                return Ok(());
            }

            // Read request from WASM memory
            let request_json = match read_memory(&mut caller, request_ptr, request_len) {
                Ok(data) => data,
                Err(e) => {
                    let error_result = HttpFetchResult::Error(HttpFetchError {
                        message: format!("Failed to read request: {}", e),
                        code: "MemoryReadError".to_string(),
                    });
                    let response_ptr =
                        write_json_response(&mut caller, &error_result).unwrap_or(-1);
                    results[0] = Val::I32(response_ptr);
                    return Ok(());
                },
            };

            // Parse request
            let fetch_request: HttpFetchRequest = match serde_json::from_slice(&request_json) {
                Ok(req) => req,
                Err(e) => {
                    let error_result = HttpFetchResult::Error(HttpFetchError {
                        message: format!("Invalid request JSON: {}", e),
                        code: "InvalidRequest".to_string(),
                    });
                    let response_ptr =
                        write_json_response(&mut caller, &error_result).unwrap_or(-1);
                    results[0] = Val::I32(response_ptr);
                    return Ok(());
                },
            };

            // Validate body size
            if let Some(ref body) = fetch_request.body {
                if body.len() > MAX_REQUEST_BODY_SIZE {
                    let error_result = HttpFetchResult::Error(HttpFetchError {
                        message: format!(
                            "Request body too large: {} > {}",
                            body.len(),
                            MAX_REQUEST_BODY_SIZE
                        ),
                        code: "RequestBodyTooLarge".to_string(),
                    });
                    let response_ptr =
                        write_json_response(&mut caller, &error_result).unwrap_or(-1);
                    results[0] = Val::I32(response_ptr);
                    return Ok(());
                }
            }

            // Get fetch client from context
            let fetch_client = match caller.data().fetch_client() {
                Some(client) => client.clone(),
                None => {
                    let error_result = HttpFetchResult::Error(HttpFetchError {
                        message: "HTTP client not available".to_string(),
                        code: "HttpClientNotAvailable".to_string(),
                    });
                    let response_ptr =
                        write_json_response(&mut caller, &error_result).unwrap_or(-1);
                    results[0] = Val::I32(response_ptr);
                    return Ok(());
                },
            };

            // Build and execute HTTP request synchronously (blocking WASM execution)
            // Note: In a real implementation, this should be async. For now, we use
            // block_on which is acceptable for MVP but should be refactored to use
            // async host functions properly.
            let result = execute_http_fetch::<RT>(fetch_client, fetch_request);

            // Write response to WASM memory
            match write_json_response(&mut caller, &result) {
                Ok(ptr) => {
                    results[0] = Val::I32(ptr);
                },
                Err(e) => {
                    let error_result = HttpFetchResult::Error(HttpFetchError {
                        message: format!("Failed to write response: {}", e),
                        code: "MemoryWriteError".to_string(),
                    });
                    let ptr = write_json_response(&mut caller, &error_result).unwrap_or(-1);
                    results[0] = Val::I32(ptr);
                },
            }

            Ok(())
        },
    )
}

/// Execute HTTP fetch request
fn execute_http_fetch<RT: Runtime>(
    fetch_client: Arc<dyn FetchClient>,
    request: HttpFetchRequest,
) -> HttpFetchResult {
    // Parse URL
    let url = match request.url.parse() {
        Ok(url) => url,
        Err(e) => {
            return HttpFetchResult::Error(HttpFetchError {
                message: format!("Invalid URL: {}", e),
                code: "InvalidUrl".to_string(),
            });
        },
    };

    // Parse method
    let method = match Method::from_bytes(request.method.as_bytes()) {
        Ok(m) => m,
        Err(e) => {
            return HttpFetchResult::Error(HttpFetchError {
                message: format!("Invalid HTTP method: {}", e),
                code: "InvalidMethod".to_string(),
            });
        },
    };

    // Build headers
    let mut headers = HeaderMap::new();
    for (name, value) in request.headers {
        let header_name = match http::header::HeaderName::from_bytes(name.as_bytes()) {
            Ok(n) => n,
            Err(e) => {
                return HttpFetchResult::Error(HttpFetchError {
                    message: format!("Invalid header name '{}': {}", name, e),
                    code: "InvalidHeader".to_string(),
                });
            },
        };
        let header_value = match http::header::HeaderValue::from_str(&value) {
            Ok(v) => v,
            Err(e) => {
                return HttpFetchResult::Error(HttpFetchError {
                    message: format!("Invalid header value for '{}': {}", name, e),
                    code: "InvalidHeader".to_string(),
                });
            },
        };
        headers.insert(header_name, header_value);
    }

    // Convert body to Bytes and create stream
    let body_stream: Pin<Box<dyn futures::Stream<Item = Result<Bytes, anyhow::Error>> + Send + Sync>> =
        match request.body {
            Some(body) => Box::pin(stream::once(async move { Ok::<_, anyhow::Error>(Bytes::from(body)) })),
            None => Box::pin(stream::empty()),
        };

    let request_stream = HttpRequestStream {
        headers,
        url,
        method,
        body: body_stream,
        signal: Box::pin(futures::future::pending()), // No cancellation for now
    };

    // Execute request with timeout
    let runtime = match tokio::runtime::Handle::try_current() {
        Ok(handle) => handle,
        Err(e) => {
            return HttpFetchResult::Error(HttpFetchError {
                message: format!("No async runtime available: {}", e),
                code: "RuntimeError".to_string(),
            });
        },
    };

    let result: Result<HttpResponseStream, anyhow::Error> = runtime.block_on(async {
        tokio::time::timeout(
            tokio::time::Duration::from_secs(HTTP_FETCH_TIMEOUT_SECS),
            fetch_client.fetch(request_stream),
        )
        .await
        .map_err(|_| anyhow::anyhow!("HTTP request timed out"))?
    });

    let response = match result {
        Ok(r) => r,
        Err(e) => {
            return HttpFetchResult::Error(HttpFetchError {
                message: format!("HTTP request failed: {}", e),
                code: "FetchFailed".to_string(),
            });
        },
    };

    // Collect response body
    let status = response.status.as_u16();
    let headers: Vec<(String, String)> = response
        .headers
        .iter()
        .filter_map(|(k, v)| {
            let key = k.to_string();
            let value = v.to_str().ok()?.to_string();
            Some((key, value))
        })
        .collect();

    // Collect body with size limit
    let body = match runtime.block_on(collect_body_with_limit(response, MAX_RESPONSE_BODY_SIZE)) {
        Ok(b) => b,
        Err(e) => {
            return HttpFetchResult::Error(HttpFetchError {
                message: format!("Failed to read response body: {}", e),
                code: "BodyReadError".to_string(),
            });
        },
    };

    HttpFetchResult::Success(HttpFetchResponse {
        status,
        headers,
        body,
    })
}

/// Collect response body with size limit
async fn collect_body_with_limit(
    response: HttpResponseStream,
    max_size: usize,
) -> anyhow::Result<Vec<u8>> {
    let mut body = Vec::new();

    if let Some(mut stream) = response.body {
        use futures::StreamExt;
        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;
            if body.len() + chunk.len() > max_size {
                anyhow::bail!("Response body exceeds maximum size of {} bytes", max_size);
            }
            body.extend_from_slice(&chunk);
        }
    }

    Ok(body)
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

    // Get memory export
    let memory = caller
        .get_export("memory")
        .and_then(|e| e.into_memory())
        .context("Memory export not found")?;

    // Read data
    let mut data = vec![0u8; len];
    memory.read(caller, ptr, &mut data)?;

    Ok(data)
}

/// Write JSON response to WASM memory
/// Returns pointer to allocated memory (first 4 bytes contain length as little-endian u32)
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

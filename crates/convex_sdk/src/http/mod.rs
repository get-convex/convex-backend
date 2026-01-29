//! HTTP client for Convex actions

pub mod router;

use crate::types::{ConvexError, Result};
pub use router::{HttpRouter, RouteSpec, Method, Request, Response, HttpContext};
use alloc::string::String;
use alloc::vec::Vec;
use serde::{Deserialize, Serialize};

/// HTTP response from a fetch call
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpResponse {
    /// HTTP status code
    pub status: u16,
    /// Response headers as key-value pairs
    pub headers: Vec<(String, String)>,
    /// Response body as raw bytes
    pub body: Vec<u8>,
}

/// Request payload for the HTTP fetch host function
#[derive(Debug, Serialize)]
struct FetchRequest<'a> {
    url: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    method: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    headers: Option<&'a [(String, String)]>,
    #[serde(skip_serializing_if = "Option::is_none")]
    body: Option<&'a [u8]>,
}

/// Response payload from the HTTP fetch host function
#[derive(Debug, Deserialize)]
struct FetchResponse {
    status: u16,
    headers: Vec<(String, String)>,
    #[serde(with = "serde_bytes")]
    body: Vec<u8>,
}

// Host functions provided by the Convex runtime
extern "C" {
    /// Fetch a URL via the host
    /// Returns a pointer to the response JSON in WASM memory
    fn __convex_http_fetch(url_ptr: i32, url_len: i32, options_ptr: i32, options_len: i32) -> i32;

    /// Allocate memory in the WASM linear memory
    /// Returns a pointer to the allocated memory
    fn __convex_alloc(size: i32) -> i32;

    /// Free memory in the WASM linear memory
    fn __convex_free(ptr: i32);
}

/// Fetch a URL (actions only)
///
/// # Arguments
///
/// * `url` - The URL to fetch
/// * `options` - Options for the fetch request
///
/// # Returns
///
/// The HTTP response from the fetch
///
/// # Errors
///
/// Returns an error if the request fails or the response cannot be parsed
pub async fn fetch(url: &str, options: FetchOptions) -> Result<HttpResponse> {
    // Serialize URL and options to JSON
    let request = FetchRequest {
        url,
        method: options.method.as_deref(),
        headers: options.headers.as_deref(),
        body: options.body.as_deref(),
    };

    let request_json = serde_json::to_vec(&request)?;

    // Allocate WASM memory for the request
    let ptr = unsafe { __convex_alloc(request_json.len() as i32) };
    if ptr == 0 {
        return Err(ConvexError::Unknown(
            "Failed to allocate WASM memory".into(),
        ));
    }

    // Write request to memory
    unsafe {
        let slice = core::slice::from_raw_parts_mut(ptr as *mut u8, request_json.len());
        slice.copy_from_slice(&request_json);
    }

    // Call host function
    let result_ptr = unsafe {
        __convex_http_fetch(ptr, request_json.len() as i32, 0, 0)
    };

    // Free the request memory
    unsafe {
        __convex_free(ptr);
    }

    if result_ptr == 0 {
        return Err(ConvexError::Unknown(
            "HTTP fetch host function failed".into(),
        ));
    }

    // Read response from memory
    // The first 4 bytes contain the length of the response
    let response_len = unsafe {
        core::ptr::read_unaligned(result_ptr as *const i32) as usize
    };

    let response_data = unsafe {
        let slice = core::slice::from_raw_parts(
            (result_ptr + 4) as *const u8,
            response_len,
        );
        Vec::from(slice)
    };

    // Free the response memory
    unsafe {
        __convex_free(result_ptr);
    }

    // Deserialize the response
    let fetch_response: FetchResponse = serde_json::from_slice(&response_data)?;

    Ok(HttpResponse {
        status: fetch_response.status,
        headers: fetch_response.headers,
        body: fetch_response.body,
    })
}

/// Options for HTTP fetch
#[derive(Debug, Default, Clone)]
pub struct FetchOptions {
    /// HTTP method (GET, POST, PUT, DELETE, etc.)
    pub method: Option<String>,
    /// HTTP headers as key-value pairs
    pub headers: Option<Vec<(String, String)>>,
    /// Request body as raw bytes
    pub body: Option<Vec<u8>>,
}

impl FetchOptions {
    /// Create a new `FetchOptions` with default values
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the HTTP method
    ///
    /// # Arguments
    ///
    /// * `method` - The HTTP method (e.g., "GET", "POST", "PUT", "DELETE")
    ///
    /// # Examples
    ///
    /// ```
    /// use convex_sdk::FetchOptions;
    ///
    /// let options = FetchOptions::new().method("POST");
    /// ```
    pub fn method(mut self, method: impl Into<String>) -> Self {
        self.method = Some(method.into());
        self
    }

    /// Add an HTTP header
    ///
    /// # Arguments
    ///
    /// * `key` - The header name
    /// * `value` - The header value
    ///
    /// # Examples
    ///
    /// ```
    /// use convex_sdk::FetchOptions;
    ///
    /// let options = FetchOptions::new()
    ///     .header("Content-Type", "application/json")
    ///     .header("Authorization", "Bearer token");
    /// ```
    pub fn header(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        if self.headers.is_none() {
            self.headers = Some(Vec::new());
        }
        self.headers.as_mut().unwrap().push((key.into(), value.into()));
        self
    }

    /// Set the request body
    ///
    /// # Arguments
    ///
    /// * `body` - The request body as raw bytes
    ///
    /// # Examples
    ///
    /// ```
    /// use convex_sdk::FetchOptions;
    ///
    /// let options = FetchOptions::new()
    ///     .method("POST")
    ///     .body(b"{\"key\": \"value\"}".to_vec());
    /// ```
    pub fn body(mut self, body: Vec<u8>) -> Self {
        self.body = Some(body);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fetch_options_builder() {
        let options = FetchOptions::new()
            .method("POST")
            .header("Content-Type", "application/json")
            .header("Authorization", "Bearer token")
            .body(b"test body".to_vec());

        assert_eq!(options.method, Some("POST".to_string()));
        assert_eq!(options.headers.as_ref().unwrap().len(), 2);
        assert_eq!(options.body, Some(b"test body".to_vec()));
    }

    #[test]
    fn test_fetch_options_default() {
        let options = FetchOptions::new();
        assert!(options.method.is_none());
        assert!(options.headers.is_none());
        assert!(options.body.is_none());
    }
}

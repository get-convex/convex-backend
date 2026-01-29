//! File storage operations for Convex

use crate::types::{ConvexError, Result};
use alloc::string::String;
use alloc::vec::Vec;
use serde::{Deserialize, Serialize};

/// Storage ID for a stored file
///
/// This is a validated identifier that can be used to retrieve files
/// from Convex storage.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct StorageId(String);

impl StorageId {
    /// Create a new `StorageId` from a string
    ///
    /// # Arguments
    ///
    /// * `id` - The storage ID string
    ///
    /// # Examples
    ///
    /// ```
    /// use convex_sdk::StorageId;
    ///
    /// let id = StorageId::new("storage_id_123");
    /// ```
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    /// Get the storage ID as a string slice
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Validate that the storage ID is well-formed
    ///
    /// # Returns
    ///
    /// `true` if the ID is valid, `false` otherwise
    pub fn is_valid(&self) -> bool {
        // Storage IDs must be non-empty and contain only alphanumeric characters,
        // hyphens, and underscores
        !self.0.is_empty()
            && self
                .0
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    }

    /// Convert the storage ID to a string representation
    pub fn to_string(&self) -> String {
        self.0.clone()
    }
}

impl From<String> for StorageId {
    fn from(s: String) -> Self {
        Self::new(s)
    }
}

impl From<&str> for StorageId {
    fn from(s: &str) -> Self {
        Self::new(s)
    }
}

/// A stored file with its content type and data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageFile {
    /// The MIME content type of the file (e.g., "image/png", "application/pdf")
    pub content_type: String,
    /// The raw file data as bytes
    pub data: Vec<u8>,
}

impl StorageFile {
    /// Create a new `StorageFile`
    ///
    /// # Arguments
    ///
    /// * `content_type` - The MIME content type
    /// * `data` - The file data
    pub fn new(content_type: impl Into<String>, data: Vec<u8>) -> Self {
        Self {
            content_type: content_type.into(),
            data,
        }
    }

    /// Get the size of the file in bytes
    pub fn len(&self) -> usize {
        self.data.len()
    }

    /// Check if the file is empty
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// Get the content type
    pub fn content_type(&self) -> &str {
        &self.content_type
    }

    /// Get the file data
    pub fn data(&self) -> &[u8] {
        &self.data
    }

    /// Convert the file data to a string if it's valid UTF-8
    ///
    /// # Returns
    ///
    /// `Some(String)` if the data is valid UTF-8, `None` otherwise
    pub fn data_as_string(&self) -> Option<String> {
        String::from_utf8(self.data.clone()).ok()
    }
}

/// Request payload for storing a file
#[derive(Debug, Serialize)]
struct StoreRequest<'a> {
    content_type: &'a str,
    #[serde(with = "serde_bytes")]
    data: &'a [u8],
}

/// Response payload from storing a file
#[derive(Debug, Deserialize)]
struct StoreResponse {
    storage_id: String,
}

/// Response payload from getting a file
#[derive(Debug, Deserialize)]
struct GetResponse {
    content_type: String,
    #[serde(with = "serde_bytes")]
    data: Vec<u8>,
}

/// Metadata for a stored file
#[derive(Debug, Clone, Deserialize)]
pub struct StorageMetadata {
    /// The MIME content type of the file
    pub content_type: String,
    /// The size of the file in bytes
    pub size: u64,
    /// When the file was created (Unix timestamp in milliseconds)
    pub created_at: i64,
    /// SHA-256 checksum of the file content (hex-encoded)
    pub sha256: String,
}

impl StorageMetadata {
    /// Get the content type
    pub fn content_type(&self) -> &str {
        &self.content_type
    }

    /// Get the file size in bytes
    pub fn size(&self) -> u64 {
        self.size
    }

    /// Get the creation timestamp
    pub fn created_at(&self) -> i64 {
        self.created_at
    }

    /// Get the SHA-256 checksum
    pub fn sha256(&self) -> &str {
        &self.sha256
    }

    /// Format the file size as a human-readable string
    pub fn format_size(&self) -> String {
        let size = self.size as f64;
        const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];

        if size == 0.0 {
            return "0 B".to_string();
        }

        let exp = (size.ln() / 1024.0_f64.ln()).min(UNITS.len() as f64 - 1.0) as usize;
        let size = size / 1024.0_f64.powi(exp as i32);

        format!("{:.2} {}", size, UNITS[exp])
    }
}

/// Options for generating a storage URL
#[derive(Debug, Clone)]
pub struct UrlOptions {
    /// How long the URL should be valid (in seconds)
    /// Default: 1 hour (3600 seconds)
    pub expires_in: u32,
    /// Whether to force download (Content-Disposition: attachment)
    pub download: bool,
    /// Optional filename for download
    pub filename: Option<String>,
}

impl Default for UrlOptions {
    fn default() -> Self {
        Self {
            expires_in: 3600,
            download: false,
            filename: None,
        }
    }
}

impl UrlOptions {
    /// Create default URL options
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the expiration time
    pub fn expires_in(mut self, seconds: u32) -> Self {
        self.expires_in = seconds;
        self
    }

    /// Force download with optional filename
    pub fn download(mut self, filename: Option<String>) -> Self {
        self.download = true;
        self.filename = filename;
        self
    }
}

/// A signed URL for accessing storage files
#[derive(Debug, Clone, Deserialize)]
pub struct StorageUrl {
    /// The signed URL
    pub url: String,
    /// When the URL expires (Unix timestamp in seconds)
    pub expires_at: i64,
}

impl StorageUrl {
    /// Get the URL string
    pub fn as_str(&self) -> &str {
        &self.url
    }

    /// Check if the URL has expired
    pub fn is_expired(&self) -> bool {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;
        now >= self.expires_at
    }

    /// Get the time remaining until expiration (in seconds)
    pub fn time_remaining(&self) -> i64 {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;
        (self.expires_at - now).max(0)
    }
}

// Host functions provided by the Convex runtime
extern "C" {
    /// Store a file in Convex storage
    /// Takes a pointer to JSON request data and returns a pointer to the response JSON
    fn __convex_storage_store(request_ptr: i32, request_len: i32) -> i32;

    /// Get a file from Convex storage
    /// Returns a pointer to the response JSON in WASM memory
    fn __convex_storage_get(storage_id_ptr: i32, storage_id_len: i32) -> i32;

    /// Get metadata for a stored file
    /// Returns a pointer to the response JSON in WASM memory
    fn __convex_storage_get_metadata(storage_id_ptr: i32, storage_id_len: i32) -> i32;

    /// Generate a signed URL for accessing a stored file
    /// Returns a pointer to the response JSON in WASM memory
    fn __convex_storage_generate_url(storage_id_ptr: i32, storage_id_len: i32, options_ptr: i32, options_len: i32) -> i32;

    /// Delete a file from Convex storage
    fn __convex_storage_delete(storage_id_ptr: i32, storage_id_len: i32) -> i32;

    /// Generate a URL for direct file upload
    /// Returns a pointer to the response JSON in WASM memory
    fn __convex_storage_generate_upload_url(options_ptr: i32, options_len: i32) -> i32;

    /// Allocate memory in the WASM linear memory
    /// Returns a pointer to the allocated memory
    fn __convex_alloc(size: i32) -> i32;

    /// Free memory in the WASM linear memory
    fn __convex_free(ptr: i32);
}

/// Store a file in Convex storage
///
/// # Arguments
///
/// * `content_type` - The MIME content type of the file (e.g., "image/png")
/// * `data` - The raw file data as bytes
///
/// # Returns
///
/// The `StorageId` for the stored file
///
/// # Errors
///
/// Returns an error if the storage operation fails
///
/// # Examples
///
/// ```ignore
/// use convex_sdk::storage::store;
///
/// let data = vec![0x89, 0x50, 0x4E, 0x47]; // PNG header
/// let id = store("image/png", data).await?;
/// ```
pub async fn store(content_type: &str, data: Vec<u8>) -> Result<StorageId> {
    // Create the request payload
    let request = StoreRequest {
        content_type,
        data: &data,
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
        __convex_storage_store(ptr, request_json.len() as i32)
    };

    // Free the request memory
    unsafe {
        __convex_free(ptr);
    }

    if result_ptr == 0 {
        return Err(ConvexError::Unknown(
            "Storage store host function failed".into(),
        ));
    }

    // Read response from memory
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
    let store_response: StoreResponse = serde_json::from_slice(&response_data)?;

    Ok(StorageId::new(store_response.storage_id))
}

/// Get a file from Convex storage
///
/// # Arguments
///
/// * `storage_id` - The `StorageId` of the file to retrieve
///
/// # Returns
///
/// The `StorageFile` containing the file data and content type
///
/// # Errors
///
/// Returns an error if the file is not found or the operation fails
///
/// # Examples
///
/// ```ignore
/// use convex_sdk::storage::{get, StorageId};
///
/// let id = StorageId::new("storage_id_123");
/// let file = get(&id).await?;
/// println!("Content type: {}", file.content_type);
/// ```
pub async fn get(storage_id: &StorageId) -> Result<StorageFile> {
    let id_str = storage_id.as_str();

    // Serialize the storage ID request
    let request_json = serde_json::to_vec(&serde_json::json!({
        "storage_id": id_str,
    }))?;

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
        __convex_storage_get(ptr, request_json.len() as i32)
    };

    // Free the request memory
    unsafe {
        __convex_free(ptr);
    }

    if result_ptr == 0 {
        return Err(ConvexError::Unknown(
            "Storage get host function failed".into(),
        ));
    }

    // Read response from memory
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
    let get_response: GetResponse = serde_json::from_slice(&response_data)?;

    Ok(StorageFile {
        content_type: get_response.content_type,
        data: get_response.data,
    })
}

/// Get metadata for a stored file
///
/// # Arguments
///
/// * `storage_id` - The `StorageId` of the file
///
/// # Returns
///
/// The `StorageMetadata` containing file metadata
///
/// # Errors
///
/// Returns an error if the file is not found or the operation fails
///
/// # Examples
///
/// ```ignore
/// use convex_sdk::storage::{get_metadata, StorageId};
///
/// let id = StorageId::new("storage_id_123");
/// let meta = get_metadata(&id).await?;
/// println!("File size: {}", meta.format_size());
/// println!("SHA-256: {}", meta.sha256());
/// ```
pub async fn get_metadata(storage_id: &StorageId) -> Result<StorageMetadata> {
    let id_str = storage_id.as_str();

    // Serialize the storage ID request
    let request_json = serde_json::to_vec(&serde_json::json!({
        "storage_id": id_str,
    }))?;

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
        __convex_storage_get_metadata(ptr, request_json.len() as i32)
    };

    // Free the request memory
    unsafe {
        __convex_free(ptr);
    }

    if result_ptr == 0 {
        return Err(ConvexError::Unknown(
            "Storage get_metadata host function failed".into(),
        ));
    }

    // Read response from memory
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
    let metadata: StorageMetadata = serde_json::from_slice(&response_data)?;

    Ok(metadata)
}

/// Generate a signed URL for accessing a stored file
///
/// # Arguments
///
/// * `storage_id` - The `StorageId` of the file
/// * `options` - Options for URL generation (expiration, download behavior)
///
/// # Returns
///
/// A `StorageUrl` containing the signed URL and expiration time
///
/// # Errors
///
/// Returns an error if the file is not found or URL generation fails
///
/// # Examples
///
/// ```ignore
/// use convex_sdk::storage::{generate_url, StorageId, UrlOptions};
///
/// let id = StorageId::new("storage_id_123");
///
/// // Generate a URL that expires in 1 hour
/// let url = generate_url(&id, UrlOptions::new()).await?;
///
/// // Generate a URL that forces download
/// let url = generate_url(&id, UrlOptions::new()
///     .download(Some("myfile.pdf".to_string()))
///     .expires_in(86400)
/// ).await?;
/// ```
pub async fn generate_url(
    storage_id: &StorageId,
    options: UrlOptions,
) -> Result<StorageUrl> {
    let id_str = storage_id.as_str();

    // Serialize the request
    let request = serde_json::json!({
        "storage_id": id_str,
        "expires_in": options.expires_in,
        "download": options.download,
        "filename": options.filename,
    });
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
        __convex_storage_generate_url(
            id_str.as_ptr() as i32,
            id_str.len() as i32,
            ptr,
            request_json.len() as i32,
        )
    };

    // Free the request memory
    unsafe {
        __convex_free(ptr);
    }

    if result_ptr == 0 {
        return Err(ConvexError::Unknown(
            "Storage generate_url host function failed".into(),
        ));
    }

    // Read response from memory
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
    let storage_url: StorageUrl = serde_json::from_slice(&response_data)?;

    Ok(storage_url)
}

/// Delete a file from Convex storage
///
/// # Arguments
///
/// * `storage_id` - The `StorageId` of the file to delete
///
/// # Returns
///
/// `Ok(())` if the file was deleted successfully
///
/// # Errors
///
/// Returns an error if the file is not found or the operation fails
///
/// # Examples
///
/// ```ignore
/// use convex_sdk::storage::{delete, StorageId};
///
/// let id = StorageId::new("storage_id_123");
/// delete(&id).await?;
/// ```
pub async fn delete(storage_id: &StorageId) -> Result<()> {
    let id_str = storage_id.as_str();

    // Allocate memory for the storage ID
    let id_ptr = wasm_helpers::alloc_and_write(id_str.as_bytes())?;

    // Call host function
    let result_ptr = unsafe {
        __convex_storage_delete(id_ptr, id_str.len() as i32)
    };

    // Free the input memory
    wasm_helpers::free_ptr(id_ptr);

    // For delete, we expect a result even though it's not strictly necessary
    // Some implementations may return 0 on success
    if result_ptr != 0 {
        // Parse the result to check for errors
        let host_result = unsafe { wasm_helpers::parse_host_result(result_ptr)? };
        wasm_helpers::handle_host_result(host_result)?;
    }

    Ok(())
}

/// Options for generating an upload URL
#[derive(Debug, Clone)]
pub struct UploadUrlOptions {
    /// Maximum file size allowed (in bytes)
    pub max_file_size: Option<u64>,
    /// Content type restriction (e.g., "image/*")
    pub content_type: Option<String>,
}

impl Default for UploadUrlOptions {
    fn default() -> Self {
        Self {
            max_file_size: None,
            content_type: None,
        }
    }
}

impl UploadUrlOptions {
    /// Create default upload URL options
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the maximum file size
    pub fn max_file_size(mut self, size: u64) -> Self {
        self.max_file_size = Some(size);
        self
    }

    /// Set the allowed content type
    pub fn content_type(mut self, content_type: impl Into<String>) -> Self {
        self.content_type = Some(content_type.into());
        self
    }
}

/// An upload URL for direct file uploads
#[derive(Debug, Clone, Deserialize)]
pub struct UploadUrl {
    /// The URL to upload to
    pub url: String,
    /// The storage ID that will be assigned to the uploaded file
    pub storage_id: String,
}

impl UploadUrl {
    /// Get the upload URL
    pub fn url(&self) -> &str {
        &self.url
    }

    /// Get the storage ID that will be assigned
    pub fn storage_id(&self) -> &str {
        &self.storage_id
    }
}

/// Generate a URL for direct file upload
///
/// This creates a URL that clients can use to upload files directly to
/// Convex storage without going through a mutation.
///
/// # Arguments
///
/// * `options` - Options for the upload URL (max file size, content type)
///
/// # Returns
///
/// An `UploadUrl` containing the upload URL and the storage ID that will be assigned
///
/// # Errors
///
/// Returns an error if URL generation fails
///
/// # Examples
///
/// ```ignore
/// use convex_sdk::storage::{generate_upload_url, UploadUrlOptions};
///
/// // Generate a basic upload URL
/// let upload = generate_upload_url(UploadUrlOptions::new()).await?;
/// println!("Upload URL: {}", upload.url());
/// println!("Will be stored as: {}", upload.storage_id());
///
/// // Generate with restrictions
/// let upload = generate_upload_url(
///     UploadUrlOptions::new()
///         .max_file_size(5 * 1024 * 1024)  // 5MB
///         .content_type("image/*")
/// ).await?;
/// ```
pub async fn generate_upload_url(options: UploadUrlOptions) -> Result<UploadUrl> {
    // Serialize the request
    let request = serde_json::json!({
        "max_file_size": options.max_file_size,
        "content_type": options.content_type,
    });
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
        __convex_storage_generate_upload_url(ptr, request_json.len() as i32)
    };

    // Free the request memory
    unsafe {
        __convex_free(ptr);
    }

    if result_ptr == 0 {
        return Err(ConvexError::Unknown(
            "Storage generate_upload_url host function failed".into(),
        ));
    }

    // Read response from memory
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
    let upload_url: UploadUrl = serde_json::from_slice(&response_data)?;

    Ok(upload_url)
}

/// Helper functions for WASM memory management
mod wasm_helpers {
    use super::*;

    /// Allocate memory and write bytes to it
    pub fn alloc_and_write(bytes: &[u8]) -> Result<i32> {
        let ptr = unsafe { __convex_alloc(bytes.len() as i32) };
        if ptr == 0 {
            return Err(ConvexError::Unknown("Failed to allocate memory".into()));
        }
        unsafe {
            core::ptr::copy_nonoverlapping(
                bytes.as_ptr(),
                ptr as *mut u8,
                bytes.len()
            );
        }
        Ok(ptr)
    }

    /// Free memory allocated by the host
    pub fn free_ptr(ptr: i32) {
        unsafe {
            __convex_free(ptr);
        }
    }

    /// Parse a host result pointer into a HostResult
    pub unsafe fn parse_host_result(result_ptr: i32) -> Result<HostResult> {
        if result_ptr == 0 {
            return Err(ConvexError::Unknown("Null result from host".into()));
        }

        // Read the length prefix (4 bytes, little-endian)
        let response_len = core::ptr::read_unaligned(result_ptr as *const u32) as usize;

        // Read the JSON data after the length prefix
        let response_data = core::slice::from_raw_parts(
            (result_ptr + 4) as *const u8,
            response_len,
        );
        let json_str = String::from_utf8(response_data.to_vec())
            .map_err(|e| ConvexError::Unknown(format!("UTF-8 decode error: {}", e)))?;

        free_ptr(result_ptr);

        let result: HostResult =
            serde_json::from_str(&json_str).map_err(ConvexError::Serialization)?;
        Ok(result)
    }

    /// Handle a host result, converting errors to ConvexError
    pub fn handle_host_result(result: HostResult) -> Result<Option<serde_json::Value>> {
        if result.success {
            Ok(result.data)
        } else {
            let error_msg = result.error.unwrap_or_else(|| "Unknown error".into());
            Err(ConvexError::Database(error_msg))
        }
    }
}

/// Result from a host function call
#[derive(Debug, Clone, Serialize, Deserialize)]
struct HostResult {
    success: bool,
    data: Option<serde_json::Value>,
    error: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_storage_id_creation() {
        let id = StorageId::new("test_id_123");
        assert_eq!(id.as_str(), "test_id_123");
    }

    #[test]
    fn test_storage_id_from_string() {
        let id: StorageId = "test_id".into();
        assert_eq!(id.as_str(), "test_id");
    }

    #[test]
    fn test_storage_id_from_str() {
        let id: StorageId = "test_id".into();
        assert_eq!(id.as_str(), "test_id");
    }

    #[test]
    fn test_storage_id_validation() {
        assert!(StorageId::new("valid_id-123").is_valid());
        assert!(StorageId::new("").is_valid() == false);
        assert!(StorageId::new("invalid id").is_valid() == false);
        assert!(StorageId::new("invalid@id").is_valid() == false);
    }

    #[test]
    fn test_storage_id_to_string() {
        let id = StorageId::new("test_id");
        assert_eq!(id.to_string(), "test_id");
    }

    #[test]
    fn test_storage_file_creation() {
        let file = StorageFile::new("image/png", vec![1, 2, 3, 4]);
        assert_eq!(file.content_type(), "image/png");
        assert_eq!(file.data(), &[1, 2, 3, 4]);
        assert_eq!(file.len(), 4);
        assert!(!file.is_empty());
    }

    #[test]
    fn test_storage_file_empty() {
        let file = StorageFile::new("text/plain", vec![]);
        assert!(file.is_empty());
        assert_eq!(file.len(), 0);
    }

    #[test]
    fn test_storage_file_data_as_string() {
        let file = StorageFile::new("text/plain", b"hello world".to_vec());
        assert_eq!(file.data_as_string(), Some("hello world".to_string()));

        let invalid_utf8 = StorageFile::new("application/octet-stream", vec![0xFF, 0xFE]);
        assert_eq!(invalid_utf8.data_as_string(), None);
    }
}

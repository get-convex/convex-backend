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

// Host functions provided by the Convex runtime
extern "C" {
    /// Store a file in Convex storage
    /// Returns a pointer to the response JSON in WASM memory
    fn __convex_storage_store(
        content_type_ptr: i32,
        content_type_len: i32,
        data_ptr: i32,
        data_len: i32,
    ) -> i32;

    /// Get a file from Convex storage
    /// Returns a pointer to the response JSON in WASM memory
    fn __convex_storage_get(storage_id_ptr: i32, storage_id_len: i32) -> i32;

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
        __convex_storage_store(ptr, request_json.len() as i32, 0, 0)
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

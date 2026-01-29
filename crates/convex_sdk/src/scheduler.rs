//! Job scheduling for Convex
//!
//! This module provides functionality for scheduling background jobs from Rust functions.
//! Jobs can be scheduled to run immediately, after a delay, or at a specific time.
//!
//! # Example
//!
//! ```ignore
//! use convex_sdk::*;
//! use convex_sdk::scheduler::ScheduleOptions;
//!
//! #[mutation]
//! pub async fn create_post(db: Database, title: String, content: String) -> Result<DocumentId> {
//!     // Create the post
//!     let post_id = db.insert("posts", json!({
//!         "title": title,
//!         "content": content,
//!     })).await?;
//!
//!     // Schedule a job to send notifications in 5 minutes
//!     schedule_job(
//!         "sendNotifications",
//!         json!({ "postId": post_id.as_str() }),
//!         ScheduleOptions::new().delay_ms(5 * 60 * 1000),
//!     ).await?;
//!
//!     Ok(post_id)
//! }
//! ```

use crate::types::{ConvexError, Result};
use alloc::string::String;
use alloc::vec::Vec;
use serde::{Deserialize, Serialize};

/// Unique identifier for a scheduled job
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct JobId(String);

impl JobId {
    /// Create a new JobId from a string
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    /// Get the job ID as a string slice
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<String> for JobId {
    fn from(s: String) -> Self {
        Self::new(s)
    }
}

impl From<&str> for JobId {
    fn from(s: &str) -> Self {
        Self::new(s)
    }
}

/// Options for scheduling a job
#[derive(Debug, Clone, Default, Serialize)]
pub struct ScheduleOptions {
    /// Delay in milliseconds before executing the job
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delay_ms: Option<u64>,

    /// Unix timestamp (in milliseconds) when the job should execute
    #[serde(skip_serializing_if = "Option::is_none")]
    pub execute_at_ms: Option<i64>,

    /// Maximum number of retry attempts if the job fails
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_retries: Option<u32>,

    /// Job name for identification (defaults to function name)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

impl ScheduleOptions {
    /// Create new default schedule options (execute immediately)
    pub fn new() -> Self {
        Self::default()
    }

    /// Schedule the job to run after a delay
    ///
    /// # Example
    ///
    /// ```ignore
    /// // Schedule to run in 5 minutes
    /// let options = ScheduleOptions::new().delay_ms(5 * 60 * 1000);
    /// ```
    pub fn delay_ms(mut self, ms: u64) -> Self {
        self.delay_ms = Some(ms);
        self.execute_at_ms = None;
        self
    }

    /// Schedule the job to run at a specific timestamp
    ///
    /// # Example
    ///
    /// ```ignore
    /// // Schedule to run at a specific time
    /// let timestamp = chrono::Utc::now().timestamp_millis() + 3600000;
    /// let options = ScheduleOptions::new().execute_at_ms(timestamp);
    /// ```
    pub fn execute_at_ms(mut self, timestamp: i64) -> Self {
        self.execute_at_ms = Some(timestamp);
        self.delay_ms = None;
        self
    }

    /// Set the maximum number of retries
    pub fn max_retries(mut self, retries: u32) -> Self {
        self.max_retries = Some(retries);
        self
    }

    /// Set a custom name for the job
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }
}

/// Status of a scheduled job
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JobStatus {
    /// Job is pending and waiting to be executed
    Pending,
    /// Job is currently being executed
    Running,
    /// Job completed successfully
    Completed,
    /// Job failed after all retry attempts
    Failed,
    /// Job was cancelled
    Cancelled,
}

impl JobStatus {
    /// Check if the job is in a terminal state
    pub fn is_terminal(&self) -> bool {
        matches!(self, JobStatus::Completed | JobStatus::Failed | JobStatus::Cancelled)
    }

    /// Check if the job is still active (pending or running)
    pub fn is_active(&self) -> bool {
        matches!(self, JobStatus::Pending | JobStatus::Running)
    }
}

/// Information about a scheduled job
#[derive(Debug, Clone, Deserialize)]
pub struct JobInfo {
    /// The job ID
    pub id: JobId,
    /// Current status of the job
    pub status: JobStatus,
    /// Name of the job
    pub name: String,
    /// Function to be called
    pub function_name: String,
    /// Arguments passed to the function
    pub args: serde_json::Value,
    /// When the job was scheduled
    pub scheduled_at: i64,
    /// When the job should execute (or did execute)
    pub execute_at: i64,
    /// Number of retry attempts made
    pub retry_count: u32,
    /// Maximum number of retry attempts
    pub max_retries: u32,
    /// Error message if the job failed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Request to schedule a job
#[derive(Debug, Serialize)]
struct ScheduleJobRequest {
    /// Name of the function to call
    pub function_name: String,
    /// Arguments to pass to the function
    pub args: serde_json::Value,
    /// Scheduling options
    #[serde(flatten)]
    pub options: ScheduleOptions,
}

/// Response from scheduling a job
#[derive(Debug, Deserialize)]
struct ScheduleJobResponse {
    /// The ID of the scheduled job
    pub job_id: String,
}

/// Result of a job operation
#[derive(Debug, Deserialize)]
struct JobOperationResult {
    /// Whether the operation was successful
    pub success: bool,
    /// Error message if the operation failed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

// Host functions provided by the Convex runtime
extern "C" {
    /// Schedule a new job
    /// Returns a pointer to the response JSON in WASM memory
    fn __convex_schedule_job(request_ptr: i32, request_len: i32) -> i32;

    /// Cancel a scheduled job
    /// Returns a pointer to the response JSON in WASM memory
    fn __convex_cancel_job(job_id_ptr: i32, job_id_len: i32) -> i32;

    /// Get information about a scheduled job
    /// Returns a pointer to the response JSON in WASM memory
    fn __convex_get_job_info(job_id_ptr: i32, job_id_len: i32) -> i32;

    /// List scheduled jobs
    /// Returns a pointer to the response JSON in WASM memory
    fn __convex_list_jobs(filter_ptr: i32, filter_len: i32) -> i32;

    /// Allocate memory in the WASM linear memory
    fn __convex_alloc(size: i32) -> i32;

    /// Free memory in the WASM linear memory
    fn __convex_free(ptr: i32);
}

/// Schedule a new background job
///
/// # Arguments
///
/// * `function_name` - The name of the function to call (e.g., "sendEmail")
/// * `args` - Arguments to pass to the function as JSON
/// * `options` - Scheduling options (delay, retries, etc.)
///
/// # Returns
///
/// The `JobId` of the scheduled job
///
/// # Example
///
/// ```ignore
/// use convex_sdk::*;
/// use convex_sdk::scheduler::ScheduleOptions;
///
/// let job_id = schedule_job(
///     "sendWelcomeEmail",
///     json!({ "userId": "user123" }),
///     ScheduleOptions::new().delay_ms(60000),
/// ).await?;
///
/// println!("Job scheduled: {}", job_id.as_str());
/// ```
pub async fn schedule_job(
    function_name: &str,
    args: serde_json::Value,
    options: ScheduleOptions,
) -> Result<JobId> {
    let request = ScheduleJobRequest {
        function_name: function_name.to_string(),
        args,
        options,
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
    let result_ptr = unsafe { __convex_schedule_job(ptr, request_json.len() as i32) };

    // Free the request memory
    unsafe { __convex_free(ptr); }

    if result_ptr == 0 {
        return Err(ConvexError::Unknown(
            "Schedule job host function failed".into(),
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
    unsafe { __convex_free(result_ptr); }

    // Deserialize the response
    let response: ScheduleJobResponse = serde_json::from_slice(&response_data)?;

    Ok(JobId::new(response.job_id))
}

/// Cancel a scheduled job
///
/// # Arguments
///
/// * `job_id` - The ID of the job to cancel
///
/// # Errors
///
/// Returns an error if the job is not found or cannot be cancelled
///
/// # Example
///
/// ```ignore
/// use convex_sdk::scheduler::{schedule_job, cancel_job, ScheduleOptions};
///
/// let job_id = schedule_job("task", json!({}), ScheduleOptions::new()).await?;
/// cancel_job(&job_id).await?;
/// ```
pub async fn cancel_job(job_id: &JobId) -> Result<()> {
    let id_str = job_id.as_str();

    // Allocate WASM memory for the job ID
    let ptr = unsafe { __convex_alloc(id_str.len() as i32) };
    if ptr == 0 {
        return Err(ConvexError::Unknown(
            "Failed to allocate WASM memory".into(),
        ));
    }

    // Write job ID to memory
    unsafe {
        let slice = core::slice::from_raw_parts_mut(ptr as *mut u8, id_str.len());
        slice.copy_from_slice(id_str.as_bytes());
    }

    // Call host function
    let result_ptr = unsafe { __convex_cancel_job(ptr, id_str.len() as i32) };

    // Free the input memory
    unsafe { __convex_free(ptr); }

    if result_ptr == 0 {
        return Err(ConvexError::Unknown(
            "Cancel job host function failed".into(),
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
    unsafe { __convex_free(result_ptr); }

    // Deserialize the response
    let result: JobOperationResult = serde_json::from_slice(&response_data)?;

    if result.success {
        Ok(())
    } else {
        Err(ConvexError::Unknown(
            result.error.unwrap_or_else(|| "Failed to cancel job".into()),
        ))
    }
}

/// Get information about a scheduled job
///
/// # Arguments
///
/// * `job_id` - The ID of the job to look up
///
/// # Returns
///
/// `Some(JobInfo)` if the job exists, `None` otherwise
///
/// # Example
///
/// ```ignore
/// use convex_sdk::scheduler::{schedule_job, get_job_info, ScheduleOptions};
///
/// let job_id = schedule_job("task", json!({}), ScheduleOptions::new()).await?;
/// let info = get_job_info(&job_id).await?;
///
/// if let Some(info) = info {
///     println!("Job status: {:?}", info.status);
/// }
/// ```
pub async fn get_job_info(job_id: &JobId) -> Result<Option<JobInfo>> {
    let id_str = job_id.as_str();

    // Allocate WASM memory for the job ID
    let ptr = unsafe { __convex_alloc(id_str.len() as i32) };
    if ptr == 0 {
        return Err(ConvexError::Unknown(
            "Failed to allocate WASM memory".into(),
        ));
    }

    // Write job ID to memory
    unsafe {
        let slice = core::slice::from_raw_parts_mut(ptr as *mut u8, id_str.len());
        slice.copy_from_slice(id_str.as_bytes());
    }

    // Call host function
    let result_ptr = unsafe { __convex_get_job_info(ptr, id_str.len() as i32) };

    // Free the input memory
    unsafe { __convex_free(ptr); }

    if result_ptr == 0 {
        return Err(ConvexError::Unknown(
            "Get job info host function failed".into(),
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
    unsafe { __convex_free(result_ptr); }

    // Deserialize the response
    let result: JobOperationResult = serde_json::from_slice(&response_data)?;

    if !result.success {
        return Ok(None);
    }

    let info: JobInfo = serde_json::from_slice(&response_data)?;
    Ok(Some(info))
}

/// List scheduled jobs with optional filters
///
/// # Arguments
///
/// * `status` - Optional status filter
/// * `limit` - Maximum number of jobs to return
///
/// # Returns
///
/// A vector of `JobInfo` for matching jobs
///
/// # Example
///
/// ```ignore
/// use convex_sdk::scheduler::{list_jobs, JobStatus};
///
/// // Get all pending jobs
/// let pending_jobs = list_jobs(Some(JobStatus::Pending), Some(100)).await?;
///
/// for job in pending_jobs {
///     println!("{}: {:?}", job.name, job.status);
/// }
/// ```
pub async fn list_jobs(
    status: Option<JobStatus>,
    limit: Option<usize>,
) -> Result<Vec<JobInfo>> {
    let filter = serde_json::json!({
        "status": status,
        "limit": limit,
    });

    let filter_json = serde_json::to_vec(&filter)?;

    // Allocate WASM memory for the filter
    let ptr = unsafe { __convex_alloc(filter_json.len() as i32) };
    if ptr == 0 {
        return Err(ConvexError::Unknown(
            "Failed to allocate WASM memory".into(),
        ));
    }

    // Write filter to memory
    unsafe {
        let slice = core::slice::from_raw_parts_mut(ptr as *mut u8, filter_json.len());
        slice.copy_from_slice(&filter_json);
    }

    // Call host function
    let result_ptr = unsafe { __convex_list_jobs(ptr, filter_json.len() as i32) };

    // Free the input memory
    unsafe { __convex_free(ptr); }

    if result_ptr == 0 {
        return Err(ConvexError::Unknown(
            "List jobs host function failed".into(),
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
    unsafe { __convex_free(result_ptr); }

    // Deserialize the response
    let jobs: Vec<JobInfo> = serde_json::from_slice(&response_data)?;
    Ok(jobs)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_job_id_creation() {
        let id = JobId::new("job123");
        assert_eq!(id.as_str(), "job123");
    }

    #[test]
    fn test_schedule_options_default() {
        let opts = ScheduleOptions::new();
        assert!(opts.delay_ms.is_none());
        assert!(opts.execute_at_ms.is_none());
        assert!(opts.max_retries.is_none());
        assert!(opts.name.is_none());
    }

    #[test]
    fn test_schedule_options_delay() {
        let opts = ScheduleOptions::new().delay_ms(5000);
        assert_eq!(opts.delay_ms, Some(5000));
        assert!(opts.execute_at_ms.is_none());
    }

    #[test]
    fn test_schedule_options_execute_at() {
        let timestamp = 1234567890i64;
        let opts = ScheduleOptions::new().execute_at_ms(timestamp);
        assert_eq!(opts.execute_at_ms, Some(timestamp));
        assert!(opts.delay_ms.is_none());
    }

    #[test]
    fn test_job_status_is_terminal() {
        assert!(!JobStatus::Pending.is_terminal());
        assert!(!JobStatus::Running.is_terminal());
        assert!(JobStatus::Completed.is_terminal());
        assert!(JobStatus::Failed.is_terminal());
        assert!(JobStatus::Cancelled.is_terminal());
    }

    #[test]
    fn test_job_status_is_active() {
        assert!(JobStatus::Pending.is_active());
        assert!(JobStatus::Running.is_active());
        assert!(!JobStatus::Completed.is_active());
        assert!(!JobStatus::Failed.is_active());
        assert!(!JobStatus::Cancelled.is_active());
    }
}

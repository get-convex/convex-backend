//! Job scheduling for Convex Rust functions
//!
//! This module provides types and utilities for scheduling background jobs
//! from Rust/WASM functions. Jobs can be scheduled to run immediately,
//! after a delay, or at a specific time.
//!
//! # Example
//!
//! ```ignore
//! use convex_sdk::*;
//! use convex_sdk::scheduler::{ScheduleOptions, JobStatus};
//!
//! #[mutation]
//! pub async fn schedule_email(db: Database, user_id: String) -> Result<String> {
//!     // Schedule a job to run in 5 minutes
//!     let job_id = schedule_job(
//!         "sendWelcomeEmail",
//!         json!({ "userId": user_id }),
//!         ScheduleOptions::new().delay_ms(5 * 60 * 1000),
//!     ).await?;
//!
//!     Ok(job_id)
//! }
//! ```

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
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
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
    pub fn delay_ms(mut self, ms: u64) -> Self {
        self.delay_ms = Some(ms);
        self.execute_at_ms = None;
        self
    }

    /// Schedule the job to run at a specific timestamp
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
#[derive(Debug, Clone, Serialize, Deserialize)]
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
pub struct ScheduleJobRequest {
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
pub struct ScheduleJobResponse {
    /// The ID of the scheduled job
    pub job_id: String,
}

/// Result of a job operation
#[derive(Debug, Deserialize)]
pub struct JobOperationResult {
    /// Whether the operation was successful
    pub success: bool,
    /// Error message if the operation failed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Job scheduler trait for host functions
///
/// This trait abstracts job scheduling operations needed by the Rust runner.
/// The actual implementation is provided by the Convex backend.
pub trait JobScheduler: Send + Sync {
    /// Schedule a new job
    fn schedule_job(
        &self,
        function_name: String,
        args: serde_json::Value,
        options: ScheduleOptions,
    ) -> anyhow::Result<JobId>;

    /// Cancel a scheduled job
    fn cancel_job(&self, job_id: JobId) -> anyhow::Result<()>;

    /// Get information about a scheduled job
    fn get_job_info(&self, job_id: JobId) -> anyhow::Result<Option<JobInfo>>;

    /// List all scheduled jobs with optional filters
    fn list_jobs(
        &self,
        status: Option<JobStatus>,
        limit: Option<usize>,
    ) -> anyhow::Result<Vec<JobInfo>>;
}

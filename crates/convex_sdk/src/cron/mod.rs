//! Cron jobs and scheduled tasks for Convex
//!
//! This module provides functionality for defining recurring scheduled tasks (cron jobs).
//!
//! # Example
//!
//! ```ignore
//! use convex_sdk::cron::{cron_jobs, CronContext};
//!
//! let crons = cron_jobs!({
//!     "cleanupOldData": {
//!         interval: "1h",
//!         handler: cleanup_old_data
//!     },
//!     "dailyReport": {
//!         cron: "0 9 * * *",
//!         handler: generate_daily_report
//!     }
//! });
//! ```

use alloc::string::String;
use alloc::vec::Vec;
use serde::{Deserialize, Serialize};

/// A cron job definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CronJob {
    /// Unique identifier for the cron job
    pub name: String,
    /// The schedule type
    pub schedule: Schedule,
    /// Function reference to execute
    pub handler: String,
}

/// Schedule types for cron jobs
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Schedule {
    /// Run at a fixed interval (in seconds)
    Interval { seconds: u64 },
    /// Run hourly at a specific minute
    Hourly { minute_utc: u8 },
    /// Run daily at a specific time
    Daily { hour_utc: u8, minute_utc: u8 },
    /// Run weekly on a specific day and time
    Weekly {
        day_of_week: DayOfWeek,
        hour_utc: u8,
        minute_utc: u8,
    },
    /// Run monthly on a specific day and time
    Monthly {
        day: u8,
        hour_utc: u8,
        minute_utc: u8,
    },
    /// Custom cron expression
    Cron { expression: String },
}

/// Days of the week
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DayOfWeek {
    Sunday,
    Monday,
    Tuesday,
    Wednesday,
    Thursday,
    Friday,
    Saturday,
}

impl DayOfWeek {
    /// Get the day as a number (0 = Sunday, 6 = Saturday)
    pub fn as_number(&self) -> u8 {
        match self {
            DayOfWeek::Sunday => 0,
            DayOfWeek::Monday => 1,
            DayOfWeek::Tuesday => 2,
            DayOfWeek::Wednesday => 3,
            DayOfWeek::Thursday => 4,
            DayOfWeek::Friday => 5,
            DayOfWeek::Saturday => 6,
        }
    }
}

/// A collection of cron jobs
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CronJobs {
    jobs: Vec<CronJob>,
}

impl CronJobs {
    /// Create a new empty cron jobs collection
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a cron job with an interval
    ///
    /// # Arguments
    ///
    /// * `name` - Unique identifier for the job
    /// * `seconds` - Interval in seconds
    /// * `handler` - Function name to execute
    ///
    /// # Example
    ///
    /// ```ignore
    /// let mut crons = CronJobs::new();
    /// crons.interval("cleanup", 3600, "cleanupOldData");
    /// ```
    pub fn interval(
        &mut self,
        name: impl Into<String>,
        seconds: u64,
        handler: impl Into<String>,
    ) -> &mut Self {
        self.jobs.push(CronJob {
            name: name.into(),
            schedule: Schedule::Interval { seconds },
            handler: handler.into(),
        });
        self
    }

    /// Schedule a job to run hourly at a specific minute
    ///
    /// # Arguments
    ///
    /// * `name` - Unique identifier for the job
    /// * `minute_utc` - Minute of the hour (0-59) in UTC
    /// * `handler` - Function name to execute
    ///
    /// # Example
    ///
    /// ```ignore
    /// let mut crons = CronJobs::new();
    /// crons.hourly("sync", 0, "syncData"); // Run at the top of every hour
    /// ```
    pub fn hourly(
        &mut self,
        name: impl Into<String>,
        minute_utc: u8,
        handler: impl Into<String>,
    ) -> Result<&mut Self, String> {
        if minute_utc > 59 {
            return Err("minute_utc must be between 0 and 59".to_string());
        }
        self.jobs.push(CronJob {
            name: name.into(),
            schedule: Schedule::Hourly { minute_utc },
            handler: handler.into(),
        });
        Ok(self)
    }

    /// Schedule a job to run daily at a specific time
    ///
    /// # Arguments
    ///
    /// * `name` - Unique identifier for the job
    /// * `hour_utc` - Hour of the day (0-23) in UTC
    /// * `minute_utc` - Minute of the hour (0-59) in UTC
    /// * `handler` - Function name to execute
    ///
    /// # Example
    ///
    /// ```ignore
    /// let mut crons = CronJobs::new();
    /// crons.daily("report", 9, 0, "generateReport"); // Run at 9:00 AM UTC
    /// ```
    pub fn daily(
        &mut self,
        name: impl Into<String>,
        hour_utc: u8,
        minute_utc: u8,
        handler: impl Into<String>,
    ) -> Result<&mut Self, String> {
        if hour_utc > 23 {
            return Err("hour_utc must be between 0 and 23".to_string());
        }
        if minute_utc > 59 {
            return Err("minute_utc must be between 0 and 59".to_string());
        }
        self.jobs.push(CronJob {
            name: name.into(),
            schedule: Schedule::Daily {
                hour_utc,
                minute_utc,
            },
            handler: handler.into(),
        });
        Ok(self)
    }

    /// Schedule a job to run weekly on a specific day and time
    ///
    /// # Arguments
    ///
    /// * `name` - Unique identifier for the job
    /// * `day_of_week` - Day of the week
    /// * `hour_utc` - Hour of the day (0-23) in UTC
    /// * `minute_utc` - Minute of the hour (0-59) in UTC
    /// * `handler` - Function name to execute
    ///
    /// # Example
    ///
    /// ```ignore
    /// use convex_sdk::cron::DayOfWeek;
    ///
    /// let mut crons = CronJobs::new();
    /// crons.weekly("cleanup", DayOfWeek::Sunday, 2, 0, "weeklyCleanup");
    /// ```
    pub fn weekly(
        &mut self,
        name: impl Into<String>,
        day_of_week: DayOfWeek,
        hour_utc: u8,
        minute_utc: u8,
        handler: impl Into<String>,
    ) -> Result<&mut Self, String> {
        if hour_utc > 23 {
            return Err("hour_utc must be between 0 and 23".to_string());
        }
        if minute_utc > 59 {
            return Err("minute_utc must be between 0 and 59".to_string());
        }
        self.jobs.push(CronJob {
            name: name.into(),
            schedule: Schedule::Weekly {
                day_of_week,
                hour_utc,
                minute_utc,
            },
            handler: handler.into(),
        });
        Ok(self)
    }

    /// Schedule a job to run monthly on a specific day and time
    ///
    /// # Arguments
    ///
    /// * `name` - Unique identifier for the job
    /// * `day` - Day of the month (1-31) in UTC
    /// * `hour_utc` - Hour of the day (0-23) in UTC
    /// * `minute_utc` - Minute of the hour (0-59) in UTC
    /// * `handler` - Function name to execute
    ///
    /// # Example
    ///
    /// ```ignore
    /// let mut crons = CronJobs::new();
    /// crons.monthly("report", 1, 9, 0, "monthlyReport"); // First of the month
    /// ```
    pub fn monthly(
        &mut self,
        name: impl Into<String>,
        day: u8,
        hour_utc: u8,
        minute_utc: u8,
        handler: impl Into<String>,
    ) -> Result<&mut Self, String> {
        if day == 0 || day > 31 {
            return Err("day must be between 1 and 31".to_string());
        }
        if hour_utc > 23 {
            return Err("hour_utc must be between 0 and 23".to_string());
        }
        if minute_utc > 59 {
            return Err("minute_utc must be between 0 and 59".to_string());
        }
        self.jobs.push(CronJob {
            name: name.into(),
            schedule: Schedule::Monthly {
                day,
                hour_utc,
                minute_utc,
            },
            handler: handler.into(),
        });
        Ok(self)
    }

    /// Schedule a job with a custom cron expression
    ///
    /// # Arguments
    ///
    /// * `name` - Unique identifier for the job
    /// * `expression` - Cron expression (e.g., "0 9 * * 1-5" for weekdays at 9 AM)
    /// * `handler` - Function name to execute
    ///
    /// # Example
    ///
    /// ```ignore
    /// let mut crons = CronJobs::new();
    /// // Run at 9:00 AM on weekdays
    /// crons.cron("weekdayReport", "0 9 * * 1-5", "generateReport");
    /// ```
    pub fn cron(
        &mut self,
        name: impl Into<String>,
        expression: impl Into<String>,
        handler: impl Into<String>,
    ) -> &mut Self {
        self.jobs.push(CronJob {
            name: name.into(),
            schedule: Schedule::Cron {
                expression: expression.into(),
            },
            handler: handler.into(),
        });
        self
    }

    /// Get all cron jobs
    pub fn get_jobs(&self) -> &[CronJob] {
        &self.jobs
    }

    /// Export the cron jobs configuration as JSON
    pub fn export(&self) -> String {
        serde_json::to_string(&self.jobs).unwrap_or_default()
    }
}

/// Context available to cron job handlers
#[derive(Debug)]
pub struct CronContext {
    // In a full implementation, this would include:
    // - run_query: Function to run queries
    // - run_mutation: Function to run mutations
    // - job_name: Name of the current cron job
    // - scheduled_time: When this job was scheduled to run
}

impl CronContext {
    /// Create a new cron context
    pub fn new() -> Self {
        Self {}
    }
}

impl Default for CronContext {
    fn default() -> Self {
        Self::new()
    }
}

/// Macro to define cron jobs
///
/// # Example
///
/// ```ignore
/// use convex_sdk::cron::{cron_jobs, CronContext};
///
/// define_crons!({
///     "cleanup": {
///         interval: "1h",
///         handler: cleanup_data
///     },
///     "daily_report": {
///         cron: "0 9 * * *",
///         handler: generate_report
///     }
/// });
/// ```
#[macro_export]
macro_rules! define_crons {
    ({ $($name:literal: { $($key:ident: $value:expr),* $(,)? }),* $(,)? }) => {{
        let mut crons = $crate::cron::CronJobs::new();
        $(
            crons.add_job($name, define_crons!(@schedule $($key: $value),*));
        )*
        crons
    }};
    (@schedule interval: $interval:expr, handler: $handler:expr) => {
        $crate::cron::Schedule::Interval {
            seconds: $crate::define_crons!(@parse_interval $interval)
        }
    };
    (@schedule cron: $cron:expr, handler: $handler:expr) => {
        $crate::cron::Schedule::Cron {
            expression: $cron.to_string()
        }
    };
    (@parse_interval $interval:expr) => {
        {
            let s = $interval;
            // Parse interval string like "1h", "30m", "60s"
            if let Some(n) = s.strip_suffix('s') {
                n.parse::<u64>().unwrap_or(60)
            } else if let Some(n) = s.strip_suffix('m') {
                n.parse::<u64>().unwrap_or(1) * 60
            } else if let Some(n) = s.strip_suffix('h') {
                n.parse::<u64>().unwrap_or(1) * 3600
            } else if let Some(n) = s.strip_suffix('d') {
                n.parse::<u64>().unwrap_or(1) * 86400
            } else {
                s.parse::<u64>().unwrap_or(60)
            }
        }
    };
}

/// Re-export the macro
pub use define_crons;

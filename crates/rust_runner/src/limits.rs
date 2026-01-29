//! Resource limits for WASM execution
//!
//! This module provides memory and table limiters to prevent
//! untrusted WASM code from consuming excessive resources.

/// Default maximum memory for queries and mutations (256 MB)
pub const DEFAULT_QUERY_MEMORY_LIMIT: usize = 256 * 1024 * 1024;

/// Default maximum memory for actions (512 MB)
pub const DEFAULT_ACTION_MEMORY_LIMIT: usize = 512 * 1024 * 1024;

/// Default maximum table size (10,000 entries)
pub const DEFAULT_TABLE_LIMIT: u32 = 10_000;

/// Resource limiter for WASM execution
///
/// Implements wasmtime's ResourceLimiter trait to control
/// memory and table growth during WASM execution.
pub struct ResourceLimiter {
    max_memory_bytes: usize,
    max_table_size: u32,
}

impl ResourceLimiter {
    /// Create a new resource limiter with the specified limits
    pub fn new(max_memory_bytes: usize, max_table_size: u32) -> Self {
        Self {
            max_memory_bytes,
            max_table_size,
        }
    }

    /// Create a limiter for queries/mutations with default limits
    pub fn for_query() -> Self {
        Self::new(DEFAULT_QUERY_MEMORY_LIMIT, DEFAULT_TABLE_LIMIT)
    }

    /// Create a limiter for actions with default limits
    pub fn for_action() -> Self {
        Self::new(DEFAULT_ACTION_MEMORY_LIMIT, DEFAULT_TABLE_LIMIT)
    }

    /// Get the memory limit in bytes
    pub fn memory_limit(&self) -> usize {
        self.max_memory_bytes
    }

    /// Get the table limit
    pub fn table_limit(&self) -> u32 {
        self.max_table_size
    }
}

impl wasmtime::ResourceLimiter for ResourceLimiter {
    /// Called when WASM memory wants to grow
    ///
    /// Returns Ok(true) if the growth should be allowed, Ok(false) otherwise.
    fn memory_growing(
        &mut self,
        _current: usize,
        desired: usize,
        _maximum: Option<usize>,
    ) -> anyhow::Result<bool> {
        Ok(desired <= self.max_memory_bytes)
    }

    /// Called when WASM table wants to grow
    ///
    /// Returns Ok(true) if the growth should be allowed, Ok(false) otherwise.
    fn table_growing(
        &mut self,
        _current: u32,
        desired: u32,
        _maximum: Option<u32>,
    ) -> anyhow::Result<bool> {
        Ok(desired <= self.max_table_size)
    }
}

/// Execution limits for a function
#[derive(Debug, Clone, Copy)]
pub struct ExecutionLimits {
    /// Maximum execution time
    pub max_duration: std::time::Duration,
    /// Maximum memory in bytes
    pub max_memory_bytes: usize,
    /// Maximum table size
    pub max_table_size: u32,
    /// Maximum CPU fuel (instructions)
    pub max_fuel: u64,
}

impl ExecutionLimits {
    /// Create limits for a query function
    pub fn query() -> Self {
        Self {
            max_duration: std::time::Duration::from_secs(30),
            max_memory_bytes: DEFAULT_QUERY_MEMORY_LIMIT,
            max_table_size: DEFAULT_TABLE_LIMIT,
            max_fuel: 10_000_000_000, // 10B instructions
        }
    }

    /// Create limits for a mutation function
    pub fn mutation() -> Self {
        Self {
            max_duration: std::time::Duration::from_secs(30),
            max_memory_bytes: DEFAULT_QUERY_MEMORY_LIMIT,
            max_table_size: DEFAULT_TABLE_LIMIT,
            max_fuel: 10_000_000_000, // 10B instructions
        }
    }

    /// Create limits for an action function
    pub fn action() -> Self {
        Self {
            max_duration: std::time::Duration::from_secs(300), // 5 minutes
            max_memory_bytes: DEFAULT_ACTION_MEMORY_LIMIT,
            max_table_size: DEFAULT_TABLE_LIMIT,
            max_fuel: 100_000_000_000, // 100B instructions
        }
    }

    /// Create limits for an HTTP action function
    pub fn http_action() -> Self {
        Self::action()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wasmtime::ResourceLimiter as _;

    #[test]
    fn test_resource_limiter_memory() {
        let mut limiter = ResourceLimiter::for_query();

        // Should allow growth within limit
        assert!(limiter.memory_growing(0, 100 * 1024 * 1024, None).unwrap());

        // Should deny growth beyond limit
        assert!(!limiter.memory_growing(0, 300 * 1024 * 1024, None).unwrap());
    }

    #[test]
    fn test_resource_limiter_table() {
        let mut limiter = ResourceLimiter::for_query();

        // Should allow growth within limit
        assert!(limiter.table_growing(0, 1000, None).unwrap());

        // Should deny growth beyond limit
        assert!(!limiter.table_growing(0, 20_000, None).unwrap());
    }

    #[test]
    fn test_execution_limits_query() {
        let limits = ExecutionLimits::query();
        assert_eq!(limits.max_duration, std::time::Duration::from_secs(30));
        assert_eq!(limits.max_memory_bytes, DEFAULT_QUERY_MEMORY_LIMIT);
        assert_eq!(limits.max_fuel, 10_000_000_000);
    }

    #[test]
    fn test_execution_limits_action() {
        let limits = ExecutionLimits::action();
        assert_eq!(limits.max_duration, std::time::Duration::from_secs(300));
        assert_eq!(limits.max_memory_bytes, DEFAULT_ACTION_MEMORY_LIMIT);
        assert_eq!(limits.max_fuel, 100_000_000_000);
    }
}

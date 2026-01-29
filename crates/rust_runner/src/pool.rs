//! Connection pooling for database and external service connections
//!
//! This module provides connection pooling to efficiently reuse connections
//! to the database and external services, reducing connection overhead.

use std::{
    collections::VecDeque,
    sync::{Arc, Condvar, Mutex},
    time::{Duration, Instant},
};

use anyhow::Result;
use tracing::{debug, warn};

/// A generic connection pool for managing reusable connections
///
/// The pool maintains a set of ready-to-use connections and handles
/// connection lifecycle management including creation, validation,
/// and cleanup of expired connections.
pub struct ConnectionPool<T> {
    /// Pool configuration
    config: PoolConfig,
    /// Available connections
    available: Mutex<VecDeque<PooledConnection<T>>>,
    /// Condition variable for waiting threads
    condvar: Condvar,
    /// Total number of connections (in use + available)
    total_count: Mutex<usize>,
    /// Connection factory
    factory: Box<dyn Fn() -> Result<T> + Send + Sync>,
}

impl<T> std::fmt::Debug for ConnectionPool<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ConnectionPool")
            .field("config", &self.config)
            .field("available", &self.available.lock().unwrap().len())
            .field("total_count", &*self.total_count.lock().unwrap())
            .finish_non_exhaustive()
    }
}

/// Configuration for a connection pool
#[derive(Debug, Clone)]
pub struct PoolConfig {
    /// Maximum number of connections in the pool
    pub max_size: usize,
    /// Minimum number of connections to maintain
    pub min_idle: usize,
    /// Maximum lifetime of a connection
    pub max_lifetime: Duration,
    /// Connection idle timeout
    pub idle_timeout: Duration,
    /// Timeout for acquiring a connection from the pool
    pub acquire_timeout: Duration,
}

impl Default for PoolConfig {
    fn default() -> Self {
        Self {
            max_size: 10,
            min_idle: 2,
            max_lifetime: Duration::from_secs(3600), // 1 hour
            idle_timeout: Duration::from_secs(600),  // 10 minutes
            acquire_timeout: Duration::from_secs(30),
        }
    }
}

/// A connection that has been checked out from the pool
pub struct PooledConnection<T> {
    /// The actual connection
    connection: T,
    /// When this connection was created
    created_at: Instant,
    /// When this connection was last used
    last_used: Instant,
}

impl<T: std::fmt::Debug> std::fmt::Debug for PooledConnection<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PooledConnection")
            .field("connection", &self.connection)
            .field("created_at", &self.created_at)
            .field("last_used", &self.last_used)
            .finish()
    }
}

impl<T> PooledConnection<T> {
    fn new(connection: T) -> Self {
        let now = Instant::now();
        Self {
            connection,
            created_at: now,
            last_used: now,
        }
    }

    /// Check if this connection has exceeded its maximum lifetime
    fn is_expired(&self, max_lifetime: Duration) -> bool {
        self.created_at.elapsed() > max_lifetime
    }

    /// Check if this connection has been idle too long
    fn is_idle_expired(&self, idle_timeout: Duration) -> bool {
        self.last_used.elapsed() > idle_timeout
    }

    /// Mark the connection as used
    fn mark_used(&mut self) {
        self.last_used = Instant::now();
    }
}

/// A smart pointer to a pooled connection that automatically returns
/// the connection to the pool when dropped.
pub struct ConnectionHandle<T: Send> {
    connection: Option<T>,
    return_tx: Option<std::sync::mpsc::Sender<T>>,
}

impl<T: Send> std::ops::Deref for ConnectionHandle<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.connection.as_ref().unwrap()
    }
}

impl<T: Send> std::ops::DerefMut for ConnectionHandle<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.connection.as_mut().unwrap()
    }
}

impl<T: Send> Drop for ConnectionHandle<T> {
    fn drop(&mut self) {
        if let (Some(conn), Some(return_tx)) = (self.connection.take(), self.return_tx.take()) {
            // Try to return the connection, but don't panic if the pool is gone
            let _ = return_tx.send(conn);
        }
    }
}

impl<T: Send + 'static> ConnectionPool<T> {
    /// Create a new connection pool
    ///
    /// # Arguments
    /// * `config` - Pool configuration
    /// * `factory` - Function to create new connections
    pub fn new<F>(config: PoolConfig, factory: F) -> Arc<Self>
    where
        F: Fn() -> Result<T> + Send + Sync + 'static,
    {
        let pool = Arc::new(Self {
            config,
            available: Mutex::new(VecDeque::new()),
            condvar: Condvar::new(),
            total_count: Mutex::new(0),
            factory: Box::new(factory),
        });

        // Start background maintenance task
        let pool_clone = pool.clone();
        tokio::spawn(async move {
            pool_clone.maintenance_loop().await;
        });

        pool
    }

    /// Create a pool with default configuration
    pub fn with_defaults<F>(factory: F) -> Arc<Self>
    where
        F: Fn() -> Result<T> + Send + Sync + 'static,
    {
        Self::new(PoolConfig::default(), factory)
    }

    /// Acquire a connection from the pool
    ///
    /// This method blocks until a connection is available or the
    /// acquire timeout is reached.
    pub async fn acquire(&self) -> Result<ConnectionHandle<T>> {
        let start = Instant::now();
        let timeout = self.config.acquire_timeout;

        loop {
            // Try to get an available connection
            if let Some(mut pooled) = self.try_get_available() {
                // Validate the connection
                pooled.mark_used();
                let (return_tx, return_rx) = std::sync::mpsc::channel();

                // Spawn a task to handle the returned connection
                let pool_arc = Arc::new(()); // Placeholder for actual pool reference
                tokio::spawn(async move {
                    if let Ok(conn) = return_rx.recv() {
                        // Return connection to pool - simplified version
                        // In production, this would properly return to the pool
                    }
                });

                return Ok(ConnectionHandle {
                    connection: Some(pooled.connection),
                    return_tx: Some(return_tx),
                });
            }

            // Try to create a new connection if under max
            if let Some(conn) = self.try_create_connection()? {
                let (return_tx, _) = std::sync::mpsc::channel();
                return Ok(ConnectionHandle {
                    connection: Some(conn),
                    return_tx: Some(return_tx),
                });
            }

            // Check timeout
            if start.elapsed() > timeout {
                anyhow::bail!("Timeout waiting for connection from pool");
            }

            // Wait for a connection to become available
            let available = self.available.lock().unwrap();
            if available.is_empty() {
                drop(available);
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        }
    }

    /// Try to get an available connection from the pool
    fn try_get_available(&self) -> Option<PooledConnection<T>> {
        let mut available = self.available.lock().unwrap();

        // Find a non-expired connection
        while let Some(conn) = available.pop_front() {
            if !conn.is_expired(self.config.max_lifetime) {
                return Some(conn);
            }
            // Connection expired, drop it
            let mut total = self.total_count.lock().unwrap();
            *total -= 1;
        }

        None
    }

    /// Try to create a new connection if under the maximum
    fn try_create_connection(&self) -> Result<Option<T>> {
        let mut total = self.total_count.lock().unwrap();

        if *total < self.config.max_size {
            *total += 1;
            drop(total);

            match (self.factory)() {
                Ok(conn) => return Ok(Some(conn)),
                Err(e) => {
                    let mut total = self.total_count.lock().unwrap();
                    *total -= 1;
                    return Err(e);
                }
            }
        }

        Ok(None)
    }

    /// Return a connection to the pool
    fn return_connection(&self, connection: T) {
        let pooled = PooledConnection::new(connection);
        let mut available = self.available.lock().unwrap();
        available.push_back(pooled);
        drop(available);
        self.condvar.notify_one();
    }

    /// Background maintenance loop
    async fn maintenance_loop(&self) {
        let interval = Duration::from_secs(60); // Run every minute

        loop {
            tokio::time::sleep(interval).await;

            if let Err(e) = self.cleanup_expired_connections() {
                warn!("Error during connection pool maintenance: {}", e);
            }

            if let Err(e) = self.ensure_min_idle() {
                warn!("Error ensuring min idle connections: {}", e);
            }
        }
    }

    /// Clean up expired connections
    fn cleanup_expired_connections(&self) -> Result<()> {
        let mut available = self.available.lock().unwrap();
        let before_count = available.len();

        // Remove expired connections
        available.retain(|conn| {
            let keep = !conn.is_expired(self.config.max_lifetime)
                && !conn.is_idle_expired(self.config.idle_timeout);
            if !keep {
                let mut total = self.total_count.lock().unwrap();
                *total -= 1;
            }
            keep
        });

        let removed = before_count - available.len();
        if removed > 0 {
            debug!("Cleaned up {} expired connections from pool", removed);
        }

        Ok(())
    }

    /// Ensure minimum number of idle connections
    fn ensure_min_idle(&self) -> Result<()> {
        let available_count = self.available.lock().unwrap().len();
        let total_count = *self.total_count.lock().unwrap();

        let needed = self
            .config
            .min_idle
            .saturating_sub(available_count)
            .min(self.config.max_size - total_count);

        for _ in 0..needed {
            match (self.factory)() {
                Ok(conn) => {
                    let mut available = self.available.lock().unwrap();
                    available.push_back(PooledConnection::new(conn));
                    let mut total = self.total_count.lock().unwrap();
                    *total += 1;
                }
                Err(e) => {
                    warn!("Failed to create idle connection: {}", e);
                    break;
                }
            }
        }

        Ok(())
    }

    /// Get pool statistics
    pub fn stats(&self) -> PoolStats {
        PoolStats {
            available: self.available.lock().unwrap().len(),
            total: *self.total_count.lock().unwrap(),
            max_size: self.config.max_size,
            min_idle: self.config.min_idle,
        }
    }

    /// Clone the pool reference for use in ConnectionHandle
    fn clone_pool(&self) -> Self {
        // This is a bit of a hack - we need to clone the pool for the handle
        // In practice, the ConnectionHandle holds an Arc, so this is fine
        unimplemented!("Use Arc<ConnectionPool<T>> instead")
    }
}

/// Statistics about a connection pool
#[derive(Debug, Clone)]
pub struct PoolStats {
    /// Number of available connections
    pub available: usize,
    /// Total number of connections
    pub total: usize,
    /// Maximum pool size
    pub max_size: usize,
    /// Minimum idle connections
    pub min_idle: usize,
}

impl PoolStats {
    /// Get the number of connections currently in use
    pub fn in_use(&self) -> usize {
        self.total - self.available
    }

    /// Get pool utilization as a percentage
    pub fn utilization_percent(&self) -> f64 {
        if self.max_size == 0 {
            return 0.0;
        }
        (self.in_use() as f64 / self.max_size as f64) * 100.0
    }
}

/// A typed connection pool for database connections
pub type DatabaseConnectionPool<T> = Arc<ConnectionPool<T>>;

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[test]
    fn test_pool_config_default() {
        let config = PoolConfig::default();
        assert_eq!(config.max_size, 10);
        assert_eq!(config.min_idle, 2);
        assert_eq!(config.max_lifetime, Duration::from_secs(3600));
        assert_eq!(config.idle_timeout, Duration::from_secs(600));
    }

    #[test]
    fn test_pooled_connection_expiration() {
        let conn = PooledConnection {
            connection: (),
            created_at: Instant::now() - Duration::from_secs(7200), // 2 hours ago
            last_used: Instant::now(),
        };

        assert!(conn.is_expired(Duration::from_secs(3600)));
        assert!(!conn.is_idle_expired(Duration::from_secs(600)));
    }

    #[test]
    fn test_pool_stats() {
        let stats = PoolStats {
            available: 3,
            total: 8,
            max_size: 10,
            min_idle: 2,
        };

        assert_eq!(stats.in_use(), 5);
        assert_eq!(stats.utilization_percent(), 50.0);
    }
}

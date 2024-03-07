use std::sync::{
    atomic::{
        AtomicU64,
        Ordering,
    },
    Arc,
};

use metrics::log_gauge;
use prometheus::Gauge;

/// Stats for a pool of connections.
#[derive(Clone)]
pub struct ConnectionPoolStats {
    active_count: Arc<AtomicU64>,
    max_count: Arc<AtomicU64>,

    active_count_gauge: &'static Gauge,
    max_count_gauge: &'static Gauge,
}

impl ConnectionPoolStats {
    pub fn new(active_count_gauge: &'static Gauge, max_count_gauge: &'static Gauge) -> Self {
        Self {
            active_count: Arc::new(AtomicU64::new(0)),
            max_count: Arc::new(AtomicU64::new(0)),
            active_count_gauge,
            max_count_gauge,
        }
    }
}

/// Tracks a single connection.
pub struct ConnectionTracker {
    active_count: Arc<AtomicU64>,
    active_count_gauge: &'static Gauge,
}

impl ConnectionTracker {
    pub fn new(stats: &ConnectionPoolStats) -> Self {
        // Increase the current count.
        let previous_count = stats.active_count.fetch_add(1, Ordering::Relaxed);
        let new_count = previous_count + 1;
        log_gauge(stats.active_count_gauge, new_count as f64);

        // Update the max count.
        let previous_max = stats.max_count.fetch_max(new_count, Ordering::SeqCst);
        let new_max = previous_max.max(new_count);
        log_gauge(stats.max_count_gauge, new_max as f64);

        Self {
            active_count: stats.active_count.clone(),
            active_count_gauge: stats.active_count_gauge,
        }
    }
}

impl Drop for ConnectionTracker {
    fn drop(&mut self) {
        // Decrease the current count.
        let previous_count = self.active_count.fetch_sub(1, Ordering::SeqCst);
        let new_count = previous_count - 1;
        log_gauge(self.active_count_gauge, new_count as f64);
    }
}

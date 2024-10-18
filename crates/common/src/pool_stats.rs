use std::sync::{
    atomic::{
        AtomicU64,
        Ordering,
    },
    Arc,
};

use metrics::{
    log_distribution_with_labels,
    log_gauge_with_labels,
    StaticMetricLabel,
};
use prometheus::{
    GaugeVec,
    VMHistogramVec,
};

/// Stats for a pool of connections.
#[derive(Clone)]
pub struct ConnectionPoolStats {
    active_count: Arc<AtomicU64>,
    max_count: Arc<AtomicU64>,

    active_count_histogram: &'static VMHistogramVec,
    max_count_gauge: &'static GaugeVec,

    labels: Vec<StaticMetricLabel>,
}

impl ConnectionPoolStats {
    pub fn new(
        active_count_histogram: &'static VMHistogramVec,
        max_count_gauge: &'static GaugeVec,
        labels: Vec<StaticMetricLabel>,
    ) -> Self {
        Self {
            active_count: Arc::new(AtomicU64::new(0)),
            max_count: Arc::new(AtomicU64::new(0)),
            active_count_histogram,
            max_count_gauge,
            labels,
        }
    }
}

/// Tracks a single connection.
pub struct ConnectionTracker {
    active_count: Arc<AtomicU64>,
    active_count_histogram: &'static VMHistogramVec,

    labels: Vec<StaticMetricLabel>,
}

impl ConnectionTracker {
    pub fn new(stats: &ConnectionPoolStats) -> Self {
        // Increase the current count.
        let previous_count = stats.active_count.fetch_add(1, Ordering::Relaxed);
        let new_count = previous_count + 1;
        log_distribution_with_labels(
            stats.active_count_histogram,
            new_count as f64,
            stats.labels.clone(),
        );

        // Update the max count.
        let previous_max = stats.max_count.fetch_max(new_count, Ordering::SeqCst);
        let new_max = previous_max.max(new_count);
        log_gauge_with_labels(stats.max_count_gauge, new_max as f64, stats.labels.clone());

        Self {
            active_count: stats.active_count.clone(),
            active_count_histogram: stats.active_count_histogram,
            labels: stats.labels.clone(),
        }
    }
}

impl Drop for ConnectionTracker {
    fn drop(&mut self) {
        // Decrease the current count.
        let previous_count = self.active_count.fetch_sub(1, Ordering::SeqCst);
        let new_count = previous_count - 1;
        log_distribution_with_labels(
            self.active_count_histogram,
            new_count as f64,
            self.labels.clone(),
        );
    }
}

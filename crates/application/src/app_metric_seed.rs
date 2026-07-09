use std::time::SystemTime;

use common::runtime::Runtime;

use crate::Application;

/// Time granularity of a seeded app-metric data point.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Granularity {
    Minute,
    Hour,
    Day,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AppMetricSeedRow {
    pub metric_name: String,
    pub granularity: Granularity,
    pub time: SystemTime,
    pub value: u64,
}

/// Signals that the seed data source failed. Delivered to the deployment
/// instead of an empty row set so it can distinguish a query error from a
/// deployment that genuinely has no historical usage, and skip hydration
/// rather than zero-fill its metric stores.
#[derive(Clone, Debug)]
pub struct AppMetricSeedError {
    pub message: String,
}

pub type AppMetricSeedResult = Result<Vec<AppMetricSeedRow>, AppMetricSeedError>;

impl<RT: Runtime> Application<RT> {
    pub fn apply_app_metric_seed(&self, result: AppMetricSeedResult) {
        let deployment_name = self.deployment_name();
        let rows = match result {
            Ok(rows) => rows,
            Err(err) => {
                // The seed query failed. Skip hydration so we don't mistake
                // the empty result for "this deployment has no usage".
                tracing::warn!(
                    "App-metrics seed query failed for {deployment_name}: {}; skipping hydration",
                    err.message,
                );
                return;
            },
        };
        // TODO(ari): hydrate dedicated per-granularity metric stores from
        // these rows. Seeded buckets must be set, not incremented, because
        // seed passes may cover overlapping windows, so adding would
        // double-count. For now this only logs a summary.
        let (mut minute, mut hour, mut day) = (0usize, 0usize, 0usize);
        for row in &rows {
            match row.granularity {
                Granularity::Minute => minute += 1,
                Granularity::Hour => hour += 1,
                Granularity::Day => day += 1,
            }
        }
        tracing::info!(
            "Received app-metrics seed for {deployment_name}: {} rows ({minute} minute, {hour} \
             hour, {day} day) [hydration not yet implemented]",
            rows.len(),
        );
    }
}

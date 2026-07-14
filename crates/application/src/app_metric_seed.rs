use std::time::SystemTime;

use common::runtime::Runtime;
use model::usage_limits::types::UsageLimitMetric;

use crate::{
    usage_limits::{
        SeedRow,
        UsageMetricResolution,
    },
    Application,
};

/// Time granularity of a seeded app-metric data point.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Granularity {
    Minute,
    Hour,
    Day,
}

impl Granularity {
    fn resolution(self) -> UsageMetricResolution {
        match self {
            Granularity::Minute => UsageMetricResolution::Minutely,
            Granularity::Hour => UsageMetricResolution::Hourly,
            Granularity::Day => UsageMetricResolution::Daily,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct AppMetricSeedRow {
    pub metric_name: String,
    pub granularity: Granularity,
    pub time: SystemTime,
    /// In the source metric's raw unit — calls, bytes, GB, or GB·s (see
    /// `UsageLimitMetric::from_seed_metric`). `f64` because the GB search and
    /// GB·s compute rollups have fractional values; call counts and byte
    /// totals are whole numbers.
    pub value: f64,
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

/// How much of a deployment's historical-usage backfill has landed. Owned by
/// the seeder — it knows its own pass protocol — and surfaced by the usage API
/// so a consumer of live usage knows whether the numbers reflect the full
/// window. Delivered alongside each seed pass to [`apply_app_metric_seed`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SeedStatus {
    /// No usable seed has landed yet: usage covers only traffic since load.
    Pending,
    /// Some history has been hydrated, but the backfill isn't finished.
    Partial,
    /// The backfill finished and hydrated history.
    Complete,
    /// The backfill finished without hydrating any history (e.g. every seed
    /// query failed).
    Failed,
}

impl<RT: Runtime> Application<RT> {
    /// Apply one seed pass: record the seeder's `status` and, when the pass
    /// carried data, hydrate the metric stores from it. The status is stored
    /// even on failure, so a terminal `Failed`/`Complete` is surfaced.
    pub fn apply_app_metric_seed(&self, result: AppMetricSeedResult, status: SeedStatus) {
        let deployment_name = self.deployment_name();
        self.usage_meter().set_seed_status(status);
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
        let num_rows = rows.len();
        let mut unknown = 0usize;
        let seed_rows: Vec<SeedRow> = rows
            .into_iter()
            .filter_map(|row| {
                let Some(metric) = UsageLimitMetric::from_seed_metric(&row.metric_name) else {
                    unknown += 1;
                    return None;
                };
                Some(SeedRow {
                    metric,
                    resolution: row.granularity.resolution(),
                    time: row.time,
                    value: row.value,
                })
            })
            .collect();
        // TODO(ENG-10808): while enforcement is log-only, monitor these
        // seeded (Databricks) totals against the live Meter. On new data the
        // Meter running higher is expected (Databricks lags); Databricks
        // higher on new data, or a gap >= 1 on historical data, is a bug.
        let num_buckets = self
            .usage_meter()
            .seed_rows(seed_rows, self.runtime().system_time());
        if unknown > 0 {
            tracing::warn!(
                "App-metrics seed for {deployment_name}: skipped {unknown} row(s) with \
                 unrecognized metric names",
            );
        }
        tracing::info!(
            "Seeded app metrics for {deployment_name}: {num_buckets} bucket(s) from {num_rows} \
             row(s)",
        );
    }
}

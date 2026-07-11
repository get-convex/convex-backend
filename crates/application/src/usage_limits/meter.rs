//! The usage meter: evaluation of configured usage limits against the
//! in-memory metric stores.

use std::time::SystemTime;

use common::types::UsageLimitStopState;
use model::usage_limits::types::{
    UsageLimitConfig,
    UsageLimitMetric,
    UsageLimitType,
};
use parking_lot::Mutex;
use tokio::sync::watch;
use value::ResolvedDocumentId;

use super::stores::{
    window_range,
    UsageMetricResolution,
    UsageMetricStores,
};

/// A limit whose window total reached its configured limit.
#[derive(Debug, Clone)]
pub struct ExceededUsageLimit {
    pub id: ResolvedDocumentId,
    pub config: UsageLimitConfig,
    /// Start of the window the limit is exceeded in.
    pub window_start: SystemTime,
}

/// Outcome of one enforcement evaluation.
#[derive(Debug)]
pub struct UsageLimitEvaluation {
    /// Every enabled limit currently at or over its configured limit.
    pub exceeded: Vec<ExceededUsageLimit>,
    /// The stop state the deployment should currently be in: `Disabled`
    /// while any enabled `Disable` limit is exceeded, `None` otherwise.
    pub desired_stop_state: UsageLimitStopState,
}

/// In-memory usage meter: owns the metric stores and the active limit
/// configs. Usage is recorded into it from the usage-event stream by
/// [`super::UsageLimitRecorder`] and evaluated against the limits by
/// [`super::UsageLimitWorker`].
pub struct UsageMeter {
    inner: Mutex<Inner>,
    has_enabled_limit_tx: watch::Sender<bool>,
}

struct Inner {
    stores: UsageMetricStores,
    configs: Vec<(ResolvedDocumentId, UsageLimitConfig)>,
}

impl UsageMeter {
    pub fn new(now: SystemTime) -> anyhow::Result<Self> {
        let (has_enabled_limit_tx, _) = watch::channel(false);
        Ok(Self {
            inner: Mutex::new(Inner {
                stores: UsageMetricStores::new(now)?,
                configs: Vec::new(),
            }),
            has_enabled_limit_tx,
        })
    }

    /// Replace the active configs, republishing whether any of them is enabled.
    pub fn refresh_configs(&self, configs: Vec<(ResolvedDocumentId, UsageLimitConfig)>) {
        let has_enabled_limit = configs.iter().any(|(_, config)| config.enabled);
        self.inner.lock().configs = configs;
        // Publish after updating so a woken observer reads the new config set.
        self.has_enabled_limit_tx.send_replace(has_enabled_limit);
    }

    /// Subscribe to whether this deployment currently has at least one enabled
    /// usage limit. The value is republished on every
    /// [`Self::refresh_configs`], so it tracks the `_usage_limits` table
    /// via [`super::UsageLimitWorker`]'s subscription — observers don't need a
    /// second subscription of their own.
    pub fn has_enabled_limit(&self) -> watch::Receiver<bool> {
        self.has_enabled_limit_tx.subscribe()
    }

    /// Record live usage deltas (raw units: calls, bytes, GB·s) that occurred
    /// at `ts` (the current time for live recording).
    ///
    /// Recording is unconditional: every metric is tracked whether or not a
    /// limit currently targets it. So enabling a limit mid-window enforces
    /// against the usage already accrued this window rather than only usage
    /// from the moment it was enabled. Enforcement stays gated on enabled
    /// configs (see [`Self::evaluate`]), and seeding stays gated on an enabled
    /// limit existing.
    pub fn record(&self, ts: SystemTime, deltas: &[(UsageLimitMetric, f64)]) {
        let mut inner = self.inner.lock();
        for (metric, delta) in deltas {
            if *delta <= 0.0 {
                continue;
            }
            inner.stores.add(metric.metric_name(), ts, *delta, ts);
        }
    }

    /// Hydrate one bucket from a seed/gap-fill row.
    pub fn seed(
        &self,
        resolution: UsageMetricResolution,
        metric_name: &str,
        ts: SystemTime,
        value: f64,
        now: SystemTime,
    ) {
        self.inner
            .lock()
            .stores
            .seed(resolution, metric_name, ts, value, now)
    }

    /// Evaluate every enabled limit against its current window. A limit is
    /// exceeded once its window total reaches the configured limit
    /// (`total >= limit`).
    pub fn evaluate(&self, now: SystemTime) -> anyhow::Result<UsageLimitEvaluation> {
        let inner = self.inner.lock();
        let mut exceeded = Vec::new();
        let mut any_disable_exceeded = false;
        for (id, config) in &inner.configs {
            if !config.enabled {
                continue;
            }
            let total =
                inner
                    .stores
                    .window_total(config.window, config.metric.metric_name(), now)?;
            if total < config.metric.limit_in_raw_units(config.limit) {
                continue;
            }
            if config.limit_type == UsageLimitType::Disable {
                any_disable_exceeded = true;
            }
            exceeded.push(ExceededUsageLimit {
                id: *id,
                config: config.clone(),
                window_start: window_range(config.window, now)?.start,
            });
        }
        Ok(UsageLimitEvaluation {
            exceeded,
            desired_stop_state: if any_disable_exceeded {
                UsageLimitStopState::Disabled
            } else {
                UsageLimitStopState::None
            },
        })
    }
}

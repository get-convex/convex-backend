//! In-memory usage metric storage for deployment usage-limit enforcement.
//!
//! Live usage is recorded from the usage-event stream as events are emitted,
//! so enforcement reads are realtime. The stores are rehydrated from
//! historical usage rollups by `AppMetricSeeder` on deployment load.
//!
//! TODO(ENG-10809): split into a `usage_limits/` module separating the store
//! model from the meter implementation.

use std::{
    collections::HashMap,
    ops::Range,
    sync::Arc,
    time::{
        Duration,
        SystemTime,
    },
};

use anyhow::Context;
use async_trait::async_trait;
use chrono::{
    DateTime,
    Datelike,
    DurationRound,
    Months,
    NaiveTime,
    TimeDelta,
    Utc,
};
use common::{
    backoff::Backoff,
    errors::report_error,
    execution_context::RequestMetadata,
    knobs::USAGE_LIMIT_EVALUATE_INTERVAL,
    log_streaming::LogSender,
    runtime::{
        Runtime,
        UnixTimestamp,
    },
    types::{
        ModuleEnvironment,
        UsageLimitStopState,
    },
};
use database::Database;
use events::usage::{
    UsageEvent,
    UsageEventLogger,
};
use futures::{
    pin_mut,
    select_biased,
    FutureExt,
};
use keybroker::Identity;
use model::{
    deployment_audit_log::{
        types::DeploymentAuditLogEvent,
        DeploymentAuditLogModel,
    },
    usage_limits::{
        types::{
            UsageLimitConfig,
            UsageLimitMetric,
            UsageLimitType,
            UsageLimitWindow,
        },
        UsageLimitsModel,
    },
};
use parking_lot::Mutex;
use udf_metrics::SeedableCounterStore;
use value::{
    DeveloperDocumentId,
    ResolvedDocumentId,
};

pub(crate) const MINUTELY_MAX_BUCKETS: u64 = 90;
pub(crate) const HOURLY_MAX_BUCKETS: u64 = 25;
pub(crate) const DAILY_MAX_BUCKETS: u64 = 32;

const MINUTELY_BUCKET_WIDTH: Duration = Duration::from_secs(60);
const HOURLY_BUCKET_WIDTH: Duration = Duration::from_secs(60 * 60);
const DAILY_BUCKET_WIDTH: Duration = Duration::from_secs(24 * 60 * 60);

/// The resolutions usage is stored at; a seed row targets exactly one of
/// them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UsageMetricResolution {
    Minutely,
    Hourly,
    Daily,
}

impl UsageMetricResolution {
    fn bucket_width(&self) -> Duration {
        match self {
            UsageMetricResolution::Minutely => MINUTELY_BUCKET_WIDTH,
            UsageMetricResolution::Hourly => HOURLY_BUCKET_WIDTH,
            UsageMetricResolution::Daily => DAILY_BUCKET_WIDTH,
        }
    }

    fn max_buckets(&self) -> u64 {
        match self {
            UsageMetricResolution::Minutely => MINUTELY_MAX_BUCKETS,
            UsageMetricResolution::Hourly => HOURLY_MAX_BUCKETS,
            UsageMetricResolution::Daily => DAILY_MAX_BUCKETS,
        }
    }
}

/// The three usage metric stores, one per resolution, sharing a `base_ts`
/// that is (a) UTC-day-aligned, so every bucket starts on its natural clock
/// boundary and window sums line up, and (b) back-dated by the daily
/// retention, so month-old seed rows land after it.
pub struct UsageMetricStores {
    minutely: SeedableCounterStore,
    hourly: SeedableCounterStore,
    daily: SeedableCounterStore,
}

impl UsageMetricStores {
    pub fn new(now: SystemTime) -> anyhow::Result<Self> {
        let base_ts = floor_to_utc_width(
            now - DAILY_BUCKET_WIDTH * DAILY_MAX_BUCKETS as u32,
            DAILY_BUCKET_WIDTH,
        )?;
        let store = |resolution: UsageMetricResolution| {
            SeedableCounterStore::new(base_ts, resolution.bucket_width(), resolution.max_buckets())
        };
        Ok(Self {
            minutely: store(UsageMetricResolution::Minutely),
            hourly: store(UsageMetricResolution::Hourly),
            daily: store(UsageMetricResolution::Daily),
        })
    }

    /// Record a live usage delta in all three resolutions. It lands in the
    /// bucket containing `ts`; a sample older than one resolution's retention
    /// is skipped there while coarser resolutions still count it.
    pub fn add(&mut self, metric_name: &str, ts: SystemTime, delta: f64, now: SystemTime) {
        self.minutely.add(metric_name, ts, delta, now);
        self.hourly.add(metric_name, ts, delta, now);
        self.daily.add(metric_name, ts, delta, now);
    }

    /// Hydrate one resolution's bucket from a seed row.
    ///
    /// `now` must be the current wall clock — it drives pruning and drops
    /// future seed rows. A row-derived `now` could prune live data.
    pub fn seed(
        &mut self,
        resolution: UsageMetricResolution,
        metric_name: &str,
        ts: SystemTime,
        value: f64,
        now: SystemTime,
    ) {
        self.store_mut(resolution)
            .seed_counter(metric_name, ts, value, now);
    }

    /// Usage within the calendar-aligned limit window containing `now`.
    ///
    /// A metric with no samples sums to 0 — which means a misspelled name
    /// reads 0 forever, so derive names from the `UsageLimitMetric` mapping,
    /// never free strings.
    pub fn window_total(
        &self,
        window: UsageLimitWindow,
        metric_name: &str,
        now: SystemTime,
    ) -> anyhow::Result<f64> {
        let range = window_range(window, now)?;
        Ok(self
            .store(window_resolution(window))
            .sum_counter(metric_name, &range))
    }

    fn store(&self, resolution: UsageMetricResolution) -> &SeedableCounterStore {
        match resolution {
            UsageMetricResolution::Minutely => &self.minutely,
            UsageMetricResolution::Hourly => &self.hourly,
            UsageMetricResolution::Daily => &self.daily,
        }
    }

    fn store_mut(&mut self, resolution: UsageMetricResolution) -> &mut SeedableCounterStore {
        match resolution {
            UsageMetricResolution::Minutely => &mut self.minutely,
            UsageMetricResolution::Hourly => &mut self.hourly,
            UsageMetricResolution::Daily => &mut self.daily,
        }
    }
}

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
/// [`UsageLimitRecorder`] and evaluated against the limits by
/// [`UsageLimitWorker`].
pub struct UsageMeter {
    inner: Mutex<Inner>,
}

struct Inner {
    stores: UsageMetricStores,
    configs: Vec<(ResolvedDocumentId, UsageLimitConfig)>,
}

impl UsageMeter {
    pub fn new(now: SystemTime) -> anyhow::Result<Self> {
        Ok(Self {
            inner: Mutex::new(Inner {
                stores: UsageMetricStores::new(now)?,
                configs: Vec::new(),
            }),
        })
    }

    /// Replace the active configs.
    pub fn refresh_configs(&self, configs: Vec<(ResolvedDocumentId, UsageLimitConfig)>) {
        self.inner.lock().configs = configs;
    }

    /// Record live usage deltas (raw units: calls, bytes, GB·s) that occurred
    /// at `ts` (the current time for live recording).
    ///
    /// Records all metrics, so a limit enabled later already has recent
    /// in-memory usage and only needs seeding for older history.
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

/// Per-metric usage deltas (raw units: calls, bytes, GB, GB·s) derived from a
/// batch of usage events.
///
/// TODO(ENG-10752): the event-to-metric mapping is provisional until the
/// seed pipeline pins which rollup feeds each metric.
pub fn usage_deltas(events: &[UsageEvent]) -> Vec<(UsageLimitMetric, f64)> {
    fn gb_s(memory_megabytes: u64, duration_millis: u64) -> f64 {
        memory_megabytes as f64 / 1024.0 * (duration_millis as f64 / 1000.0)
    }
    let mut deltas = Vec::new();
    for event in events {
        match event {
            UsageEvent::FunctionCall { fields } => {
                if fields.is_tracked {
                    deltas.push((UsageLimitMetric::FunctionCalls, 1.0));
                }
                // Compute in GB·s; the GB-hour limit converts to GB·s at
                // evaluation via `limit_in_raw_units`. Zero for calls not
                // charged for compute, e.g. cache hits.
                let compute = gb_s(fields.memory_megabytes, fields.duration_millis);
                match fields.tag.as_str() {
                    "action" | "http_action" => {
                        // Match the runtime exhaustively so a new one is a
                        // compile error.
                        match fields.environment.parse::<ModuleEnvironment>() {
                            Ok(ModuleEnvironment::Node) => {
                                deltas
                                    .push((UsageLimitMetric::ActionComputeNodeJsGBHours, compute));
                            },
                            Ok(ModuleEnvironment::Isolate) => {
                                deltas
                                    .push((UsageLimitMetric::ActionComputeConvexGBHours, compute));
                                // CPU compute (user-execution time, excluding
                                // time blocked on I/O) is charged for the
                                // Convex runtime only.
                                if let Some(user_ms) = fields.user_execution_millis {
                                    deltas.push((
                                        UsageLimitMetric::ActionComputeCpuGBHours,
                                        gb_s(fields.memory_megabytes, user_ms),
                                    ));
                                }
                            },
                            Ok(ModuleEnvironment::Invalid) | Err(_) => {},
                        }
                    },
                    // The usage pipeline groups everything that isn't an action
                    // as query/mutation compute.
                    _ => deltas.push((UsageLimitMetric::QueryMutationComputeGBHours, compute)),
                }
            },
            UsageEvent::DatabaseBandwidth {
                ingress_v2,
                egress_v2,
                ..
            } => {
                deltas.push((
                    UsageLimitMetric::DatabaseIoGB,
                    ingress_v2.saturating_add(*egress_v2) as f64,
                ));
            },
            UsageEvent::NetworkBandwidth { egress, .. } => {
                deltas.push((UsageLimitMetric::DataEgressGB, *egress as f64));
            },
            // Search usage records in GB, matching the search rollups.
            UsageEvent::TextQuery { bytes_searched, .. }
            | UsageEvent::VectorQuery { bytes_searched, .. } => {
                const BYTES_PER_GB: f64 = (1u64 << 30) as f64;
                deltas.push((
                    UsageLimitMetric::SearchQueryGB,
                    *bytes_searched as f64 / BYTES_PER_GB,
                ));
            },
            _ => {},
        }
    }
    deltas
}

/// [`UsageEventLogger`] decorator that records usage-limit deltas from the
/// usage-event stream before forwarding it. All billable usage flows through
/// this logger, so enforcement counts the same usage as billing.
pub struct UsageLimitRecorder<RT: Runtime> {
    rt: RT,
    meter: Arc<UsageMeter>,
    inner: Arc<dyn UsageEventLogger>,
}

impl<RT: Runtime> UsageLimitRecorder<RT> {
    pub fn new(rt: RT, meter: Arc<UsageMeter>, inner: Arc<dyn UsageEventLogger>) -> Arc<Self> {
        Arc::new(Self { rt, meter, inner })
    }
}

impl<RT: Runtime> std::fmt::Debug for UsageLimitRecorder<RT> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("UsageLimitRecorder").finish_non_exhaustive()
    }
}

#[async_trait]
impl<RT: Runtime> UsageEventLogger for UsageLimitRecorder<RT> {
    async fn record_async(&self, events: Vec<UsageEvent>) {
        let deltas = usage_deltas(&events);
        if !deltas.is_empty() {
            self.meter.record(self.rt.system_time(), &deltas);
        }
        self.inner.record_async(events).await
    }

    async fn shutdown(&self) -> anyhow::Result<()> {
        self.inner.shutdown().await
    }
}

const INITIAL_BACKOFF: Duration = Duration::from_secs(1);
const MAX_BACKOFF: Duration = Duration::from_secs(60);

/// Background worker that evaluates recorded usage against the configured
/// limits and records a `UsageLimitExceeded` audit event once per limit per
/// window. It loads the limit configs and subscribes to `_usage_limits`,
/// reloading whenever the table changes.
///
/// TODO(ENG-10752): set the usage-limit stop state when limits are crossed
/// so `Disable` limits actually block requests. Enforcement is log-only
/// until the metric contract is confirmed against the seed pipeline.
pub struct UsageLimitWorker<RT: Runtime> {
    rt: RT,
    database: Database<RT>,
    log_manager_client: Arc<dyn LogSender>,
    meter: Arc<UsageMeter>,
    /// Last window each limit was reported exceeded in, so we emit one
    /// `UsageLimitExceeded` audit event per limit per window. An entry is
    /// written only after its event commits, so a failed commit retries.
    /// Keyed by limit id and overwritten on rollover, so it stays bounded.
    reported: HashMap<ResolvedDocumentId, SystemTime>,
    /// Stop state implied by the last evaluation, used to log transitions
    /// exactly once. `None` forces the next evaluation to log the current
    /// state.
    last_desired: Option<UsageLimitStopState>,
}

impl<RT: Runtime> UsageLimitWorker<RT> {
    pub async fn start(
        rt: RT,
        database: Database<RT>,
        log_manager_client: Arc<dyn LogSender>,
        meter: Arc<UsageMeter>,
    ) {
        tracing::info!("Starting UsageLimitWorker");
        let mut worker = Self {
            rt,
            database,
            log_manager_client,
            meter,
            reported: HashMap::new(),
            last_desired: None,
        };
        let mut backoff = Backoff::new(INITIAL_BACKOFF, MAX_BACKOFF);
        loop {
            match worker.run().await {
                Ok(()) => backoff.reset(),
                Err(mut e) => {
                    worker.last_desired = None;
                    report_error(&mut e).await;
                    let delay = backoff.fail(&mut worker.rt.rng());
                    worker.rt.wait(delay).await;
                },
            }
        }
    }

    /// Load the limit configs, subscribe to their table, and evaluate on a
    /// fixed interval; returns when the configs change so the caller
    /// reloads.
    async fn run(&mut self) -> anyhow::Result<()> {
        let mut tx = self.database.begin(Identity::system()).await?;
        let configs = UsageLimitsModel::new(&mut tx)
            .list()
            .await?
            .into_iter()
            .map(|config| {
                let id = config.id();
                (id, config.into_value())
            })
            .collect();
        let token = tx.into_token()?;
        self.meter.refresh_configs(configs);
        let database = self.database.clone();
        let invalidated = database.subscribe_and_wait_for_invalidation(token).fuse();
        pin_mut!(invalidated);
        loop {
            self.evaluate_once().await?;
            select_biased! {
                result = invalidated => {
                    result?;
                    return Ok(());
                },
                _ = self.rt.wait(*USAGE_LIMIT_EVALUATE_INTERVAL).fuse() => {},
            }
        }
    }

    async fn evaluate_once(&mut self) -> anyhow::Result<()> {
        let now = self.rt.system_time();
        let evaluation = self.meter.evaluate(now)?;
        // TODO(ENG-10752): set the stop state via
        // `BackendStateModel::set_usage_limit_stop_state` in the transaction
        // below so exceeded `Disable` limits reject requests.
        if self.last_desired != Some(evaluation.desired_stop_state) {
            match evaluation.desired_stop_state {
                UsageLimitStopState::Disabled => tracing::warn!(
                    "Usage limit exceeded: deployment would be disabled (enforcement is log-only \
                     for now)"
                ),
                UsageLimitStopState::None => {
                    if self.last_desired.is_some() {
                        tracing::info!(
                            "Usage back under all limits: deployment would be re-enabled"
                        );
                    }
                },
            }
            self.last_desired = Some(evaluation.desired_stop_state);
        }
        let newly_exceeded: Vec<ExceededUsageLimit> = evaluation
            .exceeded
            .into_iter()
            .filter(|e| self.reported.get(&e.id) != Some(&e.window_start))
            .collect();
        if newly_exceeded.is_empty() {
            return Ok(());
        }
        // TODO(ENG-10751): this per-limit log is a placeholder. Replace it
        // with emitting the UsageLimitExceeded event so the postal service
        // emails the team, and setting the deployment's backend stop state
        // for Disable limits.
        for exceeded in &newly_exceeded {
            tracing::warn!(
                "Usage limit exceeded: {:?}/{:?}/{:?} limit of {} (id {})",
                exceeded.config.metric,
                exceeded.config.window,
                exceeded.config.limit_type,
                exceeded.config.limit,
                exceeded.id,
            );
        }
        let events: Vec<DeploymentAuditLogEvent> = newly_exceeded
            .iter()
            .map(|exceeded| DeploymentAuditLogEvent::UsageLimitExceeded {
                id: String::from(DeveloperDocumentId::from(exceeded.id)),
                config: exceeded.config.clone(),
            })
            .collect();
        let mut tx = self.database.begin(Identity::system()).await?;
        DeploymentAuditLogModel::new(&mut tx)
            .insert(events.clone(), &RequestMetadata::system())
            .await?;
        // `commit_with_audit_log_events` isn't used here because the limits
        // are marked reported between this commit and streaming the logs
        // below.
        let ts = self
            .database
            .commit_with_write_source(tx, "usage_limit_enforcement")
            .await?;
        // Mark reported once the audit events are durable, before the
        // best-effort log streaming below, so a later failure doesn't
        // re-commit the same events on the next evaluation.
        for exceeded in &newly_exceeded {
            self.reported.insert(exceeded.id, exceeded.window_start);
        }
        let logs = events
            .into_iter()
            .map(|event| {
                DeploymentAuditLogEvent::to_log_event(event, UnixTimestamp::from_nanos(ts.into()))
            })
            .collect::<anyhow::Result<Vec<_>>>()?;
        self.log_manager_client.send_logs(logs);
        Ok(())
    }
}

/// A window is summed from the resolution one step finer. Seed rollups only
/// exist for completed periods, so the in-progress hour or day is only fully
/// covered one resolution down.
fn window_resolution(window: UsageLimitWindow) -> UsageMetricResolution {
    match window {
        UsageLimitWindow::Hour => UsageMetricResolution::Minutely,
        UsageLimitWindow::Day => UsageMetricResolution::Hourly,
        UsageLimitWindow::Month => UsageMetricResolution::Daily,
    }
}

/// The calendar-aligned UTC window containing `now` (hour, day, or calendar
/// month), start-inclusive/end-exclusive to match `sum_counter`.
fn window_range(window: UsageLimitWindow, now: SystemTime) -> anyhow::Result<Range<SystemTime>> {
    match window {
        UsageLimitWindow::Hour => {
            let start = floor_to_utc_width(now, HOURLY_BUCKET_WIDTH)?;
            Ok(start..start + HOURLY_BUCKET_WIDTH)
        },
        UsageLimitWindow::Day => {
            let start = floor_to_utc_width(now, DAILY_BUCKET_WIDTH)?;
            Ok(start..start + DAILY_BUCKET_WIDTH)
        },
        UsageLimitWindow::Month => {
            let now_utc: DateTime<Utc> = now.into();
            let start = now_utc
                .date_naive()
                .with_day(1)
                .context("invalid month window start")?;
            let end = start
                .checked_add_months(Months::new(1))
                .context("invalid month window end")?;
            Ok(SystemTime::from(start.and_time(NaiveTime::MIN).and_utc())
                ..SystemTime::from(end.and_time(NaiveTime::MIN).and_utc()))
        },
    }
}

/// Floor `ts` to a multiple of `width` since the unix epoch — which is a UTC
/// midnight, so any width dividing a day lands on its natural UTC boundary.
fn floor_to_utc_width(ts: SystemTime, width: Duration) -> anyhow::Result<SystemTime> {
    let ts_utc: DateTime<Utc> = ts.into();
    let floored = ts_utc.duration_trunc(TimeDelta::from_std(width)?)?;
    Ok(floored.into())
}

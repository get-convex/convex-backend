//! The background worker that evaluates the meter against the configured
//! limits and acts on the result.

use std::{
    collections::HashMap,
    sync::Arc,
    time::{
        Duration,
        SystemTime,
    },
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
    types::UsageLimitStopState,
};
use database::Database;
use futures::{
    pin_mut,
    select_biased,
    FutureExt,
};
use keybroker::Identity;
use model::{
    backend_state::BackendStateModel,
    deployment_audit_log::{
        types::DeploymentAuditLogEvent,
        DeploymentAuditLogModel,
    },
    usage_limits::{
        types::UsageLimitWindow,
        UsageLimitsModel,
    },
};
use value::DeveloperDocumentId;

use super::{
    meter::{
        ExceededUsageLimit,
        UsageMeter,
    },
    stores::window_range,
};

const INITIAL_BACKOFF: Duration = Duration::from_secs(1);
const MAX_BACKOFF: Duration = Duration::from_secs(60);

/// Background worker that evaluates recorded usage against the configured
/// limits: it sets the deployment's usage-limit stop state when a `Disable`
/// limit is crossed or cleared, records a `UsageLimitExceeded` audit event
/// once per limit per window, and a `ChangeUsageLimitStopState` event on
/// each disable/re-enable transition. It loads the limit configs and
/// subscribes to `_usage_limits`, reloading whenever the table changes.
pub struct UsageLimitWorker<RT: Runtime> {
    rt: RT,
    database: Database<RT>,
    log_manager_client: Arc<dyn LogSender>,
    meter: Arc<UsageMeter>,
    /// Last window each limit was reported exceeded in, so we emit one
    /// `UsageLimitExceeded` audit event per limit per window. An entry is
    /// written only after its event commits, so a failed commit retries.
    /// Keyed by limit id and overwritten on rollover, so it stays bounded.
    reported: HashMap<DeveloperDocumentId, SystemTime>,
    /// Whether `reported` has been rehydrated from the audit log this
    /// process lifetime. Rehydration happens lazily, on the first evaluation
    /// that finds an exceeded limit, and keeps the once-per-limit-per-window
    /// deduplication durable across restarts.
    primed: bool,
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
            primed: false,
        };
        let mut backoff = Backoff::new(INITIAL_BACKOFF, MAX_BACKOFF);
        loop {
            match worker.run().await {
                Ok(()) => backoff.reset(),
                Err(mut e) => {
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

        // Limits at or over threshold that haven't been reported in their
        // current window yet. Prime the dedup map from the audit log on the
        // first report this process lifetime so a restart doesn't re-emit.
        let unreported = |reported: &HashMap<DeveloperDocumentId, SystemTime>,
                          e: &ExceededUsageLimit| {
            reported.get(&DeveloperDocumentId::from(e.id)) != Some(&e.window_start)
        };
        let mut newly_exceeded: Vec<ExceededUsageLimit> = evaluation
            .exceeded
            .into_iter()
            .filter(|e| unreported(&self.reported, e))
            .collect();
        if !newly_exceeded.is_empty() && !self.primed {
            self.prime_reported(now).await?;
            self.primed = true;
            newly_exceeded.retain(|e| unreported(&self.reported, e));
        }

        // Apply the desired stop state and record any audit events in one
        // transaction. `set_usage_limit_stop_state` returns the prior state
        // only when it actually changed, so the deployment's persisted state
        // is the source of truth for enable/disable transitions — restart
        // safe without extra in-memory tracking.
        let mut tx = self.database.begin(Identity::system()).await?;
        let previous = BackendStateModel::new(&mut tx)
            .set_usage_limit_stop_state(evaluation.desired_stop_state)
            .await?;

        let mut events: Vec<DeploymentAuditLogEvent> = newly_exceeded
            .iter()
            .map(|exceeded| DeploymentAuditLogEvent::UsageLimitExceeded {
                id: String::from(DeveloperDocumentId::from(exceeded.id)),
                config: exceeded.config.clone(),
            })
            .collect();
        // `previous` is `Some` only on a real change: record the transition
        // alongside the per-limit exceeded events.
        if let Some(prev) = previous {
            let old_state = prev.usage_limit;
            let new_state = evaluation.desired_stop_state;
            match new_state {
                UsageLimitStopState::Disabled => {
                    tracing::warn!("Usage limit exceeded: deployment disabled")
                },
                UsageLimitStopState::None => {
                    tracing::info!("Usage back under all limits: deployment re-enabled")
                },
            }
            events.push(DeploymentAuditLogEvent::ChangeUsageLimitStopState {
                old_state,
                new_state,
            });
        }

        // No state change and nothing new to report: drop the read-only tx.
        if events.is_empty() {
            return Ok(());
        }

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
            self.reported.insert(
                DeveloperDocumentId::from(exceeded.id),
                exceeded.window_start,
            );
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

    /// Rebuild `reported` from past `UsageLimitExceeded` audit events.
    ///
    /// Scans from the start of the current calendar month — the widest
    /// supported window, which bounds every window that could still need
    /// deduplication. Each event marks its limit reported in the window
    /// containing the event, computed from the event's own config and
    /// timestamp, so deduplication applies exactly to windows still in
    /// progress.
    async fn prime_reported(&mut self, now: SystemTime) -> anyhow::Result<()> {
        let from_ts_ms = window_range(UsageLimitWindow::Month, now)?
            .start
            .duration_since(SystemTime::UNIX_EPOCH)?
            .as_millis() as u64;
        const PAGE_SIZE: usize = 100;
        let mut cursor = None;
        loop {
            let mut tx = self.database.begin(Identity::system()).await?;
            let (entries, next_cursor) = DeploymentAuditLogModel::new(&mut tx)
                .list_events_from_time(from_ts_ms, cursor, PAGE_SIZE)
                .await?;
            for entry in entries {
                let DeploymentAuditLogEvent::UsageLimitExceeded { id, config } = entry.action
                else {
                    continue;
                };
                let Ok(id) = DeveloperDocumentId::decode(&id) else {
                    continue;
                };
                let event_ts =
                    SystemTime::UNIX_EPOCH + Duration::from_millis(entry.create_time as u64);
                // The scan is in ascending creation-time order, so the latest
                // report for each limit wins.
                self.reported
                    .insert(id, window_range(config.window, event_ts)?.start);
            }
            match next_cursor {
                Some(next_cursor) => cursor = Some(next_cursor),
                None => return Ok(()),
            }
        }
    }
}

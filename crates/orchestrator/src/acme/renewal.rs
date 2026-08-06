//! Background certificate renewal.
//!
//! Also picks up domains that have *no* certificate — a first issuance that
//! failed because DNS wasn't pointed here yet, or because the ACME server was
//! unreachable. That makes the common "I added the domain before the CNAME
//! propagated" case self-healing instead of requiring the operator to notice
//! and hit retry.

use std::time::Duration;

use crate::{
    routes::dashboard::custom_domains::spawn_issuance,
    state::OrchestratorState,
    time::now_unix_ms,
};

/// How often to look for work. Certificates last 90 days and are renewed at
/// 60, so this only has to be frequent enough to retry transient failures at
/// a sensible pace — not to hit a deadline.
const SWEEP_INTERVAL: Duration = Duration::from_secs(60 * 60);

/// Delay before the first sweep, so a restart loop can't hammer the ACME
/// server's rate limits.
const STARTUP_DELAY: Duration = Duration::from_secs(60);

pub fn spawn(state: OrchestratorState) {
    if state.config.traefik_dynamic_dir.is_none() {
        tracing::info!("custom domains disabled; not starting the renewal sweeper");
        return;
    }

    tokio::spawn(async move {
        tokio::time::sleep(STARTUP_DELAY).await;
        loop {
            if let Err(e) = sweep(&state).await {
                tracing::warn!(error = %format!("{e:#}"), "certificate renewal sweep failed");
            }
            tokio::time::sleep(SWEEP_INTERVAL).await;
        }
    });
}

async fn sweep(state: &OrchestratorState) -> anyhow::Result<()> {
    let due = state
        .storage
        .domains_needing_certificates(now_unix_ms())
        .await?;

    if due.is_empty() {
        return Ok(());
    }

    tracing::info!(count = due.len(), "issuing/renewing certificates");
    for domain in due {
        // Skip domains currently mid-issuance so a slow order isn't started
        // twice by an overlapping sweep.
        if domain.cert_state == "issuing" {
            continue;
        }
        spawn_issuance(state.clone(), domain.domain);
    }
    Ok(())
}

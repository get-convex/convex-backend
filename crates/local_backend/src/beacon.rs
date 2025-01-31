//! Beacon coroutine for collecting anonymous usage metrics to help improve
//! the self-hosted product.
//!
//! This module implements a beacon that periodically logs basic system
//! information like database version. The beacon is disabled by default
//! but can be enabled by setting CONVEX_ENABLE_BEACON=1.
//!
//! No personally identifiable information is collected. The data helps Convex
//! understand how self-hosted instances are being used to improve the product.
//! This is similar to Sentry's self-hosted telemetry approach, focusing on
//! anonymous and privacy-preserving metrics.

use std::time::{
    Duration,
    SystemTime,
};

use common::{
    backoff::Backoff,
    errors::report_error,
    runtime::Runtime,
};
use database::Database;
use model::database_globals::DatabaseGlobalsModel;
use reqwest::Client;
use runtime::prod::ProdRuntime;

const COMPILED_REVISION: &str = env!("VERGEN_GIT_SHA");
const COMMIT_TIMESTAMP: &str = env!("VERGEN_GIT_COMMIT_TIMESTAMP");
const INITIAL_BACKOFF: Duration = Duration::from_secs(1);
const MAX_BACKOFF: Duration = Duration::from_secs(900); // 15 minutes

pub async fn start_beacon(
    runtime: ProdRuntime,
    database: Database<ProdRuntime>,
    beacon_tag: String,
) {
    tracing::info!("Starting beacon coroutine...");
    let start_time = SystemTime::now();
    let mut backoff = Backoff::new(INITIAL_BACKOFF, MAX_BACKOFF);

    loop {
        let result = async {
            let mut tx = database.begin_system().await?;
            let mut db_model = DatabaseGlobalsModel::new(&mut tx);
            let globals = db_model.database_globals().await?;

            let client = Client::new();
            let uptime = SystemTime::now()
                .duration_since(start_time)
                .unwrap_or_default()
                .as_secs();

            let sent_json = serde_json::json!({
                "database_uuid": globals.id().to_string(),
                "migration_version": globals.version,
                "compiled_revision": COMPILED_REVISION,
                "commit_timestamp": COMMIT_TIMESTAMP,
                "uptime": uptime,
                "beacon_tag": beacon_tag,
            });
            let url = "https://api.convex.dev/api/self_host_beacon";
            let response = client.post(url).json(&sent_json).send().await?;

            if response.status().is_success() {
                tracing::info!(
                    "Beacon request with json {sent_json} sent successfully to {url}. This \
                     anonymized data is used to help Convex understand and improve the product. \
                     You can disable this telemetry by setting the --disable-beacon flag."
                );
            } else {
                tracing::warn!("Beacon request failed with status: {}", response.status());
            }
            Ok::<(), anyhow::Error>(())
        }
        .await;

        match result {
            Ok(()) => {
                backoff.reset();
                // Sleep for one hour before next heartbeat
                runtime.wait(Duration::from_secs(3600)).await;
            },
            Err(e) => {
                report_error(&mut e.context("Beacon coroutine error")).await;
                let delay = backoff.fail(&mut runtime.rng());
                tracing::error!("Beacon failed, retrying in {delay:?}");
                runtime.wait(delay).await;
            },
        }
    }
}

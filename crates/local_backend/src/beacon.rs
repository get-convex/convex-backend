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

use std::time::Duration;

use common::{
    backoff::Backoff,
    errors::report_error,
    runtime::Runtime,
};
use database::Database;
use model::database_globals::DatabaseGlobalsModel;
use runtime::prod::ProdRuntime;

const COMPILED_REVISION: &str = env!("VERGEN_GIT_SHA");
const COMMIT_TIMESTAMP: &str = env!("VERGEN_GIT_COMMIT_TIMESTAMP");
const INITIAL_BACKOFF: Duration = Duration::from_secs(1);
const MAX_BACKOFF: Duration = Duration::from_secs(900); // 15 minutes

pub async fn start_beacon(runtime: ProdRuntime, database: Database<ProdRuntime>) {
    tracing::info!("Starting beacon coroutine...");
    let mut backoff = Backoff::new(INITIAL_BACKOFF, MAX_BACKOFF);

    loop {
        let result = async {
            let mut tx = database.begin_system().await?;
            let mut db_model = DatabaseGlobalsModel::new(&mut tx);
            let globals = db_model.database_globals().await?;

            // For now, just log the beacon info since endpoint is TBD
            tracing::info!(
                "Beacon: document_id={:?}, database_version={}, compiled_revision={}, \
                 commit_timestamp={}",
                globals.id(),
                globals.version,
                COMPILED_REVISION,
                COMMIT_TIMESTAMP,
            );
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

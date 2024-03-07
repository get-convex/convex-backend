use async_trait::async_trait;
use common::{
    errors::report_error,
    knobs::{
        INDEX_WORKERS_INITIAL_BACKOFF,
        UDF_EXECUTOR_OCC_MAX_RETRIES,
    },
    runtime::Runtime,
};
use errors::ErrorMetadataAnyhowExt;
use sync_types::backoff::Backoff;

use crate::{
    index_workers::MAX_BACKOFF,
    Database,
};

// Overloaded means indexes are not yet ready, but if we fail to load them in
// 10ish minutes, probably something has gone wrong.
const MAX_OVERLOADED_RETRIES: usize = 20;

#[async_trait]
pub(crate) trait RetriableWorker<RT: Runtime> {
    async fn work_loop(
        &mut self,
        name: &'static str,
        rt: &RT,
        db: &Database<RT>,
        backoff: &mut Backoff,
    ) -> anyhow::Result<()>;
}

pub(crate) async fn retry_loop_expect_occs<RT: Runtime>(
    name: &'static str,
    runtime: RT,
    db: Database<RT>,
    work: impl RetriableWorker<RT>,
) {
    retry_failures(name, runtime, db, 0, work).await
}

pub(crate) async fn retry_loop_expect_occs_and_overloaded<RT: Runtime>(
    name: &'static str,
    runtime: RT,
    db: Database<RT>,
    work: impl RetriableWorker<RT>,
) {
    retry_failures(name, runtime, db, MAX_OVERLOADED_RETRIES, work).await
}

async fn retry_failures<RT: Runtime>(
    name: &'static str,
    runtime: RT,
    db: Database<RT>,
    max_overloaded_errors: usize,
    mut work: impl RetriableWorker<RT>,
) {
    let mut backoff = Backoff::new(*INDEX_WORKERS_INITIAL_BACKOFF, MAX_BACKOFF);
    let mut occ_errors = 0;
    let mut overloaded_errors = 0;

    loop {
        if let Err(e) = work.work_loop(name, &runtime, &db, &mut backoff).await {
            if e.is_occ() {
                occ_errors += 1;
                // Do not reset overloaded errors because we expect
                // overloaded to last a while during backend start and we
                // might get the occasional random OCC.
            } else if e.is_overloaded() {
                overloaded_errors += 1;
                // Reset because we got a new failure type and we don't expect OCCs to last
                // any particular period of time.
                occ_errors = 0;
            }

            // Only report OCCs that happen repeatedly
            let expected_occ = e.is_occ() && occ_errors <= *UDF_EXECUTOR_OCC_MAX_RETRIES;
            // Overloaded means indexes are not yet ready, they should eventually become
            // ready but we can be pretty patient.
            let expected_overloaded =
                e.is_overloaded() && overloaded_errors <= max_overloaded_errors;

            let expected_error = expected_occ || expected_overloaded;
            if !expected_error {
                report_error(&mut e.context(format!("{name} died")));
            }
            let delay = runtime.with_rng(|rng| backoff.fail(rng));
            tracing::error!(
                "{name} died, num_failures: {}. Backing off for {}ms, expected: {}",
                backoff.failures(),
                delay.as_millis(),
                expected_error,
            );
            runtime.wait(delay).await;
        } else {
            overloaded_errors = 0;
            occ_errors = 0;
        }
    }
}

pub mod fast_forward;
pub mod retriable_worker;
pub mod search_worker;

use std::{
    num::NonZeroU32,
    time::Duration,
};

use common::{
    knobs::{
        SEARCH_WORKER_PAGES_PER_SECOND,
        SEARCH_WORKER_PASSIVE_PAGES_PER_SECOND,
    },
    runtime::Runtime,
};
use rand::Rng;

pub const MAX_BACKOFF: Duration = Duration::from_secs(30);

#[derive(Debug, Clone, Copy)]
pub(crate) enum BuildReason {
    Backfilling,
    TooOld,
    TooLarge,
    VersionMismatch,
}

impl BuildReason {
    pub fn read_max_pages_per_second(&self) -> NonZeroU32 {
        match self {
            // In non-blocking update pathways, use a lower limit to avoid overloading the database
            // with rebuilds.
            BuildReason::TooOld | BuildReason::VersionMismatch => {
                *SEARCH_WORKER_PASSIVE_PAGES_PER_SECOND
            },
            // If the developer is writing data, then use a higher limit to try to avoid causing
            // transient 503s for the developer's writes.
            BuildReason::Backfilling | BuildReason::TooLarge => *SEARCH_WORKER_PAGES_PER_SECOND,
        }
    }
}

// Timeout from 1/2 the target duration to 1.5 the target duration.
pub async fn timeout_with_jitter<RT: Runtime>(rt: &RT, duration: Duration) {
    let half_timer = duration / 2;
    let sleep = rt.with_rng(|rng| half_timer + duration.mul_f32(rng.gen::<f32>()));
    rt.wait(sleep).await;
}

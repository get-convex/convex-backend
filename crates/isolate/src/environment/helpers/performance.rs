use std::time::Duration;

use common::runtime::{
    Runtime,
    UnixTimestamp,
};
use tokio::time::Instant;

/// Used to implement the JavaScript Performance API.
/// Stores both a base instant (used to implement [`performance.now()`](https://developer.mozilla.org/en-US/docs/Web/API/Performance/now))
/// and a base system time (used to implement [`performance.timeOrigin`](https://developer.mozilla.org/en-US/docs/Web/API/Performance/timeOrigin)).
pub struct PerformanceTimeOrigin(UnixTimestamp, Instant);

impl PerformanceTimeOrigin {
    pub fn new<RT: Runtime>(rt: &RT) -> Self {
        Self(rt.unix_timestamp(), rt.monotonic_now())
    }

    pub fn as_unix_timestamp(&self) -> UnixTimestamp {
        self.0
    }

    pub fn now<RT: Runtime>(&self, rt: &RT) -> Duration {
        let now = rt.monotonic_now();
        now.duration_since(self.1)
    }
}

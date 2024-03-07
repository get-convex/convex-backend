use std::{
    cmp,
    ops::Div,
    time::Duration,
};

use rand::Rng;

#[derive(Debug, Clone, Copy)]
pub struct Backoff {
    initial_backoff: Duration,
    max_backoff: Duration,
    num_failures: u32,
}

impl Backoff {
    pub fn new(initial_backoff: Duration, max_backoff: Duration) -> Self {
        Self {
            initial_backoff,
            max_backoff,
            num_failures: 0,
        }
    }

    pub fn reset(&mut self) {
        self.num_failures = 0;
    }

    /// Ensures that fail will return the max_backoff value the next time it is
    /// called.
    pub fn max_backoff(&mut self) {
        self.num_failures = u32::MAX.div(2);
    }

    pub fn fail(&mut self, rng: &mut impl Rng) -> Duration {
        // See https://aws.amazon.com/blogs/architecture/exponential-backoff-and-jitter/
        let p = 2u32.checked_pow(self.num_failures).unwrap_or(u32::MAX);
        self.num_failures += 1;
        let jitter = rng.gen::<f32>();
        let backoff = self
            .initial_backoff
            .checked_mul(p)
            .unwrap_or(self.max_backoff);
        cmp::min(backoff, self.max_backoff).mul_f32(jitter)
    }

    pub fn failures(&self) -> u32 {
        self.num_failures
    }
}

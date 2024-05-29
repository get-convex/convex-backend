use std::{
    ops::{
        Add,
        Sub,
    },
    time::Duration,
};

use value::heap_size::HeapSize;

use super::TestRuntime;
use crate::runtime::{
    Nanos,
    RuntimeInstant,
};

#[derive(Clone)]
pub struct TestInstant {
    pub(super) rt: TestRuntime,
    pub(super) instant: tokio::time::Instant,
}

impl RuntimeInstant for TestInstant {
    fn elapsed(&self) -> Duration {
        self.instant.elapsed()
    }

    fn as_nanos(&self) -> Nanos {
        let since_creation = self.rt.with_state(|state| {
            self.instant
                .checked_duration_since(state.creation_time)
                .expect("Created a TestInstant before creation_time?")
        });
        let nanos_u64 = u64::try_from(since_creation.as_nanos())
            .expect("Program duration lasted longer than 584 years?");
        Nanos::new(nanos_u64)
    }
}

impl HeapSize for TestInstant {
    #[inline]
    fn heap_size(&self) -> usize {
        0
    }
}

impl Sub for TestInstant {
    type Output = Duration;

    fn sub(self, rhs: Self) -> Duration {
        self.instant
            .checked_duration_since(rhs.instant)
            .unwrap_or_else(|| panic!("{:?} < {:?}", self.instant, rhs.instant))
    }
}

impl Add<Duration> for TestInstant {
    type Output = Self;

    fn add(self, rhs: Duration) -> Self {
        Self {
            rt: self.rt,
            instant: self.instant + rhs,
        }
    }
}

impl Ord for TestInstant {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.instant.cmp(&other.instant)
    }
}

impl PartialOrd for TestInstant {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Eq for TestInstant {}

impl PartialEq for TestInstant {
    fn eq(&self, other: &Self) -> bool {
        self.instant == other.instant
    }
}

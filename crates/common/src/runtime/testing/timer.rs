use std::{
    cmp::Ordering,
    collections::{
        binary_heap::PeekMut,
        BinaryHeap,
    },
    pin::Pin,
    sync::LazyLock,
    time::{
        Duration,
        SystemTime,
    },
};

use futures::{
    channel::oneshot,
    future::FusedFuture,
    FutureExt,
};

pub static CONVEX_EPOCH: LazyLock<SystemTime> =
    LazyLock::new(|| SystemTime::UNIX_EPOCH + Duration::from_secs(1620198000)); // May 5th, 2021 :)

pub const TIMER_INCREMENT: Duration = Duration::from_millis(10);

struct TimerEntry {
    time: SystemTime,
    sender: oneshot::Sender<()>,
    id: u64,
}

impl PartialEq for TimerEntry {
    fn eq(&self, other: &Self) -> bool {
        (self.id, self.time).eq(&(other.id, other.time))
    }
}
impl Eq for TimerEntry {}
impl PartialOrd for TimerEntry {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for TimerEntry {
    fn cmp(&self, other: &Self) -> Ordering {
        // Note that the order is reversed here, as we want a min-heap
        (other.time, other.id).cmp(&(self.time, self.id))
    }
}

pub struct StepTimer {
    current_time: SystemTime,
    /// Monotonically increasing id just used for satisfying the [`Eq`]
    /// requirement on [`TimerEntry`]
    id: u64,
    waiters: BinaryHeap<TimerEntry>,
    auto_complete: bool,
}

impl StepTimer {
    pub fn new() -> Self {
        StepTimer {
            current_time: *CONVEX_EPOCH,
            id: 0,
            waiters: BinaryHeap::new(),
            auto_complete: true,
        }
    }

    pub fn set_auto_complete(&mut self, on: bool) {
        self.auto_complete = on;
    }

    pub fn auto_complete(&self) -> bool {
        self.auto_complete
    }

    pub fn wait(
        &mut self,
        duration: Duration,
    ) -> Pin<Box<dyn FusedFuture<Output = ()> + Send + 'static>> {
        let (tx, rx) = oneshot::channel();
        let item = TimerEntry {
            time: self.current_time + duration,
            sender: tx,
            id: self.id,
        };
        self.waiters.push(item);
        self.id += 1;

        let future = rx.map(|r| {
            if r.is_err() {
                tracing::warn!("Waiting on timer that went away!");
            }
        });
        Box::pin(future)
    }

    /// Satisfy the next `wait` which still has a caller polling the future.
    /// If multiple `wait`s have the same duration, only the first to be queued
    /// will be satisfied.
    pub fn step(&mut self) {
        loop {
            let TimerEntry {
                time,
                sender,
                id: _,
            } = match self.waiters.pop() {
                Some(e) => e,
                None => return,
            };
            assert!(time >= self.current_time, "time going backwards!");
            self.current_time = time;
            if sender.send(()).is_err() {
                // The future was dropped; step ahead to the next one
                continue;
            }
            return;
        }
    }

    /// Advance the timer by the specified duration, satisfying all `wait`s in
    /// the interval.
    pub fn advance_time(&mut self, duration: Duration) {
        self.current_time += duration;
        let current_time = self.current_time;
        while let Some(entry) = self.waiters.peek_mut() {
            if entry.time > current_time {
                break;
            }
            // Don't worry about any failures here.
            let _ = PeekMut::pop(entry).sender.send(());
        }
    }

    /// Satisfy all outstanding `wait`s in order
    #[allow(unused)]
    pub fn complete(&mut self) {
        let mut max_time = self.current_time;
        for entry in self.waiters.drain_sorted() {
            max_time = entry.time;
            let _ = entry.sender.send(());
        }
        self.current_time = max_time;
    }

    pub fn current_time(&mut self) -> SystemTime {
        self.advance_time(TIMER_INCREMENT);
        self.current_time
    }

    pub fn timestamp_to_nanos(&self, time: SystemTime) -> u64 {
        let duration = time
            .duration_since(*CONVEX_EPOCH)
            .expect("Time provided was before the Convex epoch");
        u64::try_from(duration.as_nanos()).expect("Test duration was greater than 584 years!")
    }

    pub fn is_empty(&self) -> bool {
        self.waiters.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use futures::{
        task::{
            noop_waker_ref,
            Context,
        },
        FutureExt,
    };

    use super::{
        StepTimer,
        CONVEX_EPOCH,
    };

    #[test]
    fn test_step_timer() {
        let mut timer = StepTimer::new();
        let mut waiters = vec![];
        let mut cx = Context::from_waker(noop_waker_ref());
        for i in 1..=10 {
            waiters.push(timer.wait(Duration::from_secs(i)));
        }
        assert!(waiters
            .iter_mut()
            .all(|w| w.poll_unpin(&mut cx).is_pending()));
        timer.step();
        // Only the first future is ready.
        assert!(waiters.remove(0).poll_unpin(&mut cx).is_ready());
        assert!(waiters
            .iter_mut()
            .all(|w| w.poll_unpin(&mut cx).is_pending()));
        // Should be one second past CONVEX_EPOCH
        assert_eq!(
            timer
                .current_time()
                .duration_since(*CONVEX_EPOCH)
                .unwrap()
                .as_secs(),
            1
        );

        // Advance time by two seconds; the next two futures should be ready.
        timer.advance_time(Duration::from_secs(2));
        assert!(waiters[..2]
            .iter_mut()
            .all(|w| w.poll_unpin(&mut cx).is_ready()));
        let mut waiters = waiters.split_off(2);
        assert!(waiters
            .iter_mut()
            .all(|w| w.poll_unpin(&mut cx).is_pending()));

        // Complete all pending futures.
        timer.complete();
        assert!(waiters.iter_mut().all(|w| w.poll_unpin(&mut cx).is_ready()));
        // Should be ten seconds past CONVEX_EPOCH
        assert_eq!(
            timer
                .current_time()
                .duration_since(*CONVEX_EPOCH)
                .unwrap()
                .as_secs(),
            10
        );
    }
}

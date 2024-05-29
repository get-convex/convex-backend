use std::{
    collections::BTreeMap,
    sync::Arc,
    time::{
        Duration,
        Instant,
    },
};

use common::runtime::Runtime;
use futures::Future;
use parking_lot::Mutex;

use crate::metrics::{
    concurrency_permit_acquire_timer,
    log_concurrency_permit_used,
};

/// ConcurrencyLimiter is used to limit how many isolate threads can be
/// actively running at the same time. We use it to avoid oversubscribing the
/// CPU, which would result in unexpected user timeouts and arbitrary slow
/// downs throughout the system. Note that async-channel is implemented as a
/// linked-list which results in a constant memory overhead per permit.
#[derive(Clone, Debug)]
pub struct ConcurrencyLimiter {
    tx: async_channel::Sender<()>,
    rx: async_channel::Receiver<()>,

    tracker: Arc<Mutex<ActivePermitsTracker>>,
}

impl ConcurrencyLimiter {
    pub fn new(max_concurrency: usize) -> Self {
        assert!(
            max_concurrency > 0,
            "max_concurrency must be greater than zero"
        );
        let (tx, rx) = async_channel::bounded(max_concurrency);
        let tracker = Arc::new(Mutex::new(ActivePermitsTracker::new()));
        Self { tx, rx, tracker }
    }

    pub fn unlimited() -> Self {
        let (tx, rx) = async_channel::unbounded();
        let tracker = Arc::new(Mutex::new(ActivePermitsTracker::new()));
        Self { tx, rx, tracker }
    }

    // TODO(presley): Replace this when we have isolate_v2.
    // If a client uses a thread for too long. We still want to log periodically.
    pub fn go_log<RT: Runtime>(&self, rt: RT, frequency: Duration) -> impl Future<Output = ()> {
        let tracker = self.tracker.clone();
        async move {
            loop {
                rt.wait(frequency).await;
                let current_permits = tracker.lock().reset_start_time();
                for (client_id, start_time) in current_permits {
                    if start_time.elapsed() >= frequency {
                        tracing::warn!(
                            "{client_id} held concurrency semaphore for more than {frequency:?}"
                        );
                    }
                    log_concurrency_permit_used(client_id, start_time.elapsed());
                }
            }
        }
    }

    pub async fn acquire(&self, client_id: Arc<String>) -> ConcurrencyPermit {
        let timer = concurrency_permit_acquire_timer();
        self.tx
            .send(())
            .await
            .expect("Failed to send a message while holding reader");
        let permit_id = self.tracker.lock().register(client_id.clone());
        timer.finish(true);
        ConcurrencyPermit {
            permit_id,
            rx: self.rx.clone(),
            limiter: self.clone(),
            client_id,
        }
    }
}

#[derive(Debug, Clone, Copy, Ord, PartialOrd, Eq, PartialEq)]
struct PermitId(usize);

// This is allow us to track the currently acquired permits.
// TODO(presley): Remove this when we have isolate_v2.
#[derive(Debug)]
struct ActivePermitsTracker {
    // Generate a separate id for each concurrency limit to simplify deregistering.
    active_permits: BTreeMap<PermitId, (Arc<String>, Instant)>,
    next_permit_id: usize,
}

impl ActivePermitsTracker {
    fn new() -> Self {
        Self {
            active_permits: BTreeMap::new(),
            next_permit_id: 0,
        }
    }

    fn register(&mut self, client_id: Arc<String>) -> PermitId {
        let permit_id = PermitId(self.next_permit_id + 1);
        self.next_permit_id += 1;
        self.active_permits
            .insert(permit_id, (client_id, Instant::now()));
        permit_id
    }

    fn deregister(&mut self, id: PermitId) -> Duration {
        let (_, start) = self
            .active_permits
            .remove(&id)
            .expect("Deregistered unregistered id");
        start.elapsed()
    }

    fn reset_start_time(&mut self) -> Vec<(Arc<String>, Instant)> {
        let now = Instant::now();
        let result = self.active_permits.values().cloned().collect();
        for (_, (_, start)) in self.active_permits.iter_mut() {
            *start = now;
        }
        result
    }
}

#[derive(Debug)]
pub struct ConcurrencyPermit {
    permit_id: PermitId,

    rx: async_channel::Receiver<()>,
    limiter: ConcurrencyLimiter,
    client_id: Arc<String>,
}

impl ConcurrencyPermit {
    pub async fn with_suspend<'a, T>(
        self,
        f: impl Future<Output = T> + 'a,
    ) -> (T, ConcurrencyPermit) {
        let client_id = self.client_id.clone();
        let limiter = self.limiter.clone();
        drop(self);
        let result = f.await;
        let permit = limiter.acquire(client_id).await;
        (result, permit)
    }

    pub fn limiter(&self) -> &ConcurrencyLimiter {
        &self.limiter
    }
}

impl Drop for ConcurrencyPermit {
    fn drop(&mut self) {
        self.rx.try_recv().expect("Failed to read the item we sent");
        let duration = self.limiter.tracker.lock().deregister(self.permit_id);
        log_concurrency_permit_used(self.client_id.clone(), duration);
    }
}

#[cfg(test)]
mod tests {
    use std::{
        sync::Arc,
        time::Duration,
    };

    use common::runtime::Runtime;
    use futures::{
        select_biased,
        FutureExt,
    };
    use runtime::testing::TestDriver;

    use crate::ConcurrencyLimiter;

    #[test]
    fn test_limiter() -> anyhow::Result<()> {
        let td = TestDriver::new();
        let rt = td.rt();
        let limiter = ConcurrencyLimiter::new(8);

        // Acquire all permits.
        let mut permits = Vec::new();
        for _ in 0..8 {
            permits.push(td.run_until(limiter.acquire(Arc::new("test".to_owned()))));
        }

        // Taking another permit should fail.
        let result = td.run_until(async {
            select_biased! {
                permit = limiter.acquire(Arc::new("test".to_owned())).fuse() => Ok(permit),
                _ = rt.wait(Duration::from_secs(1)) => { anyhow::bail!("Time out"); }
            }
        });
        assert!(result.is_err());

        // Dropping two permits should allow us to reacquire them.
        for _ in 0..2 {
            permits.pop();
        }
        for _ in 0..2 {
            permits.push(td.run_until(limiter.acquire(Arc::new("test".to_owned()))));
        }
        let result = td.run_until(async {
            select_biased! {
                permit = limiter.acquire(Arc::new("test".to_owned())).fuse() => Ok(permit),
                _ = rt.wait(Duration::from_secs(1)) => { anyhow::bail!("Time out"); }
            }
        });
        assert!(result.is_err());

        Ok(())
    }
}

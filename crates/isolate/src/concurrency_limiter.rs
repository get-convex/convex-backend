use futures::Future;

use crate::metrics::concurrency_permit_acquire_timer;

/// ConcurrencyLimiter is used to limit how many isolate threads can be
/// actively running at the same time. We use it to avoid oversubscribing the
/// CPU, which would result in unexpected user timeouts and arbitrary slow
/// downs throughout the system. Note that async-channel is implemented as a
/// linked-list which results in a constant memory overhead per permit.
#[derive(Clone, Debug)]
pub struct ConcurrencyLimiter {
    tx: async_channel::Sender<()>,
    rx: async_channel::Receiver<()>,
}

impl ConcurrencyLimiter {
    pub fn new(max_concurrency: usize) -> Self {
        assert!(
            max_concurrency > 0,
            "max_concurrency must be greater than zero"
        );
        let (tx, rx) = async_channel::bounded(max_concurrency);
        Self { tx, rx }
    }

    pub fn unlimited() -> Self {
        let (tx, rx) = async_channel::unbounded();
        Self { tx, rx }
    }

    pub async fn acquire(&self) -> ConcurrencyPermit {
        let timer = concurrency_permit_acquire_timer();
        self.tx
            .send(())
            .await
            .expect("Failed to send a message while holding reader");
        timer.finish(true);
        ConcurrencyPermit {
            rx: self.rx.clone(),
            limiter: self.clone(),
        }
    }
}

#[derive(Debug)]
pub struct ConcurrencyPermit {
    rx: async_channel::Receiver<()>,
    limiter: ConcurrencyLimiter,
}

impl ConcurrencyPermit {
    pub async fn with_suspend<'a, T>(
        self,
        f: impl Future<Output = T> + 'a,
    ) -> (T, ConcurrencyPermit) {
        let limiter = self.limiter.clone();
        drop(self);
        let result = f.await;
        let permit = limiter.acquire().await;
        (result, permit)
    }

    pub fn limiter(&self) -> &ConcurrencyLimiter {
        &self.limiter
    }
}

impl Drop for ConcurrencyPermit {
    fn drop(&mut self) {
        self.rx.try_recv().expect("Failed to read the item we sent")
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use common::runtime::Runtime;
    use futures::{
        select_biased,
        FutureExt,
    };
    use runtime::testing::TestDriver;

    use crate::ConcurrencyLimiter;

    #[test]
    fn test_limiter() -> anyhow::Result<()> {
        let mut td = TestDriver::new();
        let rt = td.rt();
        let limiter = ConcurrencyLimiter::new(8);

        // Acquire all permits.
        let mut permits = Vec::new();
        for _ in 0..8 {
            permits.push(td.run_until(limiter.acquire()));
        }

        // Taking another permit should fail.
        let result = td.run_until(async {
            select_biased! {
                permit = limiter.acquire().fuse() => Ok(permit),
                _ = rt.wait(Duration::from_secs(1)) => { anyhow::bail!("Time out"); }
            }
        });
        assert!(result.is_err());

        // Dropping two permits should allow us to reacquire them.
        for _ in 0..2 {
            permits.pop();
        }
        for _ in 0..2 {
            permits.push(td.run_until(limiter.acquire()));
        }
        let result = td.run_until(async {
            select_biased! {
                permit = limiter.acquire().fuse() => Ok(permit),
                _ = rt.wait(Duration::from_secs(1)) => { anyhow::bail!("Time out"); }
            }
        });
        assert!(result.is_err());

        Ok(())
    }
}

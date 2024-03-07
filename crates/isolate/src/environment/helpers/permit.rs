// Similar to releasing the GIL in Python, it's advisable to drop the
// ConcurrencyPermit when entering async code on the V8 thread. This helper also
// integrates with our user time tracking to not count async code against the
// user timeout.
use common::runtime::Runtime;
use futures::Future;

use crate::{
    concurrency_limiter::ConcurrencyPermit,
    timeout::Timeout,
};

pub async fn with_release_permit<RT: Runtime, T>(
    timeout: &mut Timeout<RT>,
    permit_slot: &mut Option<ConcurrencyPermit>,
    f: impl Future<Output = anyhow::Result<T>>,
) -> anyhow::Result<T> {
    let permit = permit_slot.take().expect("permit should exist");
    let f = timeout.with_timeout(permit.with_suspend(f));
    let pause_guard = timeout.pause();
    let (result, permit) = f.await?;
    pause_guard.resume();
    *permit_slot = Some(permit);
    result
}

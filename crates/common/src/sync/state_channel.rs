use std::{
    collections::HashMap,
    future::Future,
    hash::Hash,
    pin::Pin,
    sync::Arc,
    task::{
        Context,
        Poll,
        Waker,
    },
};

use parking_lot::Mutex;
use thiserror::Error;

/// Create a state channel that synchronizes some `Copy` state between a single
/// sender and multiple receivers.
///
/// Receivers can asynchronously `wait_for` a particular value and receive a
/// notification when the value matches or if the sender has closed (or dropped)
/// its half.
pub fn new_state_channel<T: Copy + Eq + Hash + Unpin>(
    initial_value: T,
) -> (StateChannelSender<T>, StateChannelReceiver<T>) {
    let mut visits = HashMap::new();
    visits.insert(initial_value, 1);
    let inner = StateChannelInner {
        current_state: initial_value,
        version: 0,
        wakers: HashMap::new(),
        closed: false,
    };
    let inner = Arc::new(Mutex::new(inner));
    (
        StateChannelSender {
            inner: inner.clone(),
        },
        StateChannelReceiver { inner },
    )
}

#[derive(Debug, Error, Eq, PartialEq)]
#[error("Send on closed state channel")]
pub struct ClosedError;

#[derive(Clone)]
pub struct StateChannelReceiver<T: Copy + Eq + Hash + Unpin> {
    inner: Arc<Mutex<StateChannelInner<T>>>,
}

impl<T: Copy + Eq + Hash + Unpin> StateChannelReceiver<T> {
    pub fn current_state(&self) -> Result<T, ClosedError> {
        let inner = self.inner.lock();
        if inner.closed {
            return Err(ClosedError);
        }
        Ok(inner.current_state)
    }

    /// Wait for the state channel to have the given value, failing with a
    /// `ClosedError` if it's subsequently closed.
    pub fn wait_for(&self, value: T) -> impl Future<Output = Result<(), ClosedError>> {
        StateChannelFuture {
            waiting_for: value,
            initial_version: None,
            inner: self.inner.clone(),
        }
    }
}

struct StateChannelFuture<T: Copy + Eq + Hash + Unpin> {
    waiting_for: T,
    // What was the version of the state channel when we were first polled?
    initial_version: Option<usize>,
    inner: Arc<Mutex<StateChannelInner<T>>>,
}

impl<T: Copy + Eq + Hash + Unpin> Future for StateChannelFuture<T> {
    type Output = Result<(), ClosedError>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let self_ = self.get_mut();
        let mut inner = self_.inner.lock();
        let closed = inner.closed;

        let initial_version = match self_.initial_version {
            // Case 1: We're getting polled for the first time and want to decide if we should
            // suspend.
            None => {
                // If our state already matches or the channel's closed, return immediately.
                if inner.current_state == self_.waiting_for {
                    return Poll::Ready(Ok(()));
                }
                if closed {
                    return Poll::Ready(Err(ClosedError));
                }
                // Otherwise, save our waker and suspend.
                let initial_version = inner.version;
                inner
                    .wakers
                    .entry(self_.waiting_for)
                    .or_insert_with(|| WakerList {
                        initial_version,
                        wakers: vec![],
                    })
                    .wakers
                    .push(cx.waker().clone());
                self_.initial_version = Some(initial_version);
                return Poll::Pending;
            },
            Some(v) => v,
        };

        // Case 2: We're being polled after previously suspending.
        match inner.wakers.get_mut(&self_.waiting_for) {
            // Case 2a: The sender set our value, removing the waker list, and no one's
            // subsequently waited on the same value.
            None => Poll::Ready(Ok(())),
            // Case 2b: The sender set our value, removing the waker list, but someone else
            // subsequently waited on the same value and reinserted the waker list at a higher
            // version.
            Some(ref waker_list) if waker_list.initial_version > initial_version => {
                Poll::Ready(Ok(()))
            },
            // Case 2c: Our waker list is still up-to-date, and we're ready to suspend. Check
            // whether the channel is closed before pushing our waker onto the waker list.
            Some(ref mut waker_list) => {
                assert_eq!(waker_list.initial_version, initial_version);
                if closed {
                    return Poll::Ready(Err(ClosedError));
                }
                let waker = cx.waker();
                let found_match = waker_list.wakers.iter().any(|w| w.will_wake(waker));
                if !found_match {
                    waker_list.wakers.push(waker.clone());
                }
                Poll::Pending
            },
        }
    }
}

pub struct StateChannelSender<T: Copy + Eq + Hash + Unpin> {
    inner: Arc<Mutex<StateChannelInner<T>>>,
}

impl<T: Copy + Eq + Hash + Unpin> StateChannelSender<T> {
    pub fn set(&self, value: T) -> bool {
        let ready = {
            let mut inner = self.inner.lock();
            assert!(!inner.closed, "Live sender observing closed channel?");
            if inner.current_state == value {
                return false;
            }
            inner.current_state = value;
            inner.version += 1;

            inner
                .wakers
                .remove(&value)
                .map(|w| w.wakers)
                .unwrap_or_default()
        };
        // NB: `waker.wake()` could run arbitrary code that could potentially try to
        // acquire `self.inner`. Be sure to call it outside of the lock.
        for waker in ready {
            waker.wake();
        }
        true
    }

    pub fn close(self) {
        drop(self);
    }
}

impl<T: Copy + Eq + Hash + Unpin> Drop for StateChannelSender<T> {
    fn drop(&mut self) {
        // Wake up all of the wakers, but leave their waker lists intact since we're not
        // actually setting a new value.
        let wakers: Vec<_> = {
            let mut inner = self.inner.lock();
            inner.closed = true;
            inner
                .wakers
                .iter_mut()
                .flat_map(|(_, w)| w.wakers.drain(..))
                .collect()
        };
        for waker in wakers {
            waker.wake();
        }
    }
}

struct WakerList {
    // State version number when the first waiter was inserted.
    initial_version: usize,
    wakers: Vec<Waker>,
}

struct StateChannelInner<T: Copy + Eq + Hash + Unpin> {
    current_state: T,
    version: usize,
    wakers: HashMap<T, WakerList>,
    closed: bool,
}

#[cfg(test)]
mod tests {
    use std::{
        cell::RefCell,
        future::Future,
        rc::Rc,
        task::{
            Context,
            Poll,
        },
    };

    use futures::{
        executor::LocalPool,
        task::{
            noop_waker_ref,
            LocalSpawnExt,
        },
    };

    use super::{
        new_state_channel,
        ClosedError,
    };

    #[test]
    fn test_state_channel() {
        let waker = noop_waker_ref();
        let mut cx = Context::from_waker(waker);

        // Test receiving on an immediately closed channel.
        let (tx, rx) = new_state_channel(0);
        tx.close();

        let wait = rx.wait_for(1);
        futures::pin_mut!(wait);
        assert_eq!(wait.as_mut().poll(&mut cx), Poll::Ready(Err(ClosedError)));

        // Test closing while waiting.
        let (tx, rx) = new_state_channel(0);

        let wait = rx.wait_for(1);
        futures::pin_mut!(wait);

        assert_eq!(wait.as_mut().poll(&mut cx), Poll::Pending);

        tx.close();
        assert_eq!(wait.as_mut().poll(&mut cx), Poll::Ready(Err(ClosedError)));

        // Test waiting for the wrong value.
        let (tx, rx) = new_state_channel(0);

        let wait = rx.wait_for(1);
        futures::pin_mut!(wait);

        assert!(tx.set(2));
        assert_eq!(wait.as_mut().poll(&mut cx), Poll::Pending);

        // Test setting the same value multiple times.
        let (tx, rx) = new_state_channel(0);

        let wait = rx.wait_for(1);
        futures::pin_mut!(wait);

        assert!(tx.set(1));
        assert!(!tx.set(1));

        assert_eq!(wait.as_mut().poll(&mut cx), Poll::Ready(Ok(())));

        // Test multiple waiters.
        let (tx, rx) = new_state_channel(0);

        let wait1 = rx.wait_for(1);
        let wait2 = rx.wait_for(1);
        let wait3 = rx.wait_for(2);
        futures::pin_mut!(wait1);
        futures::pin_mut!(wait2);
        futures::pin_mut!(wait3);

        assert_eq!(wait1.as_mut().poll(&mut cx), Poll::Pending);
        assert_eq!(wait2.as_mut().poll(&mut cx), Poll::Pending);
        assert_eq!(wait3.as_mut().poll(&mut cx), Poll::Pending);

        assert!(tx.set(2));

        assert_eq!(wait1.as_mut().poll(&mut cx), Poll::Pending);
        assert_eq!(wait2.as_mut().poll(&mut cx), Poll::Pending);
        assert_eq!(wait3.as_mut().poll(&mut cx), Poll::Ready(Ok(())));

        assert!(tx.set(1));

        assert_eq!(wait1.as_mut().poll(&mut cx), Poll::Ready(Ok(())));
        assert_eq!(wait2.as_mut().poll(&mut cx), Poll::Ready(Ok(())));
    }

    #[test]
    fn test_aba_race() {
        let waker = noop_waker_ref();
        let mut cx = Context::from_waker(waker);

        let (tx, rx) = new_state_channel(0);

        let wait = rx.wait_for(1);
        futures::pin_mut!(wait);

        assert_eq!(wait.as_mut().poll(&mut cx), Poll::Pending);

        assert!(tx.set(1));
        assert!(tx.set(0));

        assert_eq!(wait.as_mut().poll(&mut cx), Poll::Ready(Ok(())));
    }

    #[test]
    fn test_with_local_pool() -> anyhow::Result<()> {
        let mut pool = LocalPool::new();
        let spawner = pool.spawner();
        let finished = Rc::new(RefCell::new(0));

        let (tx, rx) = new_state_channel(0);

        let rx_ = rx.clone();
        let finished_ = finished.clone();
        let future1 = async move {
            rx_.wait_for(1).await.unwrap();
            *finished_.borrow_mut() += 1;
        };

        let finished_ = finished.clone();
        let future2 = async move {
            rx.wait_for(2).await.unwrap();
            *finished_.borrow_mut() += 1;
        };

        spawner.spawn_local(future1)?;
        spawner.spawn_local(future2)?;

        pool.run_until_stalled();
        assert_eq!(*finished.borrow(), 0);

        tx.set(3);

        pool.run_until_stalled();
        assert_eq!(*finished.borrow(), 0);

        tx.set(2);

        pool.run_until_stalled();
        assert_eq!(*finished.borrow(), 1);

        tx.set(1);

        pool.run_until_stalled();
        assert_eq!(*finished.borrow(), 2);

        Ok(())
    }
}

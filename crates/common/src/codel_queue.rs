use std::{
    collections::VecDeque,
    pin::Pin,
    sync::Arc,
    task::{
        Context,
        Poll,
    },
    time::Duration,
};

use event_listener::Event;
use futures::{
    Future,
    Stream,
};
use parking_lot::Mutex;

use crate::{
    knobs::{
        CODEL_QUEUE_CONGESTED_EXPIRATION_MILLIS,
        CODEL_QUEUE_IDLE_EXPIRATION_MILLIS,
    },
    metrics::{
        log_codel_queue_overloaded,
        log_codel_queue_size,
        log_codel_queue_time_since_empty,
    },
    runtime::Runtime,
};

#[derive(thiserror::Error, Debug)]
#[error("Queue full")]
pub struct QueueFull;

/// Instead of simply dropping items from the queue,
/// we return expired items so the caller can dispose of them.
#[derive(thiserror::Error, Debug, PartialEq, Eq)]
#[error("Item expired in queue")]
pub struct ExpiredInQueue;

/// Queue for buffering requests while avoiding consistently large latency.
/// Following the algorithm described at https://queue.acm.org/detail.cfm?id=2839461
///
/// There's an alternate C++
/// implementation at https://github.com/facebook/folly/blob/main/folly/executors/Codel.cpp
/// which was not used in the making of this implementation.
pub struct CoDelQueue<RT: Runtime, T> {
    rt: RT,
    buffer: VecDeque<(T, RT::Instant)>,
    capacity: usize,
    last_time_empty: RT::Instant,
    idle_expiration: Duration,
    congested_expiration: Duration,
}

impl<RT: Runtime, T> CoDelQueue<RT, T> {
    pub fn new(rt: RT, capacity: usize) -> Self {
        let last_time_empty = rt.monotonic_now();
        Self {
            rt,
            buffer: VecDeque::new(),
            capacity,
            last_time_empty,
            idle_expiration: *CODEL_QUEUE_IDLE_EXPIRATION_MILLIS,
            congested_expiration: *CODEL_QUEUE_CONGESTED_EXPIRATION_MILLIS,
        }
    }

    #[cfg(test)]
    fn new_for_test(
        rt: RT,
        capacity: usize,
        idle_expiration: Duration,
        congested_expiration: Duration,
    ) -> Self {
        let last_time_empty = rt.monotonic_now();
        Self {
            rt,
            buffer: VecDeque::new(),
            capacity,
            last_time_empty,
            idle_expiration,
            congested_expiration,
        }
    }

    pub fn len(&self) -> usize {
        self.buffer.len()
    }

    pub fn is_empty(&self) -> bool {
        self.buffer.is_empty()
    }

    fn is_idle(&mut self) -> bool {
        // If queue is currently empty, set last_empty_time=now() which makes
        // the idle condition true.
        self.update_last_time_empty();
        self._is_idle()
    }

    fn _is_idle(&self) -> bool {
        (self.last_time_empty.clone() + self.idle_expiration) > self.rt.monotonic_now()
    }

    fn update_last_time_empty(&mut self) {
        if self.is_empty() {
            self.last_time_empty = self.rt.monotonic_now();
        }
        self.log_metrics();
    }

    fn log_metrics(&self) {
        log_codel_queue_size(self.len());
        log_codel_queue_overloaded(!self._is_idle());
        log_codel_queue_time_since_empty(self.rt.monotonic_now() - self.last_time_empty.clone())
    }

    pub fn push(&mut self, item: T) -> Result<(), QueueFull> {
        if self.len() >= self.capacity {
            return Err(QueueFull);
        }
        self.update_last_time_empty();
        let expiration = if self.is_idle() {
            self.idle_expiration
        } else {
            self.congested_expiration
        };
        let deadline = self.rt.monotonic_now() + expiration;
        self.buffer.push_back((item, deadline));
        Ok(())
    }

    fn pop_front(&mut self) -> Option<(T, RT::Instant)> {
        let result = self.buffer.pop_front();
        // If the queue is newly empty, update last_empty_time=now().
        // This is redundant since it will remain empty and that will only
        // matter if we check is_idle, which will also update last_empty_time.
        // But it doesn't hurt to keep it updated.
        self.update_last_time_empty();
        result
    }

    fn pop_back(&mut self) -> Option<(T, RT::Instant)> {
        let result = self.buffer.pop_back();
        self.update_last_time_empty();
        result
    }

    pub fn pop(&mut self) -> Option<(T, Option<ExpiredInQueue>)> {
        let now = self.rt.monotonic_now();
        let result = if let Some((_, oldest_expiration)) = self.buffer.front()
            && oldest_expiration < &now
        {
            // Drain expired item.
            self.pop_front()
        } else if self.is_idle() {
            // FIFO
            self.pop_front()
        } else {
            // LIFO
            self.pop_back()
        };
        match result {
            None => None,
            Some((item, expiration)) if expiration < now => Some((item, Some(ExpiredInQueue))),
            Some((item, _)) => Some((item, None)),
        }
    }
}

/// Wrapper around CoDelQueue that makes it async.
pub fn new_codel_queue_async<RT: Runtime, T>(
    rt: RT,
    capacity: usize,
) -> (CoDelQueueSender<RT, T>, CoDelQueueReceiver<RT, T>) {
    let inner = Arc::new(Mutex::new(Inner {
        queue: CoDelQueue::new(rt, capacity),
        event: Event::new(),
        senders: 1,
    }));
    (
        CoDelQueueSender {
            inner: inner.clone(),
        },
        CoDelQueueReceiver {
            inner,
            listener: None,
        },
    )
}

struct Inner<RT: Runtime, T> {
    queue: CoDelQueue<RT, T>,
    event: Event,
    senders: usize,
}

pub struct CoDelQueueReceiver<RT: Runtime, T> {
    inner: Arc<Mutex<Inner<RT, T>>>,
    listener: Option<event_listener::EventListener>,
}

impl<RT: Runtime, T> Clone for CoDelQueueReceiver<RT, T> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            listener: None,
        }
    }
}

pub struct CoDelQueueSender<RT: Runtime, T> {
    inner: Arc<Mutex<Inner<RT, T>>>,
}

impl<RT: Runtime, T> Clone for CoDelQueueSender<RT, T> {
    fn clone(&self) -> Self {
        self.inner.lock().senders += 1;
        Self {
            inner: self.inner.clone(),
        }
    }
}

impl<RT: Runtime, T> Drop for CoDelQueueSender<RT, T> {
    fn drop(&mut self) {
        let mut inner = self.inner.lock();
        inner.senders -= 1;
        if inner.senders == 0 {
            // Queue is closed. Wake up all receivers so they return None.
            inner.event.notify(usize::MAX);
        }
    }
}

impl<RT: Runtime, T> CoDelQueueSender<RT, T> {
    pub fn try_send(&self, item: T) -> Result<(), QueueFull> {
        let mut inner = self.inner.lock();
        inner.queue.push(item)?;
        inner.event.notify_additional(1);
        Ok(())
    }
}

impl<RT: Runtime, T> Stream for CoDelQueueReceiver<RT, T> {
    type Item = (T, Option<ExpiredInQueue>);

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let s = &mut *self;
        let mut inner = s.inner.lock();
        // If there is an item in the queue, pop it.
        // If the queue is closed, return None.
        if let Some(result) = inner.queue.pop() {
            return Poll::Ready(Some(result));
        } else if inner.senders == 0 {
            return Poll::Ready(None);
        }

        // Now we are waiting for the queue to become nonempty.
        loop {
            let listener = s.listener.get_or_insert_with(|| inner.event.listen());
            match Pin::new(listener).poll(cx) {
                // The queue is still empty. The listener is stored for the next
                // poll, and it has registered with cx.waker to be woken when
                // it is notified of the queue becoming nonempty.
                Poll::Pending => return Poll::Pending,
                Poll::Ready(()) => {
                    // This should not happen, because the listener is only notified
                    // when the queue state changes, which is impossible while we are
                    // holding self.inner.lock(). But we can be defensive in case of
                    // spurious wakeups, by dropping the listener and looping.
                    s.listener.take();
                    continue;
                },
            }
        }
    }
}

#[cfg(test)]
mod codel_queue_tests {
    use std::time::Duration;

    use super::CoDelQueue;
    use crate::{
        codel_queue::ExpiredInQueue,
        runtime::{
            testing::TestDriver,
            Runtime,
        },
    };

    #[test]
    fn test_fifo() -> anyhow::Result<()> {
        let td = TestDriver::new();
        let rt = td.rt();
        let mut queue = CoDelQueue::new(rt, 2);
        queue.push(1)?;
        queue.push(2)?;
        assert!(queue.push(3).is_err());
        assert_eq!(queue.pop(), Some((1, None)));
        queue.push(4)?;
        assert_eq!(queue.len(), 2);
        assert_eq!(queue.pop(), Some((2, None)));
        assert_eq!(queue.pop(), Some((4, None)));
        assert_eq!(queue.pop(), None);
        queue.push(5)?;
        assert_eq!(queue.pop(), Some((5, None)));
        assert!(queue.is_empty());
        Ok(())
    }

    #[test]
    fn test_adaptive_lifo() -> anyhow::Result<()> {
        let td = TestDriver::new();
        let rt = td.rt();
        td.run_until(async move {
            rt.wait(Duration::from_secs(10)).await;
            let mut queue = CoDelQueue::new_for_test(
                rt.clone(),
                3,
                Duration::from_secs(5),
                Duration::from_secs(1),
            );
            queue.push(1)?;
            queue.push(2)?;
            queue.push(3)?;
            assert_eq!(queue.pop(), Some((1, None)));
            assert_eq!(queue.pop(), Some((2, None)));
            // 3 stays in the queue for a while, so we switch to LIFO.
            rt.wait(Duration::from_millis(5001)).await;
            queue.push(4)?;
            queue.push(5)?;
            // But first we drain expired entries.
            assert_eq!(queue.pop(), Some((3, Some(ExpiredInQueue))));
            assert_eq!(queue.pop(), Some((5, None)));
            assert_eq!(queue.pop(), Some((4, None)));
            // Now it has been emptied, we switch back to FIFO.
            queue.push(6)?;
            queue.push(7)?;
            assert_eq!(queue.pop(), Some((6, None)));
            assert_eq!(queue.pop(), Some((7, None)));
            Ok(())
        })
    }

    #[test]
    fn test_controlled_delay() -> anyhow::Result<()> {
        let td = TestDriver::new();
        let rt = td.rt();
        td.run_until(async move {
            let mut queue = CoDelQueue::new_for_test(
                rt.clone(),
                5,
                Duration::from_secs(5),
                Duration::from_secs(1),
            );
            // 1, 2, and 3 have expiration of 5s.
            queue.push(1)?;
            queue.push(2)?;

            rt.wait(Duration::from_millis(3001)).await;

            assert_eq!(queue.pop(), Some((1, None)));
            queue.push(3)?;
            // 2 has stayed in the queue for a while, so we switch to shorter expirations.
            rt.wait(Duration::from_millis(2001)).await;
            assert_eq!(queue.pop(), Some((2, Some(ExpiredInQueue))));
            // 4 and 5 have expiration of 1s.
            queue.push(4)?;
            queue.push(5)?;
            rt.wait(Duration::from_millis(500)).await;
            assert_eq!(queue.pop(), Some((5, None)));
            rt.wait(Duration::from_millis(501)).await;
            assert_eq!(queue.pop(), Some((4, Some(ExpiredInQueue))));
            rt.wait(Duration::from_millis(1001)).await;
            // 3 has been in the queue for 4s, less than its 5s timeout.
            assert_eq!(queue.pop(), Some((3, None)));
            Ok(())
        })
    }
}
#[cfg(test)]
mod codel_queue_async_tests {

    use futures::StreamExt;

    use crate::{
        codel_queue::new_codel_queue_async,
        runtime::testing::TestDriver,
    };

    #[test]
    fn test_async_fifo() -> anyhow::Result<()> {
        let td = TestDriver::new();
        let rt = td.rt();
        td.run_until(async move {
            let (sender, mut receiver) = new_codel_queue_async(rt, 2);
            sender.try_send(1)?;
            sender.try_send(2)?;
            assert!(sender.try_send(3).is_err());
            assert_eq!(receiver.next().await, Some((1, None)));
            sender.try_send(4)?;
            assert_eq!(receiver.next().await, Some((2, None)));
            assert_eq!(receiver.next().await, Some((4, None)));
            let wait_for_next = receiver.next();
            sender.try_send(5)?;
            assert_eq!(wait_for_next.await, Some((5, None)));
            Ok(())
        })
    }

    #[test]
    fn test_multiple_sender_receiver() -> anyhow::Result<()> {
        let td = TestDriver::new();
        let rt = td.rt();
        td.run_until(async move {
            let (sender1, mut receiver1) = new_codel_queue_async(rt, 2);
            let sender2 = sender1.clone();
            sender1.try_send(1)?;
            sender2.try_send(2)?;
            assert!(sender1.try_send(3).is_err());
            assert_eq!(receiver1.next().await, Some((1, None)));
            sender1.try_send(4)?;
            let mut receiver2 = receiver1.clone();
            assert_eq!(receiver2.next().await, Some((2, None)));
            assert_eq!(receiver1.next().await, Some((4, None)));
            sender1.try_send(5)?;
            drop(sender1);
            assert_eq!(receiver1.next().await, Some((5, None)));
            let wait_for_next1 = receiver1.next();
            let wait_for_next2 = receiver2.next();
            sender2.try_send(6)?;
            drop(sender2);
            assert_eq!(wait_for_next2.await, Some((6, None)));
            assert_eq!(wait_for_next1.await, None);
            assert_eq!(receiver2.next().await, None);
            Ok(())
        })
    }
}

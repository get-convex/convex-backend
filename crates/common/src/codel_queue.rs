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
    buffer: VecDeque<(T, tokio::time::Instant)>,
    capacity: usize,
    last_time_empty: tokio::time::Instant,
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
        (self.last_time_empty + self.idle_expiration) > self.rt.monotonic_now()
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
        log_codel_queue_time_since_empty(self.rt.monotonic_now() - self.last_time_empty)
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

    fn pop_front(&mut self) -> Option<(T, tokio::time::Instant)> {
        let result = self.buffer.pop_front();
        // If the queue is newly empty, update last_empty_time=now().
        // This is redundant since it will remain empty and that will only
        // matter if we check is_idle, which will also update last_empty_time.
        // But it doesn't hurt to keep it updated.
        self.update_last_time_empty();
        result
    }

    fn pop_back(&mut self) -> Option<(T, tokio::time::Instant)> {
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

    pub fn into_sender_and_receiver(self) -> (CoDelQueueSender<RT, T>, CoDelQueueReceiver<RT, T>) {
        let inner = Arc::new(Mutex::new(Inner {
            queue: self,
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
}

/// Wrapper around CoDelQueue that makes it async.
pub fn new_codel_queue_async<RT: Runtime, T>(
    rt: RT,
    capacity: usize,
) -> (CoDelQueueSender<RT, T>, CoDelQueueReceiver<RT, T>) {
    CoDelQueue::new(rt, capacity).into_sender_and_receiver()
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

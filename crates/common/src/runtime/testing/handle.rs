use std::{
    pin::Pin,
    sync::Arc,
    task::{
        Context,
        Poll,
        Waker,
    },
};

use crossbeam::channel::{
    self as crossbeam_channel,
    RecvError,
};
use futures::{
    channel::oneshot,
    future::{
        self,
        RemoteHandle,
    },
    pin_mut,
    Future,
    FutureExt,
};
use parking_lot::Mutex;
use pin_project::pin_project;

use crate::runtime::{
    JoinError,
    SpawnHandle,
};

/// Handle to a future that runs on the same thread as the test. Dropping this
/// handle does not stop execution of the future to match the semantics of the
/// production [FutureHandle](`crate::prod::FutureHandle`).
pub struct TestFutureHandle {
    handle: Option<RemoteHandle<()>>,
}

impl Drop for TestFutureHandle {
    fn drop(&mut self) {
        // To match the semantics of the real `FutureHandle`, dropping this
        // handle should detach the task rather than cancel it.
        if let Some(h) = self.handle.take() {
            h.forget();
        }
    }
}

impl TestFutureHandle {
    pub fn spawn(
        f: impl Future<Output = ()> + Send + 'static,
    ) -> (impl Future<Output = ()> + Send, Self) {
        let (future, handle) = f.remote_handle();
        (
            future,
            Self {
                handle: Some(handle),
            },
        )
    }
}

impl SpawnHandle for TestFutureHandle {
    type Future = Pin<Box<dyn Future<Output = Result<(), JoinError>> + Send>>;

    fn shutdown(&mut self) {
        // Per the documentation of RemoteHandle, execution of the future stops
        // when the handle is dropped. The future wakes up and immediately
        // resolves to `()` without completing any further work.
        drop(self.handle.take());
    }

    fn into_join_future(mut self) -> Self::Future {
        match self.handle.take() {
            Some(handle) => handle.map(Ok).boxed(),
            None => {
                // The handle has already been shut down -- immediately return a JoinError.
                future::err(JoinError::Canceled).boxed()
            },
        }
    }
}

/// Handle to a future running on its own thread via
/// [spawn_thread](`common::runtime::Runtime::spawn_thread`). We pass this
/// handle up above the runtime interface. Dropping this handle does not join
/// the thread.
pub struct TestThreadHandle {
    /// Used to send [`ThreadCommand`]s to the remote thread.
    command_tx: crossbeam_channel::Sender<ThreadCommand>,
    /// Receives a message when the thread stops; the return value is true when
    /// the thread was canceled and false when the future completed cleanly.
    completion_rx: oneshot::Receiver<bool>,
    /// Raw handle to the thread.
    handle: Arc<Mutex<JoinHandle>>,
}

/// Crate-internal handle for communicating with the thread started by
/// [spawn_thread](`common::runtime::Runtime::spawn_thread`) from the
/// TestRuntime's scheduler.  This struct implements [`Future`] and polls are
/// forwarded to the future running on the remote thread. Dropping this handle
/// causes the remote thread to stop executing after its next await point.
#[pin_project]
pub(crate) struct InternalThreadHandle {
    /// Used to send [`ThreadCommand`]s to the remote thread.
    command_tx: crossbeam_channel::Sender<ThreadCommand>,
    /// Used to receive [`Poll`] responses from the remote thread.
    response_rx: crossbeam_channel::Receiver<Poll<bool>>,
    /// Used to send notice of completion to the [`TestThreadHandle`].
    /// See [`TestThreadHandle::completion_rx`]
    completion_tx: Option<oneshot::Sender<bool>>,
}

enum ThreadCommand {
    /// Poll the future this thread was spawned with. Uses the contained
    /// [`Waker`] to get a context for polling the future.
    Poll(Waker),
    /// Stop the thread. The future will not be polled any further.
    Shutdown,
}

impl InternalThreadHandle {
    pub fn spawn<F, Fut>(f: F) -> (Self, TestThreadHandle, Arc<Mutex<JoinHandle>>)
    where
        F: FnOnce() -> Fut + Send + 'static,
        Fut: Future<Output = ()>,
    {
        // We expect there to be at most one command in the channel at a time.
        let (command_tx, command_rx) = crossbeam_channel::bounded(1);
        let (response_tx, response_rx) = crossbeam_channel::bounded(1);
        let (completion_tx, completion_rx) = oneshot::channel();
        let handle = std::thread::spawn(move || {
            let fut = f().fuse();
            pin_mut!(fut);
            loop {
                match command_rx.recv() {
                    Ok(ThreadCommand::Shutdown) => {
                        let _ = response_tx.send(Poll::Ready(true));
                        return;
                    },
                    Ok(ThreadCommand::Poll(waker)) => {
                        let mut cx = Context::from_waker(&waker);
                        let response = fut.poll_unpin(&mut cx).map(|_| false);
                        response_tx
                            .send(response)
                            .expect("TestRuntime went away without waiting for a poll response");
                        if response.is_ready() {
                            return;
                        }
                    },
                    // The future was dropped from the TestRuntime; just stop execution and join.
                    Err(RecvError) => return,
                }
            }
        });
        let handle = Arc::new(Mutex::new(JoinHandle::Running(handle)));
        let test_handle = TestThreadHandle {
            command_tx: command_tx.clone(),
            handle: handle.clone(),
            completion_rx,
        };
        let internal_handle = InternalThreadHandle {
            command_tx,
            response_rx,
            completion_tx: Some(completion_tx),
        };
        (internal_handle, test_handle, handle)
    }
}

impl Future for InternalThreadHandle {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let self_ = self.project();
        // Forward the poll request to the thread.
        // It's okay if this fails as it indicates the thread panicked or was
        // shut down by the `TestThreadHandle`, and we'll see a response
        // indicating the latter below.
        let _ = self_
            .command_tx
            .send(ThreadCommand::Poll(cx.waker().clone()));
        match self_.response_rx.recv() {
            Ok(Poll::Ready(was_canceled)) => {
                let _ = self_
                    .completion_tx
                    .take()
                    .expect("Future completed twice?")
                    .send(was_canceled);
                Poll::Ready(())
            },
            Ok(Poll::Pending) => Poll::Pending,
            Err(RecvError) => {
                // The thread panicked or shut down without notifying us. Treat this future as
                // completed.
                self_.completion_tx.take();
                Poll::Ready(())
            },
        }
    }
}

impl SpawnHandle for TestThreadHandle {
    type Future = Pin<Box<dyn Future<Output = Result<(), JoinError>> + Send>>;

    fn shutdown(&mut self) {
        self.command_tx
            .try_send(ThreadCommand::Shutdown)
            .expect("Sending shutdown command must succeed in tests");
    }

    fn into_join_future(self) -> Self::Future {
        async move {
            // Handle clean exit (either by shutdown or task completion)
            if let Ok(was_canceled) = self.completion_rx.await {
                return if !was_canceled {
                    Ok(())
                } else {
                    Err(JoinError::Canceled)
                };
            }

            let std_handle = {
                // Holding the lock only while swapping the state doesn't protect us from
                // deadlocks where we join on our own thread, but it's better to be defensive
                // and not hold a lock during the potentially long join.
                let mut handle = self.handle.lock();
                match std::mem::replace(&mut *handle, JoinHandle::Joining) {
                    JoinHandle::Running(h) => h,
                    JoinHandle::Joining => panic!("Handle joined twice?"),
                    JoinHandle::Completed(r) => {
                        *handle = JoinHandle::Completed(r.clone());
                        let message = r.expect_err("Unclean exit didn't produce a panic?");
                        return Err(JoinError::Panicked(anyhow::anyhow!(message)));
                    },
                }
            };
            let message = std_handle
                .join()
                .expect_err("Unclean exit didn't produce a panic?")
                .downcast::<&str>()
                .expect("Panic message must be a string")
                .to_string();
            {
                let mut handle = self.handle.lock();
                *handle = JoinHandle::Completed(Err(message.clone()));
            }
            Err(JoinError::Panicked(anyhow::anyhow!(message)))
        }
        .boxed()
    }
}

/// Handle to the underlying `std::thread` behind a test thread.
pub enum JoinHandle {
    Running(std::thread::JoinHandle<()>),
    Joining,
    Completed(Result<(), String>),
}

#[cfg(test)]
mod tests {
    use std::task::{
        Context,
        Poll,
    };

    use futures::{
        channel::oneshot,
        pin_mut,
        task::noop_waker_ref,
        FutureExt,
    };

    use super::{
        InternalThreadHandle,
        TestFutureHandle,
    };
    use crate::runtime::{
        JoinError,
        SpawnHandle,
    };

    #[test]
    fn test_future_handle_basic() {
        let mut context = Context::from_waker(noop_waker_ref());
        let (tx, rx) = oneshot::channel();
        let fut = async move { rx.await.unwrap() };
        let (fut, handle) = TestFutureHandle::spawn(fut);
        pin_mut!(fut);
        assert!(fut.poll_unpin(&mut context).is_pending());
        let join_future = handle.into_join_future();
        pin_mut!(join_future);
        assert!(join_future.poll_unpin(&mut context).is_pending());
        tx.send(()).unwrap();
        assert!(fut.poll_unpin(&mut context).is_ready());
        assert!(matches!(
            join_future.poll_unpin(&mut context),
            Poll::Ready(Ok(()))
        ));
    }

    #[test]
    fn test_future_handle_shutdown() {
        let mut context = Context::from_waker(noop_waker_ref());
        let (_tx, rx) = oneshot::channel();
        let fut = async move { rx.await.unwrap() };
        let (fut, mut handle) = TestFutureHandle::spawn(fut);
        pin_mut!(fut);
        assert!(fut.poll_unpin(&mut context).is_pending());
        handle.shutdown();
        // Future is ready despite never sending anything over `tx`
        assert!(matches!(fut.poll_unpin(&mut context), Poll::Ready(())));
        let join_future = handle.into_join_future();
        pin_mut!(join_future);
        assert!(matches!(
            join_future.poll_unpin(&mut context),
            Poll::Ready(Err(JoinError::Canceled))
        ));
    }

    #[test]
    fn test_thread_handle_basic() {
        let mut context = Context::from_waker(noop_waker_ref());
        let (tx, rx) = oneshot::channel();
        let fut = async move { rx.await.unwrap() };
        let (internal_handle, test_handle, _) = InternalThreadHandle::spawn(move || fut);
        pin_mut!(internal_handle);
        assert!(internal_handle.poll_unpin(&mut context).is_pending());
        let join_future = test_handle.into_join_future();
        pin_mut!(join_future);
        assert!(join_future.poll_unpin(&mut context).is_pending());
        tx.send(()).unwrap();
        assert!(internal_handle.poll_unpin(&mut context).is_ready());
        assert!(matches!(
            join_future.poll_unpin(&mut context),
            Poll::Ready(Ok(()))
        ));
    }

    #[test]
    fn test_thread_handle_shutdown() {
        let mut context = Context::from_waker(noop_waker_ref());
        let (_tx, rx) = oneshot::channel();
        let fut = async move { rx.await.unwrap() };
        let (internal_handle, mut test_handle, _) = InternalThreadHandle::spawn(move || fut);
        pin_mut!(internal_handle);
        assert!(internal_handle.poll_unpin(&mut context).is_pending());
        test_handle.shutdown();
        assert!(matches!(
            internal_handle.poll_unpin(&mut context),
            Poll::Ready(())
        ));
        let join_future = test_handle.into_join_future();
        pin_mut!(join_future);
        assert!(matches!(
            join_future.poll_unpin(&mut context),
            Poll::Ready(Err(JoinError::Canceled))
        ));
    }

    #[test]
    fn test_thread_panic() {
        let mut context = Context::from_waker(noop_waker_ref());
        let (tx, rx) = oneshot::channel();
        let fut = async move {
            rx.await.unwrap();
            panic!("out of al pastor!");
        };
        let (internal_handle, test_handle, _) = InternalThreadHandle::spawn(move || fut);
        pin_mut!(internal_handle);
        assert!(internal_handle.poll_unpin(&mut context).is_pending());
        let join_future = test_handle.into_join_future();
        pin_mut!(join_future);
        assert!(join_future.poll_unpin(&mut context).is_pending());
        // induce a panic
        tx.send(()).unwrap();
        assert!(internal_handle.poll_unpin(&mut context).is_ready());
        assert!(matches!(
            join_future.poll_unpin(&mut context),
            Poll::Ready(Err(JoinError::Panicked(_)))
        ));
    }
}

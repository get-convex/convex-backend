use std::{
    pin::Pin,
    task::{
        Context,
        Poll,
        Waker,
    },
};

use futures::{
    future::BoxFuture,
    Future,
    FutureExt,
};

use crate::{
    knobs::RUNTIME_STACK_SIZE,
    runtime::{
        JoinError,
        SpawnHandle,
    },
};

// We have a slight divergence between `spawn` and `spawn_thread` where dropping
// the handle from `spawn_thread` cancels the thread. `spawn`, on the other
// hand, inherits tokio's behavior of detaching the task.
pub struct ThreadFutureHandle {
    pub handle: Option<tokio::task::JoinHandle<()>>,
}

impl SpawnHandle for ThreadFutureHandle {
    fn shutdown(&mut self) {
        if let Some(ref mut handle) = self.handle {
            handle.abort();
        }
    }

    fn join(&mut self) -> BoxFuture<'_, Result<(), JoinError>> {
        let handle = self.handle.take();
        let future = async move {
            if let Some(h) = handle {
                h.await?;
            }
            Ok(())
        };
        future.boxed()
    }
}

impl Drop for ThreadFutureHandle {
    fn drop(&mut self) {
        self.shutdown();
    }
}

pub struct ThreadFuture {
    std_handle: Option<std::thread::JoinHandle<()>>,
    poll_request_tx: Option<crossbeam_channel::Sender<Waker>>,
    poll_response_rx: crossbeam_channel::Receiver<Poll<bool>>,
}

impl ThreadFuture {
    pub(crate) fn new<Fut: Future<Output = ()>, F: FnOnce() -> Fut + Send + 'static>(
        tokio_handle: tokio::runtime::Handle,
        f: F,
    ) -> Self {
        let (poll_request_tx, poll_request_rx) = crossbeam_channel::bounded(1);
        let (poll_response_tx, poll_response_rx) = crossbeam_channel::bounded(1);
        let std_handle = std::thread::Builder::new()
            .stack_size(*RUNTIME_STACK_SIZE)
            .spawn(move || {
                let _guard = tokio_handle.enter();
                let fut = f();
                tokio::pin!(fut);
                loop {
                    let Ok(waker) = poll_request_rx.recv() else {
                        tracing::debug!("ThreadFuture gone away, returning.");
                        return;
                    };
                    let mut cx = Context::from_waker(&waker);
                    let response = match fut.poll_unpin(&mut cx) {
                        Poll::Ready(()) => Poll::Ready(false),
                        Poll::Pending => Poll::Pending,
                    };
                    poll_response_tx
                        .send(response)
                        .expect("TestRuntime went away without waiting for a poll response");
                    if response.is_ready() {
                        tracing::debug!("ThreadFuture polled ready, returning.");
                        return;
                    }
                }
            })
            .expect("Failed to start new thread");
        Self {
            std_handle: Some(std_handle),
            poll_request_tx: Some(poll_request_tx),
            poll_response_rx,
        }
    }
}

impl Future for ThreadFuture {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();

        // Forward the poll request to the thread.
        if this
            .poll_request_tx
            .as_ref()
            .expect("poll() called after drop?")
            .send(cx.waker().clone())
            .is_err()
        {
            tracing::error!("ThreadFuture worker thread terminated.");
            return Poll::Ready(());
        }
        let response = match this.poll_response_rx.recv() {
            Ok(response) => response,
            Err(..) => {
                tracing::error!("ThreadFuture worker thread terminated.");
                return Poll::Ready(());
            },
        };
        match response {
            Poll::Ready(was_canceled) => {
                tracing::debug!(
                    "ThreadFuture completed (was_canceled: {was_canceled}), returning."
                );
                Poll::Ready(())
            },
            Poll::Pending => Poll::Pending,
        }
    }
}

impl Drop for ThreadFuture {
    fn drop(&mut self) {
        let Some(std_handle) = self.std_handle.take() else {
            return;
        };
        let Some(command_tx) = self.poll_request_tx.take() else {
            return;
        };
        if std_handle.is_finished() {
            return;
        }
        drop(command_tx);
        tracing::debug!("Waiting for worker thread to shutdown on drop.");
        let r = self.poll_response_rx.recv();
        tracing::debug!("Worker thread shutdown response: {r:?}");
        std_handle.join().expect("Worker thread panicked");
    }
}

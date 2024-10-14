use futures::Stream;
use tokio::sync::mpsc::{
    self,
    error::{
        SendError,
        TrySendError,
    },
};
use tokio_stream::wrappers::UnboundedReceiverStream;

pub fn unbounded_channel<T>() -> (UnboundedSender<T>, UnboundedReceiver<T>) {
    let (tx, rx) = mpsc::unbounded_channel();
    (
        UnboundedSender { inner: Some(tx) },
        UnboundedReceiver { inner: rx },
    )
}
pub struct UnboundedSender<T> {
    inner: Option<mpsc::UnboundedSender<T>>,
}

impl<T> UnboundedSender<T> {
    pub fn send(&mut self, value: T) -> Result<(), SendError<T>> {
        let Some(ref mut inner) = self.inner else {
            return Err(SendError(value));
        };
        inner.send(value)
    }

    pub fn close(&mut self) {
        // Since we don't implement `Clone`, we know we're the only sender, so
        // dropping the underlying `Sender` is sufficient to close the
        // channel.
        self.inner.take();
    }
}

pub struct UnboundedReceiver<T> {
    inner: mpsc::UnboundedReceiver<T>,
}

impl<T> UnboundedReceiver<T> {
    pub async fn recv(&mut self) -> Option<T> {
        self.inner.recv().await
    }

    pub fn into_stream(self) -> impl Stream<Item = T> {
        UnboundedReceiverStream::new(self.inner)
    }
}

pub fn channel<T>(buffer: usize) -> (Sender<T>, Receiver<T>) {
    let (tx, rx) = mpsc::channel(buffer);
    (Sender { inner: Some(tx) }, Receiver { inner: rx })
}

pub struct Sender<T> {
    inner: Option<mpsc::Sender<T>>,
}

impl<T> Sender<T> {
    pub fn try_send(&mut self, value: T) -> Result<(), TrySendError<T>> {
        let Some(ref mut inner) = self.inner else {
            return Err(TrySendError::Closed(value));
        };
        inner.try_send(value)
    }

    pub async fn send(&mut self, value: T) -> Result<(), SendError<T>> {
        let Some(ref mut inner) = self.inner else {
            return Err(SendError(value));
        };
        inner.send(value).await
    }

    pub fn close(&mut self) {
        self.inner.take();
    }
}

pub struct Receiver<T> {
    inner: mpsc::Receiver<T>,
}

impl<T> Receiver<T> {
    pub async fn recv(&mut self) -> Option<T> {
        self.inner.recv().await
    }
}

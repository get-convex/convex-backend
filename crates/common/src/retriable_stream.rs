use std::{
    pin::Pin,
    sync::Arc,
    task::{
        Context,
        Poll,
    },
};

use futures::{
    Stream,
    TryStream,
};
use parking_lot::Mutex;

/// Wrapper around a stream of data that tracks whether or not the stream has
/// ever been polled. This is used where we want to retry a request if we failed
/// to open a connection to an upstream service, but we can't clone the body to
/// retry the request if it's a stream. However, if we're sure the body was
/// never polled in the first place, it's safe to retry with the same body.
///
/// This struct does not implement `Clone` but provides a fallible `try_clone`
/// method which will only succeed if the stream has never been polled.
pub struct RetriableStream<T: TryStream> {
    shared_inner: Arc<Mutex<Option<T>>>,
    inner: Option<T>,
}

impl<T: TryStream> Clone for RetriableStream<T> {
    fn clone(&self) -> Self {
        Self {
            shared_inner: self.shared_inner.clone(),
            inner: None,
        }
    }
}

impl<T: TryStream> RetriableStream<T> {
    pub fn new(body: T) -> Self {
        Self {
            shared_inner: Arc::new(Mutex::new(Some(body))),
            inner: None,
        }
    }

    /// Clones this stream wrapper, but fails if the stream has been previously
    /// polled.
    pub fn try_clone(&self) -> anyhow::Result<Self> {
        let clone = Self {
            shared_inner: self.shared_inner.clone(),
            inner: None,
        };
        if clone.shared_inner.lock().is_some() {
            return Ok(clone);
        }
        anyhow::bail!("Attempted to clone RetriableStream after it has been polled")
    }
}

impl<T: TryStream + std::marker::Unpin> Stream for RetriableStream<T>
where
    T::Error: FromAnyhowError,
{
    type Item = Result<T::Ok, T::Error>;

    #[inline]
    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let _self = self.get_mut();
        let body = match _self.inner {
            Some(ref mut body) => body,
            None => {
                let Some(body) = _self.shared_inner.lock().take() else {
                    // This can happen if the `RetriableStream` has multiple clones, and a different
                    // instance has already been polled.
                    return Poll::Ready(Some(Err(T::Error::from_anyhow_error(anyhow::anyhow!(
                        "Attempted to poll RetriableStream after it has been polled"
                    )))));
                };
                _self.inner.insert(body)
            },
        };
        Pin::new(body).try_poll_next(cx)
    }
}

pub trait FromAnyhowError {
    fn from_anyhow_error(err: anyhow::Error) -> Self;
}

impl FromAnyhowError for axum::Error {
    fn from_anyhow_error(err: anyhow::Error) -> Self {
        Self::new(err)
    }
}

//! Our own extension traits to add functionality to common types

use std::{
    collections::BTreeSet,
    ops::Bound,
    pin::Pin,
};

use async_trait::async_trait;
use futures::{
    stream::Peekable,
    Stream,
    TryStream,
};
use futures_async_stream::stream;

/// Small trait for creating a container with a single value in it
pub trait BTreeSetExt {
    /// Element type for the [`BTreeSet`]
    type T;

    /// Create a [`BTreeSet`] from a single value.
    fn one(t: Self::T) -> Self;
}

impl<T: Ord> BTreeSetExt for BTreeSet<T> {
    type T = T;

    fn one(t: T) -> Self {
        let mut out = BTreeSet::new();
        out.insert(t);
        out
    }
}

/// Extension trait for [`Bound`] functionality.
pub trait BoundExt<T> {
    /// Converts a `Bound<K>` to `Bound<fn(K)>`. Generally used when wanting to
    /// convert e.g., `Bound(c, d)` and a tuple `(a, b)` to the corresponding
    /// `Bound(a, b, c, d)`.
    fn map<U>(self, f: impl FnOnce(T) -> U) -> Bound<U>;
}

impl<T> BoundExt<T> for Bound<T> {
    fn map<U>(self, f: impl FnOnce(T) -> U) -> Bound<U> {
        match self {
            Bound::Included(b) => Bound::Included(f(b)),
            Bound::Excluded(b) => Bound::Excluded(f(b)),
            Bound::Unbounded => Bound::Unbounded,
        }
    }
}

/// StreamExt::take_while but it works better on peekable streams, not dropping
/// any elements. See `test_peeking_take_while` below.
/// Equivalent to https://docs.rs/peeking_take_while/latest/peeking_take_while/#
/// but for streams instead of iterators.
pub trait PeekableExt: Stream {
    #[stream(item=Self::Item)]
    async fn peeking_take_while<F>(self: Pin<&mut Self>, predicate: F)
    where
        F: Fn(&Self::Item) -> bool + 'static;
}

impl<S: Stream> PeekableExt for Peekable<S> {
    #[stream(item=S::Item)]
    async fn peeking_take_while<F>(mut self: Pin<&mut Self>, predicate: F)
    where
        F: Fn(&Self::Item) -> bool + 'static,
    {
        while let Some(item) = self.as_mut().next_if(&predicate).await {
            yield item;
        }
    }
}

#[async_trait]
pub trait TryPeekableExt: TryStream {
    async fn try_next_if<F>(
        self: Pin<&mut Self>,
        predicate: F,
    ) -> Result<Option<Self::Ok>, Self::Error>
    where
        F: Fn(&Self::Ok) -> bool + 'static + Send + Sync;
}

#[async_trait]
impl<Ok: Send, Error: Send, S: Stream<Item = Result<Ok, Error>> + Send> TryPeekableExt
    for Peekable<S>
{
    async fn try_next_if<F>(
        self: Pin<&mut Self>,
        predicate: F,
    ) -> Result<Option<Self::Ok>, Self::Error>
    where
        F: Fn(&Self::Ok) -> bool + 'static + Send + Sync,
    {
        self.next_if(&|result: &Result<Ok, Error>| match result {
            Ok(item) => predicate(item),
            Err(_) => true,
        })
        .await
        .transpose()
    }
}

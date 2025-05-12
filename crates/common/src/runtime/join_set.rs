#![allow(clippy::disallowed_types)]

use std::{
    future::Future,
    task::{
        Context,
        Poll,
    },
};

use fastrace::{
    future::FutureExt as _,
    Span,
};
use tokio::task::{
    AbortHandle,
    JoinError,
};

use super::{
    propagate_tracing,
    GLOBAL_TASK_MANAGER,
};

/// A wrapper around [`tokio::task::JoinSet`] that participates in runtime usage
/// instrumentation, much like [`crate::runtime::tokio_spawn`]
pub struct JoinSet<T> {
    inner: tokio::task::JoinSet<T>,
}

impl<T> JoinSet<T> {
    pub fn new() -> Self {
        Self {
            inner: tokio::task::JoinSet::new(),
        }
    }

    pub fn len(&self) -> usize {
        self.inner.len()
    }

    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }
}
impl<T: 'static> JoinSet<T> {
    pub fn spawn<F>(&mut self, name: &'static str, task: F) -> AbortHandle
    where
        F: Future<Output = T> + Send + 'static,
        T: Send,
    {
        let monitor = GLOBAL_TASK_MANAGER.lock().get(name);
        self.inner.spawn(propagate_tracing(
            monitor.instrument(task.in_span(Span::enter_with_local_parent(name))),
        ))
    }

    pub async fn join_next(&mut self) -> Option<Result<T, JoinError>> {
        self.inner.join_next().await
    }

    pub fn try_join_next(&mut self) -> Option<Result<T, JoinError>> {
        self.inner.try_join_next()
    }

    pub async fn shutdown(&mut self) {
        self.inner.shutdown().await
    }

    pub async fn join_all(self) -> Vec<T> {
        self.inner.join_all().await
    }

    pub fn abort_all(&mut self) {
        self.inner.abort_all()
    }

    pub fn detach_all(&mut self) {
        self.inner.detach_all()
    }

    pub fn poll_join_next(&mut self, cx: &mut Context<'_>) -> Poll<Option<Result<T, JoinError>>> {
        self.inner.poll_join_next(cx)
    }
}

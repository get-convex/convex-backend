use std::cmp;

use async_trait::async_trait;
use common::{
    self,
    document::GenericDocument,
    query::CursorPosition,
    runtime::Runtime,
    types::WriteTimestamp,
};

use super::{
    QueryNode,
    QueryStream,
    QueryType,
    DEFAULT_QUERY_PREFETCH,
};
use crate::Transaction;

/// See Query.limit().
pub(super) struct Limit<T: QueryType> {
    inner: QueryNode<T>,
    limit: usize,
    rows_emitted: usize,
}

impl<T: QueryType> Limit<T> {
    pub fn new(inner: QueryNode<T>, limit: usize) -> Self {
        Self {
            inner,
            limit,
            rows_emitted: 0,
        }
    }
}

#[async_trait]
impl<T: QueryType> QueryStream<T> for Limit<T> {
    fn cursor_position(&self) -> &Option<CursorPosition> {
        self.inner.cursor_position()
    }

    fn split_cursor_position(&self) -> Option<&CursorPosition> {
        self.inner.split_cursor_position()
    }

    fn is_approaching_data_limit(&self) -> bool {
        self.inner.is_approaching_data_limit()
    }

    async fn next<RT: Runtime>(
        &mut self,
        tx: &mut Transaction<RT>,
        prefetch_hint: Option<usize>,
    ) -> anyhow::Result<Option<(GenericDocument<T::T>, WriteTimestamp)>> {
        if self.rows_emitted >= self.limit {
            return Ok(None);
        }
        let inner_hint = cmp::min(
            prefetch_hint.unwrap_or(DEFAULT_QUERY_PREFETCH),
            self.limit - self.rows_emitted,
        );
        let next_value = self.inner.next(tx, Some(inner_hint)).await?;
        if next_value.is_some() {
            self.rows_emitted += 1;
        }
        Ok(next_value)
    }
}

use std::cmp;

use async_trait::async_trait;
use common::{
    query::CursorPosition,
    runtime::Runtime,
};

use super::{
    IndexRangeResponse,
    QueryNode,
    QueryStream,
    QueryStreamNext,
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
    ) -> anyhow::Result<QueryStreamNext<T>> {
        if self.rows_emitted >= self.limit {
            return Ok(QueryStreamNext::Ready(None));
        }
        let inner_hint = cmp::min(
            prefetch_hint.unwrap_or(DEFAULT_QUERY_PREFETCH),
            self.limit - self.rows_emitted,
        );
        let result = self.inner.next(tx, Some(inner_hint)).await?;
        if let QueryStreamNext::Ready(Some(_)) = result {
            self.rows_emitted += 1;
        }
        Ok(result)
    }

    fn feed(&mut self, index_range_response: IndexRangeResponse<T::T>) -> anyhow::Result<()> {
        self.inner.feed(index_range_response)
    }
}

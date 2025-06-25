use std::cmp;

use async_trait::async_trait;
use common::{
    knobs::DEFAULT_QUERY_PREFETCH,
    query::CursorPosition,
    runtime::Runtime,
    types::{
        IndexName,
        TabletIndexName,
    },
};

use super::{
    DeveloperIndexRangeResponse,
    QueryNode,
    QueryStream,
    QueryStreamNext,
};
use crate::Transaction;

/// See Query.limit().
pub(super) struct Limit {
    inner: QueryNode,
    limit: usize,
    rows_emitted: usize,
}

impl Limit {
    pub fn new(inner: QueryNode, limit: usize) -> Self {
        Self {
            inner,
            limit,
            rows_emitted: 0,
        }
    }
}

#[async_trait]
impl QueryStream for Limit {
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
    ) -> anyhow::Result<QueryStreamNext> {
        if self.rows_emitted >= self.limit {
            return Ok(QueryStreamNext::Ready(None));
        }
        let inner_hint = cmp::min(
            prefetch_hint.unwrap_or(*DEFAULT_QUERY_PREFETCH),
            self.limit - self.rows_emitted,
        );
        let result = self.inner.next(tx, Some(inner_hint)).await?;
        if let QueryStreamNext::Ready(Some(_)) = result {
            self.rows_emitted += 1;
        }
        Ok(result)
    }

    fn feed(&mut self, index_range_response: DeveloperIndexRangeResponse) -> anyhow::Result<()> {
        self.inner.feed(index_range_response)
    }

    fn tablet_index_name(&self) -> Option<&TabletIndexName> {
        self.inner.tablet_index_name()
    }

    fn printable_index_name(&self) -> &IndexName {
        self.inner.printable_index_name()
    }
}

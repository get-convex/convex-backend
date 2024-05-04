use async_trait::async_trait;
use common::{
    query::{
        CursorPosition,
        Expression,
    },
    runtime::Runtime,
    types::TabletIndexName,
};

use super::{
    IndexRangeResponse,
    QueryNode,
    QueryStream,
    QueryStreamNext,
    QueryType,
};
use crate::Transaction;

// We can likely be smarter here, like start with a medium limit
// and then dynamically adjust it up or down depending on latency and data
// fetched. Keep it simple for now.
const FILTER_QUERY_PREFETCH: usize = 100;

/// See Query.filter().
pub(super) struct Filter<T: QueryType> {
    inner: QueryNode<T>,
    expr: Expression,
}

impl<T: QueryType> Filter<T> {
    pub fn new(inner: QueryNode<T>, expr: Expression) -> Self {
        Self { inner, expr }
    }
}

#[async_trait]
impl<T: QueryType> QueryStream<T> for Filter<T> {
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
        _prefetch_hint: Option<usize>,
    ) -> anyhow::Result<QueryStreamNext<T>> {
        loop {
            let (document, write_timestamp) =
                match self.inner.next(tx, Some(FILTER_QUERY_PREFETCH)).await? {
                    QueryStreamNext::Ready(Some(v)) => v,
                    QueryStreamNext::Ready(None) => return Ok(QueryStreamNext::Ready(None)),
                    QueryStreamNext::WaitingOn(request) => {
                        return Ok(QueryStreamNext::WaitingOn(request))
                    },
                };
            let value = document.value().0.clone();
            if self.expr.eval(&value)?.into_boolean()? {
                return Ok(QueryStreamNext::Ready(Some((document, write_timestamp))));
            }
        }
    }

    fn feed(&mut self, index_range_response: IndexRangeResponse<T::T>) -> anyhow::Result<()> {
        self.inner.feed(index_range_response)
    }

    fn tablet_index_name(&self) -> Option<&TabletIndexName> {
        self.inner.tablet_index_name()
    }
}

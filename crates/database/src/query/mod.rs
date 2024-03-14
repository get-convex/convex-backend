use std::marker::PhantomData;

use async_trait::async_trait;
use common::{
    bootstrap_model::index::{
        database_index::IndexedFields,
        INDEX_TABLE,
    },
    document::GenericDocument,
    index::IndexKeyBytes,
    interval::Interval,
    query::{
        Cursor,
        CursorPosition,
        Order,
        Query,
        QueryFingerprint,
        QueryOperator,
        QuerySource,
    },
    runtime::Runtime,
    types::{
        IndexName,
        StableIndexName,
        WriteTimestamp,
    },
    version::Version,
};
use errors::ErrorMetadata;
use value::{
    GenericDocumentId,
    TableIdAndTableNumber,
    TableIdentifier,
    TableName,
    TableNumber,
};

use self::{
    filter::Filter,
    index_range::{
        CursorInterval,
        IndexRange,
    },
    limit::Limit,
    search_query::SearchQuery,
};
use crate::{
    IndexModel,
    Transaction,
    UserFacingModel,
};

mod filter;
mod index_range;
mod limit;
mod search_query;

pub use index_range::soft_data_limit;

// Even in the presence of large prefetch hints, we should never fetch too much
// data at once.
const MAX_QUERY_FETCH: usize = 1024;

// Default number of records to fetch if prefetch hint is not provided.
const DEFAULT_QUERY_PREFETCH: usize = 100;

/// The implementation of `interface Query` from the npm package.
#[async_trait]
trait QueryStream<T: QueryType>: Send {
    /// Return a position for a continuation cursor. A query defines a result
    /// set, independent of pagination, and assuming no concurrent
    /// transactions overlap with this result set, re-executing a query with
    /// this cursor will continue from just after the previous return from
    /// `next()`. The cursor is, in some sense, the "stack" that gets saved
    /// when a query is paused, and the user can decide to "resume" the
    /// query in a subsequent transaction. If there *are* overlapping
    /// transactions, the results of resuming a query are currently
    /// undefined, and we'll eventually want to define them formally and
    /// ensure they're generally useful.
    fn cursor_position(&self) -> &Option<CursorPosition>;

    fn split_cursor_position(&self) -> Option<&CursorPosition>;

    /// Returns true if the query has read so much data that it is in danger
    /// of taking too long and throwing errors. Use this as an indication that
    /// a paginated query should be split in two, or that an unpaginated query
    /// should be paginated, or a query with a filter could use an index
    /// instead.
    fn is_approaching_data_limit(&self) -> bool;

    /// Pull a value out from the query pipeline. The query has completed after
    /// returning `None`, and `.next()` should not be called again. If this
    /// method returns an error, it is safe to retry calling `.next()`, but
    /// the query may not make any progress if the error was, for
    /// example, an `QueryScannedTooManyDocumentsError`.
    async fn next<RT: Runtime>(
        &mut self,
        tx: &mut Transaction<RT>,
        prefetch_hint: Option<usize>,
    ) -> anyhow::Result<Option<(GenericDocument<T::T>, WriteTimestamp)>>;
}

#[async_trait]
pub trait QueryType {
    type T: TableIdentifier;

    async fn index_range<RT: Runtime>(
        tx: &mut Transaction<RT>,
        stable_index_name: &StableIndexName,
        interval: &Interval,
        order: Order,
        max_rows: usize,
        version: Option<Version>,
    ) -> anyhow::Result<(
        Vec<(IndexKeyBytes, GenericDocument<Self::T>, WriteTimestamp)>,
        CursorPosition,
    )>;

    async fn get_with_ts<RT: Runtime>(
        tx: &mut Transaction<RT>,
        id: GenericDocumentId<Self::T>,
        version: Option<Version>,
    ) -> anyhow::Result<Option<(GenericDocument<Self::T>, WriteTimestamp)>>;

    fn record_read_document<RT: Runtime>(
        tx: &mut Transaction<RT>,
        doc: &GenericDocument<Self::T>,
        table_name: &TableName,
    ) -> anyhow::Result<()>;

    fn table_identifier<RT: Runtime>(
        tx: &mut Transaction<RT>,
        table: &TableName,
    ) -> anyhow::Result<Self::T>;
}

pub enum Resolved {}
pub enum Developer {}

#[async_trait]
impl QueryType for Resolved {
    type T = TableIdAndTableNumber;

    async fn index_range<RT: Runtime>(
        tx: &mut Transaction<RT>,
        stable_index_name: &StableIndexName,
        interval: &Interval,
        order: Order,
        max_rows: usize,
        _version: Option<Version>,
    ) -> anyhow::Result<(
        Vec<(IndexKeyBytes, GenericDocument<Self::T>, WriteTimestamp)>,
        CursorPosition,
    )> {
        tx.index_range(stable_index_name, interval, order, max_rows)
            .await
    }

    async fn get_with_ts<RT: Runtime>(
        tx: &mut Transaction<RT>,
        id: GenericDocumentId<Self::T>,
        _version: Option<Version>,
    ) -> anyhow::Result<Option<(GenericDocument<Self::T>, WriteTimestamp)>> {
        tx.get_with_ts(id).await
    }

    fn record_read_document<RT: Runtime>(
        tx: &mut Transaction<RT>,
        doc: &GenericDocument<Self::T>,
        table_name: &TableName,
    ) -> anyhow::Result<()> {
        tx.record_read_document(doc, table_name)
    }

    fn table_identifier<RT: Runtime>(
        tx: &mut Transaction<RT>,
        table: &TableName,
    ) -> anyhow::Result<Self::T> {
        tx.table_mapping().id(table)
    }
}

#[async_trait]
impl QueryType for Developer {
    type T = TableNumber;

    async fn index_range<RT: Runtime>(
        tx: &mut Transaction<RT>,
        stable_index_name: &StableIndexName,
        interval: &Interval,
        order: Order,
        max_rows: usize,
        version: Option<Version>,
    ) -> anyhow::Result<(
        Vec<(IndexKeyBytes, GenericDocument<Self::T>, WriteTimestamp)>,
        CursorPosition,
    )> {
        UserFacingModel::new(tx)
            .index_range(stable_index_name, interval, order, max_rows, version)
            .await
    }

    async fn get_with_ts<RT: Runtime>(
        tx: &mut Transaction<RT>,
        id: GenericDocumentId<Self::T>,
        version: Option<Version>,
    ) -> anyhow::Result<Option<(GenericDocument<Self::T>, WriteTimestamp)>> {
        UserFacingModel::new(tx).get_with_ts(id, version).await
    }

    fn record_read_document<RT: Runtime>(
        tx: &mut Transaction<RT>,
        doc: &GenericDocument<Self::T>,
        table_name: &TableName,
    ) -> anyhow::Result<()> {
        UserFacingModel::new(tx).record_read_document(doc, table_name)
    }

    fn table_identifier<RT: Runtime>(
        tx: &mut Transaction<RT>,
        table: &TableName,
    ) -> anyhow::Result<Self::T> {
        Ok(tx.table_mapping().id(table)?.table_number)
    }
}

pub struct CompiledQuery<RT: Runtime, T: QueryType> {
    root: QueryNode<T>,
    query_fingerprint: QueryFingerprint,
    end_cursor: Option<Cursor>,
    _marker: PhantomData<(RT, T)>,
}

#[derive(Copy, Clone, Debug)]
pub enum TableFilter {
    IncludePrivateSystemTables,
    ExcludePrivateSystemTables,
}

pub type ResolvedQuery<RT> = CompiledQuery<RT, Resolved>;
pub type DeveloperQuery<RT> = CompiledQuery<RT, Developer>;

impl<RT: Runtime> ResolvedQuery<RT> {
    pub fn new(tx: &mut Transaction<RT>, query: Query) -> anyhow::Result<Self> {
        Self::new_bounded(
            tx,
            query,
            None,
            None,
            None,
            None,
            false,
            None,
            TableFilter::IncludePrivateSystemTables,
        )
    }

    pub fn new_with_version(
        tx: &mut Transaction<RT>,
        query: Query,
        version: Option<Version>,
    ) -> anyhow::Result<Self> {
        Self::new_bounded(
            tx,
            query,
            None,
            None,
            None,
            None,
            false,
            version,
            TableFilter::IncludePrivateSystemTables,
        )
    }
}

impl<RT: Runtime> DeveloperQuery<RT> {
    pub fn new(
        tx: &mut Transaction<RT>,
        query: Query,
        table_filter: TableFilter,
    ) -> anyhow::Result<Self> {
        Self::new_bounded(tx, query, None, None, None, None, false, None, table_filter)
    }

    pub fn new_with_version(
        tx: &mut Transaction<RT>,
        query: Query,
        version: Option<Version>,
        table_filter: TableFilter,
    ) -> anyhow::Result<Self> {
        Self::new_bounded(
            tx,
            query,
            None,
            None,
            None,
            None,
            false,
            version,
            table_filter,
        )
    }
}

impl<RT: Runtime, T: QueryType> CompiledQuery<RT, T> {
    pub fn new_bounded(
        tx: &mut Transaction<RT>,
        query: Query,
        start_cursor: Option<Cursor>,
        end_cursor: Option<Cursor>,
        maximum_rows_read: Option<usize>,
        maximum_bytes_read: Option<usize>,
        should_compute_split_cursor: bool,
        version: Option<Version>,
        table_filter: TableFilter,
    ) -> anyhow::Result<Self> {
        let index_name = match query.source {
            QuerySource::FullTableScan(ref full_table_scan) => {
                let table_name = full_table_scan.table_name.clone();
                anyhow::ensure!(
                    &table_name != &*INDEX_TABLE,
                    "`_index` can't be queried via .collect() since it doesn't have \
                     by_creation_time index. Please query via by_id index."
                );
                IndexName::by_creation_time(table_name)
            },
            QuerySource::IndexRange(ref index_range) => index_range.index_name.clone(),
            QuerySource::Search(ref search) => search.index_name.clone(),
        };
        let stable_index_name = IndexModel::new(tx).stable_index_name(&index_name, table_filter)?;
        let indexed_fields = match query.source {
            QuerySource::FullTableScan(_) => IndexedFields::creation_time(),
            QuerySource::IndexRange(_) => {
                IndexModel::new(tx).indexed_fields(&stable_index_name, &index_name)?
            },
            QuerySource::Search(_) => {
                // Hack! Search indexes don't have any concept of indexed fields.
                // Database queries need the fields for the query fingerprint
                // because the order of the fields changes the query result.
                // Search query results don't depend on the index used so we
                // can just an empty list of fields.
                IndexedFields::try_from(Vec::new())?
            },
        };
        let fingerprint = query.fingerprint(&indexed_fields)?;
        let start_cursor_position = match start_cursor {
            Some(cursor) => {
                anyhow::ensure!(cursor.query_fingerprint == fingerprint, invalid_cursor());
                Some(cursor.position)
            },
            None => None,
        };
        let end_cursor_position = match &end_cursor {
            Some(cursor) => {
                anyhow::ensure!(cursor.query_fingerprint == fingerprint, invalid_cursor());
                Some(cursor.position.clone())
            },
            None => None,
        };
        let cursor_interval = CursorInterval {
            curr_exclusive: start_cursor_position,
            end_inclusive: end_cursor_position,
        };

        let mut cur_node = match query.source {
            QuerySource::FullTableScan(full_table_scan) => QueryNode::IndexRange(IndexRange::new(
                stable_index_name,
                index_name,
                Interval::all(),
                full_table_scan.order,
                cursor_interval,
                maximum_rows_read,
                maximum_bytes_read,
                should_compute_split_cursor,
                version,
            )),
            QuerySource::IndexRange(index_range) => {
                let order = index_range.order;
                let interval = index_range.compile(indexed_fields)?;
                QueryNode::IndexRange(IndexRange::new(
                    stable_index_name,
                    index_name,
                    interval,
                    order,
                    cursor_interval,
                    maximum_rows_read,
                    maximum_bytes_read,
                    should_compute_split_cursor,
                    version,
                ))
            },
            QuerySource::Search(search) => {
                QueryNode::Search(SearchQuery::new(search, cursor_interval, version))
            },
        };
        for operator in query.operators {
            let next_node = match operator {
                QueryOperator::Filter(expr) => {
                    let filter = Filter::new(cur_node, expr);
                    QueryNode::Filter(Box::new(filter))
                },
                QueryOperator::Limit(n) => {
                    let limit = Limit::new(cur_node, n);
                    QueryNode::Limit(Box::new(limit))
                },
            };
            cur_node = next_node;
        }
        Ok(Self {
            root: cur_node,
            query_fingerprint: fingerprint,
            end_cursor,
            _marker: PhantomData,
        })
    }

    /// Get the end_cursor as specified in `new_bounded`.
    pub fn end_cursor(&self) -> Option<Cursor> {
        self.end_cursor.clone()
    }

    /// Get the current cursor for the query.
    ///
    /// Will be `None` if there was no initial cursor and `next` has
    /// never been called.
    pub fn cursor(&self) -> Option<Cursor> {
        match self.root.cursor_position().clone() {
            Some(position) => Some(Cursor {
                position,
                query_fingerprint: self.query_fingerprint.clone(),
            }),
            None => None,
        }
    }

    pub fn split_cursor(&self) -> Option<Cursor> {
        match self.root.split_cursor_position().cloned() {
            Some(position) => Some(Cursor {
                position,
                query_fingerprint: self.query_fingerprint.clone(),
            }),
            None => None,
        }
    }

    pub async fn next(
        &mut self,
        tx: &mut Transaction<RT>,
        prefetch_hint: Option<usize>,
    ) -> anyhow::Result<Option<GenericDocument<T::T>>> {
        match self.next_with_ts(tx, prefetch_hint).await? {
            None => Ok(None),
            Some((document, _)) => Ok(Some(document)),
        }
    }

    #[convex_macro::instrument_future]
    pub async fn next_with_ts(
        &mut self,
        tx: &mut Transaction<RT>,
        prefetch_hint: Option<usize>,
    ) -> anyhow::Result<Option<(GenericDocument<T::T>, WriteTimestamp)>> {
        self.root.next(tx, prefetch_hint).await
    }

    pub async fn expect_one(
        &mut self,
        tx: &mut Transaction<RT>,
    ) -> anyhow::Result<GenericDocument<T::T>> {
        let v = self
            .next(tx, Some(2))
            .await?
            .ok_or_else(|| anyhow::anyhow!("Expected one value for query, received zero"))?;

        if self.next(tx, Some(1)).await?.is_some() {
            anyhow::bail!("Received more than one value for query");
        }
        Ok(v)
    }

    pub async fn expect_at_most_one(
        &mut self,
        tx: &mut Transaction<RT>,
    ) -> anyhow::Result<Option<GenericDocument<T::T>>> {
        let v = match self.next(tx, Some(2)).await? {
            Some(v) => v,
            None => return Ok(None),
        };
        if self.next(tx, Some(1)).await?.is_some() {
            anyhow::bail!("Received more than one value for query");
        }
        Ok(Some(v))
    }

    pub async fn expect_none(&mut self, tx: &mut Transaction<RT>) -> anyhow::Result<()> {
        anyhow::ensure!(
            self.next(tx, Some(1)).await?.is_none(),
            "Expected no value for this query, but received one."
        );
        Ok(())
    }

    pub fn fingerprint(&self) -> &QueryFingerprint {
        &self.query_fingerprint
    }

    pub fn is_approaching_data_limit(&self) -> bool {
        self.root.is_approaching_data_limit()
    }
}

enum QueryNode<T: QueryType> {
    IndexRange(IndexRange<T>),
    Search(SearchQuery<T>),
    Filter(Box<Filter<T>>),
    Limit(Box<Limit<T>>),
}

#[async_trait]
impl<T: QueryType> QueryStream<T> for QueryNode<T> {
    fn cursor_position(&self) -> &Option<CursorPosition> {
        match self {
            QueryNode::IndexRange(r) => r.cursor_position(),
            QueryNode::Search(r) => r.cursor_position(),
            QueryNode::Filter(r) => r.cursor_position(),
            QueryNode::Limit(r) => r.cursor_position(),
        }
    }

    fn split_cursor_position(&self) -> Option<&CursorPosition> {
        match self {
            QueryNode::IndexRange(r) => r.split_cursor_position(),
            QueryNode::Search(r) => r.split_cursor_position(),
            QueryNode::Filter(r) => r.split_cursor_position(),
            QueryNode::Limit(r) => r.split_cursor_position(),
        }
    }

    fn is_approaching_data_limit(&self) -> bool {
        match self {
            Self::IndexRange(r) => r.is_approaching_data_limit(),
            Self::Search(r) => r.is_approaching_data_limit(),
            Self::Filter(r) => r.is_approaching_data_limit(),
            Self::Limit(r) => r.is_approaching_data_limit(),
        }
    }

    async fn next<RT: Runtime>(
        &mut self,
        tx: &mut Transaction<RT>,
        prefetch_hint: Option<usize>,
    ) -> anyhow::Result<Option<(GenericDocument<T::T>, WriteTimestamp)>> {
        match self {
            QueryNode::IndexRange(r) => r.next(tx, prefetch_hint).await,
            QueryNode::Search(r) => r.next(tx, prefetch_hint).await,
            QueryNode::Filter(r) => r.next(tx, prefetch_hint).await,
            QueryNode::Limit(r) => r.next(tx, prefetch_hint).await,
        }
    }
}

/// Return a system limit for reading too many documents in a query
fn query_scanned_too_many_documents_error(num_documents: usize) -> ErrorMetadata {
    ErrorMetadata::pagination_limit(
        "QueryScannedTooManyDocumentsError",
        format!("Query scanned too many documents (fetched {num_documents})."),
    )
}

/// Return a system limit for reading too much data in a query
fn query_scanned_too_much_data(num_bytes: usize) -> ErrorMetadata {
    ErrorMetadata::pagination_limit(
        "QueryScannedTooMuchDataError",
        format!("Query scanned too much data (fetched {num_bytes} bytes)."),
    )
}

pub fn invalid_cursor() -> ErrorMetadata {
    let message = "InvalidCursor: Tried to run a query starting from a cursor, but it looks like \
                   this cursor is from a different query.";
    ErrorMetadata::bad_request("InvalidCursor", message)
}

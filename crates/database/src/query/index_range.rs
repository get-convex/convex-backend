use std::{
    cmp,
    collections::VecDeque,
};

use anyhow::Context;
use async_trait::async_trait;
use common::{
    document::GenericDocument,
    index::IndexKeyBytes,
    interval::Interval,
    knobs::{
        TRANSACTION_MAX_READ_SIZE_BYTES,
        TRANSACTION_MAX_READ_SIZE_ROWS,
    },
    query::{
        CursorPosition,
        Order,
    },
    runtime::Runtime,
    types::{
        IndexName,
        StableIndexName,
        WriteTimestamp,
    },
    version::Version,
};
use maplit::btreemap;

use super::{
    query_scanned_too_many_documents_error,
    query_scanned_too_much_data,
    QueryStream,
    QueryType,
    DEFAULT_QUERY_PREFETCH,
    MAX_QUERY_FETCH,
};
use crate::{
    metrics,
    transaction::IndexRangeRequest,
    Transaction,
};

/// A `QueryStream` that scans a range of an index.
pub struct IndexRange<T: QueryType> {
    stable_index_name: StableIndexName,
    /// For usage and error messages. If the table mapping has changed, this
    /// might get out of sync with `stable_index_name`, which is the index
    /// actually being walked.
    printable_index_name: IndexName,
    // There is a fixed Interval which is queried by this IndexRange,
    // but we don't need to store it because we have everything we need in
    // cursor_interval, page, and unfetched_interval.
    // interval: Interval,
    order: Order,

    /// The interval defined by the optional start and end cursors.
    /// The start cursor will move as we produce results, but this
    /// `cursor_interval` must always be a subset of `interval`.
    cursor_interval: CursorInterval,
    intermediate_cursors: Option<Vec<CursorPosition>>,
    page: VecDeque<(IndexKeyBytes, GenericDocument<T::T>, WriteTimestamp)>,
    /// The interval which we have yet to fetch.
    /// This starts as an intersection of the IndexRange's `interval` and
    /// `cursor_interval`, and gets smaller as results are fetched into `page`.
    /// When `unfetched_interval` and `page` are empty, the stream is done.
    /// Note that `cursor_interval.curr_exclusive` advances whenever `next()`
    /// yields a new result, while `unfetched_interval.start` (or `.end` if
    /// order is Desc) advances whenever we repopulate `page`, even if we
    /// haven't yielded the results yet.
    unfetched_interval: Interval,
    page_count: usize,
    returned_results: usize,
    rows_read: usize,
    returned_bytes: usize,
    maximum_rows_read: Option<usize>,
    maximum_bytes_read: Option<usize>,
    soft_maximum_rows_read: usize,
    soft_maximum_bytes_read: usize,
    version: Option<Version>,
}

impl<T: QueryType> IndexRange<T> {
    pub fn new(
        stable_index_name: StableIndexName,
        printable_index_name: IndexName,
        interval: Interval,
        order: Order,
        cursor_interval: CursorInterval,
        maximum_rows_read: Option<usize>,
        maximum_bytes_read: Option<usize>,
        should_compute_split_cursor: bool,
        version: Option<Version>,
    ) -> Self {
        // unfetched_interval = intersection of interval with cursor_interval
        let unfetched_interval = match &cursor_interval.curr_exclusive {
            Some(cursor) => {
                let (_, after_curr_cursor_position) = interval.split(cursor.clone(), order);
                after_curr_cursor_position
            },
            None => interval.clone(),
        };
        let unfetched_interval = match &cursor_interval.end_inclusive {
            Some(cursor) => {
                let (up_to_end_cursor_position, _) =
                    unfetched_interval.split(cursor.clone(), order);
                up_to_end_cursor_position
            },
            None => unfetched_interval.clone(),
        };

        Self {
            stable_index_name,
            printable_index_name,
            order,
            cursor_interval,
            intermediate_cursors: if should_compute_split_cursor {
                Some(Vec::new())
            } else {
                None
            },
            page: VecDeque::new(),
            unfetched_interval,
            page_count: 0,
            returned_results: 0,
            rows_read: 0,
            returned_bytes: 0,
            maximum_rows_read,
            maximum_bytes_read,
            soft_maximum_rows_read: soft_data_limit(
                maximum_rows_read
                    .unwrap_or(*TRANSACTION_MAX_READ_SIZE_ROWS)
                    .min(*TRANSACTION_MAX_READ_SIZE_ROWS),
            ),
            soft_maximum_bytes_read: soft_data_limit(
                maximum_bytes_read
                    .unwrap_or(*TRANSACTION_MAX_READ_SIZE_BYTES)
                    .min(*TRANSACTION_MAX_READ_SIZE_BYTES),
            ),
            version,
        }
    }

    fn start_next<RT: Runtime>(
        &mut self,
        tx: &mut Transaction<RT>,
        prefetch_hint: Option<usize>,
    ) -> anyhow::Result<Result<Option<(GenericDocument<T::T>, WriteTimestamp)>, IndexRangeRequest>>
    {
        // If we have an end cursor, for correctness we need to process
        // the entire interval, so ignore `maximum_rows_read` and `maximum_bytes_read`.
        let enforce_limits = self.cursor_interval.end_inclusive.is_none();

        if enforce_limits
            && let Some(maximum_bytes_read) = self.maximum_bytes_read
            && self.returned_bytes >= maximum_bytes_read
        {
            // If we're over our data budget, throw an error.
            // We do this after we've already exceeded the limit to ensure that
            // paginated queries always scan at least one item so they can
            // make progress.
            return Err(query_scanned_too_much_data(self.returned_bytes).into());
        }

        if let Some((index_position, v, timestamp)) = self.page.pop_front() {
            let index_bytes = index_position.len();
            if let Some(intermediate_cursors) = &mut self.intermediate_cursors {
                intermediate_cursors.push(CursorPosition::After(index_position.clone()));
            }
            self.cursor_interval.curr_exclusive = Some(CursorPosition::After(index_position));
            self.returned_results += 1;
            T::record_read_document(tx, &v, self.printable_index_name.table())?;
            // Database bandwidth for index reads
            tx.usage_tracker.track_database_egress_size(
                self.printable_index_name.table().to_string(),
                index_bytes as u64,
                self.printable_index_name.is_system_owned(),
            );
            self.returned_bytes += v.size();
            return Ok(Ok(Some((v, timestamp))));
        }
        if let Some(CursorPosition::End) = self.cursor_interval.curr_exclusive {
            return Ok(Ok(None));
        }
        if self.unfetched_interval.is_empty() {
            // We're out of results. If we have an end cursor then we must
            // have reached it. Otherwise we're at the end of the entire
            // query.
            self.cursor_interval.curr_exclusive = Some(
                self.cursor_interval
                    .end_inclusive
                    .clone()
                    .unwrap_or(CursorPosition::End),
            );
            return Ok(Ok(None));
        }

        let mut max_rows = prefetch_hint
            .unwrap_or(DEFAULT_QUERY_PREFETCH)
            .clamp(1, MAX_QUERY_FETCH);

        if enforce_limits && let Some(maximum_rows_read) = self.maximum_rows_read {
            if self.rows_read >= maximum_rows_read {
                return Err(query_scanned_too_many_documents_error(self.rows_read).into());
            }
            max_rows = cmp::min(max_rows, maximum_rows_read - self.rows_read);
        }
        Ok(Err(IndexRangeRequest {
            stable_index_name: self.stable_index_name.clone(),
            interval: self.unfetched_interval.clone(),
            order: self.order,
            max_rows,
            version: self.version.clone(),
        }))
    }

    fn process_fetch(
        &mut self,
        page: Vec<(IndexKeyBytes, GenericDocument<T::T>, WriteTimestamp)>,
        fetch_cursor: CursorPosition,
    ) -> anyhow::Result<()> {
        let (_, new_unfetched_interval) = self.unfetched_interval.split(fetch_cursor, self.order);
        anyhow::ensure!(self.unfetched_interval != new_unfetched_interval);
        self.unfetched_interval = new_unfetched_interval;
        self.page_count += 1;
        self.rows_read += page.len();
        self.page.extend(page);
        Ok(())
    }

    #[convex_macro::instrument_future]
    async fn _next<RT: Runtime>(
        &mut self,
        tx: &mut Transaction<RT>,
        prefetch_hint: Option<usize>,
    ) -> anyhow::Result<Option<(GenericDocument<T::T>, WriteTimestamp)>> {
        loop {
            let request = match self.start_next(tx, prefetch_hint)? {
                Ok(result) => return Ok(result),
                Err(request) => request,
            };
            let (page, fetch_cursor) = T::index_range_batch(tx, btreemap! {0 => request})
                .await
                .remove(&0)
                .context("batch_key missing")??;
            self.process_fetch(page, fetch_cursor)?;
        }
    }
}

pub const fn soft_data_limit(hard_limit: usize) -> usize {
    hard_limit * 3 / 4
}

#[async_trait]
impl<T: QueryType> QueryStream<T> for IndexRange<T> {
    fn cursor_position(&self) -> &Option<CursorPosition> {
        &self.cursor_interval.curr_exclusive
    }

    fn split_cursor_position(&self) -> Option<&CursorPosition> {
        let intermediate_cursors = self.intermediate_cursors.as_ref()?;
        if intermediate_cursors.len() <= 2 {
            None
        } else {
            intermediate_cursors.get(intermediate_cursors.len() / 2)
        }
    }

    fn is_approaching_data_limit(&self) -> bool {
        self.rows_read > self.soft_maximum_rows_read
            || self.returned_bytes > self.soft_maximum_bytes_read
    }

    async fn next<RT: Runtime>(
        &mut self,
        tx: &mut Transaction<RT>,
        prefetch_hint: Option<usize>,
    ) -> anyhow::Result<Option<(GenericDocument<T::T>, WriteTimestamp)>> {
        self._next(tx, prefetch_hint).await
    }
}

impl<T: QueryType> Drop for IndexRange<T> {
    fn drop(&mut self) {
        metrics::log_index_range(
            self.returned_results,
            // If there are many results in the page when the query is over,
            // it means we fetched too much in a single page and may be able to
            // decrease prefetch hints.
            self.page.len(),
            // If we fetched too many pages, it means we weren't prefetching enough.
            self.page_count,
        )
    }
}

/// An interval between two optional cursors.
pub struct CursorInterval {
    pub curr_exclusive: Option<CursorPosition>,
    pub end_inclusive: Option<CursorPosition>,
}

impl CursorInterval {
    pub fn contains(&self, index_key: &IndexKeyBytes) -> bool {
        if let Some(start_exclusive) = &self.curr_exclusive {
            match start_exclusive {
                CursorPosition::After(start_key) => {
                    // If we're before the start cursor, return false.
                    if *index_key <= *start_key {
                        return false;
                    }
                },
                // If the start cursor is at the end, nothing is in range.
                CursorPosition::End => return false,
            }
        }

        if let Some(CursorPosition::After(end_key)) = &self.end_inclusive {
            // If we're after the end cursor, also return false.
            if *index_key > *end_key {
                return false;
            }
        }
        // If we didn't violate a constraint, we're in range.
        true
    }
}

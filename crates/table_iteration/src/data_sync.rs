//! `DataSyncIterator`: a streaming-export primitive that syncs many tables with
//! bounded reads/output per page and a small, self-contained cursor.
//!
//! It is generally preferable to `TableIterator` due to efficiency (only
//! iterating the document log once) as well as having a small cursor (no large
//! in-memory store).
//!
//! - `DataSyncIterator` can be used for continuous streaming export as well as
//!   to get a single snapshot.
//! - Unlike `TableIterator`, which selects the `snapshot_ts` ahead of time,
//!   `DataSyncIterator` can only select the `snapshot_ts` at the end of the
//!   sync.
//! - `DataSyncIterator` is designed to paginate with a small cursor.
//! - `DataSyncIterator` alternates iterating along `by_id` and advancing
//!   `synced_ts`.
//!
//! See
//! <https://app.notion.com/p/convex-dev/Robust-Streaming-Export-API-36db57ff32ab80c68d97e01c578518d4>
//!
//! # Guarantees
//!
//! - Transactions are not split across pages when advancing `synced_ts`
//! - Each page reads at most `max_rows_read` rows and targets `page_size_limit`
//!   emitted entries and `page_bytes_limit` bytes of emitted documents. May
//!   occasionally go over these limits for a single large transaction.
//!
//! # During Initial Sync phase
//!
//! - During initial sync, pages report [`DataSyncStatus::InProgress`]. During
//!   this phase, pages do not necessarily represent consistent snapshots.
//! - Once initial sync is complete, the final page reports
//!   [`DataSyncStatus::Synced`]
//! - Each document may be emitted more than once, at successive revisions.
//! - Each rev of a given document will be emitted in increasing timestamp
//!   order.
//!
//! # During Synced phase (CDC style)
//!
//! - The final emitted version of every captured document is the version as of
//!   `ts` (a consistent snapshot).
//! - The caller may continue iterating from [`DataSyncStatus::Synced`] to
//!   continue a streaming sync to a newer consistent snapshot.
//! - Transactions are not split across pages.
//! - May switch back to Initial Sync phase if a large operation occurs
//!   (changing the set of synced tables, or an `npx convex import` replacing a
//!   table).
//!
//! # Algorithm
//!
//! A cursor `C` **captures** a document `(tablet, id)` (its latest version at
//! `ts <= C.synced_ts`) when `ts <= C.synced_ts` **and** either
//! `tablet` is in `C.synced_tables`, or `tablet == current_table` and
//! `id <= current_id` (the last id walked in the in-progress table).
//!
//! Each page runs one of two operations, chosen by a freshness heuristic:
//!
//! 1. **`by_id` dimension** — read a page of the `current_table`'s `by_id`
//!    index *at `synced_ts`* (requires `synced_ts` within index retention),
//!    emit each row, and advance `current_id`. On end-of-table, move
//!    `current_table` into `synced_tables` and pick the next target table (or
//!    finish → `Synced`).
//! 2. **`ts` dimension** — walk the document log forward from `synced_ts`, emit
//!    captured documents at their new revision, and advance `synced_ts` to the
//!    last fully-consumed timestamp. Documents not yet captured are skipped
//!    (they will be read by `by_id` at the new `synced_ts`).
//!
//! ```text
//!     ◄══════════════════════════════ ID ════════════════════════════════=══►
//!  │
//!  │   ┌─────────┬─────────┐
//!  │   │   →1    │   →2    │   ← stable ts for ID iteration
//!  │   ├─────────┴─────────┤
//!  │   │                   │
//!  │   │        ↓3         │
//!  │   │                   ├─────────┬─────────┬─────────┐
//!  │   │                   │   →4    │   →5    │   →6    │
//!  │   ├───────────────────┴─────────┴─────────┴─────────┤
//!  │   │                                                 │
//! T│   │                       ↓7                        │
//! S│   │                                                 │
//!  │   ├─────────────────────────────────────────────────┤
//!  │   │                                                 │
//!  │   │                       ↓8                        │
//!  │   │                                                 ├─────────┬─────────┐
//!  │   │                                                 │   →9    │   →10   │
//!  │   ├─────────────────────────────────────────────────┴─────────┴─────────┤
//!  │   │                                                                     │
//!  │   │                                ↓11                                  │
//!  │   │                                                                     │
//!  │   ├─────────────────────────────────────────────────────────────────────┤
//!  │   │                                ↓12                                  │
//!  │   │       Once ID iteration completes, continue CDC style forever       │
//!  │   │                                                                     │
//!  │   └─────────────────────────────────────────────────────────────────────┘
//!  ▼
//! ```

use std::{
    collections::{
        BTreeMap,
        BTreeSet,
    },
    sync::Arc,
    time::Duration,
};

use anyhow::Context;
use common::{
    index::IndexKey,
    persistence::{
        new_static_repeatable_recent,
        DocumentLogEntry,
        PersistenceReader,
        RepeatablePersistence,
        RetentionValidator,
        TimestampRange,
    },
    query::{
        CursorPosition,
        Order,
    },
    runtime::Runtime,
    types::{
        IndexId,
        RepeatableTimestamp,
        Timestamp,
    },
};
use futures::{
    pin_mut,
    StreamExt,
    TryStreamExt,
};
use value::{
    DeveloperDocumentId,
    InternalDocumentId,
    TabletId,
};

use crate::TableScanCursor;

/// Bump a test-only [`coverage`] counter. Expands to nothing outside
/// `test`/`testing` builds, so the instrumentation is zero-cost in production.
macro_rules! cover {
    ($counter:path) => {
    };
}

/// Where a `DataSyncIterator` is in its traversal of the `by_id` (document ID)
/// dimension across the target tables.
#[derive(Clone, Debug, PartialEq, Eq)]
enum TableCursor {
    /// Every target table's ID space has been fully traversed. The view is a
    /// consistent snapshot at `synced_ts`.
    Synced,
    /// Mid-traversal of `current_table`. Documents in `current_table` with
    /// `id <= current_id` have been captured. `current_id == None` means no
    /// document in `current_table` has been walked yet.
    InProgress {
        current_table: TabletId,
        current_id: Option<DeveloperDocumentId>,
    },
}

/// A small, self-contained cursor that fully describes sync progress. It is
/// passed back into [`DataSyncIterator::next_page`] to continue, and is cheap
/// enough to serialize and persist between requests.
///
/// A cursor "captures" a document when its latest version at `ts <= synced_ts`
/// has been emitted — see the module docs for the exact predicate.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DataSyncCursor {
    /// All captured documents have a revision `<= synced_ts`.
    synced_ts: Timestamp,
    /// Tables whose entire ID space has been traversed at `synced_ts`.
    synced_tables: BTreeSet<TabletId>,
    /// Position within the `by_id` dimension.
    table_cursor: TableCursor,
}

/// Progress indicator returned while a sync is still
/// [`DataSyncStatus::InProgress`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProgressStatus {
    pub num_tables_synced: u64,
    pub total_tables: u64,
    pub current_table: Option<TabletId>,
    /// Best-effort: the number of documents emitted from the current table's
    /// `by_id` scan in *this* page (0 on `ts`-dimension pages).
    pub num_documents_in_current_table: u64,
}

/// The consistency state reported alongside a page.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DataSyncStatus {
    /// The entries emitted so far represent a consistent snapshot at `ts`.
    Synced {
        ts: Timestamp,
        /// Whether `ts` is behind the latest timestamp — i.e. the snapshot is
        /// consistent but not fully caught up to the most recent commit.
        /// `false` means the sync read all the way to latest. Callers use this
        /// to decide whether to keep iterating or take a break.
        has_more: bool,
    },
    /// More pages are required before the view is consistent.
    InProgress { progress: ProgressStatus },
}

/// A single page of sync output.
pub struct DataSyncPage {
    /// Emitted entries. `value: None` is a tombstone (delete); `prev_ts` is
    /// preserved for CDC/delete handling by consumers.
    pub entries: Vec<DocumentLogEntry>,
    /// The cursor to pass to the next [`DataSyncIterator::next_page`] call.
    pub cursor: DataSyncCursor,
    pub status: DataSyncStatus,
}

pub struct DataSyncIterator<RT: Runtime> {
    runtime: RT,
    persistence: Arc<dyn PersistenceReader>,
    retention_validator: Arc<dyn RetentionValidator>,
    page_size_limit: usize,
    page_bytes_limit: usize,
    max_rows_read: usize,
    by_id_freshness: Duration,
}

impl<RT: Runtime> DataSyncIterator<RT> {
    /// `page_size_limit` bounds entries emitted per page and `page_bytes_limit`
    /// bounds their total byte size (both soft: a single transaction is never
    /// split, so it may push a page over). `max_rows_read` bounds rows read
    /// from persistence per page. `by_id_freshness` is how close
    /// `synced_ts` must be to the latest timestamp before the iterator
    /// scans the `by_id` dimension (rather than catching up along `ts`).
    pub fn new(
        runtime: RT,
        persistence: Arc<dyn PersistenceReader>,
        retention_validator: Arc<dyn RetentionValidator>,
        page_size_limit: usize,
        page_bytes_limit: usize,
        max_rows_read: usize,
        by_id_freshness: Duration,
    ) -> anyhow::Result<Self> {
        // The `by_id` dimension reads up to `page_size_limit` rows in a single
        // page, so the row-read bound must be at least the page-size bound for
        // the "each page reads at most `max_rows_read` rows" guarantee to hold.
        anyhow::ensure!(
            page_size_limit <= max_rows_read,
            "page_size_limit ({page_size_limit}) must be <= max_rows_read ({max_rows_read})"
        );
        Ok(Self {
            runtime,
            persistence,
            retention_validator,
            page_size_limit,
            page_bytes_limit,
            max_rows_read,
            by_id_freshness,
        })
    }

    /// Produce the next page of the sync.
    ///
    /// `cursor: None` starts a fresh sync. `target_tables` is the desired set
    /// of tables to sync, mapping each tablet to its `by_id` index id; it
    /// is diffed against the cursor on every call so tables can be added or
    /// removed between pages.
    pub async fn next_page(
        &self,
        cursor: Option<DataSyncCursor>,
        target_tables: &BTreeMap<TabletId, IndexId>,
    ) -> anyhow::Result<DataSyncPage> {
        self.runtime
            .pause_client()
            .wait("data_sync_before_page")
            .await;

        // The latest repeatable timestamp bounds reads and is used by the
        // freshness heuristic. It increases monotonically, so any `synced_ts`
        // produced by a prior page is `<= latest`.
        let latest = new_static_repeatable_recent(self.persistence.as_ref()).await?;

        // Cold start, or reconcile an existing cursor against `target_tables`.
        let mut cursor = match cursor {
            None => {
                cover!(coverage::COLD_START);
                DataSyncCursor {
                    synced_ts: *latest,
                    synced_tables: BTreeSet::new(),
                    table_cursor: advance_to_next_table(&BTreeSet::new(), target_tables),
                }
            },
            Some(mut cursor) => {
                reconcile_target_tables(&mut cursor, target_tables);
                cursor
            },
        };

        let use_by_id = match &cursor.table_cursor {
            // Nothing left to traverse in the ID dimension; only the `ts`
            // dimension can make progress (or hold us at a consistent snapshot).
            TableCursor::Synced => {
                cover!(coverage::TS_SELECTED_SYNCED);
                false
            },
            TableCursor::InProgress { .. } => {
                // `latest >= synced_ts` always holds, so this subtraction is safe.
                let lag = *latest - cursor.synced_ts;
                let fresh = lag < self.by_id_freshness;
                if !fresh {
                    cover!(coverage::TS_SELECTED_LAG);
                }
                fresh
            },
        };

        let (entries, status) = if use_by_id {
            self.by_id_page(&mut cursor, latest, target_tables).await?
        } else {
            self.ts_page(&mut cursor, latest, target_tables).await?
        };

        Ok(DataSyncPage {
            entries,
            cursor,
            status,
        })
    }

    /// Advance the `by_id` dimension: read a page of `current_table`'s `by_id`
    /// index at `synced_ts` and emit each document. `synced_ts` is unchanged.
    async fn by_id_page(
        &self,
        cursor: &mut DataSyncCursor,
        latest: RepeatableTimestamp,
        target_tables: &BTreeMap<TabletId, IndexId>,
    ) -> anyhow::Result<(Vec<DocumentLogEntry>, DataSyncStatus)> {
        let TableCursor::InProgress {
            current_table,
            current_id,
        } = cursor.table_cursor.clone()
        else {
            anyhow::bail!("by_id_page called while Synced");
        };
        cover!(coverage::BY_ID_PAGE);
        let by_id = *target_tables
            .get(&current_table)
            .context("current_table missing from target_tables")?;

        // `synced_ts` was `<= latest` when written and `latest` only increases,
        // so it remains repeatable; `read_snapshot` revalidates retention.
        let synced_ts = latest.prior_ts(cursor.synced_ts)?;
        let repeatable_persistence = RepeatablePersistence::new(
            self.persistence.clone(),
            synced_ts,
            self.retention_validator.clone(),
        );
        let snapshot = repeatable_persistence.read_snapshot(synced_ts)?;

        let scan_cursor = TableScanCursor {
            index_key: current_id
                .map(|id| CursorPosition::After(IndexKey::new(vec![], id).to_bytes())),
        };
        let stream = snapshot.index_scan(
            by_id,
            current_table,
            &scan_cursor.interval(),
            Order::Asc,
            self.page_size_limit,
        );
        let page: Vec<_> = stream.take(self.page_size_limit).try_collect().await?;
        let count_limited = page.len() >= self.page_size_limit;
        if page.is_empty() {
            cover!(coverage::BY_ID_EMPTY_PAGE);
        }

        let mut entries = Vec::with_capacity(page.len());
        let mut new_current_id = current_id;
        let mut page_bytes = 0usize;
        let mut bytes_limited = false;
        for (_key, latest_doc) in page {
            page_bytes += latest_doc.value.size();
            new_current_id = Some(latest_doc.value.id().developer_id);
            entries.push(DocumentLogEntry {
                ts: latest_doc.ts,
                id: latest_doc.value.id_with_table_id(),
                value: Some(latest_doc.value),
                prev_ts: latest_doc.prev_ts,
            });
            // Soft byte limit: stop once the page is large enough. We push before
            // checking, so a single document larger than the limit is still
            // emitted (and `current_id` advances past it).
            if page_bytes >= self.page_bytes_limit {
                bytes_limited = true;
                break;
            }
        }

        // The table is exhausted only if we emitted the whole fetched page and it
        // wasn't a full page. If either limit stopped us, there is more to read.
        if count_limited {
            cover!(coverage::BY_ID_COUNT_LIMITED);
        }
        if bytes_limited {
            cover!(coverage::BY_ID_BYTES_LIMITED);
        }
        let reached_end = !count_limited && !bytes_limited;
        if reached_end {
            cover!(coverage::BY_ID_REACHED_END);
            cursor.synced_tables.insert(current_table);
            cursor.table_cursor = advance_to_next_table(&cursor.synced_tables, target_tables);
            if matches!(cursor.table_cursor, TableCursor::Synced) {
                cover!(coverage::SYNC_COMPLETE);
            }
        } else {
            cursor.table_cursor = TableCursor::InProgress {
                current_table,
                current_id: new_current_id,
            };
        }

        let status = status(
            cursor,
            *latest,
            target_tables.len() as u64,
            entries.len() as u64,
        );
        Ok((entries, status))
    }

    /// Advance the `ts` dimension: walk the document log forward from
    /// `synced_ts`, emit captured documents at their new revision, and advance
    /// `synced_ts` to the last fully-consumed timestamp.
    async fn ts_page(
        &self,
        cursor: &mut DataSyncCursor,
        latest: RepeatableTimestamp,
        target_tables: &BTreeMap<TabletId, IndexId>,
    ) -> anyhow::Result<(Vec<DocumentLogEntry>, DataSyncStatus)> {
        cover!(coverage::TS_PAGE);
        let repeatable_persistence = RepeatablePersistence::new(
            self.persistence.clone(),
            latest,
            self.retention_validator.clone(),
        );

        let mut entries = Vec::new();
        // Default: if the stream is exhausted without hitting a limit, we have
        // caught up to `latest`.
        let mut new_synced_ts = *latest;

        if let Some(start) = cursor.synced_ts.succ_opt()
            && start <= *latest
        {
            let stream = repeatable_persistence
                .load_documents(TimestampRange::new(start..=*latest), Order::Asc);
            pin_mut!(stream);

            let mut rows_read = 0usize;
            // The timestamp of the batch we are currently accumulating, the
            // captured entries within it, and their byte size. We only commit a
            // timestamp once it is fully read (a later entry has a strictly
            // greater `ts`, or the stream ends), so a partially-read commit never
            // advances `synced_ts`.
            let mut cur_ts: Option<Timestamp> = None;
            let mut cur_batch: Vec<DocumentLogEntry> = Vec::new();
            let mut cur_batch_bytes = 0usize;
            // Total byte size of the entries committed so far this page.
            let mut page_bytes = 0usize;
            // Whether we've committed at least one complete timestamp this page.
            // Once true, a later transaction that crosses `max_rows_read` is cut
            // short; while false, the first transaction is allowed to exceed the
            // limit on its own (we must read it in full to make progress).
            let mut committed_any = false;
            let mut hit_limit = false;

            while let Some(entry) = stream.try_next().await? {
                if Some(entry.ts) != cur_ts {
                    // The previous timestamp is now fully read. Commit it.
                    if let Some(committed_ts) = cur_ts {
                        entries.append(&mut cur_batch);
                        page_bytes += cur_batch_bytes;
                        cur_batch_bytes = 0;
                        new_synced_ts = committed_ts;
                        committed_any = true;
                        // `page_size_limit`/`page_bytes_limit` are soft, checked
                        // only here so a transaction is never split across pages.
                        let over_rows = rows_read >= self.max_rows_read;
                        let over_size = entries.len() >= self.page_size_limit;
                        let over_bytes = page_bytes >= self.page_bytes_limit;
                        if over_rows || over_size || over_bytes {
                            if over_rows {
                                cover!(coverage::TS_LIMIT_ROWS);
                            }
                            if over_size {
                                cover!(coverage::TS_LIMIT_PAGE_SIZE);
                            }
                            if over_bytes {
                                cover!(coverage::TS_LIMIT_PAGE_BYTES);
                            }
                            // Stop on this complete-timestamp boundary. `entry`
                            // (the first row of the next timestamp) is dropped
                            // and re-read on the next page.
                            hit_limit = true;
                            break;
                        }
                    }
                    cur_ts = Some(entry.ts);
                }
                rows_read += 1;
                if is_captured(&entry.id, cursor) {
                    cover!(coverage::TS_CAPTURED_EMITTED);
                    cur_batch_bytes += entry.value.as_ref().map_or(0, |doc| doc.size());
                    cur_batch.push(entry);
                } else {
                    cover!(coverage::TS_SKIPPED_UNCAPTURED);
                }
                // Enforce `max_rows_read` mid-transaction: if reading this
                // transaction has taken us over the limit, cut the page short at
                // the previous complete timestamp, dropping this partial
                // transaction (re-read next page). The exception is a single
                // transaction larger than `max_rows_read`: with nothing yet
                // committed this page we must read it in full.
                if rows_read >= self.max_rows_read && committed_any {
                    cover!(coverage::TS_MIDTXN_CUT);
                    hit_limit = true;
                    break;
                }
                if rows_read > self.max_rows_read {
                    // Reaching here implies `!committed_any`, and strictly
                    // exceeding the budget means we read a row *past* it: a single
                    // transaction larger than `max_rows_read`, read in full. (The
                    // exactly-at-budget case is covered by `TS_LIMIT_ROWS`.)
                    cover!(coverage::TS_LARGE_TXN_OVERRUN);
                }
            }
            if !hit_limit {
                // Stream exhausted: the final batch is complete and we are
                // caught up all the way to `latest`.
                cover!(coverage::TS_CAUGHT_UP);
                entries.append(&mut cur_batch);
                new_synced_ts = *latest;
            }
        } else {
            // `synced_ts` is already at `latest`; nothing to scan this page.
            cover!(coverage::TS_NOOP);
        }

        cursor.synced_ts = new_synced_ts;
        let status = status(cursor, *latest, target_tables.len() as u64, 0);
        Ok((entries, status))
    }
}

fn status(
    cursor: &DataSyncCursor,
    latest: Timestamp,
    total_tables: u64,
    num_documents_in_current_table: u64,
) -> DataSyncStatus {
    match &cursor.table_cursor {
        TableCursor::Synced => DataSyncStatus::Synced {
            ts: cursor.synced_ts,
            // `synced_ts <= latest` always holds; if it's strictly behind there
            // are commits past the snapshot still to sync.
            has_more: cursor.synced_ts < latest,
        },
        TableCursor::InProgress { current_table, .. } => DataSyncStatus::InProgress {
            progress: ProgressStatus {
                num_tables_synced: cursor.synced_tables.len() as u64,
                total_tables,
                current_table: Some(*current_table),
                num_documents_in_current_table,
            },
        },
    }
}

/// Whether `cursor` is responsible for `id` — i.e. its position in the `by_id`
/// dimension has already been walked, so the sync has committed to emitting it
/// and must replay any later revisions to keep it current.
fn is_captured(id: &InternalDocumentId, cursor: &DataSyncCursor) -> bool {
    let tablet = id.table();
    if cursor.synced_tables.contains(&tablet) {
        cover!(coverage::CAPTURED_VIA_SYNCED_TABLE);
        return true;
    }
    match &cursor.table_cursor {
        TableCursor::InProgress {
            current_table,
            current_id: Some(current_id),
        } if *current_table == tablet => {
            // Within a tablet the `by_id` order matches `InternalId` order, so
            // comparing internal ids is equivalent to comparing index keys.
            let captured = id.internal_id() <= current_id.internal_id();
            if captured {
                cover!(coverage::CAPTURED_VIA_CURRENT_TABLE);
            }
            captured
        },
        _ => false,
    }
}

/// Pick the next target table that hasn't been fully synced, or `Synced` if all
/// targets are done.
fn advance_to_next_table(
    synced_tables: &BTreeSet<TabletId>,
    target_tables: &BTreeMap<TabletId, IndexId>,
) -> TableCursor {
    match target_tables
        .keys()
        .find(|tablet| !synced_tables.contains(tablet))
    {
        Some(tablet) => TableCursor::InProgress {
            current_table: *tablet,
            current_id: None,
        },
        None => TableCursor::Synced,
    }
}

/// Reconcile a cursor against the current target set: drop synced tables that
/// are no longer targeted, cancel an in-progress table that was removed, and
/// start a newly-added table if we were otherwise `Synced`.
fn reconcile_target_tables(
    cursor: &mut DataSyncCursor,
    target_tables: &BTreeMap<TabletId, IndexId>,
) {
    cursor
        .synced_tables
        .retain(|tablet| target_tables.contains_key(tablet));

    match &cursor.table_cursor {
        TableCursor::InProgress { current_table, .. }
            if !target_tables.contains_key(current_table) =>
        {
            // The in-progress table was removed; cancel it and move on.
            cursor.table_cursor = advance_to_next_table(&cursor.synced_tables, target_tables);
        },
        TableCursor::Synced => {
            // A new table may have been added since we last reached Synced.
            cursor.table_cursor = advance_to_next_table(&cursor.synced_tables, target_tables);
        },
        TableCursor::InProgress { .. } => {},
    }
}

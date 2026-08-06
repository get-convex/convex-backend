#![feature(try_blocks_heterogeneous)]
#![feature(coroutines)]

use std::{
    cmp,
    collections::{
        BTreeMap,
        BTreeSet,
    },
    path::Path,
    sync::Arc,
};

use anyhow::Context as _;
use async_trait::async_trait;
use common::{
    document::{
        InternalId,
        ResolvedDocument,
    },
    index::{
        IndexEntry,
        IndexKeyBytes,
    },
    interval::{
        End,
        Interval,
        StartIncluded,
    },
    persistence::{
        ConflictStrategy,
        DocumentLogEntry,
        DocumentPrevTsQuery,
        DocumentStream,
        IndexStream,
        LatestDocument,
        Persistence,
        PersistenceGlobalKey,
        PersistenceIndexEntry,
        PersistenceReader,
        RetentionValidator,
        TimestampRange,
    },
    query::Order,
    runtime::CoopStreamExt as _,
    types::{
        IndexId,
        PersistenceVersion,
        Timestamp,
    },
    value::{
        ConvexValue,
        InternalDocumentId,
        TabletId,
    },
};
use futures::StreamExt;
use futures_async_stream::try_stream;
use parking_lot::Mutex;
use rusqlite::{
    params,
    types::Null,
    Connection,
    Row,
    ToSql,
};
use serde::Deserialize as _;
use serde_json::Value as JsonValue;

// We only have a single Sqlite connection which does not allow async calls, so
// we can't really make queries concurrent.
pub struct SqlitePersistence {
    inner: Arc<Mutex<Inner>>,
}

struct Inner {
    newly_created: bool,
    connection: Connection,
}

impl SqlitePersistence {
    pub fn new(path: &str) -> anyhow::Result<Self> {
        let newly_created = !Path::new(path).exists();
        let connection = Connection::open(path)?;
        // Execute create tables unconditionally since they are idempotent.
        connection.execute_batch(DOCUMENTS_INIT)?;
        connection.execute_batch(INDEXES_INIT)?;
        connection.execute_batch(PERSISTENCE_GLOBALS_INIT)?;
        Ok(Self {
            inner: Arc::new(Mutex::new(Inner {
                newly_created,
                connection,
            })),
        })
    }

    /// Read one page of the documents log, at most `page_size` rows, resuming
    /// strictly after `cursor` — the `(ts, table_id, id)` primary key of the
    /// last row the previous page read, in scan order.
    fn _load_documents_page(
        &self,
        tablet_id: Option<TabletId>,
        range: TimestampRange,
        order: Order,
        page_size: usize,
        cursor: Option<&(u64, Vec<u8>, Vec<u8>)>,
    ) -> anyhow::Result<Vec<DocumentLogEntry>> {
        // Outlives `params`, which borrows from it.
        let tablet_id_bytes = tablet_id.map(|tablet_id| tablet_id.0);
        let tablet_id_slice = tablet_id_bytes.as_ref().map(|bytes| &bytes[..]);
        let mut params: Vec<&dyn ToSql> = vec![];
        // Placeholder numbers are derived from `params` as each one is bound,
        // so adding a parameter can never silently renumber the ones after it.
        let tablet_bound = match tablet_id_slice {
            Some(ref tablet_id) => {
                params.push(tablet_id);
                format!(" AND table_id = ${}", params.len())
            },
            None => String::new(),
        };
        let cursor_bound = match cursor {
            // `table_id` here is the middle component of the primary key, used
            // to break ties within a timestamp. It is unrelated to
            // `tablet_bound` above, which restricts the rows considered at all.
            Some((ts, table_id, id)) => {
                let cmp = match order {
                    Order::Asc => ">",
                    Order::Desc => "<",
                };
                params.push(ts);
                let ts_param = params.len();
                params.push(table_id);
                let table_param = params.len();
                params.push(id);
                let id_param = params.len();
                format!(
                    " AND (ts {cmp} ${ts_param} OR (ts = ${ts_param} AND (table_id {cmp} \
                     ${table_param} OR (table_id = ${table_param} AND id {cmp} ${id_param}))))"
                )
            },
            None => String::new(),
        };
        let order_str = match order {
            Order::Asc => "ASC",
            Order::Desc => "DESC",
        };
        let query = format!(
            r#"
SELECT id, ts, table_id, json_value, deleted, prev_ts
FROM documents
WHERE ts >= {min} AND ts < {max}{tablet_bound}{cursor_bound}
ORDER BY ts {order_str}, table_id {order_str}, id {order_str}
LIMIT {page_size}
"#,
            min = range.min_timestamp_inclusive(),
            max = range.max_timestamp_exclusive(),
        );
        let connection = &self.inner.lock().connection;
        let mut stmt = connection.prepare(&query)?;
        let mut page = vec![];
        for row in stmt.query_map(&params[..], load_document_row)? {
            let (document_id, ts, document, prev_ts) = row_to_document(row)?;
            page.push(DocumentLogEntry {
                ts,
                id: document_id,
                value: document,
                prev_ts,
            });
        }
        Ok(page)
    }

    /// Stream the documents log one page at a time. As in the Postgres
    /// reader, the snapshot is validated against retention after each page is
    /// read and before any of its rows are yielded: pages are read at
    /// different times, and each read is only valid while the range's minimum
    /// timestamp is still within retention. Unlike `_index_scan_paginated`,
    /// nothing is dropped after the query runs — deletes are part of the log,
    /// and `tablet_id` restricts the rows the query considers at all, so
    /// `LIMIT` counts exactly the rows that get yielded — and a short page
    /// therefore always means the range is exhausted.
    #[try_stream(ok = DocumentLogEntry, error = anyhow::Error)]
    async fn _load_documents_paginated(
        &self,
        tablet_id: Option<TabletId>,
        range: TimestampRange,
        order: Order,
        page_size: usize,
        retention_validator: Arc<dyn RetentionValidator>,
    ) {
        let mut cursor: Option<(u64, Vec<u8>, Vec<u8>)> = None;
        loop {
            let page =
                self._load_documents_page(tablet_id, range, order, page_size, cursor.as_ref())?;
            let page_len = page.len();
            retention_validator
                .validate_document_snapshot(range.min_timestamp_inclusive())
                .await?;
            for entry in page {
                cursor = Some((
                    u64::from(entry.ts),
                    entry.id.table().0[..].to_vec(),
                    entry.id.internal_id()[..].to_vec(),
                ));
                yield entry;
            }
            if page_len < page_size {
                break;
            }
        }
    }

    /// Read one page of an index scan, at most `batch_size` keys, resuming
    /// after `cursor` (the last key the previous page scanned, in scan
    /// order). Tombstoned keys are returned as `(key, None)`: they must
    /// still advance the cursor, but must not be emitted to the caller.
    fn _index_scan_page(
        &self,
        index_id: IndexId,
        tablet_id: TabletId,
        read_timestamp: Timestamp,
        interval: &Interval,
        order: Order,
        batch_size: usize,
        cursor: Option<&IndexKeyBytes>,
    ) -> anyhow::Result<Vec<(IndexKeyBytes, Option<LatestDocument>)>> {
        let interval = interval.clone();
        let index_id = &index_id.0[..];
        let read_timestamp: u64 = read_timestamp.into();

        let mut params = params![index_id, read_timestamp].to_vec();

        let StartIncluded(ref start) = interval.start;
        let start_bytes = &start[..];

        params.push(&start_bytes);
        let lower = format!(" AND key >= ${}", params.len());

        let end_bytes = match interval.end {
            End::Excluded(ref t) => Some(&t[..]),
            End::Unbounded => None,
        };
        let upper = match end_bytes {
            Some(ref t) => {
                params.push(t);
                format!(" AND key < ${}", params.len())
            },
            None => "".to_owned(),
        };

        let cursor_bytes = cursor.map(|key| &key.0[..]);
        let cursor_bound = match cursor_bytes {
            Some(ref t) => {
                params.push(t);
                match order {
                    Order::Asc => format!(" AND key > ${}", params.len()),
                    Order::Desc => format!(" AND key < ${}", params.len()),
                }
            },
            None => "".to_owned(),
        };

        let order = match order {
            Order::Asc => "ASC",
            Order::Desc => "DESC",
        };
        // The inner subquery is bounded by `LIMIT`: `GROUP BY` over a prefix
        // of the `(index_id, key, ts)` primary key streams groups in key
        // order, so SQLite stops scanning after `batch_size` keys instead of
        // materializing the entire interval. `B.deleted` is selected (not
        // filtered in the join) so that tombstoned keys count toward the
        // page: otherwise a page full of tombstones would be
        // indistinguishable from the end of the interval.
        let query = format!(
            r#"
SELECT B.key, B.ts, B.document_id, C.table_id, C.json_value, C.prev_ts, B.deleted
FROM (
    SELECT index_id, key, MAX(ts) as max_ts
    FROM indexes
    WHERE index_id = $1 AND ts <= $2{lower}{upper}{cursor_bound}
    GROUP BY index_id, key
    ORDER BY key {order}
    LIMIT {batch_size}
) A
JOIN indexes B
ON A.index_id = B.index_id
AND A.key = B.key
AND A.max_ts = B.ts
LEFT JOIN documents C
ON B.ts = C.ts
AND B.table_id = c.table_id
AND B.document_id = C.id
ORDER BY B.key {order}
"#,
        );

        let connection = &self.inner.lock().connection;
        let mut stmt = connection.prepare(&query)?;
        let row_iter = stmt.query_map(&params[..], |row| {
            let key = IndexKeyBytes(row.get::<_, Vec<u8>>(0)?);
            let ts = Timestamp::try_from(row.get::<_, u64>(1)?).expect("timestamp out of bounds");
            let document_id: Option<Vec<u8>> = row.get(2)?;
            let table: Option<Vec<u8>> = row.get(3)?;
            let json_value: Option<String> = row.get(4)?;
            let prev_ts: Option<Timestamp> = row
                .get::<_, Option<u64>>(5)?
                .map(|ts| Timestamp::try_from(ts).expect("prev_ts out of bounds"));
            let deleted = row.get::<_, u32>(6)? != 0;

            Ok((key, ts, document_id, table, json_value, prev_ts, deleted))
        })?;
        let mut page = vec![];
        for row in row_iter {
            let (key, ts, document_id, table, json_value, prev_ts, deleted) = row?;
            if deleted {
                page.push((key, None));
                continue;
            }
            let table = table.ok_or_else(|| {
                anyhow::anyhow!("Dangling index reference for {:?} {:?}", key, ts)
            })?;
            let table = TabletId(table.try_into()?);
            let document_id = document_id.ok_or_else(|| {
                anyhow::anyhow!("Dangling index reference for {:?} {:?}", key, ts)
            })?;
            let _document_id = InternalDocumentId::new(table, InternalId::try_from(document_id)?);
            let json_value = json_value.ok_or_else(|| {
                anyhow::anyhow!("Index reference to deleted document {:?} {:?}", key, ts)
            })?;
            let json_value: serde_json::Value = serde_json::from_str(&json_value)?;
            let value: ConvexValue = json_value.try_into()?;
            let document = ResolvedDocument::from_database(tablet_id, value)?;
            page.push((
                key,
                Some(LatestDocument {
                    ts,
                    value: document,
                    prev_ts,
                }),
            ));
        }
        Ok(page)
    }

    /// Stream an index scan one page at a time. The snapshot is validated
    /// against retention after each page is read and before any of its rows
    /// are yielded, mirroring the Postgres reader: pages are read at
    /// different times, and each read is only valid if the snapshot is still
    /// within retention.
    #[try_stream(ok = (IndexKeyBytes, LatestDocument), error = anyhow::Error)]
    async fn _index_scan_paginated(
        &self,
        index_id: IndexId,
        tablet_id: TabletId,
        read_timestamp: Timestamp,
        interval: Interval,
        order: Order,
        batch_size: usize,
        retention_validator: Arc<dyn RetentionValidator>,
    ) {
        let mut cursor: Option<IndexKeyBytes> = None;
        loop {
            let page = self._index_scan_page(
                index_id,
                tablet_id,
                read_timestamp,
                &interval,
                order,
                batch_size,
                cursor.as_ref(),
            )?;
            let page_len = page.len();
            retention_validator
                .validate_snapshot(read_timestamp)
                .await?;
            for (key, doc) in page {
                cursor = Some(key.clone());
                if let Some(doc) = doc {
                    yield (key, doc);
                }
            }
            if page_len < batch_size {
                break;
            }
        }
    }

    fn _get_persistence_global(
        &self,
        key: PersistenceGlobalKey,
    ) -> anyhow::Result<Option<JsonValue>> {
        let connection = &self.inner.lock().connection;
        let mut stmt = connection.prepare(GET_PERSISTENCE_GLOBAL)?;
        let key = String::from(key);
        let params: Vec<&dyn ToSql> = vec![&key];
        let mut row_iter = stmt.query_map(&params[..], |row| {
            let json_value_str: String = row.get(0)?;
            Ok(json_value_str)
        })?;
        row_iter
            .next()
            .map(|json_value_str| {
                let json_value_str = json_value_str?;
                let mut json_deserializer = serde_json::Deserializer::from_str(&json_value_str);
                // XXX: this is bad, but shapes can get much more nested than convex values
                json_deserializer.disable_recursion_limit();
                let json_value = JsonValue::deserialize(&mut json_deserializer)
                    .with_context(|| format!("Invalid JSON at persistence key {key:?}"))?;
                json_deserializer.end()?;
                Ok(json_value)
            })
            .transpose()
    }
}

#[async_trait]
impl Persistence for SqlitePersistence {
    fn is_fresh(&self) -> bool {
        self.inner.lock().newly_created
    }

    fn reader(&self) -> Arc<dyn PersistenceReader> {
        Arc::new(Self {
            inner: self.inner.clone(),
        })
    }

    async fn write<'a>(
        &self,
        documents: &'a [DocumentLogEntry],
        indexes: &'a [PersistenceIndexEntry],
        conflict_strategy: ConflictStrategy,
    ) -> anyhow::Result<()> {
        let mut inner = self.inner.lock();
        let tx = inner.connection.transaction()?;
        let mut insert_document_query = match conflict_strategy {
            ConflictStrategy::Error => tx.prepare_cached(INSERT_DOCUMENT)?,
            ConflictStrategy::Overwrite => tx.prepare_cached(INSERT_OVERWRITE_DOCUMENT)?,
        };

        for update in documents {
            let (json_value, deleted) = if let Some(document) = &update.value {
                assert_eq!(update.id, document.id_with_table_id());
                let json_value = document.value().json_serialize()?;
                (Some(json_value), 0)
            } else {
                (None, 1)
            };
            insert_document_query.execute(params![
                &update.id.internal_id()[..],
                &u64::from(update.ts),
                &update.id.table().0[..],
                &json_value,
                &deleted,
                &update.prev_ts.map(u64::from),
            ])?;
        }
        drop(insert_document_query);

        let mut insert_index_query = if conflict_strategy == ConflictStrategy::Overwrite {
            tx.prepare_cached(INSERT_OVERWRITE_INDEX)?
        } else {
            tx.prepare_cached(INSERT_INDEX)?
        };
        for update in indexes {
            let index_id = update.index_id;
            let key: &[u8] = &update.key.0;
            match update.value {
                None => {
                    insert_index_query.execute(params![
                        &index_id.0[..],
                        &u64::from(update.ts),
                        key,
                        &1,
                        &Null,
                        &Null,
                    ])?;
                },
                Some(doc_id) => {
                    insert_index_query.execute(params![
                        &index_id.0[..],
                        &u64::from(update.ts),
                        key,
                        &0,
                        &doc_id.table().0[..],
                        &doc_id.internal_id()[..],
                    ])?;
                },
            };
        }
        drop(insert_index_query);

        tx.commit()?;
        Ok(())
    }

    async fn write_persistence_global(
        &self,
        key: PersistenceGlobalKey,
        value: JsonValue,
    ) -> anyhow::Result<()> {
        let mut inner = self.inner.lock();
        let tx = inner.connection.transaction()?;
        let mut write_query = tx.prepare_cached(WRITE_PERSISTENCE_GLOBAL)?;
        let json_value = serde_json::to_string(&value)?;
        write_query.execute(params![&String::from(key), &json_value])?;
        drop(write_query);
        tx.commit()?;
        Ok(())
    }

    async fn load_index_chunk(
        &self,
        cursor: Option<IndexEntry>,
        chunk_size: usize,
    ) -> anyhow::Result<Vec<IndexEntry>> {
        let connection = &self.inner.lock().connection;
        let mut walk_indexes = connection.prepare(WALK_INDEXES)?;
        let row_iter = walk_indexes.query_map([], |row| {
            let index_id: Vec<u8> = row.get(0)?;
            let key: Vec<u8> = row.get(1)?;
            let ts = Timestamp::try_from(row.get::<_, u64>(2)?).expect("timestamp out of bounds");
            let deleted = row.get::<_, u32>(3)? != 0;
            Ok((index_id, key, ts, deleted))
        })?;
        let rows = row_iter
            .map(|row| {
                let (index_id, key, ts, deleted) = row?;
                let index_row = IndexEntry {
                    index_id: IndexId(index_id.try_into()?),
                    key_prefix: key.clone(),
                    key_suffix: None,
                    key_sha256: key,
                    ts,
                    deleted,
                };
                Ok(index_row)
            })
            .filter(move |index_entry| match cursor {
                None => true,
                Some(ref cursor) => match index_entry {
                    Ok(index_entry) => index_entry > cursor,
                    Err(_) => true,
                },
            })
            .take(chunk_size)
            .collect::<anyhow::Result<Vec<_>>>()?;
        Ok(rows)
    }

    async fn delete_index_entries(&self, expired_rows: Vec<IndexEntry>) -> anyhow::Result<usize> {
        let mut inner = self.inner.lock();
        let tx = inner.connection.transaction()?;
        let mut delete_index_query = tx.prepare_cached(DELETE_INDEX)?;
        let mut count_deleted = 0;

        for IndexEntry {
            index_id,
            key_prefix,
            ts,
            ..
        } in expired_rows
        {
            count_deleted +=
                delete_index_query
                    .execute(params![&index_id.0[..], &u64::from(ts), key_prefix,])?;
        }
        drop(delete_index_query);
        tx.commit()?;
        Ok(count_deleted)
    }

    async fn delete(
        &self,
        documents: Vec<(Timestamp, InternalDocumentId)>,
    ) -> anyhow::Result<usize> {
        let mut inner = self.inner.lock();
        let tx = inner.connection.transaction()?;
        let mut delete_document_query = tx.prepare_cached(DELETE_DOCUMENT)?;
        let mut count_deleted = 0;

        for (ts, internal_id) in documents {
            let tablet_id: TabletId = internal_id.table();
            let id = internal_id.internal_id();
            count_deleted += delete_document_query.execute(params![
                &tablet_id.0[..],
                &id[..],
                &u64::from(ts),
            ])?;
        }
        drop(delete_document_query);
        tx.commit()?;
        Ok(count_deleted)
    }

    async fn delete_tablet_documents(
        &self,
        tablet_id: TabletId,
        chunk_size: usize,
    ) -> anyhow::Result<usize> {
        let mut inner = self.inner.lock();
        let tx = inner.connection.transaction()?;
        let mut delete_table_documents_query = tx.prepare_cached(DELETE_TABLE_DOCUMENTS)?;
        let count_deleted = delete_table_documents_query.execute(params![
            &tablet_id.0[..],
            &tablet_id.0[..],
            chunk_size,
        ])?;
        drop(delete_table_documents_query);
        tx.commit()?;
        Ok(count_deleted)
    }
}

impl SqlitePersistence {
    /// Read the document log in `range`, restricted to `tablet_id` if given.
    ///
    /// Restricting in SQL keeps the untargeted rows out of memory entirely,
    /// and reading one bounded page per query keeps the targeted ones from
    /// being materialized all at once.
    fn stream_document_log(
        &self,
        tablet_id: Option<TabletId>,
        range: TimestampRange,
        order: Order,
        page_size: u32,
        retention_validator: Arc<dyn RetentionValidator>,
    ) -> DocumentStream<'_> {
        // `page_size` comes from the caller; guard against 0, which would never
        // terminate.
        let page_size = page_size.max(1) as usize;
        self._load_documents_paginated(tablet_id, range, order, page_size, retention_validator)
            .cooperative()
            .boxed()
    }
}

#[async_trait]
impl PersistenceReader for SqlitePersistence {
    fn load_documents(
        &self,
        range: TimestampRange,
        order: Order,
        page_size: u32,
        retention_validator: Arc<dyn RetentionValidator>,
    ) -> DocumentStream<'_> {
        self.stream_document_log(None, range, order, page_size, retention_validator)
    }

    fn load_documents_from_table(
        &self,
        tablet_id: TabletId,
        range: TimestampRange,
        order: Order,
        page_size: u32,
        retention_validator: Arc<dyn RetentionValidator>,
    ) -> DocumentStream<'_> {
        self.stream_document_log(
            Some(tablet_id),
            range,
            order,
            page_size,
            retention_validator,
        )
    }

    async fn previous_revisions(
        &self,
        ids: BTreeSet<(InternalDocumentId, Timestamp)>,
        retention_validator: Arc<dyn RetentionValidator>,
    ) -> anyhow::Result<BTreeMap<(InternalDocumentId, Timestamp), DocumentLogEntry>> {
        let mut out = BTreeMap::new();
        let mut min_ts = Timestamp::MAX;
        {
            let inner = self.inner.lock();
            for (id, ts) in ids {
                min_ts = cmp::min(ts, min_ts);
                let mut stmt = inner.connection.prepare(PREV_REV_QUERY)?;
                let internal_id = id.internal_id();
                let params = params![&id.table().0[..], &internal_id[..], &u64::from(ts)];
                let mut row_iter = stmt.query_map(params, load_document_row)?;
                if let Some(row) = row_iter.next() {
                    let (document_id, prev_ts, document, prev_prev_ts) = row_to_document(row)?;
                    out.insert(
                        (document_id, ts),
                        DocumentLogEntry {
                            ts: prev_ts,
                            id: document_id,
                            value: document,
                            prev_ts: prev_prev_ts,
                        },
                    );
                }
            }
        }
        retention_validator
            .validate_document_snapshot(min_ts)
            .await?;
        Ok(out)
    }

    async fn previous_revisions_of_documents(
        &self,
        ids: BTreeSet<DocumentPrevTsQuery>,
        retention_validator: Arc<dyn RetentionValidator>,
    ) -> anyhow::Result<BTreeMap<DocumentPrevTsQuery, DocumentLogEntry>> {
        // Validate retention for all queried timestamps first
        let min_ts = ids.iter().map(|DocumentPrevTsQuery { ts, .. }| *ts).min();

        let mut out = BTreeMap::new();
        {
            let inner = self.inner.lock();
            for DocumentPrevTsQuery { id, ts, prev_ts } in ids {
                let mut stmt = inner.connection.prepare(EXACT_REV_QUERY)?;
                let internal_id = id.internal_id();
                let params = params![&id.table().0[..], &internal_id[..], &u64::from(prev_ts)];
                let mut row_iter = stmt.query_map(params, load_document_row)?;
                if let Some(row) = row_iter.next() {
                    let (document_id, prev_ts, document, prev_prev_ts) = row_to_document(row)?;
                    out.insert(
                        DocumentPrevTsQuery {
                            id: document_id,
                            ts,
                            prev_ts,
                        },
                        DocumentLogEntry {
                            ts: prev_ts,
                            id: document_id,
                            value: document,
                            prev_ts: prev_prev_ts,
                        },
                    );
                }
            }
        }
        if let Some(min_ts) = min_ts {
            retention_validator
                .validate_document_snapshot(min_ts)
                .await?;
        }
        Ok(out)
    }

    fn index_scan(
        &self,
        index_id: IndexId,
        tablet_id: TabletId,
        read_timestamp: Timestamp,
        interval: &Interval,
        order: Order,
        size_hint: usize,
        retention_validator: Arc<dyn RetentionValidator>,
    ) -> IndexStream<'_> {
        // Mirror the Postgres reader: use the caller's size_hint to bound how
        // much of the interval each query materializes, so a small take()
        // over a large table no longer loads the entire range into memory.
        let batch_size = size_hint.clamp(1, 5000);
        self._index_scan_paginated(
            index_id,
            tablet_id,
            read_timestamp,
            interval.clone(),
            order,
            batch_size,
            retention_validator,
        )
        .boxed()
    }

    async fn get_persistence_global(
        &self,
        key: PersistenceGlobalKey,
    ) -> anyhow::Result<Option<JsonValue>> {
        self._get_persistence_global(key)
    }

    fn version(&self) -> PersistenceVersion {
        PersistenceVersion::V5
    }
}

const DOCUMENTS_INIT: &str = r#"
CREATE TABLE IF NOT EXISTS documents (
    id BLOB NOT NULL,
    ts INTEGER NOT NULL,

    table_id BLOB NOT NULL,

    json_value TEXT NULL,
    deleted INTEGER NOT NULL,

    prev_ts INTEGER,

    PRIMARY KEY (ts, table_id, id)
);
CREATE INDEX IF NOT EXISTS documents_by_table_and_id ON documents (table_id, id, ts);
"#;

const INDEXES_INIT: &str = r#"
CREATE TABLE IF NOT EXISTS indexes (
    index_id BLOB NOT NULL,
    ts INTEGER NOT NULL,

    key BLOB NOT NULL,

    deleted INTEGER NOT NULL,

    table_id BLOB NULL,
    document_id BLOB NULL,

    PRIMARY KEY (index_id, key, ts)
);
"#;

const PERSISTENCE_GLOBALS_INIT: &str = r#"
CREATE TABLE IF NOT EXISTS persistence_globals (
    key TEXT NOT NULL,
    json_value TEXT NOT NULL,

    PRIMARY KEY (key)
);
"#;

fn row_to_document(
    row: rusqlite::Result<(Vec<u8>, u64, Vec<u8>, Option<String>, bool, Option<u64>)>,
) -> anyhow::Result<(
    InternalDocumentId,
    Timestamp,
    Option<ResolvedDocument>,
    Option<Timestamp>,
)> {
    let (id, prev_ts, table, json_value, deleted, prev_prev_ts) = row?;
    let id = InternalId::try_from(id)?;
    let prev_ts = Timestamp::try_from(prev_ts)?;
    let table = TabletId(table.try_into()?);
    let document_id = InternalDocumentId::new(table, id);
    let document = if !deleted {
        let json_value = json_value
            .ok_or_else(|| anyhow::anyhow!("Unexpected NULL json_value at {} {}", id, prev_ts))?;
        let json_value: serde_json::Value = serde_json::from_str(&json_value)?;
        let value: ConvexValue = json_value.try_into()?;
        Some(ResolvedDocument::from_database(table, value)?)
    } else {
        None
    };
    let prev_prev_ts = prev_prev_ts.map(Timestamp::try_from).transpose()?;
    Ok((document_id, prev_ts, document, prev_prev_ts))
}

fn load_document_row(
    row: &Row<'_>,
) -> rusqlite::Result<(Vec<u8>, u64, Vec<u8>, Option<String>, bool, Option<u64>)> {
    let id = row.get::<_, Vec<u8>>(0)?;
    let ts = row.get::<_, u64>(1)?;
    let table: Vec<u8> = row.get(2)?;
    let json_value: Option<String> = row.get(3)?;
    let deleted = row.get::<_, u32>(4)? != 0;
    let prev_ts: Option<u64> = row.get(5)?;
    Ok((id, ts, table, json_value, deleted, prev_ts))
}

const GET_PERSISTENCE_GLOBAL: &str = "SELECT json_value FROM persistence_globals WHERE key = ?";

const INSERT_DOCUMENT: &str = "INSERT INTO documents (id, ts, table_id, json_value, deleted, \
                               prev_ts) VALUES (?, ?, ?, ?, ?, ?)";
const INSERT_OVERWRITE_DOCUMENT: &str = "INSERT OR REPLACE INTO documents (id, ts, table_id, \
                                         json_value, deleted, prev_ts) VALUES (?, ?, ?, ?, ?, ?)";
const INSERT_INDEX: &str = "INSERT INTO indexes VALUES (?, ?, ?, ?, ?, ?)";
const INSERT_OVERWRITE_INDEX: &str = "INSERT OR REPLACE INTO indexes VALUES (?, ?, ?, ?, ?, ?)";
const WRITE_PERSISTENCE_GLOBAL: &str = "INSERT OR REPLACE INTO persistence_globals VALUES (?, ?)";

const WALK_INDEXES: &str =
    "SELECT index_id, key, ts, deleted FROM indexes ORDER BY index_id ASC, key ASC, ts ASC";

const DELETE_INDEX: &str = "DELETE FROM indexes WHERE index_id = ? AND ts <= ? AND key = ?";

const DELETE_DOCUMENT: &str = "DELETE FROM documents WHERE table_id = ? AND id = ? AND ts <= ?";

const DELETE_TABLE_DOCUMENTS: &str = "DELETE FROM documents WHERE table_id = ? AND id IN (SELECT \
                                      id FROM documents WHERE table_id = ? LIMIT ?)";

const PREV_REV_QUERY: &str = r#"
SELECT id, ts, table_id, json_value, deleted, prev_ts
FROM documents
WHERE
    table_id = $1 AND
    id = $2 AND
    ts < $3
ORDER BY ts desc
LIMIT 1
"#;

const EXACT_REV_QUERY: &str = r#"
SELECT id, ts, table_id, json_value, deleted, prev_ts
FROM documents
WHERE
    table_id = $1 AND
    id = $2 AND
    ts = $3
ORDER BY ts ASC, table_id ASC, id ASC
"#;

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use common::{
        document::{
            CreationTime,
            ResolvedDocument,
        },
        index::IndexKeyBytes,
        interval::{
            BinaryKey,
            End,
            Interval,
            StartIncluded,
        },
        obj,
        persistence::{
            ConflictStrategy,
            DocumentLogEntry,
            LatestDocument,
            NoopRetentionValidator,
            Persistence,
            PersistenceIndexEntry,
            PersistenceReader,
            TimestampRange,
        },
        query::Order,
        types::{
            IndexId,
            Timestamp,
        },
        value::{
            DeveloperDocumentId,
            InternalDocumentId,
            InternalId,
            ResolvedDocumentId,
            TableNumber,
            TabletId,
        },
    };
    use futures::TryStreamExt;

    use super::SqlitePersistence;

    /// One indexed key and its history: `(ts, Some(v))` is a live write of
    /// value `v`, `(ts, None)` is a tombstone.
    struct KeySpec {
        key: Vec<u8>,
        doc_n: u8,
        versions: Vec<(u64, Option<i64>)>,
    }

    struct Fixture {
        persistence: SqlitePersistence,
        index_id: IndexId,
        tablet_id: TabletId,
        table_number: TableNumber,
        // Kept alive so the database file outlives the test body.
        _dir: tempfile::TempDir,
    }

    fn internal_id(n: u8) -> InternalId {
        InternalId::from([n; 16])
    }

    fn make_doc(
        tablet_id: TabletId,
        table_number: TableNumber,
        doc_n: u8,
        v: i64,
    ) -> anyhow::Result<ResolvedDocument> {
        let id = ResolvedDocumentId::new(
            tablet_id,
            DeveloperDocumentId::new(table_number, internal_id(doc_n)),
        );
        ResolvedDocument::new(id, CreationTime::try_from(1234.5)?, obj!("v" => v)?)
    }

    async fn make_fixture(specs: &[KeySpec]) -> anyhow::Result<Fixture> {
        let dir = tempfile::tempdir()?;
        let path = dir.path().join("test.sqlite3");
        let persistence = SqlitePersistence::new(path.to_str().unwrap())?;
        let index_id = IndexId(internal_id(255));
        let tablet_id = TabletId(internal_id(254));
        let table_number = TableNumber::try_from(1)?;

        let mut documents = vec![];
        let mut indexes = vec![];
        for spec in specs {
            let doc_id = InternalDocumentId::new(tablet_id, internal_id(spec.doc_n));
            for &(ts, v) in &spec.versions {
                let ts = Timestamp::try_from(ts)?;
                let value = v
                    .map(|v| make_doc(tablet_id, table_number, spec.doc_n, v))
                    .transpose()?;
                documents.push(DocumentLogEntry {
                    ts,
                    id: doc_id,
                    value,
                    prev_ts: None,
                });
                indexes.push(PersistenceIndexEntry {
                    ts,
                    index_id,
                    key: IndexKeyBytes(spec.key.clone()),
                    value: v.map(|_| doc_id),
                });
            }
        }
        persistence
            .write(&documents, &indexes, ConflictStrategy::Error)
            .await?;
        Ok(Fixture {
            persistence,
            index_id,
            tablet_id,
            table_number,
            _dir: dir,
        })
    }

    fn make_interval(start: Vec<u8>, end: Option<Vec<u8>>) -> Interval {
        Interval {
            start: StartIncluded(BinaryKey::from(start)),
            end: match end {
                Some(end) => End::Excluded(BinaryKey::from(end)),
                None => End::Unbounded,
            },
        }
    }

    /// The reference implementation: latest visible version per key at
    /// `read_ts`, tombstones dropped, interval applied, in scan order.
    fn expected_scan(
        fixture: &Fixture,
        specs: &[KeySpec],
        read_ts: u64,
        interval: &Interval,
        order: Order,
    ) -> anyhow::Result<Vec<(IndexKeyBytes, LatestDocument)>> {
        let mut rows = vec![];
        for spec in specs {
            if !interval.contains(&spec.key) {
                continue;
            }
            let visible = spec
                .versions
                .iter()
                .filter(|(ts, _)| *ts <= read_ts)
                .max_by_key(|(ts, _)| *ts);
            let Some(&(ts, Some(v))) = visible else {
                continue;
            };
            rows.push((
                IndexKeyBytes(spec.key.clone()),
                LatestDocument {
                    ts: Timestamp::try_from(ts)?,
                    value: make_doc(fixture.tablet_id, fixture.table_number, spec.doc_n, v)?,
                    prev_ts: None,
                },
            ));
        }
        rows.sort_by(|(a, _), (b, _)| a.cmp(b));
        if order == Order::Desc {
            rows.reverse();
        }
        Ok(rows)
    }

    async fn run_scan(
        fixture: &Fixture,
        read_ts: u64,
        interval: &Interval,
        order: Order,
        size_hint: usize,
    ) -> anyhow::Result<Vec<(IndexKeyBytes, LatestDocument)>> {
        fixture
            .persistence
            .index_scan(
                fixture.index_id,
                fixture.tablet_id,
                Timestamp::try_from(read_ts)?,
                interval,
                order,
                size_hint,
                Arc::new(NoopRetentionValidator),
            )
            .try_collect()
            .await
    }

    /// 25 keys with updates, tombstones on page boundaries, and writes past
    /// the read snapshot, scanned under every pagination regime: the
    /// paginated scan must be indistinguishable from the one-shot scan it
    /// replaced.
    #[tokio::test]
    async fn paginated_scan_matches_reference() -> anyhow::Result<()> {
        let specs: Vec<KeySpec> = (0u8..25)
            .map(|k| {
                let mut versions = vec![(10, Some(1))];
                if k % 3 == 0 {
                    versions.push((20, Some(2)));
                }
                if k % 5 == 0 {
                    versions.push((30, None));
                }
                if k % 2 == 0 {
                    versions.push((40, Some(3)));
                }
                KeySpec {
                    key: vec![k],
                    doc_n: k,
                    versions,
                }
            })
            .collect();
        let fixture = make_fixture(&specs).await?;

        let intervals = [
            make_interval(vec![], None),
            make_interval(vec![3], Some(vec![20])),
        ];
        // At ts 35 the tombstones (ts 30) are the latest visible versions;
        // at ts 45 the ts-40 writes resurrect the even keys among them.
        for read_ts in [35, 45] {
            for interval in &intervals {
                for order in [Order::Asc, Order::Desc] {
                    let expected = expected_scan(&fixture, &specs, read_ts, interval, order)?;
                    for size_hint in [1, 2, 3, 10_000] {
                        let got = run_scan(&fixture, read_ts, interval, order, size_hint).await?;
                        assert_eq!(
                            got, expected,
                            "scan mismatch at read_ts={read_ts} order={order:?} \
                             size_hint={size_hint}"
                        );
                    }
                }
            }
        }
        Ok(())
    }

    /// Ten consecutive tombstoned keys form entire pages with no live rows.
    /// A scan that terminates on "page yielded fewer live rows than
    /// requested" would stop in the middle of the interval and silently drop
    /// every key after the tombstone run.
    #[tokio::test]
    async fn page_of_tombstones_does_not_terminate_scan() -> anyhow::Result<()> {
        let specs: Vec<KeySpec> = (0u8..30)
            .map(|k| {
                let mut versions = vec![(10, Some(1))];
                if (10..20).contains(&k) {
                    versions.push((20, None));
                }
                KeySpec {
                    key: vec![k],
                    doc_n: k,
                    versions,
                }
            })
            .collect();
        let fixture = make_fixture(&specs).await?;

        let interval = make_interval(vec![], None);
        for order in [Order::Asc, Order::Desc] {
            let expected = expected_scan(&fixture, &specs, 25, &interval, order)?;
            assert_eq!(expected.len(), 20);
            let got = run_scan(&fixture, 25, &interval, order, 5).await?;
            assert_eq!(got, expected, "tombstone run must not end the scan early");
        }
        Ok(())
    }

    #[tokio::test]
    async fn empty_table_and_empty_range() -> anyhow::Result<()> {
        let empty = make_fixture(&[]).await?;
        let all = make_interval(vec![], None);
        assert!(run_scan(&empty, 100, &all, Order::Asc, 3).await?.is_empty());

        let specs: Vec<KeySpec> = (0u8..5)
            .map(|k| KeySpec {
                key: vec![k],
                doc_n: k,
                versions: vec![(10, Some(1))],
            })
            .collect();
        let fixture = make_fixture(&specs).await?;
        let out_of_range = make_interval(vec![200], Some(vec![201]));
        assert!(run_scan(&fixture, 100, &out_of_range, Order::Asc, 3)
            .await?
            .is_empty());
        Ok(())
    }

    /// One row of the documents log for the `load_documents` tests:
    /// `value: None` is a delete entry, which the log emits as-is.
    #[derive(Clone)]
    struct LogEntrySpec {
        ts: u64,
        tablet_n: u8,
        doc_n: u8,
        value: Option<i64>,
    }

    async fn make_log_fixture(
        specs: &[LogEntrySpec],
    ) -> anyhow::Result<(SqlitePersistence, tempfile::TempDir)> {
        let dir = tempfile::tempdir()?;
        let path = dir.path().join("test.sqlite3");
        let persistence = SqlitePersistence::new(path.to_str().unwrap())?;
        let documents = specs
            .iter()
            .map(log_entry)
            .collect::<anyhow::Result<Vec<_>>>()?;
        persistence
            .write(&documents, &[], ConflictStrategy::Error)
            .await?;
        Ok((persistence, dir))
    }

    fn log_entry(spec: &LogEntrySpec) -> anyhow::Result<DocumentLogEntry> {
        let tablet_id = TabletId(internal_id(spec.tablet_n));
        let table_number = TableNumber::try_from(1)?;
        let value = spec
            .value
            .map(|v| make_doc(tablet_id, table_number, spec.doc_n, v))
            .transpose()?;
        Ok(DocumentLogEntry {
            ts: Timestamp::try_from(spec.ts)?,
            id: InternalDocumentId::new(tablet_id, internal_id(spec.doc_n)),
            value,
            prev_ts: None,
        })
    }

    /// The reference implementation: every log row in the timestamp range,
    /// deletes included, in `(ts, table_id, id)` scan order.
    fn expected_log(
        specs: &[LogEntrySpec],
        range: &TimestampRange,
        order: Order,
    ) -> anyhow::Result<Vec<DocumentLogEntry>> {
        let mut rows = vec![];
        for spec in specs {
            let ts = Timestamp::try_from(spec.ts)?;
            if ts < range.min_timestamp_inclusive() || ts >= range.max_timestamp_exclusive() {
                continue;
            }
            let entry = log_entry(spec)?;
            let key = (
                spec.ts,
                entry.id.table().0[..].to_vec(),
                entry.id.internal_id()[..].to_vec(),
            );
            rows.push((key, entry));
        }
        rows.sort_by(|(a, _), (b, _)| a.cmp(b));
        if order == Order::Desc {
            rows.reverse();
        }
        Ok(rows.into_iter().map(|(_, entry)| entry).collect())
    }

    async fn run_load(
        persistence: &SqlitePersistence,
        range: TimestampRange,
        order: Order,
        page_size: u32,
    ) -> anyhow::Result<Vec<DocumentLogEntry>> {
        persistence
            .load_documents(range, order, page_size, Arc::new(NoopRetentionValidator))
            .try_collect()
            .await
    }

    async fn run_load_from_table(
        persistence: &SqlitePersistence,
        tablet_id: TabletId,
        range: TimestampRange,
        order: Order,
        page_size: u32,
    ) -> anyhow::Result<Vec<DocumentLogEntry>> {
        persistence
            .load_documents_from_table(
                tablet_id,
                range,
                order,
                page_size,
                Arc::new(NoopRetentionValidator),
            )
            .try_collect()
            .await
    }

    /// `load_documents_from_table` restricts the scan in SQL, so `LIMIT`
    /// counts only rows that will be yielded. Two tablets are interleaved at
    /// every timestamp, so a filter applied *after* the limit would hand back
    /// short pages and terminate the stream early, losing most of the log.
    /// Page sizes below the number of rows per timestamp also put a page
    /// boundary inside a same-`ts` run, where the cursor's `(table_id, id)`
    /// tiebreak has to hold even though `table_id` is now pinned by the
    /// filter.
    #[tokio::test]
    async fn paginated_load_from_table_matches_reference() -> anyhow::Result<()> {
        let mut specs = vec![];
        for ts in [10u64, 20, 30] {
            for tablet_n in [1u8, 2] {
                for doc_n in 0u8..4 {
                    specs.push(LogEntrySpec {
                        ts,
                        tablet_n,
                        doc_n,
                        value: Some(ts as i64),
                    });
                }
            }
        }
        // A delete in the targeted tablet, and a whole timestamp belonging to
        // the other one: neither may terminate the targeted stream.
        specs.push(LogEntrySpec {
            ts: 40,
            tablet_n: 1,
            doc_n: 0,
            value: None,
        });
        specs.push(LogEntrySpec {
            ts: 50,
            tablet_n: 2,
            doc_n: 0,
            value: None,
        });
        let (persistence, _dir) = make_log_fixture(&specs).await?;

        let target_n = 2u8;
        let target = TabletId(internal_id(target_n));
        let targeted: Vec<LogEntrySpec> = specs
            .iter()
            .filter(|spec| spec.tablet_n == target_n)
            .cloned()
            .collect();

        let ranges = [
            TimestampRange::all(),
            TimestampRange::new(Timestamp::try_from(15u64)?..Timestamp::try_from(45u64)?),
        ];
        for range in &ranges {
            for order in [Order::Asc, Order::Desc] {
                let expected = expected_log(&targeted, range, order)?;
                for page_size in [1, 3, 5, 10_000] {
                    let got =
                        run_load_from_table(&persistence, target, *range, order, page_size).await?;
                    assert_eq!(
                        got, expected,
                        "load_from_table mismatch at range={range:?} order={order:?} \
                         page_size={page_size}"
                    );
                    assert!(
                        got.iter().all(|entry| entry.id.table() == target),
                        "load_from_table returned a row from another tablet"
                    );
                }
            }
        }
        Ok(())
    }

    /// Two tablets writing interleaved updates and deletes, with several rows
    /// sharing the same timestamp: the compound cursor's `(table_id, id)`
    /// tiebreak is what keeps pagination correct when a page boundary lands
    /// in the middle of a same-`ts` run.
    #[tokio::test]
    async fn paginated_load_matches_reference() -> anyhow::Result<()> {
        let mut specs = vec![];
        for tablet_n in [1u8, 2] {
            for doc_n in 0u8..6 {
                specs.push(LogEntrySpec {
                    ts: 10,
                    tablet_n,
                    doc_n,
                    value: Some(1),
                });
                if doc_n % 2 == 0 {
                    specs.push(LogEntrySpec {
                        ts: 20,
                        tablet_n,
                        doc_n,
                        value: Some(2),
                    });
                }
                if doc_n % 3 == 0 {
                    specs.push(LogEntrySpec {
                        ts: 30,
                        tablet_n,
                        doc_n,
                        value: None,
                    });
                }
                if doc_n % 2 == 1 {
                    specs.push(LogEntrySpec {
                        ts: 40,
                        tablet_n,
                        doc_n,
                        value: Some(3),
                    });
                }
            }
        }
        let (persistence, _dir) = make_log_fixture(&specs).await?;

        let ranges = [
            TimestampRange::all(),
            TimestampRange::new(Timestamp::try_from(15u64)?..Timestamp::try_from(35u64)?),
        ];
        for range in &ranges {
            for order in [Order::Asc, Order::Desc] {
                let expected = expected_log(&specs, range, order)?;
                for page_size in [1, 2, 3, 10_000] {
                    let got = run_load(&persistence, *range, order, page_size).await?;
                    assert_eq!(
                        got, expected,
                        "load mismatch at range={range:?} order={order:?} page_size={page_size}"
                    );
                }
            }
        }
        Ok(())
    }

    /// Ten documents written at one single timestamp: every page boundary
    /// falls inside the same-`ts` run, so only the `(table_id, id)` tiebreak
    /// of the cursor separates the pages.
    #[tokio::test]
    async fn same_ts_run_spans_pages() -> anyhow::Result<()> {
        let specs: Vec<LogEntrySpec> = (0u8..10)
            .map(|doc_n| LogEntrySpec {
                ts: 17,
                tablet_n: 1,
                doc_n,
                value: Some(i64::from(doc_n)),
            })
            .collect();
        let (persistence, _dir) = make_log_fixture(&specs).await?;

        let range = TimestampRange::all();
        for order in [Order::Asc, Order::Desc] {
            let expected = expected_log(&specs, &range, order)?;
            assert_eq!(expected.len(), 10);
            let got = run_load(&persistence, range, order, 3).await?;
            assert_eq!(got, expected, "same-ts run must paginate by (table_id, id)");
        }
        Ok(())
    }

    #[tokio::test]
    async fn load_empty_and_out_of_range() -> anyhow::Result<()> {
        let (empty, _dir) = make_log_fixture(&[]).await?;
        assert!(run_load(&empty, TimestampRange::all(), Order::Asc, 2)
            .await?
            .is_empty());

        let specs: Vec<LogEntrySpec> = (0u8..5)
            .map(|doc_n| LogEntrySpec {
                ts: 10,
                tablet_n: 1,
                doc_n,
                value: Some(1),
            })
            .collect();
        let (persistence, _dir) = make_log_fixture(&specs).await?;
        let out_of_range =
            TimestampRange::new(Timestamp::try_from(100u64)?..Timestamp::try_from(200u64)?);
        assert!(run_load(&persistence, out_of_range, Order::Asc, 2)
            .await?
            .is_empty());
        Ok(())
    }
}

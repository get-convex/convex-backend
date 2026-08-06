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
        row_index_retention::{
            delete_expired_entries,
            IndexRowPersistence,
        },
        ConflictStrategy,
        DocumentLogEntry,
        DocumentPrevTsQuery,
        DocumentStream,
        IndexRetentionProgress,
        IndexRetentionRequest,
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
    try_anyhow,
    types::{
        IndexId,
        IndexRef,
        PersistenceVersion,
        Timestamp,
    },
    value::{
        ConvexValue,
        InternalDocumentId,
        TabletId,
    },
};
use futures::{
    stream,
    StreamExt,
};
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

    #[allow(clippy::needless_lifetimes)]
    #[try_stream(ok = T, error = anyhow::Error)]
    async fn validate_document_snapshot<T: 'static>(
        &self,
        ts: Timestamp,
        retention_validator: Arc<dyn RetentionValidator>,
    ) {
        retention_validator.validate_document_snapshot(ts).await?;
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
        let read_timestamp: i64 = read_timestamp.into();

        // `?1` is the index id and `?2` the read timestamp. Placeholders are
        // numbered (`?N`), not named: the read timestamp is referenced after
        // the key bounds, and SQLite numbers `$name` placeholders by their
        // first appearance in the query.
        let mut params = params![index_id, read_timestamp].to_vec();

        // A page resumes strictly after `cursor`, which always lies inside the
        // interval, so the cursor replaces the interval's bound on its side.
        // Given two bounds on one side SQLite seeks by either, and seeking by
        // the interval's would re-read every key the earlier pages returned.
        let cursor_bytes = cursor.map(|key| &key.0[..]);

        let StartIncluded(ref start) = interval.start;
        let (lower_bytes, lower_op) = match (order, cursor_bytes) {
            (Order::Asc, Some(cursor_bytes)) => (cursor_bytes, ">"),
            _ => (&start[..], ">="),
        };

        params.push(&lower_bytes);
        let lower = format!(" AND B.key {lower_op} ?{}", params.len());

        let upper_bytes = match (order, cursor_bytes, &interval.end) {
            (Order::Desc, Some(cursor_bytes), _) => Some(cursor_bytes),
            (_, _, End::Excluded(t)) => Some(&t[..]),
            (_, _, End::Unbounded) => None,
        };
        let upper = match upper_bytes {
            Some(ref t) => {
                params.push(t);
                format!(" AND B.key < ?{}", params.len())
            },
            None => "".to_owned(),
        };

        let order = match order {
            Order::Asc => "ASC",
            Order::Desc => "DESC",
        };
        // A walk of the `(index_id, key, ts)` primary key in scan order that
        // keeps, per key, the newest version at or before the read timestamp
        // (a correlated `MAX(ts)`, one seek per visited row), so `LIMIT` stops
        // the walk after `batch_size` keys and nothing is sorted. Keys whose
        // versions are all newer than the read timestamp have no such version
        // and are skipped. `B.deleted` is selected (not filtered) so that
        // tombstoned keys count toward the page: a long run of them is then
        // read over several pages, with the retention check between them,
        // instead of inside one query.
        let query = format!(
            r#"
SELECT B.key, B.ts, B.document_id, C.table_id, C.json_value, C.prev_ts, B.deleted
FROM indexes B
LEFT JOIN documents C
ON B.ts = C.ts
AND B.table_id = C.table_id
AND B.document_id = C.id
WHERE B.index_id = ?1{lower}{upper}
AND B.ts = (
    SELECT MAX(X.ts)
    FROM indexes X
    WHERE X.index_id = B.index_id AND X.key = B.key AND X.ts <= ?2
)
ORDER BY B.key {order}
LIMIT {batch_size}
"#,
        );

        let connection = &self.inner.lock().connection;
        let mut stmt = connection.prepare(&query)?;
        let row_iter = stmt.query_map(&params[..], |row| {
            let key = IndexKeyBytes(row.get::<_, Vec<u8>>(0)?);
            let ts = Timestamp::try_from(row.get::<_, i64>(1)?).expect("timestamp out of bounds");
            let document_id: Option<Vec<u8>> = row.get(2)?;
            let table: Option<Vec<u8>> = row.get(3)?;
            let json_value: Option<String> = row.get(4)?;
            let prev_ts: Option<Timestamp> = row
                .get::<_, Option<i64>>(5)?
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
impl IndexRowPersistence for SqlitePersistence {
    async fn delete_index_rows(&self, expired_rows: Vec<IndexEntry>) -> anyhow::Result<usize> {
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
                    .execute(params![&index_id.0[..], &i64::from(ts), key_prefix,])?;
        }
        drop(delete_index_query);
        tx.commit()?;
        Ok(count_deleted)
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
                &i64::from(update.ts),
                &update.id.table().0[..],
                &json_value,
                &deleted,
                &update.prev_ts.map(i64::from),
            ])?;
        }
        drop(insert_document_query);

        let mut insert_index_query = if conflict_strategy == ConflictStrategy::Overwrite {
            tx.prepare_cached(INSERT_OVERWRITE_INDEX)?
        } else {
            tx.prepare_cached(INSERT_INDEX)?
        };
        for update in indexes {
            let index_id = update.index.id();
            let key: &[u8] = &update.key.0;
            match update.value {
                None => {
                    insert_index_query.execute(params![
                        &index_id.0[..],
                        &i64::from(update.ts),
                        key,
                        &1,
                        &Null,
                        &Null,
                    ])?;
                },
                Some(doc_id) => {
                    insert_index_query.execute(params![
                        &index_id.0[..],
                        &i64::from(update.ts),
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

    async fn has_index_entries(&self) -> anyhow::Result<bool> {
        let connection = &self.inner.lock().connection;
        Ok(connection.prepare(HAS_INDEX_ENTRIES)?.exists([])?)
    }

    async fn reclaim_index_history(
        &self,
        request: IndexRetentionRequest<'_>,
    ) -> anyhow::Result<IndexRetentionProgress> {
        delete_expired_entries(self, request).await
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
                &i64::from(ts),
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
            i64::try_from(chunk_size)?,
        ])?;
        drop(delete_table_documents_query);
        tx.commit()?;
        Ok(count_deleted)
    }
}

impl SqlitePersistence {
    /// Read the document log in `range`, restricted to `tablet_id` if given.
    ///
    /// Rows are collected eagerly, so restricting in SQL keeps the untargeted
    /// ones out of memory entirely.
    fn stream_document_log(
        &self,
        tablet_id: Option<TabletId>,
        range: TimestampRange,
        order: Order,
        retention_validator: Arc<dyn RetentionValidator>,
    ) -> DocumentStream<'_> {
        let entries = try_anyhow!({
            let connection = &self.inner.lock().connection;
            let load_docs_query = load_docs(range, order, tablet_id.is_some());
            let mut stmt = connection.prepare(load_docs_query.as_str())?;

            let tablet_id_bytes = tablet_id.map(|tablet_id| tablet_id.0);
            let tablet_id_slice = tablet_id_bytes.as_ref().map(|tablet_id| &tablet_id[..]);
            let params: Vec<&dyn ToSql> = match tablet_id_slice {
                Some(ref tablet_id) => vec![tablet_id],
                None => vec![],
            };

            let mut entries = vec![];
            for row in stmt.query_map(params.as_slice(), load_document_row)? {
                let (document_id, ts, document, prev_ts) = row_to_document(row)?;
                entries.push(Ok(DocumentLogEntry {
                    ts,
                    id: document_id,
                    value: document,
                    prev_ts,
                }));
            }
            entries
        });
        // The caller isn't async so we have to validate snapshot as part of the
        // stream.
        let validate =
            self.validate_document_snapshot(range.min_timestamp_inclusive(), retention_validator);
        match entries {
            Ok(s) => validate.chain(stream::iter(s).cooperative()).boxed(),
            Err(e) => stream::once(async { Err(e) }).boxed(),
        }
    }
}

#[async_trait]
impl PersistenceReader for SqlitePersistence {
    fn load_documents(
        &self,
        range: TimestampRange,
        order: Order,
        _page_size: u32,
        retention_validator: Arc<dyn RetentionValidator>,
    ) -> DocumentStream<'_> {
        self.stream_document_log(None, range, order, retention_validator)
    }

    fn load_documents_from_table(
        &self,
        tablet_id: TabletId,
        range: TimestampRange,
        order: Order,
        _page_size: u32,
        retention_validator: Arc<dyn RetentionValidator>,
    ) -> DocumentStream<'_> {
        self.stream_document_log(Some(tablet_id), range, order, retention_validator)
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
                let params = params![&id.table().0[..], &internal_id[..], &i64::from(ts)];
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
                let params = params![&id.table().0[..], &internal_id[..], &i64::from(prev_ts)];
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
        index: IndexRef,
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
            index.id(),
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
    row: rusqlite::Result<(Vec<u8>, i64, Vec<u8>, Option<String>, bool, Option<i64>)>,
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

fn load_docs(range: TimestampRange, order: Order, tablet_filter: bool) -> String {
    let order_str = match order {
        Order::Asc => " ORDER BY ts ASC, table_id ASC, id ASC ",
        Order::Desc => " ORDER BY ts DESC, table_id DESC, id DESC ",
    };
    format!(
        r#"
SELECT id, ts, table_id, json_value, deleted, prev_ts
FROM documents
WHERE ts >= {} AND ts < {}
{}
{}
"#,
        range.min_timestamp_inclusive(),
        range.max_timestamp_exclusive(),
        if tablet_filter {
            "AND table_id = $1"
        } else {
            ""
        },
        order_str,
    )
}

fn load_document_row(
    row: &Row<'_>,
) -> rusqlite::Result<(Vec<u8>, i64, Vec<u8>, Option<String>, bool, Option<i64>)> {
    let id = row.get::<_, Vec<u8>>(0)?;
    let ts = row.get::<_, i64>(1)?;
    let table: Vec<u8> = row.get(2)?;
    let json_value: Option<String> = row.get(3)?;
    let deleted = row.get::<_, u32>(4)? != 0;
    let prev_ts: Option<i64> = row.get(5)?;
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

const HAS_INDEX_ENTRIES: &str = "SELECT 1 FROM indexes LIMIT 1";

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
    use std::sync::{
        atomic::{
            AtomicUsize,
            Ordering,
        },
        Arc,
    };

    use async_trait::async_trait;
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
            RetentionValidator,
        },
        query::Order,
        types::{
            IndexId,
            IndexRef,
            IndexWriteMode,
            PersistenceIndexId,
            PrevIndexEntry,
            RepeatableTimestamp,
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
    use futures::{
        stream::BoxStream,
        StreamExt,
        TryStreamExt,
    };

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
        index: IndexRef,
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
        id: InternalId,
        v: i64,
    ) -> anyhow::Result<ResolvedDocument> {
        let id = ResolvedDocumentId::new(tablet_id, DeveloperDocumentId::new(table_number, id));
        ResolvedDocument::new(id, CreationTime::try_from(1234.5)?, obj!("v" => v)?)
    }

    fn index_ref(n: u8, persistence_index_id: u32) -> IndexRef {
        IndexRef::from_parts(
            IndexId(internal_id(n)),
            PersistenceIndexId::new(persistence_index_id),
        )
    }

    async fn make_fixture(specs: &[KeySpec]) -> anyhow::Result<Fixture> {
        let dir = tempfile::tempdir()?;
        let path = dir.path().join("test.sqlite3");
        let fixture = Fixture {
            persistence: SqlitePersistence::new(path.to_str().unwrap())?,
            index: index_ref(255, 1),
            tablet_id: TabletId(internal_id(254)),
            table_number: TableNumber::try_from(1)?,
            _dir: dir,
        };
        write_index(&fixture, fixture.index, specs).await?;
        Ok(fixture)
    }

    /// Writes each spec's documents and their entries in `index`.
    async fn write_index(
        fixture: &Fixture,
        index: IndexRef,
        specs: &[KeySpec],
    ) -> anyhow::Result<()> {
        let mut documents = vec![];
        let mut indexes = vec![];
        for spec in specs {
            let doc_id = InternalDocumentId::new(fixture.tablet_id, internal_id(spec.doc_n));
            // The live entry a write supersedes, as the indexer reports it.
            let mut prev = None;
            for &(ts, v) in &spec.versions {
                let ts = Timestamp::try_from(ts)?;
                let value = v
                    .map(|v| {
                        make_doc(
                            fixture.tablet_id,
                            fixture.table_number,
                            internal_id(spec.doc_n),
                            v,
                        )
                    })
                    .transpose()?;
                documents.push(DocumentLogEntry {
                    ts,
                    id: doc_id,
                    value,
                    prev_ts: None,
                });
                indexes.push(PersistenceIndexEntry {
                    mode: IndexWriteMode::ScanComplete,
                    ts,
                    index,
                    key: IndexKeyBytes(spec.key.clone()),
                    value: v.map(|_| doc_id),
                    prev,
                });
                prev = v.map(|_| PrevIndexEntry {
                    ts,
                    document_id: doc_id,
                });
            }
        }
        fixture
            .persistence
            .write(&documents, &indexes, ConflictStrategy::Error)
            .await
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
                    value: make_doc(
                        fixture.tablet_id,
                        fixture.table_number,
                        internal_id(spec.doc_n),
                        v,
                    )?,
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
        scan_index(fixture, fixture.index, read_ts, interval, order, size_hint).await
    }

    async fn scan_index(
        fixture: &Fixture,
        index: IndexRef,
        read_ts: u64,
        interval: &Interval,
        order: Order,
        size_hint: usize,
    ) -> anyhow::Result<Vec<(IndexKeyBytes, LatestDocument)>> {
        fixture
            .persistence
            .index_scan(
                index,
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

    /// The first `n` rows of a scan asked for `n`, as a `take(n)` over an
    /// index range reads them.
    async fn first_keys(
        fixture: &Fixture,
        interval: &Interval,
        order: Order,
        n: usize,
    ) -> anyhow::Result<Vec<(IndexKeyBytes, LatestDocument)>> {
        fixture
            .persistence
            .index_scan(
                fixture.index,
                fixture.tablet_id,
                Timestamp::try_from(15u64)?,
                interval,
                order,
                n,
                Arc::new(NoopRetentionValidator),
            )
            .take(n)
            .try_collect()
            .await
    }

    /// `n` live keys in `fixture.index` at ts 10, each the big-endian bytes
    /// of its number, over a document of its own.
    async fn write_many_keys(fixture: &Fixture, n: u32) -> anyhow::Result<()> {
        let ts = Timestamp::try_from(10u64)?;
        let mut documents = vec![];
        let mut indexes = vec![];
        for k in 0..n {
            let mut id = [0; 16];
            id[..4].copy_from_slice(&k.to_be_bytes());
            let id = InternalId::from(id);
            let doc_id = InternalDocumentId::new(fixture.tablet_id, id);
            documents.push(DocumentLogEntry {
                ts,
                id: doc_id,
                value: Some(make_doc(
                    fixture.tablet_id,
                    fixture.table_number,
                    id,
                    k.into(),
                )?),
                prev_ts: None,
            });
            indexes.push(PersistenceIndexEntry {
                mode: IndexWriteMode::ScanComplete,
                ts,
                index: fixture.index,
                key: IndexKeyBytes(k.to_be_bytes().to_vec()),
                value: Some(doc_id),
                prev: None,
            });
        }
        fixture
            .persistence
            .write(&documents, &indexes, ConflictStrategy::Error)
            .await
    }

    /// Starts the read `start` returns and returns its output with the number
    /// of SQLite VM instructions it ran on `persistence`'s connection, in
    /// tens: how much work a read did, independent of timing. The read is
    /// started only once the count has begun: the one-shot reads this file
    /// replaced ran their query when they were created, not when polled, and
    /// would otherwise go uncounted.
    async fn sqlite_work<T, F>(
        persistence: &SqlitePersistence,
        start: impl FnOnce() -> F,
    ) -> anyhow::Result<(T, usize)>
    where
        F: Future<Output = anyhow::Result<T>>,
    {
        let tens = Arc::new(AtomicUsize::new(0));
        let counter = tens.clone();
        persistence.inner.lock().connection.progress_handler(
            10,
            Some(move || {
                counter.fetch_add(1, Ordering::Relaxed);
                false
            }),
        )?;
        let output = start().await;
        persistence
            .inner
            .lock()
            .connection
            .progress_handler(0, None::<fn() -> bool>)?;
        Ok((output?, tens.load(Ordering::Relaxed)))
    }

    /// Passes the first `valid` validations and fails every later one, as if
    /// retention moved past the snapshot once `valid` pages had been read.
    struct ExpiresAfter {
        valid: usize,
        calls: AtomicUsize,
    }

    impl ExpiresAfter {
        fn new(valid: usize) -> Arc<Self> {
            Arc::new(Self {
                valid,
                calls: AtomicUsize::new(0),
            })
        }

        fn check(&self) -> anyhow::Result<()> {
            let call = self.calls.fetch_add(1, Ordering::Relaxed);
            anyhow::ensure!(call < self.valid, "snapshot expired");
            Ok(())
        }
    }

    #[async_trait]
    impl RetentionValidator for ExpiresAfter {
        fn optimistic_validate_snapshot(&self, _ts: Timestamp) -> anyhow::Result<()> {
            Ok(())
        }

        async fn validate_snapshot(&self, _ts: Timestamp) -> anyhow::Result<()> {
            self.check()
        }

        async fn validate_document_snapshot(&self, _ts: Timestamp) -> anyhow::Result<()> {
            self.check()
        }

        async fn min_snapshot_ts(&self) -> anyhow::Result<RepeatableTimestamp> {
            Ok(RepeatableTimestamp::MIN)
        }

        async fn min_document_snapshot_ts(&self) -> anyhow::Result<RepeatableTimestamp> {
            Ok(RepeatableTimestamp::MIN)
        }
    }

    /// How many rows `stream` yields before its first error, or `None` if it
    /// ends without one.
    async fn rows_before_error<T>(mut stream: BoxStream<'_, anyhow::Result<T>>) -> Option<usize> {
        let mut rows = 0;
        while let Some(row) = stream.next().await {
            match row {
                Ok(_) => rows += 1,
                Err(_) => return Some(rows),
            }
        }
        None
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
                    for size_hint in [0, 1, 2, 3, 10_000] {
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

    /// A second index on the same table holds the same keys over other
    /// documents and sorts before the first in the `(index_id, key, ts)`
    /// primary key, so paging either index in either order walks toward the
    /// other's rows: each scan must still return only the index its
    /// `IndexRef` names.
    #[tokio::test]
    async fn scan_stays_within_its_index() -> anyhow::Result<()> {
        let make_specs = |first_doc: u8, v: i64| -> Vec<KeySpec> {
            (0u8..12)
                .map(|k| KeySpec {
                    key: vec![k],
                    doc_n: first_doc + k,
                    versions: vec![(10, Some(v))],
                })
                .collect()
        };
        let (specs_a, specs_b) = (make_specs(0, 1), make_specs(100, 2));
        let fixture = make_fixture(&specs_a).await?;
        let index_b = index_ref(253, 2);
        write_index(&fixture, index_b, &specs_b).await?;

        let interval = make_interval(vec![], None);
        for (index, specs) in [(fixture.index, &specs_a), (index_b, &specs_b)] {
            for order in [Order::Asc, Order::Desc] {
                let expected = expected_scan(&fixture, specs, 15, &interval, order)?;
                assert_eq!(expected.len(), 12);
                for size_hint in [1, 5, 10_000] {
                    let got = scan_index(&fixture, index, 15, &interval, order, size_hint).await?;
                    assert_eq!(
                        got, expected,
                        "scan of {index:?} order={order:?} size_hint={size_hint}"
                    );
                }
            }
        }
        Ok(())
    }

    /// The #495 property itself, which matching a reference cannot show: a
    /// scan that runs one bounded query per page returns the same rows as
    /// one that reads the whole interval and drops what it does not need.
    /// Counting the SQLite VM instructions a read runs tells them apart. A
    /// page must cost the same however many keys the interval holds, and
    /// paging through an interval must cost about as much in small pages as
    /// in large ones, so that no page re-reads what an earlier one returned.
    #[tokio::test]
    async fn a_page_reads_only_its_keys() -> anyhow::Result<()> {
        let small = make_fixture(&[]).await?;
        write_many_keys(&small, 100).await?;
        let large = make_fixture(&[]).await?;
        write_many_keys(&large, 10_000).await?;
        let all = make_interval(vec![], None);
        for order in [Order::Asc, Order::Desc] {
            let (_, small_page) =
                sqlite_work(&small.persistence, || first_keys(&small, &all, order, 10)).await?;
            let (_, large_page) =
                sqlite_work(&large.persistence, || first_keys(&large, &all, order, 10)).await?;
            assert!(
                large_page <= 2 * small_page,
                "{order:?}: a page of 10 keys ran {} instructions over 10,000 keys, {} over 100",
                large_page * 10,
                small_page * 10,
            );

            let (rows, paged) =
                sqlite_work(&large.persistence, || run_scan(&large, 15, &all, order, 10)).await?;
            assert_eq!(rows.len(), 10_000);
            let (_, unpaged) = sqlite_work(&large.persistence, || {
                run_scan(&large, 15, &all, order, 10_000)
            })
            .await?;
            assert!(
                paged <= 2 * unpaged,
                "{order:?}: scanning 10,000 keys ran {} instructions in pages of 10, {} in pages \
                 of 5,000",
                paged * 10,
                unpaged * 10,
            );
        }
        Ok(())
    }

    /// Pages are read at different times, so each one is validated against
    /// retention after it is read and before any of its rows are yielded: a
    /// snapshot that expires after the first page must fail the scan before
    /// a row of the second page comes out.
    #[tokio::test]
    async fn scan_validates_each_page() -> anyhow::Result<()> {
        let fixture = make_fixture(&[]).await?;
        write_many_keys(&fixture, 30).await?;
        let all = make_interval(vec![], None);
        for order in [Order::Asc, Order::Desc] {
            let stream = fixture.persistence.index_scan(
                fixture.index,
                fixture.tablet_id,
                Timestamp::try_from(15u64)?,
                &all,
                order,
                10,
                ExpiresAfter::new(1),
            );
            assert_eq!(rows_before_error(stream).await, Some(10), "{order:?}");
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
}

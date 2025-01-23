#![feature(try_blocks)]
#![feature(let_chains)]
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
        DocumentStream,
        IndexStream,
        Persistence,
        PersistenceGlobalKey,
        PersistenceReader,
        RetentionValidator,
        TimestampRange,
    },
    query::Order,
    types::{
        DatabaseIndexUpdate,
        DatabaseIndexValue,
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
    pub fn new(path: &str, allow_read_only: bool) -> anyhow::Result<Self> {
        let newly_created = !Path::new(path).exists();
        let connection = Connection::open(path)?;
        // Execute create tables unconditionally since they are idempotent.
        connection.execute_batch(DOCUMENTS_INIT)?;
        connection.execute_batch(INDEXES_INIT)?;
        connection.execute_batch(READ_ONLY_INIT)?;
        connection.execute_batch(PERSISTENCE_GLOBALS_INIT)?;
        if !allow_read_only {
            let mut stmt = connection.prepare(CHECK_IS_READ_ONLY)?;
            anyhow::ensure!(stmt.raw_query().next()?.is_none());
        }
        Ok(Self {
            inner: Arc::new(Mutex::new(Inner {
                newly_created,
                connection,
            })),
        })
    }

    #[allow(clippy::needless_lifetimes)]
    #[try_stream(ok = T, error = anyhow::Error)]
    async fn validate_snapshot<T: 'static>(
        &self,
        ts: Timestamp,
        retention_validator: Arc<dyn RetentionValidator>,
    ) {
        retention_validator.validate_snapshot(ts).await?;
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

    fn _index_scan_inner(
        &self,
        index_id: IndexId,
        tablet_id: TabletId,
        read_timestamp: Timestamp,
        interval: &Interval,
        order: Order,
    ) -> anyhow::Result<Vec<anyhow::Result<(IndexKeyBytes, Timestamp, ResolvedDocument)>>> {
        let interval = interval.clone();
        let index_id = &index_id[..];
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

        let order = match order {
            Order::Asc => "ASC",
            Order::Desc => "DESC",
        };
        let query = format!(
            r#"
SELECT B.key, B.ts, B.document_id, C.table_id, C.json_value
FROM (
    SELECT index_id, key, MAX(ts) as max_ts
    FROM indexes
    WHERE index_id = $1 AND ts <= $2{lower}{upper}
    GROUP BY index_id, key
) A
JOIN indexes B
ON B.deleted is FALSE
AND A.index_id = B.index_id
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
            let document_id = row.get::<_, Vec<u8>>(2)?;
            let table: Option<Vec<u8>> = row.get(3)?;
            let json_value: Option<String> = row.get(4)?;

            Ok((key, ts, document_id, table, json_value))
        })?;
        let mut triples = vec![];
        for row in row_iter {
            let (key, ts, document_id, table, json_value) = row?;
            let table = table.ok_or_else(|| {
                anyhow::anyhow!("Dangling index reference for {:?} {:?}", key, ts)
            })?;
            let table = TabletId(table.try_into()?);
            let _document_id = InternalDocumentId::new(table, InternalId::try_from(document_id)?);
            let json_value = json_value.ok_or_else(|| {
                anyhow::anyhow!("Index reference to deleted document {:?} {:?}", key, ts)
            })?;
            let json_value: serde_json::Value = serde_json::from_str(&json_value)?;
            let value: ConvexValue = json_value.try_into()?;
            let document = ResolvedDocument::from_database(tablet_id, value)?;
            triples.push(Ok((key, ts, document)));
        }
        Ok(triples)
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
                let json_value: serde_json::Value = serde_json::from_str(&json_value_str?)?;
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

    async fn write(
        &self,
        documents: Vec<DocumentLogEntry>,
        indexes: BTreeSet<(Timestamp, DatabaseIndexUpdate)>,
        conflict_strategy: ConflictStrategy,
    ) -> anyhow::Result<()> {
        let mut inner = self.inner.lock();
        let tx = inner.connection.transaction()?;
        let mut insert_document_query = match conflict_strategy {
            ConflictStrategy::Error => tx.prepare_cached(INSERT_DOCUMENT)?,
            ConflictStrategy::Overwrite => tx.prepare_cached(INSERT_OVERWRITE_DOCUMENT)?,
        };

        for update in documents {
            let (json_value, deleted) = if let Some(document) = update.value {
                assert_eq!(update.id, document.id_with_table_id());
                let json_value: serde_json::Value = document.value().0.clone().into();
                let json_value = serde_json::to_string(&json_value)?;
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
        for (ts, update) in indexes {
            let index_id = update.index_id;
            let key: Vec<u8> = update.key.into_bytes().0;
            match update.value {
                DatabaseIndexValue::Deleted => {
                    insert_index_query.execute(params![
                        &index_id[..],
                        &u64::from(ts),
                        key,
                        &1,
                        &Null,
                        &Null,
                    ])?;
                },
                DatabaseIndexValue::NonClustered(doc_id) => {
                    insert_index_query.execute(params![
                        &index_id[..],
                        &u64::from(ts),
                        key,
                        &0,
                        &doc_id.tablet_id.0[..],
                        &doc_id.internal_id()[..],
                    ])?;
                },
            };
        }
        drop(insert_index_query);

        tx.commit()?;
        Ok(())
    }

    async fn set_read_only(&self, read_only: bool) -> anyhow::Result<()> {
        let stmt = if read_only {
            SET_READ_ONLY
        } else {
            UNSET_READ_ONLY
        };
        self.inner.lock().connection.execute_batch(stmt)?;
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
                    index_id: index_id.try_into()?,
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
                delete_index_query.execute(params![&index_id[..], &u64::from(ts), key_prefix,])?;
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
        let triples = try {
            let connection = &self.inner.lock().connection;
            let load_docs_query = load_docs(range, order);
            let mut stmt = connection.prepare(load_docs_query.as_str())?;

            let mut entries = vec![];
            for row in stmt.query_map([], load_document_row)? {
                let (id, ts, table, json_value, deleted, prev_ts) = row?;
                let id = InternalId::try_from(id)?;
                let ts = Timestamp::try_from(ts)?;
                let table = TabletId(table.try_into()?);
                let document_id = InternalDocumentId::new(table, id);
                let document = if !deleted {
                    let json_value = json_value.ok_or_else(|| {
                        anyhow::anyhow!("Unexpected NULL json_value at {} {}", id, ts)
                    })?;
                    let json_value: serde_json::Value = serde_json::from_str(&json_value)?;
                    let value: ConvexValue = json_value.try_into()?;
                    let document = ResolvedDocument::from_database(table, value)?;
                    Some(document)
                } else {
                    None
                };
                let prev_ts = prev_ts.map(Timestamp::try_from).transpose()?;
                entries.push(Ok(DocumentLogEntry {
                    ts,
                    id: document_id,
                    value: document,
                    prev_ts,
                }));
            }
            entries
        };
        // load_documents isn't async so we have to validate snapshot as part of the
        // stream.
        let validate =
            self.validate_document_snapshot(range.min_timestamp_inclusive(), retention_validator);
        match triples {
            Ok(s) => (validate.chain(stream::iter(s))).boxed(),
            Err(e) => stream::once(async { Err(e) }).boxed(),
        }
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
                let mut stmt = inner.connection.prepare(PREV_REV_QUERY)?;
                let internal_id = id.internal_id();
                let params = params![&id.table().0[..], &internal_id[..], &u64::from(ts)];
                let mut row_iter = stmt.query_map(params, load_document_row)?;
                if let Some(row) = row_iter.next() {
                    let (id, prev_ts, table, json_value, deleted, prev_prev_ts) = row?;
                    let id = InternalId::try_from(id)?;
                    let table = TabletId(table.try_into()?);
                    let prev_ts = Timestamp::try_from(prev_ts)?;
                    let document_id = InternalDocumentId::new(table, id);
                    let document = if !deleted {
                        let json_value = json_value.ok_or_else(|| {
                            anyhow::anyhow!("Unexpected NULL json_value at {} {}", id, prev_ts)
                        })?;
                        let json_value: serde_json::Value = serde_json::from_str(&json_value)?;
                        let value: ConvexValue = json_value.try_into()?;
                        let document = ResolvedDocument::from_database(table, value)?;
                        Some(document)
                    } else {
                        None
                    };
                    let prev_prev_ts = prev_prev_ts.map(Timestamp::try_from).transpose()?;
                    min_ts = cmp::min(ts, min_ts);
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

    fn index_scan(
        &self,
        index_id: IndexId,
        tablet_id: TabletId,
        read_timestamp: Timestamp,
        interval: &Interval,
        order: Order,
        _size_hint: usize,
        retention_validator: Arc<dyn RetentionValidator>,
    ) -> IndexStream<'_> {
        let triples = self._index_scan_inner(index_id, tablet_id, read_timestamp, interval, order);
        // index_scan isn't async so we have to validate snapshot as part of the stream.
        let validate = self.validate_snapshot(read_timestamp, retention_validator);
        match triples {
            Ok(s) => (validate.chain(stream::iter(s))).boxed(),
            Err(e) => stream::once(async { Err(e) }).boxed(),
        }
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

const READ_ONLY_INIT: &str = r#"
CREATE TABLE IF NOT EXISTS read_only (
    id INTEGER NOT NULL,

    PRIMARY KEY (id)
);
"#;

const PERSISTENCE_GLOBALS_INIT: &str = r#"
CREATE TABLE IF NOT EXISTS persistence_globals (
    key TEXT NOT NULL,
    json_value TEXT NOT NULL,

    PRIMARY KEY (key)
);
"#;

fn load_docs(range: TimestampRange, order: Order) -> String {
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
"#,
        range.min_timestamp_inclusive(),
        range.max_timestamp_exclusive(),
        order_str,
    )
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

const CHECK_IS_READ_ONLY: &str = "SELECT 1 FROM read_only LIMIT 1";
const SET_READ_ONLY: &str = "INSERT INTO read_only (id) VALUES (1)";
const UNSET_READ_ONLY: &str = "DELETE FROM read_only WHERE id = 1";

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

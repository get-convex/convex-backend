#![feature(coroutines)]
#![feature(proc_macro_hygiene)]
#![feature(stmt_expr_attributes)]
#![feature(type_alias_impl_trait)]
#![feature(let_chains)]
#![feature(impl_trait_in_assoc_type)]
#![feature(try_blocks)]
mod chunks;
mod connection;
mod metrics;
#[cfg(test)]
mod tests;
use std::{
    borrow::Cow,
    cmp,
    collections::{
        BTreeMap,
        BTreeSet,
        HashMap,
    },
    fmt::Write,
    future::Future,
    ops::Bound,
    pin::Pin,
    sync::{
        atomic::{
            AtomicBool,
            Ordering::SeqCst,
        },
        Arc,
        LazyLock,
    },
    time::{
        SystemTime,
        UNIX_EPOCH,
    },
};

use anyhow::Context;
use async_trait::async_trait;
use chunks::{
    smart_chunk_sizes,
    ApproxSize,
};
use common::{
    document::{
        InternalId,
        ResolvedDocument,
    },
    errors::lease_lost_error,
    heap_size::HeapSize,
    index::{
        IndexEntry,
        IndexKeyBytes,
        SplitKey,
        MAX_INDEX_KEY_PREFIX_LEN,
    },
    interval::{
        End,
        Interval,
        StartIncluded,
    },
    knobs::{
        MYSQL_MAX_QUERY_BATCH_SIZE,
        MYSQL_MAX_QUERY_DYNAMIC_BATCH_SIZE,
        MYSQL_MIN_QUERY_BATCH_SIZE,
    },
    persistence::{
        ConflictStrategy,
        DocumentLogEntry,
        DocumentStream,
        IndexStream,
        Persistence,
        PersistenceGlobalKey,
        PersistenceReader,
        PersistenceTableSize,
        RetentionValidator,
        TimestampRange,
    },
    query::Order,
    runtime::Runtime,
    sha256::Sha256,
    shutdown::ShutdownSignal,
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
        ResolvedDocumentId,
        TabletId,
    },
};
pub use connection::ConvexMySqlPool;
use connection::{
    MySqlConnection,
    MySqlTransaction,
};
use futures::{
    pin_mut,
    stream::{
        StreamExt,
        TryStreamExt,
    },
    FutureExt,
};
use futures_async_stream::try_stream;
use itertools::{
    iproduct,
    Itertools,
};
use metrics::write_persistence_global_timer;
use minitrace::prelude::*;
use mysql_async::Row;
use serde_json::Value as JsonValue;

use crate::{
    chunks::smart_chunks,
    metrics::{
        log_prev_revisions_row_read,
        QueryIndexStats,
    },
};

pub struct MySqlPersistence<RT: Runtime> {
    newly_created: AtomicBool,
    lease: Lease<RT>,

    // Used by the reader.
    read_pool: Arc<ConvexMySqlPool<RT>>,
    db_name: String,
    version: PersistenceVersion,
}

#[derive(thiserror::Error, Debug)]
pub enum ConnectError {
    #[error("persistence is read-only, data migration in progress")]
    ReadOnly,
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

#[derive(Clone)]
pub struct MySqlOptions {
    pub allow_read_only: bool,
    pub version: PersistenceVersion,
    pub use_prepared_statements: bool,
}

pub struct MySqlReaderOptions {
    pub db_should_be_leader: bool,
    pub version: PersistenceVersion,
}

impl<RT: Runtime> MySqlPersistence<RT> {
    pub async fn new(
        pool: Arc<ConvexMySqlPool<RT>>,
        db_name: String,
        options: MySqlOptions,
        lease_lost_shutdown: ShutdownSignal,
    ) -> Result<Self, ConnectError> {
        let newly_created = {
            let mut client = pool.acquire("init_sql", &db_name).await?;
            let table_count: usize = client
                .query_optional(GET_TABLE_COUNT, vec![])
                .await
                .map_err(Into::<anyhow::Error>::into)?
                .context("GET_TABLE_COUNT query returned no rows?")?
                .get(0)
                .context("GET_TABLE_COUNT query returned zero columns?")?;
            // Only run INIT_SQL if we have less tables than we expect. We suspect
            // CREATE TABLE IF EXISTS is creating lock contention due to acquiring
            // an exclusive lock https://bugs.mysql.com/bug.php?id=63144.
            if table_count < EXPECTED_TABLE_COUNT {
                tracing::info!("Initializing MySQL Persistence...");
                client
                    .execute_many(INIT_SQL)
                    .await
                    .map_err(Into::<anyhow::Error>::into)?;
            } else {
                tracing::info!("MySQL Persistence already initialized");
            }
            Self::check_newly_created(&mut client).await?
        };
        let mut client = pool.acquire("read_only", &db_name).await?;
        if !options.allow_read_only && Self::is_read_only(&mut client).await? {
            return Err(ConnectError::ReadOnly);
        }

        let lease = Lease::acquire(pool.clone(), db_name.clone(), lease_lost_shutdown).await?;
        Ok(Self {
            newly_created: newly_created.into(),
            lease,
            read_pool: pool,
            db_name,
            version: options.version,
        })
    }

    pub fn new_reader(
        pool: Arc<ConvexMySqlPool<RT>>,
        db_name: String,
        options: MySqlReaderOptions,
    ) -> MySqlReader<RT> {
        MySqlReader {
            db_name,
            read_pool: pool,
            db_should_be_leader: options.db_should_be_leader,
            version: options.version,
        }
    }

    async fn is_read_only(client: &mut MySqlConnection<'_>) -> anyhow::Result<bool> {
        Ok(client
            .query_optional(CHECK_IS_READ_ONLY, vec![])
            .await?
            .is_some())
    }

    async fn check_newly_created(client: &mut MySqlConnection<'_>) -> anyhow::Result<bool> {
        Ok(client
            .query_optional(CHECK_NEWLY_CREATED, vec![])
            .await?
            .is_none())
    }

    #[cfg(test)]
    pub(crate) async fn get_table_count(&self) -> anyhow::Result<usize> {
        let mut client = self
            .read_pool
            .acquire("get_table_count", &self.db_name)
            .await?;
        client
            .query_optional(GET_TABLE_COUNT, vec![])
            .await
            .map_err(Into::<anyhow::Error>::into)?
            .context("GET_TABLE_COUNT query returned no rows?")?
            .get(0)
            .context("GET_TABLE_COUNT query returned zero columns?")
    }
}

#[async_trait]
impl<RT: Runtime> Persistence for MySqlPersistence<RT> {
    fn is_fresh(&self) -> bool {
        self.newly_created.load(SeqCst)
    }

    fn reader(&self) -> Arc<dyn PersistenceReader> {
        Arc::new(MySqlReader {
            db_name: self.db_name.clone(),
            read_pool: self.read_pool.clone(),
            db_should_be_leader: true,
            version: self.version,
        })
    }

    #[minitrace::trace]
    async fn write(
        &self,
        documents: Vec<DocumentLogEntry>,
        indexes: BTreeSet<(Timestamp, DatabaseIndexUpdate)>,
        conflict_strategy: ConflictStrategy,
    ) -> anyhow::Result<()> {
        anyhow::ensure!(documents.len() <= MAX_INSERT_SIZE);
        let mut write_size = 0;
        for update in &documents {
            match &update.value {
                Some(doc) => {
                    anyhow::ensure!(update.id == doc.id_with_table_id());
                    write_size += doc.heap_size();
                },
                None => {},
            }
        }
        metrics::log_write_bytes(write_size);
        metrics::log_write_documents(documents.len());
        Event::add_to_local_parent("write_to_persistence_size", || {
            [
                (
                    Cow::Borrowed("num_documents"),
                    Cow::Owned(documents.len().to_string()),
                ),
                (
                    Cow::Borrowed("write_size"),
                    Cow::Owned(write_size.to_string()),
                ),
            ]
        });

        // True, the below might end up failing and not changing anything.
        self.newly_created.store(false, SeqCst);
        let cluster_name = self.read_pool.cluster_name().to_owned();
        self.lease
            .transact(move |tx| {
                async move {
                    {
                        // First, process all of the full document chunks.
                        let mut document_chunks = smart_chunks(&documents);
                        for chunk in &mut document_chunks {
                            let chunk_bytes: usize =
                                chunk.iter().map(|item| item.approx_size()).sum();
                            let insert_chunk_query = match conflict_strategy {
                                ConflictStrategy::Error => insert_document_chunk(chunk.len()),
                                ConflictStrategy::Overwrite => {
                                    insert_overwrite_document_chunk(chunk.len())
                                },
                            };
                            let mut insert_document_chunk = vec![];
                            for update in chunk {
                                insert_document_chunk = document_params(
                                    insert_document_chunk,
                                    update.ts,
                                    update.id,
                                    update.value.clone(),
                                    update.prev_ts,
                                );
                            }
                            let future = async {
                                let timer =
                                    metrics::insert_document_chunk_timer(cluster_name.as_str());
                                tx.exec_drop(insert_chunk_query, insert_document_chunk)
                                    .await?;
                                timer.finish();
                                Event::add_to_local_parent("document_smart_chunks", || {
                                    [
                                        (
                                            Cow::Borrowed("chunk_length"),
                                            Cow::Owned(chunk.len().to_string()),
                                        ),
                                        (
                                            Cow::Borrowed("chunk_bytes"),
                                            Cow::Owned(chunk_bytes.to_string()),
                                        ),
                                    ]
                                });
                                Ok::<_, anyhow::Error>(())
                            };
                            future
                                .in_span(Span::enter_with_local_parent(format!(
                                    "{}::document_chunk_write",
                                    full_name!()
                                )))
                                .await?;
                        }

                        let index_vec = indexes.into_iter().collect_vec();
                        let mut index_chunks = smart_chunks(&index_vec);
                        for chunk in &mut index_chunks {
                            let chunk_bytes: usize =
                                chunk.iter().map(|item| item.approx_size()).sum();
                            let insert_chunk_query = insert_index_chunk(chunk.len());
                            let insert_overwrite_chunk_query =
                                insert_overwrite_index_chunk(chunk.len());
                            let insert_index_chunk = match conflict_strategy {
                                ConflictStrategy::Error => &insert_chunk_query,
                                ConflictStrategy::Overwrite => &insert_overwrite_chunk_query,
                            };
                            let mut insert_index_chunk_params = vec![];
                            for (ts, update) in chunk {
                                let update = update.clone();
                                index_params(&mut insert_index_chunk_params, *ts, update);
                            }
                            let future = async {
                                let timer =
                                    metrics::insert_index_chunk_timer(cluster_name.as_str());
                                tx.exec_drop(insert_index_chunk, insert_index_chunk_params)
                                    .await?;
                                timer.finish();
                                Event::add_to_local_parent("index_smart_chunks", || {
                                    [
                                        (
                                            Cow::Borrowed("chunk_length"),
                                            Cow::Owned(chunk.len().to_string()),
                                        ),
                                        (
                                            Cow::Borrowed("chunk_bytes"),
                                            Cow::Owned(chunk_bytes.to_string()),
                                        ),
                                    ]
                                });
                                Ok::<_, anyhow::Error>(())
                            };
                            future
                                .in_span(Span::enter_with_local_parent(format!(
                                    "{}::index_chunk_write",
                                    full_name!()
                                )))
                                .await?;
                        }
                    }
                    Ok(())
                }
                .boxed()
            })
            .await
    }

    async fn set_read_only(&self, read_only: bool) -> anyhow::Result<()> {
        self.lease
            .transact(move |tx| {
                async move {
                    let statement = if read_only {
                        SET_READ_ONLY
                    } else {
                        UNSET_READ_ONLY
                    };
                    tx.exec_drop(statement, vec![]).await?;
                    Ok(())
                }
                .boxed()
            })
            .await
    }

    async fn write_persistence_global(
        &self,
        key: PersistenceGlobalKey,
        value: JsonValue,
    ) -> anyhow::Result<()> {
        let timer = write_persistence_global_timer(self.read_pool.cluster_name());
        self.lease
            .transact(move |tx| {
                async move {
                    let stmt = WRITE_PERSISTENCE_GLOBAL;
                    let params = vec![String::from(key).into(), value.into()];
                    tx.exec_drop(stmt, params).await?;
                    Ok(())
                }
                .boxed()
            })
            .await?;
        timer.finish();
        Ok(())
    }

    async fn load_index_chunk(
        &self,
        cursor: Option<IndexEntry>,
        chunk_size: usize,
    ) -> anyhow::Result<Vec<IndexEntry>> {
        let mut client = self
            .read_pool
            .acquire("load_index_chunk", &self.db_name)
            .await?;
        let stmt = LOAD_INDEXES_PAGE;
        let mut params = MySqlReader::<RT>::_index_cursor_params(cursor.as_ref());
        params.push((chunk_size as i64).into());
        let row_stream = client.query_stream(stmt, params, chunk_size).await?;

        let parsed = row_stream.map(|row| parse_row(&row?));
        parsed.try_collect().await
    }

    async fn delete_index_entries(
        &self,
        expired_entries: Vec<IndexEntry>,
    ) -> anyhow::Result<usize> {
        self.lease
            .transact(move |tx| {
                async move {
                    let mut deleted_count = 0;
                    for chunk in smart_chunks(&expired_entries) {
                        let mut params = vec![];
                        for index_entry in chunk.iter() {
                            MySqlReader::<RT>::_index_delete_params(&mut params, index_entry);
                        }
                        deleted_count += tx
                            .exec_iter(delete_index_chunk(chunk.len()), params)
                            .await?;
                    }
                    Ok(deleted_count as usize)
                }
                .boxed()
            })
            .await
    }

    async fn delete(
        &self,
        documents: Vec<(Timestamp, InternalDocumentId)>,
    ) -> anyhow::Result<usize> {
        self.lease
            .transact(move |tx| {
                async move {
                    let mut deleted_count = 0;
                    for chunk in smart_chunks(&documents) {
                        let mut params = vec![];
                        for doc in chunk.iter() {
                            MySqlReader::<RT>::_document_delete_params(&mut params, doc);
                        }
                        deleted_count += tx
                            .exec_iter(delete_document_chunk(chunk.len()), params)
                            .await?;
                    }
                    Ok(deleted_count as usize)
                }
                .boxed()
            })
            .await
    }

    // TODO(ENG-8142): Remove this implementation once we fully move to
    // conductor. In conductor, we manually shutdown the pool.
    async fn shutdown(&self) -> anyhow::Result<()> {
        self.read_pool.clone().shutdown().await
    }
}

#[derive(Clone)]
pub struct MySqlReader<RT: Runtime> {
    read_pool: Arc<ConvexMySqlPool<RT>>,
    db_name: String,
    /// Set `db_should_be_leader` if this PostgresReader should be connected
    /// to the database leader. In particular, we protect against heterogenous
    /// connection pools where one connection is to the leader and another is to
    /// a follower.
    #[allow(unused)]
    db_should_be_leader: bool,
    version: PersistenceVersion,
}

impl<RT: Runtime> MySqlReader<RT> {
    fn initial_id_param(order: Order) -> Vec<u8> {
        match order {
            Order::Asc => InternalId::BEFORE_ALL_BYTES.to_vec(),
            Order::Desc => InternalId::AFTER_ALL_BYTES.to_vec(),
        }
    }

    fn row_to_document(
        &self,
        row: Row,
    ) -> anyhow::Result<(
        Timestamp,
        InternalDocumentId,
        Option<ResolvedDocument>,
        Option<Timestamp>,
    )> {
        let (ts, id, doc, prev_ts) = self.row_to_document_inner(row)?;
        Ok((ts, id, doc, prev_ts))
    }

    fn row_to_document_inner(
        &self,
        row: Row,
    ) -> anyhow::Result<(
        Timestamp,
        InternalDocumentId,
        Option<ResolvedDocument>,
        Option<Timestamp>,
    )> {
        let bytes: Vec<u8> = row.get(0).unwrap();
        let internal_id = InternalId::try_from(bytes)?;
        let ts: i64 = row.get(1).unwrap();
        let ts = Timestamp::try_from(ts)?;
        let table_b: Vec<u8> = row.get(2).unwrap();
        let json_value: serde_json::Value = row.get(3).unwrap();
        let deleted: bool = row.get(4).unwrap();
        let table = TabletId(table_b.try_into()?);
        let document_id = InternalDocumentId::new(table, internal_id);
        let document = if !deleted {
            let value: ConvexValue = json_value.try_into()?;
            Some(ResolvedDocument::from_database(table, value)?)
        } else {
            None
        };
        let prev_ts: Option<i64> = row.get(5).unwrap();
        let prev_ts = prev_ts.map(Timestamp::try_from).transpose()?;
        Ok((ts, document_id, document, prev_ts))
    }

    #[allow(clippy::needless_lifetimes)]
    #[try_stream(
        ok = DocumentLogEntry,
        error = anyhow::Error,
    )]
    async fn _load_documents(
        &self,
        range: TimestampRange,
        order: Order,
        page_size: u32,
        tablet_id: Option<TabletId>,
        retention_validator: Arc<dyn RetentionValidator>,
    ) {
        anyhow::ensure!(page_size > 0); // 0 size pages loop forever.
        let timer = metrics::load_documents_timer(self.read_pool.cluster_name());
        let mut client = self
            .read_pool
            .acquire("load_documents", &self.db_name)
            .await?;
        let mut num_returned = 0;
        let mut num_skipped_by_table = 0;
        let mut last_ts = match order {
            Order::Asc => Timestamp::MIN,
            Order::Desc => Timestamp::MAX,
        };
        let mut last_tablet_id_param = Self::initial_id_param(order);
        let mut last_id_param = Self::initial_id_param(order);
        loop {
            let mut rows_loaded = 0;

            let query = match order {
                Order::Asc => &LOAD_DOCS_BY_TS_PAGE_ASC,
                Order::Desc => &LOAD_DOCS_BY_TS_PAGE_DESC,
            };
            let params = vec![
                i64::from(range.min_timestamp_inclusive()).into(),
                i64::from(range.max_timestamp_exclusive()).into(),
                i64::from(last_ts).into(),
                i64::from(last_ts).into(),
                last_tablet_id_param.clone().into(),
                last_tablet_id_param.clone().into(),
                last_id_param.clone().into(),
                (page_size as i64).into(),
            ];
            let row_stream = client
                .query_stream(query, params, page_size as usize)
                .await?;

            retention_validator
                .validate_document_snapshot(range.min_timestamp_inclusive())
                .await?;

            futures::pin_mut!(row_stream);

            while let Some(row) = row_stream.try_next().await? {
                let (ts, document_id, document, prev_ts) = self.row_to_document(row)?;
                rows_loaded += 1;
                last_ts = ts;
                last_tablet_id_param = internal_id_param(document_id.table().0);
                last_id_param = internal_doc_id_param(document_id);
                num_returned += 1;
                if let Some(tablet_id) = tablet_id
                    && document_id.table() != tablet_id
                {
                    num_skipped_by_table += 1;
                    continue;
                } else {
                    yield DocumentLogEntry {
                        ts,
                        id: document_id,
                        value: document,
                        prev_ts,
                    }
                }
            }
            if rows_loaded < page_size {
                break;
            }
        }

        metrics::mysql_load_documents_skipped_wrong_table(
            num_skipped_by_table,
            self.read_pool.cluster_name(),
        );
        metrics::finish_load_documents_timer(timer, num_returned, self.read_pool.cluster_name());
    }

    #[allow(clippy::needless_lifetimes)]
    #[try_stream(ok = (IndexKeyBytes, Timestamp, ResolvedDocument), error = anyhow::Error)]
    async fn _index_scan(
        &self,
        index_id: IndexId,
        tablet_id: TabletId,
        read_timestamp: Timestamp,
        interval: Interval,
        order: Order,
        size_hint: usize,
        retention_validator: Arc<dyn RetentionValidator>,
    ) {
        let scan = self._index_scan_inner(
            index_id,
            read_timestamp,
            interval,
            order,
            size_hint,
            retention_validator,
        );
        pin_mut!(scan);
        while let Some((key, ts, value)) = scan.try_next().await? {
            let document = ResolvedDocument::from_database(tablet_id, value)?;
            yield (key, ts, document);
        }
    }

    #[allow(clippy::needless_lifetimes)]
    #[try_stream(ok = (IndexKeyBytes, Timestamp, ConvexValue), error = anyhow::Error)]
    async fn _index_scan_inner(
        &self,
        index_id: IndexId,
        read_timestamp: Timestamp,
        interval: Interval,
        order: Order,
        size_hint: usize,
        retention_validator: Arc<dyn RetentionValidator>,
    ) {
        let _timer = metrics::query_index_timer(self.read_pool.cluster_name());
        let (mut lower, mut upper) = to_sql_bounds(interval.clone());

        let mut stats = QueryIndexStats::new(self.read_pool.cluster_name());

        // We use the size_hint to determine the batch size. This means in the
        // common case we should do a single query. Exceptions are if the size_hint
        // is wrong or if we truncate it or if we observe too many deletes.
        let mut batch_size =
            size_hint.clamp(*MYSQL_MIN_QUERY_BATCH_SIZE, *MYSQL_MAX_QUERY_BATCH_SIZE);

        // We iterate results in (key_prefix, key_sha256) order while we actually
        // need them in (key_prefix, key_suffix order). key_suffix is not part of the
        // primary key so we do the sort here. If see any record with maximum length
        // prefix, we should buffer it until we reach a different prefix.
        let mut result_buffer: Vec<(IndexKeyBytes, Timestamp, ConvexValue)> = Vec::new();
        let mut has_more = true;
        while has_more {
            let page = {
                let mut to_yield = vec![];
                // Avoid holding connections across yield points, to limit lifetime
                // and improve fairness.
                let mut client = self.read_pool.acquire("index_scan", &self.db_name).await?;
                stats.sql_statements += 1;
                let (query, params) = index_query(
                    index_id,
                    read_timestamp,
                    lower.clone(),
                    upper.clone(),
                    order,
                    batch_size,
                );

                let prepare_timer =
                    metrics::query_index_sql_prepare_timer(self.read_pool.cluster_name());
                prepare_timer.finish();

                let execute_timer =
                    metrics::query_index_sql_execute_timer(self.read_pool.cluster_name());
                let row_stream = client.query_stream(query, params, batch_size).await?;
                execute_timer.finish();

                let retention_validate_timer =
                    metrics::retention_validate_timer(self.read_pool.cluster_name());
                retention_validator
                    .validate_snapshot(read_timestamp)
                    .await?;
                retention_validate_timer.finish();

                futures::pin_mut!(row_stream);

                let mut batch_rows = 0;
                while let Some(row) = row_stream.try_next().await? {
                    batch_rows += 1;
                    stats.rows_read += 1;

                    // Fetch
                    let internal_row = parse_row(&row)?;

                    // Yield buffered results if applicable.
                    if let Some((buffer_key, ..)) = result_buffer.first() {
                        if buffer_key[..MAX_INDEX_KEY_PREFIX_LEN] != internal_row.key_prefix {
                            // We have exhausted all results that share the same key prefix
                            // we can sort and yield the buffered results.
                            result_buffer.sort_by(|a, b| a.0.cmp(&b.0));
                            for (key, ts, doc) in order.apply(result_buffer.drain(..)) {
                                if interval.contains(&key) {
                                    stats.rows_returned += 1;
                                    to_yield.push((key, ts, doc));
                                } else {
                                    stats.rows_skipped_out_of_range += 1;
                                }
                            }
                        }
                    }

                    // Update the bounds for future queries.
                    let bound = Bound::Excluded(SqlKey {
                        prefix: internal_row.key_prefix.clone(),
                        sha256: internal_row.key_sha256.clone(),
                    });
                    match order {
                        Order::Asc => lower = bound,
                        Order::Desc => upper = bound,
                    }

                    // Filter if needed.
                    if internal_row.deleted {
                        stats.rows_skipped_deleted += 1;
                        continue;
                    }

                    // Construct key.
                    let mut key = internal_row.key_prefix;
                    if let Some(key_suffix) = internal_row.key_suffix {
                        key.extend(key_suffix);
                    };
                    let ts = internal_row.ts;

                    // Fetch the remaining columns and construct the document
                    let table_b: Option<Vec<u8>> = row.get(7).unwrap();
                    table_b.ok_or_else(|| {
                        anyhow::anyhow!("Dangling index reference for {:?} {:?}", key, ts)
                    })?;
                    let json_value: serde_json::Value = row.get(8).unwrap();
                    anyhow::ensure!(
                        json_value != serde_json::Value::Null,
                        "Index reference to deleted document {:?} {:?}",
                        key,
                        ts
                    );
                    let value: ConvexValue = json_value.try_into()?;

                    if key.len() < MAX_INDEX_KEY_PREFIX_LEN {
                        assert!(result_buffer.is_empty());
                        if interval.contains(&key) {
                            stats.rows_returned += 1;
                            to_yield.push((IndexKeyBytes(key), ts, value));
                        } else {
                            stats.rows_skipped_out_of_range += 1;
                        }
                    } else {
                        // There might be other records with the same key_prefix that
                        // are ordered before this result. Buffer it.
                        result_buffer.push((IndexKeyBytes(key), ts, value));
                        stats.max_rows_buffered =
                            cmp::max(result_buffer.len(), stats.max_rows_buffered);
                    }
                }

                if batch_rows < batch_size {
                    // Yield any remaining values.
                    result_buffer.sort_by(|a, b| a.0.cmp(&b.0));
                    for (key, ts, doc) in order.apply(result_buffer.drain(..)) {
                        if interval.contains(&key) {
                            stats.rows_returned += 1;
                            to_yield.push((key, ts, doc));
                        } else {
                            stats.rows_skipped_out_of_range += 1;
                        }
                    }
                    has_more = false;
                }

                to_yield
            };
            for document in page {
                yield document;
            }
            // Double the batch size every iteration until we max dynamic batch size. This
            // helps correct for tombstones, long prefixes and wrong client
            // size estimates.
            // TODO: Take size into consideration and increase the max dynamic batch size.
            if batch_size < *MYSQL_MAX_QUERY_DYNAMIC_BATCH_SIZE {
                batch_size = (batch_size * 2).min(*MYSQL_MAX_QUERY_DYNAMIC_BATCH_SIZE);
            }
        }
    }

    fn _index_cursor_params(cursor: Option<&IndexEntry>) -> Vec<mysql_async::Value> {
        let (last_id_param, last_key_prefix, last_sha256, last_ts): (
            Vec<u8>,
            Vec<u8>,
            Vec<u8>,
            u64,
        ) = match cursor {
            Some(cursor) => (
                cursor.index_id.into(),
                cursor.key_prefix.clone(),
                cursor.key_sha256.clone(),
                cursor.ts.into(),
            ),
            None => (Self::initial_id_param(Order::Asc), vec![], vec![], 0),
        };
        vec![
            last_id_param.clone().into(),
            last_id_param.into(),
            last_key_prefix.clone().into(),
            last_key_prefix.into(),
            last_sha256.clone().into(),
            last_sha256.into(),
            last_ts.into(),
        ]
    }

    fn _index_delete_params(query: &mut Vec<mysql_async::Value>, entry: &IndexEntry) {
        let last_id_param: Vec<u8> = entry.index_id.into();
        let last_key_prefix: Vec<u8> = entry.key_prefix.clone();
        let last_sha256: Vec<u8> = entry.key_sha256.clone();
        let last_ts: u64 = entry.ts.into();
        query.push(last_id_param.into());
        query.push(last_key_prefix.into());
        query.push(last_sha256.into());
        query.push(last_ts.into());
    }

    fn _document_delete_params(
        query: &mut Vec<mysql_async::Value>,
        (ts, internal_id): &(Timestamp, InternalDocumentId),
    ) {
        let tablet_id: Vec<u8> = internal_id.table().0.into();
        let id: Vec<u8> = internal_id.internal_id().to_vec();
        let ts: u64 = (*ts).into();
        query.push(tablet_id.into());
        query.push(id.into());
        query.push(ts.into());
    }
}

fn parse_row(row: &Row) -> anyhow::Result<IndexEntry> {
    let bytes: Vec<u8> = row.get(0).unwrap();
    let index_id = InternalId::try_from(bytes).context("index_id wrong size")?;

    let key_prefix: Vec<u8> = row.get(1).unwrap();
    let key_sha256: Vec<u8> = row.get(2).unwrap();
    let key_suffix: Option<Vec<u8>> = row.get(3).unwrap();
    let ts: i64 = row.get(4).unwrap();
    let ts = Timestamp::try_from(ts)?;
    let deleted: bool = row.get(5).unwrap();
    Ok(IndexEntry {
        index_id,
        key_prefix,
        key_suffix,
        key_sha256,
        ts,
        deleted,
    })
}

#[async_trait]
impl<RT: Runtime> PersistenceReader for MySqlReader<RT> {
    fn load_documents(
        &self,
        range: TimestampRange,
        order: Order,
        page_size: u32,
        retention_validator: Arc<dyn RetentionValidator>,
    ) -> DocumentStream<'_> {
        self._load_documents(range, order, page_size, None, retention_validator)
            .boxed()
    }

    fn load_documents_from_table(
        &self,
        tablet_id: TabletId,
        range: TimestampRange,
        order: Order,
        page_size: u32,
        retention_validator: Arc<dyn RetentionValidator>,
    ) -> DocumentStream<'_> {
        self._load_documents(
            range,
            order,
            page_size,
            Some(tablet_id),
            retention_validator,
        )
        .boxed()
    }

    async fn previous_revisions(
        &self,
        ids: BTreeSet<(InternalDocumentId, Timestamp)>,
        retention_validator: Arc<dyn RetentionValidator>,
    ) -> anyhow::Result<BTreeMap<(InternalDocumentId, Timestamp), DocumentLogEntry>> {
        let timer = metrics::prev_revisions_timer(self.read_pool.cluster_name());

        let mut client = self
            .read_pool
            .acquire("previous_revisions", &self.db_name)
            .await?;
        let ids: Vec<_> = ids.into_iter().collect();

        let mut result = BTreeMap::new();

        let mut results = vec![];

        for chunk in smart_chunks(&ids) {
            let mut params = vec![];
            for (id, ts) in chunk {
                params.push(i64::from(*ts).into());
                params.push(internal_id_param(id.table().0).into());
                params.push(internal_doc_id_param(*id).into());
                params.push(i64::from(*ts).into());
            }
            let result_stream = client
                .query_stream(prev_rev_chunk(chunk.len()), params, chunk.len())
                .await?;
            pin_mut!(result_stream);
            while let Some(result) = result_stream.try_next().await? {
                results.push(result);
            }
        }
        let mut min_ts = Timestamp::MAX;
        for row in results.into_iter() {
            let ts: i64 = row.get(6).unwrap();
            let ts = Timestamp::try_from(ts)?;
            let (prev_ts, id, maybe_doc, prev_prev_ts) = self.row_to_document(row)?;
            min_ts = cmp::min(ts, min_ts);
            anyhow::ensure!(result
                .insert(
                    (id, ts),
                    DocumentLogEntry {
                        ts: prev_ts,
                        id,
                        value: maybe_doc,
                        prev_ts: prev_prev_ts,
                    }
                )
                .is_none());
            log_prev_revisions_row_read(self.read_pool.cluster_name());
        }

        retention_validator
            .validate_document_snapshot(min_ts)
            .await?;
        timer.finish();
        Ok(result)
    }

    fn index_scan(
        &self,
        index_id: IndexId,
        tablet_id: TabletId,
        read_timestamp: Timestamp,
        range: &Interval,
        order: Order,
        size_hint: usize,
        retention_validator: Arc<dyn RetentionValidator>,
    ) -> IndexStream<'_> {
        self._index_scan(
            index_id,
            tablet_id,
            read_timestamp,
            range.clone(),
            order,
            size_hint,
            retention_validator,
        )
        .boxed()
    }

    async fn get_persistence_global(
        &self,
        key: PersistenceGlobalKey,
    ) -> anyhow::Result<Option<JsonValue>> {
        let mut client = self
            .read_pool
            .acquire("get_persistence_global", &self.db_name)
            .await?;
        let params = vec![String::from(key).into()];
        let row_stream = client
            .query_stream(GET_PERSISTENCE_GLOBAL, params, 1)
            .await?;
        futures::pin_mut!(row_stream);

        let row = row_stream.try_next().await?;
        let value = row.map(|r| -> anyhow::Result<JsonValue> {
            let json_value: serde_json::Value = r.get(0).unwrap();
            Ok(json_value)
        });
        value.transpose()
    }

    fn version(&self) -> PersistenceVersion {
        self.version
    }

    async fn table_size_stats(&self) -> anyhow::Result<Vec<PersistenceTableSize>> {
        let mut client = self
            .read_pool
            .acquire("table_size_stats", &self.db_name)
            .await?;
        let stats = client
            .query_stream(TABLE_SIZE_QUERY, vec![self.db_name.clone().into()], 5)
            .await?
            .map(|row| {
                let row = row?;
                anyhow::Ok(PersistenceTableSize {
                    table_name: row.get_opt(0).unwrap()?,
                    data_bytes: row.get_opt(1).unwrap()?,
                    index_bytes: row.get_opt(2).unwrap()?,
                    row_count: row.get_opt(3).unwrap()?,
                })
            })
            .try_collect()
            .await?;
        Ok(stats)
    }
}

/// A `Lease` is unique for an instance across all of the processes in the
/// system. Its purpose is to make it safe to have multiple processes running
/// for the same instance at once, since we cannot truly guarantee that it will
/// not happen (e.g. processes unreachable by coordinator but still active, or
/// late-delivered packets from an already dead process) and we want to
/// purposefully run multiple so that one can coordinate all writes and the
/// others can serve stale reads, and smoothly swap between them during
/// deployment and node failure.
///
/// The only thing a `Lease` can do is execute a transaction against the
/// database and atomically ensure that the lease was still held during the
/// transaction, and otherwise return a lease lost.
struct Lease<RT: Runtime> {
    pool: Arc<ConvexMySqlPool<RT>>,
    db_name: String,
    lease_ts: i64,
    lease_lost_shutdown: ShutdownSignal,
}

impl<RT: Runtime> Lease<RT> {
    /// Acquire a lease. Makes other lease-holders get `LeaseLostError` when
    /// they commit.
    async fn acquire(
        pool: Arc<ConvexMySqlPool<RT>>,
        db_name: String,
        lease_lost_shutdown: ShutdownSignal,
    ) -> anyhow::Result<Self> {
        let timer = metrics::lease_acquire_timer(pool.cluster_name());
        let mut client = pool.acquire("lease_acquire", &db_name).await?;
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("before 1970")
            .as_nanos() as i64;

        tracing::info!("attempting to acquire lease");
        let rows_modified = client
            .exec_iter(LEASE_ACQUIRE, vec![ts.into(), ts.into()])
            .await?;
        anyhow::ensure!(
            rows_modified == 1,
            "failed to acquire lease: Already acquired with higher timestamp"
        );
        tracing::info!("lease acquired with ts {}", ts);

        timer.finish();
        Ok(Self {
            db_name,
            pool,
            lease_ts: ts,
            lease_lost_shutdown,
        })
    }

    /// Execute the transaction function f atomically ensuring that the lease is
    /// still held, otherwise return lease lost.
    ///
    /// Once `transact` returns lease lost, no future transactions using
    /// it will succeed. Instead, a new `Lease` must be made with `acquire`,
    /// and any in-memory state then resynced because of any changes that
    /// might've been made to the database state while the lease was not
    /// held.
    #[minitrace::trace]
    async fn transact<F, T>(&self, f: F) -> anyhow::Result<T>
    where
        F: for<'b> FnOnce(
            &'b mut MySqlTransaction<'_>,
        ) -> Pin<Box<dyn Future<Output = anyhow::Result<T>> + Send + 'b>>,
    {
        let mut client = self.pool.acquire("transact", &self.db_name).await?;
        let mut tx = client.transaction(&self.db_name).await?;

        let timer = metrics::lease_precond_timer(self.pool.cluster_name());
        let rows: Option<Row> = tx
            .exec_first(LEASE_PRECOND, vec![mysql_async::Value::Int(self.lease_ts)])
            .in_span(Span::enter_with_local_parent(format!(
                "{}::lease_precondition",
                full_name!()
            )))
            .await?;
        if rows.is_none() {
            self.lease_lost_shutdown.signal(lease_lost_error());
            anyhow::bail!(lease_lost_error());
        }
        timer.finish();

        let result = f(&mut tx)
            .in_span(Span::enter_with_local_parent(format!(
                "{}::execute_function",
                full_name!()
            )))
            .await?;

        let timer = metrics::commit_timer(self.pool.cluster_name());
        tx.commit().await?;
        timer.finish();

        Ok(result)
    }
}

fn document_params(
    mut query: Vec<mysql_async::Value>,
    ts: Timestamp,
    id: InternalDocumentId,
    maybe_doc: Option<ResolvedDocument>,
    prev_ts: Option<Timestamp>,
) -> Vec<mysql_async::Value> {
    let (json_value, deleted) = match maybe_doc {
        Some(document) => (document.value().0.clone().into(), false),
        None => (serde_json::Value::Null, true),
    };

    query.push(internal_doc_id_param(id).into());
    query.push(i64::from(ts).into());
    query.push(internal_id_param(id.table().0).into());
    query.push(json_value.into());
    query.push(deleted.into());
    query.push(prev_ts.map(i64::from).into());
    query
}

fn internal_id_param(id: InternalId) -> Vec<u8> {
    id.into()
}

fn internal_doc_id_param(id: InternalDocumentId) -> Vec<u8> {
    internal_id_param(id.internal_id())
}
fn resolved_id_param(id: &ResolvedDocumentId) -> Vec<u8> {
    internal_id_param(id.internal_id())
}

fn index_params(query: &mut Vec<mysql_async::Value>, ts: Timestamp, update: DatabaseIndexUpdate) {
    let key: Vec<u8> = update.key.clone().into_bytes().0;
    let key_sha256 = Sha256::hash(&key);
    let key = SplitKey::new(key);

    let (deleted, tablet_id, doc_id) = match &update.value {
        DatabaseIndexValue::Deleted => (true, None, None),
        DatabaseIndexValue::NonClustered(doc_id) => (
            false,
            Some(internal_id_param(doc_id.tablet_id.0)),
            Some(resolved_id_param(doc_id)),
        ),
    };
    query.push(internal_id_param(update.index_id).into());
    query.push(i64::from(ts).into());
    query.push(key.prefix.into());
    query.push(
        match key.suffix {
            Some(key_suffix) => Some(key_suffix),
            None => None,
        }
        .into(),
    );
    query.push(key_sha256.to_vec().into());
    query.push(deleted.into());
    query.push(tablet_id.into());
    query.push(doc_id.into());
}

const GET_TABLE_COUNT: &str = r#"
    SELECT COUNT(1) FROM INFORMATION_SCHEMA.TABLES WHERE TABLE_SCHEMA = '@db_name';
"#;

// Expected table count after INIT_SQL is ran.
const EXPECTED_TABLE_COUNT: usize = 5;

// This runs (currently) every time a MySqlPersistence is created, so it
// needs to not only be idempotent but not to affect any already-resident data.
// IF NOT EXISTS and ON CONFLICT are helpful.
const INIT_SQL: &str = r#"
        CREATE TABLE IF NOT EXISTS @db_name.documents (
            id VARBINARY(32) NOT NULL,
            ts BIGINT NOT NULL,

            table_id VARBINARY(32) NOT NULL,

            json_value LONGBLOB NOT NULL,
            deleted BOOLEAN DEFAULT false,

            prev_ts BIGINT,

            PRIMARY KEY (ts, table_id, id),
            INDEX documents_by_table_and_id (table_id, id, ts)
        ) ROW_FORMAT=DYNAMIC;

        CREATE TABLE IF NOT EXISTS @db_name.indexes (
            /* ids should be serialized as bytes but we keep it compatible with documents */
            index_id VARBINARY(32) NOT NULL,
            ts BIGINT NOT NULL,

            /*
            MySQL maximum primary key length is 3072 bytes with DYNAMIC row format,
            which is why we split up the key. The first 2500 bytes are stored in key_prefix,
            and the remaining ones are stored in key suffix if applicable.
            NOTE: The key_prefix + key_suffix is store all values of IndexKey including
            the id.
            */
            key_prefix VARBINARY(2500) NOT NULL,
            key_suffix LONGBLOB NULL,

            /* key_sha256 of the full key, used in primary key to avoid duplicates in case
            of key_prefix collision. */
            key_sha256 BINARY(32) NOT NULL,

            deleted BOOLEAN,
            /* table_id and document_id should be populated iff deleted is false. */
            table_id VARBINARY(32) NULL,
            document_id VARBINARY(32) NULL,

            PRIMARY KEY (index_id, key_prefix, key_sha256, ts)
        ) ROW_FORMAT=DYNAMIC;
        CREATE TABLE IF NOT EXISTS @db_name.leases (
            id BIGINT NOT NULL,
            ts BIGINT NOT NULL,

            PRIMARY KEY (id)
        ) ROW_FORMAT=DYNAMIC;
        INSERT IGNORE INTO @db_name.leases (id, ts) VALUES (1, 0);
        CREATE TABLE IF NOT EXISTS @db_name.read_only (
            id BIGINT NOT NULL,

            PRIMARY KEY (id)
        ) ROW_FORMAT=DYNAMIC;
        CREATE TABLE IF NOT EXISTS @db_name.persistence_globals (
            `key` VARCHAR(255) NOT NULL,
            json_value LONGBLOB NOT NULL,

            PRIMARY KEY (`key`)
        ) ROW_FORMAT=DYNAMIC;"#;
/// Load a page of documents, where timestamps are bounded by [$1, $2),
/// and ($3, $4, $5) is the (ts, table_id, id) from the last document read.
const LOAD_DOCS_BY_TS_PAGE_ASC: &str = r#"SELECT id, ts, table_id, json_value, deleted, prev_ts
    FROM @db_name.documents
    FORCE INDEX FOR ORDER BY (PRIMARY)
    WHERE ts >= ?
    AND ts < ?
    AND (ts > ? OR (ts = ? AND (table_id > ? OR (table_id = ? AND id > ?))))
    ORDER BY ts ASC, table_id ASC, id ASC
    LIMIT ?
"#;

const LOAD_DOCS_BY_TS_PAGE_DESC: &str = r#"SELECT id, ts, table_id, json_value, deleted, prev_ts
    FROM @db_name.documents
    FORCE INDEX FOR ORDER BY (PRIMARY)
    WHERE ts >= ?
    AND ts < ?
    AND (ts < ? OR (ts = ? AND (table_id < ? OR (table_id = ? AND id < ?))))
    ORDER BY ts DESC, table_id DESC, id DESC
    LIMIT ?
"#;

static INSERT_DOCUMENT_CHUNK_QUERIES: LazyLock<HashMap<usize, String>> = LazyLock::new(|| {
    smart_chunk_sizes()
        .map(|chunk_size| {
            let values = (1..=chunk_size)
                .map(|_| format!("(?, ?, ?, ?, ?, ?)"))
                .join(", ");
            let query = format!(
                r#"INSERT INTO @db_name.documents
    (id, ts, table_id, json_value, deleted, prev_ts)
    VALUES {values}"#
            );
            (chunk_size, query)
        })
        .collect()
});

fn insert_document_chunk(chunk_size: usize) -> &'static str {
    INSERT_DOCUMENT_CHUNK_QUERIES.get(&chunk_size).unwrap()
}

static INSERT_OVERWRITE_DOCUMENT_CHUNK_QUERIES: LazyLock<HashMap<usize, String>> =
    LazyLock::new(|| {
        smart_chunk_sizes()
            .map(|chunk_size| {
                let values = (1..=chunk_size)
                    .map(|_| format!("(?, ?, ?, ?, ?, ?)"))
                    .join(", ");
                let query = format!(
                    r#"REPLACE INTO @db_name.documents
    (id, ts, table_id, json_value, deleted, prev_ts)
    VALUES {values}"#
                );
                (chunk_size, query)
            })
            .collect()
    });

fn insert_overwrite_document_chunk(chunk_size: usize) -> &'static str {
    INSERT_OVERWRITE_DOCUMENT_CHUNK_QUERIES
        .get(&chunk_size)
        .unwrap()
}

const LOAD_INDEXES_PAGE: &str = r#"
SELECT
    index_id, key_prefix, key_sha256, key_suffix, ts, deleted
    FROM @db_name.indexes
    FORCE INDEX FOR ORDER BY (PRIMARY)
    WHERE index_id > ? OR (index_id = ? AND
        (key_prefix > ? OR (key_prefix = ? AND
        (key_sha256 > ? OR (key_sha256 = ? AND
        ts > ?)))))
    ORDER BY index_id ASC, key_prefix ASC, key_sha256 ASC, ts ASC
    LIMIT ?
"#;

static INSERT_INDEX_CHUNK_QUERIES: LazyLock<HashMap<usize, String>> = LazyLock::new(|| {
    smart_chunk_sizes()
        .map(|chunk_size| {
            let values = (1..=chunk_size)
                .map(|_| format!("(?, ?, ?, ?, ?, ?, ?, ?)"))
                .join(", ");
            let query = format!(
                r#"INSERT INTO @db_name.indexes
            (index_id, ts, key_prefix, key_suffix, key_sha256, deleted, table_id, document_id)
            VALUES {values}"#
            );
            (chunk_size, query)
        })
        .collect()
});

// Note that on conflict, there's no need to update any of the columns that are
// part of the primary key, nor `key_suffix` as `key_sha256` is derived from the
// prefix and suffix.
// Only the fields that could have actually changed need to be updated.
fn insert_index_chunk(chunk_size: usize) -> &'static str {
    INSERT_INDEX_CHUNK_QUERIES.get(&chunk_size).unwrap()
}

static INSERT_OVERWRITE_INDEX_CHUNK_QUERIES: LazyLock<HashMap<usize, String>> =
    LazyLock::new(|| {
        smart_chunk_sizes()
            .map(|chunk_size| {
                let values = (1..=chunk_size)
                    .map(|_| format!("(?, ?, ?, ?, ?, ?, ?, ?)"))
                    .join(", ");
                let query = format!(
                    r#"INSERT INTO @db_name.indexes
            (index_id, ts, key_prefix, key_suffix, key_sha256, deleted, table_id, document_id)
            VALUES
                {values}
                ON DUPLICATE KEY UPDATE
                deleted = VALUES(deleted),
                table_id = VALUES(table_id),
                document_id = VALUES(document_id)
        "#
                );
                (chunk_size, query)
            })
            .collect()
    });

fn insert_overwrite_index_chunk(chunk_size: usize) -> &'static str {
    INSERT_OVERWRITE_INDEX_CHUNK_QUERIES
        .get(&chunk_size)
        .unwrap()
}

static DELETE_INDEX_CHUNK_QUERIES: LazyLock<HashMap<usize, String>> = LazyLock::new(|| {
    smart_chunk_sizes()
        .map(|chunk_size| {
            let where_clauses = (1..=chunk_size)
                .map(|_| "(index_id = ? AND key_prefix = ? AND key_sha256 = ? AND ts <= ?)")
                .join(" OR ");
            (
                chunk_size,
                format!("DELETE FROM @db_name.indexes WHERE {where_clauses}"),
            )
        })
        .collect()
});

fn delete_index_chunk(chunk_size: usize) -> &'static str {
    DELETE_INDEX_CHUNK_QUERIES.get(&chunk_size).unwrap()
}

static DELETE_DOCUMENT_CHUNK_QUERIES: LazyLock<HashMap<usize, String>> = LazyLock::new(|| {
    smart_chunk_sizes()
        .map(|chunk_size| {
            let where_clauses = (1..=chunk_size)
                .map(|_| "(table_id = ? AND id = ? AND ts <= ?)")
                .join(" OR ");
            (
                chunk_size,
                format!("DELETE FROM @db_name.documents WHERE {where_clauses}"),
            )
        })
        .collect()
});

fn delete_document_chunk(chunk_size: usize) -> &'static str {
    DELETE_DOCUMENT_CHUNK_QUERIES.get(&chunk_size).unwrap()
}

const WRITE_PERSISTENCE_GLOBAL: &str = r#"INSERT INTO @db_name.persistence_globals
    (`key`, json_value)
    VALUES (?, ?)
    ON DUPLICATE KEY UPDATE
    json_value = VALUES(json_value)
"#;

const GET_PERSISTENCE_GLOBAL: &str =
    "SELECT json_value FROM @db_name.persistence_globals FORCE INDEX (PRIMARY) WHERE `key` = ?";

const MAX_INSERT_SIZE: usize = 16384;

// Gross: after initialization, the first thing database does is insert metadata
// documents.
const CHECK_NEWLY_CREATED: &str = "SELECT 1 FROM @db_name.documents LIMIT 1";

// This table has no rows (not read_only) or 1 row (read_only), so if this query
// returns any results, the persistence is read_only.
const CHECK_IS_READ_ONLY: &str = "SELECT 1 FROM @db_name.read_only LIMIT 1";
const SET_READ_ONLY: &str = "INSERT INTO @db_name.read_only (id) VALUES (1)";
const UNSET_READ_ONLY: &str = "DELETE FROM @db_name.read_only WHERE id = 1";

// If this query returns a result, the lease is still valid and will remain so
// until the end of the transaction.
const LEASE_PRECOND: &str =
    "SELECT 1 FROM @db_name.leases FORCE INDEX (PRIMARY) WHERE id=1 AND ts=? FOR SHARE";

// Acquire the lease unless acquire by someone with a higher timestamp.
const LEASE_ACQUIRE: &str = "UPDATE @db_name.leases SET ts=? WHERE id=1 AND ts<?";

#[derive(PartialEq, Eq, Hash, Copy, Clone)]
enum BoundType {
    Unbounded,
    Included,
    Excluded,
}

// Pre-build queries with various parameters.
static INDEX_QUERIES: LazyLock<HashMap<(BoundType, BoundType, Order), String>> = LazyLock::new(
    || {
        let mut queries = HashMap::new();
        // Tricks that convince MySQL to choose good query plans:
        // 1. All queries are ordered by a prefix of columns in the primary key. If you
        //    say `WHERE col1 = 'a' ORDER BY col2 ASC` it might not use the index, but
        //    `WHERE col1 = 'a' ORDER BY col1 ASC, col2 ASC` which is completely
        //    equivalent, does use the index.
        // 2. LEFT JOIN and FORCE INDEX FOR JOIN makes the join use the index for
        //    lookups. Despite having all index columns with equality checks, MySQL will
        //    do a hash join if you do an INNER JOIN or a plain FORCE INDEX.
        // 3. Tuple comparisons `(key_prefix, key_sha256) >= (?, ?)` are required for
        //    Postgres to choose the correct query plan, but MySQL requires the other
        //    format `(key_prefix > ? OR (key_prefix = ? AND key_sha256 >= ?))`.

        let bounds = [
            BoundType::Unbounded,
            BoundType::Included,
            BoundType::Excluded,
        ];
        let orders = [Order::Asc, Order::Desc];

        // Note, we always paginate using (key_prefix, key_sha256), which doesn't
        // necessary give us the order we need for long keys that have
        // key_suffix.
        for (lower, upper, order) in iproduct!(bounds.iter(), bounds.iter(), orders.iter()) {
            // Construct the where clause imperatively.
            let mut where_clause = String::new();
            write!(where_clause, "index_id = ? AND ts <= ?").unwrap();
            // Note the following clauses could be written as
            // (key_prefix, key_sha256) {comparator} (?, ?)
            match lower {
                BoundType::Unbounded => {},
                BoundType::Included => {
                    write!(
                        where_clause,
                        " AND (key_prefix > ? OR (key_prefix = ? AND key_sha256 >= ?))",
                    )
                    .unwrap();
                },
                BoundType::Excluded => {
                    write!(
                        where_clause,
                        " AND (key_prefix > ? OR (key_prefix = ? AND key_sha256 > ?))"
                    )
                    .unwrap();
                },
            };
            match upper {
                BoundType::Unbounded => {},
                BoundType::Included => {
                    write!(
                        where_clause,
                        " AND (key_prefix < ? OR (key_prefix = ? AND key_sha256 <= ?))"
                    )
                    .unwrap();
                },
                BoundType::Excluded => {
                    write!(
                        where_clause,
                        " AND (key_prefix < ? OR (key_prefix = ? AND key_sha256 < ?))"
                    )
                    .unwrap();
                },
            };
            let order_str = match order {
                Order::Asc => "ASC",
                Order::Desc => "DESC",
            };
            let query = format!(
                r#"
SELECT I2.index_id, I2.key_prefix, I2.key_sha256, I2.key_suffix, I2.ts, I2.deleted, I2.document_id, D.table_id, D.json_value FROM
(
    SELECT
        I1.index_id, I1.key_prefix, I1.key_sha256, I1.key_suffix, I1.ts,
        I1.deleted, I1.table_id, I1.document_id
    FROM
    (
        SELECT index_id, key_prefix, key_sha256, MAX(ts) as ts_at_snapshot FROM @db_name.indexes
        FORCE INDEX FOR GROUP BY (PRIMARY)
        WHERE {where_clause}
        GROUP BY index_id, key_prefix, key_sha256
        ORDER BY index_id {order_str}, key_prefix {order_str}, key_sha256 {order_str}
        LIMIT ?
    ) snapshot
    LEFT JOIN @db_name.indexes I1 FORCE INDEX FOR JOIN (PRIMARY)
    ON
    (I1.index_id, I1.key_prefix, I1.key_sha256, I1.ts) = (snapshot.index_id, snapshot.key_prefix, snapshot.key_sha256, snapshot.ts_at_snapshot)
) I2
LEFT JOIN @db_name.documents D FORCE INDEX FOR JOIN (PRIMARY)
ON
D.ts = I2.ts AND D.table_id = I2.table_id AND D.id = I2.document_id
"#
            );
            queries.insert((*lower, *upper, *order), query);
        }

        queries
    },
);

static PREV_REV_CHUNK_QUERIES: LazyLock<HashMap<usize, String>> = LazyLock::new(|| {
    smart_chunk_sizes()
        .map(|chunk_size| {
            let select = r#"
SELECT id, ts, table_id, json_value, deleted, prev_ts, ? as query_ts
FROM @db_name.documents FORCE INDEX FOR ORDER BY (documents_by_table_and_id)
WHERE table_id = ? AND id = ? and ts < ?
ORDER BY table_id DESC, id DESC, ts DESC LIMIT 1
"#;
            let queries = (1..=chunk_size)
                .map(|i| format!("q{i} AS ({select})"))
                .join(", ");
            let union_all = (1..=chunk_size)
                .map(|i| {
                    format!(
                        "(SELECT id, ts, table_id, json_value, deleted, prev_ts, query_ts FROM \
                         q{i})"
                    )
                })
                .join(" UNION ALL ");
            (chunk_size, format!("WITH {queries} {union_all}"))
        })
        .collect()
});

fn prev_rev_chunk(chunk_size: usize) -> &'static str {
    PREV_REV_CHUNK_QUERIES.get(&chunk_size).unwrap()
}

const TABLE_SIZE_QUERY: &str = "
SELECT table_name, data_length, index_length, table_rows
FROM information_schema.tables
WHERE table_schema = ?
";

const MIN_SHA256: [u8; 32] = [0; 32];
const MAX_SHA256: [u8; 32] = [255; 32];

// The key we use to paginate in SQL, note that we can't use key_prefix since
// it is not part of the primary key. We use key_sha256 instead.
#[derive(Clone)]
struct SqlKey {
    prefix: Vec<u8>,
    sha256: Vec<u8>,
}

impl SqlKey {
    // Returns the maximum possible
    fn min_with_same_prefix(key: Vec<u8>) -> Self {
        let key = SplitKey::new(key);
        Self {
            prefix: key.prefix,
            sha256: MIN_SHA256.to_vec(),
        }
    }

    fn max_with_same_prefix(key: Vec<u8>) -> Self {
        let key = SplitKey::new(key);
        Self {
            prefix: key.prefix,
            sha256: MAX_SHA256.to_vec(),
        }
    }
}

// Translates a range to a SqlKey bounds we can use to get records in that
// range. Note that because the SqlKey does not sort the same way as IndexKey
// for very long keys, the returned range might contain extra keys that needs to
// be filtered application side.
fn to_sql_bounds(interval: Interval) -> (Bound<SqlKey>, Bound<SqlKey>) {
    let lower = match interval.start {
        StartIncluded(key) => {
            // This can potentially include more results than needed.
            Bound::Included(SqlKey::min_with_same_prefix(key.into()))
        },
    };
    let upper = match interval.end {
        End::Excluded(key) => {
            if key.len() < MAX_INDEX_KEY_PREFIX_LEN {
                Bound::Excluded(SqlKey::min_with_same_prefix(key.into()))
            } else {
                // We can't exclude the bound without potentially excluding other
                // keys that fall within the range.
                Bound::Included(SqlKey::max_with_same_prefix(key.into()))
            }
        },
        End::Unbounded => Bound::Unbounded,
    };
    (lower, upper)
}

fn index_query(
    index_id: IndexId,
    read_timestamp: Timestamp,
    lower: Bound<SqlKey>,
    upper: Bound<SqlKey>,
    order: Order,
    batch_size: usize,
) -> (&'static str, Vec<mysql_async::Value>) {
    let mut params = vec![];

    let mut map_bound = |b: Bound<SqlKey>| -> BoundType {
        match b {
            Bound::Unbounded => BoundType::Unbounded,
            Bound::Excluded(sql_key) => {
                params.push(sql_key.prefix.clone());
                params.push(sql_key.prefix);
                params.push(sql_key.sha256);
                BoundType::Excluded
            },
            Bound::Included(sql_key) => {
                params.push(sql_key.prefix.clone());
                params.push(sql_key.prefix);
                params.push(sql_key.sha256);
                BoundType::Included
            },
        }
    };

    let lt = map_bound(lower);
    let ut = map_bound(upper);

    let query = INDEX_QUERIES.get(&(lt, ut, order)).unwrap();
    // Substitutions are {where_clause}, ts, {where_clause}, ts, limit.
    let mut all_params = vec![];
    all_params.push(internal_id_param(index_id).into());
    all_params.push(i64::from(read_timestamp).into());
    for param in params {
        all_params.push(param.into());
    }
    all_params.push((batch_size as i64).into());
    (query, all_params)
}

#[cfg(any(test, feature = "testing"))]
pub mod itest {
    use std::path::Path;

    use mysql_async::{
        prelude::Queryable,
        Conn,
        Params,
    };
    use rand::Rng;
    use url::Url;

    // Returns a url to connect to the test cluster. The URL includes username and
    // password but no dbname.
    pub fn cluster_opts() -> String {
        let mysql_host = if Path::new("/convex.ro").exists() {
            // itest
            "mysql"
        } else {
            // local
            "localhost"
        };
        format!("mysql://root:@{mysql_host}:3306")
    }

    pub struct MySqlOpts {
        pub db_name: String,
        pub url: Url,
    }

    /// Returns connection options for a guaranteed-fresh Postgres database.
    pub async fn new_db_opts() -> anyhow::Result<MySqlOpts> {
        let cluster_url = cluster_opts();
        let id: [u8; 16] = rand::thread_rng().gen();
        let db_name = "test_db_".to_string() + &hex::encode(&id[..]);

        // Connect using db `mysql`, create a fresh DB, and then return the connection
        // options for that one.
        let mut conn = Conn::from_url(format!("{cluster_url}/mysql")).await?;
        let query = "CREATE DATABASE ".to_string() + &db_name;
        conn.exec_drop(query.as_str(), Params::Empty).await?;

        println!("DBNAME @{db_name}");
        Ok(MySqlOpts {
            // We use the cluster URL to connect to connect to persistence and
            // then pass the db_name in the query themselves.
            url: cluster_url.parse()?,
            db_name,
        })
    }
}

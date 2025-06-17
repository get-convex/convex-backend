#![feature(coroutines)]
#![feature(proc_macro_hygiene)]
#![feature(stmt_expr_attributes)]
#![feature(let_chains)]
mod connection;
mod metrics;
#[cfg(test)]
mod tests;

use std::{
    cmp,
    collections::{
        BTreeMap,
        BTreeSet,
        HashMap,
    },
    env,
    error::Error,
    fmt::Write,
    fs,
    future::Future,
    ops::Bound,
    path::Path,
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
use bytes::BytesMut;
use cmd_util::env::env_config;
use common::{
    document::{
        InternalId,
        ResolvedDocument,
    },
    errors::LeaseLostError,
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
    persistence::{
        ConflictStrategy,
        DocumentLogEntry,
        DocumentPrevTsQuery,
        DocumentStream,
        IndexStream,
        LatestDocument,
        Persistence,
        PersistenceGlobalKey,
        PersistenceReader,
        PersistenceTableSize,
        RetentionValidator,
        TimestampRange,
    },
    query::Order,
    sha256::Sha256,
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
use fastrace::func_path;
use futures::{
    future::{
        self,
        Either,
    },
    stream::{
        self,
        FuturesUnordered,
        StreamExt,
        TryStreamExt,
    },
    try_join,
    FutureExt,
};
use futures_async_stream::try_stream;
use itertools::{
    iproduct,
    Itertools,
};
use rustls::{
    ClientConfig,
    RootCertStore,
};
use rustls_pki_types::{
    pem::PemObject,
    CertificateDer,
};
use serde::Deserialize as _;
use serde_json::Value as JsonValue;
use tokio::sync::mpsc::{
    self,
};
use tokio_postgres::{
    config::TargetSessionAttrs,
    types::{
        to_sql_checked,
        IsNull,
        ToSql,
        Type,
    },
    Row,
};
use tokio_postgres_rustls::MakeRustlsConnect;
use tokio_util::task::AbortOnDropHandle;

pub use crate::connection::ConvexPgPool;
use crate::{
    connection::{
        PostgresConnection,
        PostgresTransaction,
        SchemaName,
    },
    metrics::QueryIndexStats,
};

pub struct PostgresPersistence {
    newly_created: AtomicBool,
    lease: Lease,

    // Used by the reader.
    read_pool: Arc<ConvexPgPool>,
    version: PersistenceVersion,
    schema: SchemaName,
}

#[derive(thiserror::Error, Debug)]
pub enum ConnectError {
    #[error("persistence is read-only, data migration in progress")]
    ReadOnly,
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

#[derive(Clone)]
pub struct PostgresOptions {
    pub allow_read_only: bool,
    pub version: PersistenceVersion,
    /// If `None` uses the default schema (usually `public`)
    pub schema: Option<String>,
}

pub struct PostgresReaderOptions {
    pub version: PersistenceVersion,
    /// If `None` uses the default schema (usually `public`)
    pub schema: Option<String>,
}

async fn get_current_schema(pool: &ConvexPgPool) -> anyhow::Result<String> {
    let client = pool
        .get_connection(
            "get_current_schema",
            // This is invalid but we don't use `@db_name`
            const { &SchemaName::EMPTY },
        )
        .await?;
    let row = client
        .query_opt("SELECT current_schema()", &[])
        .await?
        .context("current_schema() returned nothing?")?;
    row.try_get::<_, Option<String>>(0)?
        .context("PostgresOptions::schema not provided and database has no current_schema()?")
}

impl PostgresPersistence {
    pub async fn new(url: &str, options: PostgresOptions) -> Result<Self, ConnectError> {
        let mut config: tokio_postgres::Config =
            url.parse().context("invalid postgres connection url")?;
        config.target_session_attrs(TargetSessionAttrs::ReadWrite);
        let pool = Self::create_pool(config)?;
        Self::with_pool(pool, options).await
    }

    pub async fn with_pool(
        pool: Arc<ConvexPgPool>,
        options: PostgresOptions,
    ) -> Result<Self, ConnectError> {
        if !pool.is_leader_only() {
            return Err(anyhow::anyhow!(
                "PostgresPersistence must be configured with target_session_attrs=read-write"
            )
            .into());
        }

        let schema = if let Some(s) = &options.schema {
            SchemaName::new(s)?
        } else {
            SchemaName::new(&get_current_schema(&pool).await?)?
        };
        let newly_created = {
            let client = pool.get_connection("init_sql", &schema).await?;
            // Only create a new schema if one was specified and it's not
            // already present. This avoids requiring extra permissions to run
            // `CREATE SCHEMA IF NOT EXISTS` if it's already been created.
            if let Some(raw_schema) = &options.schema
                && client
                    .query_opt(CHECK_SCHEMA_SQL, &[&raw_schema])
                    .await?
                    .is_none()
            {
                client
                    .batch_execute(CREATE_SCHEMA_SQL)
                    .await
                    .map_err(anyhow::Error::from)?;
            }
            client
                .batch_execute(INIT_SQL)
                .await
                .map_err(anyhow::Error::from)?;
            if !options.allow_read_only && Self::is_read_only(&client).await? {
                return Err(ConnectError::ReadOnly);
            }
            Self::check_newly_created(&client).await?
        };

        let lease = Lease::acquire(pool.clone(), &schema).await?;
        Ok(Self {
            newly_created: newly_created.into(),
            lease,
            read_pool: pool,
            version: options.version,
            schema,
        })
    }

    pub async fn new_reader(
        pool: Arc<ConvexPgPool>,
        options: PostgresReaderOptions,
    ) -> anyhow::Result<PostgresReader> {
        let schema = match options.schema {
            Some(s) => s,
            None => get_current_schema(&pool).await?,
        };
        Ok(PostgresReader {
            read_pool: pool,
            version: options.version,
            schema: SchemaName::new(&schema)?,
        })
    }

    async fn is_read_only(client: &PostgresConnection<'_>) -> anyhow::Result<bool> {
        Ok(client.query_opt(CHECK_IS_READ_ONLY, &[]).await?.is_some())
    }

    pub fn create_pool(pg_config: tokio_postgres::Config) -> anyhow::Result<Arc<ConvexPgPool>> {
        let mut roots = RootCertStore::empty();
        let native_certs = rustls_native_certs::load_native_certs();
        anyhow::ensure!(
            native_certs.errors.is_empty(),
            "failed to load native certs: {:?}",
            native_certs.errors
        );
        for cert in native_certs.certs {
            roots.add(cert)?;
        }
        if let Some(ca_file_path) = env::var_os("PG_CA_FILE")
            && !ca_file_path.is_empty()
        {
            let ca_file_path = Path::new(&ca_file_path);
            let ca_file_content = fs::read(ca_file_path)
                .with_context(|| format!("Failed to read CA file: {}", ca_file_path.display()))?;
            for ca_cert in CertificateDer::pem_slice_iter(&ca_file_content) {
                roots.add(ca_cert.with_context(|| {
                    format!("Failed to parse CA file as PEM: {}", ca_file_path.display())
                })?)?;
            }
        }
        let config = ClientConfig::builder()
            .with_root_certificates(roots)
            .with_no_client_auth();
        let connector = MakeRustlsConnect::new(config);

        Ok(ConvexPgPool::new(pg_config, connector))
    }

    async fn check_newly_created(client: &PostgresConnection<'_>) -> anyhow::Result<bool> {
        Ok(client.query_opt(CHECK_NEWLY_CREATED, &[]).await?.is_none())
    }
}

#[async_trait]
impl Persistence for PostgresPersistence {
    fn is_fresh(&self) -> bool {
        self.newly_created.load(SeqCst)
    }

    fn reader(&self) -> Arc<dyn PersistenceReader> {
        Arc::new(PostgresReader {
            read_pool: self.read_pool.clone(),
            version: self.version,
            schema: self.schema.clone(),
        })
    }

    async fn write(
        &self,
        documents: Vec<DocumentLogEntry>,
        indexes: BTreeSet<(Timestamp, DatabaseIndexUpdate)>,
        conflict_strategy: ConflictStrategy,
    ) -> anyhow::Result<()> {
        anyhow::ensure!(documents.len() <= MAX_INSERT_SIZE);
        anyhow::ensure!(documents.iter().all(|update| {
            match &update.value {
                Some(doc) => update.id == doc.id_with_table_id(),
                None => true,
            }
        }));

        // True, the below might end up failing and not changing anything.
        self.newly_created.store(false, SeqCst);
        self.lease
            .transact(move |tx| {
                async move {
                    let (insert_document, insert_document_chunk, insert_index, insert_index_chunk) =
                        try_join!(
                            match conflict_strategy {
                                ConflictStrategy::Error => tx.prepare_cached(INSERT_DOCUMENT),
                                ConflictStrategy::Overwrite =>
                                    tx.prepare_cached(INSERT_OVERWRITE_DOCUMENT),
                            },
                            match conflict_strategy {
                                ConflictStrategy::Error => tx.prepare_cached(INSERT_DOCUMENT_CHUNK),
                                ConflictStrategy::Overwrite =>
                                    tx.prepare_cached(INSERT_OVERWRITE_DOCUMENT_CHUNK),
                            },
                            match conflict_strategy {
                                ConflictStrategy::Error => tx.prepare_cached(INSERT_INDEX),
                                ConflictStrategy::Overwrite =>
                                    tx.prepare_cached(INSERT_OVERWRITE_INDEX),
                            },
                            match conflict_strategy {
                                ConflictStrategy::Error => tx.prepare_cached(INSERT_INDEX_CHUNK),
                                ConflictStrategy::Overwrite =>
                                    tx.prepare_cached(INSERT_OVERWRITE_INDEX_CHUNK),
                            },
                        )?;

                    // Since the documents and indexes already in memory, blast it into the
                    // Postgres connection as fast as we can with
                    // unbounded query pipelining. If we hadn't fully
                    // "hydrated" the inputs to this part of the system already, we
                    // could use a bounded pipeline here to allow backpressure from Postgres
                    // (or any intermediate component like TCP, tokio, etc.) to flow up to
                    // our inputs. But in this case, we just want to get `documents` done as
                    // quickly as possible.
                    {
                        // Use a `FuturesUnordered` to fork off concurrent work.
                        let mut futures = FuturesUnordered::new();

                        // First, process all of the full document chunks, forking off
                        // insertions to our `FuturesUnordered` set
                        // as we encounter them.
                        let mut document_chunks = documents.chunks_exact(CHUNK_SIZE);
                        for chunk in &mut document_chunks {
                            let mut params = Vec::with_capacity(chunk.len() * NUM_DOCUMENT_PARAMS);
                            for update in chunk {
                                params.extend(document_params(
                                    update.ts,
                                    update.id,
                                    &update.value,
                                    update.prev_ts,
                                )?);
                            }
                            let future = async {
                                let timer = metrics::insert_document_chunk_timer();
                                tx.execute_raw(&insert_document_chunk, params).await?;
                                timer.finish();
                                Ok::<_, anyhow::Error>(())
                            };
                            futures.push(Either::Left(Either::Left(future)));
                        }

                        // After we've inserted all the full document chunks, drain the
                        // remainder.
                        for update in document_chunks.remainder() {
                            let params = document_params(
                                update.ts,
                                update.id,
                                &update.value,
                                update.prev_ts,
                            )?;
                            let future = async {
                                let timer = metrics::insert_one_document_timer();
                                tx.execute_raw(&insert_document, params).await?;
                                timer.finish();
                                Ok::<_, anyhow::Error>(())
                            };
                            futures.push(Either::Left(Either::Right(future)));
                        }

                        let index_vec = indexes.into_iter().collect_vec();
                        let mut index_chunks = index_vec.chunks_exact(CHUNK_SIZE);
                        for chunk in &mut index_chunks {
                            let mut params = Vec::with_capacity(chunk.len() * NUM_INDEX_PARAMS);
                            for (ts, update) in chunk {
                                params.extend(index_params(&(*ts, update.clone())));
                            }
                            let future = async {
                                let timer = metrics::insert_index_chunk_timer();
                                tx.execute_raw(&insert_index_chunk, params).await?;
                                timer.finish();
                                Ok::<_, anyhow::Error>(())
                            };
                            futures.push(Either::Right(Either::Left(future)));
                        }

                        // After we've inserted all the full index chunks, drain the remainder.
                        for (ts, update) in index_chunks.remainder() {
                            let params = index_params(&(*ts, update.clone()));
                            let future = async {
                                let timer = metrics::insert_one_index_timer();
                                tx.execute_raw(&insert_index, params).await?;
                                timer.finish();
                                Ok::<_, anyhow::Error>(())
                            };
                            futures.push(Either::Right(Either::Right(future)));
                        }

                        // Wait on all of the futures in our `FuturesUnordered` to finish.
                        while let Some(result) = futures.next().await {
                            result?;
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
                    tx.execute_str(statement, &[]).await?;
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
        self.lease
            .transact(move |tx| {
                async move {
                    let stmt = tx.prepare_cached(WRITE_PERSISTENCE_GLOBAL).await?;
                    let params = [
                        Param::PersistenceGlobalKey(key),
                        Param::JsonValue(value.to_string()),
                    ];
                    tx.execute_raw(&stmt, params).await?;
                    Ok(())
                }
                .boxed()
            })
            .await?;
        Ok(())
    }

    async fn load_index_chunk(
        &self,
        cursor: Option<IndexEntry>,
        chunk_size: usize,
    ) -> anyhow::Result<Vec<IndexEntry>> {
        let client = self
            .read_pool
            .get_connection("load_index_chunk", &self.schema)
            .await?;
        let stmt = client.prepare_cached(LOAD_INDEXES_PAGE).await?;
        let cursor_params = PostgresReader::_index_cursor_params(cursor.as_ref())?;
        let limit = chunk_size as i64;
        let mut params = cursor_params
            .iter()
            .map(|p| p as &(dyn ToSql + Sync))
            .collect_vec();
        params.push(&limit);
        let row_stream = client.query_raw(&stmt, params).await?;

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
                    let mut expired_chunks = expired_entries.chunks_exact(CHUNK_SIZE);
                    for chunk in &mut expired_chunks {
                        let delete_chunk = tx.prepare_cached(DELETE_INDEX_CHUNK).await?;
                        let params = chunk
                            .iter()
                            .map(|index_entry| {
                                PostgresReader::_index_cursor_params(Some(index_entry))
                            })
                            .flatten_ok()
                            .collect::<anyhow::Result<Vec<_>>>()?;
                        deleted_count += tx.execute_raw(&delete_chunk, params).await?;
                    }
                    for index_entry in expired_chunks.remainder() {
                        let delete_index = tx.prepare_cached(DELETE_INDEX).await?;
                        let params = PostgresReader::_index_cursor_params(Some(index_entry))?;
                        deleted_count += tx.execute_raw(&delete_index, params).await?;
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
                    let mut expired_chunks = documents.chunks_exact(CHUNK_SIZE);
                    for chunk in &mut expired_chunks {
                        let delete_chunk = tx.prepare_cached(DELETE_DOCUMENT_CHUNK).await?;
                        let params = chunk
                            .iter()
                            .map(PostgresReader::_document_cursor_params)
                            .flatten_ok()
                            .collect::<anyhow::Result<Vec<_>>>()?;
                        deleted_count += tx.execute_raw(&delete_chunk, params).await?;
                    }
                    for document in expired_chunks.remainder() {
                        let delete_doc = tx.prepare_cached(DELETE_DOCUMENT).await?;
                        let params = PostgresReader::_document_cursor_params(document)?;
                        deleted_count += tx.execute_raw(&delete_doc, params).await?;
                    }
                    Ok(deleted_count as usize)
                }
                .boxed()
            })
            .await
    }
}

#[derive(Clone)]
pub struct PostgresReader {
    read_pool: Arc<ConvexPgPool>,
    version: PersistenceVersion,
    schema: SchemaName,
}

impl PostgresReader {
    fn initial_id_param(order: Order) -> Param {
        Param::Bytes(match order {
            Order::Asc => InternalId::BEFORE_ALL_BYTES.to_vec(),
            Order::Desc => InternalId::AFTER_ALL_BYTES.to_vec(),
        })
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
        let bytes: Vec<u8> = row.get(0);
        let internal_id = InternalId::try_from(bytes)?;
        let ts: i64 = row.get(1);
        let ts = Timestamp::try_from(ts)?;
        let tablet_id_bytes: Vec<u8> = row.get(2);
        let binary_value: Vec<u8> = row.get(3);
        let json_value: JsonValue = serde_json::from_slice(&binary_value)
            .context("Failed to deserialize database value")?;

        let deleted: bool = row.get(4);
        let table = TabletId(
            InternalId::try_from(tablet_id_bytes).context("Invalid ID stored in the database")?,
        );
        let document_id = InternalDocumentId::new(table, internal_id);
        let document = if !deleted {
            let value: ConvexValue = json_value.try_into()?;
            Some(ResolvedDocument::from_database(table, value)?)
        } else {
            None
        };
        let prev_ts: Option<i64> = row.get(5);
        let prev_ts = prev_ts.map(Timestamp::try_from).transpose()?;

        Ok((ts, document_id, document, prev_ts))
    }

    #[try_stream(
        ok = DocumentLogEntry,
        error = anyhow::Error,
    )]
    async fn _load_documents(
        &self,
        range: TimestampRange,
        order: Order,
        page_size: u32,
        retention_validator: Arc<dyn RetentionValidator>,
    ) {
        let timer = metrics::load_documents_timer();
        let mut num_returned = 0;
        let mut last_ts = match order {
            Order::Asc => Timestamp::MIN,
            Order::Desc => Timestamp::MAX,
        };
        let mut last_tablet_id_param = Self::initial_id_param(order);
        let mut last_id_param = Self::initial_id_param(order);
        loop {
            let client = self
                .read_pool
                .get_connection("load_documents", &self.schema)
                .await?;
            let mut rows_loaded = 0;

            let query = match order {
                Order::Asc => &LOAD_DOCS_BY_TS_PAGE_ASC,
                Order::Desc => &LOAD_DOCS_BY_TS_PAGE_DESC,
            };
            let stmt = client.prepare_cached(query).await?;
            let params: Vec<Param> = vec![
                Param::Ts(i64::from(range.min_timestamp_inclusive())),
                Param::Ts(i64::from(range.max_timestamp_exclusive())),
                Param::Ts(i64::from(last_ts)),
                last_tablet_id_param.clone(),
                last_id_param.clone(),
                Param::Limit(page_size as i64),
            ];
            let row_stream = client.query_raw(&stmt, params).await?;

            futures::pin_mut!(row_stream);

            let mut batch = vec![];
            while let Some(row) = row_stream.try_next().await? {
                let (ts, document_id, document, prev_ts) = self.row_to_document(row)?;
                rows_loaded += 1;
                last_ts = ts;
                last_tablet_id_param = Param::TableId(document_id.table());
                last_id_param = internal_doc_id_param(document_id);
                num_returned += 1;
                batch.push(DocumentLogEntry {
                    ts,
                    id: document_id,
                    value: document,
                    prev_ts,
                });
            }
            // Return the connection to the pool as soon as possible.
            drop(client);

            // N.B.: `retention_validator` can itself talk back to this
            // PersistenceReader (to call get_persistence_global). This uses a
            // separate connection.
            // TODO: ideally we should be using the same connection.
            // If we ever run against a replica DB this should also make sure
            // that the data read & validation read run against the same
            // replica - otherwise we may not be validating the right thing!
            retention_validator
                .validate_document_snapshot(range.min_timestamp_inclusive())
                .await?;

            for row in batch {
                yield row;
            }
            if rows_loaded < page_size {
                break;
            }
        }

        metrics::finish_load_documents_timer(timer, num_returned);
    }

    #[try_stream(ok = (IndexKeyBytes, LatestDocument), error = anyhow::Error)]
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
        // We use the size_hint to determine the batch size. This means in the
        // common case we should do a single query. Exceptions are if the size_hint
        // is wrong or if we truncate it or if we observe too many deletes.
        let batch_size = size_hint.clamp(1, 5000);
        let (tx, mut rx) = mpsc::channel(batch_size);
        let task = AbortOnDropHandle::new(common::runtime::tokio_spawn(
            func_path!(),
            self.clone()._index_scan_inner(
                index_id,
                read_timestamp,
                interval,
                order,
                batch_size,
                retention_validator,
                tx,
            ),
        ));
        while let Some((key, ts, binary_value, prev_ts)) = rx.recv().await {
            let json_value: JsonValue = serde_json::from_slice(&binary_value)
                .context("Failed to deserialize database value")?;
            anyhow::ensure!(
                json_value != JsonValue::Null,
                "Index reference to deleted document {:?} {:?}",
                key,
                ts
            );
            let value: ConvexValue = json_value.try_into()?;
            let document = ResolvedDocument::from_database(tablet_id, value)?;
            yield (
                key,
                LatestDocument {
                    ts,
                    value: document,
                    prev_ts,
                },
            );
        }
        task.await??;
    }

    async fn _index_scan_inner(
        self,
        index_id: IndexId,
        read_timestamp: Timestamp,
        interval: Interval,
        order: Order,
        batch_size: usize,
        retention_validator: Arc<dyn RetentionValidator>,
        tx: mpsc::Sender<(IndexKeyBytes, Timestamp, Vec<u8>, Option<Timestamp>)>,
    ) -> anyhow::Result<()> {
        let _timer = metrics::query_index_timer();
        let (mut lower, mut upper) = to_sql_bounds(interval.clone());

        let mut stats = QueryIndexStats::new();

        // We iterate results in (key_prefix, key_sha256) order while we actually
        // need them in (key_prefix, key_suffix order). key_suffix is not part of the
        // primary key so we do the sort here. If see any record with maximum length
        // prefix, we should buffer it until we reach a different prefix.
        let mut result_buffer: Vec<(IndexKeyBytes, Timestamp, Vec<u8>, Option<Timestamp>)> =
            Vec::new();
        loop {
            let client = self
                .read_pool
                .get_connection("index_scan", &self.schema)
                .await?;
            stats.sql_statements += 1;
            let (query, params) = index_query(
                index_id,
                read_timestamp,
                lower.clone(),
                upper.clone(),
                order,
                batch_size,
            );

            let prepare_timer = metrics::query_index_sql_prepare_timer();
            let stmt = client.prepare_cached(query).await?;
            prepare_timer.finish();

            let execute_timer = metrics::query_index_sql_execute_timer();
            let row_stream = client.query_raw(&stmt, params).await?;
            execute_timer.finish();

            futures::pin_mut!(row_stream);

            let mut batch_rows = 0;
            let mut batch = vec![];
            while let Some(row) = row_stream.try_next().await? {
                batch_rows += 1;

                // Fetch
                let internal_row = parse_row(&row)?;

                // Yield buffered results if applicable.
                if let Some((buffer_key, ..)) = result_buffer.first() {
                    if buffer_key[..MAX_INDEX_KEY_PREFIX_LEN] != internal_row.key_prefix {
                        // We have exhausted all results that share the same key prefix
                        // we can sort and yield the buffered results.
                        result_buffer.sort_by(|a, b| a.0.cmp(&b.0));
                        for (key, ts, doc, prev_ts) in order.apply(result_buffer.drain(..)) {
                            if interval.contains(&key) {
                                stats.rows_returned += 1;
                                batch.push((key, ts, doc, prev_ts));
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
                let table: Option<Vec<u8>> = row.get(7);
                table.ok_or_else(|| {
                    anyhow::anyhow!("Dangling index reference for {:?} {:?}", key, ts)
                })?;
                let binary_value: Vec<u8> = row.get(8);

                let prev_ts: Option<i64> = row.get(9);
                let prev_ts = prev_ts.map(Timestamp::try_from).transpose()?;

                if key.len() < MAX_INDEX_KEY_PREFIX_LEN {
                    assert!(result_buffer.is_empty());
                    if interval.contains(&key) {
                        stats.rows_returned += 1;
                        batch.push((IndexKeyBytes(key), ts, binary_value, prev_ts));
                    } else {
                        stats.rows_skipped_out_of_range += 1;
                    }
                } else {
                    // There might be other records with the same key_prefix that
                    // are ordered before this result. Buffer it.
                    result_buffer.push((IndexKeyBytes(key), ts, binary_value, prev_ts));
                    stats.max_rows_buffered =
                        cmp::max(result_buffer.len(), stats.max_rows_buffered);
                }
            }

            // Return the connection to the pool as soon as possible.
            drop(client);

            // N.B.: `retention_validator` can itself talk back to this
            // PersistenceReader (to call get_persistence_global). This uses a
            // separate connection.
            // TODO: ideally we should be using the same connection.
            // If we ever run against a replica DB this should also make sure
            // that the data read & validation read run against the same
            // replica - otherwise we may not be validating the right thing!
            let retention_validate_timer = metrics::retention_validate_timer();
            retention_validator
                .validate_snapshot(read_timestamp)
                .await?;
            retention_validate_timer.finish();

            for row in batch {
                // this could block arbitrarily long if the caller of
                // `index_scan` stops polling
                tx.send(row).await?;
            }

            if batch_rows < batch_size {
                break;
            }
        }

        // Yield any remaining values.
        result_buffer.sort_by(|a, b| a.0.cmp(&b.0));
        for (key, ts, doc, prev_ts) in order.apply(result_buffer.drain(..)) {
            if interval.contains(&key) {
                stats.rows_returned += 1;
                tx.send((key, ts, doc, prev_ts)).await?;
            } else {
                stats.rows_skipped_out_of_range += 1;
            }
        }

        Ok(())
    }

    fn _index_cursor_params(cursor: Option<&IndexEntry>) -> anyhow::Result<Vec<Param>> {
        match cursor {
            Some(cursor) => {
                let last_id_param = Param::Bytes(cursor.index_id.into());
                let last_key_prefix = Param::Bytes(cursor.key_prefix.clone());
                let last_sha256 = Param::Bytes(cursor.key_sha256.clone());
                let last_ts = Param::Ts(cursor.ts.into());
                Ok(vec![last_id_param, last_key_prefix, last_sha256, last_ts])
            },
            None => {
                let last_id_param = Self::initial_id_param(Order::Asc);
                let last_key_prefix = Param::Bytes(vec![]);
                let last_sha256 = Param::Bytes(vec![]);
                let last_ts = Param::Ts(0);
                Ok(vec![last_id_param, last_key_prefix, last_sha256, last_ts])
            },
        }
    }

    fn _document_cursor_params(
        (ts, internal_id): &(Timestamp, InternalDocumentId),
    ) -> anyhow::Result<Vec<Param>> {
        let tablet_id = Param::Bytes(internal_id.table().0.into());
        let id = Param::Bytes(internal_id.internal_id().into());
        let ts = Param::Ts((*ts).into());
        Ok(vec![tablet_id, id, ts])
    }
}

fn parse_row(row: &Row) -> anyhow::Result<IndexEntry> {
    let bytes: Vec<u8> = row.get(0);
    let index_id =
        InternalId::try_from(bytes).map_err(|_| anyhow::anyhow!("index_id wrong size"))?;

    let key_prefix: Vec<u8> = row.get(1);
    let key_sha256: Vec<u8> = row.get(2);
    let key_suffix: Option<Vec<u8>> = row.get(3);
    let ts: i64 = row.get(4);
    let ts = Timestamp::try_from(ts)?;
    let deleted: bool = row.get(5);
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
impl PersistenceReader for PostgresReader {
    fn load_documents(
        &self,
        range: TimestampRange,
        order: Order,
        page_size: u32,
        retention_validator: Arc<dyn RetentionValidator>,
    ) -> DocumentStream<'_> {
        self._load_documents(range, order, page_size, retention_validator)
            .boxed()
    }

    async fn previous_revisions(
        &self,
        ids: BTreeSet<(InternalDocumentId, Timestamp)>,
        retention_validator: Arc<dyn RetentionValidator>,
    ) -> anyhow::Result<BTreeMap<(InternalDocumentId, Timestamp), DocumentLogEntry>> {
        let timer = metrics::prev_revisions_timer();

        let client = self
            .read_pool
            .get_connection("previous_revisions", &self.schema)
            .await?;
        let (prev_rev_chunk, prev_rev) = try_join!(
            client.prepare_cached(PREV_REV_CHUNK),
            client.prepare_cached(PREV_REV)
        )?;
        let ids: Vec<_> = ids.into_iter().collect();

        let mut result = BTreeMap::new();

        let mut chunks = ids.chunks_exact(CHUNK_SIZE);
        // NOTE we rely on `client.query_raw` not executing anything until it is
        // awaited, i.e. we rely on it being an async fn and not just returning an
        // `impl Future`. This guarantees only `PIPELINE_QUERIES` queries are in
        // the pipeline at once.
        let mut result_futures = vec![];

        let mut min_ts = Timestamp::MAX;
        for chunk in chunks.by_ref() {
            assert_eq!(chunk.len(), 8);
            let mut params = Vec::with_capacity(24);
            for (id, ts) in chunk {
                params.push(Param::TableId(id.table()));
                params.push(internal_doc_id_param(*id));
                params.push(Param::Ts(i64::from(*ts)));
                min_ts = cmp::min(*ts, min_ts);
            }
            result_futures.push(client.query_raw(&prev_rev_chunk, params));
        }
        for (id, ts) in chunks.remainder() {
            let params = vec![
                Param::TableId(id.table()),
                internal_doc_id_param(*id),
                Param::Ts(i64::from(*ts)),
            ];
            min_ts = cmp::min(*ts, min_ts);
            result_futures.push(client.query_raw(&prev_rev, params));
        }
        let mut result_stream = stream::iter(result_futures).buffered(*PIPELINE_QUERIES);
        while let Some(row_stream) = result_stream.try_next().await? {
            futures::pin_mut!(row_stream);
            while let Some(row) = row_stream.try_next().await? {
                let ts: i64 = row.get(6);
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
            }
        }

        retention_validator
            .validate_document_snapshot(min_ts)
            .await?;
        timer.finish();
        Ok(result)
    }

    async fn previous_revisions_of_documents(
        &self,
        ids: BTreeSet<DocumentPrevTsQuery>,
        retention_validator: Arc<dyn RetentionValidator>,
    ) -> anyhow::Result<BTreeMap<DocumentPrevTsQuery, DocumentLogEntry>> {
        let timer = metrics::previous_revisions_of_documents_timer();

        let client = self
            .read_pool
            .get_connection("previous_revisions_of_documents", &self.schema)
            .await?;
        let (exact_rev_chunk, exact_rev) = try_join!(
            client.prepare_cached(EXACT_REV_CHUNK),
            client.prepare_cached(EXACT_REV)
        )?;
        let ids: Vec<_> = ids.into_iter().collect();

        let mut result = BTreeMap::new();

        let mut chunks = ids.chunks_exact(CHUNK_SIZE);
        let mut result_futures = vec![];

        for chunk in chunks.by_ref() {
            assert_eq!(chunk.len(), 8);
            let mut params = Vec::with_capacity(24);
            for DocumentPrevTsQuery { id, ts, prev_ts } in chunk {
                params.push(Param::TableId(id.table()));
                params.push(internal_doc_id_param(*id));
                params.push(Param::Ts(i64::from(*prev_ts)));
                params.push(Param::Ts(i64::from(*ts)));
            }
            result_futures.push(client.query_raw(&exact_rev_chunk, params));
        }
        for DocumentPrevTsQuery { id, ts, prev_ts } in chunks.remainder() {
            let params = vec![
                Param::TableId(id.table()),
                internal_doc_id_param(*id),
                Param::Ts(i64::from(*prev_ts)),
                Param::Ts(i64::from(*ts)),
            ];
            result_futures.push(client.query_raw(&exact_rev, params));
        }
        let mut result_stream = stream::iter(result_futures).buffered(*PIPELINE_QUERIES);
        while let Some(row_stream) = result_stream.try_next().await? {
            futures::pin_mut!(row_stream);
            while let Some(row) = row_stream.try_next().await? {
                let ts: i64 = row.get(6);
                let ts = Timestamp::try_from(ts)?;
                let (prev_ts, id, maybe_doc, prev_prev_ts) = self.row_to_document(row)?;
                anyhow::ensure!(result
                    .insert(
                        DocumentPrevTsQuery { id, ts, prev_ts },
                        DocumentLogEntry {
                            ts: prev_ts,
                            id,
                            value: maybe_doc,
                            prev_ts: prev_prev_ts,
                        }
                    )
                    .is_none());
            }
        }

        if let Some(min_ts) = ids.iter().map(|DocumentPrevTsQuery { ts, .. }| *ts).min() {
            // Validate retention after finding documents
            retention_validator
                .validate_document_snapshot(min_ts)
                .await?;
        }

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
        let client = self
            .read_pool
            .get_connection("get_persistence_global", &self.schema)
            .await?;
        let stmt = client.prepare_cached(GET_PERSISTENCE_GLOBAL).await?;
        let params = vec![Param::PersistenceGlobalKey(key)];
        let row_stream = client.query_raw(&stmt, params).await?;
        futures::pin_mut!(row_stream);

        let row = row_stream.try_next().await?;
        let value = row.map(|r| -> anyhow::Result<JsonValue> {
            let binary_value: Vec<u8> = r.get(0);
            let mut json_deserializer = serde_json::Deserializer::from_slice(&binary_value);
            // XXX: this is bad, but shapes can get much more nested than convex values
            json_deserializer.disable_recursion_limit();
            let json_value = JsonValue::deserialize(&mut json_deserializer)
                .with_context(|| format!("Invalid JSON at persistence key {key:?}"))?;
            json_deserializer.end()?;
            Ok(json_value)
        });
        value.transpose()
    }

    fn version(&self) -> PersistenceVersion {
        self.version
    }

    async fn table_size_stats(&self) -> anyhow::Result<Vec<PersistenceTableSize>> {
        let client = self
            .read_pool
            .get_connection("table_size_stats", &self.schema)
            .await?;
        let mut stats = vec![];
        for &table in TABLES {
            let row = client
                .query_opt(
                    TABLE_SIZE_QUERY,
                    &[&format!("{}.{table}", self.schema.escaped)],
                )
                .await?
                .context("nothing returned from table size query?")?;
            stats.push(PersistenceTableSize {
                table_name: table.to_owned(),
                data_bytes: row.try_get::<_, i64>(0)? as u64,
                index_bytes: row.try_get::<_, i64>(1)? as u64,
                row_count: 0, // not supported easily
            });
        }
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
/// transaction, and otherwise return a `LeaseLostError`.
struct Lease {
    pool: Arc<ConvexPgPool>,
    lease_ts: i64,
    schema: SchemaName,
}

impl Lease {
    /// Acquire a lease. Blocks as long as there is another lease holder.
    /// Returns any transient errors encountered.
    async fn acquire(pool: Arc<ConvexPgPool>, schema: &SchemaName) -> anyhow::Result<Self> {
        let timer = metrics::lease_acquire_timer();
        let client = pool.get_connection("lease_acquire", schema).await?;
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("before 1970")
            .as_nanos() as i64;

        tracing::info!("attempting to acquire lease");
        let stmt = client.prepare_cached(LEASE_ACQUIRE).await?;
        let rows_modified = client.execute(&stmt, &[&ts]).await?;
        drop(client);
        anyhow::ensure!(
            rows_modified == 1,
            "failed to acquire lease: Already acquired with higher timestamp"
        );
        tracing::info!("lease acquired with ts {}", ts);

        timer.finish();
        Ok(Self {
            pool,
            lease_ts: ts,
            schema: schema.clone(),
        })
    }

    /// Execute the transaction function f atomically ensuring that the lease is
    /// still held, otherwise return `LeaseLostError`.
    ///
    /// Once `transact` returns `LeaseLostError`, no future transactions using
    /// it will succeed. Instead, a new `Lease` must be made with `acquire`,
    /// and any in-memory state then resynced because of any changes that
    /// might've been made to the database state while the lease was not
    /// held.
    async fn transact<F, T>(&self, f: F) -> anyhow::Result<T>
    where
        F: for<'b> FnOnce(
            &'b PostgresTransaction,
        ) -> Pin<Box<dyn Future<Output = anyhow::Result<T>> + Send + 'b>>,
    {
        let mut client = self.pool.get_connection("transact", &self.schema).await?;
        let tx = client.transaction().await?;
        let lease_ts = self.lease_ts;

        let advisory_lease_check = async {
            let timer = metrics::lease_check_timer();
            let stmt = tx.prepare_cached(ADVISORY_LEASE_CHECK).await?;
            let rows = tx.query(&stmt, &[&lease_ts]).await?;
            if rows.len() != 1 {
                return Err(LeaseLostError {}.into());
            }
            timer.finish();
            Ok(())
        };

        let ((), result) = future::try_join(advisory_lease_check, f(&tx)).await?;

        // We don't run SELECT FOR UPDATE until the *end* of the transaction
        // to minimize the time spent holding the row lock, and therefore allow
        // the lease to be stolen as much as possible.
        let timer = metrics::lease_precond_timer();
        let stmt = tx.prepare_cached(LEASE_PRECOND).await?;
        let rows = tx.query(&stmt, &[&lease_ts]).await?;
        if rows.len() != 1 {
            return Err(LeaseLostError {}.into());
        }
        timer.finish();

        let timer = metrics::commit_timer();
        tx.commit().await?;
        timer.finish();

        Ok(result)
    }
}

fn document_params(
    ts: Timestamp,
    id: InternalDocumentId,
    maybe_document: &Option<ResolvedDocument>,
    prev_ts: Option<Timestamp>,
) -> anyhow::Result<[Param; NUM_DOCUMENT_PARAMS]> {
    let (json_value, deleted) = match maybe_document {
        Some(doc) => (doc.value().json_serialize()?, false),
        None => (JsonValue::Null.to_string(), true),
    };

    Ok([
        internal_doc_id_param(id),
        Param::Ts(i64::from(ts)),
        Param::TableId(id.table()),
        Param::JsonValue(json_value),
        Param::Deleted(deleted),
        match prev_ts {
            Some(prev_ts) => Param::Ts(i64::from(prev_ts)),
            None => Param::None,
        },
    ])
}

fn internal_id_param(id: InternalId) -> Param {
    Param::Bytes(id.into())
}

fn internal_doc_id_param(id: InternalDocumentId) -> Param {
    internal_id_param(id.internal_id())
}
fn resolved_id_param(id: &ResolvedDocumentId) -> Param {
    internal_id_param(id.internal_id())
}

fn index_params((ts, update): &(Timestamp, DatabaseIndexUpdate)) -> [Param; NUM_INDEX_PARAMS] {
    let key: Vec<u8> = update.key.to_bytes().0;
    let key_sha256 = Sha256::hash(&key);
    let key = SplitKey::new(key);

    let (deleted, tablet_id, doc_id) = match &update.value {
        DatabaseIndexValue::Deleted => (Param::Deleted(true), Param::None, Param::None),
        DatabaseIndexValue::NonClustered(doc_id) => (
            Param::Deleted(false),
            Param::TableId(doc_id.tablet_id),
            resolved_id_param(doc_id),
        ),
    };
    [
        internal_id_param(update.index_id),
        Param::Ts(i64::from(*ts)),
        Param::Bytes(key.prefix),
        match key.suffix {
            Some(key_suffix) => Param::Bytes(key_suffix),
            None => Param::None,
        },
        Param::Bytes(key_sha256.to_vec()),
        deleted,
        tablet_id,
        doc_id,
    ]
}

#[derive(Clone, Debug)]
enum Param {
    None,
    Ts(i64),
    Limit(i64),
    TableId(TabletId),
    JsonValue(String),
    Deleted(bool),
    Bytes(Vec<u8>),
    PersistenceGlobalKey(PersistenceGlobalKey),
}

impl ToSql for Param {
    to_sql_checked!();

    fn to_sql(
        &self,
        ty: &Type,
        out: &mut BytesMut,
    ) -> Result<IsNull, Box<dyn Error + Send + Sync + 'static>> {
        match self {
            Param::None => Ok(IsNull::Yes),
            Param::Ts(ts) => ts.to_sql(ty, out),
            Param::TableId(s) => s.0 .0.to_vec().to_sql(ty, out),
            Param::JsonValue(v) => v.as_bytes().to_sql(ty, out),
            Param::Deleted(d) => d.to_sql(ty, out),
            Param::Bytes(v) => v.to_sql(ty, out),
            Param::Limit(v) => v.to_sql(ty, out),
            Param::PersistenceGlobalKey(key) => String::from(*key).to_sql(ty, out),
        }
    }

    // TODO(presley): This does not seem right. Perhaps we should simply
    // use native types?
    fn accepts(ty: &Type) -> bool {
        i64::accepts(ty)
            || String::accepts(ty)
            || JsonValue::accepts(ty)
            || bool::accepts(ty)
            || Vec::<u8>::accepts(ty)
    }
}

const CHECK_SCHEMA_SQL: &str = r"SELECT 1 FROM information_schema.schemata WHERE schema_name = $1";
const CREATE_SCHEMA_SQL: &str = r"CREATE SCHEMA IF NOT EXISTS @db_name;";
// This runs (currently) every time a PostgresPersistence is created, so it
// needs to not only be idempotent but not to affect any already-resident data.
// IF NOT EXISTS and ON CONFLICT are helpful.
const INIT_SQL: &str = r#"
        CREATE TABLE IF NOT EXISTS @db_name.documents (
            id BYTEA NOT NULL,
            ts BIGINT NOT NULL,

            table_id BYTEA NOT NULL,

            json_value BYTEA NOT NULL,
            deleted BOOLEAN DEFAULT false,

            prev_ts BIGINT,

            PRIMARY KEY (ts, table_id, id)
        );
        CREATE INDEX IF NOT EXISTS documents_by_table_and_id ON @db_name.documents (
            table_id, id, ts
        );
        CREATE INDEX IF NOT EXISTS documents_by_table_ts_and_id ON @db_name.documents (
            table_id, ts, id
        );

        CREATE TABLE IF NOT EXISTS @db_name.indexes (
            /* ids should be serialized as bytes but we keep it compatible with documents */
            index_id BYTEA NOT NULL,
            ts BIGINT NOT NULL,

            /*
            Postgres maximum primary key length is 2730 bytes, which
            is why we split up the key. The first 2500 bytes are stored in key_prefix,
            and the remaining ones are stored in key suffix if applicable.
            NOTE: The key_prefix + key_suffix is store all values of IndexKey including
            the id.
            */
            key_prefix BYTEA NOT NULL,
            key_suffix BYTEA NULL,

            /* key_sha256 of the full key, used in primary key to avoid duplicates in case
            of key_prefix collision. */
            key_sha256 BYTEA NOT NULL,

            deleted BOOLEAN,

            /* table_id should be populated iff deleted is false. */
            table_id BYTEA NULL,
            /* document_id should be populated iff deleted is false. */
            document_id BYTEA NULL,
            PRIMARY KEY (index_id, key_prefix, key_sha256, ts)
        );
        /* This index with `ts DESC` enables our "loose index scan" queries
         * (i.e. `DISTINCT ON`) to run in both directions, complementing the
         * primary key's ts ASC ordering */
        CREATE UNIQUE INDEX IF NOT EXISTS indexes_by_index_id_key_prefix_key_sha256_ts ON @db_name.indexes (
            index_id,
            key_prefix,
            key_sha256,
            ts DESC
        );

        CREATE TABLE IF NOT EXISTS @db_name.leases (
            id BIGINT NOT NULL,
            ts BIGINT NOT NULL,

            PRIMARY KEY (id)
        );
        INSERT INTO @db_name.leases (id, ts) VALUES (1, 0) ON CONFLICT DO NOTHING;
        CREATE TABLE IF NOT EXISTS @db_name.read_only (
            id BIGINT NOT NULL,

            PRIMARY KEY (id)
        );
        CREATE TABLE IF NOT EXISTS @db_name.persistence_globals (
            key TEXT NOT NULL,
            json_value BYTEA NOT NULL,
            PRIMARY KEY (key)
        );"#;
const TABLES: &[&str] = &[
    "documents",
    "indexes",
    "leases",
    "read_only",
    "persistence_globals",
];
/// Load a page of documents, where timestamps are bounded by [$1, $2),
/// and ($3, $4) is the (ts, id) from the last document read.
const LOAD_DOCS_BY_TS_PAGE_ASC: &str = r#"
/*+
    Set(enable_seqscan OFF)
    Set(enable_sort OFF)
    Set(enable_incremental_sort OFF)
    Set(enable_hashjoin OFF)
    Set(enable_mergejoin OFF)
    Set(enable_material OFF)
    Set(plan_cache_mode force_generic_plan)
*/
SELECT id, ts, table_id, json_value, deleted, prev_ts
    FROM @db_name.documents
    WHERE ts >= $1
    AND ts < $2
    AND (ts, table_id, id) > ($3, $4, $5)
    ORDER BY ts ASC, table_id ASC, id ASC
    LIMIT $6
"#;

const LOAD_DOCS_BY_TS_PAGE_DESC: &str = r#"
/*+
    Set(enable_seqscan OFF)
    Set(enable_sort OFF)
    Set(enable_incremental_sort OFF)
    Set(enable_hashjoin OFF)
    Set(enable_mergejoin OFF)
    Set(enable_material OFF)
    Set(plan_cache_mode force_generic_plan)
*/
SELECT id, ts, table_id, json_value, deleted, prev_ts
    FROM @db_name.documents
    WHERE ts >= $1
    AND ts < $2
    AND (ts, table_id, id) < ($3, $4, $5)
    ORDER BY ts DESC, table_id DESC, id DESC
    LIMIT $6
"#;

const INSERT_DOCUMENT: &str = r#"INSERT INTO @db_name.documents
    (id, ts, table_id, json_value, deleted, prev_ts)
    VALUES ($1, $2, $3, $4, $5, $6)
"#;

const INSERT_OVERWRITE_DOCUMENT: &str = r#"INSERT INTO @db_name.documents
    (id, ts, table_id, json_value, deleted, prev_ts)
    VALUES ($1, $2, $3, $4, $5, $6)
    ON CONFLICT (id, ts, table_id) DO UPDATE
    SET deleted = excluded.deleted, json_value = excluded.json_value
"#;

const INSERT_DOCUMENT_CHUNK: &str = r#"INSERT INTO @db_name.documents
    (id, ts, table_id, json_value, deleted, prev_ts)
    VALUES
        ($1, $2, $3, $4, $5, $6),
        ($7, $8, $9, $10, $11, $12),
        ($13, $14, $15, $16, $17, $18),
        ($19, $20, $21, $22, $23, $24),
        ($25, $26, $27, $28, $29, $30),
        ($31, $32, $33, $34, $35, $36),
        ($37, $38, $39, $40, $41, $42),
        ($43, $44, $45, $46, $47, $48)
"#;

const INSERT_OVERWRITE_DOCUMENT_CHUNK: &str = r#"INSERT INTO @db_name.documents
    (id, ts, table_id, json_value, deleted, prev_ts)
    VALUES
        ($1, $2, $3, $4, $5, $6),
        ($7, $8, $9, $10, $11, $12),
        ($13, $14, $15, $16, $17, $18),
        ($19, $20, $21, $22, $23, $24),
        ($25, $26, $27, $28, $29, $30),
        ($31, $32, $33, $34, $35, $36),
        ($37, $38, $39, $40, $41, $42),
        ($43, $44, $45, $46, $47, $48)
    ON CONFLICT (id, ts, table_id) DO UPDATE
    SET deleted = excluded.deleted, json_value = excluded.json_value
"#;

const LOAD_INDEXES_PAGE: &str = r#"
/*+
    Set(enable_seqscan OFF)
    Set(enable_sort OFF)
    Set(enable_incremental_sort OFF)
    Set(enable_hashjoin OFF)
    Set(enable_mergejoin OFF)
    Set(enable_material OFF)
    Set(plan_cache_mode force_generic_plan)
*/
SELECT
    index_id, key_prefix, key_sha256, key_suffix, ts, deleted
    FROM @db_name.indexes
    WHERE (index_id, key_prefix, key_sha256, ts) > ($1, $2, $3, $4)
    ORDER BY index_id ASC, key_prefix ASC, key_sha256 ASC, ts ASC
    LIMIT $5
"#;

const INSERT_INDEX: &str = r#"INSERT INTO @db_name.indexes
    (index_id, ts, key_prefix, key_suffix, key_sha256, deleted, table_id, document_id)
    VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
"#;

// Note that on conflict, there's no need to update any of the columns that are
// part of the primary key, nor `key_suffix` as `key_sha256` is derived from the
// prefix and suffix.
// Only the fields that could have actually changed need to be updated.
const INSERT_OVERWRITE_INDEX: &str = r#"INSERT INTO @db_name.indexes
    (index_id, ts, key_prefix, key_suffix, key_sha256, deleted, table_id, document_id)
    VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
    ON CONFLICT (index_id, ts, key_prefix, key_sha256) DO UPDATE
    SET deleted = excluded.deleted, table_id = excluded.table_id, document_id = excluded.document_id
"#;

const DELETE_INDEX: &str = r#"
/*+
    Set(enable_seqscan OFF)
    Set(plan_cache_mode force_generic_plan)
*/
DELETE FROM @db_name.indexes WHERE
    (index_id = $1 AND key_prefix = $2 AND key_sha256 = $3 AND ts <= $4)
"#;

const DELETE_DOCUMENT: &str = r#"
/*+
    Set(enable_seqscan OFF)
    Set(plan_cache_mode force_generic_plan)
*/
DELETE FROM @db_name.documents WHERE
    (table_id = $1 AND id = $2 AND ts <= $3)
"#;

const INSERT_INDEX_CHUNK: &str = r#"INSERT INTO @db_name.indexes
    (index_id, ts, key_prefix, key_suffix, key_sha256, deleted, table_id, document_id)
    VALUES
        ($1, $2, $3, $4, $5, $6, $7, $8),
        ($9, $10, $11, $12, $13, $14, $15, $16),
        ($17, $18, $19, $20, $21, $22, $23, $24),
        ($25, $26, $27, $28, $29, $30, $31, $32),
        ($33, $34, $35, $36, $37, $38, $39, $40),
        ($41, $42, $43, $44, $45, $46, $47, $48),
        ($49, $50, $51, $52, $53, $54, $55, $56),
        ($57, $58, $59, $60, $61, $62, $63, $64)
"#;

const INSERT_OVERWRITE_INDEX_CHUNK: &str = r#"INSERT INTO @db_name.indexes
    (index_id, ts, key_prefix, key_suffix, key_sha256, deleted, table_id, document_id)
    VALUES
        ($1, $2, $3, $4, $5, $6, $7, $8),
        ($9, $10, $11, $12, $13, $14, $15, $16),
        ($17, $18, $19, $20, $21, $22, $23, $24),
        ($25, $26, $27, $28, $29, $30, $31, $32),
        ($33, $34, $35, $36, $37, $38, $39, $40),
        ($41, $42, $43, $44, $45, $46, $47, $48),
        ($49, $50, $51, $52, $53, $54, $55, $56),
        ($57, $58, $59, $60, $61, $62, $63, $64)
        ON CONFLICT (index_id, ts, key_prefix, key_sha256) DO UPDATE
        SET deleted = excluded.deleted, table_id = excluded.table_id, document_id = excluded.document_id
"#;

const DELETE_INDEX_CHUNK: &str = r#"
/*+
    Set(enable_seqscan OFF)
    Set(plan_cache_mode force_generic_plan)
*/
DELETE FROM @db_name.indexes WHERE
    (index_id = $1 AND key_prefix = $2 AND key_sha256 = $3 AND ts <= $4) OR
    (index_id = $5 AND key_prefix = $6 AND key_sha256 = $7 AND ts <= $8) OR
    (index_id = $9 AND key_prefix = $10 AND key_sha256 = $11 AND ts <= $12) OR
    (index_id = $13 AND key_prefix = $14 AND key_sha256 = $15 AND ts <= $16) OR
    (index_id = $17 AND key_prefix = $18 AND key_sha256 = $19 AND ts <= $20) OR
    (index_id = $21 AND key_prefix = $22 AND key_sha256 = $23 AND ts <= $24) OR
    (index_id = $25 AND key_prefix = $26 AND key_sha256 = $27 AND ts <= $28) OR
    (index_id = $29 AND key_prefix = $30 AND key_sha256 = $31 AND ts <= $32)
"#;

const DELETE_DOCUMENT_CHUNK: &str = r#"
/*+
    Set(enable_seqscan OFF)
    Set(plan_cache_mode force_generic_plan)
*/
DELETE FROM @db_name.documents WHERE
    (table_id = $1 AND id = $2 AND ts <= $3) OR
    (table_id = $5 AND id = $6 AND ts <= $7) OR
    (table_id = $9 AND id = $10 AND ts <= $11) OR
    (table_id = $13 AND id = $14 AND ts <= $15) OR
    (table_id = $17 AND id = $18 AND ts <= $19) OR
    (table_id = $21 AND id = $22 AND ts <= $23) OR
    (table_id = $25 AND id = $26 AND ts <= $27) OR
    (table_id = $29 AND id = $30 AND ts <= $31)
"#;

const WRITE_PERSISTENCE_GLOBAL: &str = r#"INSERT INTO @db_name.persistence_globals
    (key, json_value)
    VALUES ($1, $2)
    ON CONFLICT (key) DO UPDATE
    SET json_value = excluded.json_value
"#;

const GET_PERSISTENCE_GLOBAL: &str =
    "SELECT json_value FROM @db_name.persistence_globals WHERE key = $1";

const CHUNK_SIZE: usize = 8;
const NUM_DOCUMENT_PARAMS: usize = 6;
const NUM_INDEX_PARAMS: usize = 8;
const MAX_INSERT_SIZE: usize = 16384;
static PIPELINE_QUERIES: LazyLock<usize> = LazyLock::new(|| env_config("PIPELINE_QUERIES", 16));

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
const LEASE_PRECOND: &str = "SELECT 1 FROM @db_name.leases WHERE id=1 AND ts=$1 FOR SHARE";
// Checks if we still hold the lease without blocking another instance from
// stealing it.
const ADVISORY_LEASE_CHECK: &str = "SELECT 1 FROM @db_name.leases WHERE id=1 AND ts=$1";

// Acquire the lease unless acquire by someone with a higher timestamp.
const LEASE_ACQUIRE: &str = "UPDATE @db_name.leases SET ts=$1 WHERE id=1 AND ts<$1";

#[derive(PartialEq, Eq, Hash, Copy, Clone)]
enum BoundType {
    Unbounded,
    Included,
    Excluded,
}

// Pre-build queries with various parameters.
static INDEX_QUERIES: LazyLock<HashMap<(BoundType, BoundType, Order), String>> =
    LazyLock::new(|| {
        let mut queries = HashMap::new();

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
            let mut current_arg = 1..;
            let mut next_arg = || current_arg.next().unwrap();

            let mut where_clause = String::new();
            write!(where_clause, "index_id = ${}", next_arg()).unwrap();
            let ts_arg = next_arg();
            write!(where_clause, " AND ts <= ${}", ts_arg).unwrap();
            match lower {
                BoundType::Unbounded => {},
                BoundType::Included => {
                    write!(
                        where_clause,
                        " AND (key_prefix, key_sha256) >= (${}, ${})",
                        next_arg(),
                        next_arg(),
                    )
                    .unwrap();
                },
                BoundType::Excluded => {
                    write!(
                        where_clause,
                        " AND (key_prefix, key_sha256) > (${}, ${})",
                        next_arg(),
                        next_arg(),
                    )
                    .unwrap();
                },
            };
            match upper {
                BoundType::Unbounded => {},
                BoundType::Included => {
                    write!(
                        where_clause,
                        " AND (key_prefix, key_sha256) <= (${}, ${})",
                        next_arg(),
                        next_arg(),
                    )
                    .unwrap();
                },
                BoundType::Excluded => {
                    write!(
                        where_clause,
                        " AND (key_prefix, key_sha256) < (${}, ${})",
                        next_arg(),
                        next_arg(),
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
/*+
    Set(enable_seqscan OFF)
    Set(enable_bitmapscan OFF)
    Set(plan_cache_mode force_generic_plan)
*/
SELECT
    A.index_id,
    A.key_prefix,
    A.key_sha256,
    A.key_suffix,
    A.ts,
    A.deleted,
    A.document_id,
    D.table_id,
    D.json_value,
    D.prev_ts
FROM (
    SELECT DISTINCT ON (key_prefix, key_sha256)
        index_id,
        key_prefix,
        key_sha256,
        key_suffix,
        ts,
        deleted,
        document_id,
        table_id
    FROM @db_name.indexes
    WHERE {where_clause}
    ORDER BY key_prefix {order_str}, key_sha256 {order_str}, ts DESC
    LIMIT ${}
) A
LEFT JOIN @db_name.documents D
    ON  D.ts          = A.ts
    AND D.table_id    = A.table_id
    AND D.id          = A.document_id
ORDER BY key_prefix {order_str}, key_sha256 {order_str}
"#,
                next_arg()
            );
            queries.insert((*lower, *upper, *order), query);
        }

        queries
    });

const PREV_REV_CHUNK: &str = r#"
/*+
    Set(enable_seqscan OFF)
    Set(enable_sort OFF)
    Set(enable_incremental_sort OFF)
    Set(enable_hashjoin OFF)
    Set(enable_mergejoin OFF)
    Set(enable_material OFF)
    Set(plan_cache_mode force_generic_plan)
*/
WITH
    q1 AS (SELECT id, ts, table_id, json_value, deleted, prev_ts, $3::BIGINT as query_ts FROM @db_name.documents WHERE table_id = $1 AND id = $2 and ts < $3 ORDER BY ts DESC LIMIT 1),
    q2 AS (SELECT id, ts, table_id, json_value, deleted, prev_ts, $6::BIGINT as query_ts FROM @db_name.documents WHERE table_id = $4 AND id = $5 and ts < $6 ORDER BY ts DESC LIMIT 1),
    q3 AS (SELECT id, ts, table_id, json_value, deleted, prev_ts, $9::BIGINT as query_ts FROM @db_name.documents WHERE table_id = $7 AND id = $8 and ts < $9 ORDER BY ts DESC LIMIT 1),
    q4 AS (SELECT id, ts, table_id, json_value, deleted, prev_ts, $12::BIGINT as query_ts FROM @db_name.documents WHERE table_id = $10 AND id = $11 and ts < $12 ORDER BY ts DESC LIMIT 1),
    q5 AS (SELECT id, ts, table_id, json_value, deleted, prev_ts, $15::BIGINT as query_ts FROM @db_name.documents WHERE table_id = $13 AND id = $14 and ts < $15 ORDER BY ts DESC LIMIT 1),
    q6 AS (SELECT id, ts, table_id, json_value, deleted, prev_ts, $18::BIGINT as query_ts FROM @db_name.documents WHERE table_id = $16 AND id = $17 and ts < $18 ORDER BY ts DESC LIMIT 1),
    q7 AS (SELECT id, ts, table_id, json_value, deleted, prev_ts, $21::BIGINT as query_ts FROM @db_name.documents WHERE table_id = $19 AND id = $20 and ts < $21 ORDER BY ts DESC LIMIT 1),
    q8 AS (SELECT id, ts, table_id, json_value, deleted, prev_ts, $24::BIGINT as query_ts FROM @db_name.documents WHERE table_id = $22 AND id = $23 and ts < $24 ORDER BY ts DESC LIMIT 1)
SELECT id, ts, table_id, json_value, deleted, prev_ts, query_ts FROM q1
UNION ALL SELECT id, ts, table_id, json_value, deleted, prev_ts, query_ts FROM q2
UNION ALL SELECT id, ts, table_id, json_value, deleted, prev_ts, query_ts FROM q3
UNION ALL SELECT id, ts, table_id, json_value, deleted, prev_ts, query_ts FROM q4
UNION ALL SELECT id, ts, table_id, json_value, deleted, prev_ts, query_ts FROM q5
UNION ALL SELECT id, ts, table_id, json_value, deleted, prev_ts, query_ts FROM q6
UNION ALL SELECT id, ts, table_id, json_value, deleted, prev_ts, query_ts FROM q7
UNION ALL SELECT id, ts, table_id, json_value, deleted, prev_ts, query_ts FROM q8;
"#;

const PREV_REV: &str = r#"
/*+
    Set(enable_seqscan OFF)
    Set(enable_sort OFF)
    Set(enable_incremental_sort OFF)
    Set(enable_hashjoin OFF)
    Set(enable_mergejoin OFF)
    Set(enable_material OFF)
    Set(plan_cache_mode force_generic_plan)
*/
SELECT id, ts, table_id, json_value, deleted, prev_ts, $3::BIGINT as query_ts
FROM @db_name.documents
WHERE
    table_id = $1 AND
    id = $2 AND
    ts < $3
ORDER BY ts desc
LIMIT 1
"#;

const EXACT_REV_CHUNK: &str = r#"
/*+
    Set(enable_seqscan OFF)
    Set(enable_sort OFF)
    Set(enable_incremental_sort OFF)
    Set(enable_hashjoin OFF)
    Set(enable_mergejoin OFF)
    Set(enable_material OFF)
    Set(plan_cache_mode force_generic_plan)
*/
WITH
    q1 AS (SELECT id, ts, table_id, json_value, deleted, prev_ts, $4::BIGINT as query_ts FROM @db_name.documents WHERE table_id = $1 AND id = $2 and ts = $3),
    q2 AS (SELECT id, ts, table_id, json_value, deleted, prev_ts, $8::BIGINT as query_ts FROM @db_name.documents WHERE table_id = $5 AND id = $6 and ts = $7),
    q3 AS (SELECT id, ts, table_id, json_value, deleted, prev_ts, $12::BIGINT as query_ts FROM @db_name.documents WHERE table_id = $9 AND id = $10 and ts = $11),
    q4 AS (SELECT id, ts, table_id, json_value, deleted, prev_ts, $16::BIGINT as query_ts FROM @db_name.documents WHERE table_id = $13 AND id = $14 and ts = $15),
    q5 AS (SELECT id, ts, table_id, json_value, deleted, prev_ts, $20::BIGINT as query_ts FROM @db_name.documents WHERE table_id = $17 AND id = $18 and ts = $19),
    q6 AS (SELECT id, ts, table_id, json_value, deleted, prev_ts, $24::BIGINT as query_ts FROM @db_name.documents WHERE table_id = $21 AND id = $22 and ts = $23),
    q7 AS (SELECT id, ts, table_id, json_value, deleted, prev_ts, $28::BIGINT as query_ts FROM @db_name.documents WHERE table_id = $25 AND id = $26 and ts = $27),
    q8 AS (SELECT id, ts, table_id, json_value, deleted, prev_ts, $32::BIGINT as query_ts FROM @db_name.documents WHERE table_id = $29 AND id = $30 and ts = $31)
SELECT id, ts, table_id, json_value, deleted, prev_ts, query_ts FROM q1
UNION ALL SELECT id, ts, table_id, json_value, deleted, prev_ts, query_ts FROM q2
UNION ALL SELECT id, ts, table_id, json_value, deleted, prev_ts, query_ts FROM q3
UNION ALL SELECT id, ts, table_id, json_value, deleted, prev_ts, query_ts FROM q4
UNION ALL SELECT id, ts, table_id, json_value, deleted, prev_ts, query_ts FROM q5
UNION ALL SELECT id, ts, table_id, json_value, deleted, prev_ts, query_ts FROM q6
UNION ALL SELECT id, ts, table_id, json_value, deleted, prev_ts, query_ts FROM q7
UNION ALL SELECT id, ts, table_id, json_value, deleted, prev_ts, query_ts FROM q8;
"#;

const EXACT_REV: &str = r#"
/*+
    Set(enable_seqscan OFF)
    Set(enable_sort OFF)
    Set(enable_incremental_sort OFF)
    Set(enable_hashjoin OFF)
    Set(enable_mergejoin OFF)
    Set(enable_material OFF)
    Set(plan_cache_mode force_generic_plan)
*/
SELECT id, ts, table_id, json_value, deleted, prev_ts, $4::BIGINT as query_ts
FROM @db_name.documents
WHERE
    table_id = $1 AND
    id = $2 AND
    ts = $3
"#;

// N.B.: tokio-postgres doesn't know how to create regclass values
const TABLE_SIZE_QUERY: &str =
    r"SELECT pg_table_size($1::text::regclass), pg_indexes_size($1::text::regclass)";

static MIN_SHA256: LazyLock<Vec<u8>> = LazyLock::new(|| vec![0; 32]);
static MAX_SHA256: LazyLock<Vec<u8>> = LazyLock::new(|| vec![255; 32]);

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
            sha256: MIN_SHA256.clone(),
        }
    }

    fn max_with_same_prefix(key: Vec<u8>) -> Self {
        let key = SplitKey::new(key);
        Self {
            prefix: key.prefix,
            sha256: MAX_SHA256.clone(),
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
) -> (&'static str, Vec<Param>) {
    let mut params: Vec<Param> = vec![
        internal_id_param(index_id),
        Param::Ts(read_timestamp.into()),
    ];

    let mut map_bound = |b: Bound<SqlKey>| -> BoundType {
        match b {
            Bound::Unbounded => BoundType::Unbounded,
            Bound::Excluded(sql_key) => {
                params.push(Param::Bytes(sql_key.prefix));
                params.push(Param::Bytes(sql_key.sha256));
                BoundType::Excluded
            },
            Bound::Included(sql_key) => {
                params.push(Param::Bytes(sql_key.prefix));
                params.push(Param::Bytes(sql_key.sha256));
                BoundType::Included
            },
        }
    };

    let lt = map_bound(lower);
    let ut = map_bound(upper);
    params.push(Param::Limit(batch_size as i64));

    let query = INDEX_QUERIES.get(&(lt, ut, order)).unwrap();
    (query, params)
}

#[cfg(any(test, feature = "testing"))]
pub mod itest {
    use std::path::Path;

    use anyhow::Context;
    use rand::Rng;

    // Returns a url to connect to the test cluster. The URL includes username and
    // password but no dbname.
    pub fn cluster_opts() -> String {
        let (host_port, username, password) = if Path::new("/convex.ro").exists() {
            // itest
            (
                "postgres:5432".to_owned(),
                "postgres".to_owned(),
                "alpastor".to_owned(),
            )
        } else {
            // local
            let user = std::env::var("USER").unwrap();
            let pguser = std::env::var("CI_PGUSER").unwrap_or(user);
            let pgpassword = std::env::var("CI_PGPASSWORD").unwrap_or_default();
            ("localhost".to_owned(), pguser, pgpassword)
        };
        format!("postgres://{username}:{password}@{host_port}")
    }

    /// Returns connection options for a guaranteed-fresh Postgres database.
    pub async fn new_db_opts() -> anyhow::Result<String> {
        let cluster_url = cluster_opts();

        // Connect using db `postgres`, create a fresh DB, and then return the
        // connection options for that one.
        let id: [u8; 16] = rand::rng().random();
        let db_name = "test_db_".to_string() + &hex::encode(&id[..]);

        let (client, conn) = tokio_postgres::connect(
            &format!("{cluster_url}/postgres"),
            tokio_postgres::tls::NoTls,
        )
        .await
        .context(format!("Couldn't connect to {cluster_url}"))?;
        common::runtime::tokio_spawn("postgres_conn", conn);
        let query = format!("CREATE DATABASE {db_name};");
        client.batch_execute(query.as_str()).await?;

        Ok(format!("{cluster_url}/{db_name}"))
    }
}

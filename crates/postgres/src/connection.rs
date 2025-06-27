//! Implements a Postgres connection pool and statement cache.
//!
//! Unlike deadpool-postgres, we:
//! - limit the number of cached prepared statements owned by each connection in
//!   order to avoid high/unbounded memory usage on the Postgres server
//! - automatically clean up idle connections.

use std::{
    collections::VecDeque,
    future,
    sync::{
        atomic::{
            self,
            AtomicBool,
        },
        Arc,
        LazyLock,
        Weak,
    },
    task::{
        ready,
        Poll,
    },
    time::Duration,
};

use ::metrics::StaticMetricLabel;
use anyhow::Context as _;
use cmd_util::env::env_config;
use common::{
    errors::report_error_sync,
    fastrace_helpers::FutureExt as _,
    knobs::{
        POSTGRES_INACTIVE_CONNECTION_LIFETIME,
        POSTGRES_MAX_CACHED_STATEMENTS,
        POSTGRES_MAX_CONNECTIONS,
    },
    pool_stats::{
        ConnectionPoolStats,
        ConnectionTracker,
    },
};
use fastrace::{
    future::FutureExt as _,
    Span,
};
use futures::{
    pin_mut,
    select_biased,
    Future,
    FutureExt as _,
    Stream,
    TryStreamExt,
};
use futures_async_stream::try_stream;
use lru::LruCache;
use metrics::Timer;
use parking_lot::Mutex;
use postgres_protocol::escape::escape_identifier;
use prometheus::VMHistogramVec;
use tokio::{
    sync::{
        oneshot,
        Semaphore,
        SemaphorePermit,
    },
    task::JoinHandle,
    time::{
        sleep,
        Instant,
    },
};
use tokio_postgres::{
    config::TargetSessionAttrs,
    types::{
        BorrowToSql,
        ToSql,
    },
    AsyncMessage,
    Row,
    RowStream,
    Statement,
    Transaction,
};
use tokio_postgres_rustls::MakeRustlsConnect;

use crate::metrics::{
    connection_lifetime_timer,
    get_connection_timer,
    log_execute,
    log_poisoned_connection,
    log_query,
    log_query_result,
    log_transaction,
    new_connection_pool_stats,
};

static POSTGRES_TIMEOUT: LazyLock<u64> =
    LazyLock::new(|| env_config("POSTGRES_TIMEOUT_SECONDS", 30));

#[derive(Debug, thiserror::Error)]
#[error("Postgres timeout")]
pub struct PostgresTimeout;

// We have observed postgres connections hanging during bootstrapping --
// which means backends can't start -- and during commit -- which means all
// future commits fail with OCC errors.
//
// To avoid these problems, wrap anything that talks to postgres in with_timeout
// which will panic, cleaning up all broken connections,
// if the future takes more than 2 minutes to complete.
pub(crate) async fn with_timeout<R, E, Fut: Future<Output = Result<R, E>>>(
    f: Fut,
) -> anyhow::Result<R>
where
    E: Into<anyhow::Error>,
{
    select_biased! {
        r = f.fuse() => {
            match r {
                Ok(r) => Ok(r),
                Err(e) => Err(e.into())
            }
        },
        _ = sleep(Duration::from_secs(*POSTGRES_TIMEOUT)).fuse() => {
            Err(anyhow::anyhow!(PostgresTimeout))
        },
    }
}

/// Stores the escaped form of a Postgres [schema]
///
/// [schema]: https://www.postgresql.org/docs/17/ddl-schemas.html
#[derive(Clone, Debug)]
pub(crate) struct SchemaName {
    pub(crate) escaped: String,
}

impl SchemaName {
    pub(crate) const EMPTY: SchemaName = SchemaName {
        escaped: String::new(),
    };

    pub fn new(s: &str) -> anyhow::Result<Self> {
        anyhow::ensure!(!s.starts_with("pg_"));
        anyhow::ensure!(!s.contains('\0'));
        Ok(Self {
            escaped: escape_identifier(s),
        })
    }
}

type StatementCache = LruCache<String, tokio_postgres::Statement>;
/// A Postgres connection, owned by either the connection pool
/// ([`ConvexPgPool`]), or by an active connection ([`PostgresConnection`]).
struct PooledConnection {
    client: tokio_postgres::Client,
    statement_cache: Mutex<StatementCache>,
    last_used: Instant,
}

async fn prepare_cached(
    client: &tokio_postgres::Client,
    cache: &Mutex<StatementCache>,
    statement: String,
) -> anyhow::Result<tokio_postgres::Statement> {
    if let Some(prepared) = cache.lock().get(&statement) {
        return Ok(prepared.clone());
    }
    let prepared = client.prepare(&statement).await?;
    // N.B.: if the cache is at capacity, this will drop the oldest statement,
    // which will send a message on the connection asking to deallocate it
    cache.lock().put(statement, prepared.clone());
    Ok(prepared)
}

impl PooledConnection {
    fn new(client: tokio_postgres::Client) -> Self {
        Self {
            client,
            statement_cache: Mutex::new(LruCache::new(*POSTGRES_MAX_CACHED_STATEMENTS)),
            last_used: Instant::now(),
        }
    }
}

/// An active Postgres connection from a [`ConvexPgPool`].
///
/// Returns the underlying connection to the pool when dropped (unless
/// `self.poisoned` is true).
pub(crate) struct PostgresConnection<'a> {
    pool: &'a ConvexPgPool,
    _permit: SemaphorePermit<'a>,
    conn: Option<PooledConnection>,
    poisoned: AtomicBool,
    schema: &'a SchemaName,
    labels: Vec<StaticMetricLabel>,
    _tracker: ConnectionTracker,
    _timer: Timer<VMHistogramVec>,
}

fn handle_error(poisoned: &AtomicBool, e: impl Into<anyhow::Error>) -> anyhow::Error {
    let e: anyhow::Error = e.into();
    if e.downcast_ref::<tokio_postgres::Error>()
        .is_some_and(|e| e.is_closed() || e.to_string().contains("unexpected message from server"))
        || e.downcast_ref::<PostgresTimeout>().is_some()
    {
        tracing::error!("Not reusing connection after error: {e:#}");
        poisoned.store(true, atomic::Ordering::Relaxed);
    }
    e
}

impl PostgresConnection<'_> {
    fn substitute_db_name(&self, query: &'static str) -> String {
        query.replace("@db_name", &self.schema.escaped)
    }

    fn conn(&self) -> &PooledConnection {
        self.conn
            .as_ref()
            .expect("connection is only taken in Drop")
    }

    pub async fn batch_execute(&self, query: &'static str) -> anyhow::Result<()> {
        log_execute(self.labels.clone());
        let query = self.substitute_db_name(query);
        self.conn()
            .client
            .batch_execute(&query)
            .await
            .map_err(|e| handle_error(&self.poisoned, e))
    }

    pub async fn query_opt(
        &self,
        statement: &'static str,
        params: &[&(dyn ToSql + Sync)],
    ) -> anyhow::Result<Option<Row>> {
        log_query(self.labels.clone());
        let query = self.substitute_db_name(statement);
        let row = with_timeout(self.conn().client.query_opt(&query, params))
            .await
            .map_err(|e| handle_error(&self.poisoned, e))?;
        if let Some(row) = &row {
            log_query_result(row, self.labels.clone());
        }
        Ok(row)
    }

    pub async fn prepare_cached(&self, query: &'static str) -> anyhow::Result<Statement> {
        let conn = self.conn();
        with_timeout(prepare_cached(
            &conn.client,
            &conn.statement_cache,
            self.substitute_db_name(query),
        ))
        .await
        .map_err(|e| handle_error(&self.poisoned, e))
    }

    pub async fn query_raw<P, I>(
        &self,
        statement: &Statement,
        params: I,
    ) -> anyhow::Result<impl Stream<Item = anyhow::Result<Row>>>
    where
        P: BorrowToSql,
        I: IntoIterator<Item = P>,
        I::IntoIter: ExactSizeIterator,
    {
        let labels = self.labels.clone();
        log_query(labels.clone());
        let stream = with_timeout(self.conn().client.query_raw(statement, params))
            .await
            .map_err(|e| handle_error(&self.poisoned, e))?;
        Ok(Self::wrap_query_stream(stream, labels))
    }

    #[allow(clippy::needless_lifetimes)]
    #[try_stream(ok = Row, error = anyhow::Error)]
    async fn wrap_query_stream(
        stream: impl Stream<Item = <RowStream as Stream>::Item>,
        labels: Vec<StaticMetricLabel>,
    ) {
        pin_mut!(stream);
        while let Some(row) = with_timeout(stream.try_next()).await? {
            log_query_result(&row, labels.clone());
            yield row;
        }
    }

    pub async fn execute(
        &self,
        statement: &Statement,
        params: &[&(dyn ToSql + Sync)],
    ) -> anyhow::Result<u64> {
        log_execute(self.labels.clone());
        with_timeout(self.conn().client.execute(statement, params))
            .await
            .map_err(|e| handle_error(&self.poisoned, e))
    }

    pub async fn transaction(&mut self) -> anyhow::Result<PostgresTransaction<'_>> {
        log_transaction(self.labels.clone());
        let conn = self
            .conn
            .as_mut()
            .expect("connection is only taken in Drop");
        let inner = match with_timeout(conn.client.transaction()).await {
            Ok(t) => t,
            Err(e) => return Err(handle_error(&self.poisoned, e)),
        };
        Ok(PostgresTransaction {
            inner,
            statement_cache: &conn.statement_cache,
            poisoned: &self.poisoned,
            schema: self.schema,
        })
    }
}

impl Drop for PostgresConnection<'_> {
    fn drop(&mut self) {
        if *self.poisoned.get_mut() {
            // We log here (not at poison time) in case the same connection is
            // poisoned more than once.
            log_poisoned_connection();
            return;
        }
        let mut conn = self.conn.take().expect("connection is only taken in Drop");
        conn.last_used = Instant::now();
        let mut idle_conns = self.pool.connections.lock();
        // don't return connections to a closed pool
        if !self.pool.semaphore.is_closed() {
            idle_conns.push_back(conn);
        }
    }
}

/// Represents an active transaction on a [`PostgresConnection`].
pub struct PostgresTransaction<'a> {
    inner: Transaction<'a>,
    statement_cache: &'a Mutex<StatementCache>,
    schema: &'a SchemaName,
    poisoned: &'a AtomicBool,
}

impl PostgresTransaction<'_> {
    fn substitute_db_name(&self, query: &'static str) -> String {
        query.replace("@db_name", &self.schema.escaped)
    }

    pub async fn prepare_cached(&self, query: &'static str) -> anyhow::Result<Statement> {
        with_timeout(prepare_cached(
            self.inner.client(),
            self.statement_cache,
            self.substitute_db_name(query),
        ))
        .await
        .map_err(|e| handle_error(self.poisoned, e))
    }

    pub async fn query(
        &self,
        statement: &Statement,
        params: &[&(dyn ToSql + Sync)],
    ) -> anyhow::Result<Vec<Row>> {
        with_timeout(self.inner.query(statement, params))
            .await
            .map_err(|e| handle_error(self.poisoned, e))
    }

    pub async fn execute_str(
        &self,
        statement: &'static str,
        params: &[&(dyn ToSql + Sync)],
    ) -> anyhow::Result<u64> {
        with_timeout(
            self.inner
                .execute(&self.substitute_db_name(statement), params),
        )
        .await
        .map_err(|e| handle_error(self.poisoned, e))
    }

    pub async fn execute_raw<P, I>(&self, statement: &Statement, params: I) -> anyhow::Result<u64>
    where
        P: BorrowToSql,
        I: IntoIterator<Item = P>,
        I::IntoIter: ExactSizeIterator,
    {
        with_timeout(self.inner.execute_raw(statement, params))
            .await
            .map_err(|e| handle_error(self.poisoned, e))
    }

    pub async fn commit(self) -> anyhow::Result<()> {
        with_timeout(self.inner.commit())
            .await
            .map_err(|e| handle_error(self.poisoned, e))
    }
}

/// A Postgres connection pool.
///
/// This struct is always used behind an `Arc`.
pub struct ConvexPgPool {
    pg_config: tokio_postgres::Config,
    tls_connect: MakeRustlsConnect,
    /// Limits the total number of connections that can be handed out
    /// simultaneously.
    semaphore: Semaphore,
    /// Idle connections, ordered by `last_used` from oldest to newest
    connections: Mutex<VecDeque<PooledConnection>>,
    stats: ConnectionPoolStats,
    idle_worker: JoinHandle<()>,
}

impl ConvexPgPool {
    pub(crate) fn new(
        pg_config: tokio_postgres::Config,
        tls_connect: MakeRustlsConnect,
    ) -> Arc<Self> {
        let max_size = *POSTGRES_MAX_CONNECTIONS;
        tracing::info!("Postgres connection pool max size {max_size}");
        // The idle worker needs a (weak) reference to the created ConvexPgPool,
        // but the pool also wants a reference to the worker; resolve this
        // cyclic situation by sneaking the weak reference through a channel.
        let (this_tx, this_rx) = oneshot::channel();
        let idle_worker = common::runtime::tokio_spawn("postgres_idle_worker", async move {
            Self::idle_worker(this_rx.await.expect("nothing sent on this_tx?")).await
        });
        let this = Arc::new(ConvexPgPool {
            pg_config,
            tls_connect,
            semaphore: Semaphore::new(max_size),
            connections: Mutex::new(VecDeque::new()),
            stats: new_connection_pool_stats(""),
            idle_worker,
        });
        _ = this_tx.send(Arc::downgrade(&this));
        this
    }

    /// Returns whether the pool is configured to connect to a leader database
    /// only.
    pub(crate) fn is_leader_only(&self) -> bool {
        self.pg_config.get_target_session_attrs() == TargetSessionAttrs::ReadWrite
    }

    /// Assumes that we already have a semaphore permit
    async fn get_connection_internal(&self) -> anyhow::Result<PooledConnection> {
        {
            let mut conns = self.connections.lock();
            // Always reuse the newest connection
            while let Some(conn) = conns.pop_back() {
                if conn.client.is_closed() {
                    continue;
                }
                return Ok(conn);
            }
        }
        let (client, mut conn) = self
            .pg_config
            .connect(self.tls_connect.clone())
            .in_span(Span::enter_with_local_parent("postgres_connect"))
            .await?;
        common::runtime::tokio_spawn(
            "postgres_connection",
            future::poll_fn(move |cx| loop {
                match ready!(conn.poll_message(cx)) {
                    Some(Ok(AsyncMessage::Notice(notice))) => {
                        tracing::info!("{}: {}", notice.severity(), notice.message());
                    },
                    Some(Ok(msg)) => {
                        // This is unexpected; the only other message type is a
                        // Notification and we don't use LISTEN
                        tracing::warn!("unexpected message: {:?}", msg);
                    },
                    Some(Err(e)) => {
                        tracing::error!("connection error: {e}");
                        report_error_sync(&mut e.into());
                        return Poll::Ready(());
                    },
                    None => return Poll::Ready(()),
                }
            }),
        );
        Ok(PooledConnection::new(client))
    }

    pub(crate) async fn get_connection<'a>(
        &'a self,
        name: &'static str,
        schema: &'a SchemaName,
    ) -> anyhow::Result<PostgresConnection<'a>> {
        let pool_get_timer = get_connection_timer();
        let conn = with_timeout(async {
            let permit = self
                .semaphore
                .acquire()
                .trace_if_pending("postgres_semaphore_acquire")
                .await
                .context("ConvexPgPool has been shut down")?;
            let conn = self.get_connection_internal().await?;
            anyhow::Ok((permit, conn))
        })
        .await;
        pool_get_timer.finish(conn.is_ok());
        let (permit, conn) = conn?;
        Ok(PostgresConnection {
            pool: self,
            _permit: permit,
            conn: Some(conn),
            poisoned: AtomicBool::new(false),
            schema,
            labels: vec![StaticMetricLabel::new("name", name)],
            _tracker: ConnectionTracker::new(&self.stats),
            _timer: connection_lifetime_timer(name),
        })
    }

    /// Drops all pooled connections and prevents the creation of new ones.
    pub fn shutdown(&self) {
        // N.B.: this doesn't abort in-progress connections, but they won't be
        // returned to the pool on drop
        self.semaphore.close();
        self.connections.lock().clear();
        self.idle_worker.abort();
    }

    async fn idle_worker(this: Weak<Self>) {
        loop {
            let oldest = if let Some(this) = this.upgrade() {
                this.cleanup_idle_connections()
            } else {
                break;
            };
            let next_wakeup =
                oldest.unwrap_or_else(Instant::now) + *POSTGRES_INACTIVE_CONNECTION_LIFETIME;
            tokio::time::sleep_until(next_wakeup).await;
        }
    }

    // Returns the last_used time of the oldest connection
    fn cleanup_idle_connections(&self) -> Option<Instant> {
        let mut connections = self.connections.lock();
        while let Some(c) = connections.front()
            && c.last_used.elapsed() > *POSTGRES_INACTIVE_CONNECTION_LIFETIME
        {
            connections.pop_front();
        }
        connections.front().map(|c| c.last_used)
    }
}

impl Drop for ConvexPgPool {
    fn drop(&mut self) {
        self.idle_worker.abort();
    }
}

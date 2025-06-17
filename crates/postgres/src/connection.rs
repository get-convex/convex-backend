use std::{
    collections::VecDeque,
    sync::{
        Arc,
        LazyLock,
        Weak,
    },
    time::Duration,
};

use ::metrics::StaticMetricLabel;
use anyhow::Context as _;
use cmd_util::env::env_config;
use common::{
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
    future::BoxFuture,
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
    tls::{
        MakeTlsConnect,
        TlsConnect,
    },
    types::{
        BorrowToSql,
        ToSql,
    },
    Row,
    RowStream,
    Socket,
    Statement,
    Transaction,
};

use crate::metrics::{
    connection_lifetime_timer,
    get_connection_timer,
    log_execute,
    log_query,
    log_query_result,
    log_transaction,
    new_connection_pool_stats,
};

static POSTGRES_TIMEOUT: LazyLock<u64> =
    LazyLock::new(|| env_config("POSTGRES_TIMEOUT_SECONDS", 30));

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
        _ = sleep(Duration::from_secs(*POSTGRES_TIMEOUT)).fuse() => Err(anyhow::anyhow!("Postgres timeout")),
    }
}

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
struct PooledConnection {
    client: tokio_postgres::Client,
    statement_cache: Mutex<StatementCache>,
    last_used: Instant,
}

async fn prepare(
    prepare: impl AsyncFnOnce(&str) -> Result<tokio_postgres::Statement, tokio_postgres::Error>,
    cache: &Mutex<StatementCache>,
    statement: String,
) -> anyhow::Result<tokio_postgres::Statement> {
    if let Some(prepared) = cache.lock().get(&statement) {
        return Ok(prepared.clone());
    }
    let prepared = prepare(&statement).await?;
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

    async fn prepare_cached(&self, query: String) -> anyhow::Result<tokio_postgres::Statement> {
        let client = &self.client;
        prepare(
            async |query| client.prepare(query).await,
            &self.statement_cache,
            query,
        )
        .await
    }
}

pub(crate) struct PostgresConnection<'a> {
    pool: &'a ConvexPgPool,
    _permit: SemaphorePermit<'a>,
    conn: Option<PooledConnection>,
    schema: &'a SchemaName,
    labels: Vec<StaticMetricLabel>,
    _tracker: ConnectionTracker,
    _timer: Timer<VMHistogramVec>,
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
        Ok(self
            .conn()
            .client
            .batch_execute(&self.substitute_db_name(query))
            .await?)
    }

    pub async fn query_opt(
        &self,
        statement: &'static str,
        params: &[&(dyn ToSql + Sync)],
    ) -> anyhow::Result<Option<Row>> {
        log_query(self.labels.clone());
        let row = with_timeout(
            self.conn()
                .client
                .query_opt(&self.substitute_db_name(statement), params),
        )
        .await?;
        if let Some(row) = &row {
            log_query_result(row, self.labels.clone());
        }
        Ok(row)
    }

    pub async fn prepare_cached(&self, query: &'static str) -> anyhow::Result<Statement> {
        with_timeout(self.conn().prepare_cached(self.substitute_db_name(query))).await
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
        let stream = with_timeout(self.conn().client.query_raw(statement, params)).await?;
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
        with_timeout(self.conn().client.execute(statement, params)).await
    }

    pub async fn transaction(&mut self) -> anyhow::Result<PostgresTransaction<'_>> {
        log_transaction(self.labels.clone());
        let conn = self
            .conn
            .as_mut()
            .expect("connection is only taken in Drop");
        let inner = with_timeout(conn.client.transaction()).await?;
        Ok(PostgresTransaction {
            inner,
            statement_cache: &conn.statement_cache,
            schema: self.schema,
        })
    }
}

impl Drop for PostgresConnection<'_> {
    fn drop(&mut self) {
        let mut conn = self.conn.take().expect("connection is only taken in Drop");
        conn.last_used = Instant::now();
        let mut idle_conns = self.pool.connections.lock();
        // don't return connections to a closed pool
        if !self.pool.semaphore.is_closed() {
            idle_conns.push_back(conn);
        }
    }
}

pub struct PostgresTransaction<'a> {
    inner: Transaction<'a>,
    statement_cache: &'a Mutex<StatementCache>,
    schema: &'a SchemaName,
}

impl PostgresTransaction<'_> {
    fn substitute_db_name(&self, query: &'static str) -> String {
        query.replace("@db_name", &self.schema.escaped)
    }

    pub async fn prepare_cached(&self, query: &'static str) -> anyhow::Result<Statement> {
        with_timeout(prepare(
            async |query| self.inner.prepare(query).await,
            self.statement_cache,
            self.substitute_db_name(query),
        ))
        .await
    }

    pub async fn query(
        &self,
        statement: &Statement,
        params: &[&(dyn ToSql + Sync)],
    ) -> anyhow::Result<Vec<Row>> {
        with_timeout(self.inner.query(statement, params)).await
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
    }

    pub async fn execute_raw<P, I>(&self, statement: &Statement, params: I) -> anyhow::Result<u64>
    where
        P: BorrowToSql,
        I: IntoIterator<Item = P>,
        I::IntoIter: ExactSizeIterator,
    {
        with_timeout(self.inner.execute_raw(statement, params)).await
    }

    pub async fn commit(self) -> anyhow::Result<()> {
        with_timeout(self.inner.commit()).await
    }
}

pub struct ConvexPgPool {
    pg_config: tokio_postgres::Config,
    connector: Box<
        dyn for<'a> Fn(
                &'a tokio_postgres::Config,
            ) -> BoxFuture<'a, anyhow::Result<tokio_postgres::Client>>
            + Send
            + Sync,
    >,
    /// Limits the total number of connections that can be handed out
    /// simultaneously.
    semaphore: Semaphore,
    /// Idle connections, ordered by `last_used` from oldest to newest
    connections: Mutex<VecDeque<PooledConnection>>,
    stats: ConnectionPoolStats,
    worker: JoinHandle<()>,
}

impl ConvexPgPool {
    pub(crate) fn new<T: MakeTlsConnect<Socket> + Clone + Send + Sync + 'static>(
        pg_config: tokio_postgres::Config,
        connect: T,
    ) -> Arc<Self>
    where
        T::Stream: Send,
        T::TlsConnect: Send,
        <T::TlsConnect as TlsConnect<Socket>>::Future: Send,
    {
        let max_size = *POSTGRES_MAX_CONNECTIONS;
        tracing::info!("Postgres connection pool max size {max_size}");
        // The idle worker needs a (weak) reference to the created ConvexPgPool,
        // but the pool also wants a reference to the worker; resolve this
        // cyclic situation by sneaking the weak reference through a channel.
        let (this_tx, this_rx) = oneshot::channel();
        let worker = common::runtime::tokio_spawn("postgres_idle_worker", async move {
            Self::idle_worker(this_rx.await.expect("nothing sent on this_tx?")).await
        });
        let this = Arc::new(ConvexPgPool {
            pg_config,
            connector: Box::new(move |pg_config| {
                let f = pg_config.connect(connect.clone());
                async move {
                    let (client, conn) = f.await?;
                    common::runtime::tokio_spawn("postgres_connection", conn);
                    Ok(client)
                }
                .boxed()
            }),
            semaphore: Semaphore::new(max_size),
            connections: Mutex::new(VecDeque::new()),
            stats: new_connection_pool_stats(""),
            worker,
        });
        _ = this_tx.send(Arc::downgrade(&this));
        this
    }

    /// Returns whether the pool is configured to connect to a leader database
    /// only.
    pub(crate) fn is_leader_only(&self) -> bool {
        self.pg_config.get_target_session_attrs() == TargetSessionAttrs::ReadWrite
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
            {
                let mut conns = self.connections.lock();
                // Always reuse the newest connection
                while let Some(conn) = conns.pop_back() {
                    if conn.client.is_closed() {
                        continue;
                    }
                    return Ok((permit, conn));
                }
            }
            let client = (self.connector)(&self.pg_config)
                .in_span(Span::enter_with_local_parent("postgres_connect"))
                .await?;
            anyhow::Ok((permit, PooledConnection::new(client)))
        })
        .await;
        pool_get_timer.finish(conn.is_ok());
        let (permit, conn) = conn?;
        Ok(PostgresConnection {
            pool: self,
            _permit: permit,
            conn: Some(conn),
            schema,
            labels: vec![StaticMetricLabel::new("name", name)],
            _tracker: ConnectionTracker::new(&self.stats),
            _timer: connection_lifetime_timer(name),
        })
    }

    pub fn shutdown(&self) {
        // N.B.: this doesn't abort in-progress connections, but they won't be
        // returned to the pool on drop
        self.semaphore.close();
        self.connections.lock().clear();
        self.worker.abort();
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
        self.worker.abort();
    }
}

use std::{
    sync::LazyLock,
    time::Duration,
};

use ::metrics::StaticMetricLabel;
use cmd_util::env::env_config;
use common::pool_stats::{
    ConnectionPoolStats,
    ConnectionTracker,
};
use deadpool_postgres::{
    Status,
    Transaction,
};
use futures::{
    pin_mut,
    select_biased,
    Future,
    FutureExt,
    Stream,
    TryStreamExt,
};
use futures_async_stream::try_stream;
use metrics::Timer;
use prometheus::VMHistogramVec;
use tokio::time::sleep;
use tokio_postgres::{
    types::{
        BorrowToSql,
        ToSql,
    },
    Row,
    RowStream,
    Statement,
    ToStatement,
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

pub(crate) struct PostgresConnection {
    conn: deadpool_postgres::Object,
    labels: Vec<StaticMetricLabel>,
    _tracker: ConnectionTracker,
    _timer: Timer<VMHistogramVec>,
}

impl PostgresConnection {
    pub async fn batch_execute(&self, query: &str) -> anyhow::Result<()> {
        log_execute(self.labels.clone());
        Ok(self.conn.batch_execute(query).await?)
    }

    pub async fn query_opt<T>(
        &self,
        statement: &T,
        params: &[&(dyn ToSql + Sync)],
    ) -> anyhow::Result<Option<Row>>
    where
        T: ?Sized + ToStatement,
    {
        log_query(self.labels.clone());
        let row = with_timeout(self.conn.query_opt(statement, params)).await?;
        if let Some(row) = &row {
            log_query_result(row, self.labels.clone());
        }
        Ok(row)
    }

    pub async fn prepare_cached(&self, query: &'static str) -> anyhow::Result<Statement> {
        with_timeout(self.conn.prepare_cached(query)).await
    }

    pub async fn query_raw<T, P, I>(
        &self,
        statement: &T,
        params: I,
    ) -> anyhow::Result<impl Stream<Item = anyhow::Result<Row>>>
    where
        T: ?Sized + ToStatement,
        P: BorrowToSql,
        I: IntoIterator<Item = P>,
        I::IntoIter: ExactSizeIterator,
    {
        let labels = self.labels.clone();
        log_query(labels.clone());
        let stream = with_timeout(self.conn.query_raw(statement, params)).await?;
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

    pub async fn execute<T>(
        &self,
        statement: &T,
        params: &[&(dyn ToSql + Sync)],
    ) -> anyhow::Result<u64>
    where
        T: ?Sized + ToStatement,
    {
        log_execute(self.labels.clone());
        with_timeout(self.conn.execute(statement, params)).await
    }

    pub async fn transaction(&mut self) -> anyhow::Result<PostgresTransaction> {
        log_transaction(self.labels.clone());
        let inner = with_timeout(self.conn.transaction()).await?;
        Ok(PostgresTransaction { inner })
    }
}

pub struct PostgresTransaction<'a> {
    inner: Transaction<'a>,
}

impl PostgresTransaction<'_> {
    pub async fn prepare_cached(&self, query: &'static str) -> anyhow::Result<Statement> {
        with_timeout(self.inner.prepare_cached(query)).await
    }

    pub async fn query<T>(
        &self,
        statement: &T,
        params: &[&(dyn ToSql + Sync)],
    ) -> anyhow::Result<Vec<Row>>
    where
        T: ?Sized + ToStatement,
    {
        with_timeout(self.inner.query(statement, params)).await
    }

    pub async fn execute<T>(
        &self,
        statement: &T,
        params: &[&(dyn ToSql + Sync)],
    ) -> anyhow::Result<u64>
    where
        T: ?Sized + ToStatement,
    {
        with_timeout(self.inner.execute(statement, params)).await
    }

    pub async fn execute_raw<P, I, T>(&self, statement: &T, params: I) -> anyhow::Result<u64>
    where
        T: ?Sized + ToStatement,
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

#[derive(Clone)]
pub struct ConvexPgPool {
    inner: deadpool_postgres::Pool,
    stats: ConnectionPoolStats,
}

impl ConvexPgPool {
    pub(crate) fn new(pool: deadpool_postgres::Pool) -> Self {
        ConvexPgPool {
            inner: pool,
            stats: new_connection_pool_stats(""),
        }
    }

    pub(crate) async fn get_connection(
        &self,
        name: &'static str,
    ) -> anyhow::Result<PostgresConnection> {
        let pool_get_timer = get_connection_timer();
        let conn = with_timeout(self.inner.get()).await;
        pool_get_timer.finish(conn.is_ok());
        Ok(PostgresConnection {
            conn: conn?,
            labels: vec![StaticMetricLabel::new("name", name)],
            _tracker: ConnectionTracker::new(&self.stats),
            _timer: connection_lifetime_timer(name),
        })
    }

    pub(crate) fn status(&self) -> Status {
        self.inner.status()
    }
}

use std::{
    str::FromStr,
    time::Duration,
};

use ::metrics::StaticMetricLabel;
use common::{
    knobs::{
        MYSQL_INACTIVE_CONNECTION_LIFETIME,
        MYSQL_MAX_CONNECTIONS,
        MYSQL_MAX_CONNECTION_LIFETIME,
        MYSQL_TIMEOUT,
    },
    pool_stats::{
        ConnectionPoolStats,
        ConnectionTracker,
    },
    runtime::Runtime,
};
use dynfmt::{
    ArgumentSpec,
    Error,
    Format,
    FormatArgs,
    Position,
};
use errors::ErrorMetadata;
use futures::{
    pin_mut,
    select_biased,
    Future,
    FutureExt,
    Stream,
    StreamExt,
    TryStreamExt,
};
use futures_async_stream::try_stream;
use metrics::{
    ProgressCounter,
    Timer,
};
use mysql_async::{
    prelude::Queryable,
    DriverError,
    Opts,
    OptsBuilder,
    Params,
    Pool,
    PoolConstraints,
    PoolOpts,
    Row,
    TxOpts,
    Value as MySqlValue,
};
use prometheus::VMHistogramVec;
use tokio::time::sleep;
use url::Url;

use crate::metrics::{
    connection_lifetime_timer,
    get_connection_timer,
    log_execute,
    log_large_statement,
    log_query,
    log_query_result,
    log_transaction,
    new_connection_pool_stats,
    query_progress_counter,
    LARGE_STATEMENT_THRESHOLD,
};

// Guard against connections hanging during bootstrapping -- which means
// backends can't start -- and during commit -- which means all future commits
// fail with OCC errors.
//
// To avoid these problems, wrap anything that talks to mysql in with_timeout
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
                Err(e) => {
                    let e = e.into();
                    if e.chain().any(|cause| matches!(
                        cause.downcast_ref(),
                        Some(
                            mysql_async::Error::Driver(DriverError::PoolDisconnected)
                            | mysql_async::Error::Io(_)
                        )
                    )) {
                        Err(e.context(ErrorMetadata::operational_internal_server_error()))
                    } else {
                        Err(e)
                    }
                }
            }
        },
        _ = sleep(Duration::from_secs(*MYSQL_TIMEOUT)).fuse() => Err(
            anyhow::anyhow!("MySQL timeout").context(
                ErrorMetadata::operational_internal_server_error()
            )
        ),
    }
}

struct MySQLFormatArguments<'a> {
    db_name: &'a str,
    params: Vec<String>,
}

impl FormatArgs for MySQLFormatArguments<'_> {
    fn get_index(&self, index: usize) -> Result<Option<dynfmt::Argument<'_>>, ()> {
        self.params.get_index(index)
    }

    fn get_key(&self, key: &str) -> Result<Option<dynfmt::Argument<'_>>, ()> {
        if key != "db_name" {
            panic!("Unexpected named argument {key}");
        }
        Ok(Some(&self.db_name))
    }
}

const DB_NAME_ARGUMENT_PATTERN: &str = "@db_name";

// Formats both @db_name and ?
struct MySQLRawStatementFormat;

impl<'f> Format<'f> for MySQLRawStatementFormat {
    type Iter = impl Iterator<Item = Result<ArgumentSpec<'f>, Error<'f>>>;

    fn iter_args(&self, format: &'f str) -> Result<Self::Iter, Error<'f>> {
        let db_name_iter = format
            .match_indices(DB_NAME_ARGUMENT_PATTERN)
            .map(|(index, _)| {
                Ok(
                    ArgumentSpec::new(index, index + DB_NAME_ARGUMENT_PATTERN.len())
                        .with_position(Position::Key("db_name")),
                )
            });
        let args_iter = format
            .match_indices('?')
            .map(|(index, _)| Ok(ArgumentSpec::new(index, index + 1)));
        // The resulting iterator should be sorted.
        let mut args: Vec<_> = db_name_iter.chain(args_iter).collect();
        args.sort_by_key(|arg| match arg {
            Ok(arg) => arg.start(),
            Err(_) => 0,
        });
        Ok::<Self::Iter, _>(args.into_iter())
    }
}

// Formats a MySQL query with position parameters into a string, so it can be
// used with the text protocol.
fn format_mysql_text_protocol(
    db_name: &str,
    statement: &'static str,
    params: Vec<MySqlValue>,
    labels: &[StaticMetricLabel],
) -> anyhow::Result<String> {
    let args = MySQLFormatArguments {
        db_name,
        params: params
            .into_iter()
            .map(|p| match p {
                MySqlValue::NULL => "NULL".to_owned(),
                MySqlValue::Bytes(bytes) => format!("unhex('{}')", hex::encode(bytes)),
                MySqlValue::Int(i) => format!("{i}"),
                MySqlValue::UInt(u) => format!("{u}"),
                // We don't use the following and I don't want to deal with escaping them.
                MySqlValue::Float(_) => panic!("Float MySQL argument not supported"),
                MySqlValue::Double(_) => panic!("Double MySQL argument not supported"),
                MySqlValue::Date(..) => panic!("Date MySQL argument not supported"),
                MySqlValue::Time(..) => panic!("Time MySQL argument not supported"),
            })
            .collect(),
    };
    let result = MySQLRawStatementFormat.format(statement, args)?.to_string();
    if result.len() > LARGE_STATEMENT_THRESHOLD {
        log_large_statement(labels.to_vec());
    }
    Ok(result)
}

// Formats only @db_name
struct MySQLPreparedStatementFormat;

impl<'f> Format<'f> for MySQLPreparedStatementFormat {
    type Iter = impl Iterator<Item = Result<ArgumentSpec<'f>, Error<'f>>>;

    fn iter_args(&self, format: &'f str) -> Result<Self::Iter, Error<'f>> {
        Ok::<Self::Iter, _>(
            format
                .match_indices(DB_NAME_ARGUMENT_PATTERN)
                .map(|(index, _)| {
                    Ok(
                        ArgumentSpec::new(index, index + DB_NAME_ARGUMENT_PATTERN.len())
                            .with_position(Position::Key("db_name")),
                    )
                }),
        )
    }
}

// Formats a MySQL query by only replacing the @db_name but leaves positional
// arguments alone. To be used with MySQL binary protocol.
fn format_mysql_binary_protocol(db_name: &str, statement: &'static str) -> anyhow::Result<String> {
    let args = MySQLFormatArguments {
        db_name,
        params: vec![], // No positional arguments.
    };
    Ok(MySQLPreparedStatementFormat
        .format(statement, args)?
        .to_string())
}

pub(crate) struct MySqlConnection<'a> {
    conn: mysql_async::Conn,
    labels: Vec<StaticMetricLabel>,
    use_prepared_statements: bool,
    db_name: &'a str,
    _tracker: ConnectionTracker,
    _timer: Timer<VMHistogramVec>,
}

impl MySqlConnection<'_> {
    /// Executes multiple statements, separated by semicolons.
    #[minitrace::trace]
    pub async fn execute_many(&mut self, query: &'static str) -> anyhow::Result<()> {
        log_execute(self.labels.clone());
        let statement = format_mysql_text_protocol(self.db_name, query, vec![], &self.labels)?;
        with_timeout(self.conn.query_iter(statement)).await?;
        Ok(())
    }

    /// Run a readonly query that returns one or zero results.
    #[minitrace::trace]
    pub async fn query_optional(
        &mut self,
        statement: &'static str,
        params: Vec<MySqlValue>,
    ) -> anyhow::Result<Option<Row>> {
        log_query(self.labels.clone());
        let future = if self.use_prepared_statements {
            let statement = format_mysql_binary_protocol(self.db_name, statement)?;
            self.conn.exec_first(statement, params)
        } else {
            let statement =
                format_mysql_text_protocol(self.db_name, statement, params, &self.labels)?;
            self.conn.query_first(statement)
        };
        let row = with_timeout(future).await?;
        if let Some(row) = &row {
            log_query_result(row, self.labels.clone());
        }
        Ok(row)
    }

    /// Run a readonly query that returns a stream of results.
    #[minitrace::trace]
    pub async fn query_stream(
        &mut self,
        statement: &'static str,
        params: Vec<MySqlValue>,
        size_hint: usize,
    ) -> anyhow::Result<impl Stream<Item = anyhow::Result<Row>> + '_> {
        let labels = self.labels.clone();
        // Any error or dropped stream after this point leaves the connection
        // open with MySQL sending data into it. In the worst case, the data
        // will be consumed & dropped by the *next* client.acquire(), which can
        // make it hard to attribute latency. Therefore we start a progress
        // counter that will log if the stream is dropped before being consumed.
        let progress_counter = query_progress_counter(size_hint, labels.clone());
        log_query(labels.clone());
        let stream = if self.use_prepared_statements {
            let statement = format_mysql_binary_protocol(self.db_name, statement)?;
            with_timeout(self.conn.exec_stream(statement, Params::Positional(params)))
                .await?
                .boxed()
        } else {
            let statement =
                format_mysql_text_protocol(self.db_name, statement, params, &self.labels)?;
            with_timeout(self.conn.query_stream(statement))
                .await?
                .boxed()
        };
        Ok(Self::wrap_query_stream(stream, progress_counter, labels))
    }

    #[allow(clippy::needless_lifetimes)]
    #[try_stream(ok = Row, error = anyhow::Error)]
    async fn wrap_query_stream(
        stream: impl Stream<Item = mysql_async::Result<Row>>,
        mut progress_counter: ProgressCounter,
        labels: Vec<StaticMetricLabel>,
    ) {
        pin_mut!(stream);
        while let Some(row) = with_timeout(stream.try_next()).await? {
            progress_counter.add_processed(1);
            log_query_result(&row, labels.clone());

            // The caller will likely consume this stream in a CPU-intensive
            // loop, to parse the rows. And `stream.try_next().await`
            // might not yield to tokio if the rows are all available at once.
            // Avoid long poll times by intentionally yielding.
            tokio::task::consume_budget().await;

            yield row;
        }
        progress_counter.complete();
    }

    /// Execute a SQL statement, returning the number of rows affected.
    #[minitrace::trace]
    pub async fn exec_iter(
        &mut self,
        statement: &'static str,
        params: Vec<MySqlValue>,
    ) -> anyhow::Result<u64> {
        log_execute(self.labels.clone());
        let affected_rows = if self.use_prepared_statements {
            let statement = format_mysql_binary_protocol(self.db_name, statement)?;
            with_timeout(self.conn.exec_iter(statement, Params::Positional(params)))
                .await?
                .affected_rows()
        } else {
            let statement =
                format_mysql_text_protocol(self.db_name, statement, params, &self.labels)?;
            with_timeout(self.conn.query_iter(statement))
                .await?
                .affected_rows()
        };
        Ok(affected_rows)
    }

    #[minitrace::trace]
    pub async fn transaction(&mut self) -> anyhow::Result<MySqlTransaction<'_>> {
        log_transaction(self.labels.clone());
        Ok(MySqlTransaction {
            inner: with_timeout(self.conn.start_transaction(TxOpts::new())).await?,
            use_prepared_statements: self.use_prepared_statements,
            db_name: self.db_name,
            labels: &self.labels,
        })
    }
}

pub(crate) struct MySqlTransaction<'a> {
    inner: mysql_async::Transaction<'a>,
    use_prepared_statements: bool,
    db_name: &'a str,
    labels: &'a [StaticMetricLabel],
}

impl MySqlTransaction<'_> {
    /// Executes the given statement and returns the first row of the first
    /// result set.
    pub async fn exec_first(
        &mut self,
        statement: &'static str,
        params: Vec<MySqlValue>,
    ) -> anyhow::Result<Option<Row>> {
        let future = if self.use_prepared_statements {
            let statement = format_mysql_binary_protocol(self.db_name, statement)?;
            self.inner.exec_first(statement, Params::Positional(params))
        } else {
            let statement =
                format_mysql_text_protocol(self.db_name, statement, params, self.labels)?;
            self.inner.query_first(statement)
        };
        with_timeout(future).await
    }

    /// Executes the given statement and drops the result.
    pub async fn exec_drop(
        &mut self,
        statement: &'static str,
        params: Vec<MySqlValue>,
    ) -> anyhow::Result<()> {
        let future = if self.use_prepared_statements {
            let statement = format_mysql_binary_protocol(self.db_name, statement)?;
            self.inner.exec_drop(statement, Params::Positional(params))
        } else {
            let statement =
                format_mysql_text_protocol(self.db_name, statement, params, self.labels)?;
            self.inner.query_drop(statement)
        };
        with_timeout(future).await
    }

    /// Execute a SQL statement, returning the number of rows affected.
    pub async fn exec_iter(
        &mut self,
        statement: &'static str,
        params: Vec<MySqlValue>,
    ) -> anyhow::Result<u64> {
        let affected_rows = if self.use_prepared_statements {
            let statement = format_mysql_binary_protocol(self.db_name, statement)?;
            with_timeout(self.inner.exec_iter(statement, Params::Positional(params)))
                .await?
                .affected_rows()
        } else {
            let statement =
                format_mysql_text_protocol(self.db_name, statement, params, self.labels)?;
            with_timeout(self.inner.query_iter(statement))
                .await?
                .affected_rows()
        };
        Ok(affected_rows)
    }

    pub async fn commit(self) -> anyhow::Result<()> {
        with_timeout(self.inner.commit()).await?;
        Ok(())
    }
}

pub struct ConvexMySqlPool<RT: Runtime> {
    pool: Pool,
    use_prepared_statements: bool,
    runtime: Option<RT>,
    stats: ConnectionPoolStats,
    // Used for metrics
    cluster_name: String,
}

// Deriving the cluster name from the URL is a bit hacky, but seems cleaner than
// to pass cluster_name from 7 layers deep just for metric. It is easy to
// confuse those with the url and db_name that are used in the actual queries.
fn derive_cluster_name(url: &Url) -> &str {
    let mut cluster_name = url
        .host_str()
        .and_then(|host| host.split('.').next())
        .unwrap_or("");
    if cluster_name.ends_with("-proxy") {
        cluster_name = cluster_name
            .strip_suffix("-proxy")
            .expect("Failed to strip -proxy suffix even though it exists")
    }
    cluster_name
}

impl<RT: Runtime> ConvexMySqlPool<RT> {
    pub fn new(
        url: &Url,
        use_prepared_statements: bool,
        runtime: Option<RT>,
    ) -> anyhow::Result<Self> {
        let cluster_name = derive_cluster_name(url).to_owned();
        // NOTE: the inactive_connection_ttl only applies to connections > min
        // constraint. So to make it apply to all connections, set min=0.
        // Connections are accessed in FIFO order from the pool (not round robin)
        // so the pool should be kept small by limiting inactive_connection_ttl.
        let constraints = PoolConstraints::new(0, *MYSQL_MAX_CONNECTIONS).unwrap();
        let pool_opts = PoolOpts::new()
            .with_constraints(constraints)
            .with_inactive_connection_ttl(*MYSQL_INACTIVE_CONNECTION_LIFETIME)
            .with_abs_conn_ttl(Some(*MYSQL_MAX_CONNECTION_LIFETIME))
            // Jitter max connection lifetime with 20%.
            .with_abs_conn_ttl_jitter(Some(*MYSQL_MAX_CONNECTION_LIFETIME / 5))
            .with_reset_connection(false); // persist prepared statements
        let opts = OptsBuilder::from_opts(Opts::from_str(url.as_ref())?).pool_opts(pool_opts);
        Ok(Self {
            pool: Pool::new(opts),
            use_prepared_statements,
            runtime,
            stats: new_connection_pool_stats(cluster_name.as_str()),
            cluster_name,
        })
    }

    #[minitrace::trace]
    pub(crate) async fn acquire<'a>(
        &self,
        name: &'static str,
        db_name: &'a str,
    ) -> anyhow::Result<MySqlConnection<'a>> {
        let pool_get_timer = get_connection_timer(&self.cluster_name);
        let conn = with_timeout(self.pool.get_conn()).await;
        pool_get_timer.finish(conn.is_ok());
        Ok(MySqlConnection {
            conn: conn?,
            labels: vec![
                StaticMetricLabel::new("name", name),
                StaticMetricLabel::new("cluster_name", self.cluster_name.clone()),
            ],
            use_prepared_statements: self.use_prepared_statements,
            db_name,
            _tracker: ConnectionTracker::new(&self.stats),
            _timer: connection_lifetime_timer(name, &self.cluster_name),
        })
    }

    pub fn cluster_name(&self) -> &str {
        &self.cluster_name
    }

    pub async fn shutdown(&self) -> anyhow::Result<()> {
        tracing::info!("Shutting down ConvexMySqlPool");
        Ok(self.pool.clone().disconnect().await?)
    }
}

impl<RT: Runtime> Drop for ConvexMySqlPool<RT> {
    fn drop(&mut self) {
        tracing::info!("ConvexMySqlPool dropped");
        let Some(runtime) = self.runtime.take() else {
            return;
        };
        let pool = self.pool.clone();
        runtime.spawn("mysql_pool_disconnect", async move {
            let _ = pool.disconnect().await;
            tracing::info!("ConvexMySqlPool pool successfully closed");
        });
    }
}

#[cfg(test)]
mod tests {
    use mysql_async::Value as MySqlValue;

    use crate::connection::{
        derive_cluster_name,
        format_mysql_binary_protocol,
        format_mysql_text_protocol,
    };

    #[test]
    fn test_format_mysql_text_protocol() -> anyhow::Result<()> {
        let encoded = format_mysql_text_protocol(
            "presley_db",
            r#"
    SELECT * FROM @db_name.indexes
    WHERE (key, value) IN (?, ?)
    AND deleted IS ?",
"#,
            vec![MySqlValue::from(-27), "!xa?)".into(), MySqlValue::NULL],
            &[],
        )?;
        assert_eq!(
            encoded,
            r#"
    SELECT * FROM presley_db.indexes
    WHERE (key, value) IN (-27, unhex('2178613f29'))
    AND deleted IS NULL",
"#,
        );
        Ok(())
    }

    #[test]
    fn test_format_mysql_binary_protocol() -> anyhow::Result<()> {
        let encoded = format_mysql_binary_protocol(
            "presley_db",
            r#"
    SELECT * FROM @db_name.indexes
    WHERE (key, value) IN (?, ?)
    AND deleted IS ?",
"#,
        )?;
        assert_eq!(
            encoded,
            r#"
    SELECT * FROM presley_db.indexes
    WHERE (key, value) IN (?, ?)
    AND deleted IS ?",
"#,
        );
        Ok(())
    }

    #[test]
    fn test_derive_cluster_name() -> anyhow::Result<()> {
        assert_eq!(
            derive_cluster_name(
                &"mysql://admin:pass@convex-customer-prod-762db212.cluster-ctfpoce735rh.us-east-1.\
                  rds.amazonaws.com?sslrequired=true"
                    .parse()?
            ),
            "convex-customer-prod-762db212"
        );
        assert_eq!(
            derive_cluster_name(
                &"mysql://admin:pass@convex-customer-prod-762db212-proxy.cluster-ctfpoce735rh.\
                  us-east-1.rds.amazonaws.com?sslrequired=true"
                    .parse()?
            ),
            "convex-customer-prod-762db212"
        );
        Ok(())
    }
}

use std::sync::Arc;

use anyhow::Context as _;
use clusters::{
    persistence_args_from_cluster_url,
    DbDriverTag,
    PersistenceArgs,
};
use common::{
    knobs::DATABASE_USE_PREPARED_STATEMENTS,
    persistence::{
        Persistence,
        PersistenceReader,
    },
    runtime::Runtime,
    shutdown::ShutdownSignal,
};
use mysql::{
    ConvexMySqlPool,
    MySqlOptions,
    MySqlPersistence,
    MySqlReaderOptions,
};
use postgres::{
    PostgresOptions,
    PostgresPersistence,
    PostgresReaderOptions,
};
use sqlite::SqlitePersistence;

pub async fn connect_persistence<RT: Runtime>(
    db: DbDriverTag,
    db_spec: &str,
    require_ssl: bool,
    allow_read_only: bool,
    instance_name: &str,
    runtime: RT,
    shutdown_signal: ShutdownSignal,
) -> anyhow::Result<Arc<dyn Persistence>> {
    let persistence: Arc<dyn Persistence> = match db {
        DbDriverTag::Sqlite => {
            let persistence = Arc::new(SqlitePersistence::new(db_spec, false)?);
            tracing::info!("Connected to SQLite at {db_spec}");
            persistence
        },
        DbDriverTag::Postgres(version)
        | DbDriverTag::PostgresMultiSchema(version)
        | DbDriverTag::PostgresAwsIam(version)
        | DbDriverTag::MySql(version)
        | DbDriverTag::MySqlAwsIam(version) => {
            let args = persistence_args_from_cluster_url(
                instance_name,
                db_spec.parse()?,
                db,
                require_ssl,
                true, /* require_leader */
            )?;
            match args {
                PersistenceArgs::Postgres { url, schema } => {
                    let options = PostgresOptions {
                        allow_read_only,
                        version,
                        schema,
                    };
                    let persistence = Arc::new(
                        PostgresPersistence::new(url.as_str(), options, shutdown_signal).await?,
                    );
                    tracing::info!("Connected to Postgres database: {}", instance_name);
                    persistence
                },
                PersistenceArgs::MySql { url, db_name } => {
                    let options = MySqlOptions {
                        allow_read_only,
                        version,
                        use_prepared_statements: *DATABASE_USE_PREPARED_STATEMENTS,
                    };
                    let persistence = Arc::new(
                        MySqlPersistence::new(
                            Arc::new(ConvexMySqlPool::new(
                                &url,
                                options.use_prepared_statements,
                                Some(runtime),
                            )?),
                            db_name.clone(),
                            options,
                            shutdown_signal,
                        )
                        .await?,
                    );
                    tracing::info!("Connected to MySQL database: {}", db_name);
                    persistence
                },
            }
        },
        #[cfg(any(test, feature = "testing"))]
        DbDriverTag::TestPersistence => {
            let persistence = Arc::new(common::testing::TestPersistence::new());
            tracing::info!("Connected to TestPersistence");
            persistence
        },
        #[cfg(not(any(test, feature = "testing")))]
        _ => unreachable!(),
    };
    Ok(persistence)
}

pub async fn connect_persistence_reader<RT: Runtime>(
    db: DbDriverTag,
    db_spec: &str,
    require_ssl: bool,
    db_should_be_leader: bool,
    instance_name: &str,
    runtime: RT,
) -> anyhow::Result<Arc<dyn PersistenceReader>> {
    let persistence: Arc<dyn PersistenceReader> = match db {
        DbDriverTag::Sqlite => Arc::new(SqlitePersistence::new(db_spec, false)?),
        DbDriverTag::Postgres(version)
        | DbDriverTag::PostgresMultiSchema(version)
        | DbDriverTag::PostgresAwsIam(version)
        | DbDriverTag::MySql(version)
        | DbDriverTag::MySqlAwsIam(version) => {
            let args = persistence_args_from_cluster_url(
                instance_name,
                db_spec.parse()?,
                db,
                require_ssl,
                db_should_be_leader,
            )?;
            match args {
                PersistenceArgs::Postgres { url, schema } => {
                    let options = PostgresReaderOptions { version, schema };
                    Arc::new(
                        PostgresPersistence::new_reader(
                            PostgresPersistence::create_pool(
                                url.as_str()
                                    .parse()
                                    .context("Invalid postgres cluster url")?,
                            )
                            .context("failed to create postgres pool")?,
                            options,
                        )
                        .await?,
                    )
                },
                PersistenceArgs::MySql { url, db_name } => {
                    let options = MySqlReaderOptions {
                        db_should_be_leader,
                        version,
                    };
                    Arc::new(MySqlPersistence::new_reader(
                        Arc::new(ConvexMySqlPool::new(
                            &url,
                            *DATABASE_USE_PREPARED_STATEMENTS,
                            Some(runtime),
                        )?),
                        db_name,
                        options,
                    ))
                },
            }
        },
        #[cfg(any(test, feature = "testing"))]
        DbDriverTag::TestPersistence => Arc::new(common::testing::TestPersistence::new()),
        #[cfg(not(any(test, feature = "testing")))]
        _ => unreachable!(),
    };
    Ok(persistence)
}

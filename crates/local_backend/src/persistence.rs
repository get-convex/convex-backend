use std::sync::Arc;

use clusters::{
    persistence_args_from_cluster_url,
    DbDriverTag,
};
use common::{
    knobs::DATABASE_USE_PREPARED_STATEMENTS,
    persistence::Persistence,
    shutdown::ShutdownSignal,
};
use mysql::{
    ConvexMySqlPool,
    MySqlOptions,
    MySqlPersistence,
};
use postgres::{
    PostgresOptions,
    PostgresPersistence,
};
use runtime::prod::ProdRuntime;
use sqlite::SqlitePersistence;

pub async fn connect_persistence(
    db: DbDriverTag,
    db_spec: &str,
    do_not_require_ssl: bool,
    instance_name: &str,
    runtime: ProdRuntime,
    shutdown_signal: ShutdownSignal,
) -> anyhow::Result<Arc<dyn Persistence>> {
    let require_ssl = !do_not_require_ssl;
    let persistence: Arc<dyn Persistence> = match db {
        DbDriverTag::Sqlite => {
            let persistence = Arc::new(SqlitePersistence::new(db_spec, false)?);
            tracing::info!("Connected to SQLite at {db_spec}");
            persistence
        },
        DbDriverTag::Postgres(version) | DbDriverTag::PostgresAwsIam(version) => {
            let options = PostgresOptions {
                allow_read_only: false,
                version,
            };
            let args = persistence_args_from_cluster_url(
                instance_name,
                db_spec.parse()?,
                db,
                require_ssl,
            )?;
            let persistence = Arc::new(PostgresPersistence::new(args.url.as_str(), options).await?);
            tracing::info!("Connected to Postgres database: {} ", args.db_name);
            persistence
        },
        DbDriverTag::MySql(version) | DbDriverTag::MySqlAwsIam(version) => {
            let options = MySqlOptions {
                allow_read_only: false,
                version,
                use_prepared_statements: *DATABASE_USE_PREPARED_STATEMENTS,
            };
            let args = persistence_args_from_cluster_url(
                instance_name,
                db_spec.parse()?,
                db,
                require_ssl,
            )?;
            let persistence = Arc::new(
                MySqlPersistence::new(
                    Arc::new(ConvexMySqlPool::new(
                        &args.url,
                        options.use_prepared_statements,
                        Some(runtime),
                    )?),
                    args.db_name.clone(),
                    options,
                    shutdown_signal,
                )
                .await?,
            );
            tracing::info!("Connected to MySQL database: {} ", args.db_name);
            persistence
        },
        #[cfg(any(test, feature = "testing"))]
        DbDriverTag::TestPersistence => {
            let persistence = Arc::new(common::testing::TestPersistence::new());
            tracing::info!("Connected to TestPersistence");
            persistence
        },
    };
    Ok(persistence)
}

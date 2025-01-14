use std::sync::Arc;

use clusters::{
    persistence_args_from_cluster_url,
    DbDriverTag,
};
use common::{
    knobs::DATABASE_USE_PREPARED_STATEMENTS,
    persistence::Persistence,
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
) -> anyhow::Result<Arc<dyn Persistence>> {
    let require_ssl = !do_not_require_ssl;
    let persistence: Arc<dyn Persistence> = match db {
        DbDriverTag::Sqlite => Arc::new(SqlitePersistence::new(db_spec, false)?),
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
            Arc::new(PostgresPersistence::new(args.url.as_str(), options).await?)
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
            Arc::new(
                MySqlPersistence::new(
                    Arc::new(ConvexMySqlPool::new(
                        &args.url,
                        options.use_prepared_statements,
                        Some(runtime),
                    )?),
                    args.db_name,
                    options,
                )
                .await?,
            )
        },
    };
    Ok(persistence)
}

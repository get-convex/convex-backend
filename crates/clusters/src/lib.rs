use std::collections::HashMap;

use url::Url;

mod db_driver_tag;

pub use db_driver_tag::DbDriverTag;

#[derive(Debug)]
pub enum PersistenceArgs {
    MySql { url: Url, db_name: String },
    Postgres { url: Url, schema: Option<String> },
}

/// Returns a fully qualified persistence url from a cluster url. The result URL
/// contains the exact database the persistence should connect to. The cluster
/// url should contains credentials to connect to the database, and should not
/// contain any path or query string.
pub fn persistence_args_from_cluster_url(
    instance_name: &str,
    mut cluster_url: Url,
    driver: DbDriverTag,
    require_ssl: bool,
) -> anyhow::Result<PersistenceArgs> {
    anyhow::ensure!(
        cluster_url.query().is_none(),
        "cluster url already contains query string: {:?}",
        cluster_url.query()
    );
    anyhow::ensure!(
        cluster_url.username() != "",
        // Don't print the full URL since it might contains password.
        "cluster url username must be set",
    );
    match driver {
        DbDriverTag::Postgres(_) | DbDriverTag::PostgresAwsIam(_) => {
            // NOTE: for PostgresAwsIam we do not set any database so we can
            // reuse connections between databases
            let schema = if matches!(driver, DbDriverTag::PostgresAwsIam(_)) {
                // N.B.: unlike mysql we use the instance name as-is as a schema
                // name (we don't change - to _)
                Some(instance_name.to_string())
            } else {
                // selfhosted case
                let db_name = instance_name.replace('-', "_");
                anyhow::ensure!(
                    cluster_url.path() == "" || cluster_url.path() == "/",
                    "cluster url already contains db name: {}",
                    cluster_url.path()
                );
                cluster_url.set_path(&db_name);
                None
            };
            if require_ssl {
                cluster_url
                    .query_pairs_mut()
                    .append_pair("sslmode", "require");
            }
            Ok(PersistenceArgs::Postgres {
                url: cluster_url,
                schema,
            })
        },
        DbDriverTag::MySql(_) | DbDriverTag::MySqlAwsIam(_) => {
            // NOTE: We do not set any database so we can reuse connections between
            // database. The persistence layer will select the correct database.
            match driver {
                DbDriverTag::MySql(_) => {
                    if require_ssl {
                        cluster_url
                            .query_pairs_mut()
                            .append_pair("require_ssl", "true")
                            .append_pair("verify_ca", "true");
                    }
                },
                DbDriverTag::MySqlAwsIam(_) => {
                    // always require SSL
                    cluster_url
                        .query_pairs_mut()
                        .append_pair("require_ssl", "true")
                        .append_pair("verify_ca", "false");
                },
                _ => (),
            }
            let db_name = instance_name.replace('-', "_");
            Ok(PersistenceArgs::MySql {
                url: cluster_url,
                db_name,
            })
        },
        DbDriverTag::Sqlite => anyhow::bail!("no url for sqlite"),
        #[cfg(any(test, feature = "testing"))]
        DbDriverTag::TestPersistence => {
            anyhow::bail!("no url for test persistence")
        },
    }
}

// Parse a single line with format "db-name=URL".
pub fn parse_cluster_name_to_url(s: &str) -> anyhow::Result<(String, Url)> {
    let pos = s
        .find('=')
        .ok_or_else(|| anyhow::anyhow!("invalid `database=URL` entry: no `=` found in `{s}`"))?;
    Ok((s[..pos].to_owned(), s[pos + 1..].parse()?))
}

/// Path to a file containing one `db-name=URL` entry per line. The URL
/// should be of the format `mysql://user:pass@host:port`, where `user`
/// and `pass` should be percent-encoded.
pub fn parse_cluster_urls(contents: String) -> anyhow::Result<HashMap<String, Url>> {
    contents
        .lines()
        .filter_map(|line| {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed)
            }
        })
        .map(parse_cluster_name_to_url)
        .collect::<anyhow::Result<HashMap<String, Url>>>()
}

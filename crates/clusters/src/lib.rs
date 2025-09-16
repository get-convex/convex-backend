use std::collections::HashMap;

use url::Url;

mod db_driver_tag;

pub use db_driver_tag::DbDriverTag;

#[derive(Debug)]
pub enum PersistenceArgs {
    MySql {
        url: Url,
        db_name: String,
        multitenant: bool,
    },
    Postgres {
        url: Url,
        schema: Option<String>,
        multitenant: bool,
    },
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
    require_leader: bool,
) -> anyhow::Result<PersistenceArgs> {
    anyhow::ensure!(
        cluster_url.username() != "",
        // Don't print the full URL since it might contains password.
        "cluster url username must be set",
    );
    match driver {
        DbDriverTag::Postgres(_)
        | DbDriverTag::PostgresMultiSchema(_)
        | DbDriverTag::PostgresAwsIam(_)
        | DbDriverTag::PostgresMultitenant(_) => {
            let schema = if matches!(driver, DbDriverTag::Postgres(_)) {
                // selfhosted case
                let db_name = instance_name.replace('-', "_");
                anyhow::ensure!(
                    cluster_url.path() == "" || cluster_url.path() == "/",
                    "cluster url already contains db name: {}",
                    cluster_url.path()
                );
                cluster_url.set_path(&db_name);
                None
            } else if matches!(driver, DbDriverTag::PostgresMultitenant(_)) {
                let maybe_schema = cluster_url
                    .query_pairs()
                    .find(|(k, _)| k == "search_path")
                    .map(|(_, v)| v.to_string())
                    .unwrap_or_default();
                if !maybe_schema.is_empty() {
                    Some(maybe_schema)
                } else {
                    // Default to the `public` schema if not provided.
                    // Technically we'd work fine with this being empty (we query current_schema()
                    // when opening a connection to fill in the value, but would prefer to avoid
                    // doing that on every connection)
                    Some("public".to_string())
                }
            } else {
                // NOTE: we do not set any database in this case
                // N.B.: unlike mysql we use the instance name as-is as a schema
                // name (we don't change - to _)
                Some(instance_name.to_string())
            };
            if require_ssl {
                cluster_url
                    .query_pairs_mut()
                    .append_pair("sslmode", "require");
            }
            if require_leader {
                cluster_url
                    .query_pairs_mut()
                    .append_pair("target_session_attrs", "read-write");
            }
            Ok(PersistenceArgs::Postgres {
                url: cluster_url,
                schema,
                multitenant: matches!(driver, DbDriverTag::PostgresMultitenant(_)),
            })
        },
        DbDriverTag::MySql(_) => {
            // NOTE: We do not set any database so we can reuse connections between
            // database. The persistence layer will select the correct database.
            if require_ssl {
                cluster_url
                    .query_pairs_mut()
                    .append_pair("require_ssl", "true")
                    .append_pair("verify_ca", "true");
            }
            let db_name = instance_name.replace('-', "_");
            Ok(PersistenceArgs::MySql {
                url: cluster_url,
                db_name,
                multitenant: false,
            })
        },
        DbDriverTag::MySqlAwsIam(_) => {
            // NOTE: We do not set any database so we can reuse connections between
            // database. The persistence layer will select the correct database.
            // always require SSL
            cluster_url
                .query_pairs_mut()
                .append_pair("require_ssl", "true")
                .append_pair("verify_ca", "false");
            let db_name = instance_name.replace('-', "_");
            Ok(PersistenceArgs::MySql {
                url: cluster_url,
                db_name,
                multitenant: false,
            })
        },
        DbDriverTag::MySqlMultitenant(_) => {
            // NOTE: We do not set any database so we can reuse connections between
            // database. The persistence layer will select the correct database.
            // always require SSL and verify CA
            // TODO: This shouldn't be necessary anymore on multitenant databases, as all
            // connections should be to the same database.
            if require_ssl {
                cluster_url
                    .query_pairs_mut()
                    .append_pair("require_ssl", "true")
                    .append_pair("verify_ca", "true");
            }
            let path = cluster_url.path().trim_start_matches('/').to_string();
            anyhow::ensure!(
                !path.is_empty(),
                "cluster url must contain db name to use multitenant mysql driver"
            );
            Ok(PersistenceArgs::MySql {
                db_name: path,
                url: {
                    // Preserves query string
                    cluster_url.set_path("");
                    cluster_url
                },
                multitenant: true,
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
    let Some((cluster_name, url)) = s.split_once('=') else {
        anyhow::bail!("invalid `database=URL` entry: no `=` found in `{s}`")
    };
    Ok((cluster_name.to_owned(), url.parse()?))
}

// Parse a single line with format "db-name=db-driver=URL".
pub fn parse_cluster_name_to_driver_and_url(
    s: &str,
) -> anyhow::Result<(String, (DbDriverTag, Url))> {
    let [cluster_name, db_driver, url] = s.splitn(3, '=').collect::<Vec<_>>()[..] else {
        anyhow::bail!("invalid `db-name=db-driver=URL` entry: wrong number of `=` found in `{s}`")
    };
    Ok((cluster_name.to_owned(), (db_driver.parse()?, url.parse()?)))
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

/// Path to a file containing one `db-name=db-driver=URL` entry per line. The
/// URL should be of the format `mysql://user:pass@host:port`, where `user`
/// and `pass` should be percent-encoded.
pub fn parse_cluster_urls_with_driver(
    contents: String,
) -> anyhow::Result<HashMap<String, (DbDriverTag, Url)>> {
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
        .map(parse_cluster_name_to_driver_and_url)
        .collect::<anyhow::Result<HashMap<String, (DbDriverTag, Url)>>>()
}

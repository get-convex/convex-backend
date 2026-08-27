//! Structural definitions shared by every V6 persistence table.
//!
//! The V6 logical database is tenant-scoped by `deployment_id`.

pub(crate) mod documents;
pub(crate) mod indexes;

use std::sync::Arc;

use common::{
    persistence::{
        Persistence,
        PersistenceReader,
    },
    runtime::Runtime,
    shutdown::ShutdownSignal,
    types::DeploymentId as CommonDeploymentId,
};
use const_format::concatcp;
use mysql_async::Value;

use crate::{
    ConvexMySqlPool,
    MySqlOptions,
    MySqlReaderOptions,
};

pub(crate) fn not_implemented() -> anyhow::Error {
    anyhow::anyhow!("MySQL V6 persistence is not implemented")
}

pub(crate) async fn connect<RT: Runtime>(
    _pool: Arc<ConvexMySqlPool<RT>>,
    _db_name: String,
    _options: MySqlOptions,
    _lease_lost_shutdown: ShutdownSignal,
) -> anyhow::Result<Arc<dyn Persistence>> {
    Err(not_implemented())
}

pub(crate) fn connect_reader<RT: Runtime>(
    _pool: Arc<ConvexMySqlPool<RT>>,
    _db_name: String,
    _options: MySqlReaderOptions,
) -> anyhow::Result<Arc<dyn PersistenceReader>> {
    Err(not_implemented())
}

pub(crate) async fn set_persistence_read_only<RT: Runtime>(
    _pool: Arc<ConvexMySqlPool<RT>>,
    _db_name: String,
    _options: MySqlOptions,
    _read_only: bool,
) -> anyhow::Result<()> {
    Err(not_implemented())
}

/// A deployment identifier in V6 persistence tables.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) struct DeploymentId(u32);

impl DeploymentId {
    pub(crate) const fn new(value: u32) -> Self {
        Self(value)
    }
}

impl From<DeploymentId> for Value {
    fn from(deployment_id: DeploymentId) -> Self {
        Value::UInt(u64::from(deployment_id.0))
    }
}

impl TryFrom<CommonDeploymentId> for DeploymentId {
    type Error = anyhow::Error;

    fn try_from(value: CommonDeploymentId) -> Result<Self, Self::Error> {
        let value = u32::try_from(value.0).map_err(|_| {
            anyhow::anyhow!("deployment ID {} does not fit in INT UNSIGNED", value.0)
        })?;
        Ok(Self(value))
    }
}

const DOCUMENTS_DDL: &str = r#"
CREATE TABLE IF NOT EXISTS @db_name.documents (
    deployment_id INT UNSIGNED NOT NULL,
    id BINARY(16) NOT NULL,
    ts BIGINT NOT NULL,
    table_id BINARY(16) NOT NULL,
    json_value LONGBLOB NOT NULL,
    deleted BOOLEAN DEFAULT false,
    prev_ts BIGINT,
    PRIMARY KEY (deployment_id, ts, table_id, id),
    INDEX documents_by_table_and_id (deployment_id, table_id, id, ts)
) ROW_FORMAT=DYNAMIC;
"#;

const INDEXES_LATEST_DDL: &str = r#"
CREATE TABLE IF NOT EXISTS @db_name.indexes_latest (
    deployment_id INT UNSIGNED NOT NULL,
    index_id INT UNSIGNED NOT NULL,
    key_prefix VARBINARY(2500) NOT NULL,
    key_suffix LONGBLOB NULL,
    key_suffix_hash VARBINARY(16) NOT NULL,
    ts BIGINT NOT NULL,
    /* A tombstone keeps the row so a later write can see that the key was
       deleted at this timestamp. table_id and document_id are populated iff
       deleted is false. */
    deleted BOOLEAN NOT NULL DEFAULT FALSE,
    table_id BINARY(16) NULL,
    document_id BINARY(16) NULL,
    PRIMARY KEY (deployment_id, index_id, key_prefix, key_suffix_hash)
) ROW_FORMAT=DYNAMIC PARTITION BY KEY(index_id) PARTITIONS 16;
"#;

const LEASES_DDL: &str = r#"
CREATE TABLE IF NOT EXISTS @db_name.leases (
    deployment_id INT UNSIGNED NOT NULL,
    ts BIGINT NOT NULL,
    PRIMARY KEY (deployment_id)
) ROW_FORMAT=DYNAMIC;
"#;

const READ_ONLY_DDL: &str = r#"
CREATE TABLE IF NOT EXISTS @db_name.read_only (
    deployment_id INT UNSIGNED NOT NULL,
    PRIMARY KEY (deployment_id)
) ROW_FORMAT=DYNAMIC;
"#;

const PERSISTENCE_GLOBALS_DDL: &str = r#"
CREATE TABLE IF NOT EXISTS @db_name.persistence_globals (
    deployment_id INT UNSIGNED NOT NULL,
    `key` VARCHAR(255) CHARACTER SET utf8mb4 COLLATE utf8mb4_0900_bin NOT NULL,
    json_value LONGBLOB NOT NULL,
    PRIMARY KEY (deployment_id, `key`)
) ROW_FORMAT=DYNAMIC;
"#;

// This runs every time a Persistence is created, so it has to be idempotent and
// leave resident data alone. The log tables are absent because they are created
// per ten-minute bucket as writes arrive; see `indexes::log_ddl`.
pub(crate) const fn init_sql() -> &'static str {
    concatcp!(
        DOCUMENTS_DDL,
        INDEXES_LATEST_DDL,
        LEASES_DDL,
        READ_ONLY_DDL,
        PERSISTENCE_GLOBALS_DDL,
    )
}

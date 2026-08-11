#![feature(coroutines)]
#![feature(proc_macro_hygiene)]
#![feature(impl_trait_in_assoc_type)]
#![feature(try_blocks_heterogeneous)]
mod chunks;
mod connection;
mod document_encoding;
mod metrics;
mod sql;
mod v5;
use std::{
    ops::Deref,
    sync::Arc,
};

use common::{
    persistence::{
        Persistence,
        PersistenceReader,
    },
    runtime::Runtime,
    shutdown::ShutdownSignal,
    types::PersistenceVersion,
};
pub use connection::ConvexMySqlPool;

pub type MySqlPersistence<RT> = v5::Persistence<RT>;
pub type MySqlReader<RT> = v5::Reader<RT>;

#[derive(Clone, Debug)]
pub struct MySqlInstanceName {
    raw: String,
}

impl Deref for MySqlInstanceName {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.raw
    }
}

impl<T: ToString> From<T> for MySqlInstanceName {
    fn from(raw: T) -> Self {
        Self::new(raw.to_string())
    }
}

impl MySqlInstanceName {
    pub fn new(raw: String) -> Self {
        Self { raw }
    }
}

#[derive(thiserror::Error, Debug)]
pub enum ConnectError {
    #[error("persistence is read-only, data migration in progress")]
    ReadOnly,
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

#[derive(Clone, Debug)]
pub struct MySqlOptions {
    pub allow_read_only: bool,
    pub version: PersistenceVersion,
    pub instance_name: MySqlInstanceName,
    pub multitenant: bool,
}

#[derive(Debug)]
pub struct MySqlReaderOptions {
    pub db_should_be_leader: bool,
    pub version: PersistenceVersion,
    pub instance_name: MySqlInstanceName,
    pub multitenant: bool,
}

pub async fn connect_persistence<RT: Runtime>(
    pool: Arc<ConvexMySqlPool<RT>>,
    db_name: String,
    options: MySqlOptions,
    lease_lost_shutdown: ShutdownSignal,
) -> anyhow::Result<Arc<dyn Persistence>> {
    v5::connect(pool, db_name, options, lease_lost_shutdown).await
}

pub fn connect_persistence_reader<RT: Runtime>(
    pool: Arc<ConvexMySqlPool<RT>>,
    db_name: String,
    options: MySqlReaderOptions,
) -> anyhow::Result<Arc<dyn PersistenceReader>> {
    v5::connect_reader(pool, db_name, options)
}

pub async fn set_persistence_read_only<RT: Runtime>(
    pool: Arc<ConvexMySqlPool<RT>>,
    db_name: String,
    options: MySqlOptions,
    read_only: bool,
) -> anyhow::Result<()> {
    v5::set_persistence_read_only(pool, db_name, options, read_only).await
}

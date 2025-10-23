#![feature(type_alias_impl_trait)]
#![feature(try_blocks)]
#![feature(btree_extract_if)]

mod metrics;
mod state;
pub mod worker;

pub use worker::{
    SyncWorker,
    SyncWorkerConfig,
};

#[cfg(test)]
mod tests;

pub type ServerMessage = sync_types::ServerMessage<common::value::JsonPackedValue>;

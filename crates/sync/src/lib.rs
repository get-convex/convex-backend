#![feature(type_alias_impl_trait)]
#![feature(try_blocks)]
#![feature(try_blocks_heterogeneous)]

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

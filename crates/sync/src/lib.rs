#![feature(type_alias_impl_trait)]
#![feature(let_chains)]
#![feature(try_blocks)]

mod metrics;
mod state;
pub mod worker;

pub use worker::{
    SyncWorker,
    SyncWorkerConfig,
};

#[cfg(test)]
mod tests;

pub type ServerMessage = sync_types::ServerMessage<common::value::ConvexValue>;

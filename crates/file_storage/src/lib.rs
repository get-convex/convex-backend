#![feature(lazy_cell)]
#![feature(let_chains)]
use std::sync::Arc;

use common::{
    runtime::Runtime,
    sha256::Sha256Digest,
    types::ConvexOrigin,
};
use database::Database;
use futures::stream::BoxStream;
use headers::{
    ContentLength,
    ContentRange,
    ContentType,
};
use model::file_storage::types::StorageUuid;
use storage::Storage;

mod core;
mod metrics;
#[cfg(test)]
mod tests;

pub struct FileStream {
    pub sha256: Sha256Digest,
    pub content_length: ContentLength,
    pub content_type: Option<ContentType>,
    pub stream: BoxStream<'static, futures::io::Result<bytes::Bytes>>,
}

pub struct FileRangeStream {
    pub content_length: ContentLength,
    pub content_range: ContentRange,
    pub content_type: Option<ContentType>,
    pub stream: BoxStream<'static, futures::io::Result<bytes::Bytes>>,
}

#[derive(Clone)]
pub struct FileStorage<RT: Runtime> {
    pub database: Database<RT>,
    pub transactional_file_storage: TransactionalFileStorage<RT>,
}

#[derive(Clone)]
pub struct TransactionalFileStorage<RT: Runtime> {
    rt: RT,
    storage: Arc<dyn Storage>,
    convex_origin: ConvexOrigin,
}

pub struct FileMetadata {
    pub storage_id: StorageUuid,
    pub sha256: Sha256Digest,
    pub size: i64,
    pub content_type: Option<String>,
}

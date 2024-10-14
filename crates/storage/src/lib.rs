#![feature(try_blocks)]
#![feature(lazy_cell)]
#![feature(coroutines)]

use std::{
    cmp,
    env,
    fmt::{
        Debug,
        Display,
    },
    fs::{
        self,
        File,
        OpenOptions,
    },
    future::Future,
    io::{
        Read,
        Seek,
        SeekFrom,
        Write,
    },
    mem,
    path::{
        Path,
        PathBuf,
    },
    pin::Pin,
    sync::Arc,
    task::{
        Context,
        Poll,
    },
    time::Duration,
};

use anyhow::Context as _;
use async_trait::async_trait;
use bytes::Bytes;
use common::{
    errors::report_error,
    runtime::Runtime,
    types::{
        FullyQualifiedObjectKey,
        ObjectKey,
    },
};
use futures::{
    future::BoxFuture,
    io::{
        Error as IoError,
        ErrorKind as IoErrorKind,
    },
    pin_mut,
    select_biased,
    stream::{
        self,
        BoxStream,
        IntoAsyncRead,
    },
    FutureExt,
    Stream,
    StreamExt,
    TryFutureExt,
    TryStreamExt,
};
use futures_async_stream::try_stream;
use http::Uri;
use pin_project::pin_project;
use serde_json::{
    json,
    Value as JsonValue,
};
use tempfile::TempDir;
use tokio::{
    io::AsyncWrite,
    sync::mpsc,
};
use tokio_stream::wrappers::ReceiverStream;
use value::sha256::{
    Sha256,
    Sha256Digest,
};

pub const LOCAL_DIR_MIN_PART_SIZE: usize = 5 * (1 << 20);
pub const MAX_PART_SIZE: usize = 8 * (1 << 30);
pub const MAX_NUM_PARTS: usize = 10000;
pub const MAXIMUM_PARALLEL_UPLOADS: usize = 8;

#[derive(Clone, Eq, PartialEq, PartialOrd, Ord, derive_more::Display)]
pub struct UploadId(String);

impl From<String> for UploadId {
    fn from(s: String) -> Self {
        Self(s)
    }
}

pub struct StorageGetStream {
    pub content_length: i64,
    pub stream: BoxStream<'static, futures::io::Result<bytes::Bytes>>,
}

impl StorageGetStream {
    #[cfg(any(test, feature = "testing"))]
    pub async fn collect_as_bytes(self) -> anyhow::Result<Bytes> {
        use http_body_util::BodyExt;

        let Self {
            content_length,
            stream,
        } = self;
        let content = BodyExt::collect(axum::body::Body::from_stream(stream))
            .await?
            .to_bytes();
        anyhow::ensure!(
            (content_length as usize) == content.len(),
            "ContentLength mismatch"
        );
        Ok(content)
    }
}

#[derive(Clone, Hash, PartialEq, Eq, Debug, derive_more::Constructor, derive_more::Into)]
pub struct StorageCacheKey(String);

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ClientDrivenUploadToken(pub String);
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ClientDrivenUploadPartToken(pub String);

pub const DOWNLOAD_CHUNK_SIZE: u64 = 8 * (1 << 20);
pub const MAX_CONCURRENT_CHUNK_DOWNLOADS: usize = 16;

#[async_trait]
pub trait Storage: Send + Sync + Debug {
    /// Start a new upload for generated data where no authoritative hash is
    /// present to be verified and no hash is desired.
    ///
    /// Storage may choose to implement some checksuming strategy for uploads,
    /// but it's opaque to callers.
    async fn start_upload(&self) -> anyhow::Result<Box<BufferedUpload>>;

    /// A multi-part upload where the client uploads parts one at a time.
    async fn start_client_driven_upload(&self) -> anyhow::Result<ClientDrivenUploadToken>;
    async fn upload_part(
        &self,
        token: ClientDrivenUploadToken,
        part_number: u16,
        part: Bytes,
    ) -> anyhow::Result<ClientDrivenUploadPartToken>;
    async fn finish_client_driven_upload(
        &self,
        token: ClientDrivenUploadToken,
        part_tokens: Vec<ClientDrivenUploadPartToken>,
    ) -> anyhow::Result<ObjectKey>;

    /// Gets a signed url for an object.
    async fn signed_url(&self, key: ObjectKey, expires_in: Duration) -> anyhow::Result<Uri>;
    /// Creates a presigned url for uploading an object.
    async fn presigned_upload_url(&self, expires_in: Duration) -> anyhow::Result<(ObjectKey, Uri)>;
    async fn get_object_attributes(
        &self,
        key: &ObjectKey,
    ) -> anyhow::Result<Option<ObjectAttributes>>;
    /// Not intended to be called directly.
    /// Use get_range() or get() instead.
    fn get_small_range(
        &self,
        key: &ObjectKey,
        bytes_range: std::ops::Range<u64>,
    ) -> BoxFuture<'static, anyhow::Result<StorageGetStream>>;
    /// Copy from source storage (potentially different bucket) into current
    /// bucket
    async fn copy_object(&self, source: FullyQualifiedObjectKey) -> anyhow::Result<ObjectKey>;
    fn storage_type_proto(&self) -> pb::searchlight::StorageType;
    /// Return a cache key suitable for the given ObjectKey, even in
    /// a multi-tenant cache.
    fn cache_key(&self, key: &ObjectKey) -> StorageCacheKey;
    /// Return a fully qualified key, including info on bucket name
    /// and suitable for access in multi-tenant scenario
    fn fully_qualified_key(&self, key: &ObjectKey) -> FullyQualifiedObjectKey;
    fn test_only_decompose_fully_qualified_key(
        &self,
        key: FullyQualifiedObjectKey,
    ) -> anyhow::Result<ObjectKey>;
}

pub struct ObjectAttributes {
    pub size: u64,
}

pub struct SizeAndHash {
    sha256: Sha256,
    size: usize,
    err: Option<anyhow::Error>,
}

impl SizeAndHash {
    fn new() -> Self {
        Self {
            sha256: Sha256::new(),
            size: 0,
            err: None,
        }
    }

    fn update(&mut self, result: &anyhow::Result<Bytes>) {
        if self.err.is_some() {
            return;
        }
        match result {
            Ok(bytes) => {
                self.sha256.update(bytes);
                self.size += bytes.len();
            },
            Err(e) => {
                self.err = Some(anyhow::anyhow!(
                    "At least one byte value was a failure {e:?}"
                ))
            },
        }
    }

    fn finish(self) -> anyhow::Result<(usize, Sha256Digest)> {
        if let Some(err) = self.err {
            anyhow::bail!(err);
        }
        Ok((self.size, self.sha256.finalize()))
    }
}

#[async_trait]
pub trait Upload: Send + Sync {
    /// Writes data to specified object. The `data` argument must be at most
    /// 5GB, and if it isn't the final chunk of the file, it must be at
    /// least 5MB. There may be at most 10000 writes in a single upload.
    /// Must be followed by `complete` to complete the object.
    async fn write(&mut self, data: Bytes) -> anyhow::Result<()>;

    /// Writes data to the specified object in parts where multiple parts may be
    /// uploaded in parallel. The stream must contain the bytes in the same
    /// order as they are present in the file.
    /// Must be follwed by 'complete' to complete the object.
    ///
    /// See UploadExt for some easier to use variants or if you need to verify
    /// a checksum.
    async fn try_write_parallel<'a>(
        &'a mut self,
        stream: &mut Pin<Box<dyn Stream<Item = anyhow::Result<Bytes>> + Send + 'a>>,
    ) -> anyhow::Result<()>;

    /// Abort the multipart upload. Must call either abort or complete to avoid
    /// being charged for incomplete objects.
    async fn abort(self: Box<Self>) -> anyhow::Result<()>;

    /// Completes the multipart object.
    async fn complete(self: Box<Self>) -> anyhow::Result<ObjectKey>;
}

/// Helper functions for working with uploads for functions that have generic
/// types that would otherwise prevent Upload from being "object safe":
/// https://doc.rust-lang.org/reference/items/traits.html#object-safety.
#[async_trait]
pub trait UploadExt {
    /// Similar to try_write_parallel, but without support for per item Results
    async fn write_parallel(
        &mut self,
        stream: impl Stream<Item = Bytes> + Send + 'static,
    ) -> anyhow::Result<()>;

    /// Calculates a linear sha256 digest from the stream, then uploads the
    /// parts in parallel.
    async fn try_write_parallel_and_hash(
        &mut self,
        stream: impl Stream<Item = anyhow::Result<Bytes>> + Send,
    ) -> anyhow::Result<(usize, Sha256Digest)>;
}

#[async_trait]
impl<T: Upload> UploadExt for T {
    async fn write_parallel(
        &mut self,
        stream: impl Stream<Item = Bytes> + Send,
    ) -> anyhow::Result<()> {
        let mut boxed = stream.map(Ok).boxed();
        self.try_write_parallel(&mut boxed).await
    }

    async fn try_write_parallel_and_hash(
        &mut self,
        stream: impl Stream<Item = anyhow::Result<Bytes>> + Send,
    ) -> anyhow::Result<(usize, Sha256Digest)> {
        let mut size_and_hash = SizeAndHash::new();
        let mut boxed = stream
            .map(|value| {
                size_and_hash.update(&value);
                value
            })
            .boxed();
        self.try_write_parallel(&mut boxed).await?;
        drop(boxed);
        size_and_hash.finish()
    }
}

#[must_use]
pub struct BufferedUpload {
    upload: Option<Box<dyn Upload>>,
    buffer: Vec<u8>,
    min_intermediate_part_size: usize,
}

impl BufferedUpload {
    pub async fn new(
        upload: impl Upload + 'static,
        min_intermediate_part_size: usize,
    ) -> anyhow::Result<Self> {
        let buffer = Vec::with_capacity(min_intermediate_part_size);
        Ok(Self {
            upload: Some(Box::new(upload)),
            buffer,
            min_intermediate_part_size,
        })
    }

    fn update_buffer_and_get_next(&mut self, data: Bytes) -> Option<Bytes> {
        Self::_update_buffer_and_get_next(&mut self.buffer, self.min_intermediate_part_size, data)
    }

    // Hack around wanting to use this when `upload` is borrowed. Rust can't
    // otherwise tell that we're using different and not previously borrowed
    // parts of self.
    fn _update_buffer_and_get_next(
        buffer: &mut Vec<u8>,
        min_intermediate_part_size: usize,
        data: Bytes,
    ) -> Option<Bytes> {
        // Fast path, ship the buffer without copying.
        if buffer.is_empty() && data.len() >= min_intermediate_part_size {
            return Some(data);
        }
        buffer.extend_from_slice(&data);
        if buffer.len() >= min_intermediate_part_size {
            let ready = mem::replace(buffer, Vec::with_capacity(min_intermediate_part_size));
            Some(ready.into())
        } else {
            None
        }
    }
}

#[async_trait]
impl Upload for BufferedUpload {
    async fn write(&mut self, data: Bytes) -> anyhow::Result<()> {
        if let Some(buf) = self.update_buffer_and_get_next(data) {
            // self.upload is only ever taken just before drop
            self.upload
                .as_mut()
                .expect("upload must be set")
                .write(buf)
                .await?;
        }
        Ok(())
    }

    async fn try_write_parallel<'a>(
        &'a mut self,
        stream: &mut Pin<Box<dyn Stream<Item = anyhow::Result<Bytes>> + Send + 'a>>,
    ) -> anyhow::Result<()> {
        // Try to keep some buffered data ready in the channel, but not too much.
        let (tx, rx) = mpsc::channel(MAXIMUM_PARALLEL_UPLOADS / 2);

        let mut boxed_rx = ReceiverStream::new(rx).boxed();
        let mut upload = self
            .upload
            .as_mut()
            .expect("upload must be set")
            .try_write_parallel(&mut boxed_rx)
            .fuse();

        let buffer_bytes = async {
            let result: anyhow::Result<()> = try {
                while let Some(result) = stream.next().await {
                    match result {
                        Err(e) => tx.send(Err(e)).await?,
                        Ok(buf) => {
                            if let Some(buf) = Self::_update_buffer_and_get_next(
                                &mut self.buffer,
                                self.min_intermediate_part_size,
                                buf,
                            ) {
                                tx.send(Ok(buf)).await?;
                            }
                        },
                    }
                }
            };
            drop(tx);
            result
        }
        .fuse();
        pin_mut!(buffer_bytes);

        // We do loop, clippy is confused by select_biased!
        #[allow(clippy::never_loop)]
        loop {
            select_biased! {
                upload_result = upload => {
                    return upload_result;
                }
                bytes_result = buffer_bytes => {
                    bytes_result?;
                }
            }
        }
    }

    async fn abort(mut self: Box<Self>) -> anyhow::Result<()> {
        // self.upload is only ever taken just before drop
        self.upload
            .take()
            .expect("upload must be set")
            .abort()
            .await
    }

    async fn complete(mut self: Box<Self>) -> anyhow::Result<ObjectKey> {
        let ready = mem::take(&mut self.buffer);
        // self.upload is only ever taken just before drop
        let mut upload = self.upload.take().expect("upload must be set");
        upload.write(ready.into()).await?;
        upload.complete().await
    }
}

pub struct ChannelWriter {
    parts: mpsc::Sender<Bytes>,
    current_part: Vec<u8>,
    part_size: usize,
}

impl ChannelWriter {
    pub fn new(sender: mpsc::Sender<Bytes>, part_size: usize) -> Self {
        Self {
            parts: sender,
            current_part: Vec::with_capacity(part_size),
            part_size,
        }
    }
}

impl AsyncWrite for ChannelWriter {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<Result<usize, IoError>> {
        let self_ = self.get_mut();
        loop {
            if self_.current_part.len() < self_.part_size {
                let n = cmp::min(buf.len(), self_.part_size - self_.current_part.len());
                self_.current_part.extend_from_slice(&buf[..n]);
                return Poll::Ready(Ok(n));
            }
            tokio::pin! {
                let permit_future = self_.parts.reserve();
            }
            let permit = match Future::poll(permit_future, cx) {
                Poll::Ready(Ok(p)) => p,
                Poll::Ready(Err(mpsc::error::SendError(..))) => {
                    let err = Err(IoError::new(IoErrorKind::BrokenPipe, "Channel closed"));
                    return Poll::Ready(err);
                },
                Poll::Pending => return Poll::Pending,
            };
            let next_buf = Vec::with_capacity(self_.part_size);
            let buf = mem::replace(&mut self_.current_part, next_buf);
            permit.send(buf.into());
        }
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), IoError>> {
        // We want to control the part size, so don't do anything on flush.
        Poll::Ready(Ok(()))
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), IoError>> {
        let self_ = self.get_mut();
        if !self_.current_part.is_empty() {
            tokio::pin! {
                let permit_future = self_.parts.reserve();
            }
            let permit = match Future::poll(permit_future, cx) {
                Poll::Ready(Ok(p)) => p,
                Poll::Ready(Err(mpsc::error::SendError(..))) => {
                    let err = Err(IoError::new(IoErrorKind::BrokenPipe, "Channel closed"));
                    return Poll::Ready(err);
                },
                Poll::Pending => return Poll::Pending,
            };
            let next_buf = vec![];
            let buf = mem::replace(&mut self_.current_part, next_buf);
            permit.send(buf.into());
        }
        Poll::Ready(Ok(()))
    }
}

/// Read and seek an object written to storage.
#[pin_project]
pub struct StorageObjectReader {
    storage: Arc<dyn Storage>,
    object_key: ObjectKey,
    full_size: u64,
    cursor: u64,
    #[pin]
    inner: IntoAsyncRead<BoxStream<'static, futures::io::Result<Bytes>>>,
}

#[async_trait]
pub trait StorageExt {
    async fn get_reader(&self, object_key: &ObjectKey) -> anyhow::Result<StorageObjectReader>;
    /// Gets a stream for a range of a previously stored object.
    async fn get_range(
        &self,
        key: &ObjectKey,
        bytes_range: (std::ops::Bound<u64>, std::ops::Bound<u64>),
    ) -> anyhow::Result<Option<StorageGetStream>>;
    /// Gets a previously stored object.
    async fn get(&self, key: &ObjectKey) -> anyhow::Result<Option<StorageGetStream>>;
    async fn get_small_range_with_retries(
        &self,
        key: &ObjectKey,
        small_byte_range: std::ops::Range<u64>,
    ) -> anyhow::Result<StorageGetStream>;
}

#[async_trait]
impl StorageExt for Arc<dyn Storage> {
    async fn get_reader(&self, object_key: &ObjectKey) -> anyhow::Result<StorageObjectReader> {
        let full_size = self
            .get_object_attributes(object_key)
            .await?
            .with_context(|| format!("object {object_key:?} does not exist in {self:?}"))?
            .size;
        let inner =
            StorageObjectReader::new_inner_reader_starting_at(self.clone(), object_key.clone(), 0);
        Ok(StorageObjectReader {
            storage: self.clone(),
            object_key: object_key.clone(),
            full_size,
            cursor: 0,
            inner,
        })
    }

    async fn get(&self, key: &ObjectKey) -> anyhow::Result<Option<StorageGetStream>> {
        self.get_range(
            key,
            (std::ops::Bound::Unbounded, std::ops::Bound::Unbounded),
        )
        .await
    }

    async fn get_range(
        &self,
        key: &ObjectKey,
        bytes_range: (std::ops::Bound<u64>, std::ops::Bound<u64>),
    ) -> anyhow::Result<Option<StorageGetStream>> {
        let Some(attributes) = self.get_object_attributes(key).await? else {
            return Ok(None);
        };
        let start_byte = cmp::min(
            match bytes_range.0 {
                std::ops::Bound::Included(bound) => bound,
                std::ops::Bound::Excluded(bound) => bound + 1,
                std::ops::Bound::Unbounded => 0,
            },
            attributes.size,
        );
        let end_byte_bound = cmp::min(
            match bytes_range.1 {
                std::ops::Bound::Included(bound) => bound + 1,
                std::ops::Bound::Excluded(bound) => bound,
                std::ops::Bound::Unbounded => attributes.size,
            },
            attributes.size,
        );
        let num_chunks = 1 + (end_byte_bound - start_byte) / DOWNLOAD_CHUNK_SIZE;
        // A list of futures, each of which resolves to a stream.
        let mut chunk_futures = vec![];
        for idx in 0..num_chunks {
            let chunk_start = start_byte + DOWNLOAD_CHUNK_SIZE * idx;
            let chunk_end = if idx == num_chunks - 1 {
                end_byte_bound
            } else {
                start_byte + DOWNLOAD_CHUNK_SIZE * (idx + 1)
            };
            let self_ = self.clone();
            let key_ = key.clone();
            let stream_fut = async move {
                self_
                .get_small_range_with_retries(&key_, chunk_start..chunk_end)
                // Mapping everything to `io::ErrorKind::Other` feels bad, but it's what the AWS library does internally.
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))
                .map_ok(|storage_get_stream| storage_get_stream.stream).await
            };
            chunk_futures.push(stream_fut);
        }
        // Convert the list of futures into a stream, where each item is the resolved
        // output of the future (i.e. a `ByteStream`)
        let byte_stream = futures::stream::iter(chunk_futures)
            // Wrap it in `Ok` as the underlying stream is a `TryStream`, and the error types must match
            .map(Ok)
            // Limit the concurrency of the chunk downloads
            .try_buffered(MAX_CONCURRENT_CHUNK_DOWNLOADS)
            // Flatten the `Stream<Item = io::Result<Stream<Item = io::Result<Bytes>>>>` into a single `Stream<Item = io::Result<Bytes>>`
            .try_flatten();
        Ok(Some(StorageGetStream {
            content_length: (end_byte_bound - start_byte) as i64,
            stream: Box::pin(byte_stream),
        }))
    }

    async fn get_small_range_with_retries(
        &self,
        key: &ObjectKey,
        small_byte_range: std::ops::Range<u64>,
    ) -> anyhow::Result<StorageGetStream> {
        let output = self.get_small_range(key, small_byte_range.clone()).await?;
        let content_length = output.content_length;
        let initial_stream = output.stream;
        Ok(StorageGetStream {
            content_length,
            stream: Box::pin(stream_object_with_retries(
                initial_stream,
                self.clone(),
                key.clone(),
                small_byte_range,
                STORAGE_GET_RETRIES,
            )),
        })
    }
}

const STORAGE_GET_RETRIES: usize = 5;

#[allow(clippy::blocks_in_conditions)]
#[try_stream(ok = Bytes, error = futures::io::Error)]
async fn stream_object_with_retries(
    mut stream: BoxStream<'static, futures::io::Result<Bytes>>,
    storage: Arc<dyn Storage>,
    key: ObjectKey,
    small_byte_range: std::ops::Range<u64>,
    mut retries_remaining: usize,
) {
    let mut bytes_yielded = 0;
    loop {
        match stream.try_next().await {
            Ok(Some(chunk)) => {
                bytes_yielded += chunk.len();
                yield chunk;
                if small_byte_range.start + bytes_yielded as u64 >= small_byte_range.end {
                    // In case there's a later error, we don't want to retry and fetch zero bytes,
                    // so just end here.
                    return Ok(());
                }
            },
            Ok(None) => return Ok(()),
            Err(e) if retries_remaining == 0 => {
                return Err(e);
            },
            Err(e) => {
                report_error(&mut anyhow::anyhow!(e).context(format!(
                    "failed while reading stream for {key:?}. {retries_remaining} attempts \
                     remaining"
                )));
                let new_range =
                    (small_byte_range.start + bytes_yielded as u64)..small_byte_range.end;
                let output = storage
                    .get_small_range(&key, new_range)
                    .await
                    .map_err(|e| futures::io::Error::new(std::io::ErrorKind::Other, e))?;
                stream = output.stream;
                retries_remaining -= 1;
            },
        }
    }
}

impl StorageObjectReader {
    fn new_inner_reader_starting_at(
        storage: Arc<dyn Storage>,
        object_key: ObjectKey,
        starting_at_index: u64,
    ) -> IntoAsyncRead<BoxStream<'static, futures::io::Result<Bytes>>> {
        let get_range_fut = async move {
            storage
                .get_range(
                    &object_key,
                    (
                        std::ops::Bound::Included(starting_at_index),
                        std::ops::Bound::Unbounded,
                    ),
                )
                .await?
                .with_context(|| format!("{object_key:?} not found"))
        };
        stream::once(get_range_fut)
            .map(move |storage_get_stream| {
                let storage_get_stream = storage_get_stream
                    .map_err(|e| futures::io::Error::new(std::io::ErrorKind::Other, e))?;
                futures::io::Result::Ok(storage_get_stream.stream)
            })
            .try_flatten()
            .boxed()
            .into_async_read()
    }
}

impl futures::io::AsyncRead for StorageObjectReader {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<std::io::Result<usize>> {
        match self.as_mut().project().inner.poll_read(cx, buf) {
            Poll::Ready(Ok(bytes_read)) => {
                self.cursor += bytes_read as u64;
                Poll::Ready(Ok(bytes_read))
            },
            Poll::Ready(Err(e)) => Poll::Ready(Err(e)),
            Poll::Pending => Poll::Pending,
        }
    }
}

impl futures::io::AsyncSeek for StorageObjectReader {
    fn poll_seek(
        mut self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        pos: SeekFrom,
    ) -> Poll<std::io::Result<u64>> {
        let new_cursor = match pos {
            SeekFrom::Start(seek) => seek,
            SeekFrom::Current(seek) => ((self.cursor as i64) + seek) as u64,
            SeekFrom::End(seek) => ((self.full_size as i64) + seek) as u64,
        };
        if self.cursor == new_cursor {
            return Poll::Ready(Ok(self.cursor));
        }
        tracing::debug!(
            "storage object {:?} seeking from {} -> {}",
            self.object_key,
            self.cursor,
            new_cursor
        );
        self.cursor = new_cursor;
        self.inner = Self::new_inner_reader_starting_at(
            self.storage.clone(),
            self.object_key.clone(),
            self.cursor,
        );
        Poll::Ready(Ok(self.cursor))
    }
}

#[cfg(test)]
mod tests {
    use crate::ObjectKey;

    #[tokio::test]
    async fn test_object_key() -> anyhow::Result<()> {
        assert_eq!(
            &String::from(ObjectKey::try_from(
                "folder/name-to_test/9.json".to_owned()
            )?),
            "folder/name-to_test/9.json",
        );
        assert!(ObjectKey::try_from("folder>name".to_owned()).is_err());
        Ok(())
    }
}

#[derive(Clone)]
pub struct LocalDirStorage<RT: Runtime> {
    rt: RT,
    dir: PathBuf,
    _temp_dir: Option<Arc<TempDir>>,
}

impl<RT: Runtime> std::fmt::Debug for LocalDirStorage<RT> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LocalDirStorage")
            .field("dir", &self.dir)
            .finish()
    }
}

impl<RT: Runtime> LocalDirStorage<RT> {
    // Creates local storage using a temporary directory. The directory
    // is deleted when the object is dropped.
    pub fn new(rt: RT) -> anyhow::Result<Self> {
        let temp_dir = TempDir::new()?;
        let storage = Self {
            rt,
            dir: temp_dir.path().to_owned(),
            _temp_dir: Some(Arc::new(temp_dir)),
        };
        Ok(storage)
    }

    /// Create storage at the provided directory
    pub fn new_at_path(rt: RT, dir: PathBuf) -> anyhow::Result<Self> {
        let dir = if dir.is_absolute() {
            dir
        } else {
            env::current_dir()?.join(dir)
        };
        fs::create_dir_all(&dir)?;
        let storage = Self {
            rt,
            dir,
            _temp_dir: None,
        };
        Ok(storage)
    }

    /// Returns the path to the storage directory.
    pub fn path(&self) -> &PathBuf {
        &self.dir
    }

    fn path_for_key(&self, key: ObjectKey) -> String {
        String::from(key) + ".blob"
    }

    pub fn for_use_case(rt: RT, dir: &str, use_case: StorageUseCase) -> anyhow::Result<Self> {
        let use_case_str = use_case.to_string();
        anyhow::ensure!(!dir.is_empty());
        let storage = LocalDirStorage::new_at_path(rt, PathBuf::from(dir).join(use_case_str))?;
        Ok(storage)
    }
}

struct ClientDrivenUpload {
    object_key: ObjectKey,
    filepath: PathBuf,
}

impl TryFrom<ClientDrivenUpload> for ClientDrivenUploadToken {
    type Error = anyhow::Error;

    fn try_from(value: ClientDrivenUpload) -> Result<Self, Self::Error> {
        let v = json!({
            "objectKey": value.object_key.to_string(),
            "filepath": value.filepath.to_str(),
        });
        Ok(ClientDrivenUploadToken(serde_json::to_string(&v)?))
    }
}

impl TryFrom<ClientDrivenUploadToken> for ClientDrivenUpload {
    type Error = anyhow::Error;

    fn try_from(value: ClientDrivenUploadToken) -> Result<Self, Self::Error> {
        let v: JsonValue = serde_json::from_str(&value.0)?;
        let object_key = v
            .get("objectKey")
            .context("missing objectKey")?
            .as_str()
            .context("objectKey should be str")?
            .try_into()?;
        let filepath = v
            .get("filepath")
            .context("missing filepath")?
            .as_str()
            .context("filepath should be str")?
            .parse()?;
        Ok(Self {
            object_key,
            filepath,
        })
    }
}

#[async_trait]
impl<RT: Runtime> Storage for LocalDirStorage<RT> {
    async fn start_upload(&self) -> anyhow::Result<Box<BufferedUpload>> {
        let object_key: ObjectKey = self.rt.new_uuid_v4().to_string().try_into()?;
        let key = self.path_for_key(object_key.clone());
        let filepath = self.dir.join(key);

        // The filename constraints on the local file system are a bit stricter than S3,
        // so these might fail. If it fails, let's think about what kinds of paths we're
        // passing in and figure out if we want to expand LocalDirStorage, or constrain
        // the keys.
        //
        // Note that "/" (forward slash) is allowed in the key and is reinterpreted onto
        // the local file system as a directory.
        fs::create_dir_all(filepath.parent().expect("Must have parent")).context(
            "LocalDirStorage file creation failed. Perhaps the storage object key isn't valid?",
        )?;
        let file = File::create(filepath).context(
            "LocalDirStorage file creation failed. Perhaps the storage object key isn't valid?",
        )?;

        let upload = LocalDirUpload {
            object_key,
            file: Some(file),
            num_parts: 0,
        };
        let upload = BufferedUpload::new(upload, LOCAL_DIR_MIN_PART_SIZE).await?;
        Ok(Box::new(upload))
    }

    async fn start_client_driven_upload(&self) -> anyhow::Result<ClientDrivenUploadToken> {
        let object_key: ObjectKey = self.rt.new_uuid_v4().to_string().try_into()?;
        let key = self.path_for_key(object_key.clone());
        let filepath = self.dir.join(key);

        // The filename constraints on the local file system are a bit stricter than S3,
        // so these might fail. If it fails, let's think about what kinds of paths we're
        // passing in and figure out if we want to expand LocalDirStorage, or constrain
        // the keys.
        //
        // Note that "/" (forward slash) is allowed in the key and is reinterpreted onto
        // the local file system as a directory.
        fs::create_dir_all(filepath.parent().expect("Must have parent")).context(
            "LocalDirStorage file creation failed. Perhaps the storage object key isn't valid?",
        )?;
        let _file = File::create(filepath.clone()).context(
            "LocalDirStorage file creation failed. Perhaps the storage object key isn't valid?",
        )?;

        ClientDrivenUpload {
            object_key,
            filepath,
        }
        .try_into()
    }

    async fn upload_part(
        &self,
        token: ClientDrivenUploadToken,
        _part_number: u16,
        part: Bytes,
    ) -> anyhow::Result<ClientDrivenUploadPartToken> {
        let ClientDrivenUpload {
            object_key,
            filepath,
        } = token.try_into()?;
        let file = OpenOptions::new().append(true).open(filepath)?;
        let mut upload = LocalDirUpload {
            object_key,
            file: Some(file),
            num_parts: 0, // unused
        };
        upload.write(part).await?;
        Ok(ClientDrivenUploadPartToken(String::new()))
    }

    async fn finish_client_driven_upload(
        &self,
        token: ClientDrivenUploadToken,
        _part_tokens: Vec<ClientDrivenUploadPartToken>,
    ) -> anyhow::Result<ObjectKey> {
        let ClientDrivenUpload {
            object_key,
            filepath: _,
        } = token.try_into()?;
        Ok(object_key)
    }

    async fn signed_url(&self, key: ObjectKey, _expires_in: Duration) -> anyhow::Result<Uri> {
        let key = self.path_for_key(key);
        let path = self.dir.join(key);
        let path = path
            .to_str()
            .ok_or_else(|| anyhow::anyhow!("Dir isn't valid UTF8: {:?}", self.dir))?;
        let uri = if cfg!(windows) {
            // On windows, the Uri::builder does not work properly.
            // file://localhostC:\\Users\\nipunn\\src\\convex\\convex_local_storage\\modules\\4c4a018d-e534-491e-aa99-a9c16eb97add.blob
            //
            // url::Url works, but does not parse into a good Uri without localhost
            // authority file:///C:/Users/nipunn/src/convex/convex_local_storage/modules/9fb14d74-f91a-47bc-8be3-f646a460fcde.blob
            //
            // throw away the C: prefix
            let path = path.split_once(':').context("Missing drive letter")?.1;
            // Switch backslashes to URI syntax
            let path = path.replace('\\', "/");
            format!("file://localhost{path}")
                .parse()
                .context("Could not parse path")?
        } else {
            Uri::builder()
                .scheme("file")
                .authority("localhost")
                .path_and_query(path)
                .build()?
        };
        Ok(uri)
    }

    async fn presigned_upload_url(&self, expires_in: Duration) -> anyhow::Result<(ObjectKey, Uri)> {
        let object_key: ObjectKey = self.rt.new_uuid_v4().to_string().try_into()?;
        Ok((
            object_key.clone(),
            self.signed_url(object_key, expires_in).await?,
        ))
    }

    fn cache_key(&self, key: &ObjectKey) -> StorageCacheKey {
        let key = self.path_for_key(key.clone());
        let path = self.dir.join(key);
        StorageCacheKey(path.to_string_lossy().to_string())
    }

    fn fully_qualified_key(&self, key: &ObjectKey) -> FullyQualifiedObjectKey {
        let key = self.path_for_key(key.clone());
        let path = self.dir.join(key);
        path.to_string_lossy().to_string().into()
    }

    fn test_only_decompose_fully_qualified_key(
        &self,
        key: FullyQualifiedObjectKey,
    ) -> anyhow::Result<ObjectKey> {
        let key: String = key.into();
        let path = Path::new(&key);
        let remaining = path.strip_prefix(&self.dir)?.to_string_lossy();
        remaining
            .strip_suffix(".blob")
            .context("Doesn't end with .blob")?
            .try_into()
    }

    fn get_small_range(
        &self,
        key: &ObjectKey,
        bytes_range: std::ops::Range<u64>,
    ) -> BoxFuture<'static, anyhow::Result<StorageGetStream>> {
        let key = self.path_for_key(key.clone());
        let path = self.dir.join(key);
        async move {
            let mut buf = vec![0; (bytes_range.end - bytes_range.start) as usize];
            let mut file = File::open(path.clone()).context(format!(
                "Local dir storage couldn't open {}",
                path.display()
            ))?;
            file.seek(SeekFrom::Start(bytes_range.start))?;
            file.read_exact(&mut buf)?;
            Ok(StorageGetStream {
                content_length: (bytes_range.end - bytes_range.start) as i64,
                stream: stream::once(async move { Ok(buf.into()) }).boxed(),
            })
        }
        .boxed()
    }

    async fn get_object_attributes(
        &self,
        key: &ObjectKey,
    ) -> anyhow::Result<Option<ObjectAttributes>> {
        let key = self.path_for_key(key.clone());
        let path = self.dir.join(key);
        let mut buf = vec![];
        let result = File::open(path);
        if result.is_err() {
            return Ok(None);
        }
        let mut file = result.unwrap();
        file.read_to_end(&mut buf)?;
        Ok(Some(ObjectAttributes {
            size: buf.len() as u64,
        }))
    }

    async fn copy_object(&self, source: FullyQualifiedObjectKey) -> anyhow::Result<ObjectKey> {
        let source: String = source.into();
        let source_path = Path::new(&source);
        let key: ObjectKey = self.rt.new_uuid_v4().to_string().try_into()?;
        let dest_path = self.dir.join(self.path_for_key(key.clone()));
        fs::copy(source_path, dest_path)?;
        Ok(key)
    }

    fn storage_type_proto(&self) -> pb::searchlight::StorageType {
        pb::searchlight::StorageType {
            storage_type: Some(pb::searchlight::storage_type::StorageType::Local(
                pb::searchlight::LocalStorage {
                    path: self.dir.to_string_lossy().into_owned(),
                },
            )),
        }
    }
}

pub struct LocalDirUpload {
    object_key: ObjectKey,
    file: Option<File>,
    num_parts: usize,
}

#[async_trait]
impl Upload for LocalDirUpload {
    async fn write(&mut self, data: Bytes) -> anyhow::Result<()> {
        anyhow::ensure!(self.num_parts < MAX_NUM_PARTS);
        anyhow::ensure!(data.len() <= MAX_PART_SIZE);
        let file = self
            .file
            .as_mut()
            .ok_or_else(|| anyhow::anyhow!("Upload not active"))?;

        file.write_all(&data)?;

        self.num_parts += 1;

        Ok(())
    }

    async fn try_write_parallel<'a>(
        &'a mut self,
        stream: &mut Pin<Box<dyn Stream<Item = anyhow::Result<Bytes>> + Send + 'a>>,
    ) -> anyhow::Result<()> {
        while let Some(value) = stream.next().await {
            self.write(value?).await?;
        }
        Ok(())
    }

    async fn abort(mut self: Box<Self>) -> anyhow::Result<()> {
        anyhow::ensure!(self.file.is_some());
        self.file.take();
        Ok(())
    }

    async fn complete(mut self: Box<Self>) -> anyhow::Result<ObjectKey> {
        let object_key = self.object_key;

        let file = self.file.take().context("Completing inactive file")?;
        file.sync_all()?;
        Ok(object_key)
    }
}

#[derive(Clone, Copy, Eq, Ord, PartialEq, PartialOrd)]
pub enum StorageUseCase {
    /// Snapshot Exports
    Exports,
    /// Snapshot Imports, stored temporarily so we can process multiple times
    /// without holding in memory.
    SnapshotImports,
    /// Our module cache
    Modules,
    /// User/customer facing storage
    Files,
    /// Search index snapshots
    SearchIndexes,
}

impl Display for StorageUseCase {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StorageUseCase::Exports => write!(f, "exports"),
            StorageUseCase::SnapshotImports => write!(f, "snapshot_imports"),
            StorageUseCase::Modules => write!(f, "modules"),
            StorageUseCase::Files => write!(f, "files"),
            StorageUseCase::SearchIndexes => write!(f, "search"),
        }
    }
}

#[cfg(test)]
mod local_storage_tests {
    use std::{
        fs::File,
        io::Read,
        sync::Arc,
        time::Duration,
    };

    use anyhow::Context;
    use bytes::Bytes;
    use common::runtime::testing::TestRuntime;
    use futures::{
        stream,
        StreamExt,
        TryStreamExt,
    };

    use super::{
        stream_object_with_retries,
        LocalDirStorage,
        Storage,
        StorageExt,
        Upload,
        DOWNLOAD_CHUNK_SIZE,
        LOCAL_DIR_MIN_PART_SIZE,
    };

    #[convex_macro::test_runtime]
    async fn test_upload(rt: TestRuntime) -> anyhow::Result<()> {
        let storage = LocalDirStorage::new(rt)?;
        let mut test_upload = storage.start_upload().await?;
        test_upload
            .write(vec![1; LOCAL_DIR_MIN_PART_SIZE].into())
            .await?;
        test_upload.write(vec![2, 3, 4].into()).await?;
        let _object_key = test_upload.complete().await?;
        Ok(())
    }

    #[convex_macro::test_runtime]
    async fn test_upload_auto(rt: TestRuntime) -> anyhow::Result<()> {
        let storage = LocalDirStorage::new(rt)?;
        let mut test_upload = storage.start_upload().await?;
        test_upload
            .write(vec![1; LOCAL_DIR_MIN_PART_SIZE].into())
            .await?;
        test_upload.write(vec![2, 3, 4].into()).await?;
        let _object_key = test_upload.complete().await?;
        Ok(())
    }

    #[convex_macro::test_runtime]
    async fn test_abort(rt: TestRuntime) -> anyhow::Result<()> {
        let storage = LocalDirStorage::new(rt)?;
        let mut test_upload = storage.start_upload().await?;
        test_upload
            .write(vec![1; LOCAL_DIR_MIN_PART_SIZE].into())
            .await?;
        test_upload.abort().await?;
        Ok(())
    }

    #[convex_macro::test_runtime]
    async fn test_local_storage(rt: TestRuntime) -> anyhow::Result<()> {
        let storage: Arc<dyn Storage> = Arc::new(LocalDirStorage::new(rt)?);
        let mut upload = storage.start_upload().await?;
        upload.write(Bytes::from_static(b"pinna park")).await?;
        let key = upload.complete().await?;

        // Get via .get()
        let contents = storage
            .get(&key)
            .await?
            .context("Not found")?
            .collect_as_bytes()
            .await?;
        assert_eq!(&contents, "pinna park");

        // Get via signed_url
        let uri = storage.signed_url(key, Duration::from_secs(10)).await?;
        let mut f = File::open(uri.path())?;
        let mut buf = String::new();
        f.read_to_string(&mut buf)?;
        assert_eq!(&buf, "pinna park");

        Ok(())
    }

    #[convex_macro::test_runtime]
    async fn test_storage_get_paginated(rt: TestRuntime) -> anyhow::Result<()> {
        // Test that chunks are stitched together in the right order.
        let storage: Arc<dyn Storage> = Arc::new(LocalDirStorage::new(rt)?);
        let mut test_upload = storage.start_upload().await?;
        let prefix_length = (DOWNLOAD_CHUNK_SIZE * 2) as usize;
        let suffix_length = (DOWNLOAD_CHUNK_SIZE / 2) as usize;
        let length = prefix_length + suffix_length;
        test_upload.write(vec![1; prefix_length].into()).await?;
        test_upload.write(vec![2; suffix_length].into()).await?;
        let object_key = test_upload.complete().await?;

        let stream = storage.get(&object_key).await?.unwrap();
        assert_eq!(stream.content_length, length as i64);
        let bytes = stream.collect_as_bytes().await?;
        assert_eq!(bytes.len(), length);
        assert_eq!(&bytes[..prefix_length], &vec![1; prefix_length]);
        assert_eq!(&bytes[prefix_length..], &vec![2; suffix_length]);

        let suffix_stream = storage
            .get_range(
                &object_key,
                (
                    std::ops::Bound::Included(prefix_length as u64),
                    std::ops::Bound::Excluded(length as u64),
                ),
            )
            .await?
            .unwrap();
        assert_eq!(suffix_stream.content_length, suffix_length as i64);
        let bytes = suffix_stream.collect_as_bytes().await?;
        assert_eq!(bytes.len(), suffix_length);
        assert_eq!(&bytes, &vec![2; suffix_length]);
        Ok(())
    }

    #[convex_macro::test_runtime]
    async fn test_storage_get_with_retries(rt: TestRuntime) -> anyhow::Result<()> {
        // Test that if the first storage range request disconnects after
        // one chunk, the rest is fetched successfully and everything is
        // stitched together.
        let storage: Arc<dyn Storage> = Arc::new(LocalDirStorage::new(rt)?);
        let mut test_upload = storage.start_upload().await?;
        test_upload
            .write(vec![0, 1, 2, 3, 4, 5, 6, 7, 8].into())
            .await?;
        let object_key = test_upload.complete().await?;
        let disconnected_stream = stream::iter(vec![
            Ok(vec![1, 2, 3].into()),
            Err(futures::io::Error::new(
                futures::io::ErrorKind::ConnectionAborted,
                anyhow::anyhow!("err"),
            )),
        ])
        .boxed();
        let stream_with_retries = stream_object_with_retries(
            disconnected_stream,
            storage.clone(),
            object_key.clone(),
            1..8,
            1,
        );
        let results: Vec<_> = stream_with_retries.try_collect().await?;
        assert_eq!(
            results,
            vec![Bytes::from(vec![1, 2, 3]), Bytes::from(vec![4, 5, 6, 7])]
        );
        Ok(())
    }
}

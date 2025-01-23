use std::{
    path::{
        Path,
        PathBuf,
    },
    sync::Arc,
};

use anyhow::Context;
use async_lru::async_lru::{
    AsyncLru,
    SizedValue,
};
use bytesize::ByteSize;
use common::{
    async_compat::FuturesAsyncReadCompatExt,
    bounded_thread_pool::BoundedThreadPool,
    knobs::ARCHIVE_FETCH_TIMEOUT_SECONDS,
    runtime::{
        Runtime,
        SpawnHandle,
    },
    types::ObjectKey,
};
use futures::{
    pin_mut,
    select_biased,
    FutureExt,
    TryStreamExt,
};
use storage::{
    Storage,
    StorageCacheKey,
    StorageExt,
};
use tokio::{
    fs,
    io::{
        AsyncWriteExt,
        BufReader,
    },
    sync::mpsc,
};
use uuid::Uuid;
use vector::qdrant_segments::restore_segment_from_tar;

use super::{
    extract_zip,
    metrics::{
        self,
        archive_untar_timer,
    },
};
use crate::SearchFileType;

struct IndexMeta {
    size: u64,
    path: PathBuf,
    cleaner: CacheCleaner,
}

impl Drop for IndexMeta {
    fn drop(&mut self) {
        let _ = self.cleaner.attempt_cleanup(self.path.clone());
    }
}

impl SizedValue for IndexMeta {
    fn size(&self) -> u64 {
        self.size
    }
}

/// A specialized LRU cache for storing archives of Tantivy and Qdrant indexes.
/// The manager is constructed with a storage directory and a max size.
///
/// The entry point to the manager is the `get()` method which checks to see if
/// the key exists in the disk cache, and if not, fetches it from the provided
/// [`Storage`] implementation. Multiple calls to `get()` the same key will be
/// queued, such that only the head of the queue performs the remote fetching
/// and unarchiving; subsequent requests wait on a channel to receive the
/// result. This logic is implemented by the wrapped `AsyncLru`
///
/// The manager asynchronously prunes old entries when the cache is "over
/// quota". As this pruning is performed after archives are added to the cache,
/// the manager will transiently exceed the configured `max_size`.
///
/// In the interest of hot-path performance, any deletion or pruning operations
/// are best-effort and are spawned to the thread pool rather than occurring in
/// the calling task. For now, errors in these spawned tasks will panic the
/// entire process.
pub struct ArchiveCacheManager<RT: Runtime> {
    path: PathBuf,
    max_size: u64,
    cleaner: CacheCleaner,
    blocking_thread_pool: BoundedThreadPool<RT>,
    cache: AsyncLru<RT, Key, IndexMeta>,
    rt: RT,
}

impl<RT: Runtime> Clone for ArchiveCacheManager<RT> {
    fn clone(&self) -> Self {
        Self {
            path: self.path.clone(),
            max_size: self.max_size,
            cleaner: self.cleaner.clone(),
            blocking_thread_pool: self.blocking_thread_pool.clone(),
            cache: self.cache.clone(),
            rt: self.rt.clone(),
        }
    }
}

#[derive(Clone, Hash, PartialEq, Eq, Debug)]
struct Key {
    path: StorageCacheKey,
    search_file_type: SearchFileType,
}

#[derive(Clone)]
struct ArchiveFetcher<RT: Runtime> {
    cache_path: PathBuf,
    rt: RT,
    blocking_thread_pool: BoundedThreadPool<RT>,
    cleaner: CacheCleaner,
}

impl<RT: Runtime> ArchiveFetcher<RT> {
    /// Fetch the archive with the given key from storage, and extract it into a
    /// new directory owned by this manager. The caller is responsible for
    /// updating internal state to account for this new addition.
    ///
    /// This function is intentionally private, and should only be called via
    /// `get` by the task that was first to request this key.
    async fn fetch(
        self,
        search_storage: Arc<dyn Storage>,
        key: ObjectKey,
        search_file_type: SearchFileType,
        destination: PathBuf,
    ) -> anyhow::Result<IndexMeta> {
        let timer = metrics::archive_fetch_timer();
        let archive = search_storage
            .get(&key)
            .await?
            .context(format!("{:?} not found in search storage", key))?
            .stream;
        let extract_archive_timer = metrics::extract_archive_timer();
        let extract_archive_result = self
            .extract_archive(
                search_file_type,
                destination.clone(),
                archive.into_async_read().compat(),
            )
            .await;
        extract_archive_timer.finish();

        match extract_archive_result {
            Ok((bytes_used, path)) => {
                if is_immutable(search_file_type) {
                    set_readonly(&path, true).await?;
                }
                metrics::finish_archive_fetch(timer, bytes_used, search_file_type);
                Ok(IndexMeta {
                    path,
                    size: bytes_used,
                    cleaner: self.cleaner.clone(),
                })
            },
            Err(e) => {
                self.cleaner.attempt_cleanup(destination)?;
                Err(e)
            },
        }
    }

    async fn extract_archive(
        &self,
        search_file_type: SearchFileType,
        output_directory: PathBuf,
        archive: impl tokio::io::AsyncRead + Send + 'static + Unpin,
    ) -> anyhow::Result<(u64, PathBuf)> {
        if search_file_type == SearchFileType::FragmentedVectorSegment {
            self.extract_segment(&output_directory, archive).await
        } else {
            let bytes_copied = extract_zip(&output_directory, archive).await?;
            // Generic ZIPs are always extracted to the output directory.
            Ok((bytes_copied, output_directory))
        }
    }

    async fn extract_segment(
        &self,
        output_directory: &PathBuf,
        archive: impl tokio::io::AsyncRead + Send + 'static + Unpin,
    ) -> anyhow::Result<(u64, PathBuf)> {
        fs::create_dir(output_directory).await?;
        let output_file = output_directory.join("segment.tar");
        let bytes_copied = {
            let mut reader = BufReader::with_capacity(2 << 16, archive);
            let mut file = fs::File::create(&output_file).await?;
            let bytes_copied = tokio::io::copy_buf(&mut reader, &mut file).await?;
            file.flush().await?;
            bytes_copied
        };

        // We're expecting that the uncompressed tar and its contents are roughly the
        // same size. There is some file moving / copying going on in
        // this method, but hopefully it's small enough to be a rounding
        // error relative to the overall segment size.
        let path = Self::unpack_fragmented_segment_tar(output_file).await?;

        Ok((bytes_copied, path))
    }

    async fn unpack_fragmented_segment_tar(tar_path: PathBuf) -> anyhow::Result<PathBuf> {
        let timer = archive_untar_timer();
        let restored_path = restore_segment_from_tar(&tar_path).await?;
        fs::remove_file(tar_path).await?;
        timer.finish();
        Ok(restored_path)
    }
}

impl<RT: Runtime> ArchiveFetcher<RT> {
    async fn generate_value(
        self,
        search_storage: Arc<dyn Storage>,
        key: ObjectKey,
        search_file_type: SearchFileType,
    ) -> anyhow::Result<IndexMeta> {
        let mut timeout_fut = self.rt.wait(*ARCHIVE_FETCH_TIMEOUT_SECONDS).fuse();
        let destination = self.cache_path.join(Uuid::new_v4().simple().to_string());

        let new_destination = destination.clone();
        let new_self = self.clone();
        let new_key = key.clone();
        // Many parts of the fetch perform blocking operations. To avoid blocking the
        // calling thread's scheduling, punt all fetches to a separate OS thread.
        let fetch_fut = self
            .blocking_thread_pool
            .execute_async(move || {
                new_self.fetch(search_storage, new_key, search_file_type, new_destination)
            })
            .fuse();
        pin_mut!(fetch_fut);
        let res = select_biased! {
            meta = fetch_fut => {
                meta
            },
            _ = timeout_fut => {
                metrics::log_cache_fetch_timeout();
                tracing::error!("Timed out fetching archive for key {key:?}");
                Err(anyhow::anyhow!("Timed out")) }
        };

        if let Ok(Ok(index_meta)) = res {
            Ok(index_meta)
        } else {
            self.cleaner.attempt_cleanup(destination)?;
            res?
        }
    }
}

impl<RT: Runtime> ArchiveCacheManager<RT> {
    /// Create a new [ArchiveCacheManager] with the specified `max_size` bytes
    /// used on disk. The manager will attempt to create a directory under
    /// `storage_path` where archives will be extracted and
    /// cached. The manager expects to have sole custody over this
    /// directory, and will not observe any external changes that could
    /// affect the space usage of the cache.
    ///
    /// When callers query the cache with `.get()` and the key is not yet
    /// present on disk, the first caller will fetch the file from the provided
    /// [Storage] implementation, while all subsequent callers will queue behind
    /// this task and await a notification that the result is ready.
    ///
    /// Returns an error if the manager is unable to create a directory under
    /// `storage_path`, or if `storage_path` doesn't already exist.
    pub async fn new<P: AsRef<Path>>(
        local_storage_path: P,
        max_size: u64,
        blocking_thread_pool: BoundedThreadPool<RT>,
        max_concurrent_fetches: usize,
        rt: RT,
    ) -> anyhow::Result<Self> {
        let cleaner = CacheCleaner::new(rt.clone());
        let cache = AsyncLru::new(rt.clone(), max_size, max_concurrent_fetches, "cache");
        let this = Self {
            path: local_storage_path.as_ref().to_owned(),
            max_size,
            blocking_thread_pool,
            cache,
            cleaner,
            rt: rt.clone(),
        };
        Ok(this)
    }

    /// Returns the configured max capacity for this manager.
    #[expect(dead_code)]
    pub fn max_size(&self) -> u64 {
        self.max_size
    }

    /// Returns the current number of bytes used on disk for all directories
    /// tracked by this manager.
    #[cfg(test)]
    pub fn usage(&self) -> u64 {
        self.cache.size()
    }

    /// Get the absolute path for the directory referenced by a given key.
    /// Fetches the archive from storage if it doesn't already exist on disk.
    #[minitrace::trace]
    pub async fn get(
        &self,
        search_storage: Arc<dyn Storage>,
        key: &ObjectKey,
        search_file_type: SearchFileType,
    ) -> anyhow::Result<PathBuf> {
        let timer = metrics::archive_get_timer(search_file_type);
        let result = self.get_logged(search_storage, key, search_file_type).await;
        timer.finish(result.is_ok());
        result
    }

    #[minitrace::trace]
    pub async fn get_single_file(
        &self,
        search_storage: Arc<dyn Storage>,
        storage_path: &ObjectKey,
        file_type: SearchFileType,
    ) -> anyhow::Result<PathBuf> {
        // The archive cache always dumps things into directories, but we want a
        // specific file path.
        let parent_dir: PathBuf = self.get(search_storage, storage_path, file_type).await?;
        let mut read_dir = fs::read_dir(parent_dir).await?;
        let mut paths = Vec::with_capacity(1);
        while let Some(entry) = read_dir.next_entry().await? {
            paths.push(entry.path());
        }
        anyhow::ensure!(
            paths.len() == 1,
            "Expected one file but found multiple paths: {:?}",
            paths,
        );
        Ok(paths[0].to_owned())
    }

    async fn get_logged(
        &self,
        search_storage: Arc<dyn Storage>,
        key: &ObjectKey,
        search_file_type: SearchFileType,
    ) -> anyhow::Result<PathBuf> {
        let archive_fetcher = ArchiveFetcher {
            cache_path: self.path.clone(),
            rt: self.rt.clone(),
            blocking_thread_pool: self.blocking_thread_pool.clone(),
            cleaner: self.cleaner.clone(),
        };
        let cache_key = Key {
            path: search_storage.cache_key(key),
            search_file_type,
        };
        let result = self
            .cache
            .get(
                cache_key.clone(),
                archive_fetcher
                    .generate_value(search_storage.clone(), key.clone(), search_file_type)
                    .boxed(),
            )
            .await
            .with_context(|| {
                format!("Failed to get cache_key {cache_key:?} in {search_storage:?}")
            })?;

        let path = result.path.clone();
        let current_size = self.cache.size();
        tracing::debug!(
            "Finished fetching archive for key {key:?}, cached path: {}(space used: {} / {})",
            path.display(),
            ByteSize(current_size),
            ByteSize(self.max_size)
        );
        metrics::log_bytes_used(current_size, self.max_size);

        Ok(result.path.clone())
    }
}

fn is_immutable(search_file_type: SearchFileType) -> bool {
    match search_file_type {
        // At least one rocksdb instance used by the qdrant Segment is not
        // opened in read only mode.
        SearchFileType::VectorSegment => false,
        SearchFileType::FragmentedVectorSegment => true,
        SearchFileType::VectorDeletedBitset => true,
        SearchFileType::VectorIdTracker => true,
        // Text indexes do not appear to be read in readonly mode.
        SearchFileType::Text => false,
        SearchFileType::TextIdTracker => true,
        SearchFileType::TextAliveBitset => true,
        SearchFileType::TextDeletedTerms => true,
    }
}

async fn set_readonly(path: &PathBuf, readonly: bool) -> anyhow::Result<()> {
    let metadata = fs::metadata(path).await?;
    let mut permissions = metadata.permissions();
    permissions.set_readonly(readonly);
    fs::set_permissions(path, permissions).await?;
    Ok(())
}

#[derive(Clone)]
struct CacheCleaner {
    cleanup_tx: mpsc::UnboundedSender<PathBuf>,
    _cleanup_handle: Arc<Box<dyn SpawnHandle>>,
}

impl CacheCleaner {
    fn new<RT: Runtime>(rt: RT) -> Self {
        let (cleanup_tx, cleanup_rx) = mpsc::unbounded_channel();
        let cleanup_handle = Arc::new(rt.spawn_thread(|| cleanup_thread(cleanup_rx)));
        Self {
            cleanup_tx,
            _cleanup_handle: cleanup_handle,
        }
    }

    fn attempt_cleanup(&self, path: PathBuf) -> anyhow::Result<()> {
        Ok(self.cleanup_tx.send(path)?)
    }
}

/// Runs on a separate thread to delete archives that have been removed from the
/// in-memory cache.
/// Using a separate thread for this is just an optimization, recognizing that a
/// recursive deletion doesn't need to be in the critical path and may block the
/// for a meaningful amount of time as opposed to our other filesystem ops which
/// should be quite fast.
async fn cleanup_thread(mut rx: mpsc::UnboundedReceiver<PathBuf>) {
    while let Some(path) = rx.recv().await {
        // Yes, we'll panic and restart here. If we actually see panics in
        // production here, we should investigate further but for now, it's simpler
        // to disallow inconsistent filesystem state.
        tracing::debug!("Removing path {} from disk", path.display());
        let result: anyhow::Result<()> = try {
            set_readonly(&path, false).await?;
            fs::remove_dir_all(path).await?;
        };
        result.expect("ArchiveCacheManager failed to clean up archive directory");
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use async_zip::{
        Compression,
        ZipEntryBuilder,
    };
    use common::bounded_thread_pool::BoundedThreadPool;
    use rand::{
        distributions,
        thread_rng,
        Rng,
        RngCore,
    };
    use runtime::testing::TestRuntime;
    use storage::{
        LocalDirStorage,
        Storage,
        Upload,
    };
    use tempfile::TempDir;

    use super::ArchiveCacheManager;
    use crate::SearchFileType;

    // Creates a random ZIP archive and outputs it as a buffer, along with the size
    // of all contained files.
    async fn random_archive() -> (Vec<u8>, u64) {
        let mut buf = vec![];
        let mut writer = async_zip::write::ZipFileWriter::new(&mut buf);
        let mut size = 0u64;
        for _ in 0..thread_rng().gen_range(1..10) {
            let filename = thread_rng()
                .sample_iter(distributions::Alphanumeric)
                .take(8)
                .map(|i| i as char)
                .collect::<String>();
            let len = thread_rng().gen_range(100..1000);
            let mut content = vec![0; len];
            size += len as u64;
            thread_rng().fill_bytes(&mut content);
            let entry = ZipEntryBuilder::new(filename, Compression::Stored).build();
            writer.write_entry_whole(entry, &content).await.unwrap();
        }
        writer.close().await.unwrap();
        (buf, size)
    }

    #[convex_macro::test_runtime]
    async fn test_cache(rt: TestRuntime) -> anyhow::Result<()> {
        let root_dir = TempDir::new().unwrap();
        let storage_dir = TempDir::new().unwrap();
        let storage_dir_path = storage_dir.path().to_owned();
        let storage =
            Arc::new(LocalDirStorage::new_at_path(rt.clone(), storage_dir_path.clone()).unwrap());

        let (first_archive, first_size) = random_archive().await;
        let (second_archive, second_size) = loop {
            let (archive, size) = random_archive().await;
            if size < first_size {
                break (archive, size);
            }
        };

        let mut uploader = storage.start_upload().await?;
        uploader.write(first_archive.clone().into()).await?;
        let key = uploader.complete().await?;
        // Create the manager such that it is _just_ big enough to hold the first
        // archive.
        let manager = ArchiveCacheManager::new(
            root_dir.path(),
            first_size + 1,
            BoundedThreadPool::new(rt.clone(), 100, 10, "test"),
            1,
            rt,
        )
        .await?;
        assert_eq!(manager.usage(), 0);
        let path = manager
            .get(storage.clone(), &key, SearchFileType::Text)
            .await?;
        assert_eq!(
            manager
                .get(storage.clone(), &key, SearchFileType::Text)
                .await?,
            path
        );
        assert_eq!(manager.usage(), first_size);

        let mut uploader = storage.start_upload().await?;
        uploader.write(second_archive.clone().into()).await?;
        let second_key = uploader.complete().await?;
        let second_path = manager
            .get(storage, &second_key, SearchFileType::Text)
            .await?;
        assert_ne!(path, second_path);
        assert_eq!(manager.usage(), second_size);
        Ok(())
    }
}

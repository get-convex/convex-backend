//! This crate provides a `ZipReader` that is able to seek through a zip file
//! and read individual files. It requires the underlying reader to be seekable
//! (i.e. implement `tokio::io::AsyncSeek`).
//!
//! It is a wrapper on top of the `zip` crate. Because that crate is
//! synchronous, we spawn a thread to do the reading. It supports reading both
//! ZIP32 and ZIP64 archives, including files that have data descriptors (i.e.
//! were written using a streaming writer), as long as the archive has a valid
//! central directory.

use std::{
    io::{
        self,
        Read,
    },
    marker::PhantomData,
};

use anyhow::Context as _;
use bytes::Bytes;
use tokio::{
    io::{
        AsyncBufRead,
        AsyncRead,
        AsyncSeek,
    },
    sync::{
        mpsc,
        oneshot,
    },
    task::JoinHandle,
};
use tokio_stream::wrappers::ReceiverStream;
use tokio_util::io::{
    StreamReader,
    SyncIoBridge,
};
pub use zip::result::ZipError;

pub struct ZipReader {
    num_entries: usize,
    tx: mpsc::Sender<ZipReaderMessage>,
    _handle: JoinHandle<()>,
}

impl ZipReader {
    pub async fn new<R: AsyncRead + AsyncSeek + Unpin + Send + 'static>(
        reader: R,
    ) -> anyhow::Result<Self> {
        let (tx, rx) = mpsc::channel(8);
        let (init_tx, init_rx) = oneshot::channel();
        let rt = tokio::runtime::Handle::current();
        let handle = tokio::task::spawn_blocking(move || run_reader(rt, rx, reader, init_tx));
        let num_entries = init_rx.await??;
        Ok(Self {
            num_entries,
            tx,
            _handle: handle,
        })
    }

    pub fn is_empty(&self) -> bool {
        self.num_entries == 0
    }

    pub fn len(&self) -> usize {
        self.num_entries
    }

    pub async fn file_names(&self) -> anyhow::Result<Vec<String>> {
        let (tx, rx) = oneshot::channel();
        self.tx
            .send(ZipReaderMessage::GetFileNames { tx })
            .await
            .context("zip reader thread died")?;
        Ok(rx.await?)
    }

    /// Reads the zip entry at the given index, which should be in the range
    /// `0..self.len()`.
    ///
    /// This won't start streaming the file content unless
    /// `ZipFileEntry::read()` is called.
    ///
    /// The `ZipReader` can't be used again until the ZipFileEntry (and its
    /// reader, if requested) is dropped.
    pub async fn by_index(&mut self, index: usize) -> anyhow::Result<ZipFileEntry<'_>> {
        let (tx, rx) = oneshot::channel();
        self.tx
            .send(ZipReaderMessage::ReadEntry { index, tx })
            .await
            .context("zip reader thread died")?;
        let inner = rx.await??;
        Ok(ZipFileEntry {
            inner,
            _borrow: PhantomData,
        })
    }
}

struct ZipFileEntryInner {
    name: String,
    is_file: bool,
    is_dir: bool,
    start_reading: oneshot::Sender<mpsc::Sender<io::Result<Bytes>>>,
}

pub struct ZipFileEntry<'a> {
    inner: ZipFileEntryInner,
    _borrow: PhantomData<&'a mut ()>,
}

impl<'a> ZipFileEntry<'a> {
    pub fn name(&self) -> &str {
        &self.inner.name
    }

    pub fn is_file(&self) -> bool {
        self.inner.is_file
    }

    pub fn is_dir(&self) -> bool {
        self.inner.is_dir
    }

    /// Start streaming the content of the zip file.
    pub fn read(self) -> impl AsyncBufRead + Unpin + Send + 'a {
        let (tx, rx) = mpsc::channel(1);
        _ = self.inner.start_reading.send(tx);
        StreamReader::new(ReceiverStream::new(rx))
    }
}

enum ZipReaderMessage {
    GetFileNames {
        tx: oneshot::Sender<Vec<String>>,
    },
    ReadEntry {
        index: usize,
        tx: oneshot::Sender<anyhow::Result<ZipFileEntryInner>>,
    },
}

const READ_BUF_SIZE: usize = 64 * 1024; // 64KiB

fn run_reader<R: AsyncRead + AsyncSeek + Unpin + Send + 'static>(
    rt: tokio::runtime::Handle,
    mut rx: mpsc::Receiver<ZipReaderMessage>,
    reader: R,
    init_tx: oneshot::Sender<anyhow::Result<usize>>,
) {
    let reader = SyncIoBridge::new_with_handle(reader, rt);
    let mut reader = match zip::read::ZipArchive::new(reader) {
        Ok(r) => {
            _ = init_tx.send(Ok(r.len()));
            r
        },
        Err(e) => {
            _ = init_tx.send(Err(e.into()));
            return;
        },
    };
    while let Some(msg) = rx.blocking_recv() {
        match msg {
            ZipReaderMessage::GetFileNames { tx } => {
                _ = tx.send(reader.file_names().map(|s| s.to_owned()).collect());
            },
            ZipReaderMessage::ReadEntry { index, tx } => {
                match reader.by_index(index).context("Failed to open zip entry") {
                    Ok(mut entry) => {
                        let (data_tx_tx, data_tx_rx) = oneshot::channel();
                        _ = tx.send(Ok(ZipFileEntryInner {
                            name: entry.name().to_owned(),
                            is_file: entry.is_file(),
                            is_dir: entry.is_dir(),
                            start_reading: data_tx_tx,
                        }));
                        // Wait for the caller to request it before reading content from the zip
                        // file
                        let Ok(data_tx) = data_tx_rx.blocking_recv() else {
                            // The caller did not call `.read()`
                            continue;
                        };
                        loop {
                            let mut buf = vec![0; READ_BUF_SIZE];
                            match read_as_much_as_possible(&mut entry, &mut buf) {
                                Ok(0) => break, // EOF
                                Ok(n) => {
                                    buf.truncate(n);
                                    if data_tx.blocking_send(Ok(buf.into())).is_err() {
                                        // Receiver has dropped
                                        break;
                                    }
                                },
                                Err(e) => {
                                    _ = data_tx.blocking_send(Err(e));
                                    break;
                                },
                            }
                        }
                    },
                    Err(e) => {
                        _ = tx.send(Err(e));
                    },
                }
            },
        }
    }
}

fn read_as_much_as_possible(reader: &mut impl Read, buf: &mut [u8]) -> io::Result<usize> {
    let mut total = 0;
    while total < buf.len() {
        let n = reader.read(&mut buf[total..])?;
        if n == 0 {
            break;
        }
        total += n;
    }
    Ok(total)
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use async_zip::{
        tokio::write::ZipFileWriter,
        Compression,
        ZipEntryBuilder,
    };
    use futures::AsyncWriteExt as _;
    use tokio::io::{
        AsyncReadExt,
        AsyncWriteExt as _,
    };

    use crate::ZipReader;

    #[tokio::test]
    async fn test_can_read_async_zip() -> anyhow::Result<()> {
        let mut buf = Vec::new();

        let mut writer = ZipFileWriter::with_tokio(&mut buf);
        for i in 0..100 {
            let name = format!("foo{i}");
            let builder = ZipEntryBuilder::new(
                name.into(),
                if i % 5 == 0 {
                    // Create some entries with zstd compression
                    Compression::Zstd
                } else {
                    Compression::Deflate
                },
            )
            .unix_permissions(0o644);
            // Create some entries with streaming. This results in local file headers
            // that are missing file lengths and CRCs - those are written to a following
            // data descriptor instead, and are also written to the central directory.
            if i % 3 == 0 {
                let mut file_writer = writer.write_entry_stream(builder.build()).await?;
                file_writer.write_all(b"Hello ").await?;
                file_writer
                    .write_all(format!("world {i}").as_bytes())
                    .await?;
                file_writer.close().await?;
            } else {
                writer
                    .write_entry_whole(builder.build(), format!("Hello world {i}").as_bytes())
                    .await?;
            }
        }
        writer.close().await?;

        let mut reader = ZipReader::new(Cursor::new(buf)).await?;
        assert_eq!(reader.len(), 100);
        for i in 0..100 {
            let entry = reader.by_index(i).await?;
            assert!(entry.is_file());
            assert_eq!(entry.name(), format!("foo{i}"));
            // Exercise the case where we only choose to read some files
            if i % 2 == 0 {
                let mut data = String::new();
                entry.read().read_to_string(&mut data).await?;
                assert_eq!(data, format!("Hello world {i}"));
            }
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_can_read_old_async_zip() -> anyhow::Result<()> {
        // Test that `ZipReader` can read zip files created with `async_zip` version
        // 0.0.9, which creates ZIP32 archives
        let mut buf = Vec::new();

        let mut writer = async_zip_0_0_9::write::ZipFileWriter::new(&mut buf);
        writer
            .write_entry_whole(
                async_zip_0_0_9::ZipEntryBuilder::new(
                    "whole".into(),
                    async_zip_0_0_9::Compression::Deflate,
                ),
                b"whole",
            )
            .await?;
        let mut entry = writer
            .write_entry_stream(async_zip_0_0_9::ZipEntryBuilder::new(
                "stream".into(),
                async_zip_0_0_9::Compression::Deflate,
            ))
            .await?;
        entry.write_all(b"stream").await?;
        entry.close().await?;
        writer.close().await?;

        let mut reader = ZipReader::new(Cursor::new(buf)).await?;
        assert_eq!(reader.len(), 2);

        let entry = reader.by_index(0).await?;
        assert!(entry.is_file());
        assert_eq!(entry.name(), "whole");
        let mut data = String::new();
        entry.read().read_to_string(&mut data).await?;
        assert_eq!(data, "whole");

        let entry = reader.by_index(1).await?;
        assert!(entry.is_file());
        assert_eq!(entry.name(), "stream");
        let mut data = String::new();
        entry.read().read_to_string(&mut data).await?;
        assert_eq!(data, "stream");

        Ok(())
    }
}

use std::sync::Arc;

use async_zip::{
    tokio::write::ZipFileWriter,
    Compression,
    ZipEntryBuilder,
};
use bytes::Bytes;
use common::runtime::testing::TestRuntime;
use futures::AsyncWriteExt as _;
use rc_zip::parse::EntryKind;
use storage::{
    ChannelWriter,
    LocalDirStorage,
    Storage,
    Upload,
    UploadExt as _,
};
use tokio::{
    io::{
        AsyncReadExt as _,
        AsyncWriteExt,
    },
    sync::mpsc,
};
use tokio_stream::wrappers::ReceiverStream;

use crate::StorageZipArchive;

#[convex_macro::test_runtime]
async fn test_can_read_async_zip(rt: TestRuntime) -> anyhow::Result<()> {
    let storage = LocalDirStorage::new(rt)?;

    let mut upload = storage.start_upload().await?;
    let (sender, receiver) = mpsc::channel::<Bytes>(1);

    let write_task = common::runtime::tokio_spawn("make_zip", async move {
        let mut writer = ZipFileWriter::with_tokio(ChannelWriter::new(sender, 1024));
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
        let mut inner = writer.close().await?.into_inner();
        inner.shutdown().await?;
        anyhow::Ok(())
    });
    upload.write_parallel(ReceiverStream::new(receiver)).await?;
    write_task.await??;
    let key = upload.complete().await?;

    let reader = StorageZipArchive::open(Arc::new(storage), &key).await?;
    assert_eq!(reader.entries().count(), 100);
    for (i, entry) in reader.entries().enumerate() {
        assert_eq!(entry.kind(), EntryKind::File);
        assert_eq!(entry.name, format!("foo{i}"));
        // Exercise the case where we only choose to read some files
        if i % 2 == 0 {
            let mut data = String::new();
            reader
                .read_entry(entry.clone())
                .read_to_string(&mut data)
                .await?;
            assert_eq!(data, format!("Hello world {i}"));
        }
    }

    Ok(())
}

#[convex_macro::test_runtime]
async fn test_can_read_old_async_zip(rt: TestRuntime) -> anyhow::Result<()> {
    // Test that `StorageZipArchive` can read zip files created with `async_zip`
    // version 0.0.9, which creates ZIP32 archives
    let storage = LocalDirStorage::new(rt)?;

    let mut upload = storage.start_upload().await?;
    let (sender, receiver) = mpsc::channel::<Bytes>(1);

    let write_task = common::runtime::tokio_spawn("make_zip", async move {
        let mut inner = ChannelWriter::new(sender, 1024);
        let mut writer = async_zip_0_0_9::write::ZipFileWriter::new(&mut inner);
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
        inner.shutdown().await?;
        anyhow::Ok(())
    });
    upload.write_parallel(ReceiverStream::new(receiver)).await?;
    write_task.await??;
    let key = upload.complete().await?;

    let reader = StorageZipArchive::open(Arc::new(storage), &key).await?;
    assert_eq!(reader.entries().count(), 2);

    let entry = reader.entries().next().unwrap();
    assert_eq!(entry.kind(), EntryKind::File);
    assert_eq!(entry.name, "whole");
    let mut data = String::new();
    reader
        .read_entry(entry.clone())
        .read_to_string(&mut data)
        .await?;
    assert_eq!(data, "whole");

    let entry = reader.entries().nth(1).unwrap();
    assert_eq!(entry.kind(), EntryKind::File);
    assert_eq!(entry.name, "stream");
    let mut data = String::new();
    reader
        .read_entry(entry.clone())
        .read_to_string(&mut data)
        .await?;
    assert_eq!(data, "stream");

    Ok(())
}

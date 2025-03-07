use std::collections::BTreeMap;

use anyhow::Context;
use bytes::Bytes;
use common::{
    self,
    components::ComponentPath,
    document::ParsedDocument,
    knobs::{
        EXPORT_MAX_INFLIGHT_PREFETCH_BYTES,
        EXPORT_STORAGE_GET_CONCURRENCY,
    },
    persistence::LatestDocument,
    runtime::Runtime,
    types::{
        IndexId,
        RepeatableTimestamp,
        TableName,
    },
};
use fastrace::{
    future::FutureExt,
    Span,
};
use futures::{
    pin_mut,
    stream,
    StreamExt,
    TryStreamExt,
};
use mime2ext::mime2ext;
use model::{
    exports::types::ExportRequestor,
    file_storage::{
        types::FileStorageEntry,
        FILE_STORAGE_TABLE,
        FILE_STORAGE_VIRTUAL_TABLE,
    },
};
use serde::{
    Deserialize,
    Serialize,
};
use serde_json::json;
use storage::StorageExt;
use tokio_util::io::StreamReader;
use usage_tracking::{
    FunctionUsageTracker,
    StorageCallTracker,
    StorageUsageTracker,
};
use value::{
    TableNamespace,
    TabletId,
};

use crate::exports::{
    worker::ExportWorker,
    zip_uploader::ZipSnapshotUpload,
};

pub async fn write_storage_table<'a, 'b: 'a, RT: Runtime>(
    worker: &ExportWorker<RT>,
    path_prefix: &str,
    zip_snapshot_upload: &'a mut ZipSnapshotUpload<'b>,
    namespace: TableNamespace,
    component_path: &ComponentPath,
    snapshot_ts: RepeatableTimestamp,
    by_id_indexes: &BTreeMap<TabletId, IndexId>,
    system_tables: &BTreeMap<(TableNamespace, TableName), TabletId>,
    usage: &FunctionUsageTracker,
    requestor: ExportRequestor,
) -> anyhow::Result<()> {
    // _storage
    let tablet_id = system_tables
        .get(&(namespace, FILE_STORAGE_TABLE.clone()))
        .context("_file_storage does not exist")?;
    let by_id = by_id_indexes
        .get(tablet_id)
        .context("_file_storage.by_id does not exist")?;

    // First write metadata to _storage/documents.jsonl
    let mut table_upload = zip_snapshot_upload
        .start_system_table(path_prefix, FILE_STORAGE_VIRTUAL_TABLE.clone())
        .await?;
    let table_iterator = worker.database.table_iterator(snapshot_ts, 1000);
    let stream = table_iterator.stream_documents_in_table(*tablet_id, *by_id, None);
    pin_mut!(stream);
    while let Some(LatestDocument { value: doc, .. }) = stream.try_next().await? {
        let file_storage_entry = ParsedDocument::<FileStorageEntry>::try_from(doc)?;
        let virtual_storage_id = file_storage_entry.id().developer_id;
        let creation_time = f64::from(file_storage_entry.creation_time());
        table_upload
            .write_json_line(json!(FileStorageZipMetadata {
                id: virtual_storage_id.encode(),
                creation_time: Some(creation_time),
                sha256: Some(file_storage_entry.sha256.as_base64()),
                size: Some(file_storage_entry.size),
                content_type: file_storage_entry.content_type.clone(),
                internal_id: Some(file_storage_entry.storage_id.to_string()),
            }))
            .await?;
    }
    table_upload.complete().await?;

    let table_iterator = worker.database.table_iterator(snapshot_ts, 1000);
    let max_prefetch_bytes = *EXPORT_MAX_INFLIGHT_PREFETCH_BYTES;
    let inflight_bytes_semaphore = tokio::sync::Semaphore::new(max_prefetch_bytes);
    let files_stream = table_iterator
        .stream_documents_in_table(*tablet_id, *by_id, None)
        .map_ok(|LatestDocument { value: doc, .. }| async {
            let file_storage_entry = ParsedDocument::<FileStorageEntry>::try_from(doc)?;
            let virtual_storage_id = file_storage_entry.id().developer_id;
            // Add an extension, which isn't necessary for anything and might be incorrect,
            // but allows the file to be viewed at a glance in most cases.
            let extension_guess = file_storage_entry
                .content_type
                .as_ref()
                .and_then(mime2ext)
                .map(|extension| format!(".{extension}"))
                .unwrap_or_default();
            let path = format!(
                "{path_prefix}{}/{}{extension_guess}",
                *FILE_STORAGE_VIRTUAL_TABLE,
                virtual_storage_id.encode()
            );
            let file_stream = worker
                .file_storage
                .get(&file_storage_entry.storage_key)
                .await?
                .with_context(|| {
                    format!(
                        "file missing from storage: {} with key {:?}",
                        file_storage_entry.developer_id().encode(),
                        file_storage_entry.storage_key,
                    )
                })?;

            let content_type = file_storage_entry
                .content_type
                .as_ref()
                .map(|ct| ct.parse())
                .transpose()?;
            usage.track_storage_call(
                component_path.clone(),
                requestor.usage_tag(),
                file_storage_entry.storage_id.clone(),
                content_type,
                file_storage_entry.sha256.clone(),
            );
            usage.track_storage_egress_size(
                component_path.clone(),
                requestor.usage_tag().to_string(),
                file_stream.content_length as u64,
            );

            if (file_stream.content_length as usize) < max_prefetch_bytes {
                let permit = inflight_bytes_semaphore
                    .acquire_many(file_stream.content_length as u32)
                    .await?;
                // Prefetch the file before passing it to the zip writer.
                // This can happen in parallel with other files.
                let bytes: Vec<Bytes> = file_stream
                    .stream
                    .try_collect()
                    .in_span(Span::enter_with_local_parent("prefetch_storage_file"))
                    .await?;
                let stream = StreamReader::new(stream::iter(bytes.into_iter().map(Ok)).boxed());
                Ok((path, stream, permit))
            } else {
                // Wait until all other ongoing prefetches are finished, then stream this file
                // serially.
                let permit = inflight_bytes_semaphore
                    .acquire_many(max_prefetch_bytes as u32)
                    .await?;
                // Note that fetching won't start until the reader is first polled (which won't
                // happen until it's passed to `stream_full_file`).
                Ok((path, file_stream.into_tokio_reader(), permit))
            }
        })
        .try_buffer_unordered(*EXPORT_STORAGE_GET_CONCURRENCY); // Note that this will return entries in an arbitrary order
    pin_mut!(files_stream);
    while let Some((path, file_stream, permit)) = files_stream.try_next().await? {
        zip_snapshot_upload
            .stream_full_file(path, file_stream)
            .await?;
        drop(permit);
    }
    Ok(())
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileStorageZipMetadata {
    #[serde(rename = "_id")]
    pub id: String,
    #[serde(rename = "_creationTime")]
    pub creation_time: Option<f64>,
    pub sha256: Option<String>,
    pub size: Option<i64>,
    pub content_type: Option<String>,
    pub internal_id: Option<String>,
}

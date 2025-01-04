use std::collections::{
    BTreeMap,
    BTreeSet,
};

use anyhow::Context;
use bytes::Bytes;
use common::{
    self,
    async_compat::TokioAsyncWriteCompatExt,
    bootstrap_model::tables::TABLES_TABLE,
    components::{
        ComponentId,
        ComponentPath,
    },
    document::ParsedDocument,
    runtime::Runtime,
    types::{
        IndexId,
        ObjectKey,
        RepeatableTimestamp,
        TableName,
        Timestamp,
    },
};
use database::{
    IndexModel,
    TableSummary,
    COMPONENTS_TABLE,
};
use futures::{
    pin_mut,
    try_join,
    AsyncWriteExt,
    Future,
    StreamExt,
    TryStreamExt,
};
use itertools::Itertools;
use keybroker::Identity;
use mime2ext::mime2ext;
use model::{
    exports::types::{
        ExportFormat,
        ExportRequestor,
    },
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
use shape_inference::export_context::{
    ExportContext,
    GeneratedSchema,
};
use storage::{
    ChannelWriter,
    StorageExt,
    Upload,
    UploadExt,
};
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use usage_tracking::{
    FunctionUsageTracker,
    StorageUsageTracker,
};
use value::{
    TableNamespace,
    TableNumber,
    TabletId,
};

use crate::exports::{
    worker::ExportWorker,
    zip_uploader::ZipSnapshotUpload,
};

#[cfg(test)]
mod tests;
pub mod worker;
mod zip_uploader;

async fn export_inner<F, Fut, RT: Runtime>(
    worker: &mut ExportWorker<RT>,
    format: ExportFormat,
    requestor: ExportRequestor,
    update_progress: F,
) -> anyhow::Result<(Timestamp, ObjectKey, FunctionUsageTracker)>
where
    F: Fn(String) -> Fut + Send + Copy,
    Fut: Future<Output = anyhow::Result<()>> + Send,
{
    let storage = &worker.storage;
    update_progress("Beginning backup".to_string()).await?;
    let (ts, tables, component_ids_to_paths, by_id_indexes, system_tables) = {
        let mut tx = worker.database.begin(Identity::system()).await?;
        let by_id_indexes = IndexModel::new(&mut tx).by_id_indexes().await?;
        let snapshot = worker.database.snapshot(tx.begin_timestamp())?;
        let table_summaries = snapshot.must_table_summaries()?;
        let tables: BTreeMap<_, _> = snapshot
            .table_registry
            .iter_active_user_tables()
            .map(|(tablet_id, table_namespace, table_number, table_name)| {
                (
                    tablet_id,
                    (
                        table_namespace,
                        table_number,
                        table_name.clone(),
                        table_summaries.tablet_summary(&tablet_id),
                    ),
                )
            })
            .collect();
        let component_ids_to_paths = snapshot.component_ids_to_paths();
        let system_tables = snapshot
            .table_registry
            .iter_active_system_tables()
            .map(|(id, namespace, _, name)| ((namespace, name.clone()), id))
            .collect();
        (
            tx.begin_timestamp(),
            tables,
            component_ids_to_paths,
            by_id_indexes,
            system_tables,
        )
    };
    match format {
        ExportFormat::Zip { include_storage } => {
            // Start upload.
            let mut upload = storage.start_upload().await?;
            let (sender, receiver) = mpsc::channel::<Bytes>(1);
            let uploader =
                upload.try_write_parallel_and_hash(ReceiverStream::new(receiver).map(Ok));
            let writer = ChannelWriter::new(sender, 5 * (1 << 20));
            let usage = FunctionUsageTracker::new();

            let zipper = construct_zip_snapshot(
                worker,
                writer,
                tables.clone(),
                component_ids_to_paths,
                ts,
                by_id_indexes,
                system_tables,
                include_storage,
                usage.clone(),
                requestor,
                update_progress,
            );
            let (_, ()) = try_join!(uploader, zipper)?;
            let zip_object_key = upload.complete().await?;
            Ok((*ts, zip_object_key, usage))
        },
    }
}

async fn write_component<'a, 'b: 'a, F, Fut, RT: Runtime>(
    worker: &ExportWorker<RT>,
    namespace: TableNamespace,
    component_path: ComponentPath,
    zip_snapshot_upload: &'a mut ZipSnapshotUpload<'b>,
    tables: &'a mut BTreeMap<TabletId, (TableNamespace, TableNumber, TableName, TableSummary)>,
    snapshot_ts: RepeatableTimestamp,
    by_id_indexes: &BTreeMap<TabletId, IndexId>,
    system_tables: &BTreeMap<(TableNamespace, TableName), TabletId>,
    include_storage: bool,
    usage: &FunctionUsageTracker,
    requestor: ExportRequestor,
    update_progress: F,
) -> anyhow::Result<()>
where
    F: Fn(String) -> Fut + Send + Copy,
    Fut: Future<Output = anyhow::Result<()>> + Send,
{
    let path_prefix = get_export_path_prefix(&component_path);

    let in_component_str = component_path.in_component_str();
    let tablet_ids: BTreeSet<_> = tables
        .iter()
        .filter(|(_, (ns, ..))| *ns == namespace)
        .map(|(tablet_id, _)| *tablet_id)
        .collect();

    {
        update_progress(format!("Enumerating tables{in_component_str}")).await?;
        // _tables
        let mut table_upload = zip_snapshot_upload
            .start_system_table(&path_prefix, TABLES_TABLE.clone())
            .await?;

        // Write documents from stream to table uploads, in table number order.
        // This includes all user tables present in the export.
        let mut user_table_numbers_and_names: Vec<_> = tables
            .iter()
            .filter(|(_, (ns, ..))| *ns == namespace)
            .map(|(_, (_, table_number, table_name, _))| (table_number, table_name))
            .collect();
        user_table_numbers_and_names.sort();
        for (table_number, table_name) in user_table_numbers_and_names {
            table_upload
                .write_json_line(json!({
                    "name": table_name.clone(),
                    "id": *table_number,
                }))
                .await?;
        }
        table_upload.complete().await?;
    }

    if include_storage {
        update_progress(format!("Backing up _storage{in_component_str}")).await?;

        // _storage
        let tablet_id = system_tables
            .get(&(namespace, FILE_STORAGE_TABLE.clone()))
            .context("_file_storage does not exist")?;
        let by_id = by_id_indexes
            .get(tablet_id)
            .context("_file_storage.by_id does not exist")?;

        // First write metadata to _storage/documents.jsonl
        let mut table_upload = zip_snapshot_upload
            .start_system_table(&path_prefix, FILE_STORAGE_VIRTUAL_TABLE.clone())
            .await?;
        let table_iterator = worker.database.table_iterator(snapshot_ts, 1000, None);
        let stream = table_iterator.stream_documents_in_table(*tablet_id, *by_id, None);
        pin_mut!(stream);
        while let Some((doc, _ts)) = stream.try_next().await? {
            let file_storage_entry = ParsedDocument::<FileStorageEntry>::try_from(doc)?;
            let virtual_storage_id = file_storage_entry.id().developer_id;
            let creation_time = f64::from(
                file_storage_entry
                    .creation_time()
                    .context("file should have creation time")?,
            );
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

        let table_iterator = worker.database.table_iterator(snapshot_ts, 1000, None);
        let stream = table_iterator.stream_documents_in_table(*tablet_id, *by_id, None);
        pin_mut!(stream);
        while let Some((doc, _ts)) = stream.try_next().await? {
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
            worker.usage_tracking.track_independent_storage_egress_size(
                component_path.clone(),
                requestor.usage_tag().to_string(),
                file_stream.content_length as u64,
            );
            zip_snapshot_upload
                .stream_full_file(path, file_stream.stream)
                .await?;
        }
    }

    for tablet_id in tablet_ids.iter() {
        let (_, _, table_name, table_summary) =
            tables.remove(tablet_id).expect("table should have details");
        let by_id = by_id_indexes
            .get(tablet_id)
            .ok_or_else(|| anyhow::anyhow!("no by_id index for {} found", tablet_id))?;

        update_progress(format!("Backing up {table_name}{in_component_str}")).await?;

        let mut generated_schema = GeneratedSchema::new(table_summary.inferred_type().into());
        if ExportContext::is_ambiguous(table_summary.inferred_type()) {
            let table_iterator = worker.database.table_iterator(snapshot_ts, 1000, None);
            let stream = table_iterator.stream_documents_in_table(*tablet_id, *by_id, None);
            pin_mut!(stream);
            while let Some((doc, _ts)) = stream.try_next().await? {
                generated_schema.insert(doc.value(), doc.developer_id());
            }
        }

        let mut table_upload = zip_snapshot_upload
            .start_table(&path_prefix, table_name.clone(), generated_schema)
            .await?;

        let table_iterator = worker.database.table_iterator(snapshot_ts, 1000, None);
        let stream = table_iterator.stream_documents_in_table(*tablet_id, *by_id, None);
        pin_mut!(stream);

        // Write documents from stream to table uploads
        while let Some((doc, _ts)) = stream.try_next().await? {
            usage.track_database_egress_size(
                component_path.clone(),
                table_name.to_string(),
                doc.size() as u64,
                false,
            );
            table_upload.write(doc).await?;
        }
        table_upload.complete().await?;
    }

    Ok(())
}

async fn construct_zip_snapshot<F, Fut, RT: Runtime>(
    worker: &ExportWorker<RT>,
    mut writer: ChannelWriter,
    mut tables: BTreeMap<TabletId, (TableNamespace, TableNumber, TableName, TableSummary)>,
    component_ids_to_paths: BTreeMap<ComponentId, ComponentPath>,
    snapshot_ts: RepeatableTimestamp,
    by_id_indexes: BTreeMap<TabletId, IndexId>,
    system_tables: BTreeMap<(TableNamespace, TableName), TabletId>,
    include_storage: bool,
    usage: FunctionUsageTracker,
    requestor: ExportRequestor,
    update_progress: F,
) -> anyhow::Result<()>
where
    F: Fn(String) -> Fut + Send + Copy,
    Fut: Future<Output = anyhow::Result<()>> + Send,
{
    let mut zip_snapshot_upload = ZipSnapshotUpload::new(&mut writer).await?;

    for (component_id, component_path) in component_ids_to_paths {
        let namespace: TableNamespace = component_id.into();
        write_component(
            worker,
            namespace,
            component_path,
            &mut zip_snapshot_upload,
            &mut tables,
            snapshot_ts,
            &by_id_indexes,
            &system_tables,
            include_storage,
            &usage,
            requestor,
            update_progress,
        )
        .await?;
    }

    // Complete upload.
    zip_snapshot_upload.complete().await?;
    writer.compat_write().close().await?;
    Ok(())
}

fn get_export_path_prefix(component_path: &ComponentPath) -> String {
    component_path
        .iter()
        .map(|parent_name| {
            format!(
                "{}/{}/",
                &*COMPONENTS_TABLE,
                String::from(parent_name.clone())
            )
        })
        .join("")
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

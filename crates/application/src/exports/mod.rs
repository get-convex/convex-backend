use std::collections::BTreeMap;

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
    minitrace_helpers::get_sampled_span,
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
use maplit::btreemap;
use minitrace::future::FutureExt;
use model::exports::types::{
    ExportFormat,
    ExportRequestor,
};
use serde_json::json;
use shape_inference::export_context::{
    ExportContext,
    GeneratedSchema,
};
use storage::{
    ChannelWriter,
    Upload,
    UploadExt,
};
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use usage_tracking::FunctionUsageTracker;
use value::{
    InternalId,
    TableNamespace,
    TableNumber,
    TabletId,
};

use crate::exports::{
    export_storage::write_storage_table,
    worker::ExportWorker,
    zip_uploader::ZipSnapshotUpload,
};

mod export_storage;
#[cfg(test)]
mod tests;
pub mod worker;
mod zip_uploader;

pub use export_storage::FileStorageZipMetadata;

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

async fn write_tables_table<'a, 'b: 'a>(
    path_prefix: &str,
    zip_snapshot_upload: &'a mut ZipSnapshotUpload<'b>,
    namespace: TableNamespace,
    tables: &'a BTreeMap<TabletId, (TableNamespace, TableNumber, TableName, TableSummary)>,
) -> anyhow::Result<()> {
    // _tables
    let mut table_upload = zip_snapshot_upload
        .start_system_table(path_prefix, TABLES_TABLE.clone())
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
    Ok(())
}

pub async fn write_table<'a, 'b: 'a, RT: Runtime>(
    worker: &ExportWorker<RT>,
    path_prefix: &str,
    zip_snapshot_upload: &'a mut ZipSnapshotUpload<'b>,
    snapshot_ts: RepeatableTimestamp,
    component_path: &ComponentPath,
    tablet_id: &TabletId,
    table_name: TableName,
    table_summary: TableSummary,
    by_id: &InternalId,
    usage: &FunctionUsageTracker,
) -> anyhow::Result<()> {
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
        .start_table(path_prefix, table_name.clone(), generated_schema)
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
    Ok(())
}

async fn construct_zip_snapshot<F, Fut, RT: Runtime>(
    worker: &ExportWorker<RT>,
    mut writer: ChannelWriter,
    tables: BTreeMap<TabletId, (TableNamespace, TableNumber, TableName, TableSummary)>,
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

    // Aim to write things in fast -> slow order in the zip snapshot. This is
    // helpful, because TableIterator has an overhead proportional to the time
    // since `snapshot_ts`. We create many TableIterator while constructing a
    // zip snapshot, so it is helpful to do this.

    // Backup all the tables-tables. These are generally small.
    for (component_id, component_path) in component_ids_to_paths.iter() {
        let namespace: TableNamespace = (*component_id).into();
        let path_prefix = get_export_path_prefix(component_path);
        let in_component_str = component_path.in_component_str();

        update_progress(format!("Backing up _tables{in_component_str}")).await?;
        let root = get_sampled_span(
            &worker.instance_name,
            "export_worker/write_table",
            &mut worker.runtime.rng(),
            btreemap! {
                "dev.convex.component_path".to_string() => component_path.to_string(),
                "dev.convex.table_name".to_string() => "_tables".to_string(),
            },
        );
        write_tables_table(&path_prefix, &mut zip_snapshot_upload, namespace, &tables)
            .in_span(root)
            .await?;
    }

    // sort tables small to large, and write them to the zip.
    let mut sorted_tables: Vec<_> = tables.iter().collect();
    sorted_tables.sort_by_key(|(_, (_, _, _, table_summary))| table_summary.total_size());
    for (tablet_id, (namespace, _, table_name, table_summary)) in sorted_tables {
        let component_id: ComponentId = (*namespace).into();
        let component_path = component_ids_to_paths
            .get(&component_id)
            .context("Component missing")?;
        let path_prefix = get_export_path_prefix(component_path);
        let by_id = by_id_indexes
            .get(tablet_id)
            .ok_or_else(|| anyhow::anyhow!("no by_id index for {} found", tablet_id))?;

        let root = get_sampled_span(
            &worker.instance_name,
            "export_worker/write_table",
            &mut worker.runtime.rng(),
            btreemap! {
                "dev.convex.component_path".to_string() => component_path.to_string(),
                "dev.convex.table_name".to_string() => table_name.to_string(),
            },
        );
        write_table(
            worker,
            &path_prefix,
            &mut zip_snapshot_upload,
            snapshot_ts,
            component_path,
            tablet_id,
            table_name.clone(),
            table_summary.clone(),
            by_id,
            &usage,
        )
        .in_span(root)
        .await?;
    }

    // Backup the storage tables last - since the upload/download can be slower
    if include_storage {
        for (component_id, component_path) in component_ids_to_paths {
            let namespace: TableNamespace = component_id.into();
            let path_prefix = get_export_path_prefix(&component_path);
            let in_component_str = component_path.in_component_str();
            update_progress(format!("Backing up _storage{in_component_str}")).await?;

            let root = get_sampled_span(
                &worker.instance_name,
                "export_worker/write_table",
                &mut worker.runtime.rng(),
                btreemap! {
                    "dev.convex.component_path".to_string() => component_path.to_string(),
                    "dev.convex.table_name".to_string() => "_storage".to_string(),
                },
            );
            write_storage_table(
                worker,
                &path_prefix,
                &mut zip_snapshot_upload,
                namespace,
                &component_path,
                snapshot_ts,
                &by_id_indexes,
                &system_tables,
                &usage,
                requestor,
            )
            .in_span(root)
            .await?;
        }
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

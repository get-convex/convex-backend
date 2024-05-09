use std::{
    collections::{
        BTreeMap,
        BTreeSet,
    },
    sync::Arc,
    time::Duration,
};

use anyhow::Context;
use async_zip::{
    write::{
        EntryStreamWriter,
        ZipFileWriter,
    },
    Compression,
    ZipEntryBuilder,
    ZipEntryBuilderExt,
};
use bytes::Bytes;
use common::{
    async_compat::TokioAsyncWriteCompatExt,
    backoff::Backoff,
    bootstrap_model::tables::TABLES_TABLE,
    document::{
        ParsedDocument,
        ResolvedDocument,
    },
    errors::report_error,
    execution_context::ExecutionId,
    maybe_val,
    query::{
        IndexRange,
        IndexRangeExpression,
        Order,
    },
    runtime::{
        new_rate_limiter,
        Runtime,
    },
    types::{
        IndexId,
        ObjectKey,
        RepeatableTimestamp,
        TableName,
        Timestamp,
        UdfIdentifier,
    },
};
use database::{
    Database,
    IndexModel,
    ResolvedQuery,
    SystemMetadataModel,
    TableSummary,
    Transaction,
};
use futures::{
    channel::mpsc,
    pin_mut,
    stream::BoxStream,
    try_join,
    AsyncWriteExt,
    Future,
    StreamExt,
    TryStreamExt,
};
use governor::Quota;
use keybroker::Identity;
use mime2ext::mime2ext;
use model::{
    exports::{
        types::{
            Export,
            ExportFormat,
            ExportObjectKeys,
        },
        EXPORTS_BY_STATE_AND_TS_INDEX,
        EXPORTS_STATE_FIELD,
        EXPORTS_TS_FIELD,
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
use serde_json::{
    json,
    Value as JsonValue,
};
use shape_inference::{
    export_context::{
        ExportContext,
        GeneratedSchema,
    },
    ShapeConfig,
};
use storage::{
    ChannelWriter,
    Storage,
    StorageExt,
    Upload,
    UploadExt,
};
use usage_tracking::{
    CallType,
    FunctionUsageTracker,
    StorageUsageTracker,
    UsageCounter,
};
use value::{
    export::ValueFormat,
    id_v6::DeveloperDocumentId,
    TableNumber,
    TabletId,
    VirtualTableMapping,
};

use crate::metrics::{
    export_timer,
    log_worker_starting,
};

const INITIAL_BACKOFF: Duration = Duration::from_secs(1);
const MAX_BACKOFF: Duration = Duration::from_secs(900); // 15 minutes
static BEGIN_JSON_ARRAY: Bytes = Bytes::from_static("[\n".as_bytes());
static BETWEEN_DOCUMENTS: Bytes = Bytes::from_static(",\n".as_bytes());
static END_JSON_ARRAY: Bytes = Bytes::from_static("\n]\n".as_bytes());
static AFTER_DOCUMENTS_CLEAN: Bytes = Bytes::from_static("\n".as_bytes());

// 0o644 => read-write for owner, read for everyone else.
const ZIP_ENTRY_PERMISSIONS: u16 = 0o644;

static README_MD_CONTENTS: &str = r#"# Welcome to your Convex snapshot export!

This ZIP file contains a snapshot of the tables in your Convex deployment.

Documents for each table are listed as lines of JSON in
<table_name>/documents.jsonl files.

For details on the format and how to use this snapshot with npx convex import,
check out [the docs](https://docs.convex.dev/database/import-export/export) or
ask us in [Discord](http://convex.dev/community).
"#;

pub struct ExportWorker<RT: Runtime> {
    runtime: RT,
    database: Database<RT>,
    storage: Arc<dyn Storage>,
    file_storage: Arc<dyn Storage>,
    backoff: Backoff,
    usage_tracking: UsageCounter,
}

impl<RT: Runtime> ExportWorker<RT> {
    #[allow(clippy::new_ret_no_self)]
    pub fn new(
        runtime: RT,
        database: Database<RT>,
        storage: Arc<dyn Storage>,
        file_storage: Arc<dyn Storage>,
        usage_tracking: UsageCounter,
    ) -> impl Future<Output = ()> + Send {
        let mut worker = Self {
            runtime,
            database,
            storage,
            file_storage,
            backoff: Backoff::new(INITIAL_BACKOFF, MAX_BACKOFF),
            usage_tracking,
        };
        async move {
            loop {
                if let Err(e) = worker.run().await {
                    report_error(&mut e.context("ExportWorker died"));
                    let delay = worker.runtime.with_rng(|rng| worker.backoff.fail(rng));
                    worker.runtime.wait(delay).await;
                } else {
                    worker.backoff.reset();
                }
            }
        }
    }

    #[cfg(test)]
    pub fn new_test(
        runtime: RT,
        database: Database<RT>,
        storage: Arc<dyn Storage>,
        file_storage: Arc<dyn Storage>,
    ) -> Self {
        use events::usage::NoOpUsageEventLogger;

        Self {
            runtime,
            database,
            storage,
            file_storage,
            backoff: Backoff::new(INITIAL_BACKOFF, MAX_BACKOFF),
            usage_tracking: UsageCounter::new(Arc::new(NoOpUsageEventLogger)),
        }
    }

    // Subscribe to the export table. If there is a requested export, start
    // an export and mark as in_progress. If there's an export job that didn't
    // finish (it's in_progress), restart that export.
    pub async fn run(&mut self) -> anyhow::Result<()> {
        let mut tx = self.database.begin(Identity::system()).await?;
        let export_requested = Self::export_in_state(&mut tx, "requested").await?;
        let export_in_progress = Self::export_in_state(&mut tx, "in_progress").await?;
        match (export_requested, export_in_progress) {
            (Some(_), Some(_)) => {
                anyhow::bail!("Can only have one export requested or in progress at once.")
            },
            (Some(export), None) => {
                tracing::info!("Export requested.");
                let _status = log_worker_starting("ExportWorker");
                let timer = export_timer();
                let ts = self.database.now_ts_for_reads();
                let in_progress_export = (*export).clone().in_progress(*ts)?;
                let mut inner_tx = self.database.begin(Identity::system()).await?;
                let in_progress_export_doc = SystemMetadataModel::new(&mut inner_tx)
                    .replace(
                        export.id().to_owned(),
                        in_progress_export.clone().try_into()?,
                    )
                    .await?
                    .try_into()?;
                self.database
                    .commit_with_write_source(inner_tx, "export_worker_export_requested")
                    .await?;
                self.export(in_progress_export_doc).await?;
                timer.finish();
            },
            (None, Some(export)) => {
                tracing::info!("In progress export restarting...");
                let _status = log_worker_starting("ExportWorker");
                let timer = export_timer();
                self.export(export).await?;
                timer.finish();
            },
            (None, None) => {
                tracing::info!("No exports requested or in progress.");
            },
        }
        let token = tx.into_token()?;
        let subscription = self.database.subscribe(token).await?;
        subscription.wait_for_invalidation().await;
        Ok(())
    }

    pub async fn export_in_state(
        tx: &mut Transaction<RT>,
        export_state: &str,
    ) -> anyhow::Result<Option<ParsedDocument<Export>>> {
        let index_range = IndexRange {
            index_name: EXPORTS_BY_STATE_AND_TS_INDEX.clone(),
            range: vec![IndexRangeExpression::Eq(
                EXPORTS_STATE_FIELD.clone(),
                maybe_val!(export_state),
            )],
            order: Order::Asc,
        };
        let query = common::query::Query::index_range(index_range);
        let mut query_stream = ResolvedQuery::new(tx, query)?;
        query_stream
            .expect_at_most_one(tx)
            .await?
            .map(|doc| doc.try_into())
            .transpose()
    }

    pub async fn completed_export_at_ts(
        tx: &mut Transaction<RT>,
        snapshot_ts: Timestamp,
    ) -> anyhow::Result<Option<ResolvedDocument>> {
        let index_range = IndexRange {
            index_name: EXPORTS_BY_STATE_AND_TS_INDEX.clone(),
            range: vec![
                IndexRangeExpression::Eq(EXPORTS_STATE_FIELD.clone(), maybe_val!("completed")),
                IndexRangeExpression::Eq(
                    EXPORTS_TS_FIELD.clone(),
                    maybe_val!(i64::from(snapshot_ts)),
                ),
            ],
            order: Order::Desc,
        };
        let query = common::query::Query::index_range(index_range);
        let mut query_stream = ResolvedQuery::new(tx, query)?;
        query_stream.expect_at_most_one(tx).await
    }

    async fn export(&mut self, export: ParsedDocument<Export>) -> anyhow::Result<()> {
        loop {
            match self.export_and_mark_complete(export.clone()).await {
                Ok(()) => {
                    return Ok(());
                },
                Err(mut e) => {
                    report_error(&mut e);
                    let delay = self.runtime.with_rng(|rng| self.backoff.fail(rng));
                    tracing::error!("Export failed, retrying in {delay:?}");
                    self.runtime.wait(delay).await;
                },
            }
        }
    }

    async fn export_inner(
        &mut self,
        format: ExportFormat,
    ) -> anyhow::Result<(Timestamp, ExportObjectKeys, FunctionUsageTracker)> {
        tracing::info!("Beginning snapshot export...");
        let storage = &self.storage;
        let rate_limiter =
            new_rate_limiter(self.runtime.clone(), Quota::per_second(1000.try_into()?));
        let (ts, tables, by_id_indexes, system_tables, virtual_tables) = {
            let mut tx = self.database.begin(Identity::system()).await?;
            let by_id_indexes = IndexModel::new(&mut tx).by_id_indexes().await?;
            let snapshot = self.database.snapshot(tx.begin_timestamp())?;
            let tables: BTreeMap<_, _> = snapshot
                .table_registry
                .iter_active_user_tables()
                .map(|(tablet_id, table_number, table_name)| {
                    (
                        tablet_id,
                        (
                            table_number,
                            table_name.clone(),
                            snapshot.table_summaries.tablet_summary(&tablet_id),
                        ),
                    )
                })
                .collect();
            let system_tables = snapshot
                .table_registry
                .iter_active_system_tables()
                .map(|(id, _, name)| (name.clone(), id))
                .collect();
            let virtual_tables = snapshot.table_registry.virtual_table_mapping().clone();
            (
                tx.begin_timestamp(),
                tables,
                by_id_indexes,
                system_tables,
                virtual_tables,
            )
        };
        let tablet_ids: BTreeSet<TabletId> =
            tables.iter().map(|(tablet_id, ..)| *tablet_id).collect();

        match format {
            ExportFormat::Zip { include_storage } => {
                // Start upload.
                let mut upload = storage.start_upload().await?;
                let (sender, receiver) = mpsc::channel::<Bytes>(1);
                let uploader = upload.try_write_parallel_and_hash(receiver.map(Ok));
                let writer = ChannelWriter::new(sender, 5 * (1 << 20));
                let usage = FunctionUsageTracker::new();

                let zipper = self.construct_zip_snapshot(
                    writer,
                    tables.clone(),
                    ts,
                    by_id_indexes,
                    system_tables,
                    virtual_tables,
                    include_storage,
                    usage.clone(),
                );
                let (_, ()) = try_join!(uploader, zipper)?;
                let object_keys = ExportObjectKeys::Zip(upload.complete().await?);
                Ok((*ts, object_keys, usage))
            },
            ExportFormat::CleanJsonl | ExportFormat::InternalJson => {
                // Start uploads concurrently
                let start_upload_futs = tables.clone().into_keys().map(|t_id| async move {
                    anyhow::Ok((t_id, TableUpload::new(storage.to_owned(), format).await?))
                });
                let mut table_uploads: BTreeMap<_, _> = futures::stream::iter(start_upload_futs)
                    .buffer_unordered(20)
                    .try_collect()
                    .await?;

                for tablet_id in tablet_ids.iter() {
                    let by_id = by_id_indexes
                        .get(tablet_id)
                        .ok_or_else(|| anyhow::anyhow!("no by_id index for {} found", tablet_id))?;
                    let table_iterator = self.database.table_iterator(ts, 1000, None);
                    let stream = table_iterator.stream_documents_in_table(
                        *tablet_id,
                        *by_id,
                        None,
                        &rate_limiter,
                    );
                    pin_mut!(stream);

                    let (tablet_id, mut table_upload) = table_uploads
                        .remove_entry(tablet_id)
                        .ok_or_else(|| anyhow::anyhow!("No table with id {} found", tablet_id))?;

                    // Write documents from stream to table uploads
                    while let Some((doc, _ts)) = stream.try_next().await? {
                        table_upload = table_upload.write(doc).await?;
                    }
                    table_uploads.insert(tablet_id, table_upload);
                }

                let tables_ = &tables;
                // Complete table uploads concurrently
                let complete_upload_futs =
                    table_uploads
                        .into_iter()
                        .map(|(tablet_id, upload)| async move {
                            anyhow::Ok((
                                tables_
                                    .get(&tablet_id)
                                    .expect("export table id missing")
                                    .1
                                    .clone(),
                                upload.complete().await?,
                            ))
                        });
                let table_object_keys = futures::stream::iter(complete_upload_futs)
                    .buffered(20)
                    .try_collect::<BTreeMap<_, _>>()
                    .await?;
                tracing::info!(
                    "Export succeeded! {} snapshots written to storage. Format: {format:?}",
                    tablet_ids.len()
                );
                Ok((
                    *ts,
                    ExportObjectKeys::ByTable(table_object_keys),
                    FunctionUsageTracker::new(),
                ))
            },
        }
    }

    async fn construct_zip_snapshot(
        &self,
        mut writer: ChannelWriter,
        mut tables: BTreeMap<TabletId, (TableNumber, TableName, TableSummary)>,
        snapshot_ts: RepeatableTimestamp,
        by_id_indexes: BTreeMap<TabletId, IndexId>,
        system_tables: BTreeMap<TableName, TabletId>,
        virtual_tables: VirtualTableMapping,
        include_storage: bool,
        usage: FunctionUsageTracker,
    ) -> anyhow::Result<()> {
        let mut zip_snapshot_upload = ZipSnapshotUpload::new(&mut writer).await?;
        let rate_limiter =
            new_rate_limiter(self.runtime.clone(), Quota::per_second(1000.try_into()?));
        let tablet_ids: BTreeSet<_> = tables.keys().cloned().collect();

        {
            // _tables
            let mut table_upload = zip_snapshot_upload
                .start_system_table(TABLES_TABLE.clone())
                .await?;

            // Write documents from stream to table uploads, in table number order.
            // This includes all user tables present in the export.
            let mut user_table_numbers_and_names: Vec<_> = tables
                .iter()
                .map(|(_, (table_number, table_name, _))| (table_number, table_name))
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
            // _storage
            let tablet_id = system_tables
                .get(&FILE_STORAGE_TABLE)
                .context("_file_storage does not exist")?;
            let by_id = by_id_indexes
                .get(tablet_id)
                .context("_file_storage.by_id does not exist")?;
            let virtual_table_number = virtual_tables.number(&FILE_STORAGE_VIRTUAL_TABLE)?;

            // First write metadata to _storage/documents.jsonl
            let mut table_upload = zip_snapshot_upload
                .start_system_table(FILE_STORAGE_VIRTUAL_TABLE.clone())
                .await?;
            let table_iterator = self.database.table_iterator(snapshot_ts, 1000, None);
            let stream =
                table_iterator.stream_documents_in_table(*tablet_id, *by_id, None, &rate_limiter);
            pin_mut!(stream);
            while let Some((doc, _ts)) = stream.try_next().await? {
                let file_storage_entry = ParsedDocument::<FileStorageEntry>::try_from(doc)?;
                let virtual_storage_id = DeveloperDocumentId::new(
                    virtual_table_number,
                    file_storage_entry.id().internal_id(),
                );
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

            let table_iterator = self.database.table_iterator(snapshot_ts, 1000, None);
            let stream =
                table_iterator.stream_documents_in_table(*tablet_id, *by_id, None, &rate_limiter);
            pin_mut!(stream);
            while let Some((doc, _ts)) = stream.try_next().await? {
                let file_storage_entry = ParsedDocument::<FileStorageEntry>::try_from(doc)?;
                let virtual_storage_id = DeveloperDocumentId::new(
                    virtual_table_number,
                    file_storage_entry.id().internal_id(),
                );
                // Add an extension, which isn't necessary for anything and might be incorrect,
                // but allows the file to be viewed at a glance in most cases.
                let extension_guess = file_storage_entry
                    .content_type
                    .as_ref()
                    .and_then(mime2ext)
                    .map(|extension| format!(".{extension}"))
                    .unwrap_or_default();
                let path = format!(
                    "{}/{}{extension_guess}",
                    *FILE_STORAGE_VIRTUAL_TABLE,
                    virtual_storage_id.encode()
                );
                let file_stream = self
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
                usage
                    .track_storage_call("snapshot_export")
                    .track_storage_egress_size(file_stream.content_length as u64);
                zip_snapshot_upload
                    .stream_full_file(path, file_stream.stream)
                    .await?;
            }
        }

        for tablet_id in tablet_ids.iter() {
            let (_, table_name, table_summary) =
                tables.remove(tablet_id).expect("table should have details");
            let by_id = by_id_indexes
                .get(tablet_id)
                .ok_or_else(|| anyhow::anyhow!("no by_id index for {} found", tablet_id))?;

            let mut generated_schema = GeneratedSchema::new(table_summary.inferred_type().into());
            if ExportContext::is_ambiguous(table_summary.inferred_type()) {
                let table_iterator = self.database.table_iterator(snapshot_ts, 1000, None);
                let stream = table_iterator.stream_documents_in_table(
                    *tablet_id,
                    *by_id,
                    None,
                    &rate_limiter,
                );
                pin_mut!(stream);
                while let Some((doc, _ts)) = stream.try_next().await? {
                    generated_schema.insert(doc.value(), doc.developer_id());
                }
            }

            let mut table_upload = zip_snapshot_upload
                .start_table(table_name.clone(), generated_schema)
                .await?;

            let table_iterator = self.database.table_iterator(snapshot_ts, 1000, None);
            let stream =
                table_iterator.stream_documents_in_table(*tablet_id, *by_id, None, &rate_limiter);
            pin_mut!(stream);

            // Write documents from stream to table uploads
            while let Some((doc, _ts)) = stream.try_next().await? {
                usage.track_database_egress_size(table_name.to_string(), doc.size() as u64, false);
                table_upload.write(doc).await?;
            }
            table_upload.complete().await?;
        }

        // Complete upload.
        zip_snapshot_upload.complete().await?;
        writer.compat_write().close().await?;
        Ok(())
    }

    async fn export_and_mark_complete(
        &mut self,
        export: ParsedDocument<Export>,
    ) -> anyhow::Result<()> {
        let (ts, object_keys, usage) = self.export_inner(export.format()).await?;

        let mut tx = self.database.begin(Identity::system()).await?;
        let completed_export =
            (*export)
                .clone()
                .completed(ts, *tx.begin_timestamp(), object_keys)?;
        SystemMetadataModel::new(&mut tx)
            .replace(export.id(), completed_export.try_into()?)
            .await?;
        self.database
            .commit_with_write_source(tx, "export_worker_mark_complete")
            .await?;
        self.usage_tracking.track_call(
            UdfIdentifier::Cli("export".to_string()),
            ExecutionId::new(),
            CallType::Export,
            usage.gather_user_stats(),
        );
        Ok(())
    }
}

struct TableUpload {
    upload: Box<dyn Upload>,
    empty: bool,
    format: ExportFormat,
}

impl TableUpload {
    async fn new(storage: Arc<dyn Storage>, format: ExportFormat) -> anyhow::Result<Self> {
        let mut upload = storage.start_upload().await?;
        if format == ExportFormat::InternalJson {
            upload.write(BEGIN_JSON_ARRAY.clone()).await?;
        }
        Ok(Self {
            upload,
            empty: true,
            format,
        })
    }

    async fn write(mut self, doc: ResolvedDocument) -> anyhow::Result<Self> {
        let json = match self.format {
            ExportFormat::CleanJsonl | ExportFormat::Zip { .. } => {
                doc.export(ValueFormat::ConvexCleanJSON)
            },
            ExportFormat::InternalJson => doc.export(ValueFormat::ConvexEncodedJSON),
        };
        if !self.empty {
            // Between documents.
            match self.format {
                ExportFormat::InternalJson => self.upload.write(BETWEEN_DOCUMENTS.clone()).await?,
                ExportFormat::CleanJsonl | ExportFormat::Zip { .. } => {},
            }
        }
        self.empty = false;

        let buf = serde_json::to_vec(&json)?;
        self.upload.write(buf.into()).await?;

        // After documents.
        match self.format {
            ExportFormat::CleanJsonl | ExportFormat::Zip { .. } => {
                self.upload.write(AFTER_DOCUMENTS_CLEAN.clone()).await?
            },
            ExportFormat::InternalJson => {},
        }

        Ok(self)
    }

    async fn complete(mut self) -> anyhow::Result<ObjectKey> {
        if self.format == ExportFormat::InternalJson {
            self.upload.write(END_JSON_ARRAY.clone()).await?;
        }
        self.upload.complete().await
    }
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

// 'a is lifetime of entire zip file writer.
// 'b is lifetime of entry writer for a single table.
struct ZipSnapshotTableUpload<'a, 'b> {
    entry_writer: EntryStreamWriter<'b, &'a mut ChannelWriter>,
}

impl<'a, 'b> ZipSnapshotTableUpload<'a, 'b> {
    async fn new(
        zip_writer: &'b mut ZipFileWriter<&'a mut ChannelWriter>,
        table_name: TableName,
    ) -> anyhow::Result<Self> {
        let source_path = format!("{table_name}/documents.jsonl");
        let builder = ZipEntryBuilder::new(source_path.clone(), Compression::Deflate)
            .unix_permissions(ZIP_ENTRY_PERMISSIONS);
        let entry_writer = zip_writer.write_entry_stream(builder.build()).await?;
        Ok(Self { entry_writer })
    }

    async fn write(&mut self, doc: ResolvedDocument) -> anyhow::Result<()> {
        let json = doc.export(ValueFormat::ConvexCleanJSON);
        self.write_json_line(json).await
    }

    async fn write_json_line(&mut self, json: JsonValue) -> anyhow::Result<()> {
        let buf = serde_json::to_vec(&json)?;
        self.entry_writer.compat_mut_write().write_all(&buf).await?;
        self.entry_writer
            .compat_mut_write()
            .write_all(&AFTER_DOCUMENTS_CLEAN)
            .await?;
        Ok(())
    }

    async fn complete(self) -> anyhow::Result<()> {
        self.entry_writer.close().await?;
        Ok(())
    }
}

struct ZipSnapshotUpload<'a> {
    writer: ZipFileWriter<&'a mut ChannelWriter>,
}

impl<'a> ZipSnapshotUpload<'a> {
    async fn new(out: &'a mut ChannelWriter) -> anyhow::Result<Self> {
        let writer = ZipFileWriter::new(out);
        let mut zip_snapshot_upload = Self { writer };
        zip_snapshot_upload
            .write_full_file(format!("README.md"), README_MD_CONTENTS)
            .await?;
        Ok(zip_snapshot_upload)
    }

    async fn write_full_file(&mut self, path: String, contents: &str) -> anyhow::Result<()> {
        let builder = ZipEntryBuilder::new(path, Compression::Deflate)
            .unix_permissions(ZIP_ENTRY_PERMISSIONS);
        let mut entry_writer = self.writer.write_entry_stream(builder.build()).await?;
        entry_writer
            .compat_mut_write()
            .write_all(contents.as_bytes())
            .await?;
        entry_writer.close().await?;
        Ok(())
    }

    async fn stream_full_file(
        &mut self,
        path: String,
        mut contents: BoxStream<'_, std::io::Result<Bytes>>,
    ) -> anyhow::Result<()> {
        let builder = ZipEntryBuilder::new(path, Compression::Deflate)
            .unix_permissions(ZIP_ENTRY_PERMISSIONS);
        let mut entry_writer = self.writer.write_entry_stream(builder.build()).await?;
        while let Some(chunk) = contents.try_next().await? {
            entry_writer.compat_mut_write().write_all(&chunk).await?;
        }
        entry_writer.close().await?;
        Ok(())
    }

    async fn start_table<T: ShapeConfig>(
        &mut self,
        table_name: TableName,
        generated_schema: GeneratedSchema<T>,
    ) -> anyhow::Result<ZipSnapshotTableUpload<'a, '_>> {
        self.write_generated_schema(&table_name, generated_schema)
            .await?;

        ZipSnapshotTableUpload::new(&mut self.writer, table_name).await
    }

    /// System tables have known shape, so we don't need to serialize it.
    async fn start_system_table(
        &mut self,
        table_name: TableName,
    ) -> anyhow::Result<ZipSnapshotTableUpload<'a, '_>> {
        anyhow::ensure!(table_name.is_system());
        ZipSnapshotTableUpload::new(&mut self.writer, table_name).await
    }

    async fn write_generated_schema<T: ShapeConfig>(
        &mut self,
        table_name: &TableName,
        generated_schema: GeneratedSchema<T>,
    ) -> anyhow::Result<()> {
        let generated_schema_path = format!("{table_name}/generated_schema.jsonl");
        let builder = ZipEntryBuilder::new(generated_schema_path.clone(), Compression::Deflate)
            .unix_permissions(ZIP_ENTRY_PERMISSIONS);
        let mut entry_writer = self.writer.write_entry_stream(builder.build()).await?;
        let generated_schema_str = generated_schema.inferred_shape.to_string();
        entry_writer
            .compat_mut_write()
            .write_all(serde_json::to_string(&generated_schema_str)?.as_bytes())
            .await?;
        entry_writer.compat_mut_write().write_all(b"\n").await?;
        for (override_id, override_export_context) in generated_schema.overrides.into_iter() {
            let override_json =
                json!({override_id.encode(): JsonValue::from(override_export_context)});
            entry_writer
                .compat_mut_write()
                .write_all(serde_json::to_string(&override_json)?.as_bytes())
                .await?;
            entry_writer.compat_mut_write().write_all(b"\n").await?;
        }
        entry_writer.close().await?;
        Ok(())
    }

    async fn complete(self) -> anyhow::Result<()> {
        self.writer.close().await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::{
        collections::BTreeMap,
        str,
        sync::Arc,
        time::Duration,
    };

    use anyhow::Context;
    use bytes::Bytes;
    use common::{
        document::{
            ParsedDocument,
            ResolvedDocument,
        },
        types::{
            ConvexOrigin,
            TableName,
        },
        value::ConvexObject,
    };
    use database::{
        test_helpers::DbFixtures,
        TableModel,
        UserFacingModel,
    };
    use file_storage::{
        FileStorage,
        TransactionalFileStorage,
    };
    use headers::ContentType;
    use keybroker::Identity;
    use maplit::btreemap;
    use model::{
        exports::types::{
            Export,
            ExportFormat,
            ExportObjectKeys,
        },
        file_storage::types::FileStorageEntry,
        test_helpers::DbFixturesWithModel,
    };
    use must_let::must_let;
    use runtime::testing::TestRuntime;
    use serde_json::json;
    use storage::{
        LocalDirStorage,
        Storage,
        StorageExt,
    };
    use usage_tracking::FunctionUsageTracker;
    use value::{
        assert_obj,
        export::ValueFormat,
        GenericDocumentId,
        ResolvedDocumentId,
    };

    use super::{
        ExportWorker,
        TableUpload,
    };
    use crate::export_worker::README_MD_CONTENTS;

    #[convex_macro::test_runtime]
    async fn test_export(rt: TestRuntime) -> anyhow::Result<()> {
        let DbFixtures { db, .. } = DbFixtures::new(&rt).await?;
        let storage: Arc<dyn Storage> = Arc::new(LocalDirStorage::new(rt.clone())?);
        let file_storage: Arc<dyn Storage> = Arc::new(LocalDirStorage::new(rt.clone())?);
        let mut export_worker =
            ExportWorker::new_test(rt, db.clone(), storage.clone(), file_storage);

        // Write to a bunch of tables
        for i in 0..2 {
            let table: TableName = str::parse(format!("table_{i}").as_str())?;
            let mut tx = db.begin(Identity::system()).await?;
            UserFacingModel::new(&mut tx)
                .insert(table, ConvexObject::empty())
                .await?;
            db.commit(tx).await?;
        }
        let (_, tables, _) = export_worker.export_inner(ExportFormat::CleanJsonl).await?;
        must_let!(let ExportObjectKeys::ByTable(tables) = tables);
        let mut expected_tables = BTreeMap::new();
        for i in 0..2 {
            let table: TableName = str::parse(format!("table_{i}").as_str())?;
            expected_tables.insert(table.clone(), tables.get(&table).unwrap().clone());
        }
        // Check we can get the urls
        for object_key in tables.values().cloned() {
            storage
                .signed_url(object_key, Duration::from_secs(60))
                .await?;
        }

        assert_eq!(tables, expected_tables);
        Ok(())
    }

    #[convex_macro::test_runtime]
    async fn test_export_zip(rt: TestRuntime) -> anyhow::Result<()> {
        let DbFixtures { db, .. } = DbFixtures::new_with_model(&rt).await?;
        let storage: Arc<dyn Storage> = Arc::new(LocalDirStorage::new(rt.clone())?);
        let file_storage: Arc<dyn Storage> = Arc::new(LocalDirStorage::new(rt.clone())?);
        let mut export_worker =
            ExportWorker::new_test(rt, db.clone(), storage.clone(), file_storage);

        let mut expected_export_entries = BTreeMap::new();

        expected_export_entries.insert("README.md".to_string(), README_MD_CONTENTS.to_string());

        expected_export_entries.insert(
            "_tables/documents.jsonl".to_string(),
            format!(
                "{}\n{}\n{}\n",
                json!({"name": "table_0", "id": 10001}),
                json!({"name": "table_1", "id": 10002}),
                json!({"name": "table_2", "id": 10003}),
            ),
        );
        expected_export_entries.insert("_storage/documents.jsonl".to_string(), format!(""));

        // Write to a bunch of tables
        for i in 0..3 {
            let table: TableName = str::parse(format!("table_{i}").as_str())?;
            let mut tx = db.begin(Identity::system()).await?;
            let id = match i {
                0 => {
                    UserFacingModel::new(&mut tx)
                        .insert(table, assert_obj!("foo" => 1))
                        .await?
                },
                1 => {
                    UserFacingModel::new(&mut tx)
                        .insert(table, assert_obj!("foo" => [1, "1"]))
                        .await?
                },
                _ => {
                    UserFacingModel::new(&mut tx)
                        .insert(table, assert_obj!("foo" => "1"))
                        .await?
                },
            };
            let doc = UserFacingModel::new(&mut tx).get(id, None).await?.unwrap();
            let tablet_id = tx.table_mapping().inject_table_id()(doc.table())?.tablet_id;
            let doc = doc.to_resolved(tablet_id);
            let id_v6 = doc.developer_id().encode();
            expected_export_entries.insert(
                format!("table_{i}/documents.jsonl"),
                format!(
                    "{}\n",
                    serde_json::to_string(&doc.export(ValueFormat::ConvexCleanJSON))?
                ),
            );
            expected_export_entries.insert(
                format!("table_{i}/generated_schema.jsonl"),
                match i {
                    0 => format!(
                        "{}\n",
                        json!(format!(
                            "{{\"_creationTime\": normalfloat64, \"_id\": \"{id_v6}\", \"foo\": \
                             int64}}"
                        ))
                    ),
                    1 => format!(
                        "{}\n{}\n",
                        json!(format!(
                            "{{\"_creationTime\": normalfloat64, \"_id\": \"{id_v6}\", \"foo\": \
                             array<int64 | field_name>}}"
                        )),
                        json!({id_v6: {"foo": ["int64", "infer"]}})
                    ),
                    _ => format!(
                        "{}\n",
                        json!(format!(
                            "{{\"_creationTime\": normalfloat64, \"_id\": \"{id_v6}\", \"foo\": \
                             field_name}}"
                        ))
                    ),
                },
            );
            db.commit(tx).await?;
        }
        let (_, object_keys, usage) = export_worker
            .export_inner(ExportFormat::Zip {
                include_storage: true,
            })
            .await?;
        must_let!(let ExportObjectKeys::Zip(object_key) = object_keys);

        // Check we can get the stored zip.
        let storage_stream = storage
            .get(&object_key)
            .await?
            .context("object missing from storage")?;
        let stored_bytes = storage_stream.collect_as_bytes().await?;
        let mut zip_reader = async_zip::read::mem::ZipFileReader::new(&stored_bytes).await?;
        let mut zip_entries = BTreeMap::new();
        let filenames: Vec<_> = zip_reader
            .entries()
            .into_iter()
            .map(|entry| entry.filename().to_string())
            .collect();
        for (i, filename) in filenames.into_iter().enumerate() {
            let entry_reader = zip_reader.entry_reader(i).await?;
            let entry_contents = String::from_utf8(entry_reader.read_to_end_crc().await?)?;
            zip_entries.insert(filename, entry_contents);
        }
        assert_eq!(zip_entries, expected_export_entries);

        let usage = usage.gather_user_stats();
        assert_eq!(
            *usage.database_egress_size,
            btreemap! {
               "table_0".to_string() => 1024,
               "table_1".to_string() => 1024,
               "table_2".to_string() => 1024,
            }
        );

        Ok(())
    }

    #[convex_macro::test_runtime]
    async fn test_export_storage(rt: TestRuntime) -> anyhow::Result<()> {
        let DbFixtures { db, .. } = DbFixtures::new_with_model(&rt).await?;
        let storage: Arc<dyn Storage> = Arc::new(LocalDirStorage::new(rt.clone())?);
        let file_storage: Arc<dyn Storage> = Arc::new(LocalDirStorage::new(rt.clone())?);
        let mut export_worker = ExportWorker::new_test(
            rt.clone(),
            db.clone(),
            storage.clone(),
            file_storage.clone(),
        );
        let file_storage_wrapper = FileStorage {
            database: db.clone(),
            transactional_file_storage: TransactionalFileStorage::new(
                rt,
                file_storage,
                ConvexOrigin::from("origin".to_string()),
            ),
        };
        let mut expected_export_entries = BTreeMap::new();

        expected_export_entries.insert("README.md".to_string(), README_MD_CONTENTS.to_string());
        expected_export_entries.insert("_tables/documents.jsonl".to_string(), format!(""));

        // Write a few storage files.
        let usage_tracker = FunctionUsageTracker::new();
        let file1_id = file_storage_wrapper
            .store_file(
                None,
                Some(ContentType::jpeg()),
                futures::stream::iter(vec![Ok(Bytes::from_static(b"abc"))]),
                None,
                &usage_tracker,
            )
            .await?;
        let mut tx = db.begin(Identity::system()).await?;
        let storage_table_id = tx.table_mapping().id(&"_file_storage".parse()?)?;
        let file1: ParsedDocument<FileStorageEntry> = tx
            .get(GenericDocumentId::new(
                storage_table_id,
                file1_id.internal_id(),
            ))
            .await?
            .unwrap()
            .try_into()?;

        expected_export_entries.insert(format!("_storage/{file1_id}.jpeg"), format!("abc"));
        expected_export_entries.insert(
            "_storage/documents.jsonl".to_string(),
            format!(
                "{}\n",
                json!({"_id": file1_id.encode(), "_creationTime": f64::from(file1.creation_time().unwrap()), "sha256": "ungWv48Bz+pBQUDeXa4iI7ADYaOWF3qctBD/YfIAFa0=", "size": 3, "contentType": "image/jpeg", "internalId": file1.storage_id.to_string()}),
            ),
        );

        let (_, object_keys, usage) = export_worker
            .export_inner(ExportFormat::Zip {
                include_storage: true,
            })
            .await?;
        must_let!(let ExportObjectKeys::Zip(object_key) = object_keys);

        // Check we can get the stored zip.
        let storage_stream = storage
            .get(&object_key)
            .await?
            .context("object missing from storage")?;
        let stored_bytes = storage_stream.collect_as_bytes().await?;
        let mut zip_reader = async_zip::read::mem::ZipFileReader::new(&stored_bytes).await?;
        let mut zip_entries = BTreeMap::new();
        let filenames: Vec<_> = zip_reader
            .entries()
            .into_iter()
            .map(|entry| entry.filename().to_string())
            .collect();
        for (i, filename) in filenames.into_iter().enumerate() {
            let entry_reader = zip_reader.entry_reader(i).await?;
            let entry_contents = String::from_utf8(entry_reader.read_to_end_crc().await?)?;
            zip_entries.insert(filename, entry_contents);
        }
        assert_eq!(zip_entries, expected_export_entries);

        let usage = usage.gather_user_stats();
        assert!(usage.database_egress_size.is_empty());
        assert_eq!(usage.storage_egress_size, 3);

        Ok(())
    }

    // Regression test: previously we were trying to export documents from deleted
    // tables and table_mapping was failing.
    #[convex_macro::test_runtime]
    async fn test_export_with_table_delete(rt: TestRuntime) -> anyhow::Result<()> {
        let DbFixtures { db, .. } = DbFixtures::new(&rt).await?;
        let storage: Arc<dyn Storage> = Arc::new(LocalDirStorage::new(rt.clone())?);
        let file_storage: Arc<dyn Storage> = Arc::new(LocalDirStorage::new(rt.clone())?);
        let mut export_worker =
            ExportWorker::new_test(rt.clone(), db.clone(), storage.clone(), file_storage);

        // Write to two tables and delete one.
        let mut tx = db.begin(Identity::system()).await?;
        UserFacingModel::new(&mut tx)
            .insert("table_0".parse()?, ConvexObject::empty())
            .await?;
        db.commit(tx).await?;
        let mut tx = db.begin(Identity::system()).await?;
        UserFacingModel::new(&mut tx)
            .insert("table_1".parse()?, ConvexObject::empty())
            .await?;
        db.commit(tx).await?;
        let mut tx = db.begin(Identity::system()).await?;
        TableModel::new(&mut tx)
            .delete_table("table_0".parse()?)
            .await?;
        db.commit(tx).await?;

        let (_, tables, _) = export_worker.export_inner(ExportFormat::CleanJsonl).await?;
        must_let!(let ExportObjectKeys::ByTable(tables) = tables);
        let tables: Vec<_> = tables.into_keys().collect();
        assert_eq!(tables, vec!["table_1".parse()?]);
        Ok(())
    }

    #[convex_macro::test_runtime]
    async fn test_table_upload_legacy_export(rt: TestRuntime) -> anyhow::Result<()> {
        let storage: Arc<dyn Storage> = Arc::new(LocalDirStorage::new(rt)?);
        let table_upload = TableUpload::new(storage.clone(), ExportFormat::InternalJson).await?;
        let document = ResolvedDocument::new(
            ResolvedDocumentId::min(),
            (1234.0).try_into()?,
            ConvexObject::for_value("a".parse()?, 33.into())?,
        )?;
        let table_upload = table_upload.write(document).await?;
        let key = table_upload.complete().await?;
        let content = storage
            .get(&key)
            .await?
            .context("Not found")?
            .collect_as_bytes()
            .await?;

        let parsed: serde_json::Value = serde_json::from_slice(&content)?;
        let expected = serde_json::json!([{
            "_creationTime": 1234.0,
            "_id": "0400000000000000000000000000248",
            "a": {
                "$integer": "IQAAAAAAAAA=",
            },
        }]);
        assert_eq!(parsed, expected);
        Ok(())
    }

    #[convex_macro::test_runtime]
    async fn test_table_upload_clean_export(rt: TestRuntime) -> anyhow::Result<()> {
        let storage: Arc<dyn Storage> = Arc::new(LocalDirStorage::new(rt)?);
        let table_upload = TableUpload::new(storage.clone(), ExportFormat::CleanJsonl).await?;
        let document = ResolvedDocument::new(
            ResolvedDocumentId::min(),
            (1234.0).try_into()?,
            ConvexObject::for_value("a".parse()?, 33.into())?,
        )?;
        let table_upload = table_upload.write(document.clone()).await?;
        let table_upload = table_upload.write(document).await?;
        let key = table_upload.complete().await?;

        let content = storage
            .get(&key)
            .await?
            .context("Not found")?
            .collect_as_bytes()
            .await?;
        let lines: Vec<_> = str::from_utf8(&content)?.lines().collect();

        assert_eq!(lines.len(), 2);
        for line in lines {
            let parsed: serde_json::Value = serde_json::from_str(line)?;
            let expected = serde_json::json!({
                "_creationTime": 1234.0,
                "_id": "0400000000000000000000000000248",
                "a": "33",
            });
            assert_eq!(parsed, expected);
        }

        Ok(())
    }

    #[convex_macro::test_runtime]
    async fn test_export_deserialization(rt: TestRuntime) -> anyhow::Result<()> {
        let DbFixtures { db, .. } = DbFixtures::new(&rt).await?;

        // Requested
        let requested_export = Export::requested(ExportFormat::CleanJsonl);
        let object: ConvexObject = requested_export.clone().try_into()?;
        let deserialized_export = object.try_into()?;
        assert_eq!(requested_export, deserialized_export);

        let ts = db.begin(Identity::system()).await?.begin_timestamp();
        // InProgress
        let in_progress_export = requested_export.clone().in_progress(*ts)?;
        let object: ConvexObject = in_progress_export.clone().try_into()?;
        let deserialized_export = object.try_into()?;
        assert_eq!(in_progress_export, deserialized_export);

        // Completed
        let mut tables = BTreeMap::new();
        let table_name: TableName = "test_table".parse()?;
        tables.insert(table_name.clone(), "test_table_object_key".try_into()?);
        let export =
            in_progress_export
                .clone()
                .completed(*ts, *ts, ExportObjectKeys::ByTable(tables))?;
        let object: ConvexObject = export.clone().try_into()?;
        let deserialized_export = object.try_into()?;
        assert_eq!(export, deserialized_export);

        // Failed
        let export = in_progress_export.failed(*ts, *ts)?;
        let object: ConvexObject = export.clone().try_into()?;
        let deserialized_export = object.try_into()?;
        assert_eq!(export, deserialized_export);

        Ok(())
    }
}

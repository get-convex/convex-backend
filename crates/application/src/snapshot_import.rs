use std::{
    collections::{
        BTreeMap,
        BTreeSet,
        HashSet,
    },
    pin::Pin,
    str::FromStr,
    sync::{
        Arc,
        LazyLock,
    },
    time::Duration,
};

use anyhow::Context;
use async_trait::async_trait;
use async_zip::{
    error::ZipError,
    read::{
        seek::ZipFileReader,
        ZipEntryReader,
    },
};
use bytes::Bytes;
use common::{
    async_compat::{
        FuturesAsyncReadCompatExt,
        TokioAsyncRead,
        TokioAsyncReadCompatExt,
    },
    bootstrap_model::{
        schema::SchemaState,
        tables::TABLES_TABLE,
    },
    document::{
        CreationTime,
        ParsedDocument,
        CREATION_TIME_FIELD,
        ID_FIELD,
    },
    errors::report_error,
    execution_context::ExecutionId,
    knobs::{
        TRANSACTION_MAX_NUM_USER_WRITES,
        TRANSACTION_MAX_USER_WRITE_SIZE_BYTES,
    },
    pause::PauseClient,
    runtime::Runtime,
    schemas::DatabaseSchema,
    types::{
        FieldName,
        MemberId,
        ObjectKey,
        TableName,
        UdfIdentifier,
    },
};
use database::{
    Database,
    ImportFacingModel,
    IndexModel,
    SchemaModel,
    TableModel,
    Transaction,
};
use errors::{
    ErrorMetadata,
    ErrorMetadataAnyhowExt,
};
use file_storage::FileStorage;
use futures::{
    io::BufReader,
    pin_mut,
    stream::{
        self,
        BoxStream,
        Peekable,
    },
    AsyncBufReadExt,
    AsyncRead,
    AsyncReadExt,
    Future,
    Stream,
    StreamExt,
    TryStream,
    TryStreamExt,
};
use futures_async_stream::{
    stream,
    try_stream,
};
use headers::{
    ContentLength,
    ContentType,
};
use humansize::{
    FormatSize,
    BINARY,
};
use keybroker::Identity;
use model::{
    deployment_audit_log::{
        types::DeploymentAuditLogEvent,
        DeploymentAuditLogModel,
    },
    file_storage::{
        types::StorageUuid,
        FILE_STORAGE_TABLE,
        FILE_STORAGE_VIRTUAL_TABLE,
    },
    snapshot_imports::{
        types::{
            ImportFormat,
            ImportMode,
            ImportState,
            SnapshotImport,
        },
        SnapshotImportModel,
    },
};
use regex::Regex;
use serde_json::{
    json,
    Value as JsonValue,
};
use shape_inference::{
    export_context::{
        ExportContext,
        GeneratedSchema,
    },
    ProdConfigWithOptionalFields,
    Shape,
    ShapeConfig,
};
use storage::{
    Storage,
    StorageExt,
    StorageObjectReader,
};
use strum::AsRefStr;
use sync_types::{
    backoff::Backoff,
    Timestamp,
};
use thiserror::Error;
use thousands::Separable;
use usage_tracking::{
    CallType,
    FunctionUsageTracker,
    StorageUsageTracker,
    UsageCounter,
};
use value::{
    id_v6::DeveloperDocumentId,
    sha256::Sha256Digest,
    val,
    ConvexObject,
    ConvexValue,
    IdentifierFieldName,
    ResolvedDocumentId,
    Size,
    TableMapping,
    TableNamespace,
    TableNumber,
    TabletId,
    TabletIdAndTableNumber,
};

use crate::{
    export_worker::FileStorageZipMetadata,
    metrics::{
        log_snapshot_import_age,
        log_worker_starting,
        snapshot_import_timer,
    },
    Application,
};

static IMPORT_SIZE_LIMIT: LazyLock<String> =
    LazyLock::new(|| (*TRANSACTION_MAX_USER_WRITE_SIZE_BYTES.format_size(BINARY)).to_string());

const INITIAL_BACKOFF: Duration = Duration::from_secs(1);
const MAX_BACKOFF: Duration = Duration::from_secs(60);
// If an import is taking longer than a day, it's a problem (and our fault).
// But the customer is probably no longer waiting so we should fail the import.
// If an import takes more than a week, the file may be deleted from S3.
pub const MAX_IMPORT_AGE: Duration = Duration::from_secs(24 * 60 * 60);

pub struct SnapshotImportWorker<RT: Runtime> {
    runtime: RT,
    database: Database<RT>,
    snapshot_imports_storage: Arc<dyn Storage>,
    file_storage: FileStorage<RT>,
    usage_tracking: UsageCounter,
    backoff: Backoff,
    pause_client: PauseClient,
}

impl<RT: Runtime> SnapshotImportWorker<RT> {
    #[allow(clippy::new_ret_no_self)]
    pub fn new(
        runtime: RT,
        database: Database<RT>,
        snapshot_imports_storage: Arc<dyn Storage>,
        file_storage: FileStorage<RT>,
        usage_tracking: UsageCounter,
        pause_client: PauseClient,
    ) -> impl Future<Output = ()> + Send {
        let mut worker = Self {
            runtime,
            database,
            snapshot_imports_storage,
            file_storage,
            usage_tracking,
            pause_client,
            backoff: Backoff::new(INITIAL_BACKOFF, MAX_BACKOFF),
        };
        async move {
            loop {
                if let Err(e) = worker.run().await {
                    report_error(&mut e.context("SnapshotImportWorker died"));
                    let delay = worker.runtime.with_rng(|rng| worker.backoff.fail(rng));
                    worker.runtime.wait(delay).await;
                } else {
                    worker.backoff.reset();
                }
            }
        }
    }

    /// Subscribe to the _snapshot_imports table.
    /// If an import has Uploaded, parse it and set to WaitingForConfirmation.
    /// If an import is InProgress, execute it.
    pub async fn run(&mut self) -> anyhow::Result<()> {
        let status = log_worker_starting("SnapshotImport");
        let mut tx = self.database.begin(Identity::system()).await?;
        let mut import_model = SnapshotImportModel::new(&mut tx);
        if let Some(import_uploaded) = import_model.import_in_state(ImportState::Uploaded).await? {
            tracing::info!("Marking snapshot export as WaitingForConfirmation");
            self.parse_and_mark_waiting_for_confirmation(import_uploaded)
                .await?;
        } else if let Some(import_in_progress) = import_model
            .import_in_state(ImportState::InProgress {
                progress_message: String::new(),
                checkpoint_messages: vec![],
            })
            .await?
        {
            tracing::info!("Executing in-progress snapshot import");
            let timer = snapshot_import_timer();
            self.attempt_perform_import_and_mark_done(import_in_progress)
                .await?;
            timer.finish();
        }
        drop(status);
        let token = tx.into_token()?;
        let subscription = self.database.subscribe(token).await?;
        subscription.wait_for_invalidation().await;
        Ok(())
    }

    async fn parse_and_mark_waiting_for_confirmation(
        &self,
        snapshot_import: ParsedDocument<SnapshotImport>,
    ) -> anyhow::Result<()> {
        let import_id = snapshot_import.id();
        match snapshot_import.state {
            ImportState::Uploaded => {
                // Can make progress. Continue.
            },
            ImportState::Completed { .. }
            | ImportState::Failed(..)
            | ImportState::InProgress { .. }
            | ImportState::WaitingForConfirmation { .. } => {
                anyhow::bail!("unexpected state {snapshot_import:?}");
            },
        }
        match self.info_message_for_import(snapshot_import).await {
            Ok((info_message, require_manual_confirmation)) => {
                self.database
                    .execute_with_overloaded_retries(
                        Identity::system(),
                        FunctionUsageTracker::new(),
                        PauseClient::new(),
                        "snapshot_import_waiting_for_confirmation",
                        |tx| {
                            async {
                                let mut import_model = SnapshotImportModel::new(tx);
                                import_model
                                    .mark_waiting_for_confirmation(
                                        import_id,
                                        info_message.clone(),
                                        require_manual_confirmation,
                                    )
                                    .await?;
                                Ok(())
                            }
                            .into()
                        },
                    )
                    .await?;
            },
            Err(e) => {
                let e = wrap_import_err(e);
                if e.is_bad_request() {
                    self.database
                        .execute_with_overloaded_retries(
                            Identity::system(),
                            FunctionUsageTracker::new(),
                            PauseClient::new(),
                            "snapshot_import_fail",
                            |tx| {
                                async {
                                    let mut import_model = SnapshotImportModel::new(tx);
                                    import_model
                                        .fail_import(import_id, e.user_facing_message())
                                        .await?;
                                    Ok(())
                                }
                                .into()
                            },
                        )
                        .await?;
                } else {
                    anyhow::bail!(e);
                }
            },
        }
        Ok(())
    }

    /// Parse the uploaded import file, compare it to existing data, and return
    /// a message to display about the import before it begins.
    async fn info_message_for_import(
        &self,
        snapshot_import: ParsedDocument<SnapshotImport>,
    ) -> anyhow::Result<(String, bool)> {
        let mut message_lines = Vec::new();
        let (content_confirmation_messages, require_manual_confirmation) =
            self.messages_to_confirm_replace(snapshot_import).await?;
        message_lines.extend(content_confirmation_messages);
        // Consider adding confirmation messages about bandwidth usage.
        if !message_lines.is_empty() {
            message_lines.insert(0, format!("Import change summary:"))
        }
        message_lines.push(format!(
            "Once the import has started, it will run in the background.\nInterrupting `npx \
             convex import` will not cancel it."
        ));
        Ok((message_lines.join("\n"), require_manual_confirmation))
    }

    async fn messages_to_confirm_replace(
        &self,
        snapshot_import: ParsedDocument<SnapshotImport>,
    ) -> anyhow::Result<(Vec<String>, bool)> {
        let mode = snapshot_import.mode;
        let (_, mut objects) = self.parse_import(snapshot_import.id()).await?;
        // Find all tables being written to.
        let mut count_by_table: BTreeMap<TableName, u64> = BTreeMap::new();
        let mut current_table = None;
        let mut lineno = 0;
        while let Some(object) = objects.try_next().await? {
            match object {
                ImportUnit::NewTable(table_name) => {
                    lineno = 0;
                    count_by_table.entry(table_name.clone()).or_default();
                    current_table = Some(table_name);
                },
                ImportUnit::Object(exported_value) => {
                    lineno += 1;
                    let Some(current_table) = &current_table else {
                        continue;
                    };
                    if current_table == &*TABLES_TABLE {
                        let exported_object = exported_value
                            .as_object()
                            .with_context(|| ImportError::NotAnObject(lineno))?;
                        let table_name = exported_object
                            .get("name")
                            .and_then(|name| name.as_str())
                            .with_context(|| {
                                ImportError::InvalidConvexValue(
                                    lineno,
                                    anyhow::anyhow!("table requires name"),
                                )
                            })?;
                        let table_name = table_name
                            .parse()
                            .map_err(|e| ImportError::InvalidName(table_name.to_string(), e))?;
                        count_by_table.entry(table_name).or_default();
                    }
                    if let Some(count) = count_by_table.get_mut(current_table) {
                        *count += 1;
                    }
                },
                // Ignore storage file chunks and generated schemas.
                ImportUnit::StorageFileChunk(..) | ImportUnit::GeneratedSchema(..) => {},
            }
        }

        struct TableChange {
            added: u64,
            deleted: usize,
            existing: usize,
            unit: &'static str,
        }
        let mut table_changes = BTreeMap::new();
        let db_snapshot = self.database.latest_snapshot()?;
        for (table_name, count_importing) in count_by_table.iter() {
            if !table_name.is_system() {
                let table_summary = db_snapshot.table_summary(table_name);
                let to_delete = match mode {
                    ImportMode::Replace => {
                        // Overwriting nonempty user table.
                        table_summary.num_values()
                    },
                    ImportMode::Append => 0,
                    ImportMode::RequireEmpty if table_summary.num_values() > 0 => {
                        anyhow::bail!(ImportError::TableExists(table_name.clone()))
                    },
                    ImportMode::RequireEmpty => 0,
                };
                table_changes.insert(
                    table_name.clone(),
                    TableChange {
                        added: *count_importing,
                        deleted: to_delete,
                        existing: table_summary.num_values(),
                        unit: "",
                    },
                );
            }
            if table_name == &*FILE_STORAGE_VIRTUAL_TABLE {
                let table_summary = db_snapshot.table_summary(&FILE_STORAGE_TABLE);
                let to_delete = match mode {
                    ImportMode::Replace => {
                        // Overwriting nonempty file storage.
                        table_summary.num_values()
                    },
                    ImportMode::Append => 0,
                    ImportMode::RequireEmpty if table_summary.num_values() > 0 => {
                        anyhow::bail!(ImportError::TableExists(table_name.clone()))
                    },
                    ImportMode::RequireEmpty => 0,
                };
                table_changes.insert(
                    table_name.clone(),
                    TableChange {
                        added: *count_importing,
                        deleted: to_delete,
                        existing: table_summary.num_values(),
                        unit: " files",
                    },
                );
            }
        }
        let mut require_manual_confirmation = false;

        // Looks like:
        /*
        table    | create  | delete                       |
        ---------------------------------------------------
        _storage | 10      | 11 of 11 files               |
        big      | 100,000 | 100,000 of 100,000 documents |
        messages | 20      | 21 of 21 documents           |
                */
        let mut parts = vec![(
            "table".to_string(),
            "create".to_string(),
            "delete".to_string(),
        )];
        for (
            table_name,
            TableChange {
                added,
                deleted,
                existing,
                unit,
            },
        ) in table_changes
        {
            if deleted > 0 {
                // Deleting files can be destructive, so require confirmation.
                require_manual_confirmation = true;
            }
            parts.push((
                table_name.to_string(),
                added.separate_with_commas(),
                format!(
                    "{} of {}{}",
                    deleted.separate_with_commas(),
                    existing.separate_with_commas(),
                    unit
                ),
            ));
        }
        let part_lengths = (
            parts
                .iter()
                .map(|p| p.0.len())
                .max()
                .expect("should be nonempty"),
            parts
                .iter()
                .map(|p| p.1.len())
                .max()
                .expect("should be nonempty"),
            parts
                .iter()
                .map(|p| p.2.len())
                .max()
                .expect("should be nonempty"),
        );
        let mut message_lines = Vec::new();
        for (i, part) in parts.into_iter().enumerate() {
            message_lines.push(format!(
                "{:3$} | {:4$} | {:5$} |",
                part.0, part.1, part.2, part_lengths.0, part_lengths.1, part_lengths.2
            ));
            if i == 0 {
                message_lines.push(format!(
                    "{:-<1$}",
                    "",
                    part_lengths.0 + 3 + part_lengths.1 + 3 + part_lengths.2 + 2
                ));
            }
        }
        Ok((message_lines, require_manual_confirmation))
    }

    async fn attempt_perform_import_and_mark_done(
        &mut self,
        snapshot_import: ParsedDocument<SnapshotImport>,
    ) -> anyhow::Result<()> {
        let import_id = snapshot_import.id();
        match snapshot_import.state {
            ImportState::InProgress { .. } => {
                // Can make progress. Continue.
            },
            ImportState::Completed { .. }
            | ImportState::Failed(..)
            | ImportState::Uploaded
            | ImportState::WaitingForConfirmation { .. } => {
                anyhow::bail!("unexpected state {snapshot_import:?}");
            },
        }
        match self.attempt_perform_import(snapshot_import).await {
            Ok((ts, num_rows_written)) => {
                self.database
                    .execute_with_overloaded_retries(
                        Identity::system(),
                        FunctionUsageTracker::new(),
                        PauseClient::new(),
                        "snapshop_import_complete",
                        |tx| {
                            async {
                                let mut import_model = SnapshotImportModel::new(tx);
                                import_model
                                    .complete_import(import_id, ts, num_rows_written)
                                    .await?;
                                Ok(())
                            }
                            .into()
                        },
                    )
                    .await?;
            },
            Err(e) => {
                let e = wrap_import_err(e);
                if e.is_bad_request() {
                    self.database
                        .execute_with_overloaded_retries(
                            Identity::system(),
                            FunctionUsageTracker::new(),
                            PauseClient::new(),
                            "snapshot_import_fail",
                            |tx| {
                                async {
                                    let mut import_model = SnapshotImportModel::new(tx);
                                    import_model
                                        .fail_import(import_id, e.user_facing_message())
                                        .await?;
                                    Ok(())
                                }
                                .into()
                            },
                        )
                        .await?;
                } else {
                    anyhow::bail!(e);
                }
            },
        }
        Ok(())
    }

    async fn attempt_perform_import(
        &mut self,
        snapshot_import: ParsedDocument<SnapshotImport>,
    ) -> anyhow::Result<(Timestamp, usize)> {
        if let Some(creation_time) = snapshot_import.creation_time() {
            let now = CreationTime::try_from(*self.database.now_ts_for_reads())?;
            let age = Duration::from_millis((f64::from(now) - f64::from(creation_time)) as u64);
            log_snapshot_import_age(age);
            if age > MAX_IMPORT_AGE / 2 {
                tracing::warn!(
                    "SnapshotImport {} running too long ({:?})",
                    snapshot_import.id(),
                    age
                );
            }
            if age > MAX_IMPORT_AGE {
                anyhow::bail!(ErrorMetadata::bad_request(
                    "ImportFailed",
                    "Import took too long. Try again or contact Convex."
                ));
            }
        }

        let (initial_schemas, objects) = self.parse_import(snapshot_import.id()).await?;

        let usage = FunctionUsageTracker::new();

        let (table_mapping_for_import, total_documents_imported) = import_objects(
            &self.database,
            &self.file_storage,
            Identity::system(),
            snapshot_import.mode,
            objects,
            usage.clone(),
            Some(snapshot_import.id()),
        )
        .await?;

        // Truncate list of table names to avoid storing too much data in
        // audit log object.
        let table_names: Vec<_> = table_mapping_for_import
            .iter()
            .map(|(_, _, _, table_name)| {
                if table_name == &*FILE_STORAGE_TABLE {
                    FILE_STORAGE_VIRTUAL_TABLE.clone()
                } else {
                    table_name.clone()
                }
            })
            .take(20)
            .collect();
        let table_count = table_mapping_for_import.iter().count() as u64;

        self.pause_client.wait("before_finalize_import").await;
        let (ts, _documents_deleted) = finalize_import(
            &self.database,
            &self.usage_tracking,
            Identity::system(),
            snapshot_import.member_id,
            initial_schemas,
            table_mapping_for_import,
            usage,
            DeploymentAuditLogEvent::SnapshotImport {
                table_names,
                table_count,
                import_mode: snapshot_import.mode,
                import_format: snapshot_import.format.clone(),
            },
        )
        .await?;

        Ok((ts, total_documents_imported))
    }

    async fn parse_import(
        &self,
        import_id: ResolvedDocumentId,
    ) -> anyhow::Result<(
        Vec<Option<(ResolvedDocumentId, DatabaseSchema)>>,
        Peekable<BoxStream<'_, anyhow::Result<ImportUnit>>>,
    )> {
        let (object_key, format) = {
            let mut tx = self.database.begin(Identity::system()).await?;
            let mut model = SnapshotImportModel::new(&mut tx);
            let snapshot_import = model.get(import_id).await?.context("import not found")?;
            (
                snapshot_import.object_key.clone(),
                snapshot_import.format.clone(),
            )
        };
        let body_stream = move || {
            let object_key = object_key.clone();
            async move { self.read_snapshot_import(&object_key).await }
        };
        let objects = parse_objects(format.clone(), body_stream).boxed();

        // Remapping could be more extensive here, it's just relatively simple to handle
        // optional types. We do remapping after parsing rather than during parsing
        // because it seems expensive to read the data for and parse all objects inside
        // of a transaction, though I haven't explicitly tested the performance.
        let mut tx = self.database.begin(Identity::system()).await?;

        let initial_schemas = schemas_for_import(&mut tx).await?;

        let objects = match format {
            ImportFormat::Csv(table_name) => {
                remap_empty_string_by_schema(table_name, &mut tx, objects).await?
            },
            _ => objects,
        }
        .peekable();
        drop(tx);
        Ok((initial_schemas, objects))
    }

    pub async fn read_snapshot_import(
        &self,
        key: &ObjectKey,
    ) -> anyhow::Result<StorageObjectReader> {
        self.snapshot_imports_storage.get_reader(key).await
    }
}

#[derive(AsRefStr, Debug, Error)]
pub enum ImportError {
    #[error("Only deployment admins can import new tables")]
    Unauthorized,

    #[error("Table {0} already exists. Please choose a new table name.")]
    TableExists(TableName),

    #[error("{0:?} isn't a valid table name: {1}")]
    InvalidName(String, anyhow::Error),

    #[error("Import wasn't valid UTF8: {0}")]
    NotUtf8(std::io::Error),

    #[error("Import is too large for JSON ({0} bytes > maximum {}). Consider converting data to JSONLines", *IMPORT_SIZE_LIMIT)]
    JsonArrayTooLarge(usize),

    #[error("CSV file doesn't have headers")]
    CsvMissingHeaders,

    #[error("CSV header {0:?} isn't a valid field name: {1}")]
    CsvInvalidHeader(String, anyhow::Error),

    #[error("Failed to parse CSV row {0}: {1}")]
    CsvInvalidRow(usize, csv_async::Error),

    #[error("CSV row {0} doesn't have all of the fields in the header")]
    CsvRowMissingFields(usize),

    #[error("Row {0} wasn't valid JSON: {1}")]
    JsonInvalidRow(usize, serde_json::Error),

    #[error("Row {0} wasn't a valid Convex value: {1}")]
    InvalidConvexValue(usize, anyhow::Error),

    #[error("Row {0} wasn't an object")]
    NotAnObject(usize),

    #[error("Not a JSON array")]
    NotJsonArray,

    #[error("Not valid JSON: {0}")]
    NotJson(serde_json::Error),
}

impl ImportError {
    pub fn error_metadata(&self) -> ErrorMetadata {
        match self {
            ImportError::Unauthorized => {
                ErrorMetadata::forbidden(self.as_ref().to_string(), self.to_string())
            },
            _ => ErrorMetadata::bad_request(self.as_ref().to_string(), self.to_string()),
        }
    }
}

#[derive(Debug)]
enum ImportUnit {
    Object(JsonValue),
    NewTable(TableName),
    GeneratedSchema(TableName, GeneratedSchema<ProdConfigWithOptionalFields>),
    StorageFileChunk(DeveloperDocumentId, Bytes),
}

static GENERATED_SCHEMA_PATTERN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?:.*/)?([^/]+)/generated_schema\.jsonl$").unwrap());
static DOCUMENTS_PATTERN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?:.*/)?([^/]+)/documents\.jsonl$").unwrap());
// _storage/(ID) with optional ignored prefix and extension like
// snapshot/_storage/(ID).png
static STORAGE_FILE_PATTERN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?:.*/)?_storage/([^/.]+)(?:\.[^/]+)?$").unwrap());

fn map_zip_error(e: ZipError) -> anyhow::Error {
    match e {
        // UpstreamReadError is probably a transient error from S3.
        ZipError::UpstreamReadError(e) => anyhow::Error::from(e),
        // Everything else indicates a Zip file that cannot be parsed.
        e => ErrorMetadata::bad_request("InvalidZip", format!("invalid zip file: {e}")).into(),
    }
}

/// Parse and stream units from the imported file, starting with a NewTable
/// for each table and then Objects for each object to import into the table.
/// stream_body returns the file as streamed bytes. stream_body() can be called
/// multiple times to read the file multiple times, for cases where the file
/// must be read out of order, e.g. because the _tables table must be imported
/// first.
#[try_stream(ok = ImportUnit, error = anyhow::Error)]
async fn parse_objects<'a, Fut>(format: ImportFormat, stream_body: impl Fn() -> Fut + 'a)
where
    Fut: Future<Output = anyhow::Result<StorageObjectReader>> + 'a,
{
    match format {
        ImportFormat::Csv(table_name) => {
            let reader = stream_body().await?;
            yield ImportUnit::NewTable(table_name);
            let mut reader = csv_async::AsyncReader::from_reader(reader);
            if !reader.has_headers() {
                anyhow::bail!(ImportError::CsvMissingHeaders);
            }
            let field_names = {
                let headers = reader.headers().await?;
                headers
                    .iter()
                    .map(|s| {
                        let trimmed = s.trim_matches(' ');
                        let field_name = FieldName::from_str(trimmed)
                            .map_err(|e| ImportError::CsvInvalidHeader(trimmed.to_string(), e))?;
                        Ok(field_name)
                    })
                    .collect::<anyhow::Result<Vec<_>>>()?
            };
            let mut enumerate_rows = reader.records().enumerate();
            while let Some((i, row_r)) = enumerate_rows.next().await {
                let lineno = i + 1;
                let parsed_row = row_r
                    .map_err(|e| ImportError::CsvInvalidRow(lineno, e))?
                    .iter()
                    .map(parse_csv_cell)
                    .collect::<Vec<JsonValue>>();
                let mut obj = BTreeMap::new();
                if field_names.len() != parsed_row.len() {
                    anyhow::bail!(ImportError::CsvRowMissingFields(lineno));
                }
                for (field_name, value) in field_names.iter().zip(parsed_row.into_iter()) {
                    obj.insert(field_name.to_string(), value);
                }
                yield ImportUnit::Object(serde_json::to_value(obj)?);
            }
        },
        ImportFormat::JsonLines(table_name) => {
            let reader = stream_body().await?;
            yield ImportUnit::NewTable(table_name);
            let mut reader = BufReader::new(reader);
            let mut line = String::new();
            let mut lineno = 1;
            while reader
                .read_line(&mut line)
                .await
                .map_err(ImportError::NotUtf8)?
                > 0
            {
                let v: serde_json::Value = serde_json::from_str(&line)
                    .map_err(|e| ImportError::JsonInvalidRow(lineno, e))?;
                yield ImportUnit::Object(v);
                line.clear();
                lineno += 1;
            }
        },
        ImportFormat::JsonArray(table_name) => {
            let reader = stream_body().await?;
            yield ImportUnit::NewTable(table_name);
            let mut buf = Vec::new();
            let mut truncated_reader =
                reader.take((*TRANSACTION_MAX_USER_WRITE_SIZE_BYTES as u64) + 1);
            truncated_reader.read_to_end(&mut buf).await?;
            if buf.len() > *TRANSACTION_MAX_USER_WRITE_SIZE_BYTES {
                anyhow::bail!(ImportError::JsonArrayTooLarge(buf.len()));
            }
            let v: serde_json::Value =
                serde_json::from_slice(&buf).map_err(ImportError::NotJson)?;
            let array = v.as_array().ok_or(ImportError::NotJsonArray)?;
            for value in array.iter() {
                yield ImportUnit::Object(value.clone());
            }
        },
        ImportFormat::Zip => {
            let mut reader = stream_body().await?.compat();
            let mut zip_reader = ZipFileReader::new(&mut reader)
                .await
                .map_err(map_zip_error)?;
            let filenames: Vec<_> = zip_reader
                .entries()
                .into_iter()
                .map(|entry| entry.filename().to_string())
                .collect();
            {
                // First pass, all the things we can store in memory:
                // a. _tables/documents.jsonl
                // b. _storage/documents.jsonl
                // c. user_table/generated_schema.jsonl
                // _tables needs to be imported before user tables so we can
                // pick table numbers correctly for schema validation.
                // Each generated schema must be parsed before the corresponding
                // table/documents.jsonl file, so we correctly infer types from
                // export-formatted JsonValues.
                let mut table_metadata = vec![];
                let mut storage_metadata = vec![];
                let mut generated_schemas = vec![];
                for (i, filename) in filenames.iter().enumerate() {
                    let documents_table_name = parse_documents_jsonl_table_name(filename)?;
                    if let Some(table_name) = &documents_table_name
                        && table_name == &*TABLES_TABLE
                    {
                        let entry_reader =
                            zip_reader.entry_reader(i).await.map_err(map_zip_error)?;
                        table_metadata = parse_documents_jsonl(entry_reader).try_collect().await?;
                    } else if let Some(table_name) = &documents_table_name
                        && table_name == &*FILE_STORAGE_VIRTUAL_TABLE
                    {
                        let entry_reader =
                            zip_reader.entry_reader(i).await.map_err(map_zip_error)?;
                        storage_metadata =
                            parse_documents_jsonl(entry_reader).try_collect().await?;
                    } else if let Some(generated_schema_captures) =
                        GENERATED_SCHEMA_PATTERN.captures(filename)
                    {
                        let entry_reader =
                            zip_reader.entry_reader(i).await.map_err(map_zip_error)?;
                        let table_name_str = generated_schema_captures
                            .get(1)
                            .expect("regex has one capture group")
                            .as_str();
                        let table_name: TableName = table_name_str.parse().map_err(|e| {
                            ErrorMetadata::bad_request(
                                "InvalidTableName",
                                format!("table name '{table_name_str}' invalid: {e}"),
                            )
                        })?;
                        tracing::info!(
                            "importing zip file containing generated_schema {table_name}"
                        );
                        let entry_reader = BufReader::new(entry_reader.compat());
                        let generated_schema =
                            parse_generated_schema(filename, entry_reader).await?;
                        generated_schemas
                            .push(ImportUnit::GeneratedSchema(table_name, generated_schema));
                    }
                }
                for table_unit in table_metadata {
                    yield table_unit;
                }
                if !storage_metadata.is_empty() {
                    for storage_unit in storage_metadata {
                        yield storage_unit;
                    }
                    for (i, filename) in filenames.iter().enumerate() {
                        if let Some(storage_file_captures) = STORAGE_FILE_PATTERN.captures(filename)
                        {
                            let storage_id_str = storage_file_captures
                                .get(1)
                                .expect("regex has one capture group")
                                .as_str();
                            if storage_id_str == "documents" {
                                continue;
                            }
                            let entry_reader =
                                zip_reader.entry_reader(i).await.map_err(map_zip_error)?;
                            let storage_id =
                                DeveloperDocumentId::decode(storage_id_str).map_err(|e| {
                                    ErrorMetadata::bad_request(
                                        "InvalidStorageId",
                                        format!("_storage id '{storage_id_str}' invalid: {e}"),
                                    )
                                })?;
                            tracing::info!(
                                "importing zip file containing storage file {}",
                                storage_id.encode()
                            );
                            let mut entry_reader = entry_reader.compat();
                            let mut buf = [0u8; 1024];
                            while let bytes_read = entry_reader.read(&mut buf).await?
                                && bytes_read > 0
                            {
                                yield ImportUnit::StorageFileChunk(
                                    storage_id,
                                    Bytes::copy_from_slice(&buf[..bytes_read]),
                                );
                            }
                            // In case it's an empty file, make sure we send at
                            // least one chunk.
                            yield ImportUnit::StorageFileChunk(storage_id, Bytes::new());
                        }
                    }
                }
                for generated_schema_unit in generated_schemas {
                    yield generated_schema_unit;
                }
            }

            // Second pass: user tables.
            for (i, filename) in filenames.iter().enumerate() {
                if let Some(table_name) = parse_documents_jsonl_table_name(filename)?
                    && !table_name.is_system()
                {
                    let entry_reader = zip_reader.entry_reader(i).await.map_err(map_zip_error)?;
                    let stream = parse_documents_jsonl(entry_reader);
                    pin_mut!(stream);
                    while let Some(unit) = stream.try_next().await? {
                        yield unit;
                    }
                }
            }
        },
    }
}

fn parse_documents_jsonl_table_name(filename: &str) -> anyhow::Result<Option<TableName>> {
    DOCUMENTS_PATTERN
        .captures(filename)
        .map(|captures| {
            let table_name_str = captures
                .get(1)
                .expect("regex has one capture group")
                .as_str();
            let table_name = table_name_str.parse().map_err(|e| {
                ErrorMetadata::bad_request(
                    "InvalidTableName",
                    format!("table name '{table_name_str}' invalid: {e}"),
                )
            })?;
            Ok(table_name)
        })
        .transpose()
}

#[try_stream(ok = ImportUnit, error = anyhow::Error)]
async fn parse_documents_jsonl<R: TokioAsyncRead + Unpin>(entry_reader: ZipEntryReader<'_, R>) {
    let table_name = parse_documents_jsonl_table_name(entry_reader.entry().filename())?
        .context("expected documents.jsonl file")?;
    tracing::info!("importing zip file containing table {table_name}");
    yield ImportUnit::NewTable(table_name);
    let mut reader = BufReader::new(entry_reader.compat());
    let mut line = String::new();
    let mut lineno = 1;
    while reader.read_line(&mut line).await? > 0 {
        let v: serde_json::Value =
            serde_json::from_str(&line).map_err(|e| ImportError::JsonInvalidRow(lineno, e))?;
        yield ImportUnit::Object(v);
        line.clear();
        lineno += 1;
    }
}

async fn parse_generated_schema<'a, T: ShapeConfig, R: AsyncRead + Unpin>(
    filename: &str,
    mut entry_reader: BufReader<R>,
) -> anyhow::Result<GeneratedSchema<T>> {
    let mut line = String::new();
    let mut lineno = 1;
    entry_reader
        .read_line(&mut line)
        .await
        .map_err(ImportError::NotUtf8)?;
    let inferred_type_json: serde_json::Value =
        serde_json::from_str(&line).map_err(|e| ImportError::JsonInvalidRow(lineno, e))?;
    let inferred_type = Shape::from_str(inferred_type_json.as_str().with_context(|| {
        ImportError::InvalidConvexValue(
            lineno,
            anyhow::anyhow!("first line of generated_schema must be a string"),
        )
    })?)
    .with_context(|| {
        ErrorMetadata::bad_request("InvalidGeneratedSchema", format!("cannot parse {filename}"))
    })?;
    line.clear();
    lineno += 1;
    let mut overrides = BTreeMap::new();
    while entry_reader
        .read_line(&mut line)
        .await
        .map_err(ImportError::NotUtf8)?
        > 0
    {
        let mut v: serde_json::Value =
            serde_json::from_str(&line).map_err(|e| ImportError::JsonInvalidRow(lineno, e))?;
        let o = v.as_object_mut().with_context(|| {
            ImportError::InvalidConvexValue(lineno, anyhow::anyhow!("overrides should be object"))
        })?;
        if o.len() != 1 {
            anyhow::bail!(ImportError::InvalidConvexValue(
                lineno,
                anyhow::anyhow!("override object should have one item")
            ));
        }
        let (key, value) = o.into_iter().next().context("must have one item")?;
        let export_context = ExportContext::try_from(value.clone())
            .map_err(|e| ImportError::InvalidConvexValue(lineno, e))?;
        overrides.insert(
            DeveloperDocumentId::decode(key)
                .map_err(|e| ImportError::InvalidConvexValue(lineno, e.into()))?,
            export_context,
        );

        line.clear();
        lineno += 1;
    }
    let generated_schema = GeneratedSchema {
        inferred_shape: inferred_type,
        overrides,
    };
    Ok(generated_schema)
}

// For now, we only parse out floats and strings in CSV files.
fn parse_csv_cell(s: &str) -> JsonValue {
    if let Ok(r) = s.parse::<f64>() {
        return json!(r);
    }
    json!(s)
}

pub async fn upload_import_file<RT: Runtime>(
    application: &Application<RT>,
    identity: Identity,
    format: ImportFormat,
    mode: ImportMode,
    body_stream: BoxStream<'_, anyhow::Result<Bytes>>,
) -> anyhow::Result<DeveloperDocumentId> {
    if !identity.is_admin() {
        anyhow::bail!(ImportError::Unauthorized);
    }
    let object_key = application.upload_snapshot_import(body_stream).await?;
    store_uploaded_import(application, identity, format, mode, object_key).await
}

pub async fn store_uploaded_import<RT: Runtime>(
    application: &Application<RT>,
    identity: Identity,
    format: ImportFormat,
    mode: ImportMode,
    object_key: ObjectKey,
) -> anyhow::Result<DeveloperDocumentId> {
    let (_, id, _) = application
        .database
        .execute_with_overloaded_retries(
            identity,
            FunctionUsageTracker::new(),
            PauseClient::new(),
            "snapshot_import_store_uploaded",
            |tx| {
                async {
                    let mut model = SnapshotImportModel::new(tx);
                    model
                        .start_import(format.clone(), mode, object_key.clone())
                        .await
                }
                .into()
            },
        )
        .await?;
    Ok(id.into())
}

pub async fn perform_import<RT: Runtime>(
    application: &Application<RT>,
    identity: Identity,
    import_id: DeveloperDocumentId,
) -> anyhow::Result<()> {
    if !identity.is_admin() {
        anyhow::bail!(ImportError::Unauthorized);
    }
    application
        .database
        .execute_with_overloaded_retries(
            identity,
            FunctionUsageTracker::new(),
            PauseClient::new(),
            "snapshot_import_perform",
            |tx| {
                async {
                    let import_id = import_id.to_resolved(
                        tx.table_mapping()
                            .namespace(TableNamespace::Global)
                            .number_to_tablet(),
                    )?;
                    let mut import_model = SnapshotImportModel::new(tx);
                    import_model.confirm_import(import_id).await?;
                    Ok(())
                }
                .into()
            },
        )
        .await?;
    Ok(())
}

fn wrap_import_err(e: anyhow::Error) -> anyhow::Error {
    let e = e.wrap_error_message(|msg| format!("Hit an error while importing:\n{msg}"));
    if let Some(import_err) = e.downcast_ref::<ImportError>() {
        let error_metadata = import_err.error_metadata();
        e.context(error_metadata)
    } else {
        e
    }
}

async fn wait_for_import_worker<RT: Runtime>(
    application: &Application<RT>,
    identity: Identity,
    import_id: DeveloperDocumentId,
) -> anyhow::Result<ParsedDocument<SnapshotImport>> {
    let snapshot_import = loop {
        let mut tx = application.begin(identity.clone()).await?;
        let import_id = import_id.to_resolved(
            tx.table_mapping()
                .namespace(TableNamespace::Global)
                .number_to_tablet(),
        )?;
        let mut import_model = SnapshotImportModel::new(&mut tx);
        let snapshot_import =
            import_model
                .get(import_id)
                .await?
                .context(ErrorMetadata::not_found(
                    "ImportNotFound",
                    format!("import {import_id} not found"),
                ))?;
        match &snapshot_import.state {
            ImportState::Uploaded | ImportState::InProgress { .. } => {
                let token = tx.into_token()?;
                application.subscribe(token).await?;
            },
            ImportState::WaitingForConfirmation { .. }
            | ImportState::Completed { .. }
            | ImportState::Failed(..) => {
                break snapshot_import;
            },
        }
    };
    Ok(snapshot_import)
}

pub async fn do_import<RT: Runtime>(
    application: &Application<RT>,
    identity: Identity,
    format: ImportFormat,
    mode: ImportMode,
    body_stream: BoxStream<'_, anyhow::Result<Bytes>>,
) -> anyhow::Result<usize> {
    let import_id =
        upload_import_file(application, identity.clone(), format, mode, body_stream).await?;

    let snapshot_import = wait_for_import_worker(application, identity.clone(), import_id).await?;
    match &snapshot_import.state {
        ImportState::Uploaded | ImportState::InProgress { .. } | ImportState::Completed { .. } => {
            anyhow::bail!("should be WaitingForConfirmation, is {snapshot_import:?}")
        },
        ImportState::WaitingForConfirmation { .. } => {},
        ImportState::Failed(e) => {
            anyhow::bail!(ErrorMetadata::bad_request("ImportFailed", e.to_string()))
        },
    }

    perform_import(application, identity.clone(), import_id).await?;

    let snapshot_import = wait_for_import_worker(application, identity.clone(), import_id).await?;
    match &snapshot_import.state {
        ImportState::Uploaded
        | ImportState::WaitingForConfirmation { .. }
        | ImportState::InProgress { .. } => {
            anyhow::bail!("should be done, is {snapshot_import:?}")
        },
        ImportState::Completed {
            ts: _,
            num_rows_written,
        } => Ok(*num_rows_written),
        ImportState::Failed(e) => {
            anyhow::bail!(ErrorMetadata::bad_request("ImportFailed", e.to_string()))
        },
    }
}

/// Clears tables atomically.
/// Returns number of documents deleted.
/// This is implemented as an import of empty tables in Replace mode.
pub async fn clear_tables<RT: Runtime>(
    application: &Application<RT>,
    identity: &Identity,
    table_names: Vec<TableName>,
) -> anyhow::Result<u64> {
    let usage = FunctionUsageTracker::new();

    let initial_schemas = {
        let mut tx = application.begin(identity.clone()).await?;
        schemas_for_import(&mut tx).await?
    };

    let objects = stream::iter(
        table_names
            .into_iter()
            .map(ImportUnit::NewTable)
            .map(anyhow::Ok),
    )
    .boxed()
    .peekable();

    let (table_mapping_for_import, _) = import_objects(
        &application.database,
        &application.file_storage,
        identity.clone(),
        ImportMode::Replace,
        objects,
        usage.clone(),
        None,
    )
    .await?;

    let (_ts, documents_deleted) = finalize_import(
        &application.database,
        &application.usage_tracking,
        identity.clone(),
        None,
        initial_schemas,
        table_mapping_for_import,
        usage,
        DeploymentAuditLogEvent::ClearTables,
    )
    .await?;
    Ok(documents_deleted)
}

async fn best_effort_update_progress_message<RT: Runtime>(
    database: &Database<RT>,
    identity: &Identity,
    import_id: ResolvedDocumentId,
    progress_message: String,
) {
    // Ignore errors because it's not worth blocking or retrying if we can't
    // send a nice progress message on the first try.
    let _result: anyhow::Result<()> = try {
        let mut tx = database.begin(identity.clone()).await?;
        let mut import_model = SnapshotImportModel::new(&mut tx);
        import_model
            .update_progress_message(import_id, progress_message)
            .await?;
        database
            .commit_with_write_source(tx, "snapshot_update_progress_msg")
            .await?;
    };
}

async fn add_checkpoint_message<RT: Runtime>(
    database: &Database<RT>,
    identity: &Identity,
    import_id: ResolvedDocumentId,
    checkpoint_message: String,
) -> anyhow::Result<()> {
    database
        .execute_with_overloaded_retries(
            identity.clone(),
            FunctionUsageTracker::new(),
            PauseClient::new(),
            "snapshot_import_add_checkpoint_message",
            |tx| {
                async {
                    SnapshotImportModel::new(tx)
                        .add_checkpoint_message(import_id, checkpoint_message.clone())
                        .await
                }
                .into()
            },
        )
        .await?;
    Ok(())
}

async fn import_objects<RT: Runtime>(
    database: &Database<RT>,
    file_storage: &FileStorage<RT>,
    identity: Identity,
    mode: ImportMode,
    objects: Peekable<BoxStream<'_, anyhow::Result<ImportUnit>>>,
    usage: FunctionUsageTracker,
    import_id: Option<ResolvedDocumentId>,
) -> anyhow::Result<(TableMapping, usize)> {
    pin_mut!(objects);
    let mut generated_schemas = BTreeMap::new();

    let mut table_mapping_for_import = TableMapping::new();
    let mut total_num_documents = 0;

    while let Some(num_documents) = import_single_table(
        database,
        file_storage,
        &identity,
        mode,
        objects.as_mut(),
        &mut generated_schemas,
        &mut table_mapping_for_import,
        usage.clone(),
        import_id,
    )
    .await?
    {
        total_num_documents += num_documents;
    }
    Ok((table_mapping_for_import, total_num_documents))
}

/// The case where a schema can become invalid:
/// 1. import is changing the table number of table "foo".
/// 2. import does not touch table "bar".
/// 3. "bar" has a foreign reference to "foo", validated by schema.
/// 4. when the import commits, "bar" is nonempty.
/// To prevent this case we throw an error if a schema'd table outside the
/// import is nonempty and points into the import, and the import changes the
/// table number.
#[derive(Clone, Debug, Ord, PartialOrd, Eq, PartialEq)]
struct ImportSchemaTableConstraint {
    // "foo" in example above.
    foreign_ref_table_in_import: (TableName, TableNumber),
    // "bar" in example above.
    table_in_schema_not_in_import: TableName,
}

impl ImportSchemaTableConstraint {
    async fn validate<RT: Runtime>(&self, tx: &mut Transaction<RT>) -> anyhow::Result<()> {
        let existing_table_mapping = tx.table_mapping();
        let Some(existing_table) = existing_table_mapping
            .namespace(TableNamespace::by_component_TODO())
            .id_and_number_if_exists(&self.foreign_ref_table_in_import.0)
        else {
            // If a table doesn't have a table number,
            // schema validation for foreign references into the table is
            // meaningless.
            return Ok(());
        };
        if existing_table.table_number == self.foreign_ref_table_in_import.1 {
            // The import isn't changing the table number, so the schema
            // is still valid.
            return Ok(());
        }
        if TableModel::new(tx)
            .count(
                TableNamespace::by_component_TODO(),
                &self.table_in_schema_not_in_import,
            )
            .await?
            == 0
        {
            // Schema is validating an empty table which is meaningless.
            // We can change the table numbers without violating schema.
            return Ok(());
        }
        anyhow::bail!(ErrorMetadata::bad_request(
            "ImportForeignKey",
            format!(
                "Import changes table '{}' which is referenced by '{}' in the schema",
                self.foreign_ref_table_in_import.0, self.table_in_schema_not_in_import,
            ),
        ));
    }
}

#[derive(Clone, Debug)]
struct ImportSchemaConstraints {
    initial_schemas: Vec<Option<(ResolvedDocumentId, DatabaseSchema)>>,
    table_constraints: BTreeSet<ImportSchemaTableConstraint>,
}

impl ImportSchemaConstraints {
    fn new(
        table_mapping_for_import: &TableMapping,
        initial_schemas: Vec<Option<(ResolvedDocumentId, DatabaseSchema)>>,
    ) -> Self {
        let mut table_constraints = BTreeSet::new();
        for schema in initial_schemas.iter() {
            let Some((_, schema)) = schema else {
                continue;
            };
            for (table, table_schema) in &schema.tables {
                if table_mapping_for_import
                    .namespace(TableNamespace::by_component_TODO())
                    .name_exists(table)
                {
                    // Schema's table is in the import => it's valid.
                    continue;
                }
                let Some(document_schema) = &table_schema.document_type else {
                    continue;
                };
                for foreign_key_table in document_schema.foreign_keys() {
                    if let Some(foreign_key_table_number) = table_mapping_for_import
                        .namespace(TableNamespace::by_component_TODO())
                        .id_and_number_if_exists(foreign_key_table)
                    {
                        table_constraints.insert(ImportSchemaTableConstraint {
                            table_in_schema_not_in_import: table.clone(),
                            foreign_ref_table_in_import: (
                                foreign_key_table.clone(),
                                foreign_key_table_number.table_number,
                            ),
                        });
                    }
                }
            }
        }
        Self {
            initial_schemas,
            table_constraints,
        }
    }

    async fn validate<RT: Runtime>(&self, tx: &mut Transaction<RT>) -> anyhow::Result<()> {
        let final_schemas = schemas_for_import(tx).await?;
        anyhow::ensure!(
            self.initial_schemas == final_schemas,
            ErrorMetadata::bad_request(
                "ImportSchemaChanged",
                "Could not complete import because schema changed. Avoid modifying schema.ts \
                 while importing tables",
            )
        );
        for table_constraint in self.table_constraints.iter() {
            table_constraint.validate(tx).await?;
        }
        Ok(())
    }
}

async fn finalize_import<RT: Runtime>(
    database: &Database<RT>,
    usage_tracking: &UsageCounter,
    identity: Identity,
    member_id_override: Option<MemberId>,
    initial_schemas: Vec<Option<(ResolvedDocumentId, DatabaseSchema)>>,
    table_mapping_for_import: TableMapping,
    usage: FunctionUsageTracker,
    audit_log_event: DeploymentAuditLogEvent,
) -> anyhow::Result<(Timestamp, u64)> {
    let tables_in_import = table_mapping_for_import
        .iter()
        .map(|(_, _, _, table_name)| table_name.clone())
        .collect();

    // Ensure that schemas will be valid after the tables are activated.
    let schema_constraints =
        ImportSchemaConstraints::new(&table_mapping_for_import, initial_schemas);

    // If we inserted into an existing table, we're done because the table is
    // now populated and active.
    // If we inserted into an Hidden table, make it Active.
    let (ts, documents_deleted, _) = database
        .execute_with_overloaded_retries(
            identity,
            FunctionUsageTracker::new(),
            PauseClient::new(),
            "snapshot_import_finalize",
            |tx| {
                async {
                    let mut documents_deleted = 0;
                    schema_constraints.validate(tx).await?;
                    let mut table_model = TableModel::new(tx);
                    for (table_id, _, table_number, table_name) in table_mapping_for_import.iter() {
                        documents_deleted += table_model
                            .activate_table(table_id, table_name, table_number, &tables_in_import)
                            .await?;
                    }
                    DeploymentAuditLogModel::new(tx)
                        .insert_with_member_override(
                            vec![audit_log_event.clone()],
                            member_id_override,
                        )
                        .await?;

                    Ok(documents_deleted)
                }
                .into()
            },
        )
        .await?;

    usage_tracking.track_call(
        UdfIdentifier::Cli("import".to_string()),
        ExecutionId::new(),
        CallType::Import,
        usage.gather_user_stats(),
    );

    Ok((ts, documents_deleted))
}

/// Documents in an imported table should match the schema.
/// ImportFacingModel::insert checks that new documents match the schema,
/// but SchemaWorker does not check new schemas against existing documents in
/// Hidden tables. This is useful if the import fails and a Hidden table is left
/// dangling, because it should not block new schemas.
/// So, to avoid a race condition where the schema changes *during* an import
/// and SchemaWorker says the schema is valid without checking the partially
/// imported documents, we make sure all relevant schemas stay the same.
async fn schemas_for_import<RT: Runtime>(
    tx: &mut Transaction<RT>,
) -> anyhow::Result<Vec<Option<(ResolvedDocumentId, DatabaseSchema)>>> {
    let mut schema_model = SchemaModel::new(tx, TableNamespace::by_component_TODO());
    let mut schemas = vec![];
    for schema_state in [
        SchemaState::Active,
        SchemaState::Validated,
        SchemaState::Pending,
    ] {
        schemas.push(schema_model.get_by_state(schema_state).await?);
    }
    Ok(schemas)
}

async fn import_tables_table<RT: Runtime>(
    database: &Database<RT>,
    identity: &Identity,
    mode: ImportMode,
    mut objects: Pin<&mut Peekable<BoxStream<'_, anyhow::Result<ImportUnit>>>>,
) -> anyhow::Result<TableMapping> {
    let mut table_mapping_for_import = TableMapping::new();
    let mut import_tables: Vec<(TableName, TableNumber)> = vec![];
    let mut lineno = 0;
    while let Some(ImportUnit::Object(exported_value)) = objects
        .as_mut()
        .try_next_if(|line| matches!(line, ImportUnit::Object(_)))
        .await?
    {
        lineno += 1;
        let exported_object = exported_value
            .as_object()
            .with_context(|| ImportError::NotAnObject(lineno))?;
        let table_name = exported_object
            .get("name")
            .and_then(|name| name.as_str())
            .with_context(|| {
                ImportError::InvalidConvexValue(lineno, anyhow::anyhow!("table requires name"))
            })?;
        let table_name = table_name
            .parse()
            .map_err(|e| ImportError::InvalidName(table_name.to_string(), e))?;
        let table_number = exported_object
            .get("id")
            .and_then(|id| id.as_f64())
            .and_then(|id| TableNumber::try_from(id as u32).ok())
            .with_context(|| {
                ImportError::InvalidConvexValue(
                    lineno,
                    anyhow::anyhow!(
                        "table requires id (received {:?})",
                        exported_object.get("id")
                    ),
                )
            })?;
        import_tables.push((table_name, table_number));
    }
    let tables_in_import = import_tables
        .iter()
        .map(|(table_name, _)| table_name.clone())
        .collect();
    for (table_name, table_number) in import_tables.iter() {
        let table_id = prepare_table_for_import(
            database,
            identity,
            mode,
            table_name,
            Some(*table_number),
            &tables_in_import,
        )
        .await?;
        table_mapping_for_import.insert(
            table_id.tablet_id,
            TableNamespace::by_component_TODO(),
            table_id.table_number,
            table_name.clone(),
        );
    }
    Ok(table_mapping_for_import)
}

async fn import_storage_table<RT: Runtime>(
    database: &Database<RT>,
    file_storage: &FileStorage<RT>,
    identity: &Identity,
    table_id: TabletIdAndTableNumber,
    mut objects: Pin<&mut Peekable<BoxStream<'_, anyhow::Result<ImportUnit>>>>,
    usage: &dyn StorageUsageTracker,
    import_id: Option<ResolvedDocumentId>,
) -> anyhow::Result<()> {
    let virtual_table_number = database
        .latest_snapshot()?
        .table_registry
        .virtual_table_mapping()
        .number(&FILE_STORAGE_VIRTUAL_TABLE)?;
    let mut lineno = 0;
    let mut storage_metadata = BTreeMap::new();
    while let Some(ImportUnit::Object(exported_value)) = objects
        .as_mut()
        .try_next_if(|line| matches!(line, ImportUnit::Object(_)))
        .await?
    {
        lineno += 1;
        let metadata: FileStorageZipMetadata = serde_json::from_value(exported_value)
            .map_err(|e| ImportError::InvalidConvexValue(lineno, e.into()))?;
        let id = DeveloperDocumentId::decode(&metadata.id)
            .map_err(|e| ImportError::InvalidConvexValue(lineno, e.into()))?;
        anyhow::ensure!(
            *id.table() == virtual_table_number,
            ErrorMetadata::bad_request(
                "InvalidId",
                format!(
                    "_storage entry has invalid ID {id} ({:?} != {:?})",
                    id.table(),
                    virtual_table_number
                )
            )
        );
        let content_length = metadata.size.map(|size| ContentLength(size as u64));
        let content_type = metadata
            .content_type
            .map(|content_type| anyhow::Ok(ContentType::from(mime::Mime::from_str(&content_type)?)))
            .transpose()
            .map_err(|e| ImportError::InvalidConvexValue(lineno, e))?;
        let sha256 = metadata
            .sha256
            .map(|sha256| Sha256Digest::from_base64(&sha256))
            .transpose()
            .map_err(|e| ImportError::InvalidConvexValue(lineno, e))?;
        let storage_id = metadata
            .internal_id
            .map(|storage_id| StorageUuid::from_str(&storage_id))
            .transpose()
            .map_err(|e| ImportError::InvalidConvexValue(lineno, e))?;
        let creation_time = metadata
            .creation_time
            .map(CreationTime::try_from)
            .transpose()
            .map_err(|e| ImportError::InvalidConvexValue(lineno, e))?;

        storage_metadata.insert(
            id,
            (
                content_length,
                content_type,
                sha256,
                storage_id,
                creation_time,
            ),
        );
    }
    let total_num_files = storage_metadata.len();
    let mut num_files = 0;
    while let Some(Ok(ImportUnit::StorageFileChunk(id, _))) = objects.as_mut().peek().await {
        let id = *id;
        // The or_default means a storage file with a valid id will be imported
        // even if it has been explicitly removed from _storage/documents.jsonl,
        // to be robust to manual modifications.
        let (content_length, content_type, expected_sha256, storage_id, creation_time) =
            storage_metadata.remove(&id).unwrap_or_default();
        let file_chunks = objects
            .as_mut()
            .peeking_take_while(move |unit| match unit {
                Ok(ImportUnit::StorageFileChunk(chunk_id, _)) => *chunk_id == id,
                Err(_) => true,
                Ok(_) => false,
            })
            .try_filter_map(|unit| async move {
                match unit {
                    ImportUnit::StorageFileChunk(_, chunk) => Ok(Some(chunk)),
                    _ => Ok(None),
                }
            });
        let mut entry = file_storage
            .transactional_file_storage
            .upload_file(content_length, content_type, file_chunks, expected_sha256)
            .await?;
        if let Some(storage_id) = storage_id {
            entry.storage_id = storage_id;
        }
        let file_size = entry.size as u64;
        database
            .execute_with_overloaded_retries(
                identity.clone(),
                FunctionUsageTracker::new(),
                PauseClient::new(),
                "snapshot_import_storage_table",
                |tx| {
                    async {
                        // Assume table numbers of _storage and _file_storage aren't changing with
                        // this import.
                        let table_mapping = tx.table_mapping().clone();
                        let physical_id = tx
                            .virtual_system_mapping()
                            .virtual_id_v6_to_system_resolved_doc_id(
                                &id,
                                &table_mapping,
                                tx.virtual_table_mapping(),
                            )?;
                        let mut entry_object_map =
                            BTreeMap::from(ConvexObject::try_from(entry.clone())?);
                        entry_object_map.insert(ID_FIELD.clone().into(), val!(physical_id));
                        if let Some(creation_time) = creation_time {
                            entry_object_map.insert(
                                CREATION_TIME_FIELD.clone().into(),
                                val!(f64::from(creation_time)),
                            );
                        }
                        let entry_object = ConvexObject::try_from(entry_object_map)?;
                        ImportFacingModel::new(tx)
                            .insert(table_id, &FILE_STORAGE_TABLE, entry_object, &table_mapping)
                            .await?;
                        Ok(())
                    }
                    .into()
                },
            )
            .await?;
        usage
            .track_storage_call("snapshot_import")
            .track_storage_ingress_size(file_size);
        num_files += 1;
        if let Some(import_id) = import_id {
            best_effort_update_progress_message(
                database,
                identity,
                import_id,
                format!(
                    "Importing \"_storage\" ({}/{} files)",
                    num_files.separate_with_commas(),
                    total_num_files.separate_with_commas()
                ),
            )
            .await;
        }
    }
    if let Some(import_id) = import_id {
        add_checkpoint_message(
            database,
            identity,
            import_id,
            format!(
                "Imported \"_storage\" ({} files)",
                num_files.separate_with_commas()
            ),
        )
        .await?;
    }
    Ok(())
}

/// StreamExt::take_while but it works better on peekable streams, not dropping
/// any elements. See `test_peeking_take_while` below.
/// Equivalent to https://docs.rs/peeking_take_while/latest/peeking_take_while/#
/// but for streams instead of iterators.
trait PeekableExt: Stream {
    #[stream(item=Self::Item)]
    async fn peeking_take_while<F>(self: Pin<&mut Self>, predicate: F)
    where
        F: Fn(&Self::Item) -> bool + 'static;
}

impl<S: Stream> PeekableExt for Peekable<S> {
    #[stream(item=S::Item)]
    async fn peeking_take_while<F>(mut self: Pin<&mut Self>, predicate: F)
    where
        F: Fn(&Self::Item) -> bool + 'static,
    {
        while let Some(item) = self.as_mut().next_if(&predicate).await {
            yield item;
        }
    }
}

#[async_trait]
trait TryPeekableExt: TryStream {
    async fn try_next_if<F>(
        self: Pin<&mut Self>,
        predicate: F,
    ) -> Result<Option<Self::Ok>, Self::Error>
    where
        F: Fn(&Self::Ok) -> bool + 'static + Send + Sync;
}

#[async_trait]
impl<Ok: Send, Error: Send, S: Stream<Item = Result<Ok, Error>> + Send> TryPeekableExt
    for Peekable<S>
{
    async fn try_next_if<F>(
        self: Pin<&mut Self>,
        predicate: F,
    ) -> Result<Option<Self::Ok>, Self::Error>
    where
        F: Fn(&Self::Ok) -> bool + 'static + Send + Sync,
    {
        self.next_if(&|result: &Result<Ok, Error>| match result {
            Ok(item) => predicate(item),
            Err(_) => true,
        })
        .await
        .transpose()
    }
}

async fn import_single_table<RT: Runtime>(
    database: &Database<RT>,
    file_storage: &FileStorage<RT>,
    identity: &Identity,
    mode: ImportMode,
    mut objects: Pin<&mut Peekable<BoxStream<'_, anyhow::Result<ImportUnit>>>>,
    generated_schemas: &mut BTreeMap<TableName, GeneratedSchema<ProdConfigWithOptionalFields>>,
    table_mapping_for_import: &mut TableMapping,
    usage: FunctionUsageTracker,
    import_id: Option<ResolvedDocumentId>,
) -> anyhow::Result<Option<usize>> {
    while let Some(ImportUnit::GeneratedSchema(table_name, generated_schema)) = objects
        .as_mut()
        .try_next_if(|line| matches!(line, ImportUnit::GeneratedSchema(_, _)))
        .await?
    {
        generated_schemas.insert(table_name, generated_schema);
    }
    let mut table_name = match objects.try_next().await? {
        Some(ImportUnit::NewTable(table_name)) => table_name,
        Some(_) => anyhow::bail!("parse_objects should start with NewTable"),
        // No more tables to import.
        None => return Ok(None),
    };
    let mut table_number_from_docs = table_number_for_import(objects.as_mut()).await;
    if let Some(import_id) = import_id {
        best_effort_update_progress_message(
            database,
            identity,
            import_id,
            format!("Importing \"{table_name}\""),
        )
        .await;
    }

    if table_name == *FILE_STORAGE_VIRTUAL_TABLE {
        table_name = FILE_STORAGE_TABLE.clone();
        // Infer table number from existing table.
        table_number_from_docs = None;
    }

    if table_name == *TABLES_TABLE {
        table_mapping_for_import
            .update(import_tables_table(database, identity, mode, objects.as_mut()).await?);
        return Ok(Some(0));
    }

    let mut generated_schema = generated_schemas.get_mut(&table_name);
    let tables_in_import = table_mapping_for_import
        .iter()
        .map(|(_, _, _, table_name)| table_name.clone())
        .collect();
    let table_id = match table_mapping_for_import
        .namespace(TableNamespace::by_component_TODO())
        .id_and_number_if_exists(&table_name)
    {
        Some(table_id) => table_id,
        None => {
            let table_id = prepare_table_for_import(
                database,
                identity,
                mode,
                &table_name,
                table_number_from_docs,
                &tables_in_import,
            )
            .await?;
            table_mapping_for_import.insert(
                table_id.tablet_id,
                TableNamespace::by_component_TODO(),
                table_id.table_number,
                table_name.clone(),
            );
            table_id
        },
    };

    if table_name == *FILE_STORAGE_TABLE {
        import_storage_table(
            database,
            file_storage,
            identity,
            table_id,
            objects.as_mut(),
            &usage,
            import_id,
        )
        .await?;
        return Ok(Some(0));
    }

    let mut num_objects = 0;

    let mut tx = database.begin(identity.clone()).await?;
    let mut table_mapping_for_schema = tx.table_mapping().clone();
    table_mapping_for_schema.update(table_mapping_for_import.clone());
    let mut objects_to_insert = vec![];
    let mut objects_to_insert_size = 0;
    // Peek so we don't pop ImportUnit::NewTable items.
    while let Some(ImportUnit::Object(exported_value)) = objects
        .as_mut()
        .try_next_if(|line| matches!(line, ImportUnit::Object(_)))
        .await?
    {
        let row_number = num_objects + 1;
        let convex_value = GeneratedSchema::<ProdConfigWithOptionalFields>::apply(
            &mut generated_schema,
            exported_value,
        )
        .map_err(|e| ImportError::InvalidConvexValue(row_number, e))?;
        let ConvexValue::Object(convex_object) = convex_value else {
            anyhow::bail!(ImportError::NotAnObject(row_number));
        };
        objects_to_insert_size += convex_object.size();
        objects_to_insert.push(convex_object);

        if objects_to_insert_size > *TRANSACTION_MAX_USER_WRITE_SIZE_BYTES / 2
            || objects_to_insert.len() > *TRANSACTION_MAX_NUM_USER_WRITES / 2
        {
            insert_import_objects(
                database,
                identity,
                objects_to_insert,
                &table_name,
                table_id,
                &table_mapping_for_schema,
                usage.clone(),
            )
            .await?;
            objects_to_insert = Vec::new();
            objects_to_insert_size = 0;
            if let Some(import_id) = import_id {
                best_effort_update_progress_message(
                    database,
                    identity,
                    import_id,
                    format!(
                        "Importing \"{table_name}\" ({} documents)",
                        num_objects.separate_with_commas()
                    ),
                )
                .await;
            }
        }
        num_objects += 1;
    }

    insert_import_objects(
        database,
        identity,
        objects_to_insert,
        &table_name,
        table_id,
        &table_mapping_for_schema,
        usage,
    )
    .await?;

    if let Some(import_id) = import_id {
        add_checkpoint_message(
            database,
            identity,
            import_id,
            format!(
                "Imported \"{table_name}\" ({} documents)",
                num_objects.separate_with_commas()
            ),
        )
        .await?;
    }

    Ok(Some(num_objects))
}

async fn insert_import_objects<RT: Runtime>(
    database: &Database<RT>,
    identity: &Identity,
    objects_to_insert: Vec<ConvexObject>,
    table_name: &TableName,
    table_id: TabletIdAndTableNumber,
    table_mapping_for_schema: &TableMapping,
    usage: FunctionUsageTracker,
) -> anyhow::Result<()> {
    let object_ids: Vec<_> = objects_to_insert
        .iter()
        .filter_map(|object| object.get(&**ID_FIELD))
        .collect();
    let object_ids_dedup: BTreeSet<_> = object_ids.iter().collect();
    if object_ids_dedup.len() < object_ids.len() {
        anyhow::bail!(ErrorMetadata::bad_request(
            "DuplicateId",
            format!("Objects in table \"{table_name}\" have duplicate _id fields")
        ));
    }
    database
        .execute_with_overloaded_retries(
            identity.clone(),
            usage,
            PauseClient::new(),
            "snapshot_import_insert_objects",
            |tx| {
                async {
                    for object_to_insert in objects_to_insert.clone() {
                        ImportFacingModel::new(tx)
                            .insert(
                                table_id,
                                table_name,
                                object_to_insert.clone(),
                                table_mapping_for_schema,
                            )
                            .await?;
                    }
                    Ok(())
                }
                .into()
            },
        )
        .await?;
    Ok(())
}

async fn prepare_table_for_import<RT: Runtime>(
    database: &Database<RT>,
    identity: &Identity,
    mode: ImportMode,
    table_name: &TableName,
    table_number: Option<TableNumber>,
    tables_in_import: &BTreeSet<TableName>,
) -> anyhow::Result<TabletIdAndTableNumber> {
    anyhow::ensure!(
        table_name == &*FILE_STORAGE_TABLE || !table_name.is_system(),
        ErrorMetadata::bad_request(
            "InvalidTableName",
            format!("Invalid table name {table_name} starts with metadata prefix '_'")
        )
    );
    let mut tx = database.begin(identity.clone()).await?;
    let existing_table_id = tx
        .table_mapping()
        .namespace(TableNamespace::by_component_TODO())
        .id_and_number_if_exists(table_name);
    let insert_into_existing_table_id = match mode {
        ImportMode::Append => existing_table_id,
        ImportMode::RequireEmpty => {
            if !TableModel::new(&mut tx)
                .table_is_empty(TableNamespace::by_component_TODO(), table_name)
                .await?
            {
                anyhow::bail!(ImportError::TableExists(table_name.clone()));
            }
            None
        },
        ImportMode::Replace => None,
    };
    drop(tx);
    let table_id = if let Some(insert_into_existing_table_id) = insert_into_existing_table_id {
        insert_into_existing_table_id
    } else {
        let table_number = table_number.or(existing_table_id.map(|id| id.table_number));
        let (_, table_id, _) = database
            .execute_with_overloaded_retries(
                identity.clone(),
                FunctionUsageTracker::new(),
                PauseClient::new(),
                "snapshot_import_prepare_table",
                |tx| {
                    async {
                        // Create a new table in state Hidden, that will later be changed to Active.
                        let table_id = TableModel::new(tx)
                            .insert_table_for_import(
                                TableNamespace::by_component_TODO(),
                                table_name,
                                table_number,
                                tables_in_import,
                            )
                            .await?;
                        IndexModel::new(tx)
                            .copy_indexes_to_table(
                                TableNamespace::by_component_TODO(),
                                table_name,
                                table_id.tablet_id,
                            )
                            .await?;
                        Ok(table_id)
                    }
                    .into()
                },
            )
            .await?;
        // The new table is empty, so all of its indexes should be backfilled quickly.
        backfill_and_enable_indexes_on_table(database, identity, table_id.tablet_id).await?;

        table_id
    };
    Ok(table_id)
}

/// Waits for all indexes on a table to be backfilled, which may take a while
/// for large tables. After the indexes are backfilled, enable them.
async fn backfill_and_enable_indexes_on_table<RT: Runtime>(
    database: &Database<RT>,
    identity: &Identity,
    tablet_id: TabletId,
) -> anyhow::Result<()> {
    loop {
        let mut tx = database.begin(identity.clone()).await?;
        let still_backfilling = IndexModel::new(&mut tx)
            .all_indexes_on_table(tablet_id)
            .await?
            .into_iter()
            .any(|index| index.config.is_backfilling());
        if !still_backfilling {
            break;
        }
        let token = tx.into_token()?;
        let subscription = database.subscribe(token).await?;
        subscription.wait_for_invalidation().await;
    }
    // Enable the indexes now that they are backfilled.
    database
        .execute_with_overloaded_retries(
            identity.clone(),
            FunctionUsageTracker::new(),
            PauseClient::new(),
            "snapshot_import_enable_indexes",
            |tx| {
                async {
                    let mut index_model = IndexModel::new(tx);
                    let mut backfilled_indexes = vec![];
                    for index in index_model.all_indexes_on_table(tablet_id).await? {
                        if !index.config.is_enabled() {
                            backfilled_indexes.push(index.into_value());
                        }
                    }
                    index_model
                        .enable_backfilled_indexes(backfilled_indexes)
                        .await?;
                    Ok(())
                }
                .into()
            },
        )
        .await?;
    Ok(())
}

async fn table_number_for_import(
    objects: Pin<&mut Peekable<BoxStream<'_, anyhow::Result<ImportUnit>>>>,
) -> Option<TableNumber> {
    let first_object = objects.peek().await?.as_ref().ok();
    match first_object? {
        ImportUnit::Object(object) => {
            let object = object.as_object()?;
            let first_id = object.get(&**ID_FIELD)?;
            let JsonValue::String(id) = first_id else {
                return None;
            };
            let id_v6 = DeveloperDocumentId::decode(id).ok()?;
            Some(*id_v6.table())
        },
        ImportUnit::NewTable(_) => None,
        ImportUnit::GeneratedSchema(..) => None,
        ImportUnit::StorageFileChunk(..) => None,
    }
}

async fn remap_empty_string_by_schema<'a, RT: Runtime>(
    table_name: TableName,
    tx: &mut Transaction<RT>,
    objects: BoxStream<'a, anyhow::Result<ImportUnit>>,
) -> anyhow::Result<BoxStream<'a, anyhow::Result<ImportUnit>>> {
    if let Some((_, schema)) = SchemaModel::new(tx, TableNamespace::by_component_TODO())
        .get_by_state(SchemaState::Active)
        .await?
    {
        let document_schema = match schema
            .tables
            .get(&table_name)
            .and_then(|table_schema| table_schema.document_type.clone())
        {
            None => return Ok(objects),
            Some(document_schema) => document_schema,
        };
        let optional_fields = document_schema.optional_top_level_fields();
        if optional_fields.is_empty() {
            return Ok(objects);
        }

        Ok(objects
            .map_ok(move |object| match object {
                unit @ ImportUnit::NewTable(_)
                | unit @ ImportUnit::GeneratedSchema(..)
                | unit @ ImportUnit::StorageFileChunk(..) => unit,
                ImportUnit::Object(mut object) => ImportUnit::Object({
                    remove_empty_string_optional_entries(&optional_fields, &mut object);
                    object
                }),
            })
            .boxed())
    } else {
        Ok(objects)
    }
}

fn remove_empty_string_optional_entries(
    optional_fields: &HashSet<IdentifierFieldName>,
    object: &mut JsonValue,
) {
    let Some(object) = object.as_object_mut() else {
        return;
    };
    object.retain(|field_name, value| {
        // Remove optional fields that have an empty string as their value.
        let Ok(identifier_field_name) = field_name.parse::<IdentifierFieldName>() else {
            return true;
        };
        if !optional_fields.contains(&identifier_field_name) {
            return true;
        }
        let JsonValue::String(ref s) = value else {
            return true;
        };
        !s.is_empty()
    });
}

#[cfg(test)]
mod tests {
    use std::{
        collections::BTreeMap,
        str::FromStr,
        sync::Arc,
    };

    use anyhow::Context;
    use bytes::Bytes;
    use common::{
        bootstrap_model::index::{
            IndexConfig,
            IndexMetadata,
        },
        db_schema,
        document::ResolvedDocument,
        object_validator,
        pause::PauseController,
        query::Order,
        runtime::Runtime,
        schemas::{
            validator::{
                FieldValidator,
                Validator,
            },
            DatabaseSchema,
            DocumentSchema,
        },
        tokio::select,
        types::{
            IndexName,
            MemberId,
        },
        value::ConvexValue,
    };
    use database::{
        IndexModel,
        ResolvedQuery,
        SchemaModel,
        UserFacingModel,
    };
    use errors::ErrorMetadataAnyhowExt;
    use futures::{
        pin_mut,
        stream::{
            self,
            BoxStream,
        },
        FutureExt,
        StreamExt,
        TryStreamExt,
    };
    use keybroker::{
        AdminIdentity,
        Identity,
    };
    use maplit::btreemap;
    use model::snapshot_imports::types::ImportState;
    use must_let::must_let;
    use runtime::testing::TestRuntime;
    use serde_json::{
        json,
        Value as JsonValue,
    };
    use storage::{
        LocalDirStorage,
        Storage,
        StorageExt,
        StorageUseCase,
        Upload,
    };
    use usage_tracking::FunctionUsageTracker;
    use value::{
        assert_obj,
        assert_val,
        id_v6::DeveloperDocumentId,
        ConvexObject,
        FieldName,
        TableName,
        TableNamespace,
    };

    use super::{
        do_import,
        import_objects,
        parse_objects,
        ImportFormat,
        ImportMode,
        ImportUnit,
        PeekableExt,
        DOCUMENTS_PATTERN,
        GENERATED_SCHEMA_PATTERN,
        STORAGE_FILE_PATTERN,
    };
    use crate::{
        snapshot_import::{
            upload_import_file,
            wait_for_import_worker,
        },
        test_helpers::{
            ApplicationFixtureArgs,
            ApplicationTestExt,
        },
        Application,
    };

    #[test]
    fn test_filename_regex() -> anyhow::Result<()> {
        let table_name = DOCUMENTS_PATTERN
            .captures("users/documents.jsonl")
            .context("no match")?
            .get(1)
            .context("no capture group")?
            .as_str();
        assert_eq!(table_name, "users");
        // Regression test, checking that the '.' is escaped.
        assert!(DOCUMENTS_PATTERN
            .captures("users/documentsxjsonl")
            .is_none());
        // When an export is unzipped and re-zipped, sometimes there's a prefix.
        let table_name = DOCUMENTS_PATTERN
            .captures("snapshot/users/documents.jsonl")
            .context("no match")?
            .get(1)
            .context("no capture group")?
            .as_str();
        assert_eq!(table_name, "users");
        let table_name = GENERATED_SCHEMA_PATTERN
            .captures("users/generated_schema.jsonl")
            .context("no match")?
            .get(1)
            .context("no capture group")?
            .as_str();
        assert_eq!(table_name, "users");
        let storage_id = STORAGE_FILE_PATTERN
            .captures("_storage/abc123.gif")
            .context("no match")?
            .get(1)
            .context("no capture group")?
            .as_str();
        assert_eq!(storage_id, "abc123");
        let storage_id = STORAGE_FILE_PATTERN
            .captures("snapshot/_storage/abc123.gif")
            .context("no match")?
            .get(1)
            .context("no capture group")?
            .as_str();
        assert_eq!(storage_id, "abc123");
        // No file extension.
        let storage_id = STORAGE_FILE_PATTERN
            .captures("_storage/abc123")
            .context("no match")?
            .get(1)
            .context("no capture group")?
            .as_str();
        assert_eq!(storage_id, "abc123");
        Ok(())
    }

    #[convex_macro::test_runtime]
    async fn test_peeking_take_while(_rt: TestRuntime) {
        let s = stream::iter(vec![1, 2, 3, 4, 5, 6, 7, 8]);
        let mut p = Box::pin(s.peekable());
        // First check that raw take_while causes us to skip an item.
        let prefix = p.as_mut().take_while(|x| {
            let is_prefix = *x <= 2;
            async move { is_prefix }
        });
        pin_mut!(prefix);
        assert_eq!(prefix.collect::<Vec<_>>().await, vec![1, 2]);
        assert_eq!(p.next().await, Some(4));
        // Next check that peeking_take_while doesn't skip an item.
        {
            let prefix = p.as_mut().peeking_take_while(|x| *x <= 6);
            pin_mut!(prefix);
            assert_eq!(prefix.collect::<Vec<_>>().await, vec![5, 6]);
        }
        assert_eq!(p.next().await, Some(7));
    }

    async fn run_parse_objects<RT: Runtime>(
        rt: RT,
        format: ImportFormat,
        v: &str,
    ) -> anyhow::Result<Vec<JsonValue>> {
        let storage_dir = tempfile::TempDir::new()?;
        let storage: Arc<dyn Storage> = Arc::new(LocalDirStorage::for_use_case(
            rt.clone(),
            &storage_dir.path().to_string_lossy(),
            StorageUseCase::SnapshotImports,
        )?);
        let mut upload = storage.start_upload().await?;
        upload.write(Bytes::copy_from_slice(v.as_bytes())).await?;
        let object_key = upload.complete().await?;
        let stream = || storage.get_reader(&object_key);
        parse_objects(format, stream)
            .filter_map(|line| async move {
                match line {
                    Ok(super::ImportUnit::Object(object)) => Some(Ok(object)),
                    Ok(super::ImportUnit::NewTable(_)) => None,
                    Ok(super::ImportUnit::GeneratedSchema(..)) => None,
                    Ok(super::ImportUnit::StorageFileChunk(..)) => None,
                    Err(e) => Some(Err(e)),
                }
            })
            .try_collect()
            .await
    }

    fn stream_from_str(str: &str) -> BoxStream<'static, anyhow::Result<Bytes>> {
        stream::iter(vec![anyhow::Ok(str.to_string().into_bytes().into())]).boxed()
    }

    #[convex_macro::test_runtime]
    async fn test_csv(rt: TestRuntime) -> anyhow::Result<()> {
        let test1 = r#"
a,b,c
1,a string i guess,1.2
5.10,-100,"a string in quotes"
"#;
        let objects =
            run_parse_objects(rt, ImportFormat::Csv("table".parse().unwrap()), test1).await?;
        let expected = vec![
            json!({
                "a": 1.,
                "b": "a string i guess",
                "c": 1.2,
            }),
            json!({
                "a": 5.10,
                "b": -100.,
                "c": "a string in quotes",
            }),
        ];
        assert_eq!(objects, expected);
        Ok(())
    }

    #[convex_macro::test_runtime]
    async fn test_duplicate_id(rt: TestRuntime) -> anyhow::Result<()> {
        let app = Application::new_for_tests(&rt).await?;
        let table_name = "table1";
        let test_csv = r#"
_id,value
"jd7f2yq3tcc5h4ce9qhqdk0ach6hbmyb","hi"
"jd7f2yq3tcc5h4ce9qhqdk0ach6hbmyb","there"
"#;
        let err = run_csv_import(&app, table_name, test_csv)
            .await
            .unwrap_err();
        assert!(err.is_bad_request());
        assert!(
            err.to_string()
                .contains("Objects in table \"table1\" have duplicate _id fields"),
            "{err}"
        );
        Ok(())
    }

    // See https://github.com/BurntSushi/rust-csv/issues/114. TL;DR CSV can't distinguish between empty string and none.
    #[convex_macro::test_runtime]
    async fn test_csv_empty_strings(rt: TestRuntime) -> anyhow::Result<()> {
        let test1 = r#"
a,b,c,d
"",,"""",""""""
"#;
        let objects =
            run_parse_objects(rt, ImportFormat::Csv("table".parse().unwrap()), test1).await?;
        let expected = vec![json!({
            "a": "",
            "b": "",
            "c": "\"",
            "d": "\"\"",
        })];
        assert_eq!(objects, expected);
        Ok(())
    }

    #[convex_macro::test_runtime]
    #[ignore]
    async fn import_huge_csv(rt: TestRuntime) -> anyhow::Result<()> {
        let app = Application::new_for_tests(&rt).await?;
        let table_name = "table1";
        let mut test_csv = vec!["value".to_string()];
        let mut expected = vec![];
        // Too big to write or read in a single transaction.
        for value in 0..10000 {
            test_csv.push(value.to_string());
            expected.push(btreemap!("value" => ConvexValue::from(value as f64)));
        }
        run_csv_import(&app, table_name, &test_csv.join("\n")).await?;

        let objects = load_fields_as_maps(&app, table_name, vec!["value"]).await?;

        assert_eq!(objects, expected);
        Ok(())
    }

    #[convex_macro::test_runtime]
    async fn import_with_empty_strings_and_no_schema_defaults_to_empty_strings(
        rt: TestRuntime,
    ) -> anyhow::Result<()> {
        let app = Application::new_for_tests(&rt).await?;
        let table_name = "table1";
        let test_csv = r#"
a,b,c,d
"",,"""",""""""
"#;
        run_csv_import(&app, table_name, test_csv).await?;

        let objects = load_fields_as_maps(&app, table_name, vec!["a", "b", "c", "d"]).await?;

        let expected = vec![btreemap!(
            "a" => assert_val!(""),
            "b" => assert_val!(""),
            "c" => assert_val!("\""),
            "d" => assert_val!("\"\""),
        )];
        assert_eq!(objects, expected);
        Ok(())
    }

    #[convex_macro::test_runtime]
    async fn import_with_empty_strings_and_string_schema_treats_empty_as_empty(
        rt: TestRuntime,
    ) -> anyhow::Result<()> {
        let app = Application::new_for_tests(&rt).await?;
        let table_name = "table1";
        let test_csv = r#"
a,b,c,d
"",,"""",""""""
"#;

        let fields = vec!["a", "b", "c", "d"];
        let schema = db_schema!(
            table_name => DocumentSchema::Union(
                vec![
                    object_validator!(
                        "a" => FieldValidator::required_field_type(Validator::String),
                        "b" => FieldValidator::required_field_type(Validator::String),
                        "c" => FieldValidator::required_field_type(Validator::String),
                        "d" => FieldValidator::required_field_type(Validator::String),
                    )
                ]
            )
        );

        activate_schema(&app, schema).await?;

        run_csv_import(&app, table_name, test_csv).await?;

        let objects = load_fields_as_maps(&app, table_name, fields).await?;

        assert_eq!(
            objects,
            vec![btreemap!(
                "a" => assert_val!(""),
                "b" => assert_val!(""),
                "c" => assert_val!("\""),
                "d" => assert_val!("\"\""),
            )]
        );

        Ok(())
    }

    #[convex_macro::test_runtime]
    async fn import_with_empty_strings_and_optional_string_schema_treats_empty_as_none(
        rt: TestRuntime,
    ) -> anyhow::Result<()> {
        let app = Application::new_for_tests(&rt).await?;
        let table_name = "table1";
        let test_csv = r#"
a,b,c,d
"",,"""",""""""
"#;

        let schema = db_schema!(
            table_name => DocumentSchema::Union(
                vec![
                    object_validator!(
                        "a" => FieldValidator::optional_field_type(Validator::String),
                        "b" => FieldValidator::optional_field_type(Validator::String),
                        "c" => FieldValidator::optional_field_type(Validator::String),
                        "d" => FieldValidator::optional_field_type(Validator::String),
                    )
                ]
            )
        );

        activate_schema(&app, schema).await?;
        run_csv_import(&app, table_name, test_csv).await?;

        let objects = load_fields_as_maps(&app, table_name, vec!["a", "b", "c", "d"]).await?;

        assert_eq!(
            objects,
            vec![btreemap!(
                "c" => assert_val!("\""),
                "d" => assert_val!("\"\""),
            )]
        );

        Ok(())
    }

    #[convex_macro::test_runtime]
    async fn import_with_empty_strings_and_optional_number_schema_treats_empty_as_none(
        rt: TestRuntime,
    ) -> anyhow::Result<()> {
        let app = Application::new_for_tests(&rt).await?;
        let table_name = "table1";
        let test_csv = r#"
a,b
"",
"#;

        let schema = db_schema!(
            table_name => DocumentSchema::Union(
                vec![
                    object_validator!(
                        "a" => FieldValidator::optional_field_type(Validator::Float64),
                        "b" => FieldValidator::optional_field_type(Validator::Int64),
                    )
                ]
            )
        );

        activate_schema(&app, schema).await?;
        run_csv_import(&app, table_name, test_csv).await?;

        let objects = load_fields_as_maps(&app, table_name, vec!["a", "b"]).await?;

        assert_eq!(objects, vec![BTreeMap::default()]);

        Ok(())
    }

    #[convex_macro::test_runtime]
    async fn import_validates_against_schema(rt: TestRuntime) -> anyhow::Result<()> {
        let app = Application::new_for_tests(&rt).await?;
        let table_name = "table1";
        let test_csv = r#"
a
"string"
"#;

        let schema = db_schema!(
            table_name => DocumentSchema::Union(
                vec![
                    object_validator!(
                        "a" => FieldValidator::optional_field_type(Validator::Float64),
                    )
                ]
            )
        );

        activate_schema(&app, schema).await?;
        let err = run_csv_import(&app, table_name, test_csv)
            .await
            .unwrap_err();
        assert!(err.is_bad_request());

        Ok(())
    }

    #[convex_macro::test_runtime]
    async fn import_replace_confirmation_message(rt: TestRuntime) -> anyhow::Result<()> {
        let app = Application::new_for_tests(&rt).await?;
        let table_name = "table1";
        let test_csv = r#"
a
"string"
"#;
        // Create some data so there's something to replace.
        run_csv_import(&app, table_name, test_csv).await?;

        let import_id = upload_import_file(
            &app,
            new_admin_id(),
            ImportFormat::Csv(table_name.parse()?),
            ImportMode::Replace,
            stream_from_str(test_csv),
        )
        .await?;

        let snapshot_import = wait_for_import_worker(&app, new_admin_id(), import_id).await?;

        let state = snapshot_import.state.clone();
        must_let!(let ImportState::WaitingForConfirmation {
            info_message,
            require_manual_confirmation,
        } = state);

        assert_eq!(
            info_message,
            r#"Import change summary:
table  | create | delete |
--------------------------
table1 | 1      | 1 of 1 |
Once the import has started, it will run in the background.
Interrupting `npx convex import` will not cancel it."#
        );
        assert!(require_manual_confirmation);

        Ok(())
    }

    // Hard to control timing in race test with background job moving state forward.
    #[convex_macro::test_runtime]
    async fn import_races_with_schema_update(rt: TestRuntime) -> anyhow::Result<()> {
        let (mut pause_controller, pause_client) =
            PauseController::new(vec!["before_finalize_import"]);
        let app = Application::new_for_tests_with_args(
            &rt,
            ApplicationFixtureArgs {
                snapshot_import_pause_client: Some(pause_client),
                ..Default::default()
            },
        )
        .await?;
        let table_name = "table1";
        let test_csv = r#"
a
"string"
"#;

        let initial_schema = db_schema!(
            table_name => DocumentSchema::Union(
                vec![
                    object_validator!(
                        "a" => FieldValidator::optional_field_type(Validator::String),
                    )
                ]
            )
        );

        activate_schema(&app, initial_schema).await?;
        let mut import_fut = run_csv_import(&app, table_name, test_csv).boxed();

        select! {
            r = import_fut.as_mut().fuse() => {
                anyhow::bail!("import finished before pausing: {r:?}");
            },
            pause_guard = pause_controller.wait_for_blocked("before_finalize_import").fuse() => {
                let mut pause_guard = pause_guard.unwrap();
                let mismatch_schema = db_schema!(
                    table_name => DocumentSchema::Union(
                        vec![
                            object_validator!(
                                "a" => FieldValidator::optional_field_type(Validator::Float64),
                            )
                        ]
                    )
                );
                // This succeeds (even in prod) because the table is Hidden.
                activate_schema(&app, mismatch_schema).await?;
                pause_guard.unpause();
            },
        }
        let err = import_fut.await.unwrap_err();
        assert!(err.is_bad_request());
        assert!(
            err.msg()
                .contains("Could not complete import because schema changed"),
            "{err:?}"
        );

        Ok(())
    }

    #[convex_macro::test_runtime]
    async fn import_would_break_foreign_key(rt: TestRuntime) -> anyhow::Result<()> {
        let app = Application::new_for_tests(&rt).await?;
        let table_name = "table1";
        let table_with_foreign_key = "table_with_foreign_key";
        let identity = new_admin_id();

        {
            let mut tx = app.begin(identity).await?;
            let validated_id = UserFacingModel::new_root_for_test(&mut tx)
                .insert(table_name.parse()?, assert_obj!())
                .await?;
            UserFacingModel::new_root_for_test(&mut tx)
                .insert(
                    table_with_foreign_key.parse()?,
                    assert_obj!(
                        "a" => validated_id.encode()
                    ),
                )
                .await?;
            app.commit_test(tx).await?;
        }

        // table1 initially has number 10001
        // table_with_foreign_key has number 10002
        // Import table1 with number 10003
        let test_csv = r#"
_id,a
"jd7f2yq3tcc5h4ce9qhqdk0ach6hbmyb","string"
"#;

        let initial_schema = db_schema!(
            table_with_foreign_key => DocumentSchema::Union(
                vec![
                    object_validator!(
                        "a" => FieldValidator::optional_field_type(Validator::Id(table_name.parse()?)),
                    )
                ]
            )
        );

        activate_schema(&app, initial_schema).await?;

        let err = run_csv_import(&app, table_name, test_csv)
            .await
            .unwrap_err();
        assert!(err.is_bad_request());
        assert_eq!(
            err.msg(),
            "Hit an error while importing:\nImport changes table 'table1' which is referenced by \
             'table_with_foreign_key' in the schema"
        );
        Ok(())
    }

    #[convex_macro::test_runtime]
    async fn import_preserves_foreign_key(rt: TestRuntime) -> anyhow::Result<()> {
        let app = Application::new_for_tests(&rt).await?;
        let table_name = "table1";
        let identity = new_admin_id();

        {
            let mut tx = app.begin(identity).await?;
            UserFacingModel::new_root_for_test(&mut tx)
                .insert(table_name.parse()?, assert_obj!())
                .await?;
            app.commit_test(tx).await?;
        }

        let table_with_foreign_key = "table_with_foreign_key";
        // table1 initially has number 10001
        // table_with_foreign_key has number 10002
        // Import table1 with number 10001 (clearing the table)
        let test_csv = r#"
a
"#;

        let initial_schema = db_schema!(
            table_with_foreign_key => DocumentSchema::Union(
                vec![
                    object_validator!(
                        "a" => FieldValidator::optional_field_type(Validator::Id(table_name.parse()?)),
                    )
                ]
            )
        );

        activate_schema(&app, initial_schema).await?;

        run_csv_import(&app, table_name, test_csv).await?;
        Ok(())
    }

    #[convex_macro::test_runtime]
    async fn import_copies_indexes(rt: TestRuntime) -> anyhow::Result<()> {
        let app = Application::new_for_tests(&rt).await?;
        let table_name: TableName = "table1".parse()?;
        let test_csv = r#"
a
"string"
"#;
        let identity = new_admin_id();
        let index_name = IndexName::new(table_name.clone(), "by_a".parse()?)?;

        let index_id = {
            let mut tx = app.begin(identity.clone()).await?;
            let mut index_model = IndexModel::new(&mut tx);
            let index_id = index_model
                .add_application_index(
                    TableNamespace::test_user(),
                    IndexMetadata::new_enabled(index_name.clone(), vec!["a".parse()?].try_into()?),
                )
                .await?;
            app.commit_test(tx).await?;
            index_id
        };

        run_csv_import(&app, &table_name, test_csv).await?;

        {
            let mut tx = app.begin(identity.clone()).await?;
            let mut index_model = IndexModel::new(&mut tx);
            let index = index_model
                .enabled_index_metadata(TableNamespace::test_user(), &index_name)?
                .context("index does not exist")?;
            assert_ne!(index.id(), index_id);
            assert!(index.config.is_enabled());
            must_let!(let IndexConfig::Database { developer_config, .. } = &index.config);
            assert_eq!(developer_config.fields[0], "a".parse()?);
        }

        Ok(())
    }

    #[convex_macro::test_runtime]
    async fn test_import_counts_bandwidth(rt: TestRuntime) -> anyhow::Result<()> {
        let app = Application::new_for_tests(&rt).await?;
        let table_name: TableName = "table1".parse()?;
        let identity = new_admin_id();

        let storage_id = "kg21pzwemsm55e1fnt2kcsvgjh6h6gtf";
        let storage_idv6 = DeveloperDocumentId::decode(storage_id)?;

        let objects = stream::iter(vec![
            Ok(ImportUnit::NewTable("_storage".parse()?)),
            Ok(ImportUnit::Object(json!({"_id": storage_id}))),
            Ok(ImportUnit::StorageFileChunk(
                storage_idv6,
                Bytes::from_static(b"foobarbaz"),
            )),
            Ok(ImportUnit::NewTable(table_name.clone())),
            Ok(ImportUnit::Object(json!({"foo": "bar"}))),
            Ok(ImportUnit::Object(json!({"foo": "baz"}))),
        ])
        .boxed()
        .peekable();

        let usage = FunctionUsageTracker::new();

        import_objects(
            &app.database,
            &app.file_storage,
            identity,
            ImportMode::Replace,
            objects,
            usage.clone(),
            None,
        )
        .await?;

        let stats = usage.gather_user_stats();
        assert!(stats.database_ingress_size[&table_name.to_string()] > 0);
        assert_eq!(stats.storage_ingress_size, 9);

        Ok(())
    }

    async fn activate_schema<RT: Runtime>(
        app: &Application<RT>,
        schema: DatabaseSchema,
    ) -> anyhow::Result<()> {
        let mut tx = app.begin(new_admin_id()).await?;
        let mut model = SchemaModel::new_root_for_test(&mut tx);
        let (schema_id, _) = model.submit_pending(schema).await?;
        model.mark_validated(schema_id).await?;
        model.mark_active(schema_id).await?;
        app.commit_test(tx).await?;
        Ok(())
    }

    /// Returns a BTreeMap for every item in the given table that contains only
    /// the requesetd fields provided in `relevant_fields`. If one or more
    /// fields in `relevant_fields` are missing in one or more objects in the
    /// table, then the returned BTreeMap will not have an entry for the
    /// missing fields.
    async fn load_fields_as_maps<'a, RT: Runtime>(
        app: &Application<RT>,
        table_name: &str,
        relevant_fields: Vec<&'a str>,
    ) -> anyhow::Result<Vec<BTreeMap<&'a str, ConvexValue>>> {
        let mut tx = app.begin(new_admin_id()).await?;
        let table_name = TableName::from_str(table_name)?;
        let query = common::query::Query::full_table_scan(table_name.clone(), Order::Asc);
        let mut query_stream = ResolvedQuery::new(&mut tx, TableNamespace::test_user(), query)?;

        let mut docs: Vec<ResolvedDocument> = Vec::new();
        while let Some(doc) = query_stream.next(&mut tx, None).await? {
            docs.push(doc);
            if docs.len() % 100 == 0 {
                // Occasionally start a new transaction in case there are lots
                // of documents.
                tx = app.begin(new_admin_id()).await?;
            }
        }

        let objects: Vec<ConvexObject> = docs.into_iter().map(|doc| doc.into_value().0).collect();

        let mut fields_list: Vec<BTreeMap<&str, ConvexValue>> = Vec::default();
        for object in objects {
            let mut current = BTreeMap::default();
            for field in &relevant_fields {
                let value = object.get(&FieldName::from_str(field)?);
                if let Some(value) = value {
                    current.insert(*field, value.clone());
                }
            }
            fields_list.push(current);
        }
        Ok(fields_list)
    }

    fn new_admin_id() -> Identity {
        Identity::InstanceAdmin(AdminIdentity::new_for_test_only(
            "test".to_string(),
            MemberId(1),
        ))
    }

    async fn run_csv_import(
        app: &Application<TestRuntime>,
        table_name: &str,
        input: &str,
    ) -> anyhow::Result<()> {
        do_import(
            app,
            new_admin_id(),
            ImportFormat::Csv(table_name.parse()?),
            ImportMode::Replace,
            stream_from_str(input),
        )
        .await
        .map(|_| ())
    }
}

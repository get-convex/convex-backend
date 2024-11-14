use std::{
    collections::{
        BTreeMap,
        BTreeSet,
        HashSet,
    },
    pin::Pin,
    str::FromStr,
    sync::Arc,
    time::Duration,
};

use anyhow::Context;
use async_recursion::async_recursion;
use async_trait::async_trait;
use bytes::Bytes;
use common::{
    bootstrap_model::{
        components::{
            definition::{
                ComponentDefinitionMetadata,
                ComponentDefinitionType,
            },
            ComponentMetadata,
            ComponentState,
            ComponentType,
        },
        schema::SchemaState,
        tables::TABLES_TABLE,
    },
    components::{
        ComponentDefinitionPath,
        ComponentId,
        ComponentName,
        ComponentPath,
    },
    document::{
        CreationTime,
        ParsedDocument,
        CREATION_TIME_FIELD,
        ID_FIELD,
    },
    execution_context::ExecutionId,
    knobs::{
        MAX_IMPORT_AGE,
        TRANSACTION_MAX_NUM_USER_WRITES,
        TRANSACTION_MAX_USER_WRITE_SIZE_BYTES,
    },
    pause::PauseClient,
    runtime::Runtime,
    schemas::DatabaseSchema,
    types::{
        MemberId,
        ObjectKey,
        StorageUuid,
        TableName,
        UdfIdentifier,
    },
};
use database::{
    BootstrapComponentsModel,
    Database,
    ImportFacingModel,
    IndexModel,
    SchemaModel,
    SystemMetadataModel,
    TableModel,
    Transaction,
    TransactionReadSet,
    COMPONENTS_TABLE,
    SCHEMAS_TABLE,
};
use errors::{
    ErrorMetadata,
    ErrorMetadataAnyhowExt,
};
use file_storage::FileStorage;
use futures::{
    pin_mut,
    stream::{
        self,
        BoxStream,
        Peekable,
    },
    Stream,
    StreamExt,
    TryStream,
    TryStreamExt,
};
use futures_async_stream::stream;
use headers::{
    ContentLength,
    ContentType,
};
use itertools::Itertools;
use keybroker::Identity;
use maplit::{
    btreemap,
    btreeset,
};
use model::{
    components::config::{
        ComponentConfigModel,
        ComponentDefinitionConfigModel,
    },
    deployment_audit_log::{
        types::DeploymentAuditLogEvent,
        DeploymentAuditLogModel,
    },
    file_storage::{
        FILE_STORAGE_TABLE,
        FILE_STORAGE_VIRTUAL_TABLE,
    },
    snapshot_imports::{
        types::{
            ImportFormat,
            ImportMode,
            ImportRequestor,
            ImportState,
            ImportTableCheckpoint,
            SnapshotImport,
        },
        SnapshotImportModel,
    },
};
use serde_json::Value as JsonValue;
use shape_inference::{
    export_context::GeneratedSchema,
    ProdConfigWithOptionalFields,
};
use storage::{
    Storage,
    StorageExt,
    StorageObjectReader,
};
use sync_types::{
    backoff::Backoff,
    Timestamp,
};
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
    metrics::log_snapshot_import_age,
    snapshot_import::{
        import_error::ImportError,
        parse::{
            parse_objects,
            ImportUnit,
        },
        table_change::{
            render_table_changes,
            TableChange,
        },
    },
    Application,
};

mod import_error;
mod parse;
mod table_change;
#[cfg(test)]
mod tests;
mod worker;

pub use worker::SnapshotImportWorker;

struct SnapshotImportExecutor<RT: Runtime> {
    runtime: RT,
    database: Database<RT>,
    snapshot_imports_storage: Arc<dyn Storage>,
    file_storage: FileStorage<RT>,
    usage_tracking: UsageCounter,
    backoff: Backoff,
    pause_client: PauseClient,
}

impl<RT: Runtime> SnapshotImportExecutor<RT> {
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
        self.fail_if_too_old(&snapshot_import)?;
        match self.info_message_for_import(snapshot_import).await {
            Ok((info_message, require_manual_confirmation, new_checkpoints)) => {
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
                                        new_checkpoints.clone(),
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
    ) -> anyhow::Result<(String, bool, Vec<ImportTableCheckpoint>)> {
        let mut message_lines = Vec::new();
        let (content_confirmation_messages, require_manual_confirmation, new_checkpoints) =
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
        Ok((
            message_lines.join("\n"),
            require_manual_confirmation,
            new_checkpoints,
        ))
    }

    async fn messages_to_confirm_replace(
        &self,
        snapshot_import: ParsedDocument<SnapshotImport>,
    ) -> anyhow::Result<(Vec<String>, bool, Vec<ImportTableCheckpoint>)> {
        let mode = snapshot_import.mode;
        let (_, mut objects) = self.parse_import(snapshot_import.id()).await?;
        // Find all tables being written to.
        let mut count_by_table: BTreeMap<(ComponentPath, TableName), u64> = BTreeMap::new();
        let mut tables_missing_id_field: BTreeSet<(ComponentPath, TableName)> = BTreeSet::new();
        let mut current_table = None;
        let mut lineno = 0;
        while let Some(object) = objects.try_next().await? {
            match object {
                ImportUnit::NewTable(component_path, table_name) => {
                    lineno = 0;
                    count_by_table
                        .entry((component_path.clone(), table_name.clone()))
                        .or_default();
                    current_table = Some((component_path, table_name));
                },
                ImportUnit::Object(exported_value) => {
                    lineno += 1;
                    let Some(current_component_table) = &current_table else {
                        continue;
                    };
                    let (current_component, current_table) = current_component_table;
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
                        count_by_table
                            .entry((current_component.clone(), table_name))
                            .or_default();
                    }
                    if let Some(count) = count_by_table.get_mut(current_component_table) {
                        *count += 1;
                    }
                    if !tables_missing_id_field.contains(current_component_table)
                        && exported_value.get(&**ID_FIELD).is_none()
                    {
                        tables_missing_id_field.insert(current_component_table.clone());
                    }
                },
                // Ignore storage file chunks and generated schemas.
                ImportUnit::StorageFileChunk(..) | ImportUnit::GeneratedSchema(..) => {},
            }
        }

        let mut table_changes = BTreeMap::new();
        let db_snapshot = self.database.latest_snapshot()?;
        for (component_and_table, count_importing) in count_by_table.iter() {
            let (component_path, table_name) = component_and_table;
            let existing_num_values = db_snapshot
                .component_registry
                .component_path_to_ids(component_path, &mut TransactionReadSet::new())?
                .map(|(_, component_id)| {
                    let table_name = if table_name == &*FILE_STORAGE_VIRTUAL_TABLE {
                        &*FILE_STORAGE_TABLE
                    } else {
                        table_name
                    };
                    let table_summary =
                        db_snapshot.must_table_summary(component_id.into(), table_name)?;
                    anyhow::Ok(table_summary.num_values())
                })
                .transpose()?
                .unwrap_or(0);
            if !table_name.is_system() {
                let to_delete = match mode {
                    ImportMode::Replace | ImportMode::ReplaceAll => {
                        // Overwriting nonempty user table.
                        existing_num_values
                    },
                    ImportMode::Append => 0,
                    ImportMode::RequireEmpty if existing_num_values > 0 => {
                        anyhow::bail!(ImportError::TableExists(table_name.clone()))
                    },
                    ImportMode::RequireEmpty => 0,
                };
                table_changes.insert(
                    component_and_table.clone(),
                    TableChange {
                        added: *count_importing,
                        deleted: to_delete,
                        existing: existing_num_values,
                        unit: "",
                        is_missing_id_field: tables_missing_id_field.contains(component_and_table),
                    },
                );
            }
            if table_name == &*FILE_STORAGE_VIRTUAL_TABLE {
                let to_delete = match mode {
                    ImportMode::Replace | ImportMode::ReplaceAll => {
                        // Overwriting nonempty file storage.
                        existing_num_values
                    },
                    ImportMode::Append => 0,
                    ImportMode::RequireEmpty if existing_num_values > 0 => {
                        anyhow::bail!(ImportError::TableExists(table_name.clone()))
                    },
                    ImportMode::RequireEmpty => 0,
                };
                table_changes.insert(
                    component_and_table.clone(),
                    TableChange {
                        added: *count_importing,
                        deleted: to_delete,
                        existing: existing_num_values,
                        unit: " files",
                        is_missing_id_field: tables_missing_id_field.contains(component_and_table),
                    },
                );
            }
        }
        let mut require_manual_confirmation = false;
        let mut new_checkpoints = Vec::new();

        for (
            (component_path, table_name),
            TableChange {
                added,
                deleted,
                existing,
                unit: _,
                is_missing_id_field,
            },
        ) in table_changes.iter()
        {
            if *deleted > 0 {
                // Deleting files can be destructive, so require confirmation.
                require_manual_confirmation = true;
            }
            new_checkpoints.push(ImportTableCheckpoint {
                component_path: component_path.clone(),
                display_table_name: table_name.clone(),
                tablet_id: None,
                num_rows_written: 0,
                total_num_rows_to_write: *added as i64,
                existing_rows_to_delete: *deleted as i64,
                existing_rows_in_table: *existing as i64,
                is_missing_id_field: *is_missing_id_field,
            });
        }
        let mut message_lines = Vec::new();
        for (component_path, table_changes) in &table_changes
            .into_iter()
            .chunk_by(|((component_path, _), _)| component_path.clone())
        {
            if !component_path.is_root() {
                message_lines.push(format!("Component {}", String::from(component_path)));
            }
            message_lines.extend(render_table_changes(table_changes.collect()).into_iter());
        }
        Ok((message_lines, require_manual_confirmation, new_checkpoints))
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

    fn fail_if_too_old(
        &self,
        snapshot_import: &ParsedDocument<SnapshotImport>,
    ) -> anyhow::Result<()> {
        if let Some(creation_time) = snapshot_import.creation_time() {
            let now = CreationTime::try_from(*self.database.now_ts_for_reads())?;
            let age = Duration::from_millis((f64::from(now) - f64::from(creation_time)) as u64);
            log_snapshot_import_age(age);
            if age > *MAX_IMPORT_AGE / 2 {
                tracing::warn!(
                    "SnapshotImport {} running too long ({:?})",
                    snapshot_import.id(),
                    age
                );
            }
            if age > *MAX_IMPORT_AGE {
                anyhow::bail!(ErrorMetadata::bad_request(
                    "ImportFailed",
                    "Import took too long. Try again or contact Convex."
                ));
            }
        }
        Ok(())
    }

    async fn attempt_perform_import(
        &mut self,
        snapshot_import: ParsedDocument<SnapshotImport>,
    ) -> anyhow::Result<(Timestamp, u64)> {
        self.fail_if_too_old(&snapshot_import)?;
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
            snapshot_import.requestor.clone(),
            &self.usage_tracking,
        )
        .await?;

        // Truncate list of table names to avoid storing too much data in
        // audit log object.
        let table_names: Vec<_> = table_mapping_for_import
            .table_mapping_in_import
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
        let table_count = table_mapping_for_import
            .table_mapping_in_import
            .iter()
            .count() as u64;
        let mut table_names_deleted = table_mapping_for_import.deleted_for_audit_log();
        let table_count_deleted = table_names_deleted.len() as u64;
        table_names_deleted = table_names_deleted.into_iter().take(20).collect();

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
                requestor: snapshot_import.requestor.clone(),
                table_names_deleted,
                table_count_deleted,
            },
            snapshot_import.requestor.clone(),
        )
        .await?;
        let object_attributes = self
            .snapshot_imports_storage
            .get_object_attributes(&snapshot_import.object_key)
            .await?
            .context("error getting export object attributes from S3")?;

        // Charge file bandwidth for the download of the snapshot from imports storage
        self.usage_tracking.track_independent_storage_egress_size(
            ComponentPath::root(),
            snapshot_import.requestor.usage_tag().to_string(),
            object_attributes.size,
        );

        Ok((ts, total_documents_imported))
    }

    async fn parse_import(
        &self,
        import_id: ResolvedDocumentId,
    ) -> anyhow::Result<(
        SchemasForImport,
        Peekable<BoxStream<'_, anyhow::Result<ImportUnit>>>,
    )> {
        let (object_key, format, component_path) = {
            let mut tx = self.database.begin(Identity::system()).await?;
            let mut model = SnapshotImportModel::new(&mut tx);
            let snapshot_import = model.get(import_id).await?.context("import not found")?;
            (
                snapshot_import.object_key.clone(),
                snapshot_import.format.clone(),
                snapshot_import.component_path.clone(),
            )
        };
        let body_stream = move || {
            let object_key = object_key.clone();
            async move { self.read_snapshot_import(&object_key).await }
        };
        let objects = parse_objects(format.clone(), component_path.clone(), body_stream).boxed();

        let component_id = prepare_component_for_import(&self.database, &component_path).await?;
        // Remapping could be more extensive here, it's just relatively simple to handle
        // optional types. We do remapping after parsing rather than during parsing
        // because it seems expensive to read the data for and parse all objects inside
        // of a transaction, though I haven't explicitly tested the performance.
        let mut tx = self.database.begin(Identity::system()).await?;
        let initial_schemas = schemas_for_import(&mut tx).await?;
        let objects = match format {
            ImportFormat::Csv(table_name) => {
                remap_empty_string_by_schema(
                    TableNamespace::from(component_id),
                    table_name,
                    &mut tx,
                    objects,
                )
                .await?
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

pub async fn start_stored_import<RT: Runtime>(
    application: &Application<RT>,
    identity: Identity,
    format: ImportFormat,
    mode: ImportMode,
    component_path: ComponentPath,
    object_key: ObjectKey,
    requestor: ImportRequestor,
) -> anyhow::Result<DeveloperDocumentId> {
    if !(identity.is_admin() || identity.is_system()) {
        anyhow::bail!(ImportError::Unauthorized);
    }
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
                        .start_import(
                            format.clone(),
                            mode,
                            component_path.clone(),
                            object_key.clone(),
                            requestor.clone(),
                        )
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

pub async fn cancel_import<RT: Runtime>(
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
            "snapshot_import_cancel",
            |tx| {
                async {
                    let import_id = import_id.to_resolved(
                        tx.table_mapping()
                            .namespace(TableNamespace::Global)
                            .number_to_tablet(),
                    )?;
                    let mut import_model = SnapshotImportModel::new(tx);
                    import_model.cancel_import(import_id).await?;
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
    component_path: ComponentPath,
    body_stream: BoxStream<'_, anyhow::Result<Bytes>>,
) -> anyhow::Result<u64> {
    let object_key = application.upload_snapshot_import(body_stream).await?;
    let import_id = start_stored_import(
        application,
        identity.clone(),
        format,
        mode,
        component_path,
        object_key,
        ImportRequestor::SnapshotImport,
    )
    .await?;

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
        } => Ok(*num_rows_written as u64),
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
    table_names: Vec<(ComponentPath, TableName)>,
) -> anyhow::Result<u64> {
    let usage = FunctionUsageTracker::new();

    let initial_schemas = {
        let mut tx = application.begin(identity.clone()).await?;
        schemas_for_import(&mut tx).await?
    };

    let objects = stream::iter(table_names.into_iter().map(|(component_path, table_name)| {
        anyhow::Ok(ImportUnit::NewTable(component_path, table_name))
    }))
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
        ImportRequestor::SnapshotImport,
        &application.usage_tracking,
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
        ImportRequestor::SnapshotImport,
    )
    .await?;
    Ok(documents_deleted)
}

async fn best_effort_update_progress_message<RT: Runtime>(
    database: &Database<RT>,
    identity: &Identity,
    import_id: ResolvedDocumentId,
    progress_message: String,
    component_path: &ComponentPath,
    display_table_name: &TableName,
    num_rows_written: i64,
) {
    // Ignore errors because it's not worth blocking or retrying if we can't
    // send a nice progress message on the first try.
    let _result: anyhow::Result<()> = try {
        let mut tx = database.begin(identity.clone()).await?;
        let mut import_model = SnapshotImportModel::new(&mut tx);
        import_model
            .update_progress_message(
                import_id,
                progress_message,
                component_path,
                display_table_name,
                num_rows_written,
            )
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
    component_path: &ComponentPath,
    display_table_name: &TableName,
    num_rows_written: i64,
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
                        .add_checkpoint_message(
                            import_id,
                            checkpoint_message.clone(),
                            component_path,
                            display_table_name,
                            num_rows_written,
                        )
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
    requestor: ImportRequestor,
    usage_tracking: &UsageCounter,
) -> anyhow::Result<(TableMappingForImport, u64)> {
    pin_mut!(objects);
    let mut generated_schemas = BTreeMap::new();
    let mut total_num_documents = 0;

    // In ReplaceAll mode, we want to delete all unaffected user tables
    // If there's a schema, then we want to clear it instead.
    let mut tx = database.begin(identity.clone()).await?;
    let to_delete = match mode {
        ImportMode::Append | ImportMode::Replace | ImportMode::RequireEmpty => BTreeMap::new(),
        ImportMode::ReplaceAll => tx
            .table_mapping()
            .iter_active_user_tables()
            .map(|(tablet_id, namespace, table_number, table_name)| {
                (tablet_id, (namespace, table_number, table_name.clone()))
            })
            .collect(),
    };

    let mut table_mapping_for_import = TableMappingForImport {
        table_mapping_in_import: TableMapping::new(),
        to_delete,
    };

    let all_component_paths = BootstrapComponentsModel::new(&mut tx).all_component_paths();
    for (tablet_id, (namespace, _table_number, table_name)) in
        table_mapping_for_import.to_delete.clone().into_iter()
    {
        let schema_tables = SchemaModel::new(&mut tx, namespace)
            .get_by_state(SchemaState::Active)
            .await?
            .map(|(_id, active_schema)| active_schema.tables)
            .unwrap_or_default();

        // Delete if it's not in the schema
        if !schema_tables.contains_key(&table_name) {
            continue;
        }

        let old_component_id: ComponentId = namespace.into();
        let component_path = all_component_paths.get(&old_component_id).context(
            "Existing user table had a namespace that was not found in all_component_paths()",
        )?;

        // For tables in the schema, clear them
        table_mapping_for_import.to_delete.remove(&tablet_id);
        let tables_affected = table_mapping_for_import.tables_affected();
        let (table_id, component_id, _num_to_skip) = prepare_table_for_import(
            database,
            &identity,
            mode,
            component_path,
            &table_name,
            None,
            &tables_affected,
            import_id,
        )
        .await?;

        table_mapping_for_import.table_mapping_in_import.insert(
            table_id.tablet_id,
            component_id.into(),
            table_id.table_number,
            table_name.clone(),
        );
    }

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
        requestor.clone(),
        usage_tracking,
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
    namespace: TableNamespace,
    // "foo" in example above.
    foreign_ref_table_in_import: (TableName, TableNumber),
    // "bar" in example above.
    table_in_schema_not_in_import: TableName,
}

impl ImportSchemaTableConstraint {
    async fn validate<RT: Runtime>(&self, tx: &mut Transaction<RT>) -> anyhow::Result<()> {
        let existing_table_mapping = tx.table_mapping();
        let Some(existing_table) = existing_table_mapping
            .namespace(self.namespace)
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
            .must_count(self.namespace, &self.table_in_schema_not_in_import)
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
    initial_schemas: SchemasForImport,
    table_constraints: BTreeSet<ImportSchemaTableConstraint>,
}

impl ImportSchemaConstraints {
    fn new(table_mapping_for_import: &TableMapping, initial_schemas: SchemasForImport) -> Self {
        let mut table_constraints = BTreeSet::new();
        for (namespace, _, (_, schema)) in initial_schemas.iter() {
            for (table, table_schema) in &schema.tables {
                if table_mapping_for_import
                    .namespace(*namespace)
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
                        .namespace(*namespace)
                        .id_and_number_if_exists(foreign_key_table)
                    {
                        table_constraints.insert(ImportSchemaTableConstraint {
                            namespace: *namespace,
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

struct TableMappingForImport {
    table_mapping_in_import: TableMapping,
    to_delete: BTreeMap<TabletId, (TableNamespace, TableNumber, TableName)>,
}

impl TableMappingForImport {
    fn tables_affected(&self) -> BTreeSet<TableName> {
        // TODO - include compenent here
        let mut tables_affected: BTreeSet<_> = self
            .table_mapping_in_import
            .iter()
            .map(|(_, _, _, table_name)| table_name.clone())
            .collect();
        tables_affected.extend(self.to_delete.values().map(|v| v.2.clone()));
        tables_affected
    }

    fn deleted_for_audit_log(&self) -> Vec<TableName> {
        // TODO - include the component path here
        self.to_delete
            .values()
            .filter(|(_namespace, _table_number, table_name)| {
                self.table_mapping_in_import
                    .namespaces_for_name(table_name)
                    .is_empty()
            })
            .map(|(_namespace, _table_number, table_name)| table_name.clone())
            .collect()
    }
}

async fn finalize_import<RT: Runtime>(
    database: &Database<RT>,
    usage_tracking: &UsageCounter,
    identity: Identity,
    member_id_override: Option<MemberId>,
    initial_schemas: SchemasForImport,
    table_mapping_for_import: TableMappingForImport,
    usage: FunctionUsageTracker,
    audit_log_event: DeploymentAuditLogEvent,
    requestor: ImportRequestor,
) -> anyhow::Result<(Timestamp, u64)> {
    let tables_affected = table_mapping_for_import.tables_affected();

    // Ensure that schemas will be valid after the tables are activated.
    let schema_constraints = ImportSchemaConstraints::new(
        &table_mapping_for_import.table_mapping_in_import,
        initial_schemas,
    );

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
                    for tablet_id in table_mapping_for_import.to_delete.keys() {
                        let namespace = tx.table_mapping().tablet_namespace(*tablet_id)?;
                        let table_name = tx.table_mapping().tablet_name(*tablet_id)?;
                        let mut table_model = TableModel::new(tx);
                        documents_deleted += table_model
                            .count(namespace, &table_name)
                            .await?
                            .unwrap_or(0);
                        table_model.delete_table(namespace, table_name).await?;
                    }
                    schema_constraints.validate(tx).await?;
                    let mut table_model = TableModel::new(tx);
                    for (table_id, _, table_number, table_name) in
                        table_mapping_for_import.table_mapping_in_import.iter()
                    {
                        documents_deleted += table_model
                            .activate_table(table_id, table_name, table_number, &tables_affected)
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

    let tag = requestor.usage_tag().to_string();
    let call_type = match requestor {
        ImportRequestor::SnapshotImport => CallType::Import,
        ImportRequestor::CloudRestore { .. } => CallType::CloudRestore,
    };
    // Charge database bandwidth accumulated during the import
    usage_tracking.track_call(
        UdfIdentifier::Cli(tag),
        ExecutionId::new(),
        call_type,
        usage.gather_user_stats(),
    );

    Ok((ts, documents_deleted))
}

type SchemasForImport = Vec<(
    TableNamespace,
    SchemaState,
    (ResolvedDocumentId, DatabaseSchema),
)>;

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
) -> anyhow::Result<SchemasForImport> {
    let mut namespaces = tx.table_mapping().namespaces_for_name(&SCHEMAS_TABLE);
    namespaces.sort();
    let mut schemas = vec![];
    for namespace in namespaces {
        let mut schema_model = SchemaModel::new(tx, namespace);
        for schema_state in [
            SchemaState::Active,
            SchemaState::Validated,
            SchemaState::Pending,
        ] {
            if let Some(schema) = schema_model.get_by_state(schema_state.clone()).await? {
                schemas.push((namespace, schema_state, schema));
            }
        }
    }
    Ok(schemas)
}

async fn import_tables_table<RT: Runtime>(
    database: &Database<RT>,
    identity: &Identity,
    mode: ImportMode,
    mut objects: Pin<&mut Peekable<BoxStream<'_, anyhow::Result<ImportUnit>>>>,
    component_path: &ComponentPath,
    import_id: Option<ResolvedDocumentId>,
    table_mapping_for_import: &mut TableMappingForImport,
) -> anyhow::Result<()> {
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
    let tables_affected = table_mapping_for_import.tables_affected();
    for (table_name, table_number) in import_tables.iter() {
        let (table_id, component_id, _) = prepare_table_for_import(
            database,
            identity,
            mode,
            component_path,
            table_name,
            Some(*table_number),
            &tables_affected,
            import_id,
        )
        .await?;
        table_mapping_for_import.table_mapping_in_import.insert(
            table_id.tablet_id,
            component_id.into(),
            table_id.table_number,
            table_name.clone(),
        );
    }
    Ok(())
}

async fn import_storage_table<RT: Runtime>(
    database: &Database<RT>,
    file_storage: &FileStorage<RT>,
    identity: &Identity,
    table_id: TabletIdAndTableNumber,
    component_path: &ComponentPath,
    mut objects: Pin<&mut Peekable<BoxStream<'_, anyhow::Result<ImportUnit>>>>,
    usage: &dyn StorageUsageTracker,
    import_id: Option<ResolvedDocumentId>,
    num_to_skip: u64,
    requestor: ImportRequestor,
    usage_tracking: &UsageCounter,
) -> anyhow::Result<()> {
    let snapshot = database.latest_snapshot()?;
    let namespace = snapshot
        .table_mapping()
        .tablet_namespace(table_id.tablet_id)?;
    let virtual_table_number = snapshot.table_mapping().tablet_number(table_id.tablet_id)?;
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
            id.table() == virtual_table_number,
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
            .map(|content_type| anyhow::Ok(ContentType::from_str(&content_type)?))
            .transpose()
            .map_err(|e| ImportError::InvalidConvexValue(lineno, e))?;
        let sha256 = metadata
            .sha256
            .map(|sha256| Sha256Digest::from_base64(&sha256))
            .transpose()
            .map_err(|e| ImportError::InvalidConvexValue(lineno, e))?;
        let storage_id = metadata
            .internal_id
            .map(|storage_id| {
                StorageUuid::from_str(&storage_id).context("Couldn't parse storage_id")
            })
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
        if num_files < num_to_skip {
            num_files += 1;
            continue;
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
                                namespace,
                                &id,
                                &table_mapping,
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
        let content_type = entry
            .content_type
            .as_ref()
            .map(|ct| ct.parse())
            .transpose()?;
        usage.track_storage_call(
            component_path.clone(),
            requestor.usage_tag(),
            entry.storage_id,
            content_type,
            entry.sha256,
        );
        usage_tracking.track_independent_storage_ingress_size(
            component_path.clone(),
            requestor.usage_tag().to_string(),
            file_size,
        );
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
                component_path,
                &FILE_STORAGE_VIRTUAL_TABLE,
                num_files as i64,
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
                "Imported \"_storage\"{} ({} files)",
                component_path.in_component_str(),
                num_files.separate_with_commas()
            ),
            component_path,
            &FILE_STORAGE_VIRTUAL_TABLE,
            num_files as i64,
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
    generated_schemas: &mut BTreeMap<
        (ComponentPath, TableName),
        GeneratedSchema<ProdConfigWithOptionalFields>,
    >,
    table_mapping_for_import: &mut TableMappingForImport,
    usage: FunctionUsageTracker,
    import_id: Option<ResolvedDocumentId>,
    requestor: ImportRequestor,
    usage_tracking: &UsageCounter,
) -> anyhow::Result<Option<u64>> {
    while let Some(ImportUnit::GeneratedSchema(component_path, table_name, generated_schema)) =
        objects
            .as_mut()
            .try_next_if(|line| matches!(line, ImportUnit::GeneratedSchema(_, _, _)))
            .await?
    {
        generated_schemas.insert((component_path, table_name), generated_schema);
    }
    let mut component_and_table = match objects.try_next().await? {
        Some(ImportUnit::NewTable(component_path, table_name)) => (component_path, table_name),
        Some(_) => anyhow::bail!("parse_objects should start with NewTable"),
        // No more tables to import.
        None => return Ok(None),
    };
    let table_number_from_docs = table_number_for_import(objects.as_mut()).await;
    if let Some(import_id) = import_id {
        best_effort_update_progress_message(
            database,
            identity,
            import_id,
            format!(
                "Importing \"{}\"{}",
                component_and_table.1,
                component_and_table.0.in_component_str()
            ),
            &component_and_table.0,
            &component_and_table.1,
            0,
        )
        .await;
    }

    let table_name = &mut component_and_table.1;
    if *table_name == *FILE_STORAGE_VIRTUAL_TABLE {
        *table_name = FILE_STORAGE_TABLE.clone();
    }
    let (component_path, table_name) = &component_and_table;
    let component_id = prepare_component_for_import(database, component_path).await?;

    if *table_name == *TABLES_TABLE {
        import_tables_table(
            database,
            identity,
            mode,
            objects.as_mut(),
            component_path,
            import_id,
            table_mapping_for_import,
        )
        .await?;
        return Ok(Some(0));
    }

    let mut generated_schema = generated_schemas.get_mut(&component_and_table);
    let tables_affected = table_mapping_for_import.tables_affected();
    let (table_id, num_to_skip) = match table_mapping_for_import
        .table_mapping_in_import
        .namespace(component_id.into())
        .id_and_number_if_exists(table_name)
    {
        Some(table_id) => {
            let mut tx = database.begin(identity.clone()).await?;
            let num_to_skip = if tx.table_mapping().is_active(table_id.tablet_id) {
                0
            } else {
                TableModel::new(&mut tx)
                    .must_count_tablet(table_id.tablet_id)
                    .await?
            };
            (table_id, num_to_skip)
        },
        None => {
            let (table_id, component_id, num_to_skip) = prepare_table_for_import(
                database,
                identity,
                mode,
                component_path,
                table_name,
                table_number_from_docs,
                &tables_affected,
                import_id,
            )
            .await?;
            table_mapping_for_import.table_mapping_in_import.insert(
                table_id.tablet_id,
                component_id.into(),
                table_id.table_number,
                table_name.clone(),
            );
            (table_id, num_to_skip)
        },
    };

    if *table_name == *FILE_STORAGE_TABLE {
        import_storage_table(
            database,
            file_storage,
            identity,
            table_id,
            component_path,
            objects.as_mut(),
            &usage,
            import_id,
            num_to_skip,
            requestor,
            usage_tracking,
        )
        .await?;
        return Ok(Some(0));
    }

    let mut num_objects = 0;

    let mut tx = database.begin(identity.clone()).await?;
    let mut table_mapping_for_schema = tx.table_mapping().clone();
    table_mapping_for_schema.update(table_mapping_for_import.table_mapping_in_import.clone());
    let mut objects_to_insert = vec![];
    let mut objects_to_insert_size = 0;
    // Peek so we don't pop ImportUnit::NewTable items.
    while let Some(ImportUnit::Object(exported_value)) = objects
        .as_mut()
        .try_next_if(|line| matches!(line, ImportUnit::Object(_)))
        .await?
    {
        if num_objects < num_to_skip {
            num_objects += 1;
            continue;
        }
        let row_number = (num_objects + 1) as usize;
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
                table_name,
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
                    component_path,
                    table_name,
                    num_objects as i64,
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
        table_name,
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
                "Imported \"{table_name}\"{} ({} documents)",
                component_path.in_component_str(),
                num_objects.separate_with_commas()
            ),
            component_path,
            table_name,
            num_objects as i64,
        )
        .await?;
    }

    Ok(Some(num_objects))
}

#[async_recursion]
async fn prepare_component_for_import<RT>(
    database: &Database<RT>,
    component_path: &ComponentPath,
) -> anyhow::Result<ComponentId>
where
    RT: Runtime,
{
    let mut tx = database.begin(Identity::system()).await?;
    if let Some(metadata) = BootstrapComponentsModel::new(&mut tx).resolve_path(component_path)? {
        let component_id = if metadata.component_type.is_root() {
            ComponentId::Root
        } else {
            ComponentId::Child(metadata.developer_id())
        };
        return Ok(component_id);
    }

    let Some((parent_path, component_name)) = component_path.parent() else {
        tracing::info!("Creating a root component during import");
        create_root_component(&mut tx).await?;
        database
            .commit_with_write_source(tx, "snapshot_import_create_root_component")
            .await?;
        return Ok(ComponentId::Root);
    };
    drop(tx);

    prepare_component_for_import(database, &parent_path).await?;

    tracing::info!("Creating component {component_name:?} during import");
    let component_id = create_unmounted_component(database, parent_path, component_name).await?;
    Ok(component_id)
}

async fn create_unmounted_component<RT: Runtime>(
    database: &Database<RT>,
    parent_path: ComponentPath,
    component_name: ComponentName,
) -> anyhow::Result<ComponentId> {
    let mut tx = database.begin(Identity::system()).await?;
    let component_id = ComponentConfigModel::new(&mut tx)
        .initialize_component_namespace(false)
        .await?;
    database
        .commit_with_write_source(tx, "snapshot_import_prepare_unmounted_component")
        .await?;
    database
        .load_indexes_into_memory(btreeset! { SCHEMAS_TABLE.clone() })
        .await?;

    let mut tx = database.begin(Identity::system()).await?;
    let definition = ComponentDefinitionMetadata {
        path: format!("{}", parent_path.join(component_name.clone())).parse()?,
        definition_type: ComponentDefinitionType::ChildComponent {
            name: component_name.clone(),
            args: btreemap! {},
        },
        child_components: vec![],
        http_mounts: btreemap! {},
        exports: btreemap! {},
    };
    let (definition_id, _diff) = ComponentDefinitionConfigModel::new(&mut tx)
        .create_component_definition(definition)
        .await?;
    let metadata = ComponentMetadata {
        definition_id,
        component_type: ComponentType::ChildComponent {
            parent: BootstrapComponentsModel::new(&mut tx)
                .resolve_path(&parent_path)?
                .context(format!(
                    "{parent_path:?} not found in create_unmounted_component"
                ))?
                .developer_id(),
            name: component_name,
            args: btreemap! {},
        },
        state: ComponentState::Unmounted,
    };
    SystemMetadataModel::new_global(&mut tx)
        .insert_with_internal_id(
            &COMPONENTS_TABLE,
            component_id.internal_id(),
            metadata.try_into()?,
        )
        .await?;
    database
        .commit_with_write_source(tx, "snapshot_import_insert_unmounted_component")
        .await?;
    Ok(ComponentId::Child(component_id))
}

async fn create_root_component<RT: Runtime>(tx: &mut Transaction<RT>) -> anyhow::Result<()> {
    let component_id = ComponentConfigModel::new(tx)
        .initialize_component_namespace(true)
        .await?;

    let definition = ComponentDefinitionMetadata {
        path: ComponentDefinitionPath::root(),
        definition_type: ComponentDefinitionType::App,
        child_components: vec![],
        http_mounts: btreemap! {},
        exports: btreemap! {},
    };

    let (definition_id, _diff) = ComponentDefinitionConfigModel::new(tx)
        .create_component_definition(definition)
        .await?;
    let metadata = ComponentMetadata {
        definition_id,
        component_type: ComponentType::App,
        state: ComponentState::Active,
    };
    SystemMetadataModel::new_global(tx)
        .insert_with_internal_id(
            &COMPONENTS_TABLE,
            component_id.internal_id(),
            metadata.try_into()?,
        )
        .await?;
    Ok(())
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
    if objects_to_insert.is_empty() {
        return Ok(());
    }
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
                                object_to_insert,
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
    component_path: &ComponentPath,
    table_name: &TableName,
    table_number: Option<TableNumber>,
    tables_affected: &BTreeSet<TableName>,
    import_id: Option<ResolvedDocumentId>,
) -> anyhow::Result<(TabletIdAndTableNumber, ComponentId, u64)> {
    anyhow::ensure!(
        table_name == &*FILE_STORAGE_TABLE || !table_name.is_system(),
        ErrorMetadata::bad_request(
            "InvalidTableName",
            format!("Invalid table name {table_name} starts with metadata prefix '_'")
        )
    );
    let display_table_name = if table_name == &*FILE_STORAGE_TABLE {
        &*FILE_STORAGE_VIRTUAL_TABLE
    } else {
        table_name
    };
    let mut tx = database.begin(identity.clone()).await?;
    let (_, component_id) = BootstrapComponentsModel::new(&mut tx)
        .component_path_to_ids(component_path)?
        .context(format!("Component {component_path:?} should exist by now"))?;
    let existing_active_table_id = tx
        .table_mapping()
        .namespace(component_id.into())
        .id_and_number_if_exists(table_name);
    let existing_checkpoint = match import_id {
        Some(import_id) => {
            SnapshotImportModel::new(&mut tx)
                .get_table_checkpoint(import_id, component_path, display_table_name)
                .await?
        },
        None => None,
    };
    let existing_checkpoint_tablet = existing_checkpoint
        .as_ref()
        .and_then(|checkpoint| checkpoint.tablet_id);
    let (insert_into_existing_table_id, num_to_skip) = match existing_checkpoint_tablet {
        Some(tablet_id) => {
            let table_number = tx.table_mapping().tablet_number(tablet_id)?;
            let num_to_skip = TableModel::new(&mut tx)
                .must_count_tablet(tablet_id)
                .await?;
            (
                Some(TabletIdAndTableNumber {
                    tablet_id,
                    table_number,
                }),
                num_to_skip,
            )
        },
        None => {
            let tablet_id = match mode {
                ImportMode::Append => existing_active_table_id,
                ImportMode::RequireEmpty => {
                    if TableModel::new(&mut tx)
                        .must_count(component_id.into(), table_name)
                        .await?
                        != 0
                    {
                        anyhow::bail!(ImportError::TableExists(table_name.clone()));
                    }
                    None
                },
                ImportMode::Replace | ImportMode::ReplaceAll => None,
            };
            (tablet_id, 0)
        },
    };
    drop(tx);
    let table_id = if let Some(insert_into_existing_table_id) = insert_into_existing_table_id {
        insert_into_existing_table_id
    } else {
        let table_number = table_number.or(existing_active_table_id.map(|id| id.table_number));
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
                                component_id.into(),
                                table_name,
                                table_number,
                                tables_affected,
                            )
                            .await?;
                        IndexModel::new(tx)
                            .copy_indexes_to_table(
                                component_id.into(),
                                table_name,
                                table_id.tablet_id,
                            )
                            .await?;
                        if let Some(import_id) = import_id {
                            SnapshotImportModel::new(tx)
                                .checkpoint_tablet_created(
                                    import_id,
                                    component_path,
                                    display_table_name,
                                    table_id.tablet_id,
                                )
                                .await?;
                        }
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
    Ok((table_id, component_id, num_to_skip))
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
            Some(id_v6.table())
        },
        ImportUnit::NewTable(..) => None,
        ImportUnit::GeneratedSchema(..) => None,
        ImportUnit::StorageFileChunk(..) => None,
    }
}

async fn remap_empty_string_by_schema<'a, RT: Runtime>(
    namespace: TableNamespace,
    table_name: TableName,
    tx: &mut Transaction<RT>,
    objects: BoxStream<'a, anyhow::Result<ImportUnit>>,
) -> anyhow::Result<BoxStream<'a, anyhow::Result<ImportUnit>>> {
    if let Some((_, schema)) = SchemaModel::new(tx, namespace)
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
                unit @ ImportUnit::NewTable(..)
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

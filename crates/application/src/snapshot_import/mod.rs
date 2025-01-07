use std::{
    collections::{
        BTreeMap,
        BTreeSet,
        HashSet,
    },
    pin::Pin,
    sync::Arc,
    time::Duration,
};

use anyhow::Context;
use bytes::Bytes;
use common::{
    bootstrap_model::{
        schema::SchemaState,
        tables::TABLES_TABLE,
    },
    components::{
        ComponentId,
        ComponentPath,
    },
    document::{
        CreationTime,
        ParsedDocument,
        ID_FIELD,
    },
    errors::report_error,
    execution_context::ExecutionId,
    ext::TryPeekableExt,
    knobs::{
        MAX_IMPORT_AGE,
        TRANSACTION_MAX_NUM_USER_WRITES,
        TRANSACTION_MAX_USER_WRITE_SIZE_BYTES,
    },
    pause::PauseClient,
    runtime::Runtime,
    types::{
        FullyQualifiedObjectKey,
        MemberId,
        ObjectKey,
        TableName,
        UdfIdentifier,
    },
    RequestId,
};
use database::{
    BootstrapComponentsModel,
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
    pin_mut,
    stream::{
        self,
        BoxStream,
        Peekable,
    },
    StreamExt,
    TryStreamExt,
};
use keybroker::Identity;
use model::{
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
};
use sync_types::{
    backoff::Backoff,
    Timestamp,
};
use thousands::Separable;
use usage_tracking::{
    CallType,
    FunctionUsageTracker,
    UsageCounter,
};
use value::{
    id_v6::DeveloperDocumentId,
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
    snapshot_import::{
        audit_log::make_audit_log_event,
        confirmation::info_message_for_import,
        import_error::{
            wrap_import_err,
            ImportError,
        },
        import_file_storage::import_storage_table,
        metrics::log_snapshot_import_age,
        parse::{
            parse_objects,
            ImportUnit,
        },
        prepare_component::prepare_component_for_import,
        progress::{
            add_checkpoint_message,
            best_effort_update_progress_message,
        },
        schema_constraints::{
            schemas_for_import,
            ImportSchemaConstraints,
            SchemasForImport,
        },
    },
    Application,
};

mod audit_log;
mod confirmation;
mod import_error;
mod import_file_storage;
mod metrics;
mod parse;
mod prepare_component;
mod progress;
mod schema_constraints;
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
    async fn handle_uploaded_state(
        &self,
        snapshot_import: ParsedDocument<SnapshotImport>,
    ) -> anyhow::Result<()> {
        anyhow::ensure!(snapshot_import.state == ImportState::Uploaded);
        tracing::info!("Marking snapshot export as WaitingForConfirmation");
        let import_id = snapshot_import.id();
        self.fail_if_too_old(&snapshot_import)?;
        match info_message_for_import(self, snapshot_import).await {
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
                let mut e = wrap_import_err(e);
                if e.is_bad_request() {
                    report_error(&mut e).await;
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

    async fn handle_in_progress_state(
        &mut self,
        snapshot_import: ParsedDocument<SnapshotImport>,
    ) -> anyhow::Result<()> {
        anyhow::ensure!(matches!(
            snapshot_import.state,
            ImportState::InProgress { .. }
        ));
        let import_id = snapshot_import.id();
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
                let mut e = wrap_import_err(e);
                if e.is_bad_request() {
                    report_error(&mut e).await;
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

        let audit_log_event =
            make_audit_log_event(&self.database, &table_mapping_for_import, &snapshot_import)
                .await?;

        self.pause_client.wait("before_finalize_import").await;
        let (ts, _documents_deleted) = finalize_import(
            &self.database,
            &self.usage_tracking,
            Identity::system(),
            snapshot_import.member_id,
            initial_schemas,
            table_mapping_for_import,
            usage,
            audit_log_event,
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
            async move { self.snapshot_imports_storage.get_reader(&object_key).await }
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
    do_import_from_object_key(
        application,
        identity,
        format,
        mode,
        component_path,
        object_key,
    )
    .await
}

pub async fn do_import_from_fully_qualified_export<RT: Runtime>(
    application: &Application<RT>,
    identity: Identity,
    format: ImportFormat,
    mode: ImportMode,
    component_path: ComponentPath,
    export_object_key: FullyQualifiedObjectKey,
) -> anyhow::Result<u64> {
    let import_object_key: ObjectKey = application
        .snapshot_imports_storage
        .copy_object(export_object_key)
        .await?;
    do_import_from_object_key(
        application,
        identity,
        format,
        mode,
        component_path,
        import_object_key,
    )
    .await
}

async fn do_import_from_object_key<RT: Runtime>(
    application: &Application<RT>,
    identity: Identity,
    format: ImportFormat,
    mode: ImportMode,
    component_path: ComponentPath,
    object_key: ObjectKey,
) -> anyhow::Result<u64> {
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

    let mut tx = database.begin(identity.clone()).await?;
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
        // If it was written by the import, don't clear it or delete it.
        if table_mapping_for_import
            .table_mapping_in_import
            .namespace(namespace)
            .name_exists(&table_name)
        {
            table_mapping_for_import.to_delete.remove(&tablet_id);
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

    Ok((table_mapping_for_import, total_num_documents))
}

struct TableMappingForImport {
    table_mapping_in_import: TableMapping,
    to_delete: BTreeMap<TabletId, (TableNamespace, TableNumber, TableName)>,
}

impl TableMappingForImport {
    fn tables_imported(&self) -> BTreeSet<(TableNamespace, TableName)> {
        self.table_mapping_in_import
            .iter()
            .map(|(_, namespace, _, table_name)| (namespace, table_name.clone()))
            .collect()
    }

    fn tables_deleted(&self) -> BTreeSet<(TableNamespace, TableName)> {
        self.to_delete
            .values()
            .filter(|(namespace, _table_number, table_name)| {
                !self
                    .table_mapping_in_import
                    .namespace(*namespace)
                    .name_exists(table_name)
            })
            .map(|(namespace, _table_number, table_name)| (*namespace, table_name.clone()))
            .collect()
    }

    fn tables_affected(&self) -> BTreeSet<(TableNamespace, TableName)> {
        let mut tables_affected = self.tables_imported();
        tables_affected.extend(self.tables_deleted());
        tables_affected
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
        RequestId::new(),
        call_type,
        true,
        usage.gather_user_stats(),
    );

    Ok((ts, documents_deleted))
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

    let table_namespace: TableNamespace = {
        let mut tx = database.begin(identity.clone()).await?;
        let (_, component_id) = BootstrapComponentsModel::new(&mut tx)
            .component_path_to_ids(component_path)?
            .context(format!("Component {component_path:?} should exist by now"))?;
        component_id.into()
    };

    let tables_affected = table_mapping_for_import
        .tables_affected()
        .union(
            &import_tables
                .iter()
                .map(|(table_name, _)| (table_namespace, table_name.clone()))
                .collect(),
        )
        .cloned()
        .collect();
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
    tables_affected: &BTreeSet<(TableNamespace, TableName)>,
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

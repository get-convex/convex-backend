use std::sync::LazyLock;

use anyhow::Context;
use common::{
    components::ComponentPath,
    document::{
        ParsedDocument,
        ResolvedDocument,
    },
    maybe_val,
    query::{
        Expression,
        Order,
    },
    runtime::Runtime,
    types::ObjectKey,
};
use database::{
    patch_value,
    ResolvedQuery,
    SystemMetadataModel,
    Transaction,
};
use errors::ErrorMetadata;
use sync_types::Timestamp;
use types::ImportRequestor;
use value::{
    ConvexObject,
    ConvexValue,
    ResolvedDocumentId,
    TableName,
    TableNamespace,
    TabletId,
};

use self::types::{
    ImportFormat,
    ImportMode,
    ImportState,
    ImportTableCheckpoint,
    SnapshotImport,
};
use crate::{
    SystemIndex,
    SystemTable,
};

pub mod types;

pub static SNAPSHOT_IMPORTS_TABLE: LazyLock<TableName> = LazyLock::new(|| {
    "_snapshot_imports"
        .parse()
        .expect("Invalid built-in snapshot imports table")
});

pub struct SnapshotImportsTable;
impl SystemTable for SnapshotImportsTable {
    fn table_name(&self) -> &'static TableName {
        &SNAPSHOT_IMPORTS_TABLE
    }

    fn indexes(&self) -> Vec<SystemIndex> {
        vec![]
    }

    fn validate_document(&self, document: ResolvedDocument) -> anyhow::Result<()> {
        ParsedDocument::<SnapshotImport>::try_from(document).map(|_| ())
    }
}

pub struct SnapshotImportModel<'a, RT: Runtime> {
    tx: &'a mut Transaction<RT>,
}

impl<'a, RT: Runtime> SnapshotImportModel<'a, RT> {
    pub fn new(tx: &'a mut Transaction<RT>) -> Self {
        Self { tx }
    }

    pub async fn get(
        &mut self,
        id: ResolvedDocumentId,
    ) -> anyhow::Result<Option<ParsedDocument<SnapshotImport>>> {
        anyhow::ensure!(self
            .tx
            .table_mapping()
            .namespace(TableNamespace::Global)
            .tablet_matches_name(id.tablet_id, SnapshotImportsTable.table_name()));
        match self.tx.get(id).await? {
            None => Ok(None),
            Some(doc) => Ok(Some(doc.try_into()?)),
        }
    }

    pub async fn start_import(
        &mut self,
        format: ImportFormat,
        mode: ImportMode,
        component_path: ComponentPath,
        object_key: ObjectKey,
        requestor: ImportRequestor,
    ) -> anyhow::Result<ResolvedDocumentId> {
        let snapshot_import = SnapshotImport {
            state: ImportState::Uploaded,
            format,
            mode,
            component_path,
            object_key,
            member_id: self.tx.identity().member_id(),
            checkpoints: None,
            requestor,
        };
        let id = SystemMetadataModel::new_global(self.tx)
            .insert(
                SnapshotImportsTable.table_name(),
                snapshot_import.try_into()?,
            )
            .await?;
        Ok(id)
    }

    pub async fn must_get_state(&mut self, id: ResolvedDocumentId) -> anyhow::Result<ImportState> {
        let state = self
            .get(id)
            .await?
            .context(ErrorMetadata::not_found(
                "ImportNotFound",
                format!("import {id} not found"),
            ))?
            .state
            .clone();
        Ok(state)
    }

    async fn update_state(
        &mut self,
        id: ResolvedDocumentId,
        new_state: impl FnOnce(ImportState) -> ImportState,
    ) -> anyhow::Result<()> {
        let current_state = self.must_get_state(id).await?;
        let new_state = new_state(current_state.clone());
        match (&current_state, &new_state) {
            (ImportState::Uploaded, ImportState::WaitingForConfirmation { .. })
            | (ImportState::Uploaded, ImportState::Failed(..))
            | (ImportState::WaitingForConfirmation { .. }, ImportState::InProgress { .. })
            | (ImportState::WaitingForConfirmation { .. }, ImportState::Failed { .. })
            | (ImportState::InProgress { .. }, ImportState::InProgress { .. })
            | (ImportState::InProgress { .. }, ImportState::Completed { .. })
            | (ImportState::InProgress { .. }, ImportState::Failed(..)) => {},
            (..) => {
                anyhow::bail!("invalid import state transition {current_state:?} -> {new_state:?}")
            },
        }
        SystemMetadataModel::new_global(self.tx)
            .patch(
                id,
                patch_value!("state" => Some(ConvexValue::Object(new_state.try_into()?)))?,
            )
            .await?;
        Ok(())
    }

    async fn update_checkpoints(
        &mut self,
        id: ResolvedDocumentId,
        update_checkpoints: impl FnOnce(&mut Vec<ImportTableCheckpoint>),
    ) -> anyhow::Result<()> {
        let mut import = self.get(id).await?.context(ErrorMetadata::not_found(
            "ImportNotFound",
            format!("import {id} not found"),
        ))?;
        let mut checkpoints = import.checkpoints.clone().unwrap_or_default();
        update_checkpoints(&mut checkpoints);
        import.checkpoints = Some(checkpoints);
        SystemMetadataModel::new_global(self.tx)
            .replace(id, import.into_value().try_into()?)
            .await?;
        Ok(())
    }

    pub async fn mark_waiting_for_confirmation(
        &mut self,
        id: ResolvedDocumentId,
        info_message: String,
        require_manual_confirmation: bool,
        new_checkpoints: Vec<ImportTableCheckpoint>,
    ) -> anyhow::Result<()> {
        self.update_state(id, move |_| ImportState::WaitingForConfirmation {
            info_message,
            require_manual_confirmation,
        })
        .await?;
        self.update_checkpoints(id, move |checkpoints| {
            *checkpoints = new_checkpoints;
        })
        .await
    }

    pub async fn confirm_import(&mut self, id: ResolvedDocumentId) -> anyhow::Result<()> {
        let current_state = self.must_get_state(id).await?;
        // No-op if the import is already in progress or finished since the CLI may
        // show a confirmation prompt when the import was confirmed in the dashboard.
        if matches!(current_state, ImportState::WaitingForConfirmation { .. }) {
            self.update_state(id, move |_| ImportState::InProgress {
                progress_message: "Importing".to_string(),
                checkpoint_messages: vec![],
            })
            .await?;
        };
        Ok(())
    }

    pub async fn cancel_import(&mut self, id: ResolvedDocumentId) -> anyhow::Result<()> {
        let current_state = self.must_get_state(id).await?;
        match current_state {
            ImportState::Uploaded | ImportState::WaitingForConfirmation { .. } => {
                self.fail_import(id, "Import canceled".to_string()).await?
            },
            // TODO: support cancelling imports in progress
            ImportState::InProgress { .. } => anyhow::bail!("Cannot cancel an import in progress"),
            ImportState::Completed { .. } => anyhow::bail!(ErrorMetadata::bad_request(
                "CannotCancelImport",
                "Cannot cancel an import that has completed"
            )),
            ImportState::Failed(_) => anyhow::bail!(ErrorMetadata::bad_request(
                "CannotCancelImport",
                "Cannot cancel an import that has failed"
            )),
        }
        Ok(())
    }

    pub async fn complete_import(
        &mut self,
        id: ResolvedDocumentId,
        ts: Timestamp,
        num_rows_written: u64,
    ) -> anyhow::Result<()> {
        self.update_state(id, move |_| ImportState::Completed {
            ts,
            num_rows_written: num_rows_written as i64,
        })
        .await
    }

    pub async fn fail_import(
        &mut self,
        id: ResolvedDocumentId,
        error_message: String,
    ) -> anyhow::Result<()> {
        self.update_state(id, move |_| ImportState::Failed(error_message))
            .await
    }

    pub async fn checkpoint_tablet_created(
        &mut self,
        id: ResolvedDocumentId,
        component_path: &ComponentPath,
        table_name: &TableName,
        tablet_id: TabletId,
    ) -> anyhow::Result<()> {
        self.update_checkpoints(id, move |checkpoints| {
            if let Some(checkpoint) = checkpoints.iter_mut().find(|c| {
                c.component_path == *component_path && c.display_table_name == *table_name
            }) {
                checkpoint.tablet_id = Some(tablet_id);
            }
        })
        .await
    }

    pub async fn get_table_checkpoint(
        &mut self,
        id: ResolvedDocumentId,
        component_path: &ComponentPath,
        display_table_name: &TableName,
    ) -> anyhow::Result<Option<ImportTableCheckpoint>> {
        let Some(import) = self.get(id).await? else {
            return Ok(None);
        };
        let Some(checkpoints) = &import.checkpoints else {
            return Ok(None);
        };
        Ok(checkpoints
            .iter()
            .find(|c| {
                c.component_path == *component_path && c.display_table_name == *display_table_name
            })
            .cloned())
    }

    pub async fn add_checkpoint_message(
        &mut self,
        id: ResolvedDocumentId,
        checkpoint_message: String,
        component_path: &ComponentPath,
        display_table_name: &TableName,
        num_rows_written: i64,
    ) -> anyhow::Result<()> {
        let mut noop = false;
        let noop_ = &mut noop;
        self.update_checkpoints(id, move |checkpoints| {
            if let Some(checkpoint) = checkpoints.iter_mut().find(|c| {
                c.component_path == *component_path && c.display_table_name == *display_table_name
            }) {
                if num_rows_written <= checkpoint.num_rows_written {
                    *noop_ = true;
                    return;
                }
                checkpoint.num_rows_written = num_rows_written;
            }
        })
        .await?;
        self.update_state(id, move |state| {
            let (progress_message, mut checkpoint_messages) = match state {
                ImportState::InProgress {
                    progress_message,
                    checkpoint_messages,
                } => (progress_message, checkpoint_messages),
                _ => ("Importing".to_string(), vec![]),
            };
            if !checkpoint_messages.contains(&checkpoint_message) {
                checkpoint_messages.push(checkpoint_message.clone());
            }
            let progress_message = if noop {
                progress_message
            } else {
                checkpoint_message
            };
            ImportState::InProgress {
                progress_message,
                checkpoint_messages,
            }
        })
        .await
    }

    pub async fn update_progress_message(
        &mut self,
        id: ResolvedDocumentId,
        progress_message: String,
        component_path: &ComponentPath,
        display_table_name: &TableName,
        num_rows_written: i64,
    ) -> anyhow::Result<()> {
        let mut noop = false;
        let noop_ = &mut noop;
        self.update_checkpoints(id, move |checkpoints| {
            if let Some(checkpoint) = checkpoints.iter_mut().find(|c| {
                c.component_path == *component_path && c.display_table_name == *display_table_name
            }) {
                if checkpoint.num_rows_written > 0
                    && num_rows_written <= checkpoint.num_rows_written
                {
                    *noop_ = true;
                    return;
                }
                checkpoint.num_rows_written = num_rows_written;
            }
        })
        .await?;
        if noop {
            return Ok(());
        }
        self.update_state(id, move |state| {
            let checkpoint_messages = match state {
                ImportState::InProgress {
                    progress_message: _,
                    checkpoint_messages,
                } => checkpoint_messages,
                _ => vec![],
            };
            ImportState::InProgress {
                progress_message,
                checkpoint_messages,
            }
        })
        .await
    }

    pub async fn import_in_state(
        &mut self,
        import_state: ImportState,
    ) -> anyhow::Result<Option<ParsedDocument<SnapshotImport>>> {
        let import_state_type = ConvexObject::try_from(import_state)?
            .get("state")
            .context("should have state field")?
            .clone();
        let query =
            common::query::Query::full_table_scan(SNAPSHOT_IMPORTS_TABLE.clone(), Order::Asc)
                .filter(Expression::Eq(
                    // TODO(lee) change to use an index.
                    Box::new(Expression::Field("state.state".parse()?)),
                    Box::new(Expression::Literal(maybe_val!(import_state_type))),
                ));
        let mut query_stream = ResolvedQuery::new(self.tx, TableNamespace::Global, query)?;
        query_stream
            .next(self.tx, Some(1))
            .await?
            .map(|doc| doc.try_into())
            .transpose()
    }
}

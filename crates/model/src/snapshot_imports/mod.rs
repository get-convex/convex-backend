use std::sync::LazyLock;

use anyhow::Context;
use common::{
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
use value::{
    ConvexObject,
    ConvexValue,
    ResolvedDocumentId,
    TableName,
};

use self::types::{
    ImportFormat,
    ImportMode,
    ImportState,
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
            .number_matches_name(id.table().table_number, SnapshotImportsTable.table_name()));
        match self.tx.get(id).await? {
            None => Ok(None),
            Some(doc) => Ok(Some(doc.try_into()?)),
        }
    }

    pub async fn start_import(
        &mut self,
        format: ImportFormat,
        mode: ImportMode,
        object_key: ObjectKey,
    ) -> anyhow::Result<ResolvedDocumentId> {
        let snapshot_import = SnapshotImport {
            state: ImportState::Uploaded,
            format,
            mode,
            object_key,
            member_id: self.tx.identity().member_id(),
        };
        let id = SystemMetadataModel::new(self.tx)
            .insert(
                SnapshotImportsTable.table_name(),
                snapshot_import.try_into()?,
            )
            .await?;
        Ok(id)
    }

    async fn update_state(
        &mut self,
        id: ResolvedDocumentId,
        new_state: impl FnOnce(ImportState) -> ImportState,
    ) -> anyhow::Result<()> {
        let current_state = self
            .get(id)
            .await?
            .context(ErrorMetadata::not_found(
                "ImportNotFound",
                format!("import {id} not found"),
            ))?
            .state
            .clone();
        let new_state = new_state(current_state.clone());
        match (&current_state, &new_state) {
            (ImportState::Uploaded, ImportState::WaitingForConfirmation { .. })
            | (ImportState::Uploaded, ImportState::Failed(..))
            | (ImportState::WaitingForConfirmation { .. }, ImportState::InProgress { .. })
            | (ImportState::InProgress { .. }, ImportState::InProgress { .. })
            | (ImportState::InProgress { .. }, ImportState::Completed { .. })
            | (ImportState::InProgress { .. }, ImportState::Failed(..)) => {},
            (..) => {
                anyhow::bail!("invalid import state transition {current_state:?} -> {new_state:?}")
            },
        }
        SystemMetadataModel::new(self.tx)
            .patch(
                id,
                patch_value!("state" => Some(ConvexValue::Object(new_state.try_into()?)))?,
            )
            .await?;
        Ok(())
    }

    pub async fn mark_waiting_for_confirmation(
        &mut self,
        id: ResolvedDocumentId,
        info_message: String,
        require_manual_confirmation: bool,
    ) -> anyhow::Result<()> {
        self.update_state(id, move |_| ImportState::WaitingForConfirmation {
            info_message,
            require_manual_confirmation,
        })
        .await
    }

    pub async fn confirm_import(&mut self, id: ResolvedDocumentId) -> anyhow::Result<()> {
        self.update_state(id, move |_| ImportState::InProgress {
            progress_message: "Importing".to_string(),
            checkpoint_messages: vec![],
        })
        .await
    }

    pub async fn complete_import(
        &mut self,
        id: ResolvedDocumentId,
        ts: Timestamp,
        num_rows_written: usize,
    ) -> anyhow::Result<()> {
        self.update_state(id, move |_| ImportState::Completed {
            ts,
            num_rows_written,
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

    pub async fn add_checkpoint_message(
        &mut self,
        id: ResolvedDocumentId,
        checkpoint_message: String,
    ) -> anyhow::Result<()> {
        self.update_state(id, move |state| match state {
            ImportState::InProgress {
                progress_message,
                mut checkpoint_messages,
            } => {
                checkpoint_messages.push(checkpoint_message);
                ImportState::InProgress {
                    progress_message,
                    checkpoint_messages,
                }
            },
            _ => ImportState::InProgress {
                progress_message: "Importing".to_string(),
                checkpoint_messages: vec![checkpoint_message],
            },
        })
        .await
    }

    pub async fn update_progress_message(
        &mut self,
        id: ResolvedDocumentId,
        progress_message: String,
    ) -> anyhow::Result<()> {
        self.update_state(id, move |state| match state {
            ImportState::InProgress {
                progress_message: _,
                checkpoint_messages,
            } => ImportState::InProgress {
                progress_message,
                checkpoint_messages,
            },
            _ => ImportState::InProgress {
                progress_message,
                checkpoint_messages: vec![],
            },
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
        let mut query_stream = ResolvedQuery::new(self.tx, query)?;
        query_stream
            .next(self.tx, Some(1))
            .await?
            .map(|doc| doc.try_into())
            .transpose()
    }
}

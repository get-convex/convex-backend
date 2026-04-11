use std::sync::LazyLock;

use common::{
    document::ParsedDocument,
    runtime::Runtime,
};
use database::{
    SystemMetadataModel,
    Transaction,
};
use errors::ErrorMetadata;
use value::{
    TableName,
    TableNamespace,
};

use crate::{
    SystemIndex,
    SystemTable,
};

pub mod types;

use types::BackendState;

use self::types::PersistedBackendState;

pub static BACKEND_STATE_TABLE: LazyLock<TableName> = LazyLock::new(|| {
    "_backend_state"
        .parse()
        .expect("Invalid built-in backend_state table")
});

pub struct BackendStateTable;
impl SystemTable for BackendStateTable {
    type Metadata = PersistedBackendState;

    fn table_name() -> &'static TableName {
        &BACKEND_STATE_TABLE
    }

    fn indexes() -> Vec<SystemIndex<Self>> {
        vec![]
    }
}

pub struct BackendStateModel<'a, RT: Runtime> {
    tx: &'a mut Transaction<RT>,
}

impl<'a, RT: Runtime> BackendStateModel<'a, RT> {
    pub fn new(tx: &'a mut Transaction<RT>) -> Self {
        Self { tx }
    }

    pub async fn initialize(&mut self) -> anyhow::Result<()> {
        // Create _backend_state row initialized as running.
        SystemMetadataModel::new_global(self.tx)
            .insert(
                &BACKEND_STATE_TABLE,
                PersistedBackendState::Old(BackendState::Running).try_into()?,
            )
            .await?;
        Ok(())
    }

    pub async fn get_backend_state(&mut self) -> anyhow::Result<ParsedDocument<BackendState>> {
        let backend_state = self
            .tx
            .query_system(
                TableNamespace::Global,
                &SystemIndex::<BackendStateTable>::by_id(),
            )?
            .unique()
            .await?
            .ok_or_else(|| anyhow::anyhow!("Backend must have a state."))?;
        (*backend_state).clone().map(|bs| Ok(bs.to_old_lossy()))
    }

    pub async fn toggle_backend_state(&mut self, new_state: BackendState) -> anyhow::Result<()> {
        let (id, current_state) = self.get_backend_state().await?.into_id_and_value();
        anyhow::ensure!(
            current_state != new_state,
            ErrorMetadata::bad_request(
                "DeploymentAlreadyInState",
                format!("Deployment is already {new_state}")
            )
        );
        SystemMetadataModel::new_global(self.tx)
            .replace(id, PersistedBackendState::Old(new_state).try_into()?)
            .await?;
        Ok(())
    }
}

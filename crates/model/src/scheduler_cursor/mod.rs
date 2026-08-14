use std::sync::Arc;

use common::{
    document::ParsedDocument,
    runtime::Runtime,
};
use database::{
    SystemMetadataModel,
    Transaction,
};
use value::{
    TableName,
    TableNamespace,
};

use crate::{
    SystemIndex,
    SystemTable,
};

pub mod types;

use self::types::SchedulerCursor;

pub const SCHEDULER_CURSOR_TABLE: TableName = TableName::const_new("_scheduler_cursor");

pub struct SchedulerCursorTable;
impl SystemTable for SchedulerCursorTable {
    type Metadata = SchedulerCursor;

    const TABLE_NAME: TableName = SCHEDULER_CURSOR_TABLE;

    fn indexes() -> Vec<SystemIndex<Self>> {
        vec![]
    }
}

pub struct SchedulerCursorModel<'a, RT: Runtime> {
    tx: &'a mut Transaction<RT>,
    namespace: TableNamespace,
}

impl<'a, RT: Runtime> SchedulerCursorModel<'a, RT> {
    pub fn new(tx: &'a mut Transaction<RT>, namespace: TableNamespace) -> Self {
        Self { tx, namespace }
    }

    /// `None` before the executor has checkpointed this namespace for the first
    /// time, which is also the case for every deployment that predates this
    /// table.
    pub async fn get(&mut self) -> anyhow::Result<Option<SchedulerCursor>> {
        Ok(self.get_doc().await?.map(|doc| **doc))
    }

    async fn get_doc(&mut self) -> anyhow::Result<Option<Arc<ParsedDocument<SchedulerCursor>>>> {
        self.tx
            .query_system(
                self.namespace,
                &SystemIndex::<SchedulerCursorTable>::by_id(),
            )?
            .unique()
            .await
    }

    /// Replaces in place rather than appending, so the row keeps one document
    /// id and creation time and checkpointing never adds a key-change
    /// tombstone of its own.
    pub async fn set(&mut self, cursor: SchedulerCursor) -> anyhow::Result<()> {
        let existing = self.get_doc().await?.map(|doc| doc.id());
        let mut model = SystemMetadataModel::new(self.tx, self.namespace);
        match existing {
            Some(id) => {
                model.replace(id, cursor.try_into()?).await?;
            },
            None => {
                model
                    .insert(&SCHEDULER_CURSOR_TABLE, cursor.try_into()?)
                    .await?;
            },
        }
        Ok(())
    }
}

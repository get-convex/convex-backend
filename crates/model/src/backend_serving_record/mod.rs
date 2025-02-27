use std::sync::LazyLock;

use anyhow::Context;
use common::{
    document::{
        ParsedDocument,
        ResolvedDocument,
    },
    query::{
        Order,
        Query,
    },
    runtime::Runtime,
};
use database::{
    ResolvedQuery,
    SystemMetadataModel,
    Transaction,
};
use value::{
    ResolvedDocumentId,
    TableName,
    TableNamespace,
};

use self::types::BackendServingRecord;
use crate::{
    SystemIndex,
    SystemTable,
};

pub mod types;

pub static BACKEND_SERVING_RECORD_TABLE: LazyLock<TableName> = LazyLock::new(|| {
    "_backend_serving_record"
        .parse()
        .expect("_serving_backend is not a valid system table name")
});

pub struct BackendServingRecordTable;
impl SystemTable for BackendServingRecordTable {
    fn table_name(&self) -> &'static TableName {
        &BACKEND_SERVING_RECORD_TABLE
    }

    fn indexes(&self) -> Vec<SystemIndex> {
        vec![]
    }

    fn validate_document(&self, document: ResolvedDocument) -> anyhow::Result<()> {
        ParsedDocument::<BackendServingRecord>::try_from(document).map(|_| ())
    }
}

pub struct ServingBackendModel<'a, RT: Runtime> {
    tx: &'a mut Transaction<RT>,
}

impl<'a, RT: Runtime> ServingBackendModel<'a, RT> {
    pub fn new(tx: &'a mut Transaction<RT>) -> Self {
        Self { tx }
    }

    pub async fn add(
        &mut self,
        backend: BackendServingRecord,
    ) -> anyhow::Result<ParsedDocument<BackendServingRecord>> {
        let id = SystemMetadataModel::new_global(self.tx)
            .insert(&BACKEND_SERVING_RECORD_TABLE, backend.try_into()?)
            .await?;
        let x = self
            .tx
            .get(id)
            .await?
            .context("Newly inserted document missing")?;
        x.try_into()
    }

    pub async fn remove(&mut self, id: ResolvedDocumentId) -> anyhow::Result<()> {
        anyhow::ensure!(self
            .tx
            .table_mapping()
            .namespace(TableNamespace::Global)
            .tablet_matches_name(id.tablet_id, &BACKEND_SERVING_RECORD_TABLE));
        SystemMetadataModel::new_global(self.tx).delete(id).await?;
        Ok(())
    }

    // Returns list of past (and potentially current) serving backend records.
    pub async fn list(&mut self) -> anyhow::Result<Vec<ParsedDocument<BackendServingRecord>>> {
        let query = Query::full_table_scan(BACKEND_SERVING_RECORD_TABLE.clone(), Order::Asc);
        let mut query_stream = ResolvedQuery::new(self.tx, TableNamespace::Global, query)?;
        let mut backends = Vec::new();
        while let Some(doc) = query_stream.next(self.tx, None).await? {
            backends.push(doc.try_into()?);
        }
        Ok(backends)
    }
}

#[cfg(test)]
mod tests {
    use common::runtime::testing::TestRuntime;
    use database::test_helpers::DbFixtures;
    use keybroker::Identity;

    use crate::{
        backend_serving_record::{
            types::BackendServingRecord,
            ServingBackendModel,
        },
        test_helpers::DbFixturesWithModel,
    };

    #[convex_macro::test_runtime]
    async fn test_serving_backend_record(rt: TestRuntime) -> anyhow::Result<()> {
        let DbFixtures { db, .. } = DbFixtures::new_with_model(&rt).await?;

        let mut tx = db.begin(Identity::system()).await?;
        let mut model = ServingBackendModel::new(&mut tx);

        assert_eq!(model.list().await?, vec![]);

        let record1 = model
            .add(BackendServingRecord {
                preempt_url: "http://localhost:127.0.0.1/preempt".to_owned(),
            })
            .await?;
        assert_eq!(model.list().await?, vec![record1.clone()]);

        let record2 = model
            .add(BackendServingRecord {
                preempt_url: "http://localhost:127.0.0.1/preempt".to_owned(),
            })
            .await?;
        assert_eq!(model.list().await?, vec![record1.clone(), record2.clone()]);

        model.remove(record1.id()).await?;
        assert_eq!(model.list().await?, vec![record2.clone()]);

        Ok(())
    }
}

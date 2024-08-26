use std::sync::LazyLock;

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
    types::IndexName,
};
use database::{
    defaults::system_index,
    ResolvedQuery,
    SystemMetadataModel,
    Transaction,
};
use types::ExportFormat;
use value::{
    FieldPath,
    TableName,
    TableNamespace,
};

use self::types::Export;
use crate::{
    SystemIndex,
    SystemTable,
};

pub mod types;

pub static EXPORTS_TABLE: LazyLock<TableName> =
    LazyLock::new(|| "_exports".parse().expect("Invalid built-in exports table"));

pub static EXPORTS_BY_STATE_AND_TS_INDEX: LazyLock<IndexName> =
    LazyLock::new(|| system_index(&EXPORTS_TABLE, "by_state_and_ts"));

pub static EXPORTS_STATE_FIELD: LazyLock<FieldPath> =
    LazyLock::new(|| "state".parse().expect("Invalid built-in field"));

pub static EXPORTS_TS_FIELD: LazyLock<FieldPath> =
    LazyLock::new(|| "start_ts".parse().expect("Invalid built-in field"));

pub struct ExportsTable;
impl SystemTable for ExportsTable {
    fn table_name(&self) -> &'static TableName {
        &EXPORTS_TABLE
    }

    fn indexes(&self) -> Vec<SystemIndex> {
        vec![SystemIndex {
            name: EXPORTS_BY_STATE_AND_TS_INDEX.clone(),
            fields: vec![EXPORTS_STATE_FIELD.clone(), EXPORTS_TS_FIELD.clone()]
                .try_into()
                .unwrap(),
        }]
    }

    fn validate_document(&self, document: ResolvedDocument) -> anyhow::Result<()> {
        ParsedDocument::<Export>::try_from(document).map(|_| ())
    }
}

pub struct ExportsModel<'a, RT: Runtime> {
    tx: &'a mut Transaction<RT>,
}

impl<'a, RT: Runtime> ExportsModel<'a, RT> {
    pub fn new(tx: &'a mut Transaction<RT>) -> Self {
        Self { tx }
    }

    pub async fn insert_requested(&mut self, format: ExportFormat) -> anyhow::Result<()> {
        SystemMetadataModel::new_global(self.tx)
            .insert(&EXPORTS_TABLE, Export::requested(format).try_into()?)
            .await?;
        Ok(())
    }

    pub async fn list(&mut self) -> anyhow::Result<Vec<ParsedDocument<Export>>> {
        let value_query = Query::full_table_scan(EXPORTS_TABLE.clone(), Order::Asc);
        let mut query_stream = ResolvedQuery::new(self.tx, TableNamespace::Global, value_query)?;
        let mut result = vec![];
        while let Some(doc) = query_stream.next(self.tx, None).await? {
            let row: ParsedDocument<Export> = doc.try_into()?;
            result.push(row);
        }
        Ok(result)
    }
}

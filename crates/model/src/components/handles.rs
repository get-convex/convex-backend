use std::sync::LazyLock;

use common::{
    bootstrap_model::components::handles::{
        FunctionHandle,
        FunctionHandleMetadata,
    },
    components::{
        CanonicalizedComponentFunctionPath,
        ComponentId,
    },
    document::{
        ParsedDocument,
        ResolvedDocument,
    },
    query::{
        IndexRange,
        IndexRangeExpression,
        Order,
        Query,
    },
    runtime::Runtime,
    types::IndexName,
};
use database::{
    defaults::{
        system_index,
        SystemIndex,
        SystemTable,
    },
    BootstrapComponentsModel,
    ResolvedQuery,
    Transaction,
};
use errors::ErrorMetadata;
use sync_types::CanonicalizedUdfPath;
use value::{
    ConvexValue,
    DeveloperDocumentId,
    FieldPath,
    TableName,
    TableNamespace,
};

pub static FUNCTION_HANDLES_TABLE: LazyLock<TableName> = LazyLock::new(|| {
    "_function_handles"
        .parse()
        .expect("_function_handles is not a valid built-in table name")
});

pub static BY_COMPONENT_PATH_INDEX: LazyLock<IndexName> =
    LazyLock::new(|| system_index(&FUNCTION_HANDLES_TABLE, "by_component_path"));

pub static COMPONENT_FIELD: LazyLock<FieldPath> =
    LazyLock::new(|| "component".parse().expect("invalid component field"));

pub static PATH_FIELD: LazyLock<FieldPath> =
    LazyLock::new(|| "path".parse().expect("invalid path field"));

pub struct FunctionHandlesTable;

impl SystemTable for FunctionHandlesTable {
    fn table_name(&self) -> &'static TableName {
        &FUNCTION_HANDLES_TABLE
    }

    fn indexes(&self) -> Vec<SystemIndex> {
        vec![SystemIndex {
            name: BY_COMPONENT_PATH_INDEX.clone(),
            fields: vec![COMPONENT_FIELD.clone(), PATH_FIELD.clone()]
                .try_into()
                .unwrap(),
        }]
    }

    fn validate_document(&self, document: ResolvedDocument) -> anyhow::Result<()> {
        ParsedDocument::<FunctionHandleMetadata>::try_from(document)?;
        Ok(())
    }
}

pub struct FunctionHandlesModel<'a, RT: Runtime> {
    tx: &'a mut Transaction<RT>,
}

impl<'a, RT: Runtime> FunctionHandlesModel<'a, RT> {
    pub fn new(tx: &'a mut Transaction<RT>) -> Self {
        Self { tx }
    }

    pub async fn lookup(
        &mut self,
        handle: FunctionHandle,
    ) -> anyhow::Result<CanonicalizedComponentFunctionPath> {
        let id = DeveloperDocumentId::from(handle);
        let resolved_id = id.to_resolved(
            self.tx
                .table_mapping()
                .namespace(TableNamespace::Global)
                .number_to_tablet(),
        )?;
        let not_found =
            || ErrorMetadata::bad_request("FunctionHandleNotFound", "Function handle not found");
        let Some(document) = self.tx.get(resolved_id).await? else {
            anyhow::bail!(not_found());
        };
        let metadata = ParsedDocument::<FunctionHandleMetadata>::try_from(document)?.into_value();
        if metadata.deleted_ts.is_some() {
            anyhow::bail!(not_found());
        }
        let component_path = BootstrapComponentsModel::new(self.tx)
            .get_component_path(metadata.component)
            .await?;
        Ok(CanonicalizedComponentFunctionPath {
            component: component_path,
            udf_path: metadata.path,
        })
    }

    pub async fn get(
        &mut self,
        component: ComponentId,
        path: CanonicalizedUdfPath,
    ) -> anyhow::Result<FunctionHandle> {
        let serialized_component = match component.serialize_to_string() {
            Some(s) => ConvexValue::String(s.try_into()?),
            None => ConvexValue::Null,
        };
        let index_range = IndexRange {
            index_name: BY_COMPONENT_PATH_INDEX.clone(),
            range: vec![
                IndexRangeExpression::Eq(COMPONENT_FIELD.clone(), serialized_component.into()),
                IndexRangeExpression::Eq(
                    PATH_FIELD.clone(),
                    ConvexValue::String(String::from(path.clone()).try_into()?).into(),
                ),
            ],
            order: Order::Asc,
        };
        let query = Query::index_range(index_range);
        let mut query_stream = ResolvedQuery::new(self.tx, TableNamespace::Global, query)?;
        let not_found = || {
            ErrorMetadata::not_found(
                "FunctionHandleNotFound",
                format!(
                    "Function handle not found for component {:?} and path {:?}",
                    component, path
                ),
            )
        };
        let Some(document) = query_stream.expect_at_most_one(self.tx).await? else {
            anyhow::bail!(not_found())
        };
        let document: ParsedDocument<FunctionHandleMetadata> = document.try_into()?;
        if document.deleted_ts.is_some() {
            anyhow::bail!(not_found())
        }
        Ok(FunctionHandle::new(document.developer_id()))
    }
}

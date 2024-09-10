use std::sync::LazyLock;

use common::{
    components::ComponentId,
    document::{
        ParsedDocument,
        ResolvedDocument,
    },
    maybe_val,
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
    defaults::system_index,
    ResolvedQuery,
    SystemMetadataModel,
    Transaction,
};
use sync_types::Timestamp;
use types::{
    ExportFormat,
    ExportRequestor,
};
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

    pub async fn insert_requested(
        &mut self,
        format: ExportFormat,
        component: ComponentId,
        requestor: ExportRequestor,
    ) -> anyhow::Result<()> {
        SystemMetadataModel::new_global(self.tx)
            .insert(
                &EXPORTS_TABLE,
                Export::requested(format, component, requestor).try_into()?,
            )
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

    pub async fn latest_requested(&mut self) -> anyhow::Result<Option<ParsedDocument<Export>>> {
        self.export_in_state("requested").await
    }

    pub async fn latest_in_progress(&mut self) -> anyhow::Result<Option<ParsedDocument<Export>>> {
        self.export_in_state("in_progress").await
    }

    async fn export_in_state(
        &mut self,
        export_state: &str,
    ) -> anyhow::Result<Option<ParsedDocument<Export>>> {
        let index_range = IndexRange {
            index_name: EXPORTS_BY_STATE_AND_TS_INDEX.clone(),
            range: vec![IndexRangeExpression::Eq(
                EXPORTS_STATE_FIELD.clone(),
                maybe_val!(export_state),
            )],
            order: Order::Asc,
        };
        let query = common::query::Query::index_range(index_range);
        let mut query_stream = ResolvedQuery::new(self.tx, TableNamespace::Global, query)?;
        query_stream
            .expect_at_most_one(self.tx)
            .await?
            .map(|doc| doc.try_into())
            .transpose()
    }

    pub async fn completed_export_at_ts(
        &mut self,
        snapshot_ts: Timestamp,
    ) -> anyhow::Result<Option<ResolvedDocument>> {
        let index_range = IndexRange {
            index_name: EXPORTS_BY_STATE_AND_TS_INDEX.clone(),
            range: vec![
                IndexRangeExpression::Eq(EXPORTS_STATE_FIELD.clone(), maybe_val!("completed")),
                IndexRangeExpression::Eq(
                    EXPORTS_TS_FIELD.clone(),
                    maybe_val!(i64::from(snapshot_ts)),
                ),
            ],
            order: Order::Desc,
        };
        let query = common::query::Query::index_range(index_range);
        let mut query_stream = ResolvedQuery::new(self.tx, TableNamespace::Global, query)?;
        query_stream.expect_at_most_one(self.tx).await
    }
}

#[cfg(test)]
mod tests {
    use cmd_util::env::env_config;
    use common::{
        components::ComponentId,
        types::ObjectKey,
    };
    use database::test_helpers::DbFixtures;
    use proptest::prelude::*;
    use runtime::testing::{
        TestDriver,
        TestRuntime,
    };
    use sync_types::Timestamp;
    use value::ConvexObject;

    use crate::{
        exports::{
            types::{
                Export,
                ExportFormat,
                ExportRequestor,
            },
            ExportsModel,
        },
        test_helpers::DbFixturesWithModel,
    };

    #[test]
    fn test_export_deserialization() -> anyhow::Result<()> {
        // Requested
        let requested_export = Export::requested(
            ExportFormat::Zip {
                include_storage: false,
            },
            ComponentId::test_user(),
            ExportRequestor::SnapshotExport,
        );
        let object: ConvexObject = requested_export.clone().try_into()?;
        let deserialized_export = object.try_into()?;
        assert_eq!(requested_export, deserialized_export);

        let ts = Timestamp::must(1234);
        // InProgress
        let in_progress_export = requested_export.clone().in_progress(ts)?;
        let object: ConvexObject = in_progress_export.clone().try_into()?;
        let deserialized_export = object.try_into()?;
        assert_eq!(in_progress_export, deserialized_export);

        // Completed
        let export = in_progress_export
            .clone()
            .completed(ts, ts, ObjectKey::try_from("asdf")?)?;
        let object: ConvexObject = export.clone().try_into()?;
        let deserialized_export = object.try_into()?;
        assert_eq!(export, deserialized_export);

        // Failed
        let export = in_progress_export.failed(ts, ts)?;
        let object: ConvexObject = export.clone().try_into()?;
        let deserialized_export = object.try_into()?;
        assert_eq!(export, deserialized_export);

        Ok(())
    }

    proptest! {
        #![proptest_config(ProptestConfig { cases: 32 * env_config("CONVEX_PROPTEST_MULTIPLIER", 1), failure_persistence: None, .. ProptestConfig::default() })]

        #[test]
        fn proptest_export_model(
            format in any::<ExportFormat>(),
            component in any::<ComponentId>(),
            requestor in any::<ExportRequestor>(),
        ) {
            let td = TestDriver::new();
            let rt = td.rt();
            td.run_until(test_export_model(rt, format, component, requestor)).unwrap();
        }
    }

    async fn test_export_model(
        rt: TestRuntime,
        format: ExportFormat,
        component: ComponentId,
        requestor: ExportRequestor,
    ) -> anyhow::Result<()> {
        let DbFixtures { db, .. } = DbFixtures::new_with_model(&rt).await?;
        let mut tx = db.begin_system().await?;
        let mut exports_model = ExportsModel::new(&mut tx);
        exports_model
            .insert_requested(format, component, requestor)
            .await?;
        let items: Vec<_> = exports_model
            .list()
            .await?
            .into_iter()
            .map(|v| v.into_value())
            .collect();
        let expected = Export::Requested {
            format,
            component,
            requestor,
        };
        assert_eq!(items, vec![expected.clone()]);
        assert_eq!(
            exports_model
                .latest_requested()
                .await?
                .unwrap()
                .into_value(),
            expected
        );
        assert_eq!(exports_model.latest_in_progress().await?, None);
        Ok(())
    }
}

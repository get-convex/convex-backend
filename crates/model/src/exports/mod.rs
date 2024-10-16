use std::{
    sync::LazyLock,
    time::Duration,
};

use common::{
    components::ComponentId,
    document::{
        ParsedDocument,
        ResolvedDocument,
        CREATION_TIME_FIELD_PATH,
    },
    maybe_val,
    query::{
        Expression,
        IndexRange,
        IndexRangeExpression,
        Order,
        Query,
    },
    runtime::Runtime,
    types::{
        IndexName,
        ObjectKey,
    },
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
    ConvexValue,
    DeveloperDocumentId,
    FieldPath,
    ResolvedDocumentId,
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

pub static EXPORTS_BY_REQUESTOR: LazyLock<IndexName> =
    LazyLock::new(|| system_index(&EXPORTS_TABLE, "by_requestor"));

pub static EXPORTS_STATE_FIELD: LazyLock<FieldPath> =
    LazyLock::new(|| "state".parse().expect("Invalid built-in field"));

pub static EXPORTS_TS_FIELD: LazyLock<FieldPath> =
    LazyLock::new(|| "start_ts".parse().expect("Invalid built-in field"));

pub static EXPORTS_EXPIRATION_TS_FIELD: LazyLock<FieldPath> =
    LazyLock::new(|| "expiration_ts".parse().expect("Invalid built-in field"));

static EXPORTS_REQUESTOR_FIELD: LazyLock<FieldPath> =
    LazyLock::new(|| "requestor".parse().expect("Invalid built-in field"));

const DEFAULT_EXPORT_RETENTION: u64 = Duration::from_days(14).as_nanos() as u64;

pub struct ExportsTable;
impl SystemTable for ExportsTable {
    fn table_name(&self) -> &'static TableName {
        &EXPORTS_TABLE
    }

    fn indexes(&self) -> Vec<SystemIndex> {
        vec![
            SystemIndex {
                name: EXPORTS_BY_STATE_AND_TS_INDEX.clone(),
                fields: vec![EXPORTS_STATE_FIELD.clone(), EXPORTS_TS_FIELD.clone()]
                    .try_into()
                    .unwrap(),
            },
            SystemIndex {
                name: EXPORTS_BY_REQUESTOR.clone(),
                fields: vec![
                    EXPORTS_REQUESTOR_FIELD.clone(),
                    CREATION_TIME_FIELD_PATH.clone(),
                ]
                .try_into()
                .unwrap(),
            },
        ]
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
        expiration_ts_ns: Option<u64>,
    ) -> anyhow::Result<ResolvedDocumentId> {
        let default_expiration_ts =
            u64::from(*self.tx.begin_timestamp()) + DEFAULT_EXPORT_RETENTION;
        let expiration_ts_ns = expiration_ts_ns.unwrap_or(default_expiration_ts);

        SystemMetadataModel::new_global(self.tx)
            .insert(
                &EXPORTS_TABLE,
                Export::requested(format, component, requestor, expiration_ts_ns).try_into()?,
            )
            .await
    }

    #[cfg(test)]
    pub async fn insert_export(&mut self, export: Export) -> anyhow::Result<ResolvedDocumentId> {
        SystemMetadataModel::new_global(self.tx)
            .insert(&EXPORTS_TABLE, export.try_into()?)
            .await
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

    pub async fn list_unexpired_cloud_backups(
        &mut self,
    ) -> anyhow::Result<Vec<ParsedDocument<Export>>> {
        let index_range = IndexRange {
            index_name: EXPORTS_BY_REQUESTOR.clone(),
            range: vec![IndexRangeExpression::Eq(
                EXPORTS_REQUESTOR_FIELD.clone(),
                ConvexValue::try_from(ExportRequestor::CloudBackup.to_string())?.into(),
            )],
            order: Order::Asc,
        };
        let completed_filter = Expression::Eq(
            Expression::Field(EXPORTS_STATE_FIELD.clone()).into(),
            Expression::Literal(maybe_val!("completed")).into(),
        );
        let expired_filter = Expression::Gt(
            Expression::Field(EXPORTS_EXPIRATION_TS_FIELD.clone()).into(),
            Expression::Literal(maybe_val!(i64::from(*self.tx.begin_timestamp()))).into(),
        );
        let query = Query::index_range(index_range)
            .filter(Expression::And(vec![completed_filter, expired_filter]));
        let mut query_stream = ResolvedQuery::new(self.tx, TableNamespace::Global, query)?;
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
        let query = Query::index_range(index_range);
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
        let query = Query::index_range(index_range);
        let mut query_stream = ResolvedQuery::new(self.tx, TableNamespace::Global, query)?;
        query_stream.expect_at_most_one(self.tx).await
    }

    pub async fn get(
        &mut self,
        snapshot_id: DeveloperDocumentId,
    ) -> anyhow::Result<Option<ParsedDocument<Export>>> {
        let query = Query::get(EXPORTS_TABLE.clone(), snapshot_id);
        let mut query_stream = ResolvedQuery::new(self.tx, TableNamespace::Global, query)?;
        query_stream
            .expect_at_most_one(self.tx)
            .await?
            .map(|doc| doc.try_into())
            .transpose()
    }

    pub async fn cleanup_expired(
        &mut self,
        retention_duration: Duration,
    ) -> anyhow::Result<Vec<ObjectKey>> {
        let delete_before_ts = (*self.tx.begin_timestamp()).sub(retention_duration)?;
        let mut to_delete = vec![];
        for export in self.list().await? {
            let (id, export) = export.into_id_and_value();
            match export {
                Export::Requested { .. } | Export::InProgress { .. } => continue,
                Export::Completed {
                    expiration_ts,
                    zip_object_key,
                    ..
                } => {
                    if expiration_ts < delete_before_ts.into() {
                        to_delete.push(zip_object_key);
                        SystemMetadataModel::new_global(self.tx).delete(id).await?;
                    }
                },
                Export::Failed { failed_ts, .. } => {
                    if failed_ts < delete_before_ts {
                        SystemMetadataModel::new_global(self.tx).delete(id).await?;
                    }
                },
            }
        }
        Ok(to_delete)
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

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
            4321,
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
            expiration_ts in any::<u64>(),
        ) {
            let td = TestDriver::new();
            let rt = td.rt();
            td.run_until(test_export_model(
                rt,
                format,
                component,
                requestor,
                expiration_ts,
            )).unwrap();
        }
    }

    async fn test_export_model(
        rt: TestRuntime,
        format: ExportFormat,
        component: ComponentId,
        requestor: ExportRequestor,
        expiration_ts: u64,
    ) -> anyhow::Result<()> {
        let DbFixtures { db, .. } = DbFixtures::new_with_model(&rt).await?;
        let mut tx = db.begin_system().await?;
        let mut exports_model = ExportsModel::new(&mut tx);
        let snapshot_id = exports_model
            .insert_requested(format, component, requestor, Some(expiration_ts))
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
            expiration_ts,
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
        assert_eq!(
            exports_model
                .get(snapshot_id.developer_id)
                .await?
                .unwrap()
                .into_value(),
            expected
        );
        Ok(())
    }

    #[convex_macro::test_runtime]
    async fn test_list_unexpired_cloud_snapshots(rt: TestRuntime) -> anyhow::Result<()> {
        let DbFixtures { db, .. } = DbFixtures::new_with_model(&rt).await?;
        let mut tx = db.begin_system().await?;
        let ts = *tx.begin_timestamp();
        let ts_u64: u64 = ts.into();
        let mut exports_model = ExportsModel::new(&mut tx);

        // Insert an incomplete cloud backup
        exports_model
            .insert_export(Export::requested(
                ExportFormat::Zip {
                    include_storage: false,
                },
                ComponentId::test_user(),
                ExportRequestor::CloudBackup,
                ts_u64 + 1000,
            ))
            .await?;
        let backups = exports_model.list_unexpired_cloud_backups().await?;
        assert!(backups.is_empty());

        // Insert a completed snapshot export
        let export = Export::requested(
            ExportFormat::Zip {
                include_storage: false,
            },
            ComponentId::test_user(),
            ExportRequestor::SnapshotExport,
            ts_u64 + 1000,
        )
        .in_progress(ts)?
        .completed(ts, ts, ObjectKey::try_from("asdf")?)?;
        exports_model.insert_export(export).await?;
        let backups = exports_model.list_unexpired_cloud_backups().await?;
        assert!(backups.is_empty());

        // Insert a completed but expired cloud backup
        let export = Export::requested(
            ExportFormat::Zip {
                include_storage: false,
            },
            ComponentId::test_user(),
            ExportRequestor::CloudBackup,
            ts_u64 - 1000,
        )
        .in_progress(ts)?
        .completed(ts, ts, ObjectKey::try_from("asdf")?)?;
        exports_model.insert_export(export).await?;
        let backups = exports_model.list_unexpired_cloud_backups().await?;
        assert!(backups.is_empty());

        // Insert a completed cloud backup
        let export = Export::requested(
            ExportFormat::Zip {
                include_storage: false,
            },
            ComponentId::test_user(),
            ExportRequestor::CloudBackup,
            ts_u64 + 1000,
        )
        .in_progress(ts)?
        .completed(ts, ts, ObjectKey::try_from("asdf")?)?;
        exports_model.insert_export(export).await?;
        let backups = exports_model.list_unexpired_cloud_backups().await?;
        assert_eq!(backups.len(), 1);

        Ok(())
    }

    #[convex_macro::test_runtime]
    async fn test_cleanup_expired(rt: TestRuntime) -> anyhow::Result<()> {
        let DbFixtures { db, .. } = DbFixtures::new_with_model(&rt).await?;
        let mut tx = db.begin_system().await?;
        let ts = *tx.begin_timestamp();
        let ts_u64: u64 = ts.into();
        let mut exports_model = ExportsModel::new(&mut tx);

        // Insert an complete cloud backup
        let export = Export::requested(
            ExportFormat::Zip {
                include_storage: false,
            },
            ComponentId::test_user(),
            ExportRequestor::CloudBackup,
            ts_u64,
        )
        .in_progress(ts)?
        .completed(ts, ts, ObjectKey::try_from("asdf")?)?;
        exports_model.insert_export(export).await?;
        assert_eq!(exports_model.list().await?.len(), 1);
        let toremove = exports_model
            .cleanup_expired(Duration::from_days(30))
            .await?;
        assert_eq!(toremove.len(), 0);
        assert_eq!(exports_model.list().await?.len(), 1);
        rt.advance_time(Duration::from_days(31)).await;
        db.commit(tx).await?;

        let mut tx = db.begin_system().await?;
        let mut exports_model = ExportsModel::new(&mut tx);

        // Cleanup 60 days do nothing
        let toremove = exports_model
            .cleanup_expired(Duration::from_days(60))
            .await?;
        assert_eq!(toremove, vec![]);
        assert_eq!(exports_model.list().await?.len(), 1);

        // Cleanup 30 days will clean it up
        let toremove = exports_model
            .cleanup_expired(Duration::from_days(30))
            .await?;
        assert_eq!(toremove, vec![ObjectKey::try_from("asdf")?]);
        assert_eq!(exports_model.list().await?.len(), 0);
        Ok(())
    }
}

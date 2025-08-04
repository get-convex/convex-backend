use std::{
    collections::BTreeMap,
    sync::Arc,
    time::Duration,
};

use ::metrics::StatusTimer;
use common::{
    backoff::Backoff,
    bootstrap_model::schema::SchemaState,
    errors::report_error,
    persistence::LatestDocument,
    runtime::Runtime,
    schemas::DatabaseSchema,
    types::{
        IndexId,
        RepeatableTimestamp,
    },
    virtual_system_mapping::VirtualSystemMapping,
};
use database::{
    Database,
    IndexModel,
    SchemaModel,
    Transaction,
    SCHEMAS_TABLE,
};
use errors::ErrorMetadataAnyhowExt;
use futures::{
    pin_mut,
    Future,
    TryStreamExt,
};
use keybroker::Identity;
use metrics::{
    log_document_bytes,
    log_document_validated,
    schema_validation_timer,
};
use value::{
    NamespacedTableMapping,
    ResolvedDocumentId,
    TableNamespace,
    TabletId,
};

use crate::metrics::log_worker_starting;

mod metrics;

const INITIAL_BACKOFF: Duration = Duration::from_millis(10);
const MAX_BACKOFF: Duration = Duration::from_secs(5);
const INITIAL_COMMIT_BACKOFF: Duration = Duration::from_millis(10);
const MAX_COMMIT_BACKOFF: Duration = Duration::from_secs(2);
const MAX_COMMIT_FAILURES: u32 = 3;

pub struct SchemaWorker<RT: Runtime> {
    runtime: RT,
    database: Database<RT>,
}

pub struct PendingSchemaWork {
    namespace: TableNamespace,
    id: ResolvedDocumentId,
    timer: StatusTimer,
    table_mapping: NamespacedTableMapping,
    virtual_system_mapping: VirtualSystemMapping,
    db_schema: Arc<DatabaseSchema>,
    ts: RepeatableTimestamp,
    active_schema: Option<Arc<DatabaseSchema>>,
    by_id_indexes: BTreeMap<TabletId, IndexId>,
}

impl<RT: Runtime> SchemaWorker<RT> {
    pub fn start(runtime: RT, database: Database<RT>) -> impl Future<Output = ()> + Send {
        let worker = Self { runtime, database };
        async move {
            tracing::info!("Starting SchemaWorker");
            let mut backoff = Backoff::new(INITIAL_BACKOFF, MAX_BACKOFF);
            loop {
                if let Err(e) = worker.run().await {
                    let delay = backoff.fail(&mut worker.runtime.rng());
                    report_error(&mut e.context("SchemaWorker died")).await;
                    tracing::error!("Schema worker failed, sleeping {delay:?}");
                    worker.runtime.wait(delay).await;
                } else {
                    backoff.reset();
                }
            }
        }
    }

    pub(crate) async fn pending_schema_work(
        tx: &mut Transaction<RT>,
    ) -> anyhow::Result<Vec<PendingSchemaWork>> {
        let mut pending_schema_work = Vec::new();
        let namespaces: Vec<_> = tx.table_mapping().namespaces_for_name(&SCHEMAS_TABLE);
        for namespace in namespaces {
            if let Some((id, db_schema)) = SchemaModel::new(tx, namespace)
                .get_by_state(SchemaState::Pending)
                .await?
            {
                tracing::debug!("SchemaWorker found a pending schema and is validating it...");
                let timer = schema_validation_timer();
                let table_mapping = tx.table_mapping().namespace(namespace);
                let virtual_system_mapping = tx.virtual_system_mapping().clone();

                let active_schema = SchemaModel::new(tx, namespace)
                    .get_by_state(SchemaState::Active)
                    .await?
                    .map(|(_id, active_schema)| active_schema);
                let ts = tx.begin_timestamp();
                let by_id_indexes = IndexModel::new(tx).by_id_indexes().await?;
                pending_schema_work.push(PendingSchemaWork {
                    namespace,
                    id,
                    timer,
                    table_mapping,
                    virtual_system_mapping,
                    db_schema,
                    ts,
                    active_schema,
                    by_id_indexes,
                });
            }
        }
        Ok(pending_schema_work)
    }

    pub async fn run(&self) -> anyhow::Result<()> {
        let status = log_worker_starting("SchemaWorker");
        let mut tx: Transaction<RT> = self.database.begin(Identity::system()).await?;
        let snapshot = self.database.snapshot(tx.begin_timestamp())?;
        let pending_schema_work = SchemaWorker::pending_schema_work(&mut tx).await?;
        let token = tx.into_token()?;

        for PendingSchemaWork {
            namespace,
            id,
            timer,
            table_mapping,
            virtual_system_mapping,
            db_schema,
            ts,
            active_schema,
            by_id_indexes,
        } in pending_schema_work
        {
            let tables_to_check = DatabaseSchema::tables_to_validate(
                &db_schema,
                active_schema.as_deref(),
                &table_mapping,
                &virtual_system_mapping,
                &|table_name| {
                    snapshot
                        .table_summary(namespace, table_name)
                        .map(|t| t.inferred_type().clone())
                },
            )?;
            tracing::info!("SchemaWorker: Tables to check: {:?}", tables_to_check);

            let tablet_ids = tables_to_check
                .into_iter()
                .map(|table_name| table_mapping.name_to_tablet()(table_name.clone()))
                .collect::<Result<Vec<_>, _>>()?;
            let mut table_iterator = self
                .database
                .table_iterator(ts, 1000)
                .multi(tablet_ids.clone());
            for tablet_id in tablet_ids {
                let stream = table_iterator.stream_documents_in_table(
                    tablet_id,
                    *by_id_indexes.get(&tablet_id).ok_or_else(|| {
                        anyhow::anyhow!("Failed to find id index for table id {tablet_id}")
                    })?,
                    None,
                );

                {
                    pin_mut!(stream);
                    while let Some(LatestDocument { value: doc, .. }) = stream.try_next().await? {
                        let table_name = table_mapping.tablet_name(doc.id().tablet_id)?;
                        log_document_validated();
                        log_document_bytes(doc.size());
                        if let Err(schema_error) = db_schema.check_existing_document(
                            &doc,
                            table_name,
                            &table_mapping,
                            &virtual_system_mapping,
                        ) {
                            let mut backoff =
                                Backoff::new(INITIAL_COMMIT_BACKOFF, MAX_COMMIT_BACKOFF);
                            while backoff.failures() < MAX_COMMIT_FAILURES {
                                let mut tx = self.database.begin(Identity::system()).await?;
                                SchemaModel::new(&mut tx, namespace)
                                    .mark_failed(id, schema_error.clone())
                                    .await?;
                                if let Err(e) = self
                                    .database
                                    .commit_with_write_source(tx, "schema_worker_mark_failed")
                                    .await
                                {
                                    if e.is_occ() {
                                        let delay = backoff.fail(&mut self.runtime.rng());
                                        tracing::error!(
                                            "Schema worker failed to commit ({e}), retrying after \
                                             {delay:?}"
                                        );
                                        self.runtime.wait(delay).await;
                                    } else {
                                        return Err(e);
                                    }
                                } else {
                                    break;
                                }
                            }

                            tracing::info!("Schema is invalid");
                            timer.finish_developer_error();
                            return Ok(());
                        }
                    }
                }
                table_iterator.unregister_table(tablet_id)?;
            }
            let mut tx = self.database.begin(Identity::system()).await?;
            if let Err(error) = SchemaModel::new(&mut tx, namespace)
                .mark_validated(id)
                .await
            {
                if error.is_bad_request() {
                    timer.finish_developer_error();
                }
                tracing::info!("Schema not marked valid");
                return Err(error);
            }
            self.database
                .commit_with_write_source(tx, "schema_worker_mark_valid")
                .await?;
            tracing::info!("Schema is valid");
            timer.finish();
        }

        drop(status);
        tracing::debug!("SchemaWorker waiting...");
        let subscription = self.database.subscribe(token).await?;
        subscription.wait_for_invalidation().await;
        Ok(())
    }
}

#[cfg(test)]
mod tests {

    use common::{
        assert_obj,
        bootstrap_model::schema::{
            SchemaMetadata,
            SchemaState,
        },
        db_schema,
        object_validator,
        schemas::{
            validator::{
                FieldValidator,
                Validator,
            },
            DatabaseSchema,
            DocumentSchema,
            TableDefinition,
        },
    };
    use database::{
        test_helpers::new_test_database,
        SchemaModel,
        UserFacingModel,
    };
    use keybroker::Identity;
    use maplit::btreemap;
    use runtime::testing::TestRuntime;
    use value::TableName;

    use super::SchemaWorker;

    #[convex_macro::test_runtime]
    async fn test_schema_validation(rt: TestRuntime) -> anyhow::Result<()> {
        let db = new_test_database(rt.clone()).await;
        let schema_worker = SchemaWorker {
            runtime: rt.clone(),
            database: db.clone(),
        };
        let mut tx = db.begin(Identity::system()).await?;
        let table_name = "table".parse::<TableName>()?;
        let table_definition = TableDefinition {
            table_name: table_name.clone(),
            indexes: btreemap! {},
            staged_db_indexes: btreemap! {},
            text_indexes: btreemap! {},
            staged_text_indexes: btreemap! {},
            vector_indexes: btreemap! {},
            staged_vector_indexes: btreemap! {},
            document_type: Some(DocumentSchema::Any),
        };
        let db_schema = DatabaseSchema {
            tables: btreemap! { table_name.clone() => table_definition },
            schema_validation: true,
        };
        let (id, _) = SchemaModel::new_root_for_test(&mut tx)
            .submit_pending(db_schema)
            .await?;
        // Insert a document that matches the schema
        UserFacingModel::new_root_for_test(&mut tx)
            .insert(table_name.clone(), assert_obj!())
            .await?;
        db.commit(tx).await?;

        // Check that the schema passes and is active
        schema_worker.run().await?;
        let mut tx = db.begin(Identity::system()).await?;
        let doc = tx.get(id).await?.unwrap();
        let schema: SchemaMetadata = doc.into_value().into_value().try_into()?;
        assert_eq!(schema.state, SchemaState::Validated);

        // Insert a new schema that doesn't match the documents. It should fail!
        let db_schema = db_schema!(table_name =>
            DocumentSchema::Union(vec![object_validator!("field" => FieldValidator::required_field_type(Validator::Int64))]),
        );

        let (bad_schema_id, state) = SchemaModel::new_root_for_test(&mut tx)
            .submit_pending(db_schema)
            .await?;
        assert_eq!(state, SchemaState::Pending);
        db.commit(tx).await?;
        schema_worker.run().await?;

        let mut tx = db.begin(Identity::system()).await?;
        let doc = tx.get(id).await?.unwrap();
        let schema: SchemaMetadata = doc.into_value().into_value().try_into()?;
        assert_eq!(schema.state, SchemaState::Overwritten);
        let doc = tx.get(bad_schema_id).await?.unwrap();
        let schema: SchemaMetadata = doc.into_value().into_value().try_into()?;
        assert!(matches!(schema.state, SchemaState::Failed { .. }));
        Ok(())
    }
}

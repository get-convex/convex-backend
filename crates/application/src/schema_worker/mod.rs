use std::{
    collections::{
        BTreeMap,
        BTreeSet,
    },
    sync::Arc,
    time::Duration,
};

use ::metrics::StatusTimer;
use anyhow::Context;
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
    SchemaValidationProgressModel,
    Snapshot,
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
    TableName,
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

pub struct PendingSchemaValidation {
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

    pub(crate) async fn pending_schema_validations(
        tx: &mut Transaction<RT>,
    ) -> anyhow::Result<Vec<PendingSchemaValidation>> {
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
                pending_schema_work.push(PendingSchemaValidation {
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
        let pending_validations = SchemaWorker::pending_schema_validations(&mut tx).await?;
        let token = tx.into_token()?;

        for pending_validation in pending_validations {
            // FIXME: Remove clone
            let db_schema = pending_validation.db_schema.clone();
            let tables_to_validate = DatabaseSchema::tables_to_validate(
                &db_schema,
                pending_validation.active_schema.as_deref(),
                &pending_validation.table_mapping,
                &pending_validation.virtual_system_mapping,
                &|table_name| {
                    snapshot
                        .table_summary(pending_validation.namespace, table_name)
                        .map(|t| t.inferred_type().clone())
                },
            )?;
            self.validate_tables(tables_to_validate, pending_validation)
                .await?;
        }

        drop(status);
        tracing::debug!("SchemaWorker waiting...");
        let subscription = self.database.subscribe(token).await?;
        subscription.wait_for_invalidation().await;
        Ok(())
    }

    async fn validate_tables(
        &self,
        tables_to_validate: BTreeSet<&TableName>,
        PendingSchemaValidation {
            namespace,
            id,
            timer,
            table_mapping,
            virtual_system_mapping,
            db_schema,
            ts,
            active_schema: _,
            by_id_indexes,
        }: PendingSchemaValidation,
    ) -> anyhow::Result<()> {
        tracing::info!("SchemaWorker: Tables to check: {:?}", tables_to_validate);

        let mut schema_validation_progress_tracker = SchemaValidationProgressTracker::new(
            self.database.clone(),
            namespace,
            tables_to_validate.clone().into_iter().cloned().collect(),
            id,
        )
        .await?;
        let tablet_ids = tables_to_validate
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
                let table_name = table_mapping.tablet_name(tablet_id)?;
                while let Some(LatestDocument { value: doc, .. }) = stream.try_next().await? {
                    log_document_validated();
                    log_document_bytes(doc.size());
                    // If we finish with an error, we should delete progress. In all the
                    // mark_failed, mark_success or whatever methods.
                    if let Err(schema_error) = db_schema.check_existing_document(
                        &doc,
                        table_name.clone(),
                        &table_mapping,
                        &virtual_system_mapping,
                    ) {
                        let mut backoff = Backoff::new(INITIAL_COMMIT_BACKOFF, MAX_COMMIT_BACKOFF);
                        while backoff.failures() < MAX_COMMIT_FAILURES {
                            let mut tx = self.database.begin_system().await?;
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
                    // Update schema validation progress periodically, when we hit the
                    // threshold.
                    let progress_exists = schema_validation_progress_tracker
                        .record_document_validated()
                        .await?;
                    // Return early if progress does not exist - this means the schema
                    // validation has been canceled either by a document update that does
                    // not match the pending schema or by the submission of a new pending
                    // schema.
                    if !progress_exists {
                        return Ok(());
                    }
                }
            }
            table_iterator.unregister_table(tablet_id)?;
        }
        schema_validation_progress_tracker
            .record_validation_finished()
            .await?;
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
        Ok(())
    }
}

/// Tracks progress of schema validation for the tables that need to be
/// validated, periodically writing progress to the
/// `_schema_validation_progress` table for the given namespace and schema.
struct SchemaValidationProgressTracker<RT: Runtime> {
    database: Database<RT>,
    namespace: TableNamespace,
    tables_to_validate: BTreeSet<TableName>,
    schema_id: ResolvedDocumentId,
    /// The threshold at which to write validation progress to the database.
    update_threshold: u64,
    /// The number of documents that have been validated since writing progress
    /// to the database.
    docs_validated: u64,
}

impl<RT: Runtime> SchemaValidationProgressTracker<RT> {
    pub async fn new(
        database: Database<RT>,
        namespace: TableNamespace,
        tables_to_validate: BTreeSet<TableName>,
        schema_id: ResolvedDocumentId,
    ) -> anyhow::Result<Self> {
        let mut tx = database.begin(Identity::system()).await?;
        let snapshot = database.snapshot(tx.begin_timestamp())?;
        let total_docs = Self::_total_docs(&snapshot, &tables_to_validate, namespace)?;
        let mut model = SchemaValidationProgressModel::new(&mut tx, namespace);
        model
            .initialize_schema_validation_progress(schema_id, total_docs)
            .await?;
        database
            .commit_with_write_source(tx, "schema_validation_tracker_initialized")
            .await?;
        // Update schema validation progress every 5% or 500 documents, whichever is
        // lower, to slowing down schema validation with too many writes.
        let update_threshold = total_docs
            .map(|total| std::cmp::min(500, (total as f64 * 0.05).ceil() as u64))
            .unwrap_or(500);
        Ok(Self {
            database,
            namespace,
            tables_to_validate,
            schema_id,
            update_threshold,
            docs_validated: 0,
        })
    }

    fn total_docs_at_ts(&self, ts: RepeatableTimestamp) -> anyhow::Result<Option<u64>> {
        let snapshot = self.database.snapshot(ts)?;
        Self::_total_docs(&snapshot, &self.tables_to_validate, self.namespace)
    }

    fn _total_docs(
        snapshot: &Snapshot,
        tables_to_validate: &BTreeSet<TableName>,
        namespace: TableNamespace,
    ) -> anyhow::Result<Option<u64>> {
        let total_docs = if snapshot.table_summaries.is_some() {
            let doc_counts = tables_to_validate
                .iter()
                .map(|table_name| {
                    anyhow::Ok(
                        snapshot
                            .table_summary(namespace, table_name)
                            .context(
                                "Failed to retrieve table summary when table summaries were \
                                 present",
                            )?
                            .num_values(),
                    )
                })
                .try_collect::<Vec<_>>()?;
            Some(doc_counts.iter().sum())
        } else {
            None
        };
        Ok(total_docs)
    }

    /// Records that a document has been validated, writing to the db iff if we
    /// have hit the update threshold, otherwise tracking progress in memory.
    async fn record_document_validated(&mut self) -> anyhow::Result<bool> {
        self.docs_validated += 1;
        if self.docs_validated % self.update_threshold != 0 {
            return Ok(true);
        }
        tracing::debug!(
            "Updating schema validation progress with docs_validated: {}, update threshold: {}",
            self.docs_validated,
            self.update_threshold
        );
        let mut tx = self.database.begin_system().await?;
        let total_docs = self.total_docs_at_ts(tx.begin_timestamp())?;
        let mut model = SchemaValidationProgressModel::new(&mut tx, self.namespace);
        let progress_exists = model
            .update_schema_validation_progress(self.schema_id, self.docs_validated, total_docs)
            .await?;
        self.database
            .commit_with_write_source(tx, "schema_validation_progress_updated")
            .await?;
        self.docs_validated = 0;
        Ok(progress_exists)
    }

    /// Flushes the remaining schema validation progress to the database after
    /// schema validation is finished.
    async fn record_validation_finished(self) -> anyhow::Result<()> {
        tracing::debug!(
            "Finalizing schema validation progress with docs_validated: {}",
            self.docs_validated
        );
        let mut tx = self.database.begin_system().await?;
        let total_docs = self.total_docs_at_ts(tx.begin_timestamp())?;
        let mut model = SchemaValidationProgressModel::new(&mut tx, self.namespace);
        model
            .update_schema_validation_progress(self.schema_id, self.docs_validated, total_docs)
            .await?;
        self.database
            .commit_with_write_source(tx, "schema_validation_progress_finished")
            .await?;
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
            DocumentSchema,
        },
    };
    use database::{
        test_helpers::new_test_database,
        SchemaModel,
        SchemaValidationProgressModel,
        UserFacingModel,
    };
    use keybroker::Identity;
    use maplit::btreeset;
    use runtime::testing::TestRuntime;
    use value::{
        TableName,
        TableNamespace,
    };

    use super::SchemaWorker;

    #[convex_macro::test_runtime]
    async fn test_schema_validation(rt: TestRuntime) -> anyhow::Result<()> {
        let db = new_test_database(rt.clone()).await;
        let schema_worker = SchemaWorker {
            runtime: rt.clone(),
            database: db.clone(),
        };
        let mut tx = db.begin_system().await?;
        let table_name = "table".parse::<TableName>()?;
        let db_schema = db_schema!(table_name => DocumentSchema::Any);
        let (id, _) = SchemaModel::new_root_for_test(&mut tx)
            .submit_pending(db_schema)
            .await?;
        // Insert a document that matches the schema
        UserFacingModel::new_root_for_test(&mut tx)
            .insert(table_name.clone(), assert_obj!())
            .await?;
        db.commit(tx).await?;

        // Check that the schema passes and is validated
        schema_worker.run().await?;
        let mut tx = db.begin(Identity::system()).await?;
        let doc = tx.get(id).await?.unwrap();
        let schema: SchemaMetadata = doc.into_value().into_value().try_into()?;
        assert_eq!(schema.state, SchemaState::Validated);
        // Check that schema validation progress is written
        let mut model = SchemaValidationProgressModel::new(&mut tx, TableNamespace::test_user());
        let progress = model
            .existing_schema_validation_progress(id)
            .await?
            .unwrap();
        assert_eq!(progress.num_docs_validated, 0);
        // Doesn't need to validate any documents because the schema matches all
        // documents
        assert_eq!(progress.total_docs, Some(0));

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
        // Progress should be deleted when schema is marked as failed.
        let mut model = SchemaValidationProgressModel::new(&mut tx, TableNamespace::test_user());
        let progress = model.existing_schema_validation_progress(id).await?;
        assert!(progress.is_none());
        Ok(())
    }

    #[convex_macro::test_runtime]
    async fn test_schema_validation_progress_deleted_when_schema_marked_failed(
        rt: TestRuntime,
    ) -> anyhow::Result<()> {
        let db = new_test_database(rt.clone()).await;
        let schema_worker = SchemaWorker {
            runtime: rt.clone(),
            database: db.clone(),
        };
        let mut tx = db.begin_system().await?;
        let table_name = "table".parse::<TableName>()?;
        let db_schema = db_schema!(table_name => DocumentSchema::Union(vec![]));
        let (id, _) = SchemaModel::new_root_for_test(&mut tx)
            .submit_pending(db_schema)
            .await?;
        db.commit(tx).await?;
        schema_worker.run().await?;
        // Check that schema validation progress is written
        let mut tx = db.begin_system().await?;
        let mut model = SchemaValidationProgressModel::new(&mut tx, TableNamespace::test_user());
        let progress = model
            .existing_schema_validation_progress(id)
            .await?
            .unwrap();
        assert_eq!(progress.num_docs_validated, 0);
        // Doesn't need to validate any documents because the schema matches all
        // documents
        assert_eq!(progress.total_docs, Some(0));

        // Insert a document that does not match the schema
        UserFacingModel::new_root_for_test(&mut tx)
            .insert(table_name.clone(), assert_obj!())
            .await?;
        let mut model = SchemaValidationProgressModel::new(&mut tx, TableNamespace::test_user());
        let progress = model.existing_schema_validation_progress(id).await?;
        assert!(progress.is_none());
        Ok(())
    }

    #[convex_macro::test_runtime]
    async fn test_schema_validation_progress_deleted_when_schema_marked_active(
        rt: TestRuntime,
    ) -> anyhow::Result<()> {
        let db = new_test_database(rt.clone()).await;
        let schema_worker = SchemaWorker {
            runtime: rt.clone(),
            database: db.clone(),
        };
        let mut tx = db.begin_system().await?;
        let table_name = "table".parse::<TableName>()?;
        let db_schema = db_schema!(table_name => DocumentSchema::Any);
        let (id, _) = SchemaModel::new_root_for_test(&mut tx)
            .submit_pending(db_schema)
            .await?;
        db.commit(tx).await?;
        schema_worker.run().await?;
        // Check that schema validation progress is written
        let mut tx = db.begin_system().await?;
        let mut model = SchemaValidationProgressModel::new(&mut tx, TableNamespace::test_user());
        let progress = model
            .existing_schema_validation_progress(id)
            .await?
            .unwrap();
        assert_eq!(progress.num_docs_validated, 0);
        // Doesn't need to validate any documents because the schema matches all
        // documents
        assert_eq!(progress.total_docs, Some(0));

        // Marking a schema as active deletes the schema validation progress
        let mut model = SchemaModel::new_root_for_test(&mut tx);
        model.mark_active(id).await?;
        let mut model = SchemaValidationProgressModel::new(&mut tx, TableNamespace::test_user());
        let progress = model.existing_schema_validation_progress(id).await?;
        assert!(progress.is_none());
        Ok(())
    }

    #[convex_macro::test_runtime]
    async fn test_schema_validation_progress_count(rt: TestRuntime) -> anyhow::Result<()> {
        let db = new_test_database(rt.clone()).await;
        let schema_worker = SchemaWorker {
            runtime: rt.clone(),
            database: db.clone(),
        };
        let mut tx = db.begin_system().await?;
        let table_name = "table".parse::<TableName>()?;
        let db_schema = db_schema!(table_name => DocumentSchema::Any);
        let (id, _) = SchemaModel::new_root_for_test(&mut tx)
            .submit_pending(db_schema)
            .await?;
        let mut model = UserFacingModel::new_root_for_test(&mut tx);
        // Insert 21 documents to activate the update_threshold (so updates are not
        // written with each new document, but every 2 documents)
        let total_docs = 21;
        for _ in 0..total_docs {
            model.insert(table_name.clone(), assert_obj!()).await?;
        }
        db.commit(tx).await?;

        let mut tx = db.begin_system().await?;
        let pending_validation = SchemaWorker::pending_schema_validations(&mut tx)
            .await?
            .pop()
            .unwrap();

        schema_worker
            .validate_tables(btreeset! { &table_name}, pending_validation)
            .await?;

        // Make sure the number of documents validated matches the total number of
        // documents
        let mut tx = db.begin_system().await?;
        let mut model = SchemaValidationProgressModel::new(&mut tx, TableNamespace::test_user());
        let progress = model
            .existing_schema_validation_progress(id)
            .await?
            .unwrap();
        assert_eq!(progress.num_docs_validated, total_docs);
        assert_eq!(progress.total_docs, Some(total_docs));
        Ok(())
    }
}

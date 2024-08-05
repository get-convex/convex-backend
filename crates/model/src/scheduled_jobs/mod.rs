use std::{
    collections::BTreeMap,
    sync::{
        Arc,
        LazyLock,
    },
};

use common::{
    components::CanonicalizedComponentFunctionPath,
    document::{
        ParsedDocument,
        ResolvedDocument,
    },
    execution_context::ExecutionContext,
    knobs::{
        TRANSACTION_MAX_NUM_SCHEDULED,
        TRANSACTION_MAX_SCHEDULED_TOTAL_ARGUMENT_SIZE_BYTES,
    },
    maybe_val,
    query::{
        Expression,
        IndexRange,
        IndexRangeExpression,
        Order,
        Query,
    },
    runtime::{
        Runtime,
        UnixTimestamp,
    },
    types::{
        GenericIndexName,
        IndexName,
    },
    virtual_system_mapping::VirtualSystemDocMapper,
};
use database::{
    defaults::system_index,
    unauthorized_error,
    ResolvedQuery,
    SystemMetadataModel,
    Transaction,
};
use errors::ErrorMetadata;
use maplit::btreemap;
use sync_types::Timestamp;
use value::{
    id_v6::DeveloperDocumentId,
    ConvexArray,
    ConvexValue,
    FieldPath,
    ResolvedDocumentId,
    Size,
    TableName,
    TableNamespace,
};

use self::{
    types::{
        ScheduledJob,
        ScheduledJobAttempts,
        ScheduledJobState,
    },
    virtual_table::ScheduledJobsDocMapper,
};
use crate::{
    SystemIndex,
    SystemTable,
};

pub mod types;
pub mod virtual_table;

pub static SCHEDULED_JOBS_TABLE: LazyLock<TableName> = LazyLock::new(|| {
    "_scheduled_jobs"
        .parse()
        .expect("_scheduled_jobs is not a valid system table name")
});

pub static SCHEDULED_JOBS_VIRTUAL_TABLE: LazyLock<TableName> = LazyLock::new(|| {
    "_scheduled_functions"
        .parse()
        .expect("_scheduled_functions is not a valid virtual table name")
});

static SCHEDULED_JOBS_INDEX_BY_ID: LazyLock<IndexName> =
    LazyLock::new(|| GenericIndexName::by_id(SCHEDULED_JOBS_TABLE.clone()));

static SCHEDULED_JOBS_INDEX_BY_CREATION_TIME: LazyLock<IndexName> =
    LazyLock::new(|| GenericIndexName::by_creation_time(SCHEDULED_JOBS_TABLE.clone()));
static SCHEDULED_JOBS_VIRTUAL_INDEX_BY_ID: LazyLock<IndexName> =
    LazyLock::new(|| GenericIndexName::by_id(SCHEDULED_JOBS_VIRTUAL_TABLE.clone()));
static SCHEDULED_JOBS_VIRTUAL_INDEX_BY_CREATION_TIME: LazyLock<IndexName> =
    LazyLock::new(|| GenericIndexName::by_creation_time(SCHEDULED_JOBS_VIRTUAL_TABLE.clone()));

pub static SCHEDULED_JOBS_INDEX: LazyLock<IndexName> =
    LazyLock::new(|| system_index(&SCHEDULED_JOBS_TABLE, "by_next_ts"));
pub static SCHEDULED_JOBS_INDEX_BY_UDF_PATH: LazyLock<IndexName> =
    LazyLock::new(|| system_index(&SCHEDULED_JOBS_TABLE, "by_udf_path_and_next_event_ts"));
pub static SCHEDULED_JOBS_INDEX_BY_COMPLETED_TS: LazyLock<IndexName> =
    LazyLock::new(|| system_index(&SCHEDULED_JOBS_TABLE, "by_completed_ts"));
pub static NEXT_TS_FIELD: LazyLock<FieldPath> =
    LazyLock::new(|| "nextTs".parse().expect("invalid nextTs field"));
pub static COMPLETED_TS_FIELD: LazyLock<FieldPath> =
    LazyLock::new(|| "completedTs".parse().expect("invalid completedTs field"));
static UDF_PATH_FIELD: LazyLock<FieldPath> =
    LazyLock::new(|| "udfPath".parse().expect("invalid udfPath field"));
static COMPONENT_PATH_FIELD: LazyLock<FieldPath> =
    LazyLock::new(|| "component".parse().expect("invalid component field"));

pub struct ScheduledJobsTable;
impl SystemTable for ScheduledJobsTable {
    fn table_name(&self) -> &'static TableName {
        &SCHEDULED_JOBS_TABLE
    }

    fn indexes(&self) -> Vec<SystemIndex> {
        vec![
            // By completed ts. Used to efficiently find jobs to garbage collect.
            SystemIndex {
                name: SCHEDULED_JOBS_INDEX_BY_COMPLETED_TS.clone(),
                fields: vec![COMPLETED_TS_FIELD.clone()].try_into().unwrap(),
            },
            // By next ts. Used to efficiently find next jobs to execute next.
            SystemIndex {
                name: SCHEDULED_JOBS_INDEX.clone(),
                fields: vec![NEXT_TS_FIELD.clone()].try_into().unwrap(),
            },
            // By udf path and next ts. Used by the dashboard to group scheduled jobs by udf
            // function.
            SystemIndex {
                name: SCHEDULED_JOBS_INDEX_BY_UDF_PATH.clone(),
                fields: vec![UDF_PATH_FIELD.clone(), NEXT_TS_FIELD.clone()]
                    .try_into()
                    .unwrap(),
            },
        ]
    }

    fn virtual_table(
        &self,
    ) -> Option<(
        &'static TableName,
        BTreeMap<IndexName, IndexName>,
        Arc<dyn VirtualSystemDocMapper>,
    )> {
        Some((
            &SCHEDULED_JOBS_VIRTUAL_TABLE,
            btreemap! {
                SCHEDULED_JOBS_VIRTUAL_INDEX_BY_CREATION_TIME.clone() =>
                    SCHEDULED_JOBS_INDEX_BY_CREATION_TIME.clone(),
                SCHEDULED_JOBS_VIRTUAL_INDEX_BY_ID.clone() =>
                    SCHEDULED_JOBS_INDEX_BY_ID.clone(),
            },
            Arc::new(ScheduledJobsDocMapper),
        ))
    }

    fn validate_document(&self, document: ResolvedDocument) -> anyhow::Result<()> {
        ParsedDocument::<ScheduledJob>::try_from(document).map(|_| ())
    }
}

// Maintains state for scheduling asynchronous functions (scheduled jobs).
pub struct SchedulerModel<'a, RT: Runtime> {
    tx: &'a mut Transaction<RT>,
    namespace: TableNamespace,
}

impl<'a, RT: Runtime> SchedulerModel<'a, RT> {
    pub fn new(tx: &'a mut Transaction<RT>, namespace: TableNamespace) -> Self {
        Self { tx, namespace }
    }

    fn check_scheduling_limits(&mut self, args: &ConvexArray) -> anyhow::Result<()> {
        // Limit how much you can schedule from a single transaction.
        anyhow::ensure!(
            self.tx.scheduled_size.num_writes < *TRANSACTION_MAX_NUM_SCHEDULED,
            ErrorMetadata::bad_request(
                "TooManyFunctionsScheduled",
                format!(
                    "Too many functions scheduled by this mutation (limit: {})",
                    *TRANSACTION_MAX_NUM_SCHEDULED,
                )
            )
        );
        self.tx.scheduled_size.num_writes += 1;
        anyhow::ensure!(
            self.tx.scheduled_size.size + args.size()
                <= *TRANSACTION_MAX_SCHEDULED_TOTAL_ARGUMENT_SIZE_BYTES,
            ErrorMetadata::bad_request(
                "ScheduledFunctionsArgumentsTooLarge",
                format!(
                    "Too large total size of the arguments of scheduled functions from this \
                     mutation (limit: {} bytes)",
                    *TRANSACTION_MAX_SCHEDULED_TOTAL_ARGUMENT_SIZE_BYTES,
                )
            ),
        );
        self.tx.scheduled_size.size += args.size();
        Ok(())
    }

    pub async fn schedule(
        &mut self,
        path: CanonicalizedComponentFunctionPath,
        args: ConvexArray,
        ts: UnixTimestamp,
        context: ExecutionContext,
    ) -> anyhow::Result<ResolvedDocumentId> {
        if path.udf_path.is_system()
            && !(self.tx.identity().is_admin() || self.tx.identity().is_system())
        {
            anyhow::bail!(unauthorized_error("schedule"))
        }

        self.check_scheduling_limits(&args)?;

        let now: Timestamp = self.tx.runtime().generate_timestamp()?;
        let original_scheduled_ts: Timestamp = ts.as_system_time().try_into()?;
        let scheduled_job = ScheduledJob {
            path: path.clone(),
            udf_args: args.clone(),
            state: ScheduledJobState::Pending,
            // Don't set next_ts in the past to avoid scheduler incorrectly logging
            // it is falling behind. We should keep `original_scheduled_ts` intact
            // since this is exposed to the developer via the virtual table.
            next_ts: Some(original_scheduled_ts.max(now)),
            completed_ts: None,
            original_scheduled_ts,
            attempts: ScheduledJobAttempts::default(),
        };
        let job = if let Some(parent_scheduled_job) = context.parent_scheduled_job {
            let table_mapping = self.tx.table_mapping();
            let parent_scheduled_job = parent_scheduled_job
                .to_resolved(&table_mapping.namespace(self.namespace).number_to_tablet())?;
            if let Some(parent_scheduled_job_state) =
                self.check_status(parent_scheduled_job).await?
            {
                match parent_scheduled_job_state {
                    ScheduledJobState::Pending
                    | ScheduledJobState::InProgress
                    | ScheduledJobState::Failed(_)
                    | ScheduledJobState::Success => scheduled_job,
                    ScheduledJobState::Canceled => {
                        let scheduled_ts = self.tx.begin_timestamp();
                        ScheduledJob {
                            path,
                            udf_args: args,
                            state: ScheduledJobState::Canceled,
                            next_ts: None,
                            completed_ts: Some(*scheduled_ts),
                            original_scheduled_ts: *scheduled_ts,
                            attempts: ScheduledJobAttempts::default(),
                        }
                    },
                }
            } else {
                scheduled_job
            }
        } else {
            scheduled_job
        };
        let id = SystemMetadataModel::new(self.tx, self.namespace)
            .insert_metadata(&SCHEDULED_JOBS_TABLE, job.try_into()?)
            .await?;

        Ok(id)
    }

    pub async fn replace(
        &mut self,
        id: ResolvedDocumentId,
        job: ScheduledJob,
    ) -> anyhow::Result<()> {
        anyhow::ensure!(self
            .tx
            .table_mapping()
            .namespace(self.namespace)
            .tablet_matches_name(id.tablet_id, &SCHEDULED_JOBS_TABLE));
        SystemMetadataModel::new(self.tx, self.namespace)
            .replace(id, job.try_into()?)
            .await?;
        Ok(())
    }

    pub async fn complete(
        &mut self,
        id: ResolvedDocumentId,
        state: ScheduledJobState,
    ) -> anyhow::Result<()> {
        match state {
            ScheduledJobState::InProgress | ScheduledJobState::Pending => {
                anyhow::bail!("invalid state for completing a scheduled job")
            },
            ScheduledJobState::Canceled
            | ScheduledJobState::Failed(_)
            | ScheduledJobState::Success => {},
        }
        let Some(job) = self.tx.get(id).await? else {
            anyhow::bail!("scheduled job not found")
        };
        let job: ParsedDocument<ScheduledJob> = job.try_into()?;
        match job.state {
            ScheduledJobState::Pending | ScheduledJobState::InProgress => {},
            ScheduledJobState::Canceled => {
                // If the job is already canceled. Completing is a no-op. We
                // should proceed without throwing an error.
                return Ok(());
            },
            ScheduledJobState::Failed(_) | ScheduledJobState::Success => {
                anyhow::bail!(
                    "Scheduled job cannot be completed because it is in state {:?}",
                    job.state
                )
            },
        }

        let mut job: ScheduledJob = job.into_value();
        job.state = state;
        // Remove next_ts and set completed_ts so the scheduler knows that the
        // job has already been processed
        job.next_ts = None;
        job.completed_ts = Some(*self.tx.begin_timestamp());
        SystemMetadataModel::new(self.tx, self.namespace)
            .replace(id, job.try_into()?)
            .await?;

        Ok(())
    }

    /// Cancel a scheduled job if it is in Pending or InProgress state.
    /// Otherwise, it has already been completed in another transaction.
    pub async fn cancel(&mut self, id: ResolvedDocumentId) -> anyhow::Result<()> {
        if let Some(scheduled_job) = self.check_status(id).await? {
            match scheduled_job {
                ScheduledJobState::Pending | ScheduledJobState::InProgress => {
                    self.complete(id, ScheduledJobState::Canceled).await?;
                },
                ScheduledJobState::Canceled
                | ScheduledJobState::Success
                | ScheduledJobState::Failed(_) => {},
            }
        } else {
            tracing::error!("Tried to cancel a job with unknown state: {}", id)
        }
        Ok(())
    }

    pub async fn delete(&mut self, id: ResolvedDocumentId) -> anyhow::Result<()> {
        anyhow::ensure!(self
            .tx
            .table_mapping()
            .namespace(self.namespace)
            .tablet_matches_name(id.tablet_id, &SCHEDULED_JOBS_TABLE));
        self.tx.delete_inner(id).await?;
        Ok(())
    }

    // Cancel up to `limit` jobs for the UDF and return how many were canceled.
    // Note: the caller will assume all have been canceled if Result < `limit`.
    pub async fn cancel_all(
        &mut self,
        path: Option<CanonicalizedComponentFunctionPath>,
        limit: usize,
    ) -> anyhow::Result<usize> {
        let index_query = match path {
            Some(path) => {
                let udf_path = path.udf_path;
                let component_path = path.component;
                let mut component_path_filter = Expression::Eq(
                    Expression::Field(COMPONENT_PATH_FIELD.clone()).into(),
                    Expression::Literal(maybe_val!(String::from(component_path.clone()))).into(),
                );
                if component_path.is_root() {
                    component_path_filter = Expression::Or(vec![
                        component_path_filter,
                        Expression::Eq(
                            Expression::Field(COMPONENT_PATH_FIELD.clone()).into(),
                            Expression::Literal(maybe_val!(undefined)).into(),
                        ),
                    ]);
                }
                let range = vec![
                    IndexRangeExpression::Eq(
                        UDF_PATH_FIELD.clone(),
                        ConvexValue::try_from(udf_path.to_string())?.into(),
                    ),
                    IndexRangeExpression::Gt(NEXT_TS_FIELD.clone(), value::ConvexValue::Null),
                ];
                Query::index_range(IndexRange {
                    index_name: SCHEDULED_JOBS_INDEX_BY_UDF_PATH.clone(),
                    range,
                    order: Order::Asc,
                })
                .filter(component_path_filter)
            },
            None => {
                let range = vec![IndexRangeExpression::Gt(
                    NEXT_TS_FIELD.clone(),
                    value::ConvexValue::Null,
                )];
                Query::index_range(IndexRange {
                    index_name: SCHEDULED_JOBS_INDEX.clone(),
                    range,
                    order: Order::Asc,
                })
            },
        };
        let mut query_stream = ResolvedQuery::new(self.tx, self.namespace, index_query)?;
        let mut count = 0;
        while count < limit
            && let Some(doc) = query_stream.next(self.tx, None).await?
        {
            self.cancel(doc.id()).await?;
            count += 1;
        }
        Ok(count)
    }

    pub async fn list(&mut self) -> anyhow::Result<Vec<ParsedDocument<ScheduledJob>>> {
        let scheduled_query = Query::full_table_scan(SCHEDULED_JOBS_TABLE.clone(), Order::Asc);
        let mut query_stream = ResolvedQuery::new(self.tx, self.namespace, scheduled_query)?;
        let mut scheduled_jobs = Vec::new();
        while let Some(job) = query_stream.next(self.tx, None).await? {
            let job: ParsedDocument<ScheduledJob> = job.try_into()?;
            scheduled_jobs.push(job);
        }
        Ok(scheduled_jobs)
    }

    /// Checks the status of the scheduled job. If it has been garbage collected
    /// and the scheduled job is no longer in the table, it returns None.
    pub async fn check_status(
        &mut self,
        job_id: ResolvedDocumentId,
    ) -> anyhow::Result<Option<ScheduledJobState>> {
        let state = self
            .tx
            .get(job_id)
            .await?
            .map(ParsedDocument::<ScheduledJob>::try_from)
            .transpose()?
            .map(|job| job.state.clone());
        Ok(state)
    }
}

/// Same as SchedulerModel but works with the respective virtual table instead
/// of the underlying system table.
pub struct VirtualSchedulerModel<'a, RT: Runtime> {
    tx: &'a mut Transaction<RT>,
    namespace: TableNamespace,
}

impl<'a, RT: Runtime> VirtualSchedulerModel<'a, RT> {
    pub fn new(tx: &'a mut Transaction<RT>, namespace: TableNamespace) -> Self {
        Self { tx, namespace }
    }

    pub async fn schedule(
        &mut self,
        path: CanonicalizedComponentFunctionPath,
        args: ConvexArray,
        ts: UnixTimestamp,
        context: ExecutionContext,
    ) -> anyhow::Result<DeveloperDocumentId> {
        let system_id = SchedulerModel::new(self.tx, self.namespace)
            .schedule(path, args, ts, context)
            .await?;
        self.tx
            .virtual_system_mapping()
            .system_resolved_id_to_virtual_developer_id(system_id)
    }

    pub async fn cancel(&mut self, virtual_id: DeveloperDocumentId) -> anyhow::Result<()> {
        let table_mapping = self.tx.table_mapping().clone();
        let system_id = self
            .tx
            .virtual_system_mapping()
            .virtual_id_v6_to_system_resolved_doc_id(self.namespace, &virtual_id, &table_mapping)?;
        SchedulerModel::new(self.tx, self.namespace)
            .cancel(system_id)
            .await
    }
}

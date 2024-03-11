use std::{
    collections::HashSet,
    sync::Arc,
    time::Duration,
};

use common::{
    self,
    backoff::Backoff,
    document::ParsedDocument,
    errors::{
        report_error,
        JsError,
    },
    knobs::{
        SCHEDULED_JOB_EXECUTION_PARALLELISM,
        SCHEDULED_JOB_GARBAGE_COLLECTION_BATCH_SIZE,
        SCHEDULED_JOB_RETENTION,
        UDF_EXECUTOR_OCC_MAX_RETRIES,
    },
    query::{
        IndexRange,
        IndexRangeExpression,
        Order,
        Query,
    },
    runtime::{
        Runtime,
        RuntimeInstant,
        SpawnHandle,
    },
    types::{
        AllowedVisibility,
        FunctionCaller,
        UdfType,
    },
};
use database::{
    Database,
    ResolvedQuery,
    Transaction,
};
use errors::ErrorMetadataAnyhowExt;
use futures::{
    future::Either,
    select_biased,
    stream::FuturesUnordered,
    Future,
    FutureExt,
    StreamExt,
};
use isolate::{
    ActionOutcome,
    SyscallTrace,
};
use keybroker::Identity;
use model::{
    backend_state::{
        types::BackendState,
        BackendStateModel,
    },
    config::types::ModuleEnvironment,
    scheduled_jobs::{
        types::{
            ScheduledJob,
            ScheduledJobState,
        },
        SchedulerModel,
        COMPLETED_TS_FIELD,
        NEXT_TS_FIELD,
        SCHEDULED_JOBS_INDEX,
        SCHEDULED_JOBS_INDEX_BY_COMPLETED_TS,
    },
};
use parking_lot::Mutex;
use request_context::RequestContext;
use usage_tracking::FunctionUsageTracker;
use value::ResolvedDocumentId;

use crate::{
    application_function_runner::ApplicationFunctionRunner,
    function_log::{
        ActionCompletion,
        UdfExecutionLog,
    },
};

mod metrics;

pub struct ScheduledJobRunner<RT: Runtime> {
    executor: Arc<Mutex<RT::Handle>>,
    garbage_collector: Arc<Mutex<RT::Handle>>,
}

impl<RT: Runtime> Clone for ScheduledJobRunner<RT> {
    fn clone(&self) -> Self {
        Self {
            executor: self.executor.clone(),
            garbage_collector: self.garbage_collector.clone(),
        }
    }
}

impl<RT: Runtime> ScheduledJobRunner<RT> {
    pub fn start(
        rt: RT,
        database: Database<RT>,
        runner: Arc<ApplicationFunctionRunner<RT>>,
        function_log: UdfExecutionLog<RT>,
    ) -> Self {
        let executor_fut =
            ScheduledJobExecutor::start(rt.clone(), database.clone(), runner, function_log);
        let executor = Arc::new(Mutex::new(rt.spawn("scheduled_job_executor", executor_fut)));

        let garbage_collector_fut = ScheduledJobGarbageCollector::start(rt.clone(), database);
        let garbage_collector = Arc::new(Mutex::new(
            rt.spawn("scheduled_job_garbage_collector", garbage_collector_fut),
        ));
        Self {
            executor,
            garbage_collector,
        }
    }

    pub fn shutdown(&self) {
        self.executor.lock().shutdown();
        self.garbage_collector.lock().shutdown();
    }
}

const INITIAL_BACKOFF: Duration = Duration::from_millis(10);
const MAX_BACKOFF: Duration = Duration::from_secs(5);

pub struct ScheduledJobExecutor<RT: Runtime> {
    rt: RT,
    database: Database<RT>,
    runner: Arc<ApplicationFunctionRunner<RT>>,
    function_log: UdfExecutionLog<RT>,
}

impl<RT: Runtime> ScheduledJobExecutor<RT> {
    pub fn start(
        rt: RT,
        database: Database<RT>,
        runner: Arc<ApplicationFunctionRunner<RT>>,
        function_log: UdfExecutionLog<RT>,
    ) -> impl Future<Output = ()> + Send {
        let executor = Self {
            rt,
            database,
            runner,
            function_log,
        };
        async move {
            let mut backoff = Backoff::new(INITIAL_BACKOFF, MAX_BACKOFF);
            while let Err(mut e) = executor.run(&mut backoff).await {
                let delay = executor.rt.with_rng(|rng| backoff.fail(rng));
                tracing::error!("Scheduled job executor failed, sleeping {delay:?}");
                report_error(&mut e);
                executor.rt.wait(delay).await;
            }
        }
    }

    #[cfg(any(test, feature = "testing"))]
    pub fn new(
        rt: RT,
        database: Database<RT>,
        runner: Arc<ApplicationFunctionRunner<RT>>,
        function_log: UdfExecutionLog<RT>,
    ) -> Self {
        Self {
            rt,
            database,
            runner,
            function_log,
        }
    }

    async fn run(&self, backoff: &mut Backoff) -> anyhow::Result<()> {
        tracing::info!("Starting scheduled job executor");
        let mut futures = FuturesUnordered::new();
        let mut running_job_ids = HashSet::new();
        loop {
            let mut tx = self.database.begin(Identity::Unknown).await?;
            // _backend_state appears unused but is needed to make sure the backend_state
            // is part of the readset for the query we subscribe to.
            let _backend_state = BackendStateModel::new(&mut tx).get_backend_state().await?;
            let now = self.rt.generate_timestamp()?;
            let index_query = Query::index_range(IndexRange {
                index_name: SCHEDULED_JOBS_INDEX.clone(),
                range: vec![IndexRangeExpression::Gt(
                    NEXT_TS_FIELD.clone(),
                    value::ConvexValue::Null,
                )],
                order: Order::Asc,
            });
            let mut query_stream = ResolvedQuery::new(&mut tx, index_query)?;

            let mut next_job_wait = None;
            while let Some(doc) = query_stream.next(&mut tx, None).await? {
                // Get the backend state again in case of a race where jobs are scheduled and
                // after the first tx begins the backend is paused.
                let mut new_tx = self.database.begin(Identity::Unknown).await?;
                let backend_state = BackendStateModel::new(&mut new_tx)
                    .get_backend_state()
                    .await?;
                drop(new_tx);
                match backend_state {
                    BackendState::Running => {},
                    BackendState::Paused | BackendState::Disabled => break,
                }
                let job: ParsedDocument<ScheduledJob> = doc.try_into()?;
                let (job_id, job) = job.clone().into_id_and_value();
                if running_job_ids.contains(&job_id) {
                    continue;
                }
                let next_ts = job.next_ts.ok_or_else(|| {
                    anyhow::anyhow!("Could not get next_ts to run scheduled job at")
                })?;
                if next_ts > now {
                    next_job_wait = Some(next_ts - now);
                    break;
                }
                metrics::log_scheduled_job_execution_lag(now - next_ts);
                if running_job_ids.len() == *SCHEDULED_JOB_EXECUTION_PARALLELISM {
                    // We are due to execute the next job, but we can't because of
                    // parallelism limits. We should break after logging the lag
                    // here, and then wake up in few seconds to log the lag again
                    // unless something else changes in between.
                    next_job_wait = Some(Duration::from_secs(5));
                    break;
                }
                futures.push(self.execute_job(job, job_id));
                running_job_ids.insert(job_id);
            }

            let next_job_future = if let Some(next_job_wait) = next_job_wait {
                Either::Left(self.rt.wait(next_job_wait))
            } else {
                Either::Right(std::future::pending())
            };

            let token = tx.into_token()?;
            let subscription = self.database.subscribe(token).await?;
            select_biased! {
                job_id = futures.select_next_some() => {
                    running_job_ids.remove(&job_id);
                }
                _ = next_job_future.fuse() => {
                }
                _ = subscription.wait_for_invalidation().fuse() => {
                },
            };
            backoff.reset();
        }
    }

    // This handles re-running the scheduled function on transient errors. It
    // guarantees that the job was successfully run or the job state changed.
    pub async fn execute_job(
        &self,
        job: ScheduledJob,
        job_id: ResolvedDocumentId,
    ) -> ResolvedDocumentId {
        let mut backoff = Backoff::new(INITIAL_BACKOFF, MAX_BACKOFF);
        loop {
            let result = self.run_function(job.clone(), job_id).await;
            match result {
                Ok(result) => {
                    metrics::log_scheduled_job_success(backoff.failures());
                    return result;
                },
                Err(mut e) => {
                    // Only report OCCs that happen repeatedly
                    if !e.is_occ() || (backoff.failures() as usize) > *UDF_EXECUTOR_OCC_MAX_RETRIES
                    {
                        report_error(&mut e);
                    }
                    let delay = self.rt.with_rng(|rng| backoff.fail(rng));
                    tracing::error!("System error executing job, sleeping {delay:?}");
                    metrics::log_scheduled_job_failure(&e);
                    self.rt.wait(delay).await;
                },
            }
        }
    }

    async fn run_function(
        &self,
        job: ScheduledJob,
        job_id: ResolvedDocumentId,
    ) -> anyhow::Result<ResolvedDocumentId> {
        let usage_tracker = FunctionUsageTracker::new();
        let request_context = RequestContext::new(Some(job_id.into()));
        let (success, mut tx) = self
            .new_transaction_for_job_state(job_id, &job, usage_tracker.clone())
            .await?;
        if !success {
            // Continue without running function since the job state has changed
            return Ok(job_id);
        }

        tracing::info!("Executing {:?}!", job.udf_path);
        let identity = tx.inert_identity();

        // Since we don't specify the function type when we schedule, we have to
        // use the analyzed result.
        let udf_type = match self
            .runner
            .module_cache
            .get_analyzed_function(&mut tx, &job.udf_path)
            .await?
        {
            Ok(analyzed_function) => analyzed_function.udf_type,
            Err(error) => {
                // We don't know what the UdfType is since this is an invalid module.
                // Log as mutation for now.
                let udf_type = UdfType::Mutation;
                SchedulerModel::new(&mut tx)
                    .complete(job_id, ScheduledJobState::Failed(error.clone()))
                    .await?;
                self.database
                    .commit_with_write_source(tx, "scheduled_job_analyze_failure")
                    .await?;
                self.function_log.log_error(
                    job.udf_path,
                    udf_type,
                    self.rt.unix_timestamp(),
                    error,
                    FunctionCaller::Scheduler,
                    None,
                    identity,
                    request_context,
                );
                return Ok(job_id);
            },
        };

        // Note that we do validate that the scheduled function execute during
        // scheduling, but the modules can have been modified since scheduling.
        match udf_type {
            UdfType::Mutation => {
                self.handle_mutation(tx, job, job_id, usage_tracker, request_context)
                    .await?
            },
            UdfType::Action => {
                self.handle_action(tx, job, job_id, usage_tracker, request_context)
                    .await?
            },
            udf_type => {
                let message = format!(
                    r#"Unsupported function type. {:?} in module "{:?} is defined as a {udf_type}. "
                            "Only {} and {} can be scheduled."#,
                    job.udf_path.function_name(),
                    job.udf_path.module(),
                    UdfType::Mutation,
                    UdfType::Action,
                );
                SchedulerModel::new(&mut tx)
                    .complete(job_id, ScheduledJobState::Failed(message.clone()))
                    .await?;
                self.database
                    .commit_with_write_source(tx, "scheduled_job_bad_udf")
                    .await?;
                self.function_log.log_error(
                    job.udf_path,
                    udf_type,
                    self.rt.unix_timestamp(),
                    message,
                    FunctionCaller::Scheduler,
                    None,
                    identity,
                    request_context,
                );
            },
        };

        Ok(job_id)
    }

    async fn handle_mutation(
        &self,
        tx: Transaction<RT>,
        job: ScheduledJob,
        job_id: ResolvedDocumentId,
        usage_tracker: FunctionUsageTracker,
        context: RequestContext,
    ) -> anyhow::Result<()> {
        let start = self.rt.monotonic_now();
        let identity = tx.inert_identity();
        let result = self
            .runner
            .run_mutation_no_udf_log(
                tx,
                job.udf_path.clone(),
                job.udf_args.clone(),
                AllowedVisibility::All,
                context.clone(),
            )
            .await;
        let (mut tx, mut outcome) = match result {
            Ok(r) => r,
            Err(e) => {
                self.runner
                    .log_udf_system_error(
                        job.udf_path.clone(),
                        job.udf_args.clone(),
                        identity,
                        start,
                        FunctionCaller::Scheduler,
                        &e,
                        context,
                    )
                    .await?;
                return Err(e);
            },
        };

        let stats = tx.take_stats();
        let execution_time = start.elapsed();

        if outcome.result.is_ok() {
            SchedulerModel::new(&mut tx)
                .complete(job_id, ScheduledJobState::Success)
                .await?;
            if let Err(err) = self
                .database
                .commit_with_write_source(tx, "scheduled_job_mutation_success")
                .await
            {
                if err.is_deterministic_user_error() {
                    outcome.result = Err(JsError::from_error(err));
                } else {
                    return Err(err);
                }
            }
        }

        if outcome.result.is_err() {
            // UDF failed due to developer error. It is not safe to commit the
            // transaction it executed in. We should remove the job in a new
            // transaction.
            let (success, mut tx) = self
                .new_transaction_for_job_state(job_id, &job, usage_tracker.clone())
                .await?;
            if !success {
                // Continue without updating since the job state has changed
                return Ok(());
            }
            SchedulerModel::new(&mut tx)
                .complete(
                    job_id,
                    ScheduledJobState::Failed(outcome.result.clone().unwrap_err().to_string()),
                )
                .await?;
            // NOTE: We should not be getting developer errors here.
            self.database
                .commit_with_write_source(tx, "scheduled_job_mutation_error")
                .await?;
        }
        self.function_log.log_mutation(
            outcome,
            stats,
            execution_time,
            FunctionCaller::Scheduler,
            false,
            usage_tracker,
            context,
        );

        Ok(())
    }

    async fn handle_action(
        &self,
        tx: Transaction<RT>,
        job: ScheduledJob,
        job_id: ResolvedDocumentId,
        usage_tracker: FunctionUsageTracker,
        context: RequestContext,
    ) -> anyhow::Result<()> {
        let identity = tx.identity().clone();
        let mut tx = self.database.begin(identity.clone()).await?;
        match job.state {
            ScheduledJobState::Pending => {
                // Set state to in progress
                let mut updated_job = job.clone();
                updated_job.state = ScheduledJobState::InProgress;
                SchedulerModel::new(&mut tx)
                    .replace(job_id, updated_job.clone())
                    .await?;
                self.database
                    .commit_with_write_source(tx, "scheduled_job_progress")
                    .await?;

                // Execute the action
                let completion = self
                    .runner
                    .run_action_no_udf_log(
                        job.clone().udf_path,
                        job.udf_args,
                        identity,
                        AllowedVisibility::All,
                        FunctionCaller::Scheduler,
                        usage_tracker.clone(),
                        context.clone(),
                    )
                    .await?;
                let state = match &completion.outcome.result {
                    Ok(_) => ScheduledJobState::Success,
                    Err(e) => ScheduledJobState::Failed(e.to_string()),
                };

                // Mark the job as completed. Keep trying until we succeed (or
                // detect the job state has changed). Don't bubble up the error
                // since otherwise we will lose the original execution logs.
                let mut backoff = Backoff::new(INITIAL_BACKOFF, MAX_BACKOFF);
                while let Err(mut err) = self
                    .complete_action(job_id, &updated_job, usage_tracker.clone(), state.clone())
                    .await
                {
                    let delay = self.rt.with_rng(|rng| backoff.fail(rng));
                    tracing::error!("Failed to update action state, sleeping {delay:?}");
                    report_error(&mut err);
                    self.rt.wait(delay).await;
                }
                self.function_log
                    .log_action(completion, false, usage_tracker);
            },
            ScheduledJobState::InProgress => {
                // This case can happen if there is a system error while executing
                // the action or if backend exits after executing the action but
                // before updating the state. Since we execute actions at most once,
                // complete this job and log the error.
                let message = "Transient error while executing action".to_string();
                SchedulerModel::new(&mut tx)
                    .complete(job_id, ScheduledJobState::Failed(message.clone()))
                    .await?;
                self.database
                    .commit_with_write_source(tx, "scheduled_job_action_error")
                    .await?;
                let outcome = ActionOutcome {
                    unix_timestamp: self.rt.unix_timestamp(),
                    udf_path: job.udf_path,
                    arguments: job.udf_args.clone(),
                    identity: identity.into(),
                    result: Err(JsError::from_message(message)),
                    syscall_trace: SyscallTrace::new(),
                    log_lines: vec![].into(),
                    udf_server_version: None,
                };
                self.function_log.log_action(
                    ActionCompletion {
                        outcome,
                        execution_time: Duration::from_secs(0),
                        environment: ModuleEnvironment::Invalid,
                        memory_in_mb: 0,
                        context,
                        unix_timestamp: self.rt.unix_timestamp(),
                        caller: FunctionCaller::Scheduler,
                        log_lines: vec![].into(),
                    },
                    true,
                    usage_tracker,
                );
            },
            state => {
                anyhow::bail!(
                    "Invalid state for executing action. Expected Pending or InProgress, got {:?}",
                    state
                );
            },
        }
        Ok(())
    }

    // Creates a new transaction and verifies the job state matches the given one.
    async fn new_transaction_for_job_state(
        &self,
        job_id: ResolvedDocumentId,
        expected_state: &ScheduledJob,
        usage_tracker: FunctionUsageTracker,
    ) -> anyhow::Result<(bool, Transaction<RT>)> {
        let mut tx = self
            .database
            .begin_with_usage(Identity::Unknown, usage_tracker)
            .await?;
        // Verify that the scheduled job has not changed.
        let new_job = tx
            .get(job_id)
            .await?
            .map(ParsedDocument::<ScheduledJob>::try_from)
            .transpose()?
            .map(|j| j.into_value());
        Ok((new_job.as_ref() == Some(expected_state), tx))
    }

    // Completes an action in separate transaction. Returns false if the action
    // state has changed.
    async fn complete_action(
        &self,
        job_id: ResolvedDocumentId,
        expected_state: &ScheduledJob,
        usage_tracking: FunctionUsageTracker,
        job_state: ScheduledJobState,
    ) -> anyhow::Result<()> {
        let (success, mut tx) = self
            .new_transaction_for_job_state(job_id, expected_state, usage_tracking)
            .await?;
        if !success {
            // Continue without updating since the job state has changed
            return Ok(());
        }

        // Remove from the scheduled jobs table
        SchedulerModel::new(&mut tx)
            .complete(job_id, job_state)
            .await?;
        self.database
            .commit_with_write_source(tx, "scheduled_job_complete_action")
            .await?;
        Ok(())
    }
}

pub struct ScheduledJobGarbageCollector<RT: Runtime> {
    rt: RT,
    database: Database<RT>,
}

impl<RT: Runtime> ScheduledJobGarbageCollector<RT> {
    pub fn start(rt: RT, database: Database<RT>) -> impl Future<Output = ()> + Send {
        let garbage_collector = Self { rt, database };
        async move {
            let mut backoff = Backoff::new(INITIAL_BACKOFF, MAX_BACKOFF);
            while let Err(mut e) = garbage_collector.run(&mut backoff).await {
                let delay = garbage_collector.rt.with_rng(|rng| backoff.fail(rng));
                tracing::error!("Scheduled job garbage collector failed, sleeping {delay:?}");
                // Only report OCCs that happen repeatedly
                if !e.is_occ() || (backoff.failures() as usize) > *UDF_EXECUTOR_OCC_MAX_RETRIES {
                    report_error(&mut e);
                }
                report_error(&mut e);
                garbage_collector.rt.wait(delay).await;
            }
        }
    }

    async fn run(&self, backoff: &mut Backoff) -> anyhow::Result<()> {
        loop {
            let mut tx = self.database.begin(Identity::system()).await?;
            let now = self.rt.generate_timestamp()?;
            let index_query = Query::index_range(IndexRange {
                index_name: SCHEDULED_JOBS_INDEX_BY_COMPLETED_TS.clone(),
                range: vec![IndexRangeExpression::Gt(
                    COMPLETED_TS_FIELD.clone(),
                    value::ConvexValue::Null,
                )],
                order: Order::Asc,
            })
            .limit(*SCHEDULED_JOB_GARBAGE_COLLECTION_BATCH_SIZE);
            let mut query_stream = ResolvedQuery::new(&mut tx, index_query)?;

            let mut next_job_wait = None;
            let mut jobs_to_delete = vec![];
            while let Some(doc) = query_stream.next(&mut tx, None).await? {
                let job: ParsedDocument<ScheduledJob> = doc.try_into()?;
                match job.state {
                    ScheduledJobState::Success => (),
                    ScheduledJobState::Failed(_) => (),
                    ScheduledJobState::Canceled => (),
                    _ => anyhow::bail!("Scheduled job to be garbage collected has the wrong state"),
                }

                let completed_ts = match job.completed_ts {
                    Some(completed_ts) => completed_ts,
                    None => {
                        anyhow::bail!("Could not get completed_ts of finished scheduled job");
                    },
                };
                if completed_ts.add(*SCHEDULED_JOB_RETENTION)? > now {
                    next_job_wait = Some(completed_ts.add(*SCHEDULED_JOB_RETENTION)? - now);
                    break;
                }
                jobs_to_delete.push(job.id());
            }
            if !jobs_to_delete.is_empty() {
                tracing::debug!(
                    "Garbage collecting {} finished scheduled jobs",
                    jobs_to_delete.len()
                );
                let mut model = SchedulerModel::new(&mut tx);
                for job_id in jobs_to_delete {
                    model.delete(job_id).await?;
                }
                self.database
                    .commit_with_write_source(tx, "scheduled_job_gc")
                    .await?;
                continue;
            }

            let next_job_future = if let Some(next_job_wait) = next_job_wait {
                Either::Left(self.rt.wait(next_job_wait))
            } else {
                Either::Right(std::future::pending())
            };
            let token = tx.into_token()?;
            let subscription = self.database.subscribe(token).await?;
            select_biased! {
                _ = next_job_future.fuse() => {
                }
                _ = subscription.wait_for_invalidation().fuse() => {
                },
            };
            backoff.reset();
        }
    }
}

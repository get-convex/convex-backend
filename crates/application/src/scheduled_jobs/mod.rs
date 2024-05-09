use std::{
    collections::{
        BTreeMap,
        HashSet,
    },
    ops::Deref,
    sync::Arc,
    time::Duration,
};

use common::{
    backoff::Backoff,
    document::ParsedDocument,
    errors::{
        report_error,
        JsError,
    },
    execution_context::ExecutionContext,
    knobs::{
        SCHEDULED_JOB_EXECUTION_PARALLELISM,
        SCHEDULED_JOB_GARBAGE_COLLECTION_BATCH_SIZE,
        SCHEDULED_JOB_RETENTION,
        UDF_EXECUTOR_OCC_MAX_RETRIES,
    },
    minitrace_helpers::get_sampled_span,
    pause::PauseClient,
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
    sync::mpsc,
    tokio::task::yield_now,
    types::{
        FunctionCaller,
        UdfType,
    },
    RequestId,
};
use database::{
    Database,
    ResolvedQuery,
    Transaction,
};
use errors::{
    ErrorMetadata,
    ErrorMetadataAnyhowExt,
};
use futures::{
    future::Either,
    select_biased,
    Future,
    FutureExt,
};
use keybroker::Identity;
use minitrace::future::FutureExt as _;
use model::{
    backend_state::{
        types::BackendState,
        BackendStateModel,
    },
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
use sync_types::Timestamp;
use usage_tracking::FunctionUsageTracker;
use value::ResolvedDocumentId;

use crate::{
    application_function_runner::ApplicationFunctionRunner,
    function_log::FunctionExecutionLog,
};

mod metrics;

pub(crate) const SCHEDULED_JOB_EXECUTED: &str = "scheduled_job_executed";

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
        function_log: FunctionExecutionLog<RT>,
        pause_client: PauseClient,
    ) -> Self {
        let executor_fut = ScheduledJobExecutor::start(
            rt.clone(),
            database.clone(),
            runner,
            function_log,
            pause_client,
        );
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
    context: ScheduledJobContext<RT>,
    pause_client: PauseClient,
}

impl<RT: Runtime> Deref for ScheduledJobExecutor<RT> {
    type Target = ScheduledJobContext<RT>;

    fn deref(&self) -> &Self::Target {
        &self.context
    }
}

#[derive(Clone)]
pub struct ScheduledJobContext<RT: Runtime> {
    rt: RT,
    database: Database<RT>,
    runner: Arc<ApplicationFunctionRunner<RT>>,
    function_log: FunctionExecutionLog<RT>,
}

/// This roughly matches tokio's permits that it uses as part of cooperative
/// scheduling. We shouldn't use this for anything sophisticated, it's just a
/// simple way for us to yield occasionally for scheduled jobs but not yield too
/// often. We really don't need anything fancy here.
const CHECKS_BETWEEN_YIELDS: usize = 128;

impl<RT: Runtime> ScheduledJobExecutor<RT> {
    pub fn start(
        rt: RT,
        database: Database<RT>,
        runner: Arc<ApplicationFunctionRunner<RT>>,
        function_log: FunctionExecutionLog<RT>,
        pause_client: PauseClient,
    ) -> impl Future<Output = ()> + Send {
        let mut executor = Self {
            context: ScheduledJobContext {
                rt,
                database,
                runner,
                function_log,
            },
            pause_client,
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
        function_log: FunctionExecutionLog<RT>,
    ) -> Self {
        Self {
            context: ScheduledJobContext {
                rt,
                database,
                runner,
                function_log,
            },
            pause_client: PauseClient::new(),
        }
    }

    async fn drain_finished_jobs(
        running_job_ids: &mut HashSet<ResolvedDocumentId>,
        rx: &mut mpsc::Receiver<ResolvedDocumentId>,
    ) {
        let mut total_drained = 0;
        while let Ok(job_id) = rx.try_recv() {
            total_drained += 1;
            running_job_ids.remove(&job_id);
            if total_drained % CHECKS_BETWEEN_YIELDS == 0 {
                yield_now().await;
            }
        }
        tracing::debug!("Drained {total_drained} finished scheduled jobs from the channel");
    }

    async fn run(&mut self, backoff: &mut Backoff) -> anyhow::Result<()> {
        tracing::info!("Starting scheduled job executor");
        let (job_finished_tx, mut job_finished_rx) =
            mpsc::channel(*SCHEDULED_JOB_EXECUTION_PARALLELISM);
        let mut running_job_ids = HashSet::new();
        // Some if there's at least one pending job. May be in the past!
        let mut next_job_ready_time = None;
        loop {
            Self::drain_finished_jobs(&mut running_job_ids, &mut job_finished_rx).await;

            let mut tx = self.database.begin(Identity::Unknown).await?;
            let backend_state = BackendStateModel::new(&mut tx).get_backend_state().await?;
            let is_backend_stopped = match backend_state {
                BackendState::Running => false,
                BackendState::Paused | BackendState::Disabled => true,
            };

            next_job_ready_time = if is_backend_stopped {
                // If the backend is stopped we shouldn't poll. Our subscription will notify us
                // when the backend is started again.
                None
            } else if running_job_ids.len() == *SCHEDULED_JOB_EXECUTION_PARALLELISM {
                // A scheduled job may have been added, but we can't do anything because we're
                // still running jobs at our concurrency limit.
                next_job_ready_time
            } else {
                // Great! we have enough remaining concurrency and our backend is running, start
                // new job(s) if we can and update our next ready time.
                self.query_and_start_jobs(&mut tx, &mut running_job_ids, &job_finished_tx)
                    .await?
            };

            metrics::log_num_running_jobs(running_job_ids.len());
            let next_job_future = if let Some(next_job_ts) = next_job_ready_time {
                let now = self.rt.generate_timestamp()?;
                Either::Left(if next_job_ts < now {
                    metrics::log_scheduled_job_execution_lag(now - next_job_ts);
                    // If we're behind, re-run this loop every 5 seconds to log the gauge above and
                    // track how far we're behind in our metrics.
                    self.rt.wait(Duration::from_secs(5))
                } else {
                    metrics::log_scheduled_job_execution_lag(Duration::from_secs(0));
                    self.rt.wait(next_job_ts - now)
                })
            } else {
                metrics::log_scheduled_job_execution_lag(Duration::from_secs(0));
                Either::Right(std::future::pending())
            };

            let token = tx.into_token()?;
            let subscription = self.database.subscribe(token).await?;

            select_biased! {
                job_id = job_finished_rx.recv().fuse() => {
                    if let Some(job_id) = job_id {
                    self.pause_client.wait(SCHEDULED_JOB_EXECUTED).await;
                        running_job_ids.remove(&job_id);
                    } else {
                        anyhow::bail!("Job results channel closed, this is unexpected!");
                    }
                },
                _ = next_job_future.fuse() => {
                },
                _ = subscription.wait_for_invalidation().fuse() => {
                },
            }
            backoff.reset();
        }
    }

    /// Reads through scheduled jobs in timestamp ascending order and starts any
    /// that are allowed by our concurrency limit and the jobs' scheduled
    /// time.
    ///
    /// Returns the time at which the next job in the queue will be ready to
    /// run. If the scheduler is behind, the returned time may be in the
    /// past. Returns None if all jobs are finished or running.
    async fn query_and_start_jobs(
        &self,
        tx: &mut Transaction<RT>,
        running_job_ids: &mut HashSet<ResolvedDocumentId>,
        job_finished_tx: &mpsc::Sender<ResolvedDocumentId>,
    ) -> anyhow::Result<Option<Timestamp>> {
        let now = self.rt.generate_timestamp()?;
        let index_query = Query::index_range(IndexRange {
            index_name: SCHEDULED_JOBS_INDEX.clone(),
            range: vec![IndexRangeExpression::Gt(
                NEXT_TS_FIELD.clone(),
                value::ConvexValue::Null,
            )],
            order: Order::Asc,
        });
        let mut query_stream = ResolvedQuery::new(tx, index_query)?;
        while let Some(doc) = query_stream.next(tx, None).await? {
            let job: ParsedDocument<ScheduledJob> = doc.try_into()?;
            let (job_id, job) = job.clone().into_id_and_value();
            if running_job_ids.contains(&job_id) {
                continue;
            }
            let next_ts = job
                .next_ts
                .ok_or_else(|| anyhow::anyhow!("Could not get next_ts to run scheduled job at"))?;
            // If we can't execute the job return the job's target timestamp. If we're
            // caught up, we can sleep until the timestamp. If we're behind and
            // at our concurrency limit, we can use the timestamp to log how far
            // behind we get.
            if next_ts > now || running_job_ids.len() == *SCHEDULED_JOB_EXECUTION_PARALLELISM {
                return Ok(Some(next_ts));
            }

            let context = self.context.clone();
            let tx = job_finished_tx.clone();

            let root = self
                .rt
                .with_rng(|rng| get_sampled_span("scheduler/execute_job", rng, BTreeMap::new()));
            self.rt.spawn(
                "spawn_scheduled_job",
                async move {
                    let result = context.execute_job(job, job_id).await;
                    let _ = tx.send(result).await;
                }
                .in_span(root),
            );

            running_job_ids.insert(job_id);

            // We might have hit the concurrency limit by adding the new job, so
            // we could check and break immediately if we have.
            // However we want to know the time the next job in the
            // queue (if any) is due, so instead we continue the loop one more
            // time.
        }
        Ok(None)
    }
}

impl<RT: Runtime> ScheduledJobContext<RT> {
    // This handles re-running the scheduled function on transient errors. It
    // guarantees that the job was successfully run or the job state changed.
    pub async fn execute_job(
        &self,
        job: ScheduledJob,
        job_id: ResolvedDocumentId,
    ) -> ResolvedDocumentId {
        let mut backoff = Backoff::new(INITIAL_BACKOFF, MAX_BACKOFF);
        loop {
            // Generate a new request_id for every schedule job execution attempt.
            let request_id = RequestId::new();
            match self.run_function(request_id, job.clone(), job_id).await {
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
        request_id: RequestId,
        job: ScheduledJob,
        job_id: ResolvedDocumentId,
    ) -> anyhow::Result<ResolvedDocumentId> {
        let usage_tracker = FunctionUsageTracker::new();
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
        let caller = FunctionCaller::Scheduler {
            job_id: job_id.into(),
        };
        let udf_type = match self
            .runner
            .module_cache
            .get_analyzed_function(&mut tx, &job.udf_path)
            .await?
        {
            Ok(analyzed_function) => analyzed_function.udf_type,
            Err(error) => {
                SchedulerModel::new(&mut tx)
                    .complete(
                        job_id,
                        ScheduledJobState::Failed(error.user_facing_message()),
                    )
                    .await?;
                self.database
                    .commit_with_write_source(tx, "scheduled_job_analyze_failure")
                    .await?;
                // NOTE: We didn't actually run anything, so we are creating a request context
                // just report the error.
                let context = ExecutionContext::new(request_id, &caller);
                // We don't know what the UdfType is since this is an invalid module.
                // Log as mutation for now.
                self.function_log.log_mutation_system_error(
                    &error,
                    job.udf_path,
                    job.udf_args,
                    identity,
                    self.rt.monotonic_now(),
                    caller,
                    context,
                );
                return Ok(job_id);
            },
        };

        // Note that we do validate that the scheduled function execute during
        // scheduling, but the modules can have been modified since scheduling.
        match udf_type {
            UdfType::Mutation => {
                self.handle_mutation(request_id, caller, tx, job, job_id, usage_tracker)
                    .await?
            },
            UdfType::Action => {
                self.handle_action(request_id, caller, tx, job, job_id, usage_tracker)
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
                // NOTE: We didn't actually run anything, so we are creating a request context
                // just report the error.
                let context = ExecutionContext::new(request_id, &caller);
                match udf_type {
                    UdfType::Query => {
                        self.function_log.log_query_system_error(
                            &ErrorMetadata::bad_request(
                                "UnsupportedScheduledFunctionType",
                                message,
                            )
                            .into(),
                            job.udf_path,
                            job.udf_args,
                            identity,
                            self.rt.monotonic_now(),
                            caller,
                            context,
                        );
                    },
                    UdfType::HttpAction => {
                        // It would be more correct to log this as an HTTP action, but
                        // we don't have things like a URL or method to log with, so log
                        // it as an action with an error message.
                        self.function_log.log_action_system_error(
                            &ErrorMetadata::bad_request(
                                "UnsupportedScheduledFunctionType",
                                message,
                            )
                            .into(),
                            job.udf_path,
                            job.udf_args,
                            identity,
                            self.rt.monotonic_now(),
                            caller,
                            vec![].into(),
                            context,
                        );
                    },
                    // Should be unreachable given the outer match statement
                    UdfType::Mutation => unreachable!(),
                    UdfType::Action => unreachable!(),
                }
            },
        };

        Ok(job_id)
    }

    async fn handle_mutation(
        &self,
        request_id: RequestId,
        caller: FunctionCaller,
        tx: Transaction<RT>,
        job: ScheduledJob,
        job_id: ResolvedDocumentId,
        usage_tracker: FunctionUsageTracker,
    ) -> anyhow::Result<()> {
        let start = self.rt.monotonic_now();
        let context = ExecutionContext::new(request_id, &caller);
        let identity = tx.inert_identity();
        let result = self
            .runner
            .run_mutation_no_udf_log(
                tx,
                job.udf_path.clone(),
                job.udf_args.clone(),
                caller.allowed_visibility(),
                context.clone(),
            )
            .await;
        let (mut tx, mut outcome) = match result {
            Ok(r) => r,
            Err(e) => {
                self.function_log.log_mutation_system_error(
                    &e,
                    job.udf_path.clone(),
                    job.udf_args.clone(),
                    identity,
                    start,
                    caller,
                    context,
                );
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
            caller,
            usage_tracker,
            context,
        );

        Ok(())
    }

    async fn handle_action(
        &self,
        request_id: RequestId,
        caller: FunctionCaller,
        tx: Transaction<RT>,
        job: ScheduledJob,
        job_id: ResolvedDocumentId,
        usage_tracker: FunctionUsageTracker,
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
                let context = ExecutionContext::new(request_id, &caller);
                let completion = self
                    .runner
                    .run_action_no_udf_log(
                        job.clone().udf_path,
                        job.udf_args,
                        identity,
                        caller,
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
                self.function_log.log_action(completion, usage_tracker);
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
                // TODO: This is wrong. We don't know the executionId the action has been
                // started with. We generate a new executionId and use it to log the failures. I
                // guess the correct behavior here is to store the executionId in the state so
                // we can log correctly here.
                let context = ExecutionContext::new(request_id, &caller);
                self.function_log.log_action_system_error(
                    &JsError::from_message(message).into(),
                    job.udf_path,
                    job.udf_args.clone(),
                    identity.into(),
                    self.rt.monotonic_now(),
                    caller,
                    vec![].into(),
                    context,
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
            }
            backoff.reset();
        }
    }
}

use common::{
    components::{
        CanonicalizedComponentFunctionPath,
        ResolvedComponentFunctionPath,
    },
    execution_context::ExecutionContext,
};
use futures::{
    future::BoxFuture,
    select_biased,
    FutureExt,
};
use model::{
    environment_variables::types::{
        EnvVarName,
        EnvVarValue,
    },
    modules::{
        module_versions::FullModuleSource,
        user_error::FunctionNotFoundError,
    },
};
pub mod async_syscall;

pub mod outcome;
mod phase;
pub mod syscall;
use std::{
    cmp::Ordering,
    collections::VecDeque,
    sync::{
        Arc,
        LazyLock,
    },
};

use anyhow::anyhow;
use common::{
    errors::JsError,
    identity::InertIdentity,
    knobs::{
        DATABASE_UDF_SYSTEM_TIMEOUT,
        DATABASE_UDF_USER_TIMEOUT,
        FUNCTION_MAX_ARGS_SIZE,
        FUNCTION_MAX_RESULT_SIZE,
        TRANSACTION_MAX_NUM_SCHEDULED,
        TRANSACTION_MAX_NUM_USER_WRITES,
        TRANSACTION_MAX_READ_SET_INTERVALS,
        TRANSACTION_MAX_READ_SIZE_BYTES,
        TRANSACTION_MAX_READ_SIZE_ROWS,
        TRANSACTION_MAX_SCHEDULED_TOTAL_ARGUMENT_SIZE_BYTES,
        TRANSACTION_MAX_USER_WRITE_SIZE_BYTES,
    },
    log_lines::{
        LogLevel,
        LogLine,
        LogLines,
    },
    query_journal::QueryJournal,
    runtime::{
        Runtime,
        UnixTimestamp,
    },
    types::{
        PersistenceVersion,
        UdfType,
    },
    value::{
        ConvexArray,
        ConvexValue,
    },
};
use database::{
    BiggestDocumentWrites,
    FunctionExecutionSize,
    Transaction,
    OVER_LIMIT_HELP,
};
use deno_core::{
    serde_v8,
    v8,
};
use errors::ErrorMetadata;
use file_storage::TransactionalFileStorage;
use keybroker::KeyBroker;
use rand::Rng;
use rand_chacha::ChaCha12Rng;
use serde_json::Value as JsonValue;
use value::{
    heap_size::{
        HeapSize,
        WithHeapSize,
    },
    NamespacedTableMapping,
    Size,
    TableMappingValue,
    MAX_DOCUMENT_NESTING,
    MAX_USER_SIZE,
    VALUE_TOO_LARGE_SHORT_MSG,
};

use self::{
    async_syscall::{
        AsyncSyscallBatch,
        PendingSyscall,
        QueryManager,
    },
    outcome::UdfOutcome,
    phase::UdfPhase,
    syscall::syscall_impl,
};
use super::{
    helpers::permit::with_release_permit,
    warnings::{
        approaching_duration_limit_warning,
        approaching_limit_warning,
        SystemWarning,
    },
};
use crate::{
    client::{
        EnvironmentData,
        SharedIsolateHeapStats,
        UdfCallback,
        UdfRequest,
    },
    concurrency_limiter::ConcurrencyPermit,
    environment::{
        helpers::{
            module_loader::module_specifier_from_path,
            resolve_promise,
            FunctionOutcome,
            JsonPackedValue,
            SyscallTrace,
            MAX_LOG_LINES,
        },
        udf::async_syscall::DatabaseSyscallsV1,
        AsyncOpRequest,
        IsolateEnvironment,
    },
    helpers::{
        self,
        deserialize_udf_result,
        serialize_udf_args,
    },
    isolate::{
        Isolate,
        IsolateHeapStats,
    },
    metrics::{
        self,
        log_isolate_request_cancelled,
    },
    request_scope::RequestScope,
    strings,
    termination::TerminationReason,
    timeout::{
        FunctionExecutionTime,
        Timeout,
    },
};

pub static CONVEX_ORIGIN: LazyLock<EnvVarName> = LazyLock::new(|| {
    "CONVEX_CLOUD_URL"
        .parse()
        .expect("CONVEX_CLOUD_URL should be a valid EnvVarName")
});

pub static CONVEX_SITE: LazyLock<EnvVarName> = LazyLock::new(|| {
    "CONVEX_SITE_URL"
        .parse()
        .expect("CONVEX_SITE_URL should be a valid EnvVarName")
});

pub struct DatabaseUdfEnvironment<RT: Runtime> {
    rt: RT,

    udf_type: UdfType,
    path: ResolvedComponentFunctionPath,
    arguments: ConvexArray,
    identity: InertIdentity,
    udf_server_version: Option<semver::Version>,

    phase: UdfPhase<RT>,
    file_storage: TransactionalFileStorage<RT>,

    query_manager: QueryManager<RT>,

    persistence_version: PersistenceVersion,
    key_broker: KeyBroker,
    log_lines: LogLines,

    /// Journal from a previous computation of this UDF used as an input to this
    /// UDF. If this is the first run, the journal will be blank.
    prev_journal: QueryJournal,

    /// Journal to write decisions made during this UDF computation.
    next_journal: QueryJournal,

    pending_syscalls: WithHeapSize<VecDeque<PendingSyscall>>,

    syscall_trace: SyscallTrace,

    heap_stats: SharedIsolateHeapStats,

    context: ExecutionContext,

    udf_callback: Box<dyn UdfCallback<RT>>,
}

impl<RT: Runtime> IsolateEnvironment<RT> for DatabaseUdfEnvironment<RT> {
    fn trace(&mut self, level: LogLevel, messages: Vec<String>) -> anyhow::Result<()> {
        self.emit_log_line(LogLine::new_developer_log_line(
            level,
            messages,
            // Note: accessing the current time here is still deterministic since
            // we don't externalize the time to the function.
            self.rt.unix_timestamp(),
        ));
        Ok(())
    }

    fn rng(&mut self) -> anyhow::Result<&mut ChaCha12Rng> {
        self.phase.rng()
    }

    fn unix_timestamp(&self) -> anyhow::Result<UnixTimestamp> {
        self.phase.unix_timestamp()
    }

    fn get_environment_variable(
        &mut self,
        name: EnvVarName,
    ) -> anyhow::Result<Option<EnvVarValue>> {
        self.phase.get_environment_variable(name)
    }

    fn get_table_mapping_without_system_tables(&mut self) -> anyhow::Result<TableMappingValue> {
        Ok(self.phase.tx()?.table_mapping().clone().into())
    }

    fn get_all_table_mappings(&mut self) -> anyhow::Result<NamespacedTableMapping> {
        let namespace = self.phase.component()?.into();
        let tx = self.phase.tx()?;
        Ok(tx.table_mapping().namespace(namespace))
    }

    async fn lookup_source(
        &mut self,
        path: &str,
        timeout: &mut Timeout<RT>,
        permit: &mut Option<ConcurrencyPermit>,
    ) -> anyhow::Result<Option<FullModuleSource>> {
        let user_module_path = path.parse()?;
        let result = self
            .phase
            .get_module(&user_module_path, timeout, permit)
            .await?;
        Ok(result)
    }

    fn syscall(&mut self, name: &str, args: JsonValue) -> anyhow::Result<JsonValue> {
        syscall_impl(self, name, args)
    }

    fn start_async_syscall(
        &mut self,
        name: String,
        args: JsonValue,
        resolver: v8::Global<v8::PromiseResolver>,
    ) -> anyhow::Result<()> {
        self.pending_syscalls.push_back(PendingSyscall {
            name,
            args,
            resolver,
        });
        Ok(())
    }

    fn start_async_op(
        &mut self,
        request: AsyncOpRequest,
        _resolver: v8::Global<v8::PromiseResolver>,
    ) -> anyhow::Result<()> {
        anyhow::bail!(ErrorMetadata::bad_request(
                format!("No{}InQueriesOrMutations", request.name_for_error()),
                format!(
                    "Can't use {} in queries and mutations. Please consider using an action. See https://docs.convex.dev/functions/actions for more details.",
                    request.description_for_error()
                ),
            ))
    }

    fn record_heap_stats(&self, mut isolate_stats: IsolateHeapStats) {
        // Add the memory allocated by the environment itself.
        isolate_stats.environment_heap_size =
            self.pending_syscalls.heap_size() + self.syscall_trace.heap_size();
        self.heap_stats.store(isolate_stats);
    }

    fn user_timeout(&self) -> std::time::Duration {
        *DATABASE_UDF_USER_TIMEOUT
    }

    fn system_timeout(&self) -> std::time::Duration {
        *DATABASE_UDF_SYSTEM_TIMEOUT
    }
}

impl<RT: Runtime> DatabaseUdfEnvironment<RT> {
    #[minitrace::trace]
    pub fn new(
        rt: RT,
        EnvironmentData {
            key_broker,
            system_env_vars,
            file_storage,
            module_loader,
        }: EnvironmentData<RT>,
        heap_stats: SharedIsolateHeapStats,
        UdfRequest {
            path_and_args,
            udf_type,
            identity,
            transaction,
            journal,
            context,
        }: UdfRequest<RT>,
        udf_callback: Box<dyn UdfCallback<RT>>,
    ) -> Self {
        let persistence_version = transaction.persistence_version();
        let (path, arguments, udf_server_version) = path_and_args.consume();
        let component = path.component;
        Self {
            rt: rt.clone(),
            udf_type,
            path,
            arguments,
            identity,
            udf_server_version,

            phase: UdfPhase::new(
                transaction,
                rt,
                module_loader.clone(),
                system_env_vars,
                component,
            ),
            file_storage,

            query_manager: QueryManager::new(),

            persistence_version,
            key_broker,
            log_lines: vec![].into(),
            prev_journal: journal,
            next_journal: QueryJournal::new(),

            pending_syscalls: WithHeapSize::default(),
            syscall_trace: SyscallTrace::new(),
            heap_stats,
            context,

            udf_callback,
        }
    }

    #[minitrace::trace]
    pub async fn run(
        mut self,
        client_id: String,
        isolate: &mut Isolate<RT>,
        isolate_clean: &mut bool,
        cancellation: BoxFuture<'_, ()>,
    ) -> anyhow::Result<(Transaction<RT>, FunctionOutcome)> {
        // Initialize the UDF's RNG from some high-quality entropy. As with
        // `unix_timestamp` below, the UDF is only deterministic modulo this
        // system-generated input.
        let rng_seed = self.rt.rng().gen();
        let unix_timestamp = self.rt.unix_timestamp();

        // See Isolate::with_context for an explanation of this setup code. We can't use
        // that method directly since we want an `await` below, and passing in a
        // generic async closure to `Isolate` is currently difficult.
        let client_id = Arc::new(client_id);
        let (handle, state) = isolate.start_request(client_id, self).await?;
        let mut handle_scope = isolate.handle_scope();
        let v8_context = v8::Context::new(&mut handle_scope);
        let mut context_scope = v8::ContextScope::new(&mut handle_scope, v8_context);

        let mut isolate_context =
            RequestScope::new(&mut context_scope, handle.clone(), state, false).await?;
        let mut result =
            Self::run_inner(&mut isolate_context, cancellation, rng_seed, unix_timestamp).await;

        // Perform a microtask checkpoint one last time before taking the environment
        // to ensure the microtask queue is empty. Otherwise, JS from this request may
        // leak to a subsequent one on isolate reuse.
        isolate_context.scope.perform_microtask_checkpoint();
        *isolate_clean = true;

        // Override the returned result if we hit a termination error.
        match handle.take_termination_error() {
            Ok(Ok(..)) => (),
            Ok(Err(e)) => {
                result = Ok(Err(e));
            },
            Err(e) => {
                result = Err(e);
            },
        }
        // Our environment may be in an inconsistent state after a system error (e.g.
        // the transaction may be missing if we hit a system error during a
        // cross-component call), so be sure to error out here before using the
        // environment.
        let result = result?;

        let execution_time;
        (self, execution_time) = isolate_context.take_environment();
        let success_result_value = match result.as_ref() {
            Ok(v) => Some(v),
            _ => None,
        };
        Self::add_warnings_to_log_lines(
            &self.path.clone().for_logging(),
            &self.arguments,
            execution_time,
            self.phase.execution_size()?,
            self.phase.biggest_document_writes()?,
            success_result_value,
            |warning| {
                self.log_lines.push(LogLine::new_system_log_line(
                    warning.level,
                    warning.messages,
                    // Note: accessing the current time here is still deterministic since
                    // we don't externalize the time to the function.
                    self.rt.unix_timestamp(),
                    warning.system_log_metadata,
                ));
            },
        )?;
        let outcome = match self.udf_type {
            UdfType::Query => FunctionOutcome::Query(UdfOutcome {
                path: self.path.for_logging(),
                arguments: self.arguments,
                identity: self.identity,
                rng_seed,
                observed_rng: self.phase.observed_rng(),
                unix_timestamp,
                observed_time: self.phase.observed_time(),
                log_lines: self.log_lines,
                journal: self.next_journal,
                result: match result {
                    Ok(v) => Ok(JsonPackedValue::pack(v)),
                    Err(e) => Err(e),
                },
                syscall_trace: self.syscall_trace,
                udf_server_version: self.udf_server_version,
            }),
            // TODO: Add num_writes and write_bandwidth to UdfOutcome,
            // and use them in log_mutation.
            UdfType::Mutation => FunctionOutcome::Mutation(UdfOutcome {
                path: self.path.for_logging(),
                arguments: self.arguments,
                identity: self.identity,
                rng_seed,
                observed_rng: self.phase.observed_rng(),
                unix_timestamp,
                observed_time: self.phase.observed_time(),
                log_lines: self.log_lines,
                journal: self.next_journal,
                result: match result {
                    Ok(v) => Ok(JsonPackedValue::pack(v)),
                    Err(e) => Err(e),
                },
                syscall_trace: self.syscall_trace,
                udf_server_version: self.udf_server_version,
            }),
            _ => anyhow::bail!("UdfEnvironment should only run queries and mutations"),
        };
        Ok((self.phase.into_transaction()?, outcome))
    }

    #[convex_macro::instrument_future]
    #[minitrace::trace]
    async fn run_inner(
        isolate: &mut RequestScope<'_, '_, RT, Self>,
        cancellation: BoxFuture<'_, ()>,
        rng_seed: [u8; 32],
        unix_timestamp: UnixTimestamp,
    ) -> anyhow::Result<Result<ConvexValue, JsError>> {
        let handle = isolate.handle();
        let mut v8_scope = isolate.scope();

        let mut scope = RequestScope::<RT, Self>::enter(&mut v8_scope);

        // Initialize the environment, preloading the UDF config, before executing any
        // JS.
        {
            let state = scope.state_mut()?;
            state
                .environment
                .phase
                .initialize(&mut state.timeout, &mut state.permit)
                .await?;
        }

        let (udf_type, path, udf_args) = {
            let state = scope.state()?;
            let environment = &state.environment;
            (
                environment.udf_type,
                environment.path.clone(),
                environment.arguments.clone(),
            )
        };
        let udf_path = path.udf_path.clone();

        // Don't allow directly running a UDF within the `_deps` directory. We don't
        // really expect users to hit this unless someone is trying to exploit
        // an app written on Convex by calling directly into a compromised
        // dependency. So, consider it a system error so we can just
        // keep a watch on it.
        if udf_path.module().is_deps() {
            anyhow::bail!("Refusing to run {udf_path:?} within the '_deps' directory");
        }

        // First, load the user's module and find the specified function.
        let module_path = udf_path.module().clone();
        let Ok(module_specifier) = module_specifier_from_path(&module_path) else {
            let message = format!("Invalid module path: {module_path:?}");
            return Ok(Err(JsError::from_message(message)));
        };

        let module = match scope
            .eval_user_module(udf_type, false, &module_specifier)
            .await?
        {
            Ok(id) => id,
            Err(e) => return Ok(Err(e)),
        };
        let namespace = module
            .get_module_namespace()
            .to_object(&mut scope)
            .ok_or_else(|| anyhow!("Module namespace wasn't an object?"))?;
        let function_name = udf_path.function_name();
        let function_str: v8::Local<'_, v8::Value> = v8::String::new(&mut scope, function_name)
            .ok_or_else(|| anyhow!("Failed to create function name string"))?
            .into();

        if namespace.has(&mut scope, function_str) != Some(true) {
            let message = format!(
                "{}",
                FunctionNotFoundError::new(udf_path.function_name(), udf_path.module().as_str())
            );
            return Ok(Err(JsError::from_message(message)));
        }
        let function: v8::Local<v8::Function> = namespace
            .get(&mut scope, function_str)
            .ok_or_else(|| anyhow!("Did not find function in module after checking?"))?
            .try_into()?;

        // Mutations and queries are wrapped in JavaScript by a function that adds a
        // property marking it as a query or mutation UDF.
        let is_mutation_str = strings::isMutation.create(&mut scope)?.into();
        let mut is_mutation = false;
        if let Some(true) = function.has(&mut scope, is_mutation_str) {
            is_mutation = function
                .get(&mut scope, is_mutation_str)
                .ok_or_else(|| anyhow!("Missing `is_mutation` after explicit check"))?
                .is_true();
        }

        let is_query_str = strings::isQuery.create(&mut scope)?.into();
        let mut is_query = false;
        if let Some(true) = function.has(&mut scope, is_query_str) {
            is_query = function
                .get(&mut scope, is_query_str)
                .ok_or_else(|| anyhow!("Missing `is_query` after explicit check"))?
                .is_true();
        }
        let invoke_query_str = strings::invokeQuery.create(&mut scope)?.into();
        let invoke_mutation_str = strings::invokeMutation.create(&mut scope)?.into();

        let invoke_str = match (udf_type, is_query, is_mutation) {
            (UdfType::Query, true, false) => invoke_query_str,
            (UdfType::Mutation, false, true) => invoke_mutation_str,
            (_, false, false) => {
                let message = format!(
                    "Function {udf_path:?} is neither a query or mutation. Did you forget to wrap \
                     it with `query` or `mutation`?"
                );
                return Ok(Err(JsError::from_message(message)));
            },
            (UdfType::Query, false, true) => {
                let message = format!(
                    "Function {udf_path:?} is registered as a mutation but is being run as a \
                     query."
                );
                return Ok(Err(JsError::from_message(message)));
            },
            (UdfType::Mutation, true, false) => {
                let message = format!(
                    "Function {udf_path:?} is registered as a query but is being run as a \
                     mutation."
                );
                return Ok(Err(JsError::from_message(message)));
            },
            _ => {
                anyhow::bail!(
                    "Unexpected function classification: {udf_type} vs. (is_query: {is_query}, \
                     is_mutation: {is_mutation})"
                );
            },
        };

        let args_str = serialize_udf_args(udf_args)?;
        metrics::log_argument_length(&args_str);
        let args_v8_str = v8::String::new(&mut scope, &args_str)
            .ok_or_else(|| anyhow!("Failed to create argument string"))?;

        let invoke: v8::Local<v8::Function> = function
            .get(&mut scope, invoke_str)
            .ok_or_else(|| anyhow!("Couldn't find invoke function in {udf_path:?}"))?
            .try_into()?;

        // Switch our phase to executing right before calling into the UDF.
        {
            let state = scope.state_mut()?;
            state
                .environment
                .phase
                .begin_execution(rng_seed, unix_timestamp)?;
        }
        let global = scope.get_current_context().global(&mut scope);
        let promise_r =
            scope.with_try_catch(|s| invoke.call(s, global.into(), &[args_v8_str.into()]));
        // If we hit a system error within a syscall, return `Err`, even if JS thinks it
        // returned successfully. The syscall layer uses
        // `scope.terminate_execution()` when we hit a system error, which
        // unfortunately doesn't actually terminate execution immediately. So, it's
        // possible for JS after the failed syscall to keep running and return a result
        // here before checking the termination flag.
        handle.check_terminated()?;
        let promise: v8::Local<v8::Promise> = match promise_r? {
            Ok(Some(v)) => v.try_into()?,
            Ok(None) => anyhow::bail!("Successful invocation returned None"),
            Err(e) => return Ok(Err(e)),
        };
        let mut cancellation = cancellation.fuse();
        loop {
            // Advance the user's promise as far as it can go by draining the microtask
            // queue.
            scope.perform_microtask_checkpoint();
            scope.record_heap_stats()?;
            handle.check_terminated()?;

            // Check for rejected promises still unhandled, if so terminate.
            let rejections = scope.pending_unhandled_promise_rejections_mut();
            if let Some(promise) = rejections.exceptions.keys().next().cloned() {
                let error = rejections.exceptions.remove(&promise).unwrap();

                let as_local = v8::Local::new(&mut scope, error);
                let err = match scope.format_traceback(as_local) {
                    Ok(e) => e,
                    Err(e) => {
                        handle.terminate_and_throw(TerminationReason::SystemError(Some(e)))?;
                    },
                };
                handle.terminate_and_throw(TerminationReason::UnhandledPromiseRejection(err))?;
            }

            if let v8::PromiseState::Rejected = promise.state() {
                // Stop execution immediately once we hit an error.
                break;
            }

            // If the user's promise is blocked, it must have a pending syscall.
            // Execute a batch of syscalls before reentering into JS.
            // These are executed in a batch deterministically, down to which fetches
            // hit the cache. AsyncSyscallBatch decides which syscalls can run in
            // a batch together.
            // Results are externalized to user space in FIFO order.
            let (resolvers, results) = {
                let state = scope.state_mut()?;
                let Some(p) = state.environment.pending_syscalls.pop_front() else {
                    // No syscalls or javascript to run, so we're done.
                    break;
                };
                let mut batch = AsyncSyscallBatch::new(p.name, p.args);
                let mut resolvers = vec![p.resolver];
                while let Some(p) = state.environment.pending_syscalls.front()
                    && batch.can_push(&p.name, &p.args)
                {
                    let p = state
                        .environment
                        .pending_syscalls
                        .pop_front()
                        .expect("should have a syscall");
                    batch.push(p.name, p.args)?;
                    resolvers.push(p.resolver);
                }
                // Pause the user-code UDF timeout for the duration of the syscall.
                // This works because we know that the user is blocked on some syscall,
                // so running the syscall is on us and we shouldn't count this time
                // towards the user timeout. When we allow more concurrency, we
                // may have to rework this.
                // NOTE: Even though we release the permit, the syscall does in v8.
                // It is better if we run it in tokio to avoid oversubscribing the CPU.
                // TODO: Consider running the async call from a tokio thread.
                // Even though the future would be blocking on the database most of the
                // time it still does some processing that might result in oversubscribing
                // the CPU threads dedicated to v8.
                let results = select_biased! {
                    _ = cancellation => {
                        log_isolate_request_cancelled();
                        anyhow::bail!("Cancelled");
                    },
                    results = with_release_permit(
                        &mut state.timeout,
                        &mut state.permit,
                        DatabaseSyscallsV1::run_async_syscall_batch(
                            &mut state.environment, batch,
                        ).map(Ok),
                    ).fuse() => results?,
                };
                (resolvers, results)
            };
            // Every syscall must have a result (which could be an error or None).
            assert_eq!(resolvers.len(), results.len());

            // Complete the syscall's promise, which will put its handlers on the microtask
            // queue.
            for (resolver, result) in resolvers.into_iter().zip(results.into_iter()) {
                let result_v8 = match result {
                    Ok(v) => Ok(serde_v8::to_v8(&mut scope, v)?),
                    Err(e) => Err(e),
                };
                resolve_promise(&mut scope, resolver, result_v8)?;
            }
            handle.check_terminated()?;
        }

        // Check to see if the user's promise is blocked.
        let result = match promise.state() {
            v8::PromiseState::Pending => Err(JsError::from_message(
                "Returned promise will never resolve".to_string(),
            )),
            v8::PromiseState::Fulfilled => {
                anyhow::ensure!(
                    scope.state()?.environment.pending_syscalls.is_empty(),
                    "queries and mutations should run all syscalls to completion"
                );
                let promise_result_v8 = promise.result(&mut scope);
                let result_v8_str: v8::Local<v8::String> = promise_result_v8.try_into()?;
                let result_str = helpers::to_rust_string(&mut scope, &result_v8_str)?;
                metrics::log_result_length(&result_str);
                deserialize_udf_result(&path, &result_str)?
            },
            v8::PromiseState::Rejected => {
                let e = promise.result(&mut scope);
                Err(scope.format_traceback(e)?)
            },
        };

        Ok(result)
    }

    pub fn emit_log_line(&mut self, log_line: LogLine) {
        // - 1 to reserve for the [ERROR] log line
        match self.log_lines.len().cmp(&(MAX_LOG_LINES - 1)) {
            Ordering::Less => self.log_lines.push(log_line),
            Ordering::Equal => {
                drop(log_line);
                let log_line = LogLine::new_developer_log_line(
                    LogLevel::Error,
                    vec![format!(
                        "Log overflow (maximum {MAX_LOG_LINES}). Remaining log lines omitted."
                    )],
                    // Note: accessing the current time here is still deterministic since
                    // we don't externalize the time to the function.
                    self.rt.unix_timestamp(),
                );
                self.log_lines.push(log_line);
            },
            Ordering::Greater => (),
        }
    }

    // Called when a function finishes
    pub fn add_warnings_to_log_lines(
        path: &CanonicalizedComponentFunctionPath,
        arguments: &ConvexArray,
        execution_time: FunctionExecutionTime,
        execution_size: FunctionExecutionSize,
        biggest_writes: Option<BiggestDocumentWrites>,
        result: Option<&ConvexValue>,
        mut trace_system_warning: impl FnMut(SystemWarning),
    ) -> anyhow::Result<()> {
        // let execution_size = self.phase.execution_size();
        // let biggest_writes = self.phase.biggest_document_writes();
        let udf_path = path.udf_path.clone();
        let system_udf_path = if udf_path.is_system() {
            Some(udf_path.clone())
        } else {
            None
        };
        if let Some(warning) = approaching_limit_warning(
            arguments.size(),
            *FUNCTION_MAX_ARGS_SIZE,
            "TooLargeFunctionArguments",
            || "Large size of the function arguments".to_string(),
            None,
            Some(" bytes"),
            system_udf_path.as_ref(),
        )? {
            trace_system_warning(warning);
        }
        if let Some(warning) = approaching_limit_warning(
            execution_size.read_size.total_document_count,
            *TRANSACTION_MAX_READ_SIZE_ROWS,
            "TooManyDocumentsRead",
            || "Many documents read in a single function execution".to_string(),
            Some(OVER_LIMIT_HELP),
            None,
            system_udf_path.as_ref(),
        )? {
            trace_system_warning(warning);
        }
        if let Some(warning) = approaching_limit_warning(
            execution_size.num_intervals,
            *TRANSACTION_MAX_READ_SET_INTERVALS,
            "TooManyReads",
            || "Many reads in a single function execution".to_string(),
            Some(OVER_LIMIT_HELP),
            None,
            system_udf_path.as_ref(),
        )? {
            trace_system_warning(warning);
        }
        if let Some(warning) = approaching_limit_warning(
            execution_size.read_size.total_document_size,
            *TRANSACTION_MAX_READ_SIZE_BYTES,
            "TooManyBytesRead",
            || "Many bytes read in a single function execution".to_string(),
            Some(OVER_LIMIT_HELP),
            Some(" bytes"),
            system_udf_path.as_ref(),
        )? {
            trace_system_warning(warning);
        }
        if let Some(warning) = approaching_limit_warning(
            execution_size.write_size.num_writes,
            *TRANSACTION_MAX_NUM_USER_WRITES,
            "TooManyWrites",
            || "Many writes in a single function execution".to_string(),
            None,
            None,
            system_udf_path.as_ref(),
        )? {
            trace_system_warning(warning);
        }
        if let Some(warning) = approaching_limit_warning(
            execution_size.write_size.size,
            *TRANSACTION_MAX_USER_WRITE_SIZE_BYTES,
            "TooManyBytesWritten",
            || "Many bytes written in a single function execution".to_string(),
            None,
            Some(" bytes"),
            system_udf_path.as_ref(),
        )? {
            trace_system_warning(warning);
        }
        if let Some(warning) = approaching_limit_warning(
            execution_size.scheduled_size.num_writes,
            *TRANSACTION_MAX_NUM_SCHEDULED,
            "TooManyFunctionsScheduled",
            || "Many functions scheduled by this mutation".to_string(),
            None,
            None,
            system_udf_path.as_ref(),
        )? {
            trace_system_warning(warning);
        }
        if let Some(warning) = approaching_limit_warning(
            execution_size.scheduled_size.size,
            *TRANSACTION_MAX_SCHEDULED_TOTAL_ARGUMENT_SIZE_BYTES,
            "ScheduledFunctionsArgumentsTooLarge",
            || {
                "Large total size of the arguments of scheduled functions from this mutation"
                    .to_string()
            },
            None,
            Some(" bytes"),
            system_udf_path.as_ref(),
        )? {
            trace_system_warning(warning);
        }

        if let Some(biggest_writes) = biggest_writes {
            let (max_size_document_id, max_size) = biggest_writes.max_size;
            if let Some(warning) = approaching_limit_warning(
                max_size,
                MAX_USER_SIZE,
                VALUE_TOO_LARGE_SHORT_MSG,
                || format!("Large document written with ID \"{max_size_document_id}\""),
                None,
                Some(" bytes"),
                system_udf_path.as_ref(),
            )? {
                trace_system_warning(warning);
            }
            let (max_nesting_document_id, max_nesting) = biggest_writes.max_nesting;
            if let Some(warning) = approaching_limit_warning(
                max_nesting,
                MAX_DOCUMENT_NESTING,
                "TooNested",
                || format!("Deeply nested document written with ID \"{max_nesting_document_id}\""),
                None,
                Some(" levels"),
                system_udf_path.as_ref(),
            )? {
                trace_system_warning(warning);
            }
        }

        if let Some(result) = result {
            if let Some(warning) = approaching_limit_warning(
                result.size(),
                *FUNCTION_MAX_RESULT_SIZE,
                "TooLargeFunctionResult",
                || "Large size of the function return value".to_string(),
                None,
                Some(" bytes"),
                system_udf_path.as_ref(),
            )? {
                trace_system_warning(warning);
            }
        };
        if let Some(warning) = approaching_duration_limit_warning(
            execution_time.elapsed,
            execution_time.limit,
            "UserTimeout",
            "Function execution took a long time",
            system_udf_path.as_ref(),
        )? {
            trace_system_warning(warning);
        }
        Ok(())
    }
}

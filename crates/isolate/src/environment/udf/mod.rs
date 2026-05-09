use astral_future::AstralBody;
use common::{
    audit_log_lines::{
        AuditLogLine,
        AuditLogLines,
    },
    components::{
        CanonicalizedComponentFunctionPath,
        ResolvedComponentFunctionPath,
    },
    document::{
        MAX_DOCUMENT_NESTING,
        MAX_USER_SIZE,
    },
    errors::report_error_sync,
    execution_context::ExecutionContext,
    knobs::ISOLATE_MAX_USER_HEAP_SIZE,
};
use futures::{
    future::{
        self,
        BoxFuture,
    },
    FutureExt,
};
use itertools::Either;
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
use sync_types::types::SerializedArgs;
use tokio::{
    select,
    sync::oneshot,
};
use udf::{
    helpers::parse_udf_args,
    warnings::scheduled_arg_size_warning,
    FunctionOutcome,
    NestedUdfOutcome,
    SyscallTrace,
};

use crate::{
    environment::udf::astral_future::RecursiveExecutor,
    termination::{
        ContextTerminationReason,
        IsolateTerminationReason,
    },
    timeout::PauseReason,
    IsolateClient,
};
pub mod async_syscall;

mod astral_future;
mod phase;
pub mod syscall;
use std::{
    cmp::Ordering,
    collections::VecDeque,
    sync::Arc,
    time::Duration,
};

use anyhow::{
    anyhow,
    Context as _,
};
use common::{
    errors::JsError,
    identity::InertIdentity,
    knobs::{
        AUDIT_LOG_MAX_HEAP_SIZE_BYTES,
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
        DeploymentMetadata,
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
    v8::{
        self,
        scope,
    },
};
use errors::ErrorMetadata;
use file_storage::TransactionalFileStorage;
use keybroker::FunctionRunnerKeyBroker;
use rand_chacha::ChaCha12Rng;
use serde_json::Value as JsonValue;
use udf::{
    warnings::{
        approaching_duration_limit_warning,
        approaching_limit_warning,
        SystemWarning,
    },
    UdfOutcome,
};
use value::{
    heap_size::{
        HeapSize,
        WithHeapSize,
    },
    serialized_args_ext::SerializedArgsExt,
    JsonPackedValue,
    NamespacedTableMapping,
    Size,
    VALUE_TOO_LARGE_SHORT_MSG,
};

use self::{
    async_syscall::{
        AsyncSyscallBatch,
        PendingSyscall,
        QueryManager,
    },
    phase::UdfPhase,
    syscall::syscall_impl,
};
use super::ModuleCodeCacheResult;
use crate::{
    client::{
        EnvironmentData,
        SharedIsolateHeapStats,
        UdfCallback,
        UdfRequest,
    },
    environment::{
        helpers::{
            module_loader::module_specifier_from_path,
            resolve_promise,
            MAX_LOG_LINES,
        },
        udf::async_syscall::DatabaseSyscallsV1,
        AsyncOpRequest,
        IsolateEnvironment,
    },
    helpers::{
        self,
        deserialize_udf_result,
        pump_message_loop,
    },
    isolate::{
        Isolate,
        IsolateHeapStats,
    },
    metrics::{
        self,
        log_isolate_request_cancelled,
    },
    request_scope::{
        RequestScope,
        RequestState,
    },
    strings,
    termination::IsolateHandle,
    timeout::{
        FunctionExecutionTime,
        PauseGuard,
        Timeout,
    },
};

pub struct DatabaseUdfEnvironment<RT: Runtime> {
    rt: RT,

    udf_type: UdfType,
    path: ResolvedComponentFunctionPath,
    arguments: SerializedArgs,
    identity: InertIdentity,
    udf_server_version: Option<semver::Version>,
    deployment: DeploymentMetadata,
    client_id: String,

    phase: UdfPhase<RT>,
    file_storage: TransactionalFileStorage<RT>,

    query_manager: QueryManager<RT>,

    key_broker: FunctionRunnerKeyBroker,
    log_lines: LogLines,
    audit_log_lines: AuditLogLines,

    /// Journal from a previous computation of this UDF used as an input to this
    /// UDF. If this is the first run, the journal will be blank.
    prev_journal: QueryJournal,

    /// Journal to write decisions made during this UDF computation.
    next_journal: QueryJournal,

    pending_syscalls: WithHeapSize<VecDeque<PendingSyscall>>,

    syscall_trace: SyscallTrace,

    heap_stats: SharedIsolateHeapStats,

    context: ExecutionContext,

    reactor_depth: usize,
}

fn not_allowed_in_udf(name: &str, description: &str) -> ErrorMetadata {
    ErrorMetadata::bad_request(
        format!("No{name}InQueriesOrMutations"),
        format!(
            "Can't use {description} in queries and mutations. Please consider using an action. \
            See https://docs.convex.dev/functions/actions for more details.",
        ),
    )
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

    fn crypto_rng(&mut self) -> anyhow::Result<super::crypto_rng::CryptoRng> {
        anyhow::bail!(not_allowed_in_udf("CryptoRng", "cryptographic randomness"))
    }

    fn unix_timestamp(&mut self) -> anyhow::Result<UnixTimestamp> {
        self.phase.unix_timestamp()
    }

    fn performance_now(&mut self) -> anyhow::Result<Duration> {
        anyhow::bail!(not_allowed_in_udf("Performance", "the Performance API"))
    }

    fn performance_time_origin(&mut self) -> anyhow::Result<UnixTimestamp> {
        anyhow::bail!(not_allowed_in_udf("Performance", "the Performance API"))
    }

    fn get_environment_variable(
        &mut self,
        name: EnvVarName,
    ) -> anyhow::Result<Option<EnvVarValue>> {
        self.phase.get_environment_variable(name)
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
    ) -> anyhow::Result<Option<(Arc<FullModuleSource>, ModuleCodeCacheResult)>> {
        let user_module_path = path.parse()?;
        let result = self.phase.get_module(&user_module_path, timeout).await?;
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
        anyhow::bail!(not_allowed_in_udf(
            request.name_for_error(),
            &request.description_for_error(),
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

    fn is_nested_function(&self) -> bool {
        self.reactor_depth > 0
    }
}

type UdfRecursiveExecutor<RT> = RecursiveExecutor<
    anyhow::Result<(
        DatabaseUdfEnvironment<RT>,
        anyhow::Result<Result<ConvexValue, JsError>>,
    )>,
>;
struct RunUdf<'a, 'b, RT: Runtime> {
    rt: &'a RT,
    v8_scope: &'a mut v8::Isolate,
    paused_timeout: &'a mut PauseGuard<'b, RT>,
    isolate_handle: &'a IsolateHandle,
    executor: &'a UdfRecursiveExecutor<RT>,
    heap_stats: &'a SharedIsolateHeapStats,
}

impl<'a, 'b, RT: Runtime> UdfCallback<RT> for RunUdf<'a, 'b, RT> {
    async fn execute_nested_udf(
        self,
        client_id: String,
        udf_request: UdfRequest<RT>,
        environment_data: EnvironmentData<RT>,
        rng_seed: [u8; 32],
        reactor_depth: usize,
    ) -> anyhow::Result<(Transaction<RT>, NestedUdfOutcome)> {
        let function_timestamp = udf_request.unix_timestamp;
        let nested_provider = DatabaseUdfEnvironment::new(
            self.rt.clone(),
            environment_data,
            self.heap_stats.clone(),
            udf_request,
            reactor_depth,
            client_id,
        );
        // it is not necessary to propagate cancellation as the parent will already
        // cancel the entire tree of futures.
        let cancellation = future::pending().boxed();
        // N.B.: `run_nested` calls the corresponding `pop_context`.
        // This may not happen in case of a system error, but in that case we
        // are going to throw away the entire context stack anyway.
        let context_id = self.isolate_handle.push_context(true /* nested */);
        let request_state = RequestState::new(self.rt.clone(), nested_provider, context_id);
        // N.B.: we don't use this value here; only the top-level `run_nested` matters.
        let mut isolate_clean = false;
        // User code is going to run again; regain the concurrency permit.
        let mut unpause_guard = self.paused_timeout.regain().await?;
        // Actually run the UDF.
        let future = DatabaseUdfEnvironment::<RT>::run_nested(
            self.executor,
            rng_seed,
            function_timestamp,
            self.v8_scope,
            None,
            self.isolate_handle.clone(),
            request_state,
            &mut *unpause_guard,
            &mut isolate_clean,
            cancellation,
            None, /* udf_callback */
        );
        // Use an AstralFuture to move the responsibility of polling `future`
        // to the `RecursiveExecutor` (created by DatabaseUdfEnvironment::run()).
        // This avoids creating a deep stack of recursive `run_udf` calls.
        let body = std::pin::pin!(AstralBody::new(future));
        // safety: this future must not be leaked
        let (nested_provider, result) = self.executor.spawn(unsafe { body.project() }).await??;
        let outcome = NestedUdfOutcome {
            observed_identity: nested_provider.phase.observed_identity(),
            observed_rng: nested_provider.phase.observed_rng(),
            observed_time: nested_provider.phase.observed_time(),
            audit_log_lines: nested_provider.audit_log_lines,
            log_lines: nested_provider.log_lines,
            journal: nested_provider.next_journal,
            result: result?,
            syscall_trace: nested_provider.syscall_trace,
        };
        Ok((nested_provider.phase.into_transaction()?, outcome))
    }
}

impl<RT: Runtime> DatabaseUdfEnvironment<RT> {
    pub fn new(
        rt: RT,
        EnvironmentData {
            key_broker,
            default_system_env_vars,
            file_storage,
            module_loader,
            deployment,
        }: EnvironmentData<RT>,
        heap_stats: SharedIsolateHeapStats,
        UdfRequest {
            path_and_args,
            udf_type,
            transaction,
            unix_timestamp: _,
            journal,
            context,
        }: UdfRequest<RT>,
        reactor_depth: usize,
        client_id: String,
    ) -> Self {
        let (path, arguments, udf_server_version) = path_and_args.consume();
        let component = path.component;
        Self {
            rt: rt.clone(),
            udf_type,
            path,
            arguments,
            identity: transaction.inert_identity(),
            udf_server_version,

            phase: UdfPhase::new(
                transaction,
                rt,
                module_loader.clone(),
                default_system_env_vars,
                component,
            ),
            file_storage,

            query_manager: QueryManager::new(),

            key_broker,
            log_lines: vec![].into(),
            audit_log_lines: vec![].into(),
            prev_journal: journal,
            next_journal: QueryJournal::new(),

            pending_syscalls: WithHeapSize::default(),
            syscall_trace: SyscallTrace::new(),
            heap_stats,
            context,

            reactor_depth,
            deployment,
            client_id,
        }
    }

    /// Runs a top-level query or mutation.
    #[fastrace::trace]
    pub async fn run(
        self,
        client_id: String,
        isolate: &mut Isolate<RT>,
        v8_context: v8::Global<v8::Context>,
        isolate_clean: &mut bool,
        cancellation: BoxFuture<'_, ()>,
        rng_seed: [u8; 32],
        unix_timestamp: UnixTimestamp,
        function_started: Option<oneshot::Sender<()>>,
        udf_callback: Option<IsolateClient<RT>>,
    ) -> anyhow::Result<(Transaction<RT>, FunctionOutcome)> {
        let executor = UdfRecursiveExecutor::new();

        let client_id = Arc::new(client_id);
        let (handle, state, mut timeout) = isolate.start_request(client_id, self).await?;
        let heap_stats = state.environment.heap_stats.clone();
        let path_for_logging = format!("{:?}", state.environment.path.clone().for_logging());
        if let Some(tx) = function_started {
            // At this point we have acquired a permit and aren't going to
            // reject the function for capacity reasons.
            _ = tx.send(());
        }
        let (this, mut result) = executor
            .run_until(Self::run_nested(
                &executor,
                rng_seed,
                unix_timestamp,
                isolate.isolate(),
                Some(v8_context),
                handle.clone(),
                state,
                &mut timeout,
                isolate_clean,
                cancellation,
                udf_callback,
            ))
            .await?;

        // Override the top-level result if there was an isolate termination error.
        // Our environment may be in an inconsistent state after a system error (e.g.
        // the transaction may be missing if we hit a system error during a
        // cross-component call), so be sure to error out here before using the
        // environment.
        match handle.take_termination_error(Some(heap_stats.get()), &path_for_logging) {
            Ok(Ok(())) => (),
            Ok(Err(e)) => result = Ok(Err(e)),
            Err(e) => result = Err(e),
        }
        let result = result?;

        let execution_time = timeout.into_function_execution_time(this.udf_type);
        let user_execution_time = execution_time.elapsed;

        let success_result_value = result.as_ref().ok();
        let parsed_args = parse_udf_args(&this.path.udf_path, this.arguments.clone().into_args()?)?;
        let mut log_lines = this.log_lines;
        Self::add_warnings_to_log_lines(
            &this.path.clone().for_logging(),
            &parsed_args,
            execution_time,
            this.phase.execution_size()?,
            this.phase.biggest_document_writes()?,
            success_result_value,
            |warning| {
                // Note: accessing the current time here is still deterministic since
                // we don't externalize the time to the function.
                log_lines.push(warning.into_log_line(this.rt.unix_timestamp()));
            },
        )?;
        let memory_in_mb = (*ISOLATE_MAX_USER_HEAP_SIZE / (1 << 20))
            .try_into()
            .unwrap();
        // TODO: Add num_writes and write_bandwidth to UdfOutcome,
        // and use them in log_mutation.
        let outcome = UdfOutcome {
            path: this.path.for_logging(),
            arguments: this.arguments,
            identity: this.identity,
            observed_identity: this.phase.observed_identity(),
            rng_seed,
            observed_rng: this.phase.observed_rng(),
            unix_timestamp,
            observed_time: this.phase.observed_time(),
            log_lines,
            audit_log_lines: this.audit_log_lines,
            journal: this.next_journal,
            result: match result {
                Ok(v) => Ok(JsonPackedValue::pack(v)),
                Err(e) => Err(e),
            },
            syscall_trace: this.syscall_trace,
            udf_server_version: this.udf_server_version,
            memory_in_mb,
            user_execution_time: Some(user_execution_time),
        };
        let outcome = match this.udf_type {
            UdfType::Query => FunctionOutcome::Query(outcome),
            UdfType::Mutation => FunctionOutcome::Mutation(outcome),
            _ => anyhow::bail!("UdfEnvironment should only run queries and mutations"),
        };
        Ok((this.phase.into_transaction()?, outcome))
    }

    /// Runs a query or mutation, possibly nested via `runQuery`/`runMutation`.
    async fn run_nested(
        executor: &UdfRecursiveExecutor<RT>,
        rng_seed: [u8; 32],
        unix_timestamp: UnixTimestamp,
        isolate: &mut v8::Isolate,
        v8_context: Option<v8::Global<v8::Context>>,
        isolate_handle: IsolateHandle,
        request_state: RequestState<RT, Self>,
        timeout: &mut Timeout<RT>,
        isolate_clean: &mut bool,
        cancellation: BoxFuture<'_, ()>,
        udf_callback: Option<IsolateClient<RT>>,
    ) -> anyhow::Result<(Self, anyhow::Result<Result<ConvexValue, JsError>>)> {
        scope!(let handle_scope, isolate);
        let v8_context = if let Some(context) = v8_context {
            v8::Local::new(handle_scope, context)
        } else {
            v8::Context::new(handle_scope, v8::ContextOptions::default())
        };
        let context_scope = &mut v8::ContextScope::new(handle_scope, v8_context);

        let mut isolate_context =
            RequestScope::new(context_scope, isolate_handle.clone(), request_state, false).await?;
        let mut result = Self::run_inner(
            executor,
            &mut isolate_context,
            timeout,
            cancellation,
            rng_seed,
            unix_timestamp,
            udf_callback,
        )
        .await;

        // Perform a microtask checkpoint one last time before taking the environment
        // to ensure the microtask queue is empty. Otherwise, JS from this request may
        // leak to a subsequent one on isolate reuse.
        isolate_context.checkpoint();
        *isolate_clean = true;

        let request_state = isolate_context.take_state().context("Lost RequestState?")?;
        let this = request_state.environment;
        // Override the returned result if we hit a termination error.
        match isolate_handle.pop_context(request_state.context_id)? {
            Ok(()) => (),
            Err(e) => result = Ok(Err(e)),
        }

        Ok((this, result))
    }

    #[convex_macro::instrument_future]
    #[fastrace::trace]
    async fn run_inner(
        executor: &UdfRecursiveExecutor<RT>,
        isolate: &mut RequestScope<'_, '_, '_, RT, Self>,
        timeout: &mut Timeout<RT>,
        cancellation: BoxFuture<'_, ()>,
        rng_seed: [u8; 32],
        unix_timestamp: UnixTimestamp,
        udf_callback: Option<IsolateClient<RT>>,
    ) -> anyhow::Result<Result<ConvexValue, JsError>> {
        let handle = isolate.handle();
        scope!(let v8_scope, isolate.scope());

        let mut scope = RequestScope::<RT, Self>::enter(v8_scope);

        // Initialize the environment, preloading the UDF config, before executing any
        // JS.
        {
            let state = scope.state_mut()?;
            state.environment.phase.initialize(timeout).await?;
        }

        let (rt, udf_type, path, udf_args, heap_stats) = {
            let state = scope.state()?;
            let environment = &state.environment;
            (
                environment.rt.clone(),
                environment.udf_type,
                environment.path.clone(),
                environment.arguments.clone(),
                environment.heap_stats.clone(),
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
            .eval_user_module(udf_type, false, &module_specifier, timeout)
            .await?
        {
            Ok(id) => id,
            Err(e) => return Ok(Err(e)),
        };
        let namespace = module
            .get_module_namespace()
            .to_object(&scope)
            .ok_or_else(|| anyhow!("Module namespace wasn't an object?"))?;
        let function_name = udf_path.function_name();
        let function_str: v8::Local<'_, v8::Value> = v8::String::new(&scope, function_name)
            .ok_or_else(|| anyhow!("Failed to create function name string"))?
            .into();

        if namespace.has(&scope, function_str) != Some(true) {
            let message = format!(
                "{}",
                FunctionNotFoundError::new(udf_path.function_name(), udf_path.module().as_str())
            );
            return Ok(Err(JsError::from_message(message)));
        }
        let function: v8::Local<v8::Object> = namespace
            .get(&scope, function_str)
            .ok_or_else(|| anyhow!("Did not find function in module after checking?"))?
            .try_into()?;

        // Mutations and queries are wrapped in JavaScript by a function that adds a
        // property marking it as a query or mutation UDF.
        let is_mutation_str = strings::isMutation.create(&scope)?.into();
        let mut is_mutation = false;
        if let Some(true) = function.has(&scope, is_mutation_str) {
            is_mutation = function
                .get(&scope, is_mutation_str)
                .ok_or_else(|| anyhow!("Missing `is_mutation` after explicit check"))?
                .is_true();
        }

        let is_query_str = strings::isQuery.create(&scope)?.into();
        let mut is_query = false;
        if let Some(true) = function.has(&scope, is_query_str) {
            is_query = function
                .get(&scope, is_query_str)
                .ok_or_else(|| anyhow!("Missing `is_query` after explicit check"))?
                .is_true();
        }
        let invoke_query_str = strings::invokeQuery.create(&scope)?.into();
        let invoke_mutation_str = strings::invokeMutation.create(&scope)?.into();

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

        let args_str = udf_args.get();
        metrics::log_argument_length(args_str);
        let args_v8_str = v8::String::new(&scope, args_str)
            .ok_or_else(|| anyhow!("Failed to create argument string"))?;

        let invoke: v8::Local<v8::Function> = function
            .get(&scope, invoke_str)
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
        let global = scope.get_current_context().global(&scope);
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
        let mut cancellation = cancellation;
        loop {
            // Advance the user's promise as far as it can go by draining the microtask
            // queue.
            scope.perform_microtask_checkpoint();
            pump_message_loop(&scope);
            scope.record_heap_stats()?;
            handle.check_terminated()?;

            // Check for rejected promises still unhandled, if so terminate.
            let rejections = scope.pending_unhandled_promise_rejections_mut();
            if let Some(promise) = rejections.exceptions.keys().next().cloned() {
                let error = rejections.exceptions.remove(&promise).unwrap();

                let as_local = v8::Local::new(&scope, error);
                let err = match scope.format_traceback(as_local) {
                    Ok(e) => e,
                    Err(e) => {
                        handle.terminate_and_throw(
                            IsolateTerminationReason::SystemError(Some(e)).into(),
                        )?;
                    },
                };
                handle.terminate_and_throw(
                    ContextTerminationReason::UnhandledPromiseRejection(err).into(),
                )?;
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
                let state = scope.take_state()?;
                let mut guard = scopeguard::guard((state, &mut scope), |(state, scope)| {
                    if let Err(mut e) = scope.return_state(state) {
                        report_error_sync(&mut e);
                    }
                });
                let (ref mut state, ref mut scope) = *guard;
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
                let results = timeout
                    .with_release_permit_regainable(
                        PauseReason::DatabaseSyscall {
                            name: batch.name().to_string(),
                        },
                        async |paused_timeout| {
                            let run_udf = RunUdf {
                                rt: &rt,
                                v8_scope: scope,
                                paused_timeout,
                                isolate_handle: &handle,
                                executor,
                                heap_stats: &heap_stats,
                            };
                            let udf_callback = if let Some(callback) = &udf_callback {
                                Either::Left(callback)
                            } else {
                                Either::Right(run_udf)
                            };
                            select! {
                                biased;
                                _ = &mut cancellation => {
                                    log_isolate_request_cancelled();
                                    anyhow::bail!("Cancelled");
                                },
                                results = DatabaseSyscallsV1::run_async_syscall_batch(
                                    &mut state.environment, batch, udf_callback,
                                ) => Ok(results),
                            }
                        },
                    )
                    .await?;
                (resolvers, results)
            };
            // Every syscall must have a result (which could be an error or None).
            assert_eq!(resolvers.len(), results.len());

            // Complete the syscall's promise, which will put its handlers on the microtask
            // queue.
            for (resolver, result) in resolvers.into_iter().zip(results.into_iter()) {
                scope!(let result_scope, &mut *scope);
                let result_v8 = match result {
                    Ok(v) => Ok(serde_v8::to_v8(result_scope, v)?),
                    Err(e) => Err(e),
                };
                resolve_promise(result_scope, resolver, result_v8)?;
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
                let promise_result_v8 = promise.result(&scope);
                let result_v8_str: v8::Local<v8::String> = promise_result_v8.try_into()?;
                let result_str = helpers::to_rust_string(&scope, &result_v8_str)?;
                metrics::log_result_length(&result_str);
                deserialize_udf_result(&path, &result_str)?
            },
            v8::PromiseState::Rejected => {
                let e = promise.result(&scope);
                Err(scope.format_traceback(e)?)
            },
        };

        Ok(result)
    }

    pub fn emit_audit_log_line(&mut self, audit_log_line: AuditLogLine) -> anyhow::Result<()> {
        let max_heap = *AUDIT_LOG_MAX_HEAP_SIZE_BYTES;
        anyhow::ensure!(
            self.audit_log_lines.heap_size() + audit_log_line.heap_size() <= max_heap,
            ErrorMetadata::bad_request(
                "AuditLogsExceedLimits",
                "Audit logs exceed function execution limits",
            )
        );
        self.audit_log_lines.push(audit_log_line);
        Ok(())
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

    pub fn emit_sub_function_log_lines(
        &mut self,
        path: CanonicalizedComponentFunctionPath,
        log_lines: LogLines,
    ) {
        // -1 to reserve for the [ERROR] log line
        if self.log_lines.len() > MAX_LOG_LINES - 1 {
            // We have previously exceeded the logging limit, so skip these logs.
            return;
        }
        if self.log_lines.len() + log_lines.len() > MAX_LOG_LINES - 1 {
            // We are about to exceed the logging limit, so truncate the logs.
            let allowed_length = MAX_LOG_LINES - 1 - self.log_lines.len();
            self.log_lines.push(LogLine::SubFunction {
                path,
                log_lines: log_lines.truncated(allowed_length),
            });
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
        } else {
            self.log_lines
                .push(LogLine::SubFunction { path, log_lines });
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
        let udf_path = path.udf_path.clone();
        let system_udf_path = if udf_path.is_system() {
            Some(udf_path)
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
        ) {
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
        ) {
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
        ) {
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
        ) {
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
        ) {
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
        ) {
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
        ) {
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
        ) {
            trace_system_warning(warning);
        }
        if let Some(warning) = scheduled_arg_size_warning(
            execution_size.scheduled_size.max_args_size,
            &system_udf_path,
        ) {
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
            ) {
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
            ) {
                trace_system_warning(warning);
            }
        }

        if let Some(result) = result
            && let Some(warning) = approaching_limit_warning(
                result.size(),
                *FUNCTION_MAX_RESULT_SIZE,
                "TooLargeFunctionResult",
                || "Large size of the function return value".to_string(),
                None,
                Some(" bytes"),
                system_udf_path.as_ref(),
            )
        {
            trace_system_warning(warning);
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

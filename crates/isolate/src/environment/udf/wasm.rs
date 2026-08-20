//! Running a query or mutation in the wasm runtime instead of V8.
//!
//! This is the non-V8 half of [`DatabaseUdfEnvironment::run`]. Everything that
//! makes a Convex function a Convex function — the syscalls, the transaction,
//! the journal, the log lines, the outcome — is the same code the V8 path uses;
//! only the thing evaluating the JavaScript changes. What this module supplies
//! is the glue: the module tree in a form the wasm runtime can bundle, a
//! [`ConvexSyscallHost`] over [`DatabaseUdfSyscallProvider`], and the
//! translation of guest results back into a [`FunctionOutcome`].
//!
//! Compiling is deliberately not part of the timed request. A cold compile runs
//! esbuild, cargo, and Wizer, which takes minutes and would blow the system
//! timeout many times over, so callers load the sources and build the artifact
//! before starting the request and hand the bytes in.

use std::{
    collections::BTreeMap,
    sync::Arc,
};

use anyhow::Context as _;
use common::{
    components::{
        ComponentId,
        ResolvedComponentFunctionPath,
    },
    errors::JsError,
    log_lines::LogLevel,
    runtime::Runtime,
    types::{
        ModuleEnvironment,
        UdfType,
    },
};
use database::Transaction;
use errors::ErrorMetadataAnyhowExt;
use model::{
    modules::{
        user_error::FunctionNotFoundError,
        ModuleModel,
    },
    source_packages::SourcePackageModel,
};
use rand::Rng as _;
use serde_json::Value as JsonValue;
use udf::FunctionOutcome;
use wasm_runtime::udf::{
    run_udf,
    ConvexSyscallHost,
    PendingSyscall as WasmPendingSyscall,
    SyscallOutcome,
    UdfError,
    UdfTypeTag,
};

use super::{
    async_syscall::{
        run_async_syscall_batch,
        AsyncSyscallBatch,
    },
    syscall::syscall_impl,
    DatabaseUdfArgs,
    DatabaseUdfEnvironment,
    DatabaseUdfInnerProvider,
    DatabaseUdfSyscallProvider,
};
use crate::{
    client::{
        UdfCallback,
        UdfRequest,
    },
    environment::SyscallProvider,
    helpers::deserialize_udf_result_pending,
    module_cache::ModuleCache,
    timeout::start_cooperative_request,
    ConcurrencyPermit,
};

/// Every isolate module in `component`, as plain source text keyed by
/// deployment-relative path (`messages.js`, `_deps/abc.js`).
///
/// V8 pulls modules in one at a time as it walks the import graph. A wasm
/// artifact is bundled before anything is evaluated, so it needs the whole tree
/// up front — which costs no extra downloads, since every module comes out of
/// the same source package.
pub async fn load_module_sources<RT: Runtime>(
    tx: &mut Transaction<RT>,
    module_loader: &Arc<dyn ModuleCache<RT>>,
    component: ComponentId,
) -> anyhow::Result<BTreeMap<String, String>> {
    let metadata = ModuleModel::new(tx)
        .get_application_metadata(component)
        .await?;

    let mut sources = BTreeMap::new();
    for module_metadata in metadata {
        if module_metadata.environment != ModuleEnvironment::Isolate {
            continue;
        }
        let source_package = SourcePackageModel::new(tx, component.into())
            .get(module_metadata.source_package_id)
            .await?;
        let source = module_loader
            .get_module_with_metadata(&module_metadata, &source_package)
            .await?;
        sources.insert(
            module_metadata.path.as_str().to_owned(),
            source.source().to_utf8(),
        );
    }
    Ok(sources)
}

/// Run a query or mutation from `artifact`, which must have been compiled from
/// the same module tree this transaction would load.
pub async fn run_in_wasm<RT: Runtime>(
    environment: DatabaseUdfEnvironment<RT>,
    permit: ConcurrencyPermit,
    args: DatabaseUdfArgs,
    artifact: &[u8],
) -> anyhow::Result<(Transaction<RT>, FunctionOutcome)> {
    let rt = environment.syscall_provider.rt.clone();
    let (handle, context_id, mut timeout) =
        start_cooperative_request(rt, permit, &environment, None);
    let mut provider = environment.syscall_provider;

    let udf_type = provider.udf_type;
    let path = provider.path.clone();
    let udf_type_tag = match udf_type {
        UdfType::Query => UdfTypeTag::Query,
        UdfType::Mutation => UdfTypeTag::Mutation,
        UdfType::Action | UdfType::HttpAction => {
            anyhow::bail!("The wasm runtime only runs queries and mutations, not {udf_type}")
        },
    };

    provider.initialize(&mut timeout).await?;
    provider.begin_execution(args.rng_seed, args.unix_timestamp)?;

    let function_name = path.udf_path.function_name().to_owned();
    let (host, guest_result) = run_udf(
        artifact,
        udf_type_tag,
        &function_name,
        args.udf_args.get(),
        WasmSyscallHost { provider },
    )
    .await?;
    let provider = host.provider;

    let mut result = match guest_result {
        Ok(result_str) => deserialize_udf_result_pending(&path, &result_str)?,
        Err(error) => Err(describe(error, &path, udf_type)),
    };

    // A syscall that hit a system error terminates the request even if JS
    // carried on and produced a value, matching `handle.check_terminated()` on
    // the V8 path.
    match handle.pop_context(context_id)? {
        Ok(()) => (),
        Err(e) => result = Err(e),
    }

    let execution_time = timeout.into_function_execution_time(udf_type);
    provider.into_outcome(args, result, execution_time)
}

/// Turn a guest-side failure into the same message the V8 path would produce.
fn describe(error: UdfError, path: &ResolvedComponentFunctionPath, udf_type: UdfType) -> JsError {
    let udf_path = &path.udf_path;
    let message = match error {
        UdfError::FunctionNotFound { .. } => format!(
            "{}",
            FunctionNotFoundError::new(udf_path.function_name(), udf_path.module().as_str())
        ),
        UdfError::FunctionType {
            is_query,
            is_mutation,
        } => match (udf_type, is_query, is_mutation) {
            (_, false, false) => format!(
                "Function {udf_path:?} is neither a query or mutation. Did you forget to wrap it \
                 with `query` or `mutation`?"
            ),
            (UdfType::Query, false, true) => format!(
                "Function {udf_path:?} is registered as a mutation but is being run as a query."
            ),
            (UdfType::Mutation, true, false) => format!(
                "Function {udf_path:?} is registered as a query but is being run as a mutation."
            ),
            _ => format!("Function {udf_path:?} cannot run as a {udf_type}"),
        },
        UdfError::Handler { message, .. } => message,
    };
    JsError::from_message(message)
}

struct WasmSyscallHost<RT: Runtime> {
    provider: DatabaseUdfSyscallProvider<RT>,
}

impl<RT: Runtime> WasmSyscallHost<RT> {
    /// Split a failure the way `resolve_promise` does: a deterministic user
    /// error becomes something the function can catch, anything else is ours
    /// and aborts the request.
    fn classify(error: anyhow::Error) -> anyhow::Result<SyscallOutcome> {
        if error.is_deterministic_user_error() {
            Ok(Err(error.user_facing_message()))
        } else {
            Err(error)
        }
    }
}

impl<RT: Runtime> WasmSyscallHost<RT> {
    /// The slice of the V8 ops layer the bundled `convex` package needs before
    /// any user code runs. The full ops layer takes V8 values; these two are
    /// plain JSON, so they can be served without it.
    fn op(&mut self, name: &str, args: JsonValue) -> anyhow::Result<Option<JsonValue>> {
        let value = match name {
            "op/random" => {
                let value: f64 = self.provider.rng()?.random();
                JsonValue::from(value)
            },
            "op/environmentVariables/get" => {
                let name = args
                    .get(0)
                    .and_then(JsonValue::as_str)
                    .context("environment variable name should be a string")?
                    .parse()?;
                match self.provider.get_environment_variable(name)? {
                    Some(value) => JsonValue::from(value.to_string()),
                    None => JsonValue::Null,
                }
            },
            _ => return Ok(None),
        };
        Ok(Some(value))
    }
}

impl<RT: Runtime> ConvexSyscallHost for WasmSyscallHost<RT> {
    fn syscall(&mut self, name: &str, args_json: &str) -> anyhow::Result<SyscallOutcome> {
        let args: JsonValue =
            serde_json::from_str(args_json).context("syscall args should be JSON")?;

        if name.starts_with("op/") {
            return match self.op(name, args) {
                Ok(Some(value)) => Ok(Ok(serde_json::to_string(&value)?)),
                Ok(None) => Ok(Err(format!(
                    "The op `{name}` is not implemented in the wasm runtime yet"
                ))),
                Err(error) => Self::classify(error),
            };
        }

        match syscall_impl(&mut self.provider, name, args) {
            Ok(value) => Ok(Ok(serde_json::to_string(&value)?)),
            Err(error) => Self::classify(error),
        }
    }

    async fn async_syscalls(
        &mut self,
        pending: &[WasmPendingSyscall],
    ) -> anyhow::Result<Vec<(i32, SyscallOutcome)>> {
        let (first, rest) = pending
            .split_first()
            .context("asked to run an empty syscall batch")?;

        // Same batching rule as the V8 path: take from the front for as long as
        // the batch will accept them, so a `Promise.all` of reads collapses into
        // one round trip and everything else runs on its own.
        let mut batch = AsyncSyscallBatch::new(first.name.clone(), parse_args(&first.args)?);
        let mut op_ids = vec![first.op_id];
        for syscall in rest {
            let args = parse_args(&syscall.args)?;
            if !batch.can_push(&syscall.name, &args) {
                break;
            }
            batch.push(syscall.name.clone(), args)?;
            op_ids.push(syscall.op_id);
        }

        let results = run_async_syscall_batch(&mut self.provider, batch, NoNestedUdfs).await;
        anyhow::ensure!(
            op_ids.len() == results.len(),
            "batch of {} syscalls produced {} results",
            op_ids.len(),
            results.len()
        );

        op_ids
            .into_iter()
            .zip(results)
            .map(|(op_id, result)| {
                let outcome = match result {
                    Ok(value) => Ok(value),
                    Err(error) => Self::classify(error)?,
                };
                Ok((op_id, outcome))
            })
            .collect()
    }

    fn trace(&mut self, level: &str, messages: Vec<String>) -> anyhow::Result<()> {
        let level = match level {
            "debug" => LogLevel::Debug,
            "error" => LogLevel::Error,
            "warn" => LogLevel::Warn,
            "info" => LogLevel::Info,
            _ => LogLevel::Log,
        };
        SyscallProvider::trace(&mut self.provider, level, messages)
    }
}

fn parse_args(args_json: &str) -> anyhow::Result<JsonValue> {
    serde_json::from_str(args_json).context("async syscall args should be JSON")
}

/// Nested `runQuery`/`runMutation` re-enter the engine, which the wasm path
/// cannot do yet: it would need a second instance and a nested transaction.
struct NoNestedUdfs;

impl<RT: Runtime> UdfCallback<RT> for NoNestedUdfs {
    async fn execute_nested_udf(
        self,
        _client_id: String,
        _udf_request: UdfRequest<RT>,
        _rng_seed: [u8; 32],
        _reactor_depth: usize,
    ) -> anyhow::Result<(Transaction<RT>, udf::NestedUdfOutcome)> {
        anyhow::bail!("Calling another function is not supported in the wasm runtime yet")
    }
}

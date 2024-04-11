use std::{
    collections::BTreeMap,
    path::Path,
    sync::{
        Arc,
        LazyLock,
    },
    time::Duration,
};

use async_trait::async_trait;
use common::{
    errors::{
        FrameData,
        JsError,
    },
    execution_context::ExecutionContext,
    log_lines::LogLine,
    sha256::Sha256Digest,
    types::{
        ActionCallbackToken,
        ConvexOrigin,
        NodeDependency,
        ObjectKey,
        UdfType,
    },
};
use futures::channel::mpsc;
use http::Uri;
use isolate::{
    deserialize_udf_custom_error,
    deserialize_udf_result,
    format_uncaught_error,
    serialize_udf_args,
    SyscallStats,
    SyscallTrace,
    ValidatedUdfPathAndArgs,
};
use model::{
    environment_variables::types::{
        EnvVarName,
        EnvVarValue,
        EnvironmentVariable,
        PersistedEnvironmentVariable,
    },
    modules::{
        args_validator::ArgsValidator,
        module_versions::{
            AnalyzedFunction,
            AnalyzedModule,
            AnalyzedSourcePosition,
            FunctionName,
            MappedModule,
            SourceMap,
            Visibility,
        },
    },
    source_packages::types::{
        PackageSize,
        SourcePackageId,
    },
};
use serde::Deserialize;
use serde_json::{
    json,
    Value as JsonValue,
};
use sync_types::{
    CanonicalizedModulePath,
    UserIdentityAttributes,
};
use value::{
    base64,
    heap_size::WithHeapSize,
    ConvexObject,
    ConvexValue,
};

use crate::metrics::{
    log_download_time,
    log_external_deps_size_bytes_total,
    log_function_execution,
    log_import_time,
    log_node_source_map_missing,
    log_node_source_map_token_lookup_failed,
    log_overhead,
    log_total_executor_time,
    log_udf_time,
    node_executor,
};

pub fn error_response_json(message: &str) -> JsonValue {
    json!({
        "type": "error",
        "message": message,
    })
}

pub static EXECUTE_TIMEOUT_RESPONSE_JSON: LazyLock<JsonValue> = LazyLock::new(|| {
    error_response_json(
        "Function execution unexpectedly timed out. Check your function for infinite loops or \
         other long-running operations.",
    )
});

#[async_trait]
pub trait NodeExecutor: Sync + Send {
    fn enable(&self) -> anyhow::Result<()>;
    async fn invoke(
        &self,
        request: ExecutorRequest,
        log_line_sender: mpsc::UnboundedSender<LogLine>,
    ) -> anyhow::Result<InvokeResponse>;
    fn shutdown(&self);
}

pub struct InvokeResponse {
    pub response: JsonValue,
    pub memory_used_in_mb: u64,
    pub aws_request_id: Option<String>,
}

#[derive(Clone)]
pub struct Actions {
    executor: Arc<dyn NodeExecutor>,
    convex_origin: ConvexOrigin,
    user_timeout: Duration,
}

fn construct_js_error(
    error_message: String,
    error_name: String,
    serialized_custom_data: Option<String>,
    frames: Option<Vec<FrameData>>,
    source_maps: &BTreeMap<CanonicalizedModulePath, SourceMap>,
) -> anyhow::Result<JsError> {
    // Only format the error message if we have frames,
    // as errors without frames come from outside the
    // action execution.
    let message = if frames.is_some() {
        format_uncaught_error(error_message, error_name)
    } else {
        error_message
    };
    let (message, custom_data) = deserialize_udf_custom_error(message, serialized_custom_data)?;
    let error = match frames {
        Some(mut frames) => {
            // Discard frames that aren't in the convex scheme, which may correspond
            // either to our node-executor's source or node's main.
            frames.retain(|frame| match frame.file_name {
                Some(ref f) => f.starts_with("convex:/"),
                None => false,
            });
            JsError::from_frames(message, frames, custom_data, |specifier| {
                if !specifier.as_str().starts_with("convex:/") {
                    return Ok(None);
                }
                let Some(path) = specifier.path().strip_prefix("/user/") else {
                    return Ok(None);
                };
                let module_path: CanonicalizedModulePath = path.parse()?;
                let Some(source_map) = source_maps.get(&module_path) else {
                    return Ok(None);
                };
                Ok(Some(sourcemap::SourceMap::from_slice(
                    source_map.as_bytes(),
                )?))
            })?
        },
        None => JsError::from_message(message),
    };
    Ok(error)
}

impl Actions {
    pub fn new(
        executor: Arc<dyn NodeExecutor>,
        convex_origin: ConvexOrigin,
        user_timeout: Duration,
    ) -> Self {
        Self {
            executor,
            convex_origin,
            user_timeout,
        }
    }

    pub fn enable(&self) -> anyhow::Result<()> {
        self.executor.enable()
    }

    pub fn shutdown(&self) {
        self.executor.shutdown()
    }

    pub async fn execute(
        &self,
        request: ExecuteRequest,
        source_maps: &BTreeMap<CanonicalizedModulePath, SourceMap>,
        log_line_sender: mpsc::UnboundedSender<LogLine>,
    ) -> anyhow::Result<NodeActionOutcome> {
        let path = request.path_and_args.udf_path().clone();
        let timer = node_executor("execute");
        let request = ExecutorRequest::Execute {
            request,
            backend_address: self.convex_origin.clone(),
            // Use the user facing timeout here, which should be less than the
            // total Node timeout. This allows us to preempt early and give
            // better error message and logs in the common case.
            timeout: self.user_timeout,
        };
        let InvokeResponse {
            response,
            memory_used_in_mb,
            aws_request_id,
        } = self.executor.invoke(request, log_line_sender).await?;
        let execute_result = ExecuteResponse::try_from(response.clone()).map_err(|e| {
            anyhow::anyhow!(
                "Failed to deserialize execute response: {}. Response: {}",
                e.to_string(),
                response
            )
        })?;

        tracing::info!(
            "Total:{:?}, executor:{:?}, download:{:?}, import:{:?}, udf:{:?}, \
             env_invocations:{:?}, aws_request_id:{:?}",
            timer.elapsed(),
            execute_result.total_executor_time,
            execute_result.download_time,
            execute_result.import_time,
            execute_result.udf_time,
            execute_result.num_invocations,
            aws_request_id,
        );
        let total_time = timer.finish();
        if let Some(download_time) = execute_result.download_time {
            log_download_time(download_time);
        }
        if let Some(import_time) = execute_result.import_time {
            log_import_time(import_time);
        }
        if let Some(udf_time) = execute_result.udf_time {
            log_udf_time(udf_time);
            if total_time > udf_time {
                log_overhead(total_time - udf_time);
            } else {
                log_overhead(Duration::new(0, 0));
            }
        }
        if let Some(total_executor_time) = execute_result.total_executor_time {
            log_total_executor_time(total_executor_time);
        }
        let cold_start = execute_result.num_invocations.map(|n| n == 1);
        log_function_execution(cold_start);

        let syscall_trace = execute_result.syscall_trace;

        let result = match execute_result.result {
            ExecuteResponseResult::Success { udf_return, .. } => {
                deserialize_udf_result(&path, &udf_return)?
            },
            ExecuteResponseResult::Error {
                message,
                name,
                data,
                frames,
                ..
            } => {
                let error = construct_js_error(message, name, data, frames, source_maps)?;
                Err(error)
            },
        };
        Ok(NodeActionOutcome {
            result,
            syscall_trace,
            memory_used_in_mb,
        })
    }

    pub async fn build_deps(
        &self,
        request: BuildDepsRequest,
    ) -> anyhow::Result<Result<(Sha256Digest, PackageSize), JsError>> {
        let timer = node_executor("build_deps");
        let (log_line_sender, _log_line_receiver) = mpsc::unbounded();
        let request = ExecutorRequest::BuildDeps(request);
        let InvokeResponse {
            response,
            memory_used_in_mb: _,
            aws_request_id,
        } = self.executor.invoke(request, log_line_sender).await?;
        let response: BuildDepsResponse =
            serde_json::from_value(response.clone()).map_err(|e| {
                anyhow::anyhow!(
                    "Failed to deserialize build_deps response: {}. Response: {}",
                    e.to_string(),
                    response,
                )
            })?;

        let result = match response {
            BuildDepsResponse::Success {
                sha256_digest,
                zipped_size_bytes,
                unzipped_size_bytes,
            } => {
                let pkg_size = PackageSize {
                    zipped_size_bytes,
                    unzipped_size_bytes,
                };
                tracing::info!("External deps package size: {}", pkg_size);
                log_external_deps_size_bytes_total(pkg_size);

                Ok(Ok((Sha256Digest::from(sha256_digest), pkg_size)))
            },
            BuildDepsResponse::Error { message, frames } => {
                if let Some(frames) = frames {
                    Ok(Err(JsError::from_frames(message, frames, None, |_| {
                        Ok(None)
                    })?))
                } else {
                    Ok(Err(JsError::from_message(message)))
                }
            },
        };

        tracing::info!(
            "build_deps took {:?}. aws_request_id={:?}",
            timer.elapsed(),
            aws_request_id
        );
        timer.finish();

        result
    }

    #[minitrace::trace]
    pub async fn analyze(
        &self,
        request: AnalyzeRequest,
        source_maps: &BTreeMap<CanonicalizedModulePath, SourceMap>,
    ) -> anyhow::Result<Result<BTreeMap<CanonicalizedModulePath, AnalyzedModule>, JsError>> {
        let timer = node_executor("analyze");

        let (log_line_sender, _log_line_receiver) = mpsc::unbounded();
        let request = ExecutorRequest::Analyze(request);
        let InvokeResponse {
            response,
            memory_used_in_mb: _,
            aws_request_id,
        } = self.executor.invoke(request, log_line_sender).await?;
        let response: AnalyzeResponse = serde_json::from_value(response.clone()).map_err(|e| {
            anyhow::anyhow!(
                "Failed to deserialize analyze response: {}. Response: {}",
                e.to_string(),
                response
            )
        })?;
        tracing::info!(
            "Analyze took {:?}. aws_request_id={:?}",
            timer.elapsed(),
            aws_request_id
        );
        timer.finish();

        let modules = match response {
            AnalyzeResponse::Success { modules } => modules,
            AnalyzeResponse::Error { message, frames } => {
                let error = construct_js_error(message, "".to_string(), None, frames, source_maps)?;
                return Ok(Err(error));
            },
        };
        let mut result = BTreeMap::new();
        for (path, node_functions) in modules {
            let path: CanonicalizedModulePath = path.parse()?;
            // We have no concept of the origin of a Function in the Node environment, so
            // this logic just assumes the line number of the Function belongs to the
            // current module.

            // See if we have a source map that has a 'sources' field that matches the
            // current module path. We also extract the source_index which
            // corresponds to the index of the matching 'sources' field so that
            // we can access the source_contents field at the same index when
            // rebuilding the source.
            let mut source_map = None;
            let mut source_index = None;
            if let Some(buf) = source_maps.get(&path) {
                let candidate_source_map = sourcemap::SourceMap::from_slice(buf.as_bytes())?;
                for (i, filename) in candidate_source_map.sources().enumerate() {
                    if Path::new(filename).file_stem() != Path::new(path.as_str()).file_stem() {
                        continue;
                    }
                    if candidate_source_map.get_source_contents(i as u32).is_some() {
                        source_index = Some(i as u32);
                    }
                    source_map = Some(candidate_source_map);
                    break;
                }
            }

            // For each analyzed function, extract analyzed function and line numbers
            let mut functions = vec![];
            for f in node_functions {
                let udf_type = f.udf_type.as_str().parse()?;
                if udf_type != UdfType::Action {
                    return Ok(Err(JsError::from_message(format!(
                        "{} defined in {:?} is a {} function. Only \
                         actions can be defined in Node.js. See https://docs.convex.dev/functions/actions for more details.",
                        f.name, path, udf_type,
                    ))));
                }
                let args = match f.args.clone() {
                    Some(json_args) => match ArgsValidator::try_from(json_args) {
                        Ok(validator) => validator,
                        Err(parse_error) => {
                            let message =
                                format!("Unable to parse JSON from `exportArgs`: {parse_error}");
                            return Ok(Err(JsError::from_message(message)));
                        },
                    },
                    None => ArgsValidator::Unvalidated,
                };
                let visibility = f.visibility.clone().map(Visibility::from);

                // Extract source position
                let pos = if let Some(Some(token)) =
                    source_map.as_ref().map(|map| map.lookup_token(f.lineno, 0))
                {
                    Some(AnalyzedSourcePosition {
                        path: path.clone(),
                        start_lineno: token.get_src_line(),
                        start_col: token.get_src_col(),
                    })
                } else {
                    if source_map.is_none() {
                        log_node_source_map_missing();
                    } else {
                        log_node_source_map_token_lookup_failed();
                    }

                    None
                };
                let function_name = FunctionName::from_untrusted(&f.name)?;
                functions.push(AnalyzedFunction {
                    name: function_name,
                    pos,
                    udf_type,
                    visibility,
                    args,
                });
            }

            // Sort by line number where source position of None compares least
            functions.sort_by(|a, b| a.pos.cmp(&b.pos));

            let functions = WithHeapSize::from(functions);
            let module = AnalyzedModule {
                functions: functions.clone(),
                source_mapped: Some(MappedModule {
                    source_index,
                    functions,
                    http_routes: None,
                    cron_specs: None,
                }),
                http_routes: None,
                cron_specs: None,
            };
            result.insert(path, module);
        }
        Ok(Ok(result))
    }
}

pub enum ExecutorRequest {
    Execute {
        request: ExecuteRequest,
        backend_address: ConvexOrigin,
        timeout: Duration,
    },
    Analyze(AnalyzeRequest),
    BuildDeps(BuildDepsRequest),
}

impl TryFrom<ExecutorRequest> for JsonValue {
    type Error = anyhow::Error;

    fn try_from(request: ExecutorRequest) -> anyhow::Result<Self> {
        let json = match request {
            ExecutorRequest::Execute {
                request: r,
                backend_address,
                timeout,
            } => {
                let environment_variables: Vec<JsonValue> = r
                    .environment_variables
                    .into_iter()
                    .map(|(name, value)| {
                        ConvexObject::try_from(PersistedEnvironmentVariable(
                            EnvironmentVariable::new(name, value),
                        ))
                        .map(JsonValue::from)
                    })
                    .collect::<anyhow::Result<_>>()?;
                let (udf_path, args, npm_version) = r.path_and_args.consume();

                json!({
                    "type": "execute",
                    "udfPath": {
                        "canonicalizedPath": udf_path.module().as_str(),
                        "function": udf_path.function_name(),
                    },
                    // The executor expects the args to be a serialized string.
                    "args": serialize_udf_args(args)?,
                    "sourcePackage": JsonValue::from(r.source_package),
                    "backendAddress": backend_address,
                    "timeoutSecs": timeout.as_secs_f64(),
                    "backendCallbackToken": r.callback_token,
                    "authHeader": r.auth_header,
                    "userIdentity": r.user_identity.map(JsonValue::try_from).transpose()?,
                    "environmentVariables": JsonValue::Array(environment_variables),
                    "npmVersion": npm_version.map(|v| v.to_string()),
                    "executionContext": JsonValue::from(r.context),
                })
            },
            ExecutorRequest::Analyze(r) => {
                let environment_variables: Vec<JsonValue> = r
                    .environment_variables
                    .into_iter()
                    .map(|(name, value)| {
                        ConvexObject::try_from(PersistedEnvironmentVariable(
                            EnvironmentVariable::new(name, value),
                        ))
                        .map(JsonValue::from)
                    })
                    .collect::<anyhow::Result<_>>()?;
                json!({
                    "type": "analyze",
                    "sourcePackage": JsonValue::from(r.source_package),
                    "environmentVariables": JsonValue::Array(environment_variables),
                })
            },
            ExecutorRequest::BuildDeps(r) => {
                let deps: Vec<JsonValue> = r.deps.into_iter().map(JsonValue::from).collect();

                json!({
                    "type": "build_deps",
                    "uploadUrl": JsonValue::from(r.upload_url.to_string()),
                    "deps": JsonValue::Array(deps),
                })
            },
        };
        Ok(json)
    }
}

#[derive(Debug, Clone)]
pub struct SourcePackage {
    pub bundled_source: Package,

    // Info of external package if external dependencies were specified.
    pub external_deps: Option<Package>,
}

impl From<SourcePackage> for JsonValue {
    fn from(value: SourcePackage) -> Self {
        let source_package: JsonValue = value.bundled_source.clone().into();
        let external_package: JsonValue = value
            .external_deps
            .map(JsonValue::from)
            .unwrap_or(JsonValue::Null);

        json!({
            "uri": value.bundled_source.uri.to_string(),
            "key": String::from(value.bundled_source.key),
            "sha256": base64::encode_urlsafe(&*value.bundled_source.sha256),
            "bundled_source": source_package,
            "external_deps": external_package,
        })
    }
}

#[derive(Debug, Clone)]
pub struct Package {
    // Short-lived URI for fetching the source package, if desired.
    pub uri: Uri,

    // Stable key for caching the package.
    pub key: ObjectKey,

    // Checksum of the compressed package file.
    pub sha256: Sha256Digest,
}

impl From<Package> for JsonValue {
    fn from(value: Package) -> Self {
        // TODO: this is missing sha256 field
        json!({
            "uri": value.uri.to_string(),
            "key": String::from(value.key),
            "sha256": base64::encode_urlsafe(&*value.sha256),
        })
    }
}

#[derive(Debug)]
pub struct ExecuteRequest {
    // Note that the lambda executor expects arguments as string, which
    // then directly passes to invokeAction()
    pub path_and_args: ValidatedUdfPathAndArgs,

    pub source_package: SourcePackage,
    pub source_package_id: SourcePackageId,
    pub user_identity: Option<UserIdentityAttributes>,
    pub auth_header: Option<String>,
    pub environment_variables: BTreeMap<EnvVarName, EnvVarValue>,

    pub callback_token: ActionCallbackToken,
    pub context: ExecutionContext,
}

#[derive(Debug, PartialEq)]
struct ExecuteResponse {
    result: ExecuteResponseResult,
    num_invocations: Option<usize>,
    download_time: Option<Duration>,
    import_time: Option<Duration>,
    udf_time: Option<Duration>,
    total_executor_time: Option<Duration>,
    syscall_trace: SyscallTrace,
}

#[derive(Debug, PartialEq)]
enum ExecuteResponseResult {
    Success {
        udf_return: String,
    },
    Error {
        message: String,
        name: String,
        data: Option<String>,
        frames: Option<Vec<FrameData>>,
    },
}

#[derive(Debug)]
pub struct NodeActionOutcome {
    pub result: Result<ConvexValue, JsError>,
    pub syscall_trace: SyscallTrace,
    pub memory_used_in_mb: u64,
}

fn duration_from_millis_float(t: f64) -> Duration {
    Duration::from_micros((t * 1000.0) as u64)
}

impl TryFrom<JsonValue> for ExecuteResponse {
    type Error = anyhow::Error;

    fn try_from(v: JsonValue) -> anyhow::Result<Self> {
        #[derive(Deserialize, Debug, PartialEq)]
        #[serde(rename_all = "camelCase")]
        struct SyscallStatsJson {
            invocations: u32,
            errors: u32,
            total_duration_ms: f64,
        }
        impl From<SyscallStatsJson> for SyscallStats {
            fn from(value: SyscallStatsJson) -> Self {
                SyscallStats {
                    invocations: value.invocations,
                    errors: value.errors,
                    total_duration: duration_from_millis_float(value.total_duration_ms),
                }
            }
        }

        #[derive(Deserialize, Debug, PartialEq)]
        #[serde(tag = "type")]
        #[serde(rename_all = "camelCase")]
        enum ExecuteResponseJson {
            #[serde(rename_all = "camelCase")]
            Success {
                udf_return: String,
                num_invocations: usize,
                download_time_ms: Option<f64>,
                import_time_ms: Option<f64>,
                udf_time_ms: Option<f64>,
                total_executor_time_ms: Option<f64>,
                syscall_trace: Option<BTreeMap<String, SyscallStatsJson>>,
            },
            #[serde(rename_all = "camelCase")]
            Error {
                message: String,
                name: Option<String>,
                data: Option<String>,
                frames: Option<Vec<FrameData>>,
                num_invocations: Option<usize>,
                download_time_ms: Option<f64>,
                import_time_ms: Option<f64>,
                udf_time_ms: Option<f64>,
                total_executor_time_ms: Option<f64>,
                syscall_trace: Option<BTreeMap<String, SyscallStatsJson>>,
            },
        }
        let resp_json: ExecuteResponseJson = serde_json::from_value(v)?;
        let result = match resp_json {
            ExecuteResponseJson::Success {
                udf_return,
                num_invocations,
                download_time_ms,
                import_time_ms,
                udf_time_ms,
                total_executor_time_ms,
                syscall_trace,
            } => ExecuteResponse {
                result: ExecuteResponseResult::Success { udf_return },
                num_invocations: Some(num_invocations),
                download_time: download_time_ms.map(duration_from_millis_float),
                import_time: import_time_ms.map(duration_from_millis_float),
                udf_time: udf_time_ms.map(duration_from_millis_float),
                total_executor_time: total_executor_time_ms.map(duration_from_millis_float),
                syscall_trace: syscall_trace
                    .unwrap_or_default()
                    .into_iter()
                    .map(|(k, v)| (k, v.into()))
                    .collect::<BTreeMap<_, SyscallStats>>()
                    .into(),
            },
            ExecuteResponseJson::Error {
                message,
                name,
                data,
                frames,
                num_invocations,
                download_time_ms,
                import_time_ms,
                udf_time_ms,
                total_executor_time_ms,
                syscall_trace,
            } => ExecuteResponse {
                result: ExecuteResponseResult::Error {
                    message,
                    name: name.unwrap_or_default(),
                    data,
                    frames,
                },
                num_invocations,
                download_time: download_time_ms.map(duration_from_millis_float),
                import_time: import_time_ms.map(duration_from_millis_float),
                udf_time: udf_time_ms.map(duration_from_millis_float),
                total_executor_time: total_executor_time_ms.map(duration_from_millis_float),
                syscall_trace: syscall_trace
                    .unwrap_or_default()
                    .into_iter()
                    .map(|(k, v)| (k, v.into()))
                    .collect::<BTreeMap<_, SyscallStats>>()
                    .into(),
            },
        };
        Ok(result)
    }
}

#[derive(Debug)]
pub struct AnalyzeRequest {
    pub source_package: SourcePackage,
    pub environment_variables: BTreeMap<EnvVarName, EnvVarValue>,
}

#[derive(Debug, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
#[serde(tag = "type")]
pub enum AnalyzeResponse {
    #[serde(rename_all = "camelCase")]
    Success {
        modules: BTreeMap<String, Vec<AnalyzedNodeFunction>>,
    },
    #[serde(rename_all = "camelCase")]
    Error {
        message: String,
        frames: Option<Vec<FrameData>>,
    },
}

#[derive(Deserialize, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AnalyzedNodeFunction {
    name: String,
    lineno: u32,
    udf_type: String,
    visibility: Option<VisibilityJson>,
    args: Option<JsonValue>,
}

#[derive(Debug)]
pub struct BuildDepsRequest {
    pub deps: Vec<NodeDependency>,
    pub upload_url: Uri,
}

#[derive(Debug, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
#[serde(tag = "type")]
pub enum BuildDepsResponse {
    #[serde(rename_all = "camelCase")]
    Success {
        sha256_digest: [u8; 32],
        zipped_size_bytes: usize,
        unzipped_size_bytes: usize,
    },
    #[serde(rename_all = "camelCase")]
    Error {
        message: String,
        frames: Option<Vec<FrameData>>,
    },
}

#[derive(Deserialize, Debug, PartialEq, Clone)]
#[serde(tag = "kind")]
#[serde(rename_all = "camelCase")]
pub enum VisibilityJson {
    Public,
    Internal,
}

impl From<VisibilityJson> for Visibility {
    fn from(value: VisibilityJson) -> Self {
        match value {
            VisibilityJson::Public => Visibility::Public,
            VisibilityJson::Internal => Visibility::Internal,
        }
    }
}

pub enum ResponsePart {
    LogLine(LogLine),
    Result(JsonValue),
}

pub fn parse_streamed_response(s: &str) -> anyhow::Result<Vec<ResponsePart>> {
    let parts = s.trim().split('\n');
    parts
        .filter(|part| !part.trim().is_empty())
        .map(|part| {
            let json_val = serde_json::from_str(part)?;
            if let JsonValue::Object(mut o) = json_val {
                if o.get("kind") == Some(&JsonValue::String("LogLine".to_string())) {
                    if let Some(value) = o.remove("data") {
                        return Ok(ResponsePart::LogLine(LogLine::try_from(value)?));
                    };
                } else {
                    return Ok(ResponsePart::Result(JsonValue::Object(o)));
                }
            }
            anyhow::bail!("Invalid part")
        })
        .try_collect()
}

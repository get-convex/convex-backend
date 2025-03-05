use std::{
    collections::BTreeMap,
    sync::Arc,
};

use anyhow::anyhow;
use common::{
    auth::AuthConfig,
    knobs::{
        DATABASE_UDF_SYSTEM_TIMEOUT,
        DATABASE_UDF_USER_TIMEOUT,
    },
    log_lines::LogLevel,
    runtime::{
        Runtime,
        UnixTimestamp,
    },
};
use deno_core::{
    v8,
    ModuleSpecifier,
};
use errors::ErrorMetadata;
use model::{
    config::types::AUTH_CONFIG_FILE_NAME,
    environment_variables::types::{
        EnvVarName,
        EnvVarValue,
    },
    modules::module_versions::{
        FullModuleSource,
        ModuleSource,
        SourceMap,
    },
};
use rand_chacha::ChaCha12Rng;
use regex::Regex;
use serde_json::Value as JsonValue;
use value::{
    NamespacedTableMapping,
    TableMappingValue,
};

use crate::{
    concurrency_limiter::ConcurrencyPermit,
    environment::{
        helpers::syscall_error::{
            syscall_description_for_error,
            syscall_name_for_error,
        },
        AsyncOpRequest,
        IsolateEnvironment,
    },
    helpers,
    isolate::{
        Isolate,
        CONVEX_SCHEME,
    },
    request_scope::RequestScope,
    strings,
    timeout::Timeout,
};

pub struct AuthConfigEnvironment {
    auth_config_bundle: ModuleSource,
    source_map: Option<SourceMap>,
    environment_variables: BTreeMap<EnvVarName, EnvVarValue>,
}

impl<RT: Runtime> IsolateEnvironment<RT> for AuthConfigEnvironment {
    fn trace(&mut self, _level: LogLevel, messages: Vec<String>) -> anyhow::Result<()> {
        tracing::warn!(
            "Unexpected Console access when evaluating auth config file: {}",
            messages.join(" ")
        );
        Ok(())
    }

    fn rng(&mut self) -> anyhow::Result<&mut ChaCha12Rng> {
        anyhow::bail!(ErrorMetadata::bad_request(
            "NoRandomDuringAuthConfig",
            "Math.random unsupported when evaluating auth config file"
        ))
    }

    fn unix_timestamp(&mut self) -> anyhow::Result<UnixTimestamp> {
        anyhow::bail!(ErrorMetadata::bad_request(
            "NoDateDuringAuthConfig",
            "Date unsupported when evaluating auth config file"
        ))
    }

    fn get_environment_variable(
        &mut self,
        name: EnvVarName,
    ) -> anyhow::Result<Option<EnvVarValue>> {
        self.environment_variables
            .get(&name)
            .cloned()
            .ok_or_else(|| {
                anyhow::anyhow!(ErrorMetadata::bad_request(
                    // Special cased in Convex CLI!!!
                    "AuthConfigMissingEnvironmentVariable",
                    format!(
                        "Environment variable {} is used in auth config file but its value was \
                         not set",
                        name
                    ),
                ))
            })
            .map(Some)
    }

    fn get_table_mapping_without_system_tables(&mut self) -> anyhow::Result<TableMappingValue> {
        anyhow::bail!(ErrorMetadata::bad_request(
            "NoTableMappingFetchDuringAuthConfig",
            "Getting the table mapping unsupported when evaluating auth config file"
        ))
    }

    fn get_all_table_mappings(&mut self) -> anyhow::Result<NamespacedTableMapping> {
        anyhow::bail!(ErrorMetadata::bad_request(
            "NoTableMappingFetchDuringAuthConfig",
            "Getting the table mapping unsupported when evaluating auth config file"
        ))
    }

    async fn lookup_source(
        &mut self,
        path: &str,
        _timeout: &mut Timeout<RT>,
        _permit: &mut Option<ConcurrencyPermit>,
    ) -> anyhow::Result<Option<FullModuleSource>> {
        if path != AUTH_CONFIG_FILE_NAME {
            anyhow::bail!(ErrorMetadata::bad_request(
                "NoImportModuleDuringAuthConfig",
                format!("Can't import {path} while evaluating auth config file")
            ))
        }
        Ok(Some(FullModuleSource {
            source: self.auth_config_bundle.clone(),
            source_map: self.source_map.clone(),
        }))
    }

    fn syscall(&mut self, name: &str, _args: JsonValue) -> anyhow::Result<JsonValue> {
        anyhow::bail!(ErrorMetadata::bad_request(
            "NoSyscallDuringAuthConfig",
            format!("Syscall {name} unsupported when evaluating auth config file")
        ))
    }

    fn start_async_syscall(
        &mut self,
        name: String,
        _args: JsonValue,
        _resolver: v8::Global<v8::PromiseResolver>,
    ) -> anyhow::Result<()> {
        anyhow::bail!(ErrorMetadata::bad_request(
            format!("No{}DuringAuthConfig", syscall_name_for_error(&name)),
            format!(
                "{} unsupported while evaluating auth config file",
                syscall_description_for_error(&name),
            ),
        ))
    }

    fn start_async_op(
        &mut self,
        request: AsyncOpRequest,
        _resolver: v8::Global<v8::PromiseResolver>,
    ) -> anyhow::Result<()> {
        anyhow::bail!(ErrorMetadata::bad_request(
            format!("No{}DuringAuthConfig", request.name_for_error()),
            format!(
                "{} unsupported while evaluating auth config file",
                request.description_for_error()
            ),
        ))
    }

    fn user_timeout(&self) -> std::time::Duration {
        *DATABASE_UDF_USER_TIMEOUT
    }

    fn system_timeout(&self) -> std::time::Duration {
        *DATABASE_UDF_SYSTEM_TIMEOUT
    }
}

impl AuthConfigEnvironment {
    pub async fn evaluate_auth_config<RT: Runtime>(
        client_id: String,
        isolate: &mut Isolate<RT>,
        auth_config_bundle: ModuleSource,
        source_map: Option<SourceMap>,
        environment_variables: BTreeMap<EnvVarName, EnvVarValue>,
    ) -> anyhow::Result<AuthConfig> {
        let environment = Self {
            auth_config_bundle,
            source_map,
            environment_variables,
        };
        let client_id = Arc::new(client_id);
        let (handle, state) = isolate.start_request(client_id, environment).await?;
        let mut handle_scope = isolate.handle_scope();
        let v8_context = v8::Context::new(&mut handle_scope);
        let mut context_scope = v8::ContextScope::new(&mut handle_scope, v8_context);
        let mut isolate_context =
            RequestScope::new(&mut context_scope, handle.clone(), state, false).await?;
        let handle = isolate_context.handle();
        let result = Self::run_evaluate_auth_config(&mut isolate_context).await;

        // Drain the microtask queue, to clean up the isolate.
        isolate_context.scope.perform_microtask_checkpoint();

        // Unlink the request from the isolate.
        // After this point, it's unsafe to run js code in the isolate that
        // expects the current request's environment.
        // If the microtask queue is somehow nonempty after this point but before
        // the next request starts, the isolate may panic.
        drop(isolate_context);

        handle.take_termination_error(None, "auth")??;
        result
    }

    async fn run_evaluate_auth_config<RT: Runtime>(
        isolate: &mut RequestScope<'_, '_, RT, Self>,
    ) -> anyhow::Result<AuthConfig> {
        let mut v8_scope = isolate.scope();
        let mut scope = RequestScope::<RT, Self>::enter(&mut v8_scope);

        let auth_config_url = ModuleSpecifier::parse(&format!("{CONVEX_SCHEME}:/auth.config.js"))?;
        let module = scope.eval_module(&auth_config_url).await?;
        let namespace = module
            .get_module_namespace()
            .to_object(&mut scope)
            .ok_or_else(|| anyhow!("Module namespace wasn't an object?"))?;
        let default_str = strings::default.create(&mut scope)?;
        let config_val: v8::Local<v8::Value> = namespace
            .get(&mut scope, default_str.into())
            .ok_or(AuthConfigMissingExportError)?;
        if config_val.is_null_or_undefined() {
            anyhow::bail!(AuthConfigMissingExportError);
        }

        let config_str = json_stringify(&mut scope, config_val)?;
        Ok(
            serde_json::from_str(&config_str).map_err(|error| {
                AuthConfigNotMatchingSchemaError {
                    error: strip_position(&error.to_string()),
                }
            })?,
        )
    }
}

// It's not meaningful for the user to see the serialized
// position in the result of the auth config execution
fn strip_position(error_message: &str) -> String {
    let re = Regex::new(r"at line \d+ column \d+$").unwrap();
    re.replace(error_message, "").to_string()
}

fn json_stringify(
    scope: &mut v8::HandleScope,
    value: v8::Local<v8::Value>,
) -> anyhow::Result<String> {
    let json_stringify_code = strings::json_stringify.create(scope)?;
    let json_stringify_fn = v8::Script::compile(scope, json_stringify_code, None)
        .ok_or_else(|| anyhow!("Unexpected: Could not compile JSON.stringify"))?
        .run(scope)
        .ok_or_else(|| anyhow!("Unexpected: Could run compiled JSON.stringify"))?;
    let json_stringify_fn = v8::Local::<v8::Function>::try_from(json_stringify_fn).unwrap();
    let result = json_stringify_fn
        .call(scope, value, &[value])
        .ok_or(AuthConfigUnserializableError)?;
    let result: v8::Local<v8::String> = result.try_into()?;
    helpers::to_rust_string(scope, &result)
}

const SEE_AUTH_DOCS: &str =
    "To learn more, see the auth documentation at https://docs.convex.dev/auth.";

#[derive(thiserror::Error, Debug, Clone, PartialEq)]
#[error("auth config file is missing default export. {SEE_AUTH_DOCS}")]
pub struct AuthConfigMissingExportError;

#[derive(thiserror::Error, Debug, Clone, PartialEq)]
#[error("auth config file can only contain strings {SEE_AUTH_DOCS}")]
pub struct AuthConfigUnserializableError;

#[derive(thiserror::Error, Debug, Clone, PartialEq)]
#[error("auth config file must include a list of provider credentials: {error}")]
pub struct AuthConfigNotMatchingSchemaError {
    error: String,
}

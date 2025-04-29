use std::sync::Arc;

use anyhow::anyhow;
use common::{
    json::JsonSerializable,
    knobs::{
        DATABASE_UDF_SYSTEM_TIMEOUT,
        DATABASE_UDF_USER_TIMEOUT,
    },
    log_lines::LogLevel,
    runtime::{
        Runtime,
        UnixTimestamp,
    },
    schemas::{
        invalid_schema_export_error,
        missing_schema_export_error,
        DatabaseSchema,
    },
};
use deno_core::{
    v8,
    ModuleSpecifier,
};
use errors::{
    ErrorMetadata,
    ErrorMetadataAnyhowExt,
};
use model::{
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
use rand::SeedableRng;
use rand_chacha::ChaCha12Rng;
use serde_json::Value as JsonValue;
use value::NamespacedTableMapping;

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

pub struct SchemaEnvironment {
    schema_bundle: ModuleSource,
    source_map: Option<SourceMap>,
    rng: ChaCha12Rng,
    unix_timestamp: UnixTimestamp,
}

impl<RT: Runtime> IsolateEnvironment<RT> for SchemaEnvironment {
    fn trace(&mut self, _level: LogLevel, messages: Vec<String>) -> anyhow::Result<()> {
        tracing::warn!(
            "Unexpected Console access at schema evaluation time: {}",
            messages.join(" ")
        );
        Ok(())
    }

    fn rng(&mut self) -> anyhow::Result<&mut ChaCha12Rng> {
        Ok(&mut self.rng)
    }

    fn crypto_rng(&mut self) -> anyhow::Result<super::crypto_rng::CryptoRng> {
        anyhow::bail!(ErrorMetadata::bad_request(
            "NoCryptoRngInSchema",
            "Cannot use cryptographic randomness when evaluating schema"
        ))
    }

    fn unix_timestamp(&mut self) -> anyhow::Result<UnixTimestamp> {
        Ok(self.unix_timestamp)
    }

    fn get_environment_variable(
        &mut self,
        _name: EnvVarName,
    ) -> anyhow::Result<Option<EnvVarValue>> {
        anyhow::bail!(ErrorMetadata::bad_request(
            "NoEnvironmentVariablesInSchema",
            "Environment variables unsupported when evaluating schema"
        ))
    }

    fn get_all_table_mappings(&mut self) -> anyhow::Result<NamespacedTableMapping> {
        anyhow::bail!(ErrorMetadata::bad_request(
            "NoTableMappingFetchInSchema",
            "Getting the table mapping unsupported when evaluating schema"
        ))
    }

    async fn lookup_source(
        &mut self,
        path: &str,
        _timeout: &mut Timeout<RT>,
        _permit: &mut Option<ConcurrencyPermit>,
    ) -> anyhow::Result<Option<FullModuleSource>> {
        if path != "schema.js" {
            anyhow::bail!(ErrorMetadata::bad_request(
                "NoImportModuleInSchema",
                format!("Can't import {path} while evaluating schema")
            ))
        }
        Ok(Some(FullModuleSource {
            source: self.schema_bundle.clone(),
            source_map: self.source_map.clone(),
        }))
    }

    fn syscall(&mut self, name: &str, _args: JsonValue) -> anyhow::Result<JsonValue> {
        anyhow::bail!(ErrorMetadata::bad_request(
            "NoSyscallInSchema",
            format!("Syscall {name} unsupported when evaluating schema")
        ));
    }

    fn start_async_syscall(
        &mut self,
        name: String,
        _args: JsonValue,
        _resolver: v8::Global<v8::PromiseResolver>,
    ) -> anyhow::Result<()> {
        anyhow::bail!(ErrorMetadata::bad_request(
            format!("No{}InSchema", syscall_name_for_error(&name)),
            format!(
                "{} unsupported while evaluating schema",
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
            format!("No{}InSchema", request.name_for_error()),
            format!(
                "{} unsupported while evaluating schema",
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

impl SchemaEnvironment {
    pub async fn evaluate_schema<RT: Runtime>(
        client_id: String,
        isolate: &mut Isolate<RT>,
        schema_bundle: ModuleSource,
        source_map: Option<SourceMap>,
        rng_seed: [u8; 32],
        unix_timestamp: UnixTimestamp,
    ) -> anyhow::Result<DatabaseSchema> {
        let rng = ChaCha12Rng::from_seed(rng_seed);
        let environment = Self {
            schema_bundle,
            source_map,
            rng,
            unix_timestamp,
        };
        let client_id = Arc::new(client_id);
        let (handle, state) = isolate.start_request(client_id, environment).await?;
        let mut handle_scope = isolate.handle_scope();
        let v8_context = v8::Context::new(&mut handle_scope);
        let mut context_scope = v8::ContextScope::new(&mut handle_scope, v8_context);
        let mut isolate_context =
            RequestScope::new(&mut context_scope, handle.clone(), state, false).await?;
        let handle = isolate_context.handle();
        let result = Self::run_evaluate_schema(&mut isolate_context).await;

        // Drain the microtask queue, to clean up the isolate.
        isolate_context.checkpoint();

        // Unlink the request from the isolate.
        // After this point, it's unsafe to run js code in the isolate that
        // expects the current request's environment.
        // If the microtask queue is somehow nonempty after this point but before
        // the next request starts, the isolate may panic.
        drop(isolate_context);

        handle.take_termination_error(None, "schema")??;
        result
    }

    async fn run_evaluate_schema<RT: Runtime>(
        isolate: &mut RequestScope<'_, '_, RT, Self>,
    ) -> anyhow::Result<DatabaseSchema> {
        let mut v8_scope = isolate.scope();
        let mut scope = RequestScope::<RT, Self>::enter(&mut v8_scope);

        let schema_url = ModuleSpecifier::parse(&format!("{CONVEX_SCHEME}:/schema.js"))?;
        let module = scope.eval_module(&schema_url).await?;
        let namespace = module
            .get_module_namespace()
            .to_object(&mut scope)
            .ok_or_else(|| anyhow!("Module namespace wasn't an object?"))?;
        let default_str = strings::default.create(&mut scope)?;
        let schema_val: v8::Local<v8::Value> = namespace
            .get(&mut scope, default_str.into())
            .ok_or(missing_schema_export_error())?;
        if schema_val.is_null_or_undefined() {
            anyhow::bail!(missing_schema_export_error());
        }
        let export_str = strings::export.create(&mut scope)?;
        let v8_schema_result: anyhow::Result<v8::Local<v8::String>> = try {
            let schema_obj: v8::Local<v8::Object> = schema_val.try_into()?;
            let export_function: v8::Local<v8::Function> = schema_obj
                .get(&mut scope, export_str.into())
                .ok_or_else(|| anyhow!("Couldn't find 'export' method on schema object"))?
                .try_into()?;

            match scope.with_try_catch(|s| export_function.call(s, schema_obj.into(), &[]))?? {
                Some(r) => Ok(v8::Local::<v8::String>::try_from(r)?),
                None => Err(anyhow!(
                    "Missing return value from successful function call"
                )),
            }?
        };
        // If we can't export the schema into a string, probably there is
        // something funky in their `schema.ts` file, so throw
        // `invalid_schema_export_error`
        let v8_schema_str: v8::Local<v8::String> =
            v8_schema_result.map_err(|_| invalid_schema_export_error())?;

        let result_str = helpers::to_rust_string(&mut scope, &v8_schema_str)?;
        DatabaseSchema::json_deserialize(&result_str).map_err(|e| {
            if e.is_bad_request() {
                e
            } else {
                let msg = e.to_string();
                e.context(ErrorMetadata::bad_request("InvalidSchema", msg))
            }
        })
    }
}

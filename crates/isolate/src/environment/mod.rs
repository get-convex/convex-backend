use std::time::Duration;

use model::{
    environment_variables::types::{
        EnvVarName,
        EnvVarValue,
    },
    modules::module_versions::FullModuleSource,
};
pub mod action;
pub mod analyze;
pub mod async_op;
pub mod auth_config;
pub mod component_definitions;
pub mod helpers;
pub mod schema;
pub mod udf;
pub mod warnings;

use common::{
    errors::JsError,
    log_lines::LogLevel,
    runtime::{
        Runtime,
        UnixTimestamp,
    },
};
use deno_core::v8;
use rand_chacha::ChaCha12Rng;
use serde_json::Value as JsonValue;
use value::{
    NamespacedTableMapping,
    TableMappingValue,
};

pub use self::async_op::AsyncOpRequest;
use crate::{
    concurrency_limiter::ConcurrencyPermit,
    isolate::IsolateHeapStats,
    timeout::Timeout,
};

/// This trait allows fine-grained control over V8 environment we set up.
///
/// The isolate layer needs to know how to import code, so each
/// implementation of [`IsolateEnvironment`] can control code loading with the
/// [`lookup_source`] method.
///
/// We provide a set of "ops" to back JS libraries we provide in our environment
/// like `console`, `Math.random`, and `Date`. Parts of these are left
/// unimplemented on this trait to allow different implementations for each
/// environment.
///
/// To add additional functionality to the JS environment, implementors can add
/// custom syscalls with the [`syscall`] method. Syscalls must maintain
/// backwards compatibility with the JS code that call them.
///
/// Both ops and syscalls can return errors tagged with `DeveloperError` to
/// signal a user-visible error that will be turned into a JavaScript exception.
pub trait IsolateEnvironment<RT: Runtime>: 'static {
    #[allow(async_fn_in_trait)]
    async fn lookup_source(
        &mut self,
        path: &str,
        timeout: &mut Timeout<RT>,
        permit: &mut Option<ConcurrencyPermit>,
    ) -> anyhow::Result<Option<FullModuleSource>>;

    fn syscall(&mut self, name: &str, args: JsonValue) -> anyhow::Result<JsonValue>;
    fn start_async_syscall(
        &mut self,
        name: String,
        args: JsonValue,
        resolver: v8::Global<v8::PromiseResolver>,
    ) -> anyhow::Result<()>;

    fn trace(&mut self, level: LogLevel, messages: Vec<String>) -> anyhow::Result<()>;
    fn rng(&mut self) -> anyhow::Result<&mut ChaCha12Rng>;
    fn unix_timestamp(&self) -> anyhow::Result<UnixTimestamp>;

    fn get_environment_variable(&mut self, name: EnvVarName)
        -> anyhow::Result<Option<EnvVarValue>>;

    /// The table mapping omitting system tables, intended for the dashboard.
    fn get_table_mapping_without_system_tables(&mut self) -> anyhow::Result<TableMappingValue>;
    fn get_all_table_mappings(&mut self) -> anyhow::Result<NamespacedTableMapping>;

    fn start_async_op(
        &mut self,
        request: AsyncOpRequest,
        resolver: v8::Global<v8::PromiseResolver>,
    ) -> anyhow::Result<()>;

    fn record_heap_stats(&self, _heap_size: IsolateHeapStats) {}

    fn user_timeout(&self) -> Duration;
    fn system_timeout(&self) -> Duration;
}

#[derive(Debug, thiserror::Error)]
#[error("UncatchableDeveloperError")]
pub struct UncatchableDeveloperError {
    pub js_error: JsError,
}

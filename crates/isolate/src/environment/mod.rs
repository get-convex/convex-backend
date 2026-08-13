use std::{
    sync::Arc,
    time::Duration,
};

use model::environment_variables::types::{
    EnvVarName,
    EnvVarValue,
};

use self::crypto_rng::CryptoRng;
pub mod action;
pub mod analyze;
pub mod async_op;
pub mod auth_config;
pub mod component_definitions;
pub mod crypto_rng;
pub mod helpers;
pub mod schema;
pub mod udf;

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
use value::NamespacedTableMapping;

pub use self::async_op::AsyncOpRequest;
use crate::{
    module_cache::V8ModuleSource,
    timeout::Timeout,
};

/// This trait allows fine-grained control over the V8 environment we set up.
///
/// The isolate layer needs to know how to import code, so each
/// implementation of [`IsolateEnvironment`] can control code loading with
/// [`SyscallProvider::lookup_source`].
///
/// We provide a set of "ops" to back JS libraries we provide in our environment
/// like `console`, `Math.random`, and `Date`. Parts of these are left
/// unimplemented on [`SyscallProvider`] to allow different implementations for
/// each environment.
///
/// To add additional functionality to the JS environment, implementors can add
/// custom syscalls with [`SyscallProvider::syscall`]. Syscalls must maintain
/// backwards compatibility with the JS code that call them.
///
/// Both ops and syscalls can return errors tagged with
/// `ErrorMetadata::bad_request` to signal a user-visible error that will be
/// turned into a JavaScript exception.
pub trait IsolateEnvironment<RT: Runtime>: 'static {
    /// Handle the environment uses to complete an async syscall or op that it
    /// started.
    type AsyncResolver;

    type SyscallProvider: SyscallProvider<RT>;

    fn syscall_provider(&mut self) -> &mut Self::SyscallProvider;

    fn start_async_syscall(
        &mut self,
        name: String,
        args: JsonValue,
        resolver: Self::AsyncResolver,
    ) -> anyhow::Result<()>;

    fn start_async_op(
        &mut self,
        request: AsyncOpRequest,
        resolver: Self::AsyncResolver,
    ) -> anyhow::Result<()>;

    // The memory allocated by the environment itself.
    fn environment_heap_size(&self) -> usize {
        0
    }

    fn user_timeout(&self) -> Duration;
    fn system_timeout(&self) -> Duration;
}

/// Gives access to the syscalls, ops, and module loading used by the isolate.
#[allow(async_fn_in_trait)]
pub trait SyscallProvider<RT: Runtime>: 'static {
    async fn lookup_source(
        &mut self,
        path: &str,
        timeout: &mut Timeout<RT>,
    ) -> anyhow::Result<Option<(Arc<V8ModuleSource>, ModuleCodeCacheResult)>>;

    fn syscall(&mut self, name: &str, args: JsonValue) -> anyhow::Result<JsonValue>;

    fn trace(&mut self, level: LogLevel, messages: Vec<String>) -> anyhow::Result<()>;
    fn rng(&mut self) -> anyhow::Result<&mut ChaCha12Rng>;
    fn crypto_rng(&mut self) -> anyhow::Result<CryptoRng>;
    fn unix_timestamp(&mut self) -> anyhow::Result<UnixTimestamp>;
    fn performance_now(&mut self) -> anyhow::Result<Duration>;
    fn performance_time_origin(&mut self) -> anyhow::Result<UnixTimestamp>;

    fn get_environment_variable(&mut self, name: EnvVarName)
        -> anyhow::Result<Option<EnvVarValue>>;

    fn get_all_table_mappings(&mut self) -> anyhow::Result<NamespacedTableMapping>;
}

/// An [`IsolateEnvironment`] that can run inside V8, i.e. one whose async
/// resolver is a real promise resolver. Everything in this crate that touches a
/// `v8::Scope` requires this, while the environments themselves only need
/// [`IsolateEnvironment`].
pub trait V8IsolateEnvironment<RT: Runtime>:
    IsolateEnvironment<RT, AsyncResolver = v8::Global<v8::PromiseResolver>>
{
}

impl<RT: Runtime, E> V8IsolateEnvironment<RT> for E where
    E: IsolateEnvironment<RT, AsyncResolver = v8::Global<v8::PromiseResolver>>
{
}

#[derive(Debug, thiserror::Error)]
#[error("UncatchableDeveloperError")]
pub struct UncatchableDeveloperError {
    pub js_error: JsError,
}

pub enum ModuleCodeCacheResult {
    Cached(Arc<[u8]>),
    /// The module isn't cached; it can be populated by calling the callback
    /// with the generated CachedData.
    Uncached(Box<dyn FnOnce(Arc<[u8]>)>),
}

impl ModuleCodeCacheResult {
    pub fn noop() -> Self {
        ModuleCodeCacheResult::Uncached(Box::new(|_| ()))
    }
}

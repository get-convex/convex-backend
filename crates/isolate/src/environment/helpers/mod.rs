mod json_packed_value;
pub mod module_loader;
mod outcome;
pub mod permit;
mod promise;
pub mod syscall_error;
mod syscall_stats;
mod syscall_trace;
pub mod validation;
mod version;

use common::runtime::Runtime;
use deno_core::{
    serde_v8,
    v8,
};
use errors::ErrorMetadata;
use serde_json::Value as JsonValue;

pub use self::{
    json_packed_value::JsonPackedValue,
    outcome::FunctionOutcome,
    promise::{
        resolve_promise,
        resolve_promise_allow_all_errors,
    },
    syscall_stats::SyscallStats,
    syscall_trace::SyscallTrace,
    version::parse_version,
};
use crate::{
    environment::IsolateEnvironment,
    execution_scope::ExecutionScope,
};

pub const MAX_LOG_LINE_LENGTH: usize = 32768;
pub const MAX_LOG_LINES: usize = 256;

#[derive(Debug, derive_more::Display)]
pub struct ArgName(pub &'static str);

pub fn with_argument_error<T>(
    name: &str,
    f: impl FnOnce() -> anyhow::Result<T>,
) -> anyhow::Result<T> {
    f().map_err(|e| {
        anyhow::anyhow!(ErrorMetadata::bad_request(
            "InvalidArgument",
            if let Some(ArgName(arg_name)) = e.downcast_ref() {
                if let Some(cause) = e.chain().nth(1) {
                    format!("Invalid argument `{arg_name}` for `{name}`: {cause}")
                } else {
                    format!("Invalid argument `{arg_name}` for `{name}`: {e}")
                }
            } else {
                format!("Invalid arguments for `{name}`: {e}")
            }
        ))
    })
}

#[derive(Eq, PartialEq, Debug)]
pub enum Phase {
    Importing,
    Executing,
}

pub fn json_to_v8<'a, 'b: 'a, RT: Runtime, E: IsolateEnvironment<RT>>(
    scope: &mut ExecutionScope<'a, 'b, RT, E>,
    json: JsonValue,
) -> anyhow::Result<v8::Local<'a, v8::Value>> {
    let value_v8 = serde_v8::to_v8(scope, json)?;
    Ok(value_v8)
}

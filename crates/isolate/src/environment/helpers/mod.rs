pub mod module_loader;
pub mod permit;
mod promise;
pub mod syscall_error;
mod version;

use std::ops::Deref;

use anyhow::Context;
use deno_core::{
    serde_v8,
    v8,
};
use errors::{
    ErrorCode,
    ErrorMetadata,
};
use serde_json::Value as JsonValue;
use value::TableName;

pub use self::{
    promise::{
        resolve_promise,
        resolve_promise_allow_all_errors,
    },
    version::parse_version,
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

pub fn json_to_v8<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    json: JsonValue,
) -> anyhow::Result<v8::Local<'s, v8::Value>> {
    let value_v8 = serde_v8::to_v8(scope, json)?;
    Ok(value_v8)
}

/// Convert `RejectedBeforeExecution` error codes into `Overloaded`.
/// This is useful when calling nested UDFs as the code would otherwise leak out
/// of the _parent_ UDF, causing its caller to mistakenly believe the parent
/// call to be retriable.
pub fn remove_rejected_before_execution(mut e: anyhow::Error) -> anyhow::Error {
    if let Some(em) = e.downcast_mut::<ErrorMetadata>()
        && em.code == ErrorCode::RejectedBeforeExecution
    {
        em.code = ErrorCode::Overloaded;
    }
    e
}

/// For DB syscalls that take an explicit table name, checks that the
/// explicit table name that the user used (`requested_table_name`)
/// matches the name of the ID’s table (`actual_name_name`).
pub fn check_table_name(
    requested_table_name: &Option<String>,
    actual_table_name: &TableName,
) -> anyhow::Result<()> {
    if let Some(requested_table_name) = requested_table_name
        && requested_table_name != actual_table_name.deref()
    {
        return Err(ErrorMetadata::bad_request(
            "InvalidTable",
            format!(
                "expected to be an Id<\"{}\">, got Id<\"{}\"> instead.",
                requested_table_name,
                actual_table_name.deref()
            ),
        ))
        .context(ArgName("id"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_check_table_name_accepts_the_correct_name() {
        let requested_table_name = Some("documents".to_string());
        let actual_table_name: TableName = "documents".parse().unwrap();
        assert!(check_table_name(&requested_table_name, &actual_table_name).is_ok());
    }

    #[test]
    fn test_check_table_name_rejects_oher_names() {
        let requested_table_name = Some("documents".to_string());
        let actual_table_name: TableName = "users".parse().unwrap();
        assert!(check_table_name(&requested_table_name, &actual_table_name).is_err());
    }

    #[test]
    fn test_check_table_name_does_nothing_if_the_requested_table_name_is_none() {
        let requested_table_name = None;
        let actual_table_name: TableName = "documents".parse().unwrap();
        assert!(check_table_name(&requested_table_name, &actual_table_name).is_ok());
    }
}

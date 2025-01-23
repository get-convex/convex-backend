use anyhow::anyhow;
use common::errors::{
    report_error_sync,
    JsError,
};
use deno_core::v8;
use errors::ErrorMetadataAnyhowExt;
use serde_json::Value as JsonValue;

use super::json_to_v8;
use crate::strings;

pub fn resolve_promise(
    scope: &mut v8::HandleScope<'_>,
    resolver: v8::Global<v8::PromiseResolver>,
    result: anyhow::Result<v8::Local<v8::Value>>,
) -> anyhow::Result<()> {
    resolve_promise_inner(scope, resolver, result, false)
}

// Like `resolve_promise` but returns JS error even when the
// error might have been caused by Convex, not by the user.
pub fn resolve_promise_allow_all_errors(
    scope: &mut v8::HandleScope<'_>,
    resolver: v8::Global<v8::PromiseResolver>,
    result: anyhow::Result<v8::Local<v8::Value>>,
) -> anyhow::Result<()> {
    resolve_promise_inner(scope, resolver, result, true)
}

fn resolve_promise_inner(
    scope: &mut v8::HandleScope<'_>,
    resolver: v8::Global<v8::PromiseResolver>,
    result: anyhow::Result<v8::Local<v8::Value>>,
    allow_all_errors: bool,
) -> anyhow::Result<()> {
    let resolver = resolver.open(scope);
    match result {
        Ok(value_v8) => {
            resolver.resolve(scope, value_v8);
        },
        Err(mut e) => {
            // This error might have been caused by the user,
            // or by the system. We either:
            // - return it, which will result in a system error (which is logged higher in
            //   the stack)
            // - log it now, and convert it to a JsError
            if !e.is_deterministic_user_error() {
                if allow_all_errors {
                    report_error_sync(&mut e);
                } else {
                    return Err(e);
                };
            }

            let message = e.user_facing_message();
            let message_v8 = v8::String::new(scope, &message[..]).unwrap();
            let exception = v8::Exception::error(scope, message_v8);
            let custom_data = if let Some(js_error) = e.downcast_ref::<JsError>() {
                js_error.custom_data.clone()
            } else {
                None
            };
            if let Some(custom_data) = custom_data {
                let field_name = strings::data.create(scope)?;
                let exception_object = exception
                    .to_object(scope)
                    .ok_or_else(|| anyhow!("Failed to convert error to object"))?;
                let custom_data_v8 = json_to_v8(scope, JsonValue::from(custom_data))?;
                exception_object.set(scope, field_name.into(), custom_data_v8);
            }
            resolver.reject(scope, exception);
        },
    }
    Ok(())
}

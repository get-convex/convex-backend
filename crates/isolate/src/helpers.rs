use anyhow::Context;
use common::{
    components::CanonicalizedComponentFunctionPath,
    errors::JsError,
    knobs::{
        FUNCTION_MAX_ARGS_SIZE,
        FUNCTION_MAX_RESULT_SIZE,
    },
    value::{
        ConvexArray,
        ConvexValue,
    },
};
use deno_core::v8;
use errors::{
    ErrorMetadata,
    ErrorMetadataAnyhowExt,
};
use humansize::{
    FormatSize,
    BINARY,
};
use serde::Deserialize;
use serde_json::Value as JsonValue;
use sync_types::CanonicalizedUdfPath;
use value::Size;

use crate::strings;

// The below methods were taken from `deno_core`
// https://github.com/denoland/deno_core/blob/main/LICENSE.md - MIT License
// Copyright 2018-2024 the Deno authors
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in
// all copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.

/// Taken from `deno_core::bindings::module_origin`.
pub fn module_origin<'a>(
    s: &mut v8::HandleScope<'a>,
    resource_name: v8::Local<'a, v8::String>,
) -> v8::ScriptOrigin<'a> {
    // TODO: Fill this out more accurately.
    let source_map_url = strings::empty.create(s).unwrap();
    v8::ScriptOrigin::new(
        s,
        resource_name.into(),  // resource_name
        0,                     // resource_line_offset
        0,                     // resource_column_offset
        false,                 // resource_is_shared_cross_origin
        0,                     // script_id
        source_map_url.into(), // source_map_url
        true,                  // resource_is_opaque
        false,                 // is_wasm
        true,                  // is_module
    )
}

/// Taken from `deno_core::bindings::throw_type_error`.
pub fn throw_type_error(scope: &mut v8::HandleScope, message: impl AsRef<str>) {
    let message = v8::String::new(scope, message.as_ref()).unwrap();
    let exception = v8::Exception::type_error(scope, message);
    scope.throw_exception(exception);
}

pub fn to_rust_string(scope: &mut v8::Isolate, s: &v8::String) -> anyhow::Result<String> {
    let n = s.utf8_length(scope);
    let mut buf = vec![0; n];
    // Don't set `REPLACE_INVALID_UTF8` since we want unpaired surrogates to fail
    // the UTF8 check below.
    let opts = v8::WriteOptions::NO_NULL_TERMINATION;
    let num_written = s.write_utf8(scope, &mut buf, None, opts);
    anyhow::ensure!(n == num_written);
    let s = String::from_utf8(buf)?;
    Ok(s)
}

pub fn get_property<'a>(
    scope: &mut v8::HandleScope<'a>,
    object: v8::Local<v8::Object>,
    key: &str,
) -> anyhow::Result<Option<v8::Local<'a, v8::Value>>> {
    let key = v8::String::new(scope, key)
        .ok_or_else(|| anyhow::anyhow!("Failed to create string for {key}"))?;
    Ok(object.get(scope, key.into()))
}

pub fn serialize_udf_args(args: ConvexArray) -> anyhow::Result<String> {
    let json_args: JsonValue = ConvexValue::Array(args).into();
    Ok(serde_json::to_string(&json_args)?)
}

pub fn parse_udf_args(
    path: &CanonicalizedUdfPath,
    args: Vec<JsonValue>,
) -> Result<ConvexArray, JsError> {
    args.into_iter()
        .map(|arg| arg.try_into())
        .collect::<anyhow::Result<Vec<_>>>()
        .and_then(ConvexArray::try_from)
        .map_err(|err| {
            JsError::from_message(format!(
                "Invalid arguments for {}: {err}",
                String::from(path.clone()),
            ))
        })
}

pub fn validate_udf_args_size(
    path: &CanonicalizedUdfPath,
    args: &ConvexArray,
) -> Result<(), JsError> {
    if args.size() > *FUNCTION_MAX_ARGS_SIZE {
        return Err(JsError::from_message(format!(
            "Arguments for {} are too large (actual: {}, limit: {})",
            path.clone(),
            args.size().format_size(BINARY),
            (*FUNCTION_MAX_ARGS_SIZE).format_size(BINARY),
        )));
    }

    Ok(())
}

pub fn deserialize_udf_result(
    path: &CanonicalizedComponentFunctionPath,
    result_str: &str,
) -> anyhow::Result<Result<ConvexValue, JsError>> {
    // Don't print out result_str in error messages - as it may contain pii
    let result_v: serde_json::Value = serde_json::from_str(result_str).map_err(|e| {
        anyhow::anyhow!(ErrorMetadata::bad_request(
            "FunctionReturnInvalidJson",
            format!(
                "Function {} failed. Could not parse return value as json: {e}",
                path.debug_str()
            ),
        ))
    })?;
    let result = match ConvexValue::try_from(result_v) {
        Ok(value) => {
            if value.size() > *FUNCTION_MAX_RESULT_SIZE {
                Err(JsError::from_message(format!(
                    "Function {} return value is too large (actual: {}, limit: {})",
                    path.debug_str(),
                    value.size().format_size(BINARY),
                    (*FUNCTION_MAX_RESULT_SIZE).format_size(BINARY),
                )))
            } else {
                Ok(value)
            }
        },
        Err(e) if e.is_deterministic_user_error() => {
            Err(JsError::from_error(e.wrap_error_message(|msg| {
                format!("Function {} return value invalid: {msg}", path.debug_str())
            })))
        },
        Err(e) => return Err(e),
    };
    Ok(result)
}

// custom error is called `ConvexError` in udfs
pub fn deserialize_udf_custom_error(
    message: String,
    serialized_data: Option<String>,
) -> anyhow::Result<(String, Option<ConvexValue>)> {
    Ok(if let Some(serialized_data) = serialized_data {
        let deserialized_custom_data = deserialize_udf_custom_error_data(&serialized_data)?;
        match deserialized_custom_data {
            Ok(custom_data) => (message, Some(custom_data)),
            // If we can't deserialize the custom data, we'll replace
            // the ConvexError with the formatting error
            Err(custom_data_format_error) => (custom_data_format_error.message, None),
        }
    } else {
        (message, None)
    })
}

fn deserialize_udf_custom_error_data(
    result_str: &str,
) -> anyhow::Result<Result<ConvexValue, JsError>> {
    let result_v: serde_json::Value = serde_json::from_str(result_str).context(format!(
        "Unable to deserialize udf error data: {result_str}"
    ))?;
    let result = match ConvexValue::try_from(result_v) {
        Ok(value) => Ok(value),
        Err(e) if e.is_deterministic_user_error() => {
            Err(JsError::from_error(e.wrap_error_message(|msg| {
                format!("ConvexError with invalid data: {msg}")
            })))
        },
        Err(e) => return Err(e),
    };
    Ok(result)
}

pub fn format_uncaught_error(message: String, name: String) -> String {
    if !name.is_empty() && !message.is_empty() {
        format!("Uncaught {}: {}", name, message)
    } else if !name.is_empty() {
        format!("Uncaught {}", name)
    } else if !message.is_empty() {
        format!("Uncaught {}", message)
    } else {
        "Uncaught".to_string()
    }
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
#[serde(untagged)]
pub enum UdfArgsJson {
    /// For legacy positional args array
    PositionalArgs(Vec<JsonValue>),
    /// For named args
    NamedArgs(JsonValue),
}

impl UdfArgsJson {
    /// Map it into our internal representation of positional args.
    /// Modern apps just have a single positional arg with an object.
    pub fn into_arg_vec(self) -> Vec<JsonValue> {
        match self {
            UdfArgsJson::PositionalArgs(args) => args,
            UdfArgsJson::NamedArgs(obj) => vec![obj],
        }
    }
}

#[cfg(test)]
mod tests {
    use std::assert_matches::assert_matches;

    use serde_json::json;

    use crate::UdfArgsJson;

    #[test]
    fn test_udf_args_json() -> anyhow::Result<()> {
        let json1: UdfArgsJson = serde_json::from_str(r#"["a", "b", "c"]"#)?;
        let json2: UdfArgsJson = serde_json::from_str(r#"{"named": "arg"}"#)?;
        let json3: UdfArgsJson = serde_json::from_str(r#"[{"named": "arg"}]"#)?;
        assert_matches!(json1, UdfArgsJson::PositionalArgs(_));
        assert_matches!(json2, UdfArgsJson::NamedArgs(_));
        assert_matches!(json3, UdfArgsJson::PositionalArgs(_));
        assert_eq!(
            json1.into_arg_vec(),
            vec![json!("a"), json!("b"), json!("c")]
        );
        assert_eq!(json2.into_arg_vec(), vec![json!({"named": "arg"})]);
        assert_eq!(json3.into_arg_vec(), vec![json!({"named": "arg"})]);
        Ok(())
    }
}

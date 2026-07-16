use common::{
    errors::JsError,
    knobs::FUNCTION_MAX_ARGS_SIZE,
};
use humansize::{
    FormatSize,
    BINARY,
};
use serde::Deserialize;
use serde_json::{
    value::RawValue,
    Value as JsonValue,
};
use sync_types::{
    types::SerializedArgs,
    CanonicalizedUdfPath,
};
use value::{
    ConvexArray,
    ConvexValue,
    Size,
};

use crate::metrics::log_legacy_positional_args;

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

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct UdfArgsJson(Box<RawValue>);

impl UdfArgsJson {
    /// Map it into our internal representation of positional args.
    /// Modern apps just have a single positional arg with an object.
    pub fn into_serialized_args(self) -> anyhow::Result<SerializedArgs> {
        // For legacy positional args array
        // RawValue from serde is guaranteed to have no leading whitespace.
        if self.0.get().starts_with("[") {
            log_legacy_positional_args();
            return Ok(SerializedArgs::from_raw(self.0));
        }
        // For named args - stick it in an array
        Ok(SerializedArgs::from_raw(serde_json::value::to_raw_value(
            &[self.0],
        )?))
    }
}
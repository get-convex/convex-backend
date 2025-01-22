use common::{
    errors::JsError,
    knobs::FUNCTION_MAX_ARGS_SIZE,
};
use humansize::{
    FormatSize,
    BINARY,
};
use serde_json::Value as JsonValue;
use sync_types::CanonicalizedUdfPath;
use value::{
    ConvexArray,
    ConvexValue,
    Size,
};

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

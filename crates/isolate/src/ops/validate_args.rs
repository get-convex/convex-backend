use anyhow::Context;
use common::json::JsonForm as _;
use model::{
    modules::function_validators::ArgsValidator,
    virtual_system_mapping,
};
use serde_json::{
    json,
    Value as JsonValue,
};
use value::{
    serialized_args_ext::SerializedArgsExt,
    ConvexArray,
};

use super::OpProvider;
use crate::helpers::UdfArgsJson;

#[convex_macro::v8_op]
pub fn op_validate_args<'b, P: OpProvider<'b>>(
    provider: &mut P,
    validator: JsonValue,
    args: JsonValue,
) -> anyhow::Result<JsonValue> {
    let JsonValue::String(validator_string) = validator.clone() else {
        return Err(anyhow::anyhow!("export_args result not a string"));
    };

    let args_validator = match ArgsValidator::json_deserialize(&validator_string) {
        Ok(v) => v,
        Err(json_error) => {
            let message = format!("Unable to parse JSON returned from `exportArgs`: {json_error}");
            return Err(anyhow::anyhow!(message));
        },
    };

    let args: UdfArgsJson = serde_json::from_value(args)?;
    let args_array = args
        .into_serialized_args()?
        .into_args()?
        .into_iter()
        .map(|arg| arg.try_into())
        .collect::<anyhow::Result<Vec<_>>>()
        .and_then(ConvexArray::try_from)
        .map_err(|err| anyhow::anyhow!(format!("{}", err)))?;

    let table_mapping = provider.get_all_table_mappings()?;
    match args_validator.check_args(&args_array, &table_mapping, virtual_system_mapping())? {
        Some(js_error) => Ok(json!({
            "valid": false,
            "message": format!("{}", js_error)
        })),
        None => Ok(json!({
            "valid": true,
        })),
    }
}

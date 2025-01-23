use anyhow::Context;
use model::{
    modules::function_validators::ReturnsValidator,
    virtual_system_mapping,
};
use serde_json::{
    json,
    Value as JsonValue,
};
use value::ConvexValue;

use super::OpProvider;

#[convex_macro::v8_op]
pub fn op_validate_returns<'b, P: OpProvider<'b>>(
    provider: &mut P,
    validator: JsonValue,
    function_result: JsonValue,
) -> anyhow::Result<JsonValue> {
    let JsonValue::String(validator_string) = validator.clone() else {
        return Err(anyhow::anyhow!("export_args result not a string"));
    };

    let returns_validator: ReturnsValidator =
        match serde_json::from_str::<JsonValue>(&validator_string) {
            Ok(args_json) => ReturnsValidator::try_from(args_json)?,
            Err(json_error) => {
                let message =
                    format!("Unable to parse JSON returned from `exportReturns`: {json_error}");
                return Err(anyhow::anyhow!(message));
            },
        };

    let function_result = ConvexValue::try_from(function_result)?;

    let table_mapping = provider.get_all_table_mappings()?;
    match returns_validator.check_output(
        &function_result,
        &table_mapping,
        &virtual_system_mapping(),
    ) {
        Some(js_error) => Ok(json!({
            "valid": false,
            "message": format!("{}", js_error)
        })),
        None => Ok(json!({
            "valid": true,
        })),
    }
}

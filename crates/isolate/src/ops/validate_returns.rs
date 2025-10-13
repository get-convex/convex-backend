use anyhow::Context;
use common::json::JsonForm as _;
use model::{
    modules::function_validators::ReturnsValidator,
    virtual_system_mapping,
};
use serde_json::{
    json,
    Value as JsonValue,
};

use super::OpProvider;

#[convex_macro::v8_op]
pub fn op_validate_returns<'b, P: OpProvider<'b>>(
    provider: &mut P,
    validator: String,
    function_result: String,
) -> anyhow::Result<JsonValue> {
    let returns_validator = match ReturnsValidator::json_deserialize(&validator) {
        Ok(v) => v,
        Err(json_error) => {
            let message =
                format!("Unable to parse JSON returned from `exportReturns`: {json_error}");
            return Err(anyhow::anyhow!(message));
        },
    };

    let function_result = value::json_deserialize(&function_result)?;

    let table_mapping = provider.get_all_table_mappings()?;
    match returns_validator.check_output(&function_result, &table_mapping, virtual_system_mapping())
    {
        Some(js_error) => Ok(json!({
            "valid": false,
            "message": format!("{}", js_error)
        })),
        None => Ok(json!({
            "valid": true,
        })),
    }
}

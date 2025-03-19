use std::collections::BTreeMap;

use common::{
    bootstrap_model::components::definition::ComponentDefinitionMetadata,
    components::ComponentDefinitionPath,
    errors::JsError,
};
use pb::common::{
    function_result::Result as FunctionResultTypeProto,
    FunctionResult as FunctionResultProto,
};
use serde_json::Value as JsonValue;
use value::ConvexValue;

pub type EvaluateAppDefinitionsResult =
    BTreeMap<ComponentDefinitionPath, ComponentDefinitionMetadata>;

#[derive(Clone, Debug)]
#[cfg_attr(
    any(test, feature = "testing"),
    derive(proptest_derive::Arbitrary, PartialEq)
)]
pub struct FunctionResult {
    pub result: Result<ConvexValue, JsError>,
}

impl TryFrom<FunctionResultProto> for FunctionResult {
    type Error = anyhow::Error;

    fn try_from(result: FunctionResultProto) -> anyhow::Result<Self> {
        let result = match result.result {
            Some(FunctionResultTypeProto::JsonPackedValue(value)) => {
                let json: JsonValue = serde_json::from_str(&value)?;
                let value = ConvexValue::try_from(json)?;
                Ok(value)
            },
            Some(FunctionResultTypeProto::JsError(js_error)) => Err(js_error.try_into()?),
            None => anyhow::bail!("Missing result"),
        };
        Ok(FunctionResult { result })
    }
}

impl TryFrom<FunctionResult> for FunctionResultProto {
    type Error = anyhow::Error;

    fn try_from(result: FunctionResult) -> anyhow::Result<Self> {
        let result = match result.result {
            Ok(value) => {
                let json = value.json_serialize()?;
                FunctionResultTypeProto::JsonPackedValue(json)
            },
            Err(js_error) => FunctionResultTypeProto::JsError(js_error.try_into()?),
        };
        Ok(FunctionResultProto {
            result: Some(result),
        })
    }
}

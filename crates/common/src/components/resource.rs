use serde::{
    Deserialize,
    Serialize,
};
use serde_json::Value as JsonValue;
use value::ConvexValue;

use super::{
    function_paths::SerializedComponentFunctionPath,
    ComponentFunctionPath,
};

/// `Resource`s are resolved `Reference`s to objects within the components
/// data model. For now, we only have free standing `ConvexValue`s and
/// functions within a component.
#[derive(Debug, Clone, PartialEq, Eq, Ord, PartialOrd)]
#[cfg_attr(any(test, feature = "testing"), derive(proptest_derive::Arbitrary))]
pub enum Resource {
    Value(ConvexValue),
    Function(ComponentFunctionPath),
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum SerializedResource {
    #[serde(rename_all = "camelCase")]
    Value { value: String },
    #[serde(rename_all = "camelCase")]
    Function {
        path: SerializedComponentFunctionPath,
    },
}

impl TryFrom<Resource> for SerializedResource {
    type Error = anyhow::Error;

    fn try_from(r: Resource) -> anyhow::Result<Self> {
        match r {
            Resource::Value(v) => Ok(Self::Value {
                value: serde_json::to_string(&JsonValue::try_from(v)?)?,
            }),
            Resource::Function(path) => Ok(Self::Function {
                path: path.try_into()?,
            }),
        }
    }
}

impl TryFrom<SerializedResource> for Resource {
    type Error = anyhow::Error;

    fn try_from(r: SerializedResource) -> anyhow::Result<Self> {
        match r {
            SerializedResource::Value { value: s } => {
                let json_value = serde_json::from_str::<JsonValue>(&s)?;
                let value = ConvexValue::try_from(json_value)?;
                Ok(Self::Value(value))
            },
            SerializedResource::Function { path } => Ok(Self::Function(path.try_into()?)),
        }
    }
}

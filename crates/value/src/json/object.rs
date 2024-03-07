use serde_json::Value as JsonValue;

use crate::{
    ConvexObject,
    ConvexValue,
};

impl From<ConvexObject> for JsonValue {
    fn from(object: ConvexObject) -> Self {
        let v: serde_json::Map<_, _> = object
            .iter()
            .map(|(k, v)| (k.to_string(), JsonValue::from(v.clone())))
            .collect();
        JsonValue::Object(v)
    }
}

impl TryFrom<JsonValue> for ConvexObject {
    type Error = anyhow::Error;

    fn try_from(object: JsonValue) -> anyhow::Result<Self> {
        ConvexValue::try_from(object)?.try_into()
    }
}

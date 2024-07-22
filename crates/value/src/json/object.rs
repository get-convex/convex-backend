use serde_json::Value as JsonValue;

use crate::{
    ConvexObject,
    ConvexValue,
};

impl From<ConvexObject> for JsonValue {
    fn from(object: ConvexObject) -> Self {
        (&object).into()
    }
}

impl From<&ConvexObject> for JsonValue {
    fn from(object: &ConvexObject) -> Self {
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

pub mod convex_object_json_serializer {
    use serde::{
        Deserialize,
        Deserializer,
        Serialize,
        Serializer,
    };
    use serde_json::Value as JsonValue;

    use super::ConvexObject;

    pub fn serialize<S>(obj: &ConvexObject, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let value = JsonValue::from(obj);
        value.serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<ConvexObject, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = JsonValue::deserialize(deserializer)?;
        ConvexObject::try_from(value).map_err(serde::de::Error::custom)
    }
}

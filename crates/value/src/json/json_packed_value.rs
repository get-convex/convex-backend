use std::sync::Arc;

#[cfg(any(test, feature = "testing"))]
use proptest::prelude::*;
use serde_json::Value as JsonValue;

use super::json_deserialize;
use crate::{
    heap_size::HeapSize,
    ConvexValue,
};

#[derive(Clone, Debug)]
#[cfg_attr(any(test, feature = "testing"), derive(PartialEq))]
pub struct JsonPackedValue(Arc<str>);

impl JsonPackedValue {
    pub fn pack(value: ConvexValue) -> Self {
        let serialized = value
            .json_serialize()
            .expect("Failed to serialize to string");
        Self(serialized.into())
    }

    pub fn unpack(&self) -> ConvexValue {
        ConvexValue::try_from(self.json_value()).expect("Parsed JSON value wasn't a valid Value")
    }

    pub fn json_value(&self) -> JsonValue {
        serde_json::from_str(&self.0).expect("Failed to deserialize packed JSON value")
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn from_network(json: String) -> anyhow::Result<Self> {
        // TODO: just check JSON validity & size/depth constraints, then pass
        // the string data through
        json_deserialize(&json).map(Self::pack)
    }
}

impl HeapSize for JsonPackedValue {
    fn heap_size(&self) -> usize {
        self.0.len()
    }
}

// TODO: This impl is only needed to serialize sync protocol messages. Ideally
// that should transfer the JSON data as-is without parsing it into an
// intermediate data structure.
impl From<JsonPackedValue> for JsonValue {
    fn from(value: JsonPackedValue) -> Self {
        value.json_value()
    }
}

#[cfg(any(test, feature = "testing"))]
impl Arbitrary for JsonPackedValue {
    type Parameters = ();

    type Strategy = impl Strategy<Value = JsonPackedValue>;

    fn arbitrary_with(_args: Self::Parameters) -> Self::Strategy {
        any::<ConvexValue>().prop_map(JsonPackedValue::pack)
    }
}

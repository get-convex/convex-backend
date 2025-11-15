use std::sync::Arc;

use anyhow::Context as _;
#[cfg(any(test, feature = "testing"))]
use proptest::prelude::*;
use serde_json::Value as JsonValue;

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

    /// This should never return an error, but in theory
    /// `JsonPackedValue::from_network` could accept an invalid value
    pub fn unpack(&self) -> anyhow::Result<ConvexValue> {
        ConvexValue::try_from(self.json_value())
            .context("JsonPackedValue wasn't a valid ConvexValue")
    }

    pub fn json_value(&self) -> JsonValue {
        serde_json::from_str(&self.0).expect("Failed to deserialize packed JSON value")
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn from_network(json: String) -> anyhow::Result<Self> {
        // TODO: consider checking JSON validity here
        Ok(Self(json.into()))
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

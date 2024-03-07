use std::sync::Arc;

use common::value::{
    ConvexValue,
    Size,
};
#[cfg(any(test, feature = "testing"))]
use proptest::prelude::*;
use serde_json::Value as JsonValue;
use value::heap_size::HeapSize;

#[derive(Clone, Debug)]
#[cfg_attr(any(test, feature = "testing"), derive(PartialEq))]
pub struct JsonPackedValue(Arc<str>);

impl JsonPackedValue {
    pub fn pack(value: ConvexValue) -> Self {
        let serialized =
            serde_json::to_string(&JsonValue::from(value)).expect("Failed to serialize to string");
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
}

impl HeapSize for JsonPackedValue {
    fn heap_size(&self) -> usize {
        self.0.len()
    }
}

impl Size for JsonPackedValue {
    fn size(&self) -> usize {
        self.0.len()
    }

    fn nesting(&self) -> usize {
        0
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

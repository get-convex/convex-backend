mod de;
mod ser;
mod value;

pub use de::{
    from_object,
    from_value,
};
pub use ser::{
    to_object,
    to_value,
};
use serde::{
    Deserialize,
    Serialize,
};

use crate::ConvexObject;

#[macro_export]
macro_rules! codegen_convex_serialization {
    ($struct:ident, $serialized_struct:ident) => {
        codegen_convex_serialization!($struct, $serialized_struct, test_cases = 256);
    };
    ($struct:ident, $serialized_struct:ident, test_cases = $test_cases:expr) => {
        impl TryFrom<$struct> for value::ConvexObject {
            type Error = anyhow::Error;

            fn try_from(s: $struct) -> anyhow::Result<value::ConvexObject> {
                value::serde::to_object($serialized_struct::try_from(s)?)
            }
        }

        impl TryFrom<$struct> for value::ConvexValue {
            type Error = anyhow::Error;

            fn try_from(s: $struct) -> anyhow::Result<value::ConvexValue> {
                Ok(value::ConvexObject::try_from(s)?.into())
            }
        }

        impl TryFrom<value::ConvexObject> for $struct {
            type Error = anyhow::Error;

            fn try_from(s: value::ConvexObject) -> anyhow::Result<$struct> {
                value::serde::from_object::<$serialized_struct>(s)?.try_into()
            }
        }

        impl TryFrom<value::ConvexValue> for $struct {
            type Error = anyhow::Error;

            fn try_from(s: value::ConvexValue) -> anyhow::Result<$struct> {
                value::ConvexObject::try_from(s)?.try_into()
            }
        }

        $crate::paste! {
            #[cfg(test)]
            mod [<roundtrip_test_ $struct:snake:lower>] {
                use cmd_util::env::env_config;
                use proptest::prelude::*;

                use super::$struct;

                // TODO: For some reason, `proptest!` isn't usable from within this macro.
                #[test]
                #[allow(non_snake_case)]
                fn $struct() {
                    let mut config = ProptestConfig {
                        cases: $test_cases * env_config("CONVEX_PROPTEST_MULTIPLIER", 1),
                        failure_persistence: None,
                        ..ProptestConfig::default()
                    };
                    config.test_name = Some(concat!(module_path!(), "::test_roundtrips"));
                    proptest::test_runner::TestRunner::new(config)
                        .run(&any::<$struct>(), |left| {
                            let right = $struct::try_from(
                                value::ConvexObject::try_from(left.clone()).unwrap()
                            ).unwrap();
                            prop_assert_eq!(left, right);
                            Ok(())
                        })
                        .unwrap();
                }
            }
        }
    };
}

/// For forwards compatibility on enums, it's often useful to preserve an
/// unknown variant as a raw `ConvexObject`. To do so, wrap your enum in this
/// struct:
/// ```ignore,rust
/// #[derive(Serialize, Deserialize)]
/// struct SerializedStruct {
///     state: WithUnknown<SerializedEnum>,
///     another_field: String,
/// }
///
/// #[derive(Serialize, Deserialize)]
/// #[serde(tag = "type")]
/// enum SerializedEnum {
///     Variant1 {
///        field: i32,
///     },
///     Variant2 {
///        another_field: String,
///     },
/// }
/// ```
/// With this setup, `state` will be `WithUnknown::Unknown` when
/// `SerializedEnum` fails to deserialize, so we can preserve an unknown variant
/// for forwards compatibility.
#[derive(Serialize, Deserialize)]
#[serde(untagged)]
pub enum WithUnknown<T> {
    Known(T),
    Unknown(ConvexObject),
}

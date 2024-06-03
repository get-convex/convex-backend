use proptest::{
    prelude::*,
    prop_oneof,
};
use prost_types::Timestamp;

use crate::fivetran_sdk::value_type::Inner as FivetranValue;

const NS_IN_MS: i32 = 1_000_000;

pub fn arbitrary_timestamp_strategy() -> impl Strategy<Value = prost_types::Timestamp> {
    (-318384000..2206224000i64, 0..=999_999_999i32).prop_map(|(seconds, nanos)| {
        Timestamp {
            // Fivetran timestamps have millisecond precision
            // https://github.com/fivetran/fivetran_sdk/blob/main/development-guide.md#truncate
            nanos: nanos / NS_IN_MS * NS_IN_MS,
            seconds,
        }
    })
}

impl Arbitrary for FivetranValue {
    type Parameters = ();

    type Strategy = impl Strategy<Value = FivetranValue>;

    fn arbitrary_with(_args: Self::Parameters) -> Self::Strategy {
        prop_oneof![
            Just(FivetranValue::Null(true)),
            any::<bool>().prop_map(FivetranValue::Bool),
            any::<i32>().prop_map(FivetranValue::Short),
            any::<i32>().prop_map(FivetranValue::Int),
            any::<i64>().prop_map(FivetranValue::Long),
            any::<f32>().prop_map(FivetranValue::Float),
            any::<f64>().prop_map(FivetranValue::Double),
            any::<f64>().prop_map(|v| FivetranValue::Decimal(v.to_string())),
            any::<String>().prop_map(FivetranValue::String),
            any::<String>().prop_map(FivetranValue::Json),
            any::<String>().prop_map(FivetranValue::Xml),
            any::<Vec<u8>>().prop_map(FivetranValue::Binary),
            arbitrary_timestamp_strategy().prop_map(|Timestamp { seconds, .. }| {
                FivetranValue::NaiveDate(Timestamp {
                    seconds: seconds - seconds % (60 * 60 * 24),
                    nanos: 0,
                })
            }),
            arbitrary_timestamp_strategy().prop_map(|Timestamp { seconds, nanos }| {
                FivetranValue::NaiveDatetime(Timestamp { seconds, nanos })
            }),
            (0i64..60 * 60 * 24, 0..1000).prop_map(|(seconds, milliseconds)| {
                FivetranValue::NaiveTime(Timestamp {
                    seconds,
                    nanos: milliseconds * NS_IN_MS,
                })
            }),
            arbitrary_timestamp_strategy().prop_map(FivetranValue::UtcDatetime),
        ]
    }
}

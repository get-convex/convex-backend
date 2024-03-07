use serde_json::json;

use crate::ConvexValue;

#[test]
fn test_duplicates() {
    let duplicate_set = json!({"$set": [{}, {}]});
    let err = ConvexValue::try_from(duplicate_set).unwrap_err();
    assert!(format!("{err:?}").contains("Duplicate value"));

    let duplicate_set = json!({"$map": [[{}, 1], [{}, 2]]});
    let err = ConvexValue::try_from(duplicate_set).unwrap_err();
    assert!(format!("{err:?}").contains("Duplicate key"));
}

#[test]
fn test_unrecognized_system_key() {
    let one_key = json!({"$unrecognized": {}});
    let err = ConvexValue::try_from(one_key).unwrap_err();
    assert!(
        format!("{err:?}").contains("starts with '$', which is reserved"),
        "{err:?}"
    );

    let two_keys = json!({"okay": {}, "$notOkay": {}});
    let err = ConvexValue::try_from(two_keys).unwrap_err();
    assert!(
        format!("{err:?}").contains("starts with '$', which is reserved"),
        "{err:?}"
    );
}

mod json_serialize_roundtrip {
    use proptest::prelude::*;
    use serde_json::Value as JsonValue;

    use crate::{
        ConvexValue,
        ExcludeSetsAndMaps,
    };

    fn test(left: ConvexValue) -> anyhow::Result<()> {
        let json_value = JsonValue::from(left.clone());
        let string = serde_json::to_string(&json_value)?;
        let json_value_from_string: JsonValue = serde_json::from_str(&string)?;
        let right = ConvexValue::try_from(json_value_from_string).unwrap();
        assert_eq!(left, right);

        let reserialized = serde_json::to_string(&JsonValue::from(right))?;
        assert_eq!(string, reserialized);

        Ok(())
    }

    proptest! {
        #![proptest_config(
            ProptestConfig { failure_persistence: None, ..ProptestConfig::default() }
        )]

        #[test]
        fn proptest_json_serialize_roundtrip(v in any::<ConvexValue>()) {
            test(v).unwrap()
        }

        #[test]
        fn proptest_json_serialize_roundtrip_to_client(
            v in any_with::<ConvexValue>((Default::default(), ExcludeSetsAndMaps(true)))
        ) {
            let json_value: JsonValue = v.clone().into();
            let client_value: convex::Value = json_value.try_into().unwrap();
            let json_return: JsonValue = client_value.clone().into();
            let roundtripped_value: ConvexValue = json_return.try_into().unwrap();
            assert_eq!(v, roundtripped_value);

            // Send it back to client - to make sure it works the other way
            let resend_json_value: JsonValue = roundtripped_value.into();
            let resend_client_value: convex::Value = resend_json_value.try_into().unwrap();
            assert_eq!(client_value, resend_client_value);

        }
    }

    #[test]
    fn proptest_trophies() -> anyhow::Result<()> {
        let values = [ConvexValue::from(3.9466996405145095e-59)];
        for value in values {
            test(value)?;
        }
        Ok(())
    }
}

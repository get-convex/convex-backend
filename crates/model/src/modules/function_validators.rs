use common::{
    errors::JsError,
    schemas::validator::{
        ObjectValidator,
        Validator,
    },
};
use errors::ErrorMetadataAnyhowExt;
#[cfg(any(test, feature = "testing"))]
use proptest::prelude::*;
use serde_json::Value as JsonValue;
use value::{
    ConvexArray,
    ConvexValue,
    TableMapping,
    VirtualTableMapping,
};

/**
 * A validator for the arguments to a UDF.
 */
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(any(test, feature = "testing"), derive(proptest_derive::Arbitrary))]
pub enum ArgsValidator {
    Unvalidated,
    #[cfg_attr(
        any(test, feature = "testing"),
        proptest(strategy = "prop::collection::btree_set(any::<value::TableName>(), \
                             1..8).prop_flat_map(any_with::<ObjectValidator>).\
                             prop_map(ArgsValidator::Validated)")
    )]
    Validated(ObjectValidator),
}

impl ArgsValidator {
    pub fn check_args(
        &self,
        args: &ConvexArray,
        table_mapping: &TableMapping,
        virtual_table_mapping: &VirtualTableMapping,
    ) -> anyhow::Result<Option<JsError>> {
        let result = match self {
            ArgsValidator::Unvalidated => None,
            ArgsValidator::Validated(object_validator) => {
                let single_arg = match &args[..] {
                    [arg] => arg,
                    _ => {
                        let error_message = format!(
                            "Expected to receive a single object as the function's argument. \
                             Instead received {} arguments: {args}",
                            args.len()
                        );
                        return Ok(Some(JsError::from_message(error_message)));
                    },
                };
                let object_arg = match single_arg {
                    ConvexValue::Object(o) => o,
                    _ => {
                        let error_message = format!(
                            "Expected to receive an object as the function's argument. Instead \
                             received: {single_arg}"
                        );
                        return Ok(Some(JsError::from_message(error_message)));
                    },
                };

                let validation_error = Validator::Object(object_validator.clone()).check_value(
                    &ConvexValue::Object(object_arg.clone()),
                    table_mapping,
                    virtual_table_mapping,
                );
                if let Err(error) = validation_error {
                    Some(JsError::from_message(error.to_string()))
                } else {
                    None
                }
            },
        };
        Ok(result)
    }
}

impl TryFrom<JsonValue> for ArgsValidator {
    type Error = anyhow::Error;

    fn try_from(json: JsonValue) -> Result<Self, Self::Error> {
        let args = match Validator::try_from(json).map_err(|e| {
            e.wrap_error_message(|msg| {
                format!("Error in args validator: {msg}\n\
                    See https://docs.convex.dev/functions/args-validation for \
                    docs on how to do argument validation.")
            })
        })? {
            Validator::Object(o) => ArgsValidator::Validated(o),
            Validator::Any => ArgsValidator::Unvalidated,
            _ => anyhow::bail!("Args validator must be an object or any"),
        };
        Ok(args)
    }
}

impl TryFrom<ArgsValidator> for JsonValue {
    type Error = anyhow::Error;

    fn try_from(args: ArgsValidator) -> Result<Self, Self::Error> {
        let validator = match args {
            ArgsValidator::Unvalidated => Validator::Any,
            ArgsValidator::Validated(args_schema) => Validator::Object(args_schema),
        };

        JsonValue::try_from(validator)
    }
}

/**
 * A validator for the return value of a UDF.
 */
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(any(test, feature = "testing"), derive(proptest_derive::Arbitrary))]
pub enum ReturnsValidator {
    Unvalidated,
    #[cfg_attr(
        any(test, feature = "testing"),
        proptest(strategy = "prop::collection::btree_set(any::<value::TableName>(), \
                             1..8).prop_flat_map(any_with::<Validator>).\
                             prop_map(ReturnsValidator::Validated)")
    )]
    Validated(Validator),
}

impl ReturnsValidator {
    pub fn check_output(
        &self,
        output: &ConvexValue,
        table_mapping: &TableMapping,
        virtual_table_mapping: &VirtualTableMapping,
    ) -> Option<JsError> {
        match self {
            ReturnsValidator::Unvalidated => None,
            ReturnsValidator::Validated(validator) => {
                let validation_error =
                    validator.check_value(output, table_mapping, virtual_table_mapping);
                match validation_error {
                    Err(error) => Some(JsError::from_message(format!(
                        "ReturnsValidationError: {error}"
                    ))),
                    Ok(()) => None,
                }
            },
        }
    }
}

impl TryFrom<JsonValue> for ReturnsValidator {
    type Error = anyhow::Error;

    fn try_from(json: JsonValue) -> Result<Self, Self::Error> {
        Ok(match json {
            JsonValue::Null => ReturnsValidator::Unvalidated,
            json => ReturnsValidator::Validated(Validator::try_from(json).map_err(|e| {
                e.wrap_error_message(|msg| {
                    format!("Error in returns validator: {msg}\n\
                            See https://docs.convex.dev/functions/args-validation for \
                            docs on how to do return value validation.")
                })
            })?),
        })
    }
}

impl TryFrom<ReturnsValidator> for JsonValue {
    type Error = anyhow::Error;

    fn try_from(returns: ReturnsValidator) -> Result<Self, Self::Error> {
        match returns {
            ReturnsValidator::Unvalidated => Ok(JsonValue::Null),
            ReturnsValidator::Validated(output_schema) => JsonValue::try_from(output_schema),
        }
    }
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;
    use serde_json::Value as JsonValue;
    use sync_types::testing::assert_roundtrips;

    use crate::modules::function_validators::{
        ArgsValidator,
        ReturnsValidator,
    };

    proptest! {
        #![proptest_config(
            ProptestConfig { failure_persistence: None, ..ProptestConfig::default() }
        )]
        #[test]
        fn test_args_roundtrips(v in any::<ArgsValidator>()) {
            assert_roundtrips::<ArgsValidator, JsonValue>(v);
        }
    }

    proptest! {
        #![proptest_config(
            ProptestConfig { failure_persistence: None, ..ProptestConfig::default() }
        )]
        #[test]
        fn test_returns_roundtrips(v in any::<ReturnsValidator>()) {
            assert_roundtrips::<ReturnsValidator, JsonValue>(v);
        }
    }
}

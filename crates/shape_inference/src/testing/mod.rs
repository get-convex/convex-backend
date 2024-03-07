pub mod arbitrary_export_context;
pub mod arbitrary_shape;
pub mod arbitrary_value;

use proptest::prelude::*;
use value::IdentifierFieldName;

use crate::ShapeConfig;

#[derive(Copy, Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum TestConfig {}

impl ShapeConfig for TestConfig {
    const MAX_OBJECT_FIELDS: usize = 4;
    const MAX_UNION_LENGTH: usize = 4;

    fn is_valid_string_literal(s: &str) -> bool {
        s.len() <= 4 && s.chars().all(|c| c.is_ascii_alphabetic())
    }

    fn allow_optional_object_fields() -> bool {
        true
    }

    #[cfg(any(test, feature = "testing"))]
    fn string_literal_strategy() -> proptest::strategy::BoxedStrategy<String> {
        "[a-z]{0,4}".prop_map(String::from).boxed()
    }

    #[cfg(any(test, feature = "testing"))]
    fn object_field_strategy() -> proptest::strategy::BoxedStrategy<IdentifierFieldName> {
        "[a-z]{1,3}".prop_map(|s| s.parse().unwrap()).boxed()
    }
}

#[derive(Copy, Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum SmallTestConfig {}

impl ShapeConfig for SmallTestConfig {
    const MAX_OBJECT_FIELDS: usize = 2;
    const MAX_UNION_LENGTH: usize = 2;

    fn is_valid_string_literal(s: &str) -> bool {
        s.len() <= 4 && s.chars().all(|c| c.is_ascii_alphabetic())
    }

    fn allow_optional_object_fields() -> bool {
        true
    }

    #[cfg(any(test, feature = "testing"))]
    fn string_literal_strategy() -> proptest::strategy::BoxedStrategy<String> {
        "[a-z]{0,4}".prop_map(String::from).boxed()
    }

    #[cfg(any(test, feature = "testing"))]
    fn object_field_strategy() -> proptest::strategy::BoxedStrategy<IdentifierFieldName> {
        "[a-z]{1,3}".prop_map(|s| s.parse().unwrap()).boxed()
    }
}

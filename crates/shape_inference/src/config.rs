use std::fmt::Debug;

use value::identifier::is_valid_identifier;
#[cfg(any(test, feature = "testing"))]
use value::IdentifierFieldName;

/// Static config for the shape inference algorithm. This is useful for tests
/// and updating the shape inference algorithm over time.
pub trait ShapeConfig: Copy + Clone + Debug + Eq + Ord + PartialEq + PartialOrd + 'static {
    const MAX_UNION_LENGTH: usize;
    const MAX_OBJECT_FIELDS: usize;

    fn is_valid_string_literal(s: &str) -> bool;

    #[cfg(any(test, feature = "testing"))]
    fn string_literal_strategy() -> proptest::strategy::BoxedStrategy<String>;
    #[cfg(any(test, feature = "testing"))]
    fn object_field_strategy() -> proptest::strategy::BoxedStrategy<IdentifierFieldName>;
}

#[derive(Copy, Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum ProdConfig {}

impl ShapeConfig for ProdConfig {
    const MAX_OBJECT_FIELDS: usize = 64;
    const MAX_UNION_LENGTH: usize = 16;

    fn is_valid_string_literal(s: &str) -> bool {
        is_valid_identifier(s)
    }

    #[cfg(any(test, feature = "testing"))]
    fn string_literal_strategy() -> proptest::strategy::BoxedStrategy<String> {
        use proptest::prelude::*;
        use value::identifier::arbitrary_regexes::IDENTIFIER_REGEX;
        IDENTIFIER_REGEX.boxed()
    }

    #[cfg(any(test, feature = "testing"))]
    fn object_field_strategy() -> proptest::strategy::BoxedStrategy<IdentifierFieldName> {
        use proptest::prelude::*;
        any::<IdentifierFieldName>().boxed()
    }
}

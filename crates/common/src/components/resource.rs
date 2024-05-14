use value::ConvexValue;

use super::ComponentFunctionPath;

/// `Resource`s are resolved `Reference`s to objects within the components
/// data model. For now, we only have free standing `ConvexValue`s and
/// functions within a component.
#[derive(Clone, PartialEq, Eq, Ord, PartialOrd)]
#[cfg_attr(
    any(test, feature = "testing"),
    derive(Debug, proptest_derive::Arbitrary)
)]
pub enum Resource {
    Value(ConvexValue),
    Function(ComponentFunctionPath),
}

use std::{
    marker::PhantomData,
    ops::Deref,
};

use value::{
    id_v6::DeveloperDocumentId,
    identifier::is_valid_field_name,
    ConvexString,
};

use super::{
    config::ShapeConfig,
    ShapeEnum,
};
use crate::ShapeCounter;

/// String literal shape, constrained by
/// [`ShapeConfig::is_valid_string_literal`].
///
/// String literals that are valid `Id`s in some table `t` are subtypes of
/// `id<t>`, and all string literals and `id`s are subtypes of `string`.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct StringLiteralShape<C: ShapeConfig> {
    pub literal: ConvexString,
    _cfg: PhantomData<C>,
}

impl<C: ShapeConfig> StringLiteralShape<C> {
    pub fn shape_of<S: ShapeCounter>(s: &str) -> ShapeEnum<C, S> {
        if C::is_valid_string_literal(s) {
            let literal_shape = StringLiteralShape {
                literal: s
                    .to_string()
                    .try_into()
                    .expect("String literal was not valid Value::String"),
                _cfg: PhantomData,
            };
            return ShapeEnum::StringLiteral(literal_shape);
        }
        if let Ok(id) = DeveloperDocumentId::decode(s) {
            return ShapeEnum::Id(*id.table());
        }
        if is_valid_field_name(s) {
            return ShapeEnum::FieldName;
        }
        ShapeEnum::String
    }
}

impl<C: ShapeConfig> Deref for StringLiteralShape<C> {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.literal[..]
    }
}

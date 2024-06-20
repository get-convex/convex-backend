use std::collections::BTreeMap;

use value::{
    id_v6::DeveloperDocumentId,
    FieldName,
};

use super::{
    array::ArrayShape,
    config::ShapeConfig,
    map::MapShape,
    object::{
        ObjectShape,
        RecordShape,
    },
    set::SetShape,
    string::StringLiteralShape,
    union::UnionBuilder,
    ShapeEnum,
};
use crate::{
    object::ObjectField,
    CountedShape,
    ShapeCounter,
};

impl<C: ShapeConfig, S: ShapeCounter> ShapeEnum<C, S> {
    /// Quickly compute if one type is a subtype of another. Other than avoiding
    /// creating a new `Type`, this shortcut is useful for situations where
    /// comparing multisets doesn't make sense.
    pub fn is_subtype(&self, other: &Self) -> bool {
        match (self, other) {
            // `never` is a subtype of every other type.
            (ShapeEnum::Never, _) => true,

            // Primitive types without nontrivial subtyping.
            (ShapeEnum::Null, ShapeEnum::Null) => true,
            (ShapeEnum::Int64, ShapeEnum::Int64) => true,
            (ShapeEnum::Boolean, ShapeEnum::Boolean) => true,
            (ShapeEnum::Bytes, ShapeEnum::Bytes) => true,

            // Two string literal types are subtypes if they're equal.
            (ShapeEnum::StringLiteral(ref s), ShapeEnum::StringLiteral(ref other_s)) => {
                s[..] == other_s[..]
            },
            // A string literal type is a subtype of an `id<t>` type if it's a valid ID in `t`.
            (ShapeEnum::StringLiteral(ref s), ShapeEnum::Id(table_number)) => {
                match DeveloperDocumentId::decode(s) {
                    Ok(id) => id.table() == *table_number,
                    Err(_) => false,
                }
            },
            // A string literal type is a subtype of `field_name` if its string is a valid field
            // name.
            (ShapeEnum::StringLiteral(ref s), ShapeEnum::FieldName) => {
                s.parse::<FieldName>().is_ok()
            },
            // All string literal types are subtypes of `string`.
            (ShapeEnum::StringLiteral(_), ShapeEnum::String) => true,
            // `id` types are subtypes if they're equal.
            (ShapeEnum::Id(table), ShapeEnum::Id(other_table)) => table == other_table,
            // All Ids are valid field names.
            (ShapeEnum::Id(_), ShapeEnum::FieldName) => true,
            // All `id` types are subtypes of string.
            (ShapeEnum::Id(_), ShapeEnum::String) => true,
            // The `field_name` type is a subtype of itself.
            (ShapeEnum::FieldName, ShapeEnum::FieldName) => true,
            // All field names are valid strings.
            (ShapeEnum::FieldName, ShapeEnum::String) => true,
            // The `string` type is a subtype of itself.
            (ShapeEnum::String, ShapeEnum::String) => true,

            // -inf is a subtype of itself
            (ShapeEnum::NegativeInf, ShapeEnum::NegativeInf) => true,
            // -inf is a subtype of all float64s
            (ShapeEnum::NegativeInf, ShapeEnum::Float64) => true,
            // inf is a subtype of itself
            (ShapeEnum::PositiveInf, ShapeEnum::PositiveInf) => true,
            // inf is a subtype of all float64s
            (ShapeEnum::PositiveInf, ShapeEnum::Float64) => true,
            // -0 is a subtype of itself
            (ShapeEnum::NegativeZero, ShapeEnum::NegativeZero) => true,
            // -0 is a subtype of all float64s
            (ShapeEnum::NegativeZero, ShapeEnum::Float64) => true,
            // NaN is a subtype of itself
            (ShapeEnum::NaN, ShapeEnum::NaN) => true,
            // NaN is a subtype of all float64s
            (ShapeEnum::NaN, ShapeEnum::Float64) => true,
            // normal float64 is a subtype of itself
            (ShapeEnum::NormalFloat64, ShapeEnum::NormalFloat64) => true,
            // normal float64 is a subtype of all float64s
            (ShapeEnum::NormalFloat64, ShapeEnum::Float64) => true,
            // The `float64` type is a subtype of itself.
            (ShapeEnum::Float64, ShapeEnum::Float64) => true,

            // Covariance: `array<t> <= array<u>` if `t <= u`. Intuitively, the set of all arrays
            // with elements in `u` contains the set of all arrays with elements in `t` when `t` is
            // a subset of `u`.
            (ShapeEnum::Array(ref array), ShapeEnum::Array(ref other_array)) => array
                .element()
                .variant
                .is_subtype(&other_array.element().variant),
            (ShapeEnum::Set(ref set), ShapeEnum::Set(ref other_set)) => set
                .element()
                .variant
                .is_subtype(&other_set.element().variant),
            (ShapeEnum::Map(ref map), ShapeEnum::Map(ref other_map)) => {
                map.key().variant.is_subtype(&other_map.key().variant)
                    && map.value().variant.is_subtype(&other_map.value().variant)
            },

            // We do not perform any structural subtyping in our type system, so the set of all
            // objects in `{a: string}` is completely disjoint from the set of all objects in `{a:
            // string, b: string}`. This differs from, say, TypeScript, where `{a: string}` is the
            // set of all values that have at least the field `a: string`, so `{a:
            // string, b: string} <= {a: string}`.
            //
            // Object types with the same fields are covariant: {f_1: t_1, ..., f_n: t_n} <= {f_1:
            // u_1, ..., f_n: u_n}` if all `t_i <= u_i`.
            //
            // Optional fields are defined in terms of unions: `{f_1?: t_1, ..., f_n: t_n}` is
            // `{f_1: t_1, ..., f_n: t_n} | {f_2: t_1, ..., f_n: t_n}`. We can then define subtyping
            // on objects with optional fields using the union rules below: A union type `u_1 | ...
            // | u_n` is a subtype of a union type `v_1 | ... | v_m` if for every `u_i`
            // there exists some `v_m` such that `u_i <= v_m`.
            //
            // In practice, however, this approach is exponential in the number of optional fields.
            // For example, if we're checking whether `{f_1?: t_1, ..., f_n?: t_n}` is a subtype of
            // `{g_1?: u_1, ..., g_m?: t_m}`, the left and right unions will have `2^n` and `2^m`
            // members, respectively. Checking whether any type on the left is a subtype of a type
            // on the right will then have `2^(n + m)` subtype checks.
            //
            // We can optimize this into a single pass by checking three conditions:
            // 1. For every required field in the left type, check whether it's an optional or
            // required field in the right type. Every expanded object type in the left union will
            // have this field, and we only need it to appear in at least one type in the right
            // union, so it's okay for it to be optional on the right.
            // 2. For every optional field in the left type, check that it's optional in the right
            // type. By being optional in the left type, we'll have types in the left union that
            // both have the field and do not. So, it cannot be required in the right type.
            // 3. Check that every field in the right type that's missing in the left type is
            // optional.
            //
            // Put another way, for two object types `t` and `u`, we have `t <= u` when
            // 1. `t`'s fields are a subset of `u`'s,
            // 2. all of the fields only in `u` are optional,
            // 3. every field optional in `t` is optional in `u`.
            (ShapeEnum::Object(ref object), ShapeEnum::Object(ref other_object)) => {
                for (field_name, field) in object.iter() {
                    let other_field = match other_object.get(field_name) {
                        Some(other_field) => other_field,
                        None => return false,
                    };
                    if field.optional && !other_field.optional {
                        return false;
                    }
                    if !field
                        .value_shape
                        .variant
                        .is_subtype(&other_field.value_shape.variant)
                    {
                        return false;
                    }
                }
                for (field_name, field) in other_object.iter() {
                    if !object.contains_key(field_name) && !field.optional {
                        return false;
                    }
                }
                true
            },
            // Object types are subtypes of a record type if their fields (interpreted as
            // `Value::String`s) and value types are subtypes of the record's field and value types.
            (ShapeEnum::Object(ref object), ShapeEnum::Record(ref record)) => {
                for (field_name, field) in object.iter() {
                    let field_name_type = StringLiteralShape::shape_of(&field_name[..]);
                    if !field_name_type.is_subtype(&record.field().variant) {
                        return false;
                    }
                    if !field
                        .value_shape
                        .variant
                        .is_subtype(&record.value().variant)
                    {
                        return false;
                    }
                }
                true
            },
            // Like array types, record types are covariant in their field and value types.
            (ShapeEnum::Record(ref record), ShapeEnum::Record(ref other_record)) => {
                record
                    .field()
                    .variant
                    .is_subtype(&other_record.field().variant)
                    && record
                        .value()
                        .variant
                        .is_subtype(&other_record.value().variant)
            },

            // A union type `u_1 | ... | u_n` is a subtype of `t` if all `u_i <= t`.
            (ShapeEnum::Union(ref union), _) => union.iter().all(|v| v.variant.is_subtype(other)),

            // A type `t` is a subtype of a union type `u_1 | ... | u_n` if `t <= u_i` for some `i`.
            (_, ShapeEnum::Union(ref union)) => union.iter().any(|v| self.is_subtype(&v.variant)),

            // Every type is a subtype of the `unknown` type.
            (_, ShapeEnum::Unknown) => true,

            _ => false,
        }
    }
}

impl<C: ShapeConfig> CountedShape<C> {
    pub fn merge_if_subtype(&self, other: &Self) -> Option<Self> {
        // See [`TypeEnum::is_subtype`] above for general comments on subtyping. The
        // comments inline here describe multiset-specific parts of the
        // algorithm. In particular, we have to be careful to never "lose" a value when
        // adjusting our `num_values` bookkeeping.
        //
        // Match over both types' variants, falling through if we'll create a new type
        // by just adding the two types' `num_values` together. Early return with `None`
        // if the two types are not subtypes of each other.
        let variant = match (&*self.variant, &*other.variant) {
            (ShapeEnum::Never, other_variant) => other_variant.clone(),
            (ShapeEnum::Null, ShapeEnum::Null) => ShapeEnum::Null,
            (ShapeEnum::Int64, ShapeEnum::Int64) => ShapeEnum::Int64,
            (ShapeEnum::Boolean, ShapeEnum::Boolean) => ShapeEnum::Boolean,
            (ShapeEnum::Bytes, ShapeEnum::Bytes) => ShapeEnum::Bytes,

            (ShapeEnum::StringLiteral(ref s), ShapeEnum::StringLiteral(ref other_s)) => {
                if s[..] != other_s[..] {
                    return None;
                }
                ShapeEnum::StringLiteral(s.clone())
            },
            (ShapeEnum::StringLiteral(ref s), ShapeEnum::Id(table_number)) => {
                let Ok(id) = DeveloperDocumentId::decode(s) else {
                    return None;
                };
                if id.table() != *table_number {
                    return None;
                }
                ShapeEnum::Id(*table_number)
            },
            (ShapeEnum::StringLiteral(ref s), ShapeEnum::FieldName) => {
                if s.parse::<FieldName>().is_err() {
                    return None;
                }
                ShapeEnum::FieldName
            },
            (ShapeEnum::StringLiteral(_), ShapeEnum::String) => ShapeEnum::String,
            (ShapeEnum::Id(table), ShapeEnum::Id(other_table)) => {
                if table != other_table {
                    return None;
                }
                ShapeEnum::Id(*table)
            },
            (ShapeEnum::Id(_), ShapeEnum::FieldName) => ShapeEnum::FieldName,
            (ShapeEnum::Id(_), ShapeEnum::String) => ShapeEnum::String,
            (ShapeEnum::FieldName, ShapeEnum::FieldName) => ShapeEnum::FieldName,
            (ShapeEnum::FieldName, ShapeEnum::String) => ShapeEnum::String,
            (ShapeEnum::String, ShapeEnum::String) => ShapeEnum::String,

            (ShapeEnum::NegativeInf, ShapeEnum::NegativeInf) => ShapeEnum::NegativeInf,
            (ShapeEnum::NegativeInf, ShapeEnum::Float64) => ShapeEnum::Float64,
            (ShapeEnum::PositiveInf, ShapeEnum::PositiveInf) => ShapeEnum::PositiveInf,
            (ShapeEnum::PositiveInf, ShapeEnum::Float64) => ShapeEnum::Float64,
            (ShapeEnum::NegativeZero, ShapeEnum::NegativeZero) => ShapeEnum::NegativeZero,
            (ShapeEnum::NegativeZero, ShapeEnum::Float64) => ShapeEnum::Float64,
            (ShapeEnum::NaN, ShapeEnum::NaN) => ShapeEnum::NaN,
            (ShapeEnum::NaN, ShapeEnum::Float64) => ShapeEnum::Float64,
            (ShapeEnum::NormalFloat64, ShapeEnum::NormalFloat64) => ShapeEnum::NormalFloat64,
            (ShapeEnum::NormalFloat64, ShapeEnum::Float64) => ShapeEnum::Float64,
            (ShapeEnum::Float64, ShapeEnum::Float64) => ShapeEnum::Float64,

            (ShapeEnum::Array(ref array), ShapeEnum::Array(ref other_array)) => {
                let element = array.element().merge_if_subtype(other_array.element())?;
                ShapeEnum::Array(ArrayShape::new(element))
            },
            (ShapeEnum::Set(ref set), ShapeEnum::Set(ref other_set)) => {
                let element = set.element().merge_if_subtype(other_set.element())?;
                ShapeEnum::Set(SetShape::new(element))
            },
            (ShapeEnum::Map(ref map), ShapeEnum::Map(ref other_map)) => {
                let key = map.key().merge_if_subtype(other_map.key())?;
                let value = map.value().merge_if_subtype(other_map.value())?;
                ShapeEnum::Map(MapShape::new(key, value))
            },
            (ShapeEnum::Object(ref object), ShapeEnum::Object(ref other_object)) => {
                let mut fields = BTreeMap::new();
                for (field_name, field) in object.iter() {
                    let other_field = other_object.get(field_name)?;
                    if field.optional && !other_field.optional {
                        return None;
                    }
                    let merged_field = ObjectField {
                        optional: other_field.optional,
                        value_shape: field
                            .value_shape
                            .merge_if_subtype(&other_field.value_shape)?,
                    };
                    fields.insert(field_name.clone(), merged_field);
                }
                for (field_name, field) in other_object.iter() {
                    if !object.contains_key(field_name) {
                        if !field.optional {
                            return None;
                        }
                        fields.insert(field_name.clone(), field.clone());
                    }
                }
                ShapeEnum::Object(ObjectShape::<C, u64>::new(fields))
            },
            (ShapeEnum::Object(ref object), ShapeEnum::Record(ref record)) => {
                let mut field = record.field().clone();
                let mut value = record.value().clone();
                for (field_name, object_field) in object.iter() {
                    let field_type = CountedShape::new(
                        StringLiteralShape::shape_of(&field_name[..]),
                        object_field.value_shape.num_values,
                    );
                    field = field_type.merge_if_subtype(&field)?;
                    value = object_field.value_shape.merge_if_subtype(&value)?;
                }
                ShapeEnum::Record(RecordShape::new(field, value))
            },
            (ShapeEnum::Record(ref record), ShapeEnum::Record(ref other_record)) => {
                let field = record.field().merge_if_subtype(other_record.field())?;
                let value = record.value().merge_if_subtype(other_record.value())?;
                ShapeEnum::Record(RecordShape::new(field, value))
            },
            (ShapeEnum::Union(ref union), _) => {
                let mut accumulated = other.clone();
                for union_type in union.iter() {
                    accumulated = union_type.merge_if_subtype(&accumulated)?;
                }
                return Some(accumulated);
            },
            (_, ShapeEnum::Union(ref union)) => {
                let mut accumulated = UnionBuilder::new();
                let mut found_supertype = false;
                for union_type in union.iter() {
                    if !found_supertype {
                        if let Some(merged_type) = self.merge_if_subtype(union_type) {
                            found_supertype = true;
                            accumulated = accumulated.push(merged_type);
                            continue;
                        }
                    }
                    accumulated = accumulated.push(union_type.clone());
                }
                if found_supertype {
                    return Some(accumulated.build());
                } else {
                    return None;
                }
            },
            (_, ShapeEnum::Unknown) => ShapeEnum::Unknown,
            _ => return None,
        };
        Some(Self::new(variant, self.num_values + other.num_values))
    }
}

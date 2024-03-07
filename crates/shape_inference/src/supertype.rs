use std::{
    collections::{
        BTreeMap,
        BTreeSet,
    },
    iter,
};

use value::{
    id_v6::DocumentIdV6,
    FieldName,
    IdentifierFieldName,
};

use crate::{
    array::ArrayShape,
    map::MapShape,
    object::{
        ObjectField,
        ObjectShape,
        RecordShape,
    },
    set::SetShape,
    string::StringLiteralShape,
    union::UnionBuilder,
    CountedShape,
    ShapeConfig,
    ShapeEnum,
};

/// Compute a sequence of supertypes for a set of types. Each returned supertype
/// includes the indexes of which types it subsumes.
///
/// The returned supertypes are roughly ordered by specificity, where we first
/// try to merge types that overlap (and *must* be merged within a union) and
/// then pass from types to less descriptive supertypes.
pub fn supertype_candidates<C: ShapeConfig>(
    types: &[CountedShape<C>],
) -> impl Iterator<Item = (CountedShape<C>, Vec<usize>)> + '_ {
    assert!(types.len() >= 2);
    iter::from_coroutine(move || {
        // Phase 1: Try to merge overlapping types. We won't lose any precision in our
        // unions as this stage since these types *must* be merged to preserve
        // disjointness.
        if let Some(candidate) = array_candidate(types) {
            yield candidate;
        }
        if let Some(candidate) = set_candidate(types) {
            yield candidate;
        }
        if let Some(candidate) = map_candidate(types) {
            yield candidate;
        }
        // Include a record type in Phase 1 if we already have a record. Otherwise, we'd
        // be widening an object type to a record, which can happen below.
        let any_record = types
            .iter()
            .any(|t| matches!(&*t.variant, ShapeEnum::Record(..)));
        if any_record {
            if let Some(candidate) = record_candidate(types) {
                yield candidate;
            }
        }
        // Phase 2: String supertypes. Propose `id` types, `field_name`, and eventually
        // `string`.
        for candidate in id_candidates(types) {
            yield candidate;
        }
        if let Some(candidate) = field_name_candidate(types) {
            yield candidate;
        }
        if let Some(candidate) = string_candidate(types) {
            yield candidate;
        }

        // Phase 3: Float supertypes. Propose `float64` if it is a supertype of at least
        // two input types.
        if let Some(candidate) = float64_candidate(types) {
            yield candidate;
        }

        // Phase 4: Object supertypes. Propose `object` types and potentially a `record`
        // type that are a supertype of at least two input types.
        if let Some(candidate) = object_candidate(types) {
            yield candidate;
        }
        if !any_record {
            if let Some(candidate) = record_candidate(types) {
                yield candidate;
            }
        }
        // Phase 5: Finally, just emit the `unknown` type.
        let unknown_type =
            CountedShape::new(ShapeEnum::Unknown, types.iter().map(|t| t.num_values).sum());
        yield (unknown_type, (0..types.len()).collect());
    })
}

fn float64_candidate<C: ShapeConfig>(
    types: &[CountedShape<C>],
) -> Option<(CountedShape<C>, Vec<usize>)> {
    let mut indexes = Vec::new();
    for (i, t) in types.iter().enumerate() {
        if let ShapeEnum::NegativeInf
        | ShapeEnum::PositiveInf
        | ShapeEnum::NegativeZero
        | ShapeEnum::NaN
        | ShapeEnum::NormalFloat64
        | ShapeEnum::Float64 = &*t.variant
        {
            indexes.push(i);
        }
    }
    if indexes.len() < 2 {
        return None;
    }
    let new_type = CountedShape::new(
        ShapeEnum::Float64,
        indexes.iter().map(|&i| types[i].num_values).sum(),
    );
    Some((new_type, indexes))
}

fn id_candidates<C: ShapeConfig>(
    types: &[CountedShape<C>],
) -> impl Iterator<Item = (CountedShape<C>, Vec<usize>)> + '_ {
    iter::from_coroutine(move || {
        let mut candidates = BTreeMap::new();
        for (i, t) in types.iter().enumerate() {
            if let ShapeEnum::StringLiteral(ref s) = &*t.variant {
                if let Ok(id) = DocumentIdV6::decode(s) {
                    candidates
                        .entry(*id.table())
                        .or_insert_with(Vec::new)
                        .push(i);
                }
            }
            if let ShapeEnum::Id(ref table) = &*t.variant {
                candidates.entry(*table).or_insert_with(Vec::new).push(i);
            }
        }
        for (table_number, indexes) in candidates {
            if indexes.len() >= 2 {
                let new_type = CountedShape::new(
                    ShapeEnum::Id(table_number),
                    indexes.iter().map(|&i| types[i].num_values).sum(),
                );
                yield (new_type, indexes);
            }
        }
    })
}

fn field_name_candidate<C: ShapeConfig>(
    types: &[CountedShape<C>],
) -> Option<(CountedShape<C>, Vec<usize>)> {
    let mut indexes = Vec::new();
    for (i, t) in types.iter().enumerate() {
        let subtype = match &*t.variant {
            ShapeEnum::StringLiteral(ref s) => s.parse::<FieldName>().is_ok(),
            ShapeEnum::Id(..) | ShapeEnum::FieldName => true,
            _ => false,
        };
        if subtype {
            indexes.push(i);
        }
    }
    if indexes.len() < 2 {
        return None;
    }
    let new_type = CountedShape::new(
        ShapeEnum::FieldName,
        indexes.iter().map(|&i| types[i].num_values).sum(),
    );
    Some((new_type, indexes))
}

fn string_candidate<C: ShapeConfig>(
    types: &[CountedShape<C>],
) -> Option<(CountedShape<C>, Vec<usize>)> {
    let mut indexes = Vec::new();
    for (i, t) in types.iter().enumerate() {
        if let ShapeEnum::StringLiteral(..)
        | ShapeEnum::Id(..)
        | ShapeEnum::FieldName
        | ShapeEnum::String = &*t.variant
        {
            indexes.push(i);
        }
    }
    if indexes.len() < 2 {
        return None;
    }
    let new_type = CountedShape::new(
        ShapeEnum::String,
        indexes.iter().map(|&i| types[i].num_values).sum(),
    );
    Some((new_type, indexes))
}

// Only output a single array candidate, folding all other arrays' element types
// into a single union for the candidate's element type.
fn array_candidate<C: ShapeConfig>(
    types: &[CountedShape<C>],
) -> Option<(CountedShape<C>, Vec<usize>)> {
    let mut element = UnionBuilder::new();
    let mut indexes = vec![];
    for (i, t) in types.iter().enumerate() {
        if let ShapeEnum::Array(ref array) = &*t.variant {
            element = element.push(array.element().clone());
            indexes.push(i);
        }
    }
    if indexes.len() < 2 {
        return None;
    }
    let new_type = CountedShape::new(
        ShapeEnum::Array(ArrayShape::new(element.build())),
        indexes.iter().map(|&i| types[i].num_values).sum(),
    );
    Some((new_type, indexes))
}

fn set_candidate<C: ShapeConfig>(
    types: &[CountedShape<C>],
) -> Option<(CountedShape<C>, Vec<usize>)> {
    let mut element = UnionBuilder::new();
    let mut indexes = vec![];
    for (i, t) in types.iter().enumerate() {
        if let ShapeEnum::Set(ref set) = &*t.variant {
            element = element.push(set.element().clone());
            indexes.push(i);
        }
    }
    if indexes.len() < 2 {
        return None;
    }
    let new_type = CountedShape::new(
        ShapeEnum::Set(SetShape::new(element.build())),
        indexes.iter().map(|&i| types[i].num_values).sum(),
    );
    Some((new_type, indexes))
}

fn map_candidate<C: ShapeConfig>(
    types: &[CountedShape<C>],
) -> Option<(CountedShape<C>, Vec<usize>)> {
    let mut key = UnionBuilder::new();
    let mut value = UnionBuilder::new();
    let mut indexes = vec![];
    for (i, t) in types.iter().enumerate() {
        if let ShapeEnum::Map(ref map) = &*t.variant {
            key = key.push(map.key().clone());
            value = value.push(map.value().clone());
            indexes.push(i);
        }
    }
    if indexes.len() < 2 {
        return None;
    }
    let new_type = CountedShape::new(
        ShapeEnum::Map(MapShape::new(key.build(), value.build())),
        indexes.iter().map(|&i| types[i].num_values).sum(),
    );
    Some((new_type, indexes))
}

// Compute a single `object` encompassing all `object`s in `types`, and making
// fields optional as necessary.
fn object_candidate<C: ShapeConfig>(
    types: &[CountedShape<C>],
) -> Option<(CountedShape<C>, Vec<usize>)> {
    let mut indexes = vec![];
    for (i, t) in types.iter().enumerate() {
        let ShapeEnum::Object(..) = &*t.variant else {
            continue;
        };
        indexes.push(i);
    }
    if indexes.len() < 2 {
        return None;
    }
    let mut field_builders = BTreeMap::new();
    let mut num_values = 0;
    let mut optional_fields: BTreeSet<IdentifierFieldName> = BTreeSet::new();
    for &j in &indexes {
        let ShapeEnum::Object(ref object_type) = &*types[j].variant else {
            panic!("Tried to merge two non-objects");
        };
        for (field_name, field_type) in object_type.fields() {
            if field_type.optional {
                optional_fields.insert(field_name.clone());
            }
            let existing_shapes = match field_builders.remove(field_name) {
                None => {
                    // New field that didn't exist in previous object shapes => make the
                    // field optional.
                    if num_values > 0 {
                        optional_fields.insert(field_name.clone());
                    }
                    UnionBuilder::new()
                },
                Some(existing_shapes) => existing_shapes,
            };
            let builder = existing_shapes.push(field_type.value_shape.clone());
            field_builders.insert(field_name.clone(), builder);
        }
        for field_name in field_builders.keys() {
            if object_type.fields().contains_key(field_name) {
                continue;
            }
            optional_fields.insert(field_name.clone());
        }
        num_values += types[j].num_values;
    }
    if !optional_fields.is_empty() && !C::allow_optional_object_fields() {
        return None;
    }
    if field_builders.len() > C::MAX_OBJECT_FIELDS {
        return None;
    }

    let fields = field_builders
        .into_iter()
        .map(|(field_name, builder)| {
            let optional = optional_fields.contains(&field_name);
            (
                field_name,
                ObjectField {
                    value_shape: builder.build(),
                    optional,
                },
            )
        })
        .collect();
    let object_type = ObjectShape::<C, u64>::new(fields);
    let new_type = CountedShape::new(ShapeEnum::Object(object_type), num_values);
    Some((new_type, indexes))
}

fn record_candidate<C: ShapeConfig>(
    types: &[CountedShape<C>],
) -> Option<(CountedShape<C>, Vec<usize>)> {
    let mut field_union = UnionBuilder::new();
    let mut value_union = UnionBuilder::new();
    let mut indexes = vec![];
    for (i, t) in types.iter().enumerate() {
        if let ShapeEnum::Record(ref record) = &*t.variant {
            field_union = field_union.push(record.field().clone());
            value_union = value_union.push(record.value().clone());
            indexes.push(i);
        }
        if let ShapeEnum::Object(ref object) = &*t.variant {
            for (field_name, field) in object.iter() {
                let field_name_type = CountedShape::new(
                    StringLiteralShape::shape_of(&field_name[..]),
                    field.value_shape.num_values,
                );
                field_union = field_union.push(field_name_type);
                value_union = value_union.push(field.value_shape.clone());
            }
            indexes.push(i);
        }
    }
    if indexes.len() < 2 {
        return None;
    }
    let new_type = CountedShape::new(
        ShapeEnum::Record(RecordShape::new(field_union.build(), value_union.build())),
        indexes.iter().map(|&i| types[i].num_values).sum(),
    );
    Some((new_type, indexes))
}

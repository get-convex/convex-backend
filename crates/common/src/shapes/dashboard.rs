use serde_json::{
    json,
    Value as JsonValue,
};
use value::NamespacedTableMapping;

use super::reduced::ReducedShape;
use crate::virtual_system_mapping::{
    all_tables_number_to_name,
    VirtualSystemMapping,
};

pub fn dashboard_shape_json(
    shape: &ReducedShape,
    mapping: &NamespacedTableMapping,
    virtual_mapping: &VirtualSystemMapping,
) -> anyhow::Result<JsonValue> {
    let result = match shape {
        ReducedShape::Unknown => json!({"type": "Unknown"}),
        ReducedShape::Never => json!({"type": "Never"}),
        ReducedShape::Id(table_number) => {
            match all_tables_number_to_name(mapping, virtual_mapping)(*table_number) {
                Ok(table_name) => {
                    json!({
                        "type": "Id",
                        "tableName": table_name[..],
                    })
                },
                Err(_) => json!({ "type": "String" }),
            }
        },
        ReducedShape::Null => json!({"type": "Null"}),
        ReducedShape::Int64 => {
            json!({"type": "Int64" })
        },
        ReducedShape::Float64(range) => {
            json!({"type": "Float64", "float64Range": JsonValue::from(range)})
        },
        ReducedShape::Boolean => json!({"type": "Boolean"}),
        ReducedShape::String => json!({"type": "String"}),
        ReducedShape::Bytes => json!({"type": "Bytes"}),
        ReducedShape::Object(fields) => {
            let field_json = fields
                .iter()
                .map(|(field_name, field_shape)| {
                    let result = json!({
                        "fieldName": String::from(field_name.clone()),
                        "optional": field_shape.optional,
                        "shape": dashboard_shape_json(&field_shape.shape, mapping, virtual_mapping)?,
                    });
                    anyhow::Ok(result)
                })
                .try_collect::<Vec<_>>()?;
            json!({"type": "Object", "fields": field_json})
        },
        ReducedShape::Record {
            key_shape,
            value_shape,
        } => {
            json!({
                "type": "Record",
                "keyShape": dashboard_shape_json(key_shape.as_ref(), mapping, virtual_mapping)?,
                "valueShape": json!({
                    "optional": value_shape.as_ref().optional,
                    "shape": dashboard_shape_json(&value_shape.as_ref().shape, mapping, virtual_mapping)?,
                })
            })
        },
        ReducedShape::Array(shape) => {
            json!({"type": "Array", "shape": dashboard_shape_json(shape.as_ref(), mapping, virtual_mapping)?})
        },
        ReducedShape::Set(shape) => {
            json!({"type": "Set", "shape": dashboard_shape_json(shape.as_ref(), mapping, virtual_mapping)?})
        },
        ReducedShape::Map {
            key_shape,
            value_shape,
        } => {
            json!({
                "type": "Map",
                "keyShape": dashboard_shape_json(key_shape.as_ref(), mapping, virtual_mapping)?,
                "valueShape": dashboard_shape_json(value_shape.as_ref(), mapping, virtual_mapping)?,
            })
        },
        ReducedShape::Union(shapes) => {
            json!({
                "type": "Union",
                "shapes": shapes
                    .iter()
                    .map(|s| dashboard_shape_json(s, mapping, virtual_mapping))
                    .try_collect::<Vec<_>>()?,
            })
        },
    };
    Ok(result)
}

#[cfg(test)]
mod tests {
    use maplit::{
        btreemap,
        btreeset,
    };
    use serde::Deserialize;
    use serde_json::Value as JsonValue;
    use value::{
        TableMapping,
        TableName,
        TableNamespace,
    };

    use super::dashboard_shape_json;
    use crate::{
        shapes::reduced::{
            ReducedField,
            ReducedFloatRange,
            ReducedShape,
        },
        testing::TestIdGenerator,
        virtual_system_mapping::{
            all_tables_name_to_number,
            VirtualSystemMapping,
        },
    };

    fn parse_json(
        json_value: JsonValue,
        mapping: &TableMapping,
        virtual_mapping: &VirtualSystemMapping,
    ) -> anyhow::Result<ReducedShape> {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct FieldPair {
            field_name: String,
            optional: bool,
            shape: JsonValue,
        }

        #[derive(Deserialize)]
        #[serde(tag = "type")]
        enum ShapeEnumJson {
            Unknown,
            Never,
            #[serde(rename_all = "camelCase")]
            Id {
                table_name: String,
            },
            Null,
            #[serde(rename_all = "camelCase")]
            Int64,
            #[serde(rename_all = "camelCase")]
            Float64 {
                float64_range: JsonValue,
            },
            Boolean,
            String,
            Bytes,
            #[serde(rename_all = "camelCase")]
            Object {
                fields: Vec<FieldPair>,
            },
            Array {
                shape: JsonValue,
            },
            Set {
                shape: JsonValue,
            },
            #[serde(rename_all = "camelCase")]
            Map {
                key_shape: JsonValue,
                value_shape: JsonValue,
            },
            Union {
                shapes: Vec<JsonValue>,
            },
        }

        let shape_enum: ShapeEnumJson = serde_json::from_value(json_value)?;
        let result = match shape_enum {
            ShapeEnumJson::Unknown => ReducedShape::Unknown,
            ShapeEnumJson::Never => ReducedShape::Never,
            ShapeEnumJson::Id { table_name } => {
                let name: TableName = table_name.parse()?;
                ReducedShape::Id(all_tables_name_to_number(
                    TableNamespace::test_user(),
                    mapping,
                    virtual_mapping,
                )(name)?)
            },
            ShapeEnumJson::Null => ReducedShape::Null,
            ShapeEnumJson::Int64 { .. } => ReducedShape::Int64,
            ShapeEnumJson::Float64 { float64_range } => {
                ReducedShape::Float64(ReducedFloatRange::try_from(float64_range)?)
            },
            ShapeEnumJson::Boolean => ReducedShape::Boolean,
            ShapeEnumJson::String => ReducedShape::String,
            ShapeEnumJson::Bytes => ReducedShape::Bytes,
            ShapeEnumJson::Object { fields } => {
                let field_shapes = fields
                    .into_iter()
                    .map(|p| {
                        Ok((
                            p.field_name.parse()?,
                            ReducedField {
                                optional: p.optional,
                                shape: parse_json(p.shape, mapping, virtual_mapping)?,
                            },
                        ))
                    })
                    .collect::<anyhow::Result<_>>()?;
                ReducedShape::Object(field_shapes)
            },
            ShapeEnumJson::Array { shape } => {
                ReducedShape::Array(Box::new(parse_json(shape, mapping, virtual_mapping)?))
            },
            ShapeEnumJson::Set { shape } => {
                ReducedShape::Set(Box::new(parse_json(shape, mapping, virtual_mapping)?))
            },
            ShapeEnumJson::Map {
                key_shape,
                value_shape,
            } => ReducedShape::Map {
                key_shape: Box::new(parse_json(key_shape, mapping, virtual_mapping)?),
                value_shape: Box::new(parse_json(value_shape, mapping, virtual_mapping)?),
            },
            ShapeEnumJson::Union { shapes } => ReducedShape::Union(
                shapes
                    .into_iter()
                    .map(|s| parse_json(s, mapping, virtual_mapping))
                    .collect::<anyhow::Result<_>>()?,
            ),
        };
        Ok(result)
    }

    #[test]
    fn test_shape_roundtrips() -> anyhow::Result<()> {
        let mut id_generator = TestIdGenerator::new();
        let table_id = id_generator.user_table_id(&"test".parse()?).table_number;
        let shapes = vec![
            ReducedShape::Unknown,
            ReducedShape::Never,
            ReducedShape::Id(table_id),
            ReducedShape::Null,
            ReducedShape::Int64,
            ReducedShape::Float64(ReducedFloatRange::new(false)),
            ReducedShape::Boolean,
            ReducedShape::String,
            ReducedShape::Bytes,
            ReducedShape::Object(btreemap!(
                "fieldA".parse()? => ReducedField { shape: ReducedShape::String, optional: true },
                "fieldB".parse()? => ReducedField { shape: ReducedShape::Bytes, optional: false },
            )),
            ReducedShape::Array(Box::new(ReducedShape::Null)),
            ReducedShape::Set(Box::new(ReducedShape::Null)),
            ReducedShape::Map {
                key_shape: Box::new(ReducedShape::Null),
                value_shape: Box::new(ReducedShape::Null),
            },
            ReducedShape::Union(btreeset!(ReducedShape::Boolean, ReducedShape::String)),
        ];
        for shape in shapes {
            let json_value = dashboard_shape_json(
                &shape,
                &id_generator.namespace(TableNamespace::test_user()),
                &id_generator.virtual_system_mapping,
            )?;
            assert_eq!(
                parse_json(
                    json_value,
                    &id_generator,
                    &id_generator.virtual_system_mapping
                )?,
                shape
            );
        }
        Ok(())
    }

    #[test]
    fn test_shape_roundtrips_with_virtual_ids() -> anyhow::Result<()> {
        let mut id_generator = TestIdGenerator::new();
        let table_number = id_generator.generate_virtual_table(&"test".parse()?);
        let shapes = vec![
            ReducedShape::Unknown,
            ReducedShape::Never,
            ReducedShape::Id(table_number),
            ReducedShape::Null,
            ReducedShape::Int64,
            ReducedShape::Float64(ReducedFloatRange::new(false)),
            ReducedShape::Boolean,
            ReducedShape::String,
            ReducedShape::Bytes,
            ReducedShape::Object(btreemap!(
                "fieldA".parse()? => ReducedField { shape: ReducedShape::String, optional: true },
                "fieldB".parse()? => ReducedField { shape: ReducedShape::Bytes, optional: false },
            )),
            ReducedShape::Array(Box::new(ReducedShape::Null)),
            ReducedShape::Set(Box::new(ReducedShape::Null)),
            ReducedShape::Map {
                key_shape: Box::new(ReducedShape::Null),
                value_shape: Box::new(ReducedShape::Null),
            },
            ReducedShape::Union(btreeset!(ReducedShape::Boolean, ReducedShape::String)),
        ];
        for shape in shapes {
            let json_value = dashboard_shape_json(
                &shape,
                &id_generator.namespace(TableNamespace::test_user()),
                &id_generator.virtual_system_mapping,
            )?;
            assert_eq!(
                parse_json(
                    json_value,
                    &id_generator,
                    &id_generator.virtual_system_mapping,
                )?,
                shape
            );
        }
        Ok(())
    }
}

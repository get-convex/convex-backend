use std::collections::{
    BTreeMap,
    BTreeSet,
};

use serde_json::{
    json,
    Value as JsonValue,
};
use value::{
    export::ValueFormat,
    TableName,
};

pub fn any() -> JsonValue {
    json!({})
}

pub fn never() -> JsonValue {
    json!(false)
}

pub fn id(table_name: &TableName) -> JsonValue {
    json!({
        "$description": format!("Id({table_name})"),
        "type": "string",
    })
}

pub fn null() -> JsonValue {
    json!({
        "type": "null",
    })
}

pub fn float64(has_special_values: bool, value_format: ValueFormat) -> JsonValue {
    if has_special_values {
        match value_format {
            ValueFormat::ConvexCleanJSON => json!({
                "$description": "float64",
                "anyOf": [
                    {"type": "number"},
                    {
                        "type": "string",
                        "$description": "-inf, inf, or NaN",
                    },
                ],
            }),
            ValueFormat::ConvexEncodedJSON => json!({
                "$description": "float64",
                "anyOf": [
                    {"type": "number"},
                    {
                        "type": "object",
                        "$description": "-0, -inf, inf, or NaN",
                        "properties": {
                            "$float": {
                                "$description": "float64 -> little-endian -> base64",
                                "type": "string"
                            },
                        },
                    }
                ]
            }),
        }
    } else {
        json!({"type": "number"})
    }
}

pub fn int64(value_format: ValueFormat) -> JsonValue {
    match value_format {
        ValueFormat::ConvexCleanJSON => json!({
            "$description": "int64 represented as base10 string",
            "type": "string",
        }),
        ValueFormat::ConvexEncodedJSON => json!({
            "$description": "int64",
            "type": "object",
            "properties": {
                "$integer": {
                    "$description": "int64 -> little-endian -> base64",
                    "type": "string",
                },
            }
        }),
    }
}

pub fn boolean() -> JsonValue {
    json!({"type": "boolean"})
}

pub fn string() -> JsonValue {
    json!({"type": "string"})
}

pub fn bytes(value_format: ValueFormat) -> JsonValue {
    match value_format {
        ValueFormat::ConvexCleanJSON => json!({
            "$description": "base64 bytes",
            "type": "string",
        }),
        ValueFormat::ConvexEncodedJSON => json!({
            "type": "object",
            "$description": "base64 bytes",
            "properties": {
                "$bytes": {"type": "string"},
            },
        }),
    }
}

pub fn array(element_schema: JsonValue) -> JsonValue {
    json!({
        "type": "array",
        "items": element_schema
    })
}

pub fn record(key_type: String, value_schema: JsonValue) -> JsonValue {
    json!({
        "type": "object",
        "$description": format!("Record with keys of type {}", key_type),
        "additionalProperties": {
          "type": value_schema
        }
    })
}

pub fn union(variant_schemas: Vec<JsonValue>) -> JsonValue {
    json!({ "anyOf": variant_schemas })
}

pub struct FieldInfo {
    pub schema: JsonValue,
    pub optional: bool,
}

pub fn object(fields: BTreeMap<String, FieldInfo>) -> JsonValue {
    let required_fields: BTreeSet<_> = fields
        .iter()
        .filter(|(_, field_info)| !field_info.optional)
        .map(|(field_name, _)| field_name.to_string())
        .collect();
    let props: serde_json::Map<_, _> = fields
        .into_iter()
        .map(|(field_name, field_info)| (field_name, field_info.schema))
        .collect();
    json!({
        "type": "object",
        "properties": props,
        "additionalProperties": false,
        "required": required_fields.into_iter().collect::<Vec<_>>(),
    })
}

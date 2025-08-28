use std::{
    collections::BTreeMap,
    str::FromStr,
};

use anyhow::Context;
use axum::{
    debug_handler,
    extract::State,
    response::IntoResponse,
};
use common::{
    bootstrap_model::schema::SchemaState,
    components::{
        ComponentId,
        ComponentPath,
    },
    document::{
        CREATION_TIME_FIELD,
        ID_FIELD,
    },
    http::{
        extract::{
            Json,
            Query,
        },
        HttpResponseError,
    },
    json_schemas,
    schemas::{
        validator::{
            AddTopLevelFields,
            Validator,
        },
        DatabaseSchema,
        DocumentSchema,
    },
    shapes::reduced::ReducedShape,
    types::Timestamp,
    virtual_system_mapping::{
        all_tables_number_to_name,
        VirtualSystemMapping,
    },
};
use convex_fivetran_source::api_types::{
    selection::Selection,
    DocumentDeltasArgs,
    DocumentDeltasResponse,
    DocumentDeltasValue,
    GetTableColumnNameTable,
    GetTableColumnNamesResponse,
    ListSnapshotArgs,
    ListSnapshotResponse,
    ListSnapshotValue,
};
use database::{
    streaming_export_selection::StreamingExportSelection,
    BootstrapComponentsModel,
    DocumentDeltas,
    SchemaModel,
    SnapshotPage,
};
use errors::ErrorMetadata;
use http::StatusCode;
use keybroker::Identity;
use maplit::btreemap;
use model::virtual_system_mapping;
use serde::{
    Deserialize,
    Serialize,
};
use serde_json::{
    json,
    Value as JsonValue,
};
use value::{
    export::ValueFormat,
    NamespacedTableMapping,
    ResolvedDocumentId,
    TableName,
    TableNamespace,
};

use crate::{
    admin::must_be_admin,
    authentication::ExtractIdentity,
    LocalAppState,
};

#[fastrace::trace]
pub async fn document_deltas_get(
    State(st): State<LocalAppState>,
    Query(args): Query<DocumentDeltasArgs>,
    ExtractIdentity(identity): ExtractIdentity,
) -> Result<impl IntoResponse, HttpResponseError> {
    _document_deltas(st, args, identity).await
}

#[debug_handler]
#[fastrace::trace]
pub async fn document_deltas_post(
    State(st): State<LocalAppState>,
    ExtractIdentity(identity): ExtractIdentity,
    Json(args): Json<DocumentDeltasArgs>,
) -> Result<impl IntoResponse, HttpResponseError> {
    _document_deltas(st, args, identity).await
}

pub async fn _document_deltas(
    st: LocalAppState,
    args: DocumentDeltasArgs,
    identity: Identity,
) -> Result<impl IntoResponse, HttpResponseError> {
    st.application
        .ensure_streaming_export_enabled(identity.clone())
        .await?;
    identity.assert_present()?;
    let cursor = args
        .cursor
        .context(ErrorMetadata::bad_request(
            "DocumentDeltasCursorRequired",
            "/api/document_deltas requires a cursor",
        ))?
        .try_into()?;

    let selection = Selection::from(args.selection);
    let selection = StreamingExportSelection::try_from(selection)?;

    let DocumentDeltas {
        deltas,
        cursor: new_cursor,
        has_more,
    } = st
        .application
        .document_deltas(identity, cursor, selection)
        .await?;
    let value_format = args
        .format
        .map(|f| f.parse())
        .transpose()?
        .unwrap_or(ValueFormat::ConvexCleanJSON);
    let values = deltas
        .into_iter()
        .map(
            |(ts, id, component_path, table_name, maybe_doc)| -> anyhow::Result<_> {
                Ok(DocumentDeltasValue {
                    component: component_path.to_string(),
                    table: table_name.to_string(),
                    ts: i64::from(ts),
                    // Deleted documents may be used in various ways.
                    // To preserve choices for the caller, we return a json object with
                    // the same primary key and cursor, and with a special field to identify
                    // the row as deleted.
                    deleted: maybe_doc.is_none(),
                    fields: match maybe_doc {
                        Some(doc) => doc.export_fields(value_format)?,
                        None => btreemap! {
                            "_id".to_string() => JsonValue::from(id),
                        },
                    },
                })
            },
        )
        .try_collect()?;
    let response = DocumentDeltasResponse {
        values,
        cursor: new_cursor.into(),
        has_more,
    };
    Ok((StatusCode::OK, Json(response)))
}

#[fastrace::trace]
pub async fn list_snapshot_get(
    State(st): State<LocalAppState>,
    Query(query_args): Query<ListSnapshotArgs>,
    ExtractIdentity(identity): ExtractIdentity,
) -> Result<impl IntoResponse, HttpResponseError> {
    _list_snapshot(st, query_args, identity).await
}

#[debug_handler]
#[fastrace::trace]
pub async fn list_snapshot_post(
    State(st): State<LocalAppState>,
    ExtractIdentity(identity): ExtractIdentity,
    Json(args): Json<ListSnapshotArgs>,
) -> Result<impl IntoResponse, HttpResponseError> {
    _list_snapshot(st, args, identity).await
}

async fn _list_snapshot(
    st: LocalAppState,
    args: ListSnapshotArgs,
    identity: Identity,
) -> Result<impl IntoResponse, HttpResponseError> {
    st.application
        .ensure_streaming_export_enabled(identity.clone())
        .await?;
    identity.assert_present()?;
    let snapshot = args.snapshot.map(Timestamp::try_from).transpose()?;

    #[derive(Serialize, Deserialize)]
    struct ListSnapshotCursor {
        tablet: String,
        id: String,
    }

    let cursor: Option<ResolvedDocumentId> = args
        .cursor
        .map(|cursor| -> anyhow::Result<_> {
            let list_snapshot_cursor = serde_json::from_str::<ListSnapshotCursor>(&cursor)?;
            Ok(ResolvedDocumentId {
                tablet_id: list_snapshot_cursor.tablet.parse()?,
                developer_id: list_snapshot_cursor
                    .id
                    .parse()
                    .map_err(anyhow::Error::new)?,
            })
        })
        .transpose()?;

    let selection = Selection::from(args.selection);
    let selection = StreamingExportSelection::try_from(selection)?;

    let SnapshotPage {
        documents,
        snapshot,
        cursor: new_cursor,
        has_more,
    } = st
        .application
        .list_snapshot(identity, snapshot, cursor, selection)
        .await?;
    let value_format = args
        .format
        .map(|f| f.parse())
        .transpose()?
        .unwrap_or(ValueFormat::ConvexCleanJSON);
    let values = documents
        .into_iter()
        .map(
            |(ts, component_path, table_name, doc)| -> anyhow::Result<_> {
                Ok(ListSnapshotValue {
                    component: component_path.to_string(),
                    table: table_name.to_string(),
                    // _ts is the field used for ordering documents with the same
                    // _id, and determining which version is latest.
                    // Note we could use `_ts = snapshot` or even `_ts = _creationTime`
                    // which would provide the same guarantees but would be more confusing
                    // if clients try to use it for anything besides deduplication.
                    ts: i64::from(ts),
                    fields: doc.export_fields(value_format)?,
                })
            },
        )
        .try_collect()?;
    let response = ListSnapshotResponse {
        values,
        snapshot: snapshot.into(),
        cursor: new_cursor
            .map(|new_cursor| -> anyhow::Result<String> {
                serde_json::to_string(&ListSnapshotCursor {
                    tablet: new_cursor.tablet_id.to_string(),
                    id: new_cursor.developer_id.encode(),
                })
                .context("Failed to serialize cursor")
            })
            .transpose()?,
        has_more,
    };
    Ok((StatusCode::OK, Json(response)))
}

/// Confirms that streaming export is enabled
pub async fn test_streaming_export_connection(
    State(st): State<LocalAppState>,
    ExtractIdentity(identity): ExtractIdentity,
) -> Result<impl IntoResponse, HttpResponseError> {
    st.application
        .ensure_streaming_export_enabled(identity.clone())
        .await?;
    Ok(Json(()))
}

/// Used by fivetran -- returns a mapping from table name to a list of top level
/// fields in the table, taken from the shape.
///
/// It's ok for the list of columns to be incomplete since fivetran can handle
/// extra fields during an export.
///
/// TODO(nicolas): Remove this endpoint (replaced by
/// get_table_column_names)
pub async fn get_tables_and_columns(
    State(st): State<LocalAppState>,
    ExtractIdentity(identity): ExtractIdentity,
) -> Result<impl IntoResponse, HttpResponseError> {
    st.application
        .ensure_streaming_export_enabled(identity.clone())
        .await?;
    let mut out = serde_json::Map::new();

    must_be_admin(&identity)?;
    let snapshot = st.application.latest_snapshot()?;
    let mapping = snapshot.table_mapping();

    for (namespace, table_name) in snapshot.table_registry.user_table_names() {
        let table_summary = snapshot.must_table_summary(namespace, table_name)?;
        let shape = ReducedShape::from_type(
            table_summary.inferred_type(),
            &mapping.namespace(namespace).table_number_exists(),
        );
        let columns = get_columns_for_table(shape)
            .into_iter()
            .map(JsonValue::String)
            .collect();
        out.insert(String::from(table_name.clone()), JsonValue::Array(columns));
    }
    Ok(Json(out))
}

/// Used by Fivetran -- returns a mapping from table name to a list of top level
/// fields in the table, taken from the shape.
///
/// It’s ok for the list of columns to be incomplete since Fivetran can handle
/// extra fields during an export.
pub async fn get_table_column_names(
    State(st): State<LocalAppState>,
    ExtractIdentity(identity): ExtractIdentity,
) -> Result<impl IntoResponse, HttpResponseError> {
    st.application
        .ensure_streaming_export_enabled(identity.clone())
        .await?;
    must_be_admin(&identity)?;

    let snapshot = st.application.latest_snapshot()?;
    let mapping = snapshot.table_mapping();
    let component_paths = snapshot.component_ids_to_paths();

    let by_component: BTreeMap<ComponentPath, Vec<GetTableColumnNameTable>> = snapshot
        .table_registry
        .user_table_names()
        .map(|(namespace, table_name)| -> anyhow::Result<_> {
            let component_path = component_paths
                .get(&ComponentId::from(namespace))
                .ok_or_else(|| {
                    anyhow::anyhow!(
                        "Can’t find the component path for table \"{table_name}\" in namespace \
                         {namespace:?}"
                    )
                })?;

            let columns = get_columns_for_table(ReducedShape::from_type(
                snapshot
                    .must_table_summary(namespace, table_name)?
                    .inferred_type(),
                &mapping.namespace(namespace).table_number_exists(),
            ));

            Ok((
                component_path,
                GetTableColumnNameTable {
                    name: table_name.to_string(),
                    columns,
                },
            ))
        })
        .try_fold(
            BTreeMap::<ComponentPath, Vec<GetTableColumnNameTable>>::new(),
            |mut acc, row| -> anyhow::Result<_> {
                let (component_path, table) = row?;
                if let Some(vec) = acc.get_mut(component_path) {
                    vec.push(table);
                } else {
                    acc.insert(component_path.clone(), vec![table]);
                }
                Ok(acc)
            },
        )?;

    Ok(Json(GetTableColumnNamesResponse {
        by_component: by_component
            .into_iter()
            .map(|(k, v)| (k.to_string(), v))
            .collect(),
    }))
}

fn get_columns_for_table(shape: ReducedShape) -> Vec<String> {
    match shape {
        ReducedShape::Object(fields) => fields.keys().map(|f| f.to_string()).collect(),
        _ => vec![CREATION_TIME_FIELD.to_string(), ID_FIELD.to_string()],
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JsonSchemaArgs {
    /// By default, /json_schemas describes the JSON formats used by HTTP APIs
    /// like /udf to return database documents.
    /// When passing deltaSchema=true, /json_schemas returns the document schema
    /// plus the fields added by /document_deltas: _ts and _deleted.
    #[serde(default)]
    delta_schema: bool,
    /// Export format
    format: Option<String>,
    /// By default, /json_schemas returns an object mapping table name to shape.
    /// If byComponent=true, /json_schemas returns an object mapping component
    /// path to table name to shape, e.g.
    /// { "waitlist": { "users": { "type": "object", "properties": { ... } } } }
    #[serde(default)]
    by_component: bool,
}

/// Similar to /shapes2 API, but returns JSONSchema (https://json-schema.org/)
/// instead of our internal Shapes representation.
/// If byComponent=false, returns a JSONSchema for each table, e.g.
/// { "users": { "type": "object", "properties": { ... } } }
/// If byComponent=true, returns a JSONSchema for each table under each
/// component, { "waitlist": { "users": { "type": "object", "properties": { ...
/// } } } }
pub async fn json_schemas(
    State(st): State<LocalAppState>,
    Query(query_args): Query<JsonSchemaArgs>,
    ExtractIdentity(identity): ExtractIdentity,
) -> Result<impl IntoResponse, HttpResponseError> {
    st.application
        .ensure_streaming_export_enabled(identity.clone())
        .await?;
    identity.assert_present()?;
    let mut out = serde_json::Map::new();

    must_be_admin(&identity)?;
    let mut tx = st.application.begin(identity.clone()).await?;
    let snapshot = st.application.snapshot(tx.begin_timestamp())?;
    let component_paths = BootstrapComponentsModel::new(&mut tx).all_component_paths();
    for (component_id, component_path) in component_paths {
        let component_out = if query_args.by_component {
            let component_path_string = String::from(component_path);
            out.insert(
                component_path_string.clone(),
                JsonValue::Object(serde_json::Map::new()),
            );
            out.get_mut(&component_path_string)
                .unwrap()
                .as_object_mut()
                .unwrap()
        } else {
            &mut out
        };
        let namespace: TableNamespace = component_id.into();
        let active_schema = SchemaModel::new(&mut tx, namespace)
            .get_by_state(SchemaState::Active)
            .await?
            .map(|(_id, active_schema)| active_schema);

        let mapping = snapshot.table_registry.table_mapping().namespace(namespace);

        let value_format = query_args
            .format
            .as_deref()
            .map(FromStr::from_str)
            .transpose()?
            .unwrap_or(ValueFormat::ConvexCleanJSON);
        for (_, _, table_name) in mapping.iter_active_user_tables() {
            let table_summary = snapshot.must_table_summary(namespace, table_name)?;
            let shape = ReducedShape::from_type(
                table_summary.inferred_type(),
                &mapping.table_number_exists(),
            );
            let mut json_schema = shape_to_json_schema(
                &shape,
                active_schema.as_deref(),
                &mapping,
                virtual_system_mapping(),
                table_name,
                value_format,
            )?;
            if query_args.delta_schema {
                // Inject change metadata fields.
                if let Some(m) = json_schema.as_object_mut() {
                    if let Some(properties) = m.get_mut("properties") {
                        if let Some(properties) = properties.as_object_mut() {
                            properties.insert("_table".to_string(), json!({"type": "string"}));
                            properties.insert("_component".to_string(), json!({"type": "string"}));
                            properties.insert("_ts".to_string(), json!({"type": "integer"}));
                            properties.insert("_deleted".to_string(), json!({"type": "boolean"}));
                        }
                    }
                }
            }
            component_out.insert(String::from(table_name.clone()), json_schema);
        }
    }
    Ok(Json(out))
}

fn empty_table_schema(table_name: &TableName, value_format: ValueFormat) -> serde_json::Value {
    let field_infos = btreemap! {
        ID_FIELD.to_string() =>
        json_schemas::FieldInfo {
            schema: json_schemas::id(table_name),
            optional: false
        },
        CREATION_TIME_FIELD.to_string() =>
        json_schemas::FieldInfo {
            schema: json_schemas::float64(false, value_format),
            optional: false
        }
    };
    json_schemas::object(field_infos)
}

fn shape_to_json_schema(
    shape: &ReducedShape,
    active_schema: Option<&DatabaseSchema>,
    mapping: &NamespacedTableMapping,
    virtual_mapping: &VirtualSystemMapping,
    table_name: &TableName,
    value_format: ValueFormat,
) -> anyhow::Result<JsonValue> {
    // Special case the empty table, as the export still expects to see
    // the fundamental fields, notably the primary key.
    let mut shape_json = if *shape == ReducedShape::Never {
        empty_table_schema(table_name, value_format)
    } else if matches!(shape, ReducedShape::Object(_)) {
        inner_shape_to_json_schema(shape, mapping, virtual_mapping, value_format)?
    } else {
        json_schema_from_active_schema(active_schema, table_name, value_format)?
    };

    let map = shape_json
        .as_object_mut()
        .context("Top level shape must end up as a json schema object")?;
    // Inject version information at the top level.
    map.insert(
        "$schema".to_string(),
        // The rust library for testing uses Draft7, and we don't need
        // any of the newer features.
        json!("http://json-schema.org/draft-07/schema#"),
    );
    Ok(shape_json)
}

fn json_schema_from_active_schema(
    active_schema: Option<&DatabaseSchema>,
    table_name: &TableName,
    value_format: ValueFormat,
) -> anyhow::Result<JsonValue> {
    let Some(active_schema) = active_schema else {
        anyhow::bail!(ErrorMetadata::bad_request(
            "NoSchemaForExport",
            format!("There is no active schema, which is needed for streaming export.")
        ));
    };

    let table_type = active_schema
        .tables
        .get(table_name)
        .and_then(|t| t.document_type.clone())
        .ok_or_else(|| {
            anyhow::anyhow!(ErrorMetadata::bad_request(
                "NoTableDefinitionForExport",
                format!(
                    "The table \"{table_name}\" does not have a schema, which is needed for \
                     streaming export."
                )
            ))
        })?;

    let json_value = match table_type {
        DocumentSchema::Any => {
            // Return an object type with just the `_id` and `_creationTime` fields
            let fields = btreemap! {
                ID_FIELD.to_string() => json_schemas::FieldInfo {
                    schema: json_schemas::id(table_name),
                    optional: false
                },
                CREATION_TIME_FIELD.to_string() => json_schemas::FieldInfo {
                    schema: json_schemas::float64(false, value_format),
                    optional: false
                }
            };
            json_schemas::object(fields)
        },
        DocumentSchema::Union(validators) => {
            if validators.len() != 1 {
                anyhow::bail!(ErrorMetadata::bad_request(
                    "UnsupportedSchemaForExport",
                    format!(
                        "Schema for table \"{table_name}\" contains a union of object types, \
                         which is unsupported for streaming export"
                    )
                ));
            };
            let object_validator = validators[0].clone();
            let json_schema = object_validator
                .to_json_schema(AddTopLevelFields::True(table_name.clone()), value_format);
            Validator::Object(object_validator).ensure_supported_for_streaming_export()?;
            json_schema
        },
    };

    Ok(json_value)
}

/// See https://json-schema.org/
fn inner_shape_to_json_schema(
    shape: &ReducedShape,
    mapping: &NamespacedTableMapping,
    virtual_mapping: &VirtualSystemMapping,
    value_format: ValueFormat,
) -> anyhow::Result<JsonValue> {
    // For recursion, to ensure we don't accidentally call the public
    // shape_to_json_schema recursively.
    let shape_to_json_schema = inner_shape_to_json_schema;

    let result = match shape {
        ReducedShape::Unknown => json_schemas::any(),
        // TODO: add proper support for this
        ReducedShape::Record { .. } => json_schemas::any(),
        ReducedShape::Never => json_schemas::never(),
        ReducedShape::Id(table_number) => {
            let table_name = all_tables_number_to_name(mapping, virtual_mapping)(*table_number)?;
            json_schemas::id(&table_name)
        },
        ReducedShape::Null => json_schemas::null(),
        ReducedShape::Int64 => json_schemas::int64(value_format),
        ReducedShape::Float64(range) => {
            json_schemas::float64(range.has_special_values, value_format)
        },
        ReducedShape::Boolean => json_schemas::boolean(),
        ReducedShape::String => json_schemas::string(),
        ReducedShape::Bytes => json_schemas::bytes(value_format),
        ReducedShape::Object(fields) => {
            let object_fields = fields
                .iter()
                .map(|(field_name, shape)| {
                    Ok((
                        field_name.to_string(),
                        json_schemas::FieldInfo {
                            schema: shape_to_json_schema(
                                &shape.shape,
                                mapping,
                                virtual_mapping,
                                value_format,
                            )?,
                            optional: shape.optional,
                        },
                    ))
                })
                .collect::<anyhow::Result<_>>()?;
            json_schemas::object(object_fields)
        },
        ReducedShape::Array(item_shape) => json_schemas::array(shape_to_json_schema(
            item_shape,
            mapping,
            virtual_mapping,
            value_format,
        )?),
        ReducedShape::Set(item_shape) => {
            // Unconditionally use ConvexEncodedJson for (deprecated) sets
            json_schemas::set(shape_to_json_schema(
                item_shape,
                mapping,
                virtual_mapping,
                ValueFormat::ConvexEncodedJSON,
            )?)
        },
        ReducedShape::Map {
            key_shape,
            value_shape,
        } => {
            // Unconditionally use ConvexEncodedJson for (deprecated) maps
            let map_format = ValueFormat::ConvexEncodedJSON;
            json_schemas::map(
                shape_to_json_schema(key_shape, mapping, virtual_mapping, map_format)?,
                shape_to_json_schema(value_shape, mapping, virtual_mapping, map_format)?,
            )
        },
        ReducedShape::Union(inner) => {
            let options = inner
                .iter()
                .map(|s| shape_to_json_schema(s, mapping, virtual_mapping, value_format))
                .collect::<anyhow::Result<Vec<_>>>()?;
            json_schemas::union(options)
        },
    };
    Ok(result)
}

#[cfg(test)]
mod test {
    use axum::body::Body;
    use axum_extra::headers::authorization::Credentials;
    use cmd_util::env::env_config;
    use common::{
        db_schema,
        object_validator,
        schemas::{
            validator::{
                FieldValidator,
                Validator,
            },
            DocumentSchema,
        },
        shapes::reduced::{
            ReducedField,
            ReducedFloatRange,
            ReducedShape,
        },
        testing::TestIdGenerator,
    };
    use convex_fivetran_source::api_types::ListSnapshotResponse;
    use database::UserFacingModel;
    use http::{
        Request,
        StatusCode,
    };
    use keybroker::Identity;
    use maplit::btreemap;
    use model::backend_info::{
        types::BackendInfoPersisted,
        BackendInfoModel,
    };
    use proptest::prelude::*;
    use runtime::prod::ProdRuntime;
    use serde_json::{
        json,
        Value as JsonValue,
    };
    use shape_inference::{
        testing::TestConfig,
        CountedShape,
    };
    use value::{
        assert_obj,
        export::ValueFormat,
        ConvexValue,
        FieldName,
        TableName,
        TableNamespace,
        TabletId,
    };

    use crate::{
        streaming_export::{
            get_columns_for_table,
            inner_shape_to_json_schema,
            shape_to_json_schema,
        },
        test_helpers::setup_backend_for_test,
    };

    fn insert_test_table_ids(shape: &ReducedShape, mapping: &mut TestIdGenerator) {
        if let ReducedShape::Id(table_number) = shape {
            if !mapping
                .namespace(TableNamespace::test_user())
                .table_number_exists()(*table_number)
            {
                let name = mapping.generate_table_name();
                let table_id = TabletId(mapping.generate_internal());
                mapping.insert(table_id, TableNamespace::test_user(), *table_number, name);
            }
        }
        match shape {
            ReducedShape::Object(ref obj) => obj
                .values()
                .for_each(|f| insert_test_table_ids(&f.shape, mapping)),
            ReducedShape::Array(ref arr) => insert_test_table_ids(arr, mapping),
            ReducedShape::Set(ref set) => insert_test_table_ids(set, mapping),
            ReducedShape::Map {
                ref key_shape,
                ref value_shape,
            } => {
                insert_test_table_ids(key_shape, mapping);
                insert_test_table_ids(value_shape, mapping);
            },
            ReducedShape::Union(ref shapes) => shapes
                .iter()
                .for_each(|s| insert_test_table_ids(s, mapping)),
            _ => (),
        }
    }

    #[test]
    fn test_empty_table() -> anyhow::Result<()> {
        let id_generator = TestIdGenerator::new();
        let posts_table_name: TableName = "posts".parse()?;
        for value_format in [ValueFormat::ConvexEncodedJSON, ValueFormat::ConvexCleanJSON] {
            let result = shape_to_json_schema(
                &ReducedShape::Never,
                None,
                &id_generator.namespace(TableNamespace::test_user()),
                &id_generator.virtual_system_mapping,
                &posts_table_name,
                value_format,
            )?;
            assert_eq!(
                result,
                json!({
                    "$schema": "http://json-schema.org/draft-07/schema#",
                    "type": "object",
                    "properties": {
                        "_creationTime": {"type": "number"},
                        "_id": {"type": "string", "$description": "Id(posts)" },
                    },
                    "additionalProperties": false,
                    "required": ["_creationTime", "_id"],
                })
            );
        }
        Ok(())
    }

    #[test]
    fn test_specific_schema() -> anyhow::Result<()> {
        let mut id_generator = TestIdGenerator::new();
        let posts_table_name: TableName = "posts".parse()?;
        let posts_id = id_generator.user_table_id(&posts_table_name).table_number;
        let users_id = id_generator.user_table_id(&"users".parse()?).table_number;
        let storage_id = id_generator.generate_virtual_table(&"_storage".parse()?);

        let object_fields = btreemap!(
            "_creationTime".parse()? => ReducedField {
                optional: false,
                shape: ReducedShape::Float64(ReducedFloatRange::new(false)),
            },
            "_id".parse()? => ReducedField {
                optional: false,
                shape: ReducedShape::Id(posts_id),
            },
            "attachment".parse()? => ReducedField {
                optional: false,
                shape: ReducedShape::Id(storage_id),
            },
            "author".parse()? => ReducedField {
                optional: false,
                shape: ReducedShape::Id(users_id),
            },
            "body".parse()? => ReducedField {
                optional: false,
                shape: ReducedShape::String,
            },
        );
        let shape = ReducedShape::Object(object_fields);
        assert_eq!(
            shape_to_json_schema(
                &shape,
                None,
                &id_generator.namespace(TableNamespace::test_user()),
                &id_generator.virtual_system_mapping,
                &posts_table_name,
                ValueFormat::ConvexCleanJSON
            )?,
            json!({
                "$schema": "http://json-schema.org/draft-07/schema#",
                "type": "object",
                "properties": {
                    "_creationTime": {"type": "number"},
                    "_id": {"type": "string", "$description": "Id(posts)" },
                    "attachment": {"type": "string", "$description": "Id(_storage)"},
                    "author": {"type": "string", "$description": "Id(users)"},
                    "body": {"type": "string"},
                },
                "additionalProperties": false,
                "required": ["_creationTime", "_id", "attachment", "author", "body"],
            })
        );
        Ok(())
    }

    #[test]
    fn test_json_schema_fallback_to_active_schema() -> anyhow::Result<()> {
        let id_generator = TestIdGenerator::new();
        let posts_table_name: TableName = "posts".parse()?;
        let users_table_name: TableName = "users".parse()?;

        let schema = db_schema!(
            posts_table_name => DocumentSchema::Union(
                vec![
                    object_validator!(
                        "author" => FieldValidator::required_field_type(Validator::Id(users_table_name)),
                        "body" => FieldValidator::required_field_type(Validator::String),
                    )
                ]
            )
        );
        assert_eq!(
            shape_to_json_schema(
                &ReducedShape::Unknown,
                Some(&schema),
                &id_generator.namespace(TableNamespace::test_user()),
                &id_generator.virtual_system_mapping,
                &posts_table_name,
                ValueFormat::ConvexCleanJSON
            )?,
            json!({
                "$schema": "http://json-schema.org/draft-07/schema#",
                "type": "object",
                "properties": {
                    "_creationTime": {"type": "number"},
                    "_id": {"type": "string", "$description": "Id(posts)" },
                    "author": {"type": "string", "$description": "Id(users)"},
                    "body": {"type": "string"},
                },
                "additionalProperties": false,
                "required": ["_creationTime", "_id","author", "body"],
            })
        );
        Ok(())
    }

    #[test]
    fn test_columns_object_shape() -> anyhow::Result<()> {
        let mut id_generator = TestIdGenerator::new();
        let posts_table_name: TableName = "posts".parse()?;
        let posts_id = id_generator.user_table_id(&posts_table_name).table_number;
        let users_id = id_generator.user_table_id(&"users".parse()?).table_number;

        let object_fields = btreemap!(
            "_creationTime".parse()? => ReducedField {
                optional: false,
                shape: ReducedShape::Float64(ReducedFloatRange::new(false)),
            },
            "_id".parse()? => ReducedField {
                optional: false,
                shape: ReducedShape::Id(posts_id),
            },
            "author".parse()? => ReducedField {
                optional: false,
                shape: ReducedShape::Id(users_id),
            },
            "body".parse()? => ReducedField {
                optional: false,
                shape: ReducedShape::String,
            },
        );
        let shape = ReducedShape::Object(object_fields);
        assert_eq!(
            get_columns_for_table(shape),
            vec![
                "_creationTime".to_string(),
                "_id".to_string(),
                "author".to_string(),
                "body".to_string()
            ]
        );
        Ok(())
    }

    #[test]
    fn test_columns_unknown_shape() -> anyhow::Result<()> {
        let shape = ReducedShape::Unknown;
        assert_eq!(
            get_columns_for_table(shape),
            vec!["_creationTime".to_string(), "_id".to_string()]
        );
        Ok(())
    }

    proptest! {
        #![proptest_config(
            ProptestConfig { cases: 256 * env_config("CONVEX_PROPTEST_MULTIPLIER", 1), failure_persistence: None, ..ProptestConfig::default() }
        )]

        #[test]
        fn proptest_toplevel_shape_json_schema_succeeds(
            reduced_obj in proptest::collection::btree_map(
                any::<FieldName>(),
                any::<ReducedField>(),
                0..8,
            ),
            value_format in any::<ValueFormat>(),
        ) {
            let reduced = ReducedShape::Object(reduced_obj);
            // table name not important here - just for error message. Avoid spreading
            // out test case generation.
            let table_name: TableName = "posts".parse().unwrap();
            let mut mapping = TestIdGenerator::new();
            insert_test_table_ids(&reduced, &mut mapping);
            let _json_schema = shape_to_json_schema(
                &reduced,
                None,
                &mapping.namespace(TableNamespace::test_user()),
                &mapping.virtual_system_mapping,
                &table_name,
                value_format,
            ).unwrap();
        }

        #[test]
        fn proptest_shape_json_schema_succeeds(
            reduced in any::<ReducedShape>(),
            value_format in any::<ValueFormat>(),
        ) {
            let mut mapping = TestIdGenerator::new();
            insert_test_table_ids(&reduced, &mut mapping);
            let _json_schema = inner_shape_to_json_schema(
                &reduced,
                &mapping.namespace(TableNamespace::test_user()),
                &mapping.virtual_system_mapping,
                value_format,
            ).unwrap();
        }


        #[test]
        fn proptest_object_fits_schema_type(
            val in any::<ConvexValue>(),
            value_format in any::<ValueFormat>(),
        ) {
            let mut mapping = TestIdGenerator::new();
            let shape = ReducedShape::from_type(&CountedShape::<TestConfig>::empty()
                .insert_value(&val), &|_| true);

            insert_test_table_ids(&shape, &mut mapping);

            let val_json = val.export(value_format);
            let json_schema = inner_shape_to_json_schema(
                &shape,
                &mapping.namespace(TableNamespace::test_user()),
                &mapping.virtual_system_mapping,
                value_format,
            ).unwrap();
            let schema = jsonschema::validator_for(&json_schema).expect("JSONSchema compiles");
            let result = schema.validate(&val_json);

            if result.is_err() {
                let mut logs = vec![
                    format!("Value Format {value_format:?}"),
                    format!("Value {val_json}"),
                    format!("Schema {json_schema}"),
                    format!("Shape {shape:?}"),
                ];
                for error in schema.iter_errors(&val_json) {
                    logs.push(format!("Validation error: {error}"));
                    logs.push(format!("Instance path: {}", error.instance_path));
                }
                panic!("schema validation failed:\n{}\n", logs.join("\n"));
            }
        }
    }

    #[convex_macro::prod_rt_test]
    async fn test_streaming_export_not_enabled(rt: ProdRuntime) -> anyhow::Result<()> {
        let backend = setup_backend_for_test(rt).await?;
        let mut tx = backend.st.application.begin(Identity::system()).await?;
        BackendInfoModel::new(&mut tx)
            .set(BackendInfoPersisted {
                streaming_export_enabled: false,
                ..Default::default()
            })
            .await?;
        backend.st.application.commit_test(tx).await?;
        for uri in [
            "/api/document_deltas",
            "/api/json_schemas",
            "/api/list_snapshot",
        ] {
            let req = Request::builder()
                .uri(uri)
                .method("GET")
                .header("Convex-Client", "airbyte-0.2.0")
                .body(Body::empty())?;
            backend
                .expect_error(req, StatusCode::FORBIDDEN, "StreamingExportNotEnabled")
                .await?;
        }
        Ok(())
    }

    #[convex_macro::prod_rt_test]
    async fn test_document_deltas_no_cursor(rt: ProdRuntime) -> anyhow::Result<()> {
        let backend = setup_backend_for_test(rt).await?;
        let req = Request::builder()
            .uri("/api/document_deltas")
            .method("GET")
            .header("Authorization", backend.admin_auth_header.0.encode())
            .header("Convex-Client", "airbyte-0.2.0")
            .body(Body::empty())?;
        backend
            .expect_error(req, StatusCode::BAD_REQUEST, "DocumentDeltasCursorRequired")
            .await?;
        Ok(())
    }

    #[convex_macro::prod_rt_test]
    async fn test_list_snapshot_empty_backend(rt: ProdRuntime) -> anyhow::Result<()> {
        let backend = setup_backend_for_test(rt).await?;
        tracing::error!("auth header: {:?}", backend.admin_auth_header);
        let req = Request::builder()
            .uri("/api/list_snapshot")
            .method("GET")
            .header("Authorization", backend.admin_auth_header.0.encode())
            .header("Convex-Client", "airbyte-0.2.0")
            .body(Body::empty())?;
        let ListSnapshotResponse {
            values,
            snapshot,
            cursor,
            has_more,
        } = backend.expect_success(req).await?;
        assert!(values.is_empty());
        assert!(cursor.is_none());
        assert_eq!(
            snapshot,
            i64::from(*backend.st.application.now_ts_for_reads()),
        );
        assert!(!has_more);
        Ok(())
    }

    #[convex_macro::prod_rt_test]
    async fn test_list_snapshot_serialization(rt: ProdRuntime) -> anyhow::Result<()> {
        let backend = setup_backend_for_test(rt).await?;

        // Insert a document
        let mut tx = backend.st.application.begin(Identity::system()).await?;
        let mut model = UserFacingModel::new(&mut tx, TableNamespace::root_component());
        let new_document_id = model
            .insert(
                "posts".parse().unwrap(),
                assert_obj!("title" => "Hello world"),
            )
            .await?;
        let creation_time = model
            .get(new_document_id, None)
            .await?
            .unwrap()
            .creation_time();
        let timestamp = backend.st.application.commit_test(tx).await?;

        let req = Request::builder()
            .uri("/api/list_snapshot")
            .method("GET")
            .header("Authorization", backend.admin_auth_header.0.encode())
            .body(Body::empty())?;
        let response: JsonValue = backend.expect_success(req).await?;

        assert_eq!(
            response,
            json!({
                "values": [
                    {
                        "_component": "",
                        "_table": "posts",
                        "_ts": i64::from(timestamp),
                        "_id": new_document_id.to_string(),
                        "_creationTime": f64::from(creation_time),
                        "title": "Hello world",
                    }
                ],
                "snapshot": i64::from(timestamp),
                "cursor": null,
                "hasMore": false,
            })
        );

        Ok(())
    }

    #[convex_macro::prod_rt_test]
    async fn test_document_deltas_serialization(rt: ProdRuntime) -> anyhow::Result<()> {
        let backend = setup_backend_for_test(rt).await?;

        // Insert a document
        let mut tx = backend.st.application.begin(Identity::system()).await?;
        let mut model = UserFacingModel::new(&mut tx, TableNamespace::root_component());
        let new_document_id = model
            .insert(
                "posts".parse().unwrap(),
                assert_obj!("title" => "Hello world"),
            )
            .await?;
        let creation_time = model
            .get(new_document_id, None)
            .await?
            .unwrap()
            .creation_time();
        let timestamp_create = backend.st.application.commit_test(tx).await?;

        // Delete it
        let mut tx = backend.st.application.begin(Identity::system()).await?;
        UserFacingModel::new(&mut tx, TableNamespace::root_component())
            .delete(new_document_id)
            .await?;
        let timestamp_delete = backend.st.application.commit_test(tx).await?;

        let req = Request::builder()
            .uri("/api/document_deltas?cursor=0")
            .method("GET")
            .header("Authorization", backend.admin_auth_header.0.encode())
            .body(Body::empty())?;
        let response: JsonValue = backend.expect_success(req).await?;

        assert_eq!(
            response,
            json!({
                "values": [
                    {
                        "_component": "",
                        "_table": "posts",
                        "_ts": i64::from(timestamp_create),
                        "_deleted": false,
                        "_id": new_document_id.to_string(),
                        "_creationTime": f64::from(creation_time),
                        "title": "Hello world",
                    },
                    {
                        "_component": "",
                        "_table": "posts",
                        "_ts": i64::from(timestamp_delete),
                        "_deleted": true,
                        "_id": new_document_id.to_string(),
                    },
                ],
                "cursor": i64::from(timestamp_delete),
                "hasMore": false,
            })
        );

        Ok(())
    }
}

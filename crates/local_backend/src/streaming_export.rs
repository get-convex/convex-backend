use std::{
    collections::BTreeMap,
    str::FromStr,
};

use anyhow::Context;
use axum::response::IntoResponse;
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
    execution_context::ExecutionId,
    http::{
        extract::{
            Json,
            MtState,
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
    types::{
        Timestamp,
        UdfIdentifier,
    },
    virtual_system_mapping::{
        all_tables_number_to_name,
        VirtualSystemMapping,
    },
    RequestId,
};
use database::{
    streaming_export_selection::StreamingExportSelection,
    BootstrapComponentsModel,
    DocumentDeltas,
    SchemaModel,
    SnapshotPage,
};
use errors::ErrorMetadata;
use fivetran_source::api_types::{
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
// Import for usage tracking
use usage_tracking::{
    self,
    CallType,
};
use value::{
    export::ValueFormat,
    NamespacedTableMapping,
    ResolvedDocumentId,
    TableName,
    TableNamespace,
};

use crate::{
    authentication::ExtractIdentity,
    LocalAppState,
};

#[fastrace::trace]
pub async fn document_deltas_get(
    MtState(st): MtState<LocalAppState>,
    Query(args): Query<DocumentDeltasArgs>,
    ExtractIdentity(identity): ExtractIdentity,
) -> Result<impl IntoResponse, HttpResponseError> {
    _document_deltas(st, args, identity).await
}

#[fastrace::trace]
pub async fn document_deltas_post(
    MtState(st): MtState<LocalAppState>,
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
    identity.require_operation(keybroker::DeploymentOp::ViewData)?;
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
        usage,
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

    st.application
        .usage_counter()
        .track_call(
            UdfIdentifier::SystemJob("streaming_export".to_string()),
            ExecutionId::new(),
            RequestId::new(),
            CallType::Export,
            true,
            usage,
        )
        .await;

    Ok((StatusCode::OK, Json(response)))
}

#[fastrace::trace]
pub async fn list_snapshot_get(
    MtState(st): MtState<LocalAppState>,
    Query(query_args): Query<ListSnapshotArgs>,
    ExtractIdentity(identity): ExtractIdentity,
) -> Result<impl IntoResponse, HttpResponseError> {
    _list_snapshot(st, query_args, identity).await
}

#[fastrace::trace]
pub async fn list_snapshot_post(
    MtState(st): MtState<LocalAppState>,
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
    identity.require_operation(keybroker::DeploymentOp::ViewData)?;
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
        usage,
    } = st
        .application
        .list_snapshot(identity.clone(), snapshot, cursor, selection)
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

    st.application
        .usage_counter()
        .track_call(
            UdfIdentifier::SystemJob("streaming_export".to_string()),
            ExecutionId::new(),
            RequestId::new(),
            CallType::Export,
            true,
            usage,
        )
        .await;

    Ok((StatusCode::OK, Json(response)))
}

/// Confirms that streaming export is enabled
pub async fn test_streaming_export_connection(
    MtState(st): MtState<LocalAppState>,
    ExtractIdentity(identity): ExtractIdentity,
) -> Result<impl IntoResponse, HttpResponseError> {
    st.application
        .ensure_streaming_export_enabled(identity.clone())
        .await?;
    identity.require_operation(keybroker::DeploymentOp::ViewData)?;
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
    MtState(st): MtState<LocalAppState>,
    ExtractIdentity(identity): ExtractIdentity,
) -> Result<impl IntoResponse, HttpResponseError> {
    st.application
        .ensure_streaming_export_enabled(identity.clone())
        .await?;
    let mut out = serde_json::Map::new();

    identity.require_operation(keybroker::DeploymentOp::ViewData)?;
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
    MtState(st): MtState<LocalAppState>,
    ExtractIdentity(identity): ExtractIdentity,
) -> Result<impl IntoResponse, HttpResponseError> {
    st.application
        .ensure_streaming_export_enabled(identity.clone())
        .await?;
    identity.require_operation(keybroker::DeploymentOp::ViewData)?;

    let snapshot = st.application.latest_snapshot()?;
    let mapping = snapshot.table_mapping();
    let component_paths = snapshot.component_ids_to_paths();

    let by_component: BTreeMap<ComponentPath, Vec<GetTableColumnNameTable>> = snapshot
        .table_registry
        .user_table_names()
        .flat_map(
            |row| -> Option<anyhow::Result<(&ComponentPath, GetTableColumnNameTable)>> {
                let (namespace, table_name) = row;

                let Some(component_path) = component_paths.get(&ComponentId::from(namespace))
                else {
                    // table_registry.user_table_names includes tables from orphaned namespaces:
                    // it is safe to ignore tables in components that are not present in
                    // component_paths
                    return None;
                };

                let table_summary = match snapshot.must_table_summary(namespace, table_name) {
                    Ok(table_summary) => table_summary,
                    Err(err) => return Some(Err(err)),
                };
                let columns = get_columns_for_table(ReducedShape::from_type(
                    table_summary.inferred_type(),
                    &mapping.namespace(namespace).table_number_exists(),
                ));

                Some(Ok((
                    component_path,
                    GetTableColumnNameTable {
                        name: table_name.to_string(),
                        columns,
                    },
                )))
            },
        )
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
    MtState(st): MtState<LocalAppState>,
    Query(query_args): Query<JsonSchemaArgs>,
    ExtractIdentity(identity): ExtractIdentity,
) -> Result<impl IntoResponse, HttpResponseError> {
    st.application
        .ensure_streaming_export_enabled(identity.clone())
        .await?;
    identity.require_operation(keybroker::DeploymentOp::ViewData)?;
    let mut out = serde_json::Map::new();

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
                if let Some(m) = json_schema.as_object_mut()
                    && let Some(properties) = m.get_mut("properties")
                    && let Some(properties) = properties.as_object_mut()
                {
                    properties.insert("_table".to_string(), json!({"type": "string"}));
                    properties.insert("_component".to_string(), json!({"type": "string"}));
                    properties.insert("_ts".to_string(), json!({"type": "integer"}));
                    properties.insert("_deleted".to_string(), json!({"type": "boolean"}));
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

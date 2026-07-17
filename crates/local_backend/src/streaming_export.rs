use std::{
    collections::BTreeMap,
    str::FromStr,
    sync::Arc,
    time::SystemTime,
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
    execution_context::{
        ExecutionId,
        RequestMetadata,
    },
    http::{
        extract::{
            Json,
            MtState,
            Query,
        },
        ExtractClientVersion,
        ExtractRequestMetadata,
        HttpResponseError,
        PaginationMetadata,
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
        streaming_export::{
            selection::Selection,
            ActiveDataSync,
            ActiveDataSyncInProgress,
            ActiveDataSyncStatus,
            ActiveDataSyncSynced,
            DataSyncArgs,
            DataSyncInProgress,
            DataSyncResponse,
            DataSyncStatus,
            DataSyncSynced,
            DataSyncTruncate,
            DataSyncValue,
            DocumentDeltasArgs,
            DocumentDeltasResponse,
            DocumentDeltasValue,
            GetTableColumnNameTable,
            GetTableColumnNamesResponse,
            InProgressTag,
            ListActiveSyncsResponse,
            ListSnapshotArgs,
            ListSnapshotResponse,
            ListSnapshotValue,
            SyncedTag,
        },
        RepeatableTimestamp,
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
    table_summary::table_summary_bootstrapping_error,
    BootstrapComponentsModel,
    DocumentDeltas,
    SchemaModel,
    SnapshotPage,
    TableShapes,
};
use errors::{
    ErrorMetadata,
    ErrorMetadataAnyhowExt,
};
use http::StatusCode;
use keybroker::Identity;
use maplit::btreemap;
use model::{
    data_sync_progress::types::DataSyncState,
    virtual_system_mapping,
};
use roles::RequireDeploymentOp;
use serde::{
    Deserialize,
    Serialize,
};
use serde_json::{
    json,
    Value as JsonValue,
};
use streaming_export::{
    DataSyncClient,
    SyncCursor,
    SyncEntry,
    SyncResult,
    SyncStatus,
    SyncTruncate,
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
    DocumentDeltasArgs {
        cursor,
        selection,
        format,
    }: DocumentDeltasArgs,
    identity: Identity,
) -> Result<impl IntoResponse, HttpResponseError> {
    let cursor_age_secs = cursor
        .and_then(|c| Timestamp::try_from(c).ok())
        .and_then(|cursor_ts| {
            Timestamp::try_from(SystemTime::now())
                .ok()
                .map(|now| now.secs_since_f64(cursor_ts))
        });
    tracing::info!("document_deltas call with cursor={cursor:?} (age={cursor_age_secs:?}s)");
    st.application
        .ensure_streaming_export_enabled(identity.clone())
        .await?;
    identity.require_operation(keybroker::DeploymentOp::ViewData)?;
    let cursor = cursor
        .context(ErrorMetadata::bad_request(
            "DocumentDeltasCursorRequired",
            "/api/document_deltas requires a cursor",
        ))?
        .try_into()?;

    let selection = Selection::from(selection);
    let selection = StreamingExportSelection::try_from(selection)?;

    let DocumentDeltas {
        deltas,
        cursor: new_cursor,
        has_more,
        mut usage,
    } = st
        .application
        .document_deltas(identity, cursor, selection)
        .await?;
    let value_format = format
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
    let response_bytes = serde_json::to_vec(&response).context("Failed to serialize response")?;

    *usage
        .fetch_egress
        .entry("/api/document_deltas".to_string())
        .or_default() += response_bytes.len() as u64;
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

    Ok((
        StatusCode::OK,
        (
            [(http::header::CONTENT_TYPE, "application/json")],
            response_bytes,
        ),
    ))
}

/// Data sync
///
/// **Early access:** this API is not yet stable and may change in
/// backwards-incompatible ways without notice. Contact the Convex team before
/// depending on it.
///
/// Streams a consistent, resumable export of a deployment's data — either the
/// whole deployment or a subset of components, tables, and columns (see the
/// request body). Streaming export must be enabled on the deployment, and the
/// caller must have the `deployment:data:view` permission.
///
/// Call this endpoint repeatedly, passing the `pagination.nextCursor` from each
/// response back in the next request as `cursor`; omit `cursor` on the first
/// call. The cursor is opaque — store and send it back verbatim. Each response
/// contains:
///
/// - `values`: document revisions in the order they should be applied. Each
///   entry carries the document's fields under `value`; an entry with `deleted:
///   true` is a tombstone marking that document as deleted.
/// - `truncates`: tables whose contents were replaced wholesale (for example by
///   an `npx convex import`). Drop everything you have stored for each listed
///   table; the `values` in this and later responses re-populate it.
/// - `status`: `inProgress` while the export is still being assembled — the
///   data returned so far is not yet a consistent view, so keep calling. Once
///   it becomes `synced`, the values applied so far form a consistent snapshot
///   of the deployment as of the returned `syncedTs` timestamp. You can keep
///   calling to continue streaming later changes.
/// - `pagination`: `nextCursor` to pass back on the next call (always present,
///   since the sync is always resumable) and `hasMore`, which tells you whether
///   more data is already available (`true`) or you've caught up to the latest
///   commit (`false`).
///
/// Persist the results and cursor to each page atomically. Continue calling the
/// endpoint with the cursor to progress the data sync. This endpoint must be
/// called at least once every 3 days, or the sync will expire and can no longer
/// be resumed. When that happens the endpoint responds with a `400`
/// (`DataSyncCursorExpired`), and you must restart the sync from scratch by
/// calling again with no cursor.
///
/// Each sync's progress is periodically recorded while the sync is in
/// progress and can be monitored via `/data/list_active_syncs`, keyed by the
/// `syncId` returned in every response.
#[utoipa::path(
    post,
    path = "/data/sync",
    tag = "Data Sync",
    request_body = DataSyncArgs,
    responses((status = 200, body = DataSyncResponse)),
    security(
        ("Deploy Key" = []),
        ("OAuth Team Token" = []),
        ("Team Token" = []),
        ("OAuth Project Token" = []),
    ),
)]
#[fastrace::trace]
pub async fn data_sync_post(
    MtState(st): MtState<LocalAppState>,
    ExtractIdentity(identity): ExtractIdentity,
    ExtractClientVersion(client_version): ExtractClientVersion,
    ExtractRequestMetadata(request_metadata): ExtractRequestMetadata,
    Json(args): Json<DataSyncArgs>,
) -> Result<impl IntoResponse, HttpResponseError> {
    _data_sync(
        st,
        args,
        identity,
        DataSyncClient::from(client_version.client()),
        request_metadata,
    )
    .await
}

#[derive(Deserialize, utoipa::IntoParams)]
#[serde(rename_all = "camelCase")]
pub struct ListActiveSyncsArgs {
    /// Maximum number of syncs to return (defaults to 50, capped at 100).
    limit: Option<usize>,
    /// Cursor from a previous response to fetch the next page.
    cursor: Option<String>,
}

/// List active data syncs
///
/// **Early access:** this API is not yet stable and may change in
/// backwards-incompatible ways without notice. Contact the Convex team before
/// depending on it.
///
/// Returns the progress of every active data sync: one that fetched a page
/// from `/data/sync` within the past 3 days, whether it is still performing
/// its initial traversal or is already synced and streaming changes. Progress
/// is recorded periodically, so an in-flight sync's numbers may trail its
/// most recent page.
///
/// Results are paginated, most recently updated first. Pass the returned
/// `nextCursor` back as `cursor` to fetch the next page.
#[utoipa::path(
    get,
    path = "/data/list_active_syncs",
    tag = "Data Sync",
    params(ListActiveSyncsArgs),
    responses((status = 200, body = ListActiveSyncsResponse)),
    security(
        ("Deploy Key" = []),
        ("OAuth Team Token" = []),
        ("Team Token" = []),
        ("OAuth Project Token" = []),
    ),
)]
#[fastrace::trace]
pub async fn list_active_syncs_get(
    MtState(st): MtState<LocalAppState>,
    Query(args): Query<ListActiveSyncsArgs>,
    ExtractIdentity(identity): ExtractIdentity,
) -> Result<impl IntoResponse, HttpResponseError> {
    st.application
        .ensure_streaming_export_enabled(identity.clone())
        .await?;
    identity.require_operation(keybroker::DeploymentOp::ViewData)?;

    let (syncs, next_cursor) = st
        .application
        .active_data_syncs(identity, args.cursor, args.limit)
        .await?;
    let syncs = syncs
        .into_iter()
        .map(|doc| {
            let progress = doc.into_value();
            ActiveDataSync {
                sync_id: progress.sync_id,
                last_updated: progress.last_updated_ms as i64,
                status: match progress.state {
                    DataSyncState::InitialSync {
                        num_tables_synced,
                        total_tables,
                        current_component,
                        current_table,
                        num_documents_synced_in_current_table,
                        total_documents_in_current_table,
                        num_documents_synced,
                        total_documents,
                    } => ActiveDataSyncStatus::InProgress(ActiveDataSyncInProgress {
                        status_type: InProgressTag::InProgress,
                        num_tables_synced,
                        total_tables,
                        current_component: String::from(current_component),
                        current_table: current_table.to_string(),
                        num_documents_in_current_table: num_documents_synced_in_current_table,
                        total_documents_in_current_table,
                        num_documents_synced,
                        total_documents,
                    }),
                    DataSyncState::Synced {
                        total_tables,
                        num_documents_synced,
                        synced_ts,
                    } => ActiveDataSyncStatus::Synced(ActiveDataSyncSynced {
                        status_type: SyncedTag::Synced,
                        total_tables,
                        num_documents_synced,
                        synced_ts,
                    }),
                },
            }
        })
        .collect();

    Ok(Json(ListActiveSyncsResponse {
        syncs,
        pagination: PaginationMetadata {
            has_more: next_cursor.is_some(),
            next_cursor,
        },
    }))
}

/// Platform (OpenAPI-documented) routes for streaming export.
pub fn platform_router<S>() -> utoipa_axum::router::OpenApiRouter<S>
where
    LocalAppState: axum::extract::FromRef<S>,
    S: Clone + Send + Sync + 'static,
{
    utoipa_axum::router::OpenApiRouter::new()
        .routes(utoipa_axum::routes!(data_sync_post))
        .routes(utoipa_axum::routes!(list_active_syncs_get))
}

async fn _data_sync(
    st: LocalAppState,
    DataSyncArgs { cursor, selection }: DataSyncArgs,
    identity: Identity,
    sync_client: DataSyncClient,
    request_metadata: RequestMetadata,
) -> Result<impl IntoResponse, HttpResponseError> {
    st.application
        .ensure_streaming_export_enabled(identity.clone())
        .await?;
    identity.require_operation(keybroker::DeploymentOp::ViewData)?;

    let cursor = cursor
        .map(|cursor| -> anyhow::Result<SyncCursor> {
            let bytes = base64::decode(&cursor).context(ErrorMetadata::bad_request(
                "InvalidDataSyncCursor",
                "Could not base64-decode the data sync cursor",
            ))?;
            SyncCursor::from_bytes(&bytes).context(ErrorMetadata::bad_request(
                "InvalidDataSyncCursor",
                "Could not parse the data sync cursor",
            ))
        })
        .transpose()?;

    let selection = Selection::from(selection);
    let selection = StreamingExportSelection::try_from(selection)?;

    // The data sync API always uses the uniform, lossless `ConvexExportJSON`
    // encoding (the same format as snapshot/zip exports). Callers don't get to
    // choose, so the wire format is stable.
    let value_format = ValueFormat::ConvexExportJSON;

    let SyncResult {
        truncates,
        entries,
        cursor: new_cursor,
        status,
        mut usage,
    } = st
        .application
        .data_sync(identity, cursor, selection, sync_client, request_metadata)
        .await
        .map_err(|e| {
            // The cursor points at a snapshot that has aged out of the
            // deployment's data retention window (see the endpoint docs: call at
            // least once every 3 days). It can't be resumed, so surface a 400
            // telling the caller to restart the sync from scratch.
            if e.is_out_of_retention() {
                e.context(ErrorMetadata::bad_request(
                    "DataSyncCursorExpired",
                    "The data sync cursor is outside the deployment's data retention window and \
                     can no longer be resumed. Restart the sync from scratch by calling this \
                     endpoint again without a cursor.",
                ))
            } else {
                e
            }
        })?;

    let truncates = truncates
        .into_iter()
        .map(|SyncTruncate { component, table }| DataSyncTruncate {
            component: component.to_string(),
            table: table.to_string(),
        })
        .collect();

    let values = entries
        .into_iter()
        .map(|entry| -> anyhow::Result<DataSyncValue> {
            Ok(match entry {
                SyncEntry::Document {
                    ts,
                    component,
                    table,
                    document,
                } => DataSyncValue {
                    component: component.to_string(),
                    table: table.to_string(),
                    ts: i64::from(ts),
                    deleted: false,
                    value: document.export_fields(value_format)?,
                },
                SyncEntry::Tombstone {
                    ts,
                    component,
                    table,
                    id,
                } => DataSyncValue {
                    component: component.to_string(),
                    table: table.to_string(),
                    ts: i64::from(ts),
                    deleted: true,
                    value: btreemap! {
                        "_id".to_string() => JsonValue::from(id),
                    },
                },
            })
        })
        .try_collect()?;

    let (status, has_more) = match status {
        SyncStatus::Synced { ts, has_more } => (
            DataSyncStatus::Synced(DataSyncSynced {
                status_type: SyncedTag::Synced,
                synced_ts: i64::from(ts),
            }),
            has_more,
        ),
        // Progress details are not part of this response; callers monitor
        // them via `/data/list_active_syncs`, keyed by `sync_id`. The snapshot
        // isn't consistent yet, so there is always more to fetch.
        SyncStatus::InProgress { .. } => (
            DataSyncStatus::InProgress(DataSyncInProgress {
                status_type: InProgressTag::InProgress,
            }),
            true,
        ),
    };

    let response = DataSyncResponse {
        truncates,
        values,
        sync_id: new_cursor.sync_id().to_string(),
        status,
        pagination: PaginationMetadata {
            has_more,
            // The cursor is always resumable, so a data sync never signals the
            // end with a null cursor the way a finite listing does.
            next_cursor: Some(base64::encode(new_cursor.to_bytes()?)),
        },
    };
    let response_bytes = serde_json::to_vec(&response).context("Failed to serialize response")?;

    *usage
        .fetch_egress
        .entry("/api/v1/data/sync".to_string())
        .or_default() += response_bytes.len() as u64;
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

    Ok((
        StatusCode::OK,
        (
            [(http::header::CONTENT_TYPE, "application/json")],
            response_bytes,
        ),
    ))
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
    ListSnapshotArgs {
        snapshot,
        cursor,
        selection,
        format,
    }: ListSnapshotArgs,
    identity: Identity,
) -> Result<impl IntoResponse, HttpResponseError> {
    tracing::info!("Received call to list_snapshot with snapshot={snapshot:?} cursor={cursor:?}");
    st.application
        .ensure_streaming_export_enabled(identity.clone())
        .await?;
    identity.require_operation(keybroker::DeploymentOp::ViewData)?;
    let snapshot = snapshot.map(Timestamp::try_from).transpose()?;

    #[derive(Serialize, Deserialize)]
    struct ListSnapshotCursor {
        tablet: String,
        id: String,
    }

    let cursor: Option<ResolvedDocumentId> = cursor
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

    let selection = Selection::from(selection);
    let selection = StreamingExportSelection::try_from(selection)?;

    let SnapshotPage {
        documents,
        snapshot,
        cursor: new_cursor,
        has_more,
        mut usage,
    } = st
        .application
        .list_snapshot(identity.clone(), snapshot, cursor, selection)
        .await?;
    let value_format = format
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
    let response_bytes = serde_json::to_vec(&response).context("Failed to serialize response")?;

    *usage
        .fetch_egress
        .entry("/api/list_snapshot".to_string())
        .or_default() += response_bytes.len() as u64;
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

    Ok((
        StatusCode::OK,
        (
            [(http::header::CONTENT_TYPE, "application/json")],
            response_bytes,
        ),
    ))
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

    let ts = st.application.now_ts_for_reads();
    let snapshot = st.application.snapshot(ts)?;
    let table_shapes = st.application.table_shapes_at(ts).await?;
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

                let shape = match reduced_table_shape(
                    &table_shapes,
                    ts,
                    &mapping.namespace(namespace),
                    table_name,
                ) {
                    Ok(shape) => shape,
                    Err(err) => return Some(Err(err)),
                };
                let columns = get_columns_for_table(shape);

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
    let table_shapes = st.application.table_shapes_at(tx.begin_timestamp()).await?;
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
            let shape =
                reduced_table_shape(&table_shapes, tx.begin_timestamp(), &mapping, table_name)?;
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

/// A table's shape from the table shapes, reduced for export.
/// Errors while shapes have never been published (still bootstrapping).
/// The shapes must be exact at the snapshot's timestamp (`table_shapes_at`),
/// so a table missing from them has no documents and gets `Never`.
fn reduced_table_shape(
    table_shapes: &Option<Arc<TableShapes>>,
    snapshot_ts: RepeatableTimestamp,
    mapping: &NamespacedTableMapping,
    table_name: &TableName,
) -> anyhow::Result<ReducedShape> {
    let Some(table_shapes) = table_shapes else {
        return Err(table_summary_bootstrapping_error(None));
    };
    anyhow::ensure!(
        table_shapes.ts == *snapshot_ts,
        "Table shapes ts {} does not match the snapshot ts {}",
        table_shapes.ts,
        *snapshot_ts,
    );
    Ok(match table_shapes.table_shape(mapping, table_name) {
        Some(table_shape) => {
            ReducedShape::from_type(table_shape.inferred_type(), &mapping.table_number_exists())
        },
        None => ReducedShape::Never,
    })
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
            "There is no active schema, which is needed for streaming export.".to_string()
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

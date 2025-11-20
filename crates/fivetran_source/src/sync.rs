use std::collections::{
    BTreeMap,
    BTreeSet,
};

use anyhow::Context;
use convex_fivetran_common::fivetran_sdk::{
    self,
    update_response,
    value_type,
    Record,
    RecordType,
    UpdateResponse as FivetranUpdateResponse,
    ValueType,
};
use futures::{
    stream::BoxStream,
    StreamExt,
};
use futures_async_stream::try_stream;
use serde::{
    Deserialize,
    Serialize,
};
use value_type::Inner as FivetranValue;

use crate::{
    api_types::selection::Selection,
    convert::to_fivetran_row,
    convex_api::{
        DocumentDeltasCursor,
        ListSnapshotCursor,
        SnapshotValue,
        Source,
    },
    log::log,
};

/// The value currently used for the `version` field of [`State`].
const CURSOR_VERSION: i64 = 2;

/// Stores the current synchronization state of a destination. A state will be
/// send (as JSON) to Fivetran every time we perform a checkpoint, and will be
/// returned to us every time Fivetran calls the `update` method of the
/// connector.
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
#[serde(rename_all = "camelCase")]
#[serde(deny_unknown_fields)]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
pub struct State {
    /// The version of the connector that emitted this checkpoint. Could be used
    /// in the future to support backward compatibility with older state
    /// formats.
    pub version: i64,

    pub checkpoint: Checkpoint,

    /// If set, then we are tracking the full set of tables that the connector
    /// has every seen, so we are able to issue truncates the first time we
    /// see a table.
    ///
    /// Older versions of state.json do not have this field set. Once all
    /// state.json have this field, we can make this non-optional.
    ///
    /// The format of this string is `{table_name}` for the root component,
    /// or `{component_path}/{table_name}` for tables in other components.
    pub tables_seen: Option<BTreeSet<String>>,
}

impl State {
    pub fn create(checkpoint: Checkpoint, tables_seen: Option<BTreeSet<String>>) -> Self {
        Self {
            version: CURSOR_VERSION,
            checkpoint,
            tables_seen,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
#[serde(deny_unknown_fields)]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
pub enum Checkpoint {
    /// A checkpoint emitted during the initial synchonization.
    InitialSync {
        snapshot: i64,
        cursor: ListSnapshotCursor,
    },
    /// A checkpoint emitted after an initial synchronzation has been completed.
    DeltaUpdates { cursor: DocumentDeltasCursor },
}

/// A simplification of the messages sent to Fivetran in the `update` endpoint.
#[derive(Debug)]
pub enum UpdateMessage {
    Update {
        schema_name: Option<String>,
        table_name: String,
        op_type: RecordType,
        row: BTreeMap<String, FivetranValue>,
    },
    Checkpoint(State),
}

/// Conversion of the simplified update message type to the actual gRPC type.
impl From<UpdateMessage> for FivetranUpdateResponse {
    fn from(value: UpdateMessage) -> Self {
        FivetranUpdateResponse {
            operation: Some(match value {
                UpdateMessage::Update {
                    schema_name,
                    table_name,
                    op_type,
                    row,
                } => update_response::Operation::Record(Record {
                    schema_name,
                    table_name,
                    r#type: op_type as i32,
                    data: row
                        .into_iter()
                        .map(|(field_name, field_value)| {
                            (
                                field_name,
                                ValueType {
                                    inner: Some(field_value),
                                },
                            )
                        })
                        .collect(),
                }),
                UpdateMessage::Checkpoint(checkpoint) => {
                    let state_json = serde_json::to_string(&checkpoint)
                        .expect("Couldn’t serialize a checkpoint");
                    update_response::Operation::Checkpoint(fivetran_sdk::Checkpoint { state_json })
                },
            }),
        }
    }
}

/// Returns the stream that the `update` endpoint emits.
pub fn sync(
    source: impl Source + 'static,
    state: Option<State>,
    selection: Selection,
) -> BoxStream<'static, anyhow::Result<UpdateMessage>> {
    let Some(state) = state else {
        return initial_sync(source, None, Some(BTreeSet::new()), selection).boxed();
    };

    let State {
        version: _version,
        checkpoint,
        tables_seen,
    } = state;
    match checkpoint {
        Checkpoint::InitialSync { snapshot, cursor } => {
            initial_sync(source, Some((snapshot, cursor)), tables_seen, selection).boxed()
        },
        Checkpoint::DeltaUpdates { cursor } => {
            delta_sync(source, cursor, tables_seen, selection).boxed()
        },
    }
}

/// Performs (or resume) an initial synchronization.
#[try_stream(ok = UpdateMessage, error = anyhow::Error)]
async fn initial_sync(
    source: impl Source,
    mut checkpoint: Option<(i64, ListSnapshotCursor)>,
    mut tables_seen: Option<BTreeSet<String>>,
    selection: Selection,
) {
    let log_msg = if let Some((snapshot, _)) = checkpoint {
        format!("Resuming an initial sync from {source} at {snapshot}")
    } else {
        format!("Starting an initial sync from {source}")
    };
    log(&log_msg);

    let snapshot = loop {
        let snapshot = checkpoint.as_ref().map(|c| c.0);
        let cursor = checkpoint.as_ref().map(|c| c.1.clone());
        let res = source
            .list_snapshot(snapshot, cursor.clone(), selection.clone())
            .await?;

        for value in res.values {
            if let Some(ref mut tables_seen) = tables_seen {
                // Issue truncates if we see a table for the first time.
                // Skip the behavior for legacy state.json - where tables_seen wasn't tracked.
                let table_seen_key = value.table_path_for_state();
                if !tables_seen.contains(&table_seen_key) {
                    tables_seen.insert(table_seen_key);
                    yield UpdateMessage::Update {
                        schema_name: Some(value.fivetran_schema_name()),
                        table_name: value.table.clone(),
                        op_type: RecordType::Truncate,
                        row: BTreeMap::new(),
                    };
                }
            }
            yield UpdateMessage::Update {
                schema_name: Some(value.fivetran_schema_name()),
                table_name: value.table,
                op_type: RecordType::Upsert,
                row: to_fivetran_row(value.fields)?,
            };
        }

        if res.has_more {
            let cursor = ListSnapshotCursor::from(
                res.cursor.context("Missing cursor when has_more was set")?,
            );
            yield UpdateMessage::Checkpoint(State::create(
                Checkpoint::InitialSync {
                    snapshot: res.snapshot,
                    cursor: cursor.clone(),
                },
                tables_seen.clone(),
            ));
            checkpoint = Some((res.snapshot, cursor));
        } else {
            break res.snapshot;
        }
    };

    let cursor = DocumentDeltasCursor::from(snapshot);
    yield UpdateMessage::Checkpoint(State::create(
        Checkpoint::DeltaUpdates { cursor },
        tables_seen,
    ));

    log(&format!(
        "Initial sync from {source} successful at cursor {cursor}."
    ));
}

/// Synchronizes the changes that happened after an initial synchronization or
/// delta synchronization has been completed.
#[try_stream(ok = UpdateMessage, error = anyhow::Error)]
async fn delta_sync(
    source: impl Source,
    cursor: DocumentDeltasCursor,
    mut tables_seen: Option<BTreeSet<String>>,
    selection: Selection,
) {
    log(&format!("Delta sync from {source} starting at {cursor}."));

    let mut cursor = cursor;
    let mut has_more = true;
    while has_more {
        let response = source.document_deltas(cursor, selection.clone()).await?;

        for value in response.values {
            if let Some(ref mut tables_seen) = tables_seen {
                // Issue truncates if we see a table for the first time.
                // Skip the behavior for legacy state.json - where tables_seen wasn't tracked.
                let table_seen_key = value.table_path_for_state();
                if !tables_seen.contains(&table_seen_key) {
                    tables_seen.insert(table_seen_key);
                    yield UpdateMessage::Update {
                        schema_name: Some(value.fivetran_schema_name()),
                        table_name: value.table.clone(),
                        op_type: RecordType::Truncate,
                        row: BTreeMap::new(),
                    };
                }
            }

            yield UpdateMessage::Update {
                schema_name: Some(value.fivetran_schema_name()),
                table_name: value.table,
                op_type: if value.deleted {
                    RecordType::Delete
                } else {
                    RecordType::Upsert
                },
                row: to_fivetran_row(value.fields)?,
            };
        }

        cursor = DocumentDeltasCursor::from(response.cursor);
        has_more = response.has_more;

        // It is safe to take a snapshot here, because document_deltas
        // guarantees that the state given by one call is consistent.
        yield UpdateMessage::Checkpoint(State::create(
            Checkpoint::DeltaUpdates { cursor },
            tables_seen.clone(),
        ));
    }

    log(&format!(
        "Delta sync changes applied from {source}. Final cursor {cursor}"
    ));
}

#[cfg(test)]
mod state_serialization_tests {
    use cmd_util::env::env_config;
    use proptest::prelude::*;

    use crate::sync::{
        Checkpoint,
        State,
    };

    proptest! {
        #![proptest_config(ProptestConfig {
            cases: 256 * env_config("CONVEX_PROPTEST_MULTIPLIER", 1),
            failure_persistence: None, ..ProptestConfig::default()
        })]
        #[test]
        fn state_json_roundtrips(value in any::<State>()) {
            let json = serde_json::to_string(&value).unwrap();
            prop_assert_eq!(value, serde_json::from_str(&json).unwrap());
        }
    }

    #[test]
    fn refuses_unknown_state_object() {
        assert!(serde_json::from_str::<State>("{\"a\": \"b\"}").is_err());
    }

    #[test]
    fn refuses_unknown_checkpoint_object() {
        assert!(serde_json::from_str::<State>(
            "{ \"version\": 1, \"snapshot\": { \"NewState\": { \"cursor\": 42 } } }"
        )
        .is_err());
    }

    #[test]
    fn deserializes_v1_initial_sync_checkpoints() {
        assert_eq!(
            serde_json::from_str::<State>(
                "{ \"version\": 1, \"checkpoint\": { \"InitialSync\": { \"snapshot\": 42, \
                 \"cursor\": \"abc123\" } } }"
            )
            .unwrap(),
            State {
                version: 1,
                checkpoint: Checkpoint::InitialSync {
                    snapshot: 42,
                    cursor: String::from("abc123").into(),
                },
                tables_seen: None,
            },
        );
    }

    #[test]
    fn deserializes_v1_delta_update_checkpoints() {
        assert_eq!(
            serde_json::from_str::<State>(
                "{ \"version\": 1, \"checkpoint\": { \"DeltaUpdates\": { \"cursor\": 42 } } }"
            )
            .unwrap(),
            State {
                version: 1,
                checkpoint: Checkpoint::DeltaUpdates { cursor: 42.into() },
                tables_seen: None,
            },
        );
    }
}

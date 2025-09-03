use std::{
    collections::BTreeMap,
    fmt::Display,
    panic,
    vec,
};

use anyhow::{
    Context,
    Ok,
};
use async_trait::async_trait;
use convex_fivetran_common::fivetran_sdk::{
    value_type,
    RecordType,
};
use derive_more::From;
use futures::{
    Stream,
    StreamExt,
};
use maplit::btreemap;
use rand::Rng;
use serde_json::{
    json,
    Value as JsonValue,
};
use uuid::Uuid;
use value_type::Inner as FivetranValue;

use crate::{
    api_types::{
        DocumentDeltasResponse,
        DocumentDeltasValue,
        ListSnapshotResponse,
        ListSnapshotValue,
    },
    convex_api::{
        ComponentPath,
        DocumentDeltasCursor,
        FieldName,
        ListSnapshotCursor,
        Source,
        TableName,
    },
    sync::{
        sync,
        Checkpoint,
        State,
        UpdateMessage,
    },
};

type JsonDocument = BTreeMap<String, JsonValue>;

#[derive(Debug, Clone)]
struct FakeSource {
    tables_by_component: BTreeMap<ComponentPath, BTreeMap<String, Vec<JsonDocument>>>,
    changelog: Vec<DocumentDeltasValue>,
}

impl Default for FakeSource {
    fn default() -> Self {
        FakeSource {
            tables_by_component: btreemap! {},
            changelog: vec![],
        }
    }
}

impl FakeSource {
    fn seeded() -> Self {
        let mut source = Self::default();
        for component in [ComponentPath::root(), ComponentPath::test_component()] {
            for table_name in ["table1", "table2", "table3"] {
                for i in 0..25 {
                    source.insert(
                        component.clone(),
                        table_name,
                        btreemap! {
                            "name".to_string() => json!(format!("Document {} of {}", i, table_name)),
                            "index".to_string() => json!(i),
                        },
                    );
                }
            }
        }

        source
    }

    pub fn insert(&mut self, component: ComponentPath, table_name: &str, mut value: JsonDocument) {
        if value.contains_key("_id") {
            panic!("ID specified while inserting a new row");
        }
        value.insert(
            "_id".to_string(),
            JsonValue::String(Uuid::new_v4().to_string()),
        );
        value.insert("_creationTime".to_string(), json!(0));

        self.tables_by_component
            .entry(component.clone())
            .or_default()
            .entry(table_name.to_string())
            .or_default()
            .push(value.clone().into_iter().collect());

        self.changelog.push(DocumentDeltasValue {
            table: table_name.to_string(),
            deleted: false,
            component: component.to_string(),
            fields: value,
            ts: 0, // Ignored by the connector
        });
    }

    fn patch(
        &mut self,
        component: ComponentPath,
        table_name: &str,
        index: usize,
        changed_fields: JsonValue,
    ) {
        let table = self
            .tables_by_component
            .entry(component.clone())
            .or_default()
            .get_mut(table_name)
            .unwrap();
        let element = table.get_mut(index).unwrap();
        for (key, value) in changed_fields.as_object().unwrap().iter() {
            if key.starts_with('_') {
                panic!("Trying to set a system field");
            }

            element.insert(key.clone(), value.clone());
        }

        self.changelog.push(DocumentDeltasValue {
            table: table_name.to_string(),
            deleted: false,
            component: component.to_string(),
            fields: element.clone(),
            ts: 0, // Ignored by the connector
        });
    }

    fn delete(&mut self, component: ComponentPath, table_name: &str, index: usize) {
        let table = self
            .tables_by_component
            .entry(component.clone())
            .or_default()
            .get_mut(table_name)
            .unwrap();
        let id = table
            .get(index)
            .unwrap()
            .get("_id")
            .unwrap()
            .as_str()
            .unwrap()
            .to_string();
        table.remove(index);
        self.changelog.push(DocumentDeltasValue {
            table: table_name.to_string(),
            deleted: true,
            component: component.to_string(),
            fields: btreemap! { "_id".to_string() => json!(id) },
            ts: 0, // Ignored by the connector
        })
    }
}

impl Display for FakeSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("fake_source")
    }
}

#[async_trait]
impl Source for FakeSource {
    async fn test_streaming_export_connection(&self) -> anyhow::Result<()> {
        Ok(())
    }

    async fn get_table_column_names(
        &self,
    ) -> anyhow::Result<BTreeMap<ComponentPath, BTreeMap<TableName, Vec<FieldName>>>> {
        let tables_by_component = self
            .tables_by_component
            .iter()
            .map(|(component, tables)| {
                let table_to_fields: BTreeMap<TableName, Vec<FieldName>> = tables
                    .iter()
                    .map(|(table_name, rows)| {
                        let field_names = rows
                            .iter()
                            .flat_map(|row| row.keys())
                            .map(|f| FieldName(f.to_string()))
                            .collect();
                        (TableName(table_name.to_string()), field_names)
                    })
                    .collect();
                (component.clone(), table_to_fields)
            })
            .collect();

        Ok(tables_by_component)
    }

    async fn list_snapshot(
        &self,
        snapshot: Option<i64>,
        cursor: Option<ListSnapshotCursor>,
    ) -> anyhow::Result<ListSnapshotResponse> {
        if snapshot.is_some() && snapshot != Some(self.changelog.len() as i64) {
            panic!("Unexpected snapshot value");
        }

        let cursor: usize = cursor.map(|c| c.0.parse().unwrap()).unwrap_or(0);
        let values_per_call = 10;
        let values: Vec<ListSnapshotValue> = self
            .tables_by_component
            .iter()
            .flat_map(|(component, tables)| {
                tables.iter().flat_map(|(table, docs)| {
                    docs.iter()
                        .map(|fields| ListSnapshotValue {
                            component: component.to_string(),
                            table: table.to_string(),
                            ts: 0, // ignored by the connector
                            fields: fields.clone(),
                        })
                        .collect::<Vec<_>>()
                })
            })
            .skip(cursor * values_per_call)
            .take(values_per_call)
            .collect();

        Ok(ListSnapshotResponse {
            has_more: values.len() == values_per_call,
            values,
            snapshot: self.changelog.len() as i64,
            cursor: Some((cursor + 1).to_string()),
        })
    }

    async fn document_deltas(
        &self,
        cursor: DocumentDeltasCursor,
    ) -> anyhow::Result<DocumentDeltasResponse> {
        let results_per_page = 5;
        let values: Vec<DocumentDeltasValue> = self
            .changelog
            .iter()
            .skip(i64::from(cursor) as usize)
            .take(results_per_page as usize)
            .cloned()
            .collect();
        let values_len = values.len() as i64;

        Ok(DocumentDeltasResponse {
            values,
            cursor: i64::from(cursor) + values_len,
            has_more: values_len == results_per_page,
        })
    }
}

#[derive(Default, Debug, PartialEq)]
struct FakeDestination {
    current_data: FakeDestinationData,
    checkpointed_data: FakeDestinationData,
    state: Option<State>,
}

#[derive(Default, Debug, PartialEq, Clone)]
struct FakeDestinationData {
    tables_by_component: BTreeMap<String, BTreeMap<String, Vec<BTreeMap<String, FivetranValue>>>>,
}

impl FakeDestination {
    fn latest_state(&self) -> Option<State> {
        self.state.clone()
    }

    async fn receive(
        &mut self,
        stream: impl Stream<Item = anyhow::Result<UpdateMessage>>,
    ) -> anyhow::Result<()> {
        let mut stream = Box::pin(stream);

        while let (Some(result), new_stream) = stream.into_future().await {
            stream = new_stream;

            match result? {
                UpdateMessage::Update {
                    schema_name,
                    table_name,
                    op_type,
                    row,
                } => {
                    let Some(schema_name) = schema_name else {
                        panic!("FakeDestination expects to receive a schema name");
                    };

                    let tables = self
                        .current_data
                        .tables_by_component
                        .entry(schema_name)
                        .or_default();

                    if !tables.contains_key(&table_name) {
                        tables.insert(table_name.clone(), vec![]);
                    }

                    if op_type == RecordType::Truncate {
                        tables.remove(&table_name);
                        continue;
                    }

                    let table = tables.get_mut(&table_name).expect("Unknown table name");
                    let id = row.get("_id").unwrap();
                    let position = table.iter().position(|row| row.get("_id").unwrap() == id);

                    match op_type {
                        RecordType::Upsert => {
                            match position {
                                Some(index) => table[index] = row,
                                None => table.push(row),
                            };
                        },
                        RecordType::Delete => {
                            table.remove(position.expect("Could not find the row to delete"));
                        },
                        _ => panic!("Operation not supported by the fake"),
                    };
                },
                UpdateMessage::Checkpoint(state) => {
                    self.checkpointed_data = self.current_data.clone();
                    self.state = Some(state);
                },
            }
        }

        Ok(())
    }
}

#[tokio::test]
async fn initial_sync_copies_documents_from_source_to_destination() -> anyhow::Result<()> {
    let source = FakeSource::seeded();
    let mut destination = FakeDestination::default();

    destination
        .receive(sync(source.clone(), destination.latest_state()))
        .await?;

    assert_eq!(
        source
            .tables_by_component
            .get(&ComponentPath::root())
            .unwrap()
            .len(),
        destination
            .checkpointed_data
            .tables_by_component
            .get("convex")
            .unwrap()
            .len()
    );
    assert_eq!(
        source
            .tables_by_component
            .get(&ComponentPath::root())
            .unwrap()
            .get("table1")
            .unwrap()
            .len(),
        destination
            .checkpointed_data
            .tables_by_component
            .get("convex")
            .unwrap()
            .get("table1")
            .unwrap()
            .len(),
    );

    assert_eq!(
        destination
            .checkpointed_data
            .tables_by_component
            .get("convex")
            .unwrap()
            .get("table1")
            .unwrap()
            .first()
            .unwrap()
            .get("name")
            .unwrap(),
        &FivetranValue::String("Document 0 of table1".to_string())
    );

    assert_eq!(
        destination
            .checkpointed_data
            .tables_by_component
            .get("convex")
            .unwrap()
            .get("table1")
            .unwrap()
            .get(21)
            .unwrap()
            .get("name")
            .unwrap(),
        &FivetranValue::String("Document 21 of table1".to_string())
    );

    Ok(())
}

#[tokio::test]
async fn initial_sync_empty_source_works() -> anyhow::Result<()> {
    let source = FakeSource::default();
    let mut destination = FakeDestination::default();

    assert_eq!(source.tables_by_component.len(), 0);
    destination
        .receive(sync(source.clone(), destination.latest_state()))
        .await?;
    assert_eq!(destination.checkpointed_data.tables_by_component.len(), 0);
    let state = destination.latest_state().context("missing state")?;
    assert!(matches!(
        state.checkpoint,
        Checkpoint::DeltaUpdates {
            cursor: DocumentDeltasCursor(0)
        }
    ));

    Ok(())
}

/// Verifies that the source and the destination are in sync by starting a new
/// initial sync and verifying that the destinations match.
async fn assert_in_sync(source: impl Source + 'static, destination: &FakeDestination) {
    let mut parallel_destination = FakeDestination::default();
    parallel_destination
        .receive(sync(source, parallel_destination.latest_state()))
        .await
        .expect("Unexpected error during parallel synchronization");
    assert_eq!(
        destination.checkpointed_data.tables_by_component,
        parallel_destination.checkpointed_data.tables_by_component
    );
}

async fn assert_not_in_sync(source: impl Source + 'static, destination: &FakeDestination) {
    let mut parallel_destination = FakeDestination::default();
    parallel_destination
        .receive(sync(source, parallel_destination.latest_state()))
        .await
        .expect("Unexpected error during parallel synchronization");
    assert_ne!(
        destination.checkpointed_data.tables_by_component,
        parallel_destination.checkpointed_data.tables_by_component
    );
}

#[tokio::test]
async fn initial_sync_synchronizes_the_destination_with_the_source() -> anyhow::Result<()> {
    let source = FakeSource::seeded();
    let mut destination = FakeDestination::default();

    assert_not_in_sync(source.clone(), &destination).await;

    destination
        .receive(sync(source.clone(), destination.latest_state()))
        .await?;

    assert_in_sync(source, &destination).await;

    Ok(())
}

#[tokio::test]
async fn sync_after_adding_a_document() -> anyhow::Result<()> {
    let mut source = FakeSource::seeded();
    let mut destination = FakeDestination::default();

    destination
        .receive(sync(source.clone(), destination.latest_state()))
        .await?;
    let state = destination.latest_state();

    source.insert(
        ComponentPath::root(),
        "table1",
        btreemap! {
            "name".to_string() => json!("New document"),
        },
    );
    source.insert(
        ComponentPath::test_component(),
        "table2",
        btreemap! {
            "name".to_string() => json!("New document"),
        },
    );
    destination.receive(sync(source.clone(), state)).await?;
    assert_in_sync(source, &destination).await;

    Ok(())
}

#[tokio::test]
async fn sync_after_modifying_a_document() -> anyhow::Result<()> {
    let mut source = FakeSource::seeded();
    let mut destination = FakeDestination::default();

    destination
        .receive(sync(source.clone(), destination.latest_state()))
        .await?;
    let state = destination.latest_state();

    source.patch(
        ComponentPath::root(),
        "table1",
        13,
        json!({
            "name": "New name",
        }),
    );
    source.patch(
        ComponentPath::test_component(),
        "table2",
        9,
        json!({
            "name": "New name",
        }),
    );
    destination.receive(sync(source.clone(), state)).await?;
    assert_in_sync(source, &destination).await;

    Ok(())
}

#[tokio::test]
async fn sync_after_deleting_a_document() -> anyhow::Result<()> {
    let mut source = FakeSource::seeded();
    let mut destination = FakeDestination::default();

    destination
        .receive(sync(source.clone(), destination.latest_state()))
        .await?;

    source.delete(ComponentPath::root(), "table1", 8);
    source.delete(ComponentPath::test_component(), "table3", 5);
    destination
        .receive(sync(source.clone(), destination.latest_state()))
        .await?;
    assert_in_sync(source, &destination).await;

    Ok(())
}

#[tokio::test]
async fn resync_after_sync_and_delete() -> anyhow::Result<()> {
    let mut source = FakeSource::seeded();
    let mut destination = FakeDestination::default();

    destination.receive(sync(source.clone(), None)).await?;
    source.delete(ComponentPath::root(), "table1", 8);
    source.delete(ComponentPath::test_component(), "table3", 5);
    // The sync + delete + resync tests to ensure that the connector
    // correctly truncates the destination before a resync.
    destination.receive(sync(source.clone(), None)).await?;
    assert_in_sync(source, &destination).await;

    Ok(())
}

/// Wrapper around a source that fails half of its calls.
#[derive(From)]
struct UnreliableSource {
    source: FakeSource,
}

impl Display for UnreliableSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&self.source, f)
    }
}

impl UnreliableSource {
    fn maybe_fail(&self) -> anyhow::Result<()> {
        if rand::rng().random_bool(0.5) {
            anyhow::bail!("Unreliable source error");
        }

        Ok(())
    }
}

#[async_trait]
impl Source for UnreliableSource {
    async fn test_streaming_export_connection(&self) -> anyhow::Result<()> {
        self.maybe_fail()?;
        self.source.test_streaming_export_connection().await
    }

    async fn list_snapshot(
        &self,
        snapshot: Option<i64>,
        cursor: Option<ListSnapshotCursor>,
    ) -> anyhow::Result<ListSnapshotResponse> {
        self.maybe_fail()?;
        self.source.list_snapshot(snapshot, cursor).await
    }

    async fn document_deltas(
        &self,
        cursor: DocumentDeltasCursor,
    ) -> anyhow::Result<DocumentDeltasResponse> {
        self.maybe_fail()?;
        self.source.document_deltas(cursor).await
    }

    async fn get_table_column_names(
        &self,
    ) -> anyhow::Result<BTreeMap<ComponentPath, BTreeMap<TableName, Vec<FieldName>>>> {
        self.maybe_fail()?;
        self.source.get_table_column_names().await
    }
}

#[tokio::test]
async fn can_perform_an_initial_sync_from_an_unreliable_source() -> anyhow::Result<()> {
    let source = FakeSource::seeded();
    let mut destination = FakeDestination::default();

    while destination
        .receive(sync(
            UnreliableSource::from(source.clone()),
            destination.latest_state(),
        ))
        .await
        .is_err()
    {}

    assert_in_sync(source, &destination).await;

    Ok(())
}

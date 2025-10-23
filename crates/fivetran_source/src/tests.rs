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
use pretty_assertions::{
    assert_eq,
    assert_ne,
};
use rand::Rng;
use serde_json::{
    json,
    Value as JsonValue,
};
use uuid::Uuid;
use value_type::Inner as FivetranValue;

use crate::{
    api_types::{
        selection::{
            ComponentSelection,
            InclusionDefault,
            Selection,
            TableSelection,
        },
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
    timestamp: i64,
}

impl Default for FakeSource {
    fn default() -> Self {
        FakeSource {
            tables_by_component: btreemap! {},
            changelog: vec![],
            timestamp: 0,
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
        value.insert("_creationTime".to_string(), json!(self.timestamp));

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
            ts: self.timestamp,
        });

        self.timestamp += 1;
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
            ts: self.timestamp,
        });
        self.timestamp += 1;
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
            ts: self.timestamp,
        });
        self.timestamp += 1;
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
        selection: Selection,
    ) -> anyhow::Result<ListSnapshotResponse> {
        if let Some(snapshot) = snapshot
            && snapshot != self.timestamp - 1
        {
            panic!(
                "The destination has unexpectedly changed between multiple list_snapshot calls \
                 (this is not supported by the fake source)"
            );
        }

        let values_per_call = 10;
        let cursor: usize = cursor.map(|c| c.0.parse().unwrap()).unwrap_or(0);

        let page_values: Vec<ListSnapshotValue> = self
            .tables_by_component
            .iter()
            .flat_map(|(component, table_to_rows)| {
                table_to_rows.iter().flat_map(|(table_name, docs)| {
                    is_table_included_in_selection(&selection, &component.0, table_name)
                        .then(|| (component.clone(), table_name, docs))
                })
            })
            .flat_map(|(component, table_name, docs)| {
                docs.iter()
                    .map(|fields| {
                        let column_filter =
                            get_table_selection(&selection, &component.0, table_name);
                        if column_filter != TableSelection::included_with_all_columns() {
                            panic!("The fake doesn’t support partial column selection");
                        }

                        let ts: i64 = fields
                            .get("_creationTime")
                            .expect("Missing _creationTime")
                            .as_i64()
                            .expect("Can’t parse _creationTime");

                        ListSnapshotValue {
                            component: component.0.clone(),
                            table: table_name.clone(),
                            fields: fields.clone(),
                            ts,
                        }
                    })
                    .collect::<Vec<_>>()
            })
            .skip(cursor)
            .take(values_per_call)
            .collect();

        Ok(ListSnapshotResponse {
            cursor: Some((cursor + page_values.len()).to_string()),
            has_more: page_values.len() == values_per_call,
            values: page_values,
            snapshot: snapshot.unwrap_or(self.timestamp - 1),
        })
    }

    async fn document_deltas(
        &self,
        cursor: DocumentDeltasCursor,
        selection: Selection,
    ) -> anyhow::Result<DocumentDeltasResponse> {
        let results_per_page = 5;

        let values: Vec<DocumentDeltasValue> = self
            .changelog
            .iter()
            .filter(|value| {
                is_table_included_in_selection(&selection, &value.component, &value.table)
                    && value.ts > cursor.0
            })
            .take(results_per_page as usize)
            .inspect(|value| {
                let column_filter = get_table_selection(&selection, &value.component, &value.table);
                if column_filter != TableSelection::included_with_all_columns() {
                    panic!("The fake doesn’t support partial column selection");
                }
            })
            .cloned()
            .collect();

        Ok(DocumentDeltasResponse {
            cursor: (values.iter().map(|x| x.ts).max().unwrap_or(cursor.0)),
            has_more: values.len() as i64 == results_per_page,
            values,
        })
    }
}

fn is_table_included_in_selection(
    selection: &Selection,
    component: &str,
    table_name: &str,
) -> bool {
    match (
        selection.components.get(component),
        selection.other_components,
    ) {
        (
            Some(ComponentSelection::Included {
                tables,
                other_tables,
            }),
            _,
        ) => {
            matches!(
                (tables.get(table_name), other_tables),
                (Some(TableSelection::Included { .. }), _) | (None, InclusionDefault::Included)
            )
        },
        (None, InclusionDefault::Included) => true,
        _ => false,
    }
}

fn get_table_selection(selection: &Selection, component: &str, table_name: &str) -> TableSelection {
    match (
        selection.components.get(component),
        selection.other_components,
    ) {
        (
            Some(ComponentSelection::Included {
                tables,
                other_tables,
            }),
            _,
        ) => match (tables.get(table_name), other_tables) {
            (Some(table_selection), _) => table_selection.clone(),
            (None, InclusionDefault::Included) => TableSelection::included_with_all_columns(),
            _ => panic!("Table excluded"),
        },
        (None, InclusionDefault::Included) => TableSelection::included_with_all_columns(),
        _ => panic!("Component excluded"),
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
        .receive(sync(
            source.clone(),
            destination.latest_state(),
            Selection::default(),
        ))
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
        .receive(sync(
            source.clone(),
            destination.latest_state(),
            Selection::default(),
        ))
        .await?;
    assert_eq!(destination.checkpointed_data.tables_by_component.len(), 0);
    let state = destination.latest_state().context("missing state")?;
    assert!(matches!(
        state.checkpoint,
        Checkpoint::DeltaUpdates {
            cursor: DocumentDeltasCursor(-1)
        }
    ));

    Ok(())
}

/// Verifies that the source and the destination are in sync by starting a new
/// initial sync and verifying that the destinations match.
async fn assert_in_sync(
    source: impl Source + 'static,
    destination: &FakeDestination,
    selection: &Selection,
) {
    let mut parallel_destination = FakeDestination::default();
    parallel_destination
        .receive(sync(
            source,
            parallel_destination.latest_state(),
            selection.clone(),
        ))
        .await
        .expect("Unexpected error during parallel synchronization");
    assert_eq!(
        destination.checkpointed_data.tables_by_component,
        parallel_destination.checkpointed_data.tables_by_component,
        "The source is not in sync with the destination (i.e. resyncing the source from scratch \
         does not give the same result). Left = contents of the destination, right = contents \
         after a full sync from scratch"
    );
}

async fn assert_not_in_sync(
    source: impl Source + 'static,
    destination: &FakeDestination,
    selection: &Selection,
) {
    let mut parallel_destination = FakeDestination::default();
    parallel_destination
        .receive(sync(
            source,
            parallel_destination.latest_state(),
            selection.clone(),
        ))
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

    assert_not_in_sync(source.clone(), &destination, &Selection::default()).await;

    destination
        .receive(sync(
            source.clone(),
            destination.latest_state(),
            Selection::default(),
        ))
        .await?;

    assert_in_sync(source, &destination, &Selection::default()).await;

    Ok(())
}

#[tokio::test]
async fn test_sync_with_multiple_pages() -> anyhow::Result<()> {
    for document_count in [29, 30, 31] {
        let mut source = FakeSource::default();

        for _ in 0..document_count {
            source.insert(ComponentPath::root(), "table", btreemap! {});
        }

        let mut destination = FakeDestination::default();

        destination
            .receive(sync(
                source.clone(),
                destination.latest_state(),
                Selection::default(),
            ))
            .await?;

        // assert the destination has 30 documents
        assert_eq!(destination.checkpointed_data.tables_by_component.len(), 1);
        assert_eq!(
            destination.checkpointed_data.tables_by_component["convex"].len(),
            1
        );
        assert_eq!(
            destination.checkpointed_data.tables_by_component["convex"]["table"].len(),
            document_count
        );
    }

    Ok(())
}

#[tokio::test]
async fn sync_after_adding_a_document() -> anyhow::Result<()> {
    let mut source = FakeSource::seeded();
    let mut destination = FakeDestination::default();

    destination
        .receive(sync(
            source.clone(),
            destination.latest_state(),
            Selection::default(),
        ))
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
    destination
        .receive(sync(source.clone(), state, Selection::default()))
        .await?;
    assert_in_sync(source, &destination, &Selection::default()).await;

    Ok(())
}

#[tokio::test]
async fn sync_after_modifying_a_document() -> anyhow::Result<()> {
    let mut source = FakeSource::seeded();
    let mut destination = FakeDestination::default();

    destination
        .receive(sync(
            source.clone(),
            destination.latest_state(),
            Selection::default(),
        ))
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
    destination
        .receive(sync(source.clone(), state, Selection::default()))
        .await?;
    assert_in_sync(source, &destination, &Selection::default()).await;

    Ok(())
}

#[tokio::test]
async fn sync_after_deleting_a_document() -> anyhow::Result<()> {
    let mut source = FakeSource::seeded();
    let mut destination = FakeDestination::default();

    destination
        .receive(sync(
            source.clone(),
            destination.latest_state(),
            Selection::default(),
        ))
        .await?;

    source.delete(ComponentPath::root(), "table1", 8);
    source.delete(ComponentPath::test_component(), "table3", 5);
    destination
        .receive(sync(
            source.clone(),
            destination.latest_state(),
            Selection::default(),
        ))
        .await?;
    assert_in_sync(source, &destination, &Selection::default()).await;

    Ok(())
}

#[tokio::test]
async fn resync_after_sync_and_delete() -> anyhow::Result<()> {
    let mut source = FakeSource::seeded();
    let mut destination = FakeDestination::default();

    destination
        .receive(sync(source.clone(), None, Selection::default()))
        .await?;
    source.delete(ComponentPath::root(), "table1", 8);
    source.delete(ComponentPath::test_component(), "table3", 5);
    // The sync + delete + resync tests to ensure that the connector
    // correctly truncates the destination before a resync.
    destination
        .receive(sync(source.clone(), None, Selection::default()))
        .await?;
    assert_in_sync(source, &destination, &Selection::default()).await;

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
        selection: Selection,
    ) -> anyhow::Result<ListSnapshotResponse> {
        self.maybe_fail()?;
        self.source.list_snapshot(snapshot, cursor, selection).await
    }

    async fn document_deltas(
        &self,
        cursor: DocumentDeltasCursor,
        selection: Selection,
    ) -> anyhow::Result<DocumentDeltasResponse> {
        self.maybe_fail()?;
        self.source.document_deltas(cursor, selection).await
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
            Selection::default(),
        ))
        .await
        .is_err()
    {}

    assert_in_sync(source, &destination, &Selection::default()).await;

    Ok(())
}

#[tokio::test]
async fn sync_with_partial_table_selection() -> anyhow::Result<()> {
    let mut source = FakeSource::default();

    // Insert data into two tables
    for i in 0..10 {
        source.insert(
            ComponentPath::root(),
            "included_table",
            btreemap! {
                "name".to_string() => json!(format!("Document {} of included_table", i)),
                "index".to_string() => json!(i),
            },
        );
        source.insert(
            ComponentPath::root(),
            "excluded_table",
            btreemap! {
                "name".to_string() => json!(format!("Document {} of excluded_table", i)),
                "index".to_string() => json!(i),
            },
        );
    }

    let mut destination = FakeDestination::default();

    // Create a selection that only includes "included_table"
    let selection = Selection {
        components: btreemap! {
            "".to_string() => ComponentSelection::Included {
                tables: btreemap! {
                    "included_table".to_string() => TableSelection::included_with_all_columns(),
                },
                other_tables: InclusionDefault::Excluded,
            },
        },
        other_components: InclusionDefault::Excluded,
    };

    // Perform the sync with the selection
    destination
        .receive(sync(
            source.clone(),
            destination.latest_state(),
            selection.clone(),
        ))
        .await?;

    // Verify that only the included table was synced
    let destination_data = destination
        .checkpointed_data
        .tables_by_component
        .get("convex") // root component’s name in Fivetran
        .expect("No data for the root component?");
    assert!(destination_data.contains_key("included_table"));
    assert!(!destination_data.contains_key("excluded_table"));
    assert_eq!(destination_data.get("included_table").unwrap().len(), 10);

    // Verify that a parallel sync with the same selection produces the same results
    assert_in_sync(source, &destination, &selection).await;

    Ok(())
}

#[tokio::test]
async fn sync_with_partial_component_selection() -> anyhow::Result<()> {
    let mut source = FakeSource::default();

    // Insert data into two tables
    for _ in 0..10 {
        source.insert(ComponentPath::root(), "table_root", btreemap! {});
        source.insert(
            ComponentPath::test_component(),
            "table_component",
            btreemap! {},
        );
    }

    let mut destination = FakeDestination::default();

    // Create a selection that only includes "included_table"
    let selection = Selection {
        components: btreemap! {
            "".to_string() => ComponentSelection::Excluded,
        },
        other_components: InclusionDefault::Included,
    };

    // Perform the sync with the selection
    destination
        .receive(sync(
            source.clone(),
            destination.latest_state(),
            selection.clone(),
        ))
        .await?;

    // Verify that only the included table was synced
    assert!(
        !destination
            .checkpointed_data
            .tables_by_component
            .contains_key("convex") // root component’s name in Fivetran
    );
    let destination_data = destination
        .checkpointed_data
        .tables_by_component
        .get("waitlist")
        .expect("No data for the waitlist component?");
    assert!(destination_data.contains_key("table_component"));
    assert!(!destination_data.contains_key("table_root"));
    assert_eq!(destination_data.get("table_component").unwrap().len(), 10);

    // Verify that a parallel sync with the same selection produces the same results
    assert_in_sync(source, &destination, &selection).await;

    Ok(())
}

// The state that the connector keeps does not keep track of the previous
// selection setting, so resyncing with different selection settings
// will not “fix” the data for the time
mod selection_changes_tests {
    use super::*;

    #[tokio::test]
    async fn resyncing_with_a_broader_selection_does_not_sync_the_old_data() -> anyhow::Result<()> {
        let mut source = FakeSource::default();

        // Insert data into two tables
        for _ in 0..5 {
            source.insert(ComponentPath::root(), "table1", btreemap! {});
            source.insert(ComponentPath::root(), "table2", btreemap! {});
        }

        let mut destination = FakeDestination::default();

        // First sync with selection that includes one table only
        let initial_selection = Selection {
            components: btreemap! {
                "".to_string() => ComponentSelection::Included {
                    tables: btreemap! {
                        "table1".to_string() => TableSelection::included_with_all_columns(),
                    },
                    other_tables: InclusionDefault::Excluded,
                },
            },
            other_components: InclusionDefault::Excluded,
        };

        destination
            .receive(sync(
                source.clone(),
                destination.latest_state(),
                initial_selection,
            ))
            .await?;

        let initial_root_component_data = destination
            .checkpointed_data
            .tables_by_component
            .get("convex")
            .unwrap();
        assert!(initial_root_component_data.contains_key("table1"));
        assert!(!initial_root_component_data.contains_key("table2"));

        // Now resync with a selection that includes everything
        let broader_selection = Selection::default();
        destination
            .receive(sync(
                source.clone(),
                destination.latest_state(),
                broader_selection,
            ))
            .await?;

        // However, the destination still only has the data from table1
        let after_root_component_data = destination
            .checkpointed_data
            .tables_by_component
            .get("convex")
            .unwrap();
        assert!(after_root_component_data.contains_key("table1"));
        assert!(!after_root_component_data.contains_key("table2"));

        Ok(())
    }

    #[tokio::test]
    async fn resyncing_with_a_narrower_selection_does_not_erase_the_old_data() -> anyhow::Result<()>
    {
        let mut source = FakeSource::default();

        // Insert data into two tables
        for _ in 0..5 {
            source.insert(ComponentPath::root(), "table1", btreemap! {});
            source.insert(ComponentPath::root(), "table2", btreemap! {});
        }

        let mut destination = FakeDestination::default();

        // First sync with selection that includes everything
        let initial_selection = Selection::default();

        destination
            .receive(sync(
                source.clone(),
                destination.latest_state(),
                initial_selection,
            ))
            .await?;

        let initial_root_component_data = destination
            .checkpointed_data
            .tables_by_component
            .get("convex")
            .unwrap();
        assert!(initial_root_component_data.contains_key("table1"));
        assert!(initial_root_component_data.contains_key("table2"));

        // Now resync with a selection that only includes the first table
        let broader_selection = Selection {
            components: btreemap! {
                "".to_string() => ComponentSelection::Included {
                    tables: btreemap! {
                        "table1".to_string() => TableSelection::included_with_all_columns(),
                    },
                    other_tables: InclusionDefault::Excluded,
                },
            },
            other_components: InclusionDefault::Excluded,
        };
        destination
            .receive(sync(
                source.clone(),
                destination.latest_state(),
                broader_selection,
            ))
            .await?;

        // However, the destination still has the data from both tables
        let after_root_component_data = destination
            .checkpointed_data
            .tables_by_component
            .get("convex")
            .unwrap();
        assert!(after_root_component_data.contains_key("table1"));
        assert!(after_root_component_data.contains_key("table2"));

        Ok(())
    }
}

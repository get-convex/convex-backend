use std::collections::BTreeSet;

use common::{
    components::ComponentPath,
    knobs::TRANSACTION_MAX_NUM_USER_WRITES,
    query::{
        Order,
        Query,
    },
};
use database::{
    ResolvedQuery,
    TableModel,
};
use keybroker::Identity;
use maplit::btreemap;
use runtime::testing::TestRuntime;
use value::{
    assert_obj,
    TableName,
    TableNamespace,
};

use crate::{
    airbyte_import::{
        AirbyteRecord,
        PrimaryKey,
        ValidatedAirbyteStream,
    },
    test_helpers::ApplicationTestExt,
    Application,
};
#[convex_macro::test_runtime]
async fn test_clear_tables_and_import(rt: TestRuntime) -> anyhow::Result<()> {
    let application = Application::new_for_tests(&rt).await?;
    let table: TableName = "table".parse()?;

    // Insert a bunch of empty messages over multiple transactions via streaming
    // import
    let mut records = vec![];
    let rows_to_insert = *TRANSACTION_MAX_NUM_USER_WRITES + 5;
    for _ in 0..rows_to_insert {
        records.push(AirbyteRecord::new(table.clone(), false, assert_obj!()));
    }
    let streams = btreemap! { table.clone() =>
        ValidatedAirbyteStream::Append
    };
    let rows_inserted = application
        .import_airbyte_records(&Identity::system(), records, streams)
        .await?;
    assert_eq!(rows_inserted, rows_to_insert as u64);

    // clear_tables should deleted the same number of documents as import inserted
    let deleted_docs = application
        .clear_tables(
            &Identity::system(),
            vec![(ComponentPath::root(), table.clone())],
        )
        .await?;
    assert_eq!(deleted_docs, rows_inserted);
    // Check the table is empty after clearing it
    let mut tx = application.begin(Identity::system()).await?;
    assert!(
        TableModel::new(&mut tx)
            .table_is_empty(TableNamespace::test_user(), &table)
            .await?
    );
    assert!(TableModel::new(&mut tx).table_exists(TableNamespace::test_user(), &table));

    Ok(())
}

#[convex_macro::test_runtime]
async fn test_delete_tables_and_import(rt: TestRuntime) -> anyhow::Result<()> {
    let application = Application::new_for_tests(&rt).await?;
    let table: TableName = "table".parse()?;

    // Insert a bunch of empty messages over multiple transactions via streaming
    // import
    let mut records = vec![];
    let rows_to_insert = *TRANSACTION_MAX_NUM_USER_WRITES + 5;
    for _ in 0..rows_to_insert {
        records.push(AirbyteRecord::new(table.clone(), false, assert_obj!()));
    }
    let streams = btreemap! { table.clone() =>
        ValidatedAirbyteStream::Append
    };
    let rows_inserted = application
        .import_airbyte_records(&Identity::system(), records, streams)
        .await?;
    assert_eq!(rows_inserted, rows_to_insert as u64);

    // delete_tables should deleted the same number of documents as streaming import
    // inserted
    let table_namespace = TableNamespace::test_user();
    let deleted_docs = application
        .delete_tables(&Identity::system(), vec![table.clone()], table_namespace)
        .await?;
    assert_eq!(deleted_docs, rows_inserted);
    // Check the table is empty after clearing it
    let mut tx = application.begin(Identity::system()).await?;
    assert!(
        TableModel::new(&mut tx)
            .table_is_empty(table_namespace, &table)
            .await?
    );
    assert!(!TableModel::new(&mut tx).table_exists(table_namespace, &table));

    Ok(())
}

#[convex_macro::test_runtime]
async fn test_dedup_import(rt: TestRuntime) -> anyhow::Result<()> {
    let application = Application::new_for_tests(&rt).await?;
    let table: TableName = "table".parse()?;
    let primary_key = PrimaryKey::try_from(vec![vec!["primary_key".to_string()]])?;
    let indexes = btreemap! {table.clone() => primary_key.clone()};
    application
        .add_primary_key_indexes(&Identity::system(), indexes)
        .await?;
    // Make sure the indexes are enabled. Backfill would happen asynchronously, but
    // because the table is empty, it should be done here.
    application
        .wait_for_primary_key_indexes_ready(Identity::system(), BTreeSet::from([table.clone()]))
        .await?;
    let objects = vec![
        // Updated value for key1
        (
            assert_obj!("field1" => "value1", "primary_key" => "key1"),
            false,
        ),
        (
            assert_obj!("field1" => "value2", "primary_key" => "key1"),
            false,
        ),
        // No changes to key2
        (
            assert_obj!("field1" => "value1", "primary_key" => "key2"),
            false,
        ),
        // Deleted value for key3
        (
            assert_obj!("field1" => "value1", "primary_key" => "key3"),
            false,
        ),
        (assert_obj!("primary_key" => "key3"), true),
    ];
    let records = objects
        .iter()
        .cloned()
        .map(|(obj, deleted)| AirbyteRecord::new(table.clone(), deleted, obj))
        .collect();
    let streams = btreemap! { table.clone() =>
        ValidatedAirbyteStream::Dedup(primary_key)
    };
    let count = application
        .import_airbyte_records(&Identity::system(), records, streams)
        .await?;
    assert_eq!(count, objects.len() as u64);
    let mut tx = application.begin(Identity::system()).await?;
    let mut query_stream = ResolvedQuery::new(
        &mut tx,
        TableNamespace::test_user(),
        Query::full_table_scan(table, Order::Asc),
    )?;
    let mut objects_in_table = vec![];
    while let Some(doc) = query_stream.next(&mut tx, Some(3)).await? {
        objects_in_table.push(doc.into_value());
    }
    let objects = &objects[1..objects.len() - 2];
    assert_eq!(objects.len(), objects_in_table.len());
    for (i, (obj, _)) in objects.iter().enumerate() {
        let obj_in_table = &*objects_in_table[i];
        for field in ["field1", "primary_key"] {
            assert_eq!(obj.get(field), obj_in_table.get(field))
        }
    }
    Ok(())
}

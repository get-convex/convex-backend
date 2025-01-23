use std::{
    collections::{
        BTreeMap,
        BTreeSet,
        HashSet,
    },
    ops::RangeBounds,
    sync::Arc,
    time::Duration,
};

use ::usage_tracking::FunctionUsageTracker;
use cmd_util::env::env_config;
use common::{
    assert_obj,
    bootstrap_model::index::{
        database_index::{
            DeveloperDatabaseIndexConfig,
            IndexedFields,
        },
        IndexConfig,
        IndexMetadata,
    },
    db_schema,
    document::{
        CreationTime,
        PackedDocument,
        ResolvedDocument,
    },
    maybe_val,
    object_validator,
    persistence::{
        NoopRetentionValidator,
        Persistence,
    },
    query::{
        Expression,
        FullTableScan,
        IndexRange,
        IndexRangeExpression,
        Order,
        Query,
        QueryOperator,
        QuerySource,
    },
    runtime::Runtime,
    schemas::{
        validator::{
            FieldValidator,
            Validator,
        },
        DatabaseSchema,
        DocumentSchema,
        IndexSchema,
        TableDefinition,
        MAX_INDEXES_PER_TABLE,
    },
    types::{
        unchecked_repeatable_ts,
        IndexDescriptor,
        IndexName,
        PersistenceVersion,
        RepeatableTimestamp,
        TableName,
        WriteTimestamp,
    },
    value::{
        ConvexObject,
        ConvexValue,
    },
    virtual_system_mapping::{
        all_tables_name_to_number,
        all_tables_number_to_name,
        NoopDocMapper,
        VirtualSystemMapping,
    },
};
use errors::{
    ErrorMetadata,
    ErrorMetadataAnyhowExt,
};
use imbl::OrdSet;
use keybroker::Identity;
use must_let::must_let;
use pretty_assertions::assert_eq;
use proptest::prelude::*;
use runtime::testing::TestRuntime;
use sync_types::backoff::Backoff;
use value::{
    array,
    assert_val,
    id_v6::DeveloperDocumentId,
    val,
    FieldPath,
    ResolvedDocumentId,
    TableMapping,
    TableNamespace,
    TabletIdAndTableNumber,
};

use crate::{
    index_worker::{
        IndexSelector,
        IndexWriter,
    },
    query::{
        PaginationOptions,
        ResolvedQuery,
        TableFilter,
    },
    table_summary::{
        write_snapshot,
        TableSummary,
        TableSummaryWriter,
    },
    test_helpers::{
        new_test_database,
        DbFixtures,
        DbFixturesArgs,
    },
    write_log::WriteSource,
    Database,
    DatabaseSnapshot,
    ImportFacingModel,
    IndexModel,
    IndexWorker,
    SchemaModel,
    SystemMetadataModel,
    TableModel,
    TestFacingModel,
    Transaction,
    UserFacingModel,
};

mod randomized_search_tests;
mod streaming_export_tests;
mod usage_tracking;
mod vector_tests;

mod apply_function_runner_tx;
pub mod text_test_utils;
pub mod vector_test_utils;

const TEST_PREFETCH_HINT: usize = 2;

#[convex_macro::test_runtime]
async fn test_load_database(rt: TestRuntime) -> anyhow::Result<()> {
    // load once to initialize
    let DbFixtures { db, tp, .. } = DbFixtures::new(&rt).await?;
    let mut tx = db.begin(Identity::system()).await?;
    TestFacingModel::new(&mut tx)
        .insert(&"table1".parse()?, assert_obj!())
        .await?;
    TestFacingModel::new(&mut tx)
        .insert(&"table2".parse()?, assert_obj!())
        .await?;
    db.commit(tx).await?;

    // Load again with data to make sure it doesn't crash
    let _database = DbFixtures::new_with_args(
        &rt,
        DbFixturesArgs {
            tp: Some(tp),
            ..Default::default()
        },
    )
    .await?;
    Ok(())
}

#[convex_macro::test_runtime]
async fn test_load_from_table_summary_snapshot(rt: TestRuntime) -> anyhow::Result<()> {
    let DbFixtures { db, tp, .. } = DbFixtures::new(&rt).await?;
    let writer = TableSummaryWriter::new(
        rt.clone(),
        tp.clone(),
        db.clone(),
        Arc::new(NoopRetentionValidator),
    );
    let namespace = TableNamespace::test_user();

    // Populate shapes by writing objects.
    let mut tx = db.begin(Identity::system()).await?;
    let table1: TableName = "table1".parse()?;
    let value1 = assert_obj!("key1" => 0);
    let mut summary1 = TableSummary::empty();
    let value1_doc = TestFacingModel::new(&mut tx)
        .insert_and_get(table1.clone(), value1)
        .await?;
    let value1_id = value1_doc.id();
    summary1 = summary1.insert(value1_doc.value());
    db.commit(tx).await?;

    let snapshot = db.latest_snapshot()?;
    assert_eq!(
        snapshot.table_summary(namespace, &table1),
        Some(summary1.clone())
    );

    let snapshot = writer.compute_from_last_checkpoint().await?;
    write_snapshot(tp.as_ref(), &snapshot).await?;

    // Overwrite document after snapshot.
    let mut tx = db.begin(Identity::system()).await?;
    summary1 = summary1.remove(&value1_doc.into_value())?;
    let value1 = assert_obj!("key1" => null);
    let value1_doc = UserFacingModel::new_root_for_test(&mut tx)
        .replace(value1_id.into(), value1)
        .await?;
    summary1 = summary1.insert(&value1_doc.into_value());
    // Update summary after snapshot.
    let value2 = assert_obj!("key2" => 1.0);
    let value2_with_id = TestFacingModel::new(&mut tx)
        .insert_and_get(table1.clone(), value2)
        .await?
        .into_value()
        .0;
    summary1 = summary1.insert(&value2_with_id);

    // Create new table after snapshot.
    let table2: TableName = "table2".parse()?;
    let value3 = assert_obj!("key3" => null);
    let value3_with_id = TestFacingModel::new(&mut tx)
        .insert_and_get(table2.clone(), value3)
        .await?
        .into_value();
    let summary2 = TableSummary::empty().insert(&value3_with_id);
    db.commit(tx).await?;

    // Load again with data to make sure it has the correct summaries.
    let DbFixtures { db, .. } = DbFixtures::new_with_args(
        &rt,
        DbFixturesArgs {
            tp: Some(tp),
            ..Default::default()
        },
    )
    .await?;
    let snapshot = db.latest_snapshot()?;
    assert_eq!(snapshot.table_summary(namespace, &table1), Some(summary1));
    assert_eq!(snapshot.table_summary(namespace, &table2), Some(summary2));
    Ok(())
}

#[convex_macro::test_runtime]
async fn test_build_indexes(rt: TestRuntime) -> anyhow::Result<()> {
    let database = new_test_database(rt).await;

    let table_name: TableName = str::parse("table")?;
    let namespace = TableNamespace::test_user();

    // Register two indexes and make sure it works.
    let index_name1 = IndexName::new(table_name.clone(), IndexDescriptor::new("a_and_b")?)?;
    let index_name2 = IndexName::new(table_name.clone(), IndexDescriptor::new("c_and_d")?)?;

    let mut tx = database.begin(Identity::system()).await?;

    let mut indexes = BTreeMap::<IndexDescriptor, IndexSchema>::new();
    indexes.insert(
        index_name1.descriptor().clone(),
        IndexSchema {
            index_descriptor: index_name1.descriptor().clone(),
            fields: vec![str::parse("a")?, str::parse("b")?].try_into()?,
        },
    );
    indexes.insert(
        index_name2.descriptor().clone(),
        IndexSchema {
            index_descriptor: index_name2.descriptor().clone(),
            fields: vec![str::parse("c")?, str::parse("d")?].try_into()?,
        },
    );

    let mut tables = BTreeMap::<TableName, TableDefinition>::new();
    tables.insert(
        table_name.clone(),
        TableDefinition {
            table_name: table_name.clone(),
            indexes,
            search_indexes: BTreeMap::new(),
            vector_indexes: BTreeMap::new(),
            document_type: None,
        },
    );
    let schema = DatabaseSchema {
        tables,
        schema_validation: true,
    };

    let changes = IndexModel::new(&mut tx)
        .build_indexes(TableNamespace::test_user(), &schema)
        .await?;
    assert_eq!(changes.added.len(), 2);
    assert_eq!(changes.added[0].name.to_string(), "table.a_and_b");
    assert_eq!(changes.added[1].name.to_string(), "table.c_and_d");
    assert_eq!(changes.dropped.len(), 0);

    database.commit(tx).await?;

    let mut tx = database.begin(Identity::system()).await?;
    assert_eq!(
        get_pending_index_fields(&mut tx, namespace, &index_name1)?,
        vec![str::parse("a")?, str::parse("b")?].try_into()?,
    );
    assert_eq!(
        get_pending_index_fields(&mut tx, namespace, &index_name2)?,
        vec![str::parse("c")?, str::parse("d")?].try_into()?,
    );

    // Add one index, overwrite one, drop the other.
    let index_name3 = IndexName::new(table_name.clone(), IndexDescriptor::new("e_and_f")?)?;

    let mut indexes = BTreeMap::<IndexDescriptor, IndexSchema>::new();
    indexes.remove(index_name1.descriptor());
    indexes.insert(
        index_name2.descriptor().clone(),
        IndexSchema {
            index_descriptor: index_name2.descriptor().clone(),
            fields: vec![str::parse("c")?].try_into()?,
        },
    );
    indexes.insert(
        index_name3.descriptor().clone(),
        IndexSchema {
            index_descriptor: index_name3.descriptor().clone(),
            fields: vec![str::parse("e")?, str::parse("f")?].try_into()?,
        },
    );

    let mut tables = BTreeMap::<TableName, TableDefinition>::new();
    tables.insert(
        table_name.clone(),
        TableDefinition {
            table_name,
            indexes,
            search_indexes: BTreeMap::new(),
            vector_indexes: BTreeMap::new(),
            document_type: None,
        },
    );
    let schema = DatabaseSchema {
        tables,
        schema_validation: true,
    };

    let changes = IndexModel::new(&mut tx)
        .build_indexes(TableNamespace::test_user(), &schema)
        .await?;
    assert_eq!(
        changes
            .added
            .iter()
            .map(|it| it.name.to_string())
            .collect::<Vec<String>>()
            .sort(),
        vec!["table.c_and_d", "table.e_and_f"].sort()
    );
    assert_eq!(
        changes
            .dropped
            .iter()
            .map(|it| it.name.to_string())
            .collect::<Vec<String>>()
            .sort(),
        vec!["table.c_and_d", "table.a_and_b"].sort(),
    );

    database.commit(tx).await?;

    let mut tx = database.begin(Identity::system()).await?;
    assert!(IndexModel::new(&mut tx)
        .pending_index_metadata(namespace, &index_name1)?
        .is_none());
    assert_eq!(
        get_pending_index_fields(&mut tx, namespace, &index_name2)?,
        vec![str::parse("c")?].try_into()?
    );
    assert_eq!(
        get_pending_index_fields(&mut tx, namespace, &index_name3)?,
        vec![str::parse("e")?, str::parse("f")?].try_into()?
    );
    Ok(())
}

fn get_pending_index_fields(
    tx: &mut Transaction<TestRuntime>,
    namespace: TableNamespace,
    index_name: &IndexName,
) -> anyhow::Result<IndexedFields> {
    let index_c_d = IndexModel::new(tx)
        .pending_index_metadata(namespace, index_name)?
        .expect("index should exist");
    must_let!(let IndexConfig::Database { developer_config, .. } = &index_c_d.config);
    must_let!(let DeveloperDatabaseIndexConfig { fields } = developer_config);
    Ok(fields.clone())
}

#[convex_macro::test_runtime]
async fn test_delete_conflict(rt: TestRuntime) -> anyhow::Result<()> {
    let database = new_test_database(rt).await;
    let mut tx = database.begin(Identity::system()).await?;
    let id = TestFacingModel::new(&mut tx)
        .insert(&"key".parse()?, ConvexObject::empty())
        .await?;
    database.commit(tx).await?;

    let mut tx1 = database.begin(Identity::system()).await?;
    assert!(tx1.get(id).await?.is_some());
    TestFacingModel::new(&mut tx1)
        .insert(&"key2".parse()?, ConvexObject::empty())
        .await?;

    let mut tx2 = database.begin(Identity::system()).await?;
    UserFacingModel::new_root_for_test(&mut tx2)
        .delete(id.into())
        .await?;
    database
        .commit_with_write_source(tx2, "foo/bar:baz")
        .await?;

    must_let!(let Err(e) = database.commit(tx1).await);
    assert!(e.is_occ());
    assert!(
        format!("{}", e).contains(
            "Documents read from or written to the \"key\" table changed while this mutation"
        ),
        "Got:\n\n{e}"
    );
    assert!(
        format!("{}", e).contains(&format!(
            "A call to \"foo/bar:baz\" changed the document with ID \"{id}\"",
        )),
        "Got:\n\n{e}"
    );

    Ok(())
}

#[convex_macro::test_runtime]
async fn test_creation_time_success(rt: TestRuntime) -> anyhow::Result<()> {
    let database = new_test_database(rt.clone()).await;
    let mut tx = database.begin(Identity::system()).await?;
    TestFacingModel::new(&mut tx)
        .insert(&"table".parse()?, ConvexObject::empty())
        .await?;
    database.commit(tx).await?;

    let mut tx1 = database.begin(Identity::system()).await?;
    rt.advance_time(Duration::from_secs(1)).await;
    let mut tx2 = database.begin(Identity::system()).await?;

    assert!(tx1.next_creation_time < tx2.next_creation_time);

    TestFacingModel::new(&mut tx1)
        .insert(&"table".parse()?, ConvexObject::empty())
        .await?;
    TestFacingModel::new(&mut tx2)
        .insert(&"table".parse()?, ConvexObject::empty())
        .await?;

    database.commit(tx1).await?;
    database.commit(tx2).await?;

    Ok(())
}

#[convex_macro::test_runtime]
async fn test_id_reuse_across_transactions(rt: TestRuntime) -> anyhow::Result<()> {
    let database = new_test_database(rt).await;
    let mut tx = database.begin(Identity::system()).await?;
    let id = UserFacingModel::new_root_for_test(&mut tx)
        .insert("table".parse()?, assert_obj!())
        .await?;
    let id_ = id.to_resolved(
        tx.table_mapping()
            .namespace(TableNamespace::test_user())
            .number_to_tablet(),
    )?;
    let document = tx.get(id_).await?.unwrap();
    database.commit(tx).await?;

    let mut tx = database.begin(Identity::system()).await?;
    // Pretend we create another document with the same ID as the first. We can't do
    // this through the normal Transaction interface so we pretend it's an import.
    let id_v6 = DeveloperDocumentId::from(document.id()).encode();
    let table_mapping_for_schema = tx.table_mapping().clone();
    ImportFacingModel::new(&mut tx)
        .insert(
            TabletIdAndTableNumber {
                tablet_id: document.id().tablet_id,
                table_number: document.id().developer_id.table(),
            },
            &"table".parse()?,
            assert_obj!("_id" => id_v6),
            &table_mapping_for_schema,
        )
        .await?;

    // Committing this transaction show throw an exception.
    let err = database.commit(tx).await.unwrap_err();
    assert!(err.is_bad_request());

    Ok(())
}

#[convex_macro::test_runtime]
async fn test_id_reuse_within_a_transactions(rt: TestRuntime) -> anyhow::Result<()> {
    let database = new_test_database(rt).await;
    let mut tx = database.begin(Identity::system()).await?;
    let document_id = TestFacingModel::new(&mut tx)
        .insert(&"table".parse()?, ConvexObject::empty())
        .await?;
    let table_mapping = tx.table_mapping().clone();
    let table_id = table_mapping
        .namespace(TableNamespace::test_user())
        .name_to_id()("table".parse()?)?;

    // Do another insert using the same DocumentId.
    let value = assert_obj!(
        "_id" => DeveloperDocumentId::from(document_id).encode(),
    );
    let err = ImportFacingModel::new(&mut tx)
        .insert(table_id, &"table".parse()?, value, &table_mapping)
        .await
        .unwrap_err();
    assert!(format!("{err}").contains("Duplicate insert"), "{err}");
    Ok(())
}

async fn run_query(
    database: Database<TestRuntime>,
    namespace: TableNamespace,
    query: Query,
) -> anyhow::Result<Vec<ResolvedDocument>> {
    let mut tx = database.begin(Identity::system()).await?;
    let mut query_stream = ResolvedQuery::new(&mut tx, namespace, query)?;
    let mut results = vec![];
    while let Some(value) = query_stream.next(&mut tx, Some(TEST_PREFETCH_HINT)).await? {
        results.push(value);
    }
    Ok(results)
}

#[convex_macro::test_runtime]
async fn test_query_filter(rt: TestRuntime) -> anyhow::Result<()> {
    let database = new_test_database(rt).await;
    let namespace = TableNamespace::test_user();
    let mut tx = database.begin(Identity::system()).await?;
    let doc1 = TestFacingModel::new(&mut tx)
        .insert_and_get(
            "messages".parse()?,
            assert_obj!(
                "channel" => "eng",
                "text" => "hello",
            ),
        )
        .await?;
    TestFacingModel::new(&mut tx)
        .insert(
            &"messages".parse()?,
            assert_obj!(
                "channel" => "general",
                "text" => "@here",
            ),
        )
        .await?;
    let doc3 = TestFacingModel::new(&mut tx)
        .insert_and_get(
            "messages".parse()?,
            assert_obj!(
                "channel" => "eng",
                "text" => "world",
            ),
        )
        .await?;
    database.commit(tx).await?;

    let query = Query {
        source: QuerySource::FullTableScan(FullTableScan {
            table_name: "messages".parse()?,
            order: Order::Asc,
        }),
        operators: vec![QueryOperator::Filter(Expression::Eq(
            Box::new(Expression::Literal(maybe_val!("eng"))),
            Box::new(Expression::Field("channel".parse()?)),
        ))],
    };
    let results = run_query(database, namespace, query).await?;
    assert_eq!(results, vec![doc1, doc3]);
    Ok(())
}

#[convex_macro::test_runtime]
async fn test_query_limit(rt: TestRuntime) -> anyhow::Result<()> {
    let database = new_test_database(rt).await;
    let namespace = TableNamespace::test_user();
    let mut tx = database.begin(Identity::system()).await?;
    TestFacingModel::new(&mut tx)
        .insert(
            &"messages".parse()?,
            assert_obj!(
                "channel" => "eng",
                "text" => "hello",
            ),
        )
        .await?;
    TestFacingModel::new(&mut tx)
        .insert(
            &"messages".parse()?,
            assert_obj!(
                "channel" => "general",
                "text" => "@here",
            ),
        )
        .await?;
    TestFacingModel::new(&mut tx)
        .insert(
            &"messages".parse()?,
            assert_obj!(
                "channel" => "eng",
                "text" => "world",
            ),
        )
        .await?;
    database.commit(tx).await?;

    let query = Query {
        source: QuerySource::FullTableScan(FullTableScan {
            table_name: "messages".parse()?,
            order: Order::Asc,
        }),
        operators: vec![QueryOperator::Limit(1)],
    };
    let results = run_query(database, namespace, query).await?;
    assert_eq!(results.len(), 1);
    Ok(())
}

#[convex_macro::test_runtime]
async fn test_full_table_scan_order(rt: TestRuntime) -> anyhow::Result<()> {
    let database = new_test_database(rt).await;
    let namespace = TableNamespace::test_user();
    let mut tx = database.begin(Identity::system()).await?;
    let doc1 = TestFacingModel::new(&mut tx)
        .insert_and_get("messages".parse()?, ConvexObject::empty())
        .await?;
    let doc2 = TestFacingModel::new(&mut tx)
        .insert_and_get("messages".parse()?, ConvexObject::empty())
        .await?;
    database.commit(tx).await?;

    let asc_query = Query {
        source: QuerySource::FullTableScan(FullTableScan {
            table_name: "messages".parse()?,
            order: Order::Asc,
        }),
        operators: vec![],
    };
    let asc_results = run_query(database.clone(), namespace, asc_query).await?;
    assert_eq!(asc_results, vec![doc1.clone(), doc2.clone()],);

    let desc_query = Query {
        source: QuerySource::FullTableScan(FullTableScan {
            table_name: "messages".parse()?,
            order: Order::Desc,
        }),
        operators: vec![],
    };
    let desc_results = run_query(database, namespace, desc_query).await?;
    assert_eq!(desc_results, vec![doc2, doc1],);

    Ok(())
}

/// Insert enough documents to take up more than one page, to make sure
/// we can cursor between pages effectively.
async fn insert_documents(
    tx: &mut Transaction<TestRuntime>,
    table_name: TableName,
) -> anyhow::Result<Vec<ResolvedDocument>> {
    let mut values = Vec::new();
    for a in 0..10 {
        for b in 0..10 * TEST_PREFETCH_HINT {
            let doc = TestFacingModel::new(tx)
                .insert_and_get(
                    table_name.clone(),
                    assert_obj!(
                        "a" => a,
                        "b" => b as i64,
                    ),
                )
                .await?;
            values.push(doc);
        }
    }
    Ok(values)
}

// Assert that for a set of records inserted with (a, b) where a in [0, 10) and
// b in [0, TEST_PREFETCH_HINT), reading the index range `range` in `order`
// produces the values matched by `predicate(a, b)` in the proper order.
async fn test_query_index_range<F>(
    rt: TestRuntime,
    range: Vec<IndexRangeExpression>,
    order: Order,
    predicate: F,
) -> anyhow::Result<()>
where
    F: Fn(i64, i64) -> bool,
{
    let DbFixtures {
        db: database, tp, ..
    } = DbFixtures::new(&rt).await?;
    let table_name: TableName = str::parse("messages")?;
    let namespace = TableNamespace::test_user();
    let index_name = IndexName::new(table_name.clone(), IndexDescriptor::new("a_and_b")?)?;

    let mut tx = database.begin(Identity::system()).await?;
    let begin_ts = tx.begin_timestamp();
    IndexModel::new(&mut tx)
        .add_application_index(
            namespace,
            IndexMetadata::new_backfilling(
                *begin_ts,
                index_name.clone(),
                vec![str::parse("a")?, str::parse("b")?].try_into()?,
            ),
        )
        .await?;
    database.commit(tx).await?;

    let mut tx = database.begin(Identity::system()).await?;
    let values = insert_documents(&mut tx, table_name).await?;
    database.commit(tx).await?;

    let retention_validator = Arc::new(NoopRetentionValidator);

    // Backfill the index.
    let index_backfill_fut =
        IndexWorker::new_terminating(rt, tp, retention_validator, database.clone());
    index_backfill_fut.await?;

    let mut tx = database.begin_system().await?;
    IndexModel::new(&mut tx)
        .enable_index_for_testing(namespace, &index_name)
        .await?;
    database.commit(tx).await?;

    let mut expected = values
        .iter()
        .filter(|x| {
            must_let!(let ConvexValue::Int64(a) = x.value().get("a").unwrap());
            must_let!(let ConvexValue::Int64(b) = x.value().get("b").unwrap());
            predicate(*a, *b)
        })
        .cloned()
        .collect::<Vec<ResolvedDocument>>();
    if order == Order::Desc {
        expected.reverse();
    }

    let query = Query {
        source: QuerySource::IndexRange(IndexRange {
            index_name,
            range,
            order,
        }),
        operators: vec![],
    };
    let actual = run_query(database, namespace, query).await?;
    assert_eq!(actual, expected);

    Ok(())
}

#[convex_macro::test_runtime]
async fn test_query_index_range_single_page_asc(rt: TestRuntime) -> anyhow::Result<()> {
    test_query_index_range(
        rt,
        vec![
            IndexRangeExpression::Eq("a".parse()?, maybe_val!(3)),
            IndexRangeExpression::Gte("b".parse()?, val!(2)),
            IndexRangeExpression::Lte("b".parse()?, val!(3)),
        ],
        Order::Asc,
        |a, b| a == 3 && (2..=3).contains(&b),
    )
    .await
}
#[convex_macro::test_runtime]
async fn test_query_index_range_single_page_desc(rt: TestRuntime) -> anyhow::Result<()> {
    test_query_index_range(
        rt,
        vec![
            IndexRangeExpression::Eq("a".parse()?, maybe_val!(3)),
            IndexRangeExpression::Gte("b".parse()?, val!(8)),
            IndexRangeExpression::Lte("b".parse()?, val!(9)),
        ],
        Order::Desc,
        |a, b| a == 3 && (8..=9).contains(&b),
    )
    .await
}
#[convex_macro::test_runtime]
async fn test_query_index_range_multi_page_asc(rt: TestRuntime) -> anyhow::Result<()> {
    test_query_index_range(
        rt,
        vec![
            IndexRangeExpression::Gte("a".parse()?, val!(3)),
            IndexRangeExpression::Lte("a".parse()?, val!(7)),
        ],
        Order::Asc,
        |a, _| (3..=7).contains(&a),
    )
    .await
}
#[convex_macro::test_runtime]
async fn test_query_index_range_prefix(rt: TestRuntime) -> anyhow::Result<()> {
    test_query_index_range(
        rt,
        vec![IndexRangeExpression::Eq("a".parse()?, maybe_val!(3))],
        Order::Asc,
        |a, _| a == 3,
    )
    .await
}
#[convex_macro::test_runtime]
async fn test_query_index_range_multi_page_desc(rt: TestRuntime) -> anyhow::Result<()> {
    test_query_index_range(
        rt,
        vec![
            IndexRangeExpression::Gte("a".parse()?, val!(3)),
            IndexRangeExpression::Lte("a".parse()?, val!(7)),
        ],
        Order::Desc,
        |a, _| (3..=7).contains(&a),
    )
    .await
}
#[convex_macro::test_runtime]
async fn test_query_index_range_all_multi_page_asc(rt: TestRuntime) -> anyhow::Result<()> {
    test_query_index_range(rt, vec![], Order::Asc, |_, _| true).await
}
#[convex_macro::test_runtime]
async fn test_query_index_range_all_multi_page_desc(rt: TestRuntime) -> anyhow::Result<()> {
    test_query_index_range(rt, vec![], Order::Desc, |_, _| true).await
}
#[convex_macro::test_runtime]
async fn test_query_index_range_multi_key_multi_page_desc(rt: TestRuntime) -> anyhow::Result<()> {
    test_query_index_range(
        rt,
        vec![
            IndexRangeExpression::Eq("a".parse()?, maybe_val!(3)),
            IndexRangeExpression::Gte("b".parse()?, val!(2)),
            IndexRangeExpression::Lte("b".parse()?, val!(9)),
        ],
        Order::Desc,
        |a, b| a == 3 && (2..=9).contains(&b),
    )
    .await
}
#[convex_macro::test_runtime]
async fn test_query_index_range_half_bounded(rt: TestRuntime) -> anyhow::Result<()> {
    test_query_index_range(
        rt,
        vec![
            IndexRangeExpression::Eq("a".parse()?, maybe_val!(3)),
            IndexRangeExpression::Gte("b".parse()?, val!(4)),
        ],
        Order::Asc,
        |a, b| a == 3 && b >= 4,
    )
    .await
}

proptest! {
    #![proptest_config(
            ProptestConfig { cases: 256 * env_config("CONVEX_PROPTEST_MULTIPLIER", 1), failure_persistence: None, ..ProptestConfig::default() }
        )]

    #[test]
    fn proptest_ord_set_range(
        vs in any::<HashSet<u32>>(),
        start in any::<std::ops::Bound<u32>>(),
        end in any::<std::ops::Bound<u32>>(),
    ) {
        let mut expected: Vec<u32> = vs
            .iter()
            .filter(|x| (start, end).contains(x))
            .copied()
            .collect();
        expected.sort_unstable();

        let m = OrdSet::from_iter(vs.iter().copied());
        let actual: Vec<u32> = m.range((start, end)).copied().collect();

        assert_eq!(actual, expected);
    }
}

#[convex_macro::test_runtime]
async fn test_query_cursor_reuse(rt: TestRuntime) -> anyhow::Result<()> {
    let database = new_test_database(rt).await;
    let namespace = TableNamespace::test_user();
    let mut tx = database.begin(Identity::system()).await?;

    // Create 2 different queries.
    let query1 = Query::full_table_scan("table1".parse()?, Order::Asc);
    let query2 = Query::full_table_scan("table2".parse()?, Order::Asc);

    // Get a cursor from query 1.
    let mut compiled_query1 = ResolvedQuery::new_bounded(
        &mut tx,
        namespace,
        query1.clone(),
        PaginationOptions::ManualPagination {
            start_cursor: None,
            maximum_rows_read: None,
            maximum_bytes_read: None,
        },
        None,
        TableFilter::ExcludePrivateSystemTables,
    )?;
    compiled_query1.next(&mut tx, None).await?;
    let cursor1 = compiled_query1.cursor();

    // We can use this to continue query 1 without any errors.
    ResolvedQuery::<TestRuntime>::new_bounded(
        &mut tx,
        namespace,
        query1,
        PaginationOptions::ManualPagination {
            start_cursor: cursor1.clone(),
            maximum_rows_read: None,
            maximum_bytes_read: None,
        },
        None,
        TableFilter::IncludePrivateSystemTables,
    )?;

    // Using it on a different query generates an error.
    let err = ResolvedQuery::<TestRuntime>::new_bounded(
        &mut tx,
        namespace,
        query2,
        PaginationOptions::ManualPagination {
            start_cursor: cursor1,
            maximum_rows_read: None,
            maximum_bytes_read: None,
        },
        None,
        TableFilter::IncludePrivateSystemTables,
    )
    .err()
    .unwrap();
    assert!(err.is_bad_request());
    assert_eq!(err.short_msg(), "InvalidCursor");

    Ok(())
}

#[convex_macro::test_runtime]
async fn test_too_large_values(rt: TestRuntime) -> anyhow::Result<()> {
    let huge_obj = assert_obj!("huge" => vec![0; 1 << 22]);
    let smol_obj = assert_obj!("huge" => vec![0; 1 << 12]);

    let database = new_test_database(rt).await;
    let table_name: TableName = "table".parse()?;

    let mut tx = database.begin(Identity::system()).await?;
    let err = UserFacingModel::new_root_for_test(&mut tx)
        .insert(table_name.clone(), huge_obj.clone())
        .await
        .unwrap_err();
    assert!(format!("{err}").contains("Value is too large"));

    let doc_id = UserFacingModel::new_root_for_test(&mut tx)
        .insert(table_name, smol_obj)
        .await?;

    let err = UserFacingModel::new_root_for_test(&mut tx)
        .patch(doc_id, huge_obj.clone().into())
        .await
        .unwrap_err();
    assert!(format!("{err}").contains("Value is too large"), "{err}");

    let err = UserFacingModel::new_root_for_test(&mut tx)
        .replace(doc_id, huge_obj.clone())
        .await
        .unwrap_err();
    assert!(format!("{err}").contains("Value is too large"));

    // Check that inserting a 4MB value to a system table works.
    let table_name = "_test_table".parse()?;
    tx.create_system_table_testing(TableNamespace::Global, &table_name, None)
        .await?;
    SystemMetadataModel::new_global(&mut tx)
        .insert(&table_name, huge_obj)
        .await?;

    database.commit(tx).await?;

    Ok(())
}

#[convex_macro::test_runtime]
async fn test_too_nested_values(rt: TestRuntime) -> anyhow::Result<()> {
    let mut deeply_nested_but_still_ok = assert_val!(false);
    // 15 levels plus 1 for the document itself
    for _ in 0..15 {
        deeply_nested_but_still_ok =
            ConvexValue::Array(array![deeply_nested_but_still_ok.clone()]?);
    }
    let database = new_test_database(rt.clone()).await;
    let table_name: TableName = "table".parse()?;

    let mut tx = database.begin(Identity::system()).await?;
    UserFacingModel::new_root_for_test(&mut tx)
        .insert(
            table_name.clone(),
            assert_obj!("x" => deeply_nested_but_still_ok.clone()),
        )
        .await?;

    database.commit(tx).await?;

    let mut too_deeply_nested = assert_val!(false);
    // 16 levels plus 1 for the document itself
    for _ in 0..16 {
        too_deeply_nested = ConvexValue::Array(array![too_deeply_nested.clone()]?);
    }

    let database = new_test_database(rt.clone()).await;
    let table_name: TableName = "table".parse()?;

    let mut tx = database.begin(Identity::system()).await?;
    let err = UserFacingModel::new_root_for_test(&mut tx)
        .insert(
            table_name.clone(),
            assert_obj!("x" => too_deeply_nested.clone()),
        )
        .await
        .unwrap_err();

    assert!(format!("{err}").contains("Document is too nested"));

    database.commit(tx).await?;
    Ok(())
}

#[convex_macro::test_runtime]
async fn test_insert_new_table_for_import(rt: TestRuntime) -> anyhow::Result<()> {
    let object = assert_obj!("value" => 1);
    let object_with_creation_time = assert_obj!("value" => 2, "_creationTime" => 1699545341000.0);

    let database = new_test_database(rt).await;
    let table_name: TableName = "table".parse()?;

    let mut tx = database.begin(Identity::system()).await?;
    let mut table_model = TableModel::new(&mut tx);
    let table_id = table_model
        .insert_table_for_import(
            TableNamespace::test_user(),
            &table_name,
            None,
            &BTreeSet::new(),
        )
        .await?;
    let mut table_mapping_for_schema = tx.table_mapping().clone();
    table_mapping_for_schema.insert(
        table_id.tablet_id,
        TableNamespace::test_user(),
        table_id.table_number,
        table_name.clone(),
    );
    let doc1_id = ImportFacingModel::new(&mut tx)
        .insert(table_id, &table_name, object, &table_mapping_for_schema)
        .await?;
    let doc1_id = ResolvedDocumentId::new(
        table_id.tablet_id,
        DeveloperDocumentId::new(table_id.table_number, doc1_id.internal_id()),
    );
    let doc2_id = ImportFacingModel::new(&mut tx)
        .insert(
            table_id,
            &table_name,
            object_with_creation_time,
            &table_mapping_for_schema,
        )
        .await?;
    let doc2_id = ResolvedDocumentId::new(
        table_id.tablet_id,
        DeveloperDocumentId::new(table_id.table_number, doc2_id.internal_id()),
    );

    database.commit(tx).await?;

    let mut tx = database.begin(Identity::system()).await?;
    let doc1 = tx.get_inner(doc1_id, table_name.clone()).await?.unwrap().0;
    let doc2 = tx.get_inner(doc2_id, table_name.clone()).await?.unwrap().0;
    assert_eq!(doc1.id(), doc1_id);
    assert_eq!(doc2.id(), doc2_id);
    assert!(doc1.creation_time().is_some());
    assert_eq!(
        doc2.creation_time(),
        Some(CreationTime::try_from(1699545341000.0)?)
    );
    // The table is still in state Hidden, so it doesn't appear in the dashboard
    let snapshot = database.latest_snapshot()?;
    assert_eq!(snapshot.table_registry.user_table_names().count(), 0);

    Ok(())
}

#[convex_macro::test_runtime]
async fn test_importing_table_schema_validated(rt: TestRuntime) -> anyhow::Result<()> {
    let database = new_test_database(rt).await;
    let table_name: TableName = "table".parse()?;

    let mut tx = database.begin(Identity::system()).await?;
    let mut table_model = TableModel::new(&mut tx);
    let table_id = table_model
        .insert_table_for_import(
            TableNamespace::test_user(),
            &table_name,
            None,
            &BTreeSet::new(),
        )
        .await?;
    database.commit(tx).await?;

    let mut tx = database.begin(Identity::system()).await?;
    let mut schema_model = SchemaModel::new_root_for_test(&mut tx);
    let db_schema = db_schema!(table_name.clone() => DocumentSchema::Union(
        vec![
            object_validator!(
                "value" => FieldValidator::required_field_type(Validator::Float64),
            )
        ]
    ));
    let (schema_id, _) = schema_model.submit_pending(db_schema).await?;
    schema_model.mark_validated(schema_id).await?;
    schema_model.mark_active(schema_id).await?;
    database.commit(tx).await?;

    let mut tx = database.begin(Identity::system()).await?;
    let mut table_mapping_for_schema = tx.table_mapping().clone();
    table_mapping_for_schema.insert(
        table_id.tablet_id,
        TableNamespace::test_user(),
        table_id.table_number,
        table_name.clone(),
    );
    ImportFacingModel::new(&mut tx)
        .insert(
            table_id,
            &table_name,
            assert_obj!("value" => 1.),
            &table_mapping_for_schema,
        )
        .await?;
    let err = ImportFacingModel::new(&mut tx)
        .insert(
            table_id,
            &table_name,
            assert_obj!("value" => false),
            &table_mapping_for_schema,
        )
        .await
        .unwrap_err();
    assert!(err.is_bad_request());

    Ok(())
}

#[convex_macro::test_runtime]
async fn test_importing_foreign_reference_schema_validated(rt: TestRuntime) -> anyhow::Result<()> {
    let database = new_test_database(rt).await;
    let table_name: TableName = "table".parse()?;
    let foreign_table_name: TableName = "other_table".parse()?;

    let mut tx = database.begin(Identity::system()).await?;
    let mut table_model = TableModel::new(&mut tx);
    let mut table_mapping_for_import = TableMapping::new();
    let table_id = table_model
        .insert_table_for_import(
            TableNamespace::test_user(),
            &table_name,
            None,
            &BTreeSet::new(),
        )
        .await?;
    table_mapping_for_import.insert(
        table_id.tablet_id,
        TableNamespace::test_user(),
        table_id.table_number,
        table_name.clone(),
    );
    let foreign_table_id = table_model
        .insert_table_for_import(
            TableNamespace::test_user(),
            &foreign_table_name,
            None,
            &BTreeSet::new(),
        )
        .await?;
    table_mapping_for_import.insert(
        foreign_table_id.tablet_id,
        TableNamespace::test_user(),
        foreign_table_id.table_number,
        foreign_table_name.clone(),
    );
    database.commit(tx).await?;

    let mut tx = database.begin(Identity::system()).await?;
    let mut schema_model = SchemaModel::new_root_for_test(&mut tx);
    let db_schema = db_schema!(table_name.clone() => DocumentSchema::Union(
        vec![
            object_validator!(
                "foreign" => FieldValidator::required_field_type(Validator::Id(foreign_table_name.clone())),
            )
        ]
    ), foreign_table_name.clone() => DocumentSchema::Union(
        vec![object_validator!()]
    ));
    let (schema_id, _) = schema_model.submit_pending(db_schema).await?;
    schema_model.mark_validated(schema_id).await?;
    schema_model.mark_active(schema_id).await?;
    database.commit(tx).await?;

    let mut tx = database.begin(Identity::system()).await?;
    let mut table_mapping_for_schema = tx.table_mapping().clone();
    table_mapping_for_schema.update(table_mapping_for_import);
    let foreign_doc = ImportFacingModel::new(&mut tx)
        .insert(
            foreign_table_id,
            &foreign_table_name,
            assert_obj!(),
            &table_mapping_for_schema,
        )
        .await?;
    let foreign_doc_id =
        DeveloperDocumentId::new(foreign_table_id.table_number, foreign_doc.internal_id());
    ImportFacingModel::new(&mut tx)
        .insert(
            table_id,
            &table_name,
            assert_obj!("foreign" => foreign_doc_id),
            &table_mapping_for_schema,
        )
        .await?;
    database.commit(tx).await?;

    Ok(())
}

#[convex_macro::test_runtime]
async fn test_import_overwrite_foreign_reference_schema_validated(
    rt: TestRuntime,
) -> anyhow::Result<()> {
    // Schema says "table" has references to "other_table".
    // Then we do an import that swaps table numbers.
    let database = new_test_database(rt).await;
    let table_name: TableName = "table".parse()?;
    let foreign_table_name: TableName = "other_table".parse()?;

    let mut tx = database.begin(Identity::system()).await?;
    let mut table_model = TableModel::new(&mut tx);
    table_model
        .insert_table_metadata(TableNamespace::test_user(), &table_name)
        .await?;
    table_model
        .insert_table_metadata(TableNamespace::test_user(), &foreign_table_name)
        .await?;
    let active_table_number = tx
        .table_mapping()
        .namespace(TableNamespace::test_user())
        .id(&table_name)?
        .table_number;
    let active_foreign_table_number = tx
        .table_mapping()
        .namespace(TableNamespace::test_user())
        .id(&foreign_table_name)?
        .table_number;
    database.commit(tx).await?;

    let mut tx = database.begin(Identity::system()).await?;
    let mut table_model = TableModel::new(&mut tx);
    let mut table_mapping_for_import = TableMapping::new();
    // If tables_in_import is empty, there is a conflict.
    let mut tables_in_import = BTreeSet::new();
    assert!(table_model
        .insert_table_for_import(
            TableNamespace::test_user(),
            &table_name,
            Some(active_foreign_table_number),
            &tables_in_import
        )
        .await
        .is_err());
    tables_in_import.insert((TableNamespace::test_user(), table_name.clone()));
    tables_in_import.insert((TableNamespace::test_user(), foreign_table_name.clone()));
    // If tables_in_import is populated, we're allowed to create both tables.
    let table_id = table_model
        .insert_table_for_import(
            TableNamespace::test_user(),
            &table_name,
            Some(active_foreign_table_number),
            &tables_in_import,
        )
        .await?;
    table_mapping_for_import.insert(
        table_id.tablet_id,
        TableNamespace::test_user(),
        table_id.table_number,
        table_name.clone(),
    );
    let foreign_table_id = table_model
        .insert_table_for_import(
            TableNamespace::test_user(),
            &foreign_table_name,
            Some(active_table_number),
            &tables_in_import,
        )
        .await?;
    table_mapping_for_import.insert(
        foreign_table_id.tablet_id,
        TableNamespace::test_user(),
        foreign_table_id.table_number,
        foreign_table_name.clone(),
    );
    database.commit(tx).await?;

    let mut tx = database.begin(Identity::system()).await?;
    let mut schema_model = SchemaModel::new_root_for_test(&mut tx);
    let db_schema = db_schema!(table_name.clone() => DocumentSchema::Union(
        vec![
            object_validator!(
                "foreign" => FieldValidator::required_field_type(Validator::Id(foreign_table_name.clone())),
            )
        ]
    ), foreign_table_name.clone() => DocumentSchema::Union(
        vec![object_validator!()]
    ));
    let (schema_id, _) = schema_model.submit_pending(db_schema).await?;
    schema_model.mark_validated(schema_id).await?;
    schema_model.mark_active(schema_id).await?;
    database.commit(tx).await?;

    // Active tables can use schema as normal, despite the hidden table mapping.
    let mut tx = database.begin(Identity::system()).await?;
    let active_foreign_doc = UserFacingModel::new_root_for_test(&mut tx)
        .insert(foreign_table_name.clone(), assert_obj!())
        .await?;
    let active_foreign_doc_id = DeveloperDocumentId::new(
        active_foreign_table_number,
        active_foreign_doc.internal_id(),
    );
    UserFacingModel::new_root_for_test(&mut tx)
        .insert(
            table_name.clone(),
            assert_obj!("foreign" => active_foreign_doc_id),
        )
        .await?;
    database.commit(tx).await?;

    // Hidden tables can also use the schema, as long as you pass in
    // table_mapping_for_schema.
    let mut tx = database.begin(Identity::system()).await?;
    let mut table_mapping_for_schema = tx.table_mapping().clone();
    table_mapping_for_schema.update(table_mapping_for_import.clone());
    let foreign_doc = ImportFacingModel::new(&mut tx)
        .insert(
            foreign_table_id,
            &foreign_table_name,
            assert_obj!(),
            &table_mapping_for_schema,
        )
        .await?;
    let foreign_doc_id =
        DeveloperDocumentId::new(foreign_table_id.table_number, foreign_doc.internal_id());
    ImportFacingModel::new(&mut tx)
        .insert(
            table_id,
            &table_name,
            assert_obj!("foreign" => foreign_doc_id),
            &table_mapping_for_schema,
        )
        .await?;
    database.commit(tx).await?;

    let mut tx = database.begin(Identity::system()).await?;
    let mut table_model = TableModel::new(&mut tx);
    for (table_id, _, table_number, table_name) in table_mapping_for_import.iter() {
        table_model
            .activate_table(table_id, table_name, table_number, &tables_in_import)
            .await?;
    }
    database.commit(tx).await?;

    // Check everything was activated correctly.
    // Regression test, because previously activating one table might delete the
    // wrong tablet.
    let mut tx = database.begin(Identity::system()).await?;
    let table_mapping = tx.table_mapping();
    for (table_id, namespace, table_number, table_name) in table_mapping_for_import.iter() {
        assert_eq!(
            table_mapping.namespace(namespace).id_if_exists(table_name),
            Some(table_id)
        );
        assert_eq!(table_mapping.tablet_number(table_id)?, table_number);
    }

    Ok(())
}

#[convex_macro::test_runtime]
async fn test_overwrite_for_import(rt: TestRuntime) -> anyhow::Result<()> {
    let object = assert_obj!("value" => 1);

    let database = new_test_database(rt).await;
    let table_name: TableName = "table".parse()?;

    let mut tx = database.begin(Identity::system()).await?;
    let doc_id_user_facing = UserFacingModel::new_root_for_test(&mut tx)
        .insert(table_name.clone(), object.clone())
        .await?;
    let doc0_id = doc_id_user_facing.to_resolved(
        tx.table_mapping()
            .namespace(TableNamespace::test_user())
            .number_to_tablet(),
    )?;
    let doc0_id_str: String = DeveloperDocumentId::from(doc0_id).encode();
    database.commit(tx).await?;
    let object_with_id = assert_obj!("_id" => &*doc0_id_str, "value" => 2);

    let mut tx = database.begin(Identity::system()).await?;
    let mut table_model = TableModel::new(&mut tx);
    let table_id = table_model
        .insert_table_for_import(
            TableNamespace::test_user(),
            &table_name,
            Some(doc0_id.developer_id.table()),
            &BTreeSet::new(),
        )
        .await?;
    let mut table_mapping_for_schema = tx.table_mapping().clone();
    table_mapping_for_schema.insert(
        table_id.tablet_id,
        TableNamespace::test_user(),
        table_id.table_number,
        table_name.clone(),
    );
    let doc1_id = ImportFacingModel::new(&mut tx)
        .insert(
            table_id,
            &table_name,
            object_with_id,
            &table_mapping_for_schema,
        )
        .await?;
    let doc1_id = ResolvedDocumentId::new(
        table_id.tablet_id,
        DeveloperDocumentId::new(table_id.table_number, doc1_id.internal_id()),
    );
    database.commit(tx).await?;
    assert_eq!(doc1_id.internal_id(), doc0_id.internal_id());
    assert_eq!(doc1_id.developer_id.table(), doc0_id.developer_id.table());
    assert!(doc1_id.tablet_id != doc0_id.tablet_id);

    let mut tx = database.begin(Identity::system()).await?;
    let doc0 = tx.get_inner(doc0_id, table_name.clone()).await?.unwrap().0;
    let doc1 = tx.get_inner(doc1_id, table_name.clone()).await?.unwrap().0;
    assert_eq!(doc0.id(), doc0_id);
    assert_eq!(doc1.id(), doc1_id);
    assert_eq!(doc0.value().0.get("value"), Some(&val!(1)));
    assert_eq!(doc1.value().0.get("value"), Some(&val!(2)));
    let (doc_user_facing, _) = UserFacingModel::new_root_for_test(&mut tx)
        .get_with_ts(doc_id_user_facing, None)
        .await?
        .unwrap();
    assert_eq!(
        doc_user_facing.value().0.get("value"),
        Some(&assert_val!(1))
    );

    TableModel::new(&mut tx)
        .activate_table(
            table_id.tablet_id,
            &table_name,
            table_id.table_number,
            &BTreeSet::new(),
        )
        .await?;
    database.commit(tx).await?;

    let mut tx = database.begin(Identity::system()).await?;
    assert!(tx.get_inner(doc0_id, table_name.clone()).await?.is_none());
    let (doc_user_facing, _) = UserFacingModel::new_root_for_test(&mut tx)
        .get_with_ts(doc_id_user_facing, None)
        .await?
        .unwrap();
    assert_eq!(
        doc_user_facing.value().0.get("value"),
        Some(&assert_val!(2))
    );

    Ok(())
}

#[convex_macro::test_runtime]
async fn test_interrupted_import_then_delete_table(rt: TestRuntime) -> anyhow::Result<()> {
    let object = assert_obj!("value" => 1);
    let resolved_object = assert_obj!("value" => 1);

    let database = new_test_database(rt).await;
    let table_name: TableName = "table".parse()?;

    let mut tx = database.begin(Identity::system()).await?;
    let doc0_id = UserFacingModel::new_root_for_test(&mut tx)
        .insert(table_name.clone(), object)
        .await?;
    let doc0_id_inner = doc0_id.to_resolved(
        tx.table_mapping()
            .namespace(TableNamespace::test_user())
            .number_to_tablet(),
    )?;
    database.commit(tx).await?;

    let mut tx = database.begin(Identity::system()).await?;
    let mut table_model = TableModel::new(&mut tx);
    let table_id = table_model
        .insert_table_for_import(
            TableNamespace::test_user(),
            &table_name,
            None,
            &BTreeSet::new(),
        )
        .await?;
    let mut table_mapping_for_schema = tx.table_mapping().clone();
    table_mapping_for_schema.insert(
        table_id.tablet_id,
        TableNamespace::test_user(),
        table_id.table_number,
        table_name.clone(),
    );
    let doc1_id = ImportFacingModel::new(&mut tx)
        .insert(
            table_id,
            &table_name,
            resolved_object,
            &table_mapping_for_schema,
        )
        .await?;
    let doc1_id_inner = ResolvedDocumentId::new(
        table_id.tablet_id,
        DeveloperDocumentId::new(table_id.table_number, doc1_id.internal_id()),
    );
    database.commit(tx).await?;
    // Now the import fails. The hidden table never gets activated.
    // The active table still works.
    let mut tx = database.begin(Identity::system()).await?;
    assert!(UserFacingModel::new_root_for_test(&mut tx)
        .get_with_ts(doc0_id, None)
        .await?
        .is_some());
    assert!(UserFacingModel::new_root_for_test(&mut tx)
        .get_with_ts(doc1_id, None)
        .await?
        .is_none());
    // Delete the active table.
    TableModel::new(&mut tx)
        .delete_table(TableNamespace::test_user(), table_name.clone())
        .await?;
    database.commit(tx).await?;

    let mut tx = database.begin(Identity::system()).await?;
    assert!(UserFacingModel::new_root_for_test(&mut tx)
        .get_with_ts(doc0_id, None)
        .await?
        .is_none());
    assert!(UserFacingModel::new_root_for_test(&mut tx)
        .get_with_ts(doc1_id, None)
        .await?
        .is_none());
    assert!(tx
        .get_inner(doc0_id_inner, table_name.clone())
        .await?
        .is_none());
    // Document in hidden table is still accessible directly.
    assert!(tx
        .get_inner(doc1_id_inner, table_name.clone())
        .await?
        .is_some());
    // UsageWorker and friends can enumerate all enabled indexes.
    // This is a regression test.
    let enabled_indexes = tx.index.index_registry().all_enabled_indexes();
    // The Hidden table's index is there.
    assert_eq!(
        enabled_indexes
            .iter()
            .filter(|index| index.name.is_by_id() && index.name.table() == &table_id.tablet_id)
            .count(),
        1
    );
    for enabled_index in enabled_indexes {
        enabled_index
            .name
            .clone()
            .map_table(&tx.table_mapping().tablet_to_name())?;
    }
    Ok(())
}

#[convex_macro::test_runtime]
async fn add_indexes_at_limit_with_backfilling_index_adds_index(
    rt: TestRuntime,
) -> anyhow::Result<()> {
    // load once to initialize
    let DbFixtures { db, tp, .. } = DbFixtures::new(&rt).await?;
    let mut tx = db.begin(Identity::system()).await?;
    let begin_ts = tx.begin_timestamp();

    // Add the maximum allowed number of indexes.
    for i in 0..MAX_INDEXES_PER_TABLE {
        add_backfilling_index(&mut tx, begin_ts, i).await?;
    }
    db.commit(tx).await?;
    let retention_validator = Arc::new(NoopRetentionValidator);
    IndexWorker::new_terminating(rt, tp, retention_validator, db.clone()).await?;

    let mut tx = db.begin_system().await?;
    let begin_ts = tx.begin_timestamp();

    // This just needs to be any index for which we added an Index (heh) in the loop
    // above.
    let index = 0;
    let (index_name, _) = new_index_and_field_path(index)?;

    // Adding a second backfilling/backfilled index that matches an existing one
    // should fail.
    let result = add_backfilling_index(&mut tx, begin_ts, index).await;
    assert_single_pending_index_error(result);

    // Now enable the index
    IndexModel::new(&mut tx)
        .enable_index_for_testing(TableNamespace::test_user(), &index_name)
        .await?;

    // Then make sure we can add a new backfilling copy of it.
    add_backfilling_index(&mut tx, begin_ts, index).await?;
    // But not a second backfilling copy of it.
    let result = add_backfilling_index(&mut tx, begin_ts, index).await;
    assert_single_pending_index_error(result);

    Ok(())
}

fn assert_single_pending_index_error(result: anyhow::Result<ResolvedDocumentId>) {
    let err = result
        .expect_err("Successfully added a second pending index!")
        .to_string();
    assert!(
        err.contains("Cannot create a second pending index"),
        "Unexpected error {}",
        err
    );
}

fn new_index_and_field_path(index: usize) -> anyhow::Result<(IndexName, FieldPath)> {
    let field_name = format!("field_{}", index);
    let index_name = IndexName::new(
        "table".parse()?,
        IndexDescriptor::new(format!("by_{}", field_name))?,
    )?;
    Ok((index_name, field_name.parse()?))
}

async fn add_backfilling_index<RT: Runtime>(
    tx: &mut Transaction<RT>,
    begin_ts: RepeatableTimestamp,
    index: usize,
) -> anyhow::Result<ResolvedDocumentId> {
    let (index_name, field_path) = new_index_and_field_path(index)?;
    IndexModel::new(tx)
        .add_application_index(
            TableNamespace::test_user(),
            IndexMetadata::new_backfilling(*begin_ts, index_name, vec![field_path].try_into()?),
        )
        .await
}

#[convex_macro::test_runtime]
async fn test_add_indexes_limit(rt: TestRuntime) -> anyhow::Result<()> {
    // load once to initialize
    let DbFixtures { db, tp, .. } = DbFixtures::new(&rt).await?;
    let mut tx = db.begin(Identity::system()).await?;
    let begin_ts = tx.begin_timestamp();

    // Add the maximum allowed number of indexes.
    for i in 0..MAX_INDEXES_PER_TABLE {
        add_backfilling_index(&mut tx, begin_ts, i).await?;
    }
    // Try to add one more. Should fail.
    let err = add_backfilling_index(&mut tx, begin_ts, MAX_INDEXES_PER_TABLE + 1).await;
    assert_too_many_indexes_error(err);

    // Commit
    db.commit(tx).await?;

    // Load again with data to make sure we still can't add the index.
    let DbFixtures { db, .. } = DbFixtures::new_with_args(
        &rt,
        DbFixturesArgs {
            tp: Some(tp),
            ..Default::default()
        },
    )
    .await?;
    let mut tx = db.begin(Identity::system()).await?;
    let begin_ts = tx.begin_timestamp();
    let err = add_backfilling_index(&mut tx, begin_ts, MAX_INDEXES_PER_TABLE + 1).await;
    assert_too_many_indexes_error(err);

    Ok(())
}

fn assert_too_many_indexes_error(result: anyhow::Result<ResolvedDocumentId>) {
    let err = result
        .expect_err("Successfully added index field_max!")
        .to_string();
    assert!(
        err.contains(&format!(
            "Table \"table\" cannot have more than {MAX_INDEXES_PER_TABLE} indexes."
        )),
        "Unexpected error {}",
        err
    );
}

#[convex_macro::test_runtime]
async fn test_implicit_removal(rt: TestRuntime) -> anyhow::Result<()> {
    let database = new_test_database(rt).await;

    // Insert a document. That should implicitly create the table.
    let mut tx = database.begin(Identity::system()).await?;
    let document_id = TestFacingModel::new(&mut tx)
        .insert(
            &"messages".parse()?,
            assert_obj!(
                "channel" => "eng",
                "text" => "hello",
            ),
        )
        .await?;
    database.commit(tx).await?;

    assert!(database
        .table_names(Identity::system())?
        .contains(&"messages".parse()?));

    // Delete the document. The implicitly created table and default index should
    // stay.
    let mut tx = database.begin(Identity::system()).await?;
    UserFacingModel::new_root_for_test(&mut tx)
        .delete(document_id.into())
        .await
        .unwrap();
    database.commit(tx).await?;

    assert!(database
        .table_names(Identity::system())?
        .contains(&"messages".parse()?));

    // Add another document to the same table to make sure everything still works.
    let mut tx = database.begin(Identity::system()).await?;
    TestFacingModel::new(&mut tx)
        .insert(
            &"messages".parse()?,
            assert_obj!(
                "channel" => "eng",
                "text" => "hello",
            ),
        )
        .await?;
    database.commit(tx).await?;

    assert!(database
        .table_names(Identity::system())?
        .contains(&"messages".parse()?));

    Ok(())
}

/// A variant of test_query_index_range that adds the index *after* the
/// documents have been added, testing that index backfill works correctly.
#[convex_macro::test_runtime]
async fn test_index_backfill(rt: TestRuntime) -> anyhow::Result<()> {
    let DbFixtures { db, tp, .. } = DbFixtures::new(&rt).await?;

    let table_name: TableName = str::parse("table")?;
    let namespace = TableNamespace::test_user();
    let mut tx = db.begin_system().await?;
    let values = insert_documents(&mut tx, table_name.clone()).await?;
    db.commit(tx).await?;

    let index_name = IndexName::new(table_name, IndexDescriptor::new("a_and_b")?)?;
    let mut tx = db.begin_system().await?;
    let begin_ts = tx.begin_timestamp();
    IndexModel::new(&mut tx)
        .add_application_index(
            namespace,
            IndexMetadata::new_backfilling(
                *begin_ts,
                index_name.clone(),
                vec![str::parse("a")?, str::parse("b")?].try_into()?,
            ),
        )
        .await?;
    db.commit(tx).await?;

    let retention_validator = Arc::new(NoopRetentionValidator);

    let index_backfill_fut = IndexWorker::new_terminating(rt, tp, retention_validator, db.clone());
    index_backfill_fut.await?;

    let mut tx = db.begin_system().await?;
    IndexModel::new(&mut tx)
        .enable_index_for_testing(namespace, &index_name)
        .await?;
    db.commit(tx).await?;

    let tests: Vec<(_, _, Box<dyn Fn(i64, i64) -> bool>)> = vec![
        // single_page_asc
        (
            vec![
                IndexRangeExpression::Eq("a".parse()?, maybe_val!(3)),
                IndexRangeExpression::Gte("b".parse()?, val!(113)),
                IndexRangeExpression::Lte("b".parse()?, val!(117)),
            ],
            Order::Asc,
            Box::new(|a, b| a == 3 && (113..=117).contains(&b)),
        ),
        // prefix
        (
            vec![IndexRangeExpression::Eq("a".parse()?, maybe_val!(3))],
            Order::Asc,
            Box::new(|a, _| a == 3),
        ),
        // all_multi_page_desc
        (vec![], Order::Desc, Box::new(|_, _| true)),
    ];
    for (range, order, predicate) in tests {
        let mut expected = values
            .iter()
            .filter(|x| {
                must_let!(let ConvexValue::Int64(a) = x.value().get("a").unwrap());
                must_let!(let ConvexValue::Int64(b) = x.value().get("b").unwrap());
                predicate(*a, *b)
            })
            .cloned()
            .collect::<Vec<ResolvedDocument>>();
        if order == Order::Desc {
            expected.reverse();
        }

        let query = Query {
            source: QuerySource::IndexRange(IndexRange {
                index_name: index_name.clone(),
                range,
                order,
            }),
            operators: vec![],
        };
        let actual = run_query(db.clone(), namespace, query).await?;
        assert_eq!(actual, expected);
    }
    Ok(())
}

// Same as test_index_backfill but writing the index with IndexWriter directly.
#[convex_macro::test_runtime]
async fn test_index_write(rt: TestRuntime) -> anyhow::Result<()> {
    let DbFixtures {
        db: database, tp, ..
    } = DbFixtures::new(&rt).await?;

    let table_name: TableName = str::parse("table")?;
    let namespace = TableNamespace::test_user();
    let mut tx = database.begin(Identity::system()).await?;
    let values = insert_documents(&mut tx, table_name.clone()).await?;
    database.commit(tx).await?;

    let index_name = IndexName::new(table_name, IndexDescriptor::new("a_and_b")?)?;
    let mut tx = database.begin(Identity::system()).await?;
    IndexModel::new(&mut tx)
        .add_application_index(
            namespace,
            IndexMetadata::new_enabled(
                index_name.clone(),
                vec![str::parse("a")?, str::parse("b")?].try_into()?,
            ),
        )
        .await?;
    let ts = database.commit(tx).await?;

    let retention_validator = Arc::new(NoopRetentionValidator);

    let index_writer = IndexWriter::new(
        tp.clone(),
        tp.reader(),
        retention_validator.clone(),
        rt.clone(),
    );
    let database_snapshot = DatabaseSnapshot::load(
        rt.clone(),
        tp.reader(),
        unchecked_repeatable_ts(ts),
        retention_validator,
    )
    .await?;
    let index_metadata = database_snapshot.index_registry().clone();
    index_writer
        .perform_backfill(
            unchecked_repeatable_ts(ts),
            &index_metadata,
            IndexSelector::All(index_metadata.clone()),
        )
        .await?;

    let tests: Vec<(_, _, Box<dyn Fn(i64, i64) -> bool>)> = vec![
        // single_page_asc
        (
            vec![
                IndexRangeExpression::Eq("a".parse()?, maybe_val!(3)),
                IndexRangeExpression::Gte("b".parse()?, val!(113)),
                IndexRangeExpression::Lte("b".parse()?, val!(117)),
            ],
            Order::Asc,
            Box::new(|a, b| a == 3 && (113..=117).contains(&b)),
        ),
        // prefix
        (
            vec![IndexRangeExpression::Eq("a".parse()?, maybe_val!(3))],
            Order::Asc,
            Box::new(|a, _| a == 3),
        ),
        // all_multi_page_desc
        (vec![], Order::Desc, Box::new(|_, _| true)),
    ];
    for (range, order, predicate) in tests {
        let mut expected = values
            .iter()
            .filter(|x| {
                must_let!(let ConvexValue::Int64(a) = x.value().get("a").unwrap());
                must_let!(let ConvexValue::Int64(b) = x.value().get("b").unwrap());
                predicate(*a, *b)
            })
            .cloned()
            .collect::<Vec<ResolvedDocument>>();
        if order == Order::Desc {
            expected.reverse();
        }

        let query = Query {
            source: QuerySource::IndexRange(IndexRange {
                index_name: index_name.clone(),
                range,
                order,
            }),
            operators: vec![],
        };
        let actual = run_query(database.clone(), namespace, query).await?;
        assert_eq!(actual, expected);
    }
    Ok(())
}

#[convex_macro::test_runtime]
async fn create_system_table_creates_table_marked_as_system(rt: TestRuntime) -> anyhow::Result<()> {
    let db = new_test_database(rt).await;
    let table_name = "_my_system_table";
    let mut tx = db.begin_system().await?;
    assert!(
        tx.create_system_table_testing(TableNamespace::Global, &table_name.parse()?, None)
            .await?
    );
    db.commit(tx).await?;

    let mut tx = db.begin_system().await?;
    let table_id = (tx
        .table_mapping()
        .namespace(TableNamespace::Global)
        .name_to_id())(table_name.parse()?)?;
    assert!(tx.table_mapping().is_system_tablet(table_id.tablet_id));
    Ok(())
}

#[convex_macro::test_runtime]
async fn create_system_table_with_non_system_table_fails(rt: TestRuntime) -> anyhow::Result<()> {
    let db = new_test_database(rt).await;
    let table_name = "invalid_system_table_name";
    let mut tx = db.begin_system().await?;
    let result = tx
        .create_system_table_testing(TableNamespace::Global, &table_name.parse()?, None)
        .await;
    let err = result.expect_err("create_system_table allowed a non-system table name");
    assert_eq!(
        err.to_string(),
        format!("\"{table_name}\" is not a valid system table name!")
    );
    Ok(())
}

#[convex_macro::test_runtime]
async fn test_create_system_table_for_virtual_table(rt: TestRuntime) -> anyhow::Result<()> {
    let mut virtual_system_mapping = VirtualSystemMapping::default();
    virtual_system_mapping.add_table(
        &"_storage".parse()?,
        &"_file_storage".parse()?,
        BTreeMap::new(),
        Arc::new(NoopDocMapper),
    );
    let virtual_system = virtual_system_mapping.clone();
    let db = DbFixtures::new_with_args(
        &rt,
        DbFixturesArgs {
            virtual_system_mapping,
            ..Default::default()
        },
    )
    .await?
    .db;

    let table_number = 530.try_into()?;
    let mut tx = db.begin_system().await?;
    tx.create_system_table(
        TableNamespace::test_user(),
        &"_file_storage".parse()?,
        Some(table_number),
    )
    .await?;
    db.commit(tx).await?;

    let mut tx = db.begin_system().await?;
    let physical_table_number = all_tables_name_to_number(
        TableNamespace::test_user(),
        tx.table_mapping(),
        &virtual_system,
    )("_file_storage".parse()?)?;
    let virtual_table_number = all_tables_name_to_number(
        TableNamespace::test_user(),
        tx.table_mapping(),
        &virtual_system,
    )("_storage".parse()?)?;
    assert_eq!(physical_table_number, table_number);
    assert_eq!(virtual_table_number, table_number);
    let table_name = all_tables_number_to_name(
        &tx.table_mapping().namespace(TableNamespace::test_user()),
        &virtual_system,
    )(table_number)?;
    assert_eq!(table_name, "_storage".parse()?);

    Ok(())
}

#[convex_macro::test_runtime]
async fn test_retries(rt: TestRuntime) -> anyhow::Result<()> {
    let db = new_test_database(rt).await;
    let namespace = TableNamespace::test_user();
    async fn insert(tx: &mut Transaction<TestRuntime>) -> anyhow::Result<()> {
        UserFacingModel::new_root_for_test(tx)
            .insert("table".parse()?, assert_obj!())
            .await?;
        anyhow::bail!("fail this fn!");
    }
    db.execute_with_occ_retries(
        Identity::system(),
        FunctionUsageTracker::new(),
        WriteSource::unknown(),
        |tx| insert(tx).into(),
    )
    .await
    .expect_err("Retry fn should fail when f fails");

    let mut tx = db.begin_system().await?;
    let query = Query::full_table_scan("table".parse()?, Order::Asc);
    let mut compiled_query = ResolvedQuery::new(&mut tx, namespace, query)?;
    assert!(compiled_query.next(&mut tx, None).await?.is_none());
    Ok(())
}

#[convex_macro::test_runtime]
/// Test that the retry wrapper retries on failures in the function it is
/// retrying, not just commit failures.
async fn test_retries_includes_f(rt: TestRuntime) -> anyhow::Result<()> {
    let db = new_test_database(rt).await;
    let max_retries = 3;
    /// Overloaded returns an overloaded error until the channel is empty
    async fn overloaded(
        _tx: &mut Transaction<TestRuntime>,
        receiver: async_channel::Receiver<()>,
    ) -> anyhow::Result<()> {
        if receiver.try_recv().is_ok() {
            anyhow::bail!(ErrorMetadata::overloaded(
                "OverloadedTest",
                "Test overloaded error"
            ))
        }
        Ok(())
    }
    // Channel has max_retries - 1 entries so it should succeed
    let (sender, receiver) = async_channel::bounded(max_retries - 1);
    for _i in 0..max_retries - 1 {
        sender.send(()).await?;
    }
    db.execute_with_retries(
        Identity::system(),
        max_retries as u32,
        Backoff::new(Duration::from_secs(0), Duration::from_millis(10)),
        FunctionUsageTracker::new(),
        |e: &anyhow::Error| e.is_overloaded(),
        WriteSource::unknown(),
        |tx| overloaded(tx, receiver.clone()).into(),
    )
    .await?;

    // Channel that has max_retries entries should fail
    let (sender, receiver) = async_channel::bounded(max_retries);
    for _i in 0..max_retries {
        sender.send(()).await?;
    }
    let err = db
        .execute_with_retries(
            Identity::system(),
            max_retries as u32,
            Backoff::new(Duration::from_secs(0), Duration::from_millis(10)),
            FunctionUsageTracker::new(),
            |e: &anyhow::Error| e.is_overloaded(),
            WriteSource::unknown(),
            |tx| overloaded(tx, receiver.clone()).into(),
        )
        .await
        .unwrap_err();
    assert!(err.is_overloaded());

    Ok(())
}

async fn add_and_enable_index(
    rt: TestRuntime,
    database: &Database<TestRuntime>,
    tp: Arc<dyn Persistence>,
    namespace: TableNamespace,
    index_name: &IndexName,
    fields: IndexedFields,
) -> anyhow::Result<()> {
    let mut tx = database.begin(Identity::system()).await?;
    let begin_ts = tx.begin_timestamp();
    IndexModel::new(&mut tx)
        .add_application_index(
            namespace,
            IndexMetadata::new_backfilling(*begin_ts, index_name.clone(), fields),
        )
        .await?;
    database.commit(tx).await?;

    let retention_validator = Arc::new(NoopRetentionValidator);

    // Backfill the index.
    let index_backfill_fut =
        IndexWorker::new_terminating(rt, tp, retention_validator, database.clone());
    index_backfill_fut.await?;

    let mut tx = database.begin_system().await?;
    IndexModel::new(&mut tx)
        .enable_index_for_testing(namespace, index_name)
        .await?;
    database.commit(tx).await?;
    Ok(())
}

#[convex_macro::test_runtime]
async fn test_query_filter_readset(rt: TestRuntime) -> anyhow::Result<()> {
    let DbFixtures {
        db: database, tp, ..
    } = DbFixtures::new(&rt).await?;
    let namespace = TableNamespace::test_user();

    let table_name: TableName = str::parse("messages")?;
    let index_name = IndexName::new(table_name.clone(), IndexDescriptor::new("by_rank")?)?;
    let index_fields: IndexedFields = vec!["rank".parse()?].try_into()?;

    add_and_enable_index(rt, &database, tp, namespace, &index_name, index_fields).await?;

    // Insert 3 documents
    let mut tx = database.begin(Identity::system()).await?;
    let doc1 = TestFacingModel::new(&mut tx)
        .insert_and_get(
            "messages".parse()?,
            assert_obj!(
                "channel" => "eng",
                "text" => "hello",
                "rank" => 1.0
            ),
        )
        .await?;
    let doc2 = TestFacingModel::new(&mut tx)
        .insert_and_get(
            "messages".parse()?,
            assert_obj!(
                "channel" => "eng",
                "text" => "hello again",
                "rank" => 2.0
            ),
        )
        .await?;
    TestFacingModel::new(&mut tx)
        .insert(
            &"messages".parse()?,
            assert_obj!(
                "channel" => "eng",
                "text" => "@here",
                "rank" => 3.0
            ),
        )
        .await?;
    database.commit(tx).await?;

    // Query the first two -- e.g. `take(2)`, but prefetch more.
    let query = Query {
        source: QuerySource::IndexRange(IndexRange {
            index_name,
            range: vec![IndexRangeExpression::Gt(
                "rank".parse()?,
                ConvexValue::Float64(0.0),
            )],
            order: Order::Asc,
        }),
        operators: vec![QueryOperator::Filter(Expression::Eq(
            Box::new(Expression::Literal(maybe_val!("eng"))),
            Box::new(Expression::Field("channel".parse()?)),
        ))],
    };
    let mut tx = database.begin(Identity::system()).await?;
    let mut query_stream = ResolvedQuery::new(&mut tx, namespace, query)?;
    let Some(result1) = query_stream.next(&mut tx, Some(3)).await? else {
        anyhow::bail!("Query unexpectedly empty")
    };
    assert_eq!(result1, doc1);
    let Some(result2) = query_stream.next(&mut tx, Some(3)).await? else {
        anyhow::bail!("Query unexpectedly empty")
    };
    assert_eq!(result2, doc2);
    let token = tx.into_token()?;

    let mut tx = database.begin(Identity::system()).await?;
    let out_of_range_doc = TestFacingModel::new(&mut tx)
        .insert_and_get(
            "messages".parse()?,
            assert_obj!(
                "channel" => "eng",
                "text" => "world",
                "rank" => 2.5
            ),
        )
        .await?;
    let in_range_doc = TestFacingModel::new(&mut tx)
        .insert_and_get(
            "messages".parse()?,
            assert_obj!(
                "channel" => "eng",
                "text" => "world",
                "rank" => 1.5
            ),
        )
        .await?;

    // A document at rank 2.5 should not overlap with the readset, even though we
    // prefetched through rank 3.0
    assert!(token
        .reads()
        .overlaps(
            &PackedDocument::pack(out_of_range_doc),
            PersistenceVersion::default()
        )
        .is_none());

    // A document at rank 1.5 should overlap with the readest
    assert!(token
        .reads()
        .overlaps(
            &PackedDocument::pack(in_range_doc),
            PersistenceVersion::default()
        )
        .is_some());
    Ok(())
}

#[convex_macro::test_runtime]
async fn test_query_readset_empty_query(rt: TestRuntime) -> anyhow::Result<()> {
    let DbFixtures {
        db: database, tp, ..
    } = DbFixtures::new(&rt).await?;
    let namespace = TableNamespace::test_user();

    let table_name: TableName = str::parse("messages")?;
    let index_name = IndexName::new(table_name.clone(), IndexDescriptor::new("by_rank")?)?;
    let index_fields: IndexedFields = vec!["rank".parse()?].try_into()?;
    add_and_enable_index(rt, &database, tp, namespace, &index_name, index_fields).await?;

    // Insert a document
    let mut tx = database.begin(Identity::system()).await?;
    TestFacingModel::new(&mut tx)
        .insert_and_get(
            "messages".parse()?,
            assert_obj!(
                "channel" => "eng",
                "text" => "hello",
                "rank" => 1.0
            ),
        )
        .await?;
    database.commit(tx).await?;

    // Query for an empty range
    let query = Query {
        source: QuerySource::IndexRange(IndexRange {
            index_name,
            range: vec![IndexRangeExpression::Lt(
                "rank".parse()?,
                ConvexValue::Float64(0.0),
            )],
            order: Order::Asc,
        }),
        operators: vec![],
    };
    let mut tx = database.begin(Identity::system()).await?;
    let mut query_stream = ResolvedQuery::new(&mut tx, namespace, query)?;
    assert!(query_stream.next(&mut tx, Some(3)).await?.is_none());
    let token = tx.into_token()?;

    let mut tx = database.begin(Identity::system()).await?;
    let in_range_doc = TestFacingModel::new(&mut tx)
        .insert_and_get(
            "messages".parse()?,
            assert_obj!(
                "channel" => "eng",
                "text" => "world",
                "rank" => -5.0
            ),
        )
        .await?;

    // A document at rank -5.0 should overlap with the readest
    assert!(token
        .reads()
        .overlaps(
            &PackedDocument::pack(in_range_doc),
            PersistenceVersion::default()
        )
        .is_some());
    Ok(())
}

#[convex_macro::test_runtime]
async fn test_subtransaction_success_commits_writes(rt: TestRuntime) -> anyhow::Result<()> {
    let db = DbFixtures::new(&rt).await?.db;
    let mut tx = db.begin(Identity::system()).await?;
    let table_name: TableName = "table".parse()?;
    let doc_id0 = TestFacingModel::new(&mut tx)
        .insert(&table_name, assert_obj!("value" => 1))
        .await?;
    let doc_id1 = {
        let tokens = tx.begin_subtransaction();
        let (doc0, ts) = tx.get_inner(doc_id0, table_name.clone()).await?.unwrap();
        assert_eq!(ts, WriteTimestamp::Pending);
        assert_eq!(doc0.value().0.get("value"), Some(&val!(1)));
        let doc_id1 = TestFacingModel::new(&mut tx)
            .insert(&table_name, assert_obj!("value" => 2))
            .await?;
        let (doc1, ts) = tx.get_inner(doc_id1, table_name.clone()).await?.unwrap();
        assert_eq!(ts, WriteTimestamp::Pending);
        assert_eq!(doc1.value().0.get("value"), Some(&val!(2)));
        tx.commit_subtransaction(tokens)?;
        doc_id1
    };
    let (doc1, ts) = tx.get_inner(doc_id1, table_name.clone()).await?.unwrap();
    assert_eq!(ts, WriteTimestamp::Pending);
    assert_eq!(doc1.value().0.get("value"), Some(&val!(2)));
    let commit_ts = db.commit(tx).await?;
    let mut tx = db.begin(Identity::system()).await?;
    let (doc0, ts) = tx.get_inner(doc_id0, table_name.clone()).await?.unwrap();
    assert_eq!(ts, WriteTimestamp::Committed(commit_ts));
    assert_eq!(doc0.value().0.get("value"), Some(&val!(1)));
    let (doc1, ts) = tx.get_inner(doc_id1, table_name.clone()).await?.unwrap();
    assert_eq!(ts, WriteTimestamp::Committed(commit_ts));
    assert_eq!(doc1.value().0.get("value"), Some(&val!(2)));
    Ok(())
}

#[convex_macro::test_runtime]
async fn test_subtransaction_failure_rolls_back_writes(rt: TestRuntime) -> anyhow::Result<()> {
    let db = DbFixtures::new(&rt).await?.db;
    let mut tx = db.begin(Identity::system()).await?;
    let table_name: TableName = "table".parse()?;
    let doc_id0 = TestFacingModel::new(&mut tx)
        .insert(&table_name, assert_obj!("value" => 1))
        .await?;
    let doc_id1 = {
        let tokens = tx.begin_subtransaction();
        let (doc0, ts) = tx.get_inner(doc_id0, table_name.clone()).await?.unwrap();
        assert_eq!(ts, WriteTimestamp::Pending);
        assert_eq!(doc0.value().0.get("value"), Some(&val!(1)));
        let doc_id1 = TestFacingModel::new(&mut tx)
            .insert(&table_name, assert_obj!("value" => 2))
            .await?;
        let (doc1, ts) = tx.get_inner(doc_id1, table_name.clone()).await?.unwrap();
        assert_eq!(ts, WriteTimestamp::Pending);
        assert_eq!(doc1.value().0.get("value"), Some(&val!(2)));
        tx.rollback_subtransaction(tokens)?;
        doc_id1
    };
    let doc1 = tx.get_inner(doc_id1, table_name.clone()).await?;
    assert_eq!(doc1, None);
    let commit_ts = db.commit(tx).await?;
    let mut tx = db.begin(Identity::system()).await?;
    let (doc0, ts) = tx.get_inner(doc_id0, table_name.clone()).await?.unwrap();
    assert_eq!(ts, WriteTimestamp::Committed(commit_ts));
    assert_eq!(doc0.value().0.get("value"), Some(&val!(1)));
    let doc1 = tx.get_inner(doc_id1, table_name.clone()).await?;
    assert_eq!(doc1, None);
    Ok(())
}

#[convex_macro::test_runtime]
async fn test_subtransaction_failure_rolls_back_table_creation(
    rt: TestRuntime,
) -> anyhow::Result<()> {
    let db = DbFixtures::new(&rt).await?.db;
    let mut tx = db.begin(Identity::system()).await?;
    let table_name: TableName = "table".parse()?;
    {
        let tokens = tx.begin_subtransaction();
        TestFacingModel::new(&mut tx)
            .insert(&table_name, assert_obj!("value" => 2))
            .await?;
        assert!(TableModel::new(&mut tx).table_exists(TableNamespace::test_user(), &table_name));
        tx.rollback_subtransaction(tokens)?;
    };
    assert!(!TableModel::new(&mut tx).table_exists(TableNamespace::test_user(), &table_name));
    db.commit(tx).await?;
    let mut tx = db.begin(Identity::system()).await?;
    assert!(!TableModel::new(&mut tx).table_exists(TableNamespace::test_user(), &table_name));
    Ok(())
}

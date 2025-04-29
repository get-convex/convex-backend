use std::{
    cmp::max,
    collections::{
        BTreeMap,
        BTreeSet,
    },
    sync::Arc,
};

use futures::{
    pin_mut,
    Future,
    StreamExt,
    TryStreamExt,
};
use itertools::Itertools;
use maplit::{
    btreemap,
    btreeset,
};
use proptest::collection::size_range;
use serde_json::json;
use value::{
    assert_val,
    val,
    ConvexObject,
    ConvexValue,
    DeveloperDocumentId,
    InternalDocumentId,
    ResolvedDocumentId,
    TableMapping,
    TabletId,
};

use crate::{
    assert_obj,
    bootstrap_model::index::{
        database_index::IndexedFields,
        INDEX_TABLE,
    },
    document::{
        CreationTime,
        ResolvedDocument,
    },
    index::IndexKey,
    interval::{
        BinaryKey,
        End,
        Interval,
        StartIncluded,
    },
    persistence::{
        fake_retention_validator::FakeRetentionValidator,
        ConflictStrategy,
        DocumentLogEntry,
        DocumentPrevTsQuery,
        LatestDocument,
        NoopRetentionValidator,
        Persistence,
        PersistenceGlobalKey,
        TimestampRange,
    },
    query::Order,
    testing::{
        self,
        test_id_generator::TestIdGenerator,
    },
    types::{
        DatabaseIndexUpdate,
        DatabaseIndexValue,
        TableName,
        Timestamp,
    },
};

#[macro_export]
macro_rules! run_persistence_test_suite {
    ($db:ident, $create_db:expr, $create_persistence:expr, $create_persistence_read_only:expr) => {
        #[tokio::test]
        async fn test_persistence_write_and_load() -> anyhow::Result<()> {
            let $db = $create_db;
            let p = $create_persistence;
            persistence_test_suite::write_and_load(::std::sync::Arc::new(p)).await
        }

        #[tokio::test]
        async fn test_persistence_write_and_load_from_table() -> anyhow::Result<()> {
            let $db = $create_db;
            let p = $create_persistence;
            persistence_test_suite::write_and_load_from_table(::std::sync::Arc::new(p)).await
        }

        #[tokio::test]
        async fn test_persistence_write_and_load_value_types() -> anyhow::Result<()> {
            let $db = $create_db;
            let p = $create_persistence;
            persistence_test_suite::write_and_load_value_types(::std::sync::Arc::new(p)).await
        }

        #[tokio::test]
        async fn test_persistence_overwrite_document() -> anyhow::Result<()> {
            let $db = $create_db;
            let p = $create_persistence;
            persistence_test_suite::overwrite_document(::std::sync::Arc::new(p)).await
        }

        #[tokio::test]
        async fn test_persistence_overwrite_index() -> anyhow::Result<()> {
            let $db = $create_db;
            let p = $create_persistence;
            persistence_test_suite::overwrite_index(::std::sync::Arc::new(p)).await
        }

        #[tokio::test]
        async fn test_persistence_write_and_load_sorting() -> anyhow::Result<()> {
            let $db = $create_db;
            let p = $create_persistence;
            persistence_test_suite::write_and_load_sorting(::std::sync::Arc::new(p)).await
        }

        #[tokio::test]
        async fn test_persistence_same_internal_id_multiple_tables() -> anyhow::Result<()> {
            let $db = $create_db;
            let p = $create_persistence;
            persistence_test_suite::same_internal_id_multiple_tables(::std::sync::Arc::new(p)).await
        }

        #[tokio::test]
        async fn test_persistence_query_index_at_ts() -> anyhow::Result<()> {
            let $db = $create_db;
            let p = $create_persistence;
            persistence_test_suite::query_index_at_ts(::std::sync::Arc::new(p)).await
        }

        #[tokio::test]
        async fn test_persistence_query_index_range_short() -> anyhow::Result<()> {
            let $db = $create_db;
            let p = $create_persistence;
            persistence_test_suite::query_index_range_short(::std::sync::Arc::new(p)).await
        }

        #[tokio::test]
        async fn test_persistence_query_index_range_long() -> anyhow::Result<()> {
            let $db = $create_db;
            let p = $create_persistence;
            persistence_test_suite::query_index_range_long(::std::sync::Arc::new(p)).await
        }

        #[tokio::test]
        async fn test_persistence_query_multiple_indexes() -> anyhow::Result<()> {
            let $db = $create_db;
            let p = $create_persistence;
            persistence_test_suite::query_multiple_indexes(::std::sync::Arc::new(p)).await
        }

        #[tokio::test]
        async fn test_persistence_query_dangling_reference() -> anyhow::Result<()> {
            let $db = $create_db;
            let p = $create_persistence;
            persistence_test_suite::query_dangling_reference(::std::sync::Arc::new(p)).await
        }

        #[tokio::test]
        async fn test_persistence_query_reference_deleted_doc() -> anyhow::Result<()> {
            let $db = $create_db;
            let p = $create_persistence;
            persistence_test_suite::query_reference_deleted_doc(::std::sync::Arc::new(p)).await
        }

        #[tokio::test]
        async fn test_persistence_query_with_rows_estimate_short() -> anyhow::Result<()> {
            let $db = $create_db;
            let p = $create_persistence;
            persistence_test_suite::query_with_rows_estimate_short(::std::sync::Arc::new(p)).await
        }

        #[tokio::test]
        async fn test_persistence_query_with_rows_estimate_long() -> anyhow::Result<()> {
            let $db = $create_db;
            let p = $create_persistence;
            persistence_test_suite::query_with_rows_estimate_long(::std::sync::Arc::new(p)).await
        }

        #[tokio::test]
        async fn test_persistence_write_then_read() -> anyhow::Result<()> {
            let $db = $create_db;
            persistence_test_suite::write_then_read(|| async {
                Ok(::std::sync::Arc::new($create_persistence))
            })
            .await
        }

        #[tokio::test]
        async fn test_persistence_set_read_only() -> anyhow::Result<()> {
            let $db = $create_db;
            persistence_test_suite::set_read_only(
                || async { Ok($create_persistence) },
                || async { Ok($create_persistence_read_only) },
            )
            .await
        }

        #[tokio::test]
        async fn test_persistence_global() -> anyhow::Result<()> {
            let $db = $create_db;
            let p = $create_persistence;
            persistence_test_suite::persistence_global(::std::sync::Arc::new(p)).await
        }

        #[tokio::test]
        async fn test_persistence_enforce_retention() -> anyhow::Result<()> {
            let $db = $create_db;
            let p = $create_persistence;
            persistence_test_suite::persistence_enforce_retention(::std::sync::Arc::new(p)).await
        }

        #[tokio::test]
        async fn test_persistence_delete_documents() -> anyhow::Result<()> {
            let $db = $create_db;
            let p = $create_persistence;
            persistence_test_suite::persistence_delete_documents(::std::sync::Arc::new(p)).await
        }

        #[tokio::test]
        async fn test_persistence_previous_revisions_of_documents() -> anyhow::Result<()> {
            let $db = $create_db;
            let p = $create_persistence;
            persistence_test_suite::persistence_previous_revisions_of_documents(
                ::std::sync::Arc::new(p),
            )
            .await
        }

        #[tokio::test]
        async fn test_persistence_previous_revisions() -> anyhow::Result<()> {
            let $db = $create_db;
            let p = $create_persistence;
            persistence_test_suite::persistence_previous_revisions(::std::sync::Arc::new(p)).await
        }
    };
}

pub async fn write_and_load_from_table<P: Persistence>(p: Arc<P>) -> anyhow::Result<()> {
    let mut id_generator = TestIdGenerator::new();
    let table1: TableName = str::parse("table1")?;
    let doc_id1 = id_generator.user_generate(&table1);
    let doc1 = ResolvedDocument::new(doc_id1, CreationTime::ONE, ConvexObject::empty())?;

    let table2: TableName = str::parse("table2")?;
    let doc_id2 = id_generator.user_generate(&table2);
    let doc2 = ResolvedDocument::new(doc_id2, CreationTime::ONE, ConvexObject::empty())?;

    p.write(
        vec![
            // Write docs
            DocumentLogEntry {
                ts: Timestamp::must(0),
                id: doc1.id_with_table_id(),
                value: Some(doc1.clone()),
                prev_ts: None,
            },
            DocumentLogEntry {
                ts: Timestamp::must(0),
                id: doc2.id_with_table_id(),
                value: Some(doc2.clone()),
                prev_ts: None,
            },
            // Delete doc
            DocumentLogEntry {
                ts: Timestamp::must(1),
                id: doc1.id_with_table_id(),
                value: None,
                prev_ts: Some(Timestamp::must(0)),
            },
            DocumentLogEntry {
                ts: Timestamp::must(1),
                id: doc2.id_with_table_id(),
                value: None,
                prev_ts: Some(Timestamp::must(0)),
            },
        ],
        BTreeSet::new(),
        ConflictStrategy::Error,
    )
    .await?;
    id_generator.write_tables(p.clone()).await?;

    test_load_documents_from_table(
        &p,
        doc1.id().tablet_id,
        TimestampRange::all(),
        Order::Asc,
        vec![
            DocumentLogEntry {
                ts: Timestamp::must(0),
                id: doc1.id_with_table_id(),
                value: Some(doc1.clone()),
                prev_ts: None,
            },
            DocumentLogEntry {
                ts: Timestamp::must(1),
                id: doc1.id_with_table_id(),
                value: None,
                prev_ts: Some(Timestamp::must(0)),
            },
        ],
    )
    .await?;

    test_load_documents_from_table(
        &p,
        doc2.id().tablet_id,
        TimestampRange::all(),
        Order::Asc,
        vec![
            DocumentLogEntry {
                ts: Timestamp::must(0),
                id: doc2.id_with_table_id(),
                value: Some(doc2.clone()),
                prev_ts: None,
            },
            DocumentLogEntry {
                ts: Timestamp::must(1),
                id: doc2.id_with_table_id(),
                value: None,
                prev_ts: Some(Timestamp::must(0)),
            },
        ],
    )
    .await?;

    test_load_documents_from_table(
        &p,
        doc1.id().tablet_id,
        TimestampRange::new(Timestamp::must(1)..)?,
        Order::Asc,
        vec![DocumentLogEntry {
            ts: Timestamp::must(1),
            id: doc1.id_with_table_id(),
            value: None,
            prev_ts: Some(Timestamp::must(0)),
        }],
    )
    .await?;

    test_load_documents_from_table(
        &p,
        doc2.id().tablet_id,
        TimestampRange::new(Timestamp::must(1)..)?,
        Order::Asc,
        vec![DocumentLogEntry {
            ts: Timestamp::must(1),
            id: doc2.id_with_table_id(),
            value: None,
            prev_ts: Some(Timestamp::must(0)),
        }],
    )
    .await?;

    test_load_documents_from_table(
        &p,
        doc1.id().tablet_id,
        TimestampRange::new(..Timestamp::must(1))?,
        Order::Asc,
        vec![DocumentLogEntry {
            ts: Timestamp::must(0),
            id: doc1.id_with_table_id(),
            value: Some(doc1.clone()),
            prev_ts: None,
        }],
    )
    .await?;

    test_load_documents_from_table(
        &p,
        doc2.id().tablet_id,
        TimestampRange::new(..Timestamp::must(1))?,
        Order::Asc,
        vec![DocumentLogEntry {
            ts: Timestamp::must(0),
            id: doc2.id_with_table_id(),
            value: Some(doc2.clone()),
            prev_ts: None,
        }],
    )
    .await?;
    Ok(())
}

pub async fn write_and_load<P: Persistence>(p: Arc<P>) -> anyhow::Result<()> {
    let mut id_generator = TestIdGenerator::new();
    let table: TableName = str::parse("table")?;
    let doc_id = id_generator.user_generate(&table);

    let doc = ResolvedDocument::new(doc_id, CreationTime::ONE, ConvexObject::empty())?;

    p.write(
        vec![
            // Write doc
            DocumentLogEntry {
                ts: Timestamp::must(0),
                id: doc.id_with_table_id(),
                value: Some(doc.clone()),
                prev_ts: None,
            },
            // Delete doc
            DocumentLogEntry {
                ts: Timestamp::must(1),
                id: doc.id_with_table_id(),
                value: None,
                prev_ts: Some(Timestamp::must(0)),
            },
        ],
        BTreeSet::new(),
        ConflictStrategy::Error,
    )
    .await?;
    id_generator.write_tables(p.clone()).await?;

    // Equivalent of load_all_documents.
    test_load_documents(
        &p,
        &id_generator,
        TimestampRange::all(),
        Order::Asc,
        vec![
            DocumentLogEntry {
                ts: Timestamp::must(0),
                id: doc.id_with_table_id(),
                value: Some(doc.clone()),
                prev_ts: None,
            },
            DocumentLogEntry {
                ts: Timestamp::must(1),
                id: doc.id_with_table_id(),
                value: None,
                prev_ts: Some(Timestamp::must(0)),
            },
        ],
    )
    .await?;
    // Pattern used when updating shape.
    test_load_documents(
        &p,
        &id_generator,
        TimestampRange::new(Timestamp::must(1)..)?,
        Order::Asc,
        vec![DocumentLogEntry {
            ts: Timestamp::must(1),
            id: doc.id_with_table_id(),
            value: None,
            prev_ts: Some(Timestamp::must(0)),
        }],
    )
    .await?;
    // Pattern used when bootstrapping index.
    test_load_documents(
        &p,
        &id_generator,
        TimestampRange::at(Timestamp::MIN),
        Order::Asc,
        vec![DocumentLogEntry {
            ts: Timestamp::MIN,
            id: doc.id_with_table_id(),
            value: Some(doc.clone()),
            prev_ts: None,
        }],
    )
    .await?;
    // Pattern used when backfilling index.
    test_load_documents(
        &p,
        &id_generator,
        TimestampRange::new(..Timestamp::must(2))?,
        Order::Desc,
        vec![
            DocumentLogEntry {
                ts: Timestamp::must(1),
                id: doc.id_with_table_id(),
                value: None,
                prev_ts: Some(Timestamp::must(0)),
            },
            DocumentLogEntry {
                ts: Timestamp::must(0),
                id: doc.id_with_table_id(),
                value: Some(doc.clone()),
                prev_ts: None,
            },
        ],
    )
    .await?;

    Ok(())
}

pub async fn write_and_load_value_types<P: Persistence>(p: Arc<P>) -> anyhow::Result<()> {
    let mut id_generator = TestIdGenerator::new();
    let table: TableName = str::parse("table")?;
    let mut next_ts = Timestamp::MIN;
    let new_doc = |value| {
        let id = id_generator.user_generate(&table);
        let doc = ResolvedDocument::new(id, CreationTime::ONE, assert_obj!("field" => value))?;
        let r = DocumentLogEntry {
            ts: next_ts,
            id: doc.id_with_table_id(),
            value: Some(doc),
            prev_ts: None,
        };
        next_ts = next_ts.succ()?;
        Ok(r)
    };
    let values = vec![
        ConvexValue::Null,
        ConvexValue::Int64(-1),
        ConvexValue::Float64(f64::NEG_INFINITY),
        ConvexValue::Float64(f64::MIN),
        ConvexValue::Float64(-0.),
        ConvexValue::Float64(0.),
        ConvexValue::Float64(f64::MIN_POSITIVE),
        ConvexValue::Float64(f64::MAX),
        ConvexValue::Float64(f64::INFINITY),
        ConvexValue::Float64(f64::NAN),
        ConvexValue::Boolean(true),
        ConvexValue::String("".try_into()?),
        ConvexValue::String("\x00".try_into()?),
        ConvexValue::String("\u{10348}".try_into()?),
        ConvexValue::Bytes(vec![].try_into()?),
        ConvexValue::Bytes(vec![3, 3, 4, 4].try_into()?),
        ConvexValue::Bytes(vec![0; (1 << 24) - 10000].try_into()?),
        ConvexValue::Array(vec![ConvexValue::Null].try_into()?),
        ConvexValue::Set(btreeset!(ConvexValue::Null).try_into()?),
        ConvexValue::Map(btreemap!(ConvexValue::Null => ConvexValue::Null).try_into()?),
        ConvexValue::Object(assert_obj!("nested" => ConvexValue::Null)),
    ];
    let updates = values
        .into_iter()
        .map(new_doc)
        .collect::<anyhow::Result<Vec<_>>>()?;

    p.write(updates.clone(), BTreeSet::new(), ConflictStrategy::Error)
        .await?;
    id_generator.write_tables(p.clone()).await?;

    test_load_documents(
        &p,
        &id_generator,
        TimestampRange::all(),
        Order::Asc,
        updates,
    )
    .await?;

    Ok(())
}

pub async fn overwrite_document<P: Persistence>(p: Arc<P>) -> anyhow::Result<()> {
    let mut id_generator = TestIdGenerator::new();
    let table: TableName = str::parse("table")?;
    let doc_id = id_generator.user_generate(&table);

    let doc = ResolvedDocument::new(doc_id, CreationTime::ONE, ConvexObject::empty())?;

    p.write(
        vec![
            // Write doc
            DocumentLogEntry {
                ts: Timestamp::must(0),
                id: doc.id_with_table_id(),
                value: Some(doc.clone()),
                prev_ts: None,
            },
            // Delete doc
            DocumentLogEntry {
                ts: Timestamp::must(1),
                id: doc.id_with_table_id(),
                value: None,
                prev_ts: Some(Timestamp::must(0)),
            },
        ],
        BTreeSet::new(),
        ConflictStrategy::Error,
    )
    .await?;

    // Try to overwrite the original write at ts 0 -- should fail.
    let err = p
        .write(
            vec![DocumentLogEntry {
                ts: Timestamp::must(0),
                id: doc.id_with_table_id(),
                value: Some(doc.clone()),
                prev_ts: None,
            }],
            BTreeSet::new(),
            ConflictStrategy::Error,
        )
        .await
        .unwrap_err()
        .to_string();
    assert!(err.contains("constraint") || err.contains("Duplicate entry"));

    // With ConflictStrategy::Overwrite the write succeeds.
    p.write(
        vec![DocumentLogEntry {
            ts: Timestamp::must(0),
            id: doc.id_with_table_id(),
            value: Some(doc.clone()),
            prev_ts: None,
        }],
        BTreeSet::new(),
        ConflictStrategy::Overwrite,
    )
    .await?;

    Ok(())
}

pub async fn overwrite_index<P: Persistence>(p: Arc<P>) -> anyhow::Result<()> {
    let mut id_generator = TestIdGenerator::new();
    let index_id = id_generator.system_generate(&INDEX_TABLE);
    let ts = Timestamp::must(1);
    let table: TableName = str::parse("table")?;
    let doc_id = id_generator.user_generate(&table);
    let tablet_id = doc_id.tablet_id;
    let value = val!(testing::generate::<Vec<u8>>());

    let doc = ResolvedDocument::new(
        doc_id,
        CreationTime::ONE,
        assert_obj!("value" => value.clone()),
    )?;
    let fields: IndexedFields = vec!["value".parse()?].try_into()?;
    let key = doc.index_key(&fields, p.reader().version());
    let index_update = DatabaseIndexUpdate {
        index_id: index_id.internal_id(),
        key: key.clone(),
        value: DatabaseIndexValue::NonClustered(doc.id()),
        is_system_index: false,
    };
    p.write(
        vec![DocumentLogEntry {
            ts,
            id: doc.id_with_table_id(),
            value: Some(doc.clone()),
            prev_ts: None,
        }],
        btreeset!((ts, index_update.clone())),
        ConflictStrategy::Error,
    )
    .await?;
    // Writing at the same ts with `ConflictStrategy::Error` should fail.
    let err = p
        .write(
            vec![],
            btreeset!((ts, index_update.clone())),
            ConflictStrategy::Error,
        )
        .await
        .unwrap_err()
        .to_string();
    assert!(err.contains("constraint") || err.contains("Duplicate entry"));
    // Writing with `ConflictStrategy::Overwrite` should succeed.
    p.write(
        vec![],
        btreeset!((
            ts,
            DatabaseIndexUpdate {
                index_id: index_id.internal_id(),
                key: key.clone(),
                value: DatabaseIndexValue::Deleted,
                is_system_index: false,
            },
        )),
        ConflictStrategy::Overwrite,
    )
    .await?;
    // Confirm the index was overwritten.
    let results = p
        .reader()
        .index_scan(
            index_id.internal_id(),
            tablet_id,
            ts,
            &Interval::all(),
            Order::Asc,
            1,
            Arc::new(NoopRetentionValidator),
        )
        .map(|r| match r {
            Ok(ik) => ik,
            Err(err) => panic!("Error: {}", err),
        })
        .collect::<Vec<_>>()
        .await;
    assert!(results.is_empty());
    Ok(())
}

pub async fn test_load_documents_from_table<P: Persistence>(
    p: &Arc<P>,
    tablet_id: TabletId,
    range: TimestampRange,
    order: Order,
    expected: Vec<DocumentLogEntry>,
) -> anyhow::Result<()> {
    for page_size in 1..3 {
        let docs: Vec<_> = p
            .reader()
            .load_documents_from_table(
                tablet_id,
                range,
                order,
                page_size,
                Arc::new(NoopRetentionValidator),
            )
            .try_collect()
            .await?;
        let docs: Vec<_> = docs.into_iter().collect();
        assert_eq!(docs, expected);
    }
    Ok(())
}

pub async fn test_load_documents<P: Persistence>(
    p: &Arc<P>,
    table_mapping: &TableMapping,
    range: TimestampRange,
    order: Order,
    expected: Vec<DocumentLogEntry>,
) -> anyhow::Result<()> {
    let docs: Vec<_> = p
        .reader()
        .load_documents(range, order, 10, Arc::new(NoopRetentionValidator))
        .try_collect()
        .await?;
    let docs: Vec<_> = docs
        .into_iter()
        .filter(|entry| !table_mapping.is_system_tablet(entry.id.table()))
        .collect();
    assert_eq!(docs, expected);
    Ok(())
}

pub async fn write_and_load_sorting<P: Persistence>(p: Arc<P>) -> anyhow::Result<()> {
    let table1: TableName = str::parse("table1")?;
    let table2: TableName = str::parse("table2")?;
    let mut id_generator = TestIdGenerator::new();

    let doc_id1 = id_generator.user_generate(&table1);
    let doc_id2 = id_generator.user_generate(&table2);

    let doc1 = ResolvedDocument::new(doc_id1, CreationTime::ONE, ConvexObject::empty())?;
    let doc2 = ResolvedDocument::new(doc_id2, CreationTime::ONE, ConvexObject::empty())?;
    p.write(
        vec![
            // Write doc1 and doc2. Make sure sorted by TS, not ID
            DocumentLogEntry {
                ts: Timestamp::must(1),
                id: doc1.id_with_table_id(),
                value: Some(doc1.clone()),
                prev_ts: None,
            },
            DocumentLogEntry {
                ts: Timestamp::must(0),
                id: doc2.id_with_table_id(),
                value: Some(doc2.clone()),
                prev_ts: None,
            },
        ],
        BTreeSet::new(),
        ConflictStrategy::Error,
    )
    .await?;
    id_generator.write_tables(p.clone()).await?;

    let docs: Vec<_> = p.reader().load_all_documents().try_collect().await?;
    let docs: Vec<_> = docs
        .into_iter()
        .filter(|entry| !id_generator.is_system_tablet(entry.id.table()))
        .collect();
    assert_eq!(
        docs,
        vec![
            DocumentLogEntry {
                ts: Timestamp::must(0),
                id: doc2.id_with_table_id(),
                value: Some(doc2),
                prev_ts: None,
            },
            DocumentLogEntry {
                ts: Timestamp::must(1),
                id: doc1.id_with_table_id(),
                value: Some(doc1),
                prev_ts: None,
            },
        ]
    );

    Ok(())
}

pub async fn same_internal_id_multiple_tables<P: Persistence>(p: Arc<P>) -> anyhow::Result<()> {
    let mut id_generator = TestIdGenerator::new();

    // Create two documents with the same internal_id but in two different tables.
    let internal_id = id_generator.generate_internal();

    let table1_id = id_generator.user_table_id(&str::parse("table1")?);
    let table2_id = id_generator.user_table_id(&str::parse("table2")?);

    let doc1 = ResolvedDocument::new(
        ResolvedDocumentId::new(
            table1_id.tablet_id,
            DeveloperDocumentId::new(table1_id.table_number, internal_id),
        ),
        CreationTime::ONE,
        assert_obj!("value" => 1),
    )?;
    let doc2 = ResolvedDocument::new(
        ResolvedDocumentId::new(
            table2_id.tablet_id,
            DeveloperDocumentId::new(table2_id.table_number, internal_id),
        ),
        CreationTime::ONE,
        assert_obj!("value" => 2),
    )?;

    // Have an index pointing to each document.
    let index_fields: IndexedFields = vec!["value".parse()?].try_into()?;
    let index1_id = id_generator.system_generate(&INDEX_TABLE).internal_id();
    let index2_id = id_generator.system_generate(&INDEX_TABLE).internal_id();

    let ts = Timestamp::must(1000);
    p.write(
        vec![
            // Write doc1 and doc2. Make sure sorted by TS, not ID
            DocumentLogEntry {
                ts,
                id: doc1.id_with_table_id(),
                value: Some(doc1.clone()),
                prev_ts: None,
            },
            DocumentLogEntry {
                ts,
                id: doc2.id_with_table_id(),
                value: Some(doc2.clone()),
                prev_ts: None,
            },
        ],
        btreeset!(
            (
                ts,
                DatabaseIndexUpdate {
                    index_id: index1_id,
                    key: doc1.index_key(&index_fields, p.reader().version()),
                    value: DatabaseIndexValue::NonClustered(doc1.id()),
                    is_system_index: false,
                }
            ),
            (
                ts,
                DatabaseIndexUpdate {
                    index_id: index2_id,
                    key: doc1.index_key(&index_fields, p.reader().version()),
                    value: DatabaseIndexValue::NonClustered(doc2.id()),
                    is_system_index: false,
                }
            )
        ),
        ConflictStrategy::Error,
    )
    .await?;

    // Query index1 should give us the first document.
    let results = p
        .reader()
        .index_scan(
            index1_id,
            table1_id.tablet_id,
            ts,
            &Interval::all(),
            Order::Asc,
            100,
            Arc::new(NoopRetentionValidator),
        )
        .map(|r| match r {
            Ok(ik) => ik,
            Err(err) => panic!("Error: {:?}", err),
        })
        .collect::<Vec<_>>()
        .await;
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].1.value, doc1);

    // Query index2 should give us the second document.
    let results = p
        .reader()
        .index_scan(
            index2_id,
            table2_id.tablet_id,
            ts,
            &Interval::all(),
            Order::Asc,
            100,
            Arc::new(NoopRetentionValidator),
        )
        .map(|r| match r {
            Ok(ik) => ik,
            Err(err) => panic!("Error: {:?}", err),
        })
        .collect::<Vec<_>>()
        .await;
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].1.value, doc2);

    Ok(())
}

pub async fn query_index_at_ts<P: Persistence>(p: Arc<P>) -> anyhow::Result<()> {
    let mut id_generator = TestIdGenerator::new();
    let index_id = id_generator.system_generate(&INDEX_TABLE).internal_id();

    let table: TableName = str::parse("table")?;
    let doc_id = id_generator.user_generate(&table);
    let tablet_id = doc_id.tablet_id;

    let mut ts_to_value: BTreeMap<Timestamp, ConvexValue> = BTreeMap::new();
    for ts in 0..=100 {
        let bytes = testing::generate::<Vec<u8>>();
        ts_to_value.insert(Timestamp::must(ts), bytes.try_into()?);
    }

    let fields: IndexedFields = vec!["value".parse()?].try_into()?;
    let mut old_key: Option<IndexKey> = None;
    for (ts, value) in ts_to_value.iter() {
        let doc = ResolvedDocument::new(
            doc_id,
            CreationTime::ONE,
            assert_obj!("value" => value.clone()),
        )?;
        let key = doc.index_key(&fields, p.reader().version());
        let mut index_updates = vec![DatabaseIndexUpdate {
            index_id,
            key: key.clone(),
            value: DatabaseIndexValue::NonClustered(doc_id),
            is_system_index: false,
        }];
        if let Some(old_key) = old_key {
            if old_key != key {
                index_updates.push(DatabaseIndexUpdate {
                    index_id,
                    key: old_key,
                    value: DatabaseIndexValue::Deleted,
                    is_system_index: false,
                })
            }
        }
        p.write(
            vec![DocumentLogEntry {
                ts: *ts,
                id: doc.id_with_table_id(),
                value: Some(doc),
                prev_ts: None,
            }],
            index_updates.into_iter().map(|u| (*ts, u)).collect(),
            ConflictStrategy::Error,
        )
        .await?;
        old_key = Some(key);
    }
    id_generator.write_tables(p.clone()).await?;

    // Query at the each timestamp should returns the expected result.
    for (ts, expected_value) in ts_to_value.into_iter() {
        let results = p
            .reader()
            .index_scan(
                index_id,
                tablet_id,
                ts,
                &Interval::all(),
                Order::Asc,
                100,
                Arc::new(NoopRetentionValidator),
            )
            .map(|r| match r {
                Ok(ik) => ik,
                Err(err) => panic!("Error: {:?}", err),
            })
            .collect::<Vec<_>>()
            .await;
        let doc = ResolvedDocument::new(
            doc_id,
            CreationTime::ONE,
            assert_obj!("value" => expected_value),
        )?;
        let key = doc.index_key(&fields, p.reader().version()).to_bytes();
        assert_eq!(
            results,
            vec![(
                key,
                LatestDocument {
                    ts,
                    value: doc,
                    prev_ts: None
                }
            )]
        );
    }

    Ok(())
}

// Test varies ranges where all generated keys start with the given prefix.
pub async fn query_index_range_with_prefix<P: Persistence>(
    p: Arc<P>,
    prefix: Vec<u8>,
) -> anyhow::Result<()> {
    let mut id_generator = TestIdGenerator::new();
    let index_id = id_generator.system_generate(&INDEX_TABLE).internal_id();

    let table: TableName = str::parse("table")?;
    let tablet_id = id_generator.user_table_id(&table).tablet_id;
    let fields: IndexedFields = vec!["value".parse()?].try_into()?;
    let ts = Timestamp::must(1702);

    let mut documents = Vec::new();
    let mut indexes = BTreeSet::new();
    let mut keys = Vec::new();
    let mut keys_to_doc = BTreeMap::new();
    for _ in 0..10 {
        let mut value = prefix.clone();
        value.extend(testing::generate::<Vec<u8>>());
        let value: ConvexValue = value.try_into()?;

        let doc_id = id_generator.user_generate(&table);
        let doc = ResolvedDocument::new(doc_id, CreationTime::ONE, assert_obj!("value" => value))?;
        documents.push(DocumentLogEntry {
            ts,
            id: doc.id_with_table_id(),
            value: Some(doc.clone()),
            prev_ts: None,
        });
        let key = doc.index_key(&fields, p.reader().version());
        keys.push(key.clone());
        keys_to_doc.insert(key.clone(), doc.clone());
        indexes.insert((
            ts,
            DatabaseIndexUpdate {
                index_id,
                key,
                value: DatabaseIndexValue::NonClustered(doc_id),
                is_system_index: false,
            },
        ));
    }

    p.write(documents, indexes, ConflictStrategy::Error).await?;
    id_generator.write_tables(p.clone()).await?;

    keys.sort();
    for i in 0..keys.len() {
        for j in 0..keys.len() {
            for order in [Order::Asc, Order::Desc] {
                let results = p
                    .reader()
                    .index_scan(
                        index_id,
                        tablet_id,
                        ts,
                        &Interval {
                            start: StartIncluded(keys[i].to_bytes().into()),
                            end: End::after_prefix(&BinaryKey::from(keys[j].to_bytes())),
                        },
                        order,
                        100,
                        Arc::new(NoopRetentionValidator),
                    )
                    .map(|r| match r {
                        Ok(ik) => ik,
                        Err(err) => panic!("Error: {}", err),
                    })
                    .collect::<Vec<_>>()
                    .await;

                let mut expected_keys = keys[i..max(i, j + 1)].to_vec();
                match order {
                    Order::Asc => (),
                    Order::Desc => {
                        expected_keys.reverse();
                    },
                };
                let expected: Vec<_> = expected_keys
                    .into_iter()
                    .map(|k| {
                        (
                            k.to_bytes(),
                            LatestDocument {
                                ts,
                                value: keys_to_doc.get(&k).unwrap().clone(),
                                prev_ts: None,
                            },
                        )
                    })
                    .collect();
                assert_eq!(results, expected);
            }
        }
    }

    Ok(())
}

// Test without prefix.
pub async fn query_index_range_short<P: Persistence>(p: Arc<P>) -> anyhow::Result<()> {
    query_index_range_with_prefix(p, Vec::new()).await
}

// Test by prefixing all keys with the same long prefix.
pub async fn query_index_range_long<P: Persistence>(p: Arc<P>) -> anyhow::Result<()> {
    let long_prefix = testing::generate_with::<Vec<u8>>(size_range(10000).lift());
    query_index_range_with_prefix(p, long_prefix).await
}

// Make sure we correctly filter using the index_id.
pub async fn query_multiple_indexes<P: Persistence>(p: Arc<P>) -> anyhow::Result<()> {
    let table: TableName = str::parse("table")?;
    let ts = Timestamp::must(1702);

    let mut id_generator = TestIdGenerator::new();
    let tablet_id = id_generator.user_table_id(&table).tablet_id;
    let mut documents = Vec::new();
    let mut indexes = BTreeSet::new();
    let mut index_to_results: BTreeMap<_, Vec<_>> = BTreeMap::new();
    for i in 0..5 {
        let index_id = id_generator.system_generate(&INDEX_TABLE).internal_id();
        let fields: IndexedFields = vec![format!("value_{}", i).parse()?].try_into()?;

        for j in 0..5 {
            let doc_id = id_generator.user_generate(&table);
            let doc = ResolvedDocument::new(
                doc_id,
                CreationTime::ONE,
                assert_obj!(
                    format!("value_{}", i) => j
                ),
            )?;
            let key = doc.index_key(&fields, p.reader().version());
            documents.push(DocumentLogEntry {
                ts,
                id: doc.id_with_table_id(),
                value: Some(doc.clone()),
                prev_ts: None,
            });
            indexes.insert((
                ts,
                DatabaseIndexUpdate {
                    index_id,
                    key: key.clone(),
                    value: DatabaseIndexValue::NonClustered(doc_id),
                    is_system_index: false,
                },
            ));
            index_to_results.entry(index_id).or_default().push((
                key.to_bytes(),
                LatestDocument {
                    ts,
                    value: doc,
                    prev_ts: None,
                },
            ));
        }
    }

    p.write(documents, indexes, ConflictStrategy::Error).await?;
    id_generator.write_tables(p.clone()).await?;

    for (index_id, expected) in index_to_results {
        let keys = p
            .reader()
            .index_scan(
                index_id,
                tablet_id,
                ts,
                &Interval::all(),
                Order::Asc,
                100,
                Arc::new(NoopRetentionValidator),
            )
            .map(|r| match r {
                Ok(ik) => ik,
                Err(err) => panic!("Error: {}", err),
            })
            .collect::<Vec<_>>()
            .await;

        assert_eq!(keys, expected);
    }

    Ok(())
}

// Write an index without the doc itself. Querying should fail.
pub async fn query_dangling_reference<P: Persistence>(p: Arc<P>) -> anyhow::Result<()> {
    let table: TableName = str::parse("table")?;
    let ts = Timestamp::must(1702);
    let mut id_generator = TestIdGenerator::new();

    let tablet_id = id_generator.user_table_id(&table).tablet_id;

    let index_id = id_generator.system_generate(&INDEX_TABLE).internal_id();

    let index_fields: IndexedFields = vec!["value".parse()?].try_into()?;
    let doc_id = id_generator.user_generate(&table);
    let document = ResolvedDocument::new(doc_id, CreationTime::ONE, assert_obj!("value" => 20))?;
    let index_update = DatabaseIndexUpdate {
        index_id,
        key: document.index_key(&index_fields, p.reader().version()),
        value: DatabaseIndexValue::NonClustered(document.id()),
        is_system_index: false,
    };

    // Note we don't write the document!
    p.write(
        vec![],
        btreeset!((ts, index_update)),
        ConflictStrategy::Error,
    )
    .await?;

    let results: Vec<_> = p
        .reader()
        .index_scan(
            index_id,
            tablet_id,
            ts,
            &Interval::all(),
            Order::Asc,
            100,
            Arc::new(NoopRetentionValidator),
        )
        .collect()
        .await;
    assert_eq!(results.len(), 1);
    assert!(results[0].is_err());
    assert!(format!("{:?}", results[0].as_ref().unwrap_err()).contains("Dangling index reference"));

    Ok(())
}

// Write an index pointing to a deleted doc. Querying should
// fail.
pub async fn query_reference_deleted_doc<P: Persistence>(p: Arc<P>) -> anyhow::Result<()> {
    let table: TableName = str::parse("table")?;
    let ts = Timestamp::must(1702);

    let mut id_generator = TestIdGenerator::new();

    let tablet_id = id_generator.user_table_id(&table).tablet_id;
    let index_id = id_generator.system_generate(&INDEX_TABLE).internal_id();

    let index_fields: IndexedFields = vec!["value".parse()?].try_into()?;
    let doc_id = id_generator.user_generate(&table);
    let document = ResolvedDocument::new(doc_id, CreationTime::ONE, assert_obj!("value" => 20))?;
    let index_update = DatabaseIndexUpdate {
        index_id,
        key: document.index_key(&index_fields, p.reader().version()),
        value: DatabaseIndexValue::NonClustered(document.id()),
        is_system_index: false,
    };

    // Note that we write a deleted document.
    p.write(
        vec![
            (DocumentLogEntry {
                ts,
                id: document.id_with_table_id(),
                value: None,
                prev_ts: None,
            }),
        ],
        btreeset!((ts, index_update)),
        ConflictStrategy::Error,
    )
    .await?;

    let results: Vec<_> = p
        .reader()
        .index_scan(
            index_id,
            tablet_id,
            ts,
            &Interval::all(),
            Order::Asc,
            100,
            Arc::new(NoopRetentionValidator),
        )
        .collect()
        .await;
    assert_eq!(results.len(), 1);
    assert!(results[0].is_err());
    assert!(format!("{:?}", results[0].as_ref().unwrap_err())
        .contains("Index reference to deleted document"));

    Ok(())
}

pub async fn query_with_rows_estimate_with_prefix<P: Persistence>(
    p: Arc<P>,
    prefix: Vec<u8>,
) -> anyhow::Result<()> {
    let table: TableName = str::parse("table")?;
    let ts = Timestamp::must(1702);

    let mut id_generator = TestIdGenerator::new();
    let index_id = id_generator.system_generate(&INDEX_TABLE).internal_id();
    let tablet_id = id_generator.user_table_id(&table).tablet_id;

    let index_fields: IndexedFields = vec!["value".parse()?].try_into()?;
    let mut documents = Vec::new();
    for i in 0..99 {
        let doc_id = id_generator.user_generate(&table);

        let mut value = prefix.clone();
        value.push(i as u8);

        let document =
            ResolvedDocument::new(doc_id, CreationTime::ONE, assert_obj!("value" => value))?;
        let index_update = DatabaseIndexUpdate {
            index_id,
            key: document.index_key(&index_fields, p.reader().version()),
            value: DatabaseIndexValue::NonClustered(document.id()),
            is_system_index: false,
        };
        p.write(
            vec![
                (DocumentLogEntry {
                    ts,
                    id: document.id_with_table_id(),
                    value: Some(document.clone()),
                    prev_ts: None,
                }),
            ],
            btreeset!((ts, index_update)),
            ConflictStrategy::Error,
        )
        .await?;
        documents.push(document);
    }
    id_generator.write_tables(p.clone()).await?;

    // We should get the same result regardless of the rows estimate.
    for rows_estimate in [1, 10, 20, 100] {
        let results: Vec<_> = p
            .reader()
            .index_scan(
                index_id,
                tablet_id,
                ts,
                &Interval::all(),
                Order::Asc,
                rows_estimate,
                Arc::new(NoopRetentionValidator),
            )
            .try_collect::<Vec<_>>()
            .await?
            .into_iter()
            .map(|(_, rev)| rev.value)
            .collect();
        assert_eq!(results, documents);
    }

    Ok(())
}

pub async fn query_with_rows_estimate_short<P: Persistence>(p: Arc<P>) -> anyhow::Result<()> {
    query_with_rows_estimate_with_prefix(p, Vec::new()).await
}

// Test by prefixing all keys with the same long prefix.
pub async fn query_with_rows_estimate_long<P: Persistence>(p: Arc<P>) -> anyhow::Result<()> {
    let long_prefix = testing::generate_with::<Vec<u8>>(size_range(10000).lift());
    query_with_rows_estimate_with_prefix(p, long_prefix).await
}

pub async fn write_then_read<F, Fut, P: Persistence>(mut make_p: F) -> anyhow::Result<()>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = anyhow::Result<Arc<P>>>,
{
    let p_write = make_p().await?;
    let table: TableName = str::parse("table")?;
    let mut id_generator = TestIdGenerator::new();
    let doc_id = id_generator.user_generate(&table);

    let doc = ResolvedDocument::new(doc_id, CreationTime::ONE, ConvexObject::empty())?;

    p_write
        .write(
            vec![
                (DocumentLogEntry {
                    ts: Timestamp::must(0),
                    id: doc.id_with_table_id(),
                    value: Some(doc.clone()),
                    prev_ts: None,
                }),
            ],
            BTreeSet::new(),
            ConflictStrategy::Error,
        )
        .await?;
    id_generator.write_tables(p_write.clone()).await?;
    drop(p_write);

    // Read from another persistence, e.g. on a replica.
    let p_read = make_p().await?;
    let reader = p_read.reader();
    let stream = reader.load_all_documents();
    let mut results: Vec<_> = stream.try_collect().await?;
    results.retain(|entry| !id_generator.is_system_tablet(entry.id.table()));
    assert_eq!(
        results,
        vec![DocumentLogEntry {
            ts: Timestamp::must(0),
            id: doc.id_with_table_id(),
            value: Some(doc.clone()),
            prev_ts: None,
        }],
    );

    Ok(())
}

pub async fn set_read_only<F, Fut, F1, Fut1, P: Persistence>(
    mut make_p: F,
    mut make_p_read_only: F1,
) -> anyhow::Result<()>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = anyhow::Result<P>>,
    F1: FnMut() -> Fut1,
    Fut1: Future<Output = anyhow::Result<P>>,
{
    // Initially not read-only.
    let p_backend1 = make_p().await?;
    let table: TableName = str::parse("table")?;
    let mut id_generator = TestIdGenerator::new();
    let doc_id = id_generator.user_generate(&table);

    let doc = ResolvedDocument::new(doc_id, CreationTime::ONE, ConvexObject::empty())?;

    p_backend1
        .write(
            vec![DocumentLogEntry {
                ts: Timestamp::must(0),
                id: doc.id_with_table_id(),
                value: Some(doc.clone()),
                prev_ts: None,
            }],
            BTreeSet::new(),
            ConflictStrategy::Error,
        )
        .await?;
    // Release the lease.
    drop(p_backend1);

    let p_migration = make_p().await?;
    p_migration.set_read_only(true).await?;

    let result = make_p().await;
    assert!(result.is_err());

    drop(p_migration);

    // Try to acquire lease should fail because it's read-only.
    let result = make_p().await;
    assert!(result.is_err());

    let p_cleanup = make_p_read_only().await?;
    p_cleanup.set_read_only(false).await?;
    drop(p_cleanup);

    // Now it's no longer read-only.
    let p_backend2 = make_p().await?;
    p_backend2
        .write(
            vec![
                (DocumentLogEntry {
                    ts: Timestamp::must(1),
                    id: doc.id_with_table_id(),
                    value: Some(doc.clone()),
                    prev_ts: None,
                }),
            ],
            BTreeSet::new(),
            ConflictStrategy::Error,
        )
        .await?;

    Ok(())
}

pub async fn persistence_global<P: Persistence>(p: Arc<P>) -> anyhow::Result<()> {
    let key = PersistenceGlobalKey::RetentionMinSnapshotTimestamp;
    p.write_persistence_global(key, json!(5)).await?;
    assert_eq!(
        p.reader().get_persistence_global(key).await?,
        Some(json!(5))
    );
    // New value overwrites.
    p.write_persistence_global(key, json!(8)).await?;
    assert_eq!(
        p.reader().get_persistence_global(key).await?,
        Some(json!(8))
    );
    // Deeply nested values should roundtrip.
    fn very_nested_json(depth: usize) -> serde_json::Value {
        if depth == 0 {
            json!("hi")
        } else {
            json!({"a": very_nested_json(depth-1)})
        }
    }
    let value = very_nested_json(257);
    p.write_persistence_global(key, value.clone()).await?;
    assert_eq!(p.reader().get_persistence_global(key).await?, Some(value));
    Ok(())
}

pub fn doc(
    id: ResolvedDocumentId,
    ts: i32,
    val: Option<i64>,
    prev_ts: Option<i32>,
) -> anyhow::Result<DocumentLogEntry> {
    let doc = val
        .map(|val| ResolvedDocument::new(id, CreationTime::ONE, assert_obj!("value" => val)))
        .transpose()?;
    Ok(DocumentLogEntry {
        ts: Timestamp::must(ts),
        id: id.into(),
        value: doc,
        prev_ts: prev_ts.map(Timestamp::must),
    })
}

pub async fn persistence_enforce_retention<P: Persistence>(p: Arc<P>) -> anyhow::Result<()> {
    let mut id_generator = TestIdGenerator::new();
    let by_id_index_id = id_generator.system_generate(&INDEX_TABLE).internal_id();
    let by_val_index_id = id_generator.system_generate(&INDEX_TABLE).internal_id();
    let table: TableName = str::parse("table")?;
    let tablet_id = id_generator.user_table_id(&table).tablet_id;

    let by_id = |id: ResolvedDocumentId,
                 ts: i32,
                 deleted: bool|
     -> anyhow::Result<(Timestamp, DatabaseIndexUpdate)> {
        let key = IndexKey::new(vec![], id.into());
        Ok((
            Timestamp::must(ts),
            DatabaseIndexUpdate {
                index_id: by_id_index_id,
                key,
                value: if deleted {
                    DatabaseIndexValue::Deleted
                } else {
                    DatabaseIndexValue::NonClustered(id)
                },
                is_system_index: false,
            },
        ))
    };

    let by_val = |id: ResolvedDocumentId,
                  ts: i32,
                  val: i64,
                  deleted: bool|
     -> anyhow::Result<(Timestamp, DatabaseIndexUpdate)> {
        let key = IndexKey::new(vec![assert_val!(val)], id.into());
        Ok((
            Timestamp::must(ts),
            DatabaseIndexUpdate {
                index_id: by_val_index_id,
                key,
                value: if deleted {
                    DatabaseIndexValue::Deleted
                } else {
                    DatabaseIndexValue::NonClustered(id)
                },
                is_system_index: false,
            },
        ))
    };

    let id1 = id_generator.user_generate(&table);
    let id2 = id_generator.user_generate(&table);
    let id3 = id_generator.user_generate(&table);
    let id4 = id_generator.user_generate(&table);
    let id5 = id_generator.user_generate(&table);

    let documents = vec![
        doc(id1, 1, Some(5), None)?,    // expired because overwritten.
        doc(id2, 2, Some(5), None)?,    // expired because overwritten.
        doc(id1, 3, Some(6), Some(1))?, // latest.
        doc(id2, 4, None, Some(2))?,    // expired because tombstone.
        doc(id3, 5, Some(5), None)?,    // latest.
        doc(id4, 6, Some(5), None)?,    // visible at min_snapshot_ts.
        doc(id5, 7, Some(5), None)?,    // visible at min_snapshot_ts.
        // min_snapshot_ts: 8
        doc(id4, 9, None, Some(6))?,
        doc(id5, 10, Some(6), Some(7))?,
        doc(id5, 11, Some(5), Some(10))?,
    ];
    // indexes derived from documents.
    let indexes = btreeset![
        by_id(id1, 1, false)?,     // expired because overwritten.
        by_val(id1, 1, 5, false)?, // expired because overwritten.
        by_id(id2, 2, false)?,     // expired because overwritten.
        by_val(id2, 2, 5, false)?, // expired because overwritten.
        by_id(id1, 3, false)?,
        by_val(id1, 3, 5, true)?, // expired because tombstone.
        by_val(id1, 3, 6, false)?,
        by_id(id2, 4, true)?,     // expired because tombstone.
        by_val(id2, 4, 5, true)?, // expired because tombstone.
        by_id(id3, 5, false)?,
        by_val(id3, 5, 5, false)?,
        by_id(id4, 6, false)?,
        by_val(id4, 6, 5, false)?,
        by_id(id5, 7, false)?,
        by_val(id5, 7, 5, false)?,
        // min_snapshot_ts: 8
        by_id(id4, 9, true)?,
        by_val(id4, 9, 5, true)?,
        by_id(id5, 10, false)?,
        by_val(id5, 10, 5, true)?,
        by_val(id5, 10, 6, false)?,
        by_id(id5, 11, false)?,
        by_val(id5, 11, 6, true)?,
        by_val(id5, 11, 5, false)?,
    ];

    p.write(documents, indexes, ConflictStrategy::Error).await?;
    // Writes 3 tables (_tables, _index, table) with index entries.
    let tables_count = 3;
    id_generator.write_tables(p.clone()).await?;

    // Check load_index_chunk pagination.
    let mut index_entries = Vec::new();
    let mut cursor = None;
    loop {
        let index_chunk = p.load_index_chunk(cursor, 3).await?;
        assert!(index_chunk.len() <= 3);
        cursor = if index_chunk.len() < 3 {
            None
        } else {
            index_chunk.last().cloned()
        };
        index_entries.extend(index_chunk);
        if cursor.is_none() {
            break;
        }
    }
    assert_eq!(index_entries.len(), 23 + tables_count);

    let min_snapshot_ts = Timestamp::must(8);

    let mut expired = Vec::new();
    for (row, next_row) in index_entries.iter().tuple_windows() {
        if row.is_expired(min_snapshot_ts, Some(next_row))? {
            expired.push(row.clone());
        }
    }
    let last_is_expired = index_entries
        .last()
        .unwrap()
        .is_expired(min_snapshot_ts, None)?;
    assert!(!last_is_expired);
    assert_eq!(expired.len(), 7);
    assert_eq!(p.delete_index_entries(expired).await?, 7);

    let reader = p.reader();

    // All documents are still visible at snapshot ts=8.
    let stream = reader.index_scan(
        by_val_index_id,
        tablet_id,
        Timestamp::must(8),
        &Interval::all(),
        Order::Asc,
        1,
        Arc::new(NoopRetentionValidator),
    );
    let results: Vec<_> = stream
        .try_collect::<Vec<_>>()
        .await?
        .into_iter()
        .map(|(_, rev)| (rev.value.id(), i64::from(rev.ts)))
        .collect();
    assert_eq!(results, vec![(id3, 5), (id4, 6), (id5, 7), (id1, 3)]);

    // Old versions of documents at snapshot ts=2 are not visible.
    let stream = reader.index_scan(
        by_val_index_id,
        tablet_id,
        Timestamp::must(2),
        &Interval::all(),
        Order::Asc,
        1,
        Arc::new(NoopRetentionValidator),
    );
    let results: Vec<_> = stream.try_collect::<Vec<_>>().await?;
    assert_eq!(results, vec![]);

    Ok(())
}

pub async fn persistence_delete_documents<P: Persistence>(p: Arc<P>) -> anyhow::Result<()> {
    let mut id_generator = TestIdGenerator::new();
    let table: TableName = str::parse("table")?;

    let id1 = id_generator.user_generate(&table);
    let id2 = id_generator.user_generate(&table);
    let id3 = id_generator.user_generate(&table);
    let id4 = id_generator.user_generate(&table);
    let id5 = id_generator.user_generate(&table);
    let id6 = id_generator.user_generate(&table);
    let id7 = id_generator.user_generate(&table);
    let id8 = id_generator.user_generate(&table);
    let id9 = id_generator.user_generate(&table);
    let id10 = id_generator.user_generate(&table);

    let documents = vec![
        doc(id1, 1, Some(1), None)?,
        doc(id2, 2, Some(2), None)?,
        doc(id3, 3, Some(3), None)?,
        // min_document_snapshot_ts: 4
        doc(id4, 5, Some(4), None)?,
        doc(id5, 6, Some(5), None)?,
        doc(id6, 7, Some(6), None)?,
        doc(id7, 8, Some(7), None)?,
        doc(id8, 9, Some(8), None)?,
        doc(id9, 10, Some(9), None)?,
        doc(id10, 11, Some(10), None)?,
    ];

    p.write(documents.clone(), BTreeSet::new(), ConflictStrategy::Error)
        .await?;

    let reader = p.reader();

    let stream = reader.load_all_documents();
    pin_mut!(stream);
    let all_docs = stream.try_collect::<Vec<_>>().await?;
    assert_eq!(documents, all_docs);

    let docs_to_delete = documents[..3]
        .iter()
        .map(|update| (update.ts, update.id))
        .collect_vec();

    assert_eq!(p.delete(docs_to_delete).await?, 3);

    let stream = reader.load_all_documents();
    pin_mut!(stream);
    let mut all_docs = Vec::new();
    while let Some(val) = stream.try_next().await? {
        all_docs.push(val);
    }
    assert_eq!(&documents[3..], &all_docs);

    Ok(())
}

pub async fn persistence_previous_revisions_of_documents<P: Persistence>(
    p: Arc<P>,
) -> anyhow::Result<()> {
    let mut id_generator = TestIdGenerator::new();
    let table: TableName = str::parse("table")?;
    let id1 = id_generator.user_generate(&table);
    let id2 = id_generator.user_generate(&table);
    let id3 = id_generator.user_generate(&table);

    let doc = |id: ResolvedDocumentId| {
        ResolvedDocument::new(id, CreationTime::ONE, assert_obj!("field" => id)).unwrap()
    };

    // Create three documents at timestamp 1
    let writes = vec![id1, id2, id3]
        .iter()
        .map(|&id| DocumentLogEntry {
            ts: Timestamp::must(1),
            id: id.into(),
            value: Some(doc(id)),
            prev_ts: None,
        })
        .collect();
    p.write(writes, BTreeSet::new(), ConflictStrategy::Error)
        .await?;

    // Delete id2 at timestamp 2
    let writes = vec![DocumentLogEntry {
        ts: Timestamp::must(2),
        id: id2.into(),
        value: None,
        prev_ts: Some(Timestamp::must(1)),
    }];
    p.write(writes, BTreeSet::new(), ConflictStrategy::Error)
        .await?;

    // Update id1 at timestamp 3
    let writes = vec![DocumentLogEntry {
        ts: Timestamp::must(3),
        id: id1.into(),
        value: Some(doc(id1)),
        prev_ts: Some(Timestamp::must(1)),
    }];
    p.write(writes, BTreeSet::new(), ConflictStrategy::Error)
        .await?;

    // Query various timestamps
    let nonexistent_id = InternalDocumentId::new(
        TabletId(id_generator.generate_internal()),
        id_generator.generate_internal(),
    );

    // For the purposes of testing, set `ts` to be anything, because only `prev_ts`
    // is used.
    let queries = btreeset![
        // Latest revision
        DocumentPrevTsQuery {
            id: id1.into(),
            ts: Timestamp::must(4),
            prev_ts: Timestamp::must(3),
        },
        // Previous revision of latest revision
        DocumentPrevTsQuery {
            id: id1.into(),
            ts: Timestamp::must(3),
            prev_ts: Timestamp::must(1)
        },
        // Tombstone (in this case ts doesn't actually exist but it's fine)
        DocumentPrevTsQuery {
            id: id2.into(),
            ts: Timestamp::must(3),
            prev_ts: Timestamp::must(2)
        },
        // Nonexistent revision at both ts and prev_ts
        DocumentPrevTsQuery {
            id: id2.into(),
            ts: Timestamp::must(4),
            prev_ts: Timestamp::must(3)
        },
        // Unchanged document
        DocumentPrevTsQuery {
            id: id3.into(),
            ts: Timestamp::must(2),
            prev_ts: Timestamp::must(1),
        },
        // Nonexistent document
        DocumentPrevTsQuery {
            id: nonexistent_id,
            ts: Timestamp::must(2),
            prev_ts: Timestamp::must(1),
        },
    ];

    // Test with NoopRetentionValidator
    // Note: Proper retention validation testing will be added in a separate PR
    let results = p
        .reader()
        .previous_revisions_of_documents(queries.clone(), Arc::new(NoopRetentionValidator))
        .await?;

    // Should get exact matches only
    assert_eq!(results.len(), 4); // id1@3, id1@1, id2@2, id3@1
    assert!(results.contains_key(&DocumentPrevTsQuery {
        id: id1.into(),
        ts: Timestamp::must(3),
        prev_ts: Timestamp::must(1)
    }));
    assert!(results.contains_key(&DocumentPrevTsQuery {
        id: id2.into(),
        ts: Timestamp::must(3),
        prev_ts: Timestamp::must(2)
    }));
    assert!(results.contains_key(&DocumentPrevTsQuery {
        id: id3.into(),
        ts: Timestamp::must(2),
        prev_ts: Timestamp::must(1)
    }));

    // Verify document contents
    let id1_at_3 = results
        .get(&DocumentPrevTsQuery {
            id: id1.into(),
            ts: Timestamp::must(4),
            prev_ts: Timestamp::must(3),
        })
        .unwrap();
    let id1_at_1 = results
        .get(&DocumentPrevTsQuery {
            id: id1.into(),
            ts: Timestamp::must(3),
            prev_ts: Timestamp::must(1),
        })
        .unwrap();

    // Verify id1@3 has the correct document and prev_ts pointing to id1@1
    assert_eq!(id1_at_3.value, Some(doc(id1)));
    assert_eq!(id1_at_3.prev_ts, Some(Timestamp::must(1)));

    // Verify id1@1 has the correct document and no prev_ts (it's the first version)
    assert_eq!(id1_at_1.value, Some(doc(id1)));
    assert_eq!(id1_at_1.prev_ts, None);

    // Verify id1@1 and id1@3 are different versions
    assert_ne!(id1_at_1.prev_ts, id1_at_3.prev_ts);

    // Verify tombstone
    assert_eq!(
        results
            .get(&DocumentPrevTsQuery {
                id: id2.into(),
                ts: Timestamp::must(3),
                prev_ts: Timestamp::must(2)
            })
            .unwrap()
            .value,
        None
    );

    let retention_validator = FakeRetentionValidator::new(Timestamp::must(4), Timestamp::must(0));
    // Min ts queried is 1, and min_document_ts is 0, so it's a valid query.
    p.reader()
        .previous_revisions_of_documents(queries.clone(), Arc::new(retention_validator))
        .await?;

    let retention_validator = FakeRetentionValidator::new(Timestamp::must(4), Timestamp::must(4));
    // Min ts queried is 1, and min_document_ts is 4, so it's an invalid query.
    assert!(p
        .reader()
        .previous_revisions_of_documents(queries, Arc::new(retention_validator))
        .await
        .is_err());
    // Errors even if there is no document at the timestamp.
    assert!(p
        .reader()
        .previous_revisions_of_documents(
            btreeset![DocumentPrevTsQuery {
                id: nonexistent_id,
                ts: Timestamp::must(1),
                prev_ts: Timestamp::must(1)
            }],
            Arc::new(retention_validator)
        )
        .await
        .is_err());

    Ok(())
}

pub async fn persistence_previous_revisions<P: Persistence>(p: Arc<P>) -> anyhow::Result<()> {
    let reader = p.reader();

    let table: TableName = str::parse("table")?;
    let mut id_generator = TestIdGenerator::new();
    let id1 = id_generator.user_generate(&table);
    let id2 = id_generator.user_generate(&table);
    let id3 = id_generator.user_generate(&table);
    let id4 = id_generator.user_generate(&table);
    let id5 = id_generator.user_generate(&table);
    let id6 = id_generator.user_generate(&table);
    let id7 = id_generator.user_generate(&table);
    let id8 = id_generator.user_generate(&table);
    let id9 = id_generator.user_generate(&table);
    let id10 = id_generator.user_generate(&table);
    let id11 = id_generator.user_generate(&table);
    let id12 = id_generator.user_generate(&table);

    let doc = |id: ResolvedDocumentId| {
        ResolvedDocument::new(id, CreationTime::ONE, assert_obj!("field" => id)).unwrap()
    };

    // Create eight documents at timestamp 1.
    let writes = vec![id1, id2, id3, id4, id5, id6, id7, id8]
        .iter()
        .map(|&id| DocumentLogEntry {
            ts: Timestamp::must(1),
            id: id.into(),
            value: Some(doc(id)),
            prev_ts: None,
        })
        .collect();
    p.write(writes, BTreeSet::new(), ConflictStrategy::Error)
        .await?;

    // Delete four of them at timestamp 2.
    let writes = [id2, id3, id4, id5]
        .iter()
        .map(|&id| DocumentLogEntry {
            ts: Timestamp::must(2),
            id: id.into(),
            value: None,
            prev_ts: None,
        })
        .collect();
    p.write(writes, BTreeSet::new(), ConflictStrategy::Error)
        .await?;
    id_generator.write_tables(p.clone()).await?;

    // Query the eight documents + four nonexistent IDs at timestamp 3.
    let queries = vec![
        id1, id2, id3, id4, id5, id6, id7, id8, id9, id10, id11, id12,
    ]
    .iter()
    .map(|&id| (id.into(), Timestamp::must(3)))
    .collect::<BTreeSet<_>>();
    let expected = vec![
        (id1, 3, 1, true),
        (id2, 3, 2, false),
        (id3, 3, 2, false),
        (id4, 3, 2, false),
        (id5, 3, 2, false),
        (id6, 3, 1, true),
        (id7, 3, 1, true),
        (id8, 3, 1, true),
    ];
    assert_eq!(
        reader
            .previous_revisions(queries.clone(), Arc::new(NoopRetentionValidator))
            .await?,
        expected
            .into_iter()
            .map(|(id, ts, prev_ts, exists)| (
                (InternalDocumentId::from(id), Timestamp::must(ts)),
                DocumentLogEntry {
                    id: id.into(),
                    ts: Timestamp::must(prev_ts),
                    value: exists.then(|| doc(id)),
                    prev_ts: None,
                },
            ))
            .collect::<BTreeMap<_, _>>(),
    );

    let retention_validator = FakeRetentionValidator::new(Timestamp::must(2), Timestamp::must(0));
    // Queries are at ts=3, so with min timestamps <3, it's a valid query.
    reader
        .previous_revisions(queries.clone(), Arc::new(retention_validator))
        .await?;

    let retention_validator = FakeRetentionValidator::new(Timestamp::must(4), Timestamp::must(1));
    // With min_index_ts=4, the query is outside index retention but still within
    // document retention.
    reader
        .previous_revisions(queries.clone(), Arc::new(retention_validator))
        .await?;

    let retention_validator = FakeRetentionValidator::new(Timestamp::must(5), Timestamp::must(5));
    // With min_index_ts=4, the query is outside both index and document retention.
    assert!(reader
        .previous_revisions(queries, Arc::new(retention_validator))
        .await
        .is_err());

    Ok(())
}

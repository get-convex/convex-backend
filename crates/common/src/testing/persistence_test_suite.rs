use std::{
    cmp::max,
    collections::{
        BTreeMap,
        BTreeSet,
    },
    sync::Arc,
};

use futures::{
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
    InternalDocumentId,
    ResolvedDocumentId,
    TableId,
    TableMapping,
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
        Start,
    },
    persistence::{
        ConflictStrategy,
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
            persistence_test_suite::write_and_load(p).await
        }

        #[tokio::test]
        async fn test_persistence_write_and_load_from_table() -> anyhow::Result<()> {
            let $db = $create_db;
            let p = $create_persistence;
            persistence_test_suite::write_and_load_from_table(p).await
        }

        #[tokio::test]
        async fn test_persistence_write_and_load_value_types() -> anyhow::Result<()> {
            let $db = $create_db;
            let p = $create_persistence;
            persistence_test_suite::write_and_load_value_types(p).await
        }

        #[tokio::test]
        async fn test_persistence_overwrite_document() -> anyhow::Result<()> {
            let $db = $create_db;
            let p = $create_persistence;
            persistence_test_suite::overwrite_document(p).await
        }

        #[tokio::test]
        async fn test_persistence_overwrite_index() -> anyhow::Result<()> {
            let $db = $create_db;
            let p = $create_persistence;
            persistence_test_suite::overwrite_index(p).await
        }

        #[tokio::test]
        async fn test_persistence_write_and_load_sorting() -> anyhow::Result<()> {
            let $db = $create_db;
            let p = $create_persistence;
            persistence_test_suite::write_and_load_sorting(p).await
        }

        #[tokio::test]
        async fn test_persistence_same_internal_id_multiple_tables() -> anyhow::Result<()> {
            let $db = $create_db;
            let p = $create_persistence;
            persistence_test_suite::same_internal_id_multiple_tables(p).await
        }

        #[tokio::test]
        async fn test_persistence_query_index_at_ts() -> anyhow::Result<()> {
            let $db = $create_db;
            let p = $create_persistence;
            persistence_test_suite::query_index_at_ts(p).await
        }

        #[tokio::test]
        async fn test_persistence_query_index_range_short() -> anyhow::Result<()> {
            let $db = $create_db;
            let p = $create_persistence;
            persistence_test_suite::query_index_range_short(p).await
        }

        #[tokio::test]
        async fn test_persistence_query_index_range_long() -> anyhow::Result<()> {
            let $db = $create_db;
            let p = $create_persistence;
            persistence_test_suite::query_index_range_long(p).await
        }

        #[tokio::test]
        async fn test_persistence_query_multiple_indexes() -> anyhow::Result<()> {
            let $db = $create_db;
            let p = $create_persistence;
            persistence_test_suite::query_multiple_indexes(p).await
        }

        #[tokio::test]
        async fn test_persistence_query_dangling_reference() -> anyhow::Result<()> {
            let $db = $create_db;
            let p = $create_persistence;
            persistence_test_suite::query_dangling_reference(p).await
        }

        #[tokio::test]
        async fn test_persistence_query_reference_deleted_doc() -> anyhow::Result<()> {
            let $db = $create_db;
            let p = $create_persistence;
            persistence_test_suite::query_reference_deleted_doc(p).await
        }

        #[tokio::test]
        async fn test_persistence_query_with_rows_estimate_short() -> anyhow::Result<()> {
            let $db = $create_db;
            let p = $create_persistence;
            persistence_test_suite::query_with_rows_estimate_short(p).await
        }

        #[tokio::test]
        async fn test_persistence_query_with_rows_estimate_long() -> anyhow::Result<()> {
            let $db = $create_db;
            let p = $create_persistence;
            persistence_test_suite::query_with_rows_estimate_long(p).await
        }

        #[tokio::test]
        async fn test_persistence_write_then_read() -> anyhow::Result<()> {
            let $db = $create_db;
            persistence_test_suite::write_then_read(|| async { Ok($create_persistence) }).await
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
            persistence_test_suite::persistence_global(p).await
        }

        #[tokio::test]
        async fn test_persistence_enforce_retention() -> anyhow::Result<()> {
            let $db = $create_db;
            let p = $create_persistence;
            persistence_test_suite::persistence_enforce_retention(p).await
        }

        #[tokio::test]
        async fn test_persistence_previous_revisions() -> anyhow::Result<()> {
            let $db = $create_db;
            let p = $create_persistence;
            persistence_test_suite::persistence_previous_revisions(p).await
        }
    };
}

pub async fn write_and_load_from_table<P: Persistence>(p: P) -> anyhow::Result<()> {
    let mut id_generator = TestIdGenerator::new();
    let table1: TableName = str::parse("table1")?;
    let doc_id1 = id_generator.generate(&table1);
    let doc1 = ResolvedDocument::new(doc_id1, CreationTime::ONE, ConvexObject::empty())?;

    let table2: TableName = str::parse("table2")?;
    let doc_id2 = id_generator.generate(&table2);
    let doc2 = ResolvedDocument::new(doc_id2, CreationTime::ONE, ConvexObject::empty())?;

    p.write(
        vec![
            // Write docs
            (
                Timestamp::must(0),
                doc1.id_with_table_id(),
                Some(doc1.clone()),
            ),
            (
                Timestamp::must(0),
                doc2.id_with_table_id(),
                Some(doc2.clone()),
            ),
            // Delete doc
            (Timestamp::must(1), doc1.id_with_table_id(), None),
            (Timestamp::must(1), doc2.id_with_table_id(), None),
        ],
        BTreeSet::new(),
        ConflictStrategy::Error,
    )
    .await?;
    id_generator.write_tables(p.box_clone()).await?;

    test_load_documents_from_table(
        &p,
        doc1.table().table_id,
        TimestampRange::all(),
        Order::Asc,
        vec![
            (
                Timestamp::must(0),
                doc1.id_with_table_id(),
                Some(doc1.clone()),
            ),
            (Timestamp::must(1), doc1.id_with_table_id(), None),
        ],
    )
    .await?;

    test_load_documents_from_table(
        &p,
        doc2.table().table_id,
        TimestampRange::all(),
        Order::Asc,
        vec![
            (
                Timestamp::must(0),
                doc2.id_with_table_id(),
                Some(doc2.clone()),
            ),
            (Timestamp::must(1), doc2.id_with_table_id(), None),
        ],
    )
    .await?;

    test_load_documents_from_table(
        &p,
        doc1.table().table_id,
        TimestampRange::new(Timestamp::must(1)..)?,
        Order::Asc,
        vec![(Timestamp::must(1), doc1.id_with_table_id(), None)],
    )
    .await?;

    test_load_documents_from_table(
        &p,
        doc2.table().table_id,
        TimestampRange::new(Timestamp::must(1)..)?,
        Order::Asc,
        vec![(Timestamp::must(1), doc2.id_with_table_id(), None)],
    )
    .await?;

    test_load_documents_from_table(
        &p,
        doc1.table().table_id,
        TimestampRange::new(..Timestamp::must(1))?,
        Order::Asc,
        vec![(
            Timestamp::must(0),
            doc1.id_with_table_id(),
            Some(doc1.clone()),
        )],
    )
    .await?;

    test_load_documents_from_table(
        &p,
        doc2.table().table_id,
        TimestampRange::new(..Timestamp::must(1))?,
        Order::Asc,
        vec![(
            Timestamp::must(0),
            doc2.id_with_table_id(),
            Some(doc2.clone()),
        )],
    )
    .await?;
    Ok(())
}

pub async fn write_and_load<P: Persistence>(p: P) -> anyhow::Result<()> {
    let mut id_generator = TestIdGenerator::new();
    let table: TableName = str::parse("table")?;
    let doc_id = id_generator.generate(&table);

    let doc = ResolvedDocument::new(doc_id, CreationTime::ONE, ConvexObject::empty())?;

    p.write(
        vec![
            // Write doc
            (
                Timestamp::must(0),
                doc.id_with_table_id(),
                Some(doc.clone()),
            ),
            // Delete doc
            (Timestamp::must(1), doc.id_with_table_id(), None),
        ],
        BTreeSet::new(),
        ConflictStrategy::Error,
    )
    .await?;
    id_generator.write_tables(p.box_clone()).await?;

    // Equivalent of load_all_documents.
    test_load_documents(
        &p,
        &id_generator,
        TimestampRange::all(),
        Order::Asc,
        vec![
            (
                Timestamp::must(0),
                doc.id_with_table_id(),
                Some(doc.clone()),
            ),
            (Timestamp::must(1), doc.id_with_table_id(), None),
        ],
    )
    .await?;
    // Pattern used when updating shape.
    test_load_documents(
        &p,
        &id_generator,
        TimestampRange::new(Timestamp::must(1)..)?,
        Order::Asc,
        vec![(Timestamp::must(1), doc.id_with_table_id(), None)],
    )
    .await?;
    // Pattern used when bootstrapping index.
    test_load_documents(
        &p,
        &id_generator,
        TimestampRange::at(Timestamp::MIN),
        Order::Asc,
        vec![(Timestamp::MIN, doc.id_with_table_id(), Some(doc.clone()))],
    )
    .await?;
    // Pattern used when backfilling index.
    test_load_documents(
        &p,
        &id_generator,
        TimestampRange::new(..Timestamp::must(2))?,
        Order::Desc,
        vec![
            (Timestamp::must(1), doc.id_with_table_id(), None),
            (
                Timestamp::must(0),
                doc.id_with_table_id(),
                Some(doc.clone()),
            ),
        ],
    )
    .await?;

    Ok(())
}

pub async fn write_and_load_value_types<P: Persistence>(p: P) -> anyhow::Result<()> {
    let mut id_generator = TestIdGenerator::new();
    let table: TableName = str::parse("table")?;
    let mut next_ts = Timestamp::MIN;
    let new_doc = |value| {
        let id = id_generator.generate(&table);
        let doc = ResolvedDocument::new(id, CreationTime::ONE, assert_obj!("field" => value))?;
        let r = (next_ts, doc.id_with_table_id(), Some(doc));
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
        ConvexValue::Array(vec![ConvexValue::Null].try_into()?),
        ConvexValue::Set(btreeset!(ConvexValue::Null).try_into()?),
        ConvexValue::Map(btreemap!(ConvexValue::Null => ConvexValue::Null).try_into()?),
        ConvexValue::Object(assert_obj!("nested" => ConvexValue::Null)),
    ];
    let triples = values
        .into_iter()
        .map(new_doc)
        .collect::<anyhow::Result<Vec<_>>>()?;

    p.write(triples.clone(), BTreeSet::new(), ConflictStrategy::Error)
        .await?;
    id_generator.write_tables(p.box_clone()).await?;

    test_load_documents(
        &p,
        &id_generator,
        TimestampRange::all(),
        Order::Asc,
        triples,
    )
    .await?;

    Ok(())
}

pub async fn overwrite_document<P: Persistence>(p: P) -> anyhow::Result<()> {
    let mut id_generator = TestIdGenerator::new();
    let table: TableName = str::parse("table")?;
    let doc_id = id_generator.generate(&table);

    let doc = ResolvedDocument::new(doc_id, CreationTime::ONE, ConvexObject::empty())?;

    p.write(
        vec![
            // Write doc
            (
                Timestamp::must(0),
                doc.id_with_table_id(),
                Some(doc.clone()),
            ),
            // Delete doc
            (Timestamp::must(1), doc.id_with_table_id(), None),
        ],
        BTreeSet::new(),
        ConflictStrategy::Error,
    )
    .await?;

    // Try to overwrite the original write at ts 0 -- should fail.
    let err = p
        .write(
            vec![(
                Timestamp::must(0),
                doc.id_with_table_id(),
                Some(doc.clone()),
            )],
            BTreeSet::new(),
            ConflictStrategy::Error,
        )
        .await
        .unwrap_err()
        .to_string();
    assert!(err.contains("constraint") || err.contains("Duplicate entry"));

    Ok(())
}

pub async fn overwrite_index<P: Persistence>(p: P) -> anyhow::Result<()> {
    let mut id_generator = TestIdGenerator::new();
    let index_id = id_generator.generate(&INDEX_TABLE);
    let ts = Timestamp::must(1);
    let table: TableName = str::parse("table")?;
    let doc_id = id_generator.generate(&table);
    let table_id = doc_id.table().table_id;
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
        value: DatabaseIndexValue::NonClustered(*doc.id()),
        is_system_index: false,
    };
    p.write(
        vec![(ts, doc.id_with_table_id(), Some(doc.clone()))],
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
            table_id,
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
    p: &P,
    table_id: TableId,
    range: TimestampRange,
    order: Order,
    expected: Vec<(Timestamp, InternalDocumentId, Option<ResolvedDocument>)>,
) -> anyhow::Result<()> {
    for page_size in 1..3 {
        let docs: Vec<_> = p
            .reader()
            .load_documents_from_table(table_id, range, order, page_size)
            .try_collect()
            .await?;
        let docs: Vec<_> = docs.into_iter().collect();
        assert_eq!(docs, expected);
    }
    Ok(())
}

pub async fn test_load_documents<P: Persistence>(
    p: &P,
    table_mapping: &TableMapping,
    range: TimestampRange,
    order: Order,
    expected: Vec<(Timestamp, InternalDocumentId, Option<ResolvedDocument>)>,
) -> anyhow::Result<()> {
    let docs: Vec<_> = p
        .reader()
        .load_documents(range, order, 10)
        .try_collect()
        .await?;
    let docs: Vec<_> = docs
        .into_iter()
        .filter(|(_, id, _)| !table_mapping.is_system_table_id(*id.table()))
        .collect();
    assert_eq!(docs, expected);
    Ok(())
}

pub async fn write_and_load_sorting<P: Persistence>(p: P) -> anyhow::Result<()> {
    let table1: TableName = str::parse("table1")?;
    let table2: TableName = str::parse("table2")?;
    let mut id_generator = TestIdGenerator::new();

    let doc_id1 = id_generator.generate(&table1);
    let doc_id2 = id_generator.generate(&table2);

    let doc1 = ResolvedDocument::new(doc_id1, CreationTime::ONE, ConvexObject::empty())?;
    let doc2 = ResolvedDocument::new(doc_id2, CreationTime::ONE, ConvexObject::empty())?;
    p.write(
        vec![
            // Write doc1 and doc2. Make sure sorted by TS, not ID
            (
                Timestamp::must(1),
                doc1.id_with_table_id(),
                Some(doc1.clone()),
            ),
            (
                Timestamp::must(0),
                doc2.id_with_table_id(),
                Some(doc2.clone()),
            ),
        ],
        BTreeSet::new(),
        ConflictStrategy::Error,
    )
    .await?;
    id_generator.write_tables(p.box_clone()).await?;

    let docs: Vec<_> = p.reader().load_all_documents().try_collect().await?;
    let docs: Vec<_> = docs
        .into_iter()
        .filter(|(_, id, _)| !id_generator.is_system_table_id(*id.table()))
        .collect();
    assert_eq!(
        docs,
        vec![
            // Make sure sorted by TS, not ID
            (Timestamp::must(0), doc2.id_with_table_id(), Some(doc2)),
            (Timestamp::must(1), doc1.id_with_table_id(), Some(doc1)),
        ]
    );

    Ok(())
}

pub async fn same_internal_id_multiple_tables<P: Persistence>(p: P) -> anyhow::Result<()> {
    let mut id_generator = TestIdGenerator::new();

    // Create two documents with the same internal_id but in two different tables.
    let internal_id = id_generator.generate_internal();

    let table1_id = id_generator.table_id(&str::parse("table1")?);
    let table2_id = id_generator.table_id(&str::parse("table2")?);

    let doc1 = ResolvedDocument::new(
        ResolvedDocumentId::new(table1_id, internal_id),
        CreationTime::ONE,
        assert_obj!("value" => 1),
    )?;
    let doc2 = ResolvedDocument::new(
        ResolvedDocumentId::new(table2_id, internal_id),
        CreationTime::ONE,
        assert_obj!("value" => 2),
    )?;

    // Have an index pointing to each document.
    let index_fields: IndexedFields = vec!["value".parse()?].try_into()?;
    let index1_id = id_generator.generate(&INDEX_TABLE).internal_id();
    let index2_id = id_generator.generate(&INDEX_TABLE).internal_id();

    let ts = Timestamp::must(1000);
    p.write(
        vec![
            // Write doc1 and doc2. Make sure sorted by TS, not ID
            (ts, doc1.id_with_table_id(), Some(doc1.clone())),
            (ts, doc2.id_with_table_id(), Some(doc2.clone())),
        ],
        btreeset!(
            (
                ts,
                DatabaseIndexUpdate {
                    index_id: index1_id,
                    key: doc1.index_key(&index_fields, p.reader().version()),
                    value: DatabaseIndexValue::NonClustered(*doc1.id()),
                    is_system_index: false,
                }
            ),
            (
                ts,
                DatabaseIndexUpdate {
                    index_id: index2_id,
                    key: doc1.index_key(&index_fields, p.reader().version()),
                    value: DatabaseIndexValue::NonClustered(*doc2.id()),
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
            table1_id.table_id,
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
    assert_eq!(results[0].2, doc1);

    // Query index2 should give us the second document.
    let results = p
        .reader()
        .index_scan(
            index2_id,
            table2_id.table_id,
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
    assert_eq!(results[0].2, doc2);

    Ok(())
}

pub async fn query_index_at_ts<P: Persistence>(p: P) -> anyhow::Result<()> {
    let mut id_generator = TestIdGenerator::new();
    let index_id = id_generator.generate(&INDEX_TABLE).internal_id();

    let table: TableName = str::parse("table")?;
    let doc_id = id_generator.generate(&table);
    let table_id = doc_id.table().table_id;

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
            vec![(*ts, doc.id_with_table_id(), Some(doc))],
            index_updates.into_iter().map(|u| (*ts, u)).collect(),
            ConflictStrategy::Error,
        )
        .await?;
        old_key = Some(key);
    }
    id_generator.write_tables(p.box_clone()).await?;

    // Query at the each timestamp should returns the expected result.
    for (ts, expected_value) in ts_to_value.into_iter() {
        let results = p
            .reader()
            .index_scan(
                index_id,
                table_id,
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
        let key = doc.index_key(&fields, p.reader().version()).into_bytes();
        assert_eq!(results, vec![(key, ts, doc)]);
    }

    Ok(())
}

// Test varies ranges where all generated keys start with the given prefix.
pub async fn query_index_range_with_prefix<P: Persistence>(
    p: P,
    prefix: Vec<u8>,
) -> anyhow::Result<()> {
    let mut id_generator = TestIdGenerator::new();
    let index_id = id_generator.generate(&INDEX_TABLE).internal_id();

    let table: TableName = str::parse("table")?;
    let table_id = id_generator.table_id(&table).table_id;
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

        let doc_id = id_generator.generate(&table);
        let doc = ResolvedDocument::new(doc_id, CreationTime::ONE, assert_obj!("value" => value))?;
        documents.push((ts, doc.id_with_table_id(), Some(doc.clone())));
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
    id_generator.write_tables(p.box_clone()).await?;

    keys.sort();
    for i in 0..keys.len() {
        for j in 0..keys.len() {
            for order in [Order::Asc, Order::Desc] {
                let results = p
                    .reader()
                    .index_scan(
                        index_id,
                        table_id,
                        ts,
                        &Interval {
                            start: Start::Included(keys[i].clone().into_bytes().into()),
                            end: End::after_prefix(&BinaryKey::from(keys[j].clone().into_bytes())),
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
                            k.clone().into_bytes(),
                            ts,
                            keys_to_doc.get(&k).unwrap().clone(),
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
pub async fn query_index_range_short<P: Persistence>(p: P) -> anyhow::Result<()> {
    query_index_range_with_prefix(p, Vec::new()).await
}

// Test by prefixing all keys with the same long prefix.
pub async fn query_index_range_long<P: Persistence>(p: P) -> anyhow::Result<()> {
    let long_prefix = testing::generate_with::<Vec<u8>>(size_range(10000).lift());
    query_index_range_with_prefix(p, long_prefix).await
}

// Make sure we correctly filter using the index_id.
pub async fn query_multiple_indexes<P: Persistence>(p: P) -> anyhow::Result<()> {
    let table: TableName = str::parse("table")?;
    let ts = Timestamp::must(1702);

    let mut id_generator = TestIdGenerator::new();
    let table_id = id_generator.table_id(&table).table_id;
    let mut documents = Vec::new();
    let mut indexes = BTreeSet::new();
    let mut index_to_results: BTreeMap<_, Vec<_>> = BTreeMap::new();
    for i in 0..5 {
        let index_id = id_generator.generate(&INDEX_TABLE).internal_id();
        let fields: IndexedFields = vec![format!("value_{}", i).parse()?].try_into()?;

        for j in 0..5 {
            let doc_id = id_generator.generate(&table);
            let doc = ResolvedDocument::new(
                doc_id,
                CreationTime::ONE,
                assert_obj!(
                    format!("value_{}", i) => j
                ),
            )?;
            let key = doc.index_key(&fields, p.reader().version());
            documents.push((ts, doc.id_with_table_id(), Some(doc.clone())));
            indexes.insert((
                ts,
                DatabaseIndexUpdate {
                    index_id,
                    key: key.clone(),
                    value: DatabaseIndexValue::NonClustered(doc_id),
                    is_system_index: false,
                },
            ));
            index_to_results
                .entry(index_id)
                .or_default()
                .push((key.into_bytes(), ts, doc));
        }
    }

    p.write(documents, indexes, ConflictStrategy::Error).await?;
    id_generator.write_tables(p.box_clone()).await?;

    for (index_id, expected) in index_to_results {
        let keys = p
            .reader()
            .index_scan(
                index_id,
                table_id,
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
pub async fn query_dangling_reference<P: Persistence>(p: P) -> anyhow::Result<()> {
    let table: TableName = str::parse("table")?;
    let ts = Timestamp::must(1702);
    let mut id_generator = TestIdGenerator::new();

    let table_id = id_generator.table_id(&table).table_id;

    let index_id = id_generator.generate(&INDEX_TABLE).internal_id();

    let index_fields: IndexedFields = vec!["value".parse()?].try_into()?;
    let doc_id = id_generator.generate(&table);
    let document = ResolvedDocument::new(doc_id, CreationTime::ONE, assert_obj!("value" => 20))?;
    let index_update = DatabaseIndexUpdate {
        index_id,
        key: document.index_key(&index_fields, p.reader().version()),
        value: DatabaseIndexValue::NonClustered(*document.id()),
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
            table_id,
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
pub async fn query_reference_deleted_doc<P: Persistence>(p: P) -> anyhow::Result<()> {
    let table: TableName = str::parse("table")?;
    let ts = Timestamp::must(1702);

    let mut id_generator = TestIdGenerator::new();

    let table_id = id_generator.table_id(&table).table_id;
    let index_id = id_generator.generate(&INDEX_TABLE).internal_id();

    let index_fields: IndexedFields = vec!["value".parse()?].try_into()?;
    let doc_id = id_generator.generate(&table);
    let document = ResolvedDocument::new(doc_id, CreationTime::ONE, assert_obj!("value" => 20))?;
    let index_update = DatabaseIndexUpdate {
        index_id,
        key: document.index_key(&index_fields, p.reader().version()),
        value: DatabaseIndexValue::NonClustered(*document.id()),
        is_system_index: false,
    };

    // Note that we write a deleted document.
    p.write(
        vec![(ts, document.id_with_table_id(), None)],
        btreeset!((ts, index_update)),
        ConflictStrategy::Error,
    )
    .await?;

    let results: Vec<_> = p
        .reader()
        .index_scan(
            index_id,
            table_id,
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
    p: P,
    prefix: Vec<u8>,
) -> anyhow::Result<()> {
    let table: TableName = str::parse("table")?;
    let ts = Timestamp::must(1702);

    let mut id_generator = TestIdGenerator::new();
    let index_id = id_generator.generate(&INDEX_TABLE).internal_id();
    let table_id = id_generator.table_id(&table).table_id;

    let index_fields: IndexedFields = vec!["value".parse()?].try_into()?;
    let mut documents = Vec::new();
    for i in 0..99 {
        let doc_id = id_generator.generate(&table);

        let mut value = prefix.clone();
        value.push(i as u8);

        let document =
            ResolvedDocument::new(doc_id, CreationTime::ONE, assert_obj!("value" => value))?;
        let index_update = DatabaseIndexUpdate {
            index_id,
            key: document.index_key(&index_fields, p.reader().version()),
            value: DatabaseIndexValue::NonClustered(*document.id()),
            is_system_index: false,
        };
        p.write(
            vec![(ts, document.id_with_table_id(), Some(document.clone()))],
            btreeset!((ts, index_update)),
            ConflictStrategy::Error,
        )
        .await?;
        documents.push(document);
    }
    id_generator.write_tables(p.box_clone()).await?;

    // We should get the same result regardless of the rows estimate.
    for rows_estimate in [1, 10, 20, 100] {
        let results: Vec<_> = p
            .reader()
            .index_scan(
                index_id,
                table_id,
                ts,
                &Interval::all(),
                Order::Asc,
                rows_estimate,
                Arc::new(NoopRetentionValidator),
            )
            .try_collect::<Vec<_>>()
            .await?
            .into_iter()
            .map(|(_, _, doc)| (doc))
            .collect();
        assert_eq!(results, documents);
    }

    Ok(())
}

pub async fn query_with_rows_estimate_short<P: Persistence>(p: P) -> anyhow::Result<()> {
    query_with_rows_estimate_with_prefix(p, Vec::new()).await
}

// Test by prefixing all keys with the same long prefix.
pub async fn query_with_rows_estimate_long<P: Persistence>(p: P) -> anyhow::Result<()> {
    let long_prefix = testing::generate_with::<Vec<u8>>(size_range(10000).lift());
    query_with_rows_estimate_with_prefix(p, long_prefix).await
}

pub async fn write_then_read<F, Fut, P: Persistence>(mut make_p: F) -> anyhow::Result<()>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = anyhow::Result<P>>,
{
    let p_write = make_p().await?;
    let table: TableName = str::parse("table")?;
    let mut id_generator = TestIdGenerator::new();
    let doc_id = id_generator.generate(&table);

    let doc = ResolvedDocument::new(doc_id, CreationTime::ONE, ConvexObject::empty())?;

    p_write
        .write(
            vec![(
                Timestamp::must(0),
                doc.id_with_table_id(),
                Some(doc.clone()),
            )],
            BTreeSet::new(),
            ConflictStrategy::Error,
        )
        .await?;
    id_generator.write_tables(p_write.box_clone()).await?;
    drop(p_write);

    // Read from another persistence, e.g. on a replica.
    let p_read = make_p().await?;
    let reader = p_read.reader();
    let stream = reader.load_all_documents();
    let results: Vec<_> = stream.try_collect().await?;
    let results: Vec<_> = results
        .into_iter()
        .filter(|(_, id, _)| !id_generator.is_system_table_id(*id.table()))
        .collect();
    assert_eq!(
        results,
        vec![(
            Timestamp::must(0),
            doc.id_with_table_id(),
            Some(doc.clone())
        )],
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
    let doc_id = id_generator.generate(&table);

    let doc = ResolvedDocument::new(doc_id, CreationTime::ONE, ConvexObject::empty())?;

    p_backend1
        .write(
            vec![(
                Timestamp::must(0),
                doc.id_with_table_id(),
                Some(doc.clone()),
            )],
            BTreeSet::new(),
            ConflictStrategy::Error,
        )
        .await?;
    // Release the lease.
    drop(p_backend1);

    let mut p_migration = make_p().await?;
    p_migration.set_read_only(true).await?;

    let result = make_p().await;
    assert!(result.is_err());

    drop(p_migration);

    // Try to acquire lease should fail because it's read-only.
    let result = make_p().await;
    assert!(result.is_err());

    let mut p_cleanup = make_p_read_only().await?;
    p_cleanup.set_read_only(false).await?;
    drop(p_cleanup);

    // Now it's no longer read-only.
    let p_backend2 = make_p().await?;
    p_backend2
        .write(
            vec![(
                Timestamp::must(1),
                doc.id_with_table_id(),
                Some(doc.clone()),
            )],
            BTreeSet::new(),
            ConflictStrategy::Error,
        )
        .await?;

    Ok(())
}

pub async fn persistence_global<P: Persistence>(p: P) -> anyhow::Result<()> {
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
    Ok(())
}

pub async fn persistence_enforce_retention<P: Persistence>(p: P) -> anyhow::Result<()> {
    let mut id_generator = TestIdGenerator::new();
    let by_id_index_id = id_generator.generate(&INDEX_TABLE).internal_id();
    let by_val_index_id = id_generator.generate(&INDEX_TABLE).internal_id();
    let table: TableName = str::parse("table")?;
    let table_id = id_generator.table_id(&table).table_id;

    fn doc(
        id: ResolvedDocumentId,
        ts: i32,
        val: Option<i64>,
    ) -> anyhow::Result<(Timestamp, InternalDocumentId, Option<ResolvedDocument>)> {
        let doc = val
            .map(|val| ResolvedDocument::new(id, CreationTime::ONE, assert_obj!("value" => val)))
            .transpose()?;
        Ok((Timestamp::must(ts), id.into(), doc))
    }

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

    let id1 = id_generator.generate(&table);
    let id2 = id_generator.generate(&table);
    let id3 = id_generator.generate(&table);
    let id4 = id_generator.generate(&table);
    let id5 = id_generator.generate(&table);

    let documents = vec![
        doc(id1, 1, Some(5))?, // expired because overwritten.
        doc(id2, 2, Some(5))?, // expired because overwritten.
        doc(id1, 3, Some(6))?, // latest.
        doc(id2, 4, None)?,    // expired because tombstone.
        doc(id3, 5, Some(5))?, // latest.
        doc(id4, 6, Some(5))?, // visible at min_snapshot_ts.
        doc(id5, 7, Some(5))?, // visible at min_snapshot_ts.
        // min_snapshot_ts: 8
        doc(id4, 9, None)?,
        doc(id5, 10, Some(6))?,
        doc(id5, 11, Some(5))?,
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
    id_generator.write_tables(p.box_clone()).await?;

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
    let all_expired = p.index_entries_to_delete(&expired).await?;
    assert_eq!(all_expired, expired);
    assert_eq!(p.delete_index_entries(expired).await?, 7);

    let reader = p.reader();

    // All documents are still visible at snapshot ts=8.
    let stream = reader.index_scan(
        by_val_index_id,
        table_id,
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
        .map(|(_, ts, doc)| (*doc.id(), i64::from(ts)))
        .collect();
    assert_eq!(results, vec![(id3, 5), (id4, 6), (id5, 7), (id1, 3)]);

    // Old versions of documents at snapshot ts=2 are not visible.
    let stream = reader.index_scan(
        by_val_index_id,
        table_id,
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

pub async fn persistence_previous_revisions<P: Persistence>(p: P) -> anyhow::Result<()> {
    let reader = p.reader();

    let table: TableName = str::parse("table")?;
    let mut id_generator = TestIdGenerator::new();
    let id1 = id_generator.generate(&table);
    let id2 = id_generator.generate(&table);
    let id3 = id_generator.generate(&table);
    let id4 = id_generator.generate(&table);
    let id5 = id_generator.generate(&table);
    let id6 = id_generator.generate(&table);
    let id7 = id_generator.generate(&table);
    let id8 = id_generator.generate(&table);
    let id9 = id_generator.generate(&table);
    let id10 = id_generator.generate(&table);
    let id11 = id_generator.generate(&table);
    let id12 = id_generator.generate(&table);

    let doc = |id: ResolvedDocumentId| {
        ResolvedDocument::new(id, CreationTime::ONE, assert_obj!("field" => id)).unwrap()
    };

    // Create eight documents at timestamp 1.
    let writes = vec![id1, id2, id3, id4, id5, id6, id7, id8]
        .iter()
        .map(|&id| (Timestamp::must(1), id.into(), Some(doc(id))))
        .collect();
    p.write(writes, BTreeSet::new(), ConflictStrategy::Error)
        .await?;

    // Delete four of them at timestamp 2.
    let writes = [id2, id3, id4, id5]
        .iter()
        .map(|&id| (Timestamp::must(2), id.into(), None))
        .collect();
    p.write(writes, BTreeSet::new(), ConflictStrategy::Error)
        .await?;
    id_generator.write_tables(p.box_clone()).await?;

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
        reader.previous_revisions(queries).await?,
        expected
            .into_iter()
            .map(|(id, ts, prev_ts, exists)| (
                (InternalDocumentId::from(id), Timestamp::must(ts)),
                (Timestamp::must(prev_ts), exists.then(|| doc(id))),
            ))
            .collect::<BTreeMap<_, _>>(),
    );

    Ok(())
}

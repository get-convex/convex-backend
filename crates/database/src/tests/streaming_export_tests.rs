use common::{
    assert_obj,
    components::ComponentPath,
    document::ResolvedDocument,
    pause::PauseController,
    pii::PII,
    runtime::Runtime,
    types::TableName,
};
use futures::{
    future::{
        self,
        Either,
    },
    FutureExt as _,
};
use keybroker::Identity;
use maplit::btreemap;
use pretty_assertions::assert_eq;
use runtime::testing::TestRuntime;
use sync_types::Timestamp;
use value::{
    ResolvedDocumentId,
    TableNamespace,
};

use crate::{
    streaming_export_selection::{
        StreamingExportColumnInclusion,
        StreamingExportColumnSelection,
        StreamingExportComponentSelection,
        StreamingExportDocument,
        StreamingExportInclusionDefault,
        StreamingExportSelection,
        StreamingExportTableSelection,
    },
    test_helpers::DbFixtures,
    DocumentDeltas,
    SnapshotPage,
    StreamingExportFilter,
    TableModel,
    TestFacingModel,
    UserFacingModel,
};
#[convex_macro::test_runtime]
async fn test_document_deltas(rt: TestRuntime) -> anyhow::Result<()> {
    let DbFixtures { db, .. } = DbFixtures::new(&rt).await?;
    let mut tx = db.begin(Identity::system()).await?;
    let doc1 = TestFacingModel::new(&mut tx)
        .insert_and_get("table1".parse()?, assert_obj!())
        .await?;
    let doc2 = TestFacingModel::new(&mut tx)
        .insert_and_get("table2".parse()?, assert_obj!())
        .await?;
    // Same timestamp => sorted by internal id.
    let (doc1sort, doc2sort) = if doc1.internal_id() < doc2.internal_id() {
        (doc1.clone(), doc2)
    } else {
        (doc2, doc1.clone())
    };
    let ts1 = db.commit(tx).await?;
    let mut tx = db.begin(Identity::system()).await?;
    let doc3 = TestFacingModel::new(&mut tx)
        .insert_and_get("table3".parse()?, assert_obj!())
        .await?;
    let table_mapping = tx.table_mapping().clone();
    let ts2 = db.commit(tx).await?;

    let deltas = db
        .document_deltas(
            Identity::system(),
            None,
            StreamingExportFilter::default(),
            200,
            3,
        )
        .await?;
    assert_eq!(
        deltas.deltas,
        vec![
            (
                ts1,
                doc1sort.developer_id(),
                ComponentPath::root(),
                table_mapping.tablet_name(doc1sort.id().tablet_id)?,
                Some(StreamingExportDocument::with_all_fields(doc1sort.clone()))
            ),
            (
                ts1,
                doc2sort.developer_id(),
                ComponentPath::root(),
                table_mapping.tablet_name(doc2sort.id().tablet_id)?,
                Some(StreamingExportDocument::with_all_fields(doc2sort.clone()))
            ),
            (
                ts2,
                doc3.developer_id(),
                ComponentPath::root(),
                table_mapping.tablet_name(doc3.id().tablet_id)?,
                Some(StreamingExportDocument::with_all_fields(doc3.clone()))
            ),
        ],
    );
    assert_eq!(deltas.cursor, ts2);
    assert_eq!(deltas.has_more, false);

    let deltas_cursor = db
        .document_deltas(
            Identity::system(),
            Some(ts1),
            StreamingExportFilter::default(),
            200,
            3,
        )
        .await?;
    assert_eq!(
        deltas_cursor.deltas,
        vec![(
            ts2,
            doc3.developer_id(),
            ComponentPath::root(),
            table_mapping.tablet_name(doc3.id().tablet_id)?,
            Some(StreamingExportDocument::with_all_fields(doc3.clone()))
        )],
    );
    assert_eq!(deltas_cursor.cursor, ts2);
    assert_eq!(deltas_cursor.has_more, false);

    let deltas_table_filter = db
        .document_deltas(
            Identity::system(),
            None,
            StreamingExportFilter {
                selection: StreamingExportSelection::single_table(
                    ComponentPath::root(),
                    "table1".parse().unwrap(),
                ),
                ..Default::default()
            },
            200,
            3,
        )
        .await?;
    assert_eq!(
        deltas_table_filter.deltas,
        vec![(
            ts1,
            doc1.developer_id(),
            ComponentPath::root(),
            table_mapping.tablet_name(doc1.id().tablet_id)?,
            Some(StreamingExportDocument::with_all_fields(doc1.clone()))
        )],
    );
    assert_eq!(deltas_table_filter.cursor, ts2);
    assert_eq!(deltas_table_filter.has_more, false);

    // Note we're requesting 1 result, but in order to return the full transaction
    // we receive 2 deltas.
    let deltas_limit = db
        .document_deltas(
            Identity::system(),
            None,
            StreamingExportFilter::default(),
            200,
            1,
        )
        .await?;
    assert_eq!(
        deltas_limit.deltas,
        vec![
            (
                ts1,
                doc1sort.developer_id(),
                ComponentPath::root(),
                table_mapping.tablet_name(doc1sort.id().tablet_id)?,
                Some(StreamingExportDocument::with_all_fields(doc1sort.clone()))
            ),
            (
                ts1,
                doc2sort.developer_id(),
                ComponentPath::root(),
                table_mapping.tablet_name(doc2sort.id().tablet_id)?,
                Some(StreamingExportDocument::with_all_fields(doc2sort.clone()))
            ),
        ],
    );
    assert_eq!(deltas_limit.cursor, ts1);
    assert_eq!(deltas_limit.has_more, true);

    let deltas_auth = db
        .document_deltas(
            Identity::Unknown(None),
            None,
            StreamingExportFilter::default(),
            200,
            3,
        )
        .await;
    assert!(deltas_auth.is_err());

    Ok(())
}

#[convex_macro::test_runtime]
async fn document_deltas_should_ignore_rows_from_deleted_tables(
    rt: TestRuntime,
) -> anyhow::Result<()> {
    let DbFixtures { db, .. } = DbFixtures::new(&rt).await?;

    // When I insert a document…
    let mut tx = db.begin(Identity::system()).await?;
    UserFacingModel::new_root_for_test(&mut tx)
        .insert("table".parse()?, assert_obj!())
        .await?;
    db.commit(tx).await?;

    // …and then delete its table…
    let mut tx = db.begin(Identity::system()).await?;
    let mut model = TableModel::new(&mut tx);
    model
        .delete_active_table(TableNamespace::test_user(), "table".parse()?)
        .await?;
    db.commit(tx).await?;

    // …then the row should not appear in the results returned by document_deltas.
    let deltas = db
        .document_deltas(
            Identity::system(),
            None,
            StreamingExportFilter::default(),
            200,
            3,
        )
        .await?;
    assert!(deltas.deltas.is_empty());

    Ok(())
}

#[convex_macro::test_runtime]
async fn document_deltas_should_not_ignore_rows_from_tables_that_were_not_deleted(
    rt: TestRuntime,
) -> anyhow::Result<()> {
    let DbFixtures { db, .. } = DbFixtures::new(&rt).await?;

    // When I insert two documents…
    let mut tx = db.begin(Identity::system()).await?;
    let remaining_doc = TestFacingModel::new(&mut tx)
        .insert_and_get("table1".parse()?, assert_obj!())
        .await?;
    UserFacingModel::new_root_for_test(&mut tx)
        .insert("table2".parse()?, assert_obj!())
        .await?;
    let ts_insert = db.commit(tx).await?;

    // …and then delete one of the tables…
    let mut tx = db.begin(Identity::system()).await?;
    let mut model = TableModel::new(&mut tx);
    model
        .delete_active_table(TableNamespace::test_user(), "table2".parse()?)
        .await?;
    let table_mapping = tx.table_mapping().clone();
    let ts_latest = db.commit(tx).await?;

    // …then only one row should appear in the results returned by document_deltas.
    let deltas = db
        .document_deltas(
            Identity::system(),
            None,
            StreamingExportFilter::default(),
            200,
            3,
        )
        .await?;
    assert_eq!(
        deltas.deltas,
        vec![(
            ts_insert,
            remaining_doc.developer_id(),
            ComponentPath::root(),
            table_mapping.tablet_name(remaining_doc.id().tablet_id)?,
            Some(StreamingExportDocument::with_all_fields(
                remaining_doc.clone()
            ))
        ),],
    );
    assert_eq!(deltas.cursor, ts_latest);
    assert_eq!(deltas.has_more, false);

    Ok(())
}

#[convex_macro::test_runtime]
async fn test_snapshot_list(
    rt: TestRuntime,
    pause_controller: PauseController,
) -> anyhow::Result<()> {
    let DbFixtures { db, .. } = DbFixtures::new(&rt).await?;
    let mut tx = db.begin(Identity::system()).await?;
    let doc1 = TestFacingModel::new(&mut tx)
        .insert_and_get("table1".parse()?, assert_obj!("f" => 1))
        .await?;
    let doc2 = TestFacingModel::new(&mut tx)
        .insert_and_get("table2".parse()?, assert_obj!("f" => 2))
        .await?;
    let ts1 = db.commit(tx).await?;
    // Same timestamp => sorted by internal id.
    let mut docs1sorted = vec![
        (ts1, ComponentPath::root(), "table1".parse()?, doc1.clone()),
        (ts1, ComponentPath::root(), "table2".parse()?, doc2.clone()),
    ];
    docs1sorted.sort_by_key(|(_, _, _, d)| d.id());
    let mut tx = db.begin(Identity::system()).await?;
    let doc3 = TestFacingModel::new(&mut tx)
        .insert_and_get("table3".parse()?, assert_obj!("f" => 3))
        .await?;
    let doc4 = UserFacingModel::new_root_for_test(&mut tx)
        .patch(doc2.developer_id(), assert_obj!("f" => 4).into())
        .await?;
    let tablet_id = tx
        .table_mapping()
        .namespace(TableNamespace::test_user())
        .number_to_tablet()(doc4.table())?;
    let doc4 = doc4.to_resolved(tablet_id);
    let ts2 = db.commit(tx).await?;
    let mut docs2sorted = vec![
        (ts1, ComponentPath::root(), "table1".parse()?, doc1),
        (ts2, ComponentPath::root(), "table2".parse()?, doc4.clone()),
        (ts2, ComponentPath::root(), "table3".parse()?, doc3),
    ];
    docs2sorted.sort_by_key(|(_, _, _, d)| d.id());

    let snapshot_list_all =
        async |mut snapshot: Option<Timestamp>,
               table_filter: Option<TableName>,
               mut cursor: Option<ResolvedDocumentId>| {
            let mut has_more = true;
            let mut documents = Vec::new();
            let mut pages = 0;
            while has_more && pages < 10 {
                // Assert that we only create a MultiTableIterator on the first page
                let hold = pause_controller.hold("list_snapshot_new_iterator");
                let unhold = async move {
                    let mut pause_guard = hold.wait_for_blocked().await.unwrap();
                    if pages > 0 {
                        pause_guard.inject_error(anyhow::anyhow!(
                            "should not create more than 1 iterator"
                        ));
                    }
                    pause_guard.unpause();
                    future::pending::<!>().await
                };
                let Either::Right((page, _)) = future::select(
                    unhold.boxed(),
                    db.list_snapshot(
                        Identity::system(),
                        snapshot,
                        cursor,
                        StreamingExportFilter {
                            selection: table_filter
                                .clone()
                                .map(|table| {
                                    StreamingExportSelection::single_table(
                                        ComponentPath::root(),
                                        table,
                                    )
                                })
                                .unwrap_or_default(),
                            ..Default::default()
                        },
                        100,
                        5,
                    )
                    .boxed(),
                )
                .await;
                // Consume the hold if it wasn't already, this is janky
                _ = rt.pause_client().wait("list_snapshot_new_iterator").await;
                let page = page?;
                has_more = page.has_more;
                cursor = page.cursor;
                if let Some(s) = snapshot {
                    assert_eq!(page.snapshot, s);
                }
                snapshot = Some(page.snapshot);
                documents.extend(page.documents.into_iter());
                pages += 1;
            }
            assert!(
                !has_more,
                "infinite looping with cursor {cursor:?} after {documents:?}"
            );
            anyhow::Ok((documents, snapshot.unwrap()))
        };

    let to_snapshot_docs =
        |docs: Vec<(Timestamp, ComponentPath, TableName, ResolvedDocument)>| -> Vec<_> {
            docs.into_iter()
                .map(|(ts, cp, tn, doc)| {
                    (ts, cp, tn, StreamingExportDocument::with_all_fields(doc))
                })
                .collect()
        };

    let snapshot_page = snapshot_list_all(None, None, None).await?;
    assert_eq!(snapshot_page.0, to_snapshot_docs(docs2sorted.clone()));
    assert_eq!(snapshot_page.1, ts2);

    let snapshot_explicit_ts = snapshot_list_all(Some(ts2), None, None).await?;
    assert_eq!(
        snapshot_explicit_ts.0,
        to_snapshot_docs(docs2sorted.clone())
    );
    assert_eq!(snapshot_explicit_ts.1, ts2);

    let snapshot_table_filter = snapshot_list_all(None, Some("table2".parse()?), None).await?;
    assert_eq!(
        snapshot_table_filter.0,
        vec![(
            ts2,
            ComponentPath::root(),
            "table2".parse()?,
            StreamingExportDocument::with_all_fields(doc4)
        )]
    );
    assert_eq!(snapshot_table_filter.1, ts2);

    let snapshot_old = snapshot_list_all(Some(ts1), None, None).await?;
    assert_eq!(snapshot_old.0, to_snapshot_docs(docs1sorted.clone()));
    assert_eq!(snapshot_old.1, ts1);

    let snapshot_has_more = db
        .list_snapshot(
            Identity::system(),
            Some(ts1),
            None,
            StreamingExportFilter::default(),
            100,
            1,
        )
        .await?;
    assert_eq!(
        snapshot_has_more.documents,
        to_snapshot_docs(vec![docs1sorted[0].clone()])
    );
    assert_eq!(snapshot_has_more.snapshot, ts1);
    assert_eq!(snapshot_has_more.cursor, Some(docs1sorted[0].3.id()));
    assert_eq!(snapshot_has_more.has_more, true);
    // Verify usage is being tracked (should have some data)
    assert!(
        snapshot_has_more.usage.database_egress.is_empty(),
        "Usage tracking should not record egress size in v1 metric"
    );
    assert!(
        !snapshot_has_more.usage.database_egress_v2.is_empty(),
        "Usage tracking should record egress size"
    );
    assert!(
        !snapshot_has_more.usage.database_egress_rows.is_empty(),
        "Usage tracking should record egress rows"
    );

    let snapshot_cursor = snapshot_list_all(Some(ts1), None, Some(docs1sorted[0].3.id())).await?;
    assert_eq!(
        snapshot_cursor.0,
        to_snapshot_docs(vec![docs1sorted[1].clone()])
    );
    assert_eq!(snapshot_cursor.1, ts1);

    let snapshot_auth = db
        .list_snapshot(
            Identity::Unknown(None),
            None,
            None,
            StreamingExportFilter::default(),
            100,
            3,
        )
        .await;
    assert!(snapshot_auth.is_err());

    Ok(())
}

#[convex_macro::test_runtime]
async fn test_snapshot_list_with_filters(rt: TestRuntime) -> anyhow::Result<()> {
    let DbFixtures { db, .. } = DbFixtures::new(&rt).await?;
    let mut tx = db.begin(Identity::system()).await?;

    let user = TestFacingModel::new(&mut tx)
        .insert_and_get(
            "users".parse()?,
            assert_obj!("user" => "joe", "password" => "hunter2"),
        )
        .await?;
    TestFacingModel::new(&mut tx)
        .insert_and_get("tokens".parse()?, assert_obj!("secret" => "sk-123"))
        .await?;
    let ts = db.commit(tx).await?;

    let filter = StreamingExportFilter {
        selection: StreamingExportSelection {
            components: btreemap! {
                ComponentPath::root() => StreamingExportComponentSelection::Included {
                    tables: btreemap! {
                        "users".parse()? => StreamingExportTableSelection::Included (
                            StreamingExportColumnSelection::new(
                                btreemap! {
                                    "password".parse()? => StreamingExportColumnInclusion::Excluded,
                                    "_creationTime".parse()? => StreamingExportColumnInclusion::Excluded,
                                },
                                StreamingExportInclusionDefault::Included,
                            )?,
                        ),
                    },
                    other_tables: StreamingExportInclusionDefault::Excluded,
                },
            },
            other_components: StreamingExportInclusionDefault::Excluded,
        },
        ..Default::default()
    };

    let snapshot_page = db
        .list_snapshot(Identity::system(), Some(ts), None, filter, 100, 5)
        .await?;

    let partial_doc = StreamingExportDocument::new(
        user.id().into(),
        PII(assert_obj!(
            "_id" => user.id().to_string(),
            "user" => "joe",
        )),
    )?;

    assert_eq!(
        snapshot_page.documents,
        vec![(ts, ComponentPath::root(), "users".parse()?, partial_doc)]
    );

    Ok(())
}

#[convex_macro::test_runtime]
async fn test_document_deltas_with_filters(rt: TestRuntime) -> anyhow::Result<()> {
    let DbFixtures { db, .. } = DbFixtures::new(&rt).await?;
    let mut tx = db.begin(Identity::system()).await?;

    let user_id = TestFacingModel::new(&mut tx)
        .insert(
            &"users".parse()?,
            assert_obj!("user" => "joe", "password" => "hunter2"),
        )
        .await?;
    TestFacingModel::new(&mut tx)
        .insert(&"tokens".parse()?, assert_obj!("secret" => "sk-123"))
        .await?;
    let ts = db.commit(tx).await?;

    let filter = StreamingExportFilter {
        selection: StreamingExportSelection {
            components: btreemap! {
                ComponentPath::root() => StreamingExportComponentSelection::Included {
                    tables: btreemap! {
                        "users".parse()? => StreamingExportTableSelection::Included (
                            StreamingExportColumnSelection::new(
                                btreemap! {
                                    "password".parse()? => StreamingExportColumnInclusion::Excluded,
                                    "_creationTime".parse()? => StreamingExportColumnInclusion::Excluded,
                                },
                                StreamingExportInclusionDefault::Included,
                            )?,
                        ),
                    },
                    other_tables: StreamingExportInclusionDefault::Excluded,
                },
            },
            other_components: StreamingExportInclusionDefault::Excluded,
        },
        ..Default::default()
    };

    let deltas = db
        .document_deltas(Identity::system(), None, filter, 200, 3)
        .await?;

    let partial_delta = StreamingExportDocument::new(
        user_id.into(),
        PII(assert_obj!(
            "_id" => user_id.to_string(),
            "user" => "joe",
        )),
    )?;

    assert_eq!(
        deltas.deltas,
        vec![(
            ts,
            user_id.into(),
            ComponentPath::root(),
            "users".parse()?,
            Some(partial_delta)
        )]
    );

    Ok(())
}

#[convex_macro::test_runtime]
async fn test_document_deltas_usage_tracking(rt: TestRuntime) -> anyhow::Result<()> {
    let DbFixtures { db, .. } = DbFixtures::new(&rt).await?;

    // Insert documents with known sizes
    let mut tx = db.begin(Identity::system()).await?;
    let doc1_resolved = TestFacingModel::new(&mut tx)
        .insert_and_get(
            "table1".parse()?,
            assert_obj!(
                "field1" => "value1",
                "field2" => "value2",
            ),
        )
        .await?;
    let doc1_size = doc1_resolved.size();

    let doc2_resolved = TestFacingModel::new(&mut tx)
        .insert_and_get(
            "table1".parse()?,
            assert_obj!(
                "field1" => "longer_value_here",
                "field2" => "another_value",
                "field3" => "extra_field",
            ),
        )
        .await?;
    let doc2_size = doc2_resolved.size();

    let doc3_resolved = TestFacingModel::new(&mut tx)
        .insert_and_get(
            "table2".parse()?,
            assert_obj!(
                "name" => "test",
            ),
        )
        .await?;
    let doc3_size = doc3_resolved.size();

    let table_mapping = tx.table_mapping().clone();
    db.commit(tx).await?;

    // Fetch all deltas and verify usage tracking
    let DocumentDeltas { deltas, usage, .. } = db
        .document_deltas(
            Identity::system(),
            None,
            StreamingExportFilter::default(),
            200,
            200,
        )
        .await?;

    // Verify we got all 3 documents
    assert_eq!(deltas.len(), 3);

    // Verify usage stats
    let table1_name = table_mapping.tablet_name(doc1_resolved.id().tablet_id)?;
    let table2_name = table_mapping.tablet_name(doc3_resolved.id().tablet_id)?;

    // Check table1 usage (2 documents)
    let table1_egress = usage
        .database_egress_v2
        .get(&(ComponentPath::root(), table1_name.to_string()))
        .copied()
        .unwrap_or(0);
    let table1_rows = usage
        .database_egress_rows
        .get(&(ComponentPath::root(), table1_name.to_string()))
        .copied()
        .unwrap_or(0);

    assert_eq!(
        table1_egress,
        (doc1_size + doc2_size) as u64,
        "Table1 egress size should match sum of document sizes"
    );
    assert_eq!(table1_rows, 2, "Table1 should have 2 rows");

    // Check table2 usage (1 document)
    let table2_egress = usage
        .database_egress_v2
        .get(&(ComponentPath::root(), table2_name.to_string()))
        .copied()
        .unwrap_or(0);
    let table2_rows = usage
        .database_egress_rows
        .get(&(ComponentPath::root(), table2_name.to_string()))
        .copied()
        .unwrap_or(0);

    assert_eq!(
        table2_egress, doc3_size as u64,
        "Table2 egress size should match document size"
    );
    assert_eq!(table2_rows, 1, "Table2 should have 1 row");

    // Verify total bandwidth
    let total_egress: u64 = usage.database_egress_v2.values().sum();
    let total_rows: u64 = usage.database_egress_rows.values().sum();

    assert_eq!(
        total_egress,
        (doc1_size + doc2_size + doc3_size) as u64,
        "Total egress should match sum of all document sizes"
    );
    assert_eq!(total_rows, 3, "Total rows should be 3");

    // Verify that database_egress (v1) is not used
    let total_egress_v1: u64 = usage.database_egress.values().sum();
    assert_eq!(
        total_egress_v1, 0,
        "database_egress_size (v1) should not be used for streaming export"
    );

    Ok(())
}

#[convex_macro::test_runtime]
async fn test_list_snapshot_usage_tracking(rt: TestRuntime) -> anyhow::Result<()> {
    let DbFixtures { db, .. } = DbFixtures::new(&rt).await?;
    let mut tx = db.begin(Identity::system()).await?;

    // Insert documents with varying sizes in the same table
    let doc1_resolved = TestFacingModel::new(&mut tx)
        .insert_and_get(
            "table1".parse()?,
            assert_obj!(
                "field1" => "value1",
                "field2" => "value2",
            ),
        )
        .await?;
    let doc1_size = doc1_resolved.size();

    let doc2_resolved = TestFacingModel::new(&mut tx)
        .insert_and_get(
            "table1".parse()?,
            assert_obj!(
                "field1" => "longer_value_here",
                "field2" => "another_value",
                "field3" => "extra_field",
            ),
        )
        .await?;
    let doc2_size = doc2_resolved.size();

    let doc3_resolved = TestFacingModel::new(&mut tx)
        .insert_and_get(
            "table1".parse()?,
            assert_obj!(
                "name" => "test",
                "other_field" => "more data",
            ),
        )
        .await?;
    let doc3_size = doc3_resolved.size();

    let table_mapping = tx.table_mapping().clone();
    let ts = db.commit(tx).await?;

    // Fetch snapshot and verify usage tracking
    let SnapshotPage {
        documents, usage, ..
    } = db
        .list_snapshot(
            Identity::system(),
            Some(ts),
            None,
            StreamingExportFilter::default(),
            200,
            200,
        )
        .await?;

    // Verify we got all 3 documents
    assert_eq!(documents.len(), 3);

    // Verify usage stats
    let table1_name = table_mapping.tablet_name(doc1_resolved.id().tablet_id)?;

    // Check table1 usage (3 documents)
    let table1_egress = usage
        .database_egress_v2
        .get(&(ComponentPath::root(), table1_name.to_string()))
        .copied()
        .unwrap_or(0);
    let table1_rows = usage
        .database_egress_rows
        .get(&(ComponentPath::root(), table1_name.to_string()))
        .copied()
        .unwrap_or(0);

    assert_eq!(
        table1_egress,
        (doc1_size + doc2_size + doc3_size) as u64,
        "Table1 egress size should match sum of all document sizes"
    );
    assert_eq!(table1_rows, 3, "Table1 should have 3 rows");

    // Verify total bandwidth
    let total_egress: u64 = usage.database_egress_v2.values().sum();
    let total_rows: u64 = usage.database_egress_rows.values().sum();

    assert_eq!(
        total_egress,
        (doc1_size + doc2_size + doc3_size) as u64,
        "Total egress should match sum of all document sizes"
    );
    assert_eq!(total_rows, 3, "Total rows should be 3");

    // Verify that database_egress (v1) is not used
    let total_egress_v1: u64 = usage.database_egress.values().sum();
    assert_eq!(
        total_egress_v1, 0,
        "database_egress_size (v1) should not be used for streaming export"
    );

    Ok(())
}

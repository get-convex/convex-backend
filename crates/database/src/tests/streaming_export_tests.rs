use common::{
    assert_obj,
    components::ComponentPath,
    document::ResolvedDocument,
    pii::PII,
    types::TableName,
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
        deltas,
        DocumentDeltas {
            deltas: vec![
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
            cursor: ts2,
            has_more: false,
        },
    );

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
        deltas_cursor,
        DocumentDeltas {
            deltas: vec![(
                ts2,
                doc3.developer_id(),
                ComponentPath::root(),
                table_mapping.tablet_name(doc3.id().tablet_id)?,
                Some(StreamingExportDocument::with_all_fields(doc3.clone()))
            )],
            cursor: ts2,
            has_more: false,
        },
    );

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
        deltas_table_filter,
        DocumentDeltas {
            deltas: vec![(
                ts1,
                doc1.developer_id(),
                ComponentPath::root(),
                table_mapping.tablet_name(doc1.id().tablet_id)?,
                Some(StreamingExportDocument::with_all_fields(doc1.clone()))
            )],
            cursor: ts2,
            has_more: false
        },
    );

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
        deltas_limit,
        DocumentDeltas {
            deltas: vec![
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
            cursor: ts1,
            has_more: true,
        },
    );

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
        deltas,
        DocumentDeltas {
            deltas: vec![(
                ts_insert,
                remaining_doc.developer_id(),
                ComponentPath::root(),
                table_mapping.tablet_name(remaining_doc.id().tablet_id)?,
                Some(StreamingExportDocument::with_all_fields(
                    remaining_doc.clone()
                ))
            ),],
            cursor: ts_latest,
            has_more: false,
        },
    );

    Ok(())
}

#[convex_macro::test_runtime]
async fn test_snapshot_list(rt: TestRuntime) -> anyhow::Result<()> {
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

    let db_ = db.clone();
    let snapshot_list_all =
        move |mut snapshot: Option<Timestamp>,
              table_filter: Option<TableName>,
              mut cursor: Option<ResolvedDocumentId>| {
            let db = db_.clone();
            async move {
                let mut has_more = true;
                let mut documents = Vec::new();
                let mut pages = 0;
                while has_more && pages < 10 {
                    let page = db
                        .clone()
                        .list_snapshot(
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
                        .await?;
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
            }
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
        snapshot_has_more,
        SnapshotPage {
            documents: to_snapshot_docs(vec![docs1sorted[0].clone()]),
            snapshot: ts1,
            cursor: Some(docs1sorted[0].3.id()),
            has_more: true,
        },
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

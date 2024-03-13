use common::{
    assert_obj,
    types::TableName,
};
use keybroker::Identity;
use pretty_assertions::assert_eq;
use runtime::testing::TestRuntime;
use sync_types::Timestamp;
use value::id_v6::DocumentIdV6;

use crate::{
    test_helpers::DbFixtures,
    DocumentDeltas,
    SnapshotPage,
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
        .document_deltas(Identity::system(), None, None, 200, 3)
        .await?;
    assert_eq!(
        deltas,
        DocumentDeltas {
            deltas: vec![
                (
                    ts1,
                    doc1sort.id_v6(),
                    table_mapping.tablet_name(doc1sort.table().table_id)?,
                    Some(doc1sort.clone())
                ),
                (
                    ts1,
                    doc2sort.id_v6(),
                    table_mapping.tablet_name(doc2sort.table().table_id)?,
                    Some(doc2sort.clone())
                ),
                (
                    ts2,
                    doc3.id_v6(),
                    table_mapping.tablet_name(doc3.table().table_id)?,
                    Some(doc3.clone())
                ),
            ],
            cursor: ts2,
            has_more: false,
        },
    );

    let deltas_cursor = db
        .document_deltas(Identity::system(), Some(ts1), None, 200, 3)
        .await?;
    assert_eq!(
        deltas_cursor,
        DocumentDeltas {
            deltas: vec![(
                ts2,
                doc3.id_v6(),
                table_mapping.tablet_name(doc3.table().table_id)?,
                Some(doc3.clone())
            )],
            cursor: ts2,
            has_more: false,
        },
    );

    let deltas_table_filter = db
        .document_deltas(Identity::system(), None, Some("table1".parse()?), 200, 3)
        .await?;
    assert_eq!(
        deltas_table_filter,
        DocumentDeltas {
            deltas: vec![(
                ts1,
                doc1.id_v6(),
                table_mapping.tablet_name(doc1.table().table_id)?,
                Some(doc1.clone())
            )],
            cursor: ts2,
            has_more: false
        },
    );

    // Note we're requesting 1 result, but in order to return the full transaction
    // we receive 2 deltas.
    let deltas_limit = db
        .document_deltas(Identity::system(), None, None, 200, 1)
        .await?;
    assert_eq!(
        deltas_limit,
        DocumentDeltas {
            deltas: vec![
                (
                    ts1,
                    doc1sort.id_v6(),
                    table_mapping.tablet_name(doc1sort.table().table_id)?,
                    Some(doc1sort.clone())
                ),
                (
                    ts1,
                    doc2sort.id_v6(),
                    table_mapping.tablet_name(doc2sort.table().table_id)?,
                    Some(doc2sort.clone())
                ),
            ],
            cursor: ts1,
            has_more: true,
        },
    );

    let deltas_auth = db
        .document_deltas(Identity::Unknown, None, None, 200, 3)
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
    UserFacingModel::new(&mut tx)
        .insert("table".parse()?, assert_obj!())
        .await?;
    db.commit(tx).await?;

    // …and then delete its table…
    let mut tx = db.begin(Identity::system()).await?;
    let mut model = TableModel::new(&mut tx);
    model.delete_table("table".parse()?).await?;
    db.commit(tx).await?;

    // …then the row should not appear in the results returned by document_deltas.
    let deltas = db
        .document_deltas(Identity::system(), None, None, 200, 3)
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
    UserFacingModel::new(&mut tx)
        .insert("table2".parse()?, assert_obj!())
        .await?;
    let ts_insert = db.commit(tx).await?;

    // …and then delete one of the tables…
    let mut tx = db.begin(Identity::system()).await?;
    let mut model = TableModel::new(&mut tx);
    model.delete_table("table2".parse()?).await?;
    let table_mapping = tx.table_mapping().clone();
    let ts_latest = db.commit(tx).await?;

    // …then only one row should appear in the results returned by document_deltas.
    let deltas = db
        .document_deltas(Identity::system(), None, None, 200, 3)
        .await?;
    assert_eq!(
        deltas,
        DocumentDeltas {
            deltas: vec![(
                ts_insert,
                remaining_doc.id_v6(),
                table_mapping.tablet_name(remaining_doc.table().table_id)?,
                Some(remaining_doc.clone())
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
        (ts1, "table1".parse()?, doc1.clone()),
        (ts1, "table2".parse()?, doc2.clone()),
    ];
    docs1sorted.sort_by_key(|(_, _, d)| *d.id());
    let mut tx = db.begin(Identity::system()).await?;
    let doc3 = TestFacingModel::new(&mut tx)
        .insert_and_get("table3".parse()?, assert_obj!("f" => 3))
        .await?;
    let doc4 = UserFacingModel::new(&mut tx)
        .patch((*doc2.id()).into(), assert_obj!("f" => 4).into())
        .await?;
    let doc4 = doc4.to_resolved(&tx.table_mapping().inject_table_id())?;
    let ts2 = db.commit(tx).await?;
    let mut docs2sorted = vec![
        (ts1, "table1".parse()?, doc1),
        (ts2, "table2".parse()?, doc4.clone()),
        (ts2, "table3".parse()?, doc3),
    ];
    docs2sorted.sort_by_key(|(_, _, d)| *d.id());

    let db_ = db.clone();
    let snapshot_list_all = move |mut snapshot: Option<Timestamp>,
                                  table_filter: Option<TableName>,
                                  mut cursor: Option<DocumentIdV6>| {
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
                        cursor.map(DocumentIdV6::from),
                        table_filter.clone(),
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

    let snapshot_page = snapshot_list_all(None, None, None).await?;
    assert_eq!(snapshot_page.0, docs2sorted);
    assert_eq!(snapshot_page.1, ts2);

    let snapshot_explicit_ts = snapshot_list_all(Some(ts2), None, None).await?;
    assert_eq!(snapshot_explicit_ts.0, docs2sorted);
    assert_eq!(snapshot_explicit_ts.1, ts2);

    let snapshot_table_filter = snapshot_list_all(None, Some("table2".parse()?), None).await?;
    assert_eq!(
        snapshot_table_filter.0,
        vec![(ts2, "table2".parse()?, doc4)]
    );
    assert_eq!(snapshot_table_filter.1, ts2);

    let snapshot_old = snapshot_list_all(Some(ts1), None, None).await?;
    assert_eq!(snapshot_old.0, docs1sorted);
    assert_eq!(snapshot_old.1, ts1);

    let snapshot_has_more = db
        .list_snapshot(Identity::system(), Some(ts1), None, None, 100, 1)
        .await?;
    assert_eq!(
        snapshot_has_more,
        SnapshotPage {
            documents: vec![docs1sorted[0].clone()],
            snapshot: ts1,
            cursor: Some(docs1sorted[0].2.id_v6()),
            has_more: true,
        },
    );

    let snapshot_cursor =
        snapshot_list_all(Some(ts1), None, Some(docs1sorted[0].2.id_v6())).await?;
    assert_eq!(snapshot_cursor.0, vec![docs1sorted[1].clone()]);
    assert_eq!(snapshot_cursor.1, ts1);

    let snapshot_auth = db
        .list_snapshot(Identity::Unknown, None, None, None, 100, 3)
        .await;
    assert!(snapshot_auth.is_err());

    Ok(())
}

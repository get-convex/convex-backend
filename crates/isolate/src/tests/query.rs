use std::collections::BTreeMap;

use common::{
    assert_obj,
    bootstrap_model::index::{
        database_index::IndexedFields,
        IndexMetadata,
    },
    persistence::Persistence,
    query::Cursor,
    runtime::Runtime,
    testing::TestPersistence,
    types::PersistenceVersion,
    value::{
        ConvexArray,
        ConvexValue,
        Size,
    },
};
use keybroker::Identity;
use must_let::must_let;
use pretty_assertions::assert_eq;
use runtime::testing::TestRuntime;
use value::{
    id_v6::DocumentIdV6,
    ConvexObject,
};

use super::assert_contains;
use crate::{
    test_helpers::UdfTest,
    UdfOutcome,
};

async fn add_index<RT: Runtime, P: Persistence + Clone>(t: &UdfTest<RT, P>) -> anyhow::Result<()> {
    t.add_index(IndexMetadata::new_backfilling(
        "myTable.by_a_b".parse()?,
        IndexedFields::try_from(vec!["a".parse()?, "b".parse()?])?,
    ))
    .await
}

#[convex_macro::test_runtime]
async fn test_full_table_scan(rt: TestRuntime) -> anyhow::Result<()> {
    let t = UdfTest::default(rt).await?;
    add_index(&t).await?;
    t.mutation("query:insert", assert_obj!( "number" => 1))
        .await?;

    must_let!(let ConvexValue::Array(r) = t.query("query:filterScan", assert_obj!( "number" => 1)).await?);
    assert_eq!(r.len(), 1);

    // Confirm that an explicit `fullTableScan` does the same thing
    must_let!(let ConvexValue::Array(ft) = t.query("query:explicitScan", assert_obj!( "number" => 1)).await?);
    assert_eq!(r.len(), ft.len());

    must_let!(let ConvexValue::Array(r) = t.query("query:filterScan", assert_obj!( "number" => 2)).await?);
    assert_eq!(r.len(), 0);
    Ok(())
}

#[convex_macro::test_runtime]
async fn test_boolean_value_filters(rt: TestRuntime) -> anyhow::Result<()> {
    let t = UdfTest::default(rt).await?;
    t.mutation("query:insert", assert_obj!( "number" => 1))
        .await?;

    must_let!(let ConvexValue::Array(true_result) = t.query("query:trueLiteralFilter", assert_obj!()).await?);
    assert_eq!(true_result.len(), 1);

    must_let!(let ConvexValue::Array(false_result) = t.query("query:falseLiteralFilter", assert_obj!()).await?);
    assert_eq!(false_result.len(), 0);

    Ok(())
}

#[convex_macro::test_runtime]
async fn test_index(rt: TestRuntime) -> anyhow::Result<()> {
    let t = UdfTest::default(rt).await?;
    add_index(&t).await?;
    t.backfill_indexes().await?;

    must_let!(let ConvexValue::Array(r) = t.query("indexing:oneFieldEquality", assert_obj!("a" => 1)).await?);
    assert_eq!(r.len(), 0);

    t.mutation("indexing:insert", assert_obj!("a" => 1, "b" => 1))
        .await?;

    must_let!(let ConvexValue::Array(r) = t.query("indexing:oneFieldEquality", assert_obj!("a" => 1)).await?);
    assert_eq!(r.len(), 1);

    Ok(())
}

#[convex_macro::test_runtime]
async fn test_index_backfill(rt: TestRuntime) -> anyhow::Result<()> {
    let t = UdfTest::default(rt).await?;
    t.mutation("indexing:insert", assert_obj!("a" => 1, "b" => 1))
        .await?;

    // Create the index *after* inserting the document to test backfill.
    add_index(&t).await?;
    t.backfill_indexes().await?;

    must_let!(let ConvexValue::Array(r) = t.query("indexing:oneFieldEquality", assert_obj!("a" => 1)).await?);
    assert_eq!(r.len(), 1);

    Ok(())
}

#[convex_macro::test_runtime]
async fn test_index_backfill_error(rt: TestRuntime) -> anyhow::Result<()> {
    let t = UdfTest::default(rt).await?;

    // Create the index and don't backfill it.
    add_index(&t).await?;

    let error = t
        .query_js_error("indexing:oneFieldEquality", assert_obj!("a" => 1))
        .await?;
    assert_contains(
        &error,
        "Index myTable.by_a_b is currently backfilling and not available to query yet.",
    );

    Ok(())
}

#[convex_macro::test_runtime]
async fn test_index_ranges(rt: TestRuntime) -> anyhow::Result<()> {
    let t = UdfTest::default(rt).await?;
    add_index(&t).await?;
    t.backfill_indexes().await?;

    for a in 1..6 {
        t.mutation("indexing:insertMissingField", assert_obj!("a" => a))
            .await?;
        t.mutation("indexing:insert", assert_obj!("a" => a, "b" => null))
            .await?;
        for b in 1..6 {
            t.mutation("indexing:insert", assert_obj!("a" => a, "b" => b))
                .await?;
        }
    }

    // Don't make any of your tests too long or index backfill will segfault
    // for *mystery reasons*
    let checks = Box::pin(async {
        // There are 35 items in the index total.
        must_let!(let ConvexValue::Array(r) = t.query("indexing:allItemsInIndex", assert_obj!()).await?);
        assert_eq!(r.len(), 35);

        // 6 items have a=1
        must_let!(let ConvexValue::Array(r) = t.query("indexing:oneFieldEquality", assert_obj!("a" => 1)).await?);
        assert_eq!(r.len(), 7);

        // 1 item has a=1 b=2
        must_let!(let ConvexValue::Array(r) = t.query(
                "indexing:twoFieldEquality",

                assert_obj!("a" => 1, "b" => 2)

            ).await?);
        assert_eq!(r.len(), 1);

        // 1 item has a=1 b=missing, 1 has a=1 b=null
        must_let!(let ConvexValue::Array(r) = t.query(
                "indexing:twoFieldEquality",
                assert_obj!("a" => 1, "b" => null)
            ).await?);
        assert_eq!(r.len(), 1);
        must_let!(let ConvexValue::Array(r) = t.query(
                "indexing:twoFieldEqualityExplicitMissing",
                assert_obj!("a" => 1)
            ).await?);
        assert_eq!(r.len(), 1);

        // Check parity with filters.
        must_let!(let ConvexValue::Array(r) = t.query(
                "indexing:twoFieldFilterEquality",
                assert_obj!("a" => 1, "b" => null)
            ).await?);
        assert_eq!(r.len(), 1);
        must_let!(let ConvexValue::Array(r) = t.query(
                "indexing:twoFieldFilterEqualityExplicitMissing",
                assert_obj!("a" => 1)
            ).await?);
        assert_eq!(r.len(), 1);

        must_let!(let ConvexValue::Array(r) = t.query(
                "indexing:twoFieldEqualityOutOfOrder",
                assert_obj!("a" => 1, "b" => 2)
            ).await?);
        assert_eq!(r.len(), 1);

        // 7 items have 2<a<4
        must_let!(let ConvexValue::Array(r) = t.query(
                "indexing:exclusiveRangeOnFirstField",
                assert_obj!("aStart" => 2, "aEnd" => 4)
            ).await?);
        assert_eq!(r.len(), 7);

        // 21 items have 2<=a<=4
        must_let!(let ConvexValue::Array(r) = t.query(
                "indexing:inclusiveRangeOnFirstField",
                assert_obj!("aStart" => 2, "aEnd" => 4)
            ).await?);
        assert_eq!(r.len(), 21);

        // 1 item has a=1 2<b<4
        must_let!(let ConvexValue::Array(r) = t.query(
                "indexing:exclusiveRangeOnSecondField",
                assert_obj!("a" => 1, "bStart" => 2,  "bEnd" => 4)
            ).await?);
        assert_eq!(r.len(), 1);

        // 3 items have a=1 2<=b<=4
        must_let!(let ConvexValue::Array(r) = t.query(
                "indexing:inclusiveRangeOnSecondField",
                assert_obj!("a" => 1, "bStart" => 2,  "bEnd" => 4)

            ).await?);
        assert_eq!(r.len(), 3);

        must_let!(let ConvexValue::Array(r) = t.query(
                "indexing:rangeOnSecondFieldOutOfOrder",
                assert_obj!("a" => 1, "bStart" => 2,  "bEnd" => 4)
            ).await?);
        assert_eq!(r.len(), 3);
        Ok::<(), anyhow::Error>(())
    });
    checks.await?;

    Ok(())
}

#[convex_macro::test_runtime]
async fn test_index_range_errors(rt: TestRuntime) -> anyhow::Result<()> {
    async fn assert_error_contains(
        t: &UdfTest<TestRuntime, TestPersistence>,
        udf_path: &str,
        expected: &str,
    ) {
        let error = t.query_js_error(udf_path, assert_obj!()).await.unwrap();

        assert!(
            format!("{}", error).contains(expected),
            "\nExpected: {expected}\nActual: {error}"
        );
    }

    let t = UdfTest::default(rt).await?;

    assert_error_contains(
        &t,
        "indexing:allItemsInIndex",
        "Index myTable.by_a_b not found.",
    )
    .await;

    add_index(&t).await?;
    assert_error_contains(
        &t,
        "indexing:invalidIndexRange",
        "Index myTable.by_a_b is currently backfilling and not available to query yet.",
    )
    .await;
    t.backfill_indexes().await?;

    assert_error_contains(
        &t,
        "indexing:invalidIndexRange",
        "Uncaught Error: Tried to query index myTable.by_a_b but the query didn't use the index \
         fields in order.",
    )
    .await;
    assert_error_contains(
        &t,
        "indexing:eqFieldNotInIndex",
        "Uncaught Error: The index range included a comparison with \"c\", but myTable.by_a_b \
         with fields [\"a\", \"b\"] doesn't index this field.",
    )
    .await;
    assert_error_contains(
        &t,
        "indexing:ltFieldNotInIndex",
        "Uncaught Error: The index range included a comparison with \"c\", but myTable.by_a_b \
         with fields [\"a\", \"b\"] doesn't index this field.",
    )
    .await;
    assert_error_contains(
        &t,
        "indexing:defineBoundsTwice",
        "Already defined lower bound in index range. Can't add \"a\" >= 1.",
    )
    .await;
    assert_error_contains(
        &t,
        "indexing:defineEqualityBoundsTwice",
        "Already defined equality bound in index range. Can't add \"a\" == 2.0.",
    )
    .await;
    assert_error_contains(
        &t,
        "indexing:equalityAndInequalityOverlap",
        "Already defined inequality bound in index range. Can't add \"a\" == 2.0.",
    )
    .await;
    assert_error_contains(
        &t,
        "indexing:boundsOnDifferentFields",
        "Upper and lower bounds in `range` can only be applied to a single index field. This \
         query against index myTable.by_a_b attempted to set a range bound on both \"a\" and \
         \"b\".",
    )
    .await;

    Ok(())
}

fn pagination_opts(cursor: ConvexValue) -> ConvexObject {
    assert_obj!("paginationOpts" => ConvexValue::Object(assert_obj!(
        "cursor" => cursor,
        "numItems" => 1.0,
    )))
}

#[convex_macro::test_runtime]
async fn test_pagination(rt: TestRuntime) -> anyhow::Result<()> {
    let t = UdfTest::default(rt).await?;
    // First, paginate through an empty table.
    must_let!(let ConvexValue::Object(o) = t.query("query:paginateTableScan", pagination_opts(ConvexValue::Null)).await?);
    must_let!(let Some(ConvexValue::Array(page)) = o.get("page"));
    assert_eq!(page.len(), 0);
    must_let!(let Some(ConvexValue::Boolean(true)) = o.get("isDone"));
    must_let!(let Some(ConvexValue::String(done_cursor)) = o.get("continueCursor"));

    // Listing the table with the finished cursor should still return empty results.
    must_let!(let ConvexValue::Object(o) = t.query("query:paginateTableScan", pagination_opts(ConvexValue::try_from(done_cursor.to_string())?)).await?);
    must_let!(let Some(ConvexValue::Array(page)) = o.get("page"));
    assert_eq!(page.len(), 0);
    must_let!(let Some(ConvexValue::Boolean(true)) = o.get("isDone"));
    must_let!(let Some(ConvexValue::String(_)) = o.get("continueCursor"));

    // Add some values and try again.
    let id1 = t
        .mutation("query:insert", assert_obj!("number" => 1))
        .await?;
    t.mutation("query:insert", assert_obj!("number" => 2))
        .await?;

    // Let's check that we can list out all of our values.
    must_let!(let ConvexValue::Object(o) = t.query("query:paginateTableScan", pagination_opts(ConvexValue::Null)).await?);
    must_let!(let Some(ConvexValue::Array(page)) = o.get("page"));
    assert_eq!(page.len(), 1);
    must_let!(let ConvexValue::Object(row) = &page[0]);
    must_let!(let Some(ConvexValue::Int64(1)) = row.get("hello"));
    must_let!(let Some(ConvexValue::Boolean(false)) = o.get("isDone"));
    must_let!(let Some(ConvexValue::String(continue_cursor)) = o.get("continueCursor"));

    must_let!(let ConvexValue::Object(o) = t.query("query:paginateTableScan", pagination_opts(ConvexValue::try_from(continue_cursor.to_string())?)).await?);
    must_let!(let Some(ConvexValue::Array(page)) = o.get("page"));
    assert_eq!(page.len(), 1);
    must_let!(let ConvexValue::Object(row) = &page[0]);
    must_let!(let Some(ConvexValue::Int64(2)) = row.get("hello"));
    must_let!(let Some(ConvexValue::Boolean(false)) = o.get("isDone"));
    must_let!(let Some(ConvexValue::String(continue_cursor)) = o.get("continueCursor"));

    must_let!(let ConvexValue::Object(o) = t.query("query:paginateTableScan", pagination_opts(ConvexValue::try_from(continue_cursor.to_string())?)).await?);

    must_let!(let Some(ConvexValue::Array(page)) = o.get("page"));
    assert_eq!(page.len(), 0);
    must_let!(let Some(ConvexValue::Boolean(true)) = o.get("isDone"));

    // Listing the first cursor should still not produce results because it's
    // at the end of the table.
    must_let!(let ConvexValue::Object(o) = t.query("query:paginateTableScan", pagination_opts(ConvexValue::try_from(done_cursor.to_string())?)).await?);
    must_let!(let Some(ConvexValue::Array(page)) = o.get("page"));
    assert_eq!(page.len(), 0);
    must_let!(let Some(ConvexValue::Boolean(true)) = o.get("isDone"));
    must_let!(let Some(ConvexValue::String(_)) = o.get("continueCursor"));

    // List and filter to the Id.
    must_let!(let ConvexValue::Object(o) = t.query("query:paginateFilterTableScan", assert_obj!("cursor" => ConvexValue::Null, "id" => id1)).await?);
    must_let!(let Some(ConvexValue::Array(page)) = o.get("page"));
    assert_eq!(page.len(), 1);
    must_let!(let ConvexValue::Object(row) = &page[0]);
    must_let!(let Some(ConvexValue::Int64(1)) = row.get("hello"));
    must_let!(let Some(ConvexValue::Boolean(false)) = o.get("isDone"));

    Ok(())
}

/// Tests for the `maximumBytesRead` pagination option.
#[convex_macro::test_runtime]
async fn test_pagination_max_bytes_read(rt: TestRuntime) -> anyhow::Result<()> {
    let t: UdfTest<TestRuntime, TestPersistence> = UdfTest::default(rt).await?;

    let object = assert_obj!("number" => 1);
    must_let!(let ConvexValue::String(id1) = t.mutation("query:insert", object.clone()).await?);
    must_let!(let ConvexValue::String(id2) = t.mutation("query:insert", object.clone()).await?);
    must_let!(let ConvexValue::String(id3) = t.mutation("query:insert", object.clone()).await?);
    must_let!(let ConvexValue::String(id4) = t.mutation("query:insert", object.clone()).await?);
    must_let!(let ConvexValue::String(id5) = t.mutation("query:insert", object.clone()).await?);

    let expected = vec![
        DocumentIdV6::decode(&id1)?,
        DocumentIdV6::decode(&id2)?,
        DocumentIdV6::decode(&id3)?,
        DocumentIdV6::decode(&id4)?,
        DocumentIdV6::decode(&id5)?,
    ];

    async fn read_to_end(
        t: &UdfTest<TestRuntime, TestPersistence>,
        max_bytes_read: usize,
    ) -> anyhow::Result<(Vec<DocumentIdV6>, usize)> {
        let mut results = Vec::new();
        let mut num_pages = 0;
        let mut cursor = ConvexValue::Null;
        let mut is_done = false;
        while !is_done {
            let args = assert_obj!("paginationOpts" => {
                "cursor" => cursor.clone(),
                // numItems is large enough to fit all the documents on one page.
                "numItems" => 100.0,
                "maximumBytesRead" => max_bytes_read as f64,
            });
            must_let!(let ConvexValue::Object(result) = t.query("query:paginateWithOpts", args).await?);
            must_let!(let Some(ConvexValue::Boolean(new_is_done)) = result.get("isDone"));
            is_done = *new_is_done;
            must_let!(let Some(new_cursor) = result.get("continueCursor"));
            cursor = new_cursor.clone();
            must_let!(let Some(ConvexValue::Array(page)) = result.get("page"));
            num_pages += 1;
            for value in page.into_iter() {
                must_let!(let ConvexValue::Object(object) = value);
                must_let!(let Some(ConvexValue::String(id)) = object.get("_id"));
                results.push(DocumentIdV6::decode(id)?);
            }
        }
        Ok((results, num_pages))
    }

    // max_bytes_read = 1 (less than 1 document) -> each result is on its own page.
    let (results, num_pages) = read_to_end(&t, 1).await?;
    assert_eq!(num_pages, 6);
    assert_eq!(results, expected);

    // max_bytes_read = 10,000 (more than all documents) -> all results are on a
    // single page.
    let (results, num_pages) = read_to_end(&t, 10000).await?;
    assert_eq!(num_pages, 1);
    assert_eq!(results, expected);

    // max_bytes_read = 2 * 1 object's size -> pages contain a few results each.
    let first_object = t
        .query("query:get", assert_obj!("id" => ConvexValue::String(id1)))
        .await?;
    let max_bytes_read = first_object.size() * 2;
    let (results, num_pages) = read_to_end(&t, max_bytes_read).await?;
    // don't assert on the exact num_pages because sizes could change over time.
    assert!(1 < num_pages && num_pages < 5);
    assert_eq!(results, expected);
    Ok(())
}

#[convex_macro::test_runtime]
async fn test_invalid_cursor_error(rt: TestRuntime) -> anyhow::Result<()> {
    let t = UdfTest::default(rt).await?;
    // Get a cursor from `paginateTableScan`.
    must_let!(let ConvexValue::Object(o) = t.query("query:paginateTableScan",  pagination_opts(ConvexValue::Null)).await?);
    must_let!(let Some(cursor) = o.get("continueCursor"));

    // Trying to reuse it in a different query should produce an error.

    let e = t
        .query_js_error(
            "query:paginateReverseTableScan",
            pagination_opts(cursor.clone()),
        )
        .await?;
    assert_contains(&e, "InvalidCursor");
    Ok(())
}

#[convex_macro::test_runtime]
async fn test_multiple_paginated_queries_error(rt: TestRuntime) -> anyhow::Result<()> {
    let t = UdfTest::default(rt).await?;
    let e = t
        .query_js_error("query:multiplePaginatedQueries", assert_obj!())
        .await?;
    assert_contains(
        &e,
        "Uncaught Error: This query or mutation function ran multiple paginated queries. Convex \
         only supports a single paginated query in each function.",
    );
    Ok(())
}

/// Assert that these produce a the same result + query journal:
/// 1. A UDF that runs a paginated query with no query journal.
/// 2. The same UDF + args with the journal produced in (1).
/// This ensures that our journaling is deterministic.
///
/// 3. Then we insert an object which may be within the page, and asserts the
/// object is returned or not (so the journal is actually doing something).
/// Then we delete the object.
///
/// Returns (page_of_results, is_done)
pub async fn assert_paginated_query_journal_is_correct(
    t: &UdfTest<TestRuntime, TestPersistence>,
    udf_path: &str,
    args: ConvexObject,
    middle_objects: Vec<(&'static str, ConvexObject, bool)>,
) -> anyhow::Result<(ConvexArray, bool)> {
    let outcome1 = t
        .raw_query(
            udf_path,
            vec![ConvexValue::Object(args.clone())],
            Identity::system(),
            None,
        )
        .await?;
    let outcome2 = t
        .raw_query(
            udf_path,
            vec![ConvexValue::Object(args.clone())],
            Identity::system(),
            Some(outcome1.journal.clone()),
        )
        .await?;
    assert_eq!(outcome1.journal, outcome2.journal);

    // Annoyingly our cursors aren't deterministic because they use a random
    // nonce during encryption, so we can't assert that `result1 == result2`.
    // Instead assert that each field matches individually and decrypt the cursors
    // before asserting.

    let (page1, is_done1, cursor1, cursor_string) = unpack_pagination_result(t, &outcome1);
    let (page2, is_done2, cursor2, _) = unpack_pagination_result(t, &outcome2);

    assert_eq!((&page1, is_done1, &cursor1), (&page2, is_done2, &cursor2));

    for (insert_udf, middle_object, middle_object_in_page) in middle_objects {
        let middle_id = t.mutation(insert_udf, middle_object).await?;
        let outcome3 = t
            .raw_query(
                udf_path,
                vec![ConvexValue::Object(args.clone())],
                Identity::system(),
                Some(outcome1.journal.clone()),
            )
            .await?;
        assert_eq!(outcome1.journal, outcome3.journal);
        let (page3, is_done3, cursor3, _) = unpack_pagination_result(t, &outcome3);
        assert_eq!(is_done3, is_done1);
        assert_eq!(cursor3, cursor1);
        if middle_object_in_page {
            assert_eq!(page3.len(), page1.len() + 1);
        } else {
            assert_eq!(page3, page1);
        }

        // Check endCursor works in place of query journal
        let mut args: BTreeMap<_, _> = args.clone().into();
        let pagination_opts_val = args.remove("paginationOpts").unwrap();
        must_let!(let ConvexValue::Object(pagination_opts_obj) = pagination_opts_val);
        let mut pagination_opts: BTreeMap<_, _> = pagination_opts_obj.into();
        pagination_opts.insert(
            "endCursor".parse()?,
            ConvexValue::try_from(cursor_string.clone())?,
        );
        args.insert(
            "paginationOpts".parse()?,
            ConvexValue::Object(pagination_opts.try_into()?),
        );
        let outcome4 = t
            .raw_query(
                udf_path,
                vec![ConvexValue::Object(args.try_into()?)],
                Identity::system(),
                None,
            )
            .await?;
        assert_eq!(outcome1.journal, outcome4.journal);
        let (page4, is_done4, cursor4, _) = unpack_pagination_result(t, &outcome4);
        assert_eq!(is_done4, is_done1);
        assert_eq!(cursor4, cursor1);
        assert_eq!(page4, page3);

        t.mutation("query:deleteDoc", assert_obj!("id" => middle_id))
            .await?;
    }

    Ok((page1, is_done1))
}

fn unpack_pagination_result(
    t: &UdfTest<TestRuntime, TestPersistence>,
    outcome: &UdfOutcome,
) -> (ConvexArray, bool, Cursor, String) {
    must_let!(let ConvexValue::Object(output) = outcome.result.clone().unwrap().unpack());
    must_let!(let Some(ConvexValue::Array(page)) = output.get("page"));
    must_let!(let Some(ConvexValue::Boolean(is_done)) = output.get("isDone"));
    must_let!(let Some(ConvexValue::String(cursor_string)) = output.get("continueCursor"));
    let cursor = t
        .key_broker
        .decrypt_cursor(cursor_string.to_string(), PersistenceVersion::default())
        .unwrap();
    (page.clone(), *is_done, cursor, cursor_string.to_string())
}

#[convex_macro::test_runtime]
async fn test_query_journal_start_to_middle(rt: TestRuntime) -> anyhow::Result<()> {
    let t = UdfTest::default(rt).await?;
    // Add 2 documents.
    t.mutation("query:insert", assert_obj!("number" => 1))
        .await?;
    t.mutation("query:insert", assert_obj!("number" => 2))
        .await?;

    // Query begins at the start and ends after the first result.
    // Ordered by creation time ascending.
    // --- start cursor
    // 1
    // --- end cursor / journal
    // 2
    // 3 <- inserted later
    let (results, is_done) = assert_paginated_query_journal_is_correct(
        &t,
        "query:paginateTableScan",
        pagination_opts(ConvexValue::Null),
        vec![("query:insert", assert_obj!("number" => 3), false)],
    )
    .await?;
    assert_eq!(results.len(), 1);
    assert!(!is_done);

    // In the other direction, new documents are included in the page.
    // Ordered by creation time descending.
    // --- start cursor
    // 3 <- inserted later
    // 2
    // --- end cursor / journal
    // 1
    let (results, is_done) = assert_paginated_query_journal_is_correct(
        &t,
        "query:paginateReverseTableScan",
        pagination_opts(ConvexValue::Null),
        vec![("query:insert", assert_obj!("number" => 3), true)],
    )
    .await?;
    assert_eq!(results.len(), 1);
    assert!(!is_done);
    Ok(())
}

#[convex_macro::test_runtime]
async fn test_query_journal_start_to_end(rt: TestRuntime) -> anyhow::Result<()> {
    let t = UdfTest::default(rt).await?;
    // Query begins at the start and ends at the end.
    // Ordered by creation time ascending.
    // --- start cursor
    // 1 <- added later
    // --- end cursor / journal
    let (results, is_done) = assert_paginated_query_journal_is_correct(
        &t,
        "query:paginateTableScan",
        pagination_opts(ConvexValue::Null),
        vec![("query:insert", assert_obj!("number" => 1), true)],
    )
    .await?;
    assert_eq!(results.len(), 0);
    assert!(is_done);
    Ok(())
}

#[convex_macro::test_runtime]
async fn test_query_journal_middle_to_middle(rt: TestRuntime) -> anyhow::Result<()> {
    let t = UdfTest::default(rt).await?;
    t.add_index(IndexMetadata::new_backfilling(
        "test.by_hello".parse()?,
        IndexedFields::try_from(vec!["hello".parse()?])?,
    ))
    .await?;
    t.backfill_indexes().await?;

    t.mutation("query:insert", assert_obj!("number" => 1))
        .await?;
    t.mutation("query:insert", assert_obj!("number" => 3))
        .await?;
    t.mutation("query:insert", assert_obj!("number" => 5))
        .await?;

    // Run an initial query to get a cursor.
    must_let!(let ConvexValue::Object(initial_result) = t
        .query("query:paginateIndex", pagination_opts(ConvexValue::Null))
        .await?);
    must_let!(let Some(ConvexValue::Boolean(false)) = initial_result.get("isDone"));
    must_let!(let Some(ConvexValue::String(continue_cursor)) = initial_result.get("continueCursor"));

    // Query begins after the first document and ends after the second.
    // Ordered by number ascending.
    // 0 <- added later
    // 1
    // --- start cursor
    // 2 <- added later
    // 3
    // --- end cursor / journal
    // 4 <- added later
    let (results, is_done) = assert_paginated_query_journal_is_correct(
        &t,
        "query:paginateIndex",
        pagination_opts(ConvexValue::try_from(continue_cursor.to_string())?),
        vec![
            ("query:insert", assert_obj!("number" => 0), false),
            ("query:insert", assert_obj!("number" => 2), true),
            ("query:insert", assert_obj!("number" => 4), false),
        ],
    )
    .await?;
    assert_eq!(results.len(), 1);
    assert!(!is_done);
    Ok(())
}

#[convex_macro::test_runtime]
async fn test_query_journal_middle_to_end(rt: TestRuntime) -> anyhow::Result<()> {
    let t = UdfTest::default(rt).await?;
    t.mutation("query:insert", assert_obj!("number" => 1))
        .await?;

    // Run an initial query to get a cursor.
    must_let!(let ConvexValue::Object(initial_result) = t
        .query("query:paginateTableScan", pagination_opts(ConvexValue::Null))
        .await?);
    must_let!(let Some(ConvexValue::Boolean(false)) = initial_result.get("isDone"));
    must_let!(let Some(ConvexValue::String(continue_cursor)) = initial_result.get("continueCursor"));

    // Query begins at the document and ends at the end.
    // Ordered by creation time ascending.
    // 1
    // --- start cursor
    // 2 <- added later
    // --- end cursor / journal
    let (results, is_done) = assert_paginated_query_journal_is_correct(
        &t,
        "query:paginateTableScan",
        pagination_opts(ConvexValue::try_from(continue_cursor.to_string())?),
        vec![("query:insert", assert_obj!("number" => 2), true)],
    )
    .await?;
    assert_eq!(results.len(), 0);
    assert!(is_done);
    Ok(())
}

#[convex_macro::test_runtime]
async fn test_query_order_filter(rt: TestRuntime) -> anyhow::Result<()> {
    let t = UdfTest::default(rt).await?;
    t.mutation("query:insert", assert_obj!("number" => 1))
        .await?;
    t.mutation("query:insert", assert_obj!("number" => 2))
        .await?;
    t.mutation("query:insert", assert_obj!("number" => 3))
        .await?;

    let res = t
        .query("query:orderFilter", assert_obj!("min" => 2))
        .await?;
    must_let!(let ConvexValue::Array(arr) = res);
    assert_eq!(arr.len(), 2);
    must_let!(let ConvexValue::Object(obj0) = &arr[0]);
    must_let!(let ConvexValue::Object(obj1) = &arr[1]);
    assert_eq!(obj0.get("hello"), Some(&ConvexValue::from(3)));
    assert_eq!(obj1.get("hello"), Some(&ConvexValue::from(2)));

    let res = t
        .query("query:filterOrder", assert_obj!("min" => 2))
        .await?;
    must_let!(let ConvexValue::Array(arr) = res);
    assert_eq!(arr.len(), 2);
    must_let!(let ConvexValue::Object(obj0) = &arr[0]);
    must_let!(let ConvexValue::Object(obj1) = &arr[1]);
    assert_eq!(obj0.get("hello"), Some(&ConvexValue::from(3)));
    assert_eq!(obj1.get("hello"), Some(&ConvexValue::from(2)));

    let err = t.query_js_error("query:orderOrder", assert_obj!()).await?;
    assert!(err
        .to_string()
        .contains("Queries may only specify order at most once"));

    Ok(())
}

#[convex_macro::test_runtime]
async fn test_query_with_pending_deletes(rt: TestRuntime) -> anyhow::Result<()> {
    let t = UdfTest::default(rt).await?;
    for i in 0..10 {
        t.mutation("query:insert", assert_obj!("number" => i))
            .await?;
    }

    // Deletes 0 through 4, and returns the next which is 5.
    let res = t
        .mutation("query:firstAfterPendingDeletes", assert_obj!())
        .await?;
    must_let!(let ConvexValue::Int64(first) = res);
    assert_eq!(first, 5);
    Ok(())
}

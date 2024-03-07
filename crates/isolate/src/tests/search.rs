use std::{
    collections::BTreeSet,
    str::FromStr,
};

use common::{
    assert_obj,
    bootstrap_model::index::IndexMetadata,
    testing::TestPersistence,
    value::ConvexValue,
};
use maplit::btreeset;
use must_let::must_let;
use runtime::testing::TestRuntime;
use search::{
    MAX_CANDIDATE_REVISIONS,
    MAX_FILTER_CONDITIONS,
    MAX_QUERY_TERMS,
};
use value::TableName;

use super::assert_contains;
use crate::{
    test_helpers::UdfTest,
    tests::query::assert_paginated_query_journal_is_correct,
};

async fn add_search_index(t: &UdfTest<TestRuntime, TestPersistence>) -> anyhow::Result<()> {
    t.add_index(IndexMetadata::new_backfilling_search_index(
        "messages.by_body".parse()?,
        "body".parse()?,
        btreeset! { "filterField".parse()?},
    ))
    .await
}

async fn add_and_backfill_search_index(
    t: &UdfTest<TestRuntime, TestPersistence>,
) -> anyhow::Result<()> {
    add_search_index(t).await?;
    t.backfill_search_indexes().await?;

    Ok(())
}

#[convex_macro::test_runtime]
async fn test_search_disk_index_backfill_error(rt: TestRuntime) -> anyhow::Result<()> {
    common::testing::init_test_logging();

    let t = UdfTest::default(rt.clone()).await?;

    // To use the disk search index, first populate the data and then create
    // and backfill the index.
    t.mutation("search:populateSearch", assert_obj!()).await?;
    add_search_index(&t).await?;

    let error = t
        .query_js_error("search:querySearch", assert_obj!("query" => "a"))
        .await?;

    assert_contains(
        &error,
        "Index messages.by_body is currently backfilling and not available to query yet.",
    );
    Ok(())
}

#[convex_macro::test_runtime]
async fn test_search_disk_index(rt: TestRuntime) -> anyhow::Result<()> {
    common::testing::init_test_logging();

    let t = UdfTest::default(rt.clone()).await?;

    // To use the disk search index, first populate the data and then create
    // and backfill the index.
    t.mutation("search:populateSearch", assert_obj!()).await?;
    add_and_backfill_search_index(&t).await?;

    must_let!(let ConvexValue::Array(results) = t.query("search:querySearch", assert_obj!("query" => "nonexistent")  ).await?);
    assert_eq!(results.len(), 0);

    must_let!(let ConvexValue::Array(results) = t.query("search:querySearch",assert_obj!("query" => "a")  ).await?);
    assert_eq!(results.len(), 6);
    Ok(())
}

#[convex_macro::test_runtime]
async fn test_search_in_memory_index(rt: TestRuntime) -> anyhow::Result<()> {
    let t = UdfTest::default(rt.clone()).await?;
    // To use the in-memory search index, first create and backfill the search
    // index, and then add additional data that won't be included on disk.
    add_and_backfill_search_index(&t).await?;
    t.mutation("search:populateSearch", assert_obj!()).await?;

    must_let!(let ConvexValue::Array(results) = t.query("search:querySearch",assert_obj!("query" => "nonexistent")  ).await?);
    assert_eq!(results.len(), 0);

    must_let!(let ConvexValue::Array(results) = t.query("search:querySearch", assert_obj!("query" => "a")  ).await?);
    assert_eq!(results.len(), 6);
    Ok(())
}

#[convex_macro::test_runtime]
async fn test_paginated_search(rt: TestRuntime) -> anyhow::Result<()> {
    let t = UdfTest::default(rt.clone()).await?;
    t.mutation("search:populateSearch", assert_obj!()).await?;
    add_and_backfill_search_index(&t).await?;

    let get_query_page = async move |t: &UdfTest<TestRuntime, TestPersistence>,
                                     cursor: ConvexValue|
                -> anyhow::Result<(String, bool, ConvexValue)> {
        must_let!(let ConvexValue::Object(o) = t.query("search:paginatedSearch", assert_obj!("cursor" => cursor, "query" => "a")).await?);
        must_let!(let Some(ConvexValue::Array(page)) = o.get("page"));
        assert_eq!(page.len(), 1);
        must_let!(let ConvexValue::Object(row) = &page[0]);
        must_let!(let Some(ConvexValue::String(body)) = row.get("body"));
        must_let!(let Some(ConvexValue::Boolean(is_done)) = o.get("isDone"));
        must_let!(let Some(continue_cursor) = o.get("continueCursor"));
        Ok((body.to_string(), *is_done, continue_cursor.clone()))
    };

    let mut bodies = BTreeSet::new();

    let (body, is_done1, continue_cursor1) = get_query_page(&t, ConvexValue::Null).await?;
    bodies.insert(body);
    assert!(!is_done1);

    let (body, is_done2, continue_cursor2) = get_query_page(&t, continue_cursor1).await?;
    bodies.insert(body);
    assert!(!is_done2);

    let (body, is_done3, continue_cursor3) = get_query_page(&t, continue_cursor2).await?;
    bodies.insert(body);
    assert!(!is_done3);

    let (body, is_done4, continue_cursor4) = get_query_page(&t, continue_cursor3).await?;
    bodies.insert(body);
    assert!(!is_done4);

    // "a c" sorts before "a b" because they are equally relevant and then we
    // tie break on creation time (newest first).

    let (body, is_done5, continue_cursor5) = get_query_page(&t, continue_cursor4).await?;
    bodies.insert(body);
    assert!(!is_done5);

    let (body, is_done6, continue_cursor6) = get_query_page(&t, continue_cursor5).await?;
    bodies.insert(body);
    assert!(!is_done6);

    assert!(bodies.contains("a"));
    assert!(bodies.contains("a a"));
    assert!(bodies.contains("a a a"));
    assert!(bodies.contains("a a a a"));
    assert!(bodies.contains("a b"));
    assert!(bodies.contains("a c"));

    must_let!(let ConvexValue::Object(o) = t.query("search:paginatedSearch",  assert_obj!("cursor" => continue_cursor6, "query" => "a")).await?);

    must_let!(let Some(ConvexValue::Boolean(is_done7)) = o.get("isDone"));
    assert!(is_done7);

    Ok(())
}

#[convex_macro::test_runtime]
async fn test_query_journal_is_idempotent_search_query(rt: TestRuntime) -> anyhow::Result<()> {
    let t = UdfTest::default(rt).await?;
    t.mutation("search:populateSearch", assert_obj!()).await?;
    add_and_backfill_search_index(&t).await?;

    // Run a search query!
    let (results, is_done) = assert_paginated_query_journal_is_correct(
        &t,
        "search:paginatedSearch",
        assert_obj!("cursor" => ConvexValue::Null, "query" => "a"),
        vec![],
    )
    .await?;
    assert_eq!(results.len(), 1);
    assert!(!is_done);
    Ok(())
}

/// Test that mutations can search for documents that they create.
#[convex_macro::test_runtime]
async fn test_search_for_pending_document(rt: TestRuntime) -> anyhow::Result<()> {
    let t = UdfTest::default(rt.clone()).await?;
    add_and_backfill_search_index(&t).await?;
    must_let!(let ConvexValue::Array(results) = t.mutation("search:createDocumentAndSearchForIt", assert_obj!()).await?);
    assert_eq!(results.len(), 1);
    Ok(())
}

/// Tests for all of the search error cases.

#[convex_macro::test_runtime]
async fn test_incorrect_search_field(rt: TestRuntime) -> anyhow::Result<()> {
    let t = UdfTest::default(rt.clone()).await?;
    add_and_backfill_search_index(&t).await?;
    let e = t
        .query_js_error("search:incorrectSearchField", assert_obj!())
        .await?;
    assert_contains(
        &e,
        "Uncaught Error: Search query against messages.by_body contains a search filter against \
         \"nonexistentField\", which doesn't match the indexed `searchField` \"body\".",
    );
    Ok(())
}

#[convex_macro::test_runtime]
async fn test_duplicate_search_filters(rt: TestRuntime) -> anyhow::Result<()> {
    let t = UdfTest::default(rt.clone()).await?;
    add_and_backfill_search_index(&t).await?;
    let e = t
        .query_js_error("search:duplicateSearchFilters", assert_obj!())
        .await?;
    assert_contains(
        &e,
        "Uncaught Error: Search query against messages.by_body contains multiple search filters \
         against \"body\". Only one is allowed.",
    );
    Ok(())
}

#[convex_macro::test_runtime]
async fn test_incorrect_filter_field(rt: TestRuntime) -> anyhow::Result<()> {
    let t = UdfTest::default(rt.clone()).await?;
    add_and_backfill_search_index(&t).await?;
    let e = t
        .query_js_error("search:incorrectFilterField", assert_obj!())
        .await?;
    assert_contains(
        &e,
        "Uncaught Error: Search query against messages.by_body contains an equality filter on \
         \"nonexistentField\" but that field isn't indexed for filtering in `filterFields`.",
    );
    Ok(())
}

#[convex_macro::test_runtime]
async fn test_missing_search_filter(rt: TestRuntime) -> anyhow::Result<()> {
    let t = UdfTest::default(rt.clone()).await?;
    add_and_backfill_search_index(&t).await?;
    let e = t
        .query_js_error("search:missingSearchFilter", assert_obj!())
        .await?;
    assert_contains(
        &e,
        "Uncaught Error: Search query against messages.by_body does not contain any search \
         filters. You must include a search filter like `q.search(\"\"body\"\", searchText)`.",
    );
    Ok(())
}

#[convex_macro::test_runtime]
async fn test_too_many_terms_in_search_query(rt: TestRuntime) -> anyhow::Result<()> {
    let t = UdfTest::default(rt.clone()).await?;
    add_and_backfill_search_index(&t).await?;

    // Construct a query string with MAX_QUERY_TERMS terms, separated by spaces.
    let mut search_query = "".to_string();
    for i in 0..(MAX_QUERY_TERMS) {
        search_query = format!("{search_query} {i}")
    }

    // Querying with MAX_QUERY_TERMS works fine.
    t.query(
        "search:querySearch",
        assert_obj!("query" => search_query.clone()),
    )
    .await?;

    // Add one more term and it still works, just not including the last term.
    let mut tx = t.database.begin_system().await?;
    tx.insert_for_test(
        &TableName::from_str("messages")?,
        assert_obj!("body" => "oneMoreTerm"),
    )
    .await?;
    t.database.commit(tx).await?;

    search_query = format!("{search_query} oneMoreTerm");
    let result = t
        .query(
            "search:querySearch",
            assert_obj!("query" => search_query.clone()),
        )
        .await?;
    must_let!(let ConvexValue::Array(array) = result);
    assert!(array.is_empty());

    Ok(())
}

#[convex_macro::test_runtime]
async fn test_too_many_filter_conditions_in_search_query(rt: TestRuntime) -> anyhow::Result<()> {
    let t = UdfTest::default(rt.clone()).await?;
    add_and_backfill_search_index(&t).await?;

    // Querying with MAX_FILTER_CONDITIONS works fine.
    t.query(
        "search:tooManyFilterConditions",
        assert_obj!("numFilterConditions" => i64::try_from(MAX_FILTER_CONDITIONS)?),
    )
    .await?;

    // Querying with MAX_FILTER_CONDITIONS + 1 produces an error.
    let e = t
        .query_js_error(
            "search:tooManyFilterConditions",
            assert_obj!("numFilterConditions" =>i64::try_from(MAX_FILTER_CONDITIONS + 1)?),
        )
        .await?;
    assert_contains(
        &e,
        "Uncaught Error: Search query against messages.by_body has too many filter conditions. \
         Max: 8 Actual: 9",
    );
    Ok(())
}

#[convex_macro::test_runtime]
async fn test_search_query_scanned_too_many_documents(rt: TestRuntime) -> anyhow::Result<()> {
    let t = UdfTest::default(rt.clone()).await?;
    add_and_backfill_search_index(&t).await?;

    // Create MAX_CANDIDATE_REVISIONS-1 documents with "body" in their body field.
    t.mutation(
        "search:insertMany",
        assert_obj!(
            "body" => "body",
            "numDocumentsToCreate" => i64::try_from(MAX_CANDIDATE_REVISIONS - 1)?,
        ),
    )
    .await?;

    // We can query and get them all.
    must_let!(let ConvexValue::Array(results) = t.query("search:querySearch", assert_obj!("query" => "body")).await?);
    assert_eq!(results.len(), MAX_CANDIDATE_REVISIONS - 1);

    // Insert one over the limit and we error.
    t.mutation(
        "search:insertMany",
        assert_obj!(
            "body" => "body",
            "numDocumentsToCreate" => 1,
        ),
    )
    .await?;
    let e = t
        .query_js_error("search:querySearch", assert_obj!("query" => "body"))
        .await?;
    assert_contains(
        &e,
        "Uncaught Error: Search query scanned too many documents (fetched 1024). Consider using a \
         smaller limit, paginating the query, or using a filter field to limit the number of \
         documents pulled from the search index.",
    );
    Ok(())
}

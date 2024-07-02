use common::{
    assert_obj,
    query::{
        Order,
        Query,
    },
    value::ConvexValue,
};
use database::{
    ResolvedQuery,
    TableModel,
    TestFacingModel,
};
use keybroker::Identity;
use model::{
    backend_state::BACKEND_STATE_TABLE,
    file_storage::FILE_STORAGE_VIRTUAL_TABLE,
};
use must_let::must_let;
use runtime::testing::TestRuntime;
use value::{
    id_v6::DeveloperDocumentId,
    InternalId,
    TableName,
    TableNamespace,
};

use super::assert_contains;
use crate::test_helpers::{
    UdfTest,
    UdfTestType,
};

#[convex_macro::test_runtime]
async fn test_not_found(rt: TestRuntime) -> anyhow::Result<()> {
    UdfTest::run_test_with_isolate2(rt, async move |t: UdfTestType| {
        let err = t
            .query_js_error_no_validation("nonexistent", assert_obj!())
            .await?;

        // TODO: It'd be nice to be able to downcast from `anyhow` here, but we
        // intentionally stringify the error when stuffing it in the `UdfOutcome`
        // structure. This way we could provide additional context to the user on
        // error, especially in "development mode," without having to store it all
        // in the database.
        assert!(format!("{}", err).contains("Couldn't find JavaScript module"));

        let err = t
            .query_js_error_no_validation("userError:aPrivateFunction", assert_obj!())
            .await?;
        assert!(format!("{}", err).contains(r#"Couldn't find "aPrivateFunction" in module"#));

        let err = t
            .query_js_error_no_validation("userError:aNonexistentFunction", assert_obj!())
            .await?;
        assert!(format!("{}", err).contains(r#"Couldn't find "aNonexistentFunction" in module"#));
        Ok(())
    })
    .await
}

#[convex_macro::test_runtime]
async fn test_bad_arguments_error(rt: TestRuntime) -> anyhow::Result<()> {
    UdfTest::run_test_with_isolate2(rt, async move |t: UdfTestType| {
        must_let!(let Ok(ConvexValue::String(s)) = t.query("userError:badArgumentsError", assert_obj!()).await);
        assert!(s.contains("Invalid argument `id` for `db.get`"), "{s}");
        Ok(())
    }).await
}

#[convex_macro::test_runtime]
async fn test_bad_id_error(rt: TestRuntime) -> anyhow::Result<()> {
    UdfTest::run_test_with_isolate2(rt, async move |t: UdfTestType| {
        must_let!(let Ok(ConvexValue::String(s)) = t.query("userError:badIdError", assert_obj!()).await);
        // A system UDF (listById) relies on this error message being invariant.
        assert!(s.contains("Unable to decode ID"), "{s}");
        Ok(())
    }).await
}

#[convex_macro::test_runtime]
async fn test_insertion_error(rt: TestRuntime) -> anyhow::Result<()> {
    UdfTest::run_test_with_isolate2(rt, async move |t: UdfTestType| {
        must_let!(let Ok(ConvexValue::String(s)) = t.mutation("userError:insertError", assert_obj!()).await);
        assert!(
            s.contains("System tables (prefixed with `_`) are read-only."),
            "{s}"
        );
        Ok(())
    }).await
}

// BigInts cause JSON.stringify() to crash, so they're worth checking for
// specifically. Ensure that the error is catchable in JavaScript.
#[convex_macro::test_runtime]
async fn test_insert_error_with_bigint(rt: TestRuntime) -> anyhow::Result<()> {
    UdfTest::run_test_with_isolate2(rt, async move |t: UdfTestType| {
        must_let!(let Ok(ConvexValue::String(s)) = t.mutation("userError:insertErrorWithBigint", assert_obj!()).await);
        assert!(
            s.contains("undefined is not a valid Convex value (present at path .bad"),
            "{s}"
        );
        Ok(())
    }).await
}

#[convex_macro::test_runtime]
async fn test_patch_error(rt: TestRuntime) -> anyhow::Result<()> {
    UdfTest::run_test_with_isolate2(rt, async move |t: UdfTestType| {
        must_let!(let Ok(ConvexValue::String(s)) = t.mutation("userError:patchError", assert_obj!()).await);
        assert!(s.contains("Update on nonexistent document ID"), "{s}");
        Ok(())
    }).await
}

#[convex_macro::test_runtime]
async fn test_patch_value_not_an_object(rt: TestRuntime) -> anyhow::Result<()> {
    UdfTest::run_test_with_isolate2(rt, async move |t: UdfTestType| {
        must_let!(let Ok(ConvexValue::String(s)) = t.mutation("userError:patchValueNotAnObject", assert_obj!()).await);
        assert!(
            s.contains("Invalid argument `value` for `db.patch`: Value must be an Object"),
            "{s}"
        );
        Ok(())
    }).await
}

#[convex_macro::test_runtime]
async fn test_replace_error(rt: TestRuntime) -> anyhow::Result<()> {
    UdfTest::run_test_with_isolate2(rt, async move |t: UdfTestType| {
        must_let!(let Ok(ConvexValue::String(s)) = t.mutation("userError:replaceError", assert_obj!()).await);
        assert!(s.contains("Replace on nonexistent document ID"), "{s}");
        Ok(())
    }).await
}

#[convex_macro::test_runtime]
async fn test_delete_error(rt: TestRuntime) -> anyhow::Result<()> {
    UdfTest::run_test_with_isolate2(rt, async move |t: UdfTestType| {
        must_let!(let Ok(ConvexValue::String(s)) = t.mutation("userError:deleteError", assert_obj!()).await);
        assert!(s.contains("Delete on nonexistent document ID"), "{s}");
        Ok(())
    }).await
}

#[convex_macro::test_runtime]
async fn test_nonexistent_table(rt: TestRuntime) -> anyhow::Result<()> {
    UdfTest::run_test_with_isolate2(rt, async move |t: UdfTestType| {
        t.create_index("boatVotes.by_boat", "boat").await?;
        t.backfill_indexes().await?;
        let mut tx = t.database.begin(Identity::system()).await?;
        let table_number = TableModel::new(&mut tx).next_user_table_number().await?;
        let nonexistent_id = DeveloperDocumentId::new(table_number, InternalId::MIN);
        t.mutation(
            "userError:nonexistentTable",
            assert_obj!("nonexistentId" => nonexistent_id),
        )
        .await?;
        Ok(())
    })
    .await
}

#[convex_macro::test_runtime]
async fn test_index_on_nonexistent_table(rt: TestRuntime) -> anyhow::Result<()> {
    UdfTest::run_test_with_isolate2(rt, async move |t: UdfTestType| {
        t.mutation("userError:indexOnNonexistentTable", assert_obj!())
            .await?;
        Ok(())
    })
    .await
}

#[convex_macro::test_runtime]
async fn test_nonexistent_id(rt: TestRuntime) -> anyhow::Result<()> {
    UdfTest::run_test_with_isolate2(rt, async move |t: UdfTestType| {
        let mut tx = t.database.begin(Identity::system()).await?;
        let table_number = 8000.try_into()?;
        let table_name: TableName = "_my_system_table".parse()?;

        assert!(
            tx.create_system_table_testing(
                TableNamespace::test_user(),
                &table_name,
                Some(table_number),
            ).await?
        );
        let nonexistent_system_table_id = DeveloperDocumentId::new(table_number, InternalId::MIN);

        let virtual_table_number = tx
            .virtual_table_mapping()
            .namespace(TableNamespace::test_user())
            .number(&FILE_STORAGE_VIRTUAL_TABLE)?;
        let nonexistent_virtual_table_id = DeveloperDocumentId::new(
            virtual_table_number, InternalId::MIN);
        let user_document = TestFacingModel::new(&mut tx)
            .insert_and_get("table".parse()?, assert_obj!())
            .await?;
        let user_table_number = user_document.id().developer_id.table();
        let nonexistent_user_table_id = DeveloperDocumentId::new(
            user_table_number, InternalId::MIN);
        t.database.commit(tx).await?;
        t.mutation(
            "userError:nonexistentId",
            assert_obj!("nonexistentSystemId" => nonexistent_system_table_id, "nonexistentUserId" => nonexistent_user_table_id),
        )
        .await?;
        // Using db.get with an ID on a private system table is like the table doesn't
        // exist => returns null.
        t.mutation(
            "userError:nonexistentSystemIdFails",
            assert_obj!("nonexistentSystemId" => nonexistent_system_table_id),
        )
        .await?;
        // Using db.get with an ID on a virtual table, even if the ID doesn't exist,
        // throws error.
        let err = t
            .mutation_js_error(
                "userError:nonexistentSystemIdFails",
                assert_obj!("nonexistentSystemId" => nonexistent_virtual_table_id),
            )
            .await?;
        assert!(err
            .message
            .contains("System tables can only be accessed with db.system."));
        let err = t
            .mutation_js_error(
                "userError:nonexistentUserIdFails",
                assert_obj!("nonexistentUserId" => nonexistent_user_table_id),
            )
            .await?;
        assert!(err
            .message
            .contains("User tables cannot be accessed with db.system."));
        Ok(())
    }).await
}

#[convex_macro::test_runtime]
async fn test_private_system_table(rt: TestRuntime) -> anyhow::Result<()> {
    UdfTest::run_test_with_isolate2(rt, async move |t: UdfTestType| {
        let mut tx = t.database.begin(Identity::system()).await?;

        // backend state automatically created by with_model().
        let backend_state = ResolvedQuery::new(
            &mut tx,
            TableNamespace::Global,
            Query::full_table_scan(BACKEND_STATE_TABLE.clone(), Order::Asc),
        )?
        .expect_at_most_one(&mut tx)
        .await?
        .expect("backend state should exist");

        // But developer UDFs can't query it because it's a private system table.
        must_let!(let ConvexValue::Array(results) = t.query(
            "userError:privateSystemQuery",
            assert_obj!("tableName" => BACKEND_STATE_TABLE.to_string()),
        )
        .await?);
        assert!(results.is_empty());
        must_let!(let ConvexValue::Null = t.query(
            "userError:privateSystemGet",
            assert_obj!("id" => backend_state.id().to_string()),
        )
        .await?);
        Ok(())
    })
    .await
}

#[convex_macro::test_runtime]
async fn test_unhandled_promise_rejection(rt: TestRuntime) -> anyhow::Result<()> {
    UdfTest::run_test_with_isolate2(rt, async move |t: UdfTestType| {
        // Check that an unhandled promise rejection fails the UDF.
        let e = t
            .mutation_js_error("userError:unhandledRejection", assert_obj!())
            .await?;
        assert!(format!("{e}").contains("Unable to decode ID"));
        Ok(())
    })
    .await
}

#[convex_macro::test_runtime]
async fn test_catching_async_exception_thrown_before_await(rt: TestRuntime) -> anyhow::Result<()> {
    UdfTest::run_test_with_isolate2(rt, async move |t: UdfTestType| {
        must_let!(let Ok(ConvexValue::String(s)) = t.mutation("userError:asyncExceptionBeforeAwait", assert_obj!()).await);
        assert!(s.contains("This is a custom exception"), "{s}");
        Ok(())
    }).await
}

#[convex_macro::test_runtime]
async fn test_catching_async_exception_thrown_after_await(rt: TestRuntime) -> anyhow::Result<()> {
    UdfTest::run_test_with_isolate2(rt, async move |t: UdfTestType| {
        must_let!(let Ok(ConvexValue::String(s)) = t.mutation("userError:asyncExceptionAfterAwait", assert_obj!()).await);
        assert!(s.contains("This is a custom exception"), "{s}");
        Ok(())
    }).await
}

#[convex_macro::test_runtime]
async fn test_throw_string(rt: TestRuntime) -> anyhow::Result<()> {
    UdfTest::run_test_with_isolate2(rt, async move |t: UdfTestType| {
        must_let!(let Ok(ConvexValue::String(s)) = t.mutation("userError:throwString", assert_obj!()).await);
        assert!(s.contains("string - a string"), "{s}");
        Ok(())
    }).await
}

#[convex_macro::test_runtime]
async fn test_async_syscall_error(rt: TestRuntime) -> anyhow::Result<()> {
    UdfTest::run_test_with_isolate2(rt, async move |t: UdfTestType| {
        let e = t
            .mutation_js_error("userError:syscallError", assert_obj!())
            .await?;
        assert!(
            !e.frames.as_ref().unwrap().0.is_empty(),
            "message: {}, frames: {:?}",
            e.message,
            e.frames
        );
        Ok(())
    })
    .await
}

#[convex_macro::test_runtime]
async fn test_insert_with_creation_time(rt: TestRuntime) -> anyhow::Result<()> {
    UdfTest::run_test_with_isolate2(rt, async move |t: UdfTestType| {
        let e = t
            .mutation_js_error("adversarial:insertWithCreationTime", assert_obj!())
            .await?;
        assert_contains(&e, "Provided creation time");
        Ok(())
    })
    .await
}

#[convex_macro::test_runtime]
async fn test_insert_with_id(rt: TestRuntime) -> anyhow::Result<()> {
    UdfTest::run_test_with_isolate2(rt, async move |t: UdfTestType| {
        let e = t
            .mutation_js_error("adversarial:insertWithId", assert_obj!())
            .await?;

        assert_contains(&e, "Provided document ID");
        Ok(())
    })
    .await
}

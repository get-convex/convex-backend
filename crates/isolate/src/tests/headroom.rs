#![allow(clippy::float_cmp)]

use common::{
    assert_obj,
    knobs::{
        TRANSACTION_MAX_NUM_SCHEDULED,
        TRANSACTION_MAX_NUM_USER_WRITES,
        TRANSACTION_MAX_READ_SIZE_BYTES,
        TRANSACTION_MAX_READ_SIZE_ROWS,
        TRANSACTION_MAX_SCHEDULED_TOTAL_ARGUMENT_SIZE_BYTES,
        TRANSACTION_MAX_USER_WRITE_SIZE_BYTES,
    },
    testing::assert_contains,
    value::{
        ConvexObject,
        ConvexValue,
    },
};
use must_let::must_let;
use runtime::testing::TestRuntime;

use crate::test_helpers::{
    UdfTest,
    UdfTestType,
};

fn get_f64(obj: &ConvexObject, key: &str) -> f64 {
    must_let!(let Some(ConvexValue::Float64(v)) = obj.get(key));
    *v
}

fn get_obj<'a>(o: &'a ConvexObject, key: &str) -> &'a ConvexObject {
    must_let!(let Some(ConvexValue::Object(v)) = o.get(key));
    v
}

#[derive(Clone, Copy)]
struct ExpectedMetric {
    used: f64,
    remaining: f64,
}

impl ExpectedMetric {
    fn new(used: f64, remaining: f64) -> Self {
        Self { used, remaining }
    }

    fn assert_eq(&self, obj: &ConvexObject, field: &str) {
        let v = get_obj(obj, field);
        assert_eq!(get_f64(v, "used"), self.used, "{field}.used");
        assert_eq!(get_f64(v, "remaining"), self.remaining, "{field}.remaining");
    }
}

#[derive(Clone, Copy)]
struct ExpectedHeadroom {
    bytes_read: ExpectedMetric,
    bytes_written: ExpectedMetric,
    documents_read: ExpectedMetric,
    documents_written: ExpectedMetric,
    functions_scheduled: ExpectedMetric,
    scheduled_function_args_bytes: ExpectedMetric,
}

impl ExpectedHeadroom {
    fn max() -> Self {
        Self {
            bytes_read: ExpectedMetric::new(0.0, *TRANSACTION_MAX_READ_SIZE_BYTES as f64),
            bytes_written: ExpectedMetric::new(0.0, *TRANSACTION_MAX_USER_WRITE_SIZE_BYTES as f64),
            documents_read: ExpectedMetric::new(0.0, *TRANSACTION_MAX_READ_SIZE_ROWS as f64),
            documents_written: ExpectedMetric::new(0.0, *TRANSACTION_MAX_NUM_USER_WRITES as f64),
            functions_scheduled: ExpectedMetric::new(0.0, *TRANSACTION_MAX_NUM_SCHEDULED as f64),
            scheduled_function_args_bytes: ExpectedMetric::new(
                0.0,
                *TRANSACTION_MAX_SCHEDULED_TOTAL_ARGUMENT_SIZE_BYTES as f64,
            ),
        }
    }

    fn after_writes(&self, doc_size: f64, doc_count: f64) -> Self {
        Self {
            bytes_read: ExpectedMetric::new(
                self.bytes_read.used + doc_size,
                self.bytes_read.remaining - doc_size,
            ),
            documents_read: ExpectedMetric::new(
                self.documents_read.used + doc_count,
                self.documents_read.remaining - doc_count,
            ),
            bytes_written: ExpectedMetric::new(
                self.bytes_written.used + doc_size,
                self.bytes_written.remaining - doc_size,
            ),
            documents_written: ExpectedMetric::new(
                self.documents_written.used + doc_count,
                self.documents_written.remaining - doc_count,
            ),
            ..*self
        }
    }

    fn after_reads(&self, total_bytes: f64, doc_count: f64) -> Self {
        Self {
            bytes_read: ExpectedMetric::new(
                self.bytes_read.used + total_bytes,
                self.bytes_read.remaining - total_bytes,
            ),
            documents_read: ExpectedMetric::new(
                self.documents_read.used + doc_count,
                self.documents_read.remaining - doc_count,
            ),
            ..*self
        }
    }
}

fn assert_headroom(h: &ConvexObject, expected: &ExpectedHeadroom) {
    expected.bytes_read.assert_eq(h, "bytesRead");
    expected.bytes_written.assert_eq(h, "bytesWritten");
    expected.documents_read.assert_eq(h, "documentsRead");
    expected.documents_written.assert_eq(h, "documentsWritten");
    expected
        .functions_scheduled
        .assert_eq(h, "functionsScheduled");
    expected
        .scheduled_function_args_bytes
        .assert_eq(h, "scheduledFunctionArgsBytes");
}

#[convex_macro::test_runtime]
async fn test_headroom_empty(rt: TestRuntime) -> anyhow::Result<()> {
    UdfTest::run_test_with_isolate2(rt, async move |t: UdfTestType| {
        must_let!(let ConvexValue::Object(h) = t.query("headroom:headroomEmpty", assert_obj!()).await?);
        assert_headroom(&h, &ExpectedHeadroom::max());
        Ok(())
    })
    .await
}

#[convex_macro::test_runtime]
async fn test_headroom_after_insert(rt: TestRuntime) -> anyhow::Result<()> {
    UdfTest::run_test_with_isolate2(rt, async move |t: UdfTestType| {
        must_let!(let ConvexValue::Object(r) = t.mutation("headroom:headroomAfterInsert", assert_obj!()).await?);
        let doc_size = get_f64(&r, "docSize");
        assert_headroom(
            get_obj(&r, "headroom"),
            &ExpectedHeadroom::max().after_writes(doc_size, 1.0),
        );
        Ok(())
    })
    .await
}

#[convex_macro::test_runtime]
async fn test_headroom_after_query(rt: TestRuntime) -> anyhow::Result<()> {
    UdfTest::run_test_with_isolate2(rt, async move |t: UdfTestType| {
        t.mutation("headroom:headroomAfterInsert", assert_obj!())
            .await?;
        must_let!(let ConvexValue::Object(r) = t.query("headroom:headroomAfterQuery", assert_obj!()).await?);
        let total_bytes = get_f64(&r, "totalBytes");
        let doc_count = get_f64(&r, "docCount");
        assert_headroom(
            get_obj(&r, "headroom"),
            &ExpectedHeadroom::max().after_reads(total_bytes, doc_count),
        );
        Ok(())
    })
    .await
}

// Subtransactions not yet supported in isolate2.
#[convex_macro::test_runtime]
async fn test_headroom_with_subtransactions(rt: TestRuntime) -> anyhow::Result<()> {
    UdfTest::run_test_with_isolate(rt, async move |t: UdfTestType| {
        must_let!(let ConvexValue::Object(r) = t.mutation(
            "headroom:headroomWithSubTransactions", assert_obj!()
        ).await?);
        let doc_size = get_f64(get_obj(&r, "insertResult"), "docSize");

        // Initial: nothing consumed
        assert_headroom(get_obj(&r, "initial"), &ExpectedHeadroom::max());

        // After insert sub: 1 read (db.get) + 1 write
        let after_insert = ExpectedHeadroom::max().after_writes(doc_size, 1.0);
        assert_headroom(
            get_obj(get_obj(&r, "insertResult"), "headroom"),
            &after_insert,
        );
        assert_headroom(get_obj(&r, "afterInsert"), &after_insert);

        // Empty query sub + parent after: unchanged
        assert_headroom(get_obj(&r, "emptyQuery"), &after_insert);
        assert_headroom(get_obj(&r, "afterEmptyQuery"), &after_insert);

        // Query sub reads 1 more doc; final parent matches
        let after_query = after_insert.after_reads(doc_size, 1.0);
        assert_headroom(
            get_obj(get_obj(&r, "queryResult"), "headroom"),
            &after_query,
        );
        assert_headroom(get_obj(&r, "final_"), &after_query);

        Ok(())
    })
    .await
}

#[convex_macro::test_runtime]
async fn test_headroom_from_action(rt: TestRuntime) -> anyhow::Result<()> {
    UdfTest::run_test_with_isolate(rt, async move |t: UdfTestType| {
        let e = t
            .action_js_error("headroom:headroomFromAction", assert_obj!())
            .await?;
        assert_contains(
            &e,
            "getTransactionHeadroom() can only be called from a query or mutation",
        );
        Ok(())
    })
    .await
}

#[convex_macro::test_runtime]
async fn test_headroom_system_reads(rt: TestRuntime) -> anyhow::Result<()> {
    UdfTest::run_test_with_isolate2(rt, async move |t: UdfTestType| {
        must_let!(let ConvexValue::Object(r) = t.mutation(
            "headroom:headroomAfterSystemRead", assert_obj!()
        ).await?);
        // System reads don't count against user headroom
        assert_headroom(get_obj(&r, "before"), &ExpectedHeadroom::max());
        assert_headroom(get_obj(&r, "after"), &ExpectedHeadroom::max());
        Ok(())
    })
    .await
}

#[convex_macro::test_runtime]
async fn test_headroom_after_schedule(rt: TestRuntime) -> anyhow::Result<()> {
    UdfTest::run_test_with_isolate2(rt, async move |t: UdfTestType| {
        must_let!(let ConvexValue::Object(r) = t.mutation(
            "headroom:headroomAfterSchedule", assert_obj!()
        ).await?);
        let arg_size = get_f64(&r, "expectedArgSize");
        let max = ExpectedHeadroom::max();

        // Scheduling doesn't affect read/write headroom, only schedule fields
        assert_headroom(get_obj(&r, "before"), &max);

        let max_sf = *TRANSACTION_MAX_NUM_SCHEDULED as f64;
        let max_sb = *TRANSACTION_MAX_SCHEDULED_TOTAL_ARGUMENT_SIZE_BYTES as f64;
        let after_one = ExpectedHeadroom {
            functions_scheduled: ExpectedMetric::new(1.0, max_sf - 1.0),
            scheduled_function_args_bytes: ExpectedMetric::new(arg_size, max_sb - arg_size),
            ..max
        };
        assert_headroom(get_obj(&r, "afterOne"), &after_one);

        let after_two = ExpectedHeadroom {
            functions_scheduled: ExpectedMetric::new(2.0, max_sf - 2.0),
            scheduled_function_args_bytes: ExpectedMetric::new(
                2.0 * arg_size,
                max_sb - 2.0 * arg_size,
            ),
            ..max
        };
        assert_headroom(get_obj(&r, "afterTwo"), &after_two);

        Ok(())
    })
    .await
}

#![allow(clippy::float_cmp)]

use common::{
    assert_obj,
    document::CreationTime,
    testing::assert_contains,
    types::FieldName,
    value::{
        ConvexObject,
        ConvexValue,
    },
};
use keybroker::Identity;
use must_let::must_let;
use runtime::testing::TestRuntime;
use value::assert_val;

use crate::test_helpers::{
    UdfTest,
    UdfTestType,
};

#[convex_macro::test_runtime]
async fn test_basic(rt: TestRuntime) -> anyhow::Result<()> {
    UdfTest::run_test_with_isolate2(rt, async move |t: UdfTestType| {
        must_let!(let ConvexValue::Float64(r) = t.query("directory/udfs:f", assert_obj!("a" => 10., "b" => 3.)).await?);
        assert_eq!(r, 57.);

        must_let!(let ConvexValue::Null = t.query("directory/udfs:returnsUndefined", assert_obj!()).await?);

        must_let!(let ConvexValue::Float64(r) = t.query("directory/defaultTest", assert_obj!("a" => 10., "b" => 3.)).await?);
        assert_eq!(r, 110.);

        must_let!(let ConvexValue::Float64(_) = t.query("directory/udfs:pseudoRandom", assert_obj!()).await?);

        t.query("directory/udfs:usesDate", assert_obj!()).await?;

        Ok(())
    }).await
}

#[convex_macro::test_runtime]
async fn test_int64(rt: TestRuntime) -> anyhow::Result<()> {
    UdfTest::run_test_with_isolate2(rt, async move |t: UdfTestType| {
        let v = t.query("basic:addOneInt", assert_obj!("x" => 1)).await?;
        assert_eq!(v, ConvexValue::Int64(2));
        Ok(())
    })
    .await
}

#[convex_macro::test_runtime]
async fn test_javascript(rt: TestRuntime) -> anyhow::Result<()> {
    UdfTest::run_test_with_isolate2(rt, async move |t: UdfTestType| {
        let v = t.query("js:addOneInt", assert_obj!("x" => 1)).await?;
        assert_eq!(v, ConvexValue::Int64(2));
        Ok(())
    })
    .await
}

#[convex_macro::test_runtime]
async fn test_insert_object(rt: TestRuntime) -> anyhow::Result<()> {
    UdfTest::run_test_with_isolate2(rt, async move |t: UdfTestType| {
        let values = [
            assert_val!(10),
            assert_val!(-0.),
            assert_val!(2.71),
            assert_val!(true),
            assert_val!("hi there"),
            assert_val!([0, 1, 2, 3]),
        ];

        for value in values {
            must_let!(let ConvexValue::Object(obj) = t.mutation(
                "basic:insertObject",
                assert_obj!("field" => value.clone()),
            ).await?);
            must_let!(let Some(ConvexValue::String(..)) = obj.get("_id"));
            must_let!(let Some(field) = obj.get("field"));
            assert_eq!(field, &value);
        }
        Ok(())
    })
    .await
}

#[convex_macro::test_runtime]
async fn test_references(rt: TestRuntime) -> anyhow::Result<()> {
    UdfTest::run_test_with_isolate2(rt, async move |t: UdfTestType| {
        let field_name: FieldName = "field".parse()?;
        must_let!(let ConvexValue::Object(obj) = t.mutation(
                "basic:insertObject",
                ConvexObject::for_value(field_name, ConvexValue::Null)?,
            ).await?);
        must_let!(let Some(ConvexValue::String(..)) = obj.get("_id"));
        Ok(())
    })
    .await
}

#[convex_macro::test_runtime]
async fn test_observed_time(rt: TestRuntime) -> anyhow::Result<()> {
    UdfTest::run_test_with_isolate2(rt, async move |t: UdfTestType| {
        let (v, e) = t
            .query_outcome("basic:addOneInt", assert_obj!("x" => 1), Identity::system())
            .await?;
        assert_eq!(v, assert_val!(2));
        assert!(!e.observed_time);

        let (_, e) = t
            .query_outcome("basic:readTime", assert_obj!(), Identity::system())
            .await?;
        assert!(e.observed_time);
        Ok(())
    })
    .await
}

#[convex_macro::test_runtime]
async fn test_names(rt: TestRuntime) -> anyhow::Result<()> {
    UdfTest::run_test_with_isolate2(rt, async move |t: UdfTestType| {
        // No function name specified -> default
        assert_eq!(t.query("name", assert_obj!()).await?, assert_val!(1.));
        // Can also specify "default" explicitly
        assert_eq!(
            t.query("name:default", assert_obj!()).await?,
            assert_val!(1.)
        );

        assert_eq!(t.query("name:g", assert_obj!()).await?, assert_val!(2.));
        assert_eq!(t.query("name:h", assert_obj!()).await?, assert_val!(3.));

        // `export default function $name` doesn't export `$name` in addition to
        // `default`.
        let err = t
            .query_js_error_no_validation("name:f", assert_obj!())
            .await?;
        assert_contains(&err, r#"Couldn't find "f" in module "name.js""#);

        // i is exported but is not a query or mutation
        let err = t
            .query_js_error_no_validation("name:i", assert_obj!())
            .await?;
        assert_contains(&err, "is neither a query or mutation");

        // Module doesn't exist
        let err = t
            .query_js_error_no_validation("notARealModule", assert_obj!())
            .await?;
        assert_contains(
            &err,
            r#"Couldn't find JavaScript module 'notARealModule.js'"#,
        );

        // Module exists but the function doesn't
        let err = t
            .query_js_error_no_validation("name:notARealFunction", assert_obj!())
            .await?;
        assert_contains(
            &err,
            r#"Couldn't find "notARealFunction" in module "name.js""#,
        );
        Ok(())
    })
    .await
}

#[convex_macro::test_runtime]
async fn test_insert_and_get(rt: TestRuntime) -> anyhow::Result<()> {
    UdfTest::run_test_with_isolate2(rt, async move |t: UdfTestType| {
        let value = assert_val!("I am here to stay");
        must_let!(let ConvexValue::Object(obj) = t.mutation(
            "basic:insertObject",
            assert_obj!("field" => value.clone()),
        ).await?);
        must_let!(let Some(id) = obj.get("_id"));

        // Get the object and compare the id and value.
        must_let!(let ConvexValue::Object(obj) = t.query("basic:getObject", assert_obj!("id" => id.clone())).await?);
        must_let!(let Some(id2) = obj.get("_id"));
        assert_eq!(id2, id);
        must_let!(let Some(field) = obj.get("field"));
        assert_eq!(field, &value);
        Ok(())
    }).await
}

#[convex_macro::test_runtime]
async fn test_insert_increase_and_delete(rt: TestRuntime) -> anyhow::Result<()> {
    UdfTest::run_test_with_isolate2(rt, async move |t: UdfTestType| {
        t.mutation("basic:insertModifyDeleteObject", assert_obj!())
            .await?;
        Ok(())
    })
    .await
}

#[convex_macro::test_runtime]
async fn test_insert_and_delete(rt: TestRuntime) -> anyhow::Result<()> {
    UdfTest::run_test_with_isolate2(rt, async move |t: UdfTestType| {
        let value = assert_val!("I am a phantom");
        must_let!(let ConvexValue::Object(obj) = t.mutation(
                "basic:insertAndDeleteObject",
                assert_obj!("field" => value),
        ).await?);
        must_let!(let Some(id) = obj.get("_id"));

        // The object should not exist.
        must_let!(let ConvexValue::Null = t.query("basic:getObject", assert_obj!( "id" => id.clone())).await?);

        // It shouldn't be in the index either.
        must_let!(let ConvexValue::Array(values) = t.query("basic:listAllObjects", assert_obj!()).await?);
        assert!(values.is_empty());
        Ok(())
    }).await
}

#[convex_macro::test_runtime]
async fn test_count(rt: TestRuntime) -> anyhow::Result<()> {
    UdfTest::run_test_with_isolate2(rt, async move |t: UdfTestType| {
        // Counting an empty table.
        must_let!(let ConvexValue::Float64(count) = t.query(
            "basic:count",
            assert_obj!(),
        ).await?);
        assert_eq!(count as usize, 0);

        // Insert one object
        must_let!(let ConvexValue::Object(_obj) = t.mutation(
            "basic:insertObject",
            assert_obj!("a" => "You can count on me!"),
        ).await?);

        // Count should return 1
        must_let!(let ConvexValue::Float64(count) = t.query(
            "basic:count",
            assert_obj!(),
        ).await?);
        assert_eq!(count as usize, 1);

        // Add another object.
        must_let!(let ConvexValue::Object(obj) = t.mutation(
            "basic:insertObject",
            assert_obj!("a" => "Count me too please!"),
        ).await?);
        must_let!(let Some(id) = obj.get("_id"));

        must_let!(let ConvexValue::Float64(count) = t.query(
            "basic:count",
            assert_obj!(),
        ).await?);
        assert_eq!(count as usize, 2);

        // Make sure we count object inserted within the transaction.
        must_let!(let ConvexValue::Float64(count) = t.mutation(
            "basic:insertAndCount",
            assert_obj!("a" => "Need to count pending inserts!"),
        ).await?);
        assert_eq!(count as usize, 3);

        // Make sure we count deletes within the transaction.
        must_let!(let ConvexValue::Float64(count) = t.mutation(
            "basic:deleteAndCount",
            assert_obj!("id" => id.clone()),
        ).await?);
        assert_eq!(count as usize, 2);
        Ok(())
    })
    .await
}

#[convex_macro::test_runtime]
async fn test_patch(rt: TestRuntime) -> anyhow::Result<()> {
    UdfTest::run_test_with_isolate2(rt, async move |t: UdfTestType| {
        // Insert an object.
        must_let!(let ConvexValue::Object(obj) = t.mutation(
            "basic:insertObject",
            assert_obj!(
                "field1" => "value1",
                "field2" => "value2",
                "field4" => {"a" => true},
            ),
        ).await?);
        must_let!(let Some(id) = obj.get("_id"));
        must_let!(let Some(ConvexValue::Float64(creation_time)) = obj.get("_creationTime"));

        // Patch it.
        must_let!(let ConvexValue::Object(obj) = t.mutation(
            "basic:patchObject",
            assert_obj!("id" => id.clone(), "obj" => {
                "field1" => "value3",
                "field3" => "value4",
                "field4" => {"b" => true},
            }),
        ).await?);

        // The update should add and overwrite overlapping fields but
        // non-overlapping ones intact.
        // Note that field4 gets overwritten, not merged.
        let expected = assert_obj!(
            "_id" => id.clone(),
            "_creationTime" => *creation_time,
            "field1" => "value3",
            "field2" => "value2",
            "field3" => "value4",
            "field4" => {"b" => true},
        );
        assert_eq!(obj, expected);

        // Try overwriting the creation time with patch.
        let e = t
            .mutation_js_error(
                "basic:patchObject",
                assert_obj!("id" => id.clone(), "obj" => {
                    "_creationTime" => 1017.,
                }),
            )
            .await?;
        assert_contains(&e, "doesn't match '_creationTime' field");

        // Delete field3.
        must_let!(let ConvexValue::Object(obj) = t.mutation(
            "basic:deleteObjectField",
            assert_obj!(
                "id" => id.clone(),
                "fieldName" => "field3",
            ),
        ).await?);
        let expected = assert_obj!(
            "_id" => id.clone(),
            "_creationTime" => *creation_time,
            "field1" => "value3",
            "field2" => "value2",
            "field4" => {"b" => true},
        );
        assert_eq!(obj, expected);
        Ok(())
    })
    .await
}

#[convex_macro::test_runtime]
async fn test_replace(rt: TestRuntime) -> anyhow::Result<()> {
    UdfTest::run_test_with_isolate2(rt, async move |t: UdfTestType| {
        // Insert an object.
        must_let!(let ConvexValue::Object(obj) = t.mutation(
            "basic:insertObject",
            assert_obj!(
                "field1" => "value1",
                "field2" => "value2",
            ),
        ).await?);
        must_let!(let Some(id) = obj.get("_id"));
        must_let!(let Some(ConvexValue::Float64(creation_time)) = obj.get("_creationTime"));

        // Check that the creation time is valid.
        CreationTime::try_from(*creation_time)?;

        // Replace it. Both the "_id" and "_creationTime" fields should propagate.
        let obj2 = assert_obj!(
            "field1" => "value3",
            "field3" => "value4",
        );
        must_let!(let ConvexValue::Object(obj3) = t.mutation(
            "basic:replaceObject",
            assert_obj!("id" => id.clone(), "obj" => obj2.clone()),
        ).await?);

        // The replace should completely override the object.
        let expected = assert_obj!(
            "_id" => id.clone(),
            "_creationTime" => *creation_time,
            "field1" => "value3",
            "field3" => "value4",
        );
        assert_eq!(obj3, expected);
        Ok(())
    })
    .await
}

#[convex_macro::test_runtime]
async fn test_query_missing_table(rt: TestRuntime) -> anyhow::Result<()> {
    UdfTest::run_test_with_isolate2(rt, async move |t: UdfTestType| {
        // Tables are implicitly created when we insert the first record.
        // This means that query before that is querying a missing table.
        // A user will expect no results instead of an error here.
        must_let!(let ConvexValue::Array(values) = t.query("basic:listAllObjects", assert_obj!()).await?);
        assert!(values.is_empty());
        Ok(())
    }).await
}

#[convex_macro::test_runtime]
async fn test_time_constructor_args(rt: TestRuntime) -> anyhow::Result<()> {
    UdfTest::run_test_with_isolate2(rt, async move |t: UdfTestType| {
        let ms_in: f64 = 1234567890123.0;
        must_let!(let ConvexValue::Float64(ms_out) = t.query("basic:createTimeMs",  assert_obj!("args" => [ms_in] )).await?);
        assert_eq!(ms_in, ms_out);

        // multiple numbers are allowed
        must_let!(let ConvexValue::Float64(_) = t.query("basic:createTimeMs", assert_obj!("args" => [1.0, 2.0])).await?);
        must_let!(let ConvexValue::Float64(_) = t.query("basic:createTimeMs", assert_obj!("args" => [1.0, 2.0, 3.0, 4.0, 4.0, 6.0, 7.0])).await?);

        // Assert that we are parsing in UTC
        must_let!(let ConvexValue::Float64(ms_out) = t.query("basic:createTimeMs", assert_obj!("args" => [1970.0, 0.0, 1.0])).await?);
        assert_eq!(ms_out, 0.);

        must_let!(let ConvexValue::Float64(ms_out) = t.query("basic:createTimeMs", assert_obj!("args" => ["1970-01-01"])).await?);
        assert_eq!(ms_out, 0.);

        must_let!(let ConvexValue::Float64(ms_out) = t.query("basic:createTimeMs", assert_obj!("args" => ["1970-01-01T12:00"])).await?);
        assert_eq!(ms_out, 12. * 60. * 60. * 1000.);

        Ok(())
    }).await
}

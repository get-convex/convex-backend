use core::f64;

use cmd_util::env::env_config;
use common::{
    assert_obj,
    value::ConvexValue,
};
use must_let::must_let;
use proptest::prelude::*;
use runtime::testing::{
    TestDriver,
    TestRuntime,
};
use value::{
    assert_val,
    proptest::{
        RestrictNaNs,
        ValueBranching,
    },
    FieldType,
    Size,
};

use crate::test_helpers::{
    UdfTest,
    UdfTestType,
};

#[convex_macro::test_runtime]
async fn test_bigint(rt: TestRuntime) -> anyhow::Result<()> {
    UdfTest::run_test_with_isolate2(rt, async move |t: UdfTestType| {
        let value = t.query("values:intQuery", assert_obj!()).await?;
        must_let!(let ConvexValue::Int64(v) = value);
        assert_eq!(v, 1);
        Ok(())
    })
    .await
}

#[convex_macro::test_runtime]
async fn test_empty_key(rt: TestRuntime) -> anyhow::Result<()> {
    UdfTest::run_test_with_isolate2(rt, async move |t: UdfTestType| {
        // Check that an object with an empty key round trips through mutation and
        // query.
        let id = t
            .mutation("values:insertObject", assert_obj!("obj" => {"" => "hi"}))
            .await?;
        let value = t.query("values:getObject", assert_obj!("id" => id)).await?;
        must_let!(let ConvexValue::Object(o) = value);
        assert_eq!(o.len(), 3);
        assert_eq!(o.get("").unwrap().clone(), assert_val!("hi"));
        Ok(())
    })
    .await
}

async fn test_compare(rt: TestRuntime, values: Vec<ConvexValue>) -> anyhow::Result<()> {
    let udf = UdfTest::default(rt).await?;
    let values = values.clone();
    let mut sorted_values = values.clone();
    sorted_values.sort();
    let value = udf
        .query("values:compare", assert_obj!("values" => values))
        .await?;
    must_let!(let ConvexValue::Array(sorted_result) = value);

    let sorted_result_vec = sorted_result.to_vec();
    for (i, value) in sorted_result_vec.iter().enumerate() {
        if value != &sorted_values[i] {
            println!("js sort: {sorted_result_vec:?}");
            println!("rust sort: {sorted_values:?}");
            println!("js value at index {i:?}: {value:?}");
            println!("rust value at index {:?}: {:?}", i, sorted_values[i]);
        }
        assert_eq!(value, &sorted_values[i]);
    }
    Ok(())
}

#[convex_macro::test_runtime]
async fn test_compare_nan_and_zero(rt: TestRuntime) -> anyhow::Result<()> {
    // This is the most basic test for special values like `NaN` and negative zero.
    let values = vec![
        ConvexValue::Float64(-f64::NAN),
        ConvexValue::Float64(-0.0),
        ConvexValue::Float64(0.0),
        ConvexValue::Float64(1.0),
        ConvexValue::Float64(f64::NAN),
    ];
    for i in 0..values.len() {
        for j in i + 1..values.len() {
            test_compare(rt.clone(), vec![values[i].clone(), values[j].clone()]).await?;
        }
    }
    Ok(())
}

#[convex_macro::test_runtime]
async fn test_compare_utf16_strings(rt: TestRuntime) -> anyhow::Result<()> {
    // This test case was surfaced by proptests -- JS has UTF-16 strings that get
    // compared differently in JS vs. Rust.
    let values = vec![
        ConvexValue::String("𐏍ꢕ¥🕴JಏÚﶒ੫'".to_string().try_into()?),
        ConvexValue::String("ﹲ𓓄\\𝕆".to_string().try_into()?),
    ];
    test_compare(rt.clone(), values).await?;
    Ok(())
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 32 * env_config("CONVEX_PROPTEST_MULTIPLIER", 1), failure_persistence: None, .. ProptestConfig::default() })]

    #[test]
    fn proptest_compare_values(values in prop::collection::vec(any_with::<ConvexValue>((
        FieldType::User,
        ValueBranching::small(),
        RestrictNaNs(true),
    )), 10)) {
        // This tests that the JS implementation for comparing values matches the Rust implementation.
        // It does so by generating an array of values, and then calling a query that compares the values,
        // and then checking that the result is the same as the Rust implementation.
        // There's overhead to calling the query, so we generate a list of values instead of comparing
        // two values directly.
        // We also restrict this to only use one type of `NaN` because apparently some of them change when
        // put into an array.
        let td = TestDriver::new();
        let rt = td.rt();
        td.run_until(test_compare(rt, values)).unwrap();
    }
}

async fn test_size(rt: TestRuntime, value: ConvexValue) -> anyhow::Result<()> {
    let udf = UdfTest::default(rt).await?;
    let rust_size = value.size();

    let js_result = udf
        .query("values:size", assert_obj!("value" => value.clone()))
        .await?;
    must_let!(let ConvexValue::Int64(js_size) = js_result);

    assert_eq!(
        rust_size, js_size as usize,
        "Size mismatch for {value:?}: Rust got {rust_size}, JS got {js_size}"
    );
    Ok(())
}

#[convex_macro::test_runtime]
async fn test_size_null(rt: TestRuntime) -> anyhow::Result<()> {
    test_size(rt, ConvexValue::Null).await
}

#[convex_macro::test_runtime]
async fn test_size_boolean(rt: TestRuntime) -> anyhow::Result<()> {
    test_size(rt.clone(), ConvexValue::Boolean(true)).await?;
    test_size(rt, ConvexValue::Boolean(false)).await
}

#[convex_macro::test_runtime]
async fn test_size_int64(rt: TestRuntime) -> anyhow::Result<()> {
    test_size(rt.clone(), ConvexValue::Int64(0)).await?;
    test_size(rt.clone(), ConvexValue::Int64(42)).await?;
    test_size(rt, ConvexValue::Int64(-1)).await
}

#[convex_macro::test_runtime]
async fn test_size_float64(rt: TestRuntime) -> anyhow::Result<()> {
    test_size(rt.clone(), ConvexValue::Float64(0.0)).await?;
    test_size(rt.clone(), ConvexValue::Float64(2.5)).await?;
    test_size(rt.clone(), ConvexValue::Float64(-0.0)).await?;
    test_size(rt, ConvexValue::Float64(f64::INFINITY)).await
}

#[convex_macro::test_runtime]
async fn test_size_string(rt: TestRuntime) -> anyhow::Result<()> {
    // Empty string
    test_size(rt.clone(), ConvexValue::String("".try_into()?)).await?;
    // ASCII string
    test_size(rt.clone(), ConvexValue::String("hello".try_into()?)).await?;
    // 2-byte UTF-8 (Latin characters with diacritics)
    test_size(rt.clone(), ConvexValue::String("café".try_into()?)).await?;
    // 3-byte UTF-8 (Euro sign)
    test_size(rt.clone(), ConvexValue::String("€".try_into()?)).await?;
    // 4-byte UTF-8 (emoji)
    test_size(rt.clone(), ConvexValue::String("hi😀".try_into()?)).await?;
    // Mixed UTF-8
    test_size(rt, ConvexValue::String("hello 世界 🌍".try_into()?)).await
}

#[convex_macro::test_runtime]
async fn test_size_bytes(rt: TestRuntime) -> anyhow::Result<()> {
    // Empty bytes
    test_size(rt.clone(), ConvexValue::Bytes(vec![].try_into()?)).await?;
    // Some bytes
    test_size(
        rt,
        ConvexValue::Bytes(vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10].try_into()?),
    )
    .await
}

#[convex_macro::test_runtime]
async fn test_size_array(rt: TestRuntime) -> anyhow::Result<()> {
    // Empty array
    test_size(rt.clone(), ConvexValue::Array(vec![].try_into()?)).await?;
    // Array with null
    test_size(
        rt.clone(),
        ConvexValue::Array(vec![ConvexValue::Null].try_into()?),
    )
    .await?;
    // Array with number
    test_size(
        rt.clone(),
        ConvexValue::Array(vec![ConvexValue::Float64(1.0)].try_into()?),
    )
    .await?;
    // Nested array
    test_size(
        rt.clone(),
        ConvexValue::Array(vec![ConvexValue::Array(vec![].try_into()?)].try_into()?),
    )
    .await?;
    // Mixed array
    test_size(
        rt,
        ConvexValue::Array(
            vec![
                ConvexValue::String("hello world".try_into()?),
                ConvexValue::Float64(42.0),
            ]
            .try_into()?,
        ),
    )
    .await
}

#[convex_macro::test_runtime]
async fn test_size_object(rt: TestRuntime) -> anyhow::Result<()> {
    // Empty object
    test_size(rt.clone(), assert_val!({})).await?;
    // Object with null
    test_size(rt.clone(), assert_val!({"a" => ConvexValue::Null})).await?;
    // Object with number
    test_size(rt.clone(), assert_val!({"a" => 1.0})).await?;
    // Object with string
    test_size(rt.clone(), assert_val!({"foo" => "bar"})).await?;
    // Nested object
    test_size(rt.clone(), assert_val!({"a" => {}})).await?;
    // Complex nested object
    test_size(rt.clone(), assert_val!({"nested" => {"value" => 42.0}})).await?;
    // Typical document
    test_size(
        rt,
        assert_val!({"name" => "Alice", "age" => 30.0, "active" => true}),
    )
    .await
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 32 * env_config("CONVEX_PROPTEST_MULTIPLIER", 1), failure_persistence: None, .. ProptestConfig::default() })]

    #[test]
    fn proptest_size_values(value in any_with::<ConvexValue>((
        FieldType::User,
        ValueBranching::small(),
        RestrictNaNs(true),
    ))) {
        // This tests that the JS implementation for calculating value size matches the Rust
        // implementation. It calls a query that computes getConvexSize() in JS and compares
        // the result to Rust's Size::size().
        let td = TestDriver::new();
        let rt = td.rt();
        td.run_until(test_size(rt, value)).unwrap();
    }
}

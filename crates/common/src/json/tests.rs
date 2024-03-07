use std::{
    convert::TryFrom,
    str::FromStr,
};

use proptest::prelude::*;
use serde_json::{
    json,
    Value as JsonValue,
};
use value::ConvexValue;

use super::expression::JsonExpression;
use crate::{
    paths::FieldPath,
    query::{
        Expression,
        Query,
    },
    testing::assert_roundtrips,
};

#[test]
fn test_parse_expr() -> anyhow::Result<()> {
    fn test_case(v: serde_json::Value, expected: Expression) -> anyhow::Result<()> {
        assert_eq!(
            Expression::try_from(serde_json::from_str::<JsonExpression>(&v.to_string())?)?,
            expected
        );
        Ok(())
    }

    test_case(
        json!({ "$literal": "foo" }),
        Expression::Literal(ConvexValue::try_from("foo")?.into()),
    )?;
    test_case(
        json!({
            "$field": "email"
        }),
        Expression::Field(FieldPath::from_str("email")?),
    )?;
    test_case(
        json!({
            "$eq": [
                { "$field": "email" },
                { "$literal": "bw@convex.dev" },
            ],
        }),
        Expression::Eq(
            Box::new(Expression::Field(FieldPath::from_str("email")?)),
            Box::new(Expression::Literal(
                ConvexValue::try_from("bw@convex.dev")?.into(),
            )),
        ),
    )?;
    test_case(
        json!({
            "$and": [
                { "$literal": true },
                { "$literal": false },
                { "$literal": true },
            ],
        }),
        Expression::And(vec![
            Expression::Literal(ConvexValue::from(true).into()),
            Expression::Literal(ConvexValue::from(false).into()),
            Expression::Literal(ConvexValue::from(true).into()),
        ]),
    )?;

    Ok(())
}

#[test]
fn test_parse_query() -> anyhow::Result<()> {
    Query::try_from(json!({
        "source": { "type": "FullTableScan", "tableName": "users", "order": "asc" },
        "operators": [
            { "filter": { "$eq": [ {"$field": "email"}, { "$literal": "bw@convex.dev" } ] } },
        ],
    }))?;

    Ok(())
}

proptest! {
    #![proptest_config(
            ProptestConfig { failure_persistence: None, ..ProptestConfig::default() }
        )]

    #[test]
    fn test_query_roundtrips_to_json(query in any::<Query>()) {
        assert_roundtrips::<Query, JsonValue>(query);
    }
}

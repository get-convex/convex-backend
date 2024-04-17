use anyhow::Result;
use serde::{
    Deserialize,
    Serialize,
};
use serde_json::Value as JsonValue;

use crate::query::Expression;

#[derive(Deserialize, Serialize)]
pub enum JsonExpression {
    #[serde(rename = "$eq")]
    Eq(Box<JsonExpression>, Box<JsonExpression>),
    #[serde(rename = "$neq")]
    Neq(Box<JsonExpression>, Box<JsonExpression>),
    #[serde(rename = "$lt")]
    Lt(Box<JsonExpression>, Box<JsonExpression>),
    #[serde(rename = "$lte")]
    Lte(Box<JsonExpression>, Box<JsonExpression>),
    #[serde(rename = "$gt")]
    Gt(Box<JsonExpression>, Box<JsonExpression>),
    #[serde(rename = "$gte")]
    Gte(Box<JsonExpression>, Box<JsonExpression>),
    #[serde(rename = "$add")]
    Add(Box<JsonExpression>, Box<JsonExpression>),
    #[serde(rename = "$sub")]
    Sub(Box<JsonExpression>, Box<JsonExpression>),
    #[serde(rename = "$mul")]
    Mul(Box<JsonExpression>, Box<JsonExpression>),
    #[serde(rename = "$div")]
    Div(Box<JsonExpression>, Box<JsonExpression>),
    #[serde(rename = "$mod")]
    Mod(Box<JsonExpression>, Box<JsonExpression>),
    #[serde(rename = "$neg")]
    Neg(Box<JsonExpression>),
    #[serde(rename = "$and")]
    And(Vec<JsonExpression>),
    #[serde(rename = "$or")]
    Or(Vec<JsonExpression>),
    #[serde(rename = "$not")]
    Not(Box<JsonExpression>),
    #[serde(rename = "$field")]
    Field(String),
    #[serde(rename = "$literal")]
    Literal(JsonValue),
}

impl TryFrom<JsonExpression> for Expression {
    type Error = anyhow::Error;

    fn try_from(json_expr: JsonExpression) -> Result<Self> {
        let expr = match json_expr {
            JsonExpression::Eq(l, r) => Expression::Eq(
                Box::new(Expression::try_from(*l)?),
                Box::new(Expression::try_from(*r)?),
            ),
            JsonExpression::Neq(l, r) => Expression::Neq(
                Box::new(Expression::try_from(*l)?),
                Box::new(Expression::try_from(*r)?),
            ),
            JsonExpression::Lt(l, r) => Expression::Lt(
                Box::new(Expression::try_from(*l)?),
                Box::new(Expression::try_from(*r)?),
            ),
            JsonExpression::Lte(l, r) => Expression::Lte(
                Box::new(Expression::try_from(*l)?),
                Box::new(Expression::try_from(*r)?),
            ),
            JsonExpression::Gt(l, r) => Expression::Gt(
                Box::new(Expression::try_from(*l)?),
                Box::new(Expression::try_from(*r)?),
            ),
            JsonExpression::Gte(l, r) => Expression::Gte(
                Box::new(Expression::try_from(*l)?),
                Box::new(Expression::try_from(*r)?),
            ),
            JsonExpression::Add(l, r) => Expression::Add(
                Box::new(Expression::try_from(*l)?),
                Box::new(Expression::try_from(*r)?),
            ),
            JsonExpression::Sub(l, r) => Expression::Sub(
                Box::new(Expression::try_from(*l)?),
                Box::new(Expression::try_from(*r)?),
            ),
            JsonExpression::Mul(l, r) => Expression::Mul(
                Box::new(Expression::try_from(*l)?),
                Box::new(Expression::try_from(*r)?),
            ),
            JsonExpression::Div(l, r) => Expression::Div(
                Box::new(Expression::try_from(*l)?),
                Box::new(Expression::try_from(*r)?),
            ),
            JsonExpression::Mod(l, r) => Expression::Mod(
                Box::new(Expression::try_from(*l)?),
                Box::new(Expression::try_from(*r)?),
            ),
            JsonExpression::Neg(x) => Expression::Neg(Box::new(Expression::try_from(*x)?)),
            JsonExpression::And(vs) => Expression::And(
                vs.into_iter()
                    .map(Expression::try_from)
                    .collect::<anyhow::Result<Vec<Expression>>>()?,
            ),
            JsonExpression::Or(vs) => Expression::Or(
                vs.into_iter()
                    .map(Expression::try_from)
                    .collect::<anyhow::Result<Vec<Expression>>>()?,
            ),
            JsonExpression::Not(x) => Expression::Not(Box::new(Expression::try_from(*x)?)),
            JsonExpression::Field(field_path_str) => Expression::Field(field_path_str.parse()?),
            JsonExpression::Literal(v) => Expression::Literal(v.try_into()?),
        };
        Ok(expr)
    }
}

impl From<Expression> for JsonExpression {
    fn from(expression: Expression) -> Self {
        match expression {
            Expression::Eq(l, r) => {
                JsonExpression::Eq(Box::new((*l).into()), Box::new((*r).into()))
            },
            Expression::Neq(l, r) => {
                JsonExpression::Neq(Box::new((*l).into()), Box::new((*r).into()))
            },
            Expression::Lt(l, r) => {
                JsonExpression::Lt(Box::new((*l).into()), Box::new((*r).into()))
            },
            Expression::Lte(l, r) => {
                JsonExpression::Lte(Box::new((*l).into()), Box::new((*r).into()))
            },
            Expression::Gt(l, r) => {
                JsonExpression::Gt(Box::new((*l).into()), Box::new((*r).into()))
            },
            Expression::Gte(l, r) => {
                JsonExpression::Gte(Box::new((*l).into()), Box::new((*r).into()))
            },
            Expression::Add(l, r) => {
                JsonExpression::Add(Box::new((*l).into()), Box::new((*r).into()))
            },
            Expression::Sub(l, r) => {
                JsonExpression::Sub(Box::new((*l).into()), Box::new((*r).into()))
            },
            Expression::Mul(l, r) => {
                JsonExpression::Mul(Box::new((*l).into()), Box::new((*r).into()))
            },
            Expression::Div(l, r) => {
                JsonExpression::Div(Box::new((*l).into()), Box::new((*r).into()))
            },
            Expression::Mod(l, r) => {
                JsonExpression::Mod(Box::new((*l).into()), Box::new((*r).into()))
            },
            Expression::Neg(x) => JsonExpression::Neg(Box::new((*x).into())),
            Expression::And(vs) => {
                JsonExpression::And(vs.into_iter().map(JsonExpression::from).collect())
            },
            Expression::Or(vs) => {
                JsonExpression::Or(vs.into_iter().map(JsonExpression::from).collect())
            },
            Expression::Not(x) => JsonExpression::Not(Box::new((*x).into())),
            Expression::Field(field_path) => JsonExpression::Field(field_path.into()),
            Expression::Literal(v) => JsonExpression::Literal(v.into()),
        }
    }
}

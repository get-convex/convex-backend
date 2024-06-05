use std::{
    ops::Deref,
    str::FromStr,
};

use chrono::{
    DateTime,
    Utc,
};
use common::value::ConvexObject;
use serde::{
    Deserialize,
    Serialize,
};

#[derive(
    Hash, Eq, PartialEq, derive_more::Display, Debug, serde::Deserialize, Clone, PartialOrd, Ord,
)]
#[serde(transparent)]
pub struct FivetranFieldName(String);

impl FromStr for FivetranFieldName {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(s.to_owned()))
    }
}

impl Deref for FivetranFieldName {
    type Target = str;

    fn deref(&self) -> &str {
        &self.0[..]
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum BatchWriteOperation {
    Upsert,
    Update,
    HardDelete,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BatchWriteRow {
    pub table: String,
    pub operation: BatchWriteOperation,
    pub row: ConvexObject,
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq)]
pub enum DeleteType {
    SoftDelete,
    HardDelete,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TruncateTableArgs {
    pub table_name: String,
    pub delete_before: Option<DateTime<Utc>>,
    pub delete_type: DeleteType,
}

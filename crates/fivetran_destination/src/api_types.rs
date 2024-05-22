use std::{
    ops::Deref,
    str::FromStr,
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

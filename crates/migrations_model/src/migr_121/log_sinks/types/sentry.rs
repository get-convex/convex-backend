use std::{
    collections::BTreeMap,
    fmt::{
        self,
        Display,
    },
    str::FromStr,
};

use common::pii::PII;
use sentry::types::Dsn;
use serde::{
    Deserialize,
    Serialize,
};
use value::FieldName;

#[cfg(any(test, feature = "testing"))]
pub const TEST_DSN: &str =
    "https://ef1d32d342354c87869ab2db8b490b2c@o1192621.ingest.sentry.io/6333191";

#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(any(test, feature = "testing"), derive(proptest_derive::Arbitrary))]
pub struct SentryConfig {
    #[cfg_attr(
        any(test, feature = "testing"),
        proptest(value = "(TEST_DSN.parse::<Dsn>().unwrap()).into()")
    )]
    pub dsn: PII<Dsn>,
    pub tags: Option<BTreeMap<FieldName, String>>,
    pub version: ExceptionFormatVersion,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SerializedSentryConfig {
    pub dsn: String,
    pub tags: Option<BTreeMap<FieldName, String>>,
    pub version: Option<String>,
}

impl TryFrom<SentryConfig> for SerializedSentryConfig {
    type Error = anyhow::Error;

    fn try_from(value: SentryConfig) -> Result<Self, Self::Error> {
        Ok(Self {
            dsn: value.dsn.into_value().to_string(),
            tags: value.tags,
            version: Some(value.version.to_string()),
        })
    }
}

impl TryFrom<SerializedSentryConfig> for SentryConfig {
    type Error = anyhow::Error;

    fn try_from(value: SerializedSentryConfig) -> Result<Self, Self::Error> {
        Ok(Self {
            dsn: value.dsn.parse::<Dsn>()?.into(),
            tags: value.tags,
            version: match value.version {
                Some(v) => ExceptionFormatVersion::from_str(&v)?,
                // Treat missing version as V1
                None => ExceptionFormatVersion::V1,
            },
        })
    }
}

impl fmt::Display for SentryConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "SentryConfig {{ dsn: ..., version: {} }}", self.version)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[cfg_attr(any(test, feature = "testing"), derive(proptest_derive::Arbitrary))]
pub enum ExceptionFormatVersion {
    V1,
    V2,
}

impl FromStr for ExceptionFormatVersion {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "1" => Ok(Self::V1),
            "2" => Ok(Self::V2),
            v => anyhow::bail!("Invalid ExceptionFormatVersion: {v}"),
        }
    }
}

impl Display for ExceptionFormatVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::V1 => write!(f, "1"),
            Self::V2 => write!(f, "2"),
        }
    }
}

#[cfg(any(test, feature = "testing"))]
impl Default for ExceptionFormatVersion {
    fn default() -> Self {
        Self::V2
    }
}

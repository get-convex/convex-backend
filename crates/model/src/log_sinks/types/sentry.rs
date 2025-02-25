use std::{
    collections::BTreeMap,
    fmt,
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
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SerializedSentryConfig {
    pub dsn: String,
    pub tags: Option<BTreeMap<FieldName, String>>,
}

impl TryFrom<SentryConfig> for SerializedSentryConfig {
    type Error = anyhow::Error;

    fn try_from(value: SentryConfig) -> Result<Self, Self::Error> {
        Ok(Self {
            dsn: value.dsn.into_value().to_string(),
            tags: value.tags,
        })
    }
}

impl TryFrom<SerializedSentryConfig> for SentryConfig {
    type Error = anyhow::Error;

    fn try_from(value: SerializedSentryConfig) -> Result<Self, Self::Error> {
        Ok(Self {
            dsn: value.dsn.parse::<Dsn>()?.into(),
            tags: value.tags,
        })
    }
}

impl fmt::Display for SentryConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "SentryConfig {{ dsn: ... }}")
    }
}

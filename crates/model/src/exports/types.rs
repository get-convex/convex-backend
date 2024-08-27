use std::{
    fmt,
    fmt::Display,
};

use common::{
    obj,
    types::ObjectKey,
};
use maplit::btreemap;
use sync_types::Timestamp;
use value::{
    val,
    ConvexObject,
    ConvexValue,
};

const EXPORT_RETENTION: u64 = 14 * 24 * 60 * 60 * 1000000000; // 14 days

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(any(test, feature = "testing"), derive(proptest_derive::Arbitrary))]
pub enum Export {
    Requested {
        format: ExportFormat,
    },
    InProgress {
        /// Timestamp when the first attempt
        /// at the Export started.
        start_ts: Timestamp,
        format: ExportFormat,
    },
    Completed {
        /// Timestamp for the successful (final) attempt at Export.
        start_ts: Timestamp,
        /// Timestamp when the Export completed
        complete_ts: Timestamp,
        /// Expiration timestamp
        expiration_ts: u64,
        /// Object keys in S3
        object_keys: ExportObjectKeys,
        /// Format of the export
        format: ExportFormat,
    },
    Failed {
        /// Timestamp for the failed (final) attempt at Export.
        start_ts: Timestamp,
        /// Timestamp when the Export failed
        failed_ts: Timestamp,
        format: ExportFormat,
    },
}

impl Export {
    pub fn format(&self) -> ExportFormat {
        match self {
            Export::Requested { format }
            | Export::InProgress { format, .. }
            | Export::Completed { format, .. }
            | Export::Failed { format, .. } => *format,
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
#[cfg_attr(any(test, feature = "testing"), derive(proptest_derive::Arbitrary))]
pub enum ExportFormat {
    /// zip file containing a CleanJsonl for each table, and sidecar type info.
    Zip { include_storage: bool },
}

impl Export {
    pub fn requested(format: ExportFormat) -> Self {
        Self::Requested { format }
    }

    pub fn in_progress(self, ts: Timestamp) -> anyhow::Result<Export> {
        match self {
            Self::Requested { format } => Ok(Self::InProgress {
                start_ts: ts,
                format,
            }),
            Self::Completed { .. } | Self::InProgress { .. } | Self::Failed { .. } => Err(
                anyhow::anyhow!("Can only begin an export that is requested"),
            ),
        }
    }

    pub fn completed(
        self,
        snapshot_ts: Timestamp,
        complete_ts: Timestamp,
        object_keys: ExportObjectKeys,
    ) -> anyhow::Result<Export> {
        let expiration_ts = Into::<u64>::into(complete_ts) + EXPORT_RETENTION;
        match self {
            Self::InProgress { format, .. } => {
                anyhow::ensure!(snapshot_ts <= complete_ts);
                Ok(Self::Completed {
                    start_ts: snapshot_ts,
                    complete_ts,
                    expiration_ts,
                    object_keys,
                    format,
                })
            },
            Self::Requested { format: _ }
            | Self::Completed {
                start_ts: _,
                complete_ts: _,
                expiration_ts: _,
                object_keys: _,
                format: _,
            }
            | Self::Failed {
                start_ts: _,
                failed_ts: _,
                format: _,
            } => Err(anyhow::anyhow!(
                "Can only complete an export that is in_progress"
            )),
        }
    }

    pub fn failed(self, snapshot_ts: Timestamp, failed_ts: Timestamp) -> anyhow::Result<Export> {
        match self {
            Self::InProgress { format, .. } => {
                anyhow::ensure!(snapshot_ts <= failed_ts);
                Ok(Self::Failed {
                    start_ts: snapshot_ts,
                    failed_ts,
                    format,
                })
            },
            Self::Requested { format: _ }
            | Self::Completed {
                start_ts: _,
                complete_ts: _,
                expiration_ts: _,
                object_keys: _,
                format: _,
            }
            | Self::Failed {
                start_ts: _,
                failed_ts: _,
                format: _,
            } => Err(anyhow::anyhow!(
                "Can only fail an export that is in_progress"
            )),
        }
    }
}

impl Display for Export {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Requested { format: _ } => write!(f, "requested"),
            Self::InProgress {
                start_ts: _,
                format: _,
            } => write!(f, "in_progress"),
            Self::Completed {
                start_ts: _,
                complete_ts: _,
                expiration_ts: _,
                object_keys: _,
                format: _,
            } => write!(f, "completed"),
            Self::Failed {
                start_ts: _,
                failed_ts: _,
                format: _,
            } => write!(f, "failed"),
        }
    }
}

impl TryFrom<Export> for ConvexObject {
    type Error = anyhow::Error;

    fn try_from(e: Export) -> anyhow::Result<ConvexObject> {
        match e {
            Export::Completed {
                start_ts,
                complete_ts,
                expiration_ts,
                object_keys,
                format,
            } => {
                let mut o = btreemap! {
                    "start_ts".parse()? => val!(i64::from(start_ts)),
                    "complete_ts".parse()? => val!(i64::from(complete_ts)),
                    "expiration_ts".parse()? => val!(expiration_ts as i64),
                    "state".parse()? => val!("completed"),
                    "format".parse()? => val!(format),
                };
                match object_keys {
                    ExportObjectKeys::Zip(object_key) => o.insert(
                        "zip_object_key".parse()?,
                        ConvexValue::try_from(object_key.to_string())?,
                    ),
                };
                ConvexObject::try_from(o)
            },
            Export::Requested { format } => obj!(
                "state" => "requested",
                "format" => format,
            ),
            Export::InProgress { start_ts, format } => {
                obj!(
                    "state" => "in_progress",
                    "start_ts" => i64::from(start_ts),
                    "format" => format,
                )
            },
            Export::Failed {
                start_ts,
                failed_ts,
                format,
            } => {
                obj!(
                    "state" => "failed",
                    "start_ts" => i64::from(start_ts),
                    "failed_ts" => i64::from(failed_ts),
                    "format" => format,
                )
            },
        }
    }
}

impl TryFrom<ConvexObject> for Export {
    type Error = anyhow::Error;

    fn try_from(o: ConvexObject) -> anyhow::Result<Export> {
        let format = match o.get("format") {
            Some(format) => ExportFormat::try_from(format.clone())?,
            _ => anyhow::bail!("invalid format: {:?}", o),
        };
        match o.get("state") {
            Some(ConvexValue::String(s)) => match &s[..] {
                "requested" => Ok(Export::Requested { format }),
                "in_progress" => {
                    if let Some(start_ts_value) = o.get("start_ts")
                        && let ConvexValue::Int64(start_ts) = start_ts_value
                    {
                        Ok(Export::InProgress {
                            start_ts: (*start_ts).try_into()?,
                            format,
                        })
                    } else {
                        Err(anyhow::anyhow!("No start_ts found for in_progress export."))
                    }
                },
                "completed" => {
                    let start_ts = match o.get("start_ts") {
                        Some(ConvexValue::Int64(t)) => (*t).try_into()?,
                        _ => anyhow::bail!("invalid start_ts: {:?}", o),
                    };
                    let complete_ts = match o.get("complete_ts") {
                        Some(ConvexValue::Int64(t)) => (*t).try_into()?,
                        _ => anyhow::bail!("invalid complete_ts: {:?}", o),
                    };
                    let expiration_ts = match o.get("expiration_ts") {
                        Some(ConvexValue::Int64(t)) => *t as u64,
                        _ => anyhow::bail!("invalid expiration_ts: {:?}", o),
                    };
                    let object_keys = match o.get("zip_object_key") {
                        Some(ConvexValue::String(zip_object_key)) => {
                            ExportObjectKeys::Zip(String::from(zip_object_key.clone()).try_into()?)
                        },
                        _ => anyhow::bail!("invalid object keys: {:?}", o),
                    };
                    Ok(Export::Completed {
                        expiration_ts,
                        start_ts,
                        complete_ts,
                        object_keys,
                        format,
                    })
                },
                "failed" => {
                    let start_ts = match o.get("start_ts") {
                        Some(ConvexValue::Int64(t)) => (*t).try_into()?,
                        _ => anyhow::bail!("invalid start_ts: {:?}", o),
                    };
                    let failed_ts = match o.get("failed_ts") {
                        Some(ConvexValue::Int64(t)) => (*t).try_into()?,
                        _ => anyhow::bail!("invalid failed_ts: {:?}", o),
                    };
                    Ok(Export::Failed {
                        start_ts,
                        failed_ts,
                        format,
                    })
                },
                _ => Err(anyhow::anyhow!("Invalid export state {s}")),
            },
            Some(_) | None => Err(anyhow::anyhow!("No export state found for export.")),
        }
    }
}

impl TryFrom<ExportFormat> for ConvexValue {
    type Error = anyhow::Error;

    fn try_from(value: ExportFormat) -> Result<Self, Self::Error> {
        let v = match value {
            ExportFormat::Zip { include_storage } => {
                val!({"format" => "zip", "include_storage" => include_storage})
            },
        };
        Ok(v)
    }
}

impl TryFrom<ConvexValue> for ExportFormat {
    type Error = anyhow::Error;

    fn try_from(value: ConvexValue) -> Result<Self, Self::Error> {
        let f = match &value {
            ConvexValue::Object(o) => match o.get("format") {
                Some(ConvexValue::String(format)) => match &**format {
                    "zip" => match o.get("include_storage") {
                        Some(ConvexValue::Boolean(include_storage)) => Self::Zip {
                            include_storage: *include_storage,
                        },
                        _ => anyhow::bail!("invalid format {value:?}"),
                    },
                    _ => anyhow::bail!("invalid format {value:?}"),
                },
                _ => anyhow::bail!("invalid format {value:?}"),
            },
            _ => anyhow::bail!("invalid format {value:?}"),
        };
        Ok(f)
    }
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(any(test, feature = "testing"), derive(proptest_derive::Arbitrary))]
pub enum ExportObjectKeys {
    Zip(ObjectKey),
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;
    use sync_types::testing::assert_roundtrips;
    use value::ConvexObject;

    use super::Export;

    proptest! {
        #![proptest_config(
            ProptestConfig { failure_persistence: None, ..ProptestConfig::default() }
        )]

        #[test]
        fn test_export_roundtrip(v in any::<Export>()) {
            assert_roundtrips::<Export, ConvexObject>(v);
        }
    }
}

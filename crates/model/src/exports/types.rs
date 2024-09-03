use std::{
    fmt,
    fmt::Display,
};

use common::{
    components::ComponentId,
    obj,
    types::ObjectKey,
};
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
        component: ComponentId,
    },
    InProgress {
        /// Timestamp when the first attempt
        /// at the Export started.
        start_ts: Timestamp,
        format: ExportFormat,
        component: ComponentId,
    },
    Completed {
        /// Timestamp for the successful (final) attempt at Export.
        start_ts: Timestamp,
        /// Timestamp when the Export completed
        complete_ts: Timestamp,
        /// Expiration timestamp
        expiration_ts: u64,
        /// Object keys in S3
        zip_object_key: ObjectKey,
        /// Format of the export
        format: ExportFormat,
        component: ComponentId,
    },
    Failed {
        /// Timestamp for the failed (final) attempt at Export.
        start_ts: Timestamp,
        /// Timestamp when the Export failed
        failed_ts: Timestamp,
        format: ExportFormat,
        component: ComponentId,
    },
}

impl Export {
    pub fn format(&self) -> ExportFormat {
        match self {
            Export::Requested { format, .. }
            | Export::InProgress { format, .. }
            | Export::Completed { format, .. }
            | Export::Failed { format, .. } => *format,
        }
    }

    pub fn component(&self) -> ComponentId {
        match self {
            Export::Requested { component, .. }
            | Export::InProgress { component, .. }
            | Export::Completed { component, .. }
            | Export::Failed { component, .. } => *component,
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
    pub fn requested(format: ExportFormat, component: ComponentId) -> Self {
        Self::Requested { format, component }
    }

    pub fn in_progress(self, ts: Timestamp) -> anyhow::Result<Export> {
        match self {
            Self::Requested { format, component } => Ok(Self::InProgress {
                start_ts: ts,
                format,
                component,
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
        zip_object_key: ObjectKey,
    ) -> anyhow::Result<Export> {
        let expiration_ts = Into::<u64>::into(complete_ts) + EXPORT_RETENTION;
        match self {
            Self::InProgress {
                format, component, ..
            } => {
                anyhow::ensure!(snapshot_ts <= complete_ts);
                Ok(Self::Completed {
                    start_ts: snapshot_ts,
                    complete_ts,
                    expiration_ts,
                    zip_object_key,
                    format,
                    component,
                })
            },
            Self::Requested {
                format: _,
                component: _,
            }
            | Self::Completed {
                start_ts: _,
                complete_ts: _,
                expiration_ts: _,
                zip_object_key: _,
                format: _,
                component: _,
            }
            | Self::Failed {
                start_ts: _,
                failed_ts: _,
                format: _,
                component: _,
            } => Err(anyhow::anyhow!(
                "Can only complete an export that is in_progress"
            )),
        }
    }

    pub fn failed(self, snapshot_ts: Timestamp, failed_ts: Timestamp) -> anyhow::Result<Export> {
        match self {
            Self::InProgress {
                format, component, ..
            } => {
                anyhow::ensure!(snapshot_ts <= failed_ts);
                Ok(Self::Failed {
                    start_ts: snapshot_ts,
                    failed_ts,
                    format,
                    component,
                })
            },
            Self::Requested {
                format: _,
                component: _,
            }
            | Self::Completed {
                start_ts: _,
                complete_ts: _,
                expiration_ts: _,
                zip_object_key: _,
                format: _,
                component: _,
            }
            | Self::Failed {
                start_ts: _,
                failed_ts: _,
                format: _,
                component: _,
            } => Err(anyhow::anyhow!(
                "Can only fail an export that is in_progress"
            )),
        }
    }
}

impl Display for Export {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Requested {
                format: _,
                component: _,
            } => write!(f, "requested"),
            Self::InProgress {
                start_ts: _,
                format: _,
                component: _,
            } => write!(f, "in_progress"),
            Self::Completed {
                start_ts: _,
                complete_ts: _,
                expiration_ts: _,
                zip_object_key: _,
                format: _,
                component: _,
            } => write!(f, "completed"),
            Self::Failed {
                start_ts: _,
                failed_ts: _,
                format: _,
                component: _,
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
                zip_object_key,
                format,
                component,
            } => {
                obj!(
                    "start_ts" => i64::from(start_ts),
                    "complete_ts" => i64::from(complete_ts),
                    "expiration_ts" => expiration_ts as i64,
                    "state" => "completed",
                    "format" => format,
                    "component" => component.serialize_to_string(),
                    "zip_object_key" => zip_object_key.to_string(),
                )
            },
            Export::Requested { format, component } => obj!(
                "state" => "requested",
                "format" => format,
                "component" => component.serialize_to_string(),
            ),
            Export::InProgress {
                start_ts,
                format,
                component,
            } => {
                obj!(
                    "state" => "in_progress",
                    "start_ts" => i64::from(start_ts),
                    "format" => format,
                    "component" => component.serialize_to_string(),
                )
            },
            Export::Failed {
                start_ts,
                failed_ts,
                format,
                component,
            } => {
                obj!(
                    "state" => "failed",
                    "start_ts" => i64::from(start_ts),
                    "failed_ts" => i64::from(failed_ts),
                    "format" => format,
                    "component" => component.serialize_to_string(),
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
        let component = match o.get("component") {
            Some(ConvexValue::String(s)) => ComponentId::deserialize_from_string(Some(s))?,
            Some(ConvexValue::Null) => ComponentId::Root,
            None => ComponentId::Root,
            _ => anyhow::bail!("invalid component: {:?}", o),
        };
        match o.get("state") {
            Some(ConvexValue::String(s)) => match &s[..] {
                "requested" => Ok(Export::Requested { format, component }),
                "in_progress" => {
                    if let Some(start_ts_value) = o.get("start_ts")
                        && let ConvexValue::Int64(start_ts) = start_ts_value
                    {
                        Ok(Export::InProgress {
                            start_ts: (*start_ts).try_into()?,
                            format,
                            component,
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
                    let zip_object_key = match o.get("zip_object_key") {
                        Some(ConvexValue::String(zip_object_key)) => {
                            zip_object_key.clone().try_into()?
                        },
                        _ => anyhow::bail!("invalid object keys: {:?}", o),
                    };
                    Ok(Export::Completed {
                        expiration_ts,
                        start_ts,
                        complete_ts,
                        zip_object_key,
                        format,
                        component,
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
                        component,
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

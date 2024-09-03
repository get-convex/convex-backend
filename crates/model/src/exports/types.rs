use std::{
    fmt,
    fmt::Display,
};

use common::{
    components::ComponentId,
    types::ObjectKey,
};
use serde::{
    Deserialize,
    Serialize,
};
use sync_types::Timestamp;
use value::codegen_convex_serialization;

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

#[derive(Serialize, Deserialize)]
#[serde(tag = "state")]
#[serde(rename_all = "snake_case")]
enum SerializedExport {
    Requested {
        format: SerializedExportFormat,
        component: Option<String>,
    },
    InProgress {
        start_ts: u64,
        format: SerializedExportFormat,
        component: Option<String>,
    },
    Completed {
        start_ts: u64,
        complete_ts: u64,
        expiration_ts: i64,
        zip_object_key: String,
        format: SerializedExportFormat,
        component: Option<String>,
    },
    Failed {
        start_ts: u64,
        failed_ts: u64,
        format: SerializedExportFormat,
        component: Option<String>,
    },
}

impl TryFrom<Export> for SerializedExport {
    type Error = anyhow::Error;

    fn try_from(value: Export) -> Result<Self, Self::Error> {
        Ok(match value {
            Export::Requested { format, component } => SerializedExport::Requested {
                format: format.into(),
                component: component.serialize_to_string(),
            },
            Export::InProgress {
                start_ts,
                format,
                component,
            } => SerializedExport::InProgress {
                start_ts: start_ts.into(),
                format: format.into(),
                component: component.serialize_to_string(),
            },
            Export::Completed {
                start_ts,
                complete_ts,
                expiration_ts,
                zip_object_key,
                format,
                component,
            } => SerializedExport::Completed {
                start_ts: start_ts.into(),
                complete_ts: complete_ts.into(),
                expiration_ts: expiration_ts as i64,
                zip_object_key: zip_object_key.to_string(),
                format: format.into(),
                component: component.serialize_to_string(),
            },
            Export::Failed {
                start_ts,
                failed_ts,
                format,
                component,
            } => SerializedExport::Failed {
                start_ts: start_ts.into(),
                failed_ts: failed_ts.into(),
                format: format.into(),
                component: component.serialize_to_string(),
            },
        })
    }
}

impl TryFrom<SerializedExport> for Export {
    type Error = anyhow::Error;

    fn try_from(value: SerializedExport) -> Result<Self, Self::Error> {
        Ok(match value {
            SerializedExport::Requested { format, component } => Export::Requested {
                format: format.into(),
                component: ComponentId::deserialize_from_string(component.as_deref())?,
            },
            SerializedExport::InProgress {
                start_ts,
                format,
                component,
            } => Export::InProgress {
                start_ts: start_ts.try_into()?,
                format: format.into(),
                component: ComponentId::deserialize_from_string(component.as_deref())?,
            },
            SerializedExport::Completed {
                start_ts,
                complete_ts,
                expiration_ts,
                zip_object_key,
                format,
                component,
            } => Export::Completed {
                start_ts: start_ts.try_into()?,
                complete_ts: complete_ts.try_into()?,
                expiration_ts: expiration_ts as u64,
                zip_object_key: zip_object_key.try_into()?,
                format: format.into(),
                component: ComponentId::deserialize_from_string(component.as_deref())?,
            },
            SerializedExport::Failed {
                start_ts,
                failed_ts,
                format,
                component,
            } => Export::Failed {
                start_ts: start_ts.try_into()?,
                failed_ts: failed_ts.try_into()?,
                format: format.into(),
                component: ComponentId::deserialize_from_string(component.as_deref())?,
            },
        })
    }
}

codegen_convex_serialization!(Export, SerializedExport);

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

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[serde(tag = "format")]
#[serde(rename_all = "snake_case")]
enum SerializedExportFormat {
    Zip { include_storage: bool },
}

impl From<ExportFormat> for SerializedExportFormat {
    fn from(value: ExportFormat) -> Self {
        let ExportFormat::Zip { include_storage } = value;
        SerializedExportFormat::Zip { include_storage }
    }
}

impl From<SerializedExportFormat> for ExportFormat {
    fn from(value: SerializedExportFormat) -> Self {
        let SerializedExportFormat::Zip { include_storage } = value;
        ExportFormat::Zip { include_storage }
    }
}

codegen_convex_serialization!(ExportFormat, SerializedExportFormat);

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

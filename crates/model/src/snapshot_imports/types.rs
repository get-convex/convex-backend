use common::types::{
    MemberId,
    ObjectKey,
    TableName,
};
use serde::{
    Deserialize,
    Serialize,
};
use sync_types::Timestamp;
use value::codegen_convex_serialization;

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(any(test, feature = "testing"), derive(proptest_derive::Arbitrary))]
pub struct SnapshotImport {
    pub state: ImportState,
    pub format: ImportFormat,
    pub mode: ImportMode,
    pub object_key: ObjectKey,
    pub member_id: Option<MemberId>,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
struct SerializedSnapshotImport {
    state: SerializedImportState,
    format: SerializedImportFormat,
    mode: String,
    object_key: String,
    member_id: Option<i64>,
}

impl TryFrom<SnapshotImport> for SerializedSnapshotImport {
    type Error = anyhow::Error;

    fn try_from(import: SnapshotImport) -> anyhow::Result<SerializedSnapshotImport> {
        Ok(SerializedSnapshotImport {
            state: import.state.try_into()?,
            format: import.format.try_into()?,
            mode: import.mode.to_string(),
            object_key: import.object_key.to_string(),
            member_id: import.member_id.map(|member_id| member_id.0 as i64),
        })
    }
}

impl TryFrom<SerializedSnapshotImport> for SnapshotImport {
    type Error = anyhow::Error;

    fn try_from(import: SerializedSnapshotImport) -> anyhow::Result<SnapshotImport> {
        Ok(SnapshotImport {
            state: import.state.try_into()?,
            format: import.format.try_into()?,
            mode: import.mode.parse()?,
            object_key: import.object_key.try_into()?,
            member_id: import.member_id.map(|member_id| MemberId(member_id as u64)),
        })
    }
}

codegen_convex_serialization!(SnapshotImport, SerializedSnapshotImport);

#[derive(Debug, Clone, Eq, PartialEq)]
#[cfg_attr(any(test, feature = "testing"), derive(proptest_derive::Arbitrary))]
pub enum ImportFormat {
    Csv(TableName),
    JsonLines(TableName),
    JsonArray(TableName),
    Zip,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "format")]
pub enum SerializedImportFormat {
    #[serde(rename = "csv")]
    Csv { table: String },
    #[serde(rename = "jsonl")]
    JsonLines { table: String },
    #[serde(rename = "json_array")]
    JsonArray { table: String },
    #[serde(rename = "zip")]
    Zip,
}

impl TryFrom<ImportFormat> for SerializedImportFormat {
    type Error = anyhow::Error;

    fn try_from(format: ImportFormat) -> anyhow::Result<SerializedImportFormat> {
        match format {
            ImportFormat::Csv(table) => Ok(SerializedImportFormat::Csv {
                table: table.to_string(),
            }),
            ImportFormat::JsonLines(table) => Ok(SerializedImportFormat::JsonLines {
                table: table.to_string(),
            }),
            ImportFormat::JsonArray(table) => Ok(SerializedImportFormat::JsonArray {
                table: table.to_string(),
            }),
            ImportFormat::Zip => Ok(SerializedImportFormat::Zip),
        }
    }
}

impl TryFrom<SerializedImportFormat> for ImportFormat {
    type Error = anyhow::Error;

    fn try_from(format: SerializedImportFormat) -> anyhow::Result<ImportFormat> {
        match format {
            SerializedImportFormat::Csv { table } => Ok(ImportFormat::Csv(table.parse()?)),
            SerializedImportFormat::JsonLines { table } => {
                Ok(ImportFormat::JsonLines(table.parse()?))
            },
            SerializedImportFormat::JsonArray { table } => {
                Ok(ImportFormat::JsonArray(table.parse()?))
            },
            SerializedImportFormat::Zip => Ok(ImportFormat::Zip),
        }
    }
}

mod import_format_serde {
    use value::codegen_convex_serialization;

    use super::{
        ImportFormat,
        SerializedImportFormat,
    };

    codegen_convex_serialization!(ImportFormat, SerializedImportFormat);
}

/*
      │
      │
   CLI│uploads
┌─────▼─────┐
│ Uploaded  │
└─────┬─────┘
      │
Import│Worker parses
      ├─────────────────────┐
      │                     │
┌─────▼────────────────┐    │
│WaitingForConfirmation│    │
└─────┬────────────────┘    │
      │                     │
CLI requests confirmation   │
      │                     │
┌─────▼──────┐              │
│ InProgress │              │
└─────┬──────┘              │
      │                     │
Import│Worker imports       │
      ├─────────────────┐   │
      │                 │   │
┌─────▼──────┐      ┌───▼───▼─┐
│ Completed  │      │ Failed  │
└────────────┘      └─────────┘
 */
#[derive(Debug, Clone, Eq, PartialEq)]
#[cfg_attr(any(test, feature = "testing"), derive(proptest_derive::Arbitrary))]
pub enum ImportState {
    Uploaded,
    WaitingForConfirmation {
        info_message: String,
        require_manual_confirmation: bool,
    },
    InProgress {
        progress_message: String,
        checkpoint_messages: Vec<String>,
    },
    Completed {
        ts: Timestamp,
        num_rows_written: usize,
    },
    Failed(String),
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "state")]
enum SerializedImportState {
    Uploaded,
    WaitingForConfirmation {
        message_to_confirm: Option<String>,
        require_manual_confirmation: Option<bool>,
    },
    InProgress {
        progress_message: Option<String>,
        checkpoint_messages: Vec<String>,
    },
    Completed {
        timestamp: i64,
        num_rows_written: i64,
    },
    Failed {
        error_message: String,
    },
}

impl TryFrom<ImportState> for SerializedImportState {
    type Error = anyhow::Error;

    fn try_from(state: ImportState) -> anyhow::Result<SerializedImportState> {
        match state {
            ImportState::Uploaded => Ok(SerializedImportState::Uploaded),
            ImportState::WaitingForConfirmation {
                info_message,
                require_manual_confirmation,
            } => Ok(SerializedImportState::WaitingForConfirmation {
                message_to_confirm: Some(info_message),
                require_manual_confirmation: Some(require_manual_confirmation),
            }),
            ImportState::InProgress {
                progress_message,
                checkpoint_messages,
            } => Ok(SerializedImportState::InProgress {
                progress_message: Some(progress_message),
                checkpoint_messages,
            }),
            ImportState::Completed {
                ts,
                num_rows_written,
            } => Ok(SerializedImportState::Completed {
                timestamp: i64::from(ts),
                num_rows_written: num_rows_written as i64,
            }),
            ImportState::Failed(message) => Ok(SerializedImportState::Failed {
                error_message: message,
            }),
        }
    }
}

impl TryFrom<SerializedImportState> for ImportState {
    type Error = anyhow::Error;

    fn try_from(state: SerializedImportState) -> anyhow::Result<ImportState> {
        match state {
            SerializedImportState::Uploaded => Ok(ImportState::Uploaded),
            SerializedImportState::WaitingForConfirmation {
                message_to_confirm,
                require_manual_confirmation,
            } => Ok(ImportState::WaitingForConfirmation {
                info_message: message_to_confirm.unwrap_or_default(),
                require_manual_confirmation: require_manual_confirmation.unwrap_or(true),
            }),
            SerializedImportState::InProgress {
                progress_message,
                checkpoint_messages,
            } => Ok(ImportState::InProgress {
                progress_message: progress_message.unwrap_or_else(|| "Importing".to_string()),
                checkpoint_messages,
            }),
            SerializedImportState::Completed {
                timestamp,
                num_rows_written,
            } => Ok(ImportState::Completed {
                ts: timestamp.try_into()?,
                num_rows_written: num_rows_written as usize,
            }),
            SerializedImportState::Failed { error_message } => {
                Ok(ImportState::Failed(error_message))
            },
        }
    }
}

mod import_state_serde {
    use value::codegen_convex_serialization;

    use super::{
        ImportState,
        SerializedImportState,
    };

    codegen_convex_serialization!(ImportState, SerializedImportState);
}

#[derive(
    Debug, Default, Deserialize, Clone, Copy, Eq, PartialEq, strum::EnumString, strum::Display,
)]
#[cfg_attr(any(test, feature = "testing"), derive(proptest_derive::Arbitrary))]
#[serde(rename_all = "camelCase")]
pub enum ImportMode {
    Append,
    Replace,
    #[default]
    RequireEmpty,
}

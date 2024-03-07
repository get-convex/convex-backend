use std::str::FromStr;

use anyhow::Context;
use common::{
    obj,
    types::{
        MemberId,
        ObjectKey,
        TableName,
    },
};
use serde::Deserialize;
use sync_types::Timestamp;
use value::{
    val,
    ConvexObject,
    ConvexValue,
};

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(any(test, feature = "testing"), derive(proptest_derive::Arbitrary))]
pub struct SnapshotImport {
    pub state: ImportState,
    pub format: ImportFormat,
    pub mode: ImportMode,
    pub object_key: ObjectKey,
    pub member_id: Option<MemberId>,
}

#[derive(Debug, Clone, Eq, PartialEq)]
#[cfg_attr(any(test, feature = "testing"), derive(proptest_derive::Arbitrary))]
pub enum ImportFormat {
    Csv(TableName),
    JsonLines(TableName),
    JsonArray(TableName),
    Zip,
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

impl TryFrom<SnapshotImport> for ConvexObject {
    type Error = anyhow::Error;

    fn try_from(import: SnapshotImport) -> anyhow::Result<ConvexObject> {
        obj!(
            "state" => ConvexValue::Object(import.state.try_into()?),
            "format" => ConvexValue::Object(import.format.try_into()?),
            "mode" => import.mode.to_string(),
            "object_key" => import.object_key.to_string(),
            "member_id" => match import.member_id {
                None => val!(null),
                Some(member_id) => val!(u64::from(member_id) as i64),
            },
        )
    }
}

impl TryFrom<ConvexObject> for SnapshotImport {
    type Error = anyhow::Error;

    fn try_from(o: ConvexObject) -> anyhow::Result<SnapshotImport> {
        let state = match o.get("state") {
            Some(ConvexValue::Object(o)) => ImportState::try_from(o.clone())?,
            _ => anyhow::bail!("invalid state: {:?}", o),
        };
        let format = match o.get("format") {
            Some(ConvexValue::Object(o)) => ImportFormat::try_from(o.clone())?,
            _ => anyhow::bail!("invalid format: {:?}", o),
        };
        let mode = match o.get("mode") {
            Some(ConvexValue::String(mode)) => ImportMode::from_str(mode)?,
            _ => anyhow::bail!("invalid mode: {:?}", o),
        };
        let object_key = match o.get("object_key") {
            Some(ConvexValue::String(object_key)) => object_key.clone().try_into()?,
            _ => anyhow::bail!("invalid object_key: {:?}", o),
        };
        let member_id = match o.get("member_id") {
            Some(ConvexValue::Int64(member_id)) => Some(MemberId(*member_id as u64)),
            None | Some(ConvexValue::Null) => None,
            _ => anyhow::bail!("invalid member_id: {o:?}"),
        };
        Ok(Self {
            state,
            format,
            mode,
            object_key,
            member_id,
        })
    }
}

impl TryFrom<ImportState> for ConvexObject {
    type Error = anyhow::Error;

    fn try_from(import: ImportState) -> anyhow::Result<ConvexObject> {
        match import {
            ImportState::Uploaded => obj!("state" => "uploaded"),
            ImportState::WaitingForConfirmation {
                info_message,
                require_manual_confirmation,
            } => obj!(
                "state" => "waiting_for_confirmation",
                "message_to_confirm" => info_message,
                "require_manual_confirmation" => require_manual_confirmation,
            ),
            ImportState::InProgress {
                progress_message,
                checkpoint_messages,
            } => {
                let checkpoint_messages: Vec<_> = checkpoint_messages
                    .into_iter()
                    .map(ConvexValue::try_from)
                    .try_collect()?;
                obj!(
                    "state" => "in_progress",
                    "progress_message" => progress_message,
                    "checkpoint_messages" => checkpoint_messages,
                )
            },
            ImportState::Completed {
                ts,
                num_rows_written,
            } => {
                obj!("state" => "completed", "timestamp" => i64::from(ts), "num_rows_written" => num_rows_written as i64)
            },
            ImportState::Failed(message) => {
                obj!("state" => "failed", "error_message" => message)
            },
        }
    }
}

impl TryFrom<ConvexObject> for ImportState {
    type Error = anyhow::Error;

    fn try_from(o: ConvexObject) -> anyhow::Result<ImportState> {
        let state = match o.get("state") {
            Some(ConvexValue::String(state_variant)) => match &**state_variant {
                "uploaded" => ImportState::Uploaded,
                "waiting_for_confirmation" => ImportState::WaitingForConfirmation {
                    info_message: match o.get("message_to_confirm") {
                        Some(ConvexValue::String(message)) => message.clone().into(),
                        _ => String::new(),
                    },
                    require_manual_confirmation: match o.get("require_manual_confirmation") {
                        Some(ConvexValue::Boolean(require_manual_confirmation)) => {
                            *require_manual_confirmation
                        },
                        _ => true,
                    },
                },
                "in_progress" => {
                    let progress_message = match o.get("progress_message") {
                        Some(ConvexValue::String(message)) => message.clone().into(),
                        _ => "Importing".to_string(),
                    };
                    let checkpoint_messages = match o.get("checkpoint_messages") {
                        Some(ConvexValue::Array(messages)) => messages
                            .into_iter()
                            .filter_map(|message| match message {
                                ConvexValue::String(message) => Some(String::from(message.clone())),
                                _ => None,
                            })
                            .collect(),
                        _ => vec![],
                    };
                    ImportState::InProgress {
                        progress_message,
                        checkpoint_messages,
                    }
                },
                "completed" => match (o.get("timestamp"), o.get("num_rows_written")) {
                    (
                        Some(ConvexValue::Int64(timestamp)),
                        Some(ConvexValue::Int64(num_rows_written)),
                    ) => ImportState::Completed {
                        ts: (*timestamp).try_into()?,
                        num_rows_written: *num_rows_written as usize,
                    },
                    _ => anyhow::bail!("invalid completed timestamp: {o:?}"),
                },
                "failed" => match o.get("error_message") {
                    Some(ConvexValue::String(message)) => ImportState::Failed(message.to_string()),
                    _ => anyhow::bail!("invalid error message: {o:?}"),
                },
                _ => anyhow::bail!("invalid state variant: {state_variant}"),
            },
            _ => anyhow::bail!("invalid state variant: {o:?}"),
        };
        Ok(state)
    }
}

impl TryFrom<ImportFormat> for ConvexObject {
    type Error = anyhow::Error;

    fn try_from(format: ImportFormat) -> anyhow::Result<ConvexObject> {
        match format {
            ImportFormat::Zip => obj!("format" => "zip"),
            ImportFormat::Csv(table_name) => {
                obj!("format" => "csv", "table" => table_name.to_string())
            },
            ImportFormat::JsonLines(table_name) => {
                obj!("format" => "jsonl", "table" => table_name.to_string())
            },
            ImportFormat::JsonArray(table_name) => {
                obj!("format" => "json_array", "table" => table_name.to_string())
            },
        }
    }
}

impl TryFrom<ConvexObject> for ImportFormat {
    type Error = anyhow::Error;

    fn try_from(o: ConvexObject) -> anyhow::Result<ImportFormat> {
        let table_name = match o.get("table") {
            Some(ConvexValue::String(table)) => Some(table.parse()?),
            None => None,
            _ => anyhow::bail!("invalid format table: {o:?}"),
        };
        let format = match o.get("format") {
            Some(ConvexValue::String(format_variant)) => match &**format_variant {
                "zip" => ImportFormat::Zip,
                "csv" => ImportFormat::Csv(table_name.context("expected table for csv")?),
                "jsonl" => ImportFormat::JsonLines(table_name.context("expected table for jsonl")?),
                "json_array" => {
                    ImportFormat::JsonArray(table_name.context("expected table for json_array")?)
                },
                _ => anyhow::bail!("invalid format variant: {format_variant}"),
            },
            _ => anyhow::bail!("invalid format variant: {o:?}"),
        };
        Ok(format)
    }
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;
    use sync_types::testing::assert_roundtrips;
    use value::ConvexObject;

    use crate::snapshot_imports::types::SnapshotImport;

    proptest! {
        #![proptest_config(
            ProptestConfig { failure_persistence: None, ..ProptestConfig::default() }
        )]

        #[test]
        fn test_snapshot_import_roundtrip(v in any::<SnapshotImport>()) {
            assert_roundtrips::<SnapshotImport, ConvexObject>(v);
        }
    }
}

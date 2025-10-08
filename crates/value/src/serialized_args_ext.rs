use anyhow::Context;
use errors::ErrorMetadata;
use serde_json::Value as JsonValue;
use sync_types::types::SerializedArgs;

pub trait SerializedArgsExt {
    fn into_args(self) -> anyhow::Result<Vec<JsonValue>>;
}

impl SerializedArgsExt for SerializedArgs {
    fn into_args(self) -> anyhow::Result<Vec<JsonValue>> {
        serde_json::from_str(self.0.get()).context(ErrorMetadata::bad_request(
            "InvalidArguments",
            "Invalid arguments provided",
        ))
    }
}

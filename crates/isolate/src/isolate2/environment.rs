use common::{
    log_lines::{
        LogLevel,
        SystemLogMetadata,
    },
    runtime::UnixTimestamp,
    types::{
        EnvVarName,
        EnvVarValue,
    },
};
use rand_chacha::ChaCha12Rng;
use serde_json::Value as JsonValue;

pub trait Environment {
    fn syscall(&mut self, name: &str, args: JsonValue) -> anyhow::Result<JsonValue>;

    fn trace(&mut self, level: LogLevel, messages: Vec<String>) -> anyhow::Result<()>;
    fn trace_system(
        &mut self,
        level: LogLevel,
        messages: Vec<String>,
        system_log_metadata: SystemLogMetadata,
    ) -> anyhow::Result<()>;

    fn rng(&mut self) -> anyhow::Result<&mut ChaCha12Rng>;
    fn unix_timestamp(&mut self) -> anyhow::Result<UnixTimestamp>;

    fn get_environment_variable(&mut self, name: EnvVarName)
        -> anyhow::Result<Option<EnvVarValue>>;
}

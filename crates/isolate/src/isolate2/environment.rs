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
use value::{
    NamespacedTableMapping,
    TableMappingValue,
};

#[derive(Debug)]
pub struct EnvironmentOutcome {
    pub observed_rng: bool,
    pub observed_time: bool,
    pub observed_identity: bool,
}

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
    fn unix_timestamp_non_deterministic(&mut self) -> anyhow::Result<UnixTimestamp>;

    fn get_environment_variable(&mut self, name: EnvVarName)
        -> anyhow::Result<Option<EnvVarValue>>;

    // Signal that we've finished the import phase and are ready to start execution.
    fn start_execution(&mut self) -> anyhow::Result<()>;

    // Signal that we've finished execution.
    fn finish_execution(&mut self) -> anyhow::Result<EnvironmentOutcome>;

    fn get_all_table_mappings(&mut self) -> anyhow::Result<NamespacedTableMapping>;
    fn get_table_mapping_without_system_tables(&mut self) -> anyhow::Result<TableMappingValue>;
}

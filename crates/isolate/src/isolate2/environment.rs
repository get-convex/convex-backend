use serde_json::Value as JsonValue;

pub trait Environment {
    fn syscall(&mut self, name: &str, args: JsonValue) -> anyhow::Result<JsonValue>;
}

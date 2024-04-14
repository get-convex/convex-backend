use anyhow::anyhow;
use serde_json::Value as JsonValue;

use super::OpProvider;

#[convex_macro::v8_op]
pub fn op_get_table_mapping_without_system_tables<'b, P: OpProvider<'b>>(
    provider: &mut P,
) -> anyhow::Result<JsonValue> {
    let mapping = provider.get_table_mapping_without_system_tables()?;
    serde_json::to_value(mapping).map_err(|_| anyhow!("Couldn’t serialize the table mapping"))
}

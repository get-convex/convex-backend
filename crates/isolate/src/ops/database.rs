use anyhow::anyhow;
use common::runtime::Runtime;
use serde_json::Value as JsonValue;

use crate::{
    environment::IsolateEnvironment,
    execution_scope::ExecutionScope,
};

impl<'a, 'b: 'a, RT: Runtime, E: IsolateEnvironment<RT>> ExecutionScope<'a, 'b, RT, E> {
    #[convex_macro::v8_op]
    pub fn op_getTableMapping(&mut self) -> anyhow::Result<JsonValue> {
        let state = self.state_mut();
        state.environment.get_table_mapping().and_then(|mapping| {
            serde_json::to_value(mapping)
                .map_err(|_| anyhow!("Couldn’t serialize the table mapping"))
        })
    }
}

use anyhow::Context;
use common::runtime::Runtime;

use crate::{
    environment::IsolateEnvironment,
    execution_scope::ExecutionScope,
};

impl<'a, 'b: 'a, RT: Runtime, E: IsolateEnvironment<RT>> ExecutionScope<'a, 'b, RT, E> {
    #[convex_macro::v8_op]
    pub fn op_environmentVariables_get(&mut self, name: String) -> anyhow::Result<Option<String>> {
        let name = name.parse()?;
        let state = self.state_mut()?;
        let environment_variable = state.environment.get_environment_variable(name)?;
        let value = environment_variable.map(|env_var| env_var.to_string());
        Ok(value)
    }
}

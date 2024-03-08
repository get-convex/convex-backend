use common::runtime::Runtime;
use rand::Rng;
use serde_json::value::Number as JsonNumber;

use crate::{
    environment::IsolateEnvironment,
    execution_scope::ExecutionScope,
};

impl<'a, 'b: 'a, RT: Runtime, E: IsolateEnvironment<RT>> ExecutionScope<'a, 'b, RT, E> {
    #[convex_macro::v8_op]
    pub fn op_random(&mut self) -> anyhow::Result<JsonNumber> {
        let state = self.state_mut()?;
        let n = JsonNumber::from_f64(state.environment.rng()?.gen())
            .expect("f64's distribution returned a NaN or infinity?");
        Ok(n)
    }
}

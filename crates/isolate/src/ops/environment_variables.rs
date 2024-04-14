use anyhow::Context;

use super::OpProvider;

#[convex_macro::v8_op]
pub fn op_environment_variables_get<'b, P: OpProvider<'b>>(
    provider: &mut P,
    name: String,
) -> anyhow::Result<Option<String>> {
    let name = name.parse()?;
    let environment_variable = provider.get_environment_variable(name)?;
    let value = environment_variable.map(|env_var| env_var.to_string());
    Ok(value)
}

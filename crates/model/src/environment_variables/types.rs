use std::collections::BTreeMap;

pub use common::types::{
    EnvVarName,
    EnvVarValue,
    EnvironmentVariable,
};
use value::{
    obj,
    ConvexObject,
    ConvexValue,
};

#[derive(Debug, PartialEq, Clone)]
pub struct PersistedEnvironmentVariable(pub EnvironmentVariable);

impl TryFrom<PersistedEnvironmentVariable> for ConvexObject {
    type Error = anyhow::Error;

    fn try_from(
        PersistedEnvironmentVariable(
            EnvironmentVariable { name, value }
        ): PersistedEnvironmentVariable,
    ) -> anyhow::Result<ConvexObject> {
        obj!("name" => String::from(name), "value" => String::from(value))
    }
}

impl TryFrom<ConvexObject> for PersistedEnvironmentVariable {
    type Error = anyhow::Error;

    fn try_from(obj: ConvexObject) -> anyhow::Result<PersistedEnvironmentVariable> {
        let mut fields = BTreeMap::from(obj);
        let name: String = match fields.remove("name") {
            Some(ConvexValue::String(s)) => s.into(),
            v => anyhow::bail!("Invalid name field for EnvironmentVariable: {v:?}"),
        };
        let value: String = match fields.remove("value") {
            Some(ConvexValue::String(s)) => s.into(),
            v => anyhow::bail!("Invalid value field for EnvironmentVariable: {v:?}"),
        };
        Ok(Self(EnvironmentVariable {
            name: name.parse()?,
            value: value.parse()?,
        }))
    }
}

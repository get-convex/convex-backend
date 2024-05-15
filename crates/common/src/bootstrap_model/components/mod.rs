pub mod definition;

use std::collections::BTreeMap;

use serde::{
    Deserialize,
    Serialize,
};
use value::{
    codegen_convex_serialization,
    identifier::Identifier,
    InternalId,
};

use crate::components::{
    ComponentName,
    Resource,
    SerializedResource,
};

#[derive(Debug, Clone, Eq, PartialEq)]
#[cfg_attr(any(test, feature = "testing"), derive(proptest_derive::Arbitrary))]
pub struct ComponentMetadata {
    pub definition_id: InternalId,
    pub parent_and_name: Option<(InternalId, ComponentName)>,
    pub args: BTreeMap<Identifier, Resource>,
}

#[derive(Debug, Serialize, Deserialize)]
struct SerializedComponentMetadata {
    pub definition_id: String,
    pub parent: Option<String>,
    pub name: Option<String>,
    pub args: Vec<(String, SerializedResource)>,
}

impl TryFrom<ComponentMetadata> for SerializedComponentMetadata {
    type Error = anyhow::Error;

    fn try_from(m: ComponentMetadata) -> anyhow::Result<Self> {
        let (parent, name) = match m.parent_and_name {
            Some((parent, name)) => (Some(parent.to_string()), Some(name.to_string())),
            None => (None, None),
        };
        Ok(Self {
            definition_id: m.definition_id.to_string(),
            parent,
            name,
            args: m
                .args
                .into_iter()
                .map(|(k, v)| anyhow::Ok((String::from(k), v.try_into()?)))
                .try_collect()?,
        })
    }
}

impl TryFrom<SerializedComponentMetadata> for ComponentMetadata {
    type Error = anyhow::Error;

    fn try_from(m: SerializedComponentMetadata) -> anyhow::Result<Self> {
        Ok(Self {
            definition_id: m.definition_id.parse()?,
            parent_and_name: match (m.parent, m.name) {
                (Some(parent), Some(name)) => Some((parent.parse()?, name.parse()?)),
                (None, None) => None,
                _ => anyhow::bail!("expected both parent and name or neither"),
            },
            args: m
                .args
                .into_iter()
                .map(|(k, v)| anyhow::Ok((k.parse()?, v.try_into()?)))
                .try_collect()?,
        })
    }
}

codegen_convex_serialization!(ComponentMetadata, SerializedComponentMetadata);
